#!/usr/bin/env python3
"""Check current PDF text encoding and explicit loss reporting against runtime items."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys

SPEC = importlib.util.spec_from_file_location('pdf_structure', Path(__file__).with_name('pdf_check.py'))
structure = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(structure)


def expected_bytes(text):
    result = bytearray(); lost = []
    for char in text:
        try:
            if ord(char) < 0x20 or 0x7F <= ord(char) < 0xA0:
                raise UnicodeError('control character')
            result.extend(char.encode('cp1252'))
        except UnicodeError:
            result.extend(b'?')
            if char not in lost:
                lost.append(char)
    return bytes(result), lost


def decode_literal(raw):
    result = bytearray(); index = 0
    while index < len(raw):
        char = raw[index]; index += 1
        if char == 92:
            if index >= len(raw):
                raise ValueError('unterminated PDF string escape')
            char = raw[index]; index += 1
            if char not in b'()\\rn':
                raise structure.Unsupported('literal escape outside current writer profile')
            char = {ord('r'): 13, ord('n'): 10}.get(char, char)
        result.append(char)
    return bytes(result)


def inspect(data, envelope, warnings, semantic_status):
    if semantic_status not in ('pass', 'fail', 'unsupported', 'unverified'):
        raise ValueError('explicit compiler semantic status required')
    result = structure.inspect(data, envelope)
    streams = []
    for match in re.finditer(rb'\d+ 0 obj\n<< /Length (\d+) >>\nstream\n', data):
        streams.append(data[match.end():match.end()+int(match[1])])
    pages = envelope['payload']['pages']
    if len(streams) != len(pages):
        raise ValueError('expected one current-profile content stream per runtime page')
    matches = []; faithful = True
    for page, stream in zip(pages, streams):
        runs = list(structure.TEXT.finditer(stream))
        if len(runs) != len(page['items']):
            raise ValueError('dropped or duplicated text item')
        for index, (item, run) in enumerate(zip(page['items'], runs)):
            expected, lost = expected_bytes(item['text'])
            actual = decode_literal(run[4])
            if actual != expected:
                raise ValueError(f"page {page['number']} item {index}: PDF text bytes differ from runtime encoding")
            relevant = [line for line in warnings.splitlines()
                        if line.startswith(f"warning: page {page['number']}: item {index} ")
                        and "not representable in WinAnsiEncoding; written as '?'" in line]
            if lost and not any(all(f'U+{ord(c):04X}' in line for c in lost) for line in relevant):
                raise ValueError(f"page {page['number']} item {index}: substitution lacks matching codepoint warning")
            faithful &= not lost
            matches.append({'page_number':page['number'],'item_index':index,'source':item.get('source'),
                            'runtime_text':item['text'],'encoded_hex':actual.hex(),'substituted_codepoints':[f'U+{ord(c):04X}' for c in lost],
                            'warning_evidence':relevant,'text_preserved':not lost})
    result.update(compiler_semantic_status=semantic_status, text_preserved=faithful, text_items=matches,
                  semantic_gate='unverified' if semantic_status == 'pass' else semantic_status,
                  limitations='Consistency permits explicitly warned substitution. It never turns a failed/unsupported compiler case into semantic success; glyph appearance and native rendering remain unverified.')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pdf-evidence',type=Path,required=True)
    parser.add_argument('--compiler-evidence',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    pdf_evidence=json.loads(args.pdf_evidence.read_text())
    source_bytes=args.compiler_evidence.read_bytes()
    if hashlib.sha256(source_bytes).hexdigest()!=pdf_evidence['compiler_evidence_sha256']:
        raise ValueError('compiler evidence hash mismatch')
    compiler=json.loads(source_bytes)
    records=structure.compare.verify(compiler,pdf_evidence['compiler_sha'],pdf_evidence['compiler_build_command'])
    results=[];seen=set()
    for record in pdf_evidence['results']:
        ident=record['case_id']
        if ident not in records or ident in seen:
            raise ValueError('unknown or duplicate PDF case')
        seen.add(ident)
        if record['request']!=records[ident]['request'] or record['compiler_case_status']!=records[ident]['status']:
            raise ValueError('PDF case linkage/semantic status mismatch')
        path=Path(record['pdf_file'])
        if path.name!=str(path) or path.suffix!='.pdf':
            raise ValueError('PDF evidence path must be a local basename')
        data=(args.pdf_evidence.parent/path).read_bytes()
        if hashlib.sha256(data).hexdigest()!=record['pdf_sha256']:
            raise ValueError('PDF artifact hash mismatch')
        result={'case_id':ident}
        try:
            result.update(inspect(data,json.loads(records[ident]['stdout']),record['renderer_stderr'],records[ident]['status']))
        except (ValueError,KeyError,TypeError,AttributeError) as error:
            result.update(status='fail',error=str(error),compiler_semantic_status=records[ident]['status'])
        results.append(result)
    if seen!=set(records):
        raise ValueError('PDF cases missing')
    output={'schema_version':1,'compiler_sha':pdf_evidence['compiler_sha'],'renderer_sha':pdf_evidence['renderer_sha'],
            'compiler_evidence_sha256':hashlib.sha256(source_bytes).hexdigest(),
            'pdf_evidence_sha256':hashlib.sha256(args.pdf_evidence.read_bytes()).hexdigest(),'results':results}
    args.output.write_text(json.dumps(output,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps({'consistency_pass':sum(r['status']=='pass' for r in results),
                      'consistency_fail':sum(r['status']=='fail' for r in results),
                      'cases_with_substitution':sum(r.get('text_preserved') is False for r in results)}))
    return int(any(r['status']=='fail' for r in results))


if __name__=='__main__':
    sys.exit(main())
