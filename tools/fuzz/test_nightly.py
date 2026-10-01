"""Tests for nightly.py. Fake fuzzer scripts only; the real engines are
never used."""
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nightly

FAKE = """#!/usr/bin/env python3
import argparse, json, os, sys, time
ap = argparse.ArgumentParser()
ap.add_argument("--candidate", default="x")
ap.add_argument("--oracle", default="y")
ap.add_argument("--seeds", default="z")
ap.add_argument("--out", required=True)
ap.add_argument("--iterations", type=int, required=True)
ap.add_argument("--seed", type=int, default=0)
ap.add_argument("--timeout", type=float, default=1.0)
ap.add_argument("--mode", default="clean")
a = ap.parse_args()
time.sleep(0.001 * a.iterations)
os.makedirs(os.path.join(a.out, "candidate-crash"), exist_ok=True)
if a.mode == "fail":
    print("boom", flush=True)
    sys.exit(2)
if a.mode == "finding":
    sig = os.environ.get("FAKE_SIG", "candidate-crash:exit:101")
    digest = "%040d" % a.seed
    with open(os.path.join(a.out, "candidate-crash",
                           digest + ".tex"), "w") as fh:
        fh.write("input")
    with open(os.path.join(a.out, "candidate-crash",
                           digest + ".json"), "w") as fh:
        json.dump({"signature": sig}, fh)
    print("done: %d iterations: equal=%d candidate-crash=1"
          % (a.iterations, a.iterations - 1))
else:
    print("done: %d iterations: equal=%d" % (a.iterations, a.iterations))
"""


class NightlyTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.scripts = os.path.join(self.tmp, "fuzzers")
        os.mkdir(self.scripts)
        self.out = os.path.join(self.tmp, "out")
        self.old_table = nightly.FUZZERS
        nightly.FUZZERS = tuple(
            dict(name="fz%d" % i, script="fz%d.py" % i, oracle=bool(i % 2),
                 seeds=False, timeout=5.0, base=100, offset=i)
            for i in range(2))
        for i in range(2):
            path = os.path.join(self.scripts, "fz%d.py" % i)
            with open(path, "w") as fh:
                fh.write(FAKE)
            os.chmod(path, os.stat(path).st_mode | stat.S_IEXEC)
            # nightly resolves scripts relative to its own dir; point
            # each entry at the temp script through an absolute path.
            nightly.FUZZERS[i]["script"] = os.path.relpath(path,
                                                           nightly.HERE)
        self.known = os.path.join(self.tmp, "known.json")
        with open(self.known, "w") as fh:
            json.dump([], fh)

    def tearDown(self):
        nightly.FUZZERS = self.old_table
        shutil.rmtree(self.tmp, ignore_errors=True)
        os.environ.pop("FAKE_SIG", None)

    def base_args(self, extra=()):
        return ["--candidate", "c", "--oracle", "o", "--out", self.out,
                "--seed", "7", "--budget-minutes", "1",
                "--known-findings", self.known] + list(extra)

    def run_main(self, args):
        # limit fake modes via env passthrough is subprocess-inherited
        return nightly.main(args)

    def test_sizing_stays_inside_budget(self):
        rates = {"a": 0.5, "b": 2.0}
        table = [dict(name="a", base=100), dict(name="b", base=100)]
        totals = nightly.size_runs(60.0, table, rates)
        cost = sum(totals[n] * rates[n] for n in totals)
        self.assertLessEqual(cost, 60.0)
        # tiny budget scales everything down, huge budget keeps base
        tiny = nightly.size_runs(21.0, table, rates)
        self.assertLess(sum(tiny.values()), sum(totals.values()))
        full = nightly.size_runs(3600.0, table, rates)
        self.assertEqual(full, {"a": 100, "b": 100})

    def test_known_matching(self):
        self.assertTrue(nightly.is_known("fontcount-diff:3v5",
                                         ["fontcount-diff:*"]))
        self.assertTrue(nightly.is_known("signal:6", ["signal:6"]))
        self.assertFalse(nightly.is_known("signal:11", ["signal:6"]))
        self.assertFalse(nightly.is_known("other", ["fontcount-diff:*"]))

    def test_scoped_known_findings(self):
        # Entries may carry a "fuzzers" list scoping them to findings
        # from those fuzzers; entries without it stay global.
        patterns = nightly.load_known_findings(os.path.join(
            nightly.HERE, "known-findings.json"))
        stack = ("signal:SIGABRT:thread 'main' (NNNNNNNN) "
                 "has overflowed its stack")
        self.assertTrue(nightly.is_known(stack, patterns, fuzzer="type1"))
        self.assertFalse(nightly.is_known(stack, patterns,
                                          fuzzer="docgen"))
        self.assertFalse(nightly.is_known(stack, patterns))
        self.assertFalse(nightly.is_known(
            "signal:SIGABRT:assertion failed: kpse_foo", patterns,
            fuzzer="type1"))
        self.assertFalse(nightly.is_known(
            "signal:SIGABRT:fatal runtime error: something else",
            patterns, fuzzer="type1"))
        panic = ("panic:crates/flashtex-engine/src/generated/"
                 "body_0.rs:1033")
        self.assertTrue(nightly.is_known(panic, patterns,
                                         fuzzer="docgen"))
        self.assertTrue(nightly.is_known(panic, patterns,
                                         fuzzer="type1"))
        self.assertTrue(nightly.is_known(panic, patterns))

    def test_exit_zero_no_findings(self):
        rc = self.run_main(self.base_args())
        self.assertEqual(rc, 0)
        with open(os.path.join(self.out, "summary.json")) as fh:
            summary = json.load(fh)
        self.assertEqual(set(summary["fuzzers"]), {"fz0", "fz1"})
        entry = summary["fuzzers"]["fz0"]
        for key in ("iterations", "class_counts", "new_signatures",
                    "elapsed_seconds", "seed"):
            self.assertIn(key, entry)
        self.assertGreater(entry["iterations"], 0)
        with open(os.path.join(self.out, "summary.md")) as fh:
            md = fh.read()
        self.assertIn("| fz0 |", md)

    def test_exit_one_unknown_finding(self):
        os.environ["FAKE_SIG"] = "diverge:brand-new-bug"
        # fake script needs mode=finding: pass via wrapper args is not
        # supported, so flip the default mode in the temp scripts.
        for i in range(2):
            path = os.path.join(self.scripts, "fz%d.py" % i)
            with open(path) as fh:
                text = fh.read()
            with open(path, "w") as fh:
                fh.write(text.replace('default="clean"',
                                      'default="finding"'))
        rc = self.run_main(self.base_args())
        self.assertEqual(rc, 1)
        with open(os.path.join(self.out, "summary.md")) as fh:
            md = fh.read()
        self.assertIn("diverge:brand-new-bug", md)
        self.assertIn("artifact:", md)

    def test_exit_zero_known_finding(self):
        with open(self.known, "w") as fh:
            json.dump([{"signature": "candidate-crash:exit:101",
                        "note": "known"}], fh)
        os.environ["FAKE_SIG"] = "candidate-crash:exit:101"
        for i in range(2):
            path = os.path.join(self.scripts, "fz%d.py" % i)
            with open(path) as fh:
                text = fh.read()
            with open(path, "w") as fh:
                fh.write(text.replace('default="clean"',
                                      'default="finding"'))
        self.assertEqual(self.run_main(self.base_args()), 0)

    def test_exit_two_harness_failure(self):
        for i in range(2):
            path = os.path.join(self.scripts, "fz%d.py" % i)
            with open(path) as fh:
                text = fh.read()
            with open(path, "w") as fh:
                fh.write(text.replace('default="clean"',
                                      'default="fail"'))
        self.assertEqual(self.run_main(self.base_args()), 2)

    def test_benign_classes_listed_but_exit_zero(self):
        self.assertTrue(nightly.is_benign(
            {"path": "out/fz0/both-crash/abcd.tex"}))
        self.assertTrue(nightly.is_benign(
            {"path": "out/fz0/both-hang/abcd.tex"}))
        self.assertTrue(nightly.is_benign(
            {"path": "out/fz0/reference-nondeterministic/abcd.tex"}))
        self.assertFalse(nightly.is_benign(
            {"path": "out/fz0/diverge/abcd.tex"}))
        # An unknown both-crash finding on disk: listed, exit 0.
        cls_dir = os.path.join(self.out, "fz0", "both-crash")
        os.makedirs(cls_dir)
        with open(os.path.join(cls_dir, "0" * 40 + ".tex"), "w") as fh:
            fh.write("input")
        with open(os.path.join(cls_dir, "0" * 40 + ".json"), "w") as fh:
            json.dump({"signature": "both-crash:brand-new-crash"}, fh)
        self.assertEqual(self.run_main(self.base_args()), 0)
        with open(os.path.join(self.out, "summary.json")) as fh:
            summary = json.load(fh)
        sigs = [f["signature"]
                for f in summary["findings"]]
        self.assertIn("both-crash:brand-new-crash", sigs)
        with open(os.path.join(self.out, "summary.md")) as fh:
            self.assertIn("both-crash:brand-new-crash", fh.read())

    def test_fuzzer_timeout_formula(self):
        table = [dict(name="a", base=100), dict(name="b", base=300)]
        got = nightly.fuzzer_timeouts(600.0, table)
        self.assertAlmostEqual(got["a"], 600.0 * 0.25 * 1.2 + 30.0)
        self.assertAlmostEqual(got["b"], 600.0 * 0.75 * 1.2 + 30.0)

    def test_stack_overflow_matches_any_digit_count(self):
        # The Type 1 stack-overflow entry must not pin an 8-digit thread
        # id: 7- and 9-digit ids are known too (still scoped to type1).
        patterns = nightly.load_known_findings(os.path.join(
            nightly.HERE, "known-findings.json"))
        for digits in (7, 9):
            sig = ("signal:SIGABRT:thread 'main' (%s) "
                   "has overflowed its stack" % ("N" * digits))
            self.assertTrue(nightly.is_known(sig, patterns, fuzzer="type1"),
                            "not known: " + sig)
        self.assertFalse(nightly.is_known(
            "signal:SIGABRT:thread 'main' (%s) has overflowed its stack"
            % ("N" * 7), patterns, fuzzer="docgen"))

    def test_default_seed_changes_daily(self):
        import datetime
        d0 = datetime.date(2026, 9, 29)
        d1 = datetime.date(2026, 9, 30)
        self.assertEqual(nightly.default_seed(d0) + 1,
                         nightly.default_seed(d1))
        self.assertEqual(nightly.default_seed(d0),
                         (d0 - datetime.date(1970, 1, 1)).days)


