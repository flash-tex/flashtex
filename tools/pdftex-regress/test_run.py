#!/usr/bin/env python3
"""Unit tests for tools/pdftex-regress/run.py (stdlib unittest only)."""

import contextlib
import io
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


class TestSplitLocales(unittest.TestCase):
    def test_partition(self):
        present, missing = run.split_locales({"C", "C.UTF-8", "ja_JP.UTF-8"})
        self.assertEqual(present, ["C.UTF-8", "ja_JP.UTF-8"])
        self.assertEqual(missing, ["C.utf8", "en_US.UTF-8", "en_US.utf8",
                                   "ja_JP.utf8"])

    def test_empty_gives_all_missing(self):
        present, missing = run.split_locales(set())
        self.assertEqual(present, [])
        self.assertEqual(missing, run.WCFNAME_LOCALES)


class TestEngineShims(unittest.TestCase):
    """Self-tests: a broken --engine shim must FAIL the harness, never
    PASS it. All shims run through run_cmd (stdin /dev/null,
    process-group timeout)."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.cache = os.path.join(self.tmp.name, "cache")
        ptests = os.path.join(self.cache, "texk", "web2c", "pdftexdir",
                              "tests")
        os.makedirs(ptests)
        with open(os.path.join(ptests, "expanded.tex"), "wb") as f:
            f.write(b"\\START\nx\n\\END\n\\end\n")
        self.expanded_txt = os.path.join(ptests, "expanded.txt")
        with open(self.expanded_txt, "wb") as f:
            f.write(b"START x\nshow \\output here\nEND y\n")
        os.environ["SHIM_OUT"] = self.tmp.name

    def tearDown(self):
        os.environ.pop("SHIM_OUT", None)
        self.tmp.cleanup()

    def _write_shim(self, name, body):
        path = os.path.join(self.tmp.name, name)
        with open(path, "w", encoding="utf-8") as f:
            f.write(body)
        os.chmod(path, 0o755)
        return path

    def _run_main(self, argv):
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf), \
                contextlib.redirect_stderr(buf):
            code = run.main(argv)
        return code, buf.getvalue()

    def _base_argv(self, shim):
        return ["--engine", shim, "--allow-any-engine", "--cache",
                self.cache, "--tests", "expanded", "--timeout", "60"]

    def test_shim_prints_nothing_fails(self):
        shim = self._write_shim("shim-silent.py",
                                "#!/usr/bin/env python3\n"
                                "import os, sys\n"
                                "data = sys.stdin.buffer.read()\n"
                                "open(os.path.join(os.environ['SHIM_OUT'],\n"
                                "                  'stdin.bin'),\n"
                                "     'wb').write(data)\n")
        code, out = self._run_main(self._base_argv(shim))
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)
        self.assertIn("no expanded.log written", out)
        # stdin really was /dev/null: immediate EOF, zero bytes
        with open(os.path.join(self.tmp.name, "stdin.bin"), "rb") as f:
            self.assertEqual(f.read(), b"")

    def test_shim_changes_one_byte_fails(self):
        shim = self._write_shim("shim-onebyte.py",
                                "#!/usr/bin/env python3\n"
                                "import os\n"
                                "log = (b'banner\\nSTART x\\n'\n"
                                "       b'show \\\\output HERE\\n'\n"
                                "       b'END y\\ntrailer\\n')\n"
                                "open(os.path.join(os.getcwd(),\n"
                                "                  'expanded.log'),\n"
                                "     'wb').write(log)\n")
        code, out = self._run_main(self._base_argv(shim))
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)
        self.assertIn("diff at line 2", out)

    def test_shim_version_lie_rejected(self):
        shim = self._write_shim("shim-lie.sh",
                                "#!/bin/sh\n"
                                "echo 'pdfTeX 3.141592653-2.6-1.40.28"
                                " (TeX Live 2026)'\n")
        code, _ = self._run_main(["--engine", shim, "--cache", self.cache,
                                  "--timeout", "60"])
        self.assertEqual(code, 2)


if __name__ == "__main__":
    unittest.main()
