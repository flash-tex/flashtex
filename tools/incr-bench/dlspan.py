#!/usr/bin/env python3
"""dlspan.py ENGINE DOC [--edits N] [--seed S] [--from FRAC]: the display list's SOURCE SPANS of an
incremental host against a from-scratch host -- the soundness of the side table (changes/displaylist.ch,
`dl_side`), which the other sweeps cannot see: nothing in the PDF, log, aux or terminal depends on it.
Written by the independent review of PR #1300 (P4-MEMORY), which found with it that the convergence
jump did not adopt the side table; kept here as the `span` gate (gates.sh).

A persistent `flashtex-host --socket` (A) is edited N times (a letter or twelve words inserted into a
random paragraph in the part of the document from FRAC on, accumulated). After every compile a fresh
dl3-client connection fetches every page of A (a new peer is sent the whole page cache), and a fresh
host (R) compiles the same source from scratch. Per page, each glyph's source (file, line) and
column are compared. Prints one JSON line per edit and a summary; exit status 1 if any glyph's
source or any page's glyphs differ. Two hosts at a time. Needs dl3-client and dl3-dump next to the
engine (tools/incr-bench/mkeng.sh copies them).
"""
import argparse
import json
import os
import random
import re
import shutil
import signal
import subprocess
import sys
import time

ap = argparse.ArgumentParser()
ap.add_argument('engine')
ap.add_argument('doc')
ap.add_argument('--edits', type=int, default=20)
ap.add_argument('--seed', type=int, default=1)
ap.add_argument('--from', dest='frac', type=float, default=0.6)
ap.add_argument('--host-args', default='')
ap.add_argument('--timeout', type=int, default=3000)
a = ap.parse_args()
signal.alarm(a.timeout)
BASE = os.environ['INCR_BENCH_DIR']
E = f'{BASE}/{a.engine}'
env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{BASE}/fmt-{a.engine}',
           SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1')
ROOT = f'{BASE}/dlspan/{a.doc}-{a.engine}-s{a.seed}'
shutil.rmtree(ROOT, ignore_errors=True)
os.makedirs(ROOT)


def start(d):
    os.makedirs(f'{d}/out', exist_ok=True)
    sock = f'{d}/h.sock'
    hout = open(f'{d}/h.out', 'w')
    h = subprocess.Popen([f'{E}/flashtex-host', '--socket', sock, '--s0-cache', f'{d}/s0'] + a.host_args.split(),
                         env=env, stdout=hout, stderr=open(f'{d}/h.err', 'a'))
    for _ in range(600):
        if 'listening' in open(f'{d}/h.out').read():
            break
        time.sleep(0.05)
    return h, sock


def stop(h):
    if h.poll() is None:
        h.terminate()
        try:
            h.wait(10)
        except subprocess.TimeoutExpired:
            h.kill()


def fetch(d, sock, tag):
    f = f'{d}/{tag}.dl3'
    subprocess.run([f'{E}/dl3-client', '--socket', sock, '--root', d, '--main', 'main.tex', '--output-dir',
                    f'{d}/out', '--save', f, '--quiet'], check=True, capture_output=True, timeout=1200)
    dump = subprocess.run([f'{E}/dl3-dump', f], check=True, capture_output=True, text=True).stdout
    os.unlink(f)
    files, spans, pages, done = {}, {}, {}, None
    for line in dump.splitlines():
        j = json.loads(line)
        k = j.get('kind')
        if k == 'sources':
            for x in j.get('files', []):
                files[x[0]] = os.path.basename(x[1])
            for x in j.get('spans', []):
                spans[x[0]] = (x[1], x[2])
        elif k == 'page':
            pages[j['index']] = j['items']
        elif k == 'done':
            done = j.get('body', j)
    return files, spans, pages, done


def glyphs(res):
    files, spans, pages, done = res
    n = done.get('pages') if done else None
    out = {}
    for i, items in pages.items():
        if n is not None and i >= n:
            continue
        cur, g = 0, []
        for it in items:
            if it[0] == 'span':
                cur = it[1]
            elif it[0] == 'g':
                fl = spans.get(cur)
                src = (files.get(fl[0], fl[0]), fl[1]) if fl else (None, cur)
                g.append((it[2], src, it[5] if len(it) > 5 else None))
        out[i] = g
    return out, n


