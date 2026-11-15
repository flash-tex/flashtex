#!/usr/bin/env python3
"""marks.py JSONL...: the median split of COMPILE arrival -> edited page by phase mark (DONE stages.marks:
[mark, wall us, user us, sys us, instr k] since the previous mark), over the compiles that delivered a
first page, plus the queue. Marks: p request parsed, m spans moved, f find, r restore, k ..., b ..., q/v
READ-REVALIDATE probe and its verdict, j reattach, d first page delivered."""
import json, sys, statistics as st
for f in sys.argv[1:]:
    rows = []
    for l in open(f):
        r = json.loads(l)
        if 'done' not in r:
            continue
        d = r['done']; s = d.get('stages') or {}
        if not d.get('first_page_ms') or not s.get('marks'):
            continue
        seq = []
        for m in s['marks']:
            seq.append((m[0], m[1] / 1e3, m[2] / 1e3, (m[3] or 0) / 1e3, (m[4] or 0) / 1e3))
        rows.append((s.get('queue', 0), (s.get('queue_by_instr_k') or {}), seq, d['first_page_ms'], s.get('to_edited_instr_k')))
    if not rows:
        continue
    # group by mark sequence shape
    shapes = {}
    for q, qi, seq, fp, te in rows:
        shapes.setdefault(''.join(m[0] for m in seq), []).append((q, qi, seq, fp, te))
    print(f.split('/')[-1])
    for sh, rs in sorted(shapes.items(), key=lambda kv: -len(kv[1])):
        print(f'  shape {sh} n={len(rs)} first page p50 {st.median(r[3] for r in rs):.1f} ms; queue p50 {st.median(r[0] for r in rs):.1f} ms, '
              f'queue instr p50 {st.median(sum(r[1].values()) for r in rs) / 1e3:.0f} M; to_edited_instr p50 '
              f'{st.median([r[4] for r in rs if r[4]] or [0]) / 1e3:.0f} M')
        for i, c in enumerate(sh):
            w = st.median(r[2][i][1] for r in rs); u = st.median(r[2][i][2] for r in rs)
            sy = st.median(r[2][i][3] for r in rs); ins = st.median(r[2][i][4] for r in rs)
            print(f'    {c}: wall {w:6.1f} ms  user {u:6.1f}  sys {sy:5.1f}  instr {ins:7.1f} M')
