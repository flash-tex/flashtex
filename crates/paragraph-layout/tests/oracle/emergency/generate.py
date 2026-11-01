#!/usr/bin/env python3
"""Regenerate the \\emergencystretch / final-pass oracle for paragraph-layout.

MacTeX pdflatex is used here as an ORACLE ONLY. `cargo test` never runs TeX:
`tests/emergency_oracle.rs` reads the committed `fixtures/` and `expected/`.

Each fixture sets one paragraph in `\\setbox0=\\vbox{...}` under `\\sloppy` or
explicit `\\emergencystretch`/`\\tolerance` values at a narrow `\\hsize`, with
and without microtype, and dumps it in the microtype oracle's format
(`crates/microtype/tests/oracle/generate.py`, whose `extract` is reused):
`@@params` (plus `hbadness`, `hfuzz`), `@@list1`, `@@list2`, `@@result`,
`@@probe`, `@@font`. The paragraph is traced with `\\tracingparagraphs`, and
`expected/<name>.warn` records the passes pdfTeX ran (`pass second`,
`pass emergency`) and every Underfull/Overfull/Tight/Loose `\\hbox` warning
of box 0 in order (`underfull <badness>`, `overfull <pt>`, ...).

Usage:  python3 crates/paragraph-layout/tests/oracle/emergency/generate.py
Requires: pdflatex (TeX Live 2026 / pdfTeX 1.40.27 was used), microtype v3.2d.
"""

import importlib.util
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
FIX = os.path.join(HERE, "fixtures")
EXP = os.path.join(HERE, "expected")
MT = os.path.normpath(os.path.join(HERE, "../../../../microtype/tests/oracle/generate.py"))

spec = importlib.util.spec_from_file_location("mtgen", MT)
mtgen = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mtgen)

# (id, text key, class size, family, hsize, settings inside box 0)
COMBOS = [
    ("e1", "hw", 11, "ec", "120pt", r"\sloppy"),
    ("e2", "proof", 10, "ec", "100pt", r"\emergencystretch=10pt"),
    ("e3", "fox", 12, "lm", "90pt", r"\pretolerance=-1 \tolerance=100 \emergencystretch=2em"),
    ("e4", "dates", 10, "ec", "150pt", r"\sloppy"),
    ("e5", "compound", 11, "lm", "80pt", r"\emergencystretch=30pt \hbadness=500"),
    ("e6", "quotes", 10, "ec", "70pt", r"\emergencystretch=50pt"),
    ("e7", "proof", 11, "ec", "250pt", r"\tolerance=50 \emergencystretch=5pt"),
    ("e8", "fox", 10, "ec", "110pt", r"\emergencystretch=1pt \hfuzz=2pt"),
]

MODES = {
    "none": None,
    "both": "protrusion=true,expansion=true",
}

PARAMS = mtgen.PARAMS + [
    ("hbadness", r"\number\hbadness"),
    ("hfuzz", r"\number\hfuzz"),
]


def tex_source(combo, mode):
    cid, tkey, size, fam, hsize, settings = combo
    src = mtgen.tex_source((cid, tkey, size, fam, hsize, False), mode)
    params = r"\space ".join(r"%s=%s" % (k, v) for k, v in PARAMS)
    old = r"\setbox0=\vbox{\hsize=%s \parindent=0pt %%" % hsize
    assert old in src
    src = src.replace(
        old,
        r"\setbox0=\vbox{\hsize=%s \parindent=0pt %s \tracingparagraphs=1 \tracingonline=0 %%"
        % (hsize, settings),
    )
    src = re.sub(r"@@params [^}]*\}", lambda _: "@@params %s}" % params, src, count=1)
    return src


def warnings(log):
    lines = log.splitlines()
    start = next(i for i, l in enumerate(lines) if l.startswith("### horizontal mode entered"))
    end = next(i for i, l in enumerate(lines) if l == r"> \box0=")
    out = []
    for l in lines[start:end]:
        if l.startswith("@secondpass"):
            out.append("pass second")
        elif l.startswith("@emergencypass"):
            out.append("pass emergency")
        m = re.match(r"^(Underfull|Tight|Loose) \\hbox \(badness (\d+)\) in paragraph", l)
        if m:
            out.append("%s %s" % (m.group(1).lower(), m.group(2)))
        m = re.match(r"^Overfull \\hbox \(([0-9.]+)pt too wide\) in paragraph", l)
        if m:
            out.append("overfull " + m.group(1))
    return "\n".join(out) + "\n"


def main():
    pdflatex = shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex"
    env = dict(os.environ, max_print_line="1000000", error_line="254", half_error_line="238")
    os.makedirs(FIX, exist_ok=True)
    os.makedirs(EXP, exist_ok=True)
    n = 0
    with tempfile.TemporaryDirectory() as tmp:
        for combo in COMBOS:
            for mode in MODES:
                name = "%s-%s-%dpt-%s-%s" % (combo[0], combo[3], combo[2], combo[1], mode)
                src = tex_source(combo, mode)
                for d in (FIX, tmp):
                    with open(os.path.join(d, name + ".tex"), "w") as fh:
                        fh.write(src)
                subprocess.run(
                    [pdflatex, "-interaction=nonstopmode", name + ".tex"],
                    cwd=tmp, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                )
                with open(os.path.join(tmp, name + ".log"), encoding="latin-1") as fh:
                    log = fh.read()
                with open(os.path.join(tmp, name + ".dump"), encoding="latin-1") as fh:
                    dump = fh.read()
                with open(os.path.join(EXP, name + ".txt"), "w") as fh:
                    fh.write(mtgen.extract(log, dump))
                with open(os.path.join(EXP, name + ".warn"), "w") as fh:
                    fh.write(warnings(log))
                n += 1
                print("ok", name)
    print(n, "fixtures")


if __name__ == "__main__":
    sys.exit(main())
