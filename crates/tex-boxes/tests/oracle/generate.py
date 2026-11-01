#!/usr/bin/env python3
"""Oracle generator for crates/tex-boxes (KC-107).

Reads fixtures.txt, typesets every fixture with MacTeX/TeX Live pdflatex, and
writes expected.txt with exact data extracted from the run:

  * DIM   -- \\wd, \\ht, \\dp of the result box in sp and \\badness
  * DIAG  -- the exact transcript text emitted while building the box
             (over/underfull/tight/loose reports, LaTeX warnings)
  * BOX   -- the \\showbox0 display (\\showboxdepth=\\showboxbreadth=10000)
  * POS   -- \\pdfsavepos positions of every \\POS marker, relative to a
             marker at the reference point of the shipped box (dx, dy with
             dy growing downward, i.e. TeX's cur_v direction)

Fixture syntax (one per line, `#` comments):
    name | setup | body
`body` is a box (e.g. `\\hbox to 50pt{...}`) stored with \\global\\setbox0
inside a group that first runs `setup`. `<<N*text>>` repeats `text` N times.

cargo tests never run TeX; they read expected.txt. Regenerate with:
    python3 crates/tex-boxes/tests/oracle/generate.py
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPEAT = re.compile(r'<<(\d+)\*((?:(?!<<).)*?)>>')


def expand(s):
    # innermost repeats first, so `<<8*...<<5*x>>...>>` nests correctly
    while True:
        t = REPEAT.sub(lambda m: m.group(2) * int(m.group(1)), s)
        if t == s:
            return t
        s = t


def load_fixtures():
    fixtures = []
    with open(os.path.join(HERE, 'fixtures.txt'), encoding='utf-8') as f:
        for raw in f:
            line = raw.rstrip('\n')
            if not line.strip() or line.lstrip().startswith('#'):
                continue
            name, setup, body = [p.strip() for p in line.split(' | ', 2)]
            fixtures.append((name, expand(setup), expand(body)))
    names = [n for n, _, _ in fixtures]
    assert len(names) == len(set(names)), 'duplicate fixture names'
    return fixtures


PREAMBLE = r"""\documentclass{article}
\newwrite\posout
\newsavebox\Ba
\newsavebox\Bb
\newlength\La
\newlength\Lb
\begin{document}
\immediate\openout\posout=\jobname.dat
\showboxdepth=10000 \showboxbreadth=10000
\immediate\write\posout{PARAM baselineskip \number\baselineskip}
\immediate\write\posout{PARAM strutht \number\ht\strutbox}
\immediate\write\posout{PARAM strutdp \number\dp\strutbox}
\setbox1\hbox{$x$}\setbox1\hbox{}
\immediate\write\posout{PARAM axisheight \number\fontdimen22\textfont2}
\immediate\write\posout{PARAM parindent \number\parindent}
\immediate\write\posout{PARAM hsize \number\hsize}
\immediate\write\posout{PARAM fboxsep \number\fboxsep}
\immediate\write\posout{PARAM fboxrule \number\fboxrule}
\immediate\write\posout{PARAM hfuzz \number\hfuzz}
\immediate\write\posout{PARAM lineskip \number\lineskip}
"""


def build_tex(fixtures):
    lines = PREAMBLE.split('\n')
    fixture_line = {}
    for name, setup, body in fixtures:
        lines.append(r'\def\POS{}%')
        lines.append(r'\immediate\write-1{@@BEGIN %s}%%' % name)
        lines.append(r'\begingroup %s\global\setbox0%s\endgroup' % (setup, body))
        fixture_line[name] = len(lines)
        lines.append(r'\immediate\write-1{@@SHOW %s}%%' % name)
        lines.append(r'\showbox0')
        lines.append(r'\immediate\write-1{@@END %s}%%' % name)
        lines.append(r'\immediate\write\posout{DIM %s \number\wd0 \space\number\ht0 \space\number\dp0 \space\the\badness}%%' % name)
        if r'\POS' in body or r'\POS' in setup:
            lines.append(r'\def\POS{\pdfsavepos\write\posout{POS %s \the\pdflastxpos\space\the\pdflastypos}}%%' % name)
            lines.append(r'\immediate\write-1{@@SKIP}%')
            lines.append(r'\begingroup %s\global\setbox0%s\endgroup' % (setup, body))
            lines.append(r'\shipout\hbox{\POS\box0}')
    lines.append(r'\immediate\closeout\posout')
    lines.append(r'\end{document}')
    return '\n'.join(lines) + '\n', fixture_line


def main():
    fixtures = load_fixtures()
    tex, fixture_line = build_tex(fixtures)
    work = tempfile.mkdtemp(prefix='kc107-oracle-')
    try:
        with open(os.path.join(work, 'oracle.tex'), 'w', encoding='utf-8') as f:
            f.write(tex)
        env = dict(os.environ, max_print_line='100000', error_line='254', half_error_line='238')
        subprocess.run(['pdflatex', '-interaction=nonstopmode', 'oracle.tex'], cwd=work, env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
        log = open(os.path.join(work, 'oracle.log'), encoding='latin-1').read()
        dat = open(os.path.join(work, 'oracle.dat'), encoding='latin-1').read().split('\n')
        pdftex_version = subprocess.run(['pdflatex', '--version'], capture_output=True, text=True).stdout.split('\n')[0]
    finally:
        if '--keep' in sys.argv:
            print('kept', work)
        else:
            shutil.rmtree(work, ignore_errors=True)

    params, dims, pos = [], {}, {}
    for line in dat:
        parts = line.split()
        if not parts:
            continue
        if parts[0] == 'PARAM':
            params.append((parts[1], parts[2]))
        elif parts[0] == 'DIM':
            dims[parts[1]] = parts[2:]
        elif parts[0] == 'POS':
            pos.setdefault(parts[1], []).append((int(parts[2]), int(parts[3])))

    out = ['# Generated by generate.py from fixtures.txt -- do not edit.',
           '# Oracle: ' + pdftex_version, '']
    for k, v in params:
        out.append('PARAM %s %s' % (k, v))
    for name, setup, body in fixtures:
        begin = log.index('@@BEGIN %s\n' % name) + len('@@BEGIN %s\n' % name)
        show = log.index('@@SHOW %s\n' % name, begin)
        diag = log[begin:show]
        end = log.index('@@END %s\n' % name, show)
        seg = log[show:end]
        m = re.search(r'> \\box0=(.*?)\n! OK\.', seg, re.S)
        box = m.group(1) if m else '<<missing>>'
        out.append('FIX %s' % name)
        out.append('LINE %d' % fixture_line[name])
        out.append('DIM %s' % ' '.join(dims[name]))
        out.append('DIAG')
        for l in (diag[:-1].split('\n') if diag.endswith('\n') else diag.split('\n')) if diag else []:
            out.append('|' + l)
        out.append('BOX')
        for l in box.strip('\n').split('\n'):
            out.append('|' + l)
        if name in pos:
            x0, y0 = pos[name][0]
            out.append('POS ' + ' '.join('%d,%d' % (x - x0, y0 - y) for x, y in pos[name]))
        out.append('END')
    with open(os.path.join(HERE, 'expected.txt'), 'w', encoding='utf-8') as f:
        f.write('\n'.join(out) + '\n')
    print('fixtures: %d, with positions: %d' % (len(fixtures), len(pos)))


if __name__ == '__main__':
    main()
