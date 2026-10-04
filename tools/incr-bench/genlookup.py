#!/usr/bin/env python3
"""genlookup.py [BASE]: the file-existence document of #1502, under BASE (default $INCR_BENCH_DIR or
/tmp/incr-bench): src-lookup/lookup.tex. Four pages test for a file with `\\IfFileExists` (and
`\\input` it when it is there) and one more asks kpathsea through `\\pdffilesize`; the sweep
creates and deletes those files between compiles (`soundness.py --toggle-files`, with the names
`genlookup.py --names` prints), so that a lookup the old run made later answers differently now.

soundness.py runs it with `--extra $BASE/src-lookup:lookup --toggle-files "$(genlookup.py --names)"`
(gates.sh's `sound-lookup`).
"""
import os
import random
import sys

NAMES = [f'lookup-probe-{k}.tex' for k in range(1, 6)]
if sys.argv[1:] == ['--names']:
    print(','.join(NAMES))
    sys.exit(0)
BASE = sys.argv[1] if len(sys.argv) > 1 else os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
rng = random.Random(1502)
W = ('lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore '
     'et dolore magna aliqua enim minim veniam quis nostrud exercitation ullamco laboris nisi aliquip '
     'commodo consequat').split()


def para():
    return ' '.join(rng.choice(W) for _ in range(90)).capitalize() + '.\n\n'


def doc(n=150):
    s = ['\\documentclass{article}\n\\begin{document}\n\n']
    for i in range(n):
        s.append(para())
        if i in (20, 60, 100, 140):
            k = (20, 60, 100, 140).index(i) + 1
            s.append('\\IfFileExists{%s}{\\input{%s}}{[no probe %d]}\n\n' % (NAMES[k - 1], NAMES[k - 1], k))
        if i == 120:
            s.append('[size \\pdffilesize{%s}]\n\n' % NAMES[4])
    s.append('\\end{document}\n')
    return ''.join(s)


d = f'{BASE}/src-lookup'
os.makedirs(d, exist_ok=True)
open(f'{d}/lookup.tex', 'w').write(doc())
print(d)
