#!/usr/bin/env python3
"""Validate experimental rendering-v2; does not activate a production protocol.

Dependency: jsonschema==4.23.0. Schema validation alone cannot establish resource
identity, UTF-8 provenance, capability compatibility or safe coordinate sums.
Install in a venv with: python -m pip install jsonschema==4.23.0.

Optional documents/font_bytes maps contain caller-provided trusted snapshots and
bytes; this validator never resolves document-controlled paths or performs I/O
beyond its explicit JSON input/schema. Without font bytes, glyph identity is
structurally checked but font authenticity remains unverified.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import sys

from jsonschema import Draft202012Validator

SCHEMA = Path(__file__).resolve().parents[1] / 'protocol/rendering-v2.schema.json'
MAX_TICK = 2**53 - 1


class InvalidRendering(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise InvalidRendering(message)


def finite(value, depth=0):
    require(depth <= 64, 'JSON nesting exceeds 64 levels')
    if isinstance(value, str):
        try:
            value.encode('utf-8')
        except UnicodeError as error:
            raise InvalidRendering('Invalid Unicode scalar in JSON') from error
    elif isinstance(value, float):
        require(math.isfinite(value), 'Non-finite JSON number')
    elif isinstance(value, dict):
        for item in value.values():
            finite(item, depth + 1)
    elif isinstance(value, list):
        for item in value:
            finite(item, depth + 1)


def unique(values, label):
    require(len(values) == len(set(values)), 'Duplicate ' + label)


def bounds(text):
    offsets = {0}
    offset = 0
    for character in text:
        offset += len(character.encode('utf-8'))
        offsets.add(offset)
    return offsets


def checked_sum(*values):
    total = sum(values)
    require(-MAX_TICK <= total <= MAX_TICK, 'Coordinate arithmetic overflow')
    return total


def rect(item):
    checked_sum(item['x'], item['width'])
    checked_sum(item['top'], item['height'])


def validate(message, *, offer=None, documents=None, font_bytes=None):
    """Raises InvalidRendering; returns explicit verification scope on success.

    `offer` is mandatory for selection/display messages. Source snapshots use a
    path -> {revision, text} map. Each font must have verified caller-supplied bytes
    before renderer validation; this validator never grants paintability. It verifies
    identity/metadata, not every outline or consumer rendering capability.
    """
    finite(message)
    errors = list(Draft202012Validator(json.loads(SCHEMA.read_text())).iter_errors(message))
    require(not errors, 'Schema rejection: ' + (errors[0].message if errors else ''))
    kind, payload = message['type'], message['payload']
    if kind == 'render_capabilities':
        unique(payload['render_formats'], 'render format')
        unique(payload['features'], 'feature')
        return {'status': 'valid_offer', 'paintable': False}
    if kind == 'render_format_rejected':
        return {'status': 'rejected', 'paintable': False}
    require(offer is not None, 'Explicit renderer capabilities required')
    validate(offer)
    require(offer['type'] == 'render_capabilities', 'Expected capability offer')
    require('display-list-v2' in offer['payload']['render_formats'], 'Renderer did not offer v2')
    requested = payload['required_features']
    unique(requested, 'required feature')
    require(set(requested) <= set(offer['payload']['features']), 'Unsupported required feature')
    if kind == 'render_format_selected':
        require(message['id'] == offer['id'], 'Handshake ID mismatch')
        return {'status': 'selected', 'paintable': False}
    fonts = {f['font_id']: f for f in payload['fonts']}
    docs = {d['path']: d for d in payload['documents']}
    require(len(fonts) == len(payload['fonts']), 'Duplicate font identity')
    require(len(docs) == len(payload['documents']), 'Duplicate document path')
    require([p['number'] for p in payload['pages']] == list(range(1, len(payload['pages']) + 1)), 'Pages must have contiguous ordered numbers')
    verified_documents = documents is not None
    verified_fonts = font_bytes is not None or not fonts
    source_bounds = {}
    for path, document in docs.items():
        if documents is not None:
            snapshot = documents.get(path)
            require(snapshot is not None, 'Missing source snapshot: ' + path)
            require(snapshot['revision'] == document['revision'], 'Source revision mismatch')
            data = snapshot['text'].encode('utf-8')
            require(len(data) == document['byte_length'] and hashlib.sha256(data).hexdigest() == document['sha256'], 'Source digest mismatch')
            source_bounds[path] = bounds(snapshot['text'])
    for identity, font in fonts.items():
        if font_bytes is not None:
            data = font_bytes.get(identity)
            require(isinstance(data, bytes), 'Missing font bytes')
            require(len(data) == font['byte_length'] and hashlib.sha256(data).hexdigest() == font['sha256'], 'Font digest mismatch')
            require(len(data) >= 12 and data[:4] == b'\x00\x01\x00\x00', 'Expected static TrueType sfnt')
            count = struct.unpack_from('>H', data, 4)[0]
            require(12 + count * 16 <= len(data), 'Truncated sfnt directory')
            tables = {}
            for i in range(count):
                tag, _, offset, length = struct.unpack_from('>4sIII', data, 12 + i*16)
                require(offset + length <= len(data) and tag not in tables, 'Invalid sfnt table')
                tables[tag] = data[offset:offset+length]
            require(all(t in tables for t in (b'head', b'maxp', b'glyf', b'loca')), 'Missing TrueType outline tables')
            require(b'fvar' not in tables and b'CFF ' not in tables and b'CFF2' not in tables, 'Unsupported font format')
            require(len(tables[b'head']) >= 54 and len(tables[b'maxp']) >= 6, 'Truncated font metrics')
            require(struct.unpack_from('>H', tables[b'head'], 18)[0] == font['units_per_em'], 'Font units mismatch')
            require(struct.unpack_from('>H', tables[b'maxp'], 4)[0] == font['glyph_count'], 'Font glyph count mismatch')
    def provenance(item):
        for source in item.get('sources', []):
            require(source['path'] in docs, 'Unknown source document')
            start, end = source['start_byte'], source['end_byte']
            require(start <= end <= docs[source['path']]['byte_length'], 'Source range outside snapshot')
            if source_bounds:
                require(start in source_bounds[source['path']] and end in source_bounds[source['path']], 'Source range splits UTF-8')
    used = {'rgba-srgb', 'cluster-actualtext'}
    for page in payload['pages']:
        for item in page['items']:
            used.add(item['kind'])
            if item['kind'] == 'rule':
                rect(item)
                provenance(item)
                checked_sum(page['height'], -item['top'], -item['height'])
                continue
            used.add('static-truetype')
            require(item['font_id'] in fonts, 'Unknown font resource')
            boundary = bounds(item['text'])
            cursor = 0
            for cluster in item['clusters']:
                start, end = cluster['text_start_byte'], cluster['text_end_byte']
                require(start == cursor and start < end and start in boundary and end in boundary, 'Clusters must partition logical UTF-8 text')
                cursor = end
                provenance(cluster)
                # The laid-out TeX box. `ink_rect`, when the negotiated
                # `display-list-v2-ink-rect` proposal put one there, is the
                # painted outline's extent and is deliberately allowed to
                # fall outside it.
                for hit in cluster['hit_rects']:
                    rect(hit)
                if 'ink_rect' in cluster:
                    rect(cluster['ink_rect'])
                for caret in cluster['carets']:
                    require(start <= caret['text_byte'] <= end and caret['text_byte'] in boundary, 'Invalid UTF-8 caret')
                    checked_sum(caret['top'], caret['height'])
            require(cursor == len(item['text'].encode('utf-8')), 'Clusters omit logical text')
            seen = set()
            for glyph in item['glyphs']:
                require(glyph['gid'] < fonts[item['font_id']]['glyph_count'], 'Glyph outside declared font')
                require(glyph['cluster'] < len(item['clusters']), 'Glyph references unknown cluster')
                seen.add(glyph['cluster'])
                checked_sum(glyph['origin_x'], glyph['advance_x'])
                checked_sum(glyph['baseline_y'], glyph['advance_y'])
                checked_sum(page['height'], -glyph['baseline_y'])
            require(seen == set(range(len(item['clusters']))), 'Cluster has no glyph mapping')
    for diagnostic in payload['diagnostics']:
        provenance(diagnostic)
    require(used <= set(requested), 'Display list omitted required capability declarations')
    return {'status': 'valid_display_list', 'source_snapshots_verified': verified_documents,
            'font_bytes_verified': verified_fonts, 'paintable': False,
            'resources_ready_for_renderer_validation': verified_documents and verified_fonts,
            'scope': 'Experimental structure/provenance checks; no proof of outline validity, shaping correctness or renderer parity.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('message', type=Path)
    parser.add_argument('--offer', type=Path)
    args = parser.parse_args()
    try:
        require(args.message.stat().st_size <= 32*1024*1024, 'Message exceeds 32 MiB')
        if args.offer:
            require(args.offer.stat().st_size <= 65536, 'Offer exceeds 64 KiB')
        result = validate(json.loads(args.message.read_text()), offer=json.loads(args.offer.read_text()) if args.offer else None)
        print(json.dumps(result))
        return 0
    except (ValueError, OSError, KeyError, UnicodeError) as error:
        print(json.dumps({'status': 'invalid', 'message': str(error)}))
        return 1


if __name__ == '__main__':
    sys.exit(main())
