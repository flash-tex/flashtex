#!/usr/bin/env python3
"""Writes the font-family fixtures (NFSS family x series x shape) for
`tests/font_families_oracle.rs`. Each fixture is one short article whose
body sets a two-line paragraph in one font shape between roman words, in
three font set-ups:

  ot1   no fontenc                      (ot1cmr/ot1cmss/ot1cmtt.fd)
  t1    \\usepackage[T1]{fontenc}        (t1cmr/t1cmss/t1cmtt.fd, EC fonts)
  lm    T1 + \\usepackage{lmodern}       (t1lmr/t1lmss/t1lmtt.fd)

plus nesting, declaration, size and substitution fixtures. Run this, then
`oracle.py` (pdflatex, test-only) to regenerate `reference/*.json`.
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))

SCHEMES = {
    "ot1": "",
    "t1": "\\usepackage[T1]{fontenc}\n",
    "lm": "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n",
}

PHRASE = ("Several words set in this shape measure the widths of a whole line "
          "and then continue with more words that wrap onto the following line of text")

SHAPES = [
    ("rm-n", "{%s}"),
    ("rm-it", "\\textit{%s}"),
    ("rm-sl", "\\textsl{%s}"),
    ("rm-sc", "\\textsc{%s}"),
    ("rm-bf", "\\textbf{%s}"),
    ("rm-bfit", "\\textbf{\\textit{%s}}"),
    ("rm-bfsl", "\\textbf{\\textsl{%s}}"),
    ("rm-bfsc", "\\textbf{\\textsc{%s}}"),
    ("sf-n", "\\textsf{%s}"),
    ("sf-it", "\\textsf{\\textit{%s}}"),
    ("sf-bf", "\\textsf{\\textbf{%s}}"),
    ("tt-n", "\\texttt{%s}"),
]

EXTRAS = [
    ("t1", "emph-nesting",
     "Plain words \\emph{emphasis with \\emph{inner upright} words} and "
     "\\textit{italic \\emph{upright inside} again} then \\textsc{Small \\emph{caps emph}} "
     "and \\textsl{slanted \\emph{upright}} to close the paragraph with plain words at the end."),
    ("ot1", "emph-nesting",
     "Plain words \\emph{emphasis with \\emph{inner upright} words} and "
     "\\textsc{Small \\emph{caps emph}} then \\textsf{sans \\emph{oblique} words} "
     "to close the paragraph with plain words at the end of the line."),
    ("ot1", "declarations",
     "Plain {\\sffamily sans words \\bfseries bold sans} then {\\scshape Small Caps Words} "
     "and {\\bf old bold \\it old italic} with {\\ttfamily typewriter words} and "
     "{\\slshape slanted \\upshape upright} words in the end."),
    ("lm", "declarations",
     "Plain {\\sffamily sans words \\bfseries bold sans} then {\\scshape Small Caps Words} "
     "and {\\bf old bold \\it old italic} with {\\ttfamily typewriter words} and "
     "{\\slshape slanted \\upshape upright} words in the end."),
    ("t1", "sans-latex",
     "The \\textsf{\\LaTeX} logo in sans."),
    ("lm", "sizes",
     "Plain {\\small\\textsf{small sans words}} and {\\Large\\textsc{Large Caps}} then "
     "{\\footnotesize\\texttt{footnote typewriter}} and {\\large\\textsl{large slanted}} words."),
    ("ot1", "substitutions",
     "Words \\textsf{\\textbf{\\textit{bold sans italic}}} and \\textbf{\\textsc{Bold Caps}} "
     "then \\textsf{\\textsc{Sans Caps}} and \\texttt{\\textbf{bold typewriter}} to finish."),
    ("t1", "substitutions",
     "Words \\textsf{\\textbf{\\textit{bold sans italic}}} and \\textbf{\\textsc{Bold Caps}} "
     "then \\textsf{\\textsc{Sans Caps}} and \\texttt{\\textbf{bold typewriter}} to finish."),
    ("lm", "substitutions",
     "Words \\textsf{\\textbf{\\textit{bold sans italic}}} and \\textbf{\\textsc{Bold Caps}} "
     "then \\textsf{\\textsc{Sans Caps}} and \\texttt{\\textbf{bold typewriter}} to finish."),
    ("t1", "sans-shapes",
     "Plain \\textsf{Sans \\textsc{caps substituted}} and "
     "\\textsf{\\textsl{sans slanted}} with \\textsf{\\textbf{\\textit{bold sans italic}}} words "
     "and \\texttt{\\textsl{slanted typewriter} \\textit{italic typewriter} \\textsc{Typewriter Caps}} end."),
]


MATH_ALPHABETS = ("Letters $\\mathsf{ABC} + \\mathtt{T}x + \\mathit{diff} + \\mathfrak{g}\\mathfrak{A} "
                  "+ \\mathsf{A}^2$ in math.")

MATH = [
    ("t1", "math-bold", "Vectors $\\mathbf{v} + \\mathbf{Ab}_2 = x$ and $\\mathbf{F}^2$ in bold."),
    ("t1", "math-alphabets", MATH_ALPHABETS),
    ("ot1", "math-alphabets", MATH_ALPHABETS),
    ("lm", "math-alphabets", MATH_ALPHABETS),
]


def document(scheme, body, extra=""):
    return ("\\documentclass{article}\n" + SCHEMES[scheme] + extra + "\\pagestyle{empty}\n"
            "\\begin{document}\n" + body + "\n\\end{document}\n")


def main():
    n = 0
    for scheme in SCHEMES:
        for name, wrap in SHAPES:
            n += 1
            body = "Lead words in roman, then " + (wrap % PHRASE) + " and roman again."
            open(os.path.join(HERE, f"{n:02d}-{scheme}-{name}.tex"), "w").write(document(scheme, body))
    for scheme, name, body in EXTRAS:
        n += 1
        open(os.path.join(HERE, f"{n:02d}-{scheme}-{name}.tex"), "w").write(document(scheme, body))
    for scheme, name, body in MATH:
        n += 1
        open(os.path.join(HERE, f"{n:02d}-{scheme}-{name}.tex"), "w").write(document(scheme, body, "\\usepackage{amssymb}\n"))
    print(n, "fixtures")


if __name__ == "__main__":
    main()
