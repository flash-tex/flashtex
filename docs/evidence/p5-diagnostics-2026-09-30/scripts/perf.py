#!/usr/bin/env python3
"""The diagnostics side channel's cost in the resident engine: the same
editing session with the side channel on (the default) and off
(FLASHTEX_NO_DIAGNOSTICS=1), interleaved.

    perf.py ENGDIR FMTDIR [--pages-paras N] [--reps R] [--edits E]

A generated article (N paragraphs, an error, an undefined reference and an
overfull box every 40 paragraphs, a user macro), `flashtex-host iserve`:
one cold compile, then E single-character edits in the middle of the
document with their reverts; `total_s` of the cold compile and `edited` (ms
to the edited page) of the edits, and the time of the `diagnostics` command
(building every DIAG of the document). R rounds, on/off alternating.
"""
import json
import os
import statistics
import subprocess
import sys
import tempfile
import time

eng, fmt = sys.argv[1], sys.argv[2]
args = sys.argv[3:]
def opt(k, d):
    return int(args[args.index(k) + 1]) if k in args else d
N = opt('--pages-paras', 600)
R = opt('--reps', 3)
E = opt('--edits', 10)


def doc(word):
    s = ['\\documentclass{article}', '\\newcommand{\\mycmd}[1]{\\textbf{#1}}', '\\begin{document}']
    for i in range(N):
        w = word if i == N // 2 else 'alpha'
        s.append('Paragraph %d with the word %s and enough text to fill a few lines of the page '
                 'so that the document ships several pages; see \\ref{sec} and $a^{%d}+b$ '
                 'and \\mycmd{x}.\n' % (i, w, i))
        if i % 40 == 7:
            s.append('An error \\foo{} here and \\ref{nolabel} there.\n')
            s.append('\\hbox to 20pt{An overfull box}\n')
    s.append('\\section{S}\\label{sec}')
    s.append('\\end{document}')
    return '\n'.join(s) + '\n'


def session(off):
    with tempfile.TemporaryDirectory(prefix='p5d-perf-') as d:
        with open(os.path.join(d, 'doc.tex'), 'w') as f:
            f.write(doc('alpha'))
        env = dict(os.environ, FLASHTEX_POOL=os.path.join(eng, 'pdftex.pool'), FLASHTEX_FORMATS=fmt,
                   SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1',
                   FLASHTEX_PIN_CLOCK='1700000000.123456', FLASHTEX_DISPLAY_LIST='/dev/null')
        if off:
            env['FLASHTEX_NO_DIAGNOSTICS'] = '1'
        p = subprocess.Popen([os.path.join(eng, 'flashtex-host'), 'iserve', '--', '-fmt=pdflatex',
                              '-interaction=nonstopmode', '-file-line-error', 'doc.tex'],
                             stdin=subprocess.PIPE, stdout=subprocess.PIPE, cwd=d, env=env, text=True)

        def cmd(c):
            p.stdin.write(c + '\n')
            p.stdin.flush()
            return p.stdout.readline()
        cold = json.loads(cmd('compile'))
        cmd('compile')  # settle the .aux
        edited, diag_ms, ndiag = [], [], 0
        for k in range(E):
            for w in ('alphb', 'alpha'):
                with open(os.path.join(d, 'doc.tex'), 'w') as f:
                    f.write(doc(w))
                r = json.loads(cmd('compile'))
                if r.get('edited') is not None:
                    edited.append(r['edited'] * 1000 if r['edited'] < 10 else r['edited'])
                t = time.perf_counter()
                out = cmd('diagnostics')
                diag_ms.append((time.perf_counter() - t) * 1000)
                ndiag = len(json.loads(out)['diagnostics'])
        cmd('quit')
        p.wait()
        return cold['total_s'], edited, diag_ms, ndiag, cold.get('pages')


rows = {False: [], True: []}
for r in range(R):
    for off in (False, True):
        rows[off].append(session(off))
for off in (False, True):
    colds = [x[0] for x in rows[off]]
    ed = [e for x in rows[off] for e in x[1]]
    dm = [e for x in rows[off] for e in x[2]]
    print(json.dumps({'side_channel': 'off' if off else 'on', 'pages': rows[off][0][4],
                      'diagnostics': rows[off][0][3],
                      'cold_s_median': round(statistics.median(colds), 4), 'cold_s': [round(c, 4) for c in colds],
                      'edited_ms_p50': round(statistics.median(ed), 3) if ed else None,
                      'edited_ms_p95': round(sorted(ed)[int(0.95 * (len(ed) - 1))], 3) if ed else None,
                      'diagnostics_cmd_ms_p50': round(statistics.median(dm), 3)}))
