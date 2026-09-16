#!/usr/bin/env python3
"""Contents-list oracle: \\tableofcontents, \\listoffigures, \\listoftables, \\lstlistoflistings.

pdflatex is the ORACLE ONLY (never in the product path, never in cargo
tests). This script

  1. writes fixtures/toc/<name>.tex (deterministic text),
  2. runs pdflatex three times on a copy (the .toc/.lof/.lot files are read
     on the second run; the third confirms the page numbers are stable)
     with \\pdfcompresslevel=0 \\pdfobjcompresslevel=0 on the command line,
  3. reads every word's origin and baseline straight from the content
     streams with the page-frame oracle's PDF reader
     (tools/page-frame-oracle/generate.py), writing
     fixtures/toc/expected/<name>.txt in PDF big points, top-left origin.

Leader dots are separate words: pdfTeX separates the `\\leaders` boxes by
their 9mu of kerning, far beyond the reader's word split.

The fixture line `%% toc-oracle: lists end`, right after the list commands,
becomes `\\pdfsavepos\\write-1{...}` in the compiled copy: at shipout it logs
the position below the last list line, recorded as `# lists <page> <x> <y>`
(bp, top-left origin). tests/toc_oracle.rs gates every word before that
point: all earlier pages, and on that page the words above it (in its
column for two-column documents).

Usage: generate.py [--keep DIR] [NAME ...]
"""
import argparse
import importlib.util
import os
import re
import shutil
import subprocess
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(os.path.dirname(HERE))
FIXTURES = os.path.join(CRATE, "fixtures", "toc")
EXPECTED = os.path.join(FIXTURES, "expected")

_spec = importlib.util.spec_from_file_location("page_frame_oracle", os.path.join(CRATE, "tools", "page-frame-oracle", "generate.py"))
pf = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pf)
pf.BP = 72.0 / 72.27
pf.PAGE_H = 792.0

_fonts_of = pf.fonts_of


def _fonts_or_none(objs, page):
    # A blank page (`\@endpart`'s `\null\newpage`, `\cleardoublepage`) has no
    # /Font resource.
    try:
        return _fonts_of(objs, page)
    except KeyError:
        return {}


pf.fonts_of = _fonts_or_none

MARK = "%% toc-oracle: lists end"
PROBE = "\\pdfsavepos\\write-1{TOC-ORACLE-LISTS-END \\the\\pdflastxpos\\space\\the\\pdflastypos}"


def doc(cls, opts, lists, body, preamble="", clear=True):
    between = "\n\\clearpage\n" if clear else "\n"
    return pf.doc(cls, opts, lists + "\n" + MARK + between + body, preamble)


def secs(seed, titles, words=60, sub=None, subsub=None, cmd="section"):
    """`\\<cmd>{title}` + a short paragraph each; `sub`/`subsub`: titles of
    subsections/subsubsections repeated under every heading."""
    parts = []
    for i, t in enumerate(titles):
        parts.append(f"\\{cmd}{{{t}}}")
        parts.append(pf.paras(seed + i, 1, words))
        for j, s in enumerate(sub or []):
            parts.append(f"\\subsection{{{s}}}" if cmd != "chapter" else f"\\section{{{s}}}")
            parts.append(pf.paras(seed + 40 + i * 7 + j, 1, words))
            for k, ss in enumerate(subsub or []):
                parts.append(f"\\subsubsection{{{ss}}}")
                parts.append(pf.paras(seed + 80 + i * 11 + j * 3 + k, 1, words))
    return "\n\n".join(parts)


def figs(kind, captions, seed):
    parts = []
    for i, c in enumerate(captions):
        parts.append(pf.paras(seed + i, 1, 70))
        parts.append(f"\\begin{{{kind}}}[h]\n\\centering\n\\caption{{{c}}}\n\\end{{{kind}}}")
    return "\n\n".join(parts)


LONG_SEC = "A considerably longer section title that certainly needs more than one line in the contents"
LONG_SUB = "An even longer subsection title whose words have to wrap underneath the hanging number box of the entry"
LONG_CHAP = "A chapter with a title long enough to wrap onto a second line in the table of contents"
NAMES = ["Introduction", "Background", "Method", "Results", "Discussion", "Conclusion"]