# engine A: the incremental host
A = f'{ROOT}/a'
os.makedirs(A)
shutil.copy(f'{BASE}/docs/{a.doc}/main.tex', f'{A}/main.tex')
ha, sa = start(A)
for _ in range(4):  # settle
    r = fetch(A, sa, 'settle')
    if r[3] and r[3].get('mode') == 'unchanged':
        break
rng = random.Random(a.seed)
words = 'alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu'.split()
tot = {'edits': 0, 'line_bad': 0, 'col_bad': 0, 'glyph_count_bad': 0, 'pages_bad': 0, 'glyphs': 0}
for e in range(a.edits):
    src = open(f'{A}/main.tex').read()
    lines = src.split('\n')
    body = [i for i, l in enumerate(lines) if len(l) > 200 and not l.startswith('\\')]
    cand = body[int(len(body) * a.frac):] or body
    li = rng.choice(cand)
    ws = [m.start() for m in re.finditer(r'(?<= )[a-z]{3,}(?= )', lines[li])]
    p = rng.choice(ws)
    if rng.random() < 0.5:
        lines[li] = lines[li][:p + 1] + 'x' + lines[li][p + 1:]
        kind = 'letter'
    else:
        ins = ' '.join(rng.choice(words) for _ in range(12)) + ' '
        lines[li] = lines[li][:p] + ins + lines[li][p:]
        kind = 'sentence'
    open(f'{A}/main.tex', 'w').write('\n'.join(lines))
    ga_res = fetch(A, sa, f'e{e}')
    done = ga_res[3] or {}
    # the reference: a fresh host, from scratch
    R = f'{ROOT}/r{e}'
    os.makedirs(R)
    shutil.copy(f'{A}/main.tex', f'{R}/main.tex')
    hr, sr = start(R)
    rr = fetch(R, sr, 'ref')
    rr = fetch(R, sr, 'ref2')  # the settled pages (a cold compile's own stream can hold an earlier pass)
    stop(hr)
    ga, na = glyphs(ga_res)
    gr, nr = glyphs(rr)
    rec = {'edit': e, 'kind': kind, 'line': li + 1, 'mode': done.get('mode'), 'restart_page': done.get('restart_page'),
           'converged_at': done.get('converged_at'), 'pages_a': na, 'pages_r': nr, 'bad': []}
    lb = cb = 0
    for i in sorted(set(ga) | set(gr)):
        x, y = ga.get(i, []), gr.get(i, [])
        tot['glyphs'] += len(y)
        if [c for c, _, _ in x] != [c for c, _, _ in y]:
            tot['glyph_count_bad'] += 1
            rec['bad'].append({'page': i, 'glyphs_differ': True})
            continue
        l1 = sum(1 for (c1, s1, _), (c2, s2, _) in zip(x, y) if s1 != s2)
        c1 = sum(1 for (_, s1, k1), (_, s2, k2) in zip(x, y) if s1 == s2 and k1 != k2)
        if l1 or c1:
            ex = next(((s1, s2) for (_, s1, _), (_, s2, _) in zip(x, y) if s1 != s2), None)
            rec['bad'].append({'page': i, 'line_bad': l1, 'col_bad': c1, 'example': ex})
            tot['pages_bad'] += 1
        lb += l1
        cb += c1
    shutil.rmtree(R, ignore_errors=True)
    rec['line_bad'], rec['col_bad'] = lb, cb
    tot['line_bad'] += lb
    tot['col_bad'] += cb
    tot['edits'] += 1
    print(json.dumps(rec), flush=True)
stop(ha)
print(json.dumps({'summary': True, 'engine': a.engine, 'doc': a.doc, 'seed': a.seed, **tot}), flush=True)
sys.exit(1 if tot['line_bad'] or tot['col_bad'] or tot['glyph_count_bad'] else 0)
