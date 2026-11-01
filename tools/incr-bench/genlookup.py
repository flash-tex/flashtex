#!/usr/bin/env python3
"""genlookup.py [BASE]: the file-existence document of #1502, under BASE (default $INCR_BENCH_DIR or
/tmp/incr-bench): src-lookup/lookup.tex. Five probe files, each looked up more than once, early and
late, by every kind of lookup a page makes (`\\IfFileExists` with `\\input`, `\\pdffilesize`,
`\\pdffilemoddate`, `\\pdfmdfivesum file`, `\\openin`); the sweep
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


# Each probe file is looked up more than once, early and late (#1502's review: a lookup the journal
# kept only at its first occurrence), by every kind of lookup a page makes. `early`: a grouped
# `\pdffilesize` whose answer leaves no trace (only the later lookups decide the output).
KINDS = {
    'early': '\\begingroup\\edef\\x{\\pdffilesize{%(n)s}}\\endgroup',
    'iffileexists': '\\IfFileExists{%(n)s}{\\input{%(n)s}}{[no probe %(k)d]}',
    'size': '[size \\pdffilesize{%(n)s}]',
    'mtime': '[date \\pdffilemoddate{%(n)s}]',
    'md5': '[md5 \\pdfmdfivesum file{%(n)s}]',
    'openin': '\\openin15=%(n)s \\ifeof15 [no probe %(k)d]\\else[probe %(k)d]\\closein15 \\fi',
}
# paragraph -> (probe, kind)
AT = {3: (1, 'early'), 6: (2, 'early'), 9: (3, 'early'), 12: (4, 'early'), 15: (5, 'early'),
      20: (1, 'iffileexists'), 40: (2, 'md5'), 60: (3, 'openin'), 75: (4, 'mtime'), 90: (5, 'size'),
      100: (1, 'size'), 120: (2, 'iffileexists'), 140: (5, 'openin')}


def doc(n=150):
    s = ['\\documentclass{article}\n\\begin{document}\n\n']
    for i in range(n):
        s.append(para())
        if i in AT:
            k, kind = AT[i]
            s.append(KINDS[kind] % dict(n=NAMES[k - 1], k=k) + '\n\n')
    s.append('\\end{document}\n')
    return ''.join(s)


d = f'{BASE}/src-lookup'
os.makedirs(d, exist_ok=True)
open(f'{d}/lookup.tex', 'w').write(doc())
print(d)
