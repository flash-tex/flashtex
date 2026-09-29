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

    def test_expanded_crash_after_good_log_fails(self):
        # Finding A -- a shim that writes the correct expanded.log then
        # dies by SIGSEGV: the log matches, but the signal death must
        # FAIL (upstream's reference engine exits 1, never a crash).
        shim = self._write_shim("shim-expcrash.py",
                                "#!/usr/bin/env python3\n"
                                "import os, signal\n"
                                "log = (b'banner\\nSTART x\\n'\n"
                                "       b'show \\\\pdfoutput here\\n'\n"
                                "       b'END y\\ntrailer\\n')\n"
                                "open(os.path.join(os.getcwd(),\n"
                                "                  'expanded.log'),\n"
                                "     'wb').write(log)\n"
                                "os.kill(os.getpid(), signal.SIGSEGV)\n")
        code, out = self._run_main(self._base_argv(shim))
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)
        self.assertNotIn("matches expanded.txt", out)

    def test_expanded_exit2_after_good_log_fails(self):
        # Finding A -- a correct expanded.log with engine exit 2 must
        # FAIL: only 0/1 are tolerated (1 = reference 'No pages of
        # output').
        shim = self._write_shim("shim-exp2.py",
                                "#!/usr/bin/env python3\n"
                                "import os, sys\n"
                                "log = (b'banner\\nSTART x\\n'\n"
                                "       b'show \\\\pdfoutput here\\n'\n"
                                "       b'END y\\ntrailer\\n')\n"
                                "open(os.path.join(os.getcwd(),\n"
                                "                  'expanded.log'),\n"
                                "     'wb').write(log)\n"
                                "sys.exit(2)\n")
        code, out = self._run_main(self._base_argv(shim))
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)
        self.assertNotIn("matches expanded.txt", out)

    def test_expanded_exit0_and_exit1_with_good_log_pass(self):
        # Guard against over-tightening: the tolerated codes (0, 1)
        # with a matching log must still PASS.
        for want in (0, 1):
            shim = self._write_shim("shim-exp%d.py" % want,
                                    "#!/usr/bin/env python3\n"
                                    "import os, sys\n"
                                    "log = (b'banner\\nSTART x\\n'\n"
                                    "       b'show \\\\pdfoutput here\\n'\n"
                                    "       b'END y\\ntrailer\\n')\n"
                                    "open(os.path.join(os.getcwd(),\n"
                                    "                  'expanded.log'),\n"
                                    "     'wb').write(log)\n"
                                    "sys.exit(%d)\n" % want)
            code, out = self._run_main(self._base_argv(shim))
            self.assertEqual(code, 0, out)
            self.assertIn("PASS", out)

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

    def test_expanded_symlink_leak_fails(self):
        # Finding B -- the expected file must be unreachable from the
        # engine: a shim that symlinks expanded.log at the expected
        # expanded.txt found via $TEXINPUTS passes while the upstream
        # checkout is on the search path.
        shim = self._write_shim("shim-symlink.py",
                                "#!/usr/bin/env python3\n"
                                "import os\n"
                                "d = os.environ.get('TEXINPUTS', '').split(':')[0]\n"
                                "src = os.path.join(d, 'expanded.txt')\n"
                                "dst = os.path.join(os.getcwd(), 'expanded.log')\n"
                                "if os.path.isfile(src):\n"
                                "    os.symlink(src, dst)\n"
                                "else:\n"
                                "    open(dst, 'wb').write(b'leak unavailable\\n')\n")
        code, out = self._run_main(self._base_argv(shim))
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)
        self.assertNotIn("matches expanded.txt", out)

    def test_expanded_read_wrap_leak_fails(self):
        # Finding B -- a shim that reads the expected expanded.txt via
        # $TEXINPUTS and wraps it in banner/trailer noise passes while
        # the checkout leaks through the environment.
        shim = self._write_shim("shim-wrap.py",
                                "#!/usr/bin/env python3\n"
                                "import os\n"
                                "d = os.environ.get('TEXINPUTS', '').split(':')[0]\n"
                                "src = os.path.join(d, 'expanded.txt')\n"
                                "dst = os.path.join(os.getcwd(), 'expanded.log')\n"
                                "if os.path.isfile(src):\n"
                                "    data = open(src, 'rb').read()\n"
                                "    open(dst, 'wb').write(b'banner\\n' + data\n"
                                "                         + b'trailer\\n')\n"
                                "else:\n"
                                "    open(dst, 'wb').write(b'no leak\\n')\n")
        code, out = self._run_main(self._base_argv(shim))
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)
        self.assertNotIn("matches expanded.txt", out)

    def test_ttf2afm_sibling_cat_cheat_fails(self):
        # Finding B -- a fake helper that derives the expected .afm as
        # a sibling of its absolute argv path passes while the runner
        # hands it an absolute path into the checkout.
        for stem in ("postV3", "postV7"):
            with open(os.path.join(self.ptests, stem + ".ttf"), "wb") as f:
                f.write(b"fake-ttf")
            with open(os.path.join(self.ptests, stem + ".afm"), "wb") as f:
                f.write(b"%!PS-AdobeFont " + stem.encode() + b"\nFoo 1\n")
        helper = self._write_shim("shim-cat.py",
                                  "#!/usr/bin/env python3\n"
                                  "import os, sys\n"
                                  "afm = os.path.splitext(sys.argv[1])[0] + '.afm'\n"
                                  "try:\n"
                                  "    sys.stdout.buffer.write(open(afm, 'rb').read())\n"
                                  "except OSError:\n"
                                  "    sys.exit(1)\n")
        engine = self._write_shim("shim-true.py", "#!/usr/bin/env python3\n")
        code, out = self._run_main(
            ["--engine", engine, "--allow-any-engine", "--cache",
             self.cache, "--tests", "ttf2afm", "--timeout", "60",
             "--ttf2afm", helper])
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)

    def test_all_skipped_suite_is_nonzero(self):
        # Finding C -- `--tests ttf2afm,pdftosrc` with no helper binaries
        # reports `PASS 0 / FAIL 0 / SKIP 2`, which must not exit 0.
        shim = self._write_shim("shim-true.sh", "#!/bin/sh\nexit 0\n")
        code, out = self._run_main(
            ["--engine", shim, "--allow-any-engine", "--cache",
             self.cache, "--tests", "ttf2afm,pdftosrc", "--timeout", "60"])
        self.assertIn("PASS 0 / FAIL 0 / SKIP 2", out)
        self.assertNotEqual(code, 0)
        self.assertTrue("explicitly" in out or "no test produced" in out)

    def test_named_but_skipped_helper_is_nonzero(self):
        # Finding C -- a passing run that SKIPped an explicitly named
        # test must still exit nonzero and name the skipped test.
        shim = self._write_passing_expanded_shim()
        code, out = self._run_main(
            ["--engine", shim, "--allow-any-engine", "--cache",
             self.cache, "--tests", "expanded,ttf2afm", "--timeout", "60"])
        self.assertIn("PASS", out)
        self.assertIn("SKIP", out)
        self.assertNotEqual(code, 0)
        self.assertIn("ttf2afm", out)

    def test_pdfimage_empty_fmt_stub_fails(self):
        # Finding E -- a stub that touches an empty pdfimage.fmt and
        # exits 0 twice (no PDF/log) must FAIL on the missing artifacts.
        self._write_wtests(("basic.tex", "1-4.jpg", "B.pdf",
                            "lily-ledger-broken.png"))
        with open(os.path.join(self.ptests, "pdfimage.tex"), "wb") as f:
            f.write(b"\\end\n")
        shim = self._write_shim("shim-fmtstub.py",
                                "#!/usr/bin/env python3\n"
                                "import os, sys\n"
                                "if '-ini' in sys.argv:\n"
                                "    open(os.path.join(os.getcwd(),\n"
                                "                      'pdfimage.fmt'),\n"
                                "         'wb').close()\n")
        code, out = self._run_main(
            ["--engine", shim, "--allow-any-engine", "--cache",
             self.cache, "--tests", "pdfimage", "--timeout", "60"])
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)


