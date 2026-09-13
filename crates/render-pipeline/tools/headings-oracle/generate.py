#!/usr/bin/env python3
"""Sectioning-heading oracle: \\@startsection headings (section ..
subparagraph), \\chapter, \\part and \\appendix in article/report/book at
10/11/12pt.

pdflatex is the ORACLE ONLY (never in the product path, never in cargo
tests). This script writes fixtures/headings/<name>.tex, runs pdflatex on
each (uncompressed PDF, see tools/page-frame-oracle/generate.py, whose PDF
reader it reuses) and writes fixtures/headings/expected/<name>.txt:

  page <n> <width> <height>
  word <page> <x> <baseline> <font> <size> <text>
  rule <page> <x> <top> <width> <height>

in PDF big points with a top-left origin.

Usage: generate.py [--keep DIR] [NAME ...]
"""
import argparse
import importlib.util
import os
import subprocess
import tempfile
import shutil

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(os.path.dirname(HERE))
FIXTURES = os.path.join(CRATE, "fixtures", "headings")
EXPECTED = os.path.join(FIXTURES, "expected")

_spec = importlib.util.spec_from_file_location("pageframe", os.path.join(CRATE, "tools", "page-frame-oracle", "generate.py"))
pf = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pf)

sentences, paras, doc = pf.sentences, pf.paras, pf.doc


def all_levels(seed, chapters=False, per=1, words=70):
    """Every level once, with text after each."""
    out = []
    if chapters:
        out.append("\\chapter{Opening Chapter}")
        out.append(paras(seed, per, words))
    out += [
        "\\section{First Section}",
        paras(seed + 1, per, words),
        "\\subsection{A Subsection}",
        paras(seed + 2, per, words),
        "\\subsubsection{A Subsubsection}",
        paras(seed + 3, per, words),
        "\\paragraph{Run-in paragraph.}",
        sentences(seed * 10 + 4, words),
        "\\subparagraph{Run-in subparagraph.}",
        sentences(seed * 10 + 5, words),
        "\\section{Second Section}",
        paras(seed + 6, per, words),
        "\\subsection{Another Subsection}",
        paras(seed + 7, per, words),
    ]
    return "\n\n".join(out)


def many(seed, n, words=60, levels=("section", "subsection", "subsubsection")):
    """`n` headings cycling through `levels`, each followed by a paragraph of
    varying length: page breaks land at many different heading positions."""
    out = []
    for i in range(n):
        lv = levels[i % len(levels)]
        out.append(f"\\{lv}{{Heading number {i + 1}}}")
        out.append(sentences(seed * 1000 + i, words + (i * 37) % 80))
    return "\n\n".join(out)


def fill_to(seed, vspace, heading="\\section{Near the Bottom}", after=None):
    """A page of text pushed down by `\\vspace`, then a heading."""
    return "\n\n".join(
        [
            paras(seed, 2, 90),
            f"\\vspace*{{{vspace}}}",
            sentences(seed * 7, 40),
            heading,
            after or paras(seed + 1, 2, 90),
        ]
    )


