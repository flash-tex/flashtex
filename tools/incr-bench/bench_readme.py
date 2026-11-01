#!/usr/bin/env python3
"""bench_readme.py: cold compiles for README.md's benchmarks (lane BENCH-README), the new engine
against pdflatex and latexmk (Classic) or xelatex and latexmk -xelatex (Unicode).

usage: bench_readme.py OUT.jsonl --set classic|unicode [--docs a,b,...] [--reps N]

Engines, Classic:
  v3          flashtex build (crates/flashtex-build): the CLI on the new engine. It starts
              flashtex-host, runs a resident compile with the external tools, then an export compile
              whose PDF is pdflatex's (P-T2). What a user runs.
  pdflatex    pdflatex, one pass.
  latexmk     latexmk -pdf: pdflatex until it converges, with bibtex/makeindex as needed. A full build.
  engine      flashtex-initex as pdflatex (argv[0] pdftex through the engine's link, first on PATH),
              one pass: the engine alone, no host, no checkpoints.
  engine-mk   latexmk -pdf with flashtex-initex as its pdflatex: the engine's full build.
Unicode: v3 (`--mode unicode`, flashtex-host-unicode), xelatex (one pass), latexmk -xelatex.

Every run gets a fresh copy of the document (no .aux, no .toc: a cold build) and a fresh process; its
wall time is fork to exit. The OS file cache and the engine's format cache are warm: one untimed
warm-up run per (document, engine) first, which also records the peak resident memory of the run's
whole process tree (sampled every 100 ms with ps: only on the warm-up, so the timed runs are not
disturbed) and, for v3, its --progress lines (passes). The timed runs interleave the engines (rep r:
every engine on the document, then rep r+1), so drift hits them alike.

Output parity, from the warm-up: v3's PDF and engine-mk's against latexmk's (Unicode: v3's against
latexmk -xelatex's): byte for byte; else byte for byte but the trailer /ID (pdfTeX's /ID hashes the
output file's name, which v3's private output directory changes); else, Classic, P-T2
(tools/parity/tiers.py compare_pt2: identical font programs and content streams after qpdf
normalisation); Unicode, whose PDF FlashTeX writes itself where xelatex runs xdvipdfmx, the M3 export
gate's structural check (tools/xetex-pdfparity/compare.py --no-visual: pages, glyphs and their
positions, fonts, rules, paths, images, links, outline). A speed figure is valid only for a
document whose output matches.

Environment: INCR_BENCH_DIR (mkeng.sh's engine `ref` and fmt-ref, mkdocs.py's and mkbook.py's
documents), FLASHTEX_V3 (default target/release/flashtex), FLASHTEX_HOST_UNICODE, TEXBIN (default
/Library/TeX/texbin), BENCH_WORK (default $INCR_BENCH_DIR/bench-readme). SOURCE_DATE_EPOCH=0 and
FORCE_SOURCE_DATE=1 for every engine. BENCH_RSS_LIMIT_GB: a warm-up whose process tree passes it is
killed and recorded as such (a hosted runner's 7 GB), and its engine is then left out of that
document's timed runs.
"""
import argparse
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import threading
import time

S = os.path.dirname(os.path.abspath(__file__))
W = os.path.dirname(os.path.dirname(S))
sys.path[:0] = [f'{W}/tools/parity']

IB = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench').rstrip('/')
TEXBIN = os.environ.get('TEXBIN', '/Library/TeX/texbin')
WORK = os.environ.get('BENCH_WORK', f'{IB}/bench-readme')
V3 = os.environ.get('FLASHTEX_V3', f'{W}/target/release/flashtex')
HOST_U = os.environ.get('FLASHTEX_HOST_UNICODE', f'{W}/crates/flashtex-xetex/target/release/flashtex-host-unicode')

