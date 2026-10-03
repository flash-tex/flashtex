"""Source edits for the incremental soundness test.

Two groups, applied by `incr_bench.py` at a prose position and compared with a
from-scratch run:

* line and paragraph kinds (`newline`, `split`, `join`) change how the source
  is cut into lines and paragraphs, not what it says. They return None
  wherever the edit could change the meaning (verbatim-like environments,
  `\\verb`, comments, any math, alignment environments, an unbalanced group, the
  preamble), so a mismatch with the from-scratch run is a real difference and
  not a changed document.
* context kinds (`math_par`, `verbatim_blank`, `cell_blank`) change the
  meaning on purpose: a paragraph break inside inline math, an extra line inside
  verbatim, a blank line in a table cell. The document may then fail to
  compile; the from-scratch run fails the same way and the incremental result,
  including the terminal, must equal it.

All functions are pure functions of the source bytes. Line endings are kept: a
CRLF file stays CRLF, and a line holding only blanks is a paragraph break.
"""
import re

LINE_KINDS = ('newline', 'split', 'join')
CONTEXT_KINDS = ('math_par', 'verbatim_blank', 'cell_blank')
LETTER_KINDS = ('replace', 'insert', 'delete', 'sentence')
STRUCTURAL_KINDS = ('section', 'label', 'ref', 'cite', 'footnote', 'unlabel', 'unsection')
ALL_KINDS = LETTER_KINDS + STRUCTURAL_KINDS + LINE_KINDS + CONTEXT_KINDS


def parse_kinds(text):
    """The comma-separated `--kinds` value as a list; ValueError on an unknown
    or empty kind (an unknown kind used to run zero trials and look like a pass)."""
    kinds = [k.strip() for k in text.split(',')]
    bad = [k for k in kinds if k not in ALL_KINDS]
    if not kinds or bad:
        raise ValueError('unknown or empty edit kind %s; known kinds: %s' % (bad or [text], ','.join(ALL_KINDS)))
    return kinds


# ----------------------------------------------------------------------
# the context scanner

VERBATIM_ENVS = (b'verbatim', b'verbatim*', b'Verbatim', b'Verbatim*', b'BVerbatim', b'LVerbatim',
                 b'lstlisting', b'minted', b'comment', b'alltt', b'filecontents', b'filecontents*')
MATH_ENVS = tuple(e + s for e in (b'equation', b'align', b'alignat', b'flalign', b'gather', b'multline',
                                   b'eqnarray', b'displaymath', b'math', b'split', b'subequations')
                  for s in (b'', b'*'))
ALIGN_ENVS = (b'tabular', b'tabular*', b'tabularx', b'tabulary', b'array', b'longtable', b'supertabular',
              b'align', b'align*', b'alignat', b'alignat*', b'flalign', b'flalign*', b'matrix', b'pmatrix',
              b'bmatrix', b'cases', b'eqnarray', b'eqnarray*')

_BEGIN = re.compile(rb'\\begin\{([A-Za-z*]+)\}')
_CMD = re.compile(rb'\\([A-Za-z]+)(\*?)')
INLINE_VERB = (b'verb', b'lstinline', b'mintinline', b'path', b'url', b'Verb')


def _brace_end(src, i):
    """Index just past the `}` matching the `{` at i (nesting and escapes aware); len(src) if none."""
    depth = 0
    n = len(src)
    while i < n:
        c = src[i:i + 1]
        if c == b'\\':
            i += 2
            continue
        if c == b'{':
            depth += 1
        elif c == b'}':
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


def _inline_verb_end(src, m):
    """For a match of _CMD that names an inline-verbatim command, the end of
    its verbatim argument, in the forms `\\verb|x|`, `\\lstinline[opts]|x|`,
    `\\lstinline{x}`, `\\mintinline[opts]{lang}{x}`, `\\url{x}`, `\\path|x|`;
    None when the text after the name is not such an argument."""
    n = len(src)
    j = m.end()
    name = m.group(1)
    if name == b'path' and src[j:j + 1] in (b'[', b'('):
        return None  # TikZ's \\path[opts] (x,y) ..., not the path package's \\path|x|
    if name == b'mintinline':
        if src[j:j + 1] == b'[':
            k = src.find(b']', j)
            j = n if k < 0 else k + 1
        if src[j:j + 1] != b'{':
            return None
        j = _brace_end(src, j)
    elif name in (b'lstinline',):
        if src[j:j + 1] == b'[':
            k = src.find(b']', j)
            j = n if k < 0 else k + 1
    c = src[j:j + 1]
    if not c or c.isspace() or c.isalnum():
        return None
    if c == b'{':
        return _brace_end(src, j)
    k = src.find(c, j + 1)
    return n if k < 0 else k + 1


