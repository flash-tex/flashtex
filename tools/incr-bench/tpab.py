#!/usr/bin/env python3
"""tpab.py: cold command-line compiles, engine against engine and against TeX Live's pdftex,
counted in user-space instructions and cycles (`perf stat`), every PDF hashed (lane THROUGHPUT;
adapted from lane L6's l6ab.py). Measurement only.

usage: tpab.py --engines A,B [--docs D,...] [--reps N] --out DIR [--perf-record]
               [--arxiv-cache DIR] [--no-pdftex]

  --engines   engine directories under INCR_BENCH_DIR made by mkeng.sh (binaries and fmt-NAME)
  --docs      arxiv13 (arxiv-2025-01 shard 0/12, the P6-HYPEROPT set), ax:ID (an e-print of
              arxiv-2025-01 fetched into --arxiv-cache), plain-N / full-N (mkdocs.py), fx:NAME
              (fixtures/real-world/NAME)

Each document is settled first by pdftex (at most 5 passes, until its .aux and .toc hold), and
every measured run is one compile of a fresh copy of that settled directory, engines
interleaved. The command line is the parity harness's: -jobname=STEM and
`\\pdfsetrandomseed 1\\relax\\input{ENTRY}`, SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1. Within a
document every engine must give pdftex's PDF, byte for byte (the summary says so).
--perf-record samples (perf record, instructions:u) the first engine and pdftex per document
and keeps their top symbols, per document and summed over all documents.

Writes DIR/runs.jsonl, DIR/summary.md and, when asked, DIR/perf-*.txt."""
import argparse
import collections
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
LIMIT = int(os.environ.get('TPAB_LIMIT', '900'))
SEED = r'\pdfsetrandomseed 1\relax'

ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
ap.add_argument('--engines', required=True)
ap.add_argument('--docs', default='arxiv13')
ap.add_argument('--reps', type=int, default=3)
ap.add_argument('--out', required=True)
ap.add_argument('--perf-record', action='store_true')
ap.add_argument('--arxiv-cache', default=os.path.expanduser('~/.cache/flashtex-agents/throughput/parity'))
ap.add_argument('--no-pdftex', action='store_true')
a = ap.parse_args()

engines = a.engines.split(',')
PDFTEX = shutil.which('pdftex')
allengines = engines + ([] if a.no_pdftex or not PDFTEX else ['pdftex'])
a.out = os.path.abspath(a.out)
os.makedirs(a.out, exist_ok=True)
WORK = tempfile.mkdtemp(prefix='tpab.', dir=IB)
MANIFEST = json.load(open(f'{W}/tools/parity/corpus/arxiv-2025-01.json'))['entries']
ENTRY = {re.sub(r'[^A-Za-z0-9._-]', '_', e['id']): e.get('entry') or 'main.tex' for e in MANIFEST}


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
    if doc.startswith('ax:'):
        i = doc[3:]
        return f'{a.arxiv_cache}/src/arxiv/{i}', ENTRY[i]
    if os.path.isdir(f'{IB}/src-{doc}'):
        return f'{IB}/src-{doc}', f'{doc}.tex'
    sys.exit(f'tpab.py: no document {doc}')


def copy_tree(src, dst):
    shutil.rmtree(dst, ignore_errors=True)
    shutil.copytree(src, dst, symlinks=True)


