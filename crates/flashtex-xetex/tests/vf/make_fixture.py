#!/usr/bin/env python3
"""The virtual-font fixture of tests/output_vf.rs, and its expected data.

    make_fixture.py [--oracle]

Writes, next to this file:

* `fxvfbase.pfb`: a Type 1 font made for this test (three rectangles, `A`,
  `B` and `C`; nobody else's font), assembled by TeX Live's `t1asm`;
* `fxvfbase.tfm`, `fxvfslant.tfm`: its metrics (the second for a map entry
  that slants it), by `pltotf`;
* `fxvf.vf`, `fxvf.tfm`: a virtual font over them whose packets use every
  command vf.c reads (set and put in every length, rules, moves, w, x, y,
  z, push and pop, font changes, specials), written byte by byte here
  because `vptovf` writes only some of them;
* `fxvfn.vf`, `fxvfn.tfm`: a virtual font whose first font is `fxvf` (a
  virtual font in a virtual font, at another size);
* `pdftex.map`: the two map entries (no entry for the virtual fonts);
* `vf.tex`: the INITEX input.

With `--oracle` (TeX Live 2026 on the path, and tools/xetex-pdfparity's
Python needs: qpdf and fontTools) it then runs TeX Live's `xetex -ini` on
`vf.tex` in a scratch directory holding only these files, so that
xdvipdfmx draws the page with them, and writes what its PDF shows to
`expected.json`: every glyph (font, code, size, origin), every rule and
the link's rectangle, in bp. That file is the test's expected data;
nothing in it is written by hand. It also keeps the page as TeX Live's
`xetex -no-pdf` writes it, `vf.xdv`, which tools/xetex-pdfparity/test_vf.py
reads.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.normpath(os.path.join(HERE, "..", "..", "..", ".."))

# ---------------------------------------------------------------------------
# The Type 1 font: rectangles, with the advance widths of the TFM file.
# ---------------------------------------------------------------------------
GLYPHS = {  # name: (advance, left, bottom, right, top) in 1/1000 em
    "A": (500, 50, 0, 450, 700),
    "B": (700, 50, -200, 650, 500),
    "C": (300, 50, 0, 250, 400),
}


def t1_source():
    chars = ["/.notdef {\n\t0 500 hsbw\n\tendchar\n\t} ND"]
    for name, (adv, l, b, r, t) in GLYPHS.items():
        chars.append(
            "/%s {\n\t%d %d hsbw\n\t%d vmoveto\n\t%d hlineto\n\t%d vlineto\n\t%d hlineto\n"
            "\tclosepath\n\tendchar\n\t} ND" % (name, l, adv, b, r - l, t - b, l - r))
    enc = "".join("dup %d /%s put\n" % (ord(n), n) for n in GLYPHS)
    return """%%!PS-AdobeFont-1.0: FxVfBase 001.000
