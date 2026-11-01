#!/usr/bin/env python3
"""Deterministic beamer decks for T7 (lane BEAMER-LATENCY, DESIGN.md §1.2, §8 T7).

usage: genbeamer.py OUTDIR
Writes, for DOC in beamer-5 and beamer-30, OUTDIR/docs/DOC/main.tex and OUTDIR/docs/DOC/t7.json:

  beamer-5   the owner's 2026-10-06 deck: \\documentclass{beamer}, no other package, a title page
             and four simple frames (an itemize, a prose line, a one-line frame, a ten-item list).
  beamer-30  thirty frames with a theme (Madrid: headline, footline, navigation), sections (the
             `.nav`, `.snm`, `.toc` files and a table of contents), overlays (`\\pause`, `<+->`,
             `\\only`, `\\uncover`), blocks, columns and display math.

A beamer frame's body is collected as a macro argument and typeset at `\\end{frame}`, so every
glyph of a frame carries the `\\end{frame}` line, and the prose-line search of dl3-keys
(`--at`) never finds a line on one page. `t7.json` gives t7.py the line to type in and its
watched (0-based) page instead, for each `--at` fraction: {"lines": {"0.02": [LINE, PAGE], ...}}.
The lines are frame-body prose lines of plain lowercase words (the letter, sentence, newline
and split edits all apply), in frames without overlays, so each is on exactly one page."""
import json
import os
import sys

WORDS = ("lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor "
         "incididunt labore dolore magna aliqua enim minim veniam quis nostrud "
         "exercitation ullamco laboris nisi aliquip commodo consequat").split()


def words(k, n):
    """n words, deterministic in k."""
    return ' '.join(WORDS[(k * 7 + i * 3) % len(WORDS)] for i in range(n))


def beamer5():
    """The owner's deck, with frame 3's first body line a prose line to type in."""
    lines = [
        r'\documentclass{beamer}',
        '',
        r'\title{test-flashtex-slides}',
        r'\author{}',
        r'\date{\today}',
        '',
        r'\begin{document}',
        r'\begin{frame}',
        r'\titlepage',
        r'\end{frame}',
        '',
        r'\begin{frame}{Overview}',
        r'\begin{itemize}',
        r'    \item First idea',
        r'    \item Second idea',
        r'\end{itemize}',
        r'\end{frame}',
        '',
        r'\begin{frame}{Next steps}',
        r'Add the next result here before the meeting tomorrow.',
        r'Then write down what we learned from the first idea.',
        '',
        r'Keep the second idea for later.',
        r'\end{frame}',
        '',
        r'\begin{frame}{my frame}',
        r'    Some words about this frame and what comes after it.',
        r'\end{frame}',
        '',
        r'\begin{frame}{hello}',
        r'    \begin{itemize}',
    ]
    lines += [rf'        \item item {i}' for i in range(1, 11)]
    lines += [r'    \end{itemize}', r'\end{frame}', '', r'\end{document}', '']
    text = '\n'.join(lines)
    at = lambda s: text.split('\n').index(s) + 1  # noqa: E731
    # pages: title 0, Overview 1, Next steps 2, my frame 3, hello 4
    edit = {'0.02': [at('Add the next result here before the meeting tomorrow.'), 2],
            '0.5': [at('Then write down what we learned from the first idea.'), 2],
            '0.98': [at('    Some words about this frame and what comes after it.'), 3]}
    return text, edit


def beamer30():
    """Thirty frames: a theme, sections, overlays, blocks, columns, math."""
    out = [
        r'\documentclass{beamer}',
        r'\usetheme{Madrid}',
        '',
        r'\title{A Thirty Frame Deck}',
        r'\subtitle{for the typing benchmark}',
        r'\author{Flash \TeX}',
        r'\institute{Benchmarks}',
        r'\date{\today}',
        '',
        r'\begin{document}',
        r'\begin{frame}',
        r'\titlepage',
        r'\end{frame}',
        '',
        r'\begin{frame}{Outline}',
        r'\tableofcontents',
        r'\end{frame}',
        '',
    ]
    page = 2           # the next frame's first (0-based) page
    plain = {}         # frame number -> (line of its prose line, its page)
    frame = 3          # the title page and the outline are frames 1 and 2
    for sec in range(1, 8):
        out += [rf'\section{{Section {sec}: {words(sec, 2)}}}', '']
        for j in range(4):
            if frame > 30:
                break
            k = frame
            kind = k % 4
            out.append(rf'\begin{{frame}}{{Frame {k}: {words(k, 3)}}}')
            if kind == 0:
                # overlays: three slides
                out += [r'\begin{itemize}[<+->]',
                        rf'\item {words(k, 6)}',
                        rf'\item {words(k + 1, 6)}',
                        rf'\item {words(k + 2, 6)}',
                        r'\end{itemize}']
                slides = 3
            elif kind == 1:
                # plain prose: the line to type in
                plain[k] = (len(out) + 1, page)
                out += [f'{words(k, 14)}.',
                        f'{words(k + 3, 12)}.',
                        '',
                        r'\begin{block}{' + words(k, 2) + '}',
                        f'{words(k + 5, 10)}.',
                        r'\end{block}']
                slides = 1
            elif kind == 2:
                # \pause and display math: two slides
                out += [f'{words(k, 10)}.',
                        r'\pause',
                        r'\[ \sum_{i=1}^{n} i = \frac{n(n+1)}{2} \]',
                        r'\only<2>{' + words(k + 1, 5) + '}']
                slides = 2
            else:
                # columns, \uncover: two slides
                out += [r'\begin{columns}',
                        r'\column{0.5\textwidth}',
                        f'{words(k, 8)}.',
                        r'\column{0.5\textwidth}',
                        r'\uncover<2>{' + words(k + 2, 8) + '.}',
                        r'\end{columns}']
                slides = 2
            out += [r'\end{frame}', '']
            page += slides
            frame += 1
    out += [r'\end{document}', '']
    text = '\n'.join(out)
    ks = sorted(plain)
    pick = lambda f: plain[ks[min(len(ks) - 1, int(len(ks) * f))]]  # noqa: E731
    edit = {f: list(pick(float(f))) for f in ('0.02', '0.5', '0.98')}
    return text, edit


DECKS = {'beamer-5': beamer5, 'beamer-30': beamer30}


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
    for name, f in DECKS.items():
        text, edit = f()
        d = f'{base}/docs/{name}'
        os.makedirs(d, exist_ok=True)
        open(f'{d}/main.tex', 'w').write(text)
        json.dump({'lines': edit}, open(f'{d}/t7.json', 'w'), indent=1, sort_keys=True)


if __name__ == '__main__':
    main()
