#!/usr/bin/env python3
"""Per-document medians of perf instructions and cycles from a contig-pc runs.jsonl, the 13 papers'
totals, and each engine's delta against BASE; the min and max of each engine's runs are shown so a
noisy cycle count can be judged (lane CONTIG-TOKENS).

usage: pcsum.py RUNS_JSONL [BASE]"""
import collections, json, statistics, sys

R = [json.loads(l) for l in open(sys.argv[1])]
base = sys.argv[2] if len(sys.argv) > 2 else 'main'
nat = [r for r in R if r['kind'] == 'native' and r.get('instructions')]
engines = sorted({r['engine'] for r in nat}, key=lambda e: (e != base, e))
docs = sorted({r['doc'] for r in nat}, key=lambda d: (d.startswith('infdesc'), d))
med = collections.defaultdict(dict)
rng = collections.defaultdict(dict)
for d in docs:
    for e in engines:
        rs = [r for r in nat if r['doc'] == d and r['engine'] == e]
        med[d][e] = (statistics.median(r['instructions'] for r in rs), statistics.median(r['cycles'] for r in rs),
                     statistics.median(r['user'] for r in rs))
        rng[d][e] = (min(r['cycles'] for r in rs), max(r['cycles'] for r in rs), len(rs),
                     statistics.median(r['load'] for r in rs))


def pct(a, b):
    return f'{100.0 * (a - b) / b:+.2f} %'


print('| document | engine | instructions | Δ | cycles (median; min-max) | Δ | load |')
print('|---|---|---:|---:|---:|---:|---:|')
for d in docs:
    for e in engines:
        i, c, u = med[d][e]
        lo, hi, n, ld = rng[d][e]
        bi, bc, _ = med[d][base]
        print(f'| {d} | {e} | {i / 1e9:.2f} G | {pct(i, bi) if e != base else ""} | {c / 1e9:.2f} G ({lo / 1e9:.2f}-{hi / 1e9:.2f}) | '
              f'{pct(c, bc) if e != base else ""} | {ld:.1f} |')
papers = [d for d in docs if not d.startswith('infdesc')]
print(f'\n13 papers (sum of medians, {len(papers)} documents):')
for e in engines:
    ti = sum(med[d][e][0] for d in papers)
    tc = sum(med[d][e][1] for d in papers)
    bi = sum(med[d][base][0] for d in papers)
    bc = sum(med[d][base][1] for d in papers)
    print(f'  {e}: instructions {ti / 1e9:.2f} G ({pct(ti, bi)}), cycles {tc / 1e9:.2f} G ({pct(tc, bc)})')
for e in engines:
    if e == base:
        continue
    better = sum(1 for d in papers if med[d][e][1] < med[d][base][1])
    print(f'  {e}: fewer cycles than {base} on {better} of {len(papers)} papers')
