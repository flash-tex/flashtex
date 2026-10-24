#!/usr/bin/env python3
"""bench_readme_table.py: bench_readme.py's and coldopen.py's JSONL as markdown tables (lane BENCH-README).

usage: bench_readme_table.py cold FILE.jsonl...      median, p95 (nearest rank), n per document and engine
       bench_readme_table.py open FILE.jsonl...      coldopen.py: first page, all pages, DONE, peak RSS
"""
import json
import math
import os
import sys
from collections import defaultdict


def pct(xs, p):
    xs = sorted(xs)
    return xs[max(0, math.ceil(p / 100 * len(xs)) - 1)] if xs else None


def secs(s):
    if s is None:
        return '-'
    return f'{s * 1000:.0f} ms' if s < 1 else f'{s:.2f} s'


def mb(kb):
    return f'{kb / 1024:.0f} MB' if kb else '-'


def cold(files):
    walls, bad, passes, par, warm, order = defaultdict(list), defaultdict(int), defaultdict(set), {}, {}, []
    exit1 = defaultdict(int)
    sets = {}
    for f in files:
        for line in open(f):
            r = json.loads(line)
            d = r['doc']
            sets[d] = r.get('set', 'classic')
            if d not in order:
                order.append(d)
            if 'parity' in r:
                par[(d, r['parity'])] = r
                continue
            e = r['engine']
            if r['rep'] == 'warm':
                warm[(d, e)] = r
                continue
            # flashtex exits 1 when TeX or a tool reported an error but the PDF was written (as
            # pdflatex in nonstop mode does); such a run counts when the warm-up's PDF matched, and the
            # table says so
            if r['code'] == 1 and e == 'v3' and r['wall'] is not None:
                exit1[(d, e)] += 1
            elif r['code'] != 0 or r['wall'] is None:
                bad[(d, e)] += 1
                continue
            walls[(d, e)].append(r['wall'])
            if 'passes' in r:
                passes[(d, e)].add(r['passes'])

    def cell(d, e):
        v = walls.get((d, e))
        if not v:
            return 'failed' if bad.get((d, e)) or warm.get((d, e), {}).get('code') else '-'
        if exit1.get((d, e)):
            p = par.get((d, e), {})
            if not p.get('ok'):
                return 'failed (exit 1, output not verified)'
        return f'{secs(pct(v, 50))} / {secs(pct(v, 95))}' + (' †' if exit1.get((d, e)) else '')

    def ratio(d, a, b):
        x, y = walls.get((d, a)), walls.get((d, b))
        return f'{pct(x, 50) / pct(y, 50):.2f}×' if x and y else '-'

    def ok(d, e):
        p = par.get((d, e))
        return p.get('level') or p.get('why') if p else '-'

    uni = all(sets[d] == 'unicode' for d in order)
    one, mk = ('xelatex', 'latexmk') if uni else ('pdflatex', 'latexmk')
    print(f'| document | n | flashtex build p50 / p95 | {one} one pass p50 / p95 | latexmk{" -xelatex" if uni else ""} '
          f'p50 / p95 (passes) | v3 vs one pass | v3 vs latexmk | v3 output vs latexmk |'
          + ('' if uni else ' engine one pass p50 / p95 | engine latexmk p50 / p95 | engine-mk output |'))
    print('|---|---:|---:|---:|---:|---:|---:|---|' + ('' if uni else '---:|---:|---|'))
    for d in order:
        n = min((len(walls.get((d, e), [])) for e in ('v3', one, mk)), default=0)
        ps = '/'.join(map(str, sorted(passes.get((d, mk), set())))) or '-'
        row = (f'| {d} | {n} | {cell(d, "v3")} | {cell(d, one)} | {cell(d, mk)} ({ps}) | '
               f'{ratio(d, one, "v3")} | {ratio(d, mk, "v3")} | {ok(d, "v3")} |')
        if not uni:
            row += f' {cell(d, "engine")} | {cell(d, "engine-mk")} | {ok(d, "engine-mk")} |'
        print(row)
    print('\nRatios are the other engine\'s median over v3\'s: above 1× v3 is faster, below 1× slower.')
    print('\nPeak resident memory of the whole process tree, warm-up run (ps every 100 ms):\n')
    es = ['v3', one, mk] + ([] if uni else ['engine', 'engine-mk'])
    print('| document | ' + ' | '.join(es) + ' | v3 passes (--progress) |')
    print('|---|' + '---:|' * len(es) + '---|')
    for d in order:
        prog = warm.get((d, 'v3'), {}).get('progress') or []
        print(f'| {d} | ' + ' | '.join(mb(warm.get((d, e), {}).get('peak_tree_rss_kb')) for e in es)
              + f' | {(prog[-1] if prog else "-")[:90]} |')
    if exit1:
        print('\n† flashtex exited 1 (an error reported, the PDF written) in', dict(exit1),
              'timed runs; the warm-up\'s PDF of the same build matched latexmk\'s, so the times count.')
    if bad:
        print('\nFailed timed runs:', dict(bad))


def opens(files):
    rows = defaultdict(list)
    order = []
    for f in files:
        for line in open(f):
            if not line.startswith('{'):
                continue
            r = json.loads(line)
            # bench-readme.yml's names: open-DOC.jsonl, reopen-DOC.jsonl
            base = os.path.basename(f)[:-len('.jsonl')] if f.endswith('.jsonl') else os.path.basename(f)
            k = (r.get('doc') or base.split('-', 1)[-1], r.get('tag', ''))
            if k not in order:
                order.append(k)
            rows[k].append(r)
    print('| document | kind | n | pages | first page p50 / p95 | all pages p50 / p95 | DONE p50 / p95 | passes | host peak RSS |')
    print('|---|---|---:|---:|---:|---:|---:|---|---:|')
    for k in order:
        rs = [r for r in rows[k] if not (k[1].startswith('reopen') and r.get('rep') == 0)]
        f = lambda key: [r[key] / 1000 for r in rs if r.get(key) is not None]
        pp = lambda v: f'{secs(pct(v, 50))} / {secs(pct(v, 95))}' if v else '-'
        print(f'| {k[0]} | {k[1]} | {len(rs)} | {rs[0].get("pages") if rs else "-"} | {pp(f("first_page_ms"))} | '
              f'{pp(f("all_seen_ms"))} | {pp(f("done_ms"))} | {sorted({r.get("passes_all") for r in rs})} | '
              f'{mb(max((r.get("peak_rss") or 0) for r in rs) / 1024) if rs else "-"} |')


if __name__ == '__main__':
    {'cold': cold, 'open': opens}[sys.argv[1]](sys.argv[2:])
