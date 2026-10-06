#!/usr/bin/env python3
"""The DIAG-PARITY corpus: warnings and errors beyond the ERROR-RECOVERY 27.

usage: gen.py [OUT.json]   (default: corpus.json next to this script)

Each case is a small project: {"id", "group", "what", "files": {path: text}}
with `main.tex` the entry. Deterministic; nothing here is engine output.
"""
import json
import os
import sys

PRE = "\\documentclass{article}\n\\usepackage{amsmath}\n\\usepackage{graphicx}\n"
LOREM = ("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor "
         "incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud "
         "exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.")
LONG = "Antidisestablishmentarianismsupercalifragilisticexpialidocious"


def doc(body, pre="", cls=None, amsmath=True):
    head = PRE if cls is None else PRE.replace("{article}", "{" + cls + "}")
    if not amsmath:
        head = head.replace("\\usepackage{amsmath}\n", "")
    return head + pre + "\\begin{document}\n" + body + "\n\\end{document}\n"


CASES = []


def case(id, group, what, body, pre="", files=None, cls=None, amsmath=True):
    f = {"main.tex": doc(body, pre, cls, amsmath)}
    f.update(files or {})
    CASES.append({"id": id, "group": group, "what": what, "files": f})


# --- warnings
case("overfull-hbox", "warning", "one overfull line (an unbreakable word)",
     LOREM + "\n\nShort words then " + LONG + LONG + " and more text follows here.\n\n" + LOREM)
case("overfull-many", "warning", "50 overfull lines in 50 paragraphs",
     "\n\n".join(f"Paragraph {i} has {LONG}{LONG} in it." for i in range(50)))
case("underfull-hbox", "warning", "underfull line: \\\\ twice",
     "Short line.\\\\\n\\\\\nNext.\n\n" + LOREM)
case("overfull-vbox", "warning", "overfull \\vbox: a 3cm rule in a 1cm parbox",
     "Text \\parbox[t][1cm]{3cm}{\\rule{1pt}{3cm}} text.")
case("overfull-alignment", "warning", "overfull line inside a tabular p-column",
     "\\begin{tabular}{p{2cm}}\n" + LONG + " \\\\\n\\end{tabular}")
case("undefined-ref", "warning", "\\ref to a label that does not exist (a near miss exists)",
     "\\section{Intro}\\label{sec:intro}\nSee Section~\\ref{sec:intr}.\n")
case("undefined-cite", "warning", "\\cite of a key the bibliography lacks (a near miss exists)",
     "As shown by \\cite{knuth84}.\n\\begin{thebibliography}{9}\n\\bibitem{knuth1984} D. Knuth. The TeXbook.\n\\end{thebibliography}")
case("undefined-ref-many", "warning", "30 undefined references",
     "\n".join(f"See \\ref{{missing{i}}}." for i in range(30)))
case("multiply-defined-label", "warning", "the same \\label twice",
     "\\section{A}\\label{sec:a}\n\\section{B}\\label{sec:a}\nSee \\ref{sec:a}.")
case("labels-changed", "warning", "\\ref before its \\label (first pass: Rerun)",
     "See~\\ref{sec:later}.\n\\section{Later}\\label{sec:later}\nText.")
case("font-shape", "warning", "font shape substitution (bold small caps, bold italic tt)",
     "\\textsc{\\textbf{Bold Small Caps}} and \\texttt{\\textbf{\\textit{tt}}}.")
case("missing-glyph", "warning", "a character the font lacks (\\char200 in OT1 cmr)",
     "A missing glyph: \\char200.", pre="\\tracinglostchars=2\n")
case("hyperref-token", "warning", "math in a \\section title with hyperref",
     "\\section{The case $x^2$}\nText.", pre="\\usepackage{hyperref}\n")
case("float-too-large", "warning", "a figure taller than the page",
     "\\begin{figure}\\centering\\rule{1cm}{30cm}\\caption{Tall}\\end{figure}\nText.")
case("float-lost", "warning", "a figure inside a \\parbox (Not in outer par mode; Float(s) lost)",
     "Text \\parbox{4cm}{\\begin{figure}x\\end{figure}} text.")
case("empty-bibliography", "warning", "an empty thebibliography",
     "Text.\n\\begin{thebibliography}{9}\n\\end{thebibliography}")
case("include-missing", "warning", "\\include of a file that does not exist (No file chap9.tex)",
     "Text.\n\\include{chap9}\n")
case("unicode-not-set-up", "warning", "a Unicode character LaTeX has no definition for",
     "A check mark: \u2713.")
