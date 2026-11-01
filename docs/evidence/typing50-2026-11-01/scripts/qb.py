#!/usr/bin/env python3
"""qb.py JSONL...: per compile that waited, what it waited for, in M instructions and ms."""
import json, sys
for f in sys.argv[1:]:
    print(f.split('/')[-1])
    for l in open(f):
        r = json.loads(l)
        if 'done' not in r:
            continue
        s = r['done'].get('stages') or {}
        qi = s.get('queue_by_instr_k') or {}
        qm = s.get('queue_by') or {}
        big = {k: (round(qi[k] / 1e3, 1), round(qm.get(k, 0), 1)) for k in qi if qi[k] > 2000}
        if big:
            print('  id', r['done']['id'], big)