def env_for(eng):
    e = {k: v for k, v in os.environ.items() if not k.startswith('FLASHTEX_')}
    e.update(SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1', TZ='UTC')
    if eng != 'pdftex':
        e.update(FLASHTEX_POOL=f'{IB}/{eng}/pdftex.pool', FLASHTEX_FORMATS=f'{IB}/fmt-{eng}')
    return e


def load1():
    try:
        return float(open('/proc/loadavg').read().split()[0])
    except (OSError, ValueError, IndexError):
        return None


def h(path):
    try:
        return hashlib.sha256(open(path, 'rb').read()).hexdigest()[:16]
    except OSError:
        return None


def compile_once(eng, d, main, pre=()):
    """One compile in d; returns the run's numbers."""
    stem = os.path.splitext(main)[0]
    exe = PDFTEX if eng == 'pdftex' else f'{IB}/{eng}/flashtex-initex'
    # (perl's alarm survives the exec: an engine that loops is stopped)
    cmd = ['perl', '-e', 'alarm shift @ARGV; exec @ARGV or die', str(LIMIT),
           exe, '-fmt=pdflatex', '-interaction=batchmode', f'-jobname={os.path.basename(stem)}',
           SEED + r'\input{' + main + '}']
    stat = os.path.join(d, '.tpab-stat')
    if pre:
        cmd = list(pre) + cmd
    elif PERF:
        cmd = ['perf', 'stat', '-x,', '-o', stat, '-e', 'instructions:u,cycles:u', '--'] + cmd
    l1 = load1()
    t0 = time.monotonic()
    p = subprocess.Popen(cmd, cwd=d, env=env_for(eng), stdin=subprocess.DEVNULL,
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    _, status, ru = os.wait4(p.pid, 0)
    wall = time.monotonic() - t0
    r = {'wall_s': round(wall, 4), 'utime_s': round(ru.ru_utime, 4), 'stime_s': round(ru.ru_stime, 4),
         'maxrss': ru.ru_maxrss, 'status': os.waitstatus_to_exitcode(status), 'load1': l1,
         'instr': None, 'cycles': None}
    if os.path.exists(stat):
        for line in open(stat).read().splitlines():
            f = line.split(',')
            if len(f) > 2 and f[0].strip().isdigit():
                if f[2].startswith('instructions'):
                    r['instr'] = int(f[0])
                elif f[2].startswith('cycles'):
                    r['cycles'] = int(f[0])
        os.unlink(stat)
    r['pdf'] = h(os.path.join(d, stem + '.pdf'))
    r['log'] = h(os.path.join(d, stem + '.log'))
    return r


docs = []
for x in a.docs.split(','):
    if x == 'arxiv13':
        ids = [re.sub(r'[^A-Za-z0-9._-]', '_', e['id']) for e in MANIFEST]
        docs += ['ax:' + i for i in ids[0::12]]
    elif x:
        docs.append(x)
seeds = {}
for doc in docs:
    src, main = source(doc)
    seed = f'{WORK}/seed-{doc.replace(":", "_")}'
    copy_tree(src, seed)
    stem = os.path.splitext(main)[0]
    prev = None
    settler = 'pdftex' if 'pdftex' in allengines else engines[0]
    for i in range(5):
        compile_once(settler, seed, main, pre=('nice', '-n', '0'))
        cur = (h(f'{seed}/{stem}.aux'), h(f'{seed}/{stem}.toc'))
        if cur == prev:
            break
        prev = cur
    seeds[doc] = (seed, main)
    print(f'settled {doc} after {i + 1} passes', flush=True)

rows = []
out = open(os.path.join(a.out, 'runs.jsonl'), 'w')
for rep in range(a.reps):
    for doc in docs:
        seed, main = seeds[doc]
        order = allengines[rep % len(allengines):] + allengines[:rep % len(allengines)]
        for eng in order:
            d = f'{WORK}/run'
            copy_tree(seed, d)
            r = compile_once(eng, d, main)
            r.update(doc=doc, engine=eng, rep=rep)
            rows.append(r)
            out.write(json.dumps(r) + '\n')
            out.flush()
            print(f"{doc} {eng} rep {rep}: {r['instr'] or 0:,} instr, {r['cycles'] or 0:,} cycles, "
                  f"{r['utime_s']} s user, pdf {r['pdf']}", flush=True)
out.close()

if a.perf_record and PERF:
    tot = {e: collections.Counter() for e in (engines[0], 'pdftex')}
    for doc in docs:
        seed, main = seeds[doc]
        for eng in tot:
            if eng not in allengines:
                continue
            d = f'{WORK}/perf'
            copy_tree(seed, d)
            data = f'{WORK}/perf.data'
            compile_once(eng, d, main,
                         pre=('perf', 'record', '-q', '-e', 'instructions:u', '-c', '200000', '-o', data, '--'))
            # samples per symbol, so documents add up
            rep = subprocess.run(['perf', 'report', '-i', data, '--no-children', '--sort', 'dso,symbol',
                                  '--stdio', '-n', '--percent-limit', '0.05'], capture_output=True, text=True).stdout
            lines = [re.sub(r'\s+$', '', x) for x in rep.splitlines() if '%' in x and not x.startswith('#')]
            tag = 'e0' if eng != 'pdftex' else 'pdftex'
            open(os.path.join(a.out, f'perf-{tag}-{doc.replace(":", "_")}.txt'), 'w').write('\n'.join(lines[:150]) + '\n')
            for x in lines:
                m = re.match(r'\s*[\d.]+%\s+(\d+)\s+(\S+)\s+\[\.\]\s+(.*)$', x)
                if m:
                    tot[eng][(m.group(2), m.group(3))] += int(m.group(1))
            os.unlink(data)
    for eng, c in tot.items():
        n = sum(c.values()) or 1
        tag = 'e0' if eng != 'pdftex' else 'pdftex'
        with open(os.path.join(a.out, f'perf-{tag}-ALL.txt'), 'w') as f:
            f.write(f'# {eng}: {n} samples of 200000 instructions:u over {len(docs)} documents\n')
            for (dso, sym), k in c.most_common(200):
                f.write(f'{100 * k / n:7.2f}% {k:9d} {dso:24s} {sym}\n')


def med(xs):
    xs = [x for x in xs if x is not None]
    return statistics.median(xs) if xs else None


def fmt(x, div, unit):
    return f'{x / div:,.3f} {unit}' if x is not None else '-'


tot = collections.defaultdict(lambda: collections.defaultdict(float))
with open(os.path.join(a.out, 'summary.md'), 'w') as s:
    s.write(f'Host: {platform.node()} ({platform.system()} {platform.machine()}), counters: '
            f"{'perf (user space)' if PERF else 'none'}, reps {a.reps}, medians; delta against the first "
            f'engine; ratio against pdftex.\n\n')
    s.write('| document | engine | instructions | delta | /pdftex | cycles | delta | /pdftex | user CPU | PDF |\n')
    s.write('|---|---|---:|---:|---:|---:|---:|---:|---:|---|\n')
    for doc in docs:
        mine = [r for r in rows if r['doc'] == doc]
        m = {e: {k: med([r[k] for r in mine if r['engine'] == e]) for k in ('instr', 'cycles', 'utime_s')}
             for e in allengines}
        ref = {r['pdf'] for r in mine if r['engine'] == 'pdftex'}
        for eng in allengines:
            rs = [r for r in mine if r['engine'] == eng]
            for k in ('instr', 'cycles'):
                if all(m[e][k] for e in allengines):
                    tot[eng][k] += m[eng][k]

            def delta(k, base):
                return f'{100 * (m[eng][k] / m[base][k] - 1):+.2f} %' if m[eng][k] and m[base][k] else '-'

            def ratio(k):
                return f"{m[eng][k] / m['pdftex'][k]:.3f}x" if 'pdftex' in m and m[eng][k] and m['pdftex'][k] else '-'
            pdfs = {r['pdf'] for r in rs}
            same = ('=pdftex' if ref and pdfs == ref else ','.join(sorted(str(p) for p in pdfs)))
            s.write(f"| {doc} | {eng} | {fmt(m[eng]['instr'], 1e9, 'G')} | {delta('instr', engines[0])} "
                    f"| {ratio('instr')} | {fmt(m[eng]['cycles'], 1e9, 'G')} | {delta('cycles', engines[0])} "
                    f"| {ratio('cycles')} | {fmt(m[eng]['utime_s'], 1, 's')} | {same} |\n")
    s.write('\n**Totals** (sum of medians over the documents every engine has counts for):\n\n')
    s.write('| engine | instructions | delta | /pdftex | cycles | delta | /pdftex |\n|---|---:|---:|---:|---:|---:|---:|\n')
    for eng in allengines:
        t, b, p = tot[eng], tot[engines[0]], tot.get('pdftex', {})
        cell = lambda k: (f"{t[k] / 1e9:,.3f} G | {100 * (t[k] / b[k] - 1):+.2f} % | "
                          + (f"{t[k] / p[k]:.3f}x" if p.get(k) else '-')) if t.get(k) and b.get(k) else '- | - | -'
        s.write(f"| {eng} | {cell('instr')} | {cell('cycles')} |\n")
    bad = [doc for doc in docs if len({r['pdf'] for r in rows if r['doc'] == doc}) != 1]
    s.write('\nPDF identity: ' + ('every engine gave the same bytes as pdftex on every document.\n' if not bad
                                  else f"**differs** on {', '.join(bad)}.\n"))
print(open(os.path.join(a.out, 'summary.md')).read())
shutil.rmtree(WORK, ignore_errors=True)
