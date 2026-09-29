#!/usr/bin/env python3
"""Unit test for tools/lockstep/run.py:capture(). Stdlib unittest only.

Run as `python3 -m unittest tools.lockstep.test_capture` from the repo
root, or directly as `python3 tools/lockstep/test_capture.py`.
"""
import inspect
import os
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import run as lockstep_run


class CaptureTest(unittest.TestCase):
    def test_capture_case001_reference(self):
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            shutil.copy(os.path.join(lockstep_run.CASES_DIR,
                                     "001-edef-basic.tex"),
                        os.path.join(workdir, "001-edef-basic.tex"))
            sentinel = os.path.join(workdir, "sentinel.txt")
            with open(sentinel, "w") as fh:
                fh.write("capture must not wipe the workdir\n")
            cap = lockstep_run.capture(
                os.path.join(workdir, "001-edef-basic.tex"),
                "pdftex", workdir, extra_env={"LOCKSTEP_TEST": "1"})
            self.assertIs(type(cap.log), str)
            self.assertTrue(cap.log)
            self.assertEqual(cap.returncode, 0)
            self.assertIn("entering extended mode", cap.log)
            self.assertEqual(len(cap.boxes), 1)
            self.assertIs(type(cap.boxes[0]), str)
            self.assertIn("Completed box being shipped out", cap.boxes[0])
            self.assertIsNotNone(cap.pdf_path)
            self.assertTrue(os.path.isfile(cap.pdf_path))
            self.assertTrue(os.path.isfile(sentinel))
        finally:
            shutil.rmtree(workdir, ignore_errors=True)

    def test_capture_signature(self):
        sig = inspect.signature(lockstep_run.capture)
        params = list(sig.parameters.values())
        self.assertEqual([p.name for p in params[:3]],
                         ["tex_path", "engine_bin", "workdir"])
        for name in ("fmt", "extra_env"):
            self.assertEqual(sig.parameters[name].kind,
                             inspect.Parameter.KEYWORD_ONLY)
            self.assertIsNone(sig.parameters[name].default)


if __name__ == "__main__":
    unittest.main()
