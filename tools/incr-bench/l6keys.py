#!/usr/bin/env python3
"""l6keys.py R: BOX-MEMO's keystrokes, engine against engine (lane L6, measurement only): for each
R/<doc>-<site>-<eN>/raw/*.jsonl that t7.py wrote, per phase and engine, the engine thread's
instructions to the edited page (DONE.stages edited_instr_k, from just before the restore to the
edited page's shipout) and over the whole compile (instr_k), and BOX-MEMO's offers, hits and
recordings per keystroke (DONE.stages boxmemo, the session's counts, differenced key to key). The
first key of each phase is a warm-up and is left out, as t7.py leaves it out of its times; the
keystrokes are the same on every engine, so the sums are paired. Instructions in millions (M)."""
import collections
import glob
import json
import os
import re
import sys

R = sys.argv[1]
rows = collections.defaultdict(dict)  # (doc, phase) -> engine -> stats
for path in sorted(glob.glob(f'{R}/*-e[0-9]*/raw/*.jsonl')):
    m = re.match(r'(.*)-(e\d+)$', os.path.basename(os.path.dirname(os.path.dirname(path))))
    if not m:
        continue
    eng = m.group(2)
    phase = re.sub(r'\.jsonl$', '', os.path.basename(path))
    if 'debug' in m.group(1):
        phase = f'{m.group(1)} {phase}'
    keys = []
    for line in open(path):
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        if d.get('key') is None or not isinstance(d.get('host'), dict):
            continue
        keys.append(d['host'].get('stages') or {})
    if len(keys) < 3:
        rows[(phase,)][eng] = dict(n=len(keys), bad=True)
        continue
    ed = [s.get('edited_instr_k') for s in keys[1:]]
    tot = [s.get('instr_k') for s in keys[1:]]
    bm = [s.get('boxmemo') or [0, 0, 0, 0] for s in keys]
    d = [[b[j] - a[j] for j in range(3)] for a, b in zip(bm, bm[1:])]
    rows[(phase,)][eng] = dict(
        n=len(ed), bad=None in ed,
        ed=sorted(x for x in ed if x is not None), ed_sum=sum(x for x in ed if x is not None),
        tot_sum=sum(x for x in tot if x is not None),
        offers=sum(x[0] for x in d) / len(d), hits=sum(x[1] for x in d) / len(d),
        recs=sum(x[2] for x in d) / len(d))


def p50(v):
    return v[len(v) // 2] if v else None


def mm(k):
    return '' if k is None else f'{k / 1e3:.1f}'


engs = sorted({e for r in rows.values() for e in r}, key=lambda e: int(e[1:]))
print('| phase | engine | keys | to edited page p50 M | to edited page, sum M | Δ sum vs e0 | compile sum M | '
      'offers / key | hits / key | recordings / key |')
print('|---|---|---|---|---|---|---|---|---|---|')
for (phase,), r in sorted(rows.items()):
    base = r.get('e0', {}).get('ed_sum')
    for e in engs:
        s = r.get(e)
        if s is None:
            print(f'| {phase} | {e} | (missing) | | | | | | | |')
            continue
        if 'ed' not in s:
            print(f'| {phase} | {e} | {s["n"]} (too few) | | | | | | | |')
            continue
        dl = '' if not base or e == 'e0' else f'{100 * (s["ed_sum"] - base) / base:+.2f} %'
        print(f'| {phase} | {e} | {s["n"]}{" (gaps)" if s["bad"] else ""} | {mm(p50(s["ed"]))} | '
              f'{mm(s["ed_sum"])} | {dl} | {mm(s["tot_sum"])} | {s["offers"]:.1f} | {s["hits"]:.1f} | '
              f'{s["recs"]:.1f} |')
