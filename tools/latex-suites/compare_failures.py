#!/usr/bin/env python3
"""Compare two `run.py` transcripts: the reference engine's and a candidate's.

Exit 0 iff the candidate fails no test that the reference passes on the same
TeX Live. This is the T2 gate where the installed TeX Live is newer than the
checkouts `PINS.txt` names, so that `EXPECTED-FAILURES.txt` (baselined on the
pinned TeX Live) does not describe it: the reference run is the baseline.

Failures are compared as (directory label, test) pairs from run.py's
per-directory `LABEL: FAILED t1 t2 ...` lines, parsed by
tools/parity/scoreboard.py: one test name can run in two directories
(l3kernel's testfiles-backend under etex-dvips and etex-dvisvgm), and the
reference failing it under one while the candidate fails it under the other
is a candidate-only failure. A transcript with failures but no label lines
(run.py before #1299) falls back to bare names, with a warning; the other
transcript is then also compared by bare name. Exit 2 if a transcript's
label lines do not account for its FAIL counts.

Usage: compare_failures.py <reference transcript> <candidate transcript>
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                "..", "parity"))

from scoreboard import (FAILED_DIR_LINE, _suite_dirs,  # noqa: E402
                        failing_tests, suite_failures)


def failures(path):
    """(pairs, names, problem) for a run.py transcript: the failing
    (label, test) pairs, or None when it has failures but no label lines;
    the bare names under `failing tests:`; and why the label lines cannot
    be trusted (or None)."""
    with open(path, encoding="utf-8", errors="replace") as f:
        text = f.read()
    names = failing_tests(text)
    if names and not any(FAILED_DIR_LINE.match(line)
                         for line in text.splitlines()):
        return None, names, None
    pairs, problem = suite_failures(text, _suite_dirs(text))
    return pairs, names, problem


def show(key):
    return key if isinstance(key, str) else "%s (%s)" % (key[1], key[0])


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    (rpairs, rnames, rprob), (cpairs, cnames, cprob) = \
        failures(argv[1]), failures(argv[2])
    for path, prob in ((argv[1], rprob), (argv[2], cprob)):
        if prob:
            print("T2: %s: %s" % (path, prob), file=sys.stderr)
            return 2
    if rpairs is None or cpairs is None:
        for path, pairs in ((argv[1], rpairs), (argv[2], cpairs)):
            if pairs is None:
                print("warning: %s has no per-directory `LABEL: FAILED` lines;"
                      " comparing bare test names, which cannot tell"
                      " testfiles-backend's dvips and dvisvgm runs apart"
                      % path, file=sys.stderr)
        ref, cand = rnames, cnames
    else:
        ref, cand = rpairs, cpairs
    extra = sorted(cand - ref, key=show)
    print(f"T2: reference fails {len(ref)}, candidate fails {len(cand)}; "
          f"candidate-only failures: {', '.join(map(show, extra)) or 'none'}")
    return 1 if extra else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
