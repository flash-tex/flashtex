#!/usr/bin/env python3
"""modes_table.py FILE...: the performance modes side by side (lane PERF-MODES; DESIGN.md §1.2):
per document and mode, from mem.py's JSONL (`mem.py --profile MODE`), the host's peak RSS and, per
keystroke, the engine thread's instructions (DONE.stages, load-independent): to the edited page
(`edited_instr_k`, from just before the restore to the edited page's shipout), the restore alone,
and the whole compile. The open's compiles are counted apart. MB = 2^20 bytes; instructions in
millions (M)."""
import json
import sys

runs = {}
for path in sys.argv[1:]:
    recs, summary = [], None
    for line in open(path):
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        if d.get('summary') is True:
            summary = d
        else:
            recs.append(d)
    if summary is None:
        continue
    key = (summary['doc'], summary.get('engine', ''), summary.get('profile') or 'balanced')
    r = runs.setdefault(key, {'recs': [], 'peak': 0, 'killed': False, 'last': None})
    r['recs'] += recs
    r['peak'] = max(r['peak'], summary.get('peak_rss') or 0)
    r['killed'] |= bool(summary.get('killed_at_limit'))
    r['last'] = summary.get('last')


def pct(v, q):
    v = sorted(x for x in v if x is not None)
    return v[min(len(v) - 1, int(q * len(v)))] if v else None


def m(v):
    return '' if v is None else f'{v / 1e3:.1f}'


order = {'low-memory': 0, 'balanced': 1, 'high-performance': 2}
print('| doc | engine | mode | keys | peak RSS MB | logs MB (end) | ckpts (end) '
      '| to edited page p50 / p95 M | restore p50 M | compile p50 / p95 M | edited ms p50 / p95 |')
print('|---|---|---|---|---|---|---|---|---|---|---|')
for (doc, eng, mode), r in sorted(runs.items(), key=lambda kv: (kv[0][0], kv[0][1], order.get(kv[0][2], 9))):
    keys = [x for x in r['recs'] if x.get('key') is not None]
    st = [x.get('stages') or {} for x in keys]
    ed = [s.get('edited_instr_k') for s in st]
    rs = [s.get('restore_instr_k') for s in st]
    tot = [s.get('instr_k') for s in st]
    ms = [x.get('edited_ms') for x in keys]
    last = r['last'] or {}
    kill = ' (killed)' if r['killed'] else ''
    p50ms, p95ms = pct(ms, 0.5), pct(ms, 0.95)
    lat = '' if p50ms is None else f'{p50ms:.1f} / {p95ms:.1f}'
    logs = last.get('sealed_bytes')
    print(f"| {doc}{kill} | {eng} | {mode} | {len(keys)} | {r['peak'] / 2**20:.0f} | "
          f"{'' if logs is None else f'{logs / 2**20:.0f}'} | {last.get('checkpoints', '')} | "
          f"{m(pct(ed, 0.5))} / {m(pct(ed, 0.95))} | {m(pct(rs, 0.5))} | "
          f"{m(pct(tot, 0.5))} / {m(pct(tot, 0.95))} | {lat} |")
