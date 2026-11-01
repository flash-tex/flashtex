#!/usr/bin/env python3
"""Strict-minimiser tests: decoy rejection, --loose, crash signatures.

Run as `python3 -m unittest discover -s tools/fuzz` from the repo root.
Engines are small shell scripts written to a temp dir (in the style of
tools/lockstep/test_capture.py); the real candidate is never used.
"""
import contextlib
import io
import os
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import minimize
import run as fuzz_run


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, 0o755)
    return path


ECHO_BODY = ("for last do :; done\n"
             "job=${last##*/}; job=${job%%.tex}\n"
             "printf 'FAKE-OK\\n' > \"$job.log\"\n"
             "exit 0\n")

# Diverge pair with a same-shape decoy. With REAL in the input the engines
# differ on a box width 1sp wider at log line 4; with DECOY (and no REAL)
# they differ on the same masked shape at the earlier log line 2. Both
# masked diffs read: line N: '\boxwidth=NNN.NNNNNpt' vs
# '\boxwidth=NNN.NNNNNpt', so --loose cannot tell them apart but strict
# (raw pair) can.
REAL_DECOY_CAND_BODY = (
    "for last do :; done\n"
    "job=${last##*/}; job=${job%%.tex}\n"
    "{\n"
    "printf 'BASE one\\n'\n"
    "if grep -q 'REAL' \"$last\" 2>/dev/null; then\n"
    "  printf 'pad pad\\nmid mid\\n\\\\boxwidth=100.00002pt\\n'\n"
    "elif grep -q 'DECOY' \"$last\" 2>/dev/null; then\n"
    "  printf '\\\\boxwidth=100.00003pt\\nmid mid\\nsame same\\n'\n"
    "else\n"
    "  printf 'pad pad\\nmid mid\\nsame same\\n'\n"
    "fi\n"
    "printf 'BASE tail\\n'\n"
    "} > \"$job.log\"\n"
    "exit 0\n")
REAL_DECOY_ORC_BODY = (
    "for last do :; done\n"
    "job=${last##*/}; job=${job%%.tex}\n"
    "{\n"
    "printf 'BASE one\\n'\n"
    "if grep -q 'REAL' \"$last\" 2>/dev/null; then\n"
    "  printf 'pad pad\\nmid mid\\n\\\\boxwidth=100.00001pt\\n'\n"
    "elif grep -q 'DECOY' \"$last\" 2>/dev/null; then\n"
    "  printf '\\\\boxwidth=100.00001pt\\nmid mid\\nsame same\\n'\n"
    "else\n"
    "  printf 'pad pad\\nmid mid\\nsame same\\n'\n"
    "fi\n"
    "printf 'BASE tail\\n'\n"
    "} > \"$job.log\"\n"
    "exit 0\n")

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


class StrictDecoyTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-strict-")
        self.cand = make_engine(self.tmp, "cand.sh",
                                REAL_DECOY_CAND_BODY)
        self.orc = make_engine(self.tmp, "orc.sh", REAL_DECOY_ORC_BODY)
        self.text = ("%% filler one\n"
                     "a line with REAL in it\n"
                     "%% filler two\n"
                     "a line with DECOY in it\n"
                     "%% filler three\n")
        cls, _, _, diff = fuzz_run.run_one(
            self.text, self.cand, self.orc, 10)
        self.assertEqual(cls, "diverge")
        # Sanity: the decoy really is a decoy of the same masked shape.
        decoy_only = self.text.replace("REAL", "GONE")
        cls2, _, _, diff2 = fuzz_run.run_one(
            decoy_only, self.cand, self.orc, 10)
        self.assertEqual(cls2, "diverge")
        self.assertEqual(fuzz_run.normalised_diff(diff2),
                         fuzz_run.normalised_diff(diff))
        self.assertNotEqual(diff2, diff)
        self.input_path = os.path.join(self.tmp, "case.tex")
        with open(self.input_path, "w") as fh:
            fh.write(self.text)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_minimize(self, *extra):
        out = os.path.join(self.tmp, "min.tex")
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            ret = minimize.main(
                ["--candidate", self.cand, "--oracle", self.orc,
                 "--input", self.input_path, "--class", "diverge",
                 "--out", out, "--timeout", "10"] + list(extra))
        self.assertEqual(ret, 0)
        with open(out) as fh:
            return fh.read(), buf.getvalue()

    def test_strict_keeps_real_drops_decoy(self):
        small, out = self.run_minimize()
        self.assertIn("REAL", small)
        self.assertNotIn("DECOY", small)
        self.assertIn("mode: strict", out)
        cls, _, _, diff = fuzz_run.run_one(
            small, self.cand, self.orc, 10)
        self.assertEqual(cls, "diverge")

    def test_loose_may_take_decoy(self):
        small, out = self.run_minimize("--loose")
        self.assertIn("mode: loose", out)
        # Loose accepts the same-shape decoy, so ddmin drops REAL first.
        self.assertIn("DECOY", small)
        self.assertNotIn("REAL", small)


class CrashSignatureStillRequiredTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fuzz-minsig-")
        self.crash = make_engine(self.tmp, "trigger.sh", TRIGGER_BODY)
        self.echo = make_engine(self.tmp, "echo.sh", ECHO_BODY)
        filler = "".join("%% filler line %d\n" % i for i in range(10))
        self.text = (filler + "\\message{has TRIGGER-A here}\n"
                     + "\\message{has TRIGGER-B here}\n")
        self.input_path = os.path.join(self.tmp, "case.tex")
        with open(self.input_path, "w") as fh:
            fh.write(self.text)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def sig_of(self, text):
        res = fuzz_run.run_one(text, self.crash, self.echo, 10,
                               return_logs=True)
        self.assertEqual(res[0], "candidate-crash")
        err = fuzz_run.crash_stderr(text, self.crash,
                                    fuzz_run.candidate_env(), 10)
        return fuzz_run.crash_signature(res[1], res[4], err)

    def test_loose_crash_keeps_identical_signature(self):
        out = os.path.join(self.tmp, "min.tex")
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            ret = minimize.main(
                ["--candidate", self.crash, "--oracle", self.echo,
                 "--input", self.input_path, "--class", "candidate-crash",
                 "--out", out, "--timeout", "10", "--loose"])
        self.assertEqual(ret, 0)
        with open(out) as fh:
            small = fh.read()
        # --loose only loosens diverge: crash minimisation still needs
        # the identical stderr-based signature (panic site A, not B).
        self.assertIn("TRIGGER-A", small)
        self.assertNotIn("TRIGGER-B", small)
        self.assertEqual(self.sig_of(small), self.sig_of(self.text))


class MinimizeShellEscapeTest(unittest.TestCase):
    def test_every_engine_argv_carries_flag(self):
        tmp = tempfile.mkdtemp(prefix="fuzz-minsh-")
        try:
            argv_log = os.path.join(tmp, "argv.log")
            rec = "echo \"$@\" >> \"%s\"\n" % argv_log
            crash = make_engine(tmp, "c.sh", rec + TRIGGER_BODY)
            echo = make_engine(tmp, "o.sh", rec + ECHO_BODY)
            filler = "".join("%% filler %d\n" % i for i in range(6))
            inp = os.path.join(tmp, "case.tex")
            with open(inp, "w") as fh:
                fh.write(filler + "\\message{has TRIGGER-A here}\n")
            out = os.path.join(tmp, "min.tex")
            with contextlib.redirect_stdout(io.StringIO()):
                ret = minimize.main(
                    ["--candidate", crash, "--oracle", echo,
                     "--input", inp, "--class", "candidate-crash",
                     "--out", out, "--timeout", "10"])
            self.assertEqual(ret, 0)
            with open(argv_log) as fh:
                argvs = [ln for ln in fh.read().splitlines() if ln.strip()]
            # minimize runs both engines (plus crash re-runs): every argv
            # went through run.run_one/crash_stderr, so all carry the flag.
            self.assertGreater(len(argvs), 2)
            for argv in argvs:
                self.assertIn("-cnf-line=shell_escape=f", argv)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
