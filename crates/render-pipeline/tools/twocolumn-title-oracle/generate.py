#!/usr/bin/env python3
"""`\\twocolumn[\\@maketitle]` with floats: the `\\@topnewpage` page.

`\\maketitle` in a two-column document expands to `\\twocolumn[\\@maketitle]`
(article.cls), and `\\twocolumn`'s optional argument goes through
`\\@topnewpage` (latex.ltx 20466-20505):

    \\global\\setbox\\@currbox\\vbox{\\hsize\\textwidth \\@parboxrestore
                                 \\col@number\\@ne <material>
                                 \\vskip -\\dbltextfloatsep}
    \\ifdim \\ht\\@currbox>\\textheight \\ht\\@currbox \\textheight \\fi
    \\global \\count\\@currbox \\tw@
    \\@tempdima -\\ht\\@currbox \\advance \\@tempdima -\\dbltextfloatsep
    \\global \\advance \\@colht \\@tempdima
    \\@cons \\@dbltoplist \\@currbox
    \\global \\@dbltopnum \\m@ne
    ... \\global\\vsize\\@colht \\global\\@colroom\\@colht \\@floatplacement

so `\\@colht` loses the material's *natural* height (the box is that height
less `\\dbltextfloatsep`, and `\\dbltextfloatsep` is subtracted again) for
both columns of the page, and `\\@combinedblfloats` (21019) sets the box,
`\\vskip\\dbltextfloatsep` and the two-column box inside one `\\vbox
to\\textheight`. `\\@outputpage` restores `\\global\\@colht\\textheight` when
the page ships, and `\\@opcol`'s trailing `\\@floatplacement` recomputes
`\\@toproom`/`\\@botroom`/`\\@fpmin` from whichever `\\@colht` is in force --
the shortened one for the second column of this page too, because
`\\@outputdblcol` only reaches `\\@outputpage` after that column.

pdflatex is the ORACLE ONLY (never in the product path, never in cargo
tests). This script writes fixtures/twocolumn-title/<name>.tex, runs
pdflatex from the fixture directory (so `images/...` resolves) and records
every word's origin/baseline, every rule and every image rectangle of the
PDF in fixtures/twocolumn-title/expected/<name>.txt:

  page <n> <width> <height>
  word <page> <x> <baseline> <font> <size> <text>
  rule <page> <x> <top> <width> <height>
  image <page> <x> <top> <width> <height>

The first comment line records the pdfTeX that produced the file, which is
this file's `reference_engine`: these fixtures were generated with TeX Live
2025 on a Linux host, like fixtures/float-notes/ and unlike the MacTeX 2026
data under fixtures/footnotes/ and fixtures/floats/. Never regenerate those
from here.

Usage: generate.py [--keep DIR] [NAME ...]
"""
import argparse
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(os.path.dirname(HERE))
FIXTURES = os.path.join(CRATE, "fixtures", "twocolumn-title")
EXPECTED = os.path.join(FIXTURES, "expected")

_spec = importlib.util.spec_from_file_location("pageframe", os.path.join(CRATE, "tools", "page-frame-oracle", "generate.py"))
pf = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pf)

_spec2 = importlib.util.spec_from_file_location("floatnotes", os.path.join(CRATE, "tools", "float-notes-oracle", "generate.py"))
fn = importlib.util.module_from_spec(_spec2)
_spec2.loader.exec_module(fn)

sentences = pf.sentences
paras = pf.paras
rules_and_images = fn.rules_and_images

GREEN = "images/green-144dpi.png"   # 150.01 x 100.01 bp
RED = "images/red-72.png"           # 144 x 72 bp
TALL = "images/tall-72.png"         # 120 x 520 bp

TITLE = (
    "\\title{Adaptive Column Balance in a Spanning Title}\n"
    "\\author{A.~Author \\and B.~Coauthor}\n"
    "\\date{}\n"
)


def doc(opts, body, preamble=""):
    o = f"[{opts}]" if opts else ""
    return (
        f"\\documentclass{o}{{article}}\n"
        "\\usepackage[T1]{fontenc}\n"
        "\\usepackage{lmodern}\n"
        "\\usepackage{graphicx}\n"
        f"{TITLE}{preamble}"
        "\\begin{document}\n"
        "\\maketitle\n"
        f"{body}\n"
        "\\end{document}\n"
    )


def figure(where, image, caption, star=False):
    env = "figure*" if star else "figure"
    return f"\\begin{{{env}}}[{where}]\n\\centering\n\\includegraphics{{{image}}}\n\\caption{{{caption}}}\n\\end{{{env}}}"


def fixtures():
    f = {}
    # The reference case: `\@topnewpage` alone. Both columns of page 1 are
    # `\textheight` less the title's natural height, and the body starts
    # `\dbltextfloatsep` under the box `\@combinedblfloats` sets.
    f["01-title-no-floats"] = doc("twocolumn", paras(11, 9))
    # A `[t]` float on page 1: `\@addtocurcol` sees the *shortened*
    # `\@colroom` and the `\@toproom` = `\topfraction\@colht` that
    # `\@topnewpage`'s own `\@floatplacement` computed from it.
    f["02-title-top-float"] = doc(
        "twocolumn",
        paras(21, 2) + "\n\n" + figure("t", GREEN, "A top float under the spanning title.") + "\n\n" + paras(22, 8),
    )
    # A float taller than what the shortened column leaves: it cannot go at
    # the top of either column of page 1 (`\@toproom` is .7 of the reduced
    # `\@colht`) and is deferred to a column whose `\@colht` is
    # `\textheight` again.
    f["03-title-float-too-large"] = doc(
        "twocolumn",
        paras(31, 2) + "\n\n" + figure("t", TALL, "Too large for the shortened column.") + "\n\n" + paras(32, 9),
    )
    # A full-width `figure*` beside the spanning title. A double float is
    # only ever offered to a page by `\@startdblcolumn` *after*
    # `\@outputpage`, so it lands on page 2 here; `\@topnewpage`'s
    # `\global\@dbltopnum\m@ne` is the extra bar that keeps any later
    # full-width float off the page a spanning title opened (verified
    # against a no-`\maketitle` control: this fixture's float is on page 2
    # either way, so the fixture pins the spanning title's own geometry with
    # a full-width float in the document, not the `\@dbltopnum` rule).
    f["04-title-figure-star"] = doc(
        "twocolumn",
        paras(41, 2) + "\n\n" + figure("t", RED, "A full-width float beside the title.", star=True) + "\n\n" + paras(42, 9),
    )
    return f


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--keep", help="copy the pdflatex PDFs here")
    ap.add_argument("names", nargs="*")
    args = ap.parse_args()
    os.makedirs(EXPECTED, exist_ok=True)
    if not os.path.isdir(os.path.join(FIXTURES, "images")):
        sys.exit(f"missing {FIXTURES}/images (copy the three PNGs from fixtures/float-notes/images)")
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
        lines = [f"# {version}", f"# source fixtures/twocolumn-title/{name}.tex (tools/twocolumn-title-oracle/generate.py)"]
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


if __name__ == "__main__":
    main()
