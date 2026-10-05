#!/usr/bin/env python3
"""Load-independent comparison of engines from keyrun.py's JSONL (any phase): per engine, document and phase,
p50/p95 over the compiles that ran of the engine thread's instructions (millions) in the restore, the convergence
tests, to the edited page and in all; the share of compiles that shipped a page; how the waits for the engine
thread split (`queue_by`, summed ms: wall time, so loaded); and the old chunks kept/rewound (`old_kept`,
`old_rewound`, engine A on).

usage: stagecmp.py DIR... (ENGINE-DOC-PHASE.jsonl[.gz] files; several DIRs pool rounds)"""
import glob, gzip, json, os, sys


def pct(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, round((len(v) - 1) * p))] if v else None


def f(x):
    return '-' if x is None else (f'{x:.1f}' if x < 100 else f'{x:.0f}')


rows = {}
for d in sys.argv[1:]:
    for p in sorted(glob.glob(f'{d}/*.jsonl') + glob.glob(f'{d}/*.jsonl.gz')):
        name = os.path.basename(p).replace('.gz', '')[:-6]
        op = gzip.open(p, 'rt') if p.endswith('.gz') else open(p)
        recs = [json.loads(l) for l in op if l.startswith('{')]
        dones = [r['done'] for r in recs if 'done' in r] + [r['host'] for r in recs if 'host' in r and 'key' in r]
        dones = [x for x in dones if x.get('mode') in ('incremental', 'continued')]
        rows.setdefault(name, []).extend(dones)
print('| engine-doc-phase | compiles | shipped a page | restore M p50/p95 | tests M p50/p95 | to edited page M p50/p95 | '
      'all M p50/p95 | waits by part (ms) | old chunks kept/rewound |')
print('|---|---|---|---|---|---|---|---|---|')
for name, ds in sorted(rows.items()):
    def col(k):
        v = [x['stages'][k] / 1e3 for x in ds if isinstance(x.get('stages', {}).get(k), (int, float))]
        return f'{f(pct(v, .5))}/{f(pct(v, .95))}'
    by = {}
    for x in ds:
        for k, v in (x.get('stages', {}).get('queue_by') or {}).items():
            by[k] = by.get(k, 0) + v
    shipped = sum(1 for x in ds if x.get('first_page_ms') is not None)
    kept = [x['stages'].get('old_kept') for x in ds if 'old_kept' in x.get('stages', {})]
    rew = [x['stages'].get('old_rewound') for x in ds if 'old_rewound' in x.get('stages', {})]
    kr = f'{max(kept) - min(kept)}/{max(rew) - min(rew)}' if kept else '-'
    print(f"| {name} | {len(ds)} | {shipped} | {col('restore_instr_k')} | {col('test_instr_k')} | {col('edited_instr_k')} | "
          f"{col('instr_k')} | {', '.join(f'{k} {v:.0f}' for k, v in sorted(by.items(), key=lambda t: -t[1]) if v >= 1)} | {kr} |")
