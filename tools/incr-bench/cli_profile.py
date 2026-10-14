#!/usr/bin/env python3
"""cli_profile.py OUT_DIR [--docs a,b] [--reps N] [--sample DOC,...]: where `flashtex-v3 build` spends
its time against the engine alone (lane CLI-COLD). Uses bench_readme.py's documents, engines and
environment (INCR_BENCH_DIR, FLASHTEX_V3, TEXBIN).

Per document, fresh copies each time:
  engine      flashtex-initex as pdftex, one pass (wall)
  engine-mk   latexmk with it (wall, passes; its whole output kept: why it reran)
  latexmk     latexmk -pdf (wall, passes; output kept)
  v3          flashtex-v3 build --progress, every stderr line timestamped (phases: host start, each
              pass, tools, the export)
  iserve-*    flashtex-host iserve (the resident engine without the socket): one `compile` (its passes
              and per-pass times), preview (uncompressed) and --no-preview (pdflatex's compression), with
              the PDF of the --no-preview run compared with latexmk's
With --sample DOC: macOS `sample` of the host for 10 s during the v3 build of DOC.
"""
import argparse
import json
import os
import subprocess
import sys
import time

S = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, S)
import bench_readme as B  # noqa: E402


def timed_lines(env, cmd, cwd, sample_to=None):
    t0 = time.perf_counter()
    p = subprocess.Popen(cmd, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                         stderr=subprocess.STDOUT, start_new_session=True)
    lines = []
    sampler = None
    for raw in p.stdout:
        t = time.perf_counter() - t0
        lines.append([round(t, 3), raw.decode('latin-1').rstrip()])
        if sample_to and sampler is None and t > 1.0:
            host = subprocess.run(['pgrep', '-P', str(p.pid)], capture_output=True, text=True).stdout.split()
            if host:
                sampler = subprocess.Popen(['sample', host[0], '10', '-file', sample_to],
                                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    p.wait()
    if sampler:
        sampler.wait()
    return time.perf_counter() - t0, p.returncode, lines


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('out')
    ap.add_argument('--docs', default='hw1,plain-1000,full-100,beamer-30,full-1000,infdesc')
    ap.add_argument('--reps', type=int, default=2)
    ap.add_argument('--sample', default='')
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    E = B.engines('classic')
    host = f'{B.IB}/ref/flashtex-host'
    env_i = dict(B.ENV_ENG)
    fo = open(os.path.join(a.out, 'profile.jsonl'), 'a')

    def emit(r):
        fo.write(json.dumps(r) + '\n')
        fo.flush()
        print(json.dumps(r)[:600], flush=True)
    for doc in a.docs.split(','):
        src, main = B.CLASSIC[doc]
        ref_pdf = None
        for e in ('engine', 'engine-mk', 'latexmk'):
            env, cmd = E[e]
            for rep in range(a.reps):
                d = B.fresh(doc, src, main, f'p-{e}')
                r = B.run(env, cmd(main), d, 3 * 3600)
                rec = {'doc': doc, 'what': e, 'rep': rep, 'wall': r['wall'], 'code': r['code']}
                if 'mk' in e or e == 'latexmk':
                    rec['passes'] = B.latexmk_passes(r['out'])
                    open(os.path.join(a.out, f'{doc}-{e}.txt'), 'w').write(r['out'])
                emit(rec)
            if e == 'latexmk':
                ref_pdf = os.path.join(a.out, f'{doc}-latexmk.pdf')
                import shutil
                shutil.copy(B.pdf_of(d, e, main), ref_pdf)
        for rep in range(a.reps):
            d = B.fresh(doc, src, main, 'p-v3')
            env, cmd = E['v3']
            smp = os.path.join(a.out, f'sample-{doc}.txt') if (doc in a.sample.split(',') and rep == a.reps - 1) else None
            wall, code, lines = timed_lines(env, cmd(main), d, smp)
            emit({'doc': doc, 'what': 'v3', 'rep': rep, 'wall': wall, 'code': code,
                  'lines': [l for l in lines if 'flashtex-v3' in l[1]][-40:]})
        for name, extra in (('iserve-preview', []), ('iserve-nopreview', ['--no-preview']),
                            ('iserve-nopreview-lowmem', ['--no-preview', '--profile', 'low-memory'])):
            d = B.fresh(doc, src, main, f'p-{name}')
            outd = os.path.join(d, 'out')
            os.makedirs(outd)
            cmd = [host, 'iserve', *extra, '--', '-fmt=pdflatex', '-interaction=nonstopmode',
                   f'-output-directory={outd}', main]
            t0 = time.perf_counter()
            p = subprocess.run(cmd, cwd=d, env=env_i, input=b'compile\nquit\n', capture_output=True)
            wall = time.perf_counter() - t0
            rep = None
            for l in p.stdout.decode('latin-1').splitlines():
                try:
                    rep = json.loads(l)
                    break
                except ValueError:
                    pass
            keep = {k: rep.get(k) for k in ('mode', 'status', 'pages', 'passes', 'pass_modes', 'pass_s', 'total_s',
                                            'page_s', 'checkpoints', 'l5', 'test_s', 'tests')} if isinstance(rep, dict) else None
            par = B.parity(ref_pdf, os.path.join(outd, os.path.splitext(main)[0] + '.pdf'),
                           os.path.join(a.out, 'pt2', f'{doc}-{name}')) if 'nopreview' in name else None
            emit({'doc': doc, 'what': name, 'wall': wall, 'code': p.returncode, 'report': keep,
                  'parity': par, 'stderr': p.stderr.decode('latin-1')[-800:]})
        import shutil
        shutil.rmtree(f'{B.WORK}/work/{doc}', ignore_errors=True)


if __name__ == '__main__':
    main()
