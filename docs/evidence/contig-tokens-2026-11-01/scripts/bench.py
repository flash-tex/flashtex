#!/usr/bin/env python3
"""CONTIG-TOKENS cold full-compile counters on a Linux runner (lane CONTIG-TOKENS, DESIGN.md §12 P6).

usage: bench.py OUT_DIR --engine NAME=BIN_DIR[,VAR=VALUE...] ... [--reps N] [--cg sim|nosim|none]
                [--timeout S] DOC...

DOC is DIR:ENTRY (a source tree and its main file). BIN_DIR holds `pdftex` (a link to
flashtex-initex), `pdftex.pool` and `fmt/pdflatex.fmt`; VAR=VALUE pairs are extra environment for
that engine (an env-switched variant of one binary). Each document is copied once per engine into
OUT_DIR/work/<engine>/<doc> and compiled natively until its .aux/.toc/.ind files settle (at most 6
passes). Then:
  * REPS native runs per engine, engines interleaved: `perf stat -e instructions:u,cycles:u` when the
    runner's PMU is visible (null otherwise), and the child's user+sys time;
  * with --cg, one valgrind cachegrind run per engine: instructions exactly (--cg nosim), or also a
    simulated cache with Apple-M1-like geometry (--cg sim: D1 128 KiB 8-way, LL 16 MiB 16-way, 128-byte
    lines); the raw file is kept for cgsum.py.
Every run records the PDF's and the log's SHA-256 and the log's memory-usage lines, so the engines'
outputs can be compared byte for byte. One JSON line per run goes to OUT_DIR/runs.jsonl."""
import argparse, hashlib, json, os, re, resource, shutil, subprocess, sys, time

ap = argparse.ArgumentParser()
ap.add_argument('out')
ap.add_argument('--engine', action='append', required=True)
ap.add_argument('--reps', type=int, default=3)
ap.add_argument('--cg', choices=['sim', 'nosim', 'none'], default='none')
ap.add_argument('--timeout', type=float, default=5 * 3600)
ap.add_argument('docs', nargs='+')
a = ap.parse_args()

BASE = {k: v for k, v in os.environ.items() if not k.startswith('FLASHTEX_')}
BASE.update(SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1', max_print_line='10000', error_line='254',
            half_error_line='238')
ENGINES = {}
for spec in a.engine:
    name, rest = spec.split('=', 1)
    parts = rest.split(',')
    extra = dict(p.split('=', 1) for p in parts[1:])
    ENGINES[name] = (os.path.abspath(parts[0]), extra)
os.makedirs(a.out, exist_ok=True)
OUT = open(os.path.join(a.out, 'runs.jsonl'), 'a')


def log(*m):
    print(*m, file=sys.stderr, flush=True)


def cmd(eng, entry):
    d, extra = ENGINES[eng]
    stem = entry[:-4] if entry.endswith('.tex') else entry
    env = dict(BASE)
    env.update(FLASHTEX_POOL=f'{d}/pdftex.pool', FLASHTEX_FORMATS=f'{d}/fmt')
    env.update(extra)
    return [f'{d}/pdftex', '-fmt=pdflatex', '-interaction=batchmode', f'-jobname={stem}',
            r'\pdfsetrandomseed 1\relax\input{' + entry + '}'], env, stem


def perf_ok():
    try:
        p = subprocess.run(['perf', 'stat', '-x,', '-e', 'instructions:u,cycles:u', 'true'],
                           capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.TimeoutExpired):
        return False
    return bool(re.search(r'^\d+,', p.stderr, re.M)) and 'not supported' not in p.stderr


PERF = perf_ok()
log('perf counters available:', PERF)


def h(path):
    try:
        return hashlib.sha256(open(path, 'rb').read()).hexdigest()
    except OSError:
        return None


def memlines(path):
    """The log's memory accounting (its last 'memory usage' and 'words of memory' lines)."""
    try:
        t = open(path, 'rb').read().decode('latin-1')
    except OSError:
        return None
    keep = [l for l in t.splitlines() if re.search(r'words of memory|Memory usage|memory locations', l)]
    return keep[-6:]


