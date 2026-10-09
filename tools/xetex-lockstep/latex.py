#!/usr/bin/env python3
"""xelatex documents: P-T1 and XDV of the XeTeX port against TeX Live's xelatex.

Phase S1's third gate (docs/design/xetex/PLAN.md §3, S1): `xelatex`
documents with `fontspec`, compared with `xelatex -no-pdf`. TeX Live 2026's
xetex is the oracle only; it never runs in the product path.

Each engine first builds its own `xelatex.fmt` the way fmtutil does
(`-ini -jobname=xelatex -progname=xelatex -etex xelatex.ini`). Then every
case of `latex-cases/` runs with each engine and its own format, in its own
directory, twice: an untraced pass (for the `.aux` file), then a traced
pass as tools/parity's P-T1 capture runs it (`\\tracingall`,
`\\tracingonline=1`, unlimited `\\showbox...`, `max_print_line=10000`,
`error_line=254`, `SOURCE_DATE_EPOCH=0`, `FORCE_SOURCE_DATE=1`). The traced
log is compared line for line after tools/lockstep's normalisation and
accounting rule (DESIGN.md §1.1), the exit statuses must match, and the XDV
files must be byte-identical after the two normalisations of run.py
(PLAN.md §3.5).

    python3 tools/xetex-lockstep/latex.py --engine <bin> [--cases 'l00*'] [--jobs N] [--keep]
"""
import argparse
import fnmatch
import os
import shutil
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import run as X  # noqa: E402  (tools/xetex-lockstep/run.py)

LS = X.LS
CASES = os.path.join(HERE, "latex-cases")
FMT_ARGS = ["-ini", "-jobname=xelatex", "-progname=xelatex", "-etex", "xelatex.ini"]
RUN_ARGS = ["-fmt=xelatex", "-no-pdf", "-interaction=nonstopmode",
            "-cnf-line=max_print_line=10000", "-cnf-line=error_line=254"]
TRACE = ("\\tracingall\\tracingonline=1"
         "\\showboxdepth=2147483647\\showboxbreadth=2147483647\\input{%s}")


def env_for(fmt_dir):
    env = X.pinned_env()
    env["TEXFORMATS"] = fmt_dir + ":"
    return env


def link(binary, d, name):
    os.makedirs(d, exist_ok=True)
    target = os.path.realpath(shutil.which(binary) or binary)
    p = os.path.join(d, name)
    if os.path.lexists(p):
        os.remove(p)
    os.symlink(target, p)
    return p


