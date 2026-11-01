#!/usr/bin/env python3
"""Preamble rows from rows.sh: per doc and engine, page-1 ms (client) p50/max over the samples
(warm-up left out), host CPU to page 1 p50/max, and CPU before the cold run (first_page_cpu -
edited_cpu: the reset, i.e. dropping the previous run) p50/max."""
import json, os, sys
O = sys.argv[1] if len(sys.argv) > 1 else '/tmp/ib-p4pc/rows'
DOCS = ['plain-10', 'full-10', 'plain-100', 'full-100', 'plain-300', 'full-300', 'plain-1000', 'full-1000']


def q(v):
    v = sorted(v)
    return f'{v[len(v) // 2]:.0f} / {v[-1]:.0f}' if v else '-'


print('| doc | engine | n | page 1 ms p50 / max | host CPU to page 1 p50 / max | of it, before the run (reset) p50 / max | load1 |')
print('|---|---|---|---|---|---|---|')
for d in DOCS:
    for e in ('base', 'pr1300'):
        p = f'{O}/{e}-{d}/summary.json'
        if not os.path.exists(p):
            continue
        s = json.load(open(p))
        for r in s['raw']:
            ks = r['phases']['preamble']['keys'][1:]
            w = [k['first_page_ms'] for k in ks]
            c = [k['host']['stages']['first_page_cpu'] for k in ks]
            pre = [k['host']['stages']['first_page_cpu'] - (k['host']['stages'].get('edited_cpu') or 0) for k in ks]
            ld = r.get('load')
            print(f'| {d} | {"main" if e == "base" else "#1300"} | {len(ks)} | {q(w)} | {q(c)} | {q(pre)} | {ld} |')
