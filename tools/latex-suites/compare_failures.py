#!/usr/bin/env python3
"""Compare two `run.py` transcripts: the reference engine's and a candidate's.

Exit 0 iff the candidate fails no test that the reference passes on the same
TeX Live. This is the T2 gate where the installed TeX Live is newer than the
checkouts `PINS.txt` names, so that `EXPECTED-FAILURES.txt` (baselined on the
pinned TeX Live) does not describe it: the reference run is the baseline.

Usage: compare_failures.py <reference transcript> <candidate transcript>
"""
import re
import sys

LINE = re.compile(r"^  (\S+) \[(UNEXPECTED|expected)")


def failing(path):
    """The names under `failing tests:` in a run.py transcript."""
    out, on = set(), False
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            if line.startswith("failing tests:"):
                on = True
                continue
            m = LINE.match(line)
            if on and m:
                out.add(m.group(1))
            elif on and not line.startswith("  "):
                on = False
    return out


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ref, cand = failing(argv[1]), failing(argv[2])
    extra = sorted(cand - ref)
    print(f"T2: reference fails {len(ref)}, candidate fails {len(cand)}; "
          f"candidate-only failures: {', '.join(extra) or 'none'}")
    return 1 if extra else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
