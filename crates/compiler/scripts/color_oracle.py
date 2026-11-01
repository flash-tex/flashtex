#!/usr/bin/env python3
"""Colour-operator oracle: what pdfTeX writes for xcolor/color expressions.

Oracle tooling only; pdflatex never runs in the product path or in cargo.

    python3 crates/compiler/scripts/color_oracle.py [--texbin /Library/TeX/texbin]

For every suite (a package line plus preamble definitions) one document sets
`\\textcolor{<expr>}{X}` (or a raw snippet containing exactly one `X`) per
paragraph. The page content streams are read in order and, for every shown
string `X`, the current fill operator (`... rg`, `... k`, `... g`) and stroke
operator (`RG`/`K`/`G`) are recorded exactly as written. The result is pinned
in `crates/compiler/tests/color_oracle/operators.tsv`, which
`tests/color_oracle.rs` replays through `flashtex_compiler::color`.
"""
import argparse, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "tools", "visual-oracle"))
import pdftext  # noqa: E402

OUT = os.path.join(HERE, "..", "tests", "color_oracle", "operators.tsv")

BASE = ["red", "green", "blue", "brown", "lime", "orange", "pink", "purple", "teal", "violet",
        "cyan", "magenta", "yellow", "olive", "black", "darkgray", "gray", "lightgray", "white"]

MIXES = [
    "red!30", "red!30!blue", "red!50!cyan", "cyan!50!red", "black!20", "gray!37.5", "blue!20!black",
    "red!33.3!green", "-red", "-cyan", "-gray", "-red!40", "--red", "red!50!green!30!blue", "red!0", "red!100",
    "orange!25!white", "yellow!10", "teal!70!black", "olive!45!magenta", "brown!12.5", "violet!66.6!lime",
    "magenta!20!yellow", "pink!80!gray", "lightgray!50!cyan", "white!30!black", "purple!7", "green!90!-blue",
    "red!25!blue!50", "cyan!12.34!black", "yellow!99.9", "black!0.5", "blue!1",
]

DEFS = r"""
\definecolor{rgbA}{rgb}{0.1,0.2,0.3}
\definecolor{rgbB}{rgb}{1,.5,.25}
\definecolor{rgbC}{rgb}{0.123456,0.98765,0.5}
\definecolor{RGBa}{RGB}{12,200,33}
\definecolor{RGBb}{RGB}{255,128,0}
\definecolor{HTMLa}{HTML}{FF8800}
\definecolor{HTMLb}{HTML}{1a2B3c}
\definecolor{cmykA}{cmyk}{.1,.2,.3,.4}
\definecolor{cmykB}{cmyk}{0,1,0.5,0}
\definecolor{grayA}{gray}{0.35}
\definecolor{GrayA}{Gray}{7}
\colorlet{letA}{red!40!blue}
\colorlet{letB}{cmykA!50}
\colorlet{letC}{rgbA}
\colorlet{letD}[cmyk]{rgbB}
\colorlet{letE}[gray]{red}
"""

DEF_ITEMS = [
    "rgbA", "rgbB", "rgbC", "RGBa", "RGBb", "HTMLa", "HTMLb", "cmykA", "cmykB", "grayA", "GrayA",
    "letA", "letB", "letC", "letD", "letE", "cmykA!50", "RGBa!30!cmykA", "HTMLa!75", "grayA!40!red",
    "red!40!grayA", "-RGBa", "-cmykA", "-grayA", "rgbC!50", "letA!50!letB",
    ("dot-mix", r"{\color{blue}\textcolor{.!50}{X}}"),
    ("dot-mix-cmyk", r"{\color{cmykA}\textcolor{.!30!red}{X}}"),
    ("dot-plain", r"{\color{RGBa}\textcolor{.}{X}}"),
    ("undeclared-rgb", r"\textcolor[rgb]{0.1,0.2,0.3}{X}"),
    ("undeclared-cmyk", r"\textcolor[cmyk]{0,1,0.5,0}{X}"),
    ("undeclared-gray", r"\textcolor[gray]{0.5}{X}"),
    ("undeclared-RGB", r"\textcolor[RGB]{1,2,3}{X}"),
    ("undeclared-HTML", r"\textcolor[HTML]{ABCDEF}{X}"),
    ("undeclared-rgb-long", r"\textcolor[rgb]{0.333333,0.666666,0.999999}{X}"),
    ("color-decl", r"{\color{red!60}X}"),
    ("color-decl-undeclared", r"{\color[cmyk]{.2,.4,.6,.8}X}"),
]

