#!/usr/bin/env python3
"""Regenerate crates/class-geometry/tests/data/oracle.txt from pdflatex.

pdflatex is an ORACLE ONLY: this script is run by hand on a machine with
MacTeX; `cargo test` never runs TeX, it only reads the committed data file.

For every fixture it writes a probe document, runs pdflatex once, and
extracts from the log:
  * `dim <name> <value>`  -- `\\the<length>` after the class, geometry and
    `\\begin{document}` have run (TeX's own printed pt value);
  * `pos <mark> <page> <x_sp> <y_sp>` -- `\\pdfsavepos` of a mark, x from the
    left edge and y from the BOTTOM edge of the page, in scaled points;
  * `box <name> <ht> <dp>` -- heights of glyph-dependent heading lines, so
    tests can check the frame formulas without font metrics.

pdftotext -bbox-layout was not available on the generating machine; the
`\\pdfsavepos` marks are exact to the scaled point instead of 0.01bp.

Usage: python3 oracle/generate.py [--keep DIR]
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "tests", "data", "oracle.txt")

DIMS = [
    "paperwidth", "paperheight", "textwidth", "textheight", "oddsidemargin",
    "evensidemargin", "topmargin", "headheight", "headsep", "footskip",
    "topskip", "baselineskip", "parindent", "parskip", "marginparwidth",
    "marginparsep", "marginparpush", "columnsep", "columnseprule",
    "columnwidth", "maxdepth", "footnotesep", "overfullrule", "leftmargini",
    "labelsep", "pdfpageheight", "pdfpagewidth",
]

# (id, class, class options, geometry options or None, pagestyle or None, body kind)
FIXTURES = []


def fx(ident, cls, opts, geom=None, style=None, body="section", extra="", calls=()):
    FIXTURES.append((ident, cls, opts, geom, style, body, extra, tuple(calls)))


for cls in ("article", "report", "book"):
    for size in ("10pt", "11pt", "12pt"):
        fx(f"{cls}-{size}", cls, size)
for paper in ("a4paper", "a5paper", "b5paper", "legalpaper", "executivepaper"):
    for size in ("10pt", "11pt", "12pt"):
        fx(f"article-{size}-{paper}", "article", f"{size},{paper}")
fx("article-10pt-landscape", "article", "10pt,landscape")
fx("article-12pt-landscape", "article", "12pt,landscape")
fx("article-11pt-a4-landscape", "article", "11pt,a4paper,landscape")
for size in ("10pt", "11pt", "12pt"):
    fx(f"article-{size}-twoside", "article", f"{size},twoside")
fx("report-11pt-twoside", "report", "11pt,twoside")
fx("book-11pt-oneside", "book", "11pt,oneside")
for size in ("10pt", "11pt", "12pt"):
    fx(f"article-{size}-twocolumn", "article", f"{size},twocolumn")
fx("article-11pt-a4-twocolumn", "article", "11pt,a4paper,twocolumn")
fx("book-10pt-twocolumn", "book", "10pt,twocolumn")
fx("article-10pt-titlepage", "article", "10pt,titlepage")
fx("report-10pt-notitlepage", "report", "10pt,notitlepage")
fx("report-12pt-openright-twoside", "report", "12pt,openright,twoside")
fx("book-12pt-openany", "book", "12pt,openany")
fx("article-11pt-fleqn", "article", "11pt,fleqn")
fx("article-10pt-leqno", "article", "10pt,leqno")
fx("article-10pt-draft", "article", "10pt,draft")
# page styles
fx("article-10pt-headings", "article", "10pt", style="headings")
fx("article-11pt-twoside-headings", "article", "11pt,twoside", style="headings")
fx("article-12pt-myheadings", "article", "12pt", style="myheadings")
fx("article-10pt-empty", "article", "10pt", style="empty")
fx("report-10pt-headings", "report", "10pt", style="headings")
# geometry
for size in ("10pt", "11pt", "12pt"):
    fx(f"geom-{size}-margin1in", "article", size, "margin=1in")
fx("geom-a4-margin2cm", "article", "11pt,a4paper", "margin=2cm")
fx("geom-left3cm", "article", "10pt", "left=3cm")
fx("geom-right2cm", "article", "10pt", "right=2cm")
fx("geom-textwidth15cm", "article", "10pt", "textwidth=15cm")
fx("geom-textwidth-left", "article", "10pt", "textwidth=14cm,left=2cm")
fx("geom-hmargin", "article", "11pt", "hmargin={2cm,3cm}")
fx("geom-vmargin", "article", "11pt", "vmargin=1in")
fx("geom-top1in", "article", "10pt", "top=1in")
fx("geom-bottom2cm", "article", "10pt", "bottom=2cm")
fx("geom-textheight20cm", "article", "12pt", "textheight=20cm")
fx("geom-lines40", "article", "10pt", "lines=40")
fx("geom-scale08", "article", "10pt", "scale=0.8")
fx("geom-hscale06", "article", "11pt", "hscale=0.6")
fx("geom-ratio12", "article", "10pt", "ratio=1:2")
fx("geom-hratio-textwidth", "article", "10pt", "textwidth=12cm,hmarginratio=1:2")
fx("geom-includehead", "article", "10pt", "margin=1in,includehead")
fx("geom-includefoot", "article", "10pt", "margin=1in,includefoot")
fx("geom-includeheadfoot", "article", "11pt", "margin=2cm,includeheadfoot")
fx("geom-headfoot-lengths", "article", "10pt", "margin=1in,headheight=15pt,headsep=10pt,footskip=40pt,includeheadfoot")
fx("geom-includemp", "article", "10pt", "margin=2cm,marginparwidth=3cm,marginparsep=5mm,includemp")
fx("geom-bindingoffset-twoside", "article", "10pt,twoside", "margin=2cm,bindingoffset=1cm")
fx("geom-centering", "article", "10pt", "centering")
fx("geom-landscape", "article", "10pt", "landscape,margin=1in")
fx("geom-a5paper", "article", "10pt", "a5paper")
fx("geom-papersize", "article", "10pt", "papersize={15cm,20cm},margin=1.5cm")
fx("geom-twoside-inner-outer", "article", "11pt", "twoside,inner=2cm,outer=4cm")
fx("geom-heightrounded", "article", "11pt", "margin=1in,heightrounded")
fx("geom-nohead", "article", "10pt", "margin=1in,nohead")
fx("geom-nooptions", "article", "10pt", "")
fx("geom-twocolumn-columnsep", "article", "10pt,twocolumn", "margin=0.75in,columnsep=0.25in")
fx("geom-overspec", "article", "10pt", "top=1in,bottom=1in,textheight=20cm")
fx("geom-twoside-default", "article", "10pt,twoside", "")
fx("geom-width-total", "article", "10pt", "width=16cm,height=22cm")
fx("geom-book-twoside-margins", "book", "11pt", "left=2cm,right=3cm,top=2.5cm,bottom=3cm")
# sequential \geometry{} calls (\Gm@clean between them)
fx("geom-calls-margin-then-left", "article", "10pt", "margin=1in", calls=["left=2cm"])
fx("geom-calls-textwidth-then-left", "article", "10pt", "textwidth=12cm", calls=["left=3cm"])
fx("geom-calls-empty-then-a5", "article", "11pt", "", calls=["a5paper,margin=1cm"])
# chapters
for size in ("10pt", "11pt", "12pt"):
    fx(f"report-{size}-chapter", "report", size, body="chapter")
fx("book-10pt-chapter", "book", "10pt", body="chapter")
fx("report-11pt-chapterstar", "report", "11pt", body="chapterstar")
fx("report-10pt-chapter-after-text", "report", "10pt", body="chapter2")
fx("book-11pt-chapter-after-text", "book", "11pt", body="chapter2")

PREAMBLE = r"""\makeatletter
\DeclareRobustCommand\FTmark[1]{\leavevmode\pdfsavepos
  \write16{FTPOS #1 \the\c@page\space\the\pdflastxpos\space\the\pdflastypos}}
\def\FTdim#1{\typeout{FTDIM #1 \expandafter\the\csname #1\endcsname}}
\def\FTcnt#1{\typeout{FTCNT #1 \the\csname c@#1\endcsname}}
\def\FTbox#1#2{\setbox\z@\hbox{#2}\typeout{FTBOX #1 \the\ht\z@\space\the\dp\z@}}
\renewcommand\thepage{\FTmark{pg}\arabic{page}}
\makeatother
"""

SECTION_BODY = r"""\noindent\FTmark{b1}Body line one.\par
\FTmark{b2}Second paragraph indented.\par
\section[Heading]{\FTmark{sec}Heading}
\FTmark{a1}After heading text.\par
\subsection[Sub]{\FTmark{sub}Sub}
\FTmark{a2}After sub.\par
\subsubsection[Subsub]{\FTmark{ssub}Subsub}
\FTmark{a3}After subsub.\par
\paragraph[Para]{\FTmark{para}Para} \FTmark{p1}run in text.\par
\makeatletter\if@twocolumn\newpage\FTmark{col2}Column two.\par\fi\makeatother
\clearpage
\noindent\FTmark{e1}Page two first line.\par
\clearpage
\noindent\FTmark{o1}Page three first line.\par
"""

CHAPTER_BODY = r"""\makeatletter
\FTbox{chapnum}{\normalfont\huge\bfseries \@chapapp\space 1}
\FTbox{chaptitle}{\normalfont\Huge\bfseries Chap}
\makeatother
\chapter[Chap]{\FTmark{chap}Chap}
\FTmark{c1}After chapter.\par
\section[Heading]{\FTmark{sec}Heading}
\FTmark{a1}After heading text.\par
"""

CHAPTERSTAR_BODY = CHAPTER_BODY.replace(r"\chapter[Chap]{", r"\chapter*{")
CHAPTER2_BODY = "\\noindent\\FTmark{t0}Text gjpq.\\par\n" + CHAPTER_BODY


def probe(cls, opts, geom, style, body, extra, calls):
    out = [f"\\documentclass[{opts}]{{{cls}}}\n"]
    if geom is not None:
        out.append(f"\\usepackage[{geom}]{{geometry}}\n")
        for c in calls:
            out.append(f"\\geometry{{{c}}}\n")
    if style:
        out.append(f"\\pagestyle{{{style}}}\n")
    out.append(PREAMBLE)
    out.append("\\begin{document}\n")
    if style == "myheadings":
        out.append("\\markboth{LeftMark}{RightMark}\n")
    for d in DIMS:
        out.append(f"\\FTdim{{{d}}}\n")
    out.append("\\makeatletter\\typeout{FTDIM skipfootins \\the\\skip\\footins}"
               "\\@ifundefined{mathindent}{}{\\typeout{FTDIM mathindent \\the\\mathindent}}"
               "\\typeout{FTDIM em \\the\\fontdimen6\\font}\\typeout{FTDIM ex \\the\\fontdimen5\\font}"
               "\\makeatother\n")
    out.append("\\FTcnt{secnumdepth}\\FTcnt{tocdepth}\n")
    out.append({"section": SECTION_BODY, "chapter": CHAPTER_BODY,
                "chapterstar": CHAPTERSTAR_BODY, "chapter2": CHAPTER2_BODY}[body])
    out.append(extra)
    out.append("\\end{document}\n")
    return "".join(out)


def run(keep):
    work = keep or tempfile.mkdtemp(prefix="class-geometry-oracle-")
    os.makedirs(work, exist_ok=True)
    version = subprocess.run(["pdflatex", "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    lines = [
        "# Generated by crates/class-geometry/oracle/generate.py -- do not edit.",
        f"# Oracle: {version}",
        "# dim <name> <TeX-printed value>; pos <mark> <page> <x sp from left> <y sp from bottom>;",
        "# box <name> <ht> <dp>; cnt <name> <value>",
    ]
    for ident, cls, opts, geom, style, body, extra, calls in FIXTURES:
        tex = os.path.join(work, ident + ".tex")
        with open(tex, "w") as f:
            f.write(probe(cls, opts, geom, style, body, extra, calls))
        env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
        subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", ident + ".tex"],
                       cwd=work, capture_output=True, env=env)
        log = open(os.path.join(work, ident + ".log"), errors="replace").read()
        if "Output written" not in log:
            sys.exit(f"{ident}: pdflatex failed; see {work}/{ident}.log")
        lines.append("")
        lines.append(f"[{ident}]")
        lines.append(f"class {cls}")
        lines.append(f"options {opts}")
        lines.append(f"geometry {geom}" if geom is not None else "geometry -")
        lines.append(f"calls {'|'.join(calls) if calls else '-'}")
        lines.append(f"pagestyle {style or '-'}")
        lines.append(f"body {body}")
        seen = set()
        for m in re.finditer(r"^FTDIM (\S+) (\S+)$", log, re.M):
            lines.append(f"dim {m.group(1)} {m.group(2)}")
        for m in re.finditer(r"^FTCNT (\S+) (\S+)$", log, re.M):
            lines.append(f"cnt {m.group(1)} {m.group(2)}")
        for m in re.finditer(r"^FTBOX (\S+) (\S+) (\S+)$", log, re.M):
            lines.append(f"box {m.group(1)} {m.group(2)} {m.group(3)}")
        for m in re.finditer(r"^FTPOS (\S+) (\d+) (-?\d+) (-?\d+)$", log, re.M):
            key = (m.group(1), m.group(2))
            if key in seen:
                continue
            seen.add(key)
            lines.append(f"pos {m.group(1)} {m.group(2)} {m.group(3)} {m.group(4)}")
    with open(OUT, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"{len(FIXTURES)} fixtures -> {os.path.relpath(OUT)} (work dir {work})")
    if not keep:
        shutil.rmtree(work)


if __name__ == "__main__":
    keep = sys.argv[2] if len(sys.argv) > 2 and sys.argv[1] == "--keep" else None
    run(keep)
