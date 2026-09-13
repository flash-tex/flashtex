#!/usr/bin/env python3
"""Pseudocode oracle (TEST ONLY; pdflatex never runs in the product path).

For every `NN-*.tex` fixture here, runs `pdflatex` (two passes,
SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1) and writes `refs/NN-*.json` with,
per page, the word origins and the rules, extracted exactly as the float-body
and tabular corpora do (`crates/compiler/tests/tabular_corpus/oracle.py`
`ref_pages`: words from the content stream's glyph origins, rules from `re`+fill
and stroked `m`/`l` segments, continuing rules merged).

`tests/algorithms_oracle.rs` compares `flashtex-render`'s display list against
these committed files (words 0.5 bp, rules 0.1 bp), so cargo never runs TeX.

Unlike PR #162's original oracle this does not shell out to `pdftotext -bbox`
and does not recover baselines from Latin Modern box-height ratios: the glyph
origins come straight out of the PDF content stream, so every word carries a
real baseline and poppler is not a dependency.

    python3 oracle.py [--pdflatex pdflatex] [name-substring ...]
"""
import argparse, importlib.util, json, os, shutil, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
TABULAR = os.path.join(HERE, "..", "..", "..", "compiler", "tests", "tabular_corpus", "oracle.py")
spec = importlib.util.spec_from_file_location("tabular_oracle", TABULAR)
tabular = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tabular)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pdflatex", default=shutil.which("pdflatex") or "pdflatex",
                    help="the reference engine (default: the first pdflatex on PATH)")
    ap.add_argument("only", nargs="*")
    args = ap.parse_args()
    version = subprocess.run([args.pdflatex, "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    names = sorted(f[:-4] for f in os.listdir(HERE) if f.endswith(".tex") and f[:2].isdigit())
    names = [n for n in names if not args.only or any(o in n for o in args.only)]
    os.makedirs(os.path.join(HERE, "refs"), exist_ok=True)
    with tempfile.TemporaryDirectory() as work:
        for name in names:
            with open(os.path.join(HERE, name + ".tex"), encoding="utf-8") as f:
                src = f.read()
            with open(os.path.join(work, name + ".tex"), "w", encoding="utf-8") as f:
                f.write(src)
            for _ in range(2):
                r = subprocess.run([args.pdflatex, "-interaction=batchmode", "-halt-on-error", name + ".tex"],
                                   cwd=work, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                if r.returncode != 0:
                    sys.exit(f"{name}: pdflatex failed; see {os.path.join(work, name + '.log')}")
            pages = tabular.ref_pages(os.path.join(work, name + ".pdf"))
            with open(os.path.join(HERE, "refs", name + ".json"), "w", encoding="utf-8") as f:
                json.dump({"fixture": name + ".tex", "reference_engine": version,
                           "invocation": "pdflatex -interaction=batchmode, two passes, SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1",
                           "unit": "bp, y from page top; rules [x, top, width, height]",
                           "pages": [words for words, _, _ in pages],
                           "rules": [rules for _, _, rules in pages]}, f, indent=1)
                f.write("\n")
            print(f"{name}: {len(pages)} page(s), {sum(len(w) for w, _, _ in pages)} words, "
                  f"{sum(len(r) for _, _, r in pages)} rules")


if __name__ == "__main__":
    main()
