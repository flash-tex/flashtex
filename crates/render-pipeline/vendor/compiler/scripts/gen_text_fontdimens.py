#!/usr/bin/env python3
r"""Generate src/text_fontdimens.rs: TeX's `em`/`ex` for every text font the
compiler's `TextStyle` can select.

Oracle tooling only; pdflatex never runs in the product path or in cargo.

    python3 crates/compiler/scripts/gen_text_fontdimens.py [--texbin /Library/TeX/texbin] [--check]

TeX's `em` is the current font's `\fontdimen6` (quad) and `ex` its
`\fontdimen5` (x-height), both scaled to the loaded size (TeXbook ch. 10).
They are not the point size: Computer Modern's optical designs make cmr12's
quad 11.74988pt and cmr9's 9.24994pt. Which TFM a style loads depends on the
`.fd` files (substitutions, `<10.95>cmr10`, `<14.4>cmr12`, ...), so rather
than re-implementing NFSS this asks pdflatex itself: for each font setup
(OT1/T1 encoding, Computer Modern/`lmodern`), family (rm/sf/tt), series
(m/bx), shape (n/it/sl/sc) and each standard LaTeX size it selects the font and
records `\fontname\font`, `\number\fontdimen6\font` and
`\number\fontdimen5\font` (scaled points, exactly as TeX holds them).

The math-layout crate's `cm_tfm.rs` embeds per-character TFM data for the
math designs only (cmr/cmmi/cmsy/cmex); it has no bold, sans, typewriter,
italic, EC or Latin Modern text fonts and the compiler does not depend on it,
so these two parameters per font are recorded here instead.
"""
import argparse
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "src", "text_fontdimens.rs")

SIZES = ["5", "6", "7", "8", "9", "10", "10.95", "12", "14.4", "17.28", "20.74", "24.88"]
# (latin_modern, t1): the package lines of each setup.
SETUPS = [
    (False, False, ""),
    (False, True, r"\usepackage[T1]{fontenc}"),
    (True, False, r"\usepackage{lmodern}"),
    (True, True, r"\usepackage{lmodern}\usepackage[T1]{fontenc}"),
]
FAMILIES = [("Roman", r"\rmdefault"), ("Sans", r"\sfdefault"), ("Mono", r"\ttdefault")]
# `\bfseries` selects `\bfseries@rm`/`@sf`/`@tt`, all `bx`; `\bfdefault` is `b`
# (cmb10) in current LaTeX, so the series and shapes are spelled literally.
SERIES = [(False, "m"), (True, "bx")]
SHAPES = [("upright", "n"), ("italic", "it"), ("slanted", "sl"), ("small_caps", "sc")]


def measure(texbin, packages):
    lines = [r"\documentclass{article}", packages, r"\begin{document}"]
    for _, family in FAMILIES:
        for _, series in SERIES:
            for _, shape in SHAPES:
                for size in SIZES:
                    lines.append(
                        r"\fontsize{%s}{%s}\fontfamily{%s}\fontseries{%s}\fontshape{%s}\selectfont"
                        r"\typeout{FD \fontname\font|\number\fontdimen6\font|\number\fontdimen5\font}"
                        % (size, size, family, series, shape)
                    )
    lines.append(r"\end{document}")
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "fd.tex")
        with open(path, "w") as f:
            f.write("\n".join(lines) + "\n")
        log = subprocess.run(
            [os.path.join(texbin, "pdflatex"), "-interaction=nonstopmode", "-draftmode", "fd.tex"],
            cwd=tmp, capture_output=True, text=True, check=False,
        ).stdout
    rows = [line[3:].split("|") for line in log.splitlines() if line.startswith("FD ")]
    expected = len(FAMILIES) * len(SERIES) * len(SHAPES) * len(SIZES)
    if len(rows) != expected:
        raise SystemExit(f"expected {expected} measurements for {packages!r}, got {len(rows)}")
    return [(name, int(quad), int(xheight)) for name, quad, xheight in rows]


