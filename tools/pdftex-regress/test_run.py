#!/usr/bin/env python3
"""Unit tests for tools/pdftex-regress/run.py (stdlib unittest only)."""

import contextlib
import io
import os
import shutil
import sys
import tempfile
import time
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
        # whole-token match only: longer/trailing-digit versions must fail
        self.assertFalse(run.version_ok(
            "pdfTeX 3.141592653-2.6-1.40.290 (TeX Live 2026)"))
        self.assertFalse(run.version_ok(
            "pdfTeX 3.141592653-2.6-11.40.29 (TeX Live 2026)"))

    def test_wprob_ok(self):
        good = b"(./pwprob.tex\n./pwprob.tex:12: Could not open file NoSuchFile.eps.\n"
        self.assertTrue(run.wprob_ok(good))
        self.assertFalse(run.wprob_ok(b"no error here\n"))
        # upstream anchors at line start; indented must not match
        self.assertFalse(run.wprob_ok(b" ./pwprob.tex:12: Could not open file NoSuchFile.eps.\n"))

    def test_version_requires_pdftex_name(self):
        # run.py:86-87 -- an arbitrary binary reporting 1.40.29 must not
        # pass the version gate without --allow-any-engine.
        self.assertTrue(run.version_ok(
            "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)"))
        self.assertFalse(run.version_ok("custom-tool-1.40.29"))
        self.assertFalse(run.version_ok("1.40.29"))

    def test_wprob_ok_crlf(self):
        # run.py:104-105 -- CRLF log lines must match like LF ones.
        crlf = (b"(./pwprob.tex\r\n"
                b"./pwprob.tex:12: Could not open file NoSuchFile.eps.\r\n")
        self.assertTrue(run.wprob_ok(crlf))

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
        code, unexp, stale = run.gate_exit(["a", "b"], {"a": "known",
                                                        "b": "known"},
                                           ["z"])
        self.assertEqual((code, unexp, stale), (0, [], []))
        code, unexp, stale = run.gate_exit(["a", "c"], {"a": "known"},
                                           ["z"])
        self.assertEqual((code, unexp, stale), (1, ["c"], []))
        self.assertEqual(run.gate_exit([], {}, []), (0, [], []))