def fixtures():
    f = {}
    for size in ("10pt", "11pt", "12pt"):
        opt = "" if size == "10pt" else size
        f[f"01-article-{size}-levels"] = doc("article", opt, all_levels(1 + int(size[:2])))
        f[f"02-report-{size}-levels"] = doc("report", opt, all_levels(3 + int(size[:2]), chapters=True))
        f[f"03-book-{size}-levels"] = doc("book", opt, all_levels(5 + int(size[:2]), chapters=True))
    f["04-article-runin"] = doc(
        "article",
        "",
        "\n\n".join(
            [
                paras(40, 1),
                "\\paragraph{Short.} " + sentences(401, 60),
                "\\paragraph{A somewhat longer run-in title} " + sentences(402, 60),
                sentences(403, 50),
                "\\subparagraph{Indented run-in.} " + sentences(404, 60),
                "\\paragraph{Two in a row.}\n\\paragraph{Second one.} " + sentences(405, 50),
                "\\subsubsection{Before a paragraph}\n\\paragraph{Run-in after display heading.} " + sentences(406, 60),
            ]
        ),
    )
    f["05-article-consecutive"] = doc(
        "article",
        "",
        "\n\n".join(
            [
                paras(50, 1),
                "\\section{Section}\n\\subsection{Subsection}\n\\subsubsection{Subsubsection}",
                paras(51, 1),
                "\\section{Another}\n\\subsubsection{Deep directly}",
                paras(52, 2),
            ]
        ),
    )
    f["06-article-heading-list"] = doc(
        "article",
        "",
        "\n\n".join(
            [
                paras(60, 1),
                "\\section{A List Follows}\n\\begin{itemize}\n\\item " + sentences(601, 20) + "\n\\item " + sentences(602, 25) + "\n\\end{itemize}",
                sentences(603, 40),
                "\\begin{enumerate}\n\\item " + sentences(604, 20) + "\n\\item " + sentences(605, 20) + "\n\\end{enumerate}",
                "\\subsection{After a List}",
                paras(61, 1),
                "\\paragraph{Run-in before list.}\n\\begin{itemize}\n\\item " + sentences(606, 20) + "\n\\end{itemize}",
                paras(62, 1),
            ]
        ),
    )
    f["07-article-heading-display"] = doc(
        "article",
        "",
        "\n\n".join(
            [
                paras(70, 1),
                "\\section{Display Follows}\n\\[ a + b = c \\]\n" + sentences(701, 40),
                sentences(702, 40) + "\n\\[ x^2 + y^2 = z^2 \\]",
                "\\subsection{After a Display}",
                paras(71, 1),
            ]
        ),
    )
    f["08-article-long-headings"] = doc(
        "article",
        "",
        "\n\n".join(
            [
                paras(80, 1),
                "\\section{A very long section title that certainly needs more than one line to be set in the large bold font of the class}",
                paras(81, 1),
                "\\subsection{Another quite long subsection title that also wraps onto a second line at this size}",
                paras(82, 1),
                "\\subsubsection{And a subsubsection heading with a title long enough to wrap in the normal size bold font used here}",
                paras(83, 1),
                "\\section*{An unnumbered long section title which wraps onto the following line as well}",
                paras(84, 1),
            ]
        ),
    )
    f["09-article-secnumdepth-1"] = doc("article", "", all_levels(90), "\\setcounter{secnumdepth}{1}\n")
    f["10-article-secnumdepth-5"] = doc("article", "", all_levels(100), "\\setcounter{secnumdepth}{5}\n")
    f["11-article-starred"] = doc(
        "article",
        "",
        "\n\n".join(
            [
                "\\section*{Starred Section}",
                paras(110, 1),
                "\\subsection*{Starred Subsection}",
                paras(111, 1),
                "\\section{Numbered After}",
                paras(112, 1),
                "\\subsubsection*{Starred Subsubsection}",
                paras(113, 1),
                "\\paragraph*{Starred run-in.} " + sentences(114, 50),
            ]
        ),
    )
    f["12-article-appendix"] = doc(
        "article",
        "",
        "\n\n".join(
            [
                "\\section{Body}",
                paras(120, 1),
                "\\appendix",
                "\\section{First Appendix}",
                paras(121, 1),
                "\\subsection{Appendix Detail}",
                paras(122, 1),
                "\\section{Second Appendix}",
                paras(123, 1),
            ]
        ),
    )
    f["13-report-appendix"] = doc(
        "report",
        "",
        "\n\n".join(
            [
                "\\chapter{Main}",
                paras(130, 1),
                "\\section{Main Section}",
                paras(131, 1),
                "\\appendix",
                "\\chapter{Extra Material}",
                paras(132, 1),
                "\\section{Appendix Section}",
                paras(133, 1),
            ]
        ),
    )
    f["14-article-twocolumn"] = doc("article", "twocolumn", many(140, 14, levels=("section", "subsection", "paragraph")))
    f["15-article-page-bottom-a"] = doc("article", "", fill_to(150, "330pt"))
    f["16-article-page-bottom-b"] = doc("article", "", fill_to(160, "345pt", "\\subsection{Low Subsection}"))
    f["17-article-page-bottom-c"] = doc("article", "11pt", fill_to(170, "320pt"))
    f["18-article-page-top"] = doc("article", "", "\\section{At the Top}\n" + paras(180, 1) + "\n\n\\newpage\n\\subsection{New Page Top}\n" + paras(181, 2))
    f["19-article-many-10pt"] = doc("article", "", many(190, 30))
    f["20-article-many-11pt"] = doc("article", "11pt", many(200, 30))
    f["21-article-many-12pt"] = doc("article", "12pt", many(210, 30, levels=("section", "subsection", "subsubsection", "paragraph")))
    f["22-report-chapter-star"] = doc(
        "report", "", "\\chapter*{Preface}\n" + paras(220, 2) + "\n\n\\chapter{Real Chapter}\n" + many(221, 8)
    )
    f["23-book-sections-twoside"] = doc("book", "", "\\chapter{One}\n" + many(230, 10) + "\n\n\\chapter{Two}\n" + many(231, 8))
    f["24-article-part"] = doc(
        "article", "", "\\part{First Part}\n" + paras(240, 1) + "\n\n\\section{In Part}\n" + paras(241, 1) + "\n\n\\part*{Starred Part}\n" + paras(242, 1)
    )
    f["25-article-secnumdepth-body"] = doc(
        "article",
        "",
        "\\section{Numbered}\n" + paras(250, 1) + "\n\n\\setcounter{secnumdepth}{0}\n\\section{Unnumbered Now}\n" + paras(251, 1) + "\n\n\\subsection{Also Unnumbered}\n" + paras(252, 1),
    )
    f["26-article-12pt-runin-subpar"] = doc(
        "article",
        "12pt",
        "\n\n".join(["\\subparagraph{Sub.} " + sentences(260 + i, 50 + 10 * i) for i in range(6)] + ["\\paragraph{Par.} " + sentences(270, 80)]),
    )
    f["27-article-11pt-title-ligatures"] = doc(
        "article",
        "11pt",
        "\n\n".join(
            [
                "\\section{Efficient Office Workflow}",
                paras(270, 1),
                "\\subsection{Affine Fields and Flows}",
                paras(271, 1),
                "\\paragraph{Offline effects.} " + sentences(272, 60),
            ]
        ),
    )
    f["28-report-11pt-many"] = doc("report", "11pt", "\\chapter{Long Chapter}\n" + many(280, 24))
    f["29-article-noindent-after"] = doc(
        "article",
        "",
        "\n\n".join(
            [
                "\\section{Section}",
                sentences(290, 50),
                sentences(291, 50),
                "\\subsection{Sub}\n\n" + sentences(292, 50),
                "\\paragraph{Run.}\n\n" + sentences(293, 50),
            ]
        ),
    )
    f["30-article-twocolumn-12pt"] = doc("article", "12pt,twocolumn", many(300, 16, levels=("section", "subsection", "subsubsection", "paragraph")))
    f["31-article-after-list-end"] = doc(
        "article",
        "",
        "\n\n".join(
            [
                paras(310, 1),
                "\\begin{itemize}\n\\item " + sentences(311, 20) + "\n\\item " + sentences(312, 20) + "\n\\end{itemize}",
                "\\section{After Itemize}",
                paras(313, 1),
                "\\[ a = b \\]",
                "\\subsection{After Display}",
                paras(314, 1),
                "\\begin{center}\nCentered text\n\\end{center}",
                "\\paragraph{After center.} " + sentences(315, 40),
            ]
        ),
    )
    f["32-book-11pt-appendix"] = doc(
        "book", "11pt", "\\chapter{Main}\n" + many(320, 4) + "\n\n\\appendix\n\\chapter{Tables}\n" + many(321, 4)
    )
    f["33-article-math-headings"] = doc(
        "article",
        "",
        "\n\n".join(
            [
                "\\section{Squares $x^2$ and more}",
                paras(330, 1),
                "\\subsection{Greek $\\alpha$ and $\\beta_i$}",
                paras(331, 1),
                "\\subsubsection{Sums $\\sum_{i=1}^n a_i$ inline}",
                paras(332, 1),
                "\\paragraph{Run-in $y=f(x)$.} " + sentences(333, 50),
            ]
        ),
    )
    f["34-report-twocolumn-chapter"] = doc("report", "twocolumn", "\\chapter{Two Column Chapter}\n" + many(340, 10) + "\n\n\\chapter*{Starred Two Column}\n" + many(341, 4))
    f["35-book-twocolumn-chapter"] = doc("book", "twocolumn", "\\chapter{Book Chapter}\n" + many(350, 10))
    return f


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--keep", help="copy the pdflatex PDFs here")
    ap.add_argument("names", nargs="*")
    args = ap.parse_args()
    os.makedirs(EXPECTED, exist_ok=True)
    version = subprocess.run(["pdflatex", "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    for name, src in fixtures().items():
        if args.names and name not in args.names:
            continue
        tex = os.path.join(FIXTURES, name + ".tex")
        with open(tex, "w") as fh:
            fh.write(src)
        with tempfile.TemporaryDirectory() as tmp:
            pdf = pf.run_pdflatex(tex, tmp)
            if args.keep:
                os.makedirs(args.keep, exist_ok=True)
                shutil.copy(pdf, args.keep)
            pages = pf.extract(pdf)
        lines = [f"# {version}", f"# source fixtures/headings/{name}.tex (tools/headings-oracle/generate.py)"]
        for p in pages:
            lines.append(f"page {p['number']} {p['width']:.4f} {p['height']:.4f}")
            for w in p["words"]:
                lines.append(f"word {p['number']} {w['x']:.4f} {w['baseline']:.4f} {w['font']} {w['size']:.4f} {w['text']}")
            for r in p["rules"]:
                lines.append(f"rule {p['number']} {r['x']:.4f} {r['top']:.4f} {r['width']:.4f} {r['height']:.4f}")
        with open(os.path.join(EXPECTED, name + ".txt"), "w") as fh:
            fh.write("\n".join(lines) + "\n")
        print(name, len(pages), "pages", sum(len(p["words"]) for p in pages), "words")


if __name__ == "__main__":
    main()
