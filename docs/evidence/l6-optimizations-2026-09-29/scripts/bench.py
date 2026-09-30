#!/usr/bin/env python3
"""A/B compile timing for L6-OPTIMIZATIONS: engines interleaved, medians.

usage: bench.py DOC... --engines A,B,... [--reps N] [--out JSONL] [--env ENGINE:K=V ...]

An engine NAME is a directory /tmp/l6o/eng/NAME holding `flashtex-initex`,
`pdftex.pool` and `fmt/pdflatex.fmt` (scripts/mkeng.sh makes one); `pdflatex`
is TeX Live's, the oracle. Documents come from /tmp/l6o/docs (the P4-L2-L3
generator, docs/evidence/p4-l2-l3-2026-09-29/scripts/gen.py).

Each document is first compiled by every engine in its own directory until the
.aux settles. Then each repetition runs every engine once, in a rotating order.
Every run is wrapped in `/usr/bin/time -l` on macOS (on Linux the child's
rusage is read, pinned to core $BENCH_CPU when set) and records: user+sys CPU, wall, and on macOS the instructions retired
and cycles elapsed (Apple Silicon PMU counters, which a loaded machine
perturbs far less than CPU seconds). The 1-minute load average is sampled at
every run. Prints medians and the ratio to the first engine."""
import argparse
import json
import os
import platform
import re
import shutil
import signal
import statistics
import subprocess
import time

ap = argparse.ArgumentParser()
ap.add_argument('docs', nargs='+')
ap.add_argument('--engines', required=True)
ap.add_argument('--reps', type=int, default=5)
ap.add_argument('--out')
ap.add_argument('--env', action='append', default=[], help='ENGINE:K=V')
ap.add_argument('--root', default='/tmp/l6o')
a = ap.parse_args()

MAC = platform.system() == 'Darwin'
BASE_ENV = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1', TZ='UTC')
for k in list(BASE_ENV):
    if k.startswith('FLASHTEX_') and k != 'FLASHTEX_TEXLIVE_BIN':
        del BASE_ENV[k]
extra = {}
for spec in a.env:
    e, kv = spec.split(':', 1)
    k, v = kv.split('=', 1)
    extra.setdefault(e, {})[k] = v


def cmd_env(engine, doc):
    env = dict(BASE_ENV, **extra.get(engine, {}))
    if engine == 'pdflatex':
        return ['pdflatex', '-interaction=batchmode', doc + '.tex'], env
    base = engine.split('@')[0]  # NAME@tag: same binary, different --env
    d = f'{a.root}/eng/{base}'
    env.update(FLASHTEX_POOL=f'{d}/pdftex.pool', FLASHTEX_FORMATS=f'{d}/fmt')
    return [f'{d}/flashtex-initex', '-fmt=pdflatex', '-interaction=batchmode', doc + '.tex'], env


# Every compile is killed (with its process group) after this many seconds,
# so an engine that loops cannot hang a benchmark: BENCH_TIMEOUT, default 600.
TIMEOUT = float(os.environ.get('BENCH_TIMEOUT', '600'))


def run(engine, doc, d):
    cmd, env = cmd_env(engine, doc)
    rec = {'load': os.getloadavg()[0]}
    t0 = time.perf_counter()
    if MAC:
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
        u = float(re.search(r'([\d.]+) user', err).group(1))
        s = float(re.search(r'([\d.]+) sys', err).group(1))
        rec['cpu'] = u + s
        for k, pat in (('instr', 'instructions retired'), ('cycles', 'cycles elapsed'),
                       ('peak', 'peak memory footprint')):
            m = re.search(r'(\d+)\s+' + pat, err)
            rec[k] = int(m.group(1)) if m else None
    else:
        # Linux: the child's rusage (no PMU access here), optionally pinned
        # to one core with BENCH_CPU=N.
        pre = ['taskset', '-c', os.environ['BENCH_CPU']] if os.environ.get('BENCH_CPU') else []
        p = subprocess.Popen(pre + cmd, cwd=d, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                             start_new_session=True)
        while True:
            pid, _, ru = os.wait4(p.pid, os.WNOHANG)
            if pid:
                break
            if time.perf_counter() - t0 > TIMEOUT:
                os.killpg(p.pid, signal.SIGKILL)
                os.wait4(p.pid, 0)
                raise SystemExit(f'bench.py: {engine} on {doc} ran over {TIMEOUT:.0f} s; killed')
            time.sleep(0.002)
        p.returncode = 0
        rec['wall'] = time.perf_counter() - t0
        rec['cpu'] = ru.ru_utime + ru.ru_stime
        rec['instr'] = rec['cycles'] = rec['peak'] = None
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
            run(e, doc, d)
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
        rec = dict(doc=doc, engine=e, pages=pages, cpu=med('cpu'), wall=med('wall'), instr=med('instr'),
                   cycles=med('cycles'), peak=med('peak'), load_min=round(min(x['load'] for x in rs), 1),
                   load_max=round(max(x['load'] for x in rs), 1), host=platform.node(),
                   cpu_runs=[round(x['cpu'], 3) for x in rs])
        if first is None:
            first = rec
        ratio = lambda k: (f"{rec[k] / first[k]:.3f}" if rec.get(k) and first.get(k) else '-')
        ins = f"{rec['instr'] / 1e9:7.2f}G" if rec['instr'] else '      -'
        cyc = f"{rec['cycles'] / 1e9:7.2f}G" if rec['cycles'] else '      -'
        print(f"{doc:11} {e:14} p{pages:5} cpu {rec['cpu']:6.3f}s ({ratio('cpu')})  wall {rec['wall']:6.3f}s  "
              f"instr {ins} ({ratio('instr')})  cyc {cyc} ({ratio('cycles')})  "
              f"load {rec['load_min']}-{rec['load_max']}  runs {' '.join(f'{x:.3f}' for x in rec['cpu_runs'])}",
              flush=True)
        if out:
            out.write(json.dumps(rec) + '\n')
            out.flush()
