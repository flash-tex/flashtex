#!/usr/bin/env python3
"""genvol.py [BASE]: the rewritten-output documents of the review of #1300 (its mkvol.py), under
BASE (default $INCR_BENCH_DIR or /tmp/incr-bench): src-vol-closed/vol-closed.tex and
src-vol-open/vol-open.tex. Eight blocks each write a temporary file (`\\jobname-tmp.tex`) and
`\\input` it back, so that the file holds a later instance on disk than a checkpoint between its
writes stands for:

* vol-open: the file is open for output across four paragraphs (checkpoints), then closed and read;
* vol-closed: it is written and closed, then four paragraphs, then read (fancyvrb's VerbatimOut,
  beamer's .vrb, an `answers` file).

soundness.py runs them with `--extra $BASE/src-vol-closed:vol-closed` (gates.sh's `sound-vol`).
"""
import os
import random
import sys

BASE = sys.argv[1] if len(sys.argv) > 1 else os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
rng = random.Random(7)
W = ('lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore '
     'et dolore magna aliqua enim minim veniam quis nostrud exercitation ullamco laboris nisi aliquip '
     'commodo consequat').split()


def para():
    return ' '.join(rng.choice(W) for _ in range(90)).capitalize() + '.\n\n'


def doc(kind, blocks=8):
    s = ['\\documentclass{article}\n\\newwrite\\tmp\n\\begin{document}\n\n']
    for k in range(blocks):
        if kind == 'closed':
            s.append('\\immediate\\openout\\tmp=\\jobname-tmp.tex\n\\immediate\\write\\tmp{Instance %d says %s.}\n'
                     '\\immediate\\closeout\\tmp\n\n' % (k, 'x' * (k + 1)))
            s += [para() for _ in range(4)]
            s.append('\\input{\\jobname-tmp.tex}\n\n')
        else:
            s.append('\\immediate\\openout\\tmp=\\jobname-tmp.tex\n')
            for i in range(4):
                s.append(para())
                s.append('\\immediate\\write\\tmp{Instance %d line %d %s.}\n\n' % (k, i, 'y' * (k + 1)))
            s.append('\\immediate\\closeout\\tmp\n\\input{\\jobname-tmp.tex}\n\n')
        s += [para() for _ in range(3)]
    s.append('\\end{document}\n')
    return ''.join(s)


for kind in ('closed', 'open'):
    d = f'{BASE}/src-vol-{kind}'
    os.makedirs(d, exist_ok=True)
    open(f'{d}/vol-{kind}.tex', 'w').write(doc(kind))
    print(d)