11 dict begin
/FontInfo 7 dict dup begin
/version (001.000) readonly def
/Notice (A FlashTeX test fixture. Public domain.) readonly def
/FullName (FxVfBase) readonly def
/FamilyName (FxVfBase) readonly def
/Weight (Regular) readonly def
/ItalicAngle 0 def
/isFixedPitch false def
end readonly def
/FontName /FxVfBase def
/PaintType 0 def
/FontType 1 def
/FontMatrix [0.001 0 0 0.001 0 0] readonly def
/Encoding 256 array
0 1 255 {1 index exch /.notdef put} for
%sreadonly def
/FontBBox {50 -200 650 700} readonly def
currentdict end
currentfile eexec
dup /Private 8 dict dup begin
/RD {string currentfile exch readstring pop} executeonly def
/ND {noaccess def} executeonly def
/NP {noaccess put} executeonly def
/BlueValues [-10 0 700 710] def
/StdHW [100] def
/StdVW [100] def
/MinFeature {16 16} def
/password 5839 def
/Subrs 4 array
dup 0 {
\t3 0 callothersubr
\tpop
\tpop
\tsetcurrentpoint
\treturn
\t} NP
dup 1 {
\t0 1 callothersubr
\treturn
\t} NP
dup 2 {
\t0 2 callothersubr
\treturn
\t} NP
dup 3 {
\treturn
\t} NP
ND
2 index /CharStrings %d dict dup begin
%s
end
end
readonly put
noaccess put
dup /FontName get exch definefont pop
mark currentfile closefile
cleartomark
""" % (enc, len(chars), "\n".join(chars))


# ---------------------------------------------------------------------------
# Metrics (property lists for pltotf). Design size 10pt, units of it.
# ---------------------------------------------------------------------------
def pl(chars):
    out = ["(FAMILY FXVF)", "(DESIGNSIZE R 10.0)"]
    for c, (wd, ht, dp) in chars.items():
        out.append("(CHARACTER C %s (CHARWD R %s) (CHARHT R %s) (CHARDP R %s))" % (c, wd, ht, dp))
    return "\n".join(out) + "\n"


BASE = {"A": (0.5, 0.7, 0.0), "B": (0.7, 0.5, 0.2), "C": (0.3, 0.4, 0.0)}
# The virtual fonts' own widths are what TeX advances by; they need not be
# what a packet draws. `h` has no packet in the VF file at all.
FXVF = {c: (w, 0.7, 0.2) for c, w in
        dict(a=0.5, b=0.6, c=1.9, d=1.6, e=2.5, f=1.6, g=0.4, h=0.4, i=1.2).items()}
FXVFN = {"a": (2.0, 0.7, 0.2), "b": (2.0, 0.7, 0.2)}


# ---------------------------------------------------------------------------
# The VF files, byte by byte (the opcodes are DVI's).
# ---------------------------------------------------------------------------
def fw(x):
    """A fix word: a multiple of 2^-20 of the design size."""
    return int(round(x * (1 << 20)))


def num(v, n):
    """`n` bytes, two's complement, big-endian."""
    return (v & ((1 << (8 * n)) - 1)).to_bytes(n, "big")


SET1, SET_RULE, PUT1, PUT_RULE, NOP, PUSH, POP = 128, 132, 133, 137, 138, 141, 142
RIGHT1, W0, W1, X0, X1, DOWN1, Y0, Y1, Z0, Z1 = 143, 147, 148, 152, 153, 157, 161, 162, 166, 167
FNT_NUM_0, FNT1, XXX1, FNT_DEF1, PRE, POST = 171, 235, 239, 243, 247, 248


def move(op1, x, n):
    return bytes([op1 + n - 1]) + num(fw(x) if isinstance(x, float) else x, n)


def rule(op, h, w):
    return bytes([op]) + num(fw(h), 4) + num(fw(w), 4)


def special(s, n=1):
    return bytes([XXX1 + n - 1]) + num(len(s), n) + s.encode()


def vf(fonts, packets, widths):
    """fonts: [(number, bytes of the number, scale, name)]."""
    out = bytes([PRE, 202]) + bytes([len(b"FlashTeX fixture")]) + b"FlashTeX fixture"
    out += num(0, 4) + num(fw(10.0), 4)
    for k, kb, scale, name in fonts:
        out += bytes([FNT_DEF1 + len(kb) - 1]) + kb + num(0, 4) + num(fw(scale), 4) + num(fw(10.0), 4)
        out += bytes([0, len(name)]) + name.encode()
    for c, pkt in packets.items():
        tfm = fw(widths[c][0])
        if len(pkt) < 242 and c != "f":
            out += bytes([len(pkt), ord(c)]) + num(tfm, 3) + pkt
        else:  # the long form, used for `f` so that both are read
            out += bytes([242]) + num(len(pkt), 4) + num(ord(c), 4) + num(tfm, 4) + pkt
    out += bytes([POST])
    while len(out) % 4:
        out += bytes([POST])
    return out


