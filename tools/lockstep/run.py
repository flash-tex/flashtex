#!/usr/bin/env python3
"""Lockstep differential harness: reference pdfTeX vs a candidate engine.

Runs each case in cases/<name>.tex through the reference and the candidate
with identical args and environment (one temp dir per run) and compares the
normalised transcript logs (tracing + box dumps). See README.md. Stdlib only.

The single-run core is capture(), importable by other tools (see README).
"""
import argparse
import dataclasses
import fnmatch
import os
import re
import shutil
import subprocess
import sys
import tempfile
import typing

HERE = os.path.dirname(os.path.abspath(__file__))
CASES_DIR = os.path.join(HERE, "cases")
EXPECTED_DIR = os.path.join(HERE, "expected")
PRELUDE = os.path.join(HERE, "prelude.tex")

# Fixed invocation. -cnf-line raises max_print_line/error_line, which are
# texmf.cnf values in web2c pdfTeX, not settable primitives. -etex switches
# both engines to extended mode (log shows "entering extended mode"), which
# is what the prelude's e-TeX tracing switches require.
ENGINE_ARGS = ["-cnf-line=max_print_line = 1000", "-cnf-line=error_line = 254",
               "-ini", "-etex", "-interaction=nonstopmode", "-halt-on-error"]
RUN_TIMEOUT = 120
# Marker the prelude writes via \message before every \shipout; capture()
# splits the log on it to recover one string per shipped box dump.
BOX_MARKER_RE = re.compile(r"LOCKSTEP-BOX \d+")
DATE_RE = re.compile(r"\b\d{1,2} (JAN|FEB|MAR|APR|MAY|JUN|JUL|AUG|SEP|OCT|NOV|DEC)"
                     r" \d{4}( \d{2}:\d{2})?\b")


def pinned_env():
    env = dict(os.environ)
    env.update(SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TZ="UTC")
    return env


def normalise(text, tmpdir):
    """Strip only what legitimately differs: temp paths, banner, dates."""
    lines = text.replace(tmpdir, "<TMP>").splitlines()
    if lines and lines[0].startswith("This is "):
        lines[0] = "BANNER"
    return "\n".join(DATE_RE.sub("<DATE>", ln) for ln in lines) + "\n"


@dataclasses.dataclass
class Capture:
    """One traced engine run: normalised log, per-shipout box dumps, PDF."""

    log: str  # normalised transcript text (always a plain str)
    boxes: typing.List[str]  # one normalised string per shipout box dump
    pdf_path: typing.Optional[str]  # produced PDF, or None if there is none
    returncode: int


def capture(tex_path, engine_bin, workdir, *, fmt=None, extra_env=None):
    """Run one engine once on tex_path and return a Capture.

    Runs with cwd=workdir and never wipes or cleans files already in it:
    tex_path may be a file inside workdir (a caller may stage a source
    tree, run its own convergence passes, then call capture for the one
    traced pass). The transcript is read from <jobname>.log in workdir;
    when the engine wrote no log, the captured stdout is used instead, so
    log is never missing. Each entry of boxes is the normalised log text
    after one LOCKSTEP-BOX marker up to the next marker (or end of log).

    fmt=None keeps the default: -ini (-etex) plain/primitive mode. When
    fmt is given (e.g. fmt="pdflatex"), the engine runs as -fmt=<fmt>
    instead and -ini mode is not used. extra_env adds environment
    variables on top of the pinned ones. stdin is DEVNULL so a run that
    accidentally enters \\errorstopmode (e.g. after an injected
    \\tracingall, see README) fails fast on EOF instead of blocking.

    Raises FileNotFoundError when the engine binary is missing and
    subprocess.TimeoutExpired on timeout.
    """
    env = pinned_env()
    if extra_env:
        env.update(extra_env)
    if fmt is None:
        args = ENGINE_ARGS
    else:
        args = [a for a in ENGINE_ARGS if a not in ("-ini", "-etex")]
        args = args + ["-fmt=" + fmt]
    job = os.path.splitext(os.path.basename(tex_path))[0]
    proc = subprocess.run([engine_bin] + args + [tex_path],
                          cwd=workdir, env=env, stdin=subprocess.DEVNULL,
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          timeout=RUN_TIMEOUT)
    out = proc.stdout.decode("utf-8", "replace")
    log_path = os.path.join(workdir, job + ".log")
    try:
        with open(log_path, encoding="utf-8", errors="replace") as fh:
            raw = fh.read()
    except OSError:
        raw = out
    log = normalise(raw, workdir)
    boxes = [seg.strip() for seg in BOX_MARKER_RE.split(log)[1:]]
    pdf_path = os.path.join(workdir, job + ".pdf")
    if not os.path.exists(pdf_path):
        pdf_path = None
    return Capture(log=log, boxes=boxes, pdf_path=pdf_path,
                   returncode=proc.returncode)


