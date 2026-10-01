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
import subprocess
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
        # All timeouts share the "timeout:candidate" signature: 1 stored.
        self.assertEqual(len(texs), 1)
        with open(os.path.join(
                cls_dir, texs[0][:-4] + ".json")) as fh:
            info = json.load(fh)
        self.assertIn("timeout", info["first_diff"])
        self.assertEqual(info["signature"], "timeout:candidate")
        with open(os.path.join(self.out, "signatures.json")) as fh:
            self.assertEqual(json.load(fh), {"timeout:candidate": 2})

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
# Fake oracle that dies by signal: exit 101 no longer counts as an oracle
# crash, so a both-crash test needs a signal death on the oracle side.
SIGNAL_DEATH_BODY = ("for last do :; done\n"
                     "kill -ABRT $$\n")


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


class SignatureTest(unittest.TestCase):
    def test_panic_location_drops_column_and_thread(self):
        self.assertEqual(
            fuzz_run.panic_location(
                "thread 'main' panicked at "
                "crates/flashtex-engine/src/generated/body_0.rs:1033:21:\n"
                "note: run with RUST_BACKTRACE=1"),
            "crates/flashtex-engine/src/generated/body_0.rs:1033")
        self.assertEqual(
            fuzz_run.panic_location(
                "thread 'main' (123) panicked at 'boom', src/thing.rs:10:5"),
            "src/thing.rs:10")

    def test_panic_location_absent(self):
        self.assertIsNone(fuzz_run.panic_location("no panic here"))
        self.assertIsNone(
            fuzz_run.panic_location("thread 'main' panicked at 'oops'"))

    def test_signal_name(self):
        self.assertEqual(fuzz_run.signal_name(-6), "SIGABRT")
        self.assertEqual(fuzz_run.signal_name(-11), "SIGSEGV")
        self.assertIsNone(fuzz_run.signal_name(0))
        self.assertIsNone(fuzz_run.signal_name(None))

    def test_crash_signature_prefers_stderr(self):
        err = ("thread 'main' panicked at crates/fuzz/x.rs:77:2:\n"
               "stack backtrace:\n")
        self.assertEqual(fuzz_run.crash_signature(101, "plain log", err),
                         "panic:crates/fuzz/x.rs:77")

    def test_crash_signature_signal_first_line(self):
        self.assertEqual(
            fuzz_run.crash_signature(
                -6, "log",
                "fatal runtime error: stack overflow 123, aborting\nsecond"),
            "signal:SIGABRT:fatal runtime error: stack overflow NNN,"
            " aborting")

    def test_crash_signature_signal_no_stderr(self):
        self.assertEqual(fuzz_run.crash_signature(-11, "", ""),
                         "signal:SIGSEGV")

    def test_crash_signature_exit(self):
        self.assertEqual(fuzz_run.crash_signature(101, "panicked at 'x'"),
                         "exit:101")


STDERR_PANIC_BODY = ("for last do :; done\n"
                     "job=${last##*/}; job=${job%%.tex}\n"
                     "printf 'FAKE-OK\\n' > \"$job.log\"\n"
                     "echo \"thread 'main' (4242) panicked at "
                     "crates/fuzz/x.rs:77:2:\" >&2\n"
                     "exit 101\n")
ABRT_BODY = ("for last do :; done\n"
             "echo 'fatal runtime error: stack overflow, aborting' >&2\n"
             "kill -ABRT $$\n")


class CrashStderrTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-sig-")
        self.seeds = os.path.join(self.tmp, "seeds")
        os.mkdir(self.seeds)
        with open(os.path.join(self.seeds, "a.tex"), "w") as fh:
            fh.write(SEED_A)
        self.out = os.path.join(self.tmp, "out")
        self.echo = make_engine(self.tmp, "echo.sh", ECHO_BODY)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_stderr_only_panic_signature(self):
        # The panic goes to stderr only, never the transcript log: the
        # signature must come from the direct re-run.
        cand = make_engine(self.tmp, "stderr-panic.sh", STDERR_PANIC_BODY)
        cls, cand_rc, orc_rc, diff, cand_log, orc_log = fuzz_run.run_one(
            SEED_A, cand, self.echo, 10, return_logs=True)
        self.assertEqual(cls, "candidate-crash")
        self.assertNotIn("panicked at", cand_log)
        err = fuzz_run.crash_stderr(SEED_A, cand, fuzz_run.candidate_env(),
                                    10)
        self.assertIn("panicked at", err)
        sig = fuzz_run.signature(cls, cand_rc, cand_log, orc_rc, orc_log,
                                 diff, cand_stderr=err)
        self.assertEqual(sig, "panic:crates/fuzz/x.rs:77")

    def test_sigabrt_signature(self):
        cand = make_engine(self.tmp, "abrt.sh", ABRT_BODY)
        cls, cand_rc, orc_rc, diff, cand_log, orc_log = fuzz_run.run_one(
            SEED_A, cand, self.echo, 10, return_logs=True)
        self.assertEqual(cls, "candidate-crash")
        self.assertEqual(cand_rc, -6)
        err = fuzz_run.crash_stderr(SEED_A, cand, fuzz_run.candidate_env(),
                                    10)
        sig = fuzz_run.signature(cls, cand_rc, cand_log, orc_rc, orc_log,
                                 diff, cand_stderr=err)
        self.assertEqual(
            sig, "signal:SIGABRT:fatal runtime error: stack overflow,"
            " aborting")


class BothEnginesTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-both-")
        self.seeds = os.path.join(self.tmp, "seeds")
        os.mkdir(self.seeds)
        with open(os.path.join(self.seeds, "a.tex"), "w") as fh:
            fh.write(SEED_A)
        self.out = os.path.join(self.tmp, "out")
        self.echo = make_engine(self.tmp, "echo.sh", ECHO_BODY)
        self.crash_a = make_engine(self.tmp, "crash-a.sh", ALWAYS_CRASH_A)
        self.sig = make_engine(self.tmp, "sig.sh", SIGNAL_DEATH_BODY)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_both_crash_is_not_engine_diff(self):
        # pdfTeX itself crashes too: neither candidate- nor oracle-crash.
        cls = fuzz_run.run_one(SEED_A, self.crash_a, self.sig, 10)[0]
        self.assertEqual(cls, "both-crash")
        self.assertEqual(
            fuzz_run.classify(101, "panicked at x", -11, "", False),
            "both-crash")
        # Exit 101 from the oracle alone is not an oracle crash.
        self.assertEqual(
            fuzz_run.classify(101, "panicked at x", 101, "panicked at y",
                              False),
            "candidate-crash")

    def test_single_crash_stays_sided(self):
        self.assertEqual(
            fuzz_run.run_one(
                SEED_A, self.crash_a, self.echo, 10)[0],
            "candidate-crash")
        self.assertEqual(
            fuzz_run.run_one(
                SEED_A, self.echo, self.sig, 10)[0],
            "oracle-crash")

    def test_both_hang_needs_both_timeouts(self):
        self.assertEqual(
            fuzz_run.classify(0, "", 0, "", (True, True)), "both-hang")
        self.assertEqual(
            fuzz_run.classify(0, "", 0, "", (True, False)), "timeout")
        self.assertEqual(
            fuzz_run.classify(0, "", 0, "", (False, True)), "timeout")
        self.assertEqual(fuzz_run.classify(0, "", 0, "", True), "timeout")
        self.assertEqual(fuzz_run.classify(0, "", 0, "", False), "equal")

    def test_both_crash_stored_under_own_dir(self):
        counts = fuzz_run.main(
            ["--candidate", self.crash_a, "--oracle", self.sig,
             "--seeds", self.seeds, "--out", self.out,
             "--iterations", "5", "--seed", "1", "--timeout", "10"])
        self.assertEqual(counts["both-crash"], 5)
        self.assertEqual(counts["candidate-crash"], 0)
        self.assertEqual(counts["oracle-crash"], 0)
        cls_dir = os.path.join(self.out, "both-crash")
        texs = sorted(f for f in os.listdir(cls_dir) if f.endswith(".tex"))
        self.assertEqual(len(texs), 1)
        with open(os.path.join(cls_dir, texs[0][:-4] + ".json")) as fh:
            info = json.load(fh)
        self.assertTrue(info["signature"].startswith("both-crash:panic:"))

    def test_both_hang_signature(self):
        self.assertEqual(
            fuzz_run.signature("both-hang", None, "", None, "", "d"),
            "both-hang")
        self.assertEqual(
            fuzz_run.signature("both-crash", -11, "", -11, "", None),
            "both-crash:signal:SIGSEGV")


class TranscriptNeverDecidesCrashTest(unittest.TestCase):
    # A transcript as produced by a document containing
    # \message{panicked at}: the words "panicked at" appear in the log
    # with a perfectly normal return code.
    PANIC_LOG = ("This is pdfTeX\n"
                 "\\message{panicked at dawn}\n"
                 "panicked at dawn\n"
                 "Output written on fuzz.pdf (1 page).\n")

    def test_panic_words_with_normal_runs_is_equal(self):
        self.assertEqual(
            fuzz_run.classify(0, self.PANIC_LOG, 0, self.PANIC_LOG,
                              False),
            "equal")

    def test_panic_words_with_real_101_is_candidate_crash(self):
        # Both transcripts carry the words, but only the candidate really
        # crashed (exit 101): candidate-crash, never both-crash.
        self.assertEqual(
            fuzz_run.classify(101, "panicked at x", 0, self.PANIC_LOG,
                              False),
            "candidate-crash")

    def test_panic_words_with_divergence_is_diverge(self):
        cand = self.PANIC_LOG + "extra candidate line\n"
        self.assertEqual(
            fuzz_run.classify(0, cand, 0, self.PANIC_LOG, False),
            "diverge")

    def test_both_signal_deaths_is_both_crash(self):
        self.assertEqual(
            fuzz_run.classify(-11, "", -6, "", False), "both-crash")
        # ...while exit 101 from the oracle alone is not an oracle crash.
        self.assertEqual(
            fuzz_run.classify(0, "fine", 101, "panicked at y", False),
            "diverge")


PLANT_CAND_BODY = ("for last do :; done\n"
                   "job=${last##*/}; job=${job%%.tex}\n"
                   "printf 'BASE one\\n' > \"$job.log\"\n"
                   "if grep -q 'PLANT' \"$last\" 2>/dev/null; then\n"
                   "  printf 'PLANT extra detail 12345 here\\n' >> "
                   "\"$job.log\"\n"
                   "fi\n"
                   "if grep -q 'OTHER' \"$last\" 2>/dev/null; then\n"
                   "  printf 'OTHER marker line\\n' >> \"$job.log\"\n"
                   "fi\n"
                   "printf 'BASE tail\\n' >> \"$job.log\"\n"
                   "exit 0\n")
