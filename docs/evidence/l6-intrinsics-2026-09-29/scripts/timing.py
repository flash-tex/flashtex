#!/usr/bin/env python3
"""Whole-document compile CPU: engines interleaved, median of N runs.

usage: timing.py DOC... [--reps N] [--out JSONL]

Engines: `base` (/tmp/l6/base: this lane's starting point, origin/agent/kabir-claude/p4-l2-l3),
`off` and `on` (/tmp/l6/i1 with FLASHTEX_INTRINSICS=off / default), and `pdflatex` (TeX
Live's, the oracle). Each document (from /tmp/l6/docs) is first compiled to a settled
.aux by every engine in its own directory; then each repetition runs every engine once, in
a rotating order, and records the child's user+sys CPU (wait4) and wall time. Prints the
median CPU per engine and document, pages, and per-page CPU (median CPU / pages)."""
import argparse
import json
import os
import re
import shutil
import statistics
import subprocess
import time

ap = argparse.ArgumentParser()
ap.add_argument('docs', nargs='+')
ap.add_argument('--reps', type=int, default=5)
ap.add_argument('--engines', default='base,off,on,pdflatex')
ap.add_argument('--out')
a = ap.parse_args()

BASE_ENV = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1', TZ='UTC')
for k in list(BASE_ENV):
    if k.startswith('FLASHTEX_'):
        del BASE_ENV[k]


def cmd_env(engine, doc):
    if engine == 'pdflatex':
        return ['pdflatex', '-interaction=batchmode', doc + '.tex'], dict(BASE_ENV)
    e = 'base' if engine == 'base' else 'i1'
    env = dict(BASE_ENV, FLASHTEX_POOL=f'/tmp/l6/{e}/pdftex.pool', FLASHTEX_FORMATS=f'/tmp/l6/fmt-{e}')
    if engine == 'off':
        env['FLASHTEX_INTRINSICS'] = 'off'
    return [f'/tmp/l6/{e}/flashtex-initex', '-fmt=pdflatex', '-interaction=batchmode', doc + '.tex'], env


def run(engine, doc, d):
    cmd, env = cmd_env(engine, doc)
    t0 = time.perf_counter()
    p = subprocess.Popen(cmd, cwd=d, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    _, status, ru = os.wait4(p.pid, 0)
    wall = time.perf_counter() - t0
    return ru.ru_utime + ru.ru_stime, wall


engines = a.engines.split(',')
results = []
out = open(a.out, 'a') if a.out else None
for doc in a.docs:
    dirs = {}
    for e in engines:
        d = f'/tmp/l6/time-{e}-{doc}'
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(d)
        shutil.copy(f'/tmp/l6/docs/{doc}.tex', d)
        prev = None
        for _ in range(4):
            run(e, doc, d)
            aux = open(os.path.join(d, doc + '.aux'), 'rb').read()
            if aux == prev:
                break
            prev = aux
        dirs[e] = d
    cpu = {e: [] for e in engines}
    wall = {e: [] for e in engines}
    for r in range(a.reps):
        order = engines[r % len(engines):] + engines[:r % len(engines)]
        for e in order:
            c, w = run(e, doc, dirs[e])
            cpu[e].append(c)
            wall[e].append(w)
    pages = {}
    for e in engines:
        log = open(os.path.join(dirs[e], doc + '.log'), 'rb').read().decode('latin1')
        m = re.search(r'Output written on .*?\((\d+) pages?', log, re.S)
        pages[e] = int(m.group(1)) if m else 0
    load = os.getloadavg()[0]
    for e in engines:
        med = statistics.median(cpu[e])
        rec = dict(doc=doc, engine=e, pages=pages[e], cpu_median_s=round(med, 4),
                   cpu_runs=[round(x, 4) for x in cpu[e]], wall_median_s=round(statistics.median(wall[e]), 4),
                   per_page_ms=round(1000 * med / max(1, pages[e]), 3), load1=round(load, 1))
        results.append(rec)
        print(f"{doc:12} {e:9} pages {pages[e]:5}  cpu median {med:7.3f} s  per page {rec['per_page_ms']:6.3f} ms  "
              f"(runs {' '.join(f'{x:.3f}' for x in cpu[e])}; load {load:.1f})", flush=True)
        if out:
            out.write(json.dumps(rec) + '\n')
            out.flush()
