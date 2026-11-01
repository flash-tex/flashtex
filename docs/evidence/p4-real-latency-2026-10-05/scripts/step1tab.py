#!/usr/bin/env python3
"""step1tab.py DIR: per measurement window (marks.txt), the slice's CPU throttling (cg.txt), and per
phase file the key's latency split (split.py's stages) in one compact line each."""
import glob, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
D = sys.argv[1]
cg = [l.split() for l in open(f'{D}/cg.txt')]


def at(t):
    best = cg[0]
    for r in cg:
        if float(r[0]) <= t:
            best = r
    return best


def parse(r):
    kv = r[2:]
    return {kv[i]: int(kv[i + 1]) for i in range(0, len(kv), 2)}


def pct(v, p):
    v = sorted(x for x in v if x is not None)
    return v[min(len(v) - 1, round((len(v) - 1) * p))] if v else None


ms = [l.split() for l in open(f'{D}/marks.txt')]
win = {}
for i in range(0, len(ms), 2):
    t0, t1 = float(ms[i][0]), float(ms[i + 1][0])
    a, b = parse(at(t0)), parse(at(t1))
    per = b['nr_periods'] - a['nr_periods']
    thr = b['nr_throttled'] - a['nr_throttled']
    us = (b['throttled_usec'] - a['throttled_usec']) / 1e6
    loads = [float(r[1]) for r in cg if t0 <= float(r[0]) <= t1] or [0]
    tag = ' '.join(ms[i][2:])
    win[tag] = f'throttled {thr}/{per} periods ({100 * thr / max(per, 1):.0f} %, {us:.1f} s), load {min(loads):.1f}-{max(loads):.1f}'
    print(f'{tag:22s} {t1 - t0:5.0f} s  {win[tag]}')


def rows(path):
    recs = [json.loads(l) for l in open(path) if l.startswith('{')]
    dones = {r['done']['id']: r['done'] for r in recs if 'done' in r}
    keyl = [r for r in recs if 'key' in r and r['key'] >= 1]
    summ = [r for r in recs if 'interval_ms' in r]
    out = []
    if summ:
        iv, first = summ[-1]['interval_ms'], min(dones)
        for r in keyl:
            out.append((r, dones.get(r['by'], {}), (r['by'] - first - r['key']) * iv))
    else:
        out = [(r, r.get('host', {}), 0.0) for r in keyl]
    return out


def line(path):
    R = rows(path)
    g = lambda f: [f(r, d, l) for r, d, l in R]
    st = lambda k: (lambda r, d, l: (d.get('stages') or {}).get(k))
    cols = [
        ('key->page', g(lambda r, d, l: r.get('edited_page_ms'))),
        ('later', g(lambda r, d, l: l)),
        ('queue', g(st('queue'))),
        ('q.typeset', g(lambda r, d, l: ((d.get('stages') or {}).get('queue_by') or {}).get('typeset', 0))),
        ('q.jump', g(lambda r, d, l: ((d.get('stages') or {}).get('queue_by') or {}).get('jump', 0))),
        ('q.test', g(lambda r, d, l: ((d.get('stages') or {}).get('queue_by') or {}).get('test', 0))),
        ('q.prep', g(lambda r, d, l: ((d.get('stages') or {}).get('queue_by') or {}).get('prepare', 0))),
        ('spans', g(st('move_spans'))),
        ('key', g(st('key'))),
        ('find', g(st('find'))),
        ('restore', g(st('restore'))),
        ('cpu', g(st('first_page_cpu'))),
        ('offcpu', g(lambda r, d, l: d['first_page_ms'] - d['stages']['queue'] - d['stages']['first_page_cpu']
                     if d.get('first_page_ms') is not None and (d.get('stages') or {}).get('first_page_cpu') is not None else None)),
        ('Mi.page', g(lambda r, d, l: (d.get('stages') or {}).get('first_page_instr_k', 0) / 1e3 or None)),
        ('Mi.rst', g(lambda r, d, l: (d.get('stages') or {}).get('restore_instr_k', 0) / 1e3 or None)),
        ('Mc.arr', g(lambda r, d, l: ((d['stages']['first_page_mark_kc'] - d['stages']['arrival_mark_kc']) / 1e3)
                     if 'arrival_mark_kc' in (d.get('stages') or {}) and 'first_page_mark_kc' in d['stages'] else None)),
    ]
    own = sum(1 for r, d, l in R if l == 0)
    s = ' '.join(f'{n} {pct(v, .5) if pct(v, .5) is None else round(pct(v, .5), 1)}/{pct(v, .95) if pct(v, .95) is None else round(pct(v, .95), 1)}' for n, v in cols)
    return f'n {len(R)} own {own}  ' + s


for p in sorted(glob.glob(f'{D}/t7-*/raw/*letter@middle.jsonl') + glob.glob(f'{D}/kr-*/*.jsonl')):
    print(os.path.relpath(p, D))
    print('   ', line(p))
