#!/usr/bin/env python3
"""`\\includegraphics` geometry oracle (TEST ONLY; never in the product path).

Sibling of `fixtures/floats/oracle.py`, which measures where a *float* lands.
This one measures the image box itself — in running text (01-08) and, for the
`\\includegraphics*` and `[llx,lly][urx,ury]` forms the float scanner used to
miss entirely, inside a `figure` too (09-10). For every `NN-*.tex` fixture it
runs `pdflatex` on a copy
whose preamble gains position probes (`\\pdfsavepos` whatsits, which reserve
no space and so cannot change the layout) and writes
`reference/NN-*.json`:

  images:     page, x/top/width/height/baseline in bp (top-left page origin),
              in source order
  paragraphs: page, x and baseline of the first line of every body paragraph,
              with the paragraph's first source byte

`baseline` is the box's reference point, which is the baseline of the line the
image sits on; `depth` is how far the box reaches below it (a rotated box has
real depth), so `height - depth` is the part above. The paragraph probes after
an image pin down the interline spacing a tall inline image forces
(`\\lineskip` instead of `\\baselineskip`).

The `\\includegraphics` wrapper here is a full graphicx-shaped scanner
(`*`, one or two optional arguments, then the file), unlike the floats
oracle's two-argument `\\renewcommand`, because these fixtures exercise
`\\includegraphics*` and the `[llx,lly][urx,ury]` bounding-box form.

`tests/graphics_inline_oracle.rs` compares `flashtex-render`'s display list
against the committed files, so cargo never needs TeX.

Regenerating: this writes `reference/*.json` for THIS directory only. It never
touches `fixtures/floats/reference/`, which holds MacTeX-generated data.
"""
import json, os, re, shutil, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PDFLATEX = shutil.which("pdflatex") or "/usr/bin/pdflatex"
PAGE_H_BP = 792.0
BP = 72.0 / 72.27

PROBES = r"""
\makeatletter
\newwrite\ft@pos
\immediate\openout\ft@pos=\jobname.pos
\def\ft@mark#1{\pdfsavepos\write\ft@pos{#1 \thepage\space\the\pdflastxpos\space\the\pdflastypos}}
% A graphicx-shaped scanner: \includegraphics*[..][..]{file}. The real
% command is run inside a box so its dimensions can be reported, then the
% box is contributed exactly where it would have been.
\let\ft@ig\includegraphics
\def\includegraphics{\leavevmode\begingroup\ft@ig@star}
\def\ft@ig@star{\@ifstar{\def\ft@st{*}\ft@ig@opt}{\def\ft@st{}\ft@ig@opt}}
\def\ft@ig@opt{\@ifnextchar[{\ft@ig@opta}{\def\ft@oa{}\def\ft@ob{}\ft@ig@file}}
\def\ft@ig@opta[#1]{\def\ft@oa{[#1]}\@ifnextchar[{\ft@ig@optb}{\def\ft@ob{}\ft@ig@file}}
\def\ft@ig@optb[#1]{\def\ft@ob{[#1]}\ft@ig@file}
\def\ft@ig@file#1{%
  \edef\ft@args{\unexpanded\expandafter{\ft@st}\unexpanded\expandafter{\ft@oa}\unexpanded\expandafter{\ft@ob}}%
  \setbox\z@\hbox{\expandafter\ft@ig\ft@args{#1}}%
  \edef\ft@tmp{img \the\wd\z@\space\the\ht\z@\space\the\dp\z@}%
  \expandafter\ft@mark\expandafter{\ft@tmp}\box\z@\endgroup}
\newcount\ft@pc
\AtBeginDocument{\everypar{\global\advance\ft@pc\@ne\edef\ft@tmp{par \the\ft@pc}\expandafter\ft@mark\expandafter{\ft@tmp}}}
\makeatother
"""


def body_paragraph_starts(tex):
    """First source byte of every body paragraph outside a float environment."""
    begin = tex.index("\\begin{document}") + len("\\begin{document}")
    end = tex.index("\\end{document}")
    body = tex[begin:end]
    body = re.sub(r"\\begin\{(figure|table)\}.*?\\end\{\1\}", lambda m: " " * len(m.group(0)), body, flags=re.S)
    starts = []
    for m in re.finditer(r"(?:^|\n[ \t]*\n)[ \t\n]*(\S)", body):
        starts.append(begin + m.start(1))
    return starts


def run(name):
    src = os.path.join(HERE, name + ".tex")
    tex = open(src).read()
    probed = tex.replace("\\begin{document}", PROBES + "\\begin{document}", 1)
    with tempfile.TemporaryDirectory() as tmp:
        for sub in ("images", "figs"):
            here = os.path.join(HERE, sub)
            if os.path.isdir(here):
                shutil.copytree(here, os.path.join(tmp, sub))
        open(os.path.join(tmp, "doc.tex"), "w").write(probed)
        for _ in range(2):
            r = subprocess.run([PDFLATEX, "-interaction=batchmode", "-halt-on-error", "doc.tex"], cwd=tmp, capture_output=True)
            if r.returncode != 0:
                sys.exit(f"{name}: pdflatex failed\n" + open(os.path.join(tmp, "doc.log")).read()[-3000:])
        pos = open(os.path.join(tmp, "doc.pos")).read().split("\n")
        log = open(os.path.join(tmp, "doc.log")).read()
        pages = int(re.search(r"Output written on doc.pdf \((\d+) page", log).group(1))
    images, paras = [], []
    for line in pos:
        f = line.split()
        if not f:
            continue
        x_bp = int(f[-2]) / 65536 * BP
        y_bp = PAGE_H_BP - int(f[-1]) / 65536 * BP
        page = int(f[-3])
        if f[0] == "img":
            w, h, d = (float(v[:-2]) * BP for v in f[1:4])
            images.append({
                "page": page,
                "x": round(x_bp, 3),
                "top": round(y_bp - h, 3),
                "width": round(w, 3),
                "height": round(h + d, 3),
                "depth": round(d, 3),
                "baseline": round(y_bp, 3),
            })
        elif f[0] == "par":
            paras.append({"page": page, "x": round(x_bp, 3), "baseline": round(y_bp, 3)})
    starts = body_paragraph_starts(tex)
    if len(starts) != len(paras):
        sys.exit(f"{name}: {len(paras)} paragraph probes vs {len(starts)} body paragraphs")
    for p, s in zip(paras, starts):
        p["source_byte"] = s
    version = subprocess.run([PDFLATEX, "--version"], capture_output=True, text=True).stdout.split("\n")[0]
    out = {
        "fixture": name + ".tex",
        "reference_engine": "TeX Live 2025",
        "engine": version,
        "pages": pages,
        "images": images,
        "paragraphs": paras,
    }
    os.makedirs(os.path.join(HERE, "reference"), exist_ok=True)
    json.dump(out, open(os.path.join(HERE, "reference", name + ".json"), "w"), indent=1)
    return out


if __name__ == "__main__":
    names = sorted(n[:-4] for n in os.listdir(HERE) if re.match(r"\d\d-.*\.tex$", n))
    for n in names:
        o = run(n)
        print(n, "pages", o["pages"], "images", [(i["page"], i["x"], i["top"], i["width"], i["height"]) for i in o["images"]])
