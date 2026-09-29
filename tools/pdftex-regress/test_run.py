#!/usr/bin/env python3
"""Unit tests for tools/pdftex-regress/run.py (stdlib unittest only)."""

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import run  # noqa: E402


class TestExpandedNormalise(unittest.TestCase):
    def test_window_and_pdf_replacement(self):
        log = (b"banner noise\nSTART of block\nshow \\pdfoutput here\n"
               b"END of block\ntrailing noise\n")
        self.assertEqual(run.expanded_normalise(log),
                         b"START of block\nshow \\output here\nEND of block\n")

    def test_start_end_same_line(self):
        log = b"x\nSTART one END\ny\n"
        self.assertEqual(run.expanded_normalise(log), b"START one END\n")

    def test_no_markers_gives_empty(self):
        self.assertEqual(run.expanded_normalise(b"just noise\n"), b"")


class TestParsers(unittest.TestCase):
    def test_version_ok(self):
        self.assertTrue(run.version_ok(
            "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)"))
        self.assertFalse(run.version_ok("pdfTeX 3.141592653-2.6-1.40.28"))
        self.assertFalse(run.version_ok(""))

    def test_wprob_ok(self):
        good = b"(./pwprob.tex\n./pwprob.tex:12: Could not open file NoSuchFile.eps.\n"
        self.assertTrue(run.wprob_ok(good))
        self.assertFalse(run.wprob_ok(b"no error here\n"))
        # upstream anchors at line start; indented must not match
        self.assertFalse(run.wprob_ok(b" ./pwprob.tex:12: Could not open file NoSuchFile.eps.\n"))

    def test_parse_expected_failures(self):
        with tempfile.NamedTemporaryFile("w", suffix=".txt",
                                         delete=False) as f:
            f.write("# comment\n\nwprob: missing include bug\nbare-name\n")
            path = f.name
        try:
            self.assertEqual(run.parse_expected_failures(path),
                             {"wprob": "missing include bug", "bare-name": ""})
        finally:
            os.unlink(path)
        self.assertEqual(run.parse_expected_failures(path + ".missing"), {})


class TestFailurePaths(unittest.TestCase):
    def test_gate_exit(self):
        code, unexp = run.gate_exit(["a", "b"], {"a": "known", "b": "known"})
        self.assertEqual((code, unexp), (0, []))
        code, unexp = run.gate_exit(["a", "c"], {"a": "known"})
        self.assertEqual((code, unexp), (1, ["c"]))
        self.assertEqual(run.gate_exit([], {}), (0, []))

    def test_run_cmd_reports_exit_code(self):
        rc, _, _, timed_out, _ = run.run_cmd(
            [sys.executable, "-c", "import sys; sys.exit(3)"],
            tempfile.gettempdir(), run.base_env({}), 60)
        self.assertFalse(timed_out)
        self.assertEqual(rc, 3)

    def test_run_cmd_timeout_kills(self):
        rc, _, _, timed_out, secs = run.run_cmd(
            [sys.executable, "-c", "import time; time.sleep(60)"],
            tempfile.gettempdir(), run.base_env({}), 1)
        self.assertTrue(timed_out)
        self.assertIsNone(rc)
        self.assertLess(secs, 30)


if __name__ == "__main__":
    unittest.main()
