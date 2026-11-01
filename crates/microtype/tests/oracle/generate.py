#!/usr/bin/env python3
"""Regenerate the pdfTeX/microtype oracle fixtures for crates/microtype.

MacTeX pdflatex is used here as an ORACLE ONLY. `cargo test` never runs TeX:
it reads the committed files under `fixtures/` and `expected/`.

For every fixture this script writes `fixtures/<name>.tex`, runs pdflatex on a
scratch copy and extracts, from the log and a `\\write` dump:

  expected/<name>.txt   sections, each introduced by a `@@<section>` line:
    @@params     paragraph parameters (sp / integers) as pdfTeX saw them
    @@font       one block per font: name, quad, size, then 256 lines
                 `code width lpcode rpcode efcode` (sp / integers)
    @@probe      \\showbox of one-glyph paragraphs that reveal each font's
                 expansion limits (`(+20)` / `(-20)`), or nothing if unexpanded
    @@list1      \\showlists of the unbroken paragraph (pass-1 hlist)
    @@list2      \\showbox of the same paragraph set in one line with
                 \\pretolerance=-1 (pass-2 hlist, automatic discretionaries in)
    @@result     \\showbox of the broken paragraph (the thing we must reproduce)

Usage:  python3 crates/microtype/tests/oracle/generate.py [--pdflatex PATH]
Requires: pdflatex (TeX Live 2026 / pdfTeX 1.40.27 was used), microtype v3.2d.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
FIX = os.path.join(HERE, "fixtures")
EXP = os.path.join(HERE, "expected")

TEXTS = {
    "fox": r"""The quick brown fox jumps over the lazy dog. Typography, when done
well, is invisible; when done poorly, it distracts the reader from the text
itself. Margins, punctuation---and even hyphens---can protrude slightly into
the margin, which makes the edge look straighter.""",
    "proof": r"""In mathematics, a proof is a deductive argument for a mathematical
statement, showing that the stated assumptions logically guarantee the
conclusion. The argument may use other previously established statements,
such as theorems; but every proof can, in principle, be constructed using
only certain basic or original assumptions known as axioms.""",
    "hw": r"""Unless a problem explicitly says otherwise, every answer must be
justified by a proof. A counterexample must satisfy all of the hypotheses and
must be checked explicitly. You may cite results proved in lecture or
recitation, provided that you identify the result you are using.""",
    "quotes": r"""``Well,'' she said, ``it's \emph{not} that simple.'' The