PLANT_ORC_BODY = ("for last do :; done\n"
                  "job=${last##*/}; job=${job%%.tex}\n"
                  "printf 'BASE one\\nBASE tail\\n' > \"$job.log\"\n"
                  "exit 0\n")


class MinimizeDivergeTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-mindiv-")
        self.cand = make_engine(self.tmp, "plant-cand.sh", PLANT_CAND_BODY)
        self.orc = make_engine(self.tmp, "plant-orc.sh", PLANT_ORC_BODY)
        self.text = ("\\input prelude\n"
                     "%% filler one\n"
                     "%% filler two\n"
                     "a line with PLANT 777 in it\n"
                     "%% filler three\n"
                     "a line with OTHER in it\n"
                     "%% filler four\n")
        cls, _, _, diff = fuzz_run.run_one(
            self.text, self.cand, self.orc, 10)
        self.assertEqual(cls, "diverge")
        self.orig_norm = fuzz_run.normalised_diff(diff)
        self.assertIn("PLANT", self.orig_norm)
        self.input_path = os.path.join(self.tmp, "case.tex")
        with open(self.input_path, "w") as fh:
            fh.write(self.text)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_minimize_keeps_original_first_difference(self):
        out = os.path.join(self.tmp, "min.tex")
        ret = minimize.main(
            ["--candidate", self.cand, "--oracle", self.orc,
             "--input", self.input_path, "--class", "diverge",
             "--out", out, "--timeout", "10"])
        self.assertEqual(ret, 0)
        with open(out) as fh:
            small = fh.read()
        # The planted token behind the original first differing line
        # survives; the decoy divergence (OTHER) is minimized away.
        self.assertIn("PLANT", small)
        self.assertNotIn("OTHER", small)
        self.assertLess(len(small), len(self.text))
        _cls, _, _, diff = fuzz_run.run_one(
            small, self.cand, self.orc, 10)
        self.assertEqual(fuzz_run.normalised_diff(diff), self.orig_norm)


FLOOD_BODY = ("for last do :; done\n"
              "job=${last##*/}; job=${job%%.tex}\n"
              "if grep -q 'FLOODMARKER' \"$last\" 2>/dev/null; then\n"
              "  head -c 314572800 /dev/zero > \"$job.log\" 2>/dev/null\n"
              "  kill -XFSZ $$ 2>/dev/null\n"
              "  exit 152\n"
              "fi\n"
              "printf 'FAKE-OK\\n' > \"$job.log\"\n"
              "exit 0\n")
SELF_KILL_BODY = "kill -XFSZ $$ 2>/dev/null\nexit 152\n"
STDERR_FLOOD_BODY = "#!/bin/sh\nexec yes 'stderr flood line programmed' >&2\n"

FLOOD_TEXT = ("\\input prelude\n\\message{has FLOODMARKER here}\n\\end\n")
CALM_TEXT = ("\\input prelude\n\\message{plain input}\n\\end\n")

# Runs one run_one() in a child python with the file-size cap applied
# (the cap would irreversibly lower the test process's own hard limit,
# so it must live in a subprocess, like the real entry points).
FLOOD_CHILD = (
    "import sys; sys.path.insert(0, 'tools/fuzz');"
    "import run as fuzz_run; fuzz_run.apply_fsize_limit();"
    "text = open(sys.argv[1]).read();"
    "cls = fuzz_run.run_one(text, sys.argv[2], sys.argv[3], 10)[0];"
    "print('class=' + cls)")


class OutputCapTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-flood-")
        self.echo = make_engine(self.tmp, "echo.sh", ECHO_BODY)
        self.flood = make_engine(self.tmp, "flood.sh", FLOOD_BODY)
        self.root = os.path.dirname(os.path.dirname(HERE))

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_child(self, text, cand, orc):
        inp = os.path.join(self.tmp, "in.tex")
        with open(inp, "w") as fh:
            fh.write(text)
        env = dict(os.environ, FUZZ_FSIZE_LIMIT_BYTES=str(1024 * 1024))
        proc = subprocess.run(
            [sys.executable, "-c", FLOOD_CHILD, inp, cand, orc],
            cwd=self.root, env=env, capture_output=True, text=True,
            timeout=120)
        self.assertEqual(proc.returncode, 0, proc.stderr[-2000:])
        line = [ln for ln in proc.stdout.splitlines()
                if ln.startswith("class=")]
        return line[0][len("class="):]

    def test_flood_mapping(self):
        self.assertTrue(fuzz_run.is_output_flood(-25))
        self.assertTrue(fuzz_run.is_output_flood(152))
        for rc in (0, 1, 101, -6, -11, None):
            self.assertFalse(fuzz_run.is_output_flood(rc))
        self.assertEqual(
            fuzz_run.classify(-25, "", 0, "log", False), "output-flood")
        self.assertEqual(
            fuzz_run.classify(-25, "", 152, "", False), "both-flood")
        self.assertEqual(
            fuzz_run.signature("output-flood", -25, "", 0, "", None),
            "output-flood")
        self.assertEqual(
            fuzz_run.signature("both-flood", -25, "", -25, "", None),
            "both-flood")

    def test_cap_helpers_keep_head_and_tail(self):
        # A few MiB with an explicit small limit: no test holds big
        # outputs in memory.
        big = b"0123456789" * (400 * 1024)  # ~4 MiB
        capped = fuzz_run.cap_bytes(big, 1024 * 1024)
        self.assertLessEqual(len(capped), 1024 * 1024 + 100)
        self.assertTrue(capped.startswith(big[:100]))
        self.assertTrue(capped.endswith(big[-100:]))
        self.assertEqual(fuzz_run.cap_bytes(b"small"), b"small")
        text = "x" * (2 * 1024 * 1024)  # ~2 MiB
        capped_text = fuzz_run.cap_text(text, 1024 * 1024)
        self.assertLessEqual(len(capped_text), 1024 * 1024 + 100)
        self.assertTrue(capped_text.startswith(text[:100]))
        self.assertTrue(capped_text.endswith(text[-100:]))

    def test_marker_flood_is_output_flood(self):
        # The fake engine writes 300 MB only when FLOODMARKER is present;
        # the 1 MiB cap kills it with SIGXFSZ first.
        self.assertEqual(
            self.run_child(FLOOD_TEXT, self.flood, self.echo),
            "output-flood")
        self.assertEqual(
            self.run_child(FLOOD_TEXT, self.flood, self.flood),
            "both-flood")
        self.assertEqual(
            self.run_child(CALM_TEXT, self.flood, self.echo), "equal")

    def test_fsize_limit_env(self):
        os.environ["FUZZ_FSIZE_LIMIT_BYTES"] = "1048576"
        try:
            self.assertEqual(fuzz_run.fsize_limit_bytes(), 1048576)
        finally:
            del os.environ["FUZZ_FSIZE_LIMIT_BYTES"]
        self.assertEqual(fuzz_run.fsize_limit_bytes(),
                         64 * 1024 * 1024)
        os.environ["FUZZ_FSIZE_LIMIT_BYTES"] = "not-a-number"
        try:
            self.assertEqual(fuzz_run.fsize_limit_bytes(),
                             64 * 1024 * 1024)
        finally:
            del os.environ["FUZZ_FSIZE_LIMIT_BYTES"]

    def test_apply_fsize_limit_noop_keeps_process_usable(self):
        import resource
        before = resource.getrlimit(resource.RLIMIT_FSIZE)
        os.environ["FUZZ_FSIZE_LIMIT_BYTES"] = str(before[0])
        try:
            fuzz_run.apply_fsize_limit()
        finally:
            del os.environ["FUZZ_FSIZE_LIMIT_BYTES"]
        self.assertEqual(resource.getrlimit(resource.RLIMIT_FSIZE),
                         before)

    def test_parsers_map_flood(self):
        sys.path.insert(0, os.path.join(HERE, "parsers"))
        try:
            import jpeg as jpeg_fuzz
            import pdfinc as pdfinc_fuzz
            import png as png_fuzz
            import tfm as tfm_fuzz
            import type1 as type1_fuzz
        finally:
            sys.path.pop()
        self.assertEqual(png_fuzz.classify(-25, b"", False),
                         "output-flood")
        self.assertEqual(tfm_fuzz.classify(-25, ""), "output-flood")
        self.assertEqual(jpeg_fuzz.classify(152, "", False),
                         "output-flood")
        self.assertEqual(png_fuzz.signature("output-flood", -25, b""),
                         "output-flood")
        self.assertEqual(tfm_fuzz.signature("output-flood", -25, ""),
                         "output-flood")
        self.assertEqual(jpeg_fuzz.signature("output-flood", 152, ""),
                         "output-flood")
        # type1 classifies inline in run_once: a self-SIGXFSZ engine
        # (no big write needed) must come out as output-flood.
        killer = make_engine(self.tmp, "killer.sh", SELF_KILL_BODY)
        cls, _rc, _out, _err = type1_fuzz.run_once(
            b"pfb", b"tfm", killer, 10)
        self.assertEqual(cls, "output-flood")
        cls, _rc, _log = pdfinc_fuzz.run_case(
            b"%PDF-1.4\n", killer, 10)
        self.assertEqual(cls, "output-flood")


# Fake engines for the full-log comparison test. The mid-diff pair
# writes ~3 MiB logs differing by one byte in the middle; the flood
# engine tries to write 8 MiB (its own shell process writes, via exec,
# so RLIMIT_FSIZE kills the engine itself with SIGXFSZ).
MIDDIFF_BODY = ("for last do :; done\n"
                "job=${last##*/}; job=${job%%.tex}\n"
                "head -c 1500000 /dev/zero | tr '\\0' 'H' > \"$job.log\"\n"
                "echo \"\" >> \"$job.log\"\n"
                "echo \"MIDLINE-%s\" >> \"$job.log\"\n"
                "head -c 1500000 /dev/zero | tr '\\0' 'T' >> \"$job.log\"\n"
                "echo \"\" >> \"$job.log\"\n"
                "exit 0\n")
FLOOD8_BODY = ("for last do :; done\n"
               "job=${last##*/}; job=${job%%.tex}\n"
               "exec head -c 8388608 /dev/zero > \"$job.log\"\n")

# run_child() below runs run_one()/docgen.run_one() in a child python
# with a 4 MiB file-size cap and cap_text patched to a 64 KiB limit
# (the file-size cap would irreversibly lower the test process's own
# hard limit, so it must live in a subprocess, like the real entry
# points). The patched cap stands in for huge logs with small files:
# code that capped before comparing would drop the mid-log difference
# and report equal; code that compares the full logs reports diverge.


class FullLogCompareTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-fulllog-")
        self.echo = make_engine(self.tmp, "echo.sh", ECHO_BODY)
        self.mid_a = make_engine(self.tmp, "mid-a.sh", MIDDIFF_BODY % "A")
        self.mid_b = make_engine(self.tmp, "mid-b.sh", MIDDIFF_BODY % "B")
        self.flood8 = make_engine(self.tmp, "flood8.sh", FLOOD8_BODY)
        self.root = os.path.dirname(os.path.dirname(HERE))

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_child(self, text, cand, orc, which="run"):
        inp = os.path.join(self.tmp, "in.tex")
        with open(inp, "w") as fh:
            fh.write(text)
        child = "\n".join([
            "import sys",
            "sys.path.insert(0, 'tools/fuzz')",
            "import run as fuzz_run",
            "fuzz_run.apply_fsize_limit()",
            "_real_cap = fuzz_run.cap_text",
            "def _small_cap(text, limit=None):",
            "    return _real_cap(text, 65536)",
            "fuzz_run.cap_text = _small_cap",
            "text = open(sys.argv[1]).read()",
            "if sys.argv[4] == 'docgen':",
            "    import docgen",
            "    res = docgen.run_one(text, sys.argv[2], sys.argv[3],"
            "                         10, return_logs=True)",
            "else:",
            "    res = fuzz_run.run_one(text, sys.argv[2], sys.argv[3],"
            "                           10, return_logs=True)",
            "print('class=' + res[0])",
            "print('caplen=%d/%d' % (len(res[4]), len(res[5])))",
            "print('diff=' + (res[3] or ''))",
        ])
        env = dict(os.environ, FUZZ_FSIZE_LIMIT_BYTES=str(4 * 1024 * 1024))
        proc = subprocess.run(
            [sys.executable, "-c", child, inp, cand, orc, which],
            cwd=self.root, env=env, capture_output=True, text=True,
            timeout=180)
        self.assertEqual(proc.returncode, 0, proc.stderr[-2000:])
        out = {}
        for ln in proc.stdout.splitlines():
            if ln.startswith("class="):
                out["class"] = ln[len("class="):]
            elif ln.startswith("caplen="):
                out["caplen"] = ln[len("caplen="):]
            elif ln.startswith("diff="):
                out["diff"] = ln[len("diff="):]
        return out

    def test_middiff_is_diverge_not_equal(self):
        # ~3 MiB logs (under the 4 MiB file cap) differing by one byte
        # mid-log: the comparison sees the full logs, so diverge, even
        # though the shrunk 64 KiB artifact cap drops the middle.
        out = self.run_child(CALM_TEXT, self.mid_a, self.mid_b)
        self.assertEqual(out["class"], "diverge")
        self.assertIn("MIDLINE", out["diff"])
        # ...while what is kept for artifacts stays capped.
        for part in out["caplen"].split("/"):
            self.assertLessEqual(int(part), 64 * 1024 + 100)

    def test_docgen_middiff_is_diverge_not_equal(self):
        out = self.run_child(CALM_TEXT, self.mid_a, self.mid_b,
                             which="docgen")
        self.assertEqual(out["class"], "diverge")
        self.assertIn("MIDLINE", out["diff"])

    def test_overcap_is_flood(self):
        # An engine that would write 8 MiB under a 4 MiB file cap dies
        # with SIGXFSZ: output-flood, never a comparison.
        out = self.run_child(CALM_TEXT, self.flood8, self.echo)
        self.assertEqual(out["class"], "output-flood")


FLAG = "-cnf-line=shell_escape=f"


def make_argv_engine(tmpdir, name, logpath, extra=""):
    # Records every argv, then behaves like ECHO_BODY (exit 0, FAKE-OK).
    return make_engine(tmpdir, name,
                       "echo \"$@\" >> \"%s\"\n" % logpath + extra
                       + ECHO_BODY)


class ShellEscapeTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-shesc-")
        self.argv_log = os.path.join(self.tmp, "argv.log")
        self.cand = make_argv_engine(self.tmp, "cand.sh", self.argv_log)
        self.orc = make_argv_engine(self.tmp, "orc.sh", self.argv_log)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def argvs(self):
        with open(self.argv_log) as fh:
            return [ln for ln in fh.read().splitlines() if ln.strip()]

    def test_run_one_passes_flag_to_both_engines(self):
        cls = fuzz_run.run_one(SEED_A, self.cand, self.orc, 10)[0]
        self.assertEqual(cls, "equal")
        # Exactly two engine runs (no oracle re-run without a diverge).
        self.assertEqual(len(self.argvs()), 2)
        for argv in self.argvs():
            self.assertIn(FLAG, argv)

    def test_crash_stderr_passes_flag(self):
        body = ("echo \"$@\" >> \"%s\"\n" % self.argv_log
                + "echo boom >&2\nexit 1\n")
        engine = make_engine(self.tmp, "noisy.sh", body)
        fuzz_run.crash_stderr("anything", engine, None, 10)
        self.assertEqual(len(self.argvs()), 1)
        self.assertIn(FLAG, self.argvs()[0])

    def test_parser_jobs_pass_flag(self):
        sys.path.insert(0, os.path.join(HERE, "parsers"))
        try:
            import jpeg as jpeg_fuzz
            import pdfinc as pdfinc_fuzz
            import png as png_fuzz
            import tfm as tfm_fuzz
            import type1 as type1_fuzz
        finally:
            sys.path.pop()
        plain = make_engine(
            self.tmp, "plain.sh",
            "echo \"$@\" >> \"%s\"\nexit 0\n" % self.argv_log)
        tfm_fuzz.run_one(b"blob", plain, 10)
        png_fuzz.run_once(b"png", plain, 10)
        type1_fuzz.run_once(b"pfb", b"tfm", plain, 10)
        jpeg_fuzz.run_one(b"data", plain, 10)
        pdfinc_fuzz.run_case(b"%PDF-1.4\n", plain, 10)
        self.assertEqual(len(self.argvs()), 5)
        for argv in self.argvs():
            self.assertIn(FLAG, argv)

    def test_harness_default_unchanged_for_other_users(self):
        # A fresh process that never imports tools/fuzz still sees the
        # harness restricted default ([]) — the flag lives only in the
        # fuzz process's copy.
        root = os.path.dirname(os.path.dirname(HERE))
        probe = subprocess.run(
            [sys.executable, "-c",
             "import sys; sys.path.insert(0, 'tools/lockstep');"
             "import run as harness; print(repr(harness.ENGINE_SHELL_FLAGS))"],
            cwd=root, capture_output=True, text=True, timeout=30)
        self.assertEqual(probe.returncode, 0)
        self.assertEqual(probe.stdout.strip(), "[]")


