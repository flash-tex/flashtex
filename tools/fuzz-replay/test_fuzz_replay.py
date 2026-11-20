#!/usr/bin/env python3
"""Tests for tools/fuzz-replay. Stdlib unittest only.

    python3 -m unittest discover -s tools/fuzz-replay -p 'test_*.py'

The engine tests run only when FUZZ_REPLAY_ENGINE names a built
flashtex-initex (CI sets it after its release build); the rest are pure
logic.
"""
import os
import sys
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import fuzz_replay as fr  # noqa: E402
import gen  # noqa: E402
import minimize  # noqa: E402

ENGINE = os.environ.get("FUZZ_REPLAY_ENGINE")
# A fixed seed set: clean (no finding under either system) on main.
FIXED_SEEDS = "0-149"


class GenTest(unittest.TestCase):
    def test_deterministic(self):
        for s in (0, 1, 17, 999):
            self.assertEqual(gen.generate(s), gen.generate(s))
        self.assertNotEqual(gen.generate(1), gen.generate(2))

    def test_lines_balanced(self):
        # every line is one balanced item, so the line pass never splits a group
        # (but the two lines that open and close \measure's body)
        for s in range(300):
            for name, text in gen.generate(s).items():
                self.assertTrue(minimize.balanced(text), (s, name))
                for line in text.split("\n"):
                    if line.startswith(r"\def\measure") or line == "}%":
                        continue
                    self.assertTrue(minimize.balanced(line), (s, name, line))

    def test_styles_and_calls(self):
        styles = set()
        for s in range(200):
            g = gen.Gen(s)
            t = g.build()["t.tex"]
            styles.add(g.style)
            self.assertIn(r"\def\measure", t)
            self.assertGreaterEqual(t.count(r"\measure"), 3, s)
            self.assertTrue(t.rstrip().endswith(r"\end"))
        self.assertEqual(styles, {"box", "pure"})


class LogicTest(unittest.TestCase):
    def test_ddmin(self):
        need = {3, 11, 12}
        out = minimize.ddmin(list(range(20)), lambda xs: need <= set(xs))
        self.assertEqual(sorted(out), sorted(need))
        self.assertEqual(minimize.ddmin([1, 2, 3], lambda xs: True), [])

    def test_balanced(self):
        self.assertTrue(minimize.balanced(r"\def\a{\b{c}}\catcode`\{=1"))
        self.assertFalse(minimize.balanced(r"\def\a{"))
        self.assertFalse(minimize.balanced("}{"))

    def test_parse_stats(self):
        text = ("Stats {\n    offers: 5,\n    hits: 3,\n    abandoned: {\n"
                "        \"Level\": 2,\n    },\n    verify_differences: 1,\n}\n")
        self.assertEqual(fr.parse_stats(text),
                         {"offers": 5, "hits": 3, "verify_differences": 1})

    def test_signature(self):
        det = {"verify_msgs": [r"boxmemo verify (full state): \measure: 8 differs "
                               "outside what the structural comparison reads: "
                               "param_ptr: 0x0 -> 0x1"]}
        self.assertEqual(
            fr.signature("bm", ["on-log", "verify-log", "verify-exit3"], det),
            "bm:on-log+verify-exit3:param_ptr")

    def test_seeds(self):
        self.assertEqual(fr.parse_seeds("1-3,7"), [1, 2, 3, 7])


@unittest.skipUnless(ENGINE and os.path.exists(ENGINE),
                     "FUZZ_REPLAY_ENGINE does not name a built flashtex-initex")
class EngineTest(unittest.TestCase):
    def run_seeds(self, env=None):
        old = dict(os.environ)
        os.environ.update(env or {})
        try:
            with tempfile.TemporaryDirectory() as out:
                t0 = time.monotonic()
                rc = fr.main(["run", "--engine", ENGINE, "--seeds", FIXED_SEEDS,
                              "--jobs", "2", "--quiet", "--out", out])
                dt = time.monotonic() - t0
                with open(os.path.join(out, "summary.json")) as f:
                    import json
                    summary = json.load(f)
        finally:
            os.environ.clear()
            os.environ.update(old)
        return rc, dt, summary

    def test_fixed_seeds_clean(self):
        rc, dt, summary = self.run_seeds()
        self.assertEqual(summary["signatures"], {})
        self.assertEqual(rc, 0)
        self.assertLess(dt, 30, "the fixed seed set must run in under 30 s")
        # the set exercises both systems: some calls are replayed
        self.assertGreater(summary["totals"]["cases_hit_bm"], 10)
        self.assertGreater(summary["totals"]["cases_hit_d9"], 3)

    def test_fault_is_found(self):
        rc, _, summary = self.run_seeds({"FLASHTEX_INTRINSICS_FAULT": "drop-last"})
        self.assertEqual(rc, 1)
        self.assertTrue(any(s.startswith("d9:") for s in summary["signatures"]),
                        summary["signatures"])


if __name__ == "__main__":
    unittest.main()
