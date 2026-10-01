#!/usr/bin/env python3
"""Delta-debugging minimiser for fuzz findings. MIT licensed.

Takes a stored fuzz case and shrinks it with ddmin, first by line then by
token, keeping a reduction only if it reproduces the SAME finding (see
tools/fuzz/run.py), not just its class: for `diverge` the raw
(candidate line, oracle line) pair at the first difference must be
byte-identical to the original pair (strict, the default), so a decoy
divergence of the same digit-masked shape cannot replace the real one;
`--loose` falls back to the digit-masked comparison. For crashes the
stderr-based crash signature must be unchanged, so a minimized case is
the same bug, not just any divergence or crash.

Classification and signatures are reused from run.py; nothing about them
is duplicated here. Stdlib only.
"""
import argparse
import os
import re
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import run as fuzz_run

TARGETS = ("diverge", "candidate-crash", "oracle-crash")


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


def first_diff_pair(cand_log, orc_log):
    """Raw (candidate line, oracle line) at the first compared difference.

    Shared with run.first_diff_pair (harness normalisation only, digits
    intact), so a decoy divergence of the same digit-masked shape does not
    match. The compared-log line number is deliberately excluded: a
    reduction may shift the difference to an earlier line while keeping
    the same pair. Returns None when the compared logs are identical (a
    returncode-only divergence); the caller then falls back to the exact
    diff string.
    """
    return fuzz_run.first_diff_pair(cand_log, orc_log)


def diff_position(cand_rc, cand_log, orc_rc, orc_log):
    """Position of the first difference for the minimiser report.

    Returns "returncode candidate=X oracle=Y" when the return codes
    differ, else "box B line O" (1-based shipped-box number, 0-based line
    offset from the box's Completed-box SHIPOUT_LINE, same anchored rule
    as lockstep split_boxes) when the first differing compared line sits
    at or after the first shipped box, else "line N" (1-based
    compared-log line number) outside any box.
    """
    if cand_rc != orc_rc:
        return "returncode candidate=%s oracle=%s" % (cand_rc, orc_rc)
    ship = fuzz_run.lockstep_run.SHIPOUT_LINE
    a = fuzz_run.lockstep_run.compared_lines(cand_log or "")
    b = fuzz_run.lockstep_run.compared_lines(orc_log or "")
    n = max(len(a), len(b))
    for i in range(n):
        x = a[i] if i < len(a) else "<EOF>"
        y = b[i] if i < len(b) else "<EOF>"
        if x != y:
            starts = [j for j, ln in enumerate(a)
                      if ln.startswith(ship)]
            if starts and i >= starts[0]:
                box = sum(1 for s in starts if s <= i)
                return "box %d line %d" % (box, i - starts[box - 1])
            return "line %d" % (i + 1)
    return "line ?"


def position_warn(orig_pos, min_pos):
    """True when the position moved more than line/token deletion explains.

    Deleting input lines or tokens can only move the surviving difference
    earlier (and shift its offset inside the same box), so a warning fires
    when the kind changes (box vs plain line vs returncode), the
    shipped-box number changes, or a plain line number moves later.
    """
    o = orig_pos.split()
    m = min_pos.split()
    if not o or not m or o[0] != m[0]:
        return True
    if o[0] == "box":
        return len(o) < 2 or len(m) < 2 or o[1] != m[1]
    if o[0] == "line":
        try:
            return int(m[1]) > int(o[1])
        except (IndexError, ValueError):
            return orig_pos != min_pos
    return orig_pos != min_pos


