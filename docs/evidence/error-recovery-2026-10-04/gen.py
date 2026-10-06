#!/usr/bin/env python3
"""The ERROR-RECOVERY corpus: mid-typing and common LaTeX errors (lane ERROR-RECOVERY).

usage: gen.py [OUT.json]   (default: corpus.json next to this script)

Two deterministic base documents, `short` (about 4 pages) and `long` (about 60),
both correct. Each case names a base and a list of edits; an edit replaces the
first occurrence of `find` with `replace`. Applying a case's edits to its base
gives the erroneous document; applying them in reverse restores the base (the
"fix" step). `marker` is a word on the page right after the error point, so a
characterisation can tell whether pages after the error still update.
"""
import json
import os
import random
import sys

WORDS = ("lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor "
         "incididunt ut labore et dolore magna aliqua enim ad minim veniam quis nostrud "
         "exercitation ullamco laboris nisi aliquip ex ea commodo consequat").split()


def para(r, k):
    n = r.randint(80, 110)
    s = " ".join(r.choice(WORDS) for _ in range(n))
    s = s[0].upper() + s[1:]
    mid = s.index(" ", len(s) // 2)
    return s[:mid] + f" with $x_{{{k % 17}}}^2+\\frac{{a}}{{b}}$" + s[mid:] + ".\n"


def base(pages, seed):
    r = random.Random(seed)
    out = [
        "\\documentclass{article}",
        "\\usepackage{amsmath}",
        "\\usepackage{graphicx}",
        "%@PREAMBLE",
        "\\begin{document}",
        "",
    ]
    paras = int(pages * 6.1)
    sec = 0
    for k in range(paras):
        if k % 6 == 0:
            sec += 1
            out.append(f"\\section{{Part {sec}}}")
        out.append(para(r, k))
        # error points: after the first paragraph (page 1), and on page ~50 of long
        if k == 1:
            out.append("%@P1")
        if k == int(50 * 6.1):
            out.append("%@P50")
    out.append("%@END")
    out.append("\\end{document}")
    return "\n".join(out) + "\n"


def cases():
    ins = lambda at, text: {"find": at, "replace": at + "\n" + text}
    return [
        # id, base, edits, what it models
        ("unclosed-brace", "short", [ins("%@P1", "Some \\textbf{bold text")], "an unclosed { in an argument (mid-typing)"),
        ("unclosed-group", "short", [ins("%@P1", "Some {\\bfseries bold text")], "an unclosed { group"),
        ("stray-close-brace", "short", [ins("%@P1", "Some text } more text.")], "a stray }"),
        ("missing-dollar", "short", [ins("%@P1", "The value $x^2 + y is large.\n")], "an unclosed $ (inline math)"),
        ("math-outside-math", "short", [ins("%@P1", "The value x^2 is large.\n")], "math syntax outside math (Missing $ inserted)"),
        ("unclosed-display", "short", [ins("%@P1", "\\[ x = y\n\nMore text.\n")], "an unclosed \\[ display"),
        ("undefined-cs", "short", [ins("%@P1", "A \\foo command.\n")], "an undefined control sequence"),
        ("undefined-env", "short", [ins("%@P1", "\\begin{foo}x\\end{foo}\n")], "an undefined environment"),
        ("itemize-no-end", "short", [ins("%@P1", "\\begin{itemize}\n\\item One\n")], "\\begin{itemize} with no \\end"),
        ("mismatched-end", "short", [ins("%@P1", "\\begin{itemize}\n\\item One\n\\end{enumerate}\n")], "a mismatched \\end"),
        ("frac-incomplete", "short", [ins("%@P1", "A fraction $\\frac{a}{$ here.\n")], "\\frac{a}{ incomplete (mid-typing)"),
        ("section-incomplete", "short", [ins("%@P1", "\\section{Intro\n")], "\\section{ incomplete (mid-typing)"),
        ("begin-incomplete", "short", [ins("%@P1", "\\begin{ite\n")], "\\begin{ite (mid-typing)"),
        ("usepackage-typo", "short", [{"find": "\\usepackage{amsmath}", "replace": "\\usepackage{amsmth}"}], "a typo in \\usepackage{amsmth}"),
        ("input-missing", "short", [ins("%@P1", "\\input{chapter-missing}\n")], "a missing \\input file"),
        ("graphic-missing", "short", [ins("%@P1", "\\includegraphics{figure-missing}\n")], "a missing \\includegraphics file"),
        ("ampersand", "short", [ins("%@P1", "Salt & pepper.\n")], "an & outside a table"),
        ("newline-bad", "short", [ins("%@P1", "\n\\\\\nText after.\n")], "\\\\ where there is no line to end"),
        ("item-outside-list", "short", [ins("%@P1", "\\item Lonely.\n")], "\\item outside a list"),
        ("runaway-argument", "short", [ins("%@P1", "\\emph{abc\n\ndef}\n")], "a paragraph end inside an argument"),
        ("end-document-deleted", "short", [{"find": "\\end{document}", "replace": ""}], "\\end{document} deleted"),
        ("file-ended-in-argument", "short", [ins("%@END", "\\textbf{x")], "file ended while scanning an argument"),
        ("preamble-error", "short", [{"find": "%@PREAMBLE", "replace": "%@PREAMBLE\n\\foo"}], "an error in the preamble"),
        ("capacity-exceeded", "short", [ins("%@P1", "\\def\\loopy{\\loopy x}\\loopy\n")], "a fatal error: TeX capacity exceeded"),
        ("long-error-page1", "long", [ins("%@P1", "A \\foo command.\n")], "an error on page 1 of 60"),
        ("long-error-page50", "long", [ins("%@P50", "A \\foo command.\n")], "an error on page 50 of 60"),
        ("long-fatal-page50", "long", [ins("%@P50", "\\input{chapter-missing}\n")], "a fatal error on page 50 of 60"),
    ]


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "corpus.json")
    bases = {"short": base(4, 1), "long": base(60, 2)}
    cs = []
    for cid, b, edits, what in cases():
        for e in edits:
            assert e["find"] in bases[b], (cid, e["find"])
        cs.append({"id": cid, "base": b, "edits": edits, "what": what})
    with open(out, "w") as f:
        json.dump({"bases": bases, "cases": cs}, f, indent=1)
        f.write("\n")
    print(f"{out}: {len(cs)} cases")


if __name__ == "__main__":
    main()
