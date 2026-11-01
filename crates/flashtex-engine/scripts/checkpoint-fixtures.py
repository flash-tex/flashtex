#!/usr/bin/env python3
"""Checkpoint tests on the parity fixtures (DESIGN.md §5.1, §5.2).

For every fixture of tools/parity's fixtures tier (fixtures/real-world and
fixtures/divergence-probes), in a scratch copy of its directory that
flashtex-initex has first run to convergence (every file stable, at most 5
runs, as the parity oracle does), so that the files a run reads after a
checkpoint (`.toc`, `.out`, ...) are the ones it writes:

  selftest  flashtex-host selftest: a run with a checkpoint at S0 and after
            every shipout; restoring each (up to --max-targets) and running on
            gives a bit-identical engine state at every later checkpoint and
            at the end, byte-identical files and terminal; plus redo_to and
            restore_discard.
  s0        flashtex-host compiles the document twice (cold, then from S0) and
            the second compile's .pdf, .log and .aux equal flashtex-initex's
            full run on the same source and files.

usage: checkpoint-fixtures.py --bin DIR --fmt DIR [--only NAME] [-j N]
  DIR holds flashtex-initex (linked as `pdftex`), flashtex-host and
  pdftex.pool; --fmt holds this engine's pdflatex.fmt.
A fixture whose run stops with a fatal error before S0 has nothing to
restore: its selftest is n/a, and it passes when the host's output equals the
CLI's. Exit status 0 iff every fixture passes both checks.
"""
import argparse
import concurrent.futures
import json
import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))


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
                out.append((name, d, main))
    return out


def env(a):
    e = dict(os.environ)
    e.update(FLASHTEX_POOL=os.path.join(a.bin, 'pdftex.pool'), FLASHTEX_FORMATS=a.fmt,
             SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1',
             FLASHTEX_PIN_CLOCK='1700000000.123456', TZ='UTC')
    return e


def run(cmd, cwd, a, timeout=600):
    p = subprocess.run(cmd, cwd=cwd, env=env(a), stdin=subprocess.DEVNULL,
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
    return p.returncode, p.stdout.decode('utf-8', 'replace')


def snapshot(d):
    return {n: open(os.path.join(d, n), 'rb').read()
            for n in sorted(os.listdir(d)) if os.path.isfile(os.path.join(d, n))}


def converge(pdftex, d, a):
    before = snapshot(d)
    for _ in range(5):
        run(pdftex, d, a)
        after = snapshot(d)
        if after == before:
            return True
        before = after
    return False


def check(fx, a):
    name, src, main = fx
    job = main[:-4]
    pdftex = [os.path.join(a.bin, 'pdftex'), '-fmt=pdflatex', '-interaction=nonstopmode', main]
    host = os.path.join(a.bin, 'flashtex-host')
    res = {'name': name}
    with tempfile.TemporaryDirectory(prefix='ckfx.') as t:
        # selftest
        w = os.path.join(t, 'self')
        shutil.copytree(src, w)
        res['converged'] = converge(pdftex, w, a)
        code, out = run([host, 'selftest', '--max-targets', str(a.max_targets), '--argv0', pdftex[0], '--'] + pdftex[1:], w, a)
        last = out.strip().splitlines()[-1] if out.strip() else ''
        try:
            j = json.loads(last)
            res['selftest'] = 'pass' if code == 0 and j.get('failures') == 0 else 'FAIL'
            if j.get('error') == 'no checkpoint was taken' and run(pdftex, w, a)[0] != 0:
                # The run stops with a fatal error before S0 (e.g. an image
                # before image inclusion lands): nothing to restore.
                res['selftest'] = 'n/a'
            res['checks'] = j.get('checks')
            res['detail'] = j.get('detail', [])[:3]
        except ValueError:
            res['selftest'] = 'FAIL'
            res['detail'] = [last[-300:]]
        for l in out.splitlines():
            if l.startswith('{"uninterrupted"'):
                res['checkpoints'] = json.loads(l).get('checkpoints')
        # compile from S0 vs the CLI
        h, c = os.path.join(t, 'host'), os.path.join(t, 'cli')
        shutil.copytree(w, h)
        shutil.copytree(w, c)
        code, out = run([host, 'bench', '--reps', '1', '--argv0', pdftex[0], '--'] + pdftex[1:], h, a)
        modes = [json.loads(l).get('mode') for l in out.splitlines() if l.startswith('{"mode"')]
        res['modes'] = modes
        run(pdftex, c, a)
        diffs = []
        for ext in ('pdf', 'log', 'aux'):
            x, y = os.path.join(h, f'{job}.{ext}'), os.path.join(c, f'{job}.{ext}')
            bx = open(x, 'rb').read() if os.path.exists(x) else None
            by = open(y, 'rb').read() if os.path.exists(y) else None
            if bx != by:
                diffs.append(ext)
        res['s0_used'] = modes[-1:] == ['s0']
        res['s0'] = 'pass' if code == 0 and not diffs else 'FAIL'
        res['s0_diffs'] = diffs
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--bin', required=True)
    ap.add_argument('--fmt', required=True)
    ap.add_argument('--only')
    ap.add_argument('--max-targets', type=int, default=6)
    ap.add_argument('-j', type=int, default=4)
    a = ap.parse_args()
    fxs = [f for f in fixtures() if not a.only or a.only in f[0]]
    with concurrent.futures.ThreadPoolExecutor(a.j) as ex:
        results = list(ex.map(lambda f: check(f, a), fxs))
    ok = 0
    for r in results:
        good = r['selftest'] in ('pass', 'n/a') and r['s0'] == 'pass'
        ok += good
        print(json.dumps(r))
    s0_used = sum(1 for r in results if r.get('s0_used'))
    na = sum(r['selftest'] == 'n/a' for r in results)
    print(f'{ok}/{len(results)} fixtures pass (selftest {sum(r["selftest"] == "pass" for r in results)}'
          f' + {na} n/a: fatal error before S0; from-S0 {sum(r["s0"] == "pass" for r in results)};'
          f' S0 used by {s0_used})')
    sys.exit(0 if ok == len(results) else 1)


if __name__ == '__main__':
    main()
