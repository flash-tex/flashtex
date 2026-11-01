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
    python3 tools/xetex-lockstep/latex.py --engine <bin> --cases-dir DIR [--json OUT]

`--cases-dir DIR` (repeatable) takes the cases from DIR instead of
`latex-cases/`: each `NAME.tex`, and each directory `NAME/` holding
`NAME.tex` with the files it reads (tools/unicode-m3/corpus.py writes these).
Such a directory may hold `.m3case.json`: `firstline`, TeX code put ahead of
`\\input{NAME}` on the first line of every run of both engines (the effective
source of a `[fonts]` project, PROPOSAL.md §4.5), and `trace`, `full` (the
default: `\\tracingall`, as above) or `boxes` (`\\tracingoutput` and the box
dumps at every `\\shipout`, without `\\tracingall`'s macro trace: for long
documents, reported as such).
"""
import argparse
import fnmatch
import json
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
TRACE_PREFIX = {
    "full": "\\tracingall\\tracingonline=1\\showboxdepth=2147483647\\showboxbreadth=2147483647",
    "boxes": "\\tracingoutput=1\\tracingonline=1\\showboxdepth=2147483647\\showboxbreadth=2147483647",
}
META = ".m3case.json"


def find_cases(dirs):
    """{name: path} of the cases in `dirs`: `NAME.tex` files and `NAME/`
    directories holding `NAME.tex`."""
    out = {}
    for d in dirs:
        for fn in sorted(os.listdir(d)):
            p = os.path.join(d, fn)
            if fn.endswith(".tex") and os.path.isfile(p):
                out[fn[:-4]] = p
            elif os.path.isdir(p) and os.path.isfile(os.path.join(p, fn + ".tex")):
                out[fn] = p
    return out


def case_meta(src):
    p = os.path.join(src, META)
    if os.path.isdir(src) and os.path.isfile(p):
        with open(p, encoding="utf-8") as f:
            return json.load(f)
    return {}


def stage(src, name, d):
    """Copy case `name` from `src` (a file or a directory) into `d`."""
    os.makedirs(d, exist_ok=True)
    if os.path.isdir(src):
        shutil.copytree(src, d, dirs_exist_ok=True, symlinks=True,
                        ignore=shutil.ignore_patterns(META, "expect.json"))
    else:
        shutil.copy(src, os.path.join(d, name + ".tex"))


def doc_arg(name, meta, traced):
    """The last argument of a run: the main file, after the case's first-line
    code (the host's `preamble\\input{main.tex}`), traced or not."""
    first = meta.get("firstline", "")
    if traced:
        return TRACE_PREFIX[meta.get("trace", "full")] + first + "\\input{%s}" % name
    if first:
        return first + "\\input{%s.tex}" % name
    return name + ".tex"


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


def run_case(name, engines, fmts, tmp, timeout, src=None, keep=True):
    src = src or os.path.join(CASES, name + ".tex")
    meta = case_meta(src)
    res = {}
    secs = {}
    for who in ("reference", "candidate"):
        d = os.path.join(tmp, name, who)
        stage(src, name, d)
        exe = link(engines[who], os.path.join(d, ".bin"), "xelatex")
        env = env_for(fmts[who])
        t = time.time()
        errp = os.path.join(d, ".stderr")
        try:
            with open(errp, "wb") as err:
                subprocess.run([exe] + RUN_ARGS + [doc_arg(name, meta, False)], cwd=d, env=env,
                               stdout=subprocess.DEVNULL, stderr=err, timeout=timeout)
                r = subprocess.run([exe] + RUN_ARGS + [doc_arg(name, meta, True)], cwd=d, env=env,
                                   stdout=subprocess.DEVNULL, stderr=err, timeout=timeout)
            code = r.returncode
        except subprocess.TimeoutExpired:
            code = "timeout"
        logp = os.path.join(d, name + ".log")
        log = open(logp, encoding="utf-8", errors="surrogateescape").read() \
            if os.path.exists(logp) else ""
        # the first TeX error, and an engine's panic, for the report
        first_error = next((ln for ln in log.split("\n") if ln.startswith("! ")), None)
        stderr = open(errp, encoding="utf-8", errors="replace").read()
        # a panic's location, its message and (with RUST_BACKTRACE) the frames
        el = stderr.split("\n")
        at = next((k for k, ln in enumerate(el) if "panicked" in ln), None)
        panic = [] if at is None else [ln for ln in el[at:at + 40] if ln.strip()][:40]
        # only the compared lines are kept (traced logs run to millions of lines)
        lines, _ = LS.split_accounting(X.normalise(log, d).split("\n"))
        del log
        res[who] = (code, lines, os.path.join(d, name + ".xdv"), first_error, panic)
        secs[who] = round(time.time() - t, 2)
    msgs = []
    (rc, a, rxdv, rerr, _), (cc, b, cxdv, cerr, cpanic) = res["reference"], res["candidate"]
    if rc != cc:
        msgs.append("exit status reference=%s candidate=%s" % (rc, cc))
    notes = []  # shown, not failures: the oracle decides what a document does
    if rc != 0 and rerr:
        notes.append("reference's first error: %s" % rerr[:200])
    if cc != 0 and cerr:
        notes.append("candidate's first error: %s" % cerr[:200])
    if cpanic:
        msgs.append("the candidate panicked: %s" % cpanic[0][:300])
        notes.extend("  " + ln[:300] for ln in cpanic[1:])
    if a != b:
        i = next((k for k in range(min(len(a), len(b))) if a[k] != b[k]),
                 min(len(a), len(b)))
        msgs.append("log differs at line %d of %d/%d" % (i + 1, len(a), len(b)))
        msgs.append("  reference: %s" % (a[i] if i < len(a) else "<EOF>")[:300])
        msgs.append("  candidate: %s" % (b[i] if i < len(b) else "<EOF>")[:300])
    x = X.compare_xdv(rxdv, cxdv)
    if x:
        msgs.append(x)
    # `[fonts]` precedence (PROPOSAL.md §4.5): fonts each XDV must or must not
    # load (file names in its define_native_font records), in both engines,
    # so the oracle confirms what the expectation says
    for who, xdv in (("reference", rxdv), ("candidate", cxdv)):
        data = open(xdv, "rb").read() if os.path.exists(xdv) else b""
        for f in meta.get("expect_fonts", []):
            if f.encode() not in data:
                msgs.append("%s: %s is not loaded (expected)" % (who, f))
        for f in meta.get("forbid_fonts", []):
            if f.encode() in data:
                msgs.append("%s: %s is loaded (the document's font should win)" % (who, f))
    info = {"case": name, "ok": not msgs, "messages": msgs, "notes": notes, "log_lines": len(a),
            "exit": [rc, cc], "reference_error": rerr, "seconds": [secs["reference"], secs["candidate"]],
            "trace": meta.get("trace", "full"), "firstline": bool(meta.get("firstline")),
            "xdv": os.path.exists(rxdv)}
    if not keep:
        shutil.rmtree(os.path.join(tmp, name), ignore_errors=True)
    return name, not msgs, msgs + notes, len(a), info


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--engine", required=True, help="candidate engine binary")
    ap.add_argument("--reference", default="xetex")
    ap.add_argument("--cases", nargs="*", default=["*"])
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--timeout", type=float, default=900)
    ap.add_argument("--keep", action="store_true")
    ap.add_argument("--cases-dir", action="append",
                    help="take the cases from DIR (repeatable) instead of latex-cases/")
    ap.add_argument("--json", help="write every case's result here")
    a = ap.parse_args(argv)
    # the XDV comparison's own checks, before it is relied on
    X._self_test_normalise_xdv()
    X._self_test_font_files()
    if not X.check_reference(a.reference):
        print("reference %s is not %s" % (a.reference, X.PINNED_REFERENCE))
        return 2
    srcs = find_cases(a.cases_dir or [CASES])
    names = [n for n in sorted(srcs) if any(fnmatch.fnmatch(n, p) for p in a.cases)]
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
    infos = []
    with ThreadPoolExecutor(max_workers=a.jobs) as ex:
        for name, good, msgs, n, info in ex.map(
                lambda n: run_case(n, engines, fmts, tmp, a.timeout, srcs[n], a.keep), names):
            ok += good
            infos.append(info)
            print("%s %s (%d log lines%s)" % ("PASS" if good else "FAIL", name, n,
                                               ", box dumps only" if info["trace"] == "boxes" else ""),
                  flush=True)
            for m in msgs:
                print("  " + m)
    print("%d cases, %d equal, %d differ" % (len(names), ok, len(names) - ok))
    if a.json:
        with open(a.json, "w") as f:
            json.dump({"engine": a.engine, "reference": a.reference, "format_log_lines": len(fa),
                       "format_log_differences": len(fdiff) + abs(len(fa) - len(fb)),
                       "cases": infos, "equal": ok, "total": len(names)}, f, indent=1)
    if a.keep:
        print("kept:", tmp)
    else:
        shutil.rmtree(tmp, ignore_errors=True)
    return 0 if ok == len(names) else 1


if __name__ == "__main__":
    sys.exit(main())
