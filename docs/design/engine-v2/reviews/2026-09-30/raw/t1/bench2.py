#!/usr/bin/env python3
"""Nightly review T1: full-run cost of the engine vs pdflatex on Linux.

Each document is compiled by every engine in its own directory until the .aux
settles; then REPS interleaved runs per engine under `perf stat` (user-space
instructions and cycles: instructions do not move with machine load) with the
child's rusage (CPU seconds, peak RSS). Medians. Engines: `pdflatex` (TeX Live
2026, the oracle) or NAME = /tmp/t1rev/eng/NAME/{flashtex-initex,pdftex.pool,fmt}.

usage: bench2.py --engines pdflatex,main,p4f --reps 5 --out F.jsonl DOC...
"""
import argparse, json, os, re, shutil, statistics, subprocess, time

PERF = '/nix/store/gyp2si1k1w7jhw8z4xx1bwr2m0pr5445-perf-linux-7.2/bin/perf'
ap = argparse.ArgumentParser()
ap.add_argument('docs', nargs='+')
ap.add_argument('--engines', required=True)
ap.add_argument('--reps', type=int, default=5)
ap.add_argument('--out')
a = ap.parse_args()
ROOT = '/tmp/t1rev'
BASE = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1', TZ='UTC')
for k in list(BASE):
    if k.startswith('FLASHTEX_'):
        del BASE[k]


def cmd_env(e, doc):
    env = dict(BASE)
    if e == 'pdflatex':
        return ['pdflatex', '-interaction=batchmode', doc + '.tex'], env
    d = f'{ROOT}/eng/{e}'
    env.update(FLASHTEX_POOL=f'{d}/pdftex.pool', FLASHTEX_FORMATS=f'{d}/fmt')
    return [f'{d}/flashtex-initex', '-fmt=pdflatex', '-interaction=batchmode', doc + '.tex'], env


def run(e, doc, d, perf=True):
    cmd, env = cmd_env(e, doc)
    pre = [PERF, 'stat', '-x,', '-o', f'{d}/perf.txt', '-e', 'instructions:u,cycles:u', '--'] if perf else []
    t0 = time.perf_counter()
    p = subprocess.Popen(pre + cmd, cwd=d, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    _, status, ru = os.wait4(p.pid, 0)
    rec = dict(wall=time.perf_counter() - t0, cpu=ru.ru_utime + ru.ru_stime, rss_mb=ru.ru_maxrss / 1024,
               load=os.getloadavg()[0], rc=status >> 8)
    if perf:
        for line in open(f'{d}/perf.txt'):
            f = line.strip().split(',')
            if len(f) > 2 and f[0].isdigit():
                rec['instr' if f[2].startswith('instructions') else 'cycles'] = int(f[0])
    return rec


engines = a.engines.split(',')
out = open(a.out, 'a') if a.out else None
for doc in a.docs:
    dirs = {}
    for e in engines:
        d = f'{ROOT}/time/{e}-{doc}'
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(d)
        shutil.copy(f'{ROOT}/docs/{doc}.tex', d)
        prev = None
        for _ in range(4):
            run(e, doc, d, perf=False)
            aux = open(os.path.join(d, doc + '.aux'), 'rb').read()
            if aux == prev:
                break
            prev = aux
        dirs[e] = d
    runs = {e: [] for e in engines}
    for r in range(a.reps):
        for e in engines[r % len(engines):] + engines[:r % len(engines)]:
            runs[e].append(run(e, doc, dirs[e]))
    first = None
    for e in engines:
        log = open(os.path.join(dirs[e], doc + '.log'), 'rb').read().decode('latin1')
        m = re.search(r'Output written on .*?\((\d+) pages?', log, re.S)
        rs = runs[e]
        med = {k: statistics.median(x[k] for x in rs) for k in ('wall', 'cpu', 'rss_mb', 'instr', 'cycles')}
        rec = dict(doc=doc, engine=e, pages=int(m.group(1)) if m else 0, **med,
                   load_min=round(min(x['load'] for x in rs), 1), load_max=round(max(x['load'] for x in rs), 1),
                   rcs=sorted({x['rc'] for x in rs}))
        first = first or rec
        print(f"{doc:11} {e:9} p{rec['pages']:5} instr {rec['instr']/1e9:7.2f}G ({rec['instr']/first['instr']:.3f}) "
              f"cyc {rec['cycles']/1e9:6.2f}G ({rec['cycles']/first['cycles']:.3f}) cpu {rec['cpu']:.3f}s wall {rec['wall']:.3f}s "
              f"rss {rec['rss_mb']:.0f}MB load {rec['load_min']}-{rec['load_max']} rc {rec['rcs']}", flush=True)
        if out:
            out.write(json.dumps(rec) + '\n')
            out.flush()
