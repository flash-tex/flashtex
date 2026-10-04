#!/usr/bin/env python3
"""P6-HYPEROPT cold full-compile counters: instructions retired and cycles (Apple PMU, `/usr/bin/time -l`)
of one settled pass, per engine, plus user+sys and wall (non-reference on a loaded host).

usage: cold.py OUT.jsonl --engines NAME[,NAME...] [--reps N] DOC...
DOC is DIR:ENTRY (a source tree and its main file). An engine NAME is $INCR_BENCH_DIR/NAME (mkeng.sh), with
its format in $INCR_BENCH_DIR/fmt-NAME; `pdftex` is TeX Live's pdfTeX (the oracle). Each document is copied
once per engine into $INCR_BENCH_DIR/cold/<engine>/<doc>, compiled until its .aux settles (at most 5 passes),
then timed REPS times, engines interleaved."""
import argparse, hashlib, json, os, re, shutil, subprocess, sys, time

IB = os.environ.get('INCR_BENCH_DIR', '/tmp/p6h')
ap = argparse.ArgumentParser()
ap.add_argument('out')
ap.add_argument('--engines', required=True)
ap.add_argument('--reps', type=int, default=3)
ap.add_argument('--timeout', type=float, default=3600)
ap.add_argument('docs', nargs='+')
a = ap.parse_args()

BASE = {k: v for k, v in os.environ.items() if not k.startswith('FLASHTEX_')}
BASE.update(SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1', max_print_line='10000', error_line='254',
            half_error_line='238')


def cmd(eng, entry):
    stem = entry[:-4] if entry.endswith('.tex') else entry
    env = dict(BASE)
    if eng == 'pdftex':
        exe = '/Library/TeX/texbin/pdftex'
    else:
        exe = f'{IB}/{eng}/pdftex'
        env.update(FLASHTEX_POOL=f'{IB}/{eng}/pdftex.pool', FLASHTEX_FORMATS=f'{IB}/fmt-{eng}')
    return [exe, '-fmt=pdflatex', '-interaction=batchmode', f'-jobname={stem}',
            r'\pdfsetrandomseed 1\relax\input{' + entry + '}'], env, stem


def run(eng, d, entry):
    c, env, stem = cmd(eng, entry)
    t0 = time.perf_counter()
    p = subprocess.run(['/usr/bin/time', '-l'] + c, cwd=d, env=env, stdout=subprocess.DEVNULL,
                       stderr=subprocess.PIPE, stdin=subprocess.DEVNULL, timeout=a.timeout)
    wall = time.perf_counter() - t0
    e = p.stderr.decode(errors='replace')
    g = lambda pat: float(re.search(pat, e).group(1)) if re.search(pat, e) else None
    return dict(rc=p.returncode, wall=wall, user=g(r'([\d.]+) user'), sys=g(r'([\d.]+) sys'),
                instr=g(r'(\d+)\s+instructions retired'), cycles=g(r'(\d+)\s+cycles elapsed'),
                rss=g(r'(\d+)\s+maximum resident'), load=os.getloadavg()[0])


def h(path):
    try:
        return hashlib.sha256(open(path, 'rb').read()).hexdigest()
    except OSError:
        return None


engines = a.engines.split(',')
out = open(a.out, 'a')
for spec in a.docs:
    src, entry = spec.rsplit(':', 1)
    name = os.path.basename(src.rstrip('/'))
    dirs = {}
    for eng in engines:
        d = f'{IB}/cold/{eng}/{name}'
        shutil.rmtree(d, ignore_errors=True)
        shutil.copytree(src, d, symlinks=True)
        stem = cmd(eng, entry)[2]
        prev = None
        for i in range(5):
            r = run(eng, d, entry)
            cur = (h(f'{d}/{stem}.aux'), h(f'{d}/{stem}.toc'))
            if cur == prev:
                break
            prev = cur
        dirs[eng] = d
        print(f'{name} {eng}: settled after {i + 1} passes rc={r["rc"]}', file=sys.stderr, flush=True)
    for rep in range(a.reps):
        order = engines[rep % len(engines):] + engines[:rep % len(engines)]
        for eng in order:
            r = run(eng, dirs[eng], entry)
            stem = cmd(eng, entry)[2]
            r.update(doc=name, engine=eng, rep=rep, pdf=h(f'{dirs[eng]}/{stem}.pdf'))
            out.write(json.dumps(r) + '\n')
            out.flush()
            print(json.dumps(r), file=sys.stderr, flush=True)
