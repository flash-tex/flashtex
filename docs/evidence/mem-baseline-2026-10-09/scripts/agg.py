"""agg.py TAG ENGINES...: per doc and mode, each engine's figures (mean over rounds; range when
two), as a markdown table: peak / open / steady footprint MB, edited-page and keystroke instructions."""
import collections
import json
import statistics
import sys

tag, engines = sys.argv[1], sys.argv[2:]
rows = collections.defaultdict(list)
for line in open('/private/tmp/mb/out/summary.jsonl'):
    d = json.loads(line)
    for e in engines:
        if d['tag'] == f'{tag}-{e}':
            rows[(d['doc'], d['mode'], e)].append(d)
docs = []
for (doc, mode, e) in rows:
    if doc not in docs:
        docs.append(doc)


def f(vals):
    vals = [v for v in vals if v is not None]
    if not vals:
        return '-'
    if len(vals) == 1 or max(vals) - min(vals) < 0.05:
        return f'{vals[0]:.0f}' if vals[0] >= 20 else f'{vals[0]:.1f}'
    return f'{min(vals):.0f}–{max(vals):.0f}'


hdr = ' | '.join(f'{e} peak / open / steady' for e in engines)
print(f'| doc | mode | {hdr} | edited instr M ({", ".join(engines)}) | keystroke instr M |')
print('|---|---|' + '---|' * len(engines) + '---|---|')
for doc in docs:
    for mode in ('low-memory', 'balanced', 'high-performance'):
        cells, ins, kins = [], [], []
        for e in engines:
            r = rows.get((doc, mode, e), [])
            if not r:
                cells.append('-')
                ins.append('-')
                kins.append('-')
                continue
            cells.append(f"{f([x['peak_mb'] for x in r])} / {f([x['open_fp_mb'] for x in r])} / "
                         f"**{f([x['steady_fp_mb'] for x in r])}**")
            ins.append(f"{statistics.mean(x['instr_p50_M'] for x in r):.1f}")
            kins.append(f"{statistics.mean(x['all_instr_p50_M'] for x in r):.0f}")
        print(f"| {doc} | {mode} | {' | '.join(cells)} | {' / '.join(ins)} | {' / '.join(kins)} |")
