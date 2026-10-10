#!/usr/bin/env python3
"""reopen_check.py ENGINE [-j J]: for every fixture (fixtures/real-world, fixtures/divergence-probes),
save S0 (`flashtex-host bench --reps 1 --save`), open it (`flashtex-host open`) and compare the PDF,
log and aux with the saving run's; then edit the body (a word inserted before \\end{document}), open
S0 again, and compare with a from-scratch CLI run of the edited document. Lane MEM-BASELINE: S0's
pages are mapped into the word space (copy on write) instead of copied."""
import argparse
import concurrent.futures
import os
import shutil
import subprocess

ROOT = '/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a2fa2131a670119f7'
ap = argparse.ArgumentParser()
ap.add_argument('engine')
ap.add_argument('-j', type=int, default=2)
a = ap.parse_args()
IB = '/private/tmp/mb/ib'
E = f'{IB}/{a.engine}'
W = f'/private/tmp/mb/reopen-{a.engine}'
shutil.rmtree(W, ignore_errors=True)
env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{IB}/fmt-{a.engine}',
           SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1', FLASHTEX_PIN_CLOCK='1700000000.123456',
           TMPDIR='/private/tmp/mb/tmp')


def fixtures():
    out = []
    for tier in ('fixtures/real-world', 'fixtures/divergence-probes'):
        base = os.path.join(ROOT, tier)
        for name in sorted(os.listdir(base)):
            d = os.path.join(base, name)
            if not os.path.isdir(d):
                continue
            texs = [f for f in os.listdir(d) if f.endswith('.tex')]
            main = 'main.tex' if 'main.tex' in texs else (texs[0] if len(texs) == 1 else None)
            if main:
                out.append((name, d, main[:-4]))
    return out


def files(d, job):
    return {x: open(f'{d}/{job}.{x}', 'rb').read() if os.path.exists(f'{d}/{job}.{x}') else None
            for x in ('pdf', 'log', 'aux')}


def run(fx):
    name, src, job = fx
    d = f'{W}/{name}'
    shutil.copytree(src, d)
    args = ['--', '-fmt=pdflatex', '-interaction=nonstopmode', f'{job}.tex']
    s0 = f'{d}/s0.bin'
    host = [f'{E}/flashtex-host']
    p = subprocess.run(host + ['bench', '--reps', '1', '--save', s0] + args, cwd=d, env=env,
                       capture_output=True, text=True, timeout=900)
    if not os.path.exists(s0):
        return name, 'no-s0', p.stdout[-200:] + p.stderr[-200:]
    before = files(d, job)
    p = subprocess.run(host + ['open', s0] + args, cwd=d, env=env, capture_output=True, text=True, timeout=900)
    if '"mode":"s0"' not in p.stdout:
        return name, 'not-opened', (p.stdout + p.stderr)[-300:]
    mapped = '"mapped_pages":0' not in p.stdout
    after = files(d, job)
    bad = [x for x in before if before[x] != after[x]]
    if bad:
        return name, 'DIFF-open', bad
    # an edit of the body, then the same S0, against a from-scratch run from the same files
    e = f'{d}-scratch'
    shutil.copytree(d, e)
    t = open(f'{d}/{job}.tex', encoding='latin-1').read()
    k = t.rfind('\\end{document}')
    if k < 0:
        return name, 'ok-open-only', mapped
    for x in (d, e):
        open(f'{x}/{job}.tex', 'w', encoding='latin-1').write(t[:k] + 'Zebra quokka.\n\n' + t[k:])
    p = subprocess.run(host + ['open', s0] + args, cwd=d, env=env, capture_output=True, text=True, timeout=900)
    if '"mode":"s0"' not in p.stdout:
        return name, 'edit-not-opened', mapped
    inc = files(d, job)
    subprocess.run([f'{E}/pdftex', '-fmt=pdflatex', '-interaction=nonstopmode', f'{job}.tex'], cwd=e, env=env,
                   capture_output=True, timeout=900)
    ref = files(e, job)
    bad = [x for x in ref if ref[x] != inc[x]]
    return name, ('DIFF-edit' if bad else 'ok'), (bad, mapped)


os.makedirs(W)
res = {}
with concurrent.futures.ThreadPoolExecutor(a.j) as ex:
    for name, st, info in ex.map(run, fixtures()):
        res[name] = st
        print(name, st, info, flush=True)
print({k: sum(1 for v in res.values() if v == k) for k in set(res.values())})
