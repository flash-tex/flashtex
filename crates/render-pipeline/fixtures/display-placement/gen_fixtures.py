#!/usr/bin/env python3
"""Generate the display-placement oracle fixtures (`fixtures/*.tex`).

Each fixture exercises one of TeX's display rules in running text (TeXbook
ch. 19, tex.web §§1199-1206) or an amsmath/LaTeX layer on top of them:
short vs normal skips from `\\predisplaysize`, equation numbers (right,
`leqno`, moved to their own line), `fleqn`, amsmath numbering
(`\\tag`, `\\notag`, `\\eqref`, `\\numberwithin`, `subequations`), displays in
lists, after headings and at page breaks, `$$...$$`, 10/11/12pt.

    python3 gen_fixtures.py        # rewrites fixtures/*.tex
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "fixtures")

LONG = ("This paragraph line is long enough that it reaches the right margin of the "
        "text block so the display gets the normal skip before it and")
SHORT = "Short line:"
FILL = ("Filler text that runs across several lines so that the page is mostly full "
        "before the display we care about arrives at the page break. ")
WIDE = ("a_1 + a_2 + a_3 + a_4 + a_5 + a_6 + a_7 + a_8 + a_9 + a_{10} + a_{11} + "
        "a_{12} + a_{13} + a_{14} + a_{15} + a_{16} = b")


def doc(body, size="10pt", opts="", pre="", fontenc=True):
    classopts = ",".join(o for o in (size, opts) if o)
    lines = [f"\\documentclass[{classopts}]{{article}}"]
    if fontenc:
        lines += ["\\usepackage[T1]{fontenc}", "\\usepackage{lmodern}"]
    lines.append("\\usepackage{amsmath}")
    if pre:
        lines.append(pre)
    lines += ["\\pagestyle{empty}", "\\begin{document}", body.strip(), "\\end{document}", ""]
    return "\n".join(lines)


def eq(body, env="equation"):
    return f"\\begin{{{env}}} {body} \\end{{{env}}}"


F = {}
F["01-short-line-bracket"] = doc(f"{SHORT}\n\\[ a + b = c \\]\nafter the display.")
F["02-long-line-bracket"] = doc(f"{LONG}\n\\[ a + b = c \\]\nafter the display.")
F["03-par-start-bracket"] = doc("A paragraph before.\n\n\\[ a + b = c \\]\nafter the display.")
F["04-equation-short"] = doc(f"{SHORT}\n{eq('x^2 + y^2 = z^2')}\nafter.")
F["05-equation-long"] = doc(f"{LONG}\n{eq('x^2 + y^2 = z^2')}\nafter.")
F["06-equation-leqno"] = doc(f"{SHORT}\n{eq('x^2 + y^2 = z^2')}\n{LONG}\n{eq('a = b')}\nafter.", opts="leqno")
F["07-equation-fleqn"] = doc(f"{SHORT}\n{eq('x^2 + y^2 = z^2')}\n{LONG}\n\\[ a = b \\]\nafter.", opts="fleqn")
F["08-fleqn-leqno"] = doc(f"{SHORT}\n{eq('x^2 + y^2 = z^2')}\nafter.", opts="fleqn,leqno")
F["09-long-equation-number-below"] = doc(f"{SHORT}\n{eq(WIDE)}\nafter.")
F["10-long-equation-leqno-above"] = doc(f"{SHORT}\n{eq(WIDE)}\nafter.", opts="leqno")
F["11-dollars-short"] = doc(f"{SHORT}\n$$ a + b = c $$\nafter the display.")
F["12-dollars-eqno"] = doc(f"{LONG}\n$$ a + b = c \\eqno(7) $$\nafter the display.")
F["13-dollars-leqno"] = doc(f"{SHORT}\n$$ a + b = c \\leqno(8) $$\nafter the display.")
F["14-equation-star"] = doc(f"{SHORT}\n{eq('a + b = c', 'equation*')}\n{LONG}\n{eq('d = e', 'equation*')}\nafter.")
F["15-align-tag-notag-eqref"] = doc(
    "Before the alignment.\n\\begin{align}\n a &= b \\label{e:a}\\\\\n c &= d \\notag\\\\\n"
    " e &= f \\tag{$*$}\\label{e:star}\\\\\n g &= h \\tag*{B}\n\\end{align}\n"
    "See \\eqref{e:a} and \\eqref{e:star} and \\ref{e:a}.")
F["16-gather-numbers"] = doc(
    "Before.\n\\begin{gather}\n a = b \\\\\n c + d = e \\notag \\\\\n f = g\n\\end{gather}\n"
    f"After {eq('h = i')} end.")
F["17-numberwithin-section"] = doc(
    f"\\section{{First}}\nText.\n{eq('a = b')}\n\\section{{Second}}\nText.\n"
    + eq("c = d \\label{e:c}") + "\nRef \\eqref{e:c}.",
    pre="\\numberwithin{equation}{section}")
F["18-subequations"] = doc(
    f"Before.\n{eq('z = 0')}\n\\begin{{subequations}}\\label{{e:sub}}\n"
    "\\begin{align} a &= b \\label{e:s1}\\\\ c &= d \\end{align}\n"
    f"{eq('e = f')}\n\\end{{subequations}}\nRefs \\eqref{{e:sub}}, \\eqref{{e:s1}}.\n{eq('g = h')}")
F["19-itemize-display"] = doc(
    "\\begin{itemize}\n\\item First item with a display:\n\\[ a + b = c \\]\nand more text.\n"
    f"\\item Second item with a numbered one\n{eq('x = y')}\nend.\n\\end{{itemize}}")
F["20-enumerate-nested-display"] = doc(
    "\\begin{enumerate}\n\\item Outer item.\n\\begin{enumerate}\n"
    "\\item Inner item text that is long enough to reach toward the right margin of the list\n"
    "\\[ a + b = c \\]\nafter.\n\\end{enumerate}\n"
    f"\\item Outer again {eq('p = q')} tail.\n\\end{{enumerate}}")
F["21-after-heading"] = doc(
    f"\\section{{Heading}}\n\\[ a + b = c \\]\nText after.\n\\subsection{{Sub}}\n{eq('d = e')}\nText.")
F["22-page-bottom"] = doc(FILL * 24 + "\n\\[ a + b = c \\]\nafter the display.\n\n" + FILL * 4)
F["23-page-top"] = doc(FILL * 25 + f"\n{eq('a + b = c')}\nafter the display.\n\n" + FILL * 4)
F["24-11pt-short-long"] = doc(f"{SHORT}\n\\[ a + b = c \\]\n{LONG}\n{eq('d = e')}\nafter.", size="11pt")
F["25-12pt-short-long"] = doc(f"{SHORT}\n\\[ a + b = c \\]\n{LONG}\n{eq('d = e')}\nafter.", size="12pt")
F["26-multline-number"] = doc("Before.\n\\begin{multline} a + b + c + d + e \\\\ = f + g + h \\end{multline}\nAfter.")
F["27-display-then-blank-line"] = doc(f"{SHORT}\n\\[ a = b \\]\n\nNew paragraph after a blank line.")
F["28-line-ends-inline-math"] = doc(f"Text ending with math $x+y$\n\\[ a = b \\]\nText $u$ {eq('c = d')}\nend.")
F["29-quote-display"] = doc(
    f"\\begin{{quote}}\nQuoted text.\n\\[ a + b = c \\]\nand a numbered one\n{eq('d = e')}\nend.\n\\end{{quote}}")
F["30-widow-display"] = doc(FILL * 23 + f"\n{eq('a + b = c')}\nlast line of the paragraph.\n\n" + FILL * 4)
F["31-tag-equation"] = doc("Before.\n" + eq("a = b \\tag{A}") + "\nmiddle\n" + eq("c = d \\tag*{B.1}") + "\nend.")
F["32-consecutive-displays"] = doc(f"Before.\n{eq('a &= b', 'align*')}\n{eq('c = d')}\n\\[ e = f \\]\nafter.")
F["33-wide-tag-shifts-formula"] = doc(
    SHORT + "\n" + eq("a_1 + a_2 + a_3 + a_4 + a_5 + a_6 + a_7 + a_8 + a_9 + a_{10} + a_{11} + a_{12} = b \\tag{long tag text}")
    + "\nafter.")
F["34-parindent-medium-line"] = doc(
    "\\setlength{\\parindent}{15pt}\nA medium length line here\n\\[ a + b + c + d = e \\]\nafter the display.")
F["35-12pt-fleqn-leqno-align"] = doc("Before.\n\\begin{align} a &= b \\\\ c &= d \\end{align}\nAfter.",
                                     size="12pt", opts="fleqn,leqno")
F["36-cm-default-fonts"] = doc(f"{SHORT}\n\\[ a + b = c \\]\n{LONG}\n{eq('d = e')}\nafter.", fontenc=False)
# #441: `\tag`s whose label holds math or text-font switches, set by
# `\tagform@`/`\maketag@@@` flush to the margin in every amsmath display.
F["37-rich-tag-equation"] = doc(
    "Before.\n" + eq("a = b \\tag{hi $x^2$}") + "\nmiddle\n" + eq("c = d \\tag{$\\ast$}")
    + "\nand\n" + eq("e = f \\tag{\\textbf{B} $y_1$}") + "\nend.")
F["38-rich-tag-star"] = doc("Before.\n" + eq("a + b = c \\tag*{$\\pm$ note}") + "\nmiddle\n"
                            + eq("d = e \\tag*{[$n+1$]}") + "\nend.")
F["39-rich-tag-leqno"] = doc("Before.\n" + eq("a + b = c \\tag{hi $x^2$}") + "\nmiddle\n"
                             + eq("d = e \\tag*{$\\ast$}") + "\nend.", opts="leqno")
F["40-align-rich-tags"] = doc(
    "Before.\n\\begin{align}\n a &= b \\tag{$a_1$}\\\\\n c + d &= e \\\\\n f &= g \\tag{step $k^2$}\n\\end{align}\nAfter.")
F["41-gather-rich-tags"] = doc(
    "Before.\n\\begin{gather}\n a = b \\tag{$\\ast$}\\\\\n c + d = e \\\\\n f = g \\tag*{(ii $n$)}\n\\end{gather}\nAfter.")
F["42-multline-tag"] = doc(
    "Before.\n\\begin{multline} a + b + c + d + e \\tag{M $x$} \\\\ = f + g \\\\ = h + i \\end{multline}\nAfter.")
F["43-multline-tag-leqno"] = doc(
    "Before.\n\\begin{multline} a + b + c + d + e \\\\ = f + g \\\\ = h + i \\tag{$\\ast$} \\end{multline}\nAfter.",
    opts="leqno")
F["44-wide-rich-tag-own-line"] = doc(
    SHORT + "\n" + eq(WIDE + " + c_1 + c_2 \\tag{a long tag with $x^2$ in it}") + "\nafter.")
F["45-text-math-display"] = doc(
    "Before.\n\\[ x = 1 \\quad \\text{for all $x$ in $S$} \\]\nand\n"
    + eq("y = 2 \\text{ if $y \\in T$} \\tag{\\emph{i}}") + "\nend.")
F["46-align-wide-tag-own-line"] = doc(
    "Before.\n\\begin{align}\n " + WIDE + " &= c_1 + c_2 + c_3 + c_4 \\tag{a long tag $x$}\\\\\n d &= e\n\\end{align}\nAfter.")
F["47-gather-wide-tag-own-line"] = doc(
    "Before.\n\\begin{gather}\n " + WIDE + " + c_1 + c_2 \\tag{tag $\\ast$ here}\\\\\n d = e\n\\end{gather}\nAfter.")
F["48-gather-wide-tag-leqno"] = doc(
    "Before.\n\\begin{gather}\n d = e\\\\\n " + WIDE + " + c_1 + c_2 \\tag{tag $\\ast$ here}\n\\end{gather}\nAfter.",
    opts="leqno")


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, src in F.items():
        with open(os.path.join(OUT, name + ".tex"), "w", encoding="utf-8") as f:
            f.write(src)
    print(f"{len(F)} fixtures")


if __name__ == "__main__":
    main()
