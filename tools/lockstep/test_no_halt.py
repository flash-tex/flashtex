#!/usr/bin/env python3
"""Tests for the `% lockstep: no-halt` case option of run.py. Stdlib unittest only.

Run as `python3 -m unittest tools.lockstep.test_no_halt` from the repo root.
Tests that run an engine need pdftex on PATH and skip without it.
"""
import contextlib
import io
import os
import shutil
import stat
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import run as lockstep_run

SHOW = (r"\input prelude" "\n"
        r"\show\par" "\n"
        r"\message{AFTER-SHOW}" "\n"
        r"\setbox0=\hbox{\vrule width5pt height5pt}" "\n"
        r"\lsshipbox0" "\n"
        r"\end" "\n")


def write(path, text):
    with open(path, "w") as fh:
        fh.write(text)


class CaseOptionsTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="lockstep-nohalt-")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def opts(self, text):
        p = os.path.join(self.tmp, "c.tex")
        write(p, text)
        return lockstep_run.case_options(p)

    def test_no_option(self):
        self.assertEqual(self.opts("% c: x\n% new versus none: y\n\\end\n"),
                         frozenset())

    def test_option_on_line_one_two_or_three(self):
        for text in ("% lockstep: no-halt\n% a\n% b\n",
                     "% a\n% lockstep: no-halt\n% b\n",
                     "% a\n% b\n%lockstep:   no-halt  \n"):
            self.assertEqual(self.opts(text), frozenset({"no-halt"}), text)

    def test_option_on_line_four_is_not_read(self):
        self.assertEqual(self.opts("% a\n% b\n% c\n% lockstep: no-halt\n"),
                         frozenset())

    def test_unknown_option_is_an_error(self):
        with self.assertRaises(ValueError):
            self.opts("% a\n% lockstep: no-hault\n")


class FakeCasesTest(unittest.TestCase):
    """main() on a temporary cases directory, with the real pdftex."""

    def setUp(self):
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        self.tmp = tempfile.mkdtemp(prefix="lockstep-nohalt-")
        self.saved = lockstep_run.CASES_DIR
        lockstep_run.CASES_DIR = os.path.join(self.tmp, "cases")
        os.mkdir(lockstep_run.CASES_DIR)

    def tearDown(self):
        lockstep_run.CASES_DIR = self.saved
        shutil.rmtree(self.tmp, ignore_errors=True)

    def case(self, name, header):
        write(os.path.join(lockstep_run.CASES_DIR, name + ".tex"),
              header + SHOW)

    def run_main(self, *argv):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), \
                contextlib.redirect_stderr(io.StringIO()):
            rc = lockstep_run.main(list(argv))
        return rc, out.getvalue()

    def wrapper(self, exit_code):
        """A 'candidate' that runs the real pdftex and then exits `exit_code`
        whenever pdftex exited nonzero."""
        path = os.path.join(self.tmp, "cand-%d.sh" % exit_code)
        write(path, "#!/bin/sh\n%s \"$@\"\nrc=$?\n[ $rc -ne 0 ] && exit %d\n"
              "exit 0\n" % (shutil.which("pdftex"), exit_code))
        os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR)
        return path

    def test_halted_by_default_so_show_is_not_a_case(self):
        self.case("t1", "% t1: show without the opt-out\n")
        rc, out = self.run_main("--self-test", "--cases", "t1")
        self.assertEqual(rc, 1, out)
        self.assertIn("FAIL t1", out)

    def test_no_halt_show_case_passes_the_self_test(self):
        self.case("t2", "% t2: show with the opt-out\n% lockstep: no-halt\n")
        rc, out = self.run_main("--self-test", "--cases", "t2")
        self.assertEqual(rc, 0, out)
        self.assertIn("PASS t2", out)

    def test_no_halt_run_continues_past_the_error(self):
        self.case("t3", "% t3: x\n% lockstep: no-halt\n")
        self.case("t3h", "% t3h: x\n")
        halted = lockstep_run.run_engine("pdftex", "t3h")
        free = lockstep_run.run_engine("pdftex", "t3")
        self.assertNotIn("AFTER-SHOW", halted["log"])
        self.assertIn("AFTER-SHOW", free["log"])
        self.assertEqual((halted["returncode"], free["returncode"]), (1, 1))
        self.assertTrue(free["no_halt"])
        self.assertIn("> \\par=\\par.", free["terminal"])

    def test_no_halt_with_a_clean_run_is_a_case_error(self):
        write(os.path.join(lockstep_run.CASES_DIR, "t4.tex"),
              "% t4: opts out but never errors\n% lockstep: no-halt\n"
              "\\input prelude\n\\setbox0=\\hbox{\\vrule width5pt}\n"
              "\\lsshipbox0\n\\end\n")
        rc, out = self.run_main("--self-test", "--cases", "t4")
        self.assertEqual(rc, 1, out)
        self.assertIn("case error", out)

    def test_same_exit_code_passes_and_a_different_one_fails(self):
        self.case("t5", "% t5: x\n% lockstep: no-halt\n")
        rc, out = self.run_main("--engine", self.wrapper(1), "--cases", "t5")
        self.assertEqual(rc, 0, out)
        rc, out = self.run_main("--engine", self.wrapper(3), "--cases", "t5")
        self.assertEqual(rc, 1, out)
        self.assertIn("returncode reference=1 vs candidate=3", out)

    def test_unknown_option_fails_the_case(self):
        self.case("t6", "% t6: x\n% lockstep: no-hault\n")
        rc, out = self.run_main("--self-test", "--cases", "t6")
        self.assertEqual(rc, 1, out)
        self.assertIn("unknown lockstep option", out)


class ReturncodeRulesTest(unittest.TestCase):
    def check(self, ref, cand):
        with contextlib.redirect_stdout(io.StringIO()):
            return lockstep_run.check_returncodes("n", ref, cand, "candidate")

    def test_default_still_requires_exit_zero(self):
        self.assertTrue(self.check({"returncode": 0}, {"returncode": 0}))
        self.assertFalse(self.check({"returncode": 1}, {"returncode": 1}))

    def test_no_halt_requires_equal_nonzero_codes(self):
        ref = {"returncode": 1, "no_halt": True}
        self.assertTrue(self.check(ref, {"returncode": 1}))
        self.assertFalse(self.check(ref, {"returncode": 0}))
        self.assertFalse(self.check(ref, {"returncode": 2}))
        self.assertFalse(self.check({"returncode": 0, "no_halt": True},
                                    {"returncode": 0}))


if __name__ == "__main__":
    unittest.main()
