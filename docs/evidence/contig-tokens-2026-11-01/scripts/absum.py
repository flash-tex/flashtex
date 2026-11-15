#!/usr/bin/env python3
"""absum.py RES_DIR: the CONTIG-TOKENS follow-up (review of #2008, items 2-4, 8) in tables.

Cold (RES_DIR/cold/runs.jsonl, bench2.py): per document the median instructions, cycles and peak
RSS of main, mainB (main's build again: the A/A control) and lazy (#2016), each engine's delta
against main with a 95 % bootstrap interval of the ratio of medians (10,000 resamples of each
engine's reps), and whether every PDF and log equals main's. Host (RES_DIR/host/*.jsonl and
*.perf, hostab.sh): per page and engine, the keystrokes' edited-page and DONE latency (p50/p95,
pooled over rounds), the host's stage medians, and the whole host process's instructions and
cycles (median over rounds)."""
import collections, glob, json, os, random, re, statistics, sys

R = sys.argv[1]
ENG = ['main', 'mainB', 'lazy']


def pct(a, b):
    return 100.0 * (a - b) / b


def boot(xs, ys, n=10000, seed=1):
    rnd = random.Random(seed)
    v = []
    for _ in range(n):
        a = statistics.median(rnd.choices(xs, k=len(xs)))
        b = statistics.median(rnd.choices(ys, k=len(ys)))
        v.append(100.0 * (a - b) / b)
    v.sort()
    return v[int(0.025 * n)], v[int(0.975 * n)]


def cold():
    rows = [json.loads(l) for l in open(os.path.join(R, 'cold', 'runs.jsonl'))]
    nat = [r for r in rows if r['kind'] == 'native' and r.get('instructions')]
    docs = sorted({r['doc'] for r in nat}, key=lambda d: (d.startswith('infdesc'), d))
    by = collections.defaultdict(list)
    for r in nat:
        by[(r['doc'], r['engine'])].append(r)
    print('## Cold compiles (perf stat, random order, medians)\n')
    print('| document | engine | n | instructions | Δ vs main | cycles | Δ vs main [95 % CI] | peak RSS | same PDF, log |')
    print('|---|---|---:|---:|---:|---:|---:|---:|---|')
    sums = collections.defaultdict(lambda: [0, 0])
    for d in docs:
        m = by[(d, 'main')]
        mi = statistics.median(r['instructions'] for r in m)
        mc = statistics.median(r['cycles'] for r in m)
        for e in ENG:
            rs = by[(d, e)]
            if not rs:
                continue
            i = statistics.median(r['instructions'] for r in rs)
            c = statistics.median(r['cycles'] for r in rs)
            rss = statistics.median(r['maxrss_kb'] for r in rs) / 1024
            same = all(r['pdf'] == m[0]['pdf'] and r['log'] == m[0]['log'] for r in rs)
            if e == 'main':
                di = dc = ''
            else:
                lo, hi = boot([r['cycles'] for r in rs], [r['cycles'] for r in m])
                di = f'{pct(i, mi):+.2f} %'
                dc = f'{pct(c, mc):+.2f} % [{lo:+.2f}, {hi:+.2f}]'
            print(f'| {d} | {e} | {len(rs)} | {i / 1e9:.2f} G | {di} | {c / 1e9:.2f} G | {dc} | {rss:.0f} MB | {"yes" if same else "NO"} |')
            if not d.startswith('infdesc'):
                sums[e][0] += i
                sums[e][1] += c
    papers = [d for d in docs if not d.startswith('infdesc')]
    print(f'\nThe {len(papers)} papers, sums of medians:')
    for e in ENG:
        i, c = sums[e]
        bi, bc = sums['main']
        print(f'- {e}: instructions {i / 1e9:.2f} G ({pct(i, bi):+.2f} %), cycles {c / 1e9:.2f} G ({pct(c, bc):+.2f} %)')
    for e in ('mainB', 'lazy'):
        k = sum(1 for d in papers if statistics.median(r['cycles'] for r in by[(d, e)])
                < statistics.median(r['cycles'] for r in by[(d, 'main')]))
        print(f'- {e}: fewer cycles than main on {k} of {len(papers)} papers')
    # order effect: cycles of a run against its document's median for that engine, by predecessor
    eff = collections.defaultdict(list)
    for (d, e), rs in by.items():
        med = statistics.median(r['cycles'] for r in rs)
        for r in rs:
            eff[(e, 'after itself' if r.get('after') == e else 'after another')].append(pct(r['cycles'], med))
    print('\nOrder effect (cycles of a run against its own median, all documents):')
    for k in sorted(eff):
        print(f'- {k[0]} {k[1]}: n={len(eff[k])}, mean {statistics.mean(eff[k]):+.2f} %')


