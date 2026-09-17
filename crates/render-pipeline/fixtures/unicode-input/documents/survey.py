#!/usr/bin/env python3
"""Compiles every document here with pdfLaTeX and with `flashtex build`, and
prints one table row per document: errors on both sides, whether the
extracted text matches, and how far glyphs land from pdfLaTeX's (test-only
evidence for GH-UNICODE-INPUT; cargo never runs it).

    python3 survey.py [path/to/flashtex]      # needs PyMuPDF

Text extraction emulates `pdftotext` (not installed on the measuring host):
pdfTeX's OT1 accents are separate spacing-accent glyphs, which poppler
combines with the letter they overlap; the same is done here before
comparing. Positions compare glyph origins in reading order, pdfLaTeX's
separate accent glyphs left out (FlashTeX draws the precomposed letter at
the base letter's origin).
"""
import glob, os, re, subprocess, sys, tempfile, unicodedata
import pymupdf

HERE = os.path.dirname(os.path.abspath(__file__))
PDFLATEX = os.environ.get("PDFLATEX", "/Library/TeX/texbin/pdflatex")
ACCENTS = {"´": "́", "¨": "̈", "˜": "̃", "˚": "̊", "˝": "̋",
           "˘": "̆", "ˆ": "̂", "`": "̀", "ˇ": "̌", "¯": "̄",
           "¸": "̧", "˙": "̇"}


def text_of(pdf):
    t = pymupdf.open(pdf)[0].get_text("text")
    t = re.sub("([%s])([A-Za-zı])" % re.escape("".join(ACCENTS)),
               lambda m: m.group(2).replace("ı", "i") + ACCENTS[m.group(1)], t)
    return unicodedata.normalize("NFC", t).split()


def origins(pdf):
    d = pymupdf.open(pdf)[0].get_text("rawdict")
    chars = [c for b in d["blocks"] for l in b.get("lines", []) for s in l["spans"] for c in s["chars"]]
    return [c["origin"][0] for c in chars if not c["c"].isspace() and c["c"] not in ACCENTS and c["origin"][1] < 700]


def main():
    flashtex = sys.argv[1] if len(sys.argv) > 1 else "flashtex"
    print("| document | pdflatex errors | flashtex errors | text | glyphs | max glyph dx (bp) |")
    print("|---|---|---|---|---|---|")
    for tex in sorted(glob.glob(os.path.join(HERE, "*.tex"))):
        name = os.path.basename(tex)[:-4]
        with tempfile.TemporaryDirectory() as d:
            subprocess.run([PDFLATEX, "-interaction=nonstopmode", "-output-directory", d, tex],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            ref_errors = len(re.findall(r"^! ", open(os.path.join(d, name + ".log"), errors="replace").read(), re.M))
            out = subprocess.run([flashtex, "build", tex, "-o", os.path.join(d, "ft.pdf"), "--diagnostics", "short"],
                                 capture_output=True, text=True)
            ft_errors = len(re.findall(r": error\[", out.stderr))
            ref_pdf, ft_pdf = os.path.join(d, name + ".pdf"), os.path.join(d, "ft.pdf")
            same = text_of(ref_pdf) == text_of(ft_pdf)
            a, b = origins(ref_pdf), origins(ft_pdf)
            dx = max((abs(x - y) for x, y in zip(a, b)), default=0.0) if len(a) == len(b) else float("nan")
            print(f"| {name} | {ref_errors} | {ft_errors} | {'match' if same else 'DIFF'} | {len(a)}/{len(b)} | {dx:.3f} |")


if __name__ == "__main__":
    main()