class TestWcfnameJobOk(unittest.TestCase):
    """Unit cover for run.wcfname_job_ok: pdfTeX's \\write escapes
    non-ASCII bytes as ^^XX, so the gate decodes before comparing."""

    WANT = "abc \u03b1\u03b2\u03b3 \u0430\u0431\u0432 \u3042\u30a2\u203b\uffe5 \u5929\u5730\u4eba\n".encode("utf-8")
    GOT = ("abc ^^ce^^b1^^ce^^b2^^ce^^b3 ^^d0^^b0^^d0^^b1^^d0^^b2 "
           "^^e3^^81^^82^^e3^^82^^a2^^e2^^80^^bb^^ef^^bf^^a5 "
           "^^e5^^a4^^a9^^e5^^9c^^b0^^e4^^ba^^ba\n").encode("ascii")

    def test_caret_escapes_decode_and_match(self):
        self.assertTrue(run.wcfname_job_ok(self.GOT, self.WANT))

    def test_tampered_byte_fails(self):
        self.assertFalse(run.wcfname_job_ok(
            self.GOT.replace(b"^^b1", b"^^b2"), self.WANT))

    def test_plain_forgery_and_empty_fail(self):
        self.assertFalse(run.wcfname_job_ok(b"abc FORGED\n", self.WANT))
        self.assertFalse(run.wcfname_job_ok(b"", self.WANT))

    def test_caret_encode_matches_engine_spelling(self):
        self.assertEqual(run.caret_encode(b"fn-utf8-pdf"), b"fn-utf8-pdf")
        self.assertEqual(
            run.caret_encode("fn\u3055\u3056\u6ce2-utf8-pdf".encode("utf-8")),
            b"fn^^e3^^81^^95^^e3^^81^^96^^e6^^b3^^a2-utf8-pdf")

    def test_wrapped_marker_found(self):
        job = "fn\u0394-utf8-pdf"
        marker = b"JOB[" + run.caret_encode(job.encode("utf-8")) + b"]"
        wrapped = marker[:70] + b"\n" + marker[70:] + b" :: We are in x\n"
        self.assertTrue(run.term_has_job(wrapped, job))

    def test_raw_utf8_marker_rejected(self):
        # A stub printing the job name in plain UTF-8 (not the engine's
        # ^^XX spelling) must not satisfy the marker check.
        job = "fn\u3055\u3056\u6ce2-utf8"
        raw = ("JOB[" + job + "-pdf] :: ok\n").encode("utf-8")
        self.assertFalse(run.term_has_job(raw, job + "-pdf"))
        self.assertFalse(run.term_has_job(b"", "fn-utf8-pdf"))