SLEEPER = """#!/usr/bin/env python3
import argparse, os, subprocess, sys, time
ap = argparse.ArgumentParser()
for flag in ("--candidate", "--oracle", "--seeds", "--out"):
    ap.add_argument(flag, default="x")
ap.add_argument("--iterations", type=int, default=1)
ap.add_argument("--seed", type=int, default=0)
ap.add_argument("--timeout", type=float, default=5.0)
ap.add_argument("--spawn-kid", action="store_true")
ap.add_argument("--sleep", type=float, default=30.0)
a = ap.parse_args()
if a.spawn_kid:
    kid = subprocess.Popen(["sleep", "30"])
    with open(os.path.join(a.out, "kid.pid"), "w") as fh:
        fh.write(str(kid.pid))
time.sleep(a.sleep)
print("done: 0 iterations: equal=0")
sys.stdout.flush()
"""


class NightlyTimeoutTest(unittest.TestCase):
    """Wall-clock enforcement: fake fuzzer scripts that sleep."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.out = os.path.join(self.tmp, "out")
        self.old_table = nightly.FUZZERS
        self.old_fn = nightly.fuzzer_timeouts
        self.old_grace = nightly.OVERBUDGET_GRACE_SECONDS
        nightly.FUZZERS = tuple(
            dict(name="fz%d" % i,
                 script="fz%d.py" % i, oracle=False,
                 seeds=False, timeout=5.0, base=100, offset=i)
            for i in range(2))
        for i in range(2):
            path = os.path.join(self.tmp, "fz%d.py" % i)
            with open(path, "w") as fh:
                fh.write(SLEEPER)
            os.chmod(path, os.stat(path).st_mode | stat.S_IEXEC)
            nightly.FUZZERS[i]["script"] = os.path.relpath(
                path, nightly.HERE)
        self.known = os.path.join(self.tmp, "known.json")
        with open(self.known, "w") as fh:
            json.dump([], fh)

    def tearDown(self):
        nightly.FUZZERS = self.old_table
        nightly.fuzzer_timeouts = self.old_fn
        nightly.OVERBUDGET_GRACE_SECONDS = self.old_grace
        shutil.rmtree(self.tmp, ignore_errors=True)

    def base_args(self, budget="1"):
        return ["--candidate", "c", "--oracle", "o", "--out", self.out,
                "--seed", "7", "--budget-minutes", budget,
                "--known-findings", self.known]

    def test_run_fuzzer_kills_whole_process_group(self):
        spec = dict(nightly.FUZZERS[0])
        sub = os.path.join(self.out, "fz0")
        os.makedirs(sub, exist_ok=True)
        # The sleeper spawns a grandchild sleep: both must die with the
        # group, proving start_new_session + killpg (not just the parent).
        rc, out, killed = nightly.run_fuzzer(
            spec, "c", "o", "seeds", sub, 1, 7, timeout=1.0,
            extra_args=["--spawn-kid"])
        self.assertTrue(killed)
        self.assertIn("wall-clock timeout", out)
        with open(os.path.join(sub, "kid.pid")) as fh:
            kid = int(fh.read().strip())
        self.assertRaises(OSError, os.kill, kid, 0)

    def test_main_records_timed_out_and_exit_two(self):
        nightly.FUZZERS = nightly.FUZZERS[:1]
        nightly.fuzzer_timeouts = lambda budget, table: {"fz0": 1.0}  # noqa
        rc = nightly.main(self.base_args())
        self.assertEqual(rc, 2)
        with open(os.path.join(self.out, "summary.json")) as fh:
            summary = json.load(fh)
        self.assertEqual(summary["fuzzers"]["fz0"]["status"], "timed-out")
        with open(os.path.join(self.out, "summary.md")) as fh:
            self.assertIn("timed-out", fh.read())

    def test_detached_session_child_killed_and_reported(self):
        # A fuzzer child started in its OWN session (start_new_session)
        # escapes the fuzzer's process group; nightly must still kill it
        # via a descendant-tree snapshot, not just killpg.
        try:
            probe = subprocess.run(
                ["ps", "-axo", "pid=,ppid=,pgid="],
                capture_output=True, text=True, timeout=10)
        except (OSError, subprocess.SubprocessError):
            self.skipTest("ps unavailable: tree kill unverifiable here")
        if probe.returncode != 0 or not (probe.stdout or "").strip():
            self.skipTest("ps unavailable: tree kill unverifiable here")
        script = os.path.join(self.tmp, "detached.py")
        with open(script, "w") as fh:
            fh.write(DETACHED)
        os.chmod(script, os.stat(script).st_mode | stat.S_IEXEC)
        nightly.FUZZERS = (dict(name="fz0",
                                script=os.path.relpath(script,
                                                       nightly.HERE),
                                oracle=False, seeds=False, timeout=5.0,
                                base=100, offset=0),)
        nightly.fuzzer_timeouts = lambda budget, table: {"fz0": 1.0}  # noqa
        rc = nightly.main(self.base_args())
        self.assertEqual(rc, 2)
        pid_path = os.path.join(self.out, "fz0", "detached.pid")
        with open(pid_path) as fh:
            kid = int(fh.read().strip())
        try:
            self.assertRaises(OSError, os.kill, kid, 0)
            with open(os.path.join(self.out, "summary.json")) as fh:
                summary = json.load(fh)
            self.assertEqual(summary["fuzzers"]["fz0"]["status"],
                             "timed-out")
            self.assertIn("unkilled_pids", summary)
            self.assertEqual(summary["unkilled_pids"], [])
        finally:
            # Never leave the sleep behind, even on failure.
            try:
                os.kill(kid, 9)
            except OSError:
                pass

    def test_descendant_snapshot_follows_ppid_chains(self):
        # Pure logic test: needs no ps, no processes. Grandchildren and
        # reparented-looking entries resolve through ppid links only.
        table = {100: (1, 100), 101: (100, 101), 102: (101, 102),
                 103: (1, 103), 104: (102, 500)}
        old = nightly._ps_table
        nightly._ps_table = lambda: dict(table)  # noqa
        try:
            pids, pgids = nightly._descendant_snapshot(100)
        finally:
            nightly._ps_table = old
        self.assertEqual(pids, {101, 102, 104})
        self.assertEqual(pgids, {101: 101, 102: 102, 104: 500})

    def test_over_budget_stops_starting_fuzzers(self):
        nightly.fuzzer_timeouts = lambda budget, table: {  # noqa
            "fz0": 1.0, "fz1": 1.0}
        nightly.OVERBUDGET_GRACE_SECONDS = 0.5
        rc = nightly.main(self.base_args(budget="0"))
        self.assertEqual(rc, 2)
        with open(os.path.join(self.out, "summary.json")) as fh:
            summary = json.load(fh)
        # fz0 overran and was killed; fz1 never started.
        self.assertEqual(summary["fuzzers"]["fz0"]["status"], "timed-out")
        self.assertGreater(
            summary["fuzzers"]["fz0"]["elapsed_seconds"], 0)
        self.assertEqual(summary["fuzzers"]["fz1"]["status"], "timed-out")
        self.assertEqual(summary["fuzzers"]["fz1"]["iterations"], 0)


# Fake fuzzer that keeps spawning session-detached children (like real
# engines started with start_new_session) and ignores SIGTERM, so only a
# frozen-then-killed tree leaves nothing behind.
SPAWNER = """#!/usr/bin/env python3
import signal, subprocess, time
signal.signal(signal.SIGTERM, signal.SIG_IGN)
while True:
    subprocess.Popen(["sleep", "299"], start_new_session=True)
    time.sleep(0.01)
