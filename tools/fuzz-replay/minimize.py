#!/usr/bin/env python3
"""Delta-debug a replay finding to a minimal repro (README.md).

    minimize.py --engine BIN [--sub bm|d9] CASE_DIR OUT_DIR

CASE_DIR holds t.tex (and u.tex, and any .tfm files). The finding's
signature (fuzz_replay.signature) is computed first; every candidate must
keep exactly that signature. The passes are: ddmin over the lines of t.tex
and of u.tex, then ddmin over the tokens of each line of t.tex (a candidate
whose braces do not balance is skipped without a run), repeated until
nothing changes.

Stdlib only.
"""
import argparse
import hashlib
import os
import re
import shutil
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import fuzz_replay as fr  # noqa: E402

TOKEN = re.compile(r"\\[A-Za-z]+ ?|\\.|.", re.S)


def ddmin(items, test):
    """Zeller's ddmin: a 1-minimal sublist of `items` for which `test` holds
    (`test(items)` is assumed)."""
    n = 2
    while len(items) >= 2:
        size = max(1, len(items) // n)
        chunks = [items[i:i + size] for i in range(0, len(items), size)]
        done = False
        for i in range(len(chunks)):
            comp = [x for j, c in enumerate(chunks) if j != i for x in c]
            if comp and test(comp):
                items, n, done = comp, max(n - 1, 2), True
                break
        if not done:
            for c in chunks:
                if len(c) < len(items) and test(c):
                    items, n, done = c, 2, True
                    break
        if not done:
            if size == 1:
                break
            n = min(len(items), n * 2)
    if len(items) == 1 and test([]):
        return []
    return items


def balanced(text):
    depth = 0
    for t in TOKEN.findall(text):
        if t == "{":
            depth += 1
        elif t == "}":
            depth -= 1
            if depth < 0:
                return False
    return depth == 0


class Budget(Exception):
    """The check budget is spent: the passes stop with the smallest case so far."""


class Minimizer:
    def __init__(self, engine, pool, case_dir, sub=None, max_checks=2000, max_seconds=600):
        self.engine, self.pool = engine, pool
        self.max_checks, self.deadline = max_checks, time.monotonic() + max_seconds
        self.cache = {}
        self.files = {}
        self.extra = []
        for n in sorted(os.listdir(case_dir)):
            p = os.path.join(case_dir, n)
            if n.endswith(".tex"):
                with open(p) as f:
                    self.files[n] = f.read()
            elif n.endswith(".tfm"):
                self.extra.append(p)
        self.work = tempfile.mkdtemp(prefix="fr-min-")
        self.runs = 0
        res = self.check(self.files, subs=(sub,) if sub else ("bm", "d9"))
        bad = [(s, r) for s, r in res.items() if r[0]]
        if not bad:
            raise SystemExit("the case is clean: nothing to minimise")
        self.sub, (kinds, det, _) = bad[0]
        self.sig = fr.signature(self.sub, kinds, det)

    def check(self, files, subs=None):
        d = os.path.join(self.work, f"c{self.runs}")
        self.runs += 1
        fr.write_case(d, files, self.extra)
        try:
            return fr.check_dir(self.engine, self.pool, d, subs or (self.sub,))
        finally:
            shutil.rmtree(d, ignore_errors=True)

    def holds(self, files):
        if not balanced(files.get("t.tex", "")):
            return False
        key = hashlib.sha1(repr(sorted(files.items())).encode()).hexdigest()
        if key in self.cache:
            return self.cache[key]
        if self.runs >= self.max_checks or time.monotonic() > self.deadline:
            raise Budget()
        kinds, det, _ = self.check(files)[self.sub]
        ok = bool(kinds) and fr.signature(self.sub, kinds, det) == self.sig
        self.cache[key] = ok
        if ok:
            self.best = dict(files)
        return ok

    def lines_pass(self, name):
        if name not in self.files:
            return False
        lines = self.files[name].split("\n")

        def t(ls):
            return self.holds({**self.files, name: "\n".join(ls)})
        new = ddmin(lines, t)
        changed = new != lines
        self.files[name] = "\n".join(new)
        return changed

    def tokens_pass(self):
        changed = False
        lines = self.files["t.tex"].split("\n")
        for i in range(len(lines)):
            toks = TOKEN.findall(lines[i])
            if len(toks) < 2:
                continue

            def t(ts):
                ls = lines[:i] + ["".join(ts)] + lines[i + 1:]
                return self.holds({**self.files, "t.tex": "\n".join(ls)})
            new = ddmin(toks, t)
            if new != toks:
                lines[i] = "".join(new)
                changed = True
        self.files["t.tex"] = "\n".join(lines)
        return changed

    def run(self):
        self.best = dict(self.files)
        try:
            while True:
                c = self.lines_pass("t.tex")
                c |= self.lines_pass("u.tex")
                c |= self.tokens_pass()
                if not c:
                    break
        except Budget:
            # the smallest case that kept the signature
            self.files = min((self.best, self.files),
                             key=lambda f: sum(len(v) for v in f.values()))
            print(f"budget spent after {self.runs} checks: the case is not 1-minimal",
                  flush=True)
            return self.files
        # drop u.tex when the repro no longer needs it
        if "u.tex" in self.files:
            without = {k: v for k, v in self.files.items() if k != "u.tex"}
            if self.holds(without):
                self.files = without
        return self.files

    def close(self):
        shutil.rmtree(self.work, ignore_errors=True)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--engine", required=True)
    ap.add_argument("--pool", default=fr.DEFAULT_POOL)
    ap.add_argument("--sub", choices=("bm", "d9"))
    ap.add_argument("--max-checks", type=int, default=2000,
                    help="stop after this many checks (each is three runs)")
    ap.add_argument("--max-seconds", type=int, default=600)
    ap.add_argument("case")
    ap.add_argument("out")
    a = ap.parse_args(argv)
    m = Minimizer(a.engine, a.pool, a.case, a.sub, a.max_checks, a.max_seconds)
    try:
        print(f"signature {m.sig}", flush=True)
        files = m.run()
    finally:
        m.close()
    fr.write_case(a.out, files, m.extra)
    print(f"{m.runs} checks")
    for n, t in files.items():
        print(f"%%%% {n}\n{t}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
