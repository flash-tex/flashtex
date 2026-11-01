#!/usr/bin/env python3
"""barrier_test.py ENGINE: a document writes a file (from a macro the edit
changes) and \\input's it 40 paragraphs later; an edit of the macro must not
converge past the read. Compares the incremental compile with a scratch CLI
run of the same sources."""
import json
import os
import shutil
import subprocess
import sys
import tempfile

E = f'/tmp/p4l5/{sys.argv[1]}'
env = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1',
           FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'/tmp/p4l5/fmt-{sys.argv[1]}',
           FLASHTEX_PIN_CLOCK='1700000000.250000', TZ='UTC')
w = tempfile.mkdtemp(prefix='barrier.', dir='/tmp/p4l5')
shutil.copy('/tmp/p4l5/src-wr/wr.tex', w)
cmdline = ['-fmt=pdflatex', '-interaction=batchmode', 'wr.tex']
h = subprocess.Popen([f'{E}/flashtex-host', 'iserve', '--'] + cmdline, cwd=w, env=env,
                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)


def cmd(c):
    h.stdin.write(c + '\n')
    h.stdin.flush()
    return json.loads(h.stdout.readline())


for _ in range(5):
    if cmd('compile')['mode'] == 'unchanged':
        break
src = open(f'{w}/wr.tex').read()
ok = True
for new_word in ('alphawore', 'alphaword', 'betaword'):
    open(f'{w}/wr.tex', 'w').write(src.replace('alphaword', new_word))
    r = cmd('compile')
    d = tempfile.mkdtemp(prefix='barrier-ref.', dir='/tmp/p4l5')
    for n in os.listdir(w):
        if os.path.isfile(os.path.join(w, n)) and not n.endswith('.pdf') and not n.endswith('.log'):
            shutil.copy(os.path.join(w, n), d)
    # the reference sees the directory as the compile did: before it, the
    # .aux/.tmp the previous compile left (the host's own outputs are newer)
    subprocess.run([f'{E}/pdftex'] + cmdline, cwd=d, env=dict(env, FLASHTEX_PREVIEW='1'),
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    same = open(f'{w}/wr.pdf', 'rb').read() == open(f'{d}/wr.pdf', 'rb').read()
    ok &= same
    print(new_word, r['mode'], 'restart', r['restart_pages'], 'converged_at', r['converged_at'],
          'diffs', r['diffs'][:2], 'PDF', 'identical' if same else 'DIFFERS')
    shutil.rmtree(d)
cmd('quit')
shutil.rmtree(w)
print('PASS' if ok else 'FAIL')
