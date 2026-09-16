#!/usr/bin/env python3
"""Generates the GH-69 graphics-sizing fixtures (sources + image assets).

Deterministic sources. The images are a 200x100 px PNG without pHYs (so
72 dpi: 200x100 bp) and two one-page PDFs written by pdfTeX with its default
PDF 1.5 object streams (`\\pdfobjcompresslevel` 2), so the catalog and page
tree are inside compressed `/ObjStm` streams: 150x60 bp, and the same page
with `/Rotate 90`. pdfTeX is only needed to regenerate the PDFs; cargo never
runs it.
"""
import os, random, shutil, struct, subprocess, tempfile, zlib

HERE = os.path.dirname(os.path.abspath(__file__))
IMG = os.path.join(HERE, "images")
PDFTEX = shutil.which("pdftex") or "/Library/TeX/texbin/pdftex"


def png(path, w, h, rgb):
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + bytes(rgb) * w for _ in range(h))
    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    open(path, "wb").write(data + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def objstm_pdf(path, attr):
    src = ("\\pdfinfoomitdate=1 \\pdftrailerid{} \\pdfsuppressptexinfo=-1\n"
           "\\pdfpagewidth=150bp \\pdfpageheight=60bp \\hoffset=-1in \\voffset=-1in\n"
           + attr + "\\shipout\\hbox{\\vrule width 150bp height 60bp}\\end\n")
    with tempfile.TemporaryDirectory() as tmp:
        open(os.path.join(tmp, "p.tex"), "w").write(src)
        subprocess.run([PDFTEX, "-interaction=batchmode", "p.tex"], cwd=tmp, check=True, capture_output=True)
        data = open(os.path.join(tmp, "p.pdf"), "rb").read()
    assert b"/ObjStm" in data, "pdfTeX wrote no object stream"
    open(path, "wb").write(data)


random.seed(69)
WORDS = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu".split()


def para(n):
    return " ".join(random.choice(WORDS) for _ in range(n)).capitalize() + ".\n\n"


PRE = r"""\documentclass{article}
\usepackage[T1]{fontenc}
\usepackage{lmodern}
\usepackage[margin=1in]{geometry}
\usepackage{graphicx}
\setlength{\parindent}{0pt}
\pagestyle{empty}
\begin{document}

"""


def g(keys, file):
    return "\\begin{figure}[h]\n\\centering\n\\includegraphics[%s]{images/%s}\n\\end{figure}\n\n" % (keys, file)


if __name__ == "__main__":
    os.makedirs(IMG, exist_ok=True)
    png(os.path.join(IMG, "wide-72.png"), 200, 100, (200, 30, 30))
    objstm_pdf(os.path.join(IMG, "objstm.pdf"), "")
    objstm_pdf(os.path.join(IMG, "objstm-rotate.pdf"), "\\pdfpageattr{/Rotate 90}\n")
    docs = {
        "01-png-keys": PRE + para(30) + g("width=\\linewidth", "wide-72.png") + g("scale=0.5", "wide-72.png")
        + g("height=1in,width=3in,keepaspectratio", "wide-72.png") + para(20) + g("angle=90", "wide-72.png")
        + g("angle=90,width=1in", "wide-72.png") + g("height=1in,angle=90", "wide-72.png")
        + g("width=0.5\\textwidth,height=1in", "wide-72.png") + para(20) + "\\end{document}\n",
        "02-objstm-pdf": PRE + para(30) + g("width=\\linewidth", "objstm.pdf") + g("scale=0.5", "objstm.pdf")
        + g("width=3in,height=0.5in,keepaspectratio", "objstm.pdf") + para(20) + g("angle=90", "objstm.pdf")
        + g("", "objstm-rotate.pdf") + g("width=1in", "objstm-rotate") + para(20) + "\\end{document}\n",
    }
    for name, text in docs.items():
        open(os.path.join(HERE, name + ".tex"), "w").write(text)