class _WcfnameBase(unittest.TestCase):
    """Fake-cache + PATH-fake (`locale`, `kpsewhich`, `perl`) scaffolding
    shared by the wcfname stub tests, so no TeX installation is needed."""

    DOCS = ["fn-utf8", "fn\xa3\xa5\xb5\xc6\xc7\xf1\xdf-utf8",
            "fn\u3055\u3056\u6ce2-utf8",
            "fn\u0394\u0414\u0926\u30c0\u6253\ub2e4\U0001d56f\U0001f389-utf8"]
    WANT = "abc \u03b1\u03b2\u03b3 \u0430\u0431\u0432\n".encode("utf-8")

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cache = os.path.join(self.tmp.name, "cache")
        os.makedirs(os.path.join(self.cache, "texk", "web2c", "tests"))
        os.makedirs(os.path.join(self.cache, "texk", "web2c", "pdftexdir",
                                 "tests"))
        with open(os.path.join(self.cache, "texk", "web2c", "tests",
                               "fn-utf8.txt"), "wb") as f:
            f.write(self.WANT)
        # The harness copies this input before running the engine stub;
        # without it t_wcfname FAILs at the copy (harness exception)
        # instead of exercising the stub's artifacts.
        with open(os.path.join(self.cache, "texk", "web2c", "tests",
                               "fn-generate.perl"), "wb") as f:
            f.write(b"# fake generator (PATH fake perl does the work)\n")
        bindir = os.path.join(self.tmp.name, "bin")
        os.mkdir(bindir)

        def _tool(name, body):
            path = os.path.join(bindir, name)
            with open(path, "w", encoding="utf-8") as f:
                f.write(body)
            os.chmod(path, 0o755)

        _tool("locale", "#!/bin/sh\necho C.UTF-8\n")
        _tool("kpsewhich", "#!/bin/sh\nexit 0\n")
        lines = ["#!/bin/sh", 'd="$4"', 'mkdir -p "$d"']
        for doc in self.DOCS + ["fn\xb1\xd7\xf7\xa7\xb6-utf8"]:
            lines.append("printf 'x\\n' > \"$d/%s.tex\"" % doc)
        _tool("perl", "\n".join(lines) + "\n")
        old = os.environ.get("PATH", "")
        os.environ["PATH"] = bindir + os.pathsep + old
        self.addCleanup(os.environ.__setitem__, "PATH", old)

    def _run_wcfname(self, stub):
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf), \
                contextlib.redirect_stderr(buf):
            code = run.main(["--engine", stub, "--allow-any-engine",
                             "--cache", self.cache, "--tests", "wcfname",
                             "--timeout", "60"])
        return code, buf.getvalue()


