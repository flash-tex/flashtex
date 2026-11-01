#!/usr/bin/env python3
"""rt_end.py ENGDIR DOC.tex --fmt FMTDIR [--at end|WORD] [--keys N] [--env K=V ...] [--verify]:
Classic iserve host, cold settle, then N keystrokes ('x') typed at the start of the
\\end{document} line (or after WORD), printing each compile's report. --verify compiles each
state from scratch with the CLI engine (until the files repeat, at most 5 runs) and compares
the PDF, log, aux and terminal-free outputs byte for byte."""
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import tempfile

ap = argparse.ArgumentParser()
ap.add_argument('eng')
ap.add_argument('doc')
ap.add_argument('--at', default='end')
ap.add_argument('--keys', type=int, default=2)
ap.add_argument('--env', action='append', default=[])
ap.add_argument('--fmt', required=True)
ap.add_argument('--verify', action='store_true')
ap.add_argument('--then-chunk', action='store_true', help='then edit chunk.tex')
ap.add_argument('--callgrind', metavar='OUT', help='the host under callgrind, its total instructions to OUT')
a = ap.parse_args()
work = tempfile.mkdtemp(prefix='rt-')
name = os.path.splitext(os.path.basename(a.doc))[0]
shutil.copy(a.doc, os.path.join(work, name + '.tex'))
for extra in ('chunk.tex',):
    q = os.path.join(os.path.dirname(os.path.abspath(a.doc)), extra)
    if os.path.exists(q):
        shutil.copy(q, work)
env = dict(os.environ, FLASHTEX_POOL=f'{a.eng}/pdftex.pool', FLASHTEX_FORMATS=a.fmt,
           SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1',
           FLASHTEX_PIN_CLOCK='1700000000.250000', TZ='UTC')
import sys  # noqa: E402
sys.path.insert(0, os.path.expanduser('~/code/flashtex-readtrack/tools/parity'))
from capture import split_accounting  # noqa: E402
for kv in a.env:
    k, v = kv.split('=', 1)
    env[k] = v
pre_cmd = (['valgrind', '--tool=callgrind', '--callgrind-out-file=' + a.callgrind] if a.callgrind else [])
host = subprocess.Popen(pre_cmd + [f'{a.eng}/flashtex-host', 'iserve', '--', '-fmt=pdflatex',
                         '-interaction=nonstopmode', name + '.tex'],
                        cwd=work, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                        text=True, bufsize=1)


def cmd(c):
    host.stdin.write(c + '\n')
    host.stdin.flush()
    return json.loads(host.stdout.readline())


KEYS = ('mode', 'pages', 'restart_pages', 'converged_at', 'rerun_from', 'dormant', 'rerun_pages',
        'passes', 'edited')
bad = 0


def files(d):
    out = {}
    for n in sorted(os.listdir(d)):
        if n.endswith(('.pdf', '.log', '.aux', '.out', '.toc')):
            out[n] = open(os.path.join(d, n), 'rb').read()
    return out


def scratch(pre):
    """The CLI engine from the directory as it was before the compile (`pre`, the edit in),
    rerun while the files change, as incr_bench.py's reference runs."""
    d = tempfile.mkdtemp(prefix='rt-ref-')
    for n, b in pre.items():
        open(os.path.join(d, n), 'wb').write(b)
    cenv = {k: v for k, v in env.items() if not k.startswith('FLASHTEX_INCR')}
    cenv['FLASHTEX_PREVIEW'] = '1'
    seen = []
    for _ in range(5):
        before = files(d)
        seen.append({k: v for k, v in before.items() if not k.endswith(('.pdf', '.log'))})
        subprocess.run(['pdftex', '-fmt=pdflatex', '-interaction=nonstopmode', name + '.tex'],
                       executable=f'{a.eng}/pdftex', cwd=d, env=cenv, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL)
        f = {k: v for k, v in files(d).items() if not k.endswith(('.pdf', '.log'))}
        if f in seen:
            break
    r = files(d)
    shutil.rmtree(d)
    return r


def snapshot():
    return {n: open(os.path.join(work, n), 'rb').read() for n in os.listdir(work)
            if os.path.isfile(os.path.join(work, n))}


def verify(tag, pre):
    global bad
    mine, ref = files(work), scratch(pre)
    diff = []
    for n in sorted(set(mine) | set(ref)):
        x, y = mine.get(n), ref.get(n)
        if x == y:
            continue
        if n.endswith('.log') and x is not None and y is not None:
            if split_accounting(x.decode('latin-1'))[0] == split_accounting(y.decode('latin-1'))[0]:
                continue
        diff.append(n)
    if diff:
        bad += 1
    print(f'  verify {tag}: {"DIFFERS " + str(diff) if diff else "equal"}',
          {n: hashlib.md5(v).hexdigest()[:8] for n, v in mine.items()}, flush=True)


def show(tag, r):
    print(tag, {k: r[k] for k in KEYS if k in r}, flush=True)


for i in range(6):
    pre = snapshot()
    r = cmd('compile')
    show('settle', r)
    if r['mode'] == 'unchanged':
        break
p = os.path.join(work, name + '.tex')
src = open(p, 'rb').read()
if a.at == 'end':
    anchor = src.index(b'\\end{document}')
elif a.at == 'tail':
    anchor = src.index(b'\\tail\n') + 6
else:
    anchor = src.index(a.at.encode()) + len(a.at)
cur = src
for i in range(a.keys):
    cur = cur[:anchor + i] + b'x' + cur[anchor + i:]
    open(p, 'wb').write(cur)
    pre = snapshot()
    show(f'key{i + 1}', cmd('compile'))
    if a.verify:
        verify(f'key{i + 1}', pre)
if a.then_chunk:
    q = os.path.join(work, 'chunk.tex')
    open(q, 'w').write(open(q).read().replace('chunk', 'piece'))
    pre = snapshot()
    show('chunk', cmd('compile'))
    if a.verify:
        verify('chunk', pre)
host.stdin.close()
host.wait()
shutil.rmtree(work)
print('BAD', bad)
