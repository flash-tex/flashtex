#!/usr/bin/env python3
"""allocsum.py OUT...: per memr.py run (OUT.json, OUT.jsonl), peak and steady RSS, and per step the
median edited-page and whole-keystroke instructions (DONE.stages), first keystroke of a step left out."""
import json, statistics, sys
for out in sys.argv[1:]:
    d = json.load(open(out + '.json'))
    steps = {}
    for l in open(out + '.jsonl'):
        r = json.loads(l)
        if not r['line'].startswith('{'):
            continue
        x = json.loads(r['line'])
        st = (x.get('host') or {}).get('stages') or {}
        if 'instr_k' in st:
            steps.setdefault(r['phase'], []).append((st.get('edited_instr_k'), st['instr_k']))
    m = lambda v: statistics.median(v) / 1e3 if v else float('nan')
    parts = []
    for ph, v in steps.items():
        if ph == 'open':
            parts.append(f'open: {sum(b for _, b in v) / 1e6:.2f} G instructions')
            continue
        v = v[1:] if len(v) > 1 else v
        e = [a for a, _ in v if a is not None]
        parts.append(f'{ph}: edited {m(e):.1f} M, key {m([b for _, b in v]):.0f} M')
    print(f'{out.split("/")[-1]:24} peak {max(d["peak"].values()) >> 20} MB steady {d.get("steady_rss", 0) >> 20} MB | '
          + ' | '.join(parts))