def host():
    files = sorted(glob.glob(os.path.join(R, 'host', '*.jsonl')))
    if not files:
        return
    print('\n## Host: keystrokes through the socket (checkpoints on), p50/p95 pooled over rounds; CPU is the host thread\'s CPU time, which CPU-quota throttling does not inflate, wall is shown but is not comparable on a throttled PC\n')
    print('| page | engine | rounds | keys | edited page CPU ms | request CPU ms | convergence test ms | restore ms | edited page wall ms | session instructions | Δ | session cycles | Δ |')
    print('|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|')
    grp = collections.defaultdict(lambda: dict(ep=[], done=[], run=[], ins=[], cyc=[], n=0))
    for f in files:
        m = re.search(r'-p(\d+)-(\w+)-r(\d+)\.jsonl$', f)
        page, e = int(m.group(1)), m.group(2)
        g = grp[(page, e)]
        g['n'] += 1
        for l in open(f):
            if not l.startswith('{"key"'):
                continue
            r = json.loads(l)
            if isinstance(r.get('edited_page_ms'), (int, float)):
                g['ep'].append(r['edited_page_ms'])
            if isinstance(r.get('done_ms'), (int, float)):
                g['done'].append(r['done_ms'])
            h = r.get('host') or {}
            st = h.get('stages') or {}
            for k in ('edited_cpu', 'cpu', 'test', 'restore'):
                if isinstance(st.get(k), (int, float)):
                    g.setdefault(k, []).append(st[k])
        p = f[:-6] + '.perf'
        if os.path.exists(p):
            t = open(p).read()
            for ev, k in (('instructions', 'ins'), ('cycles', 'cyc')):
                mm = re.search(r'^(\d+),[^,]*,' + ev, t, re.M)
                if mm:
                    g[k].append(int(mm.group(1)))

    def q(v, p):
        v = sorted(v)
        return v[round((len(v) - 1) * p)] if v else float('nan')

    for page in sorted({k[0] for k in grp}):
        b = grp[(page, 'main')]
        bi = statistics.median(b['ins']) if b['ins'] else None
        bc = statistics.median(b['cyc']) if b['cyc'] else None
        for e in ENG:
            g = grp.get((page, e))
            if not g:
                continue
            i = statistics.median(g['ins']) if g['ins'] else float('nan')
            c = statistics.median(g['cyc']) if g['cyc'] else float('nan')
            di = '' if e == 'main' or not bi else f'{pct(i, bi):+.2f} %'
            dc = '' if e == 'main' or not bc else f'{pct(c, bc):+.2f} %'
            pp = lambda k: f'{q(g.get(k, []), .5):.1f} / {q(g.get(k, []), .95):.1f}'
            print(f'| {page} | {e} | {g["n"]} | {len(g["ep"])} | {pp("edited_cpu")} | {pp("cpu")} | {pp("test")} | {pp("restore")} | '
                  f'{pp("ep")} | {i / 1e9:.2f} G | {di} | {c / 1e9:.2f} G | {dc} |')


if os.path.exists(os.path.join(R, 'cold', 'runs.jsonl')):
    cold()
host()
