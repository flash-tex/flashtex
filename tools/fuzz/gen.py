#!/usr/bin/env python3
"""Structure-aware TeX input generator and mutator for fuzzing. MIT licensed.

Deterministic given a seed integer: every choice goes through the passed
random.Random instance. Every output starts with the lockstep prelude
convention (``\\input prelude``) so it runs under -ini -etex.
"""
import random
import re

PREAMBLE = "\\input prelude\n"

CS_RE = re.compile(r"\\[A-Za-z]+")
NUM_RE = re.compile(
    r"-?\d+(?:\.\d+)?\s*(?:pt|pc|in|bp|cm|mm|dd|cc|nd|nc|sp|em|ex)?\b")
BOUNDARY_VALUES = ["0", "1", "-1", "0pt", "1sp", "16383.99999pt",
                   "2147483647"]
CATCODE_VALUES = [1, 2, 3, 4, 6, 7, 8, 10, 11, 12, 13, 14]
WORDS = ["alpha", "beta", "gamma", "delta", "foo", "bar", "mu", "nu",
         "red", "green", "box", "glue"]


def _boundaries(text):
    pts = {0, len(text)}
    for m in CS_RE.finditer(text):
        pts.add(m.start())
        pts.add(m.end())
    return sorted(pts)


def _swap_or_dup_cs(text, rng):
    css = list(CS_RE.finditer(text))
    if len(css) >= 2:
        a, b = rng.sample(range(len(css)), 2)
        spans = [(m.start(), m.end()) for m in css]
        (s1, e1), (s2, e2) = spans[a], spans[b]
        if s1 > s2:
            (s1, e1), (s2, e2) = (s2, e2), (s1, e1)
        new = (text[:s1] + text[s2:e2] + text[e1:s2] + text[s1:e1]
               + text[e2:])
        return new, "swap-cs"
    if len(css) == 1:
        s, e = css[0].start(), css[0].end()
        return text[:e] + text[s:e] + text[e:], "dup-cs"
    return None


def _boundary_number(text, rng):
    nums = list(NUM_RE.finditer(text))
    if not nums:
        return None
    m = rng.choice(nums)
    val = rng.choice(BOUNDARY_VALUES)
    return text[:m.start()] + val + text[m.end():], "num->%s" % val


def _brace(text, rng):
    spots = [i for i, c in enumerate(text) if c in "{}"]
    if spots and rng.random() < 0.5:
        i = rng.choice(spots)
        return text[:i] + text[i + 1:], "del-brace"
    i = rng.choice(_boundaries(text))
    br = rng.choice(["{", "}"])
    return text[:i] + br + text[i:], "ins-brace-%s" % br


def _insert_relax(text, rng):
    i = rng.choice(_boundaries(text))
    tok = rng.choice(["\\relax ", "\\par ", " "])
    return text[:i] + tok + text[i:], "ins-%s" % tok.strip("\\ ")


def _catcode(text, rng):
    letter = rng.choice("abcdefXYZ")
    val = rng.choice(CATCODE_VALUES)
    line = "\\catcode`\\%s=%d\n" % (letter, val)
    head, _, tail = text.partition("\n")
    return head + "\n" + line + tail, "catcode-%s=%d" % (letter, val)


def _wrap_span(text, rng):
    lines = text.splitlines(keepends=True)
    if len(lines) < 3:
        return None
    i = rng.randrange(1, len(lines) - 1)
    j = rng.randrange(i + 1, len(lines))
    kind = rng.choice(["group", "hbox", "vbox"])
    if kind == "group":
        pre, post = "\\begingroup\n", "\\endgroup\n"
    elif kind == "hbox":
        pre, post = "\\setbox0=\\hbox{\n", "}\\lsshipbox0\n"
    else:
        pre, post = "\\setbox0=\\vbox{\n", "}\\lsshipbox0\n"
    return "".join(lines[:i] + [pre] + lines[i:j] + [post] + lines[j:]), \
        "wrap-%s" % kind


def _halign_sep(text, rng):
    if "&" in text and ( "\\cr" not in text or rng.random() < 0.5):
        return text.replace("&", "\\cr", 1), "amp->cr"
    if "\\cr" in text:
        return text.replace("\\cr", "&", 1), "cr->amp"
    return None


_MUTATIONS = (_swap_or_dup_cs, _boundary_number, _brace, _insert_relax,
              _catcode, _wrap_span, _halign_sep)


def mutate_with_info(text, rng):
    """Apply ONE token-level mutation; return (new_text, description)."""
    start = rng.randrange(len(_MUTATIONS))
    for k in range(len(_MUTATIONS)):
        got = _MUTATIONS[(start + k) % len(_MUTATIONS)](text, rng)
        if got is not None:
            return got
    return text + "\\relax\n", "ins-relax-fallback"


def mutate(text, rng):
    """Apply ONE token-level mutation to a seed case; return the new text."""
    return mutate_with_info(text, rng)[0]


def _frag_para(rng):
    words = " ".join(rng.choice(WORDS) for _ in range(rng.randint(2, 8)))
    return ("\\setbox0=\\hbox{%s}\n\\message{gen para wd=\\the\\wd0}\n"
            "\\lsshipbox0\n" % words)


def _frag_glue(rng):
    w = rng.choice(["60pt", "100pt", "3cm"])
    fill = rng.choice(["fil", "fill", "filll"])
    kern = rng.choice(["1pt", "-2pt", "5pt"])
    return ("\\setbox0=\\hbox to %s{\\vrule width10pt height8pt depth2pt"
            "\\hskip 0pt plus 1%s\\kern%s\\vrule width20pt height8pt "
            "depth2pt}\n\\message{gen glue wd=\\the\\wd0}\n\\lsshipbox0\n"
            % (w, fill, kern))


def _frag_halign(rng):
    a = rng.choice(WORDS)
    b = rng.choice(WORDS)
    return ("\\setbox0=\\vbox{\\halign{#\\hfil&#\\hfil\\cr %s&%s\\cr}}\n"
            "\\message{gen halign wd=\\the\\wd0}\n\\lsshipbox0\n" % (a, b))


def _frag_disc(rng):
    return ("\\setbox0=\\hbox{dis\\discretionary{-}{}{}cretionary "
            "%s}\n\\lsshipbox0\n" % rng.choice(WORDS))


def _frag_math(rng):
    v = rng.choice(["x", "y", "z"])
    return ("\\setbox0=\\hbox{$%s\\left(%s+1\\right)$}\n\\lsshipbox0\n"
            % (v, v))


def _frag_def(rng):
    a = rng.choice(WORDS)
    b = rng.choice(WORDS)
    return ("\\def\\gm#1:#2\\gend{#1/#2}\n"
            "\\message{gen def=\\gm %s:%s\\gend}\n" % (a, b))


def _frag_case(rng):
    return ("\\setbox0=\\hbox{\\uppercase{abc} \\lowercase{XYZ}}\n"
            "\\lsshipbox0\n")


def _frag_csname(rng):
    return "\\message{gen cs=\\csname gencs\\endcsname}\n"


_FRAGS = (_frag_para, _frag_glue, _frag_halign, _frag_disc, _frag_math,
          _frag_def, _frag_case, _frag_csname)


def generate(rng):
    """Build a small valid document from a grammar of primitives."""
    frags = rng.sample(_FRAGS, rng.randint(2, 5))
    return PREAMBLE + "".join(f(rng) for f in frags) + "\\end\n"