UNSEEDED_TEXT = ("\\input prelude\n"
                 "\\message{r=\\the\\pdfuniformdeviate}\n\\end\n")
SEEDED_TEXT = ("\\input prelude\n\\pdfsetrandomseed 12345\n"
               "\\message{r=\\the\\pdfuniformdeviate}\n\\end\n")

# Fake oracle whose log changes every run (a counter file): the first two
# runs always disagree, so any divergence against it is nondeterministic.
COUNTING_ORC_BODY = ("for last do :; done\n"
                     "job=${last##*/}; job=${job%%.tex}\n"
                     "n=$(cat \"%s\" 2>/dev/null || echo 0)\n"
                     "n=$((n + 1)); echo \"$n\" > \"%s\"\n"
                     "printf \"RUN-${n}\\n\" > \"$job.log\"\n"
                     "exit 0\n")


class RandomSeedTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-rand-")
        self.argv_log = os.path.join(self.tmp, "argv.log")
        self.cand = make_argv_engine(self.tmp, "cand.sh", self.argv_log)
        self.orc = make_argv_engine(self.tmp, "orc.sh", self.argv_log)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_has_unseeded_random_read(self):
        self.assertTrue(fuzz_run.has_unseeded_random_read(UNSEEDED_TEXT))
        self.assertTrue(fuzz_run.has_unseeded_random_read(
            "\\message{\\the\\pdfrandomseed}\n"))
        self.assertTrue(fuzz_run.has_unseeded_random_read(
            "\\message{\\the\\pdfnormaldeviate}\n"))
        self.assertFalse(fuzz_run.has_unseeded_random_read(SEEDED_TEXT))
        self.assertFalse(fuzz_run.has_unseeded_random_read(SEED_A))
        # A commented-out read does not count; a longer control word
        # (letters continue the name) is not the primitive.
        self.assertFalse(fuzz_run.has_unseeded_random_read(
            "% \\pdfuniformdeviate\n\\end\n"))
        self.assertFalse(fuzz_run.has_unseeded_random_read(
            "\\pdfuniformdeviateX\n"))
        # A set later in the file means the pre-filter accepts the
        # input (ordering is left to the oracle re-run); a primitive
        # merely named via \string or \meaning reads nothing.
        self.assertFalse(fuzz_run.has_unseeded_random_read(
            "\\message{\\the\\pdfuniformdeviate}\n\\pdfsetrandomseed 1\n"))
        self.assertFalse(fuzz_run.has_unseeded_random_read(
            "\\message{\\string\\pdfuniformdeviate}\n"))
        self.assertFalse(fuzz_run.has_unseeded_random_read(
            "\\message{\\meaning\\pdfrandomseed}\n"))

    def test_unseeded_read_rejected_without_running_engines(self):
        # A seed whose \pdfsetrandomseed was deleted: invalid, and neither
        # engine ever started (no argv recorded).
        res = fuzz_run.run_one(UNSEEDED_TEXT, self.cand, self.orc, 10,
                               return_logs=True)
        self.assertEqual(res[0], "invalid")
        self.assertIsNone(res[1])
        self.assertIsNone(res[2])
        self.assertFalse(os.path.exists(self.argv_log))

    def test_seeded_read_runs(self):
        self.assertEqual(
            fuzz_run.run_one(SEEDED_TEXT, self.cand, self.orc, 10)[0],
            "equal")

    def test_mutator_outputs_never_unseeded(self):
        # Neither the generator nor the seed mutator may emit a random
        # read without a preceding set (today they emit no such
        # primitives at all; the scan locks that in).
        for s in range(100):
            rng = random.Random(s)
            self.assertFalse(fuzz_run.has_unseeded_random_read(
                gen.generate(rng)))
            self.assertFalse(fuzz_run.has_unseeded_random_read(
                gen.mutate(SEED_A, rng)))
            text, _, _ = fuzz_run.draw_input(rng, [("a.tex", SEED_A)])
            self.assertFalse(fuzz_run.has_unseeded_random_read(text))

    def test_changing_reference_is_not_a_finding(self):
        count = os.path.join(self.tmp, "count")
        orc = make_engine(self.tmp, "counting.sh",
                          COUNTING_ORC_BODY % (count, count))
        cls, _, _, diff = fuzz_run.run_one(
            CALM_TEXT, self.cand, orc, 10)
        self.assertEqual(cls, "reference-nondeterministic")
        self.assertIn("oracle logs differ", diff)
        self.assertEqual(fuzz_run.signature(
            cls, 0, "a", 0, "b", diff), "reference-nondeterministic")

    def test_stable_reference_stays_diverge(self):
        cls, _, _, _diff = fuzz_run.run_one(
            CALM_TEXT, self.cand, self.orc, 10)
        # Identical FAKE-OK logs: equal, not nondeterministic.
        self.assertEqual(cls, "equal")


