#!/usr/bin/env python3
"""Convergence statistics of an L3 latency matrix (review 2026-09-30, track 1).

usage: convergence.py MATRIX_DIR            (matrix.py output: <doc>-<region>-<type>.jsonl)
       convergence.py --p4l5 LATENCY.jsonl.gz  (docs/evidence/p4-l5-2026-09-29/raw; 66 records
                                                per document, documents in glob order)

Per document: how many compiles converged (the old run's later pages reused), the
background work of the rest (`total_s`: the whole compile, edited page included;
`rerun_pages`), and, for compiles that did not converge although only one page
changed in one pass, the reason the *last* convergence test failed (the first test
is the edited page's, which must differ), with numbers blanked.
"""
import collections, glob, gzip, json, os, re, statistics as st, sys


def load():
    if sys.argv[1] == '--p4l5':
        recs = [json.loads(l) for l in gzip.open(sys.argv[2], 'rt')]
        names = ['full-10', 'full-100', 'full-1000', 'full-300', 'plain-10', 'plain-100', 'plain-1000', 'plain-300']
        out = collections.defaultdict(list)
        for i, r in enumerate(recs):
            r['_type'] = 'char' if (i % 66) % 22 < 16 else 'sentence'  # 3 regions x (16 char, 6 sentence)
            out[names[i // 66]].append(r)
        return out
    out = collections.defaultdict(list)
    for p in sorted(glob.glob(os.path.join(sys.argv[1], '*.jsonl'))):
        doc, region, typ = os.path.basename(p)[:-6].rsplit('-', 2)
        for l in open(p):
            r = json.loads(l)
            if 'converged_at' not in r:
                continue
            r['_type'] = typ
            out[doc].append(r)
    return out


def q(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, int(p * len(v)))] if v else float('nan')


docs = load()
order = sorted(docs, key=lambda d: (d.split('-')[0], int(d.split('-')[1])))
print('| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |')
print('|---|---|---|---|---|---|')
reasons = collections.Counter()
for d in order:
    rs = docs[d]
    conv = lambda t: (sum(1 for r in rs if r['_type'] == t and r['converged_at'] is not None), sum(1 for r in rs if r['_type'] == t))
    c, s = conv('char'), conv('sentence')
    tot = [r['total_s'] for r in rs]
    rer = [r['rerun_pages'] for r in rs]
    cpu = [r['edited_page_cpu'] * 1000 for r in rs if r.get('edited_page_cpu') is not None]
    print(f'| {d} | {len(rs)} | {c[0] + s[0]} ({c[0]}/{c[1]} / {s[0]}/{s[1]}) | {st.median(tot):.3f} / {q(tot, .95):.3f} / {max(tot):.2f} '
          f'| {st.median(rer):.0f} / {max(rer)} | {q(cpu, .95):.1f} |')
    for r in rs:
        if r['converged_at'] is None and r.get('changed_pages') == 1 and r.get('passes', 1) == 1 and r.get('diffs'):
            reasons[(d.split('-')[0], re.sub(r'0x[0-9a-f]+|\d+', 'N', r['diffs'][-1])[:110])] += 1
print('\nLast failed convergence test of compiles that changed one page in one pass and did not converge:\n')
print('| kind | count | reason |')
print('|---|---|---|')
for (k, why), n in reasons.most_common(14):
    print(f'| {k} | {n} | `{why}` |')