def run_engine(binary, name):
    """Run one engine on one case in a fresh temp dir. Returns a dict."""
    tmpdir = tempfile.mkdtemp(prefix="lockstep-")
    shutil.copy(PRELUDE, os.path.join(tmpdir, "prelude.tex"))
    shutil.copy(os.path.join(CASES_DIR, name + ".tex"),
                os.path.join(tmpdir, name + ".tex"))
    try:
        cap = capture(os.path.join(tmpdir, name + ".tex"), binary, tmpdir)
    except FileNotFoundError:
        return {"ok": False, "tmpdir": tmpdir, "error": "binary not found"}
    except subprocess.TimeoutExpired:
        return {"ok": False, "tmpdir": tmpdir, "error": "timed out"}
    if not cap.log.strip():
        return {"ok": False, "tmpdir": tmpdir,
                "error": "exit %d, no log" % cap.returncode}
    if cap.returncode != 0:
        tail = "\n".join(cap.log.splitlines()[-5:])
        return {"ok": True, "tmpdir": tmpdir, "log": cap.log,
                "error": "exit %d. tail:\n%s" % (cap.returncode, tail)}
    return {"ok": True, "tmpdir": tmpdir, "log": cap.log}


def check_pair(name, a_label, a_lines, b_label, b_lines):
    """Compare two normalised logs; report the first differing line."""
    n = max(len(a_lines), len(b_lines))
    idx = next((i for i in range(n)
                if (a_lines[i] if i < len(a_lines) else "<EOF>") !=
                   (b_lines[i] if i < len(b_lines) else "<EOF>")), None)
    if idx is None:
        print("PASS %s" % name)
        return True
    print("FAIL %s (%s vs %s differ)" % (name, a_label, b_label))
    print("  first difference at log line %d:" % (idx + 1))
    for j in range(max(0, idx - 3), idx + 1):
        for label, lines in ((a_label, a_lines), (b_label, b_lines)):
            if j < len(lines):
                print("  %4d %s: %s" % (j + 1, label, lines[j]))
            elif j == idx:
                print("  %4d %s: <EOF>" % (j + 1, label))
    return False


def write_expected(name, log):
    os.makedirs(EXPECTED_DIR, exist_ok=True)
    with open(os.path.join(EXPECTED_DIR, name + ".log"), "w") as fh:
        fh.write(log)


def select_cases(patterns):
    names = sorted(fn[:-4] for fn in os.listdir(CASES_DIR) if fn.endswith(".tex"))
    if not patterns:
        return names
    return sorted(nm for nm in names
                  if any(fnmatch.fnmatch(nm, p) or fnmatch.fnmatch(nm + ".tex", p)
                         for p in patterns))


def valid_run(result, name, what):
    if result.get("ok") and "log" in result:
        return True
    print("FAIL %s (%s failed: %s)" % (name, what, result.get("error")))
    return False


def main(argv=None):
    ap = argparse.ArgumentParser(description="lockstep differential harness")
    ap.add_argument("--engine", help="path to candidate engine binary")
    ap.add_argument("--cases", nargs="*", default=[], help="globs or case names")
    ap.add_argument("--reference", default="pdftex", help="reference engine")
    ap.add_argument("--update-expected", action="store_true",
                    help="regenerate expected/*.log from the reference")
    ap.add_argument("--keep", action="store_true", help="keep per-run temp dirs")
    ap.add_argument("--self-test", action="store_true",
                    help="run the reference against itself for every case")
    args = ap.parse_args(argv)

    if not os.path.isfile(PRELUDE):
        print("error: missing prelude.tex", file=sys.stderr)
        return 2
    names = select_cases(args.cases)
    if not names:
        print("error: no cases match %r" % (args.cases,), file=sys.stderr)
        return 2
    if shutil.which(args.reference) is None and not os.path.isfile(args.reference):
        print("error: reference not found: %s" % args.reference, file=sys.stderr)
        return 2
    if not args.self_test and not args.engine and not args.update_expected:
        print("error: --engine is required (or use --self-test)", file=sys.stderr)
        return 2

    kept, equal, differ = [], 0, 0
    for name in names:
        ref = run_engine(args.reference, name)
        if not valid_run(ref, name, "reference"):
            differ += 1
            continue
        if args.update_expected:
            write_expected(name, ref["log"])
        if args.self_test or not args.engine:
            if args.self_test:
                again = run_engine(args.reference, name)
                if not valid_run(again, name, "reference re-run"):
                    differ += 1
                    continue
                same = check_pair(name, "ref-run1", ref["log"].splitlines(),
                                  "ref-run2", again["log"].splitlines())
                kept.append(again["tmpdir"])
                exp = os.path.join(EXPECTED_DIR, name + ".log")
                if same and os.path.exists(exp):
                    with open(exp) as fh:
                        same = check_pair(name, "reference",
                                          ref["log"].splitlines(),
                                          "expected", fh.read().splitlines())
                equal, differ = equal + same, differ + (not same)
            else:
                print("wrote expected/%s.log" % name)
                equal += 1
        else:
            cand = run_engine(args.engine, name)
            if not valid_run(cand, name, "candidate"):
                differ += 1
                continue
            same = check_pair(name, "reference", ref["log"].splitlines(),
                              "candidate", cand["log"].splitlines())
            equal, differ = equal + same, differ + (not same)
            kept.append(cand["tmpdir"])
        kept.append(ref["tmpdir"])
    if not args.keep:
        for path in kept:
            shutil.rmtree(path, ignore_errors=True)
    else:
        for path in kept:
            print("kept %s" % path)
    if args.update_expected and not args.engine and not args.self_test:
        print("wrote %d expected file(s)" % equal)
        return 0 if differ == 0 else 1
    print("%d cases, %d equal, %d differ" % (len(names), equal, differ))
    return 1 if differ else 0


if __name__ == "__main__":
    sys.exit(main())