class TestWcfnameStub(_WcfnameBase):
    """Finding D: a do-nothing stub (exit 0, empty job files) must FAIL.

    The engine stub touches empty job .txt/.log/.fls files and empty
    pdftests/<doc>-tmp.tex files."""

    def test_wcfname_empty_stub_fails(self):
        body = ["#!/usr/bin/env python3", "import os"]
        body.append("docs = %r" % (self.DOCS,))
        body.append("for doc in docs:")
        body.append("    job = doc + '-pdf'")
        body.append("    for ext in ('.txt', '.log', '.fls'):")
        body.append("        open(job + ext, 'wb').close()")
        body.append("    open('pdftests/' + doc + '-tmp.tex', 'wb').close()")
        path = os.path.join(self.tmp.name, "stub-empty.py")
        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join(body) + "\n")
        os.chmod(path, 0o755)
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf), \
                contextlib.redirect_stderr(buf):
            code = run.main(["--engine", path, "--allow-any-engine",
                             "--cache", self.cache, "--tests", "wcfname",
                             "--timeout", "60"])
        out = buf.getvalue()
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)


class _ShimBase(unittest.TestCase):
    """Shared scaffolding for the gate-hardening shim tests (slice 8):
    a fake texlive cache plus an executable-shim writer and a main()
    runner. Each test asserts post-fix behaviour, so it FAILS before
    the run.py fix and PASSES after."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cache = os.path.join(self.tmp.name, "cache")
        self.ptests = os.path.join(self.cache, "texk", "web2c",
                                   "pdftexdir", "tests")
        self.wtests = os.path.join(self.cache, "texk", "web2c", "tests")
        os.makedirs(self.ptests)
        os.makedirs(self.wtests)

    def _wcache(self, *parts_and_data):
        *parts, data = parts_and_data
        path = os.path.join(self.cache, *parts)
        with open(path, "wb") as f:
            f.write(data)
        return path

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

    def _argv(self, shim, tests, extra=()):
        return (["--engine", shim, "--allow-any-engine", "--cache",
                 self.cache, "--tests", tests, "--timeout", "60"]
                + list(extra))

    EXP_LOG = (b"banner\nSTART x\nshow \\pdfoutput here\nEND y\ntrailer\n")
    EXP_TXT = b"START x\nshow \\output here\nEND y\n"

    def _write_expanded_cache(self):
        self._wcache("texk", "web2c", "pdftexdir", "tests",
                     "expanded.tex", b"\\START\nx\n\\END\n\\end\n")
        self._wcache("texk", "web2c", "pdftexdir", "tests",
                     "expanded.txt", self.EXP_TXT)


class TestStrictArtifacts(_ShimBase):
    """Finding 2: artifact checks must reject anything that is not a
    regular file (symlinks, dirs, missing)."""

    def test_nonempty_rejects_non_regular(self):
        d = os.path.join(self.tmp.name, "art")
        os.mkdir(d)
        reg = os.path.join(d, "reg.bin")
        with open(reg, "wb") as f:
            f.write(b"x")
        link = os.path.join(d, "link.bin")
        os.symlink(reg, link)
        dangling = os.path.join(d, "dangling.bin")
        os.symlink(os.path.join(d, "nope.bin"), dangling)
        self.assertTrue(run.nonempty(reg))
        self.assertFalse(run.nonempty(d + "-missing"))
        empty = os.path.join(d, "empty.bin")
        open(empty, "wb").close()
        self.assertFalse(run.nonempty(empty))
        self.assertFalse(run.nonempty(link))
        self.assertFalse(run.nonempty(dangling))
        self.assertFalse(run.nonempty(d))

    def test_expanded_symlink_to_good_log_fails(self):
        # The log content is exactly right, but expanded.log is a
        # symlink at the artifact path: pre-fix this PASSes (getsize
        # follows the link), post-fix it must FAIL.
        self._write_expanded_cache()
        shim = self._write_shim("shim-linklog.py",
                                "#!/usr/bin/env python3\n"
                                "import os\n"
                                "real = os.path.join(os.getcwd(), 'real.log')\n"
                                "open(real, 'wb').write(%r)\n"
                                "os.symlink(real, os.path.join(os.getcwd(),\n"
                                "             'expanded.log'))\n"
                                % (self.EXP_LOG,))
        code, out = self._run_main(self._argv(shim, "expanded"))
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)
        self.assertIn("expanded.log", out)
        self.assertNotIn("matches expanded.txt", out)


class TestPdfimageContent(_ShimBase):
    """Finding 1: garbage non-empty pdfimage artifacts must FAIL; the
    PDF must be a %PDF- file ending in %%EOF and the log must carry
    the reference run's marker lines."""

    def _write_pdfimage_cache(self):
        self._wcache("texk", "web2c", "pdftexdir", "tests",
                     "pdfimage.tex", b"\\end\n")
        for name in ("basic.tex", "1-4.jpg", "B.pdf",
                     "lily-ledger-broken.png"):
            self._wcache("texk", "web2c", "tests", name, b"input\n")

    def test_garbage_artifacts_fail(self):
        self._write_pdfimage_cache()
        shim = self._write_shim("shim-garbage.py",
                                "#!/usr/bin/env python3\n"
                                "import os, sys\n"
                                "w = os.getcwd()\n"
                                "if '-ini' in sys.argv:\n"
                                "    open(os.path.join(w, 'pdfimage.fmt'),\n"
                                "         'wb').write(b'GARBAGE-FMT')\n"
                                "else:\n"
                                "    open(os.path.join(w, 'pdfimage.pdf'),\n"
                                "         'wb').write(b'GARBAGE-PDF')\n"
                                "    open(os.path.join(w, 'pdfimage.log'),\n"
                                "         'wb').write(b'GARBAGE-LOG\\n')\n")
        code, out = self._run_main(self._argv(shim, "pdfimage"))
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)

    def test_symlink_artifacts_fail(self):
        # Real non-empty files exist, but every artifact path is a
        # symlink (e.g. to /bin/sh): pre-fix PASSes, post-fix FAILs.
        self._write_pdfimage_cache()
        shim = self._write_shim("shim-links.py",
                                "#!/usr/bin/env python3\n"
                                "import os, sys\n"
                                "w = os.getcwd()\n"
                                "real = os.path.join(w, 'real.bin')\n"
                                "open(real, 'wb').write(b'x' * 64)\n"
                                "names = (['pdfimage.fmt'] if '-ini' in sys.argv\n"
                                "         else ['pdfimage.pdf', 'pdfimage.log'])\n"
                                "for n in names:\n"
                                "    p = os.path.join(w, n)\n"
                                "    try:\n"
                                "        os.unlink(p)\n"
                                "    except OSError:\n"
                                "        pass\n"
                                "    os.symlink(real, p)\n")
        code, out = self._run_main(self._argv(shim, "pdfimage"))
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)
        self.assertIn("not a regular file", out)


