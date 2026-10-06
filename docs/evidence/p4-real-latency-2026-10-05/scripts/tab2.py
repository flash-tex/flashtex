#!/usr/bin/env python3
"""tab2.py FILE...: the step-1 rows (p50/p95) for each dl3-keys file."""
import json, sys


def pct(v, p):
    v = sorted(x for x in v if x is not None)
    return v[min(len(v) - 1, round((len(v) - 1) * p))] if v else None


def rows(path):
    recs = [json.loads(l) for l in open(path) if l.startswith('{')]
    dones = {r['done']['id']: r['done'] for r in recs if 'done' in r}
    keyl = [r for r in recs if 'key' in r and r['key'] >= 1]
    summ = [r for r in recs if 'interval_ms' in r]
    if summ:
        iv, first = summ[-1]['interval_ms'], min(dones)
        return [(r, dones.get(r['by'], {}), (r['by'] - first - r['key']) * iv) for r in keyl]
    return [(r, r.get('host', {}), 0.0) for r in keyl]


def S(d, k, dflt=None):
    return (d.get('stages') or {}).get(k, dflt)


for p in sys.argv[1:]:
    R = rows(p)
    cols = {
        'key->PAGE': [r.get('edited_page_ms') for r, d, l in R],
        'queue': [S(d, 'queue') for r, d, l in R],
        'apply+spans': [(S(d, 'apply') or 0) + (S(d, 'move_spans') or 0) for r, d, l in R],
        'S0key': [S(d, 'key') for r, d, l in R],
        'changes': [S(d, 'changes') for r, d, l in R],
        'paused+rest of find': [(S(d, 'find') or 0) - (S(d, 'key') or 0) - (S(d, 'changes') or 0) for r, d, l in R],
        'restore': [S(d, 'restore') for r, d, l in R],
        'typeset+DL+send': [d['first_page_ms'] - sum(S(d, k) or 0 for k in ('queue', 'apply', 'move_spans', 'find', 'restore'))
                            if d.get('first_page_ms') is not None else None for r, d, l in R],
        'DL(first page)': [S(d, 'first_page_dl') for r, d, l in R],
        'engine CPU': [S(d, 'first_page_cpu') for r, d, l in R],
        'off-CPU': [d['first_page_ms'] - S(d, 'queue') - S(d, 'first_page_cpu')
                    if d.get('first_page_ms') is not None and S(d, 'first_page_cpu') is not None else None for r, d, l in R],
        'after 1st PAGE': [r['edited_page_ms'] - l - d['first_page_ms'] if d.get('first_page_ms') is not None else None for r, d, l in R],
        'Minstr page': [S(d, 'first_page_instr_k', 0) / 1e3 or None for r, d, l in R],
        'Minstr restore': [S(d, 'restore_instr_k', 0) / 1e3 or None for r, d, l in R],
        'Minstr typeset': [S(d, 'typeset_instr_k', 0) / 1e3 or None for r, d, l in R],
        'Mcyc arrival->page': [(S(d, 'first_page_mark_kc') - S(d, 'arrival_mark_kc')) / 1e3
                               if S(d, 'first_page_mark_kc') and S(d, 'arrival_mark_kc') else None for r, d, l in R],
    }
    qb = {}
    for r, d, l in R:
        for k, v in (S(d, 'queue_by') or {}).items():
            qb.setdefault(k, []).append(v)
    print(p.split('/out/')[-1], 'n', len(R), 'own', sum(1 for r, d, l in R if l == 0))
    for k, v in cols.items():
        a, b = pct(v, .5), pct(v, .95)
        print(f'   {k:22s} {"-" if a is None else f"{a:.1f}"}/{"-" if b is None else f"{b:.1f}"}')
    print('   queue_by p95:', {k: round(pct(v + [0] * (len(R) - len(v)), .95), 1) for k, v in qb.items()})
