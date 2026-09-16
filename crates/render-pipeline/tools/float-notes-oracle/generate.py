#!/usr/bin/env python3
"""Footnotes *and* floats on the same page: the `\\@makecol` column that
LaTeX's default `build/column/outputbox` plug (`footnotes-floats-legacy`)
assembles as

    top floats, \\floatsep between them, \\textfloatsep,
    the body, the \\vfil \\@outputbox@removebskip lifted off its end,
    \\skip\\footins, \\footnoterule, the notes,
    \\textfloatsep, the bottom floats, \\vskip-\\dp, \\@textbottom

packed `\\vbox to\\@colht`, plus the page-goal side: floats reserve space
through `\\@colroom` while the `\\footins` insertion charges `\\skip\\footins`
and each note against `\\pagegoal` (TeX §1008-§1010), and
`\\@specialoutput` adds `\\ht\\footins + \\skip\\footins + \\dp\\footins` to
`\\@pageht` before `\\@addtocurcol` decides whether a float still fits.

pdflatex is the ORACLE ONLY (never in the product path, never in cargo
tests). This script writes fixtures/float-notes/<name>.tex, runs pdflatex
from the fixture directory (so `images/...` resolves) and records every
word's origin/baseline, every rule and every image rectangle of the PDF in
fixtures/float-notes/expected/<name>.txt:

  page <n> <width> <height>
  word <page> <x> <baseline> <font> <size> <text>
  rule <page> <x> <top> <width> <height>
  image <page> <x> <top> <width> <height>

The first comment line records the pdfTeX that produced the file, which is
this file's `reference_engine`: these fixtures were generated with TeX Live
2025 on a Linux host, not with the MacTeX 2026 that the older committed
expected data under fixtures/footnotes/ and fixtures/floats/ carries. Never
regenerate those from here.

Usage: generate.py [--keep DIR] [NAME ...]
"""
import argparse
import importlib.util
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(os.path.dirname(HERE))
FIXTURES = os.path.join(CRATE, "fixtures", "float-notes")
EXPECTED = os.path.join(FIXTURES, "expected")

_spec = importlib.util.spec_from_file_location("pageframe", os.path.join(CRATE, "tools", "page-frame-oracle", "generate.py"))
pf = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pf)

sentences = pf.sentences

GREEN = "images/green-144dpi.png"   # 150.01 x 100.01 bp
RED = "images/red-72.png"           # 144 x 72 bp
TALL = "images/tall-72.png"         # 120 x 520 bp


def doc(opts, body, preamble=""):
    o = f"[{opts}]" if opts else ""
    return (
        f"\\documentclass{o}{{article}}\n"
        "\\usepackage[T1]{fontenc}\n"
        "\\usepackage{lmodern}\n"
        "\\usepackage{graphicx}\n"
        f"{preamble}"
        "\\begin{document}\n"
        f"{body}\n"
        "\\end{document}\n"
    )


def note(seed, words):
    return sentences(seed, words)


def para(seed, words=90, notes=()):
    """One paragraph; `notes` is a sequence of (word position, note text)."""
    ws = sentences(seed, words).split(" ")
    for pos, text in sorted(notes, reverse=True):
        pos = min(pos, len(ws) - 1)
        ws[pos] = ws[pos] + "\\footnote{" + text + "}"
    return " ".join(ws)


def paras(seed, count, words=90, notes_at=None):
    notes_at = notes_at or {}
    return "\n\n".join(para(seed * 100 + i, words, notes_at.get(i, ())) for i in range(count))


def figure(where, image, caption):
    return f"\\begin{{figure}}[{where}]\n\\centering\n\\includegraphics{{{image}}}\n\\caption{{{caption}}}\n\\end{{figure}}"