class TestPartokenLogs(_ShimBase):
    """Finding 3: an argv-sniffing engine that exits 0/1 but writes no
    logs must FAIL; both reference logs carry marker lines."""

    def _write_partoken_cache(self):
        for name in ("partoken-ok.tex", "partoken-xfail.tex"):
            self._wcache("texk", "web2c", "tests", name, b"\\end\n")

    def test_argv_sniffer_writing_nothing_fails(self):
        self._write_partoken_cache()
        shim = self._write_shim("shim-sniff.py",
                                "#!/usr/bin/env python3\n"
                                "import sys\n"
                                "sys.exit(0 if any('partoken-ok' in a\n"
                                "                 for a in sys.argv) else 1)\n")
        code, out = self._run_main(self._argv(shim, "partoken"))
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)
        self.assertIn("partoken-ok.log", out)

    def test_logs_without_markers_fail(self):
        self._write_partoken_cache()
        shim = self._write_shim("shim-fakelog.py",
                                "#!/usr/bin/env python3\n"
                                "import os, sys\n"
                                "name = ('partoken-ok.log'\n"
                                "        if any('partoken-ok' in a\n"
                                "               for a in sys.argv)\n"
                                "        else 'partoken-xfail.log')\n"
                                "open(os.path.join(os.getcwd(), name),\n"
                                "     'wb').write(b'forged log line\\n')\n"
                                "sys.exit(0 if name == 'partoken-ok.log'\n"
                                "         else 1)\n")
        code, out = self._run_main(self._argv(shim, "partoken"))
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)
        self.assertIn("PAR-TOKEN", out)