A, B, C = b"A", b"B", b"C"
FXVF_FONTS = [
    (0, b"\x00", 1.0, "fxvfbase"),
    (1, b"\x01", 0.75, "fxvfslant"),
    # `fnt_def1 200`: read as 200, but `fnt1 200` reads -56 (vf.c)
    (200, b"\xc8", 0.5, "fxvfbase"),
    (300, b"\x01\x2c", 1.5, "fxvfbase"),
]
FXVF_PACKETS = {
    # one character of the first font
    "a": A,
    # another font, at another size
    "b": bytes([FNT_NUM_0 + 1]) + B,
    # moves in every register and length, put and set in every length
    "c": (bytes([PUSH]) + move(RIGHT1, 0.1, 3) + move(DOWN1, -0.3, 3) + bytes([PUT1]) + C
          + bytes([POP]) + A
          + move(W1, 0.05, 3) + bytes([SET1]) + B + bytes([W0])
          + move(X1, 0.02, 2) + C + bytes([X0])
          + move(Y1, 0.1, 3) + A + bytes([Y0])
          + move(Z1, -0.25, 4) + C + bytes([Z0])
          + move(RIGHT1, 100, 1) + move(RIGHT1, -0.01, 2) + move(RIGHT1, 0.03, 4)
          + move(DOWN1, 50, 1) + move(DOWN1, 0.01, 2) + move(DOWN1, 0.02, 4)
          + bytes([SET1 + 1, 0]) + A + bytes([PUT1 + 1, 0]) + B
          + bytes([SET1 + 2, 0, 0]) + C + bytes([PUT1 + 2, 0, 0]) + A),
    # rules: set and put, thin (stroked) and thick (filled), and ones with
    # no height or no width, which draw nothing (a set rule still moves)
    "d": (rule(SET_RULE, 0.05, 0.3) + move(DOWN1, 0.2, 3) + rule(PUT_RULE, 0.6, 0.02)
          + move(RIGHT1, 0.1, 3) + rule(PUT_RULE, 0.6, 0.6) + move(DOWN1, -0.2, 3)
          + move(RIGHT1, 0.7, 3) + A + rule(PUT_RULE, -1.0, 0.1) + rule(SET_RULE, 0.0, 0.1) + C
          + rule(SET_RULE, 0.5, -0.1) + B),
    # font numbers: `fnt1 200` (not found, as vf.c reads it), `fnt2 300`,
    # `fnt_num_0`, a number the file does not define, `fnt1 1`
    "e": (bytes([FNT1, 200]) + A + bytes([FNT1 + 1, 1, 0x2c]) + B + bytes([FNT_NUM_0]) + C
          + bytes([FNT1, 5]) + C + bytes([FNT1, 1]) + A + bytes([FNT1 + 3, 0, 0, 1, 0x2c]) + C),
    # specials: the page's, in each length; a warning, which is not one
    "f": (special("color push rgb 1 0 0") + A + special("color pop") + special("  Warning: hello")
          + B + bytes([NOP]) + special("color push gray 0.5", 2) + C + special("color pop", 3)
          + special("color push cmyk 0 1 0 0", 4) + A + special("color pop")),
    # no bytes: "a nonexistent character"
    "g": b"",
    # put, then a move
    "i": bytes([PUT1]) + A + move(RIGHT1, 0.6, 3) + bytes([PUT1]) + B,
}
FXVFN_FONTS = [(0, b"\x00", 0.5, "fxvf"), (1, b"\x01", 1.0, "fxvfbase")]
FXVFN_PACKETS = {
    "a": b"c" + bytes([FNT_NUM_0 + 1]) + A,
    "b": b"b" + b"d" + b"e",
}

TEX = r"""\catcode`\{=1 \catcode`\}=2 \catcode`\#=6
\pdfpagewidth=612bp \pdfpageheight=792bp
\baselineskip=60pt \lineskiplimit=-1000pt
\font\v=fxvf at 12pt
\font\w=fxvf scaled 2000
\font\n=fxvfn
\font\m=fxvfn at 17.3pt
\shipout\vbox{\hbox{\v abc}\hbox{\v def}\hbox{\v ghi\w ab\v a}\hbox{\n ab}\hbox{\m ab}%
\hbox{\v a\special{pdf:bann <</Type/Annot/Subtype/Link/Border[0 0 0]/A<</S/URI/URI(https://example.com/)>>>>}%
\v bc\n a\special{pdf:eann}\v a}}
\end
"""

MAP = "fxvfbase FxVfBase <fxvfbase.pfb\nfxvfslant FxVfBase \" .25 SlantFont \" <fxvfbase.pfb\n"

FILES = ["fxvfbase.pfb", "fxvfbase.tfm", "fxvfslant.tfm", "fxvf.vf", "fxvf.tfm",
         "fxvfn.vf", "fxvfn.tfm", "pdftex.map", "vf.tex"]


