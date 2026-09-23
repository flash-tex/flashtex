#!/usr/bin/env python3
"""Verbatim/listings oracle: glyph origins and rules of pdfLaTeX output.

pdflatex is the ORACLE ONLY: it never runs in the product path or in cargo
tests. `refs` typesets every fixture here with MacTeX pdflatex (two passes,
SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1) and writes
`reference/<name>.json`:

    {"fixture", "reference_engine", "invocation", "unit",
     "pages": <count>,
     "glyphs": [[page, x, y, text, font, size], ...],
     "rules":  [[page, x, top, width, height], ...]}

Glyphs are every shown character of the page content streams
(tools/visual-oracle/pdftext.py: origin x and baseline y in bp, y from the
page top), in stream order. Rules are the filled rectangles of the page
(`re` + `f`, and single stroked `m`/`l` segments at the stroke width, which
is how pdfTeX writes `\\hrule`/`\\vrule`), merged where they continue one
another (../../../compiler/tests/tabular_corpus/oracle.py).

    python3 oracle.py refs [--texbin /Library/TeX/texbin] [NAME ...]
    python3 oracle.py show NAME      # reference glyphs grouped by baseline
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "tools", "visual-oracle"))
sys.path.insert(0, os.path.join(REPO, "crates", "compiler", "tests", "tabular_corpus"))
import pdftext  # noqa: E402

REFERENCE = os.path.join(HERE, "reference")


def rules_of(doc, page):
    """Rules (x, top, width, height) in bp, merged (tabular oracle)."""
    import oracle as tabular  # crates/compiler/tests/tabular_corpus/oracle.py
    return tabular.ref_rules(doc, page)


def fixtures(only):
    names = sorted(f[:-4] for f in os.listdir(HERE) if f.endswith(".tex") and f[:2].isdigit())
    return [n for n in names if not only or any(o in n for o in only)]


def reference_of(pdf):
    doc = pdftext.PdfDocument.load(pdf)
    glyphs, rules, pages = [], [], 0
    for number, page in enumerate(doc.pages(), start=1):
        pages = number
        found, _notes = pdftext.page_glyphs(doc, page)
        for g in found:
            glyphs.append([number, round(g["x"], 4), round(g["y_top"], 4), g["text"], g["font"], round(g["size"], 4)])
        for r in rules_of(doc, page):
            rules.append([number] + [round(v, 4) for v in r])
    return pages, glyphs, rules


def cmd_refs(args):
    os.makedirs(REFERENCE, exist_ok=True)
    pdflatex = os.path.join(args.texbin, "pdflatex")
    version = subprocess.run([pdflatex, "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    with tempfile.TemporaryDirectory() as work:
        for name in fixtures(args.only):
            for entry in os.listdir(HERE):
                if entry.endswith((".c", ".py", ".java", ".txt")):
                    with open(os.path.join(HERE, entry), "rb") as src, open(os.path.join(work, entry), "wb") as dst:
                        dst.write(src.read())
            with open(os.path.join(HERE, name + ".tex"), "rb") as src, open(os.path.join(work, name + ".tex"), "wb") as dst:
                dst.write(src.read())
            for _ in range(2):
                run = subprocess.run([pdflatex, "-interaction=batchmode", name + ".tex"], cwd=work, env=env,
                                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            if run.returncode != 0:
                log = open(os.path.join(work, name + ".log"), encoding="latin-1").read()
                sys.exit(f"{name}: pdflatex failed\n{log[-2000:]}")
            pages, glyphs, rules = reference_of(os.path.join(work, name + ".pdf"))
            with open(os.path.join(REFERENCE, name + ".json"), "w", encoding="utf-8") as f:
                json.dump({"fixture": name + ".tex", "reference_engine": version,
                           "invocation": "pdflatex -interaction=batchmode, two passes, SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1",
                           "unit": "bp, y from page top; glyphs [page, x, baseline, text, font, size]; rules [page, x, top, width, height]",
                           "pages": pages, "glyphs": glyphs, "rules": rules}, f, indent=None, ensure_ascii=False)
                f.write("\n")
            print(f"{name}: {pages} page(s), {len(glyphs)} glyphs, {len(rules)} rules")


def cmd_show(args):
    ref = json.load(open(os.path.join(REFERENCE, args.name + ".json"), encoding="utf-8"))
    lines = {}
    for page, x, y, text, font, size in ref["glyphs"]:
        lines.setdefault((page, round(y, 2), font, size), []).append((x, text))
    for (page, y, font, size), gl in sorted(lines.items()):
        gl.sort()
        print(f"p{page} y={y:8.3f} x0={gl[0][0]:8.3f} {font} {size}: {''.join(t for _, t in gl)!r}")
    for r in ref["rules"]:
        print("rule", r)


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("refs")
    r.add_argument("--texbin", default="/Library/TeX/texbin")
    r.add_argument("only", nargs="*")
    s = sub.add_parser("show")
    s.add_argument("name")
    args = ap.parse_args()
    {"refs": cmd_refs, "show": cmd_show}[args.cmd](args)


if __name__ == "__main__":
    main()
