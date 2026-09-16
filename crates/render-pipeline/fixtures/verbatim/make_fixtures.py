#!/usr/bin/env python3
"""Writes the verbatim/listings oracle fixtures (NN-name.tex) next to this
script. Deterministic; tabs are written as real U+0009 characters.
Run `oracle.py refs` afterwards to regenerate the pdfLaTeX references."""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
T = "\t"

PARA = "Some ordinary body text that runs for a little while so that the paragraph fills a line of the page."
AFTER = "Text right after the environment continues the paragraph without an indent."

FIXTURES = {}


def doc(body, preamble="", options=""):
    cls = f"\\documentclass[{options}]{{article}}" if options else "\\documentclass{article}"
    return f"{cls}\n{preamble}\\begin{{document}}\n{body}\n\\end{{document}}\n"


def fixture(name, body, preamble="", options=""):
    FIXTURES[name] = doc(body, preamble, options)


# ---------------------------------------------------------------- verbatim
fixture("01-verbatim-basic", f"""{PARA}
\\begin{{verbatim}}
100% \\foo ${{x}}^2 & #1 _a ~b
  indented line with   three spaces
\\end{{verbatim}}
{AFTER}""")

fixture("02-verbatim-ligatures", f"""None of these form ligatures inside verbatim text.
\\begin{{verbatim}}
``quoted'' -- --- << >> ,, !` ?` fi ff
'single' `back` a-b a--b a---b
\\end{{verbatim}}
{AFTER}""")

fixture("03-verbatim-tabs", f"""{PARA}
\\begin{{verbatim}}
{T}one tab
a{T}b{T}{T}c
    four spaces{T}then tab
\\end{{verbatim}}
{AFTER}""")

fixture("04-verbatim-blank-lines", f"""{PARA}
\\begin{{verbatim}}

first after blank

    indented after blank


two blanks above
\\end{{verbatim}}

{PARA}""")

fixture("05-verbatim-star", f"""{PARA}
\\begin{{verbatim*}}
a b  c   d
{T}tab and  spaces
\\end{{verbatim*}}
{AFTER}""")

fixture("06-verb-delimiters", """Inline \\verb|a b| then \\verb+x_1 = {y}+ and \\verb!%$#&! with
\\verb*|two  spaces| and \\verb=\\begin{x}= plus \\verb"--" and (\\verb|`|)
all in one paragraph that is long enough to wrap onto a second line of text.
""")

fixture("07-verbatim-t1", f"""{PARA} \\verb|<a> --b-- "c"|.
\\begin{{verbatim}}
<html> --x-- "quoted" 'single' `grave`
\\{{}}|_^~ @#$%&*
\\end{{verbatim}}
{AFTER}""", preamble="\\usepackage[T1]{fontenc}\n")

fixture("08-verbatim-lmodern", f"""{PARA} \\verb|<a> --b-- "c"|.
\\begin{{verbatim}}
<html> --x-- "quoted" 'single' `grave`
\\{{}}|_^~ @#$%&*
\\end{{verbatim}}
{AFTER}""", preamble="\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n")

fixture("09-verbatim-vmode", f"""{PARA}

\\begin{{verbatim}}
after a blank line
\\end{{verbatim}}

{PARA}
\\begin{{verbatim}}
directly after text
\\end{{verbatim}}
{AFTER}""")

# `enumerate`, and a blank line after the list: itemize's TS1 bullet and
# `\\@endpe` after `\\end{itemize}` belong to the list layout, not to verbatim.
fixture("10-verbatim-itemize", """\\begin{enumerate}
\\item First item text.
\\begin{verbatim}
inside item
  more
\\end{verbatim}
\\item Second item.
\\end{enumerate}

""" + PARA)

fixture("11-verbatim-11pt", f"""{PARA} \\verb|inline 11pt|.
\\begin{{verbatim}}
eleven point class
{T}x
\\end{{verbatim}}
{AFTER}""", options="11pt")

fixture("12-verbatim-12pt", f"""{PARA} \\verb|inline 12pt|.
\\begin{{verbatim}}
twelve point class
{T}x
\\end{{verbatim}}
{AFTER}""", options="12pt")

fixture("13-verbatim-long-line", f"""{PARA}
\\begin{{verbatim}}
this is a very long verbatim line that certainly runs past the right margin of the text block
short
\\end{{verbatim}}
{AFTER}""")

fixture("14-verbatim-pagebreak", "\n\n".join([PARA] * 6) + "\n\\begin{verbatim}\n"
        + "\n".join(f"line {i:02d} of a long verbatim block" for i in range(1, 41))
        + "\n\\end{verbatim}\n" + AFTER)

