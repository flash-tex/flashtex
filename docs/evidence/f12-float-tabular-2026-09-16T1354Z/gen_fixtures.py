#!/usr/bin/env python3
"""Writes crates/render-pipeline/fixtures/float-tabular-boundary/.

Each fixture is one of the F12 probes; `expected/<id>.txt` carries, for every
word pdflatex typeset, its baseline in bp from the page top, plus the residual
this engine is expected to sit at and the tolerance. Residual 0 means "exact";
a non-zero residual is the measured cmr - ec-lmr `charht` difference of the
line's tallest glyph, which is what the `\\lineskip` branch after a deep box
exposes.
"""
import os, re, subprocess, sys, tempfile
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
sys.path.insert(0, os.path.join(REPO, "tools/visual-oracle"))
import pdftext

DST = os.path.join(REPO, "crates/render-pipeline/fixtures/float-tabular-boundary")
TEX = "/Library/TeX/texbin/pdflatex"
BP = 1.00375  # 1 bp in TeX pt

# cmr / ec-lmr `charht` in em, from tftopl (TeX Live 2026).
CMR = {10: {"asc": 0.694445, "dig": 0.644444}, 12: {"asc": 0.694444, "dig": 0.644444}}
LMR = {10: {"asc": 0.688875, "dig": 0.6297245}, 12: {"asc": 0.688874, "dig": 0.629729}}
# (document size option) -> (\normalsize in pt, cmr/ec-lmr design used)
SIZE = {"10": (10.0, 10), "11": (10.95, 10), "12": (12.0, 12)}


def residual(sz, kind):
    pt, design = SIZE[sz]
    return -(CMR[design][kind] - LMR[design][kind]) * pt / BP


TAB = r"""\begin{tabular}{|l|l|}
\hline
Problem & Proposed Fix \\
\hline
Duplicate work & A shared task register \\
Stale information & A single decision log \\
Bottlenecks & Rotating point-of-contact duty \\
\hline
\end{tabular}"""

HEAD = "Alpha alpha alpha alpha.\n\nBravo bravo bravo bravo.\n\n"
TAIL = "\n%s\n\nDelta delta delta delta.\n\nEcho echo echo echo.\n"

# id -> (body, {word-prefix: residual-kind or None})
CASES = {
    # A float whose body is a rule: every float skip, \abovecaptionskip and
    # the caption's own line are exact.
    "figure-rule-caption": (
        HEAD + "\\begin{figure}[h]\n\\centering\n\\rule{0.7\\linewidth}{60pt}\n"
        "\\caption{Representative sparse sensor deployment topology used in the field study.}\n"
        "\\end{figure}\n" + TAIL % "Charlie charlie charlie charlie.", None),
    # The same float with no caption: \intextsep on both sides is exact.
    "figure-rule-nocaption": (
        HEAD + "\\begin{figure}[h]\n\\centering\n\\rule{0.7\\linewidth}{60pt}\n"
        "\\end{figure}\n" + TAIL % "Charlie charlie charlie charlie.", None),
    # A [t] float: \textfloatsep above the body text is exact.
    "figure-top": (
        "\\begin{figure}[t]\n\\centering\n\\rule{0.7\\linewidth}{60pt}\n"
        "\\caption{Summary of proposed fixes.}\n\\end{figure}\n" + HEAD
        + TAIL % "Charlie charlie charlie charlie.", None),
    # A table float holding a ruled tabular and no caption: the tabular's
    # rows, its total height and the body text below it are all exact.
    "table-tabular-nocaption": (
        HEAD + "\\begin{table}[h]\n\\centering\n" + TAB + "\n\\end{table}\n"
        + TAIL % "Charlie charlie charlie charlie.", None),
    # The tabular in running text, followed by a line whose tallest glyph is
    # `x`: cmr and ec-lmr agree on `x` to 5e-6 em, so the `\lineskip`
    # boundary itself is exact.
    "tabular-follower-xheight": (
        HEAD + TAB + "\n" + TAIL % "xxx xxx xxx xxx xxx.", None),
    # ... and by a line whose tallest glyph is `(`, where they agree exactly.
    "tabular-follower-paren": (
        HEAD + TAB + "\n" + TAIL % "(xxx) (xxx) (xxx) (xxx).", None),
    # Follower with an ascender: cmr's `h`/`l` are 0.00557 em taller than
    # ec-lmr's, and `\lineskip` puts that difference straight into the
    # baseline.
    "tabular-follower-ascender": (
        HEAD + TAB + "\n" + TAIL % "Charlie charlie charlie charlie.",
        ("Charlie", "asc")),
    # Follower of digits: 0.01472 em at the 10 pt design.
    "tabular-follower-digits": (
        HEAD + TAB + "\n" + TAIL % "123 456 789 123 456.", ("123", "dig")),
}

PRE = "\\documentclass[%SZ%pt]{article}\n\\usepackage[margin=1in]{geometry}\n\\pagestyle{empty}\n\\setlength{\\parindent}{0pt}\n\\begin{document}\n"


def main():
    os.makedirs(os.path.join(DST, "expected"), exist_ok=True)
    work = tempfile.mkdtemp(prefix="float-tabular-boundary-")
    for sz in ("10", "11", "12"):
        for cid, (body, marker) in CASES.items():
            fid = f"{cid}-{sz}pt"
            src = PRE.replace("%SZ%", sz) + body + "\\end{document}\n"
            with open(os.path.join(DST, fid + ".tex"), "w") as f:
                f.write(src)
            d = os.path.join(work, fid)
            os.makedirs(d, exist_ok=True)
            with open(os.path.join(d, "main.tex"), "w") as f:
                f.write(src)
            env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
            for _ in range(2):
                subprocess.run([TEX, "-interaction=batchmode", "main.tex"], cwd=d, env=env,
                               capture_output=True, timeout=120)
            pages = pdftext.page_words(os.path.join(d, "main.pdf"))
            assert len(pages) == 1, (fid, len(pages))
            lines = [
                "# " + fid + ": every baseline pdflatex set on the page.",
                "# <baseline, bp from the page top> <expected residual bp> <tol bp> <first word>",
                "# Residual 0 means this engine must agree with pdfTeX. A non-zero residual",
                "# is the cmr - ec-lmr `charht` difference of that line's tallest glyph; see",
                "# the test's module docs for why `\\lineskip` exposes it.",
            ]
            res_kind = marker[1] if marker else None
            rows = {}
            for w in pages[0]["words"]:
                key = round(w["y_alnum"], 3)
                if key not in rows:
                    rows[key] = w["text"]
            started = False
            for key in sorted(rows):
                if marker and rows[key].startswith(marker[0]):
                    started = True
                r = residual(sz, res_kind) if (started and res_kind) else 0.0
                tol = 0.1 if r == 0.0 else 0.003
                lines.append(f"{key:.5f} {r:.5f} {tol} {rows[key]}")
            with open(os.path.join(DST, "expected", fid + ".txt"), "w") as f:
                f.write("\n".join(lines) + "\n")
    print("wrote", len(CASES) * 3, "fixtures")


main()
