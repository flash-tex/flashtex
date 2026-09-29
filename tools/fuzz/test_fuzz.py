#!/usr/bin/env python3
"""Unit tests for tools/fuzz. Stdlib unittest only.

Run as `python3 -m unittest discover -s tools/fuzz` from the repo root.
Engines are small shell scripts written to a temp dir (in the style of
tools/lockstep/test_capture.py); the real candidate is never used.
"""
import json
import os
import random
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import gen
import run as fuzz_run

SEED_A = ("\\input prelude\n\\count0=10\\advance\\count0 by 5\n"
          "\\message{a=\\the\\count0}\n"
          "\\setbox0=\\hbox{hi}\\lsshipbox0\n\\end\n")
SEED_B = ("\\input prelude\n\\setbox0=\\hbox to 100pt{\\vrule width10pt"
          "\\hskip 0pt plus 1fil\\vrule width20pt}\\lsshipbox0\n\\end\n")

ECHO_BODY = ("for last do :; done\n"
             "job=${last##*/}; job=${job%%.tex}\n"
             "printf 'FAKE-OK\\n' > \"$job.log\"\n"
             "exit 0\n")
CRASH_BODY = ("for last do :; done\n"
              "if grep -q 'hbox' \"$last\" 2>/dev/null; then\n"
              "  echo \"thread 'main' panicked at 'not implemented'\"\n"
              "  exit 101\n"
              "fi\n") + ECHO_BODY
FAIL_BODY = "exit 1\n"
SLEEP_BODY = "sleep 30\nexit 0\n"


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, 0o755)
    return path


class GenTest(unittest.TestCase):
    def test_mutate_deterministic(self):
        self.assertEqual(gen.mutate(SEED_A, random.Random(42)),
                         gen.mutate(SEED_A, random.Random(42)))

    def test_generate_deterministic(self):
        self.assertEqual(gen.generate(random.Random(7)),
                         gen.generate(random.Random(7)))

    def test_generate_has_preamble(self):
        for s in range(10):
            self.assertTrue(
                gen.generate(random.Random(s)).startswith(gen.PREAMBLE))

    def test_mutate_keeps_preamble(self):
        self.assertTrue(
            gen.mutate(SEED_A, random.Random(3)).startswith("\\input prelude"))


class FuzzRunTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-test-")
        self.seeds = os.path.join(self.tmp, "seeds")
        os.mkdir(self.seeds)
        for name, text in (("a.tex", SEED_A), ("b.tex", SEED_B)):
            with open(os.path.join(self.seeds, name), "w") as fh:
                fh.write(text)
        self.out = os.path.join(self.tmp, "out")
        self.echo_a = make_engine(self.tmp, "echo-a.sh", ECHO_BODY)
        self.echo_b = make_engine(self.tmp, "echo-b.sh", ECHO_BODY)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def base_args(self, cand, orc, n=20, seed=123, timeout=10):
        return ["--candidate", cand, "--oracle", orc, "--seeds", self.seeds,
                "--out", self.out, "--iterations", str(n),
                "--seed", str(seed), "--timeout", str(timeout)]

    def test_identical_engines_no_diverge(self):
        counts = fuzz_run.main(
            self.base_args(self.echo_a, self.echo_b))
        self.assertEqual(sum(counts.values()), 20)
        self.assertEqual(counts["diverge"], 0)
        self.assertEqual(counts["candidate-crash"], 0)
        self.assertEqual(counts["oracle-crash"], 0)
        self.assertEqual(counts["timeout"], 0)

    def test_candidate_crash_artifacts(self):
        crash = make_engine(self.tmp, "crash.sh", CRASH_BODY)
        counts = fuzz_run.main(
            self.base_args(crash, self.echo_a, seed=5))
        self.assertEqual(sum(counts.values()), 20)
        self.assertGreater(counts["candidate-crash"], 0)
        cls_dir = os.path.join(self.out, "candidate-crash")
        texs = sorted(f for f in os.listdir(cls_dir) if f.endswith(".tex"))
        self.assertEqual(len(texs), counts["candidate-crash"])
        for tex in texs:
            with open(os.path.join(cls_dir, tex)) as fh:
                self.assertIn("hbox", fh.read())
            meta = os.path.join(cls_dir, tex[:-4] + ".json")
            self.assertTrue(os.path.isfile(meta))
            with open(meta) as fh:
                info = json.load(fh)
            for key in ("seed", "mutation", "candidate_returncode",
                        "oracle_returncode", "first_diff"):
                self.assertIn(key, info)
            self.assertEqual(info["candidate_returncode"], 101)

    def test_both_fail_not_stored(self):
        fail = make_engine(self.tmp, "fail.sh", FAIL_BODY)
        counts = fuzz_run.main(
            self.base_args(fail, fail, n=5, seed=9))
        self.assertEqual(counts["both-fail"], 5)
        self.assertFalse(os.path.exists(self.out))

    def test_timeout_classification(self):
        sleepy = make_engine(self.tmp, "sleep.sh", SLEEP_BODY)
        counts = fuzz_run.main(
            self.base_args(sleepy, self.echo_a, n=2, seed=1, timeout=0.2))
        self.assertEqual(counts["timeout"], 2)
        cls_dir = os.path.join(self.out, "timeout")
        texs = [f for f in os.listdir(cls_dir) if f.endswith(".tex")]
        self.assertEqual(len(texs), 2)
        with open(os.path.join(
                cls_dir, texs[0][:-4] + ".json")) as fh:
            info = json.load(fh)
        self.assertIn("timeout", info["first_diff"])

    def test_candidate_env_passthrough(self):
        os.environ["FLASHTEX_POOL"] = "/tmp/pool-x"
        os.environ["FLASHTEX_FORMATS"] = "/tmp/fmt-x"
        try:
            env = fuzz_run.candidate_env()
        finally:
            del os.environ["FLASHTEX_POOL"]
            del os.environ["FLASHTEX_FORMATS"]
        self.assertEqual(env, {"FLASHTEX_POOL": "/tmp/pool-x",
                               "FLASHTEX_FORMATS": "/tmp/fmt-x"})


if __name__ == "__main__":
    unittest.main()
