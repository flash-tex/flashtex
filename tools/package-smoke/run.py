#!/usr/bin/env python3
"""Compare the candidate engine with pdfTeX on the package-smoke documents. MIT.

For every tools/package-smoke/*.tex, run it through the lockstep capture()
(format pdflatex) on the candidate and on the reference, in separate directories,
and report equal / DIFFERENT / one-side failure. Standard library only.

  FLASHTEX_POOL=... FLASHTEX_FORMATS=... \
  python3 tools/package-smoke/run.py --candidate BIN [--reference pdftex] [PKG ...]

Exit status: 0 when every document is equal, 1 otherwise.
"""
import argparse
import importlib.util
import os
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "lockstep_run", os.path.join(HERE, "..", "lockstep", "run.py"))
lockstep = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(lockstep)

CANDIDATE_ENV = ("FLASHTEX_POOL", "FLASHTEX_FORMATS")


def run_engine(tex, binary, extra):
    work = tempfile.mkdtemp(prefix="pkgsmoke-")
    try:
        path = os.path.join(work, "doc.tex")
        shutil.copy(tex, path)
        cap = lockstep.capture(path, binary, work, fmt="pdflatex", extra_env=extra)
        return cap.returncode, lockstep.compared_lines(cap.log)
    finally:
        shutil.rmtree(work, ignore_errors=True)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--candidate", required=True, help="candidate engine binary")
    ap.add_argument("--reference", default="/Library/TeX/texbin/pdftex",
                    help="reference pdfTeX")
    ap.add_argument("packages", nargs="*", help="package names (default: all)")
    args = ap.parse_args(argv)
    cand_env = {k: os.environ[k] for k in CANDIDATE_ENV if k in os.environ} or None
    names = sorted(f[:-4] for f in os.listdir(HERE) if f.endswith(".tex"))
    if args.packages:
        names = [n for n in names if n in args.packages]
    bad = 0
    for name in names:
        tex = os.path.join(HERE, name + ".tex")
        crc, clines = run_engine(tex, args.candidate, cand_env)
        rrc, rlines = run_engine(tex, args.reference, None)
        if crc == rrc and clines == rlines:
            status = "equal"
        else:
            bad += 1
            first = next((i for i, (a, b) in enumerate(zip(clines, rlines)) if a != b), None)
            status = "DIFFERENT (exit candidate=%s reference=%s%s)" % (
                crc, rrc, "" if first is None else ", first differing line %d" % (first + 1))
        print("%-16s %s" % (name, status), flush=True)
    print("%d documents, %d differ" % (len(names), bad))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
