#!/usr/bin/env python3
"""A/B timing of full pdflatex-style runs.

usage: timeit.py REPS DOC BINDIR:FMTDIR [BINDIR:FMTDIR ...]
Runs are interleaved (A B A B ...) so load drift hits both equally.
Prints per-arm wall and user+sys CPU (min / median) and the median ratio vs
the first arm.
"""
import os, shutil, statistics, subprocess, sys, tempfile, time

reps = int(sys.argv[1]); doc = sys.argv[2]; arms = [a.split(':') for a in sys.argv[3:]]
wall = {i: [] for i in range(len(arms))}
cpu = {i: [] for i in range(len(arms))}
work = {}
for i, (b, f) in enumerate(arms):
    w = tempfile.mkdtemp(prefix=f'arm{i}.', dir='/tmp/p4l1')
    for n in os.listdir('/tmp/p4l1/docs'):
        shutil.copy(os.path.join('/tmp/p4l1/docs', n), w)
    work[i] = w
for r in range(reps):
    for i, (b, f) in enumerate(arms):
        env = dict(os.environ, FLASHTEX_POOL=f'{b}/pdftex.pool', FLASHTEX_FORMATS=f,
                   SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1')
        t = time.perf_counter()
        p = subprocess.Popen([f'{b}/pdftex', '-fmt=pdflatex', '-interaction=batchmode', f'{doc}.tex'],
                             cwd=work[i], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        _, _, ru = os.wait4(p.pid, 0)
        wall[i].append(time.perf_counter() - t)
        cpu[i].append(ru.ru_utime + ru.ru_stime)
bw = statistics.median(wall[0]); bc = statistics.median(cpu[0]); bcm = min(cpu[0])
for i, (b, f) in enumerate(arms):
    mw = statistics.median(wall[i]); mc = statistics.median(cpu[i])
    print(f'{os.path.basename(b):>8}: wall min {min(wall[i]):.4f} med {mw:.4f} ({mw/bw:.4f})'
          f' | cpu min {min(cpu[i]):.4f} ({min(cpu[i])/bcm:.4f}) med {mc:.4f} ({mc/bc:.4f})')
for w in work.values():
    shutil.rmtree(w)
