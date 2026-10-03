#!/usr/bin/env python3
"""Split the cost of a full-1000 cold run to page 1: format+preamble, the .aux, page 1.
CLI runs of the given engine (wall and child CPU), each variant REPS times, median."""
import os, resource, shutil, subprocess, sys, time

E = sys.argv[1] if len(sys.argv) > 1 else 'pr1300'
REPS = int(sys.argv[2]) if len(sys.argv) > 2 else 5
B = '/tmp/ib-p4pc'
D = f'{B}/split'
os.makedirs(D, exist_ok=True)
src = open(f'{B}/docs/full-1000/main.tex').read()
i = src.index('\\begin{document}') + len('\\begin{document}\n')
pre, body = src[:i], src[i:]
paras = body.split('\n\n')
open(f'{D}/triv.tex', 'w').write('\\documentclass{article}\\begin{document}\\end{document}\n')
open(f'{D}/pre.tex', 'w').write(pre + '\\end{document}\n')
open(f'{D}/p1.tex', 'w').write(pre + '\n\n'.join(paras[:14]) + '\n\\end{document}\n')
AUX = f'{B}/t7-work/full-1000/out/main.aux'
env = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1',
           FLASHTEX_POOL=f'{B}/{E}/pdftex.pool', FLASHTEX_FORMATS=f'{B}/fmt-{E}')


def run(job, aux):
    for ext in ('aux', 'out', 'toc'):
        try:
            os.remove(f'{D}/{job}.{ext}')
        except FileNotFoundError:
            pass
    if aux:
        shutil.copy(AUX, f'{D}/{job}.aux')
    r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
    t = time.perf_counter()
    subprocess.run([f'{B}/{E}/flashtex-initex', '-fmt=pdflatex', '-interaction=batchmode', f'{job}.tex'],
                   cwd=D, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    w = time.perf_counter() - t
    r1 = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = (r1.ru_utime - r0.ru_utime) + (r1.ru_stime - r0.ru_stime)
    return w * 1e3, cpu * 1e3


res = {}
for _ in range(REPS):
    for job, aux in (('triv', False), ('pre', False), ('pre', True), ('p1', False), ('p1', True)):
        res.setdefault((job, aux), []).append(run(job, aux))
for (job, aux), v in res.items():
    ws = sorted(x[0] for x in v)
    cs = sorted(x[1] for x in v)
    print(f'{job:5} {"aux" if aux else "-  "} wall p50 {ws[len(ws)//2]:7.1f} min {ws[0]:7.1f}  cpu p50 {cs[len(cs)//2]:7.1f} min {cs[0]:7.1f}')
print(open(f'{D}/p1.log').read().count('\n'), [l for l in open(f'{D}/p1.log') if 'Output written' in l])
