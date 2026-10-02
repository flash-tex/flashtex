"""Source edits for the incremental soundness test that change how the source
is cut into lines and paragraphs, not what it says: a line break moved, a
paragraph split in two, two paragraphs joined. Pure functions on the bytes of
a document; `incr_bench.py` (kinds `newline`, `split`, `join`) applies them at
a prose position and compares the incremental result with a from-scratch run.

Each returns the edited source, or None when the position has nothing to apply
the edit to. None is also returned wherever the edit could change what the
document means (an unbalanced group, a comment, a command line), so a
mismatch with the from-scratch run is a real difference and not a changed
document.
"""
import re


def _word_end(src, p):
    while p < len(src) and chr(src[p]).isalpha():
        p += 1
    return p


def _line_start(src, p):
    return src.rfind(b'\n', 0, p) + 1


def _plain_prose_before(src, q):
    """True when the line holding q starts with a letter and has no comment
    before q, and the text of the paragraph so far has balanced groups."""
    ls = _line_start(src, q)
    if ls >= len(src) or not chr(src[ls]).isalpha():
        return False
    if b'%' in src[ls:q]:
        return False
    para = src.rfind(b'\n\n', 0, q) + 2 if src.rfind(b'\n\n', 0, q) >= 0 else 0
    text = src[para:q]
    return text.count(b'{') == text.count(b'}') and text.count(b'$') % 2 == 0


def newline(src, p):
    """Toggle a line break at the word boundary after p. A space becomes a
    newline (one line is cut in two; TeX reads both as a space), or a newline
    between two prose lines becomes a space (two lines are joined)."""
    q = _word_end(src, p)
    if q + 1 >= len(src) or not _plain_prose_before(src, q):
        return None
    if src[q] == 32 and chr(src[q + 1]).isalpha():
        return src[:q] + b'\n' + src[q + 1:]
    if src[q] == 10 and chr(src[q + 1]).isalpha():
        return src[:q] + b' ' + src[q + 1:]
    return None


def split(src, p):
    """Start a new paragraph inside one: the space after the word at p becomes
    a blank line. Only between two words, with the paragraph's groups and
    math balanced so far, so the paragraph break is legal there."""
    q = _word_end(src, p)
    if q + 1 >= len(src) or src[q] != 32 or not chr(src[q + 1]).isalpha():
        return None
    if not _plain_prose_before(src, q):
        return None
    return src[:q] + b'\n\n' + src[q + 1:]


def join(src, p):
    """Join two paragraphs: the blank line nearest p becomes a space. Only
    between two prose paragraphs inside the document body (the first ends in
    a letter or punctuation, the second starts with a capital letter)."""
    body = src.find(b'\\begin{document}')
    end = src.rfind(b'\\end{document}')
    best = None
    for m in re.finditer(rb'(?<=[A-Za-z.,;:!?)])\n\n(?=[A-Z])', src):
        if not (body < m.start() < end):
            continue
        ls = _line_start(src, m.start())
        pstart = max(src.rfind(b'\n\n', 0, m.start()) + 2, body + len(b'\\begin{document}'))
        before = src[pstart:m.start()].lstrip(b'\n')
        if not before[:1].isalpha() or b'%' in src[ls:m.start()]:
            continue
        if before.count(b'{') != before.count(b'}') or before.count(b'$') % 2:
            continue
        if best is None or abs(m.start() - p) < abs(best.start() - p):
            best = m
    if best is None:
        return None
    return src[:best.start()] + b' ' + src[best.end():]


# Edits that change the meaning on purpose: a line break or a blank line where
# TeX treats it differently (a paragraph break inside math or a table cell, an
# extra line inside verbatim). The document may then fail to compile; the
# from-scratch run fails the same way, and the incremental result (including
# the terminal) must equal it.

def _body(src):
    b = src.find(b'\\begin{document}')
    e = src.rfind(b'\\end{document}')
    return (b if b >= 0 else 0), (e if e > b else len(src))


def _nearest(matches, p):
    return min(matches, key=lambda m: abs(m.start() - p)) if matches else None


def math_par(src, p):
    """A blank line inside inline math: a space between two tokens of the
    nearest `$...$` becomes a paragraph break (`\\par` in math mode)."""
    b, e = _body(src)
    ms = [m for m in re.finditer(rb'(?<![\\$])\$([^$\n]*? [^$\n]*)\$(?!\$)', src) if b < m.start() < e]
    m = _nearest(ms, p)
    if m is None:
        return None
    q = m.start(1) + m.group(1).index(b' ')
    return src[:q] + b'\n\n' + src[q + 1:]


def verbatim_blank(src, p):
    """An extra blank line inside the nearest `verbatim` environment, after
    its first line: the printed text changes."""
    ms = [m for m in re.finditer(rb'\\begin\{verbatim\}\n', src)]
    m = _nearest(ms, p)
    if m is None:
        return None
    nl = src.find(b'\n', m.end())
    if nl < 0 or src.find(b'\\end{verbatim}', m.end()) < nl:
        return None
    return src[:nl + 1] + b'\n' + src[nl + 1:]


def cell_blank(src, p):
    """A blank line inside a table cell: the ` & ` nearest p inside a
    `tabular` becomes ` &` followed by a blank line."""
    b, e = _body(src)
    best = None
    for t in re.finditer(rb'\\begin\{tabular\*?\}.*?\\end\{tabular\*?\}', src, re.S):
        if not (b < t.start() < e):
            continue
        for m in re.finditer(rb' & ', src[t.start():t.end()]):
            q = t.start() + m.start()
            if best is None or abs(q - p) < abs(best - p):
                best = q
    if best is None:
        return None
    return src[:best] + b' &\n\n' + src[best + 3:]
