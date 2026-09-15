#!/usr/bin/env python3
"""Validate runtime-v1 compile/capture JSONLines transcripts; no compiler/model execution.

Usage: check_runtime.py transcript.jsonl (or - for stdin). Requests and responses
must appear in observed transport order in one transcript. Use --requests FILE
for a request stream followed by a separate response stream; every response older
than the newest supplied request is then classified as stale for preview purposes.
Use --compiler /explicit/path to run a local compiler with transcript as requests.
Probe mode bounds input/output, drains stderr, closes stdin, and enforces --timeout.
Exit 1 means invalid protocol, not stale but valid asynchronous output.
"""
import argparse
import copy
import hashlib
import base64
import binascii
import struct
import zlib
import json
import math
import io
import os
import selectors
import signal
import subprocess
import time
import unicodedata
from pathlib import PurePosixPath, PureWindowsPath
import sys


class Invalid(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Invalid(message)


def integer(value):
    return isinstance(value, int) and not isinstance(value, bool)


def number(value):
    try:
        return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)
    except OverflowError:
        return False


def string(value, label, nonempty=False):
    require(isinstance(value, str) and (bool(value) or not nonempty), label + ' must be a string' + (' (nonempty)' if nonempty else ''))
    try:
        value.encode('utf-8')
    except UnicodeEncodeError as exc:
        raise Invalid(label + ' contains an invalid Unicode surrogate') from exc


def path(value):
    string(value, 'path', True)
    require(not PurePosixPath(value).is_absolute() and not PureWindowsPath(value).drive
            and not value.startswith('\\') and '..' not in value.replace('\\', '/').split('/')
            and '\x00' not in value, 'path must be project-relative without parent traversal')


def object_value(value, label):
    require(isinstance(value, dict), label + ' must be an object')


def array(value, label):
    require(isinstance(value, list), label + ' must be an array')


def layout_capabilities(payload):
    values = payload.get('layout_capabilities', [])
    array(values, 'layout_capabilities')
    require(len(values) <= 16, 'layout_capabilities exceeds 16 entries')
    for value in values:
        string(value, 'layout capability', True)
        require(len(value.encode('utf-8')) <= 64, 'layout capability exceeds 64 UTF-8 bytes')
    require(len(set(values)) == len(values), 'duplicate layout capability')
    return frozenset(values)


def request_date(payload):
    """`payload.date` -- the civil date `\\today` renders, YYYY-MM-DD.

    Optional: an absent field means the compiler uses the Unix epoch, which is
    what it printed before the field existed, so old producers and committed
    fixtures stay byte-identical. A present value is strict -- no time, no
    timezone, no alternative separator -- because a caller that sent a date
    meant it, and guessing is the bug this field exists to end. See
    protocol/proposals/runtime-v1-request-date.md.
    """
    if 'date' not in payload:
        return None
    value = payload['date']
    string(value, 'date', True)
    require(len(value) == 10 and value[4] == '-' and value[7] == '-',
            'date must be a civil date in YYYY-MM-DD form')
    parts = (value[0:4], value[5:7], value[8:10])
    require(all(part.isdigit() and part.isascii() for part in parts),
            'date must be a civil date in YYYY-MM-DD form')
    year, month, day = (int(part) for part in parts)
    require(1 <= year <= 9999, 'date year must be 0001-9999')
    require(1 <= month <= 12, 'date month must be 01-12')
    leap = year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)
    lengths = (31, 29 if leap else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)
    require(1 <= day <= lengths[month - 1],
            'date names a day that does not exist in that month')
    return value


def strict_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON key: ' + key)
        result[key] = value
    return result