def minimize(text, candidate, oracle, timeout, target, loose=False):
    """Shrink text while it still reproduces the same finding.

    Strict (default): a `diverge` reduction is kept only if the raw
    (candidate line, oracle line) pair at the first difference is
    byte-identical to the original pair. With loose=True the digit-masked
    normalised diff must match instead. Crash targets always require the
    identical stderr-based signature. Returns (minimized_text,
    engine_runs, orig_pos, min_pos, warned); each run_one call counts as
    2 engine runs (candidate + oracle), each crash-signature re-run as 1
    (a diverge check costs one extra oracle re-run inside run_one, for
    its reference-determinism guard, not counted here).
    Raises ValueError if the input does not reproduce target.
    """
    runs = [0]

    def crash_sig(res, t):
        # Stderr-based crash signature for a run_one return_logs tuple;
        # each direct re-run counts as one extra engine run.
        cls, cand_rc, orc_rc, _diff, cand_log, orc_log = res[:6]
        if target == "candidate-crash":
            err = fuzz_run.crash_stderr(t, candidate,
                                        fuzz_run.candidate_env(), timeout)
            runs[0] += 1
            return fuzz_run.crash_signature(cand_rc, cand_log, err)
        err = fuzz_run.crash_stderr(t, oracle, None, timeout)
        runs[0] += 1
        return fuzz_run.crash_signature(orc_rc, orc_log, err)

    first = fuzz_run.run_one(text, candidate, oracle, timeout,
                             return_logs=True)
    runs[0] += 2
    if first[0] != target:
        raise ValueError("input classifies as %s, not %s"
                         % (first[0], target))
    cls, cand_rc, orc_rc, diff, cand_log, orc_log = first[:6]
    orig_pos = diff_position(cand_rc, cand_log, orc_rc, orc_log)
    if target == "diverge":
        if loose:
            orig_mark = fuzz_run.normalised_diff(diff)
            orig_pair = None
        else:
            orig_mark = None
            orig_pair = first_diff_pair(cand_log, orc_log)
        orig_diff = diff
    elif target in ("candidate-crash", "oracle-crash"):
        orig_mark = crash_sig(first, text)
        orig_pair = None
        orig_diff = None
    else:
        orig_mark = None
        orig_pair = None
        orig_diff = None

    def check(t):
        res = fuzz_run.run_one(t, candidate, oracle, timeout,
                               return_logs=True)
        runs[0] += 2
        if res[0] != target:
            return False
        if target == "diverge":
            if loose:
                return (fuzz_run.normalised_diff(res[3]) == orig_mark)
            if orig_pair is None:
                return res[3] == orig_diff
            return first_diff_pair(res[4], res[5]) == orig_pair
        if target in ("candidate-crash", "oracle-crash"):
            return crash_sig(res, t) == orig_mark
        return True

    lines = text.splitlines(keepends=True) or [text]
    if len(lines) >= 2:
        lines = ddmin(lines, lambda part: check("".join(part)))
    current = "".join(lines)
    tokens = split_tokens(current)
    if len(tokens) > len(lines):
        tokens = ddmin(tokens, lambda part: check("".join(part)))
        current = "".join(tokens)
    # One final verification run for the minimised case's position (its
    # logs are not kept: only the small position string is retained).
    final = fuzz_run.run_one(current, candidate, oracle, timeout,
                             return_logs=True)
    runs[0] += 2
    min_pos = diff_position(final[1], final[4], final[2], final[5])
    warned = target == "diverge" and position_warn(orig_pos, min_pos)
    return current, runs[0], orig_pos, min_pos, warned


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
    ap.add_argument("--loose", action="store_true",
                    help="diverge: match the digit-masked first differing "
                    "line instead of the raw pair (may minimise across "
                    "changing numbers)")
    args = ap.parse_args(argv)
    fuzz_run.apply_fsize_limit()
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
        current, runs, orig_pos, min_pos, warned = minimize(
            text, args.candidate, args.oracle, args.timeout,
            args.target_class, loose=args.loose)
    except ValueError as exc:
        print("error: %s" % exc, file=sys.stderr)
        return 1
    with open(args.out, "w") as fh:
        fh.write(current)
    print("mode: %s" % ("loose" if args.loose else "strict"))
    print("orig first difference at %s" % orig_pos)
    print("minimized first difference at %s" % min_pos)
    if warned:
        print("warning: first-difference position moved from %s to %s, "
              "more than line/token deletion explains"
              % (orig_pos, min_pos))
    print("minimize: %d engine runs, %d -> %d bytes (%s kept)"
          % (runs, len(text), len(current), args.target_class))
    return 0


if __name__ == "__main__":
    sys.exit(main())
