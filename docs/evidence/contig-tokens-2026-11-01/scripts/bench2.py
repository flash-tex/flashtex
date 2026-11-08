#!/usr/bin/env python3
"""CONTIG-TOKENS (follow-up, A/A-controlled, random order) cold full-compile counters on a Linux runner (lane CONTIG-TOKENS, DESIGN.md §12 P6).

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
import argparse, random, gzip, hashlib, json, os, re, resource, shutil, subprocess, sys, time

ap = argparse.ArgumentParser()
ap.add_argument('out')
ap.add_argument('--engine', action='append', required=True)
ap.add_argument('--reps', type=int, default=3)
ap.add_argument('--cg', choices=['sim', 'nosim', 'none'], default='none')
ap.add_argument('--seed', default='1')
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
    """One compile. Its rusage comes from wait4 on that child alone, so maxrss_kb is this
    compile's peak (review of #2008, item 8: RUSAGE_CHILDREN's is a running maximum). Under
    perf stat the child is perf, whose peak is the engine's (perf's own is far smaller)."""
    c, env, stem = cmd(eng, entry)
    t0 = time.perf_counter()
    p = subprocess.Popen(list(wrap) + c, cwd=d, env=env, stdout=subprocess.DEVNULL,
                         stderr=subprocess.PIPE, stdin=subprocess.DEVNULL)
    import threading
    err = []
    th = threading.Thread(target=lambda: err.append(p.stderr.read()))
    th.start()
    timer = threading.Timer(a.timeout, p.kill)
    timer.start()
    _, status, ru = os.wait4(p.pid, 0)
    timer.cancel()
    th.join()
    p.returncode = os.waitstatus_to_exitcode(status)
    wall = time.perf_counter() - t0
    return dict(rc=p.returncode, wall=round(wall, 3), user=round(ru.ru_utime, 3),
                sys=round(ru.ru_stime, 3), maxrss_kb=ru.ru_maxrss,
                load=os.getloadavg()[0]), err[0].decode(errors='replace')


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
    # Settled once, by the first engine; every engine's copy then gets one pass of its own, which
    # must leave the settled files as they are (the engines print the same files: checked below).
    first = list(ENGINES)[0]
    d0 = os.path.join(a.out, 'work', first, name)
    shutil.rmtree(d0, ignore_errors=True)
    shutil.copytree(src, d0, symlinks=True)
    prev = None
    for i in range(6):
        r, err = run(first, d0, entry)
        cur = settle_key(d0)
        if cur == prev:
            break
        prev = cur
    log(f'{name} {first}: settled after {i + 1} passes rc={r["rc"]} user={r["user"]}')
    for eng in ENGINES:
        d = os.path.join(a.out, 'work', eng, name)
        if eng != first:
            shutil.rmtree(d, ignore_errors=True)
            shutil.copytree(d0, d, symlinks=True)
        stem = cmd(eng, entry)[2]
        r, err = run(eng, d, entry)
        r['settle_changed'] = settle_key(d) != cur
        dirs[eng] = d
        record(r, eng, name, 'settle', i, d, stem)
    engines = list(ENGINES)
    # Review of #2008 (item 4): a seeded random order per rep, recorded with its predecessor, so
    # no engine always follows the same one; an A/A pair (the same build twice) bounds the bias.
    rng = random.Random(f'{name}-{a.seed}')
    prev_eng = None
    for rep in range(a.reps):
        order = list(engines)
        rng.shuffle(order)
        for pos, eng in enumerate(order):
            stem = cmd(eng, entry)[2]
            pf = os.path.join(a.out, f'perf-{name}-{eng}-{rep}.txt')
            wrap = ['perf', 'stat', '-x,', '-o', pf, '-e', 'instructions:u,cycles:u', '--'] if PERF else []
            r, err = run(eng, dirs[eng], entry, wrap)
            if PERF:
                txt = open(pf).read()
                for ev in ('instructions', 'cycles'):
                    m = re.search(r'^(\d+),[^,]*,' + ev + r':u', txt, re.M)
                    r[ev] = int(m.group(1)) if m else None
            r.update(pos=pos, after=prev_eng)
            prev_eng = eng
            record(r, eng, name, 'native', rep, dirs[eng], stem)
    # a log that is not the first engine's is kept (gzipped), with the first engine's, to be diffed
    logs = {eng: f"{dirs[eng]}/{cmd(eng, entry)[2]}.log" for eng in engines}
    first = h(logs[engines[0]])
    for eng in engines[1:]:
        if h(logs[eng]) != first:
            os.makedirs(os.path.join(a.out, 'logs'), exist_ok=True)
            for e in (engines[0], eng):
                with open(logs[e], 'rb') as src, gzip.open(os.path.join(a.out, 'logs', f'{name}-{e}.log.gz'), 'wb') as dst:
                    shutil.copyfileobj(src, dst)
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