\textbf{committee's} well-known report (published in 1987) argued---rather
convincingly---that ``efficiency'' and ``fairness'' were not, in fact,
opposed; \emph{Victory} was, they wrote, ``a matter of taste.''""",
    "dates": r"""Solutions to the following six exercises and optional bonus
problem are to be submitted to Gradescope by 11 PM on Wednesday, the 2nd of
September, 2026. Late submissions, however well-intentioned, will receive 0
points; no exceptions will be made. Try (again) tomorrow!""",
    "compound": r"""Self-evident, state-of-the-art, and/or long-term: these
compound words, together with dis\-cre\-tion\-ary hyphens, test how pdfTeX
treats hyphens at the right margin. Yes---really. Why? Because ``T'' and
``V'' and ``W'' and ``Y'' all protrude, as do 1, 4 and 7.""",
}

# (combo id, text key, class size, font family, hsize, parindent default?)
COMBOS = [
    ("c1", "fox", 10, "ec", "200pt", False),
    ("c2", "proof", 11, "ec", "250pt", True),
    ("c3", "hw", 11, "ec", "469.75499pt", False),
    ("c4", "quotes", 12, "ec", "220pt", False),
    ("c5", "dates", 11, "lm", "300pt", False),
    ("c6", "compound", 10, "lm", "160pt", False),
    ("c7", "proof", 12, "lm", "345pt", True),
    ("c8", "foxhw", 10, "ec", "120pt", False),
]

MODES = {
    "none": None,
    "pr": "protrusion=true,expansion=false",
    "ex": "protrusion=false,expansion=true",
    "both": "protrusion=true,expansion=true",
}

PARAMS = [
    ("hsize", r"\number\hsize"),
    ("pretolerance", r"\number\pretolerance"),
    ("tolerance", r"\number\tolerance"),
    ("emergencystretch", r"\number\emergencystretch"),
    ("linepenalty", r"\number\linepenalty"),
    ("hyphenpenalty", r"\number\hyphenpenalty"),
    ("exhyphenpenalty", r"\number\exhyphenpenalty"),
    ("adjdemerits", r"\number\adjdemerits"),
    ("doublehyphendemerits", r"\number\doublehyphendemerits"),
    ("finalhyphendemerits", r"\number\finalhyphendemerits"),
    ("looseness", r"\number\looseness"),
    ("lastlinefit", r"\number\lastlinefit"),
    ("parindent", r"\number\parindent"),
    ("leftskip", r"\the\leftskip"),
    ("rightskip", r"\the\rightskip"),
    ("parfillskip", r"\the\parfillskip"),
    ("pdfadjustspacing", r"\number\pdfadjustspacing"),
    ("pdfprotrudechars", r"\number\pdfprotrudechars"),
]


def text_for(key):
    if key == "foxhw":
        return TEXTS["fox"] + "\n" + TEXTS["hw"]
    return TEXTS[key]


def tex_source(combo, mode):
    cid, tkey, size, fam, hsize, indent = combo
    pkg = MODES[mode]
    lines = [
        r"\documentclass[%dpt]{article}" % size,
        r"\usepackage[T1]{fontenc}",
        r"\usepackage[utf8]{inputenc}",
    ]
    if fam == "lm":
        lines.append(r"\usepackage{lmodern}")
    if pkg is not None:
        lines.append(r"\usepackage[%s]{microtype}" % pkg)
    params = r"\space ".join(
        r"%s=%s" % (k, v) for k, v in PARAMS
    )
    lines += [
        r"\showboxdepth=10000 \showboxbreadth=1000000 \errorcontextlines=-1",
        r"\makeatletter",
        r"\newwrite\mtdump",
        r"\def\mtdumpfont{\edef\mtname{\expandafter\string\the\font}%",
        r"  \immediate\write\mtdump{@@font \mtname\space quad=\number\fontdimen6\font\space size=\number\pdffontsize\font}%",
        r"  \count@=0 \loop",
        r"  \immediate\write\mtdump{\number\count@\space\number\fontcharwd\font\count@\space\number\lpcode\font\count@\space\number\rpcode\font\count@\space\number\efcode\font\count@}%",
        r"  \advance\count@ 1 \ifnum\count@<256 \repeat}",
        r"\def\mtprobe#1{%",
        r"  \setbox4=\vbox{\pdfprotrudechars=0 \hsize=500pt \parindent=0pt \parfillskip=0pt \leftskip=0pt \rightskip=0pt \pretolerance=-1 \tolerance=10000 #1M\par}\showbox4",
        r"  \setbox4=\vbox{\pdfprotrudechars=0 \hsize=1pt \parindent=0pt \parfillskip=0pt \leftskip=0pt \rightskip=0pt \pretolerance=-1 \tolerance=10000 #1M\par}\showbox4",
        r"  {#1\mtdumpfont}}",
        r"\makeatother",
        r"\begin{document}",
        r"\immediate\openout\mtdump=\jobname.dump",
    ]
    body = text_for(tkey)
    pi = "" if indent else r"\parindent=0pt "
    lines += [
        r"\setbox2=\vbox{\hsize=\maxdimen \pretolerance=-1 %s%%" % pi,
        body + r"\par}\showbox2",
        r"\setbox0=\vbox{\hsize=%s %s%%" % (hsize, pi),
        r"\immediate\write\mtdump{@@params %s}%%" % params,
        body,
        r"\showlists\par}\showbox0",
        r"\mtprobe{\normalfont}\mtprobe{\itshape}\mtprobe{\bfseries}",
        r"\immediate\closeout\mtdump",
        r"\end{document}",
        "",
    ]
    return "\n".join(lines)


def extract(log, dump):
    out = []
    out.extend(l for l in dump.splitlines() if l.startswith("@@params"))
    lines = log.splitlines()
    i = 0
    boxes = []
    list1 = None
    while i < len(lines):
        l = lines[i]
        if l.startswith("### horizontal mode entered"):
            j = i + 1
            blk = []
            while not lines[j].startswith("spacefactor"):
                blk.append(lines[j])
                j += 1
            list1 = blk
            i = j
        elif re.match(r"^> \\box\d+=$", l):
            num = l[6:-1]
            j = i + 1
            blk = []
            while j < len(lines) and lines[j] != "" and not lines[j].startswith("! OK"):
                blk.append(lines[j])
                j += 1
            boxes.append((num, blk))
            i = j
        i += 1
    assert list1 is not None, "no \\showlists output"
    by = {}
    for num, blk in boxes:
        by.setdefault(num, []).append(blk)
    assert len(by["2"]) == 1 and len(by["0"]) == 1, "unexpected box dumps"
    out.append("@@list1")
    out.extend(list1)
    out.append("@@list2")
    out.extend(by["2"][0])
    out.append("@@result")
    out.extend(by["0"][0])
    for blk in by.get("4", []):
        out.append("@@probe")
        out.extend(blk)
    # font tables last (largest)
    cur = None
    for l in dump.splitlines():
        if l.startswith("@@font"):
            cur = l
            out.append(l)
        elif cur is not None and not l.startswith("@@"):
            f = l.split()
            # keep only rows carrying information
            if any(x != "0" for x in f[1:4]) or f[4] != "1000":
                out.append(l)
    return "\n".join(out) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pdflatex", default=shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex")
    args = ap.parse_args()
    env = dict(os.environ, max_print_line="1000000", error_line="254", half_error_line="238")
    os.makedirs(FIX, exist_ok=True)
    os.makedirs(EXP, exist_ok=True)
    names = []
    with tempfile.TemporaryDirectory() as tmp:
        for combo in COMBOS:
            for mode in MODES:
                name = "%s-%s-%dpt-%s-%s" % (combo[0], combo[3], combo[2], combo[1], mode)
                src = tex_source(combo, mode)
                with open(os.path.join(FIX, name + ".tex"), "w") as fh:
                    fh.write(src)
                with open(os.path.join(tmp, name + ".tex"), "w") as fh:
                    fh.write(src)
                subprocess.run(
                    # no -halt-on-error: every \showbox reports "! OK."
                    [args.pdflatex, "-interaction=nonstopmode", name + ".tex"],
                    cwd=tmp, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                )
                with open(os.path.join(tmp, name + ".log"), encoding="latin-1") as fh:
                    log = fh.read()
                with open(os.path.join(tmp, name + ".dump"), encoding="latin-1") as fh:
                    dump = fh.read()
                with open(os.path.join(EXP, name + ".txt"), "w") as fh:
                    fh.write(extract(log, dump))
                names.append(name)
                print("ok", name)
    print(len(names), "fixtures")


if __name__ == "__main__":
    sys.exit(main())
