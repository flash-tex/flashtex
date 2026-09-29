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


class TestWcfnameStub(unittest.TestCase):
    """Finding D: a do-nothing stub (exit 0, empty job files) must FAIL.

    Uses fakes for `locale`, `kpsewhich` and `perl` on PATH so the test
    needs no TeX installation; the engine stub touches empty job
    .txt/.log/.fls files and empty pdftests/<doc>-tmp.tex files."""

    DOCS = ["fn-utf8", "fn\xa3\xa5\xb5\xc6\xc7\xf1\xdf-utf8",
            "fn\u3055\u3056\u6ce2-utf8",
            "fn\u0394\u0414\u0926\u30c0\u6253\ub2e4\U0001d56f\U0001f389-utf8"]

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cache = os.path.join(self.tmp.name, "cache")
        os.makedirs(os.path.join(self.cache, "texk", "web2c", "tests"))
        os.makedirs(os.path.join(self.cache, "texk", "web2c", "pdftexdir",
                                 "tests"))
        with open(os.path.join(self.cache, "texk", "web2c", "tests",
                               "fn-utf8.txt"), "wb") as f:
            f.write("abc \u03b1\u03b2\u03b3 \u0430\u0431\u0432\n".encode("utf-8"))
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


if __name__ == "__main__":
    unittest.main()