def run(eng, d, entry, wrap=()):
    c, env, stem = cmd(eng, entry)
    r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
    t0 = time.perf_counter()
    p = subprocess.run(list(wrap) + c, cwd=d, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                       stdin=subprocess.DEVNULL, timeout=a.timeout)
    wall = time.perf_counter() - t0
    r1 = resource.getrusage(resource.RUSAGE_CHILDREN)
    return dict(rc=p.returncode, wall=round(wall, 3), user=round(r1.ru_utime - r0.ru_utime, 3),
                sys=round(r1.ru_stime - r0.ru_stime, 3), maxrss_kb=r1.ru_maxrss,
                load=os.getloadavg()[0]), p.stderr.decode(errors='replace')


def settle_key(d):
    keys = []
    for root, _, files in os.walk(d):
        for f in sorted(files):
            if f.endswith(('.aux', '.toc', '.ind', '.idx', '.lof', '.lot', '.bbl', '.out', '.nav', '.snm')):
                keys.append((os.path.relpath(os.path.join(root, f), d), h(os.path.join(root, f))))
    return sorted(keys)


def record(r, eng, name, kind, rep, d, stem):
    r.update(doc=name, engine=eng, kind=kind, rep=rep, pdf=h(f'{d}/{stem}.pdf'), log=h(f'{d}/{stem}.log'),
             mem=memlines(f'{d}/{stem}.log'))
    OUT.write(json.dumps(r) + '\n')
    OUT.flush()
    log(json.dumps({k: r[k] for k in r if k != 'mem'}))


for spec in a.docs:
    src, entry = spec.rsplit(':', 1)
    name = os.path.basename(src.rstrip('/'))
    dirs = {}
    for eng in ENGINES:
        d = os.path.join(a.out, 'work', eng, name)
        shutil.rmtree(d, ignore_errors=True)
        shutil.copytree(src, d, symlinks=True)
        stem = cmd(eng, entry)[2]
        prev = None
        for i in range(6):
            r, err = run(eng, d, entry)
            cur = settle_key(d)
            if cur == prev:
                break
            prev = cur
        dirs[eng] = d
        log(f'{name} {eng}: settled after {i + 1} passes rc={r["rc"]} user={r["user"]}')
        record(r, eng, name, 'settle', i, d, stem)
    engines = list(ENGINES)
    for rep in range(a.reps):
        order = engines[rep % len(engines):] + engines[:rep % len(engines)]
        for eng in order:
            stem = cmd(eng, entry)[2]
            pf = os.path.join(a.out, f'perf-{name}-{eng}-{rep}.txt')
            wrap = ['perf', 'stat', '-x,', '-o', pf, '-e', 'instructions:u,cycles:u', '--'] if PERF else []
            r, err = run(eng, dirs[eng], entry, wrap)
            if PERF:
                txt = open(pf).read()
                for ev in ('instructions', 'cycles'):
                    m = re.search(r'^(\d+),[^,]*,' + ev + r':u', txt, re.M)
                    r[ev] = int(m.group(1)) if m else None
            record(r, eng, name, 'native', rep, dirs[eng], stem)
    if a.cg != 'none':
        for eng in engines:
            stem = cmd(eng, entry)[2]
            cgo = os.path.join(a.out, f'cg-{name}-{eng}.out')
            wrap = ['valgrind', '--tool=cachegrind', f'--cachegrind-out-file={cgo}']
            if a.cg == 'sim':
                wrap += ['--cache-sim=yes', '--I1=131072,8,128', '--D1=131072,8,128', '--LL=16777216,16,128']
            else:
                wrap += ['--cache-sim=no']
            r, err = run(eng, dirs[eng], entry, wrap + ['--'])
            m = re.findall(r'I\s+refs:\s+([\d,]+)', err)
            r['cg_ir'] = int(m[-1].replace(',', '')) if m else None
            r['cg_tail'] = err[-1500:]
            record(r, eng, name, 'cg-' + a.cg, 0, dirs[eng], stem)
