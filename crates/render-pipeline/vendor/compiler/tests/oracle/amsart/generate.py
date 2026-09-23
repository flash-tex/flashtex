#!/usr/bin/env python3
"""Regenerate the pdfLaTeX oracle for the AMS classes' top matter.

Oracle only: needs a TeX Live pdflatex and pdftotext (verified with pdfTeX
3.141592653-2.6-1.40.29, TeX Live 2026). Cargo tests never run TeX; they read
`expected.json`.

Each probe under `probes/` is one amsart/amsbook/amsproc document. pdflatex
compiles it with `-halt-on-error`; `expected.json` records, per probe, the
SHA-256 of the source, the page count, and the whole page text pdfTeX set
(`pdftotext`, reading order) with every whitespace character removed.
`tests/amsart_topmatter.rs` compiles the same source and requires that each
piece of top matter the class sets (the uppercased title and author line,
the `\\@adminfootnotes` for date, subject classification, key words and
`\\thanks`, the dedication, and the `\\address`/`\\email`/`\\curraddr`/`\\urladdr`
lines at the end of the document) is present, whitespace-free, in both the
oracle text and the compiler's, and that the compiler reports no error.

Usage: python3 crates/compiler/tests/oracle/amsart/generate.py
"""
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PROBES = os.path.join(HERE, "probes")
OUT = os.path.join(HERE, "expected.json")


def main():
    pdflatex = shutil.which("pdflatex")
    pdftotext = shutil.which("pdftotext")
    if not pdflatex or not pdftotext:
        sys.exit("pdflatex and pdftotext are required (oracle only)")
    version = subprocess.run([pdflatex, "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    probes = {}
    for name in sorted(os.listdir(PROBES)):
        if not name.endswith(".tex"):
            continue
        source = open(os.path.join(PROBES, name), encoding="utf-8").read()
        with tempfile.TemporaryDirectory() as tmp:
            shutil.copy(os.path.join(PROBES, name), tmp)
            env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
            for _ in range(2):
                run = subprocess.run(
                    [pdflatex, "-interaction=nonstopmode", "-halt-on-error", name],
                    cwd=tmp, capture_output=True, text=True, env=env,
                )
            if run.returncode != 0:
                sys.exit(f"{name}: pdflatex failed\n{run.stdout[-2000:]}")
            pdf = os.path.join(tmp, name[:-4] + ".pdf")
            text = subprocess.run([pdftotext, pdf, "-"], capture_output=True, text=True).stdout
            pages = text.count("\f") or 1
        probes[name] = {
            "sha256": hashlib.sha256(source.encode("utf-8")).hexdigest(),
            "pages": pages,
            "text": "".join(text.split()),
        }
    json.dump({"engine": version, "probes": probes}, open(OUT, "w"), indent=1, sort_keys=True)
    open(OUT, "a").write("\n")
    print(f"wrote {OUT}: {len(probes)} probes")


if __name__ == "__main__":
    main()
