#!/usr/bin/env python3
"""Generates the FT-063 float/graphics fixtures (sources + image assets).

Deterministic: rerunning rewrites byte-identical sources. Images are tiny
solid fills with explicit resolution metadata so the natural size is
exercised: PNG with and without pHYs, a JPEG with JFIF density (made once with
macOS `sips`; committed), and hand-written uncompressed PDFs with a MediaBox
and a CropBox. No TeX is needed to generate them.
"""
import os, struct, zlib, subprocess, textwrap

HERE = os.path.dirname(os.path.abspath(__file__))
IMG = os.path.join(HERE, "images")
os.makedirs(IMG, exist_ok=True)


def png(path, w, h, rgb, dpi=None):
    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + bytes(rgb) * w for _ in range(h))
    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    if dpi:
        ppm = round(dpi / 0.0254)
        data += chunk(b"pHYs", struct.pack(">IIB", ppm, ppm, 1))
    data += chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    open(path, "wb").write(data)


def pdf(path, media, crop=None):
    content = b"0.2 0.4 0.8 rg %g %g %g %g re f" % (media[0], media[1], media[2] - media[0], media[3] - media[1])
    page = b"<< /Type /Page /Parent 2 0 R /MediaBox [%g %g %g %g]" % tuple(media)
    if crop:
        page += b" /CropBox [%g %g %g %g]" % tuple(crop)
    page += b" /Contents 4 0 R /Resources << >> >>"
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        page,
        b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream",
    ]
    out = b"%PDF-1.4\n"
    offs = []
    for i, o in enumerate(objs, 1):
        offs.append(len(out))
        out += b"%d 0 obj\n" % i + o + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    out += b"".join(b"%010d 00000 n \n" % o for o in offs)
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
    open(path, "wb").write(out)


png(os.path.join(IMG, "red-72.png"), 144, 72, (200, 40, 40))  # no pHYs: 144x72 bp
png(os.path.join(IMG, "green-144dpi.png"), 300, 200, (40, 160, 60), 144)  # 150x100 bp
png(os.path.join(IMG, "tall-72.png"), 120, 520, (90, 90, 90))  # 120x520 bp: a float page
jpg = os.path.join(IMG, "blue-96dpi.jpg")
if not os.path.exists(jpg):
    tmp = os.path.join(IMG, "_blue.png")
    png(tmp, 192, 96, (40, 60, 200))
    subprocess.run(
        ["sips", "-s", "format", "jpeg", "-s", "dpiWidth", "96", "-s", "dpiHeight", "96", tmp, "--out", jpg],
        check=True,
        capture_output=True,
    )
    os.remove(tmp)
pdf(os.path.join(IMG, "box-media.pdf"), (0, 0, 200, 100))
pdf(os.path.join(IMG, "box-crop.pdf"), (0, 0, 300, 300), (50, 100, 250, 220))

WORDS = (
    "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike "
    "november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu"
).split()


def para(seed, n):
    out, k = [], seed
    for i in range(n):
        k = (k * 1103515245 + 12345) % 2**31
        w = WORDS[(k >> 8) % len(WORDS)]
        out.append(w.capitalize() if i == 0 else w)
    return textwrap.fill(" ".join(out) + ".", 72)


PREAMBLE = r"""\documentclass[12pt]{article}
\usepackage[T1]{fontenc}
\usepackage{lmodern}
\usepackage[margin=1in]{geometry}
\usepackage{graphicx}
\setlength{\parindent}{0pt}
\pagestyle{empty}
"""


def doc(parts, extra=""):
    return PREAMBLE + extra + "\\begin{document}\n\n" + "\n\n".join(parts) + "\n\n\\end{document}\n"


def fig(env, place, body):
    p = f"[{place}]" if place is not None else ""
    return f"\\begin{{{env}}}{p}\n" + body + f"\n\\end{{{env}}}"