def fixtures():
    f = {}
    # A `[t]` float and a note anchored on the same page: `\@colroom` is
    # short by the float and `\pagegoal` by `\skip\footins` plus the note.
    f["01-top-float-note"] = doc(
        "",
        para(11, 90, [(20, note(111, 14))])
        + "\n\n"
        + figure("t", GREEN, "A top float on a page with a note.")
        + "\n\n"
        + paras(12, 4),
    )
    # `\@cflb`: the notes attach to the body before the floats wrap around
    # it, so the bottom float is set *below* the `\footnoterule` and its
    # notes, `\textfloatsep` under them.
    f["02-bottom-float-note"] = doc(
        "",
        paras(21, 3, notes_at={0: [(30, note(211, 16))]})
        + "\n\n"
        + figure("b", RED, "A bottom float under the notes.")
        + "\n\n"
        + paras(22, 3),
    )
    # A note long enough that `\@pageht` (which counts `\ht\footins`,
    # `\skip\footins` and `\dp\footins`) makes `\@addtocurcol` refuse the
    # float it would otherwise take: it is deferred to the next column.
    f["03-note-defers-float"] = doc(
        "",
        para(31, 330, [(300, note(311, 150))])
        + "\n\n"
        + figure("t", TALL, "Deferred because the note took the room.")
        + "\n\n"
        + paras(32, 3),
    )
    # Three `[t]` floats and notes on the pages they compete for: with
    # `topnumber` 2 and `\topfraction` .7 one is deferred past the notes.
    f["04-deferred-float-notes"] = doc(
        "",
        paras(41, 2, notes_at={0: [(25, note(411, 12))], 1: [(40, note(412, 18))]})
        + "\n\n"
        + figure("t", GREEN, "First of three.")
        + "\n\n"
        + figure("t", RED, "Second of three.")
        + "\n\n"
        + figure("t", GREEN, "Third of three.")
        + "\n\n"
        + paras(42, 6, notes_at={1: [(30, note(413, 20))]}),
    )
    # A note too tall for what is left of `\@colroom` after the float:
    # §1010 splits it, the remainder is held over and set at the foot of
    # the next column.
    f["05-split-note-float"] = doc(
        "",
        paras(51, 3)
        + "\n\n"
        + figure("t", GREEN, "Above a split note.")
        + "\n\n"
        + para(52, 90, [(60, note(511, 650))])
        + "\n\n"
        + paras(53, 4),
    )
    # `[p]` floats: `\@tryfcolumn` makes a float page, which takes no body
    # text, while a note anchored in the body waits for the next column.
    f["06-float-page-note"] = doc(
        "",
        paras(61, 2, notes_at={1: [(40, note(611, 25))]})
        + "\n\n"
        + figure("p", TALL, "On a float page of its own.")
        + "\n\n"
        + paras(62, 5, notes_at={2: [(20, note(612, 15))]}),
    )
    # An `h` float (`\@midlist`, `\@textfloatsheight`, `\intextsep`) on a
    # page whose notes are charged against the same goal.
    f["07-here-float-note"] = doc(
        "",
        paras(71, 2, notes_at={0: [(35, note(711, 14))]})
        + "\n\n"
        + figure("h", RED, "Set where it stands.")
        + "\n\n"
        + paras(72, 3, notes_at={0: [(15, note(712, 20))]}),
    )
    # Two columns: each column is its own `\@colroom` and its own
    # `\footins` (`\@outputdblcol` ships them side by side).
    f["08-twocolumn-float-note"] = doc(
        "twocolumn",
        paras(81, 4, notes_at={0: [(20, note(811, 18))], 2: [(30, note(812, 14))]})
        + "\n\n"
        + figure("t", RED, "A column float with notes.")
        + "\n\n"
        + paras(82, 8, notes_at={3: [(25, note(813, 22))]}),
    )
    # `\clearpage` with `\footins` not void: `\@doclearpage` ships one more
    # ordinary column (`\box\@cclv\vfil`, `\@makecol`) before `\@makefcolumn`
    # sets the floats left over on float pages.
    f["09-clearpage-note-float"] = doc(
        "",
        paras(91, 2, notes_at={1: [(50, note(911, 20))]})
        + "\n\n"
        + figure("t", TALL, "Left over at the clearpage.")
        + "\n\n"
        + figure("t", GREEN, "Left over as well.")
        + "\n\n"
        + para(92, 40, [(20, note(912, 30))]),
    )
    # A top float, a bottom float and several notes on one page: every glue
    # of `\@makecol`'s column (the two `\textfloatsep`s, `\skip\footins`,
    # the body's own) is set by the single `\vbox to\@colht` ratio.
    f["10-top-bottom-notes"] = doc(
        "",
        para(101, 90, [(10, note(1011, 12)), (60, note(1012, 16))])
        + "\n\n"
        + figure("t", RED, "Above the text.")
        + "\n\n"
        + paras(102, 2, notes_at={0: [(45, note(1013, 10))]})
        + "\n\n"
        + figure("b", GREEN, "Below the notes.")
        + "\n\n"
        + paras(103, 4),
    )
    return f


