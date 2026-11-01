#!/usr/bin/env python3
"""A/B of engines by keystroke instruction counts: runs keyprof.py for each (doc, phase) and engine, in turn,
and prints p50 of the DONE stages per engine.

usage: abkeys.py OUTDIR ENGINES DOC=PHASE[;PHASE...] ... [--keys N]
ENGINES: comma-separated. PHASE as keyprof.py's (NAME=args,with,commas)."""
import json, os, subprocess, sys

here = os.path.dirname(os.path.abspath(__file__))
args = sys.argv[1:]
keys = '10'
if '--keys' in args:
    i = args.index('--keys')
    keys = args[i + 1]
    del args[i:i + 2]
out, engines, specs = args[0], args[1].split(','), args[2:]
KEYS = ['instr_k', 'edited_instr_k', 'restore_instr_k', 'test_instr_k', 'first_page_instr_k', 'first_page_dl',
        'edited_wall', 'restore', 'test']
rows = []
for spec in specs:
    doc, phases = spec.split('=', 1)
    phases = phases.split(';')
    for eng in engines:
        p = subprocess.run([sys.executable, f'{here}/keyprof.py', eng, doc, out] + phases + ['--keys', keys],
                           capture_output=True, text=True)
        for l in p.stdout.splitlines():
            if ' {' not in l:
                continue
            n, j = l.split(' ', 1)
            j = json.loads(j)
            r = dict(engine=eng, doc=doc, phase=n, conv=j['converged'], n=j['n'],
                     pages=j['typeset_pages'][len(j['typeset_pages']) // 2] if j['typeset_pages'] else None,
                     **{k: j['stages'].get(k, {}).get('p50') for k in KEYS})
            rows.append(r)
            print(json.dumps(r), flush=True)
        if p.returncode:
            print(f'keyprof {eng} {doc}: exit {p.returncode}: {p.stderr[-500:]}', flush=True)
with open(f'{out}/ab.jsonl', 'a') as f:
    for r in rows:
        f.write(json.dumps(r) + '\n')
