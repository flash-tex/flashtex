#!/usr/bin/env python3
"""Unit tests for tools/fuzz. Stdlib unittest only.

Run as `python3 -m unittest discover -s tools/fuzz` from the repo root.
Engines are small shell scripts written to a temp dir (in the style of
tools/lockstep/test_capture.py); the real candidate is never used.
"""
import contextlib
import io
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
import minimize
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

    def test_weights_are_documented_constant(self):
        self.assertIsInstance(gen.MUTATION_WEIGHTS, dict)
        ops = {key for key, _, _ in gen._MUTATION_TABLE}
        self.assertEqual(set(gen.MUTATION_WEIGHTS), ops)
        value = sum(w for k, _, kind in gen._MUTATION_TABLE
                    if kind == "value" for w in (gen.MUTATION_WEIGHTS[k],))
        struct = sum(w for k, _, kind in gen._MUTATION_TABLE
                     if kind == "structural"
                     for w in (gen.MUTATION_WEIGHTS[k],))
        self.assertGreater(value / len(ops), struct / len(ops))

    @staticmethod
    def _kinds(descs):
        kinds = []
        for desc in descs:
            if desc.startswith("ins-brace"):
                kinds.append("structural")
            elif desc.startswith(("num->", "wrap-", "ins-")):
                kinds.append("value")
            else:
                kinds.append("structural")
        return kinds

    def test_weights_change_mix_but_stay_deterministic(self):
        value_w = {k: (5 if kind == "value" else 1)
                   for k, _, kind in gen._MUTATION_TABLE}
        struct_w = {k: (1 if kind == "value" else 5)
                    for k, _, kind in gen._MUTATION_TABLE}
        seq_value = [gen.mutate_with_info(SEED_A, random.Random(11),
                                          value_w)[1]
                     for _ in range(200)]
        seq_struct = [gen.mutate_with_info(SEED_A, random.Random(11),
                                           struct_w)[1]
                     for _ in range(200)]
        # Deterministic: same seed and weights give the same sequence.
        seq_value2 = [gen.mutate_with_info(SEED_A, random.Random(11),
                                           value_w)[1]
                      for _ in range(200)]
        self.assertEqual(seq_value, seq_value2)
        frac_value = (sum(1 for k in self._kinds(seq_value)
                          if k == "value") / len(seq_value))
        frac_struct = (sum(1 for k in self._kinds(seq_struct)
                           if k == "value") / len(seq_struct))
        self.assertGreater(frac_value, frac_struct + 0.15)


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
        # Dedupe: every crash here shares one signature, so exactly one
        # case is stored no matter how many iterations crashed.
        self.assertEqual(len(texs), 1)
        for tex in texs:
            with open(os.path.join(cls_dir, tex)) as fh:
                self.assertIn("hbox", fh.read())
            meta = os.path.join(cls_dir, tex[:-4] + ".json")
            self.assertTrue(os.path.isfile(meta))
            with open(meta) as fh:
                info = json.load(fh)
            for key in ("seed", "mutation", "candidate_returncode",
                        "oracle_returncode", "first_diff", "signature"):
                self.assertIn(key, info)
            self.assertEqual(info["candidate_returncode"], 101)
        with open(os.path.join(self.out, "signatures.json")) as fh:
            sigs = json.load(fh)
        self.assertEqual(sum(sigs.values()), counts["candidate-crash"])

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
        # All timeouts share the constant "timeout" signature: 1 stored.
        self.assertEqual(len(texs), 1)
        with open(os.path.join(
                cls_dir, texs[0][:-4] + ".json")) as fh:
            info = json.load(fh)
        self.assertIn("timeout", info["first_diff"])
        with open(os.path.join(self.out, "signatures.json")) as fh:
            self.assertEqual(json.load(fh), {"timeout": 2})

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

    def test_draw_input_redraws_identical(self):
        calls = []
        real = gen.mutate_with_info

        def stub(base, rng):
            calls.append(1)
            if len(calls) < 3:
                return base, "noop"
            return base + "%x\n", "changed"

        gen.mutate_with_info = stub
        try:
            text, mutation, _origin = fuzz_run.draw_input(
                random.Random(1), [("s.tex", SEED_A)])
        finally:
            gen.mutate_with_info = real
        self.assertEqual(text, SEED_A + "%x\n")
        self.assertEqual(mutation, "changed")
        self.assertEqual(len(calls), 3)

    def test_draw_input_gives_up_after_3_redraws(self):
        calls = []
        real = gen.mutate_with_info

        def stub(base, rng):
            calls.append(1)
            return base, "noop"

        gen.mutate_with_info = stub
        try:
            text, _, _ = fuzz_run.draw_input(
                random.Random(1), [("s.tex", SEED_A)])
        finally:
            gen.mutate_with_info = real
        self.assertEqual(text, SEED_A)
        self.assertEqual(len(calls), 4)


# Note: capture() reads the transcript from <job>.log, so the panic line
# must be written there (not just stdout) for panic_location to see it.
ALWAYS_CRASH_A = ("for last do :; done\n"
                  "job=${last##*/}; job=${job%%.tex}\n"
                  "echo \"thread 'main' panicked at 'boom-A', "
                  "src/thing.rs:10:5\" > \"$job.log\"\n"
                  "cat \"$job.log\"\n"
                  "exit 101\n")