"""


class NightlyKillRaceTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.out = os.path.join(self.tmp, "out")
        os.mkdir(self.out)
        path = os.path.join(self.tmp, "spawner.py")
        with open(path, "w") as fh:
            fh.write(SPAWNER)
        os.chmod(path, os.stat(path).st_mode | stat.S_IEXEC)
        self.spec = dict(name="fz0", script=os.path.relpath(path,
                                                             nightly.HERE),
                         oracle=False, seeds=False, timeout=5.0, base=100,
                         offset=0)
        self.addCleanup(self._clean_strays)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _clean_strays(self):
        # Never leave spawned sleeps behind, even on failure.
        try:
            subprocess.run(["pkill", "-f", "sleep 299"],
                           capture_output=True, timeout=10)
        except OSError:
            pass

    def _marker_procs(self):
        proc = subprocess.run(["ps", "-axo", "args="], capture_output=True,
                              text=True, timeout=10)
        if proc.returncode != 0:
            self.skipTest("ps unavailable: kill race unverifiable here")
        return [ln for ln in (proc.stdout or "").splitlines()
                if "sleep 299" in ln]

    def test_deadline_kill_leaves_no_children(self):
        try:
            self._marker_procs()
        except unittest.SkipTest:
            raise
        except OSError:
            self.skipTest("ps unavailable: kill race unverifiable here")
        # Slow the SIGKILL pass to model scheduling delay: an unfrozen
        # spawner keeps forking detached children through the gap, so the
        # old snapshot-without-freeze code misses them.
        import signal as sigmod
        real_tree = nightly._signal_tree

        def slow_tree(root, pids, pgids, sig):
            if sig == sigmod.SIGKILL:
                time.sleep(0.5)
            return real_tree(root, pids, pgids, sig)

        nightly._signal_tree = slow_tree
        t0 = time.monotonic()
        try:
            unkilled = []
            _rc, out, killed = nightly.run_fuzzer(
                self.spec, "c", "o", "seeds", self.out, 1, 7, timeout=2.0,
                unkilled_pids=unkilled)
        finally:
            nightly._signal_tree = real_tree
        dt = time.monotonic() - t0
        self.assertTrue(killed)
        self.assertIn("wall-clock timeout", out)
        # The kill must return promptly: without the re-freeze the
        # spawner forks through the grace period, missed `sleep 299`
        # children hold the stdout pipe, and this takes ~300 s.
        self.assertLess(dt, nightly.KILL_AFTER_SECONDS + 10)
        self.assertEqual(unkilled, [])
        # No polling, no waiting: the sleeps are gone already.
        self.assertEqual(self._marker_procs(), [])

    def test_grace_period_spawner_leaves_nothing(self):
        # The case that failed: the TERM-ignoring spawner keeps forking
        # session-detached `sleep 299` children all through the
        # KILL_AFTER_SECONDS grace period (no scheduling-delay
        # injection here). The re-freeze loop must still leave nothing
        # behind, promptly.
        try:
            self._marker_procs()
        except unittest.SkipTest:
            raise
        except OSError:
            self.skipTest("ps unavailable: kill race unverifiable here")
        t0 = time.monotonic()
        unkilled = []
        _rc, out, killed = nightly.run_fuzzer(
            self.spec, "c", "o", "seeds", self.out, 1, 7, timeout=2.0,
            unkilled_pids=unkilled)
        dt = time.monotonic() - t0
        self.assertTrue(killed)
        self.assertIn("wall-clock timeout", out)
        self.assertLess(dt, nightly.KILL_AFTER_SECONDS + 10)
        self.assertEqual(unkilled, [])
        # No polling, no waiting: the sleeps are gone already.
        self.assertEqual(self._marker_procs(), [])

    def test_refreeze_catches_grace_period_spawn(self):
        # Hermetic (no ps, no spawned children): scripted snapshots model
        # a child born after SIGCONT during the grace period. The kill
        # must SIGSTOP and snapshot until no new pid appears, and the
        # SIGKILL pass must cover everything found.
        script = os.path.join(self.tmp, "sleeper.py")
        with open(script, "w") as fh:
            fh.write("import time\ntime.sleep(30)\n")
        os.chmod(script, os.stat(script).st_mode | stat.S_IEXEC)
        spec = dict(self.spec)
        spec["script"] = os.path.relpath(script, nightly.HERE)
        # Fake pids/pgids are large so they cannot collide with a real
        # process the kill pass would actually signal.
        views = [({400101}, {400101: 400101}),
                 ({400101, 400102}, {400101: 400101, 400102: 400102}),
                 ({400101, 400102},
                  {400101: 400101, 400102: 400102})]
        events = []
        calls = {"n": 0}
        real_snap = nightly._descendant_snapshot
        real_tree = nightly._signal_tree
        import signal as sigmod

        def fake_snap(pid):
            i = min(calls["n"], len(views) - 1)
            calls["n"] += 1
            events.append(("snapshot", set(views[i][0])))
            return (set(views[i][0]), dict(views[i][1]))

        def rec_tree(root, pids, pgids, sig):
            events.append(("signal", sig, set(pids)))
            return real_tree(root, pids, pgids, sig)

        nightly._descendant_snapshot = fake_snap
        nightly._signal_tree = rec_tree
        try:
            unkilled = []
            _rc, _out, killed = nightly.run_fuzzer(
                spec, "c", "o", "seeds", self.out, 1, 7, timeout=1.0,
                unkilled_pids=unkilled)
        finally:
            nightly._descendant_snapshot = real_snap
            nightly._signal_tree = real_tree
        self.assertTrue(killed)
        self.assertEqual(unkilled, [])
        snaps = [i for i, e in enumerate(events) if e[0] == "snapshot"]
        cont = next(i for i, e in enumerate(events)
                    if e[0] == "signal" and e[1] == sigmod.SIGCONT)
        kill = next(i for i, e in enumerate(events)
                    if e[0] == "signal" and e[1] == sigmod.SIGKILL)
        # First snapshot plus two re-freeze rounds (new pid, then
        # stable); the SIGKILL pass covers the grace-period child.
        self.assertEqual(len(snaps), 3)
        self.assertEqual(events[kill][2], {400101, 400102})
        # Order: SIGCONT, re-freeze SIGSTOP, last snapshot, SIGKILL --
        # nothing is born between the last snapshot and SIGKILL, and the
        # newly found pid was SIGSTOPped before it was re-snapshotted.
        self.assertLess(cont, snaps[-1])
        self.assertLess(snaps[-1], kill)
        stops = [i for i, e in enumerate(events)
                 if e[0] == "signal" and e[1] == sigmod.SIGSTOP]
        self.assertTrue(any(i < snaps[-1] and 400102 in events[i][2]
                            for i in stops))
        self.assertTrue(any(cont < i < snaps[-1] for i in stops))

    def test_freeze_precedes_snapshot(self):
        # Hermetic (no ps, no spawned children): the deadline kill must
        # SIGSTOP the fuzzer's group before snapshotting the tree, so no
        # child can be born between the snapshot and the fuzzer's death.
        script = os.path.join(self.tmp, "sleeper.py")
        with open(script, "w") as fh:
            fh.write("import time\ntime.sleep(30)\n")
        os.chmod(script, os.stat(script).st_mode | stat.S_IEXEC)
        spec = dict(self.spec)
        spec["script"] = os.path.relpath(script, nightly.HERE)
        calls = []
        real_killpg = os.killpg
        real_snap = nightly._descendant_snapshot
        real_ps = nightly._ps_table
        import signal as sigmod

        def rec_killpg(pid, sig):
            calls.append(("killpg", pid, sig))
            return real_killpg(pid, sig)

        def rec_snap(pid):
            calls.append(("snapshot", pid))
            return real_snap(pid)

        os.killpg = rec_killpg
        nightly._ps_table = lambda: {}
        nightly._descendant_snapshot = rec_snap
        try:
            unkilled = []
            _rc, _out, killed = nightly.run_fuzzer(
                spec, "c", "o", "seeds", self.out, 1, 7, timeout=1.0,
                unkilled_pids=unkilled)
        finally:
            os.killpg = real_killpg
            nightly._ps_table = real_ps
            nightly._descendant_snapshot = real_snap
        self.assertTrue(killed)
        stop = next(i for i, c in enumerate(calls)
                    if c[0] == "killpg" and c[2] == sigmod.SIGSTOP)
        snap = next(i for i, c in enumerate(calls) if c[0] == "snapshot")
        self.assertLess(stop, snap)
        self.assertEqual(unkilled, [])


# Fake fuzzer that traps SIGTERM and writes a marker file before
# exiting: without a SIGCONT after the deadline SIGTERM (sent while the
# fuzzer is SIGSTOPped) the handler never runs and the kill takes the
# full KILL_AFTER_SECONDS.
TERMHANDLER = """#!/usr/bin/env python3
import argparse, os, signal, sys
ap = argparse.ArgumentParser()
for flag in ("--candidate", "--oracle", "--seeds", "--out"):
    ap.add_argument(flag, default="x")