DVIPS = """GreenYellow Yellow Goldenrod Dandelion Apricot Peach Melon YellowOrange Orange BurntOrange
Bittersweet RedOrange Mahogany Maroon BrickRed Red OrangeRed RubineRed WildStrawberry Salmon CarnationPink
Magenta VioletRed Rhodamine Mulberry RedViolet Fuchsia Lavender Thistle Orchid DarkOrchid Purple Plum Violet
RoyalPurple BlueViolet Periwinkle CadetBlue CornflowerBlue MidnightBlue NavyBlue RoyalBlue Blue Cerulean Cyan
ProcessBlue SkyBlue Turquoise TealBlue Aquamarine BlueGreen Emerald JungleGreen SeaGreen Green ForestGreen
PineGreen LimeGreen YellowGreen SpringGreen OliveGreen RawSienna Sepia Brown Tan Gray Black White""".split()


def def_names(path, kind):
    names = []
    text = open(path, encoding="latin-1").read()
    if kind == "dvips":
        import re
        names = re.findall(r"\\DefineNamedColor\{named\}\{(\w+)\}", text)
    else:
        body = text.split(r"\preparecolorset{rgb}{}{}{%", 1)[1].split("}", 1)[0]
        for entry in body.replace("%", "").split(";"):
            entry = entry.strip()
            if entry:
                names.append(entry.split(",", 1)[0])
    return names


def suites(texbin):
    kpse = lambda f: subprocess.run([os.path.join(texbin, "kpsewhich"), f], capture_output=True, text=True).stdout.strip()
    dvips = def_names(kpse("dvipsnam.def"), "dvips")
    assert dvips == DVIPS, "dvipsnam.def changed"
    svg = def_names(kpse("svgnam.def"), "svg")
    x11 = def_names(kpse("x11nam.def"), "x11")
    return [
        ("xcolor", r"\usepackage{xcolor}", DEFS, BASE + MIXES + DEF_ITEMS),
        ("xcolor-rgb", r"\usepackage[rgb]{xcolor}", DEFS, BASE + MIXES + DEF_ITEMS),
        ("xcolor-cmyk", r"\usepackage[cmyk]{xcolor}", DEFS, BASE + MIXES + DEF_ITEMS),
        ("xcolor-gray", r"\usepackage[gray]{xcolor}", DEFS, BASE + MIXES + DEF_ITEMS),
        ("xcolor-dvipsnames", r"\usepackage[dvipsnames]{xcolor}", "",
         dvips + ["Apricot!50", "RoyalBlue!30!white", "-OliveGreen", "Red!50!blue", "Mahogany!20!SkyBlue"]),
        ("xcolor-svgnames", r"\usepackage[svgnames]{xcolor}", "", svg + ["Crimson!40", "Navy!50!Gold"]),
        ("xcolor-x11names", r"\usepackage[x11names]{xcolor}", "", x11 + ["Firebrick3!60", "SteelBlue2!25!black"]),
        ("color", r"\usepackage{color}",
         "\\definecolor{rgbA}{rgb}{0.1,0.2,0.3}\n\\definecolor{cmykA}{cmyk}{.1,.2,.3,.4}\n"
         "\\definecolor{grayA}{gray}{0.35}\n\\definecolor{RGBa}{RGB}{12,200,33}\n",
         ["red", "green", "blue", "cyan", "magenta", "yellow", "black", "white", "rgbA", "cmykA", "grayA", "RGBa",
          ("undeclared-rgb", r"\textcolor[rgb]{0.1,0.2,0.3}{X}")]),
        # color.sty keeps dvipsnam.def colours in the `named` model only.
        ("color-dvipsnames", r"\usepackage[dvipsnames]{color}", "",
         [("[named]" + n, "\\textcolor[named]{%s}{X}" % n) for n in dvips]),
    ]