ALWAYS_CRASH_B = ("for last do :; done\n"
                  "job=${last##*/}; job=${job%%.tex}\n"
                  "echo \"thread 'main' panicked at 'boom-B', "
                  "src/other.rs:20:7\" > \"$job.log\"\n"
                  "cat \"$job.log\"\n"
                  "exit 101\n")


class DedupeTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-dedupe-")
        self.seeds = os.path.join(self.tmp, "seeds")
        os.mkdir(self.seeds)
        with open(os.path.join(self.seeds, "a.tex"), "w") as fh:
            fh.write(SEED_A)
        self.out = os.path.join(self.tmp, "out")
        self.echo = make_engine(self.tmp, "echo.sh", ECHO_BODY)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def args(self, cand, n=10, seed=42):
        return ["--candidate", cand, "--oracle", self.echo,
                "--seeds", self.seeds, "--out", self.out,
                "--iterations", str(n), "--seed", str(seed),
                "--timeout", "10"]

    def stored_texs(self):
        found = []
        for root, _dirs, files in os.walk(self.out):
            found.extend(os.path.join(root, f) for f in files
                         if f.endswith(".tex"))
        return found

    def test_one_case_for_shared_signature(self):
        crash = make_engine(self.tmp, "crash-a.sh", ALWAYS_CRASH_A)
        counts = fuzz_run.main(self.args(crash))
        self.assertEqual(counts["candidate-crash"], 10)
        self.assertEqual(len(self.stored_texs()), 1)
        with open(os.path.join(self.out, "signatures.json")) as fh:
            sigs = json.load(fh)
        self.assertEqual(len(sigs), 1)
        self.assertEqual(sum(sigs.values()), 10)

    def test_second_signature_stored_and_names_persist(self):
        crash_a = make_engine(self.tmp, "crash-a.sh", ALWAYS_CRASH_A)
        crash_b = make_engine(self.tmp, "crash-b.sh", ALWAYS_CRASH_B)
        fuzz_run.main(self.args(crash_a, seed=42))
        self.assertEqual(len(self.stored_texs()), 1)
        # Different seed so the second run draws different texts (stored
        # cases are content-addressed); only the signature is shared.
        fuzz_run.main(self.args(crash_b, seed=43))
        # Two distinct panic locations: two stored cases.
        self.assertEqual(len(self.stored_texs()), 2)
        with open(os.path.join(self.out, "signatures.json")) as fh:
            sigs = json.load(fh)
        self.assertEqual(len(sigs), 2)
        # A repeat run finds A's signature already on disk: nothing new.
        fuzz_run.main(self.args(crash_a))
        self.assertEqual(len(self.stored_texs()), 2)


TRIGGER_BODY = ("for last do :; done\n"
                "job=${last##*/}; job=${job%%.tex}\n"
                "if grep -q 'TRIGGER-A' \"$last\" 2>/dev/null; then\n"
                "  echo \"thread 'main' panicked at 'boom-A', "
                "src/thing.rs:10:5\" > \"$job.log\"\n"
                "  cat \"$job.log\"\n"
                "  exit 101\n"
                "fi\n"
                "if grep -q 'TRIGGER-B' \"$last\" 2>/dev/null; then\n"
                "  echo \"thread 'main' panicked at 'boom-B', "
                "src/other.rs:20:7\" > \"$job.log\"\n"
                "  cat \"$job.log\"\n"
                "  exit 101\n"
                "fi\n") + ECHO_BODY


class MinimizeTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-min-")
        self.crash = make_engine(self.tmp, "trigger.sh", TRIGGER_BODY)
        self.echo = make_engine(self.tmp, "echo.sh", ECHO_BODY)
        filler = "".join("%% filler line %d\n" % i for i in range(15))
        filler2 = "".join("%% more filler %d\n" % i for i in range(14))
        self.text = (filler + "\\message{has TRIGGER-A here}\n"
                     + filler2 + "\\message{has TRIGGER-B here}\n")
        self.assertEqual(len(self.text.splitlines()), 31)
        self.input_path = os.path.join(self.tmp, "case.tex")
        with open(self.input_path, "w") as fh:
            fh.write(self.text)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_minimize_keeps_trigger_and_class(self):
        out = os.path.join(self.tmp, "min.tex")
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            ret = minimize.main(
                ["--candidate", self.crash, "--oracle", self.echo,
                 "--input", self.input_path, "--class", "candidate-crash",
                 "--out", out, "--timeout", "10"])
        self.assertEqual(ret, 0)
        with open(out) as fh:
            small = fh.read()
        # One panic trigger among 30 filler lines shrinks to <= 3 lines,
        # keeping the original panic location (A), not just any crash.
        self.assertLessEqual(len(small.splitlines()), 3)
        self.assertIn("TRIGGER-A", small)
        self.assertNotIn("TRIGGER-B", small)
        cls = fuzz_run.run_one(
            small, self.crash, self.echo, 10)[0]
        self.assertEqual(cls, "candidate-crash")
        self.assertRegex(buf.getvalue(), r"\d+ engine runs")

    def test_minimize_rejects_wrong_class(self):
        out = os.path.join(self.tmp, "min.tex")
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            with contextlib.redirect_stderr(buf):
                ret = minimize.main(
                    ["--candidate", self.crash, "--oracle", self.echo,
                     "--input", self.input_path, "--class", "diverge",
                     "--out", out, "--timeout", "10"])
        self.assertNotEqual(ret, 0)
        self.assertFalse(os.path.exists(out))


if __name__ == "__main__":
    unittest.main()