HRULE = re.compile(rb"1 0 0 1 (-?[\d.]+) (-?[\d.]+) cm\s*\[\]0 d 0 J (-?[\d.]+) w 0 0 m (-?[\d.]+) 0 l S")
XOBJ = re.compile(rb"q\s+(-?[\d.]+) 0 0 (-?[\d.]+) (-?[\d.]+) (-?[\d.]+) cm\s*/\w+ Do")


def run_pdflatex(tex_path, workdir):
    """pdflatex with the fixture directory as the working directory, so the
    fixtures' relative `images/...` paths resolve."""
    name = os.path.splitext(os.path.basename(tex_path))[0]
    cmd = [
        "pdflatex",
        "-interaction=nonstopmode",
        "-halt-on-error",
        f"-jobname={name}",
        f"-output-directory={workdir}",
        "\\pdfcompresslevel=0\\pdfobjcompresslevel=0\\input{" + tex_path + "}",
    ]
    for _ in range(2):
        r = subprocess.run(cmd, cwd=FIXTURES, capture_output=True, text=True)
        if r.returncode != 0:
            sys.exit(f"pdflatex failed for {name}:\n{r.stdout[-3000:]}")
    return os.path.join(workdir, name + ".pdf")


def rules_and_images(pdf):
    """Per page: the horizontal rules pdfTeX strokes as a wide thin line
    (the `\\footnoterule`, which the page-frame reader's `re`/vertical-stroke
    scan skips) and every image XObject rectangle, in bp from a top-left
    origin."""
    data = open(pdf, "rb").read()
    objs = pf.read_objects(data)
    out = []
    for p in pf.pages_in_order(objs):
        mb = pf.media_box(objs, p)
        height = mb[3] - mb[1]
        cm = re.search(rb"/Contents\s*(\d+) 0 R", objs[p])
        content = pf.stream_of(objs[int(cm.group(1))])
        rules = [
            {"x": round(float(m.group(1)), 4), "top": round(height - (float(m.group(2)) + float(m.group(3)) / 2), 4),
             "width": round(float(m.group(4)), 4), "height": round(float(m.group(3)), 4)}
            for m in HRULE.finditer(content)
        ]
        images = [
            {"x": round(float(m.group(3)), 4), "top": round(height - (float(m.group(4)) + float(m.group(2))), 4),
             "width": round(float(m.group(1)), 4), "height": round(float(m.group(2)), 4)}
            for m in XOBJ.finditer(content)
        ]
        out.append((rules, images))
    return out


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
            pdf = run_pdflatex(tex, tmp)
            if args.keep:
                os.makedirs(args.keep, exist_ok=True)
                shutil.copy(pdf, args.keep)
            pages = pf.extract(pdf)
            extra = rules_and_images(pdf)
        lines = [f"# {version}", f"# source fixtures/float-notes/{name}.tex (tools/float-notes-oracle/generate.py)"]
        for p, (rules, images) in zip(pages, extra):
            lines.append(f"page {p['number']} {p['width']:.4f} {p['height']:.4f}")
            for w in p["words"]:
                lines.append(f"word {p['number']} {w['x']:.4f} {w['baseline']:.4f} {w['font']} {w['size']:.4f} {w['text']}")
            for r in list(p["rules"]) + rules:
                lines.append(f"rule {p['number']} {r['x']:.4f} {r['top']:.4f} {r['width']:.4f} {r['height']:.4f}")
            for im in images:
                lines.append(f"image {p['number']} {im['x']:.4f} {im['top']:.4f} {im['width']:.4f} {im['height']:.4f}")
        with open(os.path.join(EXPECTED, name + ".txt"), "w") as fh:
            fh.write("\n".join(lines) + "\n")
        print(name, len(pages), "pages", sum(len(p["words"]) for p in pages), "words")


if __name__ == "__main__":
    main()