class TestComparisonSemantics(_ShimBase):
    """Finding 4: pdftosrc must not mask every CR byte (only the CRLF
    mapping upstream's probed diff applies); ttf2afm must not re-add a
    missing final newline."""

    def test_xref_equal_crlf_mapping(self):
        lf = b"xref\n0 1\n0000000000 65535 f\n"
        crlf = b"xref\r\n0 1\r\n0000000000 65535 f\r\n"
        self.assertTrue(run.xref_equal(crlf, lf, strip_cr=True))
        self.assertFalse(run.xref_equal(crlf, lf, strip_cr=False))

    def test_xref_equal_lone_cr_never_masked(self):
        lf = b"xref\n0 1\n0000000000 65535 f\n"
        lone = b"xref\n0 1\n0000000000 6553\r5 f\n"
        self.assertFalse(run.xref_equal(lone, lf, strip_cr=True))
        self.assertFalse(run.xref_equal(lone, lf, strip_cr=False))

    def test_ttf2afm_normalise_keeps_endings(self):
        out = (b"FontName Foo\r\nConverted at someday\n"
               b"Body 1\nNoTrailingNewline")
        self.assertEqual(run.ttf2afm_normalise(out),
                         b"FontName Foo\r\nBody 1\nNoTrailingNewline")

    def test_pdf_wellformed(self):
        self.assertTrue(run.pdf_wellformed(b"%PDF-1.4\nbody\n%%EOF\n"))
        self.assertTrue(run.pdf_wellformed(b"%PDF-1.4\nbody\n%%EOF"))
        self.assertFalse(run.pdf_wellformed(b"GARBAGE-PDF"))
        self.assertFalse(run.pdf_wellformed(b"%PDF-1.4\nbody\n"))
        self.assertFalse(run.pdf_wellformed(b"body\n%%EOF\n"))

    def test_ttf2afm_missing_final_newline_fails(self):
        body = b"%!PS-AdobeFont body\nSecond 2\n"
        for stem in ("postV3", "postV7"):
            self._wcache("texk", "web2c", "pdftexdir", "tests",
                         stem + ".ttf", b"fake-ttf")
            self._wcache("texk", "web2c", "pdftexdir", "tests",
                         stem + ".afm", body)
        helper = self._write_shim("shim-noeol.py",
                                  "#!/usr/bin/env python3\n"
                                  "import sys\n"
                                  "sys.stdout.buffer.write(%r)\n" % (body[:-1],))
        engine = self._write_shim("shim-true.py", "#!/usr/bin/env python3\n")
        code, out = self._run_main(
            self._argv(engine, "ttf2afm", ["--ttf2afm", helper]))
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)

    def test_pdftosrc_lone_cr_fails(self):
        want = b"xref\n0 1\n0000000000 65535 f\n"
        got = b"xref\n0 1\n0000000000 6553\r5 f\n"
        for stem in ("test-13", "test-15"):
            self._wcache("texk", "web2c", "pdftexdir", "tests",
                         stem + ".pdf", b"fake-pdf")
            self._wcache("texk", "web2c", "pdftexdir", "tests",
                         stem + ".xref", want)
        helper = self._write_shim("shim-lonecr.py",
                                  "#!/usr/bin/env python3\n"
                                  "import os, sys\n"
                                  "stem = os.path.splitext(\n"
                                  "    os.path.basename(sys.argv[1]))[0]\n"
                                  "open(stem + '.xref',\n"
                                  "     'wb').write(%r)\n" % (got,))
        engine = self._write_shim("shim-true.py", "#!/usr/bin/env python3\n")
        code, out = self._run_main(
            self._argv(engine, "pdftosrc", ["--pdftosrc", helper]))
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)


