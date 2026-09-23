#!/usr/bin/env python3
"""GH-69 graphics-sizing oracle (TEST ONLY; never in the product path).

For every `NN-*.tex` fixture, runs MacTeX `pdflatex` on a copy whose
`\\includegraphics` is wrapped to record the box it sets (`\\wd`, `\\ht`,
`\\dp`) and, with `\\pdfsavepos` (a zero-size whatsit), where it lands.
Writes `reference/NN-*.json`: the page count and every image's page and
x/top/width/height in bp (top-left origin), in source order.
`tests/float_graphics_oracle.rs` compares `flashtex-render` against these.
"""
import json, os, re, shutil, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PDFLATEX = shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex"
PAGE_H_BP = 792.0
BP = 72.0 / 72.27

PROBES = r"""
\makeatletter
\newwrite\ft@pos
\immediate\openout\ft@pos=\jobname.pos
\let\ft@ig\includegraphics
\renewcommand{\includegraphics}[2][]{\leavevmode\setbox\z@\hbox{\ft@ig[#1]{#2}}%
  \edef\ft@tmp{img \the\wd\z@\space\the\ht\z@\space\the\dp\z@}%
  \pdfsavepos\expandafter\write\expandafter\ft@pos\expandafter{\ft@tmp\space\the\ReadonlyShipoutCounter\space\the\pdflastxpos\space\the\pdflastypos}%
  \box\z@}
\makeatother
"""


def run(name):
    tex = open(os.path.join(HERE, name + ".tex")).read()
    with tempfile.TemporaryDirectory() as tmp:
        os.symlink(os.path.join(HERE, "images"), os.path.join(tmp, "images"))
        open(os.path.join(tmp, "doc.tex"), "w").write(tex.replace("\\begin{document}", PROBES + "\\begin{document}", 1))
        r = subprocess.run([PDFLATEX, "-interaction=batchmode", "-halt-on-error", "doc.tex"], cwd=tmp, capture_output=True)
        if r.returncode != 0:
            sys.exit(f"{name}: pdflatex failed\n" + open(os.path.join(tmp, "doc.log")).read()[-3000:])
        pos = open(os.path.join(tmp, "doc.pos")).read().split("\n")
        log = open(os.path.join(tmp, "doc.log")).read()
        pages = int(re.search(r"Output written on doc.pdf \((\d+) page", log).group(1))
    images = []
    for line in pos:
        f = line.split()
        if not f:
            continue
        w, h, d = (float(v[:-2]) * BP for v in f[1:4])
        x, y = int(f[5]) / 65536 * BP, PAGE_H_BP - int(f[6]) / 65536 * BP
        images.append({"page": int(f[4]), "x": round(x, 3), "top": round(y - h, 3), "width": round(w, 3), "height": round(h + d, 3)})
    out = {"pages": pages, "images": images}
    with open(os.path.join(HERE, "reference", name + ".json"), "w") as fh:
        json.dump(out, fh, indent=1)
        fh.write("\n")
    print(name, pages, [(i["page"], i["width"], i["height"]) for i in images])


if __name__ == "__main__":
    for n in sorted(f[:-4] for f in os.listdir(HERE) if re.match(r"\d\d-.*\.tex$", f)):
        run(n)
