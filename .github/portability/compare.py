#!/usr/bin/env python3
"""The engine against TeX Live's pdfTeX on the portability workflow's
Windows runner (.github/workflows/portability.yml). Oracle tooling only:
pdftex.exe never runs in the product path.

  compare.py write18 --oracle PDFTEX --engine INITEX --pool POOL --work DIR
      INITEX of write18.tex with restricted \\write18 by both programs, each
      as `pdftex` (argv[0]), in its own directory: their logs after the
      banner must be equal, and on Windows the single-quoted pipe must run.
      Then a full \\write18 through cmd.exe (`echo ... > file`) by the engine.

  compare.py pt1 --oracle PDFTEX --engine INITEX --pool POOL --work DIR
      DESIGN.md §1.1 P-T1 on pt1.tex: each program makes pdftex.fmt as
      fmtutil does (-etex, cp227.tcx, pdfetex.ini), then runs the traced
      pass of tools/parity/capture.py (its seed, \\tracingall and
      \\showbox settings, environment and normalisation): the box dumps at
      every \\shipout and the strict log must be equal.

Exit status 0 when equal, 1 when not, 2 when a run failed to produce
output.
"""

import argparse
import difflib
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(ROOT, "tools", "parity"))
import capture  # noqa: E402


def env_for(engine, pool, extra=None):
    env = {k: v for k, v in os.environ.items() if not k.startswith("FLASHTEX_")}
    env.update(capture.TRACE_ENV)
    if engine:
        env["FLASHTEX_POOL"] = pool
    env.update(extra or {})
    return env


def run(prog, cwd, args, env, timeout=900):
    """`prog` run as `pdftex` (argv[0], as TeX Live runs it), stdin closed."""
    p = subprocess.run(["pdftex"] + args, executable=prog, cwd=cwd, env=env,
                       stdin=subprocess.DEVNULL, capture_output=True, timeout=timeout)
    return p.returncode, p.stdout.decode("latin-1"), p.stderr.decode("latin-1")


def after_banner(log):
    lines = log.replace("\r\n", "\n").split("\n")
    start = next((i for i, ln in enumerate(lines) if capture.banner_end(ln)), 0)
    return lines[start:]


def show_diff(a, b, what):
    d = list(difflib.unified_diff(a, b, "oracle", "engine", lineterm="", n=2))
    print(f"{what}: DIFFERENT")
    print("\n".join(d[:60]))


def fresh(d):
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d)
    return d


def write18(a):
    results = {}
    for side, prog in (("oracle", a.oracle), ("engine", a.engine)):
        d = fresh(os.path.join(a.work, "write18-" + side))
        shutil.copy(os.path.join(HERE, "write18.tex"), d)
        rc, out, err = run(prog, d, ["-ini", "-shell-restricted", "-interaction=nonstopmode",
                                     "write18.tex"], env_for(side == "engine", a.pool))
        log_path = os.path.join(d, "write18.log")
        if not os.path.isfile(log_path):
            print(f"{side}: no log (exit {rc})\n{out}\n{err}")
            return 2
        with open(log_path, encoding="latin-1") as f:
            log = after_banner(f.read())
        print(f"--- {side} (exit {rc}) log after the banner:")
        print("\n".join(log))
        if err.strip():
            print(f"--- {side} stderr:\n{err}")
        # Not compared: in a pipe pdfTeX's terminal is fully buffered, so
        # kpsewhich's line comes before it, where the engine's is in order.
        print(f"--- {side} terminal:\n{out}")
        results[side] = (rc, log)
    status = 0
    if results["oracle"][1] != results["engine"][1]:
        show_diff(results["oracle"][1], results["engine"][1], "write18 log")
        status = 1
    if results["oracle"][0] != results["engine"][0]:
        print(f"exit codes differ: oracle {results['oracle'][0]}, engine {results['engine'][0]}")
        status = 1
    joined = " ".join(results["engine"][1])
    for want in ("runsystem(kpsewhich -var-value=TEXMFROOT)...executed safely (allowed).",):
        if want not in joined:
            print(f"engine log lacks {want!r}")
            status = 1
    # Windows only: elsewhere `'` is a quotation error (both programs agree).
    if os.name == "nt" and ("[pipe: closed]" in joined or "plain.tex]" not in "".join(results["engine"][1])):
        print("engine: the single-quoted pipe did not run")
        status = 1

    # Full \write18 through cmd.exe: a redirection the restricted quoting
    # would have turned into an argument.
    d = fresh(os.path.join(a.work, "write18-full"))
    with open(os.path.join(d, "full.tex"), "w") as f:
        f.write("\\catcode`\\{=1 \\catcode`\\}=2\n\\immediate\\write18{echo full shell escape> w18.txt}\n\\end\n")
    rc, out, err = run(a.engine, d, ["-ini", "-shell-escape", "-interaction=nonstopmode", "full.tex"],
                       env_for(True, a.pool))
    got = open(os.path.join(d, "w18.txt"), encoding="latin-1").read() if os.path.isfile(
        os.path.join(d, "w18.txt")) else None
    print(f"full \\write18 (exit {rc}): w18.txt = {got!r}")
    if got is None or got.strip() != "full shell escape":
        print(out, err)
        status = 1
    print("write18:", "EQUAL" if status == 0 else "FAILED")
    return status


