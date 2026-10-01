#!/usr/bin/env python3
"""Self-tests for run.py: parser, crash attribution, engine gate.

Stdlib unittest only. No TeX, no network, no checkouts needed:
`python3 tools/latex-suites/test_run.py`.
"""

import os
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from run import (KILL_GRACE, attribute, collect_diffs, diffline_re,
                 dir_label, dir_summary, engine_deaths, engine_version_firstline, gate,
                 hash_diff_files, list_tests, load_expected, main,
                 make_shim, make_shims, normalise_diff, parse_engine_env,
                 parse_l3build_log, reference_check, run_capture,
                 run_l3build, snapshot_diffs, summarize,
                 update_baseline_file)

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

    def test_longer_version_token_refused(self):
        # Substring gate accepted a fake "1.40.290" engine as the 1.40.29
        # reference: the version must match as a whole token.
        eng = make_fake_engine(
            'echo "pdfTeX 3.141592653-2.6-1.40.290 (TeX Live 2027)"; exit 0\n')
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
        rc, ran, failed, notes, timedout, info = run_l3build(
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


    def _reaped(self, pid, timeout=10):
        """True once os.kill(pid, 0) says the pid is gone."""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                os.kill(pid, 0)
            except ProcessLookupError:
                return True
            except PermissionError:
                return False
            time.sleep(0.1)
        return False

    def test_sigterm_ignoring_child_returns_bounded(self):
        # Blocker: an engine that traps and ignores SIGTERM (the exact
        # `trap '' TERM; while :; do sleep 1; done` shim from the report)
        # must not hang run_capture: SIGKILL must follow the grace period
        # and nothing may survive.
        fd, pidfile = tempfile.mkstemp(prefix="sigterm-child-pid-")
        os.close(fd)
        self.addCleanup(os.unlink, pidfile)
        fake = make_fake_l3build(
            "echo $$ > \"$PIDFILE_OUT\"\n"
            "trap '' TERM; while :; do sleep 1; done\n")
        env = dict(os.environ, PIDFILE_OUT=pidfile)
        t0 = time.monotonic()
        rc, _, timed_out = run_capture(
            [fake, "check"], os.path.expanduser("~"), env, timeout=2)
        dt = time.monotonic() - t0
        self.assertTrue(timed_out)
        self.assertLess(dt, 2 + KILL_GRACE + 15)
        self.assertIsNotNone(rc)
        with open(pidfile, encoding="utf-8") as fh:
            child = int(fh.read().strip())
        self.assertTrue(self._reaped(child),
                        "SIGTERM-ignoring child %d survived" % child)

    def test_sigterm_ignoring_grandchild_returns_bounded(self):
        # Blocker cause (1)+(2): the group leader (l3build) exits on
        # SIGTERM within the grace period, so a wait-then-maybe-KILL
        # never SIGKILLs; the surviving TERM-ignoring grandchild holds
        # the stdout pipe open and closing it deadlocks the reader.
        # run_capture must still return within timeout + KILL_GRACE +
        # a few seconds with no survivor.
        fd, pidfile = tempfile.mkstemp(prefix="sigterm-grandchild-pid-")
        os.close(fd)
        self.addCleanup(os.unlink, pidfile)
        fake = make_fake_l3build(
            "( trap '' TERM; while :; do sleep 1; done ) &\n"
            "echo $! > \"$PIDFILE_OUT\"\n"
            "exec sleep 60\n")
        env = dict(os.environ, PIDFILE_OUT=pidfile)
        t0 = time.monotonic()
        rc, _, timed_out = run_capture(
            [fake, "check"], os.path.expanduser("~"), env, timeout=2)
        dt = time.monotonic() - t0
        self.assertTrue(timed_out)
        self.assertLess(dt, 2 + KILL_GRACE + 15)
        self.assertIsNotNone(rc)
        with open(pidfile, encoding="utf-8") as fh:
            grandchild = int(fh.read().strip())
        self.assertTrue(self._reaped(grandchild),
                        "SIGTERM-ignoring grandchild %d survived"
                        % grandchild)


class TestExpectedFile(unittest.TestCase):

    def write_expected(self, text):
        fd, path = tempfile.mkstemp(prefix="expected-")
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write(text)
        self.addCleanup(os.unlink, path)
        return path

    def test_legacy_line_has_no_sha(self):
        path = self.write_expected("t1: some old reason\n")
        self.assertEqual(load_expected(path), {"t1": (None, "some old reason")})

    def test_sha_line_parsed(self):
        sha = "ab12" * 16
        path = self.write_expected("t1: sha256=%s some reason\n" % sha)
        self.assertEqual(load_expected(path), {"t1": (sha, "some reason")})

    def test_update_baseline_rewrites_token(self):
        old, new = "ab12" * 16, "cd34" * 16
        path = self.write_expected(
            "# comment stays\nt1: legacy reason\nt2: sha256=%s r2\n" % old)
        n = update_baseline_file(path, {"t1": new, "t2": new, "t3": new})
        self.assertEqual(n, 2)  # t3 has no line: never invented
        with open(path, encoding="utf-8") as fh:
            text = fh.read()
        self.assertIn("# comment stays\n", text)
        self.assertIn("t1: sha256=%s legacy reason\n" % new, text)
        self.assertIn("t2: sha256=%s r2\n" % new, text)


class TestDiffHash(unittest.TestCase):

    DIFF_A = (b"*** old.tlg\tTue Sep 29 05:26:49 2026\n"
              b"--- old.log\tTue Sep 29 05:26:49 2026\n"
              b"***************\n*** 4,10 ****\n! ignored error: x\n--- 4,10 ----\n"
              b"! ignored: x\n")
    DIFF_B = (b"*** new.tlg\tWed Sep 30 05:26:49 2026\n"
              b"--- new.log\tWed Sep 30 05:26:49 2026\n"
              b"***************\n*** 4,10 ****\n! ignored error: x\n--- 4,10 ----\n"
              b"! ignored: x\n")

    def test_normalise_ignores_headers_and_dates(self):
        self.assertEqual(normalise_diff(self.DIFF_A),
                         normalise_diff(self.DIFF_B))
        self.assertIn(b"*** 4,10 ****", normalise_diff(self.DIFF_A))

    def test_collect_scopes_to_this_run(self):
        workdir = tempfile.mkdtemp(prefix="diffscope-")
        self.addCleanup(shutil.rmtree, workdir, True)
        build = os.path.join(workdir, "build", "test")
        os.makedirs(build)
        stale = os.path.join(build, "stale.pdftex.diff")
        with open(stale, "wb") as fh:
            fh.write(self.DIFF_A)
        before = snapshot_diffs(workdir)
        fresh = os.path.join(build, "fresh.pdftex.diff")
        with open(fresh, "wb") as fh:
            fh.write(self.DIFF_B)
        found = collect_diffs(workdir, before)
        self.assertEqual(set(found), {"fresh"})

    def test_run_l3build_reports_diffhash(self):
        workdir = tempfile.mkdtemp(prefix="diffhash-workdir-")
        self.addCleanup(shutil.rmtree, workdir, True)
        os.mkdir(os.path.join(workdir, "testfiles"))
        with open(os.path.join(workdir, "testfiles", "t1.lvt"), "w",
                  encoding="utf-8") as fh:
            fh.write("% t1\n")
        engine = make_fake_engine("exit 0\n")
        fd, diffsrc = tempfile.mkstemp(prefix="srcdir-diff-")
        with os.fdopen(fd, "wb") as fh:
            fh.write(self.DIFF_A)
        self.addCleanup(os.unlink, diffsrc)
        build_out = os.path.join(workdir, "build", "test")
        fake = make_fake_l3build(
            'if [ "$1" = "clean" ]; then exit 0; fi\n'
            'mkdir -p "$BUILD_OUT"\n'
            'cp "$DIFF_SRC" "$BUILD_OUT"/t1.pdftex.diff\n'
            'printf "Running checks on\\n  t1 (1/1)\\n          --> failed\\n"\n'
            'exit 1\n')
        fd, logpath = tempfile.mkstemp(prefix="diffhash-log-")
        os.close(fd)
        self.addCleanup(os.unlink, logpath)
        for key, val in (("BUILD_OUT", build_out),
                          ("DIFF_SRC", diffsrc)):
            self.addCleanup(_restore_env, key, os.environ.get(key))
            os.environ[key] = val
        rc, ran, failed, notes, timedout, info = run_l3build(
            workdir, ["t1"], engine, logpath, timeout=60, l3build_exe=fake)
        self.assertEqual(set(failed), {"t1"})
        self.assertEqual(info["diffhash"]["t1"],
                         hash_diff_files([os.path.join(build_out,
                                                       "t1.pdftex.diff")]))


def _restore_env(key, val):
    if val is None:
        os.environ.pop(key, None)
    else:
        os.environ[key] = val


def _results(failed=(), ran=(), notes=None, rc=0, timedout=(),
             deaths=(), neverran=(), diffhash=None):
    return {"d": {"ran": list(ran), "failed": set(failed),
                  "notes": dict(notes or {}), "rc": rc,
                  "timedout": set(timedout),
                  "info": {"diffhash": dict(diffhash or {}),
                           "deaths": set(deaths),
                           "neverran": set(neverran), "difffiles": {}}}}


class TestGate(unittest.TestCase):
    # F1: a listed entry excuses ONLY an ordinary diff mismatch (a .diff
    # was produced, with a matching recorded hash if the entry has one) --
    # never a death, timeout, never-ran, or a different diff. F2: stale
    # entries fail unless allowed.

    def test_matrix(self):
        sha, other = "ab12" * 16, "cd34" * 16
        cases = [
            # (label, results-kw, expected, want_rc, want_tag)
            ("legacy entry + diff excuses",
             dict(failed=["t1"], ran=["t1"], diffhash={"t1": sha}),
             {"t1": (None, "r")}, 0, "expected"),
            ("matching hash excuses",
             dict(failed=["t1"], ran=["t1"], diffhash={"t1": sha}),
             {"t1": (sha, "r")}, 0, "expected"),
            ("different diff is unexpected",
             dict(failed=["t1"], ran=["t1"], diffhash={"t1": other}),
             {"t1": (sha, "r")}, 1, "UNEXPECTED"),
            ("listed crash is unexpected (F1 headline)",
             dict(failed=["t1"], ran=["t1"],
                  notes={"t1": "engine process died (exit not 0/1)"},
                  deaths=["t1"], diffhash={"t1": sha}),
             {"t1": (sha, "r")}, 1, "UNEXPECTED"),
            ("listed without any diff is unexpected",
             dict(failed=["t1"], ran=["t1"]), {"t1": (None, "r")},
             1, "UNEXPECTED"),
            ("listed timeout is unexpected",
             dict(failed=["t1"], ran=["t1"], timedout=["t1"],
                  notes={"t1": "timeout after 2 s"}, diffhash={"t1": sha}),
             {"t1": (sha, "r")}, 1, "UNEXPECTED"),
            ("listed never-ran is unexpected",
             dict(failed=["t1"], neverran=["t1"],
                  notes={"t1": "check never ran: l3build aborted (rc 1)"}),
             {"t1": (None, "r")}, 1, "UNEXPECTED"),
            ("unlisted diff failure is unexpected",
             dict(failed=["t1"], ran=["t1"], diffhash={"t1": sha}),
             {}, 1, "UNEXPECTED"),
        ]
        for label, kw, expected, want_rc, want_tag in cases:
            with self.subTest(label):
                rc, text = summarize(_results(**kw), expected,
                                     allow_stale=False)
                self.assertEqual(rc, want_rc, label)
                self.assertIn("t1 [%s]" % want_tag, text, label)

    def test_stale_fails_unless_allowed(self):
        res = _results(ran=["t1"])
        rc, text = summarize(res, {"t1": (None, "r")}, allow_stale=False)
        self.assertEqual(rc, 1)
        self.assertIn("stale EXPECTED-FAILURES entries now passing", text)
        rc, _ = summarize(res, {"t1": (None, "r")}, allow_stale=True)
        self.assertEqual(rc, 0)


class TestRequestedSanity(unittest.TestCase):
    # F4: l3build exiting 0 with no transcript while tests were requested
    # must FAIL, not PASS 0 / FAIL 0.

    def test_silent_zero_exit_fails_requested(self):
        workdir = tempfile.mkdtemp(prefix="sanity-workdir-")
        self.addCleanup(shutil.rmtree, workdir, True)
        os.mkdir(os.path.join(workdir, "testfiles"))
        for name in ("t1", "t2"):
            with open(os.path.join(workdir, "testfiles", name + ".lvt"),
                      "w", encoding="utf-8") as fh:
                fh.write("% " + name + "\n")
        engine = make_fake_engine("exit 0\n")
        fake = make_fake_l3build(
            'if [ "$1" = "clean" ]; then exit 0; fi\nexit 0\n')
        fd, logpath = tempfile.mkstemp(prefix="sanity-log-")
        os.close(fd)
        self.addCleanup(os.unlink, logpath)
        rc, ran, failed, notes, timedout, info = run_l3build(
            workdir, ["t1", "t2"], engine, logpath, timeout=60,
            l3build_exe=fake)
        self.assertEqual(ran, [])
        self.assertEqual(set(failed), {"t1", "t2"})
        self.assertEqual(set(info["neverran"]), {"t1", "t2"})


class TestEngineEnv(unittest.TestCase):
    # --engine-env KEY=VALUE reaches the engine under test ONLY, through
    # the recording shim's exports: l3build and the reference probe run
    # without it.

    KEY = "FLASHTEX_FORMATS"

    def setUp(self):
        self.addCleanup(_restore_env, self.KEY, os.environ.get(self.KEY))
        os.environ.pop(self.KEY, None)
        self.addCleanup(_restore_env, "ENGINE_SENTINEL",
                        os.environ.get("ENGINE_SENTINEL"))
        os.environ.pop("ENGINE_SENTINEL", None)

    def run_shim(self, engine_env):
        """Run the shim around a fake engine; return what the engine saw."""
        fd, seen = tempfile.mkstemp(prefix="engine-seen-")
        os.close(fd)
        self.addCleanup(os.unlink, seen)
        engine = make_fake_engine(
            'echo "ENGINE:${%s-unset}" > "$ENGINE_SENTINEL"\nexit 0\n'
            % self.KEY)
        os.environ["ENGINE_SENTINEL"] = seen
        fd, calllog = tempfile.mkstemp(prefix="env-calls-")
        os.close(fd)
        self.addCleanup(os.unlink, calllog)
        shimdir = make_shim(engine, calllog, engine_env)
        self.addCleanup(shutil.rmtree, shimdir, True)
        env = dict(os.environ)
        env.pop(self.KEY, None)
        subprocess.run([os.path.join(shimdir, "pdftex"), "--version"],
                       env=env, capture_output=True, timeout=60)
        with open(seen, encoding="utf-8") as fh:
            return fh.read().strip()

    def test_candidate_sees_variable_through_shim(self):
        self.assertEqual(
            self.run_shim({self.KEY: "/tmp/candidate-fmt"}),
            "ENGINE:/tmp/candidate-fmt")

    def test_reference_shim_leaves_variable_absent(self):
        self.assertEqual(self.run_shim(None), "ENGINE:unset")

    def test_probe_absent_by_default_present_with_extra(self):
        fd, seen = tempfile.mkstemp(prefix="probe-seen-")
        os.close(fd)
        self.addCleanup(os.unlink, seen)
        eng = make_fake_engine(
            'echo "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)"\n'
            'echo "PROBE:${%s-unset}" > "%s"\nexit 0\n' % (self.KEY, seen))

        def read_seen():
            with open(seen, encoding="utf-8") as fh:
                return fh.read().strip()

        engine_version_firstline(eng)  # reference probe: no extra env
        self.assertEqual(read_seen(), "PROBE:unset")
        engine_version_firstline(eng, {self.KEY: "/tmp/candidate-fmt"})
        self.assertEqual(read_seen(), "PROBE:/tmp/candidate-fmt")

    def test_l3build_env_clean_but_engine_sees(self):
        workdir = tempfile.mkdtemp(prefix="env-workdir-")
        self.addCleanup(shutil.rmtree, workdir, True)
        os.mkdir(os.path.join(workdir, "testfiles"))
        with open(os.path.join(workdir, "testfiles", "t1.lvt"), "w",
                  encoding="utf-8") as fh:
            fh.write("% t1\n")
        fd, engine_seen = tempfile.mkstemp(prefix="e2e-engine-")
        os.close(fd)
        self.addCleanup(os.unlink, engine_seen)
        fd, l3_seen = tempfile.mkstemp(prefix="e2e-l3build-")
        os.close(fd)
        self.addCleanup(os.unlink, l3_seen)
        engine = make_fake_engine(
            'echo "ENGINE:${%s-unset}" > "$ENGINE_SENTINEL"\nexit 0\n'
            % self.KEY)
        fake = make_fake_l3build(
            'if [ "$1" = "clean" ]; then exit 0; fi\n'
            'echo "L3BUILD:${%s-unset}" > "$L3_SENTINEL"\n'
            'pdftex -jobname=t1 "\\input t1.lvt"\n'
            'printf "Running checks on\\n  t1 (1/1)\\n\\n  All checks passed\\n"\n'
            'exit 0\n' % self.KEY)
        for key, val in (("ENGINE_SENTINEL", engine_seen),
                         ("L3_SENTINEL", l3_seen)):
            self.addCleanup(_restore_env, key, os.environ.get(key))
            os.environ[key] = val
        fd, logpath = tempfile.mkstemp(prefix="env-log-")
        os.close(fd)
        self.addCleanup(os.unlink, logpath)
        rc, ran, failed, _, _, _ = run_l3build(
            workdir, ["t1"], engine, logpath, timeout=60, l3build_exe=fake,
            engine_env={self.KEY: "/tmp/candidate-fmt"})
        self.assertEqual(ran, ["t1"])
        self.assertEqual(set(failed), set())
        with open(engine_seen, encoding="utf-8") as fh:
            self.assertEqual(fh.read().strip(), "ENGINE:/tmp/candidate-fmt")
        with open(l3_seen, encoding="utf-8") as fh:
            self.assertEqual(fh.read().strip(), "L3BUILD:unset")

    def test_parse_pairs(self):
        self.assertEqual(parse_engine_env(["A=1", "B=2/x=y"]),
                         {"A": "1", "B": "2/x=y"})
        self.assertEqual(parse_engine_env(["EMPTY="]), {"EMPTY": ""})
        self.assertEqual(parse_engine_env(["_A9=x", "Z_9=/a b;$x"]),
                         {"_A9": "x", "Z_9": "/a b;$x"})
        self.assertEqual(parse_engine_env([]), {})
        self.assertEqual(parse_engine_env(None), {})

    def test_bad_keys_rejected(self):
        # The key is inserted unquoted into the shim's `export KEY=VALUE`
        # line, so anything outside [A-Za-z_][A-Za-z0-9_]* must not parse.
        for bad in ("NOEQUALS", "=empty-key", "", "A;B=1", "A B=1",
                    "=nokey", "9LIVES=1", "-A=1", "A.B=1"):
            with self.subTest(bad):
                with self.assertRaises(ValueError):
                    parse_engine_env([bad])

    def test_malformed_exits_2(self):
        eng = make_fake_engine("exit 0\n")
        for bad in ("NOEQUALS", "=empty-key", "", "BAD;KEY=1",
                    "BAD KEY=1", "=nokey", "9LIVES=1"):
            with self.subTest(bad):
                with self.assertRaises(SystemExit) as ctx:
                    main(["--engine", eng, "--engine-env", bad,
                          "--suite", "base"])
                self.assertEqual(ctx.exception.code, 2)

    def test_value_with_shell_metachars_arrives_literally(self):
        # Values stay shlex-quoted in the shim: spaces, `;` and `$`
        # must arrive as data, never run as shell.
        self.assertEqual(self.run_shim({self.KEY: "FLAG POOL;$x"}),
                         "ENGINE:FLAG POOL;$x")

    def test_unflagged_flashtex_vars_scrubbed(self):
        # FLASHTEX_OTHER exported in the parent shell but never flagged
        # must reach neither the candidate engine nor l3build; only
        # --engine-env values may supply FLASHTEX_* to the engine.
        for key, val in (("FLASHTEX_OTHER", "parent-leak"),
                         (self.KEY, "parent-formats")):
            self.addCleanup(_restore_env, key, os.environ.get(key))
            os.environ[key] = val
        workdir = tempfile.mkdtemp(prefix="scrub-workdir-")
        self.addCleanup(shutil.rmtree, workdir, True)
        os.mkdir(os.path.join(workdir, "testfiles"))
        with open(os.path.join(workdir, "testfiles", "t1.lvt"), "w",
                  encoding="utf-8") as fh:
            fh.write("% t1\n")
        fd, engine_seen = tempfile.mkstemp(prefix="scrub-engine-")
        os.close(fd)
        self.addCleanup(os.unlink, engine_seen)
        fd, l3_seen = tempfile.mkstemp(prefix="scrub-l3build-")
        os.close(fd)
        self.addCleanup(os.unlink, l3_seen)
        engine = make_fake_engine(
            'echo "ENGINE:${%s-unset}:${FLASHTEX_OTHER-unset}" '
            '> "$ENGINE_SENTINEL"\nexit 0\n' % self.KEY)
        fake = make_fake_l3build(
            'if [ "$1" = "clean" ]; then exit 0; fi\n'
            'echo "L3BUILD:${%s-unset}:${FLASHTEX_OTHER-unset}" '
            '> "$L3_SENTINEL"\n'
            'pdftex -jobname=t1 "\\input t1.lvt"\n'
            'printf "Running checks on\\n  t1 (1/1)\\n\\n  All checks passed\\n"\n'
            'exit 0\n' % self.KEY)
        for key, val in (("ENGINE_SENTINEL", engine_seen),
                         ("L3_SENTINEL", l3_seen)):
            self.addCleanup(_restore_env, key, os.environ.get(key))
            os.environ[key] = val
        fd, logpath = tempfile.mkstemp(prefix="scrub-log-")
        os.close(fd)
        self.addCleanup(os.unlink, logpath)

        def read(path):
            with open(path, encoding="utf-8") as fh:
                return fh.read().strip()

        rc, ran, failed, _, _, _ = run_l3build(
            workdir, ["t1"], engine, logpath, timeout=60, l3build_exe=fake,
            engine_env={self.KEY: "/tmp/candidate-fmt"})
        self.assertEqual((rc, ran, set(failed)), (0, ["t1"], set()))
        self.assertEqual(read(engine_seen),
                         "ENGINE:/tmp/candidate-fmt:unset")
        self.assertEqual(read(l3_seen), "L3BUILD:unset:unset")
        # With no --engine-env at all, the parent's FLASHTEX_* reach
        # neither side either.
        rc, ran, failed, _, _, _ = run_l3build(
            workdir, ["t1"], engine, logpath, timeout=60, l3build_exe=fake,
            engine_env=None)
        self.assertEqual((rc, ran, set(failed)), (0, ["t1"], set()))
        self.assertEqual(read(engine_seen), "ENGINE:unset:unset")
        self.assertEqual(read(l3_seen), "L3BUILD:unset:unset")

    def test_probe_scrubs_unflagged_vars(self):
        self.addCleanup(_restore_env, "FLASHTEX_OTHER",
                        os.environ.get("FLASHTEX_OTHER"))
        os.environ["FLASHTEX_OTHER"] = "parent-leak"
        fd, seen = tempfile.mkstemp(prefix="probe-scrub-")
        os.close(fd)
        self.addCleanup(os.unlink, seen)
        eng = make_fake_engine(
            'echo "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)"\n'
            'echo "PROBE:${FLASHTEX_OTHER-unset}" > "%s"\nexit 0\n' % seen)

        def read_seen():
            with open(seen, encoding="utf-8") as fh:
                return fh.read().strip()

        engine_version_firstline(eng)  # reference probe: no extra env
        self.assertEqual(read_seen(), "PROBE:unset")
        engine_version_firstline(eng, {"FLASHTEX_OTHER": "flagged"})
        self.assertEqual(read_seen(), "PROBE:flagged")

    def test_option_repeatable(self):
        eng = make_fake_engine("exit 0\n")
        rc = main(["--engine", eng, "--list", "--suite", "base",
                   "--engine-env", "FLASHTEX_FORMATS=/tmp/fmt",
                   "--engine-env",
                   "FLASHTEX_POOL=/repo/crates/flashtex-engine/pdftex.pool"])
        self.assertEqual(rc, 0)


class TestEtexShim(unittest.TestCase):
    # DVI-mode runs (graphics `-e etex`, backend `-e etex-dvips` /
    # `etex-dvisvgm`) need a second recording shim `etex` next to
    # `pdftex`: unpack invokes the pdftex binary, format/check runs the
    # etex binary. The etex shim execs the engine under test with
    # `-progname=etex`, like the real TeX Live etex symlink.

    def run_which_shim(self, shim_name, args, engine_body='exit 0\n',
                       engine_env=None):
        fd, seen = tempfile.mkstemp(prefix="etex-argv-")
        os.close(fd)
        self.addCleanup(os.unlink, seen)
        engine = make_fake_engine(
            'echo "$@" > "%s"\n%s' % (seen, engine_body))
        fd, calllog = tempfile.mkstemp(prefix="etex-calls-")
        os.close(fd)
        self.addCleanup(os.unlink, calllog)
        shimdir = make_shims(engine, calllog, engine_env)
        self.addCleanup(shutil.rmtree, shimdir, True)
        proc = subprocess.run([os.path.join(shimdir, shim_name)] + args,
                              capture_output=True, timeout=60)
        with open(seen, encoding="utf-8") as fh:
            argv = fh.read().strip()
        with open(calllog, encoding="utf-8") as fh:
            logged = fh.read().strip()
        return proc.returncode, argv, logged

    def test_both_shims_generated_and_executable(self):
        fd, calllog = tempfile.mkstemp(prefix="etex-gen-")
        os.close(fd)
        self.addCleanup(os.unlink, calllog)
        shimdir = make_shims("/nonexistent-engine", calllog)
        self.addCleanup(shutil.rmtree, shimdir, True)
        for name in ("pdftex", "etex"):
            path = os.path.join(shimdir, name)
            self.assertTrue(os.access(path, os.X_OK), name)

    def test_etex_shim_prepends_progname(self):
        rc, argv, logged = self.run_which_shim(
            "etex", ["--fmt=latex", "-jobname=t1"])
        self.assertEqual(rc, 0)
        self.assertTrue(argv.startswith("-progname=etex "),
                        "engine argv %r lacks -progname=etex first" % argv)
        self.assertIn("-jobname=t1", argv)
        self.assertIn("rc=0 argv=-progname=etex", logged)

    def test_pdftex_shim_unchanged_no_progname(self):
        _, argv, logged = self.run_which_shim(
            "pdftex", ["--fmt=pdflatex", "-jobname=t1"])
        self.assertNotIn("-progname", argv)
        self.assertNotIn("-progname", logged)

    def test_engine_env_on_etex_shim_only(self):
        key = "FLASHTEX_FORMATS"
        self.addCleanup(_restore_env, key, os.environ.get(key))
        os.environ.pop(key, None)
        fd, seen = tempfile.mkstemp(prefix="etex-env-")
        os.close(fd)
        self.addCleanup(os.unlink, seen)
        engine = make_fake_engine(
            'echo "ENV:${%s-unset}" > "%s"\nexit 0\n' % (key, seen))
        fd, calllog = tempfile.mkstemp(prefix="etex-env-calls-")
        os.close(fd)
        self.addCleanup(os.unlink, calllog)
        shimdir = make_shims(engine, calllog, {key: "/tmp/candidate-fmt"})
        self.addCleanup(shutil.rmtree, shimdir, True)
        env = dict(os.environ)
        env.pop(key, None)
        subprocess.run([os.path.join(shimdir, "etex"), "--version"],
                       env=env, capture_output=True, timeout=60)
        with open(seen, encoding="utf-8") as fh:
            self.assertEqual(fh.read().strip(), "ENV:/tmp/candidate-fmt")

    def test_death_detected_through_etex_shim(self):
        workdir = tempfile.mkdtemp(prefix="etex-death-")
        self.addCleanup(shutil.rmtree, workdir, True)
        os.mkdir(os.path.join(workdir, "testfiles"))
        with open(os.path.join(workdir, "testfiles", "t1.lvt"), "w",
                  encoding="utf-8") as fh:
            fh.write("% t1\n")
        engine = make_fake_engine("exit 3\n")  # crash, not TeX error (1)
        fake = make_fake_l3build(
            'if [ "$1" = "clean" ]; then exit 0; fi\n'
            'etex -jobname=t1 "\\input t1.lvt"\n'
            'printf "Running checks on\\n  t1 (1/1)\\n"\n'
            'exit 1\n')
        fd, logpath = tempfile.mkstemp(prefix="etex-death-log-")
        os.close(fd)
        self.addCleanup(os.unlink, logpath)
        _, ran, failed, notes, _, info = run_l3build(
            workdir, ["t1"], engine, logpath, timeout=60, l3build_exe=fake,
            l3build_engine="etex")
        self.assertEqual(ran, ["t1"])
        self.assertEqual(set(failed), {"t1"})
        self.assertEqual(info["deaths"], {"t1"})
        self.assertIn("died", notes["t1"])

    def test_etex_diff_scoped_to_engine(self):
        workdir = tempfile.mkdtemp(prefix="etex-diff-")
        self.addCleanup(shutil.rmtree, workdir, True)
        os.mkdir(os.path.join(workdir, "testfiles"))
        with open(os.path.join(workdir, "testfiles", "t1.lvt"), "w",
                  encoding="utf-8") as fh:
            fh.write("% t1\n")
        engine = make_fake_engine("exit 0\n")
        fd, diffsrc = tempfile.mkstemp(prefix="etex-diffsrc-")
        with os.fdopen(fd, "wb") as fh:
            fh.write(b"*** a\n--- b\n! x\n")
        self.addCleanup(os.unlink, diffsrc)
        build_out = os.path.join(workdir, "build", "test")
        fake = make_fake_l3build(
            'if [ "$1" = "clean" ]; then exit 0; fi\n'
            'mkdir -p "$BUILD_OUT"\n'
            'cp "$DIFF_SRC" "$BUILD_OUT"/t1.etex.diff\n'
            'printf "Running checks on\\n  t1 (1/1)\\n'
            '          --> failed\\n  - $BUILD_OUT/t1.etex.diff\\n"\n'
            'exit 1\n')
        for key, val in (("BUILD_OUT", build_out),
                         ("DIFF_SRC", diffsrc)):
            self.addCleanup(_restore_env, key, os.environ.get(key))
            os.environ[key] = val
        fd, logpath = tempfile.mkstemp(prefix="etex-diff-log-")
        os.close(fd)
        self.addCleanup(os.unlink, logpath)
        _, _, failed, _, _, info = run_l3build(
            workdir, ["t1"], engine, logpath, timeout=60, l3build_exe=fake,
            l3build_engine="etex")
        self.assertEqual(set(failed), {"t1"})
        self.assertIn("t1", info["diffhash"])
        # Transcript harvest is per-engine: a pdftex run must not pick
        # up an etex diff line, and hyphenated variants match literally.
        lines = ["  - build/test/t1.etex.diff\n"]
        _, _, etex_failed = parse_l3build_log(lines, "etex")
        _, _, pdftex_failed = parse_l3build_log(lines)
        self.assertEqual(etex_failed, {"t1"})
        self.assertEqual(pdftex_failed, set())
        dvips_lines = ["  - build/test/d3pdfmode.etex-dvips.diff\n"]
        _, _, dvips_failed = parse_l3build_log(dvips_lines, "etex-dvips")
        self.assertEqual(dvips_failed, {"d3pdfmode"})
        self.assertTrue(diffline_re("etex-dvips").match(
            "  - build/test/d3pdfmode.etex-dvips.diff"))

    def test_check_cmd_carries_engine_and_config(self):
        workdir = tempfile.mkdtemp(prefix="etex-cmd-")
        self.addCleanup(shutil.rmtree, workdir, True)
        os.mkdir(os.path.join(workdir, "testfiles-backend"))
        with open(os.path.join(workdir, "testfiles-backend", "b1.lvt"),
                  "w", encoding="utf-8") as fh:
            fh.write("% b1\n")
        self.assertEqual(list_tests(workdir, "testfiles-backend"), ["b1"])
        engine = make_fake_engine("exit 0\n")
        fd, sentinel = tempfile.mkstemp(prefix="etex-args-")
        os.close(fd)
        self.addCleanup(os.unlink, sentinel)
        self.addCleanup(_restore_env, "ARGS_SENTINEL",
                        os.environ.get("ARGS_SENTINEL"))
        os.environ["ARGS_SENTINEL"] = sentinel
        fake = make_fake_l3build(
            'if [ "$1" = "clean" ]; then exit 0; fi\n'
            'echo "$@" > "$ARGS_SENTINEL"\n'
            'printf "Running checks on\\n  b1 (1/1)\\n\\n'
            '  All checks passed\\n"\n'
            'exit 0\n')
        fd, logpath = tempfile.mkstemp(prefix="etex-cmd-log-")
        os.close(fd)
        self.addCleanup(os.unlink, logpath)
        rc, ran, failed, _, _, _ = run_l3build(
            workdir, ["b1"], engine, logpath, timeout=60, l3build_exe=fake,
            l3build_engine="etex-dvips",
            l3build_configs=("config-backend",),
            testdir="testfiles-backend")
        self.assertEqual((rc, ran, set(failed)), (0, ["b1"], set()))
        with open(sentinel, encoding="utf-8") as fh:
            self.assertEqual(fh.read().strip(),
                             "check -c config-backend -e etex-dvips b1")

    def test_dir_label(self):
        self.assertEqual(dir_label("latex2e", "base"), "latex2e/base")
        self.assertEqual(dir_label("latex2e", "required/graphics", "etex"),
                         "latex2e/required/graphics@etex")
        self.assertEqual(dir_label("latex3", "l3kernel", "etex-dvips",
                                   ("config-backend",)),
                         "latex3/l3kernel[config-backend]@etex-dvips")



class TestDirSummary(unittest.TestCase):
    """main()'s per-directory lines name each directory's own failures, so a
    test that runs in two directories (testfiles-backend under etex-dvips and
    etex-dvisvgm) keeps which one failed."""

    def test_failed_line_per_directory(self):
        label = dir_label("latex3", "l3kernel", "etex-dvips", ["config-backend"])
        self.assertEqual(dir_summary(label, ["m3backend01", "m3backend02"], ["m3backend01"]),
                         ["%s: PASS 1 / FAIL 1 / SKIP 0" % label, "%s: FAILED m3backend01" % label])

    def test_no_failed_line_when_all_pass(self):
        self.assertEqual(dir_summary("latex2e/base", ["a", "b"], []),
                         ["latex2e/base: PASS 2 / FAIL 0 / SKIP 0"])


if __name__ == "__main__":
    unittest.main(verbosity=1)
