#!/usr/bin/env python3
"""Delta-debugging minimiser for fuzz findings. MIT licensed.

Takes a stored fuzz case and shrinks it with ddmin, first by line then by
token, keeping a reduction only if it reproduces the SAME classification
(see tools/fuzz/run.py). For `candidate-crash` the panic location (text
after "panicked at" up to the first colon-number pair, when present) must
also match, so a minimized crash is the same bug, not just any crash.

Classification is reused from run.py (run_one); nothing is duplicated here.
Stdlib only.
"""
import argparse
import os
import re
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import run as fuzz_run

TARGETS = ("diverge", "candidate-crash")


def split_tokens(text):
    """Split into non-space/space runs; join(tokens) == text."""
    return re.findall(r"\S+|\s+", text)


def ddmin(atoms, test):
    """Minimize a list of atoms keeping test() true (Zeller's ddmin).

    test(full_list) must already be true. Returns the reduced list.
    """
    atoms = list(atoms)
    n = 2
    while len(atoms) >= 2:
        n = min(n, len(atoms))
        size = -(-len(atoms) // n)
        idx = [list(range(i, min(i + size, len(atoms))))
               for i in range(0, len(atoms), size)]
        if len(idx) < 2:
            break
        reduced = False
        for chunk in idx:
            drop = set(chunk)
            rest = [a for i, a in enumerate(atoms) if i not in drop]
            if rest and test(rest):
                atoms = rest
                n = 2
                reduced = True
                break
        if reduced:
            continue
        for chunk in idx:
            sub = [atoms[i] for i in chunk]
            if len(sub) < len(atoms) and test(sub):
                atoms = sub
                n = 2
                reduced = True
                break
        if reduced:
            continue
        if n >= len(atoms):
            break
        n = min(len(atoms), 2 * n)
    return atoms


def minimize(text, candidate, oracle, timeout, target):
    """Shrink text while it still classifies as target.

    Returns (minimized_text, engine_runs); each run_one call counts as 2
    engine runs (candidate + oracle). Raises ValueError if the input does
    not reproduce target in the first place.
    """
    runs = [0]

    def check(t):
        res = fuzz_run.run_one(t, candidate, oracle, timeout,
                               return_logs=True)
        runs[0] += 2
        if res[0] != target:
            return False
        if target == "candidate-crash" and orig_loc is not None:
            return fuzz_run.panic_location(res[4]) == orig_loc
        return True

    first = fuzz_run.run_one(text, candidate, oracle, timeout,
                             return_logs=True)
    runs[0] += 2
    if first[0] != target:
        raise ValueError("input classifies as %s, not %s"
                         % (first[0], target))
    orig_loc = (fuzz_run.panic_location(first[4])
                if target == "candidate-crash" else None)

    lines = text.splitlines(keepends=True) or [text]
    if len(lines) >= 2:
        lines = ddmin(lines, lambda part: check("".join(part)))
    current = "".join(lines)
    tokens = split_tokens(current)
    if len(tokens) > len(lines):
        tokens = ddmin(tokens, lambda part: check("".join(part)))
        current = "".join(tokens)
    return current, runs[0]


def main(argv=None):
    ap = argparse.ArgumentParser(description="minimize a fuzz finding")
    ap.add_argument("--candidate", required=True, help="candidate engine")
    ap.add_argument("--oracle", required=True, help="oracle engine")
    ap.add_argument("--input", required=True, help="failing .tex case")
    ap.add_argument("--class", dest="target_class", required=True,
                    choices=TARGETS, help="class to preserve")
    ap.add_argument("--out", required=True, help="minimized .tex output")
    ap.add_argument("--timeout", type=float, required=True,
                    help="per-engine timeout in seconds")
    args = ap.parse_args(argv)
    for label, binary in (("candidate", args.candidate),
                          ("oracle", args.oracle)):
        if not (os.path.isfile(binary) or shutil.which(binary)):
            print("error: %s not found: %s" % (label, binary),
                  file=sys.stderr)
            return 2
    try:
        with open(args.input) as fh:
            text = fh.read()
    except OSError as exc:
        print("error: cannot read --input: %s" % exc, file=sys.stderr)
        return 2
    try:
        current, runs = minimize(text, args.candidate, args.oracle,
                                 args.timeout, args.target_class)
    except ValueError as exc:
        print("error: %s" % exc, file=sys.stderr)
        return 1
    with open(args.out, "w") as fh:
        fh.write(current)
    print("minimize: %d engine runs, %d -> %d bytes (%s kept)"
          % (runs, len(text), len(current), args.target_class))
    return 0


if __name__ == "__main__":
    sys.exit(main())
