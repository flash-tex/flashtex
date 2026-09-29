#!/usr/bin/env python3
"""Self-tests for run.py: parser, crash attribution, engine gate.

Stdlib unittest only. No TeX, no network, no checkouts needed:
`python3 tools/latex-suites/test_run.py`.
"""

import os
import shutil
import stat
import sys
import tempfile
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from run import (attribute, engine_deaths, engine_version_firstline,
                 parse_l3build_log, reference_check, run_capture,
                 run_l3build)

PASS_RUN = """Running checks on
  alpha (1/2)
  beta (2/2)

  All checks passed
"""

FAIL_RUN = """Running checks on
  alpha (1/2)
          --> failed
  beta (2/2)

  Check failed with difference files
  - ../../build/test/alpha.pdftex.diff
"""

MULTI_CONFIG = """Running checks on
  zeta (1/1)

  All checks passed

Running l3build with target "check" and configuration "config-TU"
Skipping unknown engine pdftex
No applicable engine requested, config ignored
Running l3build with target "check" and configuration "config-legacy"
Running checks on
  omega (1/1)

  All checks passed
"""

# Real transcript shape when the engine writes nothing: progress line,
# then l3build's Lua assertion on the missing .log (observed with an
# exit-139 shim), process exit 1, no `--> failed`, no .diff.
CRASH_RUN = """Running checks on
  tlb2888 (1/1)
.../l3build-check.lua:99: ../../build/test/tlb2888.log: No such file or directory
stack traceback:
\t[C]: in function 'assert'
"""


class TestParse(unittest.TestCase):

    def test_pass(self):
        ran, completed, failed = parse_l3build_log(PASS_RUN.splitlines(True))
        self.assertEqual(ran, ["alpha", "beta"])
        self.assertEqual(completed, {"alpha", "beta"})
        self.assertEqual(failed, set())

    def test_fail(self):
        ran, completed, failed = parse_l3build_log(FAIL_RUN.splitlines(True))
        self.assertEqual(ran, ["alpha", "beta"])
        self.assertEqual(completed, {"beta"})
        self.assertEqual(failed, {"alpha"})

    def test_config_boundary_kills_stale_current(self):
        # "Skipping unknown engine pdftex" is whole-config, not per-test:
        # it must not fail/skip zeta (previous config's last test).
        ran, completed, failed = parse_l3build_log(
            MULTI_CONFIG.splitlines(True))
        self.assertEqual(ran, ["zeta", "omega"])
        self.assertEqual(completed, {"zeta", "omega"})
        self.assertEqual(failed, set())

    def test_crash_has_no_pass_evidence(self):
        ran, completed, failed = parse_l3build_log(
            CRASH_RUN.splitlines(True))
        self.assertEqual(ran, ["tlb2888"])
        self.assertEqual(completed, set())
        self.assertEqual(failed, set())


class TestAttribute(unittest.TestCase):

    def test_clean_pass(self):
        failed, _ = attribute(["a", "b"], {"a", "b"}, set(), set(), 0,
                              ["a", "b"])
        self.assertEqual(failed, set())

    def test_diff_failure(self):
        failed, _ = attribute(["a", "b"], {"b"}, {"a"}, set(), 1, ["a", "b"])
        self.assertEqual(failed, {"a"})

    def test_crash_in_flight_fails(self):
        # Engine died on tlb2888: started, never resolved -> FAIL, not PASS.
        failed, notes = attribute(["tlb2888"], set(), set(), set(), 1,
                                  ["tlb2888"])
        self.assertEqual(failed, {"tlb2888"})
        self.assertIn("tlb2888", notes)

    def test_writes_nothing_shim_fails(self):
        # Shim producing no output: l3build aborts before any verdict.
        # With an empty build/ (clean runs first) there is no stale
        # result to read, so the run must FAIL, never PASS.
        failed, _ = attribute([], set(), set(), set(), 1, ["t1", "t2"])
        self.assertEqual(failed, {"t1", "t2"})

    def test_late_crash_after_earlier_failure(self):
        # a failed by diff, then l3build died on c: both FAIL, b stays pass.
        failed, _ = attribute(["a", "b", "c"], {"a", "b"}, {"a"}, set(), 1,
                              ["a", "b", "c"])
        self.assertEqual(failed, {"a", "c"})


class TestDeaths(unittest.TestCase):

    def write_log(self, text):
        fd, path = tempfile.mkstemp(prefix="calls-")
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write(text)
        self.addCleanup(os.unlink, path)
        return path

    def test_normal_exits_ignored(self):
        path = self.write_log(
            "rc=0 argv=--fmt=pdflatex -jobname=t1 \\input t1.lvt\n"
            "rc=1 argv=--fmt=pdflatex -jobname=t2 \\input t2.lvt\n"
            "rc=0 argv=-etex -ini pdflatex.ini\n")
        self.assertEqual(engine_deaths(path), set())

    def test_crash_and_foreign_exit_mapped(self):
        path = self.write_log(
            "rc=3 argv=--fmt=pdflatex -jobname=t3 \\input t3.lvt\n"
            "rc=139 argv=--fmt=pdflatex -jobname=t4 \\input t4.lvt\n"
            "rc=139 argv=-etex -ini pdflatex.ini\n"
            "garbage line\n")
        self.assertEqual(engine_deaths(path), {"t3", "t4"})

    def test_missing_log_is_no_evidence(self):
        self.assertEqual(engine_deaths("/nonexistent-calls.log"), set())


