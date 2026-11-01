#!/usr/bin/env python3
"""Shared pdflatex-oracle helper: run, version stamp, overfull-box count.

pdflatex is the ORACLE ONLY (never in the product path, never in cargo
tests). The render-pipeline `*-oracle` generators each grew their own copy
of this logic; this module is the single copy. Import it with the repo root
on `sys.path`:

    REPO = <path to the repo root>
    sys.path.insert(0, REPO)
    from tools.oracle import pdflatex as oracle

Deliberately NOT here: `\\showoutput` parsing (no Python script implements
it) and `\\pdfsavepos` mark extraction (each oracle's mark format and
coordinate handling is bespoke to that oracle, so there is no shared logic
to extract).
"""
import os
import subprocess
import sys


def run_pdflatex(tex_path, workdir, cwd=None, error_tail=3000):
    """Run pdflatex twice on `tex_path`, return the PDF path in `workdir`.

    The two passes resolve references; `\\pdfcompresslevel=0
    \\pdfobjcompresslevel=0` on the command line leave the content streams
    uncompressed for the PDF readers (changes nothing typeset). `cwd` is the
    processes' working directory (fixtures with relative `images/...` paths
    pass their fixture directory; otherwise the caller's `workdir`); only
    the last `error_tail` characters of stdout are shown when pdflatex fails.
    """
    name = os.path.splitext(os.path.basename(tex_path))[0]
    cmd = [
        "pdflatex",
        "-interaction=nonstopmode",
        "-halt-on-error",
        f"-jobname={name}",
        f"-output-directory={workdir}",
        "\\pdfcompresslevel=0\\pdfobjcompresslevel=0\\input{" + tex_path + "}",
    ]
    for _ in range(2):
        r = subprocess.run(cmd, cwd=workdir if cwd is None else cwd, capture_output=True, text=True)
        if r.returncode != 0:
            sys.exit(f"pdflatex failed for {name}:\n{r.stdout[-error_tail:]}")
    return os.path.join(workdir, name + ".pdf")


def pdftex_version():
    """First line of `pdflatex --version`, stamped into expected files."""
    return subprocess.run(["pdflatex", "--version"], capture_output=True, text=True).stdout.splitlines()[0]


def count_overfull_boxes(log_path):
    """Overfull `\\hbox` + `\\vbox` count in a pdflatex log (0 if missing)."""
    over = 0
    if os.path.exists(log_path):
        text = open(log_path, errors="replace").read()
        over = text.count("Overfull \\hbox") + text.count("Overfull \\vbox")
    return over
