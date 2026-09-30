#!/usr/bin/env python3
"""Both-paths sweep of the guarded intrinsics over a corpus (DESIGN.md §5.6 item 4).

usage: sweep.py ENGINE {fixtures|lockstep|docs} [--mode verify-all] [-j N] [--only ID...]

ENGINE is /tmp/l6/ENGINE (flashtex-initex, pdftex.pool; its format in /tmp/l6/fmt-ENGINE).
For every document it compiles to convergence (at most 4 runs, like the parity harness)
with FLASHTEX_INTRINSICS=off, then once more from the same .aux with the intrinsics in
MODE ("verify": the registered intrinsics, "verify-all": every parameterless macro that
big_switch expands), which runs both paths for every replayable call and diffs the
complete state change. It reports, per document, the calls, the verified calls, the
differences, and whether the .log (less memory statistics) and .pdf are byte-identical to
the "off" run; then the totals. Exit status 1 if any document differs.

  fixtures  tools/parity's fixture tier (fixtures/real-world, fixtures/divergence-probes)
  lockstep  tools/lockstep/cases in INITEX, with the prelude's tracing switched off (with
            it on, every call falls back: that is the lockstep gate's own run)
  docs      /tmp/l6/docs/*.tex (the benchmark documents)
"""
import argparse
import concurrent.futures as cf
import glob
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), '../../../..'))
sys.path.insert(0, os.path.join(REPO, 'tools/parity'))
sys.path.insert(0, os.path.join(REPO, 'tools'))


def stats_of(path):
    s = {'calls': 0, 'replays': 0, 'verified': 0, 'verify_differences': 0, 'details': ''}
    if not os.path.exists(path):
        s['missing'] = True
        return s
    txt = open(path).read()
    for k in ('calls', 'replays', 'verified', 'verify_differences'):
        m = re.search(r'^\s*' + k + r': (\d+)', txt, re.M)
        if m:
            s[k] = int(m.group(1))
    m = re.search(r'verify_details: \[\s*"(.*?)",?\s*$', txt, re.M)
    s['details'] = m.group(1)[:400] if m else ''
    return s


def filt_log(p):
    if not os.path.exists(p):
        return None
    out = []
    for l in open(p, 'rb').read().decode('latin1').splitlines():
        if 'words of memory' in l or 'Memory usage' in l or 'still untouched' in l:
            continue
        out.append(l)
    return '\n'.join(out)


def engine_env(E, mode, stats=None):
    env = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1', TZ='UTC',
               FLASHTEX_POOL=f'/tmp/l6/{E}/pdftex.pool', FLASHTEX_FORMATS=f'/tmp/l6/fmt-{E}',
               FLASHTEX_INTRINSICS=mode)
    for k in ('FLASHTEX_INTRINSICS_STATS', 'FLASHTEX_INTRINSIC_NAMES'):
        env.pop(k, None)
    if stats:
        env['FLASHTEX_INTRINSICS_STATS'] = stats
    return env


def latex_doc(E, mode, src_dir, entry, did):
    """Compile to convergence with the intrinsics off, then once in MODE."""
    work = tempfile.mkdtemp(dir='/tmp/l6', prefix='sweep.')
    try:
        a = os.path.join(work, 'off')
        shutil.copytree(src_dir, a, symlinks=True)
        job = os.path.splitext(os.path.basename(entry))[0]
        rel = os.path.relpath(os.path.join(src_dir, entry), src_dir) if os.path.isabs(entry) else entry
        cwd = os.path.join(a, os.path.dirname(rel))
        cmd = [f'/tmp/l6/{E}/flashtex-initex', '-fmt=pdflatex', '-interaction=nonstopmode',
               '-no-shell-escape', os.path.basename(rel)]
        prev = None
        for _ in range(4):
            subprocess.run(cmd, cwd=cwd, env=engine_env(E, 'off'), capture_output=True, timeout=900)
            aux = os.path.join(cwd, job + '.aux')
            cur = open(aux, 'rb').read() if os.path.exists(aux) else b''
            if cur == prev:
                break
            prev = cur
        b = os.path.join(work, 'mode')
        shutil.copytree(a, b, symlinks=True)
        cwd_b = os.path.join(b, os.path.dirname(rel))
        st = os.path.join(work, 'stats.txt')
        r = subprocess.run(cmd, cwd=cwd_b, env=engine_env(E, mode, st), capture_output=True, timeout=3600)
        # one more "off" run from the same state, to compare with
        subprocess.run(cmd, cwd=cwd, env=engine_env(E, 'off'), capture_output=True, timeout=900)
        s = stats_of(st)
        la, lb = filt_log(os.path.join(cwd, job + '.log')), filt_log(os.path.join(cwd_b, job + '.log'))
        pa, pb = os.path.join(cwd, job + '.pdf'), os.path.join(cwd_b, job + '.pdf')
        same_pdf = (not os.path.exists(pa) and not os.path.exists(pb)) or (
            os.path.exists(pa) and os.path.exists(pb) and open(pa, 'rb').read() == open(pb, 'rb').read())
        s['same_log'] = la == lb
        s['same_pdf'] = same_pdf
        s['stderr'] = r.stderr.decode('latin1')[-300:] if 'intrinsics verify' in r.stderr.decode('latin1') else ''
        s['id'] = did
        return s
    except subprocess.TimeoutExpired:
        return {'id': did, 'timeout': True, 'calls': 0, 'replays': 0, 'verified': 0, 'verify_differences': 0,
                'same_log': False, 'same_pdf': False, 'details': 'timeout'}
    finally:
        shutil.rmtree(work, ignore_errors=True)