_FAKE_ENGINES = []


def make_fake_engine(body):
    fd, path = tempfile.mkstemp(prefix="fake-engine-")
    with os.fdopen(fd, "w", encoding="utf-8") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR)
    _FAKE_ENGINES.append(path)
    return path


def _cleanup_fake_engines():
    for path in _FAKE_ENGINES:
        try:
            os.unlink(path)
        except OSError:
            pass


import atexit as _atexit
_atexit.register(_cleanup_fake_engines)


class TestEngineGate(unittest.TestCase):

    def test_reference_accepted(self):
        eng = make_fake_engine(
            'echo "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)"; exit 0\n')
        self.assertIn("1.40.29", engine_version_firstline(eng))
        self.assertEqual(reference_check(eng, False), (0, None))

    def test_foreign_refused(self):
        eng = make_fake_engine(
            'echo "pdfTeX 3.141592653-2.6-1.40.28 (TeX Live 2025)"; exit 0\n')
        code, msg = reference_check(eng, False)
        self.assertEqual(code, 2)
        self.assertIn("1.40.29", msg)

    def test_foreign_allowed_with_warning(self):
        eng = make_fake_engine(
            'echo "pdfTeX 3.141592653-2.6-1.40.28 (TeX Live 2025)"; exit 0\n')
        code, msg = reference_check(eng, True)
        self.assertEqual(code, 0)
        self.assertIn("warning", msg)

    def test_silent_crash_refused(self):
        eng = make_fake_engine("exit 139\n")
        self.assertIsNone(engine_version_firstline(eng))
        code, _ = reference_check(eng, False)
        self.assertEqual(code, 2)

    def test_nonzero_exit_with_real_banner_passes(self):
        # Shim runs the real engine then exits 3: banner decides, not rc.
        eng = make_fake_engine(
            'echo "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)"; exit 3\n')
        code, _ = reference_check(eng, False)
        self.assertEqual(code, 0)


def make_fake_l3build(body):
    """Executable fake `l3build` script; cleaned up at exit like engines."""
    fd, path = tempfile.mkstemp(prefix="fake-l3build-")
    with os.fdopen(fd, "w", encoding="utf-8") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR)
    _FAKE_ENGINES.append(path)
    return path


class TestTimeout(unittest.TestCase):

    def test_sleeping_l3build_times_out(self):
        fake = make_fake_l3build("exec sleep 30\n")
        t0 = time.monotonic()
        rc, lines, timed_out = run_capture(
            [fake, "check", "-e", "pdftex", "t1"], os.path.expanduser("~"),
            dict(os.environ), timeout=2)
        dt = time.monotonic() - t0
        self.assertTrue(timed_out)
        self.assertLess(dt, 25)  # must not wait out the 30 s sleep
        self.assertIsNotNone(rc)

    def test_process_group_is_killed(self):
        # A grandchild `sleep` shares the process group (no setsid of its
        # own): killing only the direct child would leave it alive.
        fd, pidfile = tempfile.mkstemp(prefix="grandchild-pid-")
        os.close(fd)
        self.addCleanup(os.unlink, pidfile)
        fake = make_fake_l3build(
            "sleep 60 &\necho $! > \"$PIDFILE_OUT\"\nexec sleep 60\n")
        env = dict(os.environ, PIDFILE_OUT=pidfile)
        _, _, timed_out = run_capture(
            [fake, "check"], os.path.expanduser("~"), env, timeout=2)
        self.assertTrue(timed_out)
        with open(pidfile, encoding="utf-8") as fh:
            grandchild = int(fh.read().strip())
        for _ in range(100):
            try:
                os.kill(grandchild, 0)
            except ProcessLookupError:
                break
            time.sleep(0.1)
        else:
            self.fail("grandchild %d survived the process-group kill"
                      % grandchild)

    def test_run_l3build_timeout_fails_unfinished(self):
        workdir = tempfile.mkdtemp(prefix="timeout-workdir-")
        self.addCleanup(shutil.rmtree, workdir, True)
        os.mkdir(os.path.join(workdir, "testfiles"))
        for name in ("t1", "t2"):
            with open(os.path.join(workdir, "testfiles", name + ".lvt"),
                      "w", encoding="utf-8") as fh:
                fh.write("% " + name + "\n")
        engine = make_fake_engine("exit 0\n")
        fake = make_fake_l3build(
            'if [ "$1" = "clean" ]; then exit 0; fi\nexec sleep 30\n')
        fd, logpath = tempfile.mkstemp(prefix="timeout-log-")
        os.close(fd)
        self.addCleanup(os.unlink, logpath)
        t0 = time.monotonic()
        rc, ran, failed, notes, timedout = run_l3build(
            workdir, ["t1", "t2"], engine, logpath, timeout=2,
            l3build_exe=fake)
        dt = time.monotonic() - t0
        self.assertLess(dt, 25)
        self.assertEqual(ran, [])
        self.assertEqual(set(failed), {"t1", "t2"})
        self.assertEqual(timedout, {"t1", "t2"})
        for t in ("t1", "t2"):
            self.assertIn("timeout", notes[t])
        with open(logpath, encoding="utf-8") as fh:
            self.assertIn("TIMEOUT", fh.read())


if __name__ == "__main__":
    unittest.main(verbosity=1)
