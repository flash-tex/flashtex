"""Tests for nightly.py. Fake fuzzer scripts only; the real engines are
never used."""
import json
import os
import shutil
import stat
import sys
import tempfile
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nightly

FAKE = """#!/usr/bin/env python3
import argparse, json, os, sys, time
ap = argparse.ArgumentParser()
ap.add_argument("--candidate", default="x")
ap.add_argument("--oracle", default="y")
ap.add_argument("--seeds", default="z")
ap.add_argument("--out", required=True)
ap.add_argument("--iterations", type=int, required=True)
ap.add_argument("--seed", type=int, default=0)
ap.add_argument("--timeout", type=float, default=1.0)
ap.add_argument("--mode", default="clean")
a = ap.parse_args()
time.sleep(0.001 * a.iterations)
os.makedirs(os.path.join(a.out, "candidate-crash"), exist_ok=True)
if a.mode == "fail":
    print("boom", flush=True)
    sys.exit(2)
if a.mode == "finding":
    sig = os.environ.get("FAKE_SIG", "candidate-crash:exit:101")
    digest = "%040d" % a.seed
    with open(os.path.join(a.out, "candidate-crash",
                           digest + ".tex"), "w") as fh:
        fh.write("input")
    with open(os.path.join(a.out, "candidate-crash",
                           digest + ".json"), "w") as fh:
        json.dump({"signature": sig}, fh)
    print("done: %d iterations: equal=%d candidate-crash=1"
          % (a.iterations, a.iterations - 1))
else:
    print("done: %d iterations: equal=%d" % (a.iterations, a.iterations))
"""


class NightlyTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.scripts = os.path.join(self.tmp, "fuzzers")
        os.mkdir(self.scripts)
        self.out = os.path.join(self.tmp, "out")
        self.old_table = nightly.FUZZERS
        nightly.FUZZERS = tuple(
            dict(name="fz%d" % i, script="fz%d.py" % i, oracle=bool(i % 2),
                 seeds=False, timeout=5.0, base=100, offset=i)
            for i in range(2))
        for i in range(2):
            path = os.path.join(self.scripts, "fz%d.py" % i)
            with open(path, "w") as fh:
                fh.write(FAKE)
            os.chmod(path, os.stat(path).st_mode | stat.S_IEXEC)
            # nightly resolves scripts relative to its own dir; point
            # each entry at the temp script through an absolute path.
            nightly.FUZZERS[i]["script"] = os.path.relpath(path,
                                                           nightly.HERE)
        self.known = os.path.join(self.tmp, "known.json")
        with open(self.known, "w") as fh:
            json.dump([], fh)

    def tearDown(self):
        nightly.FUZZERS = self.old_table
        shutil.rmtree(self.tmp, ignore_errors=True)
        os.environ.pop("FAKE_SIG", None)

    def base_args(self, extra=()):
        return ["--candidate", "c", "--oracle", "o", "--out", self.out,
                "--seed", "7", "--budget-minutes", "1",
                "--known-findings", self.known] + list(extra)

    def run_main(self, args):
        # limit fake modes via env passthrough is subprocess-inherited
        return nightly.main(args)

    def test_sizing_stays_inside_budget(self):
        rates = {"a": 0.5, "b": 2.0}
        table = [dict(name="a", base=100), dict(name="b", base=100)]
        totals = nightly.size_runs(60.0, table, rates)
        cost = sum(totals[n] * rates[n] for n in totals)
        self.assertLessEqual(cost, 60.0)
        # tiny budget scales everything down, huge budget keeps base
        tiny = nightly.size_runs(21.0, table, rates)
        self.assertLess(sum(tiny.values()), sum(totals.values()))
        full = nightly.size_runs(3600.0, table, rates)
        self.assertEqual(full, {"a": 100, "b": 100})

    def test_known_matching(self):
        self.assertTrue(nightly.is_known("fontcount-diff:3v5",
                                         ["fontcount-diff:*"]))
        self.assertTrue(nightly.is_known("signal:6", ["signal:6"]))
        self.assertFalse(nightly.is_known("signal:11", ["signal:6"]))
        self.assertFalse(nightly.is_known("other", ["fontcount-diff:*"]))

    def test_exit_zero_no_findings(self):
        rc = self.run_main(self.base_args())
        self.assertEqual(rc, 0)
        with open(os.path.join(self.out, "summary.json")) as fh:
            summary = json.load(fh)
        self.assertEqual(set(summary["fuzzers"]), {"fz0", "fz1"})
        entry = summary["fuzzers"]["fz0"]
        for key in ("iterations", "class_counts", "new_signatures",
                    "elapsed_seconds", "seed"):
            self.assertIn(key, entry)
        self.assertGreater(entry["iterations"], 0)
        with open(os.path.join(self.out, "summary.md")) as fh:
            md = fh.read()
        self.assertIn("| fz0 |", md)

    def test_exit_one_unknown_finding(self):
        os.environ["FAKE_SIG"] = "diverge:brand-new-bug"
        # fake script needs mode=finding: pass via wrapper args is not
        # supported, so flip the default mode in the temp scripts.
        for i in range(2):
            path = os.path.join(self.scripts, "fz%d.py" % i)
            with open(path) as fh:
                text = fh.read()
            with open(path, "w") as fh:
                fh.write(text.replace('default="clean"',
                                      'default="finding"'))
        rc = self.run_main(self.base_args())
        self.assertEqual(rc, 1)
        with open(os.path.join(self.out, "summary.md")) as fh:
            md = fh.read()
        self.assertIn("diverge:brand-new-bug", md)
        self.assertIn("artifact:", md)

    def test_exit_zero_known_finding(self):
        with open(self.known, "w") as fh:
            json.dump([{"signature": "candidate-crash:exit:101",
                        "note": "known"}], fh)
        os.environ["FAKE_SIG"] = "candidate-crash:exit:101"
        for i in range(2):
            path = os.path.join(self.scripts, "fz%d.py" % i)
            with open(path) as fh:
                text = fh.read()
            with open(path, "w") as fh:
                fh.write(text.replace('default="clean"',
                                      'default="finding"'))
        self.assertEqual(self.run_main(self.base_args()), 0)

    def test_exit_two_harness_failure(self):
        for i in range(2):
            path = os.path.join(self.scripts, "fz%d.py" % i)
            with open(path) as fh:
                text = fh.read()
            with open(path, "w") as fh:
                fh.write(text.replace('default="clean"',
                                      'default="fail"'))
        self.assertEqual(self.run_main(self.base_args()), 2)

    def test_default_seed_changes_daily(self):
        import datetime
        d0 = datetime.date(2026, 9, 29)
        d1 = datetime.date(2026, 9, 30)
        self.assertEqual(nightly.default_seed(d0) + 1,
                         nightly.default_seed(d1))
        self.assertEqual(nightly.default_seed(d0),
                         (d0 - datetime.date(1970, 1, 1)).days)


if __name__ == "__main__":
    unittest.main()
