#!/usr/bin/env python3
"""The soundness harness, for diagnostics: every compile of an incremental
editing session reports the same diag-v1 messages as a compile from scratch.

    diag_soundness.py ENGDIR FMTDIR REPO [-j N] [--only NAME] [--out FILE]

For every parity fixture (fixtures/real-world, fixtures/divergence-probes;
the main file as tests/host_incremental.rs finds it), in a scratch copy:
`flashtex-host iserve` (side channel on, a display list written) compiles
twice (the .aux settles), then applies edits to the main file, each followed
by its revert: a letter changed in a word at 20%, 50% and 80% of the file,
an undefined control sequence inserted at 50% (a new error), and a line
break inserted at 50% (lines move). After every compile its `diagnostics`
are compared with those of a fresh `iserve`'s compile of a copy of the
directory as the compile found it (the host's passes included), after
normalising what differs between two directories and two processes (the
directory in paths; span ids, which are names per process).
One JSON line per fixture; a summary at the end.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

eng, fmt, repo = sys.argv[1], sys.argv[2], sys.argv[3]
args = sys.argv[4:]
J = int(args[args.index('-j') + 1]) if '-j' in args else 8
ONLY = args[args.index('--only') + 1] if '--only' in args else None
OUT = args[args.index('--out') + 1] if '--out' in args else None


def env():
    e = dict(os.environ, FLASHTEX_POOL=os.path.join(eng, 'pdftex.pool'), FLASHTEX_FORMATS=fmt,
             SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1',
             FLASHTEX_PIN_CLOCK='1700000000.123456', FLASHTEX_DISPLAY_LIST='/dev/null')
    e.pop('FLASHTEX_NO_DIAGNOSTICS', None)
    return e


class Host:
    def __init__(self, d, main):
        self.p = subprocess.Popen([os.path.join(eng, 'flashtex-host'), 'iserve', '--', '-fmt=pdflatex',
                                   '-interaction=nonstopmode', '-file-line-error', main],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                  cwd=d, env=env(), text=True)

    def cmd(self, c):
        self.p.stdin.write(c + '\n')
        self.p.stdin.flush()
        line = self.p.stdout.readline()
        if line.startswith('{"error"'):
            raise RuntimeError('%s: %s' % (c, line))
        return line

    def diagnostics(self):
        return json.loads(self.cmd('diagnostics'))['diagnostics']

    def close(self):
        try:
            self.p.stdin.write('quit\n')
            self.p.stdin.flush()
        except OSError:
            pass
        self.p.wait()


def norm(v, d):
    real = os.path.realpath(d)

    def strip(j):
        if isinstance(j, dict):
            return {k: strip(x) for k, x in j.items() if k != 'span'}
        if isinstance(j, list):
            return [strip(x) for x in j]
        if isinstance(j, str):
            return j.replace(real, '<DIR>').replace(d, '<DIR>')
        return j
    return json.dumps(strip(v), sort_keys=True)


def copy_dir(a, b):
    shutil.rmtree(b, ignore_errors=True)
    os.makedirs(b)
    for f in os.listdir(a):
        p = os.path.join(a, f)
        if os.path.isfile(p):
            shutil.copy(p, os.path.join(b, f))


def edits(text):
    out = []
    lines = text.split('\n')
    starts = [i for i, l in enumerate(lines) if re.search(r'[A-Za-z]{5,}', l) and not l.lstrip().startswith('%')
              and not l.lstrip().startswith('\\')]
    for frac in (0.2, 0.5, 0.8):
        if not starts:
            break
        i = starts[int(frac * (len(starts) - 1))]
        m = re.search(r'([A-Za-z])([A-Za-z])([A-Za-z]{3,})', lines[i])
        if not m:
            continue
        l2 = lines[i][:m.start()] + m.group(2) + m.group(1) + lines[i][m.start() + 2:]
        out.append(('letter %d%%' % int(frac * 100), '\n'.join(lines[:i] + [l2] + lines[i + 1:])))
    if starts:
        i = starts[len(starts) // 2]
        m = re.search(r' ', lines[i])
        if m:
            l2 = lines[i][:m.start()] + ' \\flashtexundefined ' + lines[i][m.start() + 1:]
            out.append(('new error', '\n'.join(lines[:i] + [l2] + lines[i + 1:])))
            l3 = lines[i][:m.start()] + '\n' + lines[i][m.start() + 1:]
            out.append(('line break', '\n'.join(lines[:i] + [l3] + lines[i + 1:])))
    return out


def fixture(path):
    name = os.path.basename(path)
    texs = [f for f in os.listdir(path) if f.endswith('.tex')]
    main = 'main.tex' if 'main.tex' in texs else (texs[0] if len(texs) == 1 else None)
    if not main:
        return {'fixture': name, 'skipped': 'no main file'}
    work = tempfile.mkdtemp(prefix='p5d-snd-')
    d = os.path.join(work, 'doc')
    ref = os.path.join(work, 'ref')
    copy_dir(path, d)
    orig = open(os.path.join(d, main), encoding='latin-1').read()
    h = Host(d, main)
    res = {'fixture': name, 'compiles': 0, 'mismatches': [], 'diagnostics': 0}

    def step(text, what):
        with open(os.path.join(d, main), 'w', encoding='latin-1') as f:
            f.write(text)
        copy_dir(d, ref)
        h.cmd('compile')
        got = h.diagnostics()
        r = Host(ref, main)
        r.cmd('compile')
        want = r.diagnostics()
        r.close()
        res['compiles'] += 1
        res['diagnostics'] += len(got)
        if norm(got, d) != norm(want, ref):
            res['mismatches'].append({'edit': what, 'got': len(got), 'want': len(want)})
    try:
        step(orig, 'first')
        step(orig, 'settle')
        for what, t in edits(orig):
            step(t, what)
            step(orig, what + ' reverted')
    except Exception as e:  # a host failure is a finding, not a skip
        res['error'] = str(e)[:300]
    finally:
        h.close()
        shutil.rmtree(work, ignore_errors=True)
    return res


fx = []
for tier in ('real-world', 'divergence-probes'):
    base = os.path.join(repo, 'fixtures', tier)
    for n in sorted(os.listdir(base)):
        if os.path.isdir(os.path.join(base, n)) and (ONLY is None or n == ONLY):
            fx.append(os.path.join(base, n))
out = open(OUT, 'w') if OUT else None
tot = {'fixtures': 0, 'compiles': 0, 'mismatches': 0, 'errors': 0, 'diagnostics': 0}
with ThreadPoolExecutor(J) as ex:
    for r in ex.map(fixture, fx):
        line = json.dumps(r)
        print(line, flush=True)
        if out:
            out.write(line + '\n')
        if 'skipped' in r:
            continue
        tot['fixtures'] += 1
        tot['compiles'] += r['compiles']
        tot['mismatches'] += len(r['mismatches'])
        tot['errors'] += 1 if 'error' in r else 0
        tot['diagnostics'] += r['diagnostics']
print(json.dumps({'summary': tot}))
if out:
    out.write(json.dumps({'summary': tot}) + '\n')
sys.exit(1 if tot['mismatches'] or tot['errors'] else 0)
