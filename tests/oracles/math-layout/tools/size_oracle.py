#!/usr/bin/env python3
r"""Reference glyph and rule positions for math set at LaTeX text sizes.

ORACLE TOOLING ONLY. This script writes LaTeX documents into a scratch
directory, runs the reference pdflatex there (MacTeX 2026), and records where
pdfTeX placed every glyph and rule of each formula. The result,
`tests/size_oracle/expected.txt`, is committed; `tests/size_oracle.rs` lays the
same formulas out with math-layout (no TeX involved) and compares.

Each fixture is one page holding one inline formula in a text size selected
by a size command, in a document of one class size with or without amsmath
(which changes how family 3, cmex, is sized: `cm::ExtensionSizing::Designs`).
The fixture names are the keys of the builders in `tests/size_oracle.rs`.

Output format (bp, PDF coordinates, y up), one block per fixture:
  fixture <name> <class pt> <packages|-> <text pt> <size command|->
  g <font> <code> <size bp> <x> <y>
  r <x> <centre y> <length> <thickness>

Usage: python3 tools/size_oracle.py [scratch-dir] > tests/size_oracle/expected.txt
"""
import os
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from oracle_compare import parse_content  # noqa: E402

# (name, class pt, packages, size command, text size pt, formula)
FIXTURES = [
    # Upright one-letter units with a power (siunitx `\kelvin^{-1}`).
    ("k-inv-10", 10, "", "", 10, r"\mathrm{K}^{-1}"),
    ("k-inv-11a-footnote", 11, "amsmath", r"\footnotesize", 9, r"\mathrm{K}^{-1}"),
    ("k-inv-12a", 12, "amsmath", "", 12, r"\mathrm{K}^{-1}"),
    ("k-inv-11a-Large", 11, "amsmath", r"\Large", 14.4, r"\mathrm{K}^{-1}"),
    ("m2-s-11a", 11, "amsmath", "", 10.95, r"\mathrm{m}^2\,\mathrm{s}^{-1}"),
    ("d-x-10", 10, "", "", 10, r"\mathrm{d}x"),
    # Primes: ' is ^\prime, set from cmsy at script size.
    ("prime-f-10", 10, "", "", 10, r"f'(x)"),
    ("prime-f2-11a", 11, "amsmath", "", 10.95, r"f''(x)"),
    ("prime-footnote-10", 10, "", r"\footnotesize", 8, r"x'"),
    ("prime-small-12a", 12, "amsmath", r"\small", 10.95, r"g'"),
    # \epsilon (cmmi "0F) and \varepsilon (cmmi "22).
    ("epsilon-10", 10, "", "", 10, r"\epsilon+\varepsilon"),
    ("epsilon-11a-footnote", 11, "amsmath", r"\footnotesize", 9, r"\epsilon\varepsilon"),
    # amsmath \bmod, \pmod; the kernel's \pmod.
    ("bmod-10a", 10, "amsmath", "", 10, r"a\bmod b"),
    ("bmod-11a-footnote", 11, "amsmath", r"\footnotesize", 9, r"a\bmod b"),
    ("pmod-11a", 11, "amsmath", "", 10.95, r"a\equiv b\pmod{m}"),
    ("pmod-12", 12, "", "", 12, r"a\pmod{m}"),
    # \dots family.
    ("dotsc-10a", 10, "amsmath", "", 10, r"a,\dots,b"),
    ("dotsb-11a", 11, "amsmath", "", 10.95, r"a+\dots+b"),
    ("cdots-11a-footnote", 11, "amsmath", r"\footnotesize", 9, r"x_1\cdots x_n"),
    ("ldots-10", 10, "", "", 10, r"(1,\ldots,n)"),
    ("dots-paren-12a", 12, "amsmath", "", 12, r"(x_1,\dots)"),
    # Operator names with scripts.
    ("sin2-10", 10, "", "", 10, r"\sin^2 x"),
    ("sin2-11a-Large", 11, "amsmath", r"\Large", 14.4, r"\sin^2 x"),
    ("lim-11a-footnote", 11, "amsmath", r"\footnotesize", 9, r"\lim_{n\to\infty}a_n"),
    # Family 3 at the math sizes (fixed cmex10 vs amsmath's designs).
    ("frac-11a", 11, "amsmath", "", 10.95, r"\frac{1}{2}"),
    ("frac-11a-footnote", 11, "amsmath", r"\footnotesize", 9, r"\frac{a}{b}"),
    ("frac-10-footnote", 10, "", r"\footnotesize", 8, r"\frac{a}{b}"),
    ("sum-11a", 11, "amsmath", "", 10.95, r"\sum_{i=1}^n x_i"),
    ("sum-10a-small", 10, "amsmath", r"\small", 9, r"\sum_i x_i"),
    ("sqrt-12a-footnote", 12, "amsmath", r"\footnotesize", 10, r"\sqrt{x+1}"),
    ("frac-12a-huge", 12, "amsmath", r"\huge", 24.88, r"\frac{1}{x}"),
    # Digits and letters with scripts at heading and note sizes.
    ("digits-11a-footnote", 11, "amsmath", r"\footnotesize", 9, r"10^{-3}"),
    ("section-10", 10, "", r"\Large", 14.4, r"x^2+\alpha_1"),
    ("subsection-11a", 11, "amsmath", r"\large", 12, r"x^2_i"),
    ("LARGE-10", 10, "", r"\LARGE", 17.28, r"2^{2^n}"),
    ("small-10", 10, "", r"\small", 9, r"y_{ij}^2"),
    ("scriptsize-11a", 11, "amsmath", r"\scriptsize", 8, r"e^{i\pi}"),
]


