#!/usr/bin/env python3
"""Float *bodies* that are not pictures, against pdflatex.

`\\@xfloat` opens the float's box as

    \\vbox{\\hsize\\columnwidth \\@parboxrestore \\@floatboxreset <body>}

so the body is ordinary vertical material: a `tabular` is an hbox on a line
of the box's vertical list, a list is a `\\list`, a display carries its own
`\\abovedisplayskip`, and a paragraph is broken at `\\columnwidth` with
`\\parindent` and `\\parskip` zero and `\\sloppy` in force. `\\@makecaption`
adds `\\vskip\\abovecaptionskip` and the numbered caption wherever
`\\caption` stands in the body.

pdflatex is the ORACLE ONLY (never in the product path, never in cargo
tests). Unlike the other generators here the fixtures are **written by
hand** and checked in: this script only runs pdflatex over
fixtures/float-body/*.tex and records every word's origin/baseline, every
rule (the tabular's own rules included, as `re` rectangles) and every image
rectangle of the PDF in fixtures/float-body/expected/<name>.txt:

  page <n> <width> <height>
  word <page> <x> <baseline> <font> <size> <text>
  rule <page> <x> <top> <width> <height>
  image <page> <x> <top> <width> <height>

The first comment line records the pdfTeX that produced the file, which is
this suite's `reference_engine`: TeX Live 2025 on a Linux host. The MacTeX
2026 data under fixtures/footnotes/ and fixtures/floats/ is never
regenerated from here.

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
FIXTURES = os.path.join(CRATE, "fixtures", "float-body")
EXPECTED = os.path.join(FIXTURES, "expected")

_spec = importlib.util.spec_from_file_location("pageframe", os.path.join(CRATE, "tools", "page-frame-oracle", "generate.py"))
pf = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pf)

# A horizontal rule pdfTeX strokes as a wide thin line (`\hline`,
# booktabs' `\toprule`, an `\hrule`); the page-frame reader's own scan
# keeps the vertical strokes and the `re` rectangles.
HRULE = re.compile(rb"1 0 0 1 (-?[\d.]+) (-?[\d.]+) cm\s*\[\]0 d 0 J (-?[\d.]+) w 0 0 m (-?[\d.]+) 0 l S")

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
    log = os.path.join(workdir, name + ".log")
    over = 0
    if os.path.exists(log):
        text = open(log, errors="replace").read()
        over = text.count("Overfull \\hbox") + text.count("Overfull \\vbox")
    return os.path.join(workdir, name + ".pdf"), over


def extras_on_page(content, height):
    """The horizontal stroked rules (`\\hline`, booktabs' `\\toprule`) and the
    image rectangles of one content stream, in bp from a top-left origin.

    The page-frame reader keeps the vertical strokes and the `re`
    rectangles; both of these need the same `q`/`cm` transform state, which
    a regular expression over the bytes gets wrong as soon as a scaled
    `\\includegraphics` nests one `cm` inside another.
    """
    ctm, stack, operands = [1, 0, 0, 1, 0, 0], [], []
    line_width, path = 1.0, []
    rules, images = [], []
    for kind, v in pf.tokens(content):
        if kind != "op":
            operands.append(v)
            continue
        if v == "q":
            stack.append(ctm[:])
        elif v == "Q" and stack:
            ctm = stack.pop()
        elif v == "cm" and len(operands) >= 6:
            ctm = pf.mul(operands[-6:], ctm)
        elif v == "w" and operands:
            line_width = operands[-1]
        elif v == "m" and len(operands) >= 2:
            path = [pf.mul([1, 0, 0, 1, operands[-2], operands[-1]], ctm)[4:]]
        elif v == "l" and len(operands) >= 2:
            path.append(pf.mul([1, 0, 0, 1, operands[-2], operands[-1]], ctm)[4:])
        elif v == "S" and len(path) == 2 and abs(path[0][1] - path[1][1]) < 1e-6:
            # A horizontal stroked rule, centred on y and `line_width` thick.
            (x0, y), (x1, _) = path
            lo, hi = sorted([x0, x1])
            rules.append({"x": round(lo, 4), "top": round(height - (y + line_width / 2), 4),
                          "width": round(hi - lo, 4), "height": round(line_width, 4)})
            path = []
        elif v == "Do":
            # The image XObject fills the unit square of the current
            # transform (pdfTeX writes `<w> 0 0 <h> <x> <y> cm /Im Do`).
            p0 = pf.mul([1, 0, 0, 1, 0, 0], ctm)[4:]
            p1 = pf.mul([1, 0, 0, 1, 1, 1], ctm)[4:]
            x0, x1 = sorted([p0[0], p1[0]])
            y0, y1 = sorted([p0[1], p1[1]])
            images.append({"x": round(x0, 4), "top": round(height - y1, 4),
                           "width": round(x1 - x0, 4), "height": round(y1 - y0, 4)})
        operands = []
    return rules, images


def rules_and_images(pdf):
    """[`extras_on_page`] for every page of `pdf`."""
    data = open(pdf, "rb").read()
    objs = pf.read_objects(data)
    out = []
    for p in pf.pages_in_order(objs):
        mb = pf.media_box(objs, p)
        height = mb[3] - mb[1]
        cm = re.search(rb"/Contents\s*(\d+) 0 R", objs[p])
        out.append(extras_on_page(pf.stream_of(objs[int(cm.group(1))]), height))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--keep", help="copy the pdflatex PDFs here")
    ap.add_argument("names", nargs="*")
    args = ap.parse_args()
    os.makedirs(EXPECTED, exist_ok=True)
    version = subprocess.run(["pdflatex", "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    names = sorted(f[:-4] for f in os.listdir(FIXTURES) if f.endswith(".tex"))
    for name in names:
        if args.names and name not in args.names:
            continue
        tex = os.path.join(FIXTURES, name + ".tex")
        with tempfile.TemporaryDirectory() as tmp:
            pdf, over = run_pdflatex(tex, tmp)
            if args.keep:
                os.makedirs(args.keep, exist_ok=True)
                shutil.copy(pdf, args.keep)
            pages = pf.extract(pdf)
            extra = rules_and_images(pdf)
        lines = [
            f"# {version}",
            f"# source fixtures/float-body/{name}.tex (tools/float-body-oracle/generate.py)",
            f"# pdflatex overfull boxes: {over}",
        ]
        for p, (hrules, ims) in zip(pages, extra):
            lines.append(f"page {p['number']} {p['width']:.4f} {p['height']:.4f}")
            for w in p["words"]:
                lines.append(f"word {p['number']} {w['x']:.4f} {w['baseline']:.4f} {w['font']} {w['size']:.4f} {w['text']}")
            for r in list(p["rules"]) + hrules:
                lines.append(f"rule {p['number']} {r['x']:.4f} {r['top']:.4f} {r['width']:.4f} {r['height']:.4f}")
            for im in ims:
                lines.append(f"image {p['number']} {im['x']:.4f} {im['top']:.4f} {im['width']:.4f} {im['height']:.4f}")
        with open(os.path.join(EXPECTED, name + ".txt"), "w") as fh:
            fh.write("\n".join(lines) + "\n")
        print(name, len(pages), "pages", sum(len(p["words"]) for p in pages), "words",
              sum(len(p["rules"]) for p in pages) + sum(len(r) for r, _ in extra), "rules", over, "overfull")


if __name__ == "__main__":
    main()
