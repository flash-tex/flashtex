#!/usr/bin/env python3
"""Footnote oracle: \\footnote, \\footnotemark/\\footnotetext, page-bottom
\\footins material, \\footnoterule, splitting and two-column footnotes.

pdflatex is the ORACLE ONLY (never in the product path, never in cargo
tests). This script writes fixtures/footnotes/<name>.tex, runs pdflatex and
records every word's origin/baseline and every rule of the PDF in
fixtures/footnotes/expected/<name>.txt, with the same content-stream reader
and record format as tools/page-frame-oracle/generate.py (imported from
there):

  page <n> <width> <height>
  word <page> <x> <baseline> <font> <size> <text>
  rule <page> <x> <top> <width> <height>

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
FIXTURES = os.path.join(CRATE, "fixtures", "footnotes")
EXPECTED = os.path.join(FIXTURES, "expected")

_spec = importlib.util.spec_from_file_location("pageframe", os.path.join(CRATE, "tools", "page-frame-oracle", "generate.py"))
pf = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pf)

sentences = pf.sentences
paras = pf.paras
doc = pf.doc


def note(seed, words):
    return pf.sentences(seed, words)


def with_notes(seed, count, notes_at, words=90):
    """`count` paragraphs; `notes_at` maps paragraph index -> list of
    (word position, footnote argument text)."""
    out = []
    for i in range(count):
        ws = sentences(seed * 100 + i, words).split(" ")
        for pos, text in sorted(notes_at.get(i, []), reverse=True):
            pos = min(pos, len(ws) - 1)
            ws[pos] = ws[pos] + "\\footnote{" + text + "}"
        out.append(" ".join(ws))
    return "\n\n".join(out)


def fixtures():
    f = {}
    f["01-single"] = doc("article", "", with_notes(1, 4, {1: [(20, note(11, 12))]}))
    f["02-multiple"] = doc(
        "article",
        "",
        with_notes(2, 9, {0: [(10, note(21, 10)), (50, note(22, 25))], 2: [(30, note(23, 8))], 6: [(15, note(24, 30))], 7: [(40, note(25, 6))]}),
    )
    f["03-long-split"] = doc("article", "", with_notes(3, 9, {4: [(60, note(31, 700))]}))
    f["04-near-bottom"] = doc("article", "", with_notes(4, 8, {5: [(85, note(41, 40))], 6: [(5, note(42, 20))]}))
    f["05-footnotemark-text"] = doc(
        "article",
        "",
        paras(5, 1)
        + " Here\\footnotemark{} and there\\footnotemark{} are two marks.\n\\footnotetext[1]{"
        + note(51, 15)
        + "}\n\\footnotetext{"
        + note(52, 12)
        + "}\n\n"
        + paras(6, 2),
    )
    f["06-optional-number"] = doc("article", "", with_notes(7, 3, {1: [(10, note(61, 10))]}).replace("\\footnote{", "\\footnote[7]{", 1))
    f["07-twoside-flushbottom"] = doc("article", "twoside", with_notes(8, 16, {2: [(30, note(71, 20))], 9: [(10, note(72, 40))], 13: [(70, note(73, 15))]}))
    f["08-11pt"] = doc("article", "11pt", with_notes(9, 10, {1: [(25, note(81, 30))], 7: [(40, note(82, 18))]}))
    f["09-12pt"] = doc("article", "12pt", with_notes(10, 10, {1: [(25, note(91, 30))], 7: [(40, note(92, 18))]}))
    f["10-report-chapter-reset"] = doc(
        "report",
        "",
        "\\chapter{First}\n" + with_notes(11, 3, {0: [(20, note(101, 10))], 2: [(30, note(102, 12))]}) + "\n\n\\chapter{Second}\n" + with_notes(12, 3, {1: [(20, note(103, 14))]}),
    )
    f["11-twocolumn"] = doc("article", "twocolumn", with_notes(13, 20, {1: [(20, note(111, 25))], 6: [(30, note(112, 15))], 12: [(10, note(113, 20))]}))
    f["12-thanks"] = doc(
        "article",
        "",
        "\\maketitle\n" + paras(14, 3),
        "\\title{A Title\\thanks{Supported by a grant.}}\n\\author{An Author\\thanks{Corresponding author.}}\n\\date{3 July 2022}\n",
    )
    f["13-minipage"] = doc(
        "article",
        "",
        paras(15, 1)
        + "\n\n\\noindent\\begin{minipage}{0.6\\textwidth}\n"
        + sentences(1501, 40).replace(" and ", " and\\footnote{" + note(151, 10) + "} ", 1)
        + "\n\\end{minipage}\n\n"
        + paras(16, 1),
    )
    f["14-multipar-note"] = doc("article", "", with_notes(17, 4, {1: [(20, note(171, 30) + "\n\n" + note(172, 25))]}))
    f["15-math-note"] = doc("article", "", with_notes(18, 4, {1: [(20, "Where $x^2+y^2=z^2$ holds for " + note(181, 10))]}))
    f["16-many-notes"] = doc(
        "article",
        "",
        with_notes(19, 6, {0: [(k * 9 + 3, note(190 + k, 30)) for k in range(9)], 1: [(k * 9 + 3, note(200 + k, 30)) for k in range(9)]}),
    )
    f["17-last-page"] = doc("article", "", with_notes(20, 5, {4: [(30, note(211, 20))]}))
    f["18-list-item"] = doc(
        "article",
        "",
        paras(21, 1)
        + "\n\n\\begin{itemize}\n\\item First item with a note\\footnote{"
        + note(221, 12)
        + "} inside it.\n\\item Second item.\n\\end{itemize}\n\n"
        + paras(22, 2),
    )
    f["19-adjacent-marks"] = doc("article", "", paras(23, 1) + " Several\\footnote{" + note(231, 5) + "}\\footnote{" + note(232, 7) + "} notes.\n\n" + paras(24, 2))
    f["20-twocolumn-split"] = doc("article", "twocolumn", with_notes(25, 20, {3: [(60, note(251, 400))]}))
    f["21-book-chapter"] = doc("book", "", "\\chapter{Opening}\n" + with_notes(26, 8, {1: [(20, note(261, 15))], 5: [(40, note(262, 20))]}))
    f["22-newpage"] = doc("article", "", with_notes(27, 2, {1: [(20, note(271, 15))]}) + "\n\n\\newpage\n\n" + with_notes(28, 3, {0: [(30, note(272, 10))]}))
    f["23-sections"] = doc(
        "article",
        "",
        "\\section{Introduction}\n" + with_notes(29, 3, {0: [(15, note(291, 12))]}) + "\n\n\\section{Method}\n" + with_notes(30, 5, {2: [(40, note(292, 18))], 4: [(30, note(293, 10))]}),
    )
    f["24-footnotemark-optional"] = doc(
        "article",
        "",
        paras(31, 1) + " A mark\\footnotemark[3] here.\n\\footnotetext[3]{" + note(311, 14) + "}\n\n" + paras(32, 2),
    )
    # Floats with footnotes. The float bodies are `\\includegraphics` with
    # both width and height (the pipeline keeps that box when the image is
    # not in the project); pdflatex finds the image through TEXINPUTS.
    gfx = "\\usepackage{graphicx}\n"

    def fig(pos, h="1in", cap="A figure."):
        return "\\begin{figure}[" + pos + "]\n\\centering\n\\includegraphics[width=2in,height=" + h + "]{images/red-72.png}\n\\caption{" + cap + "}\n\\end{figure}\n"

    f["25-float-top-note"] = doc("article", "", with_notes(33, 2, {1: [(20, note(331, 15))]}) + "\n\n" + fig("t") + "\n" + with_notes(34, 3, {0: [(30, note(332, 12))]}), gfx)
    f["26-float-bottom-note"] = doc("article", "", with_notes(35, 2, {1: [(20, note(351, 15))]}) + "\n\n" + fig("b") + "\n" + with_notes(36, 3, {0: [(30, note(352, 12))]}), gfx)
    f["27-float-top-bottom-notes"] = doc(
        "article", "", with_notes(37, 1, {0: [(40, note(371, 20))]}) + "\n\n" + fig("t") + "\n" + fig("b", "0.8in", "Another figure.") + "\n" + with_notes(38, 3, {1: [(30, note(372, 14))]}), gfx
    )
    f["28-float-page-note"] = doc("article", "", with_notes(39, 4, {1: [(20, note(391, 18))]}) + "\n\n" + fig("p", "5in") + "\n" + with_notes(40, 6, {3: [(25, note(392, 16))]}), gfx)
    f["29-float-here-note"] = doc("article", "", with_notes(41, 2, {0: [(30, note(411, 12))]}) + "\n\n" + fig("h") + "\n" + with_notes(42, 3, {0: [(20, note(412, 15))]}), gfx)
    f["30-float-split-note"] = doc("article", "", with_notes(43, 1, {}) + "\n\n" + fig("t", "2in") + "\n" + with_notes(44, 8, {3: [(60, note(431, 500))]}), gfx)
    f["31-float-deferred-notes"] = doc(
        "article",
        "",
        with_notes(45, 1, {0: [(20, note(451, 10))]}) + "\n\n" + fig("t", "3in") + "\n" + fig("t", "3in", "Deferred figure.") + "\n" + with_notes(46, 9, {2: [(30, note(452, 20))], 7: [(40, note(453, 25))]}),
        gfx,
    )
    f["32-thanks-authors"] = doc(
        "article",
        "",
        "\\maketitle\n" + with_notes(47, 3, {1: [(30, note(471, 12))]}),
        "\\title{On Notes\\thanks{Draft version.}}\n\\author{First Author\\thanks{First affiliation.} \\and Second Author\\thanks{Second affiliation, with a longer note that " + note(472, 20) + "}}\n\\date{4 July 2022\\thanks{Revised.}}\n",
    )
    f["33-thanks-body-notes"] = doc(
        "article",
        "11pt",
        "\\maketitle\n" + with_notes(48, 4, {0: [(20, note(481, 10))], 2: [(40, note(482, 14))]}),
        "\\title{Counting Again\\thanks{" + note(483, 15) + "}}\n\\author{Some One}\n\\date{5 July 2022}\n",
    )
    f["34-report-chapters"] = doc(
        "report",
        "",
        "\\chapter{Alpha}\n"
        + with_notes(49, 2, {0: [(20, note(491, 10))], 1: [(30, note(492, 12))]})
        + "\n\n\\chapter*{Interlude}\n"
        + with_notes(50, 1, {0: [(25, note(501, 10))]})
        + "\n\n\\chapter{Beta}\n"
        + with_notes(51, 2, {1: [(20, note(511, 14))]}),
    )
    f["35-book-chapters"] = doc(
        "book",
        "",
        "\\chapter{One}\n" + with_notes(52, 2, {0: [(15, note(521, 12))], 1: [(35, note(522, 10))]}) + "\n\n\\chapter{Two}\n" + with_notes(53, 2, {0: [(30, note(531, 16))]}),
    )
    f["36-math-note-11pt"] = doc("article", "11pt", with_notes(54, 4, {1: [(20, "Here $a_i^2+b_{j}=c$ and " + note(541, 12))]}))
    f["37-math-note-12pt"] = doc("article", "12pt", with_notes(55, 4, {1: [(20, "Here $x^{n}-y_{k}$ and " + note(551, 12))]}))
    f["38-math-note-fraction"] = doc("article", "", with_notes(56, 4, {1: [(20, "Since $\\frac{a}{b}\\le\\sqrt{2}$ with $\\sum_{k=1}^{n} k$ " + note(561, 10))]}))
    f["39-minipage-notes"] = doc(
        "article",
        "",
        paras(57, 1)
        + "\n\n\\noindent\\begin{minipage}{0.6\\textwidth}\n"
        + sentences(5701, 30)
        + "\\footnote{"
        + note(571, 10)
        + "} "
        + sentences(5702, 20)
        + "\\footnote{"
        + note(572, 8)
        + "}\n\\end{minipage}\n\n"
        + paras(58, 1),
    )
    return f


HRULE = re.compile(
    rb"1 0 0 1 (-?[\d.]+) (-?[\d.]+) cm\s*\[\]0 d 0 J (-?[\d.]+) w 0 0 m (-?[\d.]+) 0 l S"
)


def horizontal_rules(pdf):
    """pdfTeX strokes an \\hrule wider than it is tall as a line at its
    vertical centre (`1 0 0 1 x y cm [...] w 0 0 m len 0 l S`), which the
    page-frame reader (vertical strokes and `re` only) skips: rules per page
    as (x, top, width, height) in bp with a top-left origin."""
    data = open(pdf, "rb").read()
    objs = pf.read_objects(data)
    out = []
    for p in pf.pages_in_order(objs):
        mb = pf.media_box(objs, p)
        height = mb[3] - mb[1]
        cm = re.search(rb"/Contents\s*(\d+) 0 R", objs[p])
        content = pf.stream_of(objs[int(cm.group(1))])
        rules = []
        for m in HRULE.finditer(content):
            x, y, w, length = (float(g) for g in m.groups())
            rules.append({"x": round(x, 4), "top": round(height - (y + w / 2), 4), "width": round(length, 4), "height": round(w, 4)})
        out.append(rules)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--keep", help="copy the pdflatex PDFs here")
    ap.add_argument("names", nargs="*")
    args = ap.parse_args()
    os.makedirs(EXPECTED, exist_ok=True)
    # The float fixtures' images live with the float fixtures.
    os.environ["TEXINPUTS"] = os.path.join(CRATE, "fixtures", "floats") + ":"
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
            for page, extra in zip(pages, horizontal_rules(pdf)):
                page["rules"].extend(extra)
        lines = [f"# {version}", f"# source fixtures/footnotes/{name}.tex (tools/footnotes-oracle/generate.py)"]
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
