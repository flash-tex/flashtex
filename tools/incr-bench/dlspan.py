#!/usr/bin/env python3
"""dlspan.py ENGINE DOC [--edits N] [--seed S] [--from FRAC]: the display list's SOURCE SPANS of an
incremental host against a from-scratch host -- the soundness of the side table (changes/displaylist.ch,
`dl_side`), which the other sweeps cannot see: nothing in the PDF, log, aux or terminal depends on it.
Written by the independent review of PR #1300 (P4-MEMORY), which found with it that the convergence
jump did not adopt the side table; kept here as the `span` gate (gates.sh).

A persistent `flashtex-host --socket` (A) is edited N times (a letter or twelve words inserted into a
random paragraph in the part of the document from FRAC on, accumulated; with `--kinds`, also a space
turned into a line break or a blank line, which moves every later line: DESIGN.md §5.3 rule (c)). With
`--eol cr|crlf|mixed` the document's line ends, and those the edits insert, are CR, CR LF or a mix
(TeX ends a line at each; `crate::texlines`), and `wedge` puts a letter between a CR LF's CR and LF.
Files are read and written as bytes. After every compile a fresh
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
ap.add_argument('--kinds', default='letter,sentence',
                help='comma list of letter, sentence, newline, split, wedge (a letter between the CR and LF of a CR LF)')
ap.add_argument('--eol', default='lf', choices=['lf', 'cr', 'crlf', 'mixed'],
                help="the document's line ends and those of inserted breaks (TeX ends a line at a LF, a CR, a CR LF)")
a = ap.parse_args()
KINDS = [k.strip() for k in a.kinds.split(',') if k.strip()]
if not KINDS or [k for k in KINDS if k not in ('letter', 'sentence', 'newline', 'split', 'wedge')]:
    ap.error(f'unknown or empty --kinds {a.kinds}')
signal.alarm(a.timeout)
BASE = os.environ['INCR_BENCH_DIR']
E = f'{BASE}/{a.engine}'
env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{BASE}/fmt-{a.engine}',
           SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1')
ROOT = f'{BASE}/dlspan/{a.doc}-{a.engine}-{a.eol}-s{a.seed}'
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
EOLS = {'lf': ['\n'], 'cr': ['\r'], 'crlf': ['\r\n'], 'mixed': ['\n', '\r', '\r\n']}[a.eol]
erng = random.Random(1000 + a.seed)
def eol():
    return erng.choice(EOLS)
def parse(s):
    out, i = [], 0
    for m in re.finditer(r'\r\n|\r|\n', s):
        out.append([s[i:m.start()], m.group()])
        i = m.end()
    out.append([s[i:], ''])
    return out
src0 = open(f'{BASE}/docs/{a.doc}/main.tex', 'rb').read().decode('latin-1')
doc0 = ''.join(t + (eol() if e else '') for t, e in parse(src0))
open(f'{A}/main.tex', 'wb').write(doc0.encode('latin-1'))
ha, sa = start(A)
for _ in range(4):  # settle
    r = fetch(A, sa, 'settle')
    if r[3] and r[3].get('mode') == 'unchanged':
        break
rng = random.Random(a.seed)
words = 'alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu'.split()
tot = {'edits': 0, 'line_bad': 0, 'col_bad': 0, 'glyph_count_bad': 0, 'pages_bad': 0, 'glyphs': 0}
for e in range(a.edits):
    src = open(f'{A}/main.tex', 'rb').read().decode('latin-1')
    lines = parse(src)
    body = [i for i, l in enumerate(lines) if len(l[0]) > 200 and not l[0].startswith('\\')]
    cand = body[int(len(body) * a.frac):] or body
    li = rng.choice(cand)
    ws = [m.start() for m in re.finditer(r'(?<= )[a-z]{3,}(?= )', lines[li][0])]
    p = rng.choice(ws)
    if KINDS == ['letter', 'sentence']:
        # (the default keeps its random sequence: the seeds of earlier runs give the same edits)
        kind = 'letter' if rng.random() < 0.5 else 'sentence'
    else:
        kind = rng.choice(KINDS)
    t, end = lines[li]
    if kind == 'wedge':
        # a letter between the CR and LF of a CR LF: one more line from one byte
        crlf = [i for i in cand if lines[i][1] == '\r\n']
        if crlf:
            li = rng.choice(crlf)
            t, end = lines[li]
            lines[li:li + 1] = [[t, '\r'], ['x', '\n']]
        else:
            kind = 'newline'
    if kind == 'letter':
        lines[li][0] = t[:p + 1] + 'x' + t[p + 1:]
    elif kind == 'sentence':
        ins = ' '.join(rng.choice(words) for _ in range(12)) + ' '
        lines[li][0] = t[:p] + ins + t[p:]
    elif kind == 'newline':
        lines[li:li + 1] = [[t[:p - 1], eol()], [t[p:], end]]
    elif kind == 'split':
        lines[li:li + 1] = [[t[:p - 1], eol()], ['', eol()], [t[p:], end]]
    open(f'{A}/main.tex', 'wb').write(''.join(x + y for x, y in lines).encode('latin-1'))
    ga_res = fetch(A, sa, f'e{e}')
    done = ga_res[3] or {}
    # the reference: a fresh host, from scratch
    R = f'{ROOT}/r{e}'
    os.makedirs(R)
    shutil.copy(f'{A}/main.tex', f'{R}/main.tex')  # bytes
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
print(json.dumps({'summary': True, 'eol': a.eol, 'kinds': a.kinds, 'engine': a.engine, 'doc': a.doc, 'seed': a.seed, **tot}), flush=True)
sys.exit(1 if tot['line_bad'] or tot['col_bad'] or tot['glyph_count_bad'] else 0)
