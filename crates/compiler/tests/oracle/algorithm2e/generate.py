#!/usr/bin/env python3
"""Regenerate the pdfLaTeX oracle for the algorithm2e model.

Oracle only: needs a TeX Live pdflatex and pdftotext (verified with pdfTeX
3.141592653-2.6-1.40.29, TeX Live 2026). Cargo tests never run TeX; they read
`expected.json`.

Each probe under `probes/` is one document. pdflatex compiles it with
`-halt-on-error`; `expected.json` records, per probe, the SHA-256 of the
source, the page count, and the whitespace-normalised `pdftotext` text of the
document without its page number: every keyword, line number, caption and
comment algorithm2e set, in pdftotext's reading order. `tests/
algorithm2e_oracle.rs` checks that the compiler sets the same characters
(the multiset of non-blank characters, since its own layout orders a side
comment differently from pdftotext) and reports no error.

Usage: python3 crates/compiler/tests/oracle/algorithm2e/generate.py
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
            # The page number is the last token of each page's text (article's
            # plain page style); the compiler's items carry none.
            body = []
            for page_index, page in enumerate(text.split("\f")):
                words = page.split()
                if words and words[-1] == str(page_index + 1):
                    words = words[:-1]
                body.extend(words)
            flat = " ".join(body)
        probes[name] = {
            "sha256": hashlib.sha256(source.encode("utf-8")).hexdigest(),
            "pages": pages,
            "text": flat,
        }
    json.dump({"engine": version, "probes": probes}, open(OUT, "w"), indent=1, sort_keys=True)
    open(OUT, "a").write("\n")
    print(f"wrote {OUT}: {len(probes)} probes")


if __name__ == "__main__":
    main()
