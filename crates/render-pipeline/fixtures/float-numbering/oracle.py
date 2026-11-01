#!/usr/bin/env python3
"""Float caption numbering oracle (TEST ONLY; never in the product path).

For every `NN-*.tex` fixture, and every `NN-*/` directory (a multi-file
project whose entry is `main.tex`, the rest `\\include`d/`\\input`), runs MacTeX `pdflatex` twice on a copy whose
preamble redefines `\\@makecaption` to record, with `\\pdfsavepos` (a
zero-size whatsit that changes no layout), the page and position of every
caption box and the expansion of `\\the<captype>`. `\\newlabel` values come
from the `.aux` file. Writes `reference/NN-*.json`:

  pages:    page count
  captions: label ("Figure 1.1"), page, x and baseline in bp (top-left
            origin) of the caption box start
  refs:     the fixture's "Refs ..." line with every `\\ref` resolved

`tests/float_numbering_oracle.rs` compares `flashtex-render` against these
committed files, so cargo never needs TeX.
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
\def\ft@mark#1{\pdfsavepos\write\ft@pos{#1 \the\ReadonlyShipoutCounter\space\the\pdflastxpos\space\the\pdflastypos}}
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
\makeatother
"""


def run(name):
    project = os.path.join(HERE, name)
    multi = os.path.isdir(project)
    tex = open(os.path.join(project, "main.tex") if multi else project + ".tex").read()
    probed = tex.replace("\\begin{document}", PROBES + "\\begin{document}", 1)
    with tempfile.TemporaryDirectory() as tmp:
        os.symlink(os.path.join(HERE, "images"), os.path.join(tmp, "images"))
        if multi:
            # A fresh directory: no `.aux` of an `\\includeonly`-excluded
            # file, so its counters are never restored (as a first run).
            for f in os.listdir(project):
                if f != "main.tex":
                    shutil.copy(os.path.join(project, f), tmp)
        open(os.path.join(tmp, "doc.tex"), "w").write(probed)
        for _ in range(2):
            r = subprocess.run([PDFLATEX, "-interaction=batchmode", "-halt-on-error", "doc.tex"], cwd=tmp, capture_output=True)
            if r.returncode != 0:
                sys.exit(f"{name}: pdflatex failed\n" + open(os.path.join(tmp, "doc.log")).read()[-3000:])
        pos = open(os.path.join(tmp, "doc.pos")).read().split("\n")
        aux = "".join(open(os.path.join(tmp, f)).read() for f in sorted(os.listdir(tmp)) if f.endswith(".aux"))
        log = open(os.path.join(tmp, "doc.log")).read()
        pages = int(re.search(r"Output written on doc.pdf \((\d+) page", log).group(1))
    captions = []
    for line in pos:
        f = line.split()
        if not f:
            continue
        captype, number, page, x, y = f[1], f[2], int(f[3]), int(f[4]), int(f[5])
        captions.append({
            "label": captype.capitalize() + " " + number,
            "page": page,
            "x": round(x / 65536 * BP, 3),
            "baseline": round(PAGE_H_BP - y / 65536 * BP, 3),
        })
    values = dict(re.findall(r"\\newlabel\{([^}]*)\}\{\{([^}]*)\}", aux))
    refs_src = next(l for l in tex.splitlines() if l.startswith("Refs "))
    refs = re.sub(r"\\ref\{([^}]*)\}", lambda m: values[m.group(1)], refs_src)
    out = {"pages": pages, "captions": captions, "refs": refs}
    with open(os.path.join(HERE, "reference", name + ".json"), "w") as fh:
        json.dump(out, fh, indent=1)
        fh.write("\n")
    print(name, pages, [c["label"] for c in captions], refs)


if __name__ == "__main__":
    for n in sorted(f[:-4] for f in os.listdir(HERE) if re.match(r"\d\d-.*\.tex$", f)):
        run(n)
    for n in sorted(f for f in os.listdir(HERE) if re.match(r"\d\d-", f) and os.path.isdir(os.path.join(HERE, f))):
        run(n)
