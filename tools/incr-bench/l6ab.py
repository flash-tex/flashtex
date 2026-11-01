#!/usr/bin/env python3
"""l6ab.py: cold command-line compiles of benchmark documents, engine against engine and
variant against variant, counted in instructions and cycles (lane L6, DESIGN.md §5.6 item 1:
"profile first; every item needs a measured win"; docs/design/engine-v2/L6.md §4).

usage: l6ab.py --engines A,B [--docs D,...] [--variant NAME=ENV ...] [--reps N] --out DIR
               [--perf-record] [--macro-profile DOC,...]

  --engines   engine directories under INCR_BENCH_DIR made by mkeng.sh (binaries and fmt-NAME)
  --docs      plain-N / full-N (mkdocs.py's src-DOC), beamer-5 / beamer-30 (docs/DOC), fx:NAME
              (fixtures/real-world/NAME), and books under docs/ (mkbook.py: infdesc). Default:
              plain-100,full-100,beamer-30,fx:beamer-madrid,fx:framed-tikz-theorems
  --variant   'NAME=VAR=VALUE [VAR=VALUE ...]': an environment the engine runs in; repeatable
              (default: one variant, base, with nothing set)
  --reps      measured compiles per (engine, variant, document), interleaved (default 3)

Each document is settled first (engine A, the first variant, three passes, so its .aux and
the other auxiliary files hold), and every measured run is one compile of a fresh copy of
that settled directory: what a pdflatex rerun does. Linux counts user-space instructions and
cycles with `perf stat`; macOS with `/usr/bin/time -l` where the machine has the counters
(GitHub's macOS VMs have none). The child's CPU time comes from wait4 either way.

Every run's PDF is hashed: within a document every engine and variant must give the same
bytes (the summary says so, or names the ones that differ). --perf-record samples the first
engine's first variant per document (perf record, instructions:u) and keeps its top symbols;
--macro-profile runs FLASHTEX_MACRO_PROFILE once per engine and variant on those documents.

Writes DIR/runs.jsonl, DIR/summary.md and, when asked, DIR/perf-DOC.txt, DIR/macro-*.tsv."""
import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

S = os.path.dirname(os.path.abspath(__file__))
W = os.path.dirname(os.path.dirname(S))
IB = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
LINUX = platform.system() == 'Linux'
LIMIT = int(os.environ.get('L6AB_LIMIT', '900'))

ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
ap.add_argument('--engines', required=True)
ap.add_argument('--docs', default='plain-100,full-100,beamer-30,fx:beamer-madrid,fx:framed-tikz-theorems')
ap.add_argument('--variant', action='append', default=[])
ap.add_argument('--reps', type=int, default=3)
ap.add_argument('--out', required=True)
ap.add_argument('--perf-record', action='store_true')
ap.add_argument('--macro-profile', default='')
a = ap.parse_args()

engines = a.engines.split(',')
variants = []
for v in a.variant or ['base=']:
    name, _, env = v.partition('=')
    kv = dict(x.split('=', 1) for x in env.split())
    variants.append((name, kv))
a.out = os.path.abspath(a.out)
os.makedirs(a.out, exist_ok=True)
WORK = tempfile.mkdtemp(prefix='l6ab.', dir=IB)


def perf_ok():
    if not LINUX or not shutil.which('perf'):
        return False
    r = subprocess.run(['perf', 'stat', '-x,', '-e', 'instructions:u', 'true'], capture_output=True, text=True)
    return r.returncode == 0 and 'not supported' not in r.stderr and '<not counted>' not in r.stderr


PERF = perf_ok()


def source(doc):
    """(directory, main file) of a document's sources."""
    if doc.startswith('fx:'):
        return f'{W}/fixtures/real-world/{doc[3:]}', 'main.tex'
    if os.path.isdir(f'{IB}/src-{doc}'):
        return f'{IB}/src-{doc}', f'{doc}.tex'
    if os.path.isdir(f'{IB}/docs/{doc}'):
        return f'{IB}/docs/{doc}', 'main.tex'
    sys.exit(f'l6ab.py: no document {doc} (mkdocs.py, mkbook.py)')