def scan(src):
    """Regions of `src` where a line or paragraph edit is unsafe or the edit is
    a deliberate context change: a list of (kind, start, end), kind one of
    'comment' (a `%` comment up to and including its newline), 'verb'
    (a verbatim-like environment or `\\verb`), 'math' (every math form),
    'inline' (inline math only: `$..$` and `\\(..\\)`), 'align' (an alignment
    environment). Regions can nest (math inside a table). Escapes `\\$`,
    `\\%`, `\\\\` are skipped."""
    out = []
    n = len(src)
    i = 0
    while i < n:
        c = src[i:i + 1]
        if c == b'\\':
            m = _CMD.match(src, i)
            if m and m.group(1) in INLINE_VERB:
                j = _inline_verb_end(src, m)
                if j is not None:
                    out.append(('verb', i, j))
                    i = j
                    continue
            m = _BEGIN.match(src, i)
            if m:
                env = m.group(1)
                if env in VERBATIM_ENVS:
                    j = src.find(b'\\end{' + env + b'}', m.end())
                    j = n if j < 0 else j + len(b'\\end{' + env + b'}')
                    out.append(('verb', i, j))
                    i = j
                    continue
                if env in MATH_ENVS or env in ALIGN_ENVS:
                    j = src.find(b'\\end{' + env + b'}', m.end())
                    j = n if j < 0 else j + len(b'\\end{' + env + b'}')
                    if env in MATH_ENVS:
                        out.append(('math', i, j))
                    if env in ALIGN_ENVS:
                        out.append(('align', i, j))
                    # (look inside for comments and nested math)
                    i = m.end()
                    continue
            nxt = src[i + 1:i + 2]
            if nxt in (b'[', b'('):
                close = b'\\]' if nxt == b'[' else b'\\)'
                j = src.find(close, i + 2)
                j = n if j < 0 else j + 2
                out.append(('math', i, j))
                if nxt == b'(':
                    out.append(('inline', i, j))
                i = j
                continue
            i += 2
            continue
        if c == b'%':
            j = src.find(b'\n', i)
            j = n if j < 0 else j + 1
            out.append(('comment', i, j))
            i = j
            continue
        if c == b'$':
            if src[i + 1:i + 2] == b'$':
                j = src.find(b'$$', i + 2)
                j = n if j < 0 else j + 2
                out.append(('math', i, j))
            else:
                j = i + 1
                while j < n:
                    d = src[j:j + 1]
                    if d == b'\\':
                        j += 2
                        continue
                    if d == b'$':
                        break
                    j += 1
                j = n if j >= n else j + 1
                out.append(('math', i, j))
                out.append(('inline', i, j))
            i = j
            continue
        i += 1
    return out


def _inside(regions, kinds, p):
    return any(k in kinds and s <= p < e for k, s, e in regions)


def _body_start(src):
    b = src.find(b'\\begin{document}')
    return b + len(b'\\begin{document}') if b >= 0 else 0


def _body_end(src):
    e = src.rfind(b'\\end{document}')
    return e if e >= 0 else len(src)


def _eol(src):
    """The file's line ending: CRLF when most lines have it."""
    return b'\r\n' if src.count(b'\r\n') * 2 > src.count(b'\n') else b'\n'


_BREAK = re.compile(rb'(?:[ \t]*\r?\n){2,}')


def _word_end(src, p):
    while p < len(src) and chr(src[p]).isalpha():
        p += 1
    return p


def _para_start(src, q):
    """Where the paragraph holding q starts (after the last paragraph break, or
    after `\\begin{document}`)."""
    start = _body_start(src)
    for m in _BREAK.finditer(src, start, q):
        start = m.end()
    return start


def _safe_prose(src, regions, q):
    """q is in the document body, outside comments, verbatim, math and
    alignment environments, on a line that starts with a letter, with the
    paragraph's groups balanced up to q."""
    if not (_body_start(src) <= q < _body_end(src)):
        return False
    if _inside(regions, ('comment', 'verb', 'math', 'align'), q):
        return False
    ls = src.rfind(b'\n', 0, q) + 1
    if ls >= len(src) or not chr(src[ls]).isalpha():
        return False
    text = src[_para_start(src, q):q]
    return text.count(b'{') == text.count(b'}') and text.count(b'[') == text.count(b']')


# ----------------------------------------------------------------------
# line and paragraph kinds


def newline(src, p):
    """Toggle a line break at the word boundary after p: a space becomes a
    line break, or a line break between two prose lines becomes a space."""
    q = _word_end(src, p)
    regions = scan(src)
    nl = _eol(src)
    if not _safe_prose(src, regions, q):
        return None
    if src[q:q + 1] == b' ' and src[q + 1:q + 2].isalpha():
        return src[:q] + nl + src[q + 1:]
    if src[q:q + len(nl)] == nl and src[q + len(nl):q + len(nl) + 1].isalpha():
        if _inside(regions, ('comment', 'verb', 'math', 'align'), q + len(nl)):
            return None
        return src[:q] + b' ' + src[q + len(nl):]
    return None


