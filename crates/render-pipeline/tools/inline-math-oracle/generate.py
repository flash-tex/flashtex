#!/usr/bin/env python3
"""Inline-math line-break oracle: word positions of pdflatex's PDF for
fixtures/real-world/inline-math/main.tex (paragraphs of dense inline math
that TeX breaks inside formulas after Bin/Rel atoms).

pdflatex (MacTeX) is the ORACLE ONLY: never in the product path, never in
cargo tests. The reference PDF is `reference.pdf` next to the
source (regenerate it with

    SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 pdflatex -interaction=nonstopmode \
        '\\pdfcompresslevel=0\\pdfobjcompresslevel=0\\input{main.tex}'   # twice

in that directory). This script replays its content streams with
tools/visual-oracle/pdftext.py and writes every word (a run of glyphs with
no gap wider than 0.16 em, one text object, one baseline) to
crates/render-pipeline/fixtures/inline-math/expected.txt:

  page <n> <width> <height>
  word <page> <x> <baseline> <size> <text>

x and baseline in bp, baseline measured from the top of the page (the
display-list-v2 convention); `tests/inline_math_breaks.rs` compares the
line breaks (the words on each baseline) and the word origins.
"""
import importlib.util
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(os.path.dirname(HERE))
REPO = os.path.dirname(os.path.dirname(CRATE))
FIXTURE = os.path.join(REPO, "fixtures", "real-world", "inline-math")
EXPECTED = os.path.join(CRATE, "fixtures", "inline-math", "expected.txt")

_spec = importlib.util.spec_from_file_location("pdftext", os.path.join(REPO, "tools", "visual-oracle", "pdftext.py"))
pdftext = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pdftext)


def main():
    pdf = sys.argv[1] if len(sys.argv) > 1 else os.path.join(FIXTURE, "reference.pdf")
    os.makedirs(os.path.dirname(EXPECTED), exist_ok=True)
    lines = [f"# source fixtures/real-world/inline-math/main.tex, reference {os.path.basename(pdf)} (tools/inline-math-oracle/generate.py)"]
    total = 0
    for rec in pdftext.page_words(pdf):
        if rec["notes"]:
            raise SystemExit(f"page {rec['page']}: {rec['notes']}")
        mb = rec["media_box"]
        lines.append(f"page {rec['page']} {mb[2] - mb[0]:.4f} {mb[3] - mb[1]:.4f}")
        for w in rec["words"]:
            lines.append(f"word {rec['page']} {w['x']:.4f} {w['y_top']:.4f} {w['size']:.4f} {w['text']}")
            total += 1
    with open(EXPECTED, "w") as fh:
        fh.write("\n".join(lines) + "\n")
    print(EXPECTED, total, "words")


if __name__ == "__main__":
    main()