def copy_tree(src, dst):
    shutil.rmtree(dst, ignore_errors=True)
    shutil.copytree(src, dst, symlinks=True)


def env_for(eng, extra):
    e = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1', TZ='UTC',
             FLASHTEX_POOL=f'{IB}/{eng}/pdftex.pool', FLASHTEX_FORMATS=f'{IB}/fmt-{eng}')
    e.update(extra)
    return e


def load1():
    try:
        if LINUX:
            return float(open('/proc/loadavg').read().split()[0])
        return float(subprocess.run(['sysctl', '-n', 'vm.loadavg'], capture_output=True,
                                    text=True).stdout.split()[1])
    except (OSError, ValueError, IndexError):
        return None


def compile_once(eng, extra, d, main, pre=()):
    """One compile in d; returns the run's numbers."""
    # (perl's alarm survives the exec: an engine that loops is stopped)
    cmd = ['perl', '-e', 'alarm shift @ARGV; exec @ARGV or die', str(LIMIT),
           f'{IB}/{eng}/flashtex-initex', '-fmt=pdflatex', '-interaction=batchmode', main]
    stat = os.path.join(d, '.l6ab-stat')
    if pre:
        cmd = list(pre) + cmd
    elif PERF:
        cmd = ['perf', 'stat', '-x,', '-o', stat, '-e', 'instructions:u,cycles:u', '--'] + cmd
    elif not LINUX and os.path.exists('/usr/bin/time'):
        cmd = ['/usr/bin/time', '-l', '-o', stat] + cmd
    l1 = load1()
    t0 = time.monotonic()
    p = subprocess.Popen(cmd, cwd=d, env=env_for(eng, extra), stdin=subprocess.DEVNULL,
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    _, status, ru = os.wait4(p.pid, 0)
    p.returncode = os.waitstatus_to_exitcode(status)
    wall = time.monotonic() - t0
    r = {'wall_s': round(wall, 4), 'utime_s': round(ru.ru_utime, 4), 'stime_s': round(ru.ru_stime, 4),
         'maxrss': ru.ru_maxrss, 'status': p.returncode, 'load1': l1, 'instr': None, 'cycles': None}
    if os.path.exists(stat):
        t = open(stat).read()
        if PERF:
            for line in t.splitlines():
                f = line.split(',')
                if len(f) > 2 and f[0].strip().isdigit():
                    if f[2].startswith('instructions'):
                        r['instr'] = int(f[0])
                    elif f[2].startswith('cycles'):
                        r['cycles'] = int(f[0])
        else:
            m = re.search(r'(\d+)\s+instructions retired', t)
            r['instr'] = int(m.group(1)) if m and int(m.group(1)) else None
            m = re.search(r'(\d+)\s+cycles elapsed', t)
            r['cycles'] = int(m.group(1)) if m and int(m.group(1)) else None
        os.unlink(stat)
    pdf = os.path.join(d, os.path.splitext(main)[0] + '.pdf')
    r['pdf'] = hashlib.sha256(open(pdf, 'rb').read()).hexdigest()[:16] if os.path.exists(pdf) else None
    return r


docs = [x for x in a.docs.split(',') if x]
seeds = {}
for doc in docs:
    src, main = source(doc)
    seed = f'{WORK}/seed-{doc.replace(":", "_")}'
    copy_tree(src, seed)
    for _ in range(3):
        compile_once(engines[0], variants[0][1], seed, main, pre=('nice', '-n', '0'))
    seeds[doc] = (seed, main)
    print(f'settled {doc}', flush=True)

rows = []
out = open(os.path.join(a.out, 'runs.jsonl'), 'w')
for rep in range(a.reps):
    for doc in docs:
        seed, main = seeds[doc]
        for eng in engines:
            for vname, venv in variants:
                d = f'{WORK}/run'
                copy_tree(seed, d)
                r = compile_once(eng, venv, d, main)
                r.update(doc=doc, engine=eng, variant=vname, rep=rep)
                rows.append(r)
                out.write(json.dumps(r) + '\n')
                out.flush()
                print(f"{doc} {eng} {vname} rep {rep}: {r['instr'] or 0:,} instr, {r['cycles'] or 0:,} cycles, "
                      f"{r['utime_s']} s user, pdf {r['pdf']}", flush=True)
out.close()

if a.perf_record and PERF:
    for doc in docs:
        seed, main = seeds[doc]
        d = f'{WORK}/perf'
        copy_tree(seed, d)
        data = f'{WORK}/perf.data'
        compile_once(engines[0], variants[0][1], d, main,
                     pre=('perf', 'record', '-q', '-e', 'instructions:u', '-c', '200000', '-o', data, '--'))
        rep = subprocess.run(['perf', 'report', '-i', data, '--no-children', '--sort', 'symbol', '--stdio',
                              '-F', 'overhead,sym'], capture_output=True, text=True).stdout
        lines = [x for x in rep.splitlines() if x.strip() and not x.startswith('#')][:80]
        open(os.path.join(a.out, f'perf-{doc.replace(":", "_")}.txt'), 'w').write('\n'.join(lines) + '\n')
        os.unlink(data)

for doc in [x for x in a.macro_profile.split(',') if x]:
    seed, main = seeds[doc]
    for eng in engines:
        for vname, venv in variants:
            d = f'{WORK}/macro'
            copy_tree(seed, d)
            f = os.path.join(a.out, f'macro-{doc.replace(":", "_")}-{eng}-{vname}.tsv')
            compile_once(eng, dict(venv, FLASHTEX_MACRO_PROFILE=f), d, main, pre=('nice', '-n', '0'))


def med(xs):
    xs = [x for x in xs if x is not None]
    return statistics.median(xs) if xs else None


def fmt(x, div, unit):
    return f'{x / div:,.3f} {unit}' if x is not None else '-'


with open(os.path.join(a.out, 'summary.md'), 'w') as s:
    s.write(f'Host: {platform.node()} ({platform.system()} {platform.machine()}), counters: '
            f"{'perf' if PERF else ('/usr/bin/time -l' if not LINUX else 'none')}, reps {a.reps}, "
            f'medians; Δ against the first engine and first variant.\n\n')
    s.write('| document | engine | variant | instructions | Δ | cycles | Δ | user CPU | Δ | PDF |\n')
    s.write('|---|---|---|---:|---:|---:|---:|---:|---:|---|\n')
    for doc in docs:
        mine = [r for r in rows if r['doc'] == doc]
        base = [r for r in mine if r['engine'] == engines[0] and r['variant'] == variants[0][0]]
        b = {k: med([r[k] for r in base]) for k in ('instr', 'cycles', 'utime_s')}
        pdfs = {r['pdf'] for r in mine}
        for eng in engines:
            for vname, _ in variants:
                rs = [r for r in mine if r['engine'] == eng and r['variant'] == vname]
                m = {k: med([r[k] for r in rs]) for k in ('instr', 'cycles', 'utime_s')}

                def delta(k):
                    return f'{100 * (m[k] / b[k] - 1):+.2f} %' if m[k] and b[k] else '-'
                same = 'same' if len(pdfs) == 1 else ','.join(sorted({str(r['pdf']) for r in rs}))
                s.write(f"| {doc} | {eng} | {vname} | {fmt(m['instr'], 1e9, 'G')} | {delta('instr')} "
                        f"| {fmt(m['cycles'], 1e9, 'G')} | {delta('cycles')} | {fmt(m['utime_s'], 1, 's')} "
                        f"| {delta('utime_s')} | {same} |\n")
    bad = [doc for doc in docs if len({r['pdf'] for r in rows if r['doc'] == doc}) != 1]
    s.write('\nPDF identity: ' + ('every engine and variant gave the same bytes on every document.\n' if not bad
                                  else f"**differs** on {', '.join(bad)}.\n"))
print(open(os.path.join(a.out, 'summary.md')).read())
shutil.rmtree(WORK, ignore_errors=True)
sys.exit(1 if any(len({r['pdf'] for r in rows if r['doc'] == doc}) != 1 for doc in docs) else 0)
