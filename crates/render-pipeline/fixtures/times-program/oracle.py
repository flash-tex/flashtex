#!/usr/bin/env python3
"""Times-program oracle (TEST ONLY; pdflatex never runs in the product path).

Runs MacTeX `pdflatex` on `main.tex` (a `\\usepackage{times}` page: Times
roman, bold, italic, bold italic, Helvetica and Courier) in a temporary
directory, replays the PDF's content stream with the oracle reader
`tools/visual-oracle/pdftext.py`, and writes `reference.json`:

  pdflatex: the engine banner
  fonts:    every BaseFont on the page (subset tag dropped)
  glyphs:   page, character, font, size (bp), x and baseline (bp, y from
            the page top) of every glyph

`tests/times_program.rs` compares the pipeline's display list with this
committed file, so cargo never needs TeX.
"""
import json, os, re, shutil, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "../../../.."))
sys.path.insert(0, os.path.join(ROOT, "tools/visual-oracle"))
import pdftext  # noqa: E402

PDFLATEX = shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex"


def main():
    with tempfile.TemporaryDirectory() as work:
        shutil.copy(os.path.join(HERE, "main.tex"), work)
        env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
        subprocess.run([PDFLATEX, "-interaction=nonstopmode", "-halt-on-error", "main.tex"],
                       cwd=work, env=env, check=True, capture_output=True)
        log = open(os.path.join(work, "main.log"), encoding="latin-1").read()
        banner = log.splitlines()[0].split("  ")[0]
        doc = pdftext.PdfDocument.load(os.path.join(work, "main.pdf"))
        fonts, glyphs = set(), []
        for pi, page in enumerate(doc.pages()):
            gl, notes = pdftext.page_glyphs(doc, page)
            assert not notes, notes
            for g in gl:
                font = re.sub(r"^[A-Z]{6}\+", "", g["font"] or "")
                fonts.add(font)
                glyphs.append({"page": pi + 1, "text": g["text"], "font": font, "size": round(g["size"], 4),
                               "x": round(g["x"], 4), "baseline": round(g["y_top"], 4)})
    head = json.dumps({"pdflatex": banner, "source": "main.tex", "fonts": sorted(fonts)}, indent=1)[:-2]
    rows = ",\n".join("  " + json.dumps(g) for g in glyphs)
    with open(os.path.join(HERE, "reference.json"), "w") as f:
        f.write(f'{head},\n "glyphs": [\n{rows}\n ]\n}}\n')
    json.load(open(os.path.join(HERE, "reference.json")))
    print(f"{len(glyphs)} glyphs, fonts {sorted(fonts)}")


if __name__ == "__main__":
    main()
