#!/usr/bin/env python3
"""Terminal-interaction fuzzer: error prompts and \\read16 answered from stdin. MIT.

    python3 tools/fuzz-parity/interact.py --candidate BIN [--oracle pdftex]
        --cases 0:300 [--out DIR]

The lockstep capture runs every document with stdin closed, so the error
prompt (`?`, tex.web §84), `\\read16` and the file-name prompt only ever
see end of file. Here each case is a small INITEX document of errors in
one of the interaction modes plus a scripted stdin (help, numbers that
delete tokens, `i`/`I` insertions, `x`, `q`/`r`/`s`, `e`/`E`, plain lines,
Control-D). Both engines run `-ini` in directories of their own, as
`pdftex`, with `SOURCE_DATE_EPOCH=0`, the mktex scripts off and
`TEXEDIT=true`, so `E` runs no editor. Compared: the exit code, stdout
after the banner (each engine's own path masked) and the log after the
banner. A differing case is written to OUT/<case>/ (`t.tex`, `stdin`).
Exit 0 when every case was equal.
"""
import argparse
import difflib
import os
import random
import shutil
import subprocess
import sys
import tempfile

ERRS = ["\\undefinedzz", "\\hbox{\\vskip1pt}", "$x^^y$", "\\endgroup", "}", "\\hrule\\par x\\vrule",
        "\\count0=\\undefinedzz", "\\dimen0=1zz", "\\def\\a#2{}", "\\errmessage{custom}",
        "{\\errhelp{help me}\\errmessage{with help}}", "\\font\\x=nonexistentzz", "\\input nonexistentzz ",
        "\\openout3=|nope \\immediate\\write3{x}", "\\read16 to\\x \\show\\x", "\\read-1 to\\y \\show\\y",
        "\\show\\par", "\\showthe\\count0", "\\showbox0", "\\showlists", "\\message{m}", "$$x$$\\endgroup",
        "\\halign{#\\cr x&y\\cr}", "\\char-1", "\\catcode300=1", "\\outer\\def\\o{}\\message{\\o}",
        "\\uppercase{\\undefinedzz}", "\\csname\\relax\\endcsname"]
ANSWERS = ["", "\n", "h\n", "1\n", "2\n", "9\n", "99\n", "i\\message{ins}\n", "I\\relax\n", "x\n", "X\n",
           "q\n", "r\n", "s\n", "e\n", "E\n", "?\n", "abc\n", "\\relax\n", "fzread line\n", "{unbalanced\n",
           "nonexistentzz\n", "\\end\n", "  spaces  \n", "\x04", "h\nh\n"]


def case(n):
    """(document, stdin bytes) of case n: a pure function of n."""
    r = random.Random(n)
    body = " ".join(r.choice(ERRS) for _ in range(r.randint(1, 6)))
    pre = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\catcode`\\$=3 \\catcode`\\&=4 \\catcode`\\^=7 "
    pre += r.choice(["", "\\scrollmode ", "\\nonstopmode ", "\\batchmode ", "\\errorstopmode "])
    pre += r.choice(["", "\\tracingonline1 ", "\\errorcontextlines=%d " % r.randint(-1, 5)])
    stdin = "".join(r.choice(ANSWERS) for _ in range(r.randint(0, 8)))
    return pre + body + "\n\\end\n", stdin.encode("latin-1")


def run(engine, d, doc, stdin, timeout):
    """(exit code, stdout after the banner, log after the banner), None on a timeout."""
    os.makedirs(d)
    with open(os.path.join(d, "t.tex"), "w", encoding="latin-1") as fh:
        fh.write(doc)
    exe = os.path.join(d, "pdftex")
    os.symlink(engine, exe)
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", MKTEXTFM="0",
               MKTEXPK="0", MKTEXMF="0", TEXEDIT="true %d %s")
    try:
        p = subprocess.run([exe, "-ini", "t.tex"], cwd=d, env=env, input=stdin,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
    except subprocess.TimeoutExpired:
        return None
    out = p.stdout.decode("latin-1").replace(exe, "PDFTEX").partition("\n")[2]
    try:
        with open(os.path.join(d, "t.log"), encoding="latin-1") as fh:
            log = fh.read().partition("\n")[2]
    except OSError:
        log = None
    return p.returncode, out, log


def compare(n, candidate, oracle, tmp=None, timeout=20):
    """(class, detail) for case n: equal, timeout, rc, stdout or log."""
    doc, stdin = case(n)
    w = tempfile.mkdtemp(prefix="fzi-%d-" % n, dir=tmp)
    try:
        o = run(oracle, os.path.join(w, "o"), doc, stdin, timeout)
        c = run(candidate, os.path.join(w, "c"), doc, stdin, timeout)
    finally:
        shutil.rmtree(w, ignore_errors=True)
    if o == c:
        return "equal", None
    if o is None or c is None:
        return "timeout", "oracle" if o is None else "candidate"
    if o[0] != c[0]:
        return "rc", "%s/%s" % (o[0], c[0])
    i = 1 if o[1] != c[1] else 2
    d = list(difflib.unified_diff((o[i] or "").split("\n"), (c[i] or "").split("\n"), lineterm="", n=1))
    return ("stdout" if i == 1 else "log"), "\n".join(x[:160] for x in d[:14])


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--oracle", default="pdftex")
    ap.add_argument("--cases", default="0:100", help="START:COUNT")
    ap.add_argument("--out", default="fuzz-interact-out")
    ap.add_argument("--tmp", default=None)
    ap.add_argument("--timeout", type=float, default=20)
    args = ap.parse_args(argv)
    oracle = os.path.realpath(shutil.which(args.oracle) or args.oracle)
    candidate = os.path.abspath(args.candidate)
    a, _, k = args.cases.partition(":")
    differ = 0
    for n in range(int(a), int(a) + int(k or 1)):
        cls, detail = compare(n, candidate, oracle, args.tmp, args.timeout)
        if cls == "equal":
            continue
        differ += 1
        d = os.path.join(args.out, str(n))
        os.makedirs(d, exist_ok=True)
        doc, stdin = case(n)
        with open(os.path.join(d, "t.tex"), "w", encoding="latin-1") as fh:
            fh.write(doc)
        with open(os.path.join(d, "stdin"), "wb") as fh:
            fh.write(stdin)
        print("case %d: %s %s" % (n, cls, detail or ""), flush=True)
    print("%s cases: %d differ" % (k or 1, differ))
    return 0 if differ == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
