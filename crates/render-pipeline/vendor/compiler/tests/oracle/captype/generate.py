#!/usr/bin/env python3
"""Regenerate the pdfLaTeX oracle for `\\caption`'s float type (`\\@captype`).

Oracle only: needs a TeX Live pdflatex and pdftotext (verified with pdfTeX
3.141592653-2.6-1.40.29, TeX Live 2026). Cargo tests never run TeX; they read
`expected.json`.

Each probe under `probes/` is one document. pdflatex compiles it with
`-halt-on-error`; `expected.json` records, per probe, the SHA-256 of the
source, the page count, and the caption labels pdfTeX set, in reading order:
every `<label> <number>` (with its colon when the class's `\\@makecaption`
adds one, without under float.sty's `ruled` style) in the whitespace-
normalised `pdftotext` output, for the labels `Figure`, `Table`, `Algorithm`
and the `\\floatname`s given in the probe's `%% labels:` header.
`tests/captype_oracle.rs` checks that the compiler sets the same label text
(number, colon or no colon) for the same `\\caption`s and reports no error.

Usage: python3 crates/compiler/tests/oracle/captype/generate.py
"""
import hashlib
import json
import os
import re
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
        labels = ["Figure", "Table", "Algorithm"]
        m = re.search(r"^%% labels: (.*)$", source, re.M)
        if m:
            labels += [w.strip() for w in m.group(1).split(",") if w.strip()]
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
            flat = " ".join(text.split())
            pattern = re.compile(r"(?:" + "|".join(re.escape(l) for l in labels) + r") \d+(?:\.\d+)*:?(?= )")
            captions = pattern.findall(flat)
        probes[name] = {
            "sha256": hashlib.sha256(source.encode("utf-8")).hexdigest(),
            "pages": pages,
            "caption_labels": captions,
        }
    json.dump({"engine": version, "probes": probes}, open(OUT, "w"), indent=1, sort_keys=True)
    open(OUT, "a").write("\n")
    print(f"wrote {OUT}: {len(probes)} probes")


if __name__ == "__main__":
    main()
