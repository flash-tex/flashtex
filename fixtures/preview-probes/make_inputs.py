#!/usr/bin/env python3
"""Writes the image and PDF inputs of fixtures/preview-probes (deterministic:
plain zlib PNGs and hand-written PDFs, no imaging library). It writes beside
itself. The JPEGs are not made here: sips (macOS) made them once from
rgb8.png (README.md), and they are kept as made.

    python3 -I fixtures/preview-probes/make_inputs.py

These are inputs only. Every expected pixel comes from the oracle: CI's
preview-parity sweep renders pdflatex's own PDF of each fixture.
"""
import os
import struct
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
IMG = os.path.join(HERE, "pv-images")
PDF = os.path.join(HERE, "pv-pdf-inclusion")


def chunk(kind, data):
    c = kind + data
    return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)


def png(path, w, h, depth, ctype, rows, extra=()):
    """rows: one bytes object per row, packed (the filter byte is added here)."""
    raw = b"".join(b"\x00" + r for r in rows)
    out = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, depth, ctype, 0, 0, 0))
    for kind, data in extra:
        out += chunk(kind, data)
    out += chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(out)


def images():
    os.makedirs(IMG, exist_ok=True)
    # 8x8 palette, 6 colours in 2x2 blocks: magnified, every sample shows.
    pal = bytes([255, 0, 0, 0, 160, 0, 0, 0, 255, 255, 200, 0, 40, 40, 40, 200, 120, 255])
    rows = [bytes((x // 2 + y // 2 * 3) % 6 for x in range(8)) for y in range(8)]
    png(os.path.join(IMG, "pal8.png"), 8, 8, 8, 3, rows, [(b"PLTE", pal)])
    # The same with gAMA 1.0: pdfTeX ignores it (no \pdfimageapplygamma) and
    # writes the palette as /Indexed /DeviceRGB.
    png(os.path.join(IMG, "pal8-gamma.png"), 8, 8, 8, 3, rows,
        [(b"gAMA", struct.pack(">I", 100000)), (b"PLTE", pal)])
    # The same with tRNS: index 1 transparent, index 4 half (pdfTeX: RGB + SMask).
    png(os.path.join(IMG, "pal8-trns.png"), 8, 8, 8, 3, rows,
        [(b"PLTE", pal), (b"tRNS", bytes([255, 0, 255, 255, 128]))])
    # 4-bit palette, 16x4 (copied as /BitsPerComponent 4).
    pal4 = bytes(sum(([v, 255 - v, (v * 3) % 256] for v in range(0, 256, 16)), []))
    rows4 = []
    for y in range(4):
        vals = [(x + y * 4) % 16 for x in range(16)]
        rows4.append(bytes((vals[i] << 4) | vals[i + 1] for i in range(0, 16, 2)))
    png(os.path.join(IMG, "pal4.png"), 16, 4, 4, 3, rows4, [(b"PLTE", pal4)])
    # 1-bit grey checkerboard (line-art scans): copied as /BitsPerComponent 1.
    rows1 = []
    for y in range(16):
        bits = [((x // 2 + y // 2) & 1) for x in range(16)]
        rows1.append(bytes(int("".join(map(str, bits[i:i + 8])), 2) for i in range(0, 16, 8)))
    png(os.path.join(IMG, "gray1.png"), 16, 16, 1, 0, rows1)
    # 8-bit grey, and grey with alpha (an SMask).
    png(os.path.join(IMG, "gray8.png"), 16, 16, 8, 0,
        [bytes((x * 16 + y * 3) % 256 for x in range(16)) for y in range(16)])
    png(os.path.join(IMG, "gray-alpha.png"), 16, 16, 8, 4,
        [bytes(sum(([x * 16, 255 - y * 16] for x in range(16)), [])) for y in range(16)])
    # 8-bit RGB (copied), and with gAMA 1/2.2 (decoded, samples unchanged).
    rgb = [bytes(sum(([x * 16, y * 16, 255 - x * 8 - y * 8] for x in range(16)), [])) for y in range(16)]
    png(os.path.join(IMG, "rgb8.png"), 16, 16, 8, 2, rgb)
    png(os.path.join(IMG, "rgb8-gamma.png"), 16, 16, 8, 2, rgb, [(b"gAMA", struct.pack(">I", 45455))])
    # 16-bit RGB (copied, /BitsPerComponent 16), low bytes not zero.
    png(os.path.join(IMG, "rgb16.png"), 16, 16, 16, 2,
        [b"".join(struct.pack(">HHH", x * 4096 + 0xFF, y * 4096 + 0x80, 0xFFFF - x * 2048 - y * 2048 - 0x7F)
                  for x in range(16)) for y in range(16)])
    # 16-bit RGBA: pdfTeX writes the colour at 16 bpc and the SMask at 8
    # (the alpha's high byte); the alpha's low bytes are not zero.
    png(os.path.join(IMG, "rgba16.png"), 16, 16, 16, 6,
        [b"".join(struct.pack(">HHHH", x * 4096 + 0x7F, 0x8000 + y * 0x700, 0x3000,
                              min(0xFFFF, 0x1000 + x * 3000 + y * 900 + 0xC3)) for x in range(16)) for y in range(16)])
    # 8-bit RGBA (an SMask; pdfTeX also gives the page a transparency /Group).
    png(os.path.join(IMG, "rgba8.png"), 16, 16, 8, 6,
        [bytes(sum(([255 - x * 16, x * 16, 128, 32 + y * 14] for x in range(16)), [])) for y in range(16)])


def pdf(path, pages):
    """pages: (MediaBox, extra page entries, content)."""
    objs = [b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Count %d /Kids [%s] >>"
            % (len(pages), b" ".join(b"%d 0 R" % (3 + 2 * i) for i in range(len(pages))))]
    for i, (box, extra, content) in enumerate(pages):
        objs.append(b"<< /Type /Page /Parent 2 0 R /MediaBox [%s] %s /Resources << >> /Contents %d 0 R >>"
                    % (b" ".join(str(v).encode() for v in box), extra, 4 + 2 * i))
        objs.append(b"<< /Length %d >>\nstream\n%s\nendstream" % (len(content), content))
    out = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n"
    offs = []
    for n, o in enumerate(objs, 1):
        offs.append(len(out))
        out += b"%d 0 obj\n%s\nendobj\n" % (n, o)
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    for o in offs:
        out += b"%010d 00000 n \n" % o
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
    with open(path, "wb") as f:
        f.write(out)


def pdfs():
    os.makedirs(PDF, exist_ok=True)
    # An asymmetric picture (which way is up shows), with its own colours.
    art = (b"0.9 0.2 0.1 rg 10 10 60 20 re f 0.1 0.3 0.8 rg 10 40 20 30 re f "
           b"0 0.6 0.2 RG 3 w 40 45 m 70 70 l S")
    # The same with no colour or line operator: drawn as a Form XObject it
    # paints with the state it is drawn in (the form inherits it).
    plain = b"10 10 60 20 re f 10 40 20 30 re f 40 45 m 70 70 l S"
    pdf(os.path.join(PDF, "pages.pdf"), [
        ([0, 0, 80, 80], b"", art),
        ([0, 0, 80, 80], b"/Rotate 90", art),
        ([0, 0, 80, 80], b"/Rotate 180", art),
        ([0, 0, 80, 80], b"/Rotate 270", art),
        ([-20, -10, 80, 80], b"/CropBox [0 0 75 60] /Rotate 90", art),
        ([0, 0, 80, 80], b"/UserUnit 2", art),
        ([0, 0, 80, 80], b"", plain),
    ])


if __name__ == "__main__":
    images()
    pdfs()