F = {}
F["01-here"] = doc([
    para(1, 60),
    fig("figure", "h", "\\centering\n\\includegraphics{images/red-72.png}\n\\caption{A red box placed here.}"),
    para(2, 60),
])
F["02-top"] = doc([
    para(3, 90),
    fig("figure", "t", "\\centering\n\\includegraphics{images/green-144dpi.png}\n\\caption{Top of the page.}"),
    para(4, 90),
])
F["03-bottom"] = doc([
    para(5, 70),
    fig("figure", "b", "\\centering\n\\includegraphics[width=2in]{images/red-72.png}\n\\caption{Bottom of the page.}"),
    para(6, 70),
])
F["04-default-tbp"] = doc([
    para(7, 120),
    fig("figure", None, "\\centering\n\\includegraphics[height=1.5in]{images/blue-96dpi.jpg}\n\\caption{Default placement.}"),
    para(8, 120),
])
F["05-here-overflow"] = doc([
    para(9, 520),
    fig("figure", "h", "\\centering\n\\includegraphics[scale=1.5]{images/green-144dpi.png}\n\\caption{Did not fit here.}"),
    para(10, 160),
])
F["06-float-page"] = doc([
    para(11, 150),
    fig("figure", "tbp", "\\centering\n\\includegraphics{images/tall-72.png}\n\\caption{A float page.}"),
    para(12, 700),
])
F["07-top-number"] = doc([
    para(13, 40),
    fig("figure", "t", "\\centering\n\\includegraphics[width=1in]{images/red-72.png}\n\\caption{First top.}"),
    para(14, 40),
    fig("figure", "t", "\\centering\n\\includegraphics[width=1in]{images/red-72.png}\n\\caption{Second top.}"),
    para(15, 40),
    fig("figure", "t", "\\centering\n\\includegraphics[width=1in]{images/red-72.png}\n\\caption{Third top waits.}"),
    para(16, 500),
])
F["08-table-ref"] = doc([
    para(17, 50) + "\nSee Table~\\ref{tab:box} and Figure~\\ref{fig:pdf}.",
    fig("table", "htbp", "\\centering\n\\caption{A caption above.}\\label{tab:box}\n\\includegraphics[width=3cm,height=1cm]{images/blue-96dpi.jpg}"),
    para(18, 50),
    fig("figure", "htbp", "\\centering\n\\includegraphics[scale=0.5]{images/box-media.pdf}\n\\caption{A PDF at half size.}\\label{fig:pdf}"),
    para(19, 50),
])
F["09-crop-keepaspect"] = doc([
    para(20, 80),
    fig("figure", "h", "\\centering\n\\includegraphics[width=4in,height=1in,keepaspectratio]{images/box-crop.pdf}\n\\caption{CropBox with keepaspectratio.}"),
    para(21, 80),
    fig("figure", "b", "\\includegraphics[angle=90,height=1in]{images/red-72.png}\n\\caption{Rotated, not centred.}"),
    para(22, 80),
])
F["10-mixed"] = doc([
    para(23, 200),
    fig("figure", "tb", "\\centering\n\\includegraphics[width=0.4\\textwidth]{images/green-144dpi.png}\n\\caption{" + para(24, 30).replace("\n", " ") + "}"),
    para(25, 200),
    fig("table", "h", "\\centering\n\\includegraphics[width=0.3\\linewidth]{images/box-media}\n\\caption{Extension found by search.}"),
    para(26, 300),
    fig("figure", "p", "\\centering\n\\includegraphics{images/red-72.png}\n\\caption{Only on a float page.}"),
    para(27, 200),
])
# float.sty `[H]` (`\@float@HH`, `\float@endH`): the box is set in the text
# with `\vskip\intextsep` on both sides, never deferred and never counted.
F["11-float-h"] = doc([
    para(28, 60),
    fig("figure", "H", "\\centering\n\\includegraphics{images/red-72.png}\n\\caption{Exactly here.}"),
    para(29, 60),
    fig("table", "H", "\\centering\n\\caption{A table caption above.}\n\\includegraphics[width=3cm,height=1cm]{images/blue-96dpi.jpg}"),
    para(30, 120),
    fig("figure", "t", "\\centering\n\\includegraphics{images/green-144dpi.png}\n\\caption{A top float after them.}"),
    para(31, 150),
    fig("figure", "H", "\\centering\n\\includegraphics[height=3in]{images/tall-72.png}\n\\caption{Too tall for what is left.}"),
    para(32, 60),
], "\\usepackage{float}\n")
F["12-h-passes-deferred"] = doc([
    para(33, 150),
    fig("figure", "t", "\\centering\n\\includegraphics{images/tall-72.png}\n\\caption{Deferred to the end.}"),
    fig("figure", "H", "\\centering\n\\includegraphics{images/red-72.png}\n\\caption{Placed before the deferred one.}"),
    para(34, 80),
    fig("figure", "H", "\\centering\n\\includegraphics[width=2in]{images/red-72.png}\n\\caption{Another.}"),
    para(35, 200),
], "\\usepackage{float}\n")

for name, text in F.items():
    open(os.path.join(HERE, name + ".tex"), "w").write(text)
print("wrote", len(F), "fixtures")