BASE = {k: v for k, v in os.environ.items() if not k.startswith('FLASHTEX_')}
BASE.update(PATH=f'{TEXBIN}:/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin:/usr/local/bin',
            SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1')
# v3: the product path (its format cache, in the work directory, never the user's)
ENV_V3 = dict(BASE, FLASHTEX_HOST=f'{IB}/ref/flashtex-host', FLASHTEX_HOST_UNICODE=HOST_U,
              FLASHTEX_FORMAT_CACHE_DIR=f'{WORK}/fmtcache', FLASHTEX_V3_CACHE=f'{WORK}/v3cache',
              XDG_CACHE_HOME=f'{WORK}/xdg')
# the engine as pdflatex: its link first on PATH, its format from mkeng.sh
ENV_ENG = dict(BASE, PATH=f'{IB}/ref:' + BASE['PATH'], FLASHTEX_FORMATS=f'{IB}/fmt-ref',
               FLASHTEX_POOL=f'{IB}/ref/pdftex.pool')
ENV_TEX = BASE

CLASSIC = {
    'hw1': (f'{W}/fixtures/real-world/hw1', 'HW1.tex'),
    'hw2': (f'{W}/fixtures/real-world/hw2', 'HW2.tex'),
    'infdesc': (f'{IB}/docs/infdesc', 'main.tex'),
    'infdesc-x2': (f'{IB}/docs/infdesc-x2', 'main.tex'),
}
for d in ('plain-100', 'plain-300', 'plain-1000', 'full-100', 'full-1000', 'beamer-5', 'beamer-30'):
    CLASSIC[d] = (f'{IB}/docs/{d}', 'main.tex')
UNICODE = {n[:3]: (f'{W}/tools/incr-bench/unicode-corpus/{n}', 'main.tex')
           for n in ('b01-article.tex', 'b02-georgia.tex', 'b03-unicode-math.tex', 'b04-fontspec-11pages.tex')}

OUTPUTS = ('*.pdf', '*.aux', '*.toc', '*.log', '*.out', '*.idx', '*.ind', '*.ilg', '*.bbl', '*.blg', '*.fls',
           '*.fdb_latexmk', '*.synctex.gz', '*.lof', '*.lot', '*.xdv', '*.nav', '*.snm', '*.vrb', 'out')


def engines(kind):
    if kind == 'unicode':
        return {
            'v3': (ENV_V3, lambda m: [V3, 'build', m, '-o', 'v3-out.pdf', '--progress', '--mode', 'unicode']),
            'xelatex': (ENV_TEX, lambda m: ['xelatex', '-interaction=batchmode', m]),
            'latexmk': (ENV_TEX, lambda m: ['latexmk', '-xelatex', '-interaction=batchmode', m]),
        }
    return {
        'v3': (ENV_V3, lambda m: [V3, 'build', m, '-o', 'v3-out.pdf', '--progress', '--mode', 'classic']),
        'pdflatex': (ENV_TEX, lambda m: ['pdflatex', '-interaction=batchmode', m]),
        'latexmk': (ENV_TEX, lambda m: ['latexmk', '-pdf', '-interaction=batchmode', m]),
        'engine': (ENV_ENG, lambda m: ['pdftex', '-fmt=pdflatex', '-interaction=batchmode', m]),
        'engine-mk': (ENV_ENG, lambda m: ['latexmk', '-pdf', '-pdflatex=pdftex -fmt=pdflatex %O %S',
                                          '-interaction=batchmode', m]),
    }


def load1():
    try:
        if sys.platform == 'darwin':
            return float(subprocess.run(['sysctl', '-n', 'vm.loadavg'], capture_output=True,
                                        text=True).stdout.split()[1])
        return float(open('/proc/loadavg').read().split()[0])
    except (OSError, ValueError, IndexError):
        return None


def fresh(doc, src, main, tag):
    d = f'{WORK}/work/{doc}/{tag}'
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(os.path.dirname(d), exist_ok=True)
    if os.path.isdir(src):
        shutil.copytree(src, d, ignore=shutil.ignore_patterns(*OUTPUTS))
        # figures: a PDF that is not a build output (main's, or a book's own compiled copy)
        for root, dirs, files in os.walk(src):
            dirs[:] = [x for x in dirs if x != 'out']
            for f in files:
                if f.endswith('.pdf') and os.path.splitext(f)[0] not in (os.path.splitext(main)[0], 'infdesc'):
                    rel = os.path.relpath(os.path.join(root, f), src)
                    os.makedirs(os.path.dirname(os.path.join(d, rel)) or d, exist_ok=True)
                    shutil.copy2(os.path.join(root, f), os.path.join(d, rel))
    else:
        os.makedirs(d)
        shutil.copy2(src, os.path.join(d, main))
    return d


def tree_rss(root):
    """Resident KB of root and its descendants (ps)."""
    try:
        out = subprocess.run(['ps', '-Ao', 'pid=,ppid=,rss='], capture_output=True, text=True).stdout
    except OSError:
        return 0, 0
    kids, rss = {}, {}
    for l in out.splitlines():
        p = l.split()
        if len(p) == 3:
            pid, ppid, r = map(int, p)
            kids.setdefault(ppid, []).append(pid)
            rss[pid] = r
    tot, top, todo = 0, 0, [root]
    while todo:
        p = todo.pop()
        tot += rss.get(p, 0)
        top = max(top, rss.get(p, 0))
        todo += kids.get(p, [])
    return tot, top


def run(env, cmd, cwd, timeout, sample=False):
    t0 = time.perf_counter()
    p = subprocess.Popen(cmd, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                         stderr=subprocess.STDOUT, start_new_session=True)

    def kill():
        try:
            os.killpg(p.pid, signal.SIGKILL)  # the run's whole group: latexmk's pdflatex, v3's host
        except OSError:
            p.kill()
    peak = [0, 0]
    killed = []
    stop = threading.Event()
    limit_kb = float(os.environ.get('BENCH_RSS_LIMIT_GB') or 0) * 2**20

    def sampler():
        while not stop.wait(0.1):
            t, m = tree_rss(p.pid)
            peak[0], peak[1] = max(peak[0], t), max(peak[1], m)
            if limit_kb and t > limit_kb and not killed:
                # the runner's memory: stop the run (v3's host ends with its connection) and say so
                killed.append(t)
                kill()
    th = threading.Thread(target=sampler, daemon=True) if sample else None
    if th:
        th.start()
    try:
        out, _ = p.communicate(timeout=timeout)
        wall, code = time.perf_counter() - t0, p.returncode
    except subprocess.TimeoutExpired:
        kill()
        out, _ = p.communicate()
        wall, code = None, 'timeout'
    stop.set()
    if th:
        th.join()
    r = {'wall': wall, 'code': code, 'out': out.decode('latin-1')}
    if sample:
        r['peak_tree_rss_kb'], r['peak_proc_rss_kb'] = peak
        if killed:
            r['killed_at_limit_kb'] = killed[0]
            r['code'] = 'killed at BENCH_RSS_LIMIT_GB'
    return r


def pdf_of(d, e, main):
    return os.path.join(d, 'v3-out.pdf' if e == 'v3' else os.path.splitext(main)[0] + '.pdf')


def parity_unicode(ref, cand, work):
    """Unicode mode's export is FlashTeX's own PDF writer, not xdvipdfmx: its parity is structural
    (tools/xetex-pdfparity/compare.py, the M3 export gate's check, without the raster)."""
    if not ref or not cand or not os.path.exists(ref) or not os.path.exists(cand):
        return {'ok': False, 'why': 'missing PDF'}
    os.makedirs(work, exist_ok=True)
    out = os.path.join(work, 'compare.json')
    p = subprocess.run([sys.executable, f'{W}/tools/xetex-pdfparity/compare.py', ref, cand, '--no-visual',
                        '--json', out], capture_output=True, text=True)
    try:
        rep = json.load(open(out))
    except (OSError, ValueError):
        return {'ok': False, 'why': f'compare.py exit {p.returncode}: {(p.stderr or p.stdout)[-300:]}'}
    kinds = rep.get('kinds') or rep.get('differences') or {}
    ok = p.returncode == 0
    st = rep.get('stats') or {}
    return {'ok': ok, 'level': 'structurally equal (xetex-pdfparity)' if ok else 'MISMATCH (structural)',
            'summary': (p.stdout or '').splitlines()[:1], 'kinds': {k: (v.get('count') if isinstance(v, dict) else v)
                                                                     for k, v in kinds.items()} if isinstance(kinds, dict) else kinds,
            'structural_differences': rep.get('structural_differences'), 'glyphs': [st.get('glyphs_matched'), st.get('glyphs_ref')], 'glyph_max_dev_bp': st.get('glyph_max_dev')}


def parity(ref, cand, work, unicode=False):
    if not ref or not cand or not os.path.exists(ref) or not os.path.exists(cand):
        return {'ok': False, 'why': 'missing PDF: ' + ', '.join(x for x, f in (('reference', ref), ('candidate', cand))
                                                                if not f or not os.path.exists(f))}
    a, b = open(ref, 'rb').read(), open(cand, 'rb').read()
    strip = lambda x: re.sub(rb'/ID \[<[0-9A-Fa-f]+> ?<[0-9A-Fa-f]+>\]', b'/ID []', x)
    if a == b:
        return {'ok': True, 'level': 'byte-identical', 'bytes': len(a)}
    if strip(a) == strip(b):
        return {'ok': True, 'level': 'byte-identical but the trailer /ID', 'bytes': len(a)}
    if unicode:
        return dict(parity_unicode(ref, cand, work), bytes=[len(a), len(b)])
    try:
        import tiers
        os.makedirs(work, exist_ok=True)
        pt2 = tiers.compare_pt2(ref, cand, work)
    except Exception as e:  # harness: say so, never pass
        return {'ok': False, 'why': f'P-T2 harness error: {e}'[:300]}
    keep = {k: pt2.get(k) for k in ('pages', 'pages_equal', 'fonts_equal', 'content_equal', 'first_page', 'why')}
    return dict(keep, ok=bool(pt2.get('ok')), level='P-T2 equal' if pt2.get('ok') else 'MISMATCH',
                bytes=[len(a), len(b)])


def latexmk_passes(out):
    return len(re.findall(r"Run number \d+ of rule '(?:pdf|xe)?latex'", out))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('out')
    ap.add_argument('--set', choices=['classic', 'unicode'], default='classic')
    ap.add_argument('--docs')
    ap.add_argument('--engines', help='a subset of the set\'s engines, comma separated')
    ap.add_argument('--reps', type=int, default=7)
    ap.add_argument('--timeout', type=int, default=3 * 3600, help='per run, s')
    a = ap.parse_args()
    table = UNICODE if a.set == 'unicode' else CLASSIC
    docs = a.docs.split(',') if a.docs else list(table)
    E = engines(a.set)
    if a.engines:
        E = {k: v for k, v in E.items() if k in a.engines.split(',')}
    ref_engine = 'latexmk'
    os.makedirs(WORK, exist_ok=True)
    with open(a.out, 'a') as fo:
        def emit(rec):
            rec.setdefault('utc', time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()))
            fo.write(json.dumps(rec) + '\n')
            fo.flush()
        for doc in docs:
            src, main = table[doc]
            pdfs, dead = {}, set()
            for e, (env, cmd) in E.items():
                d = fresh(doc, src, main, f'warm-{e}')
                l0 = load1()
                r = run(env, cmd(main), d, a.timeout, sample=True)
                pdfs[e] = pdf_of(d, e, main)
                rec = {'doc': doc, 'set': a.set, 'engine': e, 'rep': 'warm', 'wall': r['wall'], 'code': r['code'],
                       'load1': [l0, load1()], 'peak_tree_rss_kb': r['peak_tree_rss_kb'],
                       'peak_proc_rss_kb': r['peak_proc_rss_kb']}
                if e in ('latexmk', 'engine-mk'):
                    rec['passes'] = latexmk_passes(r['out'])
                if e == 'v3':
                    rec['progress'] = [l for l in r['out'].splitlines() if l.strip()][-12:]
                if r.get('killed_at_limit_kb'):
                    rec['killed_at_limit_kb'] = r['killed_at_limit_kb']
                if r['code'] != 0:
                    rec['tail'] = r['out'][-1500:]
                if r['code'] in ('timeout', 'killed at BENCH_RSS_LIMIT_GB'):
                    dead.add(e)
                emit(rec)
                print(f"{doc} {e} warm-up {r['wall'] and round(r['wall'], 2)} s, code {r['code']}, "
                      f"peak {r['peak_tree_rss_kb'] / 1024:.0f} MB", flush=True)
            for cand in [e for e in E if e in ('v3', 'engine-mk')]:
                par = parity(pdfs.get(ref_engine), pdfs.get(cand), f'{WORK}/pt2/{doc}-{cand}', a.set == 'unicode')
                emit({'doc': doc, 'set': a.set, 'parity': cand, 'against': ref_engine, **par})
                print(f"{doc} parity {cand} vs {ref_engine}: {par.get('level') or par.get('why')}", flush=True)
            live = {e: v for e, v in E.items() if e not in dead}
            for rep in range(a.reps):
                for e, (env, cmd) in live.items():
                    d = fresh(doc, src, main, f'r-{e}')
                    l0 = load1()
                    r = run(env, cmd(main), d, a.timeout)
                    rec = {'doc': doc, 'set': a.set, 'engine': e, 'rep': rep, 'wall': r['wall'], 'code': r['code'],
                           'load1': [l0, load1()]}
                    if e in ('latexmk', 'engine-mk'):
                        rec['passes'] = latexmk_passes(r['out'])
                    emit(rec)
                    print(f"  {doc} {e} r{rep} {r['wall'] and round(r['wall'], 3)} s code {r['code']}", flush=True)
            shutil.rmtree(f'{WORK}/work/{doc}', ignore_errors=True)


if __name__ == '__main__':
    main()
