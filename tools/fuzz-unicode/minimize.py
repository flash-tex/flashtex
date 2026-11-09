#!/usr/bin/env python3
"""Delta-debug a fuzz.py finding to a short document with the same class. MIT.

    python3 tools/fuzz-unicode/minimize.py --candidate BIN OUT/<class>/<seed> [--out min.tex]

ddmin over the document's lines (every generated line is balanced on its
own, so removing one never splits a group), then over the space-separated
words of each remaining line (a candidate whose braces do not balance is
skipped). A candidate counts when both engines still differ in the same
class (log, xdv, ...) and, for a log difference, when the first differing
oracle line still matches the original one with digits masked. The last
line (`\\end`) is always kept.
"""
import argparse
import json
import os
import re
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fuzz  # noqa: E402

DIGITS = re.compile(r"\d+")


def balanced(text):
    depth = 0
    for ch in text:
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth < 0:
                return False
    return depth == 0


class Oracle:
    def __init__(self, candidate, oracle, timeout, want_cls, want_key, tmp):
        self.candidate, self.oracle, self.timeout = candidate, oracle, timeout
        self.want_cls, self.want_key, self.tmp = want_cls, want_key, tmp
        self.checks = 0

    def key(self, res):
        if res["cls"] == "log":
            return DIGITS.sub("#", res["detail"]["oracle"])
        if res["cls"] == "pdf":
            return res.get("sig")
        return None

    def test(self, lines):
        text = "\n".join(lines) + "\n"
        if not balanced(text):
            return False
        self.checks += 1
        work = tempfile.mkdtemp(prefix="fzu-min-", dir=self.tmp)
        try:
            res = fuzz.compare(text, self.candidate, self.oracle, work, self.timeout,
                               pdf=self.want_cls == "pdf")
        finally:
            shutil.rmtree(work, ignore_errors=True)
        if res["cls"] != self.want_cls:
            return False
        return self.want_key is None or self.key(res) == self.want_key


def ddmin(items, test, keep_last=True):
    """Classic ddmin over a list; test(list) -> bool (still failing)."""
    fixed = [items[-1]] if keep_last else []
    cur = items[:-1] if keep_last else items[:]
    n = 2
    while len(cur) >= 1:
        chunk = max(1, len(cur) // n)
        reduced = False
        for i in range(0, len(cur), chunk):
            cand = cur[:i] + cur[i + chunk:]
            if test(cand + fixed):
                cur = cand
                n = max(n - 1, 2)
                reduced = True
                break
        if not reduced:
            if chunk == 1:
                break
            n = min(len(cur), n * 2)
    return cur + fixed


def minimize(lines, o):
    lines = ddmin(lines, o.test)
    # Words of each line.
    for idx in range(len(lines) - 1):
        words = lines[idx].split(" ")
        if len(words) < 2:
            continue

        def t(ws, idx=idx):
            return o.test(lines[:idx] + [" ".join(ws)] + lines[idx + 1:])
        words = ddmin(words, t, keep_last=False)
        lines[idx] = " ".join(words)
    return lines


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--oracle", default="xetex")
    ap.add_argument("--timeout", type=float, default=60)
    ap.add_argument("--tmp", default=None)
    ap.add_argument("--out", default=None)
    ap.add_argument("finding")
    args = ap.parse_args(argv)
    args.oracle = shutil.which(args.oracle) or args.oracle
    args.candidate = os.path.abspath(args.candidate)
    with open(os.path.join(args.finding, "result.json")) as fh:
        r = json.load(fh)
    with open(os.path.join(args.finding, "doc.tex"), encoding="utf-8") as fh:
        lines = fh.read().rstrip("\n").split("\n")
    o = Oracle(args.candidate, args.oracle, args.timeout, r["cls"], None, args.tmp)
    if r["cls"] == "log":
        o.want_key = DIGITS.sub("#", r["detail"]["oracle"])
    elif r["cls"] == "pdf":
        # The same kinds of PDF difference (run under a Python with fontTools).
        o.want_key = r.get("sig")
    if not o.test(lines):
        print("the finding does not reproduce", file=sys.stderr)
        return 2
    out = minimize(lines, o)
    text = "\n".join(out) + "\n"
    dest = args.out or os.path.join(args.finding, "min.tex")
    with open(dest, "w", encoding="utf-8") as fh:
        fh.write(text)
    sys.stdout.write(text)
    print("%% %d lines, %d checks -> %s" % (len(out), o.checks, dest), file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