def lockstep_case(E, mode, case):
    work = tempfile.mkdtemp(dir='/tmp/l6', prefix='sweep.')
    try:
        prelude = open(os.path.join(REPO, 'tools/lockstep/prelude.tex')).read()
        # tracing off: every \tracing... = <positive> becomes 0 (the lockstep gate runs
        # the real prelude; here the point is to let the intrinsics replay)
        prelude = re.sub(r'(\\tracing[a-z]+)\s*=\s*-?\d+', r'\1=0', prelude)
        outs = {}
        for m in ('off', mode):
            d = os.path.join(work, m)
            os.makedirs(d)
            open(os.path.join(d, 'prelude.tex'), 'w').write(prelude)
            for f in glob.glob(os.path.join(REPO, 'tools/lockstep/mathsetup.tex')):
                shutil.copy(f, d)
            shutil.copy(case, d)
            st = os.path.join(work, f'stats-{m}.txt')
            subprocess.run([f'/tmp/l6/{E}/flashtex-initex', '-ini', '-etex', '-interaction=nonstopmode',
                            os.path.basename(case)], cwd=d, env=engine_env(E, m, st), capture_output=True,
                           timeout=600)
            job = os.path.splitext(os.path.basename(case))[0]
            outs[m] = (filt_log(os.path.join(d, job + '.log')), os.path.join(d, job + '.pdf'), st)
        s = stats_of(outs[mode][2])
        s['same_log'] = outs['off'][0] == outs[mode][0]
        pa, pb = outs['off'][1], outs[mode][1]
        s['same_pdf'] = (not os.path.exists(pa) and not os.path.exists(pb)) or (
            os.path.exists(pa) and os.path.exists(pb) and open(pa, 'rb').read() == open(pb, 'rb').read())
        s['id'] = os.path.basename(case)
        return s
    finally:
        shutil.rmtree(work, ignore_errors=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('engine')
    ap.add_argument('corpus', choices=['fixtures', 'lockstep', 'docs'])
    ap.add_argument('--mode', default='verify-all')
    ap.add_argument('-j', type=int, default=4)
    ap.add_argument('--only', nargs='*', default=[])
    ap.add_argument('--json', default=None)
    a = ap.parse_args()
    jobs = []
    if a.corpus == 'fixtures':
        import parity
        for d in parity.fixture_documents(only=a.only):
            jobs.append((latex_doc, (a.engine, a.mode, d['dir'], d['entry'], d['id'])))
    elif a.corpus == 'docs':
        for f in sorted(glob.glob('/tmp/l6/docs/*.tex')):
            n = os.path.basename(f)
            if a.only and os.path.splitext(n)[0] not in a.only:
                continue
            d = tempfile.mkdtemp(dir='/tmp/l6', prefix='src.')
            shutil.copy(f, d)
            jobs.append((latex_doc, (a.engine, a.mode, d, n, n)))
    else:
        for f in sorted(glob.glob(os.path.join(REPO, 'tools/lockstep/cases/*.tex'))):
            if a.only and not any(o in f for o in a.only):
                continue
            jobs.append((lockstep_case, (a.engine, a.mode, f)))
    res = []
    with cf.ThreadPoolExecutor(a.j) as ex:
        futs = [ex.submit(fn, *args) for fn, args in jobs]
        for fu in cf.as_completed(futs):
            s = fu.result()
            res.append(s)
            flag = '' if (s['verify_differences'] == 0 and s['same_log'] and s['same_pdf']) else '  <-- DIFFERS'
            print(f"{s['id']}: calls {s['calls']} verified {s['verified']} differences "
                  f"{s['verify_differences']} same log {s['same_log']} same pdf {s['same_pdf']}{flag}", flush=True)
            if flag and s.get('details'):
                print('   ', s['details'])
    tot = {k: sum(r.get(k, 0) for r in res) for k in ('calls', 'verified', 'verify_differences')}
    bad = [r['id'] for r in res if r['verify_differences'] or not r['same_log'] or not r['same_pdf']]
    print(f"TOTAL {a.corpus} mode={a.mode}: documents {len(res)}, calls {tot['calls']}, verified calls "
          f"{tot['verified']}, differences {tot['verify_differences']}, documents with any difference "
          f"{len(bad)} {bad}")
    if a.json:
        json.dump(res, open(a.json, 'w'), indent=1)
    sys.exit(1 if bad else 0)


if __name__ == '__main__':
    main()
