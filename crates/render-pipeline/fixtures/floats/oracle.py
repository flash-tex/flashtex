#!/usr/bin/env python3
"""FT-063 float-placement oracle (TEST ONLY; never in the product path).

For every `NN-*.tex` fixture, runs MacTeX `pdflatex` on a copy whose preamble
gains position probes (`\\pdfsavepos`, zero-size whatsits that do not change
layout) and writes `reference/NN-*.json`:

  images:     page, x/top/width/height in bp (top-left origin), in source order
  captions:   page, x and baseline in bp of the caption box start, "Figure 1"...
  paragraphs: page, x and baseline of the first line of every body paragraph
              outside floats, with the paragraph's first source byte

`tests/floats_oracle.rs` compares `flashtex-render`'s display list against
these committed files (1 pt tolerance), so cargo never needs TeX.
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
\def\ft@mark#1{\pdfsavepos\write\ft@pos{#1 \thepage\space\the\pdflastxpos\space\the\pdflastypos}}
\let\ft@ig\includegraphics
\renewcommand{\includegraphics}[2][]{\leavevmode\setbox\z@\hbox{\ft@ig[#1]{#2}}%
  \edef\ft@tmp{img \the\wd\z@\space\the\ht\z@\space\the\dp\z@}%
  \expandafter\ft@mark\expandafter{\ft@tmp}\box\z@}
\long\def\@makecaption#1#2{%
  \vskip\abovecaptionskip
  \sbox\@tempboxa{#1: #2}%
  \edef\ft@tmp{cap \@captype\space\csname the\@captype\endcsname}%
  \ifdim \wd\@tempboxa >\hsize
    \leavevmode\expandafter\ft@mark\expandafter{\ft@tmp}#1: #2\par
  \else
    \global \@minipagefalse
    \hb@xt@\hsize{\hfil\expandafter\ft@mark\expandafter{\ft@tmp}\box\@tempboxa\hfil}%
  \fi
  \vskip\belowcaptionskip}
\newcount\ft@pc
\AtBeginDocument{\everypar{\global\advance\ft@pc\@ne\edef\ft@tmp{par \the\ft@pc}\expandafter\ft@mark\expandafter{\ft@tmp}}}
\makeatother
"""


def body_paragraph_starts(tex):
    """First source byte of every body paragraph outside float environments."""
    begin = tex.index("\\begin{document}") + len("\\begin{document}")
    end = tex.index("\\end{document}")
    body = tex[begin:end]
    masked = re.sub(r"\\begin\{(figure|table)\}.*?\\end\{\1\}", lambda m: " " * len(m.group(0)), body, flags=re.S)
    starts = []
    for m in re.finditer(r"(?:^|\n[ \t]*\n)[ \t\n]*(\S)", masked):
        starts.append(begin + m.start(1))
    return starts


def run(name):
    src = os.path.join(HERE, name + ".tex")
    tex = open(src).read()
    probed = tex.replace("\\begin{document}", PROBES + "\\begin{document}", 1)
    with tempfile.TemporaryDirectory() as tmp:
        os.symlink(os.path.join(HERE, "images"), os.path.join(tmp, "images"))
        open(os.path.join(tmp, "doc.tex"), "w").write(probed)
        for _ in range(2):
            r = subprocess.run([PDFLATEX, "-interaction=batchmode", "-halt-on-error", "doc.tex"], cwd=tmp, capture_output=True)
            if r.returncode != 0:
                sys.exit(f"{name}: pdflatex failed\n" + open(os.path.join(tmp, "doc.log")).read()[-3000:])
        pos = open(os.path.join(tmp, "doc.pos")).read().split("\n")
        log = open(os.path.join(tmp, "doc.log")).read()
        pages = int(re.search(r"Output written on doc.pdf \((\d+) page", log).group(1))
    images, captions, paras = [], [], []
    for line in pos:
        f = line.split()
        if not f:
            continue
        x_bp = int(f[-2]) / 65536 * BP
        y_bp = PAGE_H_BP - int(f[-1]) / 65536 * BP
        page = int(f[-3])
        if f[0] == "img":
            w, h, d = (float(v[:-2]) * BP for v in f[1:4])
            images.append({"page": page, "x": round(x_bp, 3), "top": round(y_bp - h, 3), "width": round(w, 3), "height": round(h + d, 3)})
        elif f[0] == "cap":
            captions.append({"page": page, "label": f"{f[1].capitalize()} {f[2]}", "x": round(x_bp, 3), "baseline": round(y_bp, 3)})
        elif f[0] == "par":
            paras.append({"page": page, "x": round(x_bp, 3), "baseline": round(y_bp, 3)})
    starts = body_paragraph_starts(tex)
    if len(starts) != len(paras):
        sys.exit(f"{name}: {len(paras)} paragraph probes vs {len(starts)} body paragraphs")
    for p, s in zip(paras, starts):
        p["source_byte"] = s
    out = {
        "fixture": name + ".tex",
        "engine": subprocess.run([PDFLATEX, "--version"], capture_output=True, text=True).stdout.split("\n")[0],
        "pages": pages,
        "images": images,
        "captions": captions,
        "paragraphs": paras,
    }
    os.makedirs(os.path.join(HERE, "reference"), exist_ok=True)
    json.dump(out, open(os.path.join(HERE, "reference", name + ".json"), "w"), indent=1)
    return out


if __name__ == "__main__":
    # `oracle.py 11-float-h` re-pins only the named fixtures.
    names = sys.argv[1:] or sorted(n[:-4] for n in os.listdir(HERE) if re.match(r"\d\d-.*\.tex$", n))
    for n in names:
        o = run(n)
        print(n, "pages", o["pages"], "images", [(i["page"], i["top"]) for i in o["images"]], "captions", [(c["page"], c["baseline"]) for c in o["captions"]])