def build_format(binary, d):
    """xelatex.fmt in `d`, as fmtutil builds it; (seconds, exit status, log)."""
    exe = link(binary, os.path.join(d, ".bin"), "xetex")
    t = time.time()
    r = subprocess.run([exe] + FMT_ARGS, cwd=d, env=X.pinned_env(),
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log = open(os.path.join(d, "xelatex.log"), encoding="utf-8",
               errors="surrogateescape").read() if os.path.exists(
                   os.path.join(d, "xelatex.log")) else ""
    return time.time() - t, r.returncode, log


def run_case(name, engines, fmts, tmp, timeout):
    src = os.path.join(CASES, name + ".tex")
    res = {}
    for who in ("reference", "candidate"):
        d = os.path.join(tmp, name, who)
        os.makedirs(d, exist_ok=True)
        shutil.copy(src, os.path.join(d, name + ".tex"))
        exe = link(engines[who], os.path.join(d, ".bin"), "xelatex")
        env = env_for(fmts[who])
        try:
            subprocess.run([exe] + RUN_ARGS + [name + ".tex"], cwd=d, env=env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                           timeout=timeout)
            r = subprocess.run([exe] + RUN_ARGS + [TRACE % name], cwd=d, env=env,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                               timeout=timeout)
            code = r.returncode
        except subprocess.TimeoutExpired:
            code = "timeout"
        logp = os.path.join(d, name + ".log")
        log = open(logp, encoding="utf-8", errors="surrogateescape").read() \
            if os.path.exists(logp) else ""
        res[who] = (code, X.normalise(log, d), os.path.join(d, name + ".xdv"))
    msgs = []
    (rc, rlog, rxdv), (cc, clog, cxdv) = res["reference"], res["candidate"]
    if rc != cc:
        msgs.append("exit status reference=%s candidate=%s" % (rc, cc))
    a, _ = LS.split_accounting(rlog.split("\n"))
    b, _ = LS.split_accounting(clog.split("\n"))
    if a != b:
        i = next((k for k in range(min(len(a), len(b))) if a[k] != b[k]),
                 min(len(a), len(b)))
        msgs.append("log differs at line %d of %d/%d" % (i + 1, len(a), len(b)))
        msgs.append("  reference: %s" % (a[i] if i < len(a) else "<EOF>")[:300])
        msgs.append("  candidate: %s" % (b[i] if i < len(b) else "<EOF>")[:300])
    x = X.compare_xdv(rxdv, cxdv)
    if x:
        msgs.append(x)
    return name, not msgs, msgs, len(a)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--engine", required=True, help="candidate engine binary")
    ap.add_argument("--reference", default="xetex")
    ap.add_argument("--cases", nargs="*", default=["*"])
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--timeout", type=float, default=900)
    ap.add_argument("--keep", action="store_true")
    a = ap.parse_args(argv)
    if not X.check_reference(a.reference):
        print("reference %s is not %s" % (a.reference, X.PINNED_REFERENCE))
        return 2
    names = sorted(fn[:-4] for fn in os.listdir(CASES) if fn.endswith(".tex"))
    names = [n for n in names if any(fnmatch.fnmatch(n, p) for p in a.cases)]
    tmp = tempfile.mkdtemp(prefix="xelatex-lockstep-")
    engines = {"reference": a.reference, "candidate": a.engine}
    fmts = {}
    logs = {}
    for who in ("reference", "candidate"):
        d = os.path.join(tmp, "fmt-" + who)
        os.makedirs(d)
        secs, code, log = build_format(engines[who], d)
        print("format (%s): exit %s, %.1f s" % (who, code, secs))
        if code != 0:
            print("the %s could not build xelatex.fmt" % who)
            return 1
        fmts[who] = d
        logs[who] = X.normalise(log, d).split("\n")
    # The format logs differ in one known line, `\dump`'s "N strings of
    # total length L" (capacity accounting, DESIGN.md §1.1): TeX Live's
    # xetex has 15 more pool strings (428 more characters) than the port.
    # Measured by diffing the two formats' string pools: TeX Live's tex.ch
    # changes string literals without changing what is printed
    # (print_mode/print_in_mode's "vertical mode" and "' in vertical mode",
    # runaway's "Runaway definition", " while scanning definition", the
    # plurals " lines", " preloaded fonts", " hyphenation exceptions",
    # " ops") and adds ML\TeX's strings ("charsubdef", "substitution for
    # ", ...), while the port keeps xetex.web's split literals and
    # "TeXinputs:"/"TeXfonts:". Only string numbers and this count differ.
    # It is shown, not hidden.
    fa, fb = logs["reference"], logs["candidate"]
    fdiff = [(x, y) for x, y in zip(fa, fb) if x != y]
    print("format logs: %d lines, %d differ%s" % (
        len(fa), len(fdiff) + abs(len(fa) - len(fb)),
        "".join("\n  ref:  %s\n  cand: %s" % p for p in fdiff[:5])))
    ok = 0
    with ThreadPoolExecutor(max_workers=a.jobs) as ex:
        for name, good, msgs, n in ex.map(
                lambda n: run_case(n, engines, fmts, tmp, a.timeout), names):
            ok += good
            print("%s %s (%d log lines)" % ("PASS" if good else "FAIL", name, n))
            for m in msgs:
                print("  " + m)
    print("%d cases, %d equal, %d differ" % (len(names), ok, len(names) - ok))
    if a.keep:
        print("kept:", tmp)
    else:
        shutil.rmtree(tmp, ignore_errors=True)
    return 0 if ok == len(names) else 1


if __name__ == "__main__":
    sys.exit(main())