ap.add_argument("--iterations", type=int, default=1)
ap.add_argument("--seed", type=int, default=0)
ap.add_argument("--timeout", type=float, default=5.0)
a = ap.parse_args()
def on_term(signum, frame):
    with open(os.path.join(a.out, "term.marker"), "w") as fh:
        fh.write("term\\n")
    sys.exit(0)
signal.signal(signal.SIGTERM, on_term)
signal.pause()
print("done: 0 iterations: equal=0")
sys.stdout.flush()
"""


class NightlySigcontTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.out = os.path.join(self.tmp, "out")
        os.mkdir(self.out)
        path = os.path.join(self.tmp, "termhandler.py")
        with open(path, "w") as fh:
            fh.write(TERMHANDLER)
        os.chmod(path, os.stat(path).st_mode | stat.S_IEXEC)
        self.spec = dict(name="fz0", script=os.path.relpath(path,
                                                             nightly.HERE),
                         oracle=False, seeds=False, timeout=5.0, base=100,
                         offset=0)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_sigcont_after_sigterm_runs_cleanup(self):
        t0 = time.monotonic()
        unkilled = []
        _rc, out, killed = nightly.run_fuzzer(
            self.spec, "c", "o", "seeds", self.out, 1, 7, timeout=1.0,
            unkilled_pids=unkilled)
        dt = time.monotonic() - t0
        self.assertTrue(killed)
        self.assertIn("wall-clock timeout", out)
        # The TERM handler ran (marker written) and the kill returned
        # well before the SIGKILL grace period expired.
        self.assertTrue(
            os.path.isfile(os.path.join(self.out, "term.marker")))
        self.assertLess(dt, nightly.KILL_AFTER_SECONDS)
        self.assertEqual(unkilled, [])


DETACHED = """#!/usr/bin/env python3
import argparse, os, subprocess, sys, time
ap = argparse.ArgumentParser()
for flag in ("--candidate", "--oracle", "--seeds", "--out"):
    ap.add_argument(flag, default="x")
ap.add_argument("--iterations", type=int, default=1)
ap.add_argument("--seed", type=int, default=0)
ap.add_argument("--timeout", type=float, default=5.0)
a = ap.parse_args()
kid = subprocess.Popen(["sleep", "300"], start_new_session=True)
with open(os.path.join(a.out, "detached.pid"), "w") as fh:
    fh.write(str(kid.pid))
time.sleep(300)
print("done: 0 iterations: equal=0")
sys.stdout.flush()
"""


if __name__ == "__main__":
    unittest.main()