def item_tex(item):
    if isinstance(item, tuple):
        return item
    return item, "\\textcolor{%s}{X}" % item


def colours_before_x(pdf):
    doc = pdftext.PdfDocument.load(pdf)
    out = []
    for page in doc.pages():
        contents = page.get("Contents")
        resolved = doc.resolve(contents) if not isinstance(contents, list) else contents
        data = b"\n".join(doc.stream_of(c) for c in resolved) if isinstance(resolved, list) else doc.stream_of(contents)
        lx = pdftext._Lexer(data)
        stack, fill, stroke = [], "0 g", "0 G"
        while True:
            tok = lx.token()
            if tok is None:
                break
            obj = lx.object(tok)
            if not (isinstance(obj, tuple) and obj and obj[0] == "op"):
                stack.append(obj)
                continue
            op = obj[1].decode()
            if op in ("rg", "k", "g"):
                fill = " ".join(num(v) for v in stack) + " " + op
            elif op in ("RG", "K", "G"):
                stroke = " ".join(num(v) for v in stack) + " " + op
            elif op in ("TJ", "Tj"):
                arg = stack[-1] if stack else None
                parts = arg if isinstance(arg, list) else [arg]
                if any(isinstance(p, (bytes, bytearray)) and b"X" in p for p in parts):
                    out.append((fill, stroke))
            stack = []
    return out


def num(v):
    return v.decode() if isinstance(v, (bytes, bytearray)) else str(v)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--texbin", default="/Library/TeX/texbin")
    args = ap.parse_args()
    pdflatex = os.path.join(args.texbin, "pdflatex")
    version = subprocess.run([pdflatex, "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    rows = []
    with tempfile.TemporaryDirectory() as work:
        for sid, package, defs, items in suites(args.texbin):
            pairs = [item_tex(i) for i in items]
            body = "\n".join(tex + "\\par" for _, tex in pairs)
            src = ("\\documentclass{article}\n%s\n%s\\pagestyle{empty}\n\\begin{document}\n%s\n\\end{document}\n"
                   % (package, defs, body))
            with open(os.path.join(work, sid + ".tex"), "w", encoding="utf-8") as f:
                f.write(src)
            subprocess.run([pdflatex, "-interaction=batchmode", sid + ".tex"], cwd=work,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            log = open(os.path.join(work, sid + ".log"), encoding="latin-1").read()
            errors = [l for l in log.splitlines() if l.startswith("!")]
            if errors:
                sys.exit(f"{sid}: pdflatex errors: {errors[:5]}")
            found = colours_before_x(os.path.join(work, sid + ".pdf"))
            if len(found) != len(pairs):
                sys.exit(f"{sid}: {len(found)} X glyphs for {len(pairs)} items")
            for (label, _), (fill, stroke) in zip(pairs, found):
                rows.append((sid, label, fill, stroke))
            print(f"{sid}: {len(pairs)} items")
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(f"# {version}; xcolor.sty 2024/09/29 v3.02; pdftex.def 2025/09/29 v1.2d\n")
        f.write("# generated by crates/compiler/scripts/color_oracle.py (oracle only)\n")
        f.write("# suite\texpression\tfill operator\tstroke operator\n")
        for r in rows:
            f.write("\t".join(r) + "\n")
    print(f"wrote {len(rows)} rows to {os.path.relpath(OUT, REPO)}")


if __name__ == "__main__":
    main()
