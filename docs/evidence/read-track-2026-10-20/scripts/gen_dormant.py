#!/usr/bin/env python3
"""The documents of tests/incremental.rs::a_dormant_line_is_kept_until_it_is_read, to OUTDIR."""
import os
import sys


def para(i, word):
    return (f"Paragraph {i} with the word {word}, and enough text to fill a few lines "
            f"of the page so that the document ships several pages; math $a^{i}+b$.\n\n")


def tail(mid):
    return ''.join(para(i, 'tail') for i in range(80)) + mid + ''.join(para(i, 'tail') for i in range(80, 160))


def doc(pre, mid, end, line):
    body = ''.join(para(i, 'body') for i in range(40))
    more = ''.join(para(i, 'more') for i in range(60))
    return (f"\\documentclass{{article}}\n{pre}"
            f"\\newcommand\\tail[1][]{{#1{tail(mid)}{end}}}\n\\newcommand\\more{{{more}}}\n"
            f"\\begin{{document}}\n{body}\\tail\n{line}\\end{{document}}\n")


words = ''.join(f"\\w{{w{i}}}" for i in range(400))
cases = {
    'end': doc('', '', '', ''),
    'read': doc('', '', '', 'Zword \\more\n'),
    'late': doc('', '', '\\clearpage', 'Z\\input{chunk}\n'),
    'show': doc('', '\\showthe\\count0\n\n', '', ''),
    'input': doc('', '\\input{chunk}\n', '', ''),
    'csname': doc('\\def\\w#1{\\csname @firstofone\\endcsname{#1} }\n', words, '', ''),
}
out = sys.argv[1]
os.makedirs(out, exist_ok=True)
for k, v in cases.items():
    open(os.path.join(out, f'd-{k}.tex'), 'w').write(v)
open(os.path.join(out, 'chunk.tex'), 'w').write(''.join(para(i, 'chunk') for i in range(6)))
