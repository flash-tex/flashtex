#!/usr/bin/env python3
"""ab_table.py DIR VARIANT...: per document and variant, main's instructions and each other
engine's delta against main (medians), and BOX-MEMO's offers/hits/recordings from bm-*.txt."""
import json
import os
import re
import statistics as st
import sys

d = sys.argv[1]
vs = sys.argv[2:]
rows = [json.loads(l) for l in open(f'{d}/runs.jsonl')]
E0 = 'e0-origin_main'
others = sorted({r['engine'] for r in rows if r['engine'] != E0})
docs = []
for r in rows:
    if r['doc'] not in docs:
        docs.append(r['doc'])


def m(doc, e, v):
    xs = [r['instr'] for r in rows if r['doc'] == doc and r['engine'] == e and r['variant'] == v]
    return st.median(xs) if xs else None


def bm(doc, e, v):
    f = f"{d}/bm-{doc.replace(':', '_')}-{e}-{v}.txt"
    if not os.path.exists(f):
        return '-'
    t = open(f).read()
    g = lambda k: int(re.search(rf'\b{k}: (\d+)', t).group(1)) if re.search(rf'\b{k}: (\d+)', t) else 0
    return f"{g('hits')}/{g('recordings')}"


print('engines:', ', '.join(f'e{i + 1} = {e}' for i, e in enumerate(others)))
cols = []
for v in vs:
    cols.append(f'{v} main')
    cols += [f'{v} e{i + 1}' for i in range(len(others))]
print('| document | ' + ' | '.join(cols) + ' | hits/rec ' + ' '.join(v for v in vs if v != 'off') + ' (main; e1; ...) |')
print('|---|' + '---:|' * len(cols) + '---|')
for doc in docs:
    cells = []
    for v in vs:
        a = m(doc, E0, v)
        cells.append(f'{a / 1e9:.3f} G' if a else '-')
        for e in others:
            b = m(doc, e, v)
            cells.append(f'{100 * (b / a - 1):+.2f} %' if a and b else '-')
    st_ = ' · '.join('; '.join(bm(doc, e, v) for e in [E0] + others) for v in vs if v != 'off')
    print(f'| {doc} | ' + ' | '.join(cells) + f' | {st_} |')
print('PDF identical per document:', all(len({r['pdf'] for r in rows if r['doc'] == doc}) == 1 for doc in docs),
      'statuses', {r['status'] for r in rows}, 'load1', min(r['load1'] for r in rows), max(r['load1'] for r in rows))