def image_metadata(value):
    """Bound and inspect encoded image structure; this is not a pixel decoder."""
    object_value(value, 'image')
    mime, encoded = value.get('mime_type'), value.get('data_base64')
    require(mime in ('image/png', 'image/jpeg'), 'unsupported image.mime_type')
    string(encoded, 'image.data_base64', True)
    limit = 8 * 1024 * 1024
    require(len(encoded) <= 4 * ((limit + 2) // 3), 'encoded image exceeds 8 MiB decoded limit')
    try:
        data = base64.b64decode(encoded, validate=True)
    except (ValueError, binascii.Error) as exc:
        raise Invalid('image.data_base64 is malformed') from exc
    require(0 < len(data) <= limit, 'image exceeds 8 MiB decoded limit')
    width = height = 0
    if mime == 'image/png':
        require(data.startswith(b'\x89PNG\r\n\x1a\n'), 'PNG MIME/signature mismatch')
        offset, kinds = 8, []
        while offset < len(data):
            require(offset + 12 <= len(data), 'truncated PNG chunk')
            size = int.from_bytes(data[offset:offset + 4], 'big')
            end = offset + 12 + size
            require(end <= len(data), 'truncated PNG chunk payload')
            kind, body = data[offset + 4:offset + 8], data[offset + 8:end - 4]
            require(zlib.crc32(data[offset + 4:end - 4]) == int.from_bytes(data[end - 4:end], 'big'),
                    'PNG chunk CRC mismatch')
            if not kinds:
                require(kind == b'IHDR' and size == 13, 'PNG must begin with 13-byte IHDR')
                width, height = struct.unpack('>II', body[:8])
            elif kind == b'IHDR':
                raise Invalid('duplicate PNG IHDR')
            kinds.append(kind)
            offset = end
            if kind == b'IEND':
                require(size == 0 and offset == len(data), 'invalid PNG IEND/trailing data')
                break
        require(kinds and kinds[-1] == b'IEND' and b'IDAT' in kinds, 'PNG lacks IDAT/IEND')
    else:
        require(data.startswith(b'\xff\xd8') and data.endswith(b'\xff\xd9'), 'JPEG MIME/signature or EOI mismatch')
        offset, saw_scan = 2, False
        while offset < len(data) - 2:
            require(data[offset] == 255, 'invalid JPEG segment marker')
            while offset < len(data) and data[offset] == 255:
                offset += 1
            require(offset < len(data), 'truncated JPEG marker')
            marker = data[offset]
            offset += 1
            require(marker not in (0, 0xd8, 0xd9) and offset + 2 <= len(data), 'invalid JPEG segment')
            size = int.from_bytes(data[offset:offset + 2], 'big')
            require(size >= 2 and offset + size <= len(data), 'truncated JPEG segment')
            if marker == 0xda:
                require(size >= 6 and size == 6 + 2 * data[offset + 2], 'invalid JPEG scan header')
                saw_scan = True
                break  # Entropy/pixel decoding is outside this metadata check.
            if marker in (0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf):
                require(size >= 8 and size == 8 + 3 * data[offset + 7], 'invalid JPEG frame header')
                height, width = struct.unpack('>HH', data[offset + 3:offset + 7])
            offset += size
        require(saw_scan, 'JPEG lacks a scan header')
    require(0 < width <= 16384 and 0 < height <= 16384 and width * height <= 64 * 1024 * 1024,
            'image dimensions missing or exceed 16384/64-megapixel bounds')
    return {'width': width, 'height': height, 'decoded_bytes': len(data)}


class Validator:
    def __init__(self):
        self.requests = {}
        self.completed = set()
        self.latest = {}
        self.latest_request = {}
        self.results = []
        self.captures = {}
        self.capture_requests = {}

    def source(self, value, documents):
        object_value(value, 'source')
        name = value.get('path')
        path(name)
        require(name in documents, 'source.path is absent from the request documents: ' + name)
        start, end = value.get('start_byte'), value.get('end_byte')
        data = documents[name].encode('utf-8')
        require(integer(start) and integer(end) and 0 <= start <= end <= len(data),
                'source byte range must satisfy 0 <= start_byte <= end_byte <= UTF-8 length')
        try:
            data[:start].decode('utf-8')
            data[:end].decode('utf-8')
        except UnicodeDecodeError as exc:
            raise Invalid('source offsets split a UTF-8 character') from exc

    def capture(self, ident, kind, payload):
        capture_id = payload.get('capture_id')
        string(capture_id, 'capture_id', True)
        if kind == 'capture_submit':
            require(ident not in self.requests, 'capture request id collides with compile request')
            string(payload.get('destination_id'), 'destination_id', True)
            require(integer(payload.get('base_revision')) and payload['base_revision'] >= 0,
                    'base_revision must be a nonnegative integer')
            string(payload.get('instructions'), 'instructions')
            image_metadata(payload.get('image'))
            require(ident not in self.capture_requests or self.capture_requests[ident] == capture_id,
                    'request id reused for a different capture')
            if capture_id in self.captures:
                require(payload == self.captures[capture_id]['submission'],
                        'capture_id reused with conflicting image/destination/revision/context')
            else:
                self.captures[capture_id] = {'submission': payload, 'responses': {}}
            self.capture_requests[ident] = capture_id
            return
        require(self.capture_requests.get(ident) == capture_id,
                'capture response id/capture_id does not match a submission')
        capture = self.captures[capture_id]
        if kind == 'capture_proposal':
            string(payload.get('latex'), 'capture_proposal.latex')
            for field in ('ambiguities', 'required_dependencies'):
                array(payload.get(field), field)
                for item in payload[field]:
                    string(item, field + ' item')
        previous = capture['responses'].get(kind)
        require(previous is None or previous == payload, 'conflicting duplicate ' + kind)
        if previous is None:
            capture['responses'][kind] = payload
            self.results.append({'id': ident, 'capture_id': capture_id, 'type': kind,
                                 'insertion': 'not_validated_by_runtime_v1'})

    def consume(self, value):
        object_value(value, 'envelope')
        require(integer(value.get('protocol_version')) and value['protocol_version'] == 1,
                'unsupported protocol_version (expected integer 1)')
        ident, kind, payload = value.get('id'), value.get('type'), value.get('payload')
        string(ident, 'id', True)
        object_value(payload, 'payload')
        if kind in ('capture_submit', 'capture_received', 'capture_proposal'):
            self.capture(ident, kind, payload)
            return
        require(kind in ('compile', 'compile_result', 'error'),
                'unsupported message type for runtime-v1 transcript validator: ' + str(kind))
        if kind == 'error':
            # runtime-v1 does not yet define the error payload schema.
            require(ident in self.requests, 'error has no matching compile request id')
            require(ident not in self.completed, 'duplicate terminal response id')
            self.completed.add(ident)
            self.results.append({'id': ident, 'type': 'error', 'preview': 'unchanged',
                                 'note': 'error payload schema is not defined by runtime-v1'})
            return
        project, revision = payload.get('project_id'), payload.get('revision')
        string(project, 'project_id', True)
        require(integer(revision) and revision >= 0, 'revision must be a nonnegative integer')
        if kind == 'compile':
            require(ident not in self.capture_requests, 'compile request id collides with capture request')
            require(ident not in self.requests, 'duplicate request id')
            require(project not in self.latest or revision >= self.latest[project],
                    'compile request revisions must not decrease within a project/session')
            path(payload.get('entry_path'))
            documents = payload.get('documents')
            array(documents, 'documents')
            mapped = {}
            for document in documents:
                object_value(document, 'document')
                name, text = document.get('path'), document.get('text')
                path(name)
                string(text, 'document.text')
                require(name not in mapped, 'duplicate document path: ' + name)
                mapped[name] = text
            require(payload['entry_path'] in mapped, 'entry_path must exist in request documents')
            requested = layout_capabilities(payload)
            request_date(payload)
            self.requests[ident] = (project, revision, mapped, requested)
            self.latest_request[project] = ident
            self.latest[project] = revision
            return
        require(ident in self.requests, 'compile_result has no matching request id')
        require(ident not in self.completed, 'duplicate terminal response id')
        expected_project, expected_revision, documents, requested = self.requests[ident]
        accepted = layout_capabilities(payload)
        require(accepted <= requested, 'result accepted an unrequested layout capability')
        require(accepted <= {'rules-v1', 'font-hints-v1'}, 'result accepted an unknown layout capability')
        require((project, revision) == (expected_project, expected_revision),
                'compile_result project_id/revision does not match its request id')
        require(payload.get('status') in ('ok', 'recovered', 'failed'), 'invalid compile_result status')
        pages, diagnostics = payload.get('pages'), payload.get('diagnostics')
        array(pages, 'pages')
        array(diagnostics, 'diagnostics')
        seen_pages = set()
        for page in pages:
            object_value(page, 'page')
            n = page.get('number')
            require(integer(n) and n >= 1 and n not in seen_pages, 'page.number must be unique and positive')
            seen_pages.add(n)
            for field in ('width_pt', 'height_pt'):
                require(number(page.get(field)) and page[field] > 0, field + ' must be finite and positive')
            array(page.get('items'), 'page.items')
            for item in page['items']:
                object_value(item, 'item')
                kind = item.get('kind')
                require(kind in ('text', 'rule'), 'unsupported layout primitive kind')
                if kind == 'rule':
                    require('rules-v1' in accepted, 'rule requires accepted rules-v1')
                    for field in ('x_pt', 'y_pt', 'width_pt', 'height_pt'):
                        require(number(item.get(field)) and abs(item[field]) <= 1000000,
                                'rule ' + field + ' must be finite and bounded')
                    require(item['width_pt'] > 0 and item['height_pt'] > 0,
                            'rule dimensions must be positive')
                else:
                    string(item.get('text'), 'item.text')
                    for field in ('x_pt', 'baseline_y_pt', 'font_size_pt'):
                        require(number(item.get(field)), field + ' must be a finite number')
                    require(item['font_size_pt'] > 0, 'font_size_pt must be positive')
                    if 'font' in item:
                        require('font-hints-v1' in accepted, 'font requires accepted font-hints-v1')
                        font = item['font']
                        object_value(font, 'item.font')
                        family = font.get('family')
                        string(family, 'font.family', True)
                        require(len(family.encode('utf-8')) <= 128 and
                                all(unicodedata.category(c) != 'Cc' for c in family),
                                'font.family must be bounded and contain no control characters')
                        require(font.get('weight') in ('normal', 'bold'), 'invalid font.weight')
                        require(font.get('style') in ('normal', 'italic'), 'invalid font.style')
                self.source(item.get('source'), documents)
        for diagnostic in diagnostics:
            object_value(diagnostic, 'diagnostic')
            require(diagnostic.get('severity') in ('error', 'warning'), 'invalid diagnostic.severity')
            string(diagnostic.get('message'), 'diagnostic.message')
            require('source' in diagnostic and 'recovery' in diagnostic, 'diagnostic needs source and recovery (null allowed)')
            if diagnostic['source'] is not None:
                self.source(diagnostic['source'], documents)
            if diagnostic['recovery'] is not None:
                string(diagnostic['recovery'], 'diagnostic.recovery')
        if payload.get('pdf_path') is not None:
            string(payload['pdf_path'], 'pdf_path', True)
        self.completed.add(ident)
        self.results.append({'id': ident, 'project_id': project, 'revision': revision,
                             'status': payload['status'],
                             'accepted_layout_capabilities': sorted(accepted),
                             'missing_layout_capabilities': sorted(requested - accepted),
                             'preview': 'stale_ignore' if ident != self.latest_request[project] else 'current'})


class TransferValidator:
    """Stateful bridge-side transcript check. Requires observed request/reply order.

    A successful reply commits an observed state transition. Requests that the
    bridge rejects may contain invalid semantic values; an error reply is valid.
    This does not prove native ledger durability, pixel decoding, or paid calls.
    """
    replies = {'document_open': 'document_opened', 'document_edit': 'document_updated',
               'destination_pin': 'destination_pinned', 'capture_submit': 'capture_received',
               'capture_convert': 'capture_proposal', 'capture_prepare_insert': 'capture_edit',
               'capture_applied': 'capture_application_received', 'capture_reject': 'capture_rejected',
               'capture_status': 'capture_status'}

    def __init__(self):
        self.requests, self.completed, self.results = {}, set(), []
        self.captures, self.documents, self.anchors, self.responses = {}, {}, {}, {}

    @staticmethod
    def identifier(value):
        string(value, 'identifier', True)
        require(len(value) <= 128 and all(c.isascii() and (c.isalnum() or c in '-_') for c in value), 'invalid bridge identifier')

    @staticmethod
    def revision(value):
        require(integer(value) and 0 <= value < 2 ** 64, 'revision must be u64')

    @staticmethod
    def bridge_path(value):
        path(value)
        require('\\' not in value and ':' not in value and all(x not in ('', '.', '..') for x in value.split('/')), 'bridge path must be normalized')

    @staticmethod
    def text(value, label, maximum, nonempty=False):
        string(value, label, nonempty)
        require(len(value.encode()) <= maximum, label + ' exceeds byte limit')

    @staticmethod
    def span(text, start, end):
        Validator().source({'path': 'source.tex', 'start_byte': start, 'end_byte': end}, {'source.tex': text})

    def document(self, request):
        key = (request.get('project_id'), request.get('path'))
        self.identifier(key[0])
        self.bridge_path(key[1])
        require(key in self.documents, 'successful reply references unopened document')
        return key, self.documents[key]

    def target(self, record):
        anchor = self.anchors.get(record['submission']['destination_id'])
        require(anchor is not None and anchor['valid'], 'successful reply uses missing/invalid destination')
        require(anchor['binding'] == record['binding'], 'capture destination binding changed')
        key, doc = self.document(anchor)
        return anchor, key, doc

    def edited(self, key, revision, start, end, replacement):
        doc = self.documents[key]
        self.span(doc['text'], start, end)
        text = doc['text'].encode()
        result = (text[:start] + replacement.encode() + text[end:]).decode()
        self.text(result, 'document', 8 * 1024 * 1024)
        delta = len(replacement.encode()) - (end - start)
        for anchor in self.anchors.values():
            if (anchor['project_id'], anchor['path']) != key or not anchor['valid']:
                continue
            a, b = anchor['start_byte'], anchor['end_byte']
            if (start == end and a <= start <= b) or (start < b and end > a) or (a == b and start <= a < end):
                anchor['valid'] = False
            elif end <= a:
                anchor['start_byte'] += delta
                anchor['end_byte'] += delta
            anchor['current_revision'] = revision
        self.documents[key] = {'revision': revision, 'text': result}

    def success(self, kind, request, payload):
        def scalar_types(value):
            if isinstance(value, dict):
                for field, child in value.items():
                    if field in ('revision', 'base_revision', 'expected_revision', 'new_revision', 'context_revision', 'pinned_revision', 'current_revision'):
                        self.revision(child)
                    elif field in ('start_byte', 'end_byte'):
                        require(integer(child) and child >= 0, field + ' must be a nonnegative integer')
                    elif field in ('durable', 'applied', 'has_proposal', 'valid', 'approved', 'rejected') and not isinstance(child, dict) and child is not None:
                        require(type(child) is bool, field + ' must be boolean')
                    scalar_types(child)
            elif isinstance(value, list):
                for child in value:
                    scalar_types(child)
        scalar_types(request)
        scalar_types(payload)
        if kind == 'document_open':
            self.identifier(request.get('project_id'))
            self.bridge_path(request.get('path'))
            self.revision(request.get('revision'))
            self.text(request.get('text'), 'document.text', 8 * 1024 * 1024)
            require(payload == {}, 'document_opened payload must be empty')
            key = (request['project_id'], request['path'])
            doc = {'revision': request['revision'], 'text': request['text']}
            old = self.documents.get(key)
            require(old is None or old == doc or doc['revision'] > old['revision'], 'stale snapshot accepted')
            if old is not None and old != doc:
                for anchor in self.anchors.values():
                    if (anchor['project_id'], anchor['path']) == key:
                        anchor['valid'] = False
            self.documents[key] = doc
        elif kind == 'document_edit':
            key, doc = self.document(request)
            self.revision(request.get('revision'))
            require(request.get('base_revision') == doc['revision'] and request['revision'] > doc['revision'], 'stale edit accepted')
            self.text(request.get('replacement'), 'replacement', 8 * 1024 * 1024)
            require(payload == {'revision': request['revision']}, 'document_updated revision mismatch')
            self.edited(key, request['revision'], request.get('start_byte'), request.get('end_byte'), request['replacement'])
        elif kind == 'destination_pin':
            _, doc = self.document(request)
            self.identifier(request.get('destination_id'))
            require(request.get('revision') == doc['revision'], 'stale pin accepted')
            self.span(doc['text'], request.get('start_byte'), request.get('end_byte'))
            expected = {k: request[k] for k in ('destination_id', 'project_id', 'path', 'start_byte', 'end_byte')}
            expected.update(pinned_revision=doc['revision'], current_revision=doc['revision'], valid=True,
                            binding={**{k: request[k] for k in ('project_id', 'path', 'revision', 'start_byte', 'end_byte')},
                                     'source_sha256': hashlib.sha256(doc['text'].encode()).hexdigest()})
            require(payload == expected, 'destination_pinned binding/range/revision mismatch')
            old = self.anchors.get(request['destination_id'])
            require(old is None or old == expected, 'conflicting destination ID reuse accepted')
            self.anchors[request['destination_id']] = copy.deepcopy(expected)
        elif kind == 'capture_submit':
            self.identifier(request.get('capture_id'))
            self.identifier(request.get('destination_id'))
            self.revision(request.get('base_revision'))
            self.text(request.get('instructions'), 'instructions', 4096)
            meta = image_metadata(request.get('image'))
            require(meta['width'] <= 8192 and meta['height'] <= 8192, 'bridge image dimensions exceed8192')
            cap = request['capture_id']
            old = self.captures.get(cap)
            if old:
                require(old['submission'] == request, 'conflicting capture ID reuse accepted')
            else:
                anchor = self.anchors.get(request['destination_id'])
                require(anchor is not None and anchor['valid'], 'capture accepted without valid destination')
                require(request['base_revision'] == anchor['pinned_revision'], 'capture pinned revision mismatch')
                old = {'submission': copy.deepcopy(request), 'binding': copy.deepcopy(anchor['binding']),
                       'proposal': None, 'prepared': None, 'applied': None, 'rejected': False}
            require(payload == {'capture_id': cap, 'durable': True, 'has_proposal': old['proposal'] is not None,
                                'applied': old['applied'] is not None}, 'capture_received durable/status mismatch')
            self.captures[cap] = old
        else:
            cap = request.get('capture_id')
            self.identifier(cap)
            require(cap in self.captures, 'successful reply references unknown capture; include its receipt history')
            record = self.captures[cap]
            if kind == 'capture_convert':
                require(not record['rejected'], 'conversion succeeded after rejection')
                supported = request.get('supported_features', [])
                array(supported, 'supported_features')
                require(len(supported) <= 64, 'supported_features exceeds64 entries')
                for item in supported:
                    self.text(item, 'supported feature', 128)
                require(payload.get('capture_id') == cap, 'proposal capture ID mismatch')
                self.text(payload.get('latex'), 'proposal.latex', 65536, True)
                for field in ('ambiguities', 'required_dependencies'):
                    array(payload.get(field), field)
                    require(len(payload[field]) <= 32, 'proposal list exceeds32 entries')
                    for item in payload[field]:
                        self.text(item, field + ' item', 2048)
                if record['proposal'] is None:
                    _, _, doc = self.target(record)
                    require(payload.get('context_revision') == doc['revision'], 'proposal context revision mismatch')
                else:
                    require(payload == record['proposal'], 'cached proposal changed')
                record['proposal'] = copy.deepcopy(payload)
            elif kind == 'capture_prepare_insert':
                require(request.get('approved') is True, 'insertion issued without explicit approval')
                require(not record['rejected'] and record['applied'] is None and record['proposal'] is not None, 'invalid capture review state')
                if record['prepared'] is not None:
                    edit = record['prepared']
                    _, doc = self.document(edit)
                    require(doc['revision'] == request.get('expected_revision') == edit['expected_revision'] and
                            hashlib.sha256(doc['text'].encode()).hexdigest() == edit['document_before_sha256'], 'stale prepared edit replay')
                    require(payload == edit, 'duplicate prepared edit changed')
                else:
                    anchor, _, doc = self.target(record)
                    require(request.get('expected_revision') == doc['revision'], 'stale approval revision accepted')
                    start, end = anchor['start_byte'], anchor['end_byte']
                    require(len(doc['text'].encode()) - (end - start) + len(record['proposal']['latex'].encode()) <= 8 * 1024 * 1024, 'prepared edit exceeds document size bound')
                    expected = {'capture_id': cap, 'project_id': anchor['project_id'], 'path': anchor['path'],
                                'expected_revision': doc['revision'], 'start_byte': start, 'end_byte': end,
                                'removed_text': doc['text'].encode()[start:end].decode(),
                                'replacement': record['proposal']['latex'],
                                'document_before_sha256': hashlib.sha256(doc['text'].encode()).hexdigest()}
                    string(payload.get('edit_id'), 'edit_id', True)
                    require(payload == {**expected, 'edit_id': payload['edit_id']}, 'prepared edit target/hash/replacement mismatch')
                    record['prepared'] = copy.deepcopy(payload)
            elif kind == 'capture_applied':
                edit = record['prepared']
                require(edit is not None, 'application receipt without prepared edit')
                self.revision(request.get('new_revision'))
                expected = {k: request[k] for k in ('capture_id', 'edit_id', 'new_revision')}
                require(payload == expected, 'application response mismatch')
                receipt = {'edit_id': request['edit_id'], 'new_revision': request['new_revision']}
                require(receipt['edit_id'] == edit['edit_id'] and receipt['new_revision'] > edit['expected_revision'], 'application edit ID/revision mismatch')
                if record['applied'] is not None:
                    require(record['applied'] == receipt, 'conflicting application receipt accepted')
                else:
                    key, doc = self.document(edit)
                    require(doc['revision'] == edit['expected_revision'] and hashlib.sha256(doc['text'].encode()).hexdigest() == edit['document_before_sha256'], 'stale source receipt accepted')
                    self.edited(key, receipt['new_revision'], edit['start_byte'], edit['end_byte'], edit['replacement'])
                    record['applied'] = receipt
            elif kind == 'capture_reject':
                require(record['prepared'] is None and record['applied'] is None, 'rejection accepted after edit was issued')
                require(payload == {'capture_id': cap}, 'rejection capture mismatch')
                record['rejected'] = True
            elif kind == 'capture_status':
                proposal = record['proposal']
                expected = {'capture_id': cap, 'proposal': None if proposal is None else {k: proposal[k] for k in ('latex', 'ambiguities', 'required_dependencies')},
                            'prepared': record['prepared'], 'applied': record['applied'], 'rejected': record['rejected']}
                require(payload == expected, 'status disagrees with observed capture journal history')

    def consume(self, value):
        object_value(value, 'envelope')
        require(type(value.get('protocol_version')) is int and value['protocol_version'] == 1, 'unsupported bridge protocol version')
        ident, kind, payload = value.get('id'), value.get('type'), value.get('payload')
        string(ident, 'id', True)
        require(len(ident.encode()) <= 128, 'bridge envelope ID exceeds128 bytes')
        object_value(payload, 'payload')
        if ident not in self.requests:
            require(kind in self.replies, 'reply has no matching request ID')
            self.requests[ident] = copy.deepcopy(value)
            return
        request = self.requests[ident]
        require(kind == 'error' or kind == self.replies[request['type']], 'reply type does not match request')
        if ident in self.completed:
            require(value == self.responses[ident], 'conflicting duplicate terminal response')
            return
        if kind == 'error':
            string(payload.get('code'), 'error.code', True)
            string(payload.get('message'), 'error.message')
        else:
            # Invalid replies must not partially mutate subsequent validation state.
            trial = copy.deepcopy(self)
            trial.success(request['type'], request['payload'], payload)
            self.documents, self.anchors, self.captures = trial.documents, trial.anchors, trial.captures
        self.completed.add(ident)
        self.responses[ident] = copy.deepcopy(value)
        self.results.append({'id': ident, 'type': kind, 'capture_id': payload.get('capture_id')})


def validate_stream(stream, validator, name='<stdin>', max_line_bytes=8 * 1024 * 1024):
    errors = []
    line_number = 0
    while True:
        # Bounded readline avoids allocating an arbitrarily large transport line.
        raw = stream.readline(max_line_bytes + 1)
        if not raw:
            break
        line_number += 1
        try:
            # The compiler limit counts payload bytes, excluding the LF framing
            # byte. An unterminated final record is accepted at EOF, as in Rust.
            content = raw[:-1] if raw.endswith(b'\n') else raw
            measured = len(raw) if isinstance(validator, TransferValidator) else len(content)
            require(measured <= max_line_bytes, 'JSON line exceeds --max-line-bytes')
            value = json.loads(raw.decode('utf-8'), object_pairs_hook=strict_object,
                               parse_constant=lambda token: (_ for _ in ()).throw(Invalid('non-finite JSON number: ' + token)))
            validator.consume(value)
        except (Invalid, ValueError, UnicodeError, RecursionError, TypeError, KeyError, IndexError, OverflowError) as exc:
            errors.append({'file': name, 'line': line_number, 'message': str(exc)})
            if len(raw) > max_line_bytes:
                # Consume the oversized record in bounded chunks, then recover.
                while raw and not raw.endswith(b'\n'):
                    raw = stream.readline(max_line_bytes + 1)
    return errors



def probe(executable, request_bytes, timeout=10, max_output_bytes=32 * 1024 * 1024,
          max_line_bytes=8 * 1024 * 1024):
    """Run one explicitly chosen local executable; bounded pipes, no shell.

    This is a subprocess lifetime timeout, not an individual-response timeout.
    The child receives EOF after all input. All requests are considered sent before
    validating replies, so older replies are classified conservatively as stale.
    Caller bounds request_bytes; stdout is capped and stderr stores only 64 KiB.
    """
    validator = Validator()
    errors = validate_stream(io.BytesIO(request_bytes), validator, 'requests', max_line_bytes)
    if validator.completed or validator.captures:
        errors.append({'message': 'probe input must contain compile requests only'})
    if not validator.requests:
        errors.append({'message': 'probe needs at least one compile request'})
    metadata = {'executable': executable, 'returncode': None, 'stderr': '', 'stderr_truncated': False}
    if errors:
        return validator, errors, metadata
    # Allocate selector before spawning so allocation failure cannot leak a child.
    selector = selectors.DefaultSelector()
    try:
        child = subprocess.Popen([executable], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, start_new_session=True)
    except OSError as exc:
        selector.close()
        errors.append({'message': 'compiler launch failed: ' + str(exc)})
        return validator, errors, metadata
    stdout, stderr = bytearray(), bytearray()
    offset, deadline = 0, time.monotonic() + timeout
    stopped = False

    def stop(message):
        nonlocal stopped
        errors.append({'message': message})
        stopped = True
        # Isolate the probe's process group; also close inherited pipes in children.
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass

    with selector:
        try:
            for stream, event, kind in ((child.stdin, selectors.EVENT_WRITE, 'stdin'),
                                        (child.stdout, selectors.EVENT_READ, 'stdout'),
                                        (child.stderr, selectors.EVENT_READ, 'stderr')):
                os.set_blocking(stream.fileno(), False)
                selector.register(stream, event, kind)
            while selector.get_map() and not stopped:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    stop('compiler probe timed out')
                    break
                for key, _ in selector.select(min(remaining, 0.1)):
                    stream, kind = key.fileobj, key.data
                    if kind == 'stdin':
                        try:
                            offset += os.write(stream.fileno(), request_bytes[offset:offset + 65536])
                        except BrokenPipeError:
                            if offset < len(request_bytes):
                                errors.append({'message': 'compiler closed stdin before all requests were sent'})
                            offset = len(request_bytes)
                        except BlockingIOError:
                            continue
                        if offset == len(request_bytes):
                            selector.unregister(stream)
                            stream.close()
                    else:
                        try:
                            data = os.read(stream.fileno(), 65536)
                        except BlockingIOError:
                            continue
                        if not data:
                            selector.unregister(stream)
                            stream.close()
                        elif kind == 'stdout':
                            if len(stdout) + len(data) > max_output_bytes:
                                stop('compiler stdout exceeds --max-total-bytes')
                                break
                            stdout.extend(data)
                        else:
                            room = 65536 - len(stderr)
                            stderr.extend(data[:room])
                            metadata['stderr_truncated'] |= len(data) > room
            if not stopped:
                try:
                    child.wait(timeout=max(0.001, deadline - time.monotonic()))
                except subprocess.TimeoutExpired:
                    stop('compiler probe timed out after closing output streams')
        finally:
            # Do not poll/reap before signalling: an unreaped child PID cannot
            # be reused for an unrelated process group during cleanup.
            if child.returncode is None:
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            child.wait()
            for stream in (child.stdin, child.stdout, child.stderr):
                if not stream.closed:
                    stream.close()
    metadata.update(returncode=child.returncode, stderr=stderr.decode('utf-8', errors='replace'))
    if child.returncode != 0 and not stopped:
        errors.append({'message': 'compiler exited with nonzero status', 'returncode': child.returncode})
    errors.extend(validate_stream(io.BytesIO(stdout), validator, 'compiler.stdout', max_line_bytes))
    return validator, errors, metadata


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('transcript', help='JSONLines file, or - for stdin')
    parser.add_argument('--protocol', choices=('runtime-v1', 'transfer-v1'), default='runtime-v1')
    parser.add_argument('--compiler', help='launch this explicit executable; transcript is its request input, no shell')
    parser.add_argument('--timeout', type=float, default=10, help='probe lifetime limit in seconds')
    parser.add_argument('--max-total-bytes', type=int, default=32 * 1024 * 1024, help='probe input/stdout byte cap')
    parser.add_argument('--requests', help='optional request-only JSONLines prefix')
    parser.add_argument('--allow-pending', action='store_true', help='allow requests without terminal responses in a partial capture')
    parser.add_argument('--max-line-bytes', type=int, default=None)
    args = parser.parse_args(argv)
    if args.max_line_bytes is None:
        args.max_line_bytes = (12 if args.protocol == 'transfer-v1' else 8) * 1024 * 1024
    if args.max_line_bytes < 1:
        parser.error('--max-line-bytes must be positive')
    if not math.isfinite(args.timeout) or args.timeout <= 0 or args.max_total_bytes < 1:
        parser.error('probe timeout and byte cap must be finite and positive')
    if args.compiler and args.requests:
        parser.error('--compiler takes requests from transcript; do not also use --requests')
    if args.compiler and args.protocol != 'runtime-v1':
        parser.error('--compiler is compile-scoped; capture bridge transcripts use file validation')
    validator, errors, metadata = (TransferValidator() if args.protocol == 'transfer-v1' else Validator()), [], None
    if args.compiler:
        try:
            if args.transcript == '-':
                data = sys.stdin.buffer.read(args.max_total_bytes + 1)
            else:
                with open(args.transcript, 'rb') as stream:
                    data = stream.read(args.max_total_bytes + 1)
            require(len(data) <= args.max_total_bytes, 'probe input exceeds --max-total-bytes')
            validator, errors, metadata = probe(args.compiler, data, args.timeout, args.max_total_bytes, args.max_line_bytes)
        except (OSError, Invalid) as exc:
            errors.append({'message': str(exc)})
    for filename in ([] if args.compiler else ([args.requests] if args.requests else []) + [args.transcript]):
        try:
            if filename == '-':
                errors.extend(validate_stream(sys.stdin.buffer, validator, filename, args.max_line_bytes))
            else:
                with open(filename, 'rb') as stream:
                    errors.extend(validate_stream(stream, validator, filename, args.max_line_bytes))
        except OSError as exc:
            errors.append({'file': filename, 'message': str(exc)})
    pending = sorted(set(validator.requests) - validator.completed)
    pending_captures = sorted(capture_id for capture_id, capture in validator.captures.items()
                              if isinstance(validator, Validator) and 'capture_received' not in capture['responses'])
    if pending_captures and not args.allow_pending:
        errors.append({'message': 'missing durable capture receipts', 'capture_ids': pending_captures})
    if pending and not args.allow_pending:
        errors.append({'message': 'missing terminal responses', 'request_ids': pending})
    print(json.dumps({'valid': not errors, 'errors': errors, 'responses': validator.results,
                      'pending_request_ids': pending, 'pending_capture_ids': pending_captures, 'probe': metadata}, indent=2))
    return 1 if errors else 0


if __name__ == '__main__':
    sys.exit(main())
