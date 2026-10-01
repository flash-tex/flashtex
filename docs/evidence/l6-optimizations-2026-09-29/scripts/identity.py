#!/usr/bin/env python3
"""Do the optimised engines write the same files as the lane's starting point?

usage: identity.py REF ENGINE... [-j N] [--docs a,b] [--no-fixtures]

Every parity fixture (fixtures/real-world, fixtures/divergence-probes; the
fixture directory is copied) and every benchmark document in /tmp/l6o/docs
is compiled by each engine (/tmp/l6o/eng/NAME) until its .aux settles (at
most 4 runs), in its own directory, and the final .pdf and .log are compared
byte for byte with REF's. Prints one line per document that differs and a
summary; exit status 1 if anything differs."""
import argparse
import concurrent.futures as cf
import glob
import os
import shutil
import subprocess

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), '../../../..'))
ap = argparse.ArgumentParser()
ap.add_argument('ref')
ap.add_argument('engines', nargs='+')
ap.add_argument('-j', type=int, default=6)
ap.add_argument('--docs', default='hello,plain-10,plain-100,plain-1000,full-10,full-100,full-300,full-1000,fullnosi-100')
ap.add_argument('--no-fixtures', action='store_true')
a = ap.parse_args()

ENV = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1', TZ='UTC')
for k in list(ENV):
    if k.startswith('FLASHTEX_') and k != 'FLASHTEX_TEXLIVE_BIN':
        del ENV[k]

jobs = []  # (name, srcdir or file, main)
if not a.no_fixtures:
    for f in sorted(glob.glob(f'{REPO}/fixtures/real-world/*/main.tex') +
                    glob.glob(f'{REPO}/fixtures/divergence-probes/*/main.tex')):
        jobs.append((os.path.basename(os.path.dirname(f)), os.path.dirname(f), 'main'))
for d in a.docs.split(','):
    if d:
        jobs.append((d, f'/tmp/l6o/docs/{d}.tex', d))


def compile_one(engine, name, src, main):
    d = f'/tmp/l6o/ident/{engine}/{name}'
    shutil.rmtree(d, ignore_errors=True)
    if os.path.isdir(src):
        shutil.copytree(src, d)
    else:
        os.makedirs(d)
        shutil.copy(src, d)
    e = f'/tmp/l6o/eng/{engine}'
    env = dict(ENV, FLASHTEX_POOL=f'{e}/pdftex.pool', FLASHTEX_FORMATS=f'{e}/fmt')
    prev = None
    for _ in range(4):
        subprocess.run([f'{e}/flashtex-initex', '-fmt=pdflatex', '-interaction=batchmode', main + '.tex'],
                       cwd=d, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=600)
        try:
            aux = open(f'{d}/{main}.aux', 'rb').read()
        except OSError:
            break
        if aux == prev:
            break
        prev = aux
    out = {}
    for ext in ('pdf', 'log'):
        try:
            out[ext] = open(f'{d}/{main}.{ext}', 'rb').read()
        except OSError:
            out[ext] = None
    return out


def one(job):
    name, src, main = job
    ref = compile_one(a.ref, name, src, main)
    res = []
    for e in a.engines:
        o = compile_one(e, name, src, main)
        bad = [ext for ext in ('pdf', 'log') if o[ext] != ref[ext]]
        res.append((e, bad, ref['pdf'] is not None))
    return name, res


diffs = 0
n = 0
with cf.ThreadPoolExecutor(a.j) as ex:
    for name, res in ex.map(one, jobs):
        n += 1
        for e, bad, has_pdf in res:
            if bad:
                diffs += 1
                print(f'DIFF {name} {e}: {",".join(bad)}', flush=True)
print(f'{n} documents x {len(a.engines)} engines vs {a.ref}: {diffs} differ')
raise SystemExit(1 if diffs else 0)