def fixtures():
    f = {}
    toc = "\\tableofcontents"
    f["01-article-basic"] = doc("article", "", toc, secs(1, NAMES[:4], sub=["Overview", "Details"]))
    f["02-article-tocdepth1"] = doc("article", "", toc, secs(2, NAMES[:4], sub=["Overview", "Details"]), "\\setcounter{tocdepth}{1}\n")
    f["03-article-tocdepth2"] = doc(
        "article", "", toc, secs(3, NAMES[:3], sub=["Overview"], subsub=["Fine print"]), "\\setcounter{tocdepth}{2}\n"
    )
    f["04-article-tocdepth3"] = doc("article", "", toc, secs(4, NAMES[:3], sub=["Overview", "Details"], subsub=["Fine print"]))
    f["05-article-long-titles"] = doc("article", "", toc, secs(5, ["Short", LONG_SEC, "Tail"], sub=[LONG_SUB]))
    f["06-article-starred-addcontentsline"] = doc(
        "article",
        "",
        toc,
        "\\section*{Preface}\n\\addcontentsline{toc}{section}{Preface}\n"
        + pf.paras(6, 1, 60)
        + "\n\n"
        + secs(7, NAMES[:2], sub=["Overview"])
        + "\n\n\\subsection*{Unnumbered remarks}\n\\addcontentsline{toc}{subsection}{Unnumbered remarks}\n"
        + pf.paras(8, 1, 60),
    )
    f["07-article-lof"] = doc(
        "article", "", "\\listoffigures", secs(9, NAMES[:2]) + "\n\n" + figs("figure", ["A first figure", "The second figure", "Third"], 10)
    )
    f["08-article-lot"] = doc(
        "article", "", "\\listoftables", secs(11, NAMES[:2]) + "\n\n" + figs("table", ["Measurements", "Totals by year"], 12)
    )
    f["09-article-toc-lof-lot"] = doc(
        "article",
        "",
        "\\tableofcontents\n\\listoffigures\n\\listoftables",
        secs(13, NAMES[:3], sub=["Overview"]) + "\n\n" + figs("figure", ["Setup", "Outcome"], 14) + "\n\n" + figs("table", ["Numbers"], 15),
    )
    f["10-article-appendix"] = doc(
        "article", "", toc, secs(16, NAMES[:2], sub=["Overview"]) + "\n\n\\appendix\n\n" + secs(17, ["Proofs", "Tables"], sub=["Lemma"])
    )
    f["11-article-twocolumn"] = doc("article", "twocolumn", toc, secs(18, NAMES[:5], sub=["Overview", "Details"], words=90))
    f["12-article-contentsname"] = doc(
        "article", "", toc, secs(19, NAMES[:3], sub=["Overview"]), "\\renewcommand{\\contentsname}{Table of Contents}\n"
    )
    f["13-article-11pt"] = doc("article", "11pt", toc, secs(20, NAMES[:4], sub=["Overview", "Details"]))
    f["14-article-12pt"] = doc("article", "12pt", toc, secs(21, NAMES[:4], sub=["Overview", "Details"]))
    f["15-article-many-pages"] = doc("article", "", toc, secs(22, NAMES * 2, words=260, sub=["Overview"]))
    f["16-article-wide-numbers"] = doc("article", "", toc, secs(23, NAMES[:1], words=20, sub=[f"Part {i}" for i in range(1, 13)]))
    f["17-report-basic"] = doc(
        "report", "", toc, secs(24, ["Getting Started", "Going Further"], cmd="chapter", sub=["Overview", "Details"]), clear=False
    )
    f["18-report-tocdepth1-lof"] = doc(
        "report",
        "",
        "\\tableofcontents\n\\listoffigures",
        "\\chapter{Pictures}\n" + figs("figure", ["Chapter figure", "Another"], 25) + "\n\n" + secs(26, ["Overview"], cmd="section"),
        "\\setcounter{tocdepth}{1}\n",
        clear=False,
    )
    f["19-report-long-chapter"] = doc("report", "", toc, secs(27, [LONG_CHAP, "Short"], cmd="chapter", sub=[LONG_SUB]), clear=False)
    f["20-report-appendix"] = doc(
        "report",
        "",
        toc,
        secs(28, ["Main"], cmd="chapter", sub=["Part"]) + "\n\n\\appendix\n\n" + secs(29, ["Extra"], cmd="chapter", sub=["More"]),
        clear=False,
    )
    f["21-book-basic"] = doc(
        "book",
        "",
        toc,
        "\\chapter*{Preface}\n\\addcontentsline{toc}{chapter}{Preface}\n"
        + pf.paras(30, 1, 60)
        + "\n\n"
        + secs(31, ["Opening", "Middle"], cmd="chapter", sub=["Overview"]),
        clear=False,
    )
    f["22-article-roman-frontmatter"] = doc(
        "article",
        "",
        "\\pagenumbering{roman}\n\\tableofcontents\n\\listoffigures",
        "\\pagenumbering{arabic}\n" + secs(32, NAMES[:3], sub=["Overview"]) + "\n\n" + figs("figure", ["Only figure"], 33),
    )
    f["23-article-same-page"] = doc("article", "", toc, secs(34, NAMES[:3], sub=["Overview"]), clear=False)
    # `\part` entries (`\l@part`) and pages.
    f["24-article-part"] = doc(
        "article",
        "",
        toc,
        "\\part{Foundations}\n\n" + secs(35, NAMES[:2], sub=["Overview"]) + "\n\n\\part{Applications}\n\n" + secs(36, NAMES[2:4], sub=["Details"]),
    )
    f["25-report-part"] = doc(
        "report",
        "",
        toc,
        "\\part{Basics}\n\n" + secs(37, ["Getting Started", "Going Further"], cmd="chapter", sub=["Overview"])
        + "\n\n\\part{Advanced Topics}\n\n" + secs(38, ["Beyond"], cmd="chapter", sub=["Details"]),
        clear=False,
    )
    f["26-book-part"] = doc(
        "book",
        "",
        toc,
        "\\mainmatter\n\\part{First Half}\n\n" + secs(39, ["Opening"], cmd="chapter", sub=["Overview"])
        + "\n\n\\part{Second Half}\n\n" + secs(40, ["Closing"], cmd="chapter", sub=["Details"]),
        clear=False,
    )
    f["27-article-part-starred-tocdepth"] = doc(
        "article",
        "",
        toc,
        "\\part*{Prologue}\n\\addcontentsline{toc}{part}{Prologue}\n" + pf.paras(41, 1, 60)
        + "\n\n\\part{Main Matter}\n\n" + secs(42, NAMES[:3], sub=["Overview"]),
        "\\setcounter{tocdepth}{0}\n",
    )
    # `\caption[<short>]{<long>}`: the list shows `<short>`.
    f["28-article-caption-short"] = doc(
        "article",
        "",
        "\\listoffigures\n\\listoftables",
        secs(43, NAMES[:2])
        + "\n\n"
        + pf.paras(44, 1, 70)
        + "\n\n\\begin{figure}[h]\n\\centering\n\\caption[Setup]{The experimental setup, drawn to scale, with every component labelled}\n\\end{figure}\n\n"
        + pf.paras(45, 1, 70)
        + "\n\n\\begin{table}[h]\n\\centering\n\\caption[Short table name]{A long table caption that is only shown under the table itself}\n\\end{table}\n\n"
        + figs("figure", ["Plain caption"], 46),
    )
    # Macros in the entry texts: set like body text.
    f["29-article-inline-macros"] = doc(
        "article",
        "",
        "\\tableofcontents\n\\listoffigures",
        "\\section{The \\emph{Main} Result}\n" + pf.paras(47, 1, 60)
        + "\n\n\\section{Bounds on $x$ and $y$}\n" + pf.paras(48, 1, 60)
        + "\n\n\\subsection{About \\proj{} and \\textbf{bold} words}\n" + pf.paras(49, 1, 60)
        + "\n\n\\section*{Extra}\n\\addcontentsline{toc}{section}{Extra \\emph{notes} on \\proj}\n" + pf.paras(50, 1, 60)
        + "\n\n\\begin{figure}[h]\n\\centering\n\\caption{A \\textbf{bold} caption with $y_1$}\n\\end{figure}\n\n"
        + figs("figure", ["Made with \\proj"], 51),
        "\\newcommand{\\proj}{FlashTeX}\n",
    )
    f["30-report-inline-macros-short"] = doc(
        "report",
        "",
        "\\tableofcontents\n\\listoffigures",
        "\\chapter{The \\emph{First} Chapter}\n" + pf.paras(52, 1, 60)
        + "\n\n\\section{Some \\textit{italic} words}\n" + pf.paras(53, 1, 60)
        + "\n\n\\begin{figure}[h]\n\\centering\n\\caption[Short \\emph{one}]{A long caption for the figure}\n\\end{figure}\n\n"
        + secs(54, ["Second"], cmd="chapter"),
        clear=False,
    )
    # Two-column report/book: the lists are set in one column.
    f["31-report-twocolumn"] = doc(
        "report",
        "twocolumn",
        "\\tableofcontents\n\\listoffigures",
        # A paragraph first: the lists-end probe lands above it, not inside
        # a two-column `\chapter` head (`\@topnewpage`).
        pf.paras(60, 1, 40) + "\n\n"
        + secs(55, ["Getting Started", "Going Further"], cmd="chapter", sub=["Overview", "Details"], words=90)
        + "\n\n" + figs("figure", ["A figure"], 56),
        clear=False,
    )
    f["32-book-twocolumn"] = doc(
        "book",
        "twocolumn",
        toc,
        "\\mainmatter\n" + secs(57, ["Opening", "Middle", "Closing"], cmd="chapter", sub=["Overview"], words=90),
        clear=False,
    )
    # `\clearpage` in two-column mode ends the page, `\newpage` the column.
    f["33-article-twocolumn-newpage"] = doc(
        "article",
        "twocolumn",
        toc,
        "\\newpage\n" + secs(58, NAMES[:3], sub=["Overview"], words=90) + "\n\n\\clearpage\n\n" + secs(59, NAMES[3:5], words=90),
        clear=False,
    )
    # Three figures and two tables over three body pages, one short caption.
    f["34-article-three-figures-two-tables"] = doc(
        "article",
        "",
        "\\listoffigures\n\\listoftables",
        secs(61, NAMES[:2], words=90)
        + "\n\n" + figs("figure", ["Apparatus"], 62)
        + "\n\n" + figs("table", ["Raw measurements"], 63)
        + "\n\n\\clearpage\n\n" + secs(64, NAMES[2:4], words=90)
        + "\n\n" + pf.paras(65, 1, 70)
        + "\n\n\\begin{figure}[h]\n\\centering\n\\caption[Fitted curve]{The fitted curve with its residuals underneath}\n\\end{figure}\n\n"
        + figs("table", ["Summary of the fitted parameters"], 66)
        + "\n\n\\clearpage\n\n" + secs(67, NAMES[4:6], words=90)
        + "\n\n" + figs("figure", ["Outlook"], 68),
    )
    f["35-report-three-figures-two-tables"] = doc(
        "report",
        "",
        "\\listoffigures\n\\listoftables",
        "\\chapter{Measurements}\n" + pf.paras(69, 1, 80)
        + "\n\n" + figs("figure", ["Apparatus", "Detector response"], 70)
        + "\n\n" + figs("table", ["Raw measurements"], 71)
        + "\n\n\\chapter{Analysis}\n" + pf.paras(72, 1, 80)
        + "\n\n\\begin{table}[h]\n\\centering\n\\caption[Fit parameters]{The fitted parameters and their uncertainties}\n\\end{table}\n\n"
        + figs("figure", ["Residuals"], 73),
        clear=False,
    )
    # `\lstlistoflistings`: `\tableofcontents` reading `.lol`
    # (listings.sty `\lstlistoflistings`, `\l@lstlisting`). Only captioned
    # listings step `\c@lstlisting` and write an entry; `nolol` writes none;
    # report numbers by chapter.
    lst = lambda opts, code: f"\\begin{{lstlisting}}[{opts}]\n{code}\n\\end{{lstlisting}}"
    f["36-article-lstlistoflistings"] = doc(
        "article",
        "",
        "\\lstlistoflistings",
        secs(74, NAMES[:1], words=90)
        + "\n\n" + lst("caption={Hello world}", "print hello")
        + "\n\n" + pf.paras(75, 1, 70)
        + "\n\n" + lst("", "uncaptioned")
        + "\n\n" + pf.paras(76, 1, 70)
        + "\n\n\\clearpage\n\n" + secs(77, NAMES[1:2], words=90)
        + "\n\n" + lst("caption={Second listing}", "x = 1")
        + "\n\n" + lst("caption={Not listed},nolol", "y = 2")
        + "\n\n" + pf.paras(78, 1, 70)
        + "\n\n" + lst("caption={Third listing}", "z = 3"),
        "\\usepackage{listings}\n",
    )
    f["37-report-lstlistoflistings"] = doc(
        "report",
        "",
        "\\lstlistoflistings",
        "\\chapter{First}\n" + pf.paras(79, 1, 80)
        + "\n\n" + lst("caption={Setup}", "make setup")
        + "\n\n" + pf.paras(80, 1, 70)
        + "\n\n" + lst("caption={Build}", "make")
        + "\n\n\\chapter{Second}\n" + pf.paras(81, 1, 80)
        + "\n\n" + lst("caption={Deploy}", "make deploy"),
        "\\usepackage{listings}\n",
        clear=False,
    )
    # Nothing but the list: pdflatex still ships the page with its head.
    f["38-article-list-only"] = doc("article", "", "\\listoffigures", "", clear=False)
    return f


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--keep", help="copy the pdflatex PDFs and .toc/.lof/.lot here")
    ap.add_argument("names", nargs="*")
    args = ap.parse_args()
    os.makedirs(EXPECTED, exist_ok=True)
    stale = [n for n in os.listdir(FIXTURES) if n.endswith(".tex")]
    version = subprocess.run(["pdflatex", "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    for name, src in fixtures().items():
        if args.names and name not in args.names:
            continue
        assert src.count(MARK) == 1, name
        tex = os.path.join(FIXTURES, name + ".tex")
        with open(tex, "w") as fh:
            fh.write(src)
        with tempfile.TemporaryDirectory() as tmp:
            probed = os.path.join(tmp, "src", name + ".tex")
            os.makedirs(os.path.dirname(probed))
            with open(probed, "w") as fh:
                fh.write(src.replace(MARK, PROBE))
            pdf = pf.run_pdflatex(probed, tmp)
            # A third run: the lists' own length can move entries by a page.
            subprocess.run(
                ["pdflatex", "-interaction=nonstopmode", "-halt-on-error", f"-jobname={name}", f"-output-directory={tmp}",
                 "\\pdfcompresslevel=0\\pdfobjcompresslevel=0\\input{" + probed + "}"],
                cwd=tmp, capture_output=True, check=True,
            )
            log = open(os.path.join(tmp, name + ".log"), encoding="latin-1").read()
            at = log.index("TOC-ORACLE-LISTS-END")
            # The write runs while its page ships out, after that page's `[<n>`.
            lists_page = len(re.findall(r"\[\d+", log[:at]))
            sx, sy = (int(v) for v in log[at:].split("\n", 1)[0].split()[1:3])
            lists_end = f"{lists_page} {sx / 65536 * pf.BP:.4f} {pf.PAGE_H - sy / 65536 * pf.BP:.4f}"
            if args.keep:
                os.makedirs(args.keep, exist_ok=True)
                shutil.copy(pdf, args.keep)
                for ext in ("toc", "lof", "lot"):
                    aux = os.path.join(tmp, f"{name}.{ext}")
                    if os.path.exists(aux):
                        shutil.copy(aux, args.keep)
            pages = pf.extract(pdf)
        lines = [f"# {version}", f"# source fixtures/toc/{name}.tex (tools/toc-oracle/generate.py)", f"# lists {lists_end}"]
        for p in pages:
            lines.append(f"page {p['number']} {p['width']:.4f} {p['height']:.4f}")
            for w in p["words"]:
                lines.append(f"word {p['number']} {w['x']:.4f} {w['baseline']:.4f} {w['font']} {w['size']:.4f} {w['text']}")
            for r in p["rules"]:
                lines.append(f"rule {p['number']} {r['x']:.4f} {r['top']:.4f} {r['width']:.4f} {r['height']:.4f}")
        with open(os.path.join(EXPECTED, name + ".txt"), "w") as fh:
            fh.write("\n".join(lines) + "\n")
        print(name, len(pages), "pages; lists end on page", lists_end, sum(w["text"] == "." for p in pages[:lists_page] for w in p["words"]), "dots")
    if not args.names:
        for n in stale:
            if n[:-4] not in fixtures():
                os.remove(os.path.join(FIXTURES, n))


if __name__ == "__main__":
    main()