def make():
    with tempfile.TemporaryDirectory() as tmp:
        src = os.path.join(tmp, "fxvfbase.txt")
        with open(src, "w") as f:
            f.write(t1_source())
        subprocess.run(["t1asm", "-b", src, os.path.join(HERE, "fxvfbase.pfb")], check=True)
        for name, chars in (("fxvfbase", BASE), ("fxvfslant", BASE), ("fxvf", FXVF), ("fxvfn", FXVFN)):
            p = os.path.join(tmp, name + ".pl")
            with open(p, "w") as f:
                f.write(pl(chars))
            subprocess.run(["pltotf", p, os.path.join(HERE, name + ".tfm")], check=True,
                           stdout=subprocess.DEVNULL)
    with open(os.path.join(HERE, "fxvf.vf"), "wb") as f:
        f.write(vf(FXVF_FONTS, FXVF_PACKETS, FXVF))
    with open(os.path.join(HERE, "fxvfn.vf"), "wb") as f:
        f.write(vf(FXVFN_FONTS, FXVFN_PACKETS, FXVFN))
    with open(os.path.join(HERE, "pdftex.map"), "w") as f:
        f.write(MAP)
    with open(os.path.join(HERE, "vf.tex"), "w") as f:
        f.write(TEX)


def oracle():
    """TeX Live's xetex (xdvipdfmx) on the fixture, and what its PDF shows."""
    sys.path.insert(0, os.path.join(REPO, "tools", "xetex-pdfparity"))
    from compare import as_rule  # noqa: E402
    from model import extract  # noqa: E402

    with tempfile.TemporaryDirectory() as tmp:
        for n in FILES:
            shutil.copy(os.path.join(HERE, n), tmp)
        env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TZ="UTC")
        r = subprocess.run(["xetex", "-ini", "-interaction=nonstopmode", "vf.tex"], cwd=tmp, env=env,
                           capture_output=True, text=True)
        log = open(os.path.join(tmp, "vf.log")).read()
        if "Output written on vf.pdf (1 page" not in log:
            sys.exit("xetex wrote no PDF:\n" + r.stdout + r.stderr + log)
        version = subprocess.run(["xdvipdfmx", "--version"], capture_output=True, text=True).stdout
        if os.environ.get("FXVF_KEEP"):
            shutil.copy(os.path.join(tmp, "vf.pdf"), os.environ["FXVF_KEEP"])
        # the same page as XDV, for tools/xetex-pdfparity/test_vf.py
        subprocess.run(["xetex", "-ini", "-interaction=nonstopmode", "-no-pdf", "vf.tex"], cwd=tmp, env=env,
                       capture_output=True, text=True, check=True)
        shutil.copy(os.path.join(tmp, "vf.xdv"), os.path.join(HERE, "vf.xdv"))
        m = extract(os.path.join(tmp, "vf.pdf"))
        page = m.pages[0]
        glyphs = []
        for g in page.content.glyphs:
            a, b, c, d = g["m"]
            glyphs.append({
                "font": g["font"], "code": g["code"],
                # the text rendering matrix's linear part, per em
                "m": [round(v * 1000.0, 4) for v in (a, b, c, d)],
                "x": round(g["x"], 4), "y": round(g["y"], 4),
                # the fill colour: its space and components
                "fill": [g["fill"][0], [round(float(v), 4) for v in g["fill"][1]]],
            })
        rules = []
        for p in page.content.paths:
            r = as_rule(p, 0.01)
            if r is None:
                sys.exit("a path that is not a rule: %r" % (p,))
            rules.append({"rect": [round(v, 4) for v in r["rect"]], "src": r["src"]})
        links = [{"rect": a["rect"], "action": a.get("action")} for a in page.annots]
        out = {
            "oracle": version.splitlines()[0] if version else "?",
            "media_box": page.box["MediaBox"],
            "glyphs": glyphs, "rules": rules, "links": links,
        }
    with open(os.path.join(HERE, "expected.json"), "w") as f:
        json.dump(out, f, indent=1, sort_keys=True)
        f.write("\n")
    print("%d glyphs, %d rules, %d links" % (len(glyphs), len(rules), len(links)))


if __name__ == "__main__":
    make()
    if "--oracle" in sys.argv[1:]:
        oracle()
