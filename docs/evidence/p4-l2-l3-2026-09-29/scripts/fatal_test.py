#!/usr/bin/env python3
"""fatal_test.py ENGINE FIXTURE_DIR OLD NEW: settle, apply OLD->NEW (an edit
that makes the run fail), compile, revert, compile; print each report."""
import json
import os
import shutil
import subprocess
import sys
import tempfile

E = f'/tmp/p4l2/{sys.argv[1]}'
src, old, new = sys.argv[2], sys.argv[3], sys.argv[4]
env = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1',
           FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'/tmp/p4l2/fmt-{sys.argv[1]}',
           FLASHTEX_PIN_CLOCK='1700000000.250000', TZ='UTC')
w = tempfile.mkdtemp(prefix='fatal.', dir='/tmp/p4l2')
for n in os.listdir(src):
    if os.path.isfile(os.path.join(src, n)):
        shutil.copy(os.path.join(src, n), w)
h = subprocess.Popen([f'{E}/flashtex-host', 'iserve', '--', '-fmt=pdflatex', '-interaction=batchmode', 'main.tex'],
                     cwd=w, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)


def cmd(c):
    h.stdin.write(c + '\n')
    h.stdin.flush()
    return h.stdout.readline().strip()


for _ in range(4):
    r = cmd('compile')
    if '"unchanged"' in r:
        break
s = open(f'{w}/main.tex').read()
assert old in s
open(f'{w}/main.tex', 'w').write(s.replace(old, new, 1))
print('edit  :', cmd('compile')[:300], 'pdf exists:', os.path.exists(f'{w}/main.pdf'))
open(f'{w}/main.tex', 'w').write(s)
print('revert:', cmd('compile')[:300], 'pdf exists:', os.path.exists(f'{w}/main.pdf'))
h.stdin.write('quit\n')
h.stdin.flush()
shutil.rmtree(w)