# --- packages and files
case("package-not-found", "files", "\\usepackage of a package that does not exist",
     "Text.", pre="\\usepackage{nosuchpackage}\n")
case("class-not-found", "files", "\\documentclass typo", "Text.", cls="artcle")
case("figure-missing", "files", "\\includegraphics of a missing file",
     "\\begin{figure}\\centering\\includegraphics[width=3cm]{figs/plot}\\caption{P}\\end{figure}")
case("option-clash", "files", "a package loaded twice with different options",
     "Text.", pre="\\usepackage{xcolor}\n\\usepackage[dvipsnames]{xcolor}\n")
case("usepackage-in-body", "files", "\\usepackage after \\begin{document}",
     "Text.\n\\usepackage{amssymb}\nMore.")
# --- math
case("display-unclosed", "math", "\\[ never closed before a blank line",
     "Before\n\\[ x^2 + y^2\n\nAfter.")
case("dollar-dollar-unclosed", "math", "$$ display closed by a single $",
     "Before $$ x^2 $ after.\n\nNext.")
case("left-no-right", "math", "\\left( without \\right", "We have $\\left( x + y $ here.")
case("right-no-left", "math", "\\right) without \\left", "We have $ x + y \\right) $ here.")
case("missing-rbrace-math", "math", "\\frac{a}{b with the brace closed by $",
     "We have $\\frac{a}{b $ here.\n\nNext.")
case("extra-alignment-tab", "math", "three cells in a two-column tabular",
     "\\begin{tabular}{ll}\na & b & c \\\\\n\\end{tabular}")
case("double-superscript", "math", "x^2^3", "We have $x^2^3$ here.")
case("align-no-amsmath", "math", "align without amsmath", "\\begin{align}x&=1\\end{align}", amsmath=False)
case("mathcmd-in-text-macro", "math", "\\mathbf in text mode, inside a preamble macro",
     "The vector \\vect{v} is long.\n", pre="\\newcommand{\\vect}[1]{\\mathbf{#1}}\n")
# --- structure
case("mismatch-across-files", "structure", "\\begin{itemize} in main, \\end{enumerate} in an \\input file",
     "\\begin{itemize}\n\\input{list}\nText.",
     files={"list.tex": "\\item one\n\\item two\n\\end{enumerate}\n"})
case("error-in-input", "structure", "an undefined command in an \\input file",
     "Intro.\n\\input{chap}\nOutro.",
     files={"chap.tex": "Chapter text.\nHere is \\textbff{bold}.\nMore.\n"})
case("error-in-include", "structure", "an unmatched } in an \\include file",
     "Intro.\n\\include{part}\nOutro.",
     files={"part.tex": "Part text.\nOops } here.\n"})
case("error-in-macro", "structure", "an undefined command inside a preamble macro, used twice",
     "First use: \\note{hello}.\n\nSecond use: \\note{world}.",
     pre="\\newcommand{\\note}[1]{\\textbf{Note:} \\emphh{#1}}\n")
case("error-in-env-def", "structure", "a misplaced & inside a preamble environment, used twice",
     "\\begin{mybox}one\\end{mybox}\n\\begin{mybox}two\\end{mybox}",
     pre="\\newenvironment{mybox}{\\par A & B:}{\\par}\n")
case("bib-syntax", "structure", "a .bib entry missing a comma (BibTeX error)",
     "See \\cite{knuth1984}.\n\\bibliographystyle{plain}\n\\bibliography{refs}",
     files={"refs.bib": "@book{knuth1984,\n  author = {Donald Knuth}\n  title = {The TeXbook},\n  year = 1984\n}\n"})
case("caption-outside-float", "structure", "\\caption outside a float", "Text \\caption{Hi}.")
case("command-already-defined", "structure", "\\newcommand of an existing command",
     "Text.", pre="\\newcommand{\\section}{x}\n")
case("missing-item", "structure", "text directly inside itemize",
     "\\begin{itemize}\nNo item here.\n\\end{itemize}")
case("illegal-unit", "structure", "\\vspace{2} without a unit", "Text.\n\n\\vspace{2}\nMore.")
# --- engine
case("capacity-grouping", "engine", "unbounded group nesting (grouping levels)", "\\def\\x{{\\x}}\\x")
case("capacity-recursion", "engine", "a macro that calls itself and grows",
     "\\loopit", pre="\\newcommand{\\loopit}{a\\loopit b}\n")


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "corpus.json")
    json.dump({"cases": CASES}, open(out, "w"), indent=1)
    print(len(CASES), "cases ->", out)


if __name__ == "__main__":
    main()
