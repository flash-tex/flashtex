#!/usr/bin/env python3
"""Paired A/B: each rep starts every arm at the same moment (one process per
arm, on separate cores), so background load hits all arms alike; the ratio is
taken per rep, then the median over reps. Reports cycles (from /usr/bin/time -l),
instructions and user+sys CPU.

usage: timeit3.py REPS DOC NAME:BINDIR:FMTDIR [...]   (first arm = baseline)
"""
import json, os, re, shutil, statistics, subprocess, sys, tempfile

reps = int(sys.argv[1]); doc = sys.argv[2]; arms = [a.split(':') for a in sys.argv[3:]]
work = {}
for i, a in enumerate(arms):
    w = tempfile.mkdtemp(prefix=f't3arm{i}.', dir='/tmp/p4l1')
    for n in os.listdir('/tmp/p4l1/docs'):
        shutil.copy(os.path.join('/tmp/p4l1/docs', n), w)
    work[i] = w
rows = []
for r in range(reps):
    procs = []
    order = list(range(len(arms)))
    if r % 2:
        order.reverse()
    for i in order:
        name, b, f = arms[i]
        env = dict(os.environ, FLASHTEX_POOL=f'{b}/pdftex.pool', FLASHTEX_FORMATS=f,
                   SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1')
        procs.append((i, subprocess.Popen(['/usr/bin/time', '-l', f'{b}/pdftex', '-fmt=pdflatex',
                                           '-interaction=batchmode', f'{doc}.tex'], cwd=work[i], env=env,
                                          stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)))
    row = {}
    for i, p in procs:
        e = p.communicate()[1].decode()
        rt = re.search(r'([\d.]+) real\s+([\d.]+) user\s+([\d.]+) sys', e)
        row[i] = {'cpu': float(rt.group(2)) + float(rt.group(3)),
                  'ins': int(re.search(r'(\d+)\s+instructions retired', e).group(1)),
                  'cyc': int(re.search(r'(\d+)\s+cycles elapsed', e).group(1))}
    rows.append(row)
for i, (name, b, f) in enumerate(arms):
    cyc = [row[i]['cyc'] / row[0]['cyc'] for row in rows]
    ins = [row[i]['ins'] / row[0]['ins'] for row in rows]
    cpu = [row[i]['cpu'] / max(row[0]['cpu'], 1e-9) for row in rows]
    print(json.dumps({'arm': name, 'doc': doc, 'reps': reps,
                      'cyc_med': statistics.median(row[i]['cyc'] for row in rows),
                      'cyc_ratio_med': round(statistics.median(cyc), 4),
                      'cyc_ratio_q1': round(statistics.quantiles(cyc, n=4)[0], 4) if reps >= 4 else None,
                      'cyc_ratio_q3': round(statistics.quantiles(cyc, n=4)[2], 4) if reps >= 4 else None,
                      'ins_ratio_med': round(statistics.median(ins), 4),
                      'cpu_ratio_med': round(statistics.median(cpu), 4)}))
for w in work.values():
    shutil.rmtree(w)