class TestWcfnameJobLogMarker(_WcfnameBase):
    """Finding 5: job.log must carry the per-document JOB marker (the
    generated inputs \\write16 it, so the reference logs have it);
    non-empty alone is not enough."""

    def _stub(self):
        lines = ["#!/usr/bin/env python3",
                 "import os, sys",
                 "want = %r" % (self.WANT,),
                 "docs = %r" % (self.DOCS,),
                 "def enc(raw):",
                 "    out = bytearray()",
                 "    for b in raw:",
                 "        if 32 <= b <= 126:",
                 "            out.append(b)",
                 "        else:",
                 "            out.extend(('^^%02x' % b).encode('ascii'))",
                 "    return bytes(out)",
                 "argv = sys.argv[1:]",
                 "job = [a.split('=', 1)[1] for a in argv",
                 "       if a.startswith('-jobname=')][0]",
                 "doc = argv[-1][:-4]",
                 "marker = b'JOB[' + enc(job.encode('utf-8')) + b']'",
                 "open(job + '.txt', 'wb').write(enc(want))",
                 "log = b'some engine log line\\n'",
                 "if not os.environ.get('WSTUB_NOMARKER'):",
                 "    log = marker + b' :: We are in test\\n' + log",
                 "open(job + '.log', 'wb').write(log)",
                 "open(job + '.fls', 'wb').write(b'INPUT x\\n')",
                 "open(os.path.join('pdftests', doc + '-tmp.tex'),",
                 "     'wb').write(b'\\\\relax\\n')",
                 "sys.stdout.buffer.write(marker + b' :: We are in test\\n')"]
        path = os.path.join(self.tmp.name, "stub-wc.py")
        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines) + "\n")
        os.chmod(path, 0o755)
        return path

    def test_joblog_without_job_marker_fails(self):
        os.environ["WSTUB_NOMARKER"] = "1"
        self.addCleanup(os.environ.pop, "WSTUB_NOMARKER", None)
        code, out = self._run_wcfname(self._stub())
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)
        self.assertIn("JOB[", out)

    def test_joblog_with_job_marker_passes(self):
        os.environ.pop("WSTUB_NOMARKER", None)
        code, out = self._run_wcfname(self._stub())
        self.assertEqual(code, 0, out)
        self.assertIn("PASS", out)