fixture("15-verbatim-small", f"""{PARA} {{\\small \\verb|small inline|}} and {{\\footnotesize\\verb|fn|}}.
\\par
\\begin{{small}}
\\begin{{verbatim}}
small verbatim
\\end{{verbatim}}
\\end{{small}}
{AFTER}""")

# ---------------------------------------------------------------- listings
LST = "\\usepackage{listings}\n"
CODE = """int main(void) {
    int x = 42; /* comment */
    return x;
}"""
C_CODE = """#include <stdio.h>
/* block comment with if and int */
int main(int argc, char **argv) {
    // line comment: return
    char *s = "a string with spaces";
    for (int i = 0; i < argc; i++) {
        printf("%d\\n", i);
    }
    return 0;
}"""
PY_CODE = """def fib(n):
    # comment with def and return
    if n < 2:
        return n
    s = 'string with spaces'
    return fib(n - 1) + fib(n - 2)"""
JAVA_CODE = """public class Hello {
    public static void main(String[] args) {
        System.out.println("hi there");
    }
}"""


def listing(options, code):
    opt = f"[{options}]" if options else ""
    return f"\\begin{{lstlisting}}{opt}\n{code}\n\\end{{lstlisting}}"


def lst_fixture(name, options, code, before=PARA, after=AFTER, preamble=LST):
    fixture(name, before + "\n" + listing(options, code) + "\n" + after, preamble=preamble)


TT = "basicstyle=\\ttfamily"
lst_fixture("16-lst-default", "", CODE)
lst_fixture("17-lst-tt-fixed", TT, CODE)
lst_fixture("18-lst-tt-flexible", TT + ",columns=flexible", CODE)
lst_fixture("19-lst-fullflexible", TT + ",columns=fullflexible", CODE)
lst_fixture("20-lst-numbers", TT + ",numbers=left,numberstyle=\\tiny,numbersep=5pt", CODE)
lst_fixture("21-lst-frame-single", TT + ",frame=single", CODE)
lst_fixture("22-lst-frame-lines-numbers", TT + "\\small,frame=lines,numbers=left", CODE)
lst_fixture("23-lst-c-keywords", "language=C," + TT, C_CODE)
lst_fixture("24-lst-python-keywords", "language=Python," + TT + "\\small", PY_CODE)
lst_fixture("25-lst-java-roman", "language=Java", JAVA_CODE)
lst_fixture("26-lst-breaklines", TT + ",breaklines=true",
            "x = some_function(argument_one, argument_two, argument_three, argument_four, argument_five);\ny = 1;")
fixture("27-lst-showstringspaces", PARA + "\n"
        + listing("language=C," + TT, 'puts("two  spaces");\nputs("x y");') + "\n"
        + listing("language=C," + TT + ",showstringspaces=false", 'puts("x y");') + "\n" + AFTER, preamble=LST)
fixture("28-lstinline", "\\lstset{basicstyle=\\ttfamily}\n"
        "Inline code \\lstinline|x = a + b;| and \\lstinline{int y;} and \\lstinline!s -- t! in a paragraph\n"
        "of ordinary text that wraps onto a second line so that line breaking is exercised too.\n", preamble=LST)
lst_fixture("29-lst-tabs-gobble", TT + ",tabsize=4,gobble=2", "  a" + T + "b\n  " + T + "c\n    d")
fixture("30-lst-lstset-margin", "\\lstset{basicstyle=\\ttfamily\\footnotesize,xleftmargin=2em,language=C}\n"
        + PARA + "\n" + listing("", CODE) + "\n" + AFTER, preamble=LST)
lst_fixture("31-lst-t1-lmodern-bold", "language=C," + TT, CODE,
            preamble="\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n" + LST)

# microtype protrudes and expands the roman text around the verbatim
# material but not the typewriter text (its default sets are rm*/sf*).
fixture("32-verbatim-microtype", f"""{PARA} Inline \\verb|code, here.| ends a line of text that is justified and protruded.
\\begin{{verbatim}}
-- line with "quotes", commas, and a hyphen-
\\end{{verbatim}}
{AFTER}""", preamble="\\usepackage{microtype}\n")


def main():
    for name in list(os.listdir(HERE)):
        if name.endswith(".tex") and name[:2].isdigit() and name[:-4] not in FIXTURES:
            os.remove(os.path.join(HERE, name))
    for name, text in FIXTURES.items():
        with open(os.path.join(HERE, name + ".tex"), "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
    print(f"{len(FIXTURES)} fixtures")


if __name__ == "__main__":
    main()
