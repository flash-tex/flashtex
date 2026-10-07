#!/usr/bin/env python3
"""The app-parity row -> test check (retirement plan #1236 §4.4, S4(h)).

`app-parity-rows.json` maps each row of the checklist of record
(docs/evidence/app-parity-2026-10-05/README.md) to the tests that prove it, and
says what each row needs before S5 (the flip):

  s5 = "test"      done, with named tests that pass and are not skipped
       "fallback"  covered at S5 by a visible fallback to the previous engine (A9, A17)
       "owner"     "different (intended)": needs the owner's retirement in writing
                   (`owner_retirement`, a link), or a move to "test"
       "s6" / "s7" not gated at S5 (old-route rows; the CLI)

A test is `Suite.testName` (XCTest, apps/mac/Tests), `Suite` (every test of that
suite), or `rust:<file>::<fn>`. A Rust row test that can return early (skip) must
honour FLASHTEX_REQUIRE_TEXLIVE=1, which turns the skip into a failure, and the legs
that run it set it (`check` enforces the first). `host_driven` lists the Swift tests that need a
real flashtex-host and TeX Live: they skip on the hosted mac-app job and must run
on the host leg instead.

Commands:
  check                    the file is well formed, its rows are the checklist's,
                           and every named test exists in the source (CI: checks job)
  gate S5|S6 [--log F]...  the stage's app-parity state; with swift test logs, also
                           whether every gating Swift test passed, not skipped
  check-log --leg hosted|host LOG
                           CI: every gating Swift test of that leg passed in LOG
                           (hosted: all but host_driven; host: host_driven)
"""

import argparse
import glob
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
ROWS_FILE = os.path.join(HERE, "app-parity-rows.json")
SCHEMA = "flashtex-app-parity-rows/1"
S5_CLASSES = ("test", "fallback", "owner", "s6", "s7")
STATUSES = ("done", "partial", "missing", "different")
SWIFT_TEST_DIRS = ("apps/mac/Tests",)
ROW_RE = re.compile(r"^\| ([A-E]\d+) \| ([^|]*) \| ([^|]*) \| (.*) \|$")
CASE_RE = re.compile(r"Test Case '-\[(?:\w+)\.(\w+) (\w+)\]' (passed|skipped|failed)")


def load(path=ROWS_FILE):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def checklist_rows(doc, root=ROOT):
    """Row id -> status word, from the checklist of record's gap tables."""
    out = {}
    with open(os.path.join(root, doc["checklist"]), encoding="utf-8") as f:
        for line in f:
            m = ROW_RE.match(line.strip())
            if m:
                out[m.group(1)] = re.sub(r"\*", "", m.group(3)).strip().split(" ")[0].rstrip(";")
    return out


def gating_rows(doc, stage):
    """The rows whose tests gate `stage` (S5: s5 == test or fallback; S6: every row with tests)."""
    if stage == "S5":
        return [r for r in doc["rows"] if r["s5"] in ("test", "fallback")]
    return [r for r in doc["rows"] if r["s5"] != "s7"]


def swift_tests(rows):
    return sorted({t for r in rows for t in r["tests"] if not t.startswith("rust:")})


# --------------------------------------------------------------------------
# source


def swift_index(root=ROOT):
    """Suite -> set of test function names, over every Swift test file."""
    decl = {}
    for d in SWIFT_TEST_DIRS:
        for path in glob.glob(os.path.join(root, d, "**", "*.swift"), recursive=True):
            with open(path, encoding="utf-8", errors="replace") as f:
                text = f.read()
            suites = set(re.findall(r"^\s*(?:final\s+|@\w+\s+)*(?:class|extension)\s+(\w+Tests)\b", text, re.M))
            funcs = set(re.findall(r"func\s+(test\w+)\s*\(", text))
            for s in suites:
                decl.setdefault(s, set()).update(funcs)
    return decl


