#!/usr/bin/env python3
"""Generate ~1000-page test bodies for the galley-vs-pagination split.

Usage: gen.py BODY NPARAS PARAS_PER_SECTION CHUNK
  BODY  : a (prose) | b (prose+math) | c (b + footnotes + siunitx \\qty)
  CHUNK : 0 -> plain body; N>0 -> body wrapped in timing chunks of N paragraphs
Writes body-<BODY>-<CHUNK>.inc to stdout-named file.

Chunk boundaries are only placed at paragraph boundaries. Every section heading
starts a new chunk (so \\@afterheading's local \\everypar stays inside one group).
Deterministic (seeded).
"""
import random, sys

WORDS = ("lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod "
         "tempor incididunt ut labore et dolore magna aliqua ut enim ad minim veniam "
         "quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo "
         "consequat").split()
UNITS = [r"\metre", r"\kilo\gram", r"\second", r"\metre\per\second",
         r"\newton", r"\joule", r"\kelvin", r"\milli\ampere", r"\hertz", r"\volt"]


def words(r, n):
    return " ".join(r.choice(WORDS) for _ in range(n))


def paragraph(r, body, i, state):
    """Return the TeX source of paragraph i (no trailing blank line)."""
    parts = [words(r, r.randint(90, 115))]
    parts.append(r" with $x_{%d}^2+\frac{a}{b}=\sum_{i=1}^n c_i$." % (i % 97))
    if body in "bc":
        # extra inline math inside the running text
        parts.insert(1, r" where $\alpha_{%d}=\int_0^1 f(t)\,dt$ and $y=\sqrt{%d}$ hold" % (i % 13, i % 7 + 2))
        k = i % 6
        if k == 1:
            state["eq"] += 1
            lab = "eq:%d" % state["eq"]
            state["labels"].append(lab)
            parts.append("\n\\begin{equation}\\label{%s}\n\\int_0^\\infty e^{-x^2}\\,dx=\\frac{\\sqrt\\pi}{2}+\\sum_{k=0}^{%d}\\binom{n}{k}\n\\end{equation}\n%s"
                         % (lab, i % 9 + 1, words(r, r.randint(20, 35)) + "."))
        elif k == 3:
            parts.append("\n\\begin{align}\nf(x) &= \\sum_{n=0}^\\infty \\frac{f^{(n)}(0)}{n!}x^n \\\\\ng(x) &= \\prod_{j=1}^{%d} (x-\\lambda_j) \\nonumber\n\\end{align}\n%s"
                         % (i % 5 + 2, words(r, r.randint(20, 35)) + "."))
        elif k == 5 and state["labels"]:
            parts.append(" As shown in \\eqref{%s}, %s." % (r.choice(state["labels"][-20:]), words(r, 12)))
    if body == "c":
        # ~5 \qty per page and ~1 footnote per page; paragraphs are ~1/5 page
        # here (calibrated), so 1 \qty per paragraph and a footnote every 5th.
        parts.insert(1, r" measured at \qty{%d.%d}{%s}" % (i % 50, i % 10, UNITS[i % len(UNITS)]))
        if i % 5 == 2:
            parts.insert(2, r"\footnote{%s \qty{%d}{%s}.}" % (words(r, r.randint(10, 25)).capitalize(), i % 100, UNITS[(i + 3) % len(UNITS)]))
    return "".join(parts)


def main():
    body, npar, per_sec, chunk = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
    r = random.Random(12345)
    state = {"eq": 0, "labels": []}
    items = []  # (is_section_start, text)
    sec = 0
    for i in range(npar):
        if i % per_sec == 0:
            sec += 1
            items.append((True, "\\section{Section %d}\\label{sec:%d}\n\n%s" % (sec, sec, paragraph(r, body, i, state))))
        else:
            items.append((False, paragraph(r, body, i, state)))
    out = []
    if chunk == 0:
        for _, t in items:
            out.append(t + "\n\n")
    else:
        n_in = 0
        open_ = False
        for sec_start, t in items:
            if open_ and (sec_start or n_in >= chunk):
                out.append("\\chunkend\n")
                open_ = False
            if not open_:
                out.append("\\chunkbeginS\n" if sec_start else "\\chunkbeginP\n")
                open_ = True
                n_in = 0
            out.append(t + "\n\n")
            n_in += 1
        out.append("\\chunkend\n")
    name = "body-%s-%d.inc" % (body, chunk)
    with open(name, "w") as f:
        f.write("".join(out))
    print(name)


if __name__ == "__main__":
    main()
