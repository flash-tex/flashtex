#!/usr/bin/env python3
"""Ask pdfTeX for the exact height/depth of a tabular box and of the draft
\\includegraphics box, at 10/11/12 pt."""
import os, re, subprocess, sys, tempfile

WORK = tempfile.mkdtemp(prefix="f12-showbox-")
TEX = "/Library/TeX/texbin/pdflatex"

TAB4 = r"""\begin{tabular}{|l|l|}
\hline
Problem & Proposed Fix \\
\hline
Duplicate work & A shared task register \\
Stale information & A single decision log \\
Bottlenecks & Rotating point-of-contact duty \\
\hline
\end{tabular}"""

TAB1 = r"""\begin{tabular}{|l|l|}
Problem & Proposed Fix \\
\end{tabular}"""

TAB2 = r"""\begin{tabular}{|l|l|}
Problem & Proposed Fix \\
Duplicate work & A shared task register \\
\end{tabular}"""

TAB1H = r"""\begin{tabular}{|l|l|}
\hline
Problem & Proposed Fix \\
\hline
\end{tabular}"""

TAB1S = r"""\renewcommand{\arraystretch}{1.5}\begin{tabular}{|l|l|}
Problem & Proposed Fix \\
\end{tabular}"""

TAB1T = r"""\begin{tabular}[t]{|l|l|}
Problem & Proposed Fix \\
\end{tabular}"""

CASES = {"tab4": TAB4, "tab1": TAB1, "tab2": TAB2, "tab1h": TAB1H, "tab1s": TAB1S, "tab1t": TAB1T}

DOC = r"""\documentclass[%SZ%pt]{article}
\usepackage{graphicx}
\pagestyle{empty}
\showboxbreadth=200 \showboxdepth=1 \tracingonline=0
\begin{document}
\typeout{AXIS=\the\fontdimen22\textfont2}
\typeout{BASELINESKIP=\the\baselineskip}
%BOXES%
\typeout{DONE}
\end{document}
"""


def main():
    os.makedirs(WORK, exist_ok=True)
    for sz in ("10", "11", "12"):
        boxes = []
        for cid, body in CASES.items():
            boxes.append("\\typeout{BOX=%s}\\setbox0=\\hbox{%s}\\showbox0" % (cid, body))
        boxes.append(r"\typeout{BOX=draft}\setbox0=\hbox{\includegraphics[draft,width=0.7\linewidth]{network.pdf}}\showbox0")
        src = DOC.replace("%SZ%", sz).replace("%BOXES%", "\n".join(boxes))
        d = os.path.join(WORK, sz)
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, "s.tex"), "w") as f:
            f.write(src)
        subprocess.run([TEX, "-interaction=batchmode", "s.tex"], cwd=d, capture_output=True,
                       env=dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1"), timeout=120)
        log = open(os.path.join(d, "s.log"), errors="replace").read()
        print(f"--- {sz}pt ---")
        for m in re.finditer(r"AXIS=(\S+)|BASELINESKIP=(\S+)", log):
            print("   ", m.group(0))
        cur = None
        for line in log.splitlines():
            if line.startswith("BOX="):
                cur = line[4:]
            m = re.match(r"^\\hbox\(([-\d.]+)\+([-\d.]+)\)x([-\d.]+)", line)
            if m and cur:
                print(f"    {cur:8s} h={m.group(1)} d={m.group(2)} w={m.group(3)}")
                cur = None
        for line in log.splitlines():
            if "not found" in line or "Warning" in line:
                print("    !", line[:110])


main()