class TestGateStale(unittest.TestCase):
    """A listed-but-now-passing entry rots the list: gate FAILs (exit 1)
    unless --allow-stale is given."""

    def test_stale_entry_fails_gate(self):
        code, unexpected, stale = run.gate_exit(
            ["a"], {"a": "known", "b": "rotted"}, ["b"])
        self.assertEqual(code, 1)
        self.assertEqual(unexpected, [])
        self.assertEqual(stale, ["b"])

    def test_allow_stale_excuses_it(self):
        code, unexpected, stale = run.gate_exit(
            ["a"], {"a": "known", "b": "rotted"}, ["b"],
            allow_stale=True)
        self.assertEqual(code, 0)
        self.assertEqual(unexpected, [])
        self.assertEqual(stale, ["b"])

    def test_expected_but_still_failing_is_not_stale(self):
        code, _, stale = run.gate_exit(["a"], {"a": "known"}, ["x"])
        self.assertEqual((code, stale), (0, []))

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

    @unittest.skipUnless(hasattr(os, "fork"), "needs fork")
    def test_run_cmd_timeout_with_detached_pipe_holder_returns(self):
        # run.py:53 -- a worker that escapes the process group (setsid)
        # and holds the pipes open must not deadlock the post-kill
        # communicate(): the drain is bounded.
        tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, tmp, True)
        shim = os.path.join(tmp, "pipeholder.py")
        with open(shim, "w", encoding="utf-8") as f:
            f.write("import os, time\n"
                    "if os.fork() == 0:\n"
                    "    os.setsid()\n"
                    "    time.sleep(20)\n"
                    "    os._exit(0)\n"
                    "time.sleep(20)\n")
        t0 = time.monotonic()
        rc, _, _, timed_out, _ = run.run_cmd(
            [sys.executable, shim],
            tempfile.gettempdir(), run.base_env({}), 1)
        self.assertTrue(timed_out)
        self.assertIsNone(rc)
        self.assertLess(time.monotonic() - t0, 15)


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
        self.ptests = ptests
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

    def test_cnfline_silent_shim_reports_no_log(self):
        with open(os.path.join(self.ptests, "cnfline.tex"), "wb") as f:
            f.write(b"\\end\n")
        shim = self._write_shim("shim-silent.py",
                                "#!/usr/bin/env python3\n")
        code, out = self._run_main(
            ["--engine", shim, "--allow-any-engine", "--cache",
             self.cache, "--tests", "cnfline", "--timeout", "60"])
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)
        self.assertIn("no cnfline.log written", out)
        self.assertNotIn("harness exception", out)
        self.assertNotIn("FileNotFoundError", out)

    def _write_wtests(self, names):
        wtests = os.path.join(self.cache, "texk", "web2c", "tests")
        os.makedirs(wtests, exist_ok=True)
        for name in names:
            with open(os.path.join(wtests, name), "wb") as f:
                f.write(b"\\end\n")

    def test_partoken_crash_is_fail_not_pass(self):
        # run.py:243-247 -- a SIGSEGV on partoken-xfail (rc=-11) must
        # FAIL, not bypass the `rc == 0` check into a false PASS.
        self._write_wtests(("partoken-ok.tex", "partoken-xfail.tex"))
        shim = self._write_shim("shim-crash.py",
                                "#!/usr/bin/env python3\n"
                                "import os, signal, sys\n"
                                "if any('partoken-xfail' in a "
                                "for a in sys.argv):\n"
                                "    os.kill(os.getpid(), "
                                "signal.SIGSEGV)\n")
        code, out = self._run_main(
            ["--engine", shim, "--allow-any-engine", "--cache",
             self.cache, "--tests", "partoken", "--timeout", "60"])
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)
        self.assertNotIn("xfail exits -11", out)

    def test_wprob_crash_is_fail_not_pass(self):
        # run.py:243-247 -- a crash after writing the expected log line
        # must still FAIL (signal death is not a normal nonzero exit).
        self._write_wtests(("wprob.tex",))
        shim = self._write_shim("shim-wprobcrasher.py",
                                "#!/usr/bin/env python3\n"
                                "import os, signal\n"
                                "open(os.path.join(os.getcwd(),\n"
                                "                  'pwprob.log'),\n"
                                "     'wb').write(b'(./pwprob.tex\\n'\n"
                                " b'./pwprob.tex:12: Could not open file '\n"
                                " b'NoSuchFile.eps.\\n')\n"
                                "os.kill(os.getpid(), signal.SIGSEGV)\n")
        code, out = self._run_main(
            ["--engine", shim, "--allow-any-engine", "--cache",
             self.cache, "--tests", "wprob", "--timeout", "60"])
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)

    def test_relative_engine_path_runs(self):
        # run.py:274 -- `--engine build/shim` must run from the scratch
        # dir instead of raising FileNotFoundError there.
        build = os.path.join(self.tmp.name, "build")
        os.mkdir(build)
        shim_src = self._write_passing_expanded_shim()
        rel_shim = os.path.join(build, "shim-pass.py")
        shutil.copy(shim_src, rel_shim)
        os.chmod(rel_shim, 0o755)
        old = os.getcwd()
        os.chdir(self.tmp.name)
        try:
            code, out = self._run_main(
                ["--engine", os.path.join("build", "shim-pass.py"),
                 "--allow-any-engine", "--cache", self.cache,
                 "--tests", "expanded", "--timeout", "60"])
        finally:
            os.chdir(old)
        self.assertEqual(code, 0)
        self.assertIn("PASS", out)

    def test_explicit_missing_ttf2afm_is_config_error(self):
        # run.py:267-271 -- a mistyped --ttf2afm path must exit 2, not
        # silently SKIP the test.
        shim = self._write_shim("shim-true.sh", "#!/bin/sh\nexit 0\n")
        code, out = self._run_main(
            ["--engine", shim, "--allow-any-engine", "--cache",
             self.cache, "--tests", "ttf2afm", "--timeout", "60",
             "--ttf2afm", "/opt/bin/ttf2afm_typo"])
        self.assertEqual(code, 2)
        self.assertIn("--ttf2afm", out)

    def _write_passing_expanded_shim(self):
        return self._write_shim("shim-pass.py",
                                "#!/usr/bin/env python3\n"
                                "import os\n"
                                "log = (b'banner\\nSTART x\\n'\n"
                                "       b'show \\\\pdfoutput here\\n'\n"
                                "       b'END y\\ntrailer\\n')\n"
                                "open(os.path.join(os.getcwd(),\n"
                                "                  'expanded.log'),\n"
                                "     'wb').write(log)\n")

    def _run_with_expected_file(self, argv, text):
        path = os.path.join(self.tmp.name, "EXPECTED-FAILURES.txt")
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        old, run.EXPECTED_FAILURES = run.EXPECTED_FAILURES, path
        try:
            return self._run_main(argv)
        finally:
            run.EXPECTED_FAILURES = old

    def test_stale_expected_failure_fails_gate(self):
        shim = self._write_passing_expanded_shim()
        code, out = self._run_with_expected_file(
            self._base_argv(shim), "expanded: stale entry\n")
        self.assertEqual(code, 1)
        self.assertIn("stale", out)

    def test_allow_stale_excuses_passing_listed_test(self):
        shim = self._write_passing_expanded_shim()
        code, out = self._run_with_expected_file(
            self._base_argv(shim) + ["--allow-stale"],
            "expanded: stale entry\n")
        self.assertEqual(code, 0)
        self.assertIn("stale", out)

    def test_hanging_shim_budget_exhausts_remaining(self):
        shim = self._write_shim("shim-hang.py",
                                "#!/usr/bin/env python3\n"
                                "import time\n"
                                "time.sleep(60)\n")
        code, out = self._run_main(
            ["--engine", shim, "--allow-any-engine", "--cache",
             self.cache, "--tests", "pdftex,expanded", "--timeout", "2",
             "--budget", "1"])
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)
        self.assertIn("budget exhausted", out)


if __name__ == "__main__":
    unittest.main()