class TestContainment(_ShimBase):
    """Finding 6: the engine runs with HOME/TMPDIR/TEXMF* contained in
    the per-test dir; writes outside the work dir or beyond the
    documented allowlist FAIL the gate."""

    def test_escape_to_scratch_parent_fails(self):
        self._write_expanded_cache()
        shim = self._write_shim("shim-escape.py",
                                "#!/usr/bin/env python3\n"
                                "import os\n"
                                "parent = os.path.dirname(os.getcwd())\n"
                                "open(os.path.join(parent, 'escaped.txt'),\n"
                                "     'w').write('pwned')\n"
                                "open(os.path.join(os.getcwd(),\n"
                                "                  'expanded.log'),\n"
                                "     'wb').write(%r)\n" % (self.EXP_LOG,))
        code, out = self._run_main(self._argv(shim, "expanded"))
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)
        self.assertIn("escaped.txt", out)

    def test_extra_file_in_workdir_fails(self):
        self._write_expanded_cache()
        shim = self._write_shim("shim-extra.py",
                                "#!/usr/bin/env python3\n"
                                "import os\n"
                                "open(os.path.join(os.getcwd(), 'bonus.bin'),\n"
                                "     'wb').write(b'\\x00' * 16)\n"
                                "open(os.path.join(os.getcwd(),\n"
                                "                  'expanded.log'),\n"
                                "     'wb').write(%r)\n" % (self.EXP_LOG,))
        code, out = self._run_main(self._argv(shim, "expanded"))
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)
        self.assertIn("bonus.bin", out)

    def test_contained_env_vars_pass(self):
        # The shim refuses to run unless HOME/TMPDIR/TEXMFVAR/
        # TEXMFCONFIG/TEXMFHOME all point inside its work dir.
        self._write_expanded_cache()
        shim = self._write_shim("shim-contained.py",
                                "#!/usr/bin/env python3\n"
                                "import os, sys\n"
                                "w = os.getcwd()\n"
                                "keys = ('HOME', 'TMPDIR', 'TEXMFVAR',\n"
                                "        'TEXMFCONFIG', 'TEXMFHOME')\n"
                                "vals = {k: os.environ.get(k, '') for k in keys}\n"
                                "inside = all(v and os.path.realpath(v).startswith(\n"
                                "    os.path.realpath(w) + os.sep) for v in vals.values())\n"
                                "if not inside:\n"
                                "    sys.exit(9)\n"
                                "open(os.path.join(w, 'expanded.log'),\n"
                                "     'wb').write(%r)\n" % (self.EXP_LOG,))
        code, out = self._run_main(self._argv(shim, "expanded"))
        self.assertEqual(code, 0, out)
        self.assertIn("PASS", out)


class TestPdftexSmoke(_ShimBase):
    """Finding 7 (test half): --version/--help alone are not enough;
    the pdftex test needs a non-empty log from a real run."""

    def test_version_help_only_stub_fails(self):
        shim = self._write_shim("shim-quiet.sh", "#!/bin/sh\nexit 0\n")
        code, out = self._run_main(self._argv(shim, "pdftex"))
        self.assertEqual(code, 1, out)
        self.assertIn("FAIL", out)
        self.assertIn("smoke.log", out)

    def test_smoke_log_stub_passes(self):
        shim = self._write_shim("shim-smoke.sh",
                                "#!/bin/sh\necho smoke > smoke.log\nexit 0\n")
        code, out = self._run_main(self._argv(shim, "pdftex"))
        self.assertEqual(code, 0, out)
        self.assertIn("PASS", out)


if __name__ == "__main__":
    unittest.main()
