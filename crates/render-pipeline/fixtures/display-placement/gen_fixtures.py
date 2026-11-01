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


def doc(body, size="10pt", opts="", pre="", fontenc=True, lmodern=True):
    classopts = ",".join(o for o in (size, opts) if o)
    lines = [f"\\documentclass[{classopts}]{{article}}"]
    if fontenc:
        lines.append("\\usepackage[T1]{fontenc}")
    if lmodern:
        lines.append("\\usepackage{lmodern}")
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
F["36-cm-default-fonts"] = doc(f"{SHORT}\n\\[ a + b = c \\]\n{LONG}\n{eq('d = e')}\nafter.", fontenc=False, lmodern=False)

# Math family 0 (`operators`) is `cmr`, not `rm-lmr`, unless `lmodern` is
# loaded -- and `[T1]{fontenc}` alone does not load it, which is what most of
# the real-world corpus does. Every display below has a *family-0* character
# (a digit) as the tallest thing in its numerator, so the wrong roman design
# makes the display's box 0.0147 em short and every baseline under it rises.
# Eight displays down one page, because that is the shape of the defect: one
# display is 0.11-0.18 bp, under the 0.5 bp gate on its own, and the page is
# over it. No `\\sqrt`, `\\sum` or `\\left(` here -- a cmex glyph at a size
# other than 10 bp is not classified as an extension glyph on the candidate
# side (see `oracle.py`), so it would fail the 11 and 12 pt fixtures for a
# reason that has nothing to do with the box height.
_FD = "\\[ x = \\frac{%s}{1 + y} \\]"
DIGIT_BOXES = "\n".join([SHORT]
    + [line for i, n in enumerate(("1", "2", "3", "4", "5", "6", "7", "8"))
       for line in (_FD % n, LONG if i % 2 == 0 else SHORT)]
    + ["after."])
F["47-cm-math-roman-boxes"] = doc(DIGIT_BOXES, lmodern=False)
F["48-cm-math-roman-boxes-11pt"] = doc(DIGIT_BOXES, size="11pt", lmodern=False)
F["49-cm-math-roman-boxes-12pt"] = doc(DIGIT_BOXES, size="12pt", lmodern=False)
# The same page with `lmodern`, which really does rebind `operators` to `lmr`:
# the other branch of the same choice, which must not move.
F["50-lm-math-roman-boxes"] = doc(DIGIT_BOXES)
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
F["51-gather-wide-tag-own-line"] = doc(
    "Before.\n\\begin{gather}\n " + WIDE + " + c_1 + c_2 \\tag{tag $\\ast$ here}\\\\\n d = e\n\\end{gather}\nAfter.")
F["52-gather-wide-tag-leqno"] = doc(
    "Before.\n\\begin{gather}\n d = e\\\\\n " + WIDE + " + c_1 + c_2 \\tag{tag $\\ast$ here}\n\\end{gather}\nAfter.",
    opts="leqno")
# `\eqref` to a rich tag is `\textup{\tagform@{\ref{..}}}`: the tag's content
# set again in running text -- its math as math, `\textbf` kept, a `\tag*`
# label between `\tagform@`'s parentheses all the same -- with `\textup`'s
# `\check@icl` italic correction before it.
F["53-eqref-rich-tags"] = doc(
    "Before.\n" + eq("a = b \\tag{hi $x^2$}\\label{e:a}") + "\nmiddle\n"
    + eq("c = d \\tag*{$\\pm$}\\label{e:b}") + "\nand\n"
    + eq("e = f \\tag{\\textbf{B} $y_1$}\\label{e:c}") + "\nand\n"
    + "\\begin{align}\n g &= h \\tag{$\\ast$}\\label{e:d}\\\\\n i &= j \\label{e:e}\n\\end{align}\n"
    + "See \\eqref{e:a}, \\eqref{e:b} and \\eqref{e:c}; \\emph{then} \\eqref{e:d} and \\eqref{e:e}.")
# A display's penalties at a `\\flushbottom` page break (TeX §890, §1145,
# §1200). `$$` breaks the lines before it with
# `line_break(display_widow_penalty)`, so the penultimate line of the part
# before a display costs `\\displaywidowpenalty` (50), not `\\widowpenalty`
# (150): 54 puts that line exactly at the foot of page 1 (b=0, c=50) with the
# break one line up at b=78 (13 pt of `\\parskip` stretch, 12 pt short), so
# pdflatex ends the page with the penultimate line; with 150 it would end one
# line earlier. And `\\everypar` runs only from `new_graf`, never from
# `resume_after_display`, so `\\@afterheading`'s `\\clubpenalty\\@M` holds
# for every part of the paragraph after a heading: in 55 the first line after
# the display fits the page (b=0) but its break is illegal (10000), so
# pdflatex ends the page with the display (b=892) instead; with the class's
# 150 it would keep that line. Both are the article-twocolumn column-1 case
# (`fixtures/real-world/article-twocolumn`, `\\tracingpages`: `p=50 c=50` at
# "ural height h, depth d ..." and `\\penalty 10100` after "where q is the
# insertion penalty"). 54's page 1 is exactly full, so `\\flushbottom` as a
# command is enough; 55's page 1 is 19 pt short and its glue is set at 2.07,
# and the pipeline reads `\\flushbottom` from the class options only
# (`style.rs`: `\\if@twoside\\else\\raggedbottom\\fi`), so it takes the
# `twoside` option instead.
_FILL3 = FILL + "Filler text that runs across several lines so that the page is mostly full before the display we"
_FILL4 = _FILL3 + " care about arrives at the page break, and one more clause to reach a fourth line here."
_PRE = ("The part of the paragraph before the display has four lines, so its third line is the "
        "penultimate one and carries the widow penalty that the display hands to the line breaker; "
        "this sentence keeps going until the fourth line has begun and the equation can follow it "
        "on the very next line of the page.")
F["54-display-widow-page-break"] = doc(
    "\n\n".join([_FILL4] * 4 + [_FILL3] * 9
                 + [_PRE + "\n" + eq("a + b = c") + "\nlast line of the paragraph after the display.\n\nA closing paragraph."]),
    pre="\\flushbottom")
F["55-heading-club-through-display"] = doc(
    " ".join([_FILL3] * 13) + "\n\n\\section*{A heading}\n"
    + "The paragraph after the heading keeps the club penalty the heading set, "
    + "through the display and into the lines after it; this pre-display part runs to a second line.\n"
    + eq("a + b = c") + "\n"
    + "The first line after the display cannot end the page: the club penalty is still ten thousand. "
    + "So the page ends after the display instead, and this line and the next two go to page two, "
    + "as the reference shows, with one more line to make four.\n\nA closing paragraph.",
    opts="twoside")


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, src in F.items():
        with open(os.path.join(OUT, name + ".tex"), "w", encoding="utf-8") as f:
            f.write(src)
    print(f"{len(F)} fixtures")


if __name__ == "__main__":
    main()
