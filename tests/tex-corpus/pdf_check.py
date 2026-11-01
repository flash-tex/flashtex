#!/usr/bin/env python3
"""Check the current FlashTeX uncompressed PDF writer profile, not arbitrary PDF conformance."""
import argparse
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import re
import subprocess
import sys

SPEC = importlib.util.spec_from_file_location('corpus_compare', Path(__file__).with_name('compare.py'))
compare = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(compare)
NUMBER = rb'-?\d+(?:\.\d+)?'
TEXT = re.compile(rb'BT\s+/F1 (' + NUMBER + rb') Tf\s+(' + NUMBER + rb') (' + NUMBER + rb') Td\s+\(((?:\\.|[^\\)])*)\) Tj\s+ET\s*', re.DOTALL)


class Unsupported(ValueError):
    pass


def inspect(data, envelope):
    if not data.startswith(b'%PDF-1.4\n'):
        raise ValueError('missing supported PDF-1.4 header')
    tail = re.search(rb'startxref\s+(\d+)\s+%%EOF\s*$', data)
    if not tail:
        raise ValueError('missing terminal startxref/EOF')
    offset = int(tail[1])
    table = re.match(rb'xref\n0 (\d+)\n', data[offset:])
    if not table:
        raise ValueError('startxref does not address the expected classic xref table')
    size = int(table[1]); cursor = offset + table.end(); offsets = {}
    for number in range(size):
        entry = re.match(rb'(\d{10}) (\d{5}) ([nf]) [\r]?\n', data[cursor:])
        if not entry:
            raise ValueError('invalid xref entry')
        cursor += entry.end()
        position, generation, state = int(entry[1]), int(entry[2]), entry[3]
        if number == 0:
            if (position, generation, state) != (0, 65535, b'f'):
                raise ValueError('invalid free object zero')
        else:
            if state != b'n' or generation != 0 or position >= offset or not data[position:].startswith(f'{number} 0 obj\n'.encode()):
                raise ValueError('xref offset does not point to its declared object')
            offsets[number] = position
    trailer = data[cursor:tail.start()]
    root = re.search(rb'/Root (\d+) 0 R', trailer)
    declared_size = re.search(rb'/Size (\d+)', trailer)
    if not trailer.startswith(b'trailer\n') or not root or not declared_size or int(declared_size[1]) != size:
        raise ValueError('invalid trailer root/size')
    if len(set(offsets.values())) != len(offsets):
        raise ValueError('duplicate object offsets')
    ordered = sorted(offsets, key=offsets.get); objects = {}
    for index, number in enumerate(ordered):
        end = offsets[ordered[index + 1]] if index + 1 < len(ordered) else offset
        chunk = data[offsets[number]:end]
        if not chunk.endswith(b'endobj\n'):
            raise ValueError('invalid object boundary')
        objects[number] = chunk.split(b'\n', 1)[1][:-7]
    catalog = objects[int(root[1])]
    if b'/Type /Catalog' not in catalog:
        raise ValueError('root is not catalog')
    tree = objects[int(re.search(rb'/Pages (\d+) 0 R', catalog)[1])]
    kids_match = re.search(rb'/Kids\s*\[([^]]*)\]', tree)
    if not kids_match:
        raise Unsupported('nested/non-flat page trees are outside this checker profile')
    kids = [int(n) for n in re.findall(rb'(\d+) 0 R', kids_match[1])]
    count = int(re.search(rb'/Count (\d+)', tree)[1])
    pages = envelope['payload']['pages']
    if count != len(kids) or count != len(pages) or not count or len(set(kids)) != count:
        raise ValueError('page count/tree differs from compile_result')
    evidence = []
    for number, source in zip(kids, pages):
        page = objects[number]
        if not re.search(rb'/Type /Page\b', page):
            raise ValueError('page tree child is not page')
        box = re.search(rb'/MediaBox\s*\[\s*(' + NUMBER + rb')\s+(' + NUMBER + rb')\s+(' + NUMBER + rb')\s+(' + NUMBER + rb')\s*\]', page)
        if not box:
            raise ValueError('page MediaBox missing')
        values = [float(n) for n in box.groups()]
        width, height = source['width_pt'], source['height_pt']
        if not all(math.isfinite(n) for n in values) or min(width, height) <= 0 or any(abs(a-b) > .0011 for a,b in zip(values,[0,0,width,height])):
            raise ValueError('MediaBox differs from source dimensions')
        stream_obj = objects[int(re.search(rb'/Contents (\d+) 0 R', page)[1])]
        if b'/Filter' in stream_obj:
            raise Unsupported('compressed streams are outside this checker profile')
        stream_start = stream_obj.index(b'stream\n') + len(b'stream\n')
        length = int(re.search(rb'/Length (\d+)', stream_obj[:stream_start])[1])
        content = stream_obj[stream_start:stream_start+length]
        if stream_obj[stream_start+length:] != b'\nendstream\n':
            raise ValueError('content stream length mismatch')
        runs = list(TEXT.finditer(content))
        residual = TEXT.sub(b'', content).strip()
        if residual != b'0 g':
            raise Unsupported('content operators differ from the current text-only writer profile')
        if any(i['kind'] != 'text' for i in source['items']):
            raise Unsupported('non-text items need an expanded artifact checker')
        if len(runs) != len(source['items']):
            raise ValueError('PDF text run count differs from display list')
        placements = []
        for run, item in zip(runs, source['items']):
            font, x, y = [float(n) for n in run.groups()[:3]]
            expected = [item['font_size_pt'], item['x_pt'], height-item['baseline_y_pt']]
            if font <= 0 or not 0 <= x <= width or not 0 <= y <= height:
                raise ValueError('text baseline anchor/font is outside page bounds')
            if any(abs(a-b) > .0011 for a,b in zip([font,x,y],expected)):
                raise ValueError('text placement differs from display list')
            placements.append({'x_pt':x,'pdf_baseline_y_pt':y,'font_size_pt':font,'source':item.get('source')})
        evidence.append({'number':source['number'],'media_box':values,'text_placements':placements})
    return {'status':'pass','page_count':count,'xref_entries':size,'pages':evidence,
            'pdf_sha256':hashlib.sha256(data).hexdigest(),
            'unverified':['visual rendering','font embedding and glyph fidelity','math/diagram correctness','glyph ink/advance bounds','native display/export flow']}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--results',type=Path,required=True)
    parser.add_argument('--compiler-sha',required=True)
    parser.add_argument('--compiler-build-command',required=True)
    parser.add_argument('--renderer',type=Path,required=True)
    parser.add_argument('--renderer-sha',required=True)
    parser.add_argument('--renderer-build-command',required=True)
    parser.add_argument('--output-dir',type=Path,required=True)
    args=parser.parse_args()
    if len(args.renderer_sha)!=40 or any(c not in '0123456789abcdef' for c in args.renderer_sha) or not args.renderer_build_command.strip():
        parser.error('exact renderer SHA and build command required')
    artifact=json.loads(args.results.read_text())
    records=compare.verify(artifact,args.compiler_sha,args.compiler_build_command)
    args.output_dir.mkdir(parents=True,exist_ok=True)
    results=[]
    for ident,record in sorted(records.items()):
        pdf=args.output_dir/(ident+'.pdf')
        if pdf.exists():
            raise ValueError('refusing to replace prior artifact: '+str(pdf))
        result={'case_id':ident,'request':record['request'],'compiler_case_status':record['status'],'pdf_file':pdf.name}
        try:
            process=subprocess.run([str(args.renderer),'--out',str(pdf),'--verify'],input=record['stdout'],text=True,capture_output=True,timeout=10)
            result.update(renderer_returncode=process.returncode,renderer_stdout=process.stdout,renderer_stderr=process.stderr)
            if process.returncode:
                raise ValueError('renderer failed')
            result.update(inspect(pdf.read_bytes(),json.loads(record['stdout'])))
        except Unsupported as error:
            result.update(status='unsupported',error=str(error))
        except (OSError,ValueError,KeyError,TypeError,AttributeError,subprocess.TimeoutExpired) as error:
            result.update(status='fail',error=str(error))
        results.append(result)
    evidence={'schema_version':1,'compiler_sha':args.compiler_sha,'compiler_build_command':args.compiler_build_command,
              'renderer_sha':args.renderer_sha,'renderer_build_command':args.renderer_build_command,
              'compiler_evidence_sha256':hashlib.sha256(args.results.read_bytes()).hexdigest(),
              'limitations':'Structural text-writer profile only; placement checks baseline anchors, not glyph extents. A pass does not establish semantic/font/visual correctness.',
              'results':results}
    (args.output_dir/'results.json').write_text(json.dumps(evidence,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps({s:sum(r['status']==s for r in results) for s in ('pass','fail','unsupported')}))
    return int(any(r['status']=='fail' for r in results))


if __name__=='__main__':
    sys.exit(main())
