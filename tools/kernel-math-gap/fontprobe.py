#!/usr/bin/env python3
"""Can the bundled faces draw the glyphs the remaining kernel commands need?

For each character the gap needs, reports whether the two bundled math faces
carry it, at what advance, and how that advance compares with the Computer
Modern advance pdflatex actually sets (measured by `measure.py`). A command
can only be set at pdflatex's metrics if some face carries its glyph.

    python3 tools/kernel-math-gap/fontprobe.py [--fonts apps/mac/Fonts]
"""
import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from otfmath import OpenType  # noqa: E402

FACES = ["latinmodern-math.otf", "NewCMMath-Regular.otf"]

# (command, char, cm advance in pt at 10pt from measure.py, note)
WANTED = [
    # the seven pieces: what each would have to be painted from
    ("\\braceld",    0x23B0, 4.50005, "cmex \"7A bracehtipdownleft"),
    ("\\bracerd",    0x23B1, 4.50005, "cmex \"7B bracehtipdownright"),
    ("\\bracelu",    None,   4.50005, "cmex \"7C bracehtipupleft -- no code point"),
    ("\\braceru",    None,   4.50005, "cmex \"7D bracehtipupright -- no code point"),
    ("\\lhook",      None,   2.77779, "cmmi \"2C arrowhookleft -- no code point"),
    ("\\rhook",      None,   2.77779, "cmmi \"2D arrowhookright -- no code point"),
    ("\\mapstochar", None,   0.00000, "cmsy \"37 mapsto bar -- no code point"),
    # the nine latexsym symbols, against the nearest Unicode shape
    ("\\mho",        0x2127, 7.22223, "lasy \"30"),
    ("\\Join",       0x22C8, 7.22223, "lasy \"31"),
    ("\\Box",        0x25A1, 7.47224, "lasy \"32"),
    ("\\Diamond",    0x25C7, 7.91673, "lasy \"33"),
    ("\\leadsto",    0x21DD, 10.00002, "lasy \"3B"),
    ("\\lhd",        0x22B2, 7.77780, "lasy \"01"),
    ("\\unlhd",      0x22B4, 7.77780, "lasy \"02"),
    ("\\rhd",        0x22B3, 7.77780, "lasy \"03"),
    ("\\unrhd",      0x22B5, 7.77780, "lasy \"04"),
]

# Extensibles whose MathVariants assembly is the only place an OpenType face
# keeps the shapes TeX exposes as separate pieces.
ASSEMBLIES = [(0x23DE, "TOP CURLY BRACKET, \\overbrace"),
              (0x23DF, "BOTTOM CURLY BRACKET, \\underbrace"),
              (0x21A6, "RIGHTWARDS ARROW FROM BAR, \\mapsto")]


def grid(headers, rows):
    w = [max(len(str(r[i])) for r in [headers] + rows) for i in range(len(headers))]
    def fmt(r):
        return "  " + "  ".join(str(r[i]).ljust(w[i]) for i in range(len(headers)))
    print(fmt(headers))
    print("  " + "  ".join("-" * x for x in w))
    for r in rows:
        print(fmt(r))
    print()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fonts", default="apps/mac/Fonts")
    a = ap.parse_args()

    faces = []
    for n in FACES:
        p = os.path.join(a.fonts, n)
        if not os.path.exists(p):
            sys.exit("missing face: %s" % p)
        faces.append((n, OpenType(p)))

    print("bundled math faces: %s\n" % ", ".join(n for n, _ in faces))

    rows = []
    for cmd, ch, cmpt, note in WANTED:
        cells = []
        for _, f in faces:
            if ch is None:
                cells.append("n/a")
                continue
            g = f.cmap().get(ch)
            if g is None:
                cells.append("absent")
            else:
                pt = f.advance_pt(g)
                cells.append("%.5f (%+.2f)" % (pt, pt - cmpt))
        rows.append((cmd, "U+%04X" % ch if ch else "-none-",
                     "%.5f" % cmpt, cells[0], cells[1], note))
    grid(("command", "char", "cm pt", "LM Math pt (delta)",
          "NewCM Math pt (delta)", "what pdflatex sets"), rows)

    print("== the pieces are not separately addressable in the faces ==\n")
    for ch, what in ASSEMBLIES:
        for n, f in faces:
            g = f.cmap().get(ch)
            if g is None:
                print("  %-22s U+%04X  absent" % (n, ch))
                continue
            _vert, horiz = f.assemblies()
            parts = horiz.get(g)
            print("  %-22s U+%04X %s" % (n, ch, what))
            if not parts:
                print("      no horizontal assembly")
                continue
            for pg, fl, fa in parts:
                print("      part gid %-5d %-10s fullAdvance %4d units = %.5f pt"
                      % (pg, "extender" if fl & 1 else "fixed", fa,
                         fa / f.upm * 10))
        print()


if __name__ == "__main__":
    main()