# A document that reads a counter file and writes it back (like a TeX
# \openin/\read/\immediate\openout round-trip): with one workdir per
# engine each side sees a fresh counter, so a correct candidate is
# equal; sharing one dir would let the oracle read the candidate's
# leftover counter and fake nondeterminism.
COUNTER_BODY = ("for last do :; done\n"
                "job=${last##*/}; job=${job%%.tex}\n"
                "n=$(cat counter 2>/dev/null || echo 0)\n"
                "printf 'count=%s\\n' \"$n\" > \"$job.log\"\n"
                "echo $((n + 1)) > counter\n"
                "exit 0\n")
# Same, but the candidate's log carries a planted 1sp box difference.
COUNTER_BUG_BODY = COUNTER_BODY.replace(
    "exit 0\n", "echo 'box0: width 1sp' >> \"$job.log\"\nexit 0\n")

# Fake engines that flood (exit 152, like a SIGXFSZ kill) with 100-line
# logs differing only at line 57.
FLOOD57_BODY = ("for last do :; done\n"
                "job=${last##*/}; job=${job%%.tex}\n"
                "i=1\n"
                "while [ $i -le 100 ]; do\n"
                "  if [ $i -eq 57 ]; then echo \"LINE57-%s\"\n"
                "  else echo \"filler line number $i padding padding\"; fi\n"
                "  i=$((i + 1))\n"
                "done > \"$job.log\"\n"
                "exit 152\n")


class FreshWorkdirTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-fresh-")
        self.correct = make_engine(self.tmp, "correct.sh", COUNTER_BODY)
        self.buggy = make_engine(self.tmp, "buggy.sh", COUNTER_BUG_BODY)
        self.oracle = make_engine(self.tmp, "oracle.sh", COUNTER_BODY)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_counter_roundtrip_equal_with_correct_candidate(self):
        cls = fuzz_run.run_one(SEED_A, self.correct, self.oracle, 10)[0]
        self.assertEqual(cls, "equal")

    def test_counter_roundtrip_diverge_with_box_bug(self):
        cls, _, _, diff = fuzz_run.run_one(
            SEED_A, self.buggy, self.oracle, 10)
        self.assertEqual(cls, "diverge")
        self.assertIn("1sp", diff)


class BothFloodPrefixTest(unittest.TestCase):
    @staticmethod
    def logs(word):
        lines = ["filler line number %d padding" % i
                 for i in range(1, 101)]
        lines[56] = "LINE57-" + word
        return "\n".join(lines) + "\n"

    def test_prefix_difference_is_diverge(self):
        a, b = self.logs("A"), self.logs("B")
        self.assertEqual(
            fuzz_run.classify(152, a, 152, b, False), "diverge")
        self.assertFalse(fuzz_run.flood_prefix_equal(a, b))
        self.assertIn("line 57", fuzz_run.first_diff(152, a, 152, b))

    def test_identical_prefix_is_both_flood(self):
        a = self.logs("A")
        self.assertTrue(fuzz_run.flood_prefix_equal(a, a))
        self.assertEqual(
            fuzz_run.classify(152, a, -25, a, False), "both-flood")
        # A longer tail past the cap is truncation noise, not a diff.
        self.assertEqual(
            fuzz_run.classify(
                152, a, -25, a + "extra tail line\n", False),
            "both-flood")

    def test_flooded_engines_with_line57_diff_diverge(self):
        tmp = tempfile.mkdtemp(prefix="fuzz-flood57-")
        try:
            cand = make_engine(tmp, "c.sh", FLOOD57_BODY % "A")
            orc = make_engine(tmp, "o.sh", FLOOD57_BODY % "B")
            cls, _, _, diff = fuzz_run.run_one(
                SEED_A, cand, orc, 10)
            self.assertEqual(cls, "diverge")
            self.assertIn("line 57", diff)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


class SeedPrefilterTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-seedpre-")
        self.argv_log = os.path.join(self.tmp, "argv.log")
        self.cand = make_argv_engine(self.tmp, "cand.sh", self.argv_log)
        self.orc = make_argv_engine(self.tmp, "orc.sh", self.argv_log)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_seed_before_read_runs(self):
        text = ("\\input prelude\n\\def\\roll{\\pdfuniformdeviate 6}"
                "\\pdfsetrandomseed 42 \\message{\\roll}\n\\end\n")
        self.assertFalse(fuzz_run.has_unseeded_random_read(text))
        self.assertEqual(
            fuzz_run.run_one(text, self.cand, self.orc, 10)[0], "equal")
        self.assertTrue(os.path.exists(self.argv_log))

    def test_string_and_meaning_names_run(self):
        for text in ("\\input prelude\n"
                     "\\message{\\string\\pdfuniformdeviate}\n\\end\n",
                     "\\input prelude\n"
                     "\\message{\\meaning\\pdfrandomseed}\n\\end\n"):
            self.assertFalse(fuzz_run.has_unseeded_random_read(text))
            self.assertEqual(
                fuzz_run.run_one(text, self.cand, self.orc, 10)[0],
                "equal")

    def test_late_seed_left_to_rerun(self):
        text = ("\\input prelude\n\\message{\\the\\pdfuniformdeviate}\n"
                "\\pdfsetrandomseed 1\n\\end\n")
        self.assertFalse(fuzz_run.has_unseeded_random_read(text))
        count = os.path.join(self.tmp, "count")
        orc = make_engine(self.tmp, "counting.sh",
                          COUNTING_ORC_BODY % (count, count))
        cls, _, _, diff = fuzz_run.run_one(text, self.cand, orc, 10)
        self.assertEqual(cls, "reference-nondeterministic")
        self.assertIn("oracle logs differ", diff)

    def test_reference_nondeterministic_stored_once(self):
        seeds = os.path.join(self.tmp, "seeds")
        os.mkdir(seeds)
        with open(os.path.join(seeds, "a.tex"), "w") as fh:
            fh.write(SEED_A)
        count = os.path.join(self.tmp, "count")
        orc = make_engine(self.tmp, "counting.sh",
                          COUNTING_ORC_BODY % (count, count))
        out = os.path.join(self.tmp, "out")
        counts = fuzz_run.main(
            ["--candidate", self.cand, "--oracle", orc,
             "--seeds", seeds, "--out", out, "--iterations", "3",
             "--seed", "1", "--timeout", "10"])
        self.assertEqual(counts["reference-nondeterministic"], 3)
        cls_dir = os.path.join(out, "reference-nondeterministic")
        texs = sorted(f for f in os.listdir(cls_dir)
                      if f.endswith(".tex"))
        # Constant signature: exactly one stored case.
        self.assertEqual(len(texs), 1)
        with open(os.path.join(cls_dir, texs[0][:-4] + ".json")) as fh:
            info = json.load(fh)
        self.assertEqual(info["signature"], "reference-nondeterministic")


class DiffSnippetTest(unittest.TestCase):
    def test_late_difference_window_and_distinct_signatures(self):
        pre = "k" * 250
        a1, b1 = pre + "A\n", pre + "B\n"
        a2, b2 = pre + "C\n", pre + "D\n"
        # Premise: a fixed 160-char cut shows identical text on all four.
        self.assertEqual(a1[:160], b1[:160])
        self.assertEqual(a1[:160], a2[:160])
        self.assertEqual(a1[:160], b2[:160])
        d1 = fuzz_run.first_diff(0, a1, 0, b1)
        d2 = fuzz_run.first_diff(0, a2, 0, b2)
        # The window sits on the differing column and shows the new text.
        self.assertIn("col 251", d1)
        self.assertIn("B", d1)
        self.assertIn("...", d1)
        s1 = fuzz_run.signature("diverge", 0, a1, 0, b1, d1)
        s2 = fuzz_run.signature("diverge", 0, a2, 0, b2, d2)
        self.assertTrue(s1.startswith("diverge:"))
        self.assertNotEqual(s1, s2)

    def test_short_difference_has_no_ellipsis(self):
        d = fuzz_run.first_diff(0, "aaa\n", 0, "aab\n")
        self.assertIn("col 3", d)
        self.assertNotIn("...", d)


class TimeoutSideTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-toside-")
        self.echo = make_engine(self.tmp, "echo.sh", ECHO_BODY)
        self.sleepy = make_engine(self.tmp, "sleep.sh", SLEEP_BODY)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_candidate_hang_names_candidate(self):
        cls, _, _, diff = fuzz_run.run_one(
            SEED_A, self.sleepy, self.echo, 0.5)
        self.assertEqual(cls, "timeout")
        self.assertEqual(diff, "timeout: candidate")
        sig = fuzz_run.signature(cls, None, "", None, "", diff)
        self.assertEqual(sig, "timeout:candidate")

    def test_oracle_hang_names_oracle(self):
        cls, _, _, diff = fuzz_run.run_one(
            SEED_A, self.echo, self.sleepy, 0.5)
        self.assertEqual(cls, "timeout")
        self.assertEqual(diff, "timeout: oracle")
        sig = fuzz_run.signature(cls, None, "", None, "", diff)
        self.assertEqual(sig, "timeout:oracle")

    def test_unparseable_diff_stays_plain_timeout(self):
        self.assertEqual(
            fuzz_run.signature("timeout", None, "", None, "", "stale"),
            "timeout")

    def test_both_sides_hanging_is_both_hang(self):
        # Both sides hanging is both-hang, never timeout:both.
        self.assertEqual(
            fuzz_run.classify(None, "", None, "", (True, True)),
            "both-hang")
        self.assertEqual(
            fuzz_run.signature("both-hang", None, "", None, "",
                               "timeout: candidate/oracle"),
            "both-hang")
        for diff, want in (("timeout: candidate", "timeout:candidate"),
                           ("timeout: oracle", "timeout:oracle")):
            sig = fuzz_run.signature("timeout", None, "", None, "",
                                     diff)
            self.assertEqual(sig, want)
            self.assertNotEqual(sig, "timeout:both")
        # The removed "timeout:both" branch: a diff naming both sides
        # must not produce it either.
        self.assertNotEqual(
            fuzz_run.signature("timeout", None, "", None, "",
                               "timeout: candidate/oracle"),
            "timeout:both")


class CrashStderrBoundTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-stderr-")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_stderr_flood_stays_bounded(self):
        flooder = make_engine(self.tmp, "flooder.sh", STDERR_FLOOD_BODY)
        err = fuzz_run.crash_stderr("anything", flooder, None, 30)
        self.assertIn("stderr flood line", err)
        self.assertLessEqual(len(err.encode("utf-8")), 64 * 1024)


if __name__ == "__main__":
    unittest.main()
