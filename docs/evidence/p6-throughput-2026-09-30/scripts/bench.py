#!/usr/bin/env python3
"""P6-THROUGHPUT cold full-compile timing: engines interleaved, load-gated.

usage: bench.py DOC... --engines A,B,... [--reps N] [--out JSONL] [--root DIR]
                [--max-load L]

Derived from docs/evidence/l6-optimizations-2026-09-29/scripts/bench.py; the
differences are the load gate and the statistics reported.

An engine NAME is a directory ROOT/eng/NAME holding `flashtex-initex`,
`pdftex.pool` and `fmt/pdflatex.fmt` (mkeng.sh makes one); `pdflatex` is TeX
Live's pdfTeX 1.40.29, the oracle. Documents come from ROOT/docs (the P4-L2-L3
generator, docs/evidence/p4-l2-l3-2026-09-29/scripts/gen.py).

"Cold full compile": a new process from the format, one pass over the whole
document with its .aux already settled (no resident host, no snapshot). Each
document is first compiled by every engine in its own directory until the .aux
settles. Then each repetition runs every engine once, in a rotating order.

Load gate: before every timed run the 1-minute load average must be below
--max-load (default 3, the lane's rule); the script waits (polling every 5 s)
until it is. Every run is wrapped in `/usr/bin/time -l` and records user+sys
CPU, wall, instructions retired and cycles elapsed (Apple Silicon PMU) and peak
memory. Prints median and min of CPU and wall per engine, and the ratio of the
medians to the first engine."""
import argparse
import json
import os
import platform
import re
import shutil
import signal
import statistics
import subprocess
import sys
import time

ap = argparse.ArgumentParser()
ap.add_argument('docs', nargs='+')
ap.add_argument('--engines', required=True)
ap.add_argument('--reps', type=int, default=5)
ap.add_argument('--out')
ap.add_argument('--root', default=os.path.expanduser('~/flashtex-wt/d2'))
ap.add_argument('--max-load', type=float, default=3.0)
a = ap.parse_args()

BASE_ENV = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1', TZ='UTC')
for k in list(BASE_ENV):
    if k.startswith('FLASHTEX_') and k != 'FLASHTEX_TEXLIVE_BIN':
        del BASE_ENV[k]
TIMEOUT = float(os.environ.get('BENCH_TIMEOUT', '600'))


def cmd_env(engine, doc):
    env = dict(BASE_ENV)
    if engine == 'pdflatex':
        return ['pdflatex', '-interaction=batchmode', doc + '.tex'], env
    d = f'{a.root}/eng/{engine}'
    env.update(FLASHTEX_POOL=f'{d}/pdftex.pool', FLASHTEX_FORMATS=f'{d}/fmt')
    return [f'{d}/flashtex-initex', '-fmt=pdflatex', '-interaction=batchmode', doc + '.tex'], env


def gate():
    waited = 0
    while os.getloadavg()[0] >= a.max_load:
        if waited % 60 == 0:
            print(f'  (load {os.getloadavg()[0]:.2f} >= {a.max_load}; waiting)', file=sys.stderr, flush=True)
        time.sleep(5)
        waited += 5


def run(engine, doc, d, timed=True):
    if timed:
        gate()
    cmd, env = cmd_env(engine, doc)
    rec = {'load': os.getloadavg()[0]}
    t0 = time.perf_counter()
    p = subprocess.Popen(['/usr/bin/time', '-l'] + cmd, cwd=d, env=env, stdout=subprocess.DEVNULL,
                         stderr=subprocess.PIPE, start_new_session=True)
    try:
        _, err = p.communicate(timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        os.killpg(p.pid, signal.SIGKILL)
        p.wait()
        raise SystemExit(f'bench.py: {engine} on {doc} ran over {TIMEOUT:.0f} s; killed')
    rec['wall'] = time.perf_counter() - t0
    err = err.decode('latin1')
    rec['cpu'] = float(re.search(r'([\d.]+) user', err).group(1)) + float(re.search(r'([\d.]+) sys', err).group(1))
    for k, pat in (('instr', 'instructions retired'), ('cycles', 'cycles elapsed'),
                   ('peak', 'peak memory footprint')):
        m = re.search(r'(\d+)\s+' + pat, err)
        rec[k] = int(m.group(1)) if m else None
    return rec


engines = a.engines.split(',')
out = open(a.out, 'a') if a.out else None
for doc in a.docs:
    dirs = {}
    for e in engines:
        d = f'{a.root}/time/{e}-{doc}'
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(d)
        shutil.copy(f'{a.root}/docs/{doc}.tex', d)
        prev = None
        for _ in range(4):
            run(e, doc, d, timed=False)
            aux = open(os.path.join(d, doc + '.aux'), 'rb').read()
            if aux == prev:
                break
            prev = aux
        dirs[e] = d
    runs = {e: [] for e in engines}
    for r in range(a.reps):
        order = engines[r % len(engines):] + engines[:r % len(engines)]
        for e in order:
            runs[e].append(run(e, doc, dirs[e]))
    first = None
    for e in engines:
        log = open(os.path.join(dirs[e], doc + '.log'), 'rb').read().decode('latin1')
        m = re.search(r'Output written on .*?\((\d+) pages?', log, re.S)
        pages = int(m.group(1)) if m else 0
        rs = runs[e]
        med = lambda k: statistics.median([x[k] for x in rs]) if rs[0].get(k) is not None else None
        mn = lambda k: min(x[k] for x in rs)
        rec = dict(doc=doc, engine=e, pages=pages, cpu=med('cpu'), cpu_min=mn('cpu'), wall=med('wall'),
                   wall_min=mn('wall'), instr=med('instr'), cycles=med('cycles'), peak=med('peak'),
                   load_min=round(min(x['load'] for x in rs), 2), load_max=round(max(x['load'] for x in rs), 2),
                   host=platform.node(), reps=len(rs), runs=rs)
        if first is None:
            first = rec
        ratio = lambda k: (f"{rec[k] / first[k]:.3f}" if rec.get(k) and first.get(k) else '-')
        print(f"{doc:11} {e:12} p{pages:5} cpu med {rec['cpu']:6.3f} min {rec['cpu_min']:6.3f} ({ratio('cpu')})  "
              f"wall med {rec['wall']:6.3f} min {rec['wall_min']:6.3f} ({ratio('wall')})  "
              f"cyc {rec['cycles'] / 1e9:6.2f}G ({ratio('cycles')})  instr {rec['instr'] / 1e9:6.2f}G  "
              f"peak {rec['peak'] / 2**20:5.0f}M  load {rec['load_min']}-{rec['load_max']}", flush=True)
        if out:
            out.write(json.dumps(rec) + '\n')
            out.flush()