def write_or_check(path, text, check):
    """Write `text` to `path`, or with `--check` compare instead.

    `--check` exit codes (the same for every generator; see
    scripts/check-generated.py): 0 the committed file is what this generator
    produces now, 1 it is stale (a diff excerpt is printed), 2 it cannot be
    checked here (a tool or input is missing; the message says which).
    """
    data = text.encode("utf-8")
    rel = os.path.relpath(path)
    if not check:
        with open(path, "wb") as fh:
            fh.write(data)
        return 0
    try:
        with open(path, "rb") as fh:
            old = fh.read()
    except FileNotFoundError:
        print("STALE: %s does not exist" % rel)
        return 1
    if old == data:
        print("up to date: %s" % rel)
        return 0
    import difflib
    diff = list(difflib.unified_diff(old.decode("utf-8", "replace").splitlines(),
                                     text.splitlines(), "committed", "regenerated",
                                     lineterm="", n=0))
    for line in diff[:40]:
        print(line)
    if len(diff) > 40:
        print("... %d more diff lines" % (len(diff) - 40))
    print("STALE: %s differs from what %s generates now" % (rel, os.path.relpath(sys.argv[0])))
    return 1


def require_tools(*tools):
    """Exit 2 (cannot check here) when a TeX tool is not on PATH."""
    import shutil
    missing = [t for t in tools if not shutil.which(t)]
    if missing:
        print("CANNOT CHECK: %s not found on PATH (needs TeX Live 2026)" % ", ".join(missing))
        sys.exit(2)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--texbin", default="/Library/TeX/texbin")
    ap.add_argument("--check", action="store_true",
                    help="compare with src/text_fontdimens.rs instead of writing it")
    args = ap.parse_args()
    if args.check:
        require_tools(os.path.join(args.texbin, "pdflatex"))
    version = subprocess.run(
        [os.path.join(args.texbin, "pdflatex"), "--version"], capture_output=True, text=True
    ).stdout.splitlines()[0]
    out = [
        "//! Generated by `crates/compiler/scripts/gen_text_fontdimens.py`; do not edit.",
        f"//! Measured with {version}.",
        "//!",
        "//! `\\fontdimen6` (quad, TeX's `em`) and `\\fontdimen5` (x-height, `ex`) in",
        "//! scaled points for each text font NFSS loads, indexed by [`row`].",
        "",
        "/// The standard LaTeX sizes (`\\@vpt`..`\\@xxvpt`), the last index of a row.",
        "pub(crate) const SIZES_PT: [f64; %d] = [%s];" % (len(SIZES), ", ".join(
            s if "." in s else s + ".0" for s in SIZES)),
        "",
        "/// The row of `FONTDIMENS` for a font setup and face.",
        "pub(crate) const fn row(latin_modern: bool, t1: bool, family: usize, bold: bool, shape: usize) -> usize {",
        "    ((((latin_modern as usize * 2 + t1 as usize) * 3 + family) * 2 + bold as usize) * 4) + shape",
        "}",
        "",
        "/// `(quad_sp, x_height_sp)` per [`SIZES_PT`] entry. Families are",
        "/// `[rm, sf, tt]`.",
        "pub(crate) static FONTDIMENS: [[(i32, i32); %d]; %d] = [" % (
            len(SIZES), len(SETUPS) * len(FAMILIES) * len(SERIES) * len(SHAPES)),
    ]
    for latin_modern, t1, packages in SETUPS:
        rows = iter(measure(args.texbin, packages))
        for family, _ in FAMILIES:
            for bold, _ in SERIES:
                for shape, _ in SHAPES:
                    cells = [next(rows) for _ in SIZES]
                    names = sorted({re.sub(r" at .*", "", name) for name, _, _ in cells})
                    out.append(
                        "    // lm=%s t1=%s %s bold=%s shape=%s: %s" % (
                            latin_modern, t1, family, bold, shape, ", ".join(names)))
                    out.append("    [%s]," % ", ".join(f"({q}, {x})" for _, q, x in cells))
    out.append("];")
    rc = write_or_check(OUT, "\n".join(out) + "\n", args.check)
    if not args.check:
        print(f"wrote {os.path.relpath(OUT)}")
    return rc


if __name__ == "__main__":
    sys.exit(main())