def split(src, p):
    """Start a new paragraph inside one: the space after the word at p becomes
    a blank line. Only between two words of plain prose."""
    q = _word_end(src, p)
    if src[q:q + 1] != b' ' or not src[q + 1:q + 2].isalpha():
        return None
    if not _safe_prose(src, scan(src), q):
        return None
    nl = _eol(src)
    return src[:q] + nl + nl + src[q + 1:]


def _mask(regions, kinds, n):
    m = bytearray(n + 1)
    for k, s, e in regions:
        if k in kinds:
            m[s:e] = b'\x01' * (min(e, n) - s)
    return m


def join(src, p):
    """Join two paragraphs: the paragraph break (a blank or blank-only line)
    nearest p becomes a space. Only between two plain-prose paragraphs of the
    body (the first ends in a letter or punctuation, the second starts with a
    capital letter), outside verbatim, math, alignment and comments."""
    regions = scan(src)
    unsafe = _mask(regions, ('comment', 'verb', 'math', 'align'), len(src))
    b, e = _body_start(src), _body_end(src)
    best = None
    for m in _BREAK.finditer(src, b, e):
        if m.start() == 0:
            continue
        last = chr(src[m.start() - 1])
        if not (last.isalpha() or last in '.,;:!?)'):
            continue
        if not src[m.end():m.end() + 1].isupper():
            continue
        if unsafe[m.start() - 1] or unsafe[m.start()] or unsafe[m.end()]:
            continue
        before = src[_para_start(src, m.start()):m.start()].lstrip()
        if (not before[:1].isalpha() or before.count(b'{') != before.count(b'}')
                or before.count(b'[') != before.count(b']')):
            continue
        if best is None or abs(m.start() - p) < abs(best.start() - p):
            best = m
    if best is None:
        return None
    return src[:best.start()] + b' ' + src[best.end():]


# ----------------------------------------------------------------------
# context kinds (they change the meaning on purpose)


def _nearest(items, p, key=lambda x: x):
    return min(items, key=lambda x: abs(key(x) - p)) if items else None


def math_par(src, p):
    """A blank line inside one inline math span (`$..$`, `\\(..\\)`): a space
    between two tokens of the span nearest p becomes a paragraph break."""
    regions = scan(src)
    b, e = _body_start(src), _body_end(src)
    spans = []
    for k, s, t in regions:
        if k != 'inline' or not (b < s < e) or _inside(regions, ('comment', 'verb'), s):
            continue
        inner = src[s:t]
        if inner.startswith(b'$'):
            lo, hi = s + 1, t - 1
        else:
            lo, hi = s + 2, t - 2
        sp = [q for q in range(lo, hi) if src[q:q + 1] == b' ']
        if sp:
            spans.append((s, sp))
    m = _nearest(spans, p, key=lambda x: x[0])
    if m is None:
        return None
    q = m[1][0]
    nl = _eol(src)
    return src[:q] + nl + nl + src[q + 1:]


def verbatim_blank(src, p):
    """An extra blank line inside the nearest `verbatim` environment, after its
    first content line (it needs at least two): the printed text changes."""
    regions = scan(src)
    nl = _eol(src)
    cands = []
    for k, s, t in regions:
        if k != 'verb' or not src.startswith(b'\\begin{verbatim}', s):
            continue
        first = src.find(b'\n', s)
        if first < 0:
            continue
        content = src[first + 1:t - len(b'\\end{verbatim}')]
        second = content.find(b'\n')
        if second < 0 or content.count(b'\n') < 2:
            continue
        cands.append((s, first + 1 + second + 1))
    m = _nearest(cands, p, key=lambda x: x[0])
    if m is None:
        return None
    q = m[1]
    return src[:q] + nl + src[q:]


def cell_blank(src, p):
    """A blank line inside a table cell: the ` & ` nearest p in a tabular
    (outside comments, verbatim and nested math) becomes ` &` followed by a
    blank line."""
    regions = scan(src)
    b, e = _body_start(src), _body_end(src)
    nl = _eol(src)
    best = None
    for k, s, t in regions:
        if k != 'align' or not src.startswith((b'\\begin{tabular', b'\\begin{array'), s) or not (b < s < e):
            continue
        for m in re.finditer(rb' & ', src[s:t]):
            q = s + m.start()
            if _inside(regions, ('comment', 'verb', 'math'), q):
                continue
            if best is None or abs(q - p) < abs(best - p):
                best = q
    if best is None:
        return None
    return src[:best] + b' &' + nl + nl + src[best + 3:]


def apply(kind, src, p):
    """The edit of that kind at p (None when it does not apply)."""
    return globals()[kind](src, p)


def zero_trials_error(compiles, kinds):
    """The message for a run that made no trial (None when it made some): an
    unusable `--kinds` for the document, or no prose position, used to look like
    a pass with 0 mismatches."""
    if compiles:
        return None
    return ('0 trials for --kinds %s: no position or no applicable edit in this document; '
            'a run with no trials is not a pass' % kinds)