def pt1(a):
    caps = {}
    for side, prog in (("oracle", a.oracle), ("engine", a.engine)):
        d = fresh(os.path.join(a.work, "pt1-" + side))
        env = env_for(side == "engine", a.pool, {"FLASHTEX_FORMATS": d} if side == "engine" else None)
        rc, out, err = run(prog, d, ["-ini", "-jobname=pdftex", "-progname=pdftex", "-etex",
                                     "-translate-file=cp227.tcx", "pdfetex.ini"], env)
        if not os.path.isfile(os.path.join(d, "pdftex.fmt")):
            print(f"{side}: no pdftex.fmt (exit {rc})\n{out[-3000:]}\n{err[-3000:]}")
            return 2
        shutil.copy(os.path.join(HERE, "pt1.tex"), d)
        args = ["-fmt=pdftex", "-interaction=nonstopmode", "-halt-on-error", "-jobname=pt1",
                capture.first_line("pt1.tex", trace=True)]
        rc, out, err = run(prog, d, args, env)
        log_path = os.path.join(d, "pt1.log")
        if not os.path.isfile(log_path) or not os.path.isfile(os.path.join(d, "pt1.pdf")):
            print(f"{side}: no log or PDF (exit {rc})\n{out[-3000:]}\n{err[-3000:]}")
            return 2
        with open(log_path, "rb") as f:
            text = f.read().decode("latin-1").replace("\r\n", "\n")
        norm = capture.normalise_log(text, d)
        strict, accounting = capture.split_accounting(norm)
        boxes = capture.split_boxes(norm)
        caps[side] = (rc, strict, boxes, accounting)
        print(f"{side}: exit {rc}, {len(text)} log bytes, {len(boxes)} shipouts, "
              f"{len(accounting)} accounting lines")
    o, e = caps["oracle"], caps["engine"]
    status = 0
    if o[2] != e[2]:
        show_diff("\n\n".join(o[2]).split("\n"), "\n\n".join(e[2]).split("\n"), "P-T1 box dumps")
        status = 1
    if o[1] != e[1]:
        show_diff(o[1].split("\n"), e[1].split("\n"), "P-T1 strict log")
        status = 1
    if o[0] != e[0]:
        print(f"exit codes differ: oracle {o[0]}, engine {e[0]}")
        status = 1
    if not o[2]:
        print("no shipout in the oracle's log")
        status = 1
    print("accounting (non-gating):", "equal" if o[3] == e[3] else "differs")
    print("P-T1:", "PASS" if status == 0 else "FAIL")
    return status


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("what", choices=["write18", "pt1"])
    p.add_argument("--oracle", required=True, help="TeX Live's pdftex.exe")
    p.add_argument("--engine", required=True, help="flashtex-initex.exe")
    p.add_argument("--pool", required=True, help="pdftex.pool")
    p.add_argument("--work", required=True)
    a = p.parse_args()
    # Absolute: each program runs in its own work directory.
    a.oracle, a.engine, a.pool, a.work = map(os.path.abspath, (a.oracle, a.engine, a.pool, a.work))
    os.makedirs(a.work, exist_ok=True)
    sys.exit(write18(a) if a.what == "write18" else pt1(a))


if __name__ == "__main__":
    main()