def missing_tests(doc, root=ROOT):
    idx = swift_index(root)
    out = []
    for r in doc["rows"]:
        for t in r["tests"]:
            if t.startswith("rust:"):
                path, _, fn = t[5:].partition("::")
                full = os.path.join(root, path)
                ok = False
                if os.path.isfile(full):
                    with open(full, encoding="utf-8") as f:
                        ok = bool(re.search(r"\bfn\s+%s\s*\(" % re.escape(fn), f.read()))
            elif "." in t:
                suite, name = t.split(".", 1)
                ok = name in idx.get(suite, ())
            else:
                ok = t in idx
            if not ok:
                out.append("%s: %s" % (r["id"], t))
    return out


REQUIRE_ENV = "FLASHTEX_REQUIRE_TEXLIVE"


def rust_fn_body(text, fn):
    """The body of `fn <fn>(`, by brace matching from its first `{`, or None.

    Braces inside strings or comments can unbalance it; the test bodies this reads are
    plain enough, and an unbalanced body only makes the check stricter."""
    m = re.search(r"\bfn\s+%s\s*\(" % re.escape(fn), text)
    if not m:
        return None
    start = text.find("{", m.end())
    if start < 0:
        return None
    depth = 0
    for i in range(start, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[start:i + 1]
    return text[start:]


def rust_self_skips(doc, root=ROOT):
    """Named Rust tests that can skip (an early `return`) without FLASHTEX_REQUIRE_TEXLIVE=1
    turning that skip into a failure: in the test, its file, or the tests' `common` module.

    A Rust test that returns early still reports `ok`, so a leg without a host or TeX Live
    would "pass" the row; under the switch the same run fails instead."""
    out = []
    for r in doc["rows"]:
        for t in r["tests"]:
            if not t.startswith("rust:"):
                continue
            path, _, fn = t[5:].partition("::")
            full = os.path.join(root, path)
            if not os.path.isfile(full):
                continue
            with open(full, encoding="utf-8") as f:
                text = f.read()
            body = rust_fn_body(text, fn)
            if body is None or not re.search(r"\breturn\b", body):
                continue
            seen = text
            for c in ("common/mod.rs", "common.rs"):
                cp = os.path.join(os.path.dirname(full), c)
                if os.path.isfile(cp):
                    with open(cp, encoding="utf-8") as f:
                        seen += f.read()
            if REQUIRE_ENV not in seen:
                out.append("%s: %s can skip without %s=1 failing it" % (r["id"], t, REQUIRE_ENV))
    return out


def check(doc, root=ROOT):
    errors = []
    if doc.get("schema") != SCHEMA:
        errors.append("schema is %r, expected %r" % (doc.get("schema"), SCHEMA))
    listed = checklist_rows(doc, root)
    ids = [r["id"] for r in doc["rows"]]
    if len(set(ids)) != len(ids):
        errors.append("duplicate row ids")
    if set(ids) != set(listed):
        errors.append("rows differ from the checklist: missing %s, extra %s" % (
            sorted(set(listed) - set(ids)), sorted(set(ids) - set(listed))))
    for r in doc["rows"]:
        if r.get("s5") not in S5_CLASSES:
            errors.append("%s: s5 is %r" % (r["id"], r.get("s5")))
        if r.get("status") not in STATUSES:
            errors.append("%s: status is %r" % (r["id"], r.get("status")))
        if r["id"] in listed and listed[r["id"]] != r.get("status"):
            errors.append("%s: status %r, the checklist says %r" % (r["id"], r.get("status"), listed[r["id"]]))
        if r.get("s5") == "owner" and "owner_retirement" not in r:
            errors.append("%s: an owner row needs an owner_retirement field (null until ruled)" % r["id"])
    named = {t for r in doc["rows"] for t in r["tests"]}
    for t in doc.get("host_driven", []):
        if t not in named:
            errors.append("host_driven names %s, which no row names" % t)
        if "." not in t or t.startswith("rust:"):
            errors.append("host_driven names %s, which is not one Swift test" % t)
    errors += ["test not found in the source: %s" % m for m in missing_tests(doc, root)]
    errors += ["a Rust row test self-skips: %s" % m for m in rust_self_skips(doc, root)]
    return errors


# --------------------------------------------------------------------------
# stage state (static) and logs


def stage_state(doc, stage):
    """('met' | 'not met', [reasons]) from the file alone: classes, tests named, rulings."""
    why = []
    if stage == "S5":
        pending = [r["id"] for r in doc["rows"] if r["s5"] in ("test", "fallback") and not r["tests"]]
        unruled = [r["id"] for r in doc["rows"] if r["s5"] == "owner" and not r.get("owner_retirement")]
        if pending:
            why.append("no named test: " + ", ".join(pending))
        if unruled:
            why.append("owner retirement not in writing: " + ", ".join(unruled))
    elif stage == "S6":
        fallback = [r["id"] for r in doc["rows"] if r["s5"] == "fallback"]
        old = [r["id"] for r in doc["rows"] if r["s5"] == "s6" and not (r["tests"] and r["status"] == "done")
               and not r.get("owner_retirement")]
        if fallback:
            why.append("still on the previous engine's fallback: " + ", ".join(fallback))
        if old:
            why.append("old-route rows open: " + ", ".join(old))
        why += stage_state(doc, "S5")[1]
    else:
        return "none", []
    return ("not met" if why else "met"), why


def read_log(paths):
    """'Suite.test' -> set of outcomes over every log."""
    out = {}
    for p in paths:
        with open(p, encoding="utf-8", errors="replace") as f:
            for m in CASE_RE.finditer(f.read()):
                out.setdefault("%s.%s" % (m.group(1), m.group(2)), set()).add(m.group(3))
    return out


def outcome(test, results):
    """'passed' | 'skipped' | 'failed' | 'absent' for one entry (a suite: all of its tests)."""
    if "." in test:
        got = results.get(test, set())
    else:
        got = set().union(*([v for k, v in results.items() if k.split(".", 1)[0] == test] or [set()]))
    for o in ("failed", "skipped", "passed"):
        if o in got:
            return o
    return "absent"


def log_problems(tests, results):
    return ["%s: %s" % (t, outcome(t, results)) for t in tests if outcome(t, results) != "passed"]


def leg_tests(doc, leg, stage="S6"):
    tests = swift_tests(gating_rows(doc, stage))
    host = set(doc.get("host_driven", []))
    return [t for t in tests if (t in host) == (leg == "host")]


# --------------------------------------------------------------------------


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--rows", default=ROWS_FILE)
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    g = sub.add_parser("gate")
    g.add_argument("stage", choices=("S5", "S6"))
    g.add_argument("--log", action="append", default=[])
    c = sub.add_parser("check-log")
    c.add_argument("--leg", choices=("hosted", "host"), required=True)
    c.add_argument("log", nargs="+")
    a = ap.parse_args(argv)
    doc = load(a.rows)
    if a.cmd == "check":
        errors = check(doc)
        for e in errors:
            print("app-parity-rows: " + e)
        if not errors:
            print("app-parity-rows: %d rows, %d named tests, all found" % (
                len(doc["rows"]), len({t for r in doc["rows"] for t in r["tests"]})))
        return 1 if errors else 0
    if a.cmd == "check-log":
        tests = leg_tests(doc, a.leg)
        bad = log_problems(tests, read_log(a.log))
        for b in bad:
            print("app-parity-rows (%s leg): not passed: %s" % (a.leg, b))
        print("app-parity-rows (%s leg): %d of %d row tests passed, not skipped" % (
            a.leg, len(tests) - len(bad), len(tests)))
        return 1 if bad else 0
    state, why = stage_state(doc, a.stage)
    if a.log:
        bad = log_problems(swift_tests(gating_rows(doc, a.stage)), read_log(a.log))
        if bad:
            state = "not met"
            why.append("not passed in the logs: " + "; ".join(bad))
    print("%s app parity: %s" % (a.stage, state))
    for w in why:
        print("  - " + w)
    return 0 if state == "met" else 1


if __name__ == "__main__":
    sys.exit(main())
