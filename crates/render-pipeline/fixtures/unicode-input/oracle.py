#!/usr/bin/env python3
"""pdfLaTeX reference for literal UTF-8 input characters (test-only; cargo never runs TeX).

For every character `utf8.def`/the `*.dfu` files declare (read from
crates/tex-text-encoding/src/generated.rs) plus a few undeclared ones, and for
each of four preambles (OT1/T1, with/without lmodern), typesets
`\\setbox0\\hbox{<char>}` and records the LaTeX errors it raises and the box
width in sp (0 = nothing typeset). Writes reference.json next to this file.

    python3 crates/render-pipeline/fixtures/unicode-input/oracle.py
"""
import json, os, re, subprocess, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
GEN = os.path.join(ROOT, "crates", "tex-text-encoding", "src", "generated.rs")
PDFLATEX = os.environ.get("PDFLATEX", "/Library/TeX/texbin/pdflatex")

SETUPS = {
    "ot1": "",
    "t1": "\\usepackage[T1]{fontenc}\n",
    "ot1-lm": "\\usepackage{lmodern}\n",
    "t1-lm": "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n",
}
# Undeclared samples: a combining acute, Greek, Cyrillic, CJK, an emoji, a
# private-use character.
UNDECLARED = [0x0301, 0x0308, 0x03B1, 0x03C0, 0x0416, 0x4E2D, 0x1F600, 0xE000]


def declared():
    src = open(GEN, encoding="utf-8").read()
    start = src.index("UNICODE_DECLARATIONS")
    return [int(m.group(1), 16) for m in re.finditer(r"^\s*\(0x([0-9A-F]+),", src[start:], re.M)]


def run(setup, cps):
    body = "".join(
        "\\message{[CP=%04X]}\\setbox0\\hbox{%s}\\message{[WD=\\number\\wd0]}\n" % (cp, chr(cp)) for cp in cps
    )
    tex = "\\documentclass{article}\n" + SETUPS[setup] + "\\begin{document}\n" + body + "x\n\\end{document}\n"
    with tempfile.TemporaryDirectory() as d:
        open(os.path.join(d, "t.tex"), "w", encoding="utf-8").write(tex)
        subprocess.run(
            [PDFLATEX, "-interaction=nonstopmode", "-halt-on-error=false", "t.tex"],
            cwd=d, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            env=dict(os.environ, max_print_line="100000", error_line="254", half_error_line="238"),
        )
        log = open(os.path.join(d, "t.log"), encoding="utf-8", errors="replace").read()
    out, cur = {}, None
    # Messages and errors interleave in log order.
    for m in re.finditer(r"\[CP=([0-9A-F]+)\]|\[WD=(-?\d+)\]|^! (.*?)\n(?:\s+(.*?)\n)?", log, re.M):
        if m.group(1):
            cur = m.group(1)
            out.setdefault(cur, {"errors": [], "width_sp": None})
        elif m.group(2) is not None and cur:
            out[cur]["width_sp"] = int(m.group(2))
        elif m.group(3) and cur:
            msg = m.group(3)
            # pdflatex wraps "Unicode character X (U+XXXX)" / "not set up ..." over two lines.
            if m.group(4) and not m.group(4).startswith(("See the", "Type", "l.")):
                msg = msg + " " + m.group(4).strip()
            out[cur]["errors"].append(msg)
    return out


def main():
    cps = sorted(set(declared()) | set(UNDECLARED))
    ref = {"engine": subprocess.run([PDFLATEX, "--version"], capture_output=True, text=True).stdout.splitlines()[0],
           "setups": {}}
    for setup in SETUPS:
        ref["setups"][setup] = run(setup, cps)
    with open(os.path.join(HERE, "reference.json"), "w", encoding="utf-8") as f:
        json.dump(ref, f, ensure_ascii=False, indent=1, sort_keys=True)
        f.write("\n")


if __name__ == "__main__":
    main()