def pdf_objects(data):
    return {int(m.group(1)): m.group(2)
            for m in re.finditer(rb"(\d+) 0 obj\s*(.*?)\s*endobj", data, re.S)}


def deref(objs, body, key):
    m = re.search(rb"/" + key + rb"\s+(\d+) 0 R", body)
    return objs[int(m.group(1))] if m else None


def font_table(objs, page):
    res = deref(objs, page, b"Resources") or page
    fonts = re.search(rb"/Font\s*<<(.*?)>>", res, re.S)
    if fonts is None:
        ref = deref(objs, res, b"Font")
        fonts_body = ref
    else:
        fonts_body = fonts.group(1)
    table = {}
    for name, num in re.findall(rb"/(F\d+) (\d+) 0 R", fonts_body):
        f = objs[int(num)]
        base = re.search(rb"/BaseFont /(?:[A-Z]{6}\+)?(\w+)", f).group(1).decode()
        first = int(re.search(rb"/FirstChar (\d+)", f).group(1))
        wref = re.search(rb"/Widths (\d+) 0 R", f)
        wsrc = objs[int(wref.group(1))] if wref else re.search(rb"/Widths (\[.*?\])", f, re.S).group(1)
        table[name.decode()] = (base.lower(), first, [float(x) for x in re.findall(rb"[-\d.]+", wsrc)])
    return table


def pages(pdf_path):
    data = open(pdf_path, "rb").read()
    objs = pdf_objects(data)
    out = []
    for num, body in objs.items():
        if not re.search(rb"/Type /Page\b", body):
            continue
        contents = int(re.search(rb"/Contents (\d+) 0 R", body).group(1))
        stream = re.search(rb"stream\r?\n(.*?)\r?\nendstream", objs[contents], re.S).group(1)
        glyphs, rules = parse_content(stream, font_table(objs, body))
        out.append((contents, glyphs, rules))
    out.sort(key=lambda p: p[0])
    return [(g, r) for _, g, r in out]


def run_config(scratch, class_pt, packages, fixtures):
    name = f"size-oracle-{class_pt}pt-{packages or 'kernel'}".replace(",", "-")
    tex = [r"\pdfcompresslevel=0 \pdfobjcompresslevel=0",
           rf"\documentclass[{class_pt}pt]{{article}}"]
    if packages:
        tex.append(rf"\usepackage{{{packages}}}")
    tex.append(r"\pagestyle{empty}")
    tex.append(r"\begin{document}")
    for f in fixtures:
        tex.append(rf"\noindent{{{f[3]}${f[5]}$}}\newpage")
    tex.append(r"\end{document}")
    path = os.path.join(scratch, name + ".tex")
    open(path, "w").write("\n".join(tex) + "\n")
    subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", name + ".tex"],
                   cwd=scratch, check=True, stdout=subprocess.DEVNULL,
                   env=dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1"))
    got = pages(os.path.join(scratch, name + ".pdf"))
    assert len(got) == len(fixtures), (name, len(got), len(fixtures))
    return got


def main():
    scratch = sys.argv[1] if len(sys.argv) > 1 else tempfile.mkdtemp(prefix="size-oracle-")
    os.makedirs(scratch, exist_ok=True)
    version = subprocess.run(["pdflatex", "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    configs = []
    for f in FIXTURES:
        key = (f[1], f[2])
        if key not in configs:
            configs.append(key)
    results = {}
    for class_pt, packages in configs:
        group = [f for f in FIXTURES if (f[1], f[2]) == (class_pt, packages)]
        for f, page in zip(group, run_config(scratch, class_pt, packages, group)):
            results[f[0]] = page
    print(f"# generated by tools/size_oracle.py with {version}")
    for f in FIXTURES:
        glyphs, rules = results[f[0]]
        print(f"fixture {f[0]} {f[1]} {f[2] or '-'} {f[4]} {f[3] or '-'}")
        print(f"# {f[5]}")
        for g in glyphs:
            print(f"g {g['font']} {g['code']} {g['size']:.4f} {g['x']:.3f} {g['y']:.3f}")
        for r in rules:
            print(f"r {r['x']:.3f} {r['y_center']:.3f} {r['w']:.3f} {r['h']:.3f}")


if __name__ == "__main__":
    main()
