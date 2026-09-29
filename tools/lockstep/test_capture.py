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

    def test_capture_stale_workdir_reuse(self):
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-stale-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            tex = os.path.join(workdir, "001-edef-basic.tex")
            shutil.copy(os.path.join(lockstep_run.CASES_DIR,
                                     "001-edef-basic.tex"), tex)
            first = lockstep_run.capture(tex, "pdftex", workdir)
            self.assertEqual(first.returncode, 0)
            self.assertTrue(first.log.strip())
            self.assertIsNotNone(first.pdf_path)
            keep_aux = os.path.join(workdir, "001-edef-basic.aux")
            with open(keep_aux, "w") as fh:
                fh.write("caller-owned aux must survive\n")
            fake = os.path.join(workdir, "fake139.sh")
            with open(fake, "w") as fh:
                fh.write("#!/bin/sh\nexit 139\n")
            os.chmod(fake, 0o755)
            cap = lockstep_run.capture(tex, fake, workdir)
            self.assertNotEqual(cap.returncode, 0)
            self.assertEqual(cap.returncode, 139)
            self.assertEqual(cap.log, "")
            self.assertIsNone(cap.pdf_path)
            self.assertTrue(os.path.isfile(keep_aux))
        finally:
            shutil.rmtree(workdir, ignore_errors=True)

    def test_capture_037_lastbox_single_shipout(self):
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-037-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            tex = os.path.join(workdir, "037-lastbox.tex")
            shutil.copy(os.path.join(lockstep_run.CASES_DIR,
                                     "037-lastbox.tex"), tex)
            cap = lockstep_run.capture(tex, "pdftex", workdir)
            self.assertEqual(cap.returncode, 0)
            self.assertEqual(cap.log.count(
                "Completed box being shipped out"), 1)
            self.assertEqual(len(cap.boxes), 1)
            self.assertIn("Completed box being shipped out", cap.boxes[0])
        finally:
            shutil.rmtree(workdir, ignore_errors=True)

    def test_capture_twopage_pdflatex_two_boxes(self):
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        fmt_check = shutil.which("kpsewhich") is None or os.system(
            "kpsewhich -engine=pdftex pdflatex.fmt >/dev/null 2>&1") != 0
        if fmt_check:
            self.skipTest("pdflatex format not available")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-2page-")
        try:
            tex = os.path.join(workdir, "twopage.tex")
            with open(tex, "w") as fh:
                fh.write("\\documentclass{article}\n"
                         "\\tracingoutput=1\n"
                         "\\begin{document}\n"
                         "Page one.\n"
                         "\\newpage\n"
                         "Page two.\n"
                         "\\end{document}\n")
            cap = lockstep_run.capture(tex, "pdftex", workdir,
                                       fmt="pdflatex")
            self.assertEqual(cap.returncode, 0)
            self.assertEqual(cap.log.count(
                "Completed box being shipped out"), 2)
            self.assertEqual(len(cap.boxes), 2)
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
