"""prtab.py: PR tables, main against the PR, steady and peak footprint (MB) and instructions."""
import collections
import json
import statistics
import sys

rows = collections.defaultdict(list)
for line in open('/private/tmp/mb/out/summary.jsonl'):
    d = json.loads(line)
    rows[(d['tag'], d['doc'], d['mode'])].append(d)


def rng(v):
    v = [x for x in v if x is not None]
    lo, hi = min(v), max(v)
    return f'{lo:.0f}' if hi - lo < 1 else f'{lo:.0f}–{hi:.0f}'


def mean(v):
    return statistics.mean(v)


def table(spec, docs, label):
    """spec: list of (mode, base_tag, pr_tag)."""
    print(f'| doc | mode | steady: main → {label} | peak: main → {label} | open: main → {label} | edited-page instr M | keystroke instr M |')
    print('|---|---|---|---|---|---|---|')
    for doc in docs:
        for mode, bt, pt in spec:
            b, p = rows.get((bt, doc, mode)), rows.get((pt, doc, mode))
            if not b or not p:
                continue
            ib, ip = mean([x['instr_p50_M'] for x in b]), mean([x['instr_p50_M'] for x in p])
            kb, kp = mean([x['all_instr_p50_M'] for x in b]), mean([x['all_instr_p50_M'] for x in p])
            print(f"| {doc} | {mode} | {rng([x['steady_fp_mb'] for x in b])} → **{rng([x['steady_fp_mb'] for x in p])}** | "
                  f"{rng([x['peak_mb'] for x in b])} → {rng([x['peak_mb'] for x in p])} | "
                  f"{rng([x['open_fp_mb'] for x in b])} → {rng([x['open_fp_mb'] for x in p])} | "
                  f"{ib:.1f} → {ip:.1f} ({(ip / ib - 1) * 100:+.1f} %) | {kb:.0f} → {kp:.0f} ({(kp / kb - 1) * 100:+.1f} %) |")


DOCS = ['blank', 'art1', 'art4', 'hw1', 'beamer-default', 'full-100', 'infdesc']
which = sys.argv[1]
if which == 'FA':
    table([('low-memory', 'fa-m3', 'fa-a10'), ('balanced', 'fa-m3', 'fa-a10'), ('high-performance', 'fa-m3', 'fa-a10')], DOCS, 'PR')
    table([('balanced', 'fi-m3', 'fi-a10'), ('high-performance', 'fi-m3', 'fi-a10')], ['infdesc'], 'PR')
if which == 'A':
    table([('low-memory', 'c2-m2', 'c2-a6'), ('balanced', 'c1-m2', 'c1-a5'),
           ('high-performance', 'c1-m2', 'c1-a5')], DOCS, 'PR')
elif which == 'B1':
    # a5 = PR A alone, all = PR A + B1 + B2 (B2 changes nothing from the format)
    table([('low-memory', 'c1-a5', 'c1-all'), ('balanced', 'c1-a5', 'c1-all'),
           ('high-performance', 'c1-a5', 'c1-all')], DOCS, '+B1')
elif which == 'B2':
    table([('low-memory', 's1-m2', 's1-b2m'), ('balanced', 's1-m2', 's1-b2m'),
           ('high-performance', 's1-m2', 's1-b2m')], DOCS, 'B1+B2')
elif which == 'ALL':
    table([('low-memory', 's1-m2', 's1-all'), ('balanced', 's1-m2', 's1-all'),
           ('high-performance', 's1-m2', 's1-all')], DOCS, 'all three')
elif which == 'INF':
    for t in sys.argv[2:]:
        b, p = t.split(',')
        table([('low-memory', b, p), ('balanced', b, p), ('high-performance', b, p)], ['infdesc'], p)
