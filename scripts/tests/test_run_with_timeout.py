#!/usr/bin/env python3
"""Tests for scripts/run-with-timeout.py, the trip/etrip runs' timeout
(#1208). Stdlib unittest; about 10 seconds.

    python3 scripts/tests/test_run_with_timeout.py
"""
import os
import subprocess
import sys
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
HELPER = os.path.join(HERE, "..", "run-with-timeout.py")


def helper(*args, **kw):
    return subprocess.run([sys.executable, HELPER] + list(args),
                          capture_output=True, text=True, **kw)


def running(pid):
    """True while pid exists and is not a zombie."""
    p = subprocess.run(["ps", "-o", "stat=", "-p", str(pid)],
                       capture_output=True, text=True)
    st = p.stdout.strip()
    return bool(st) and not st.startswith("Z")


class RunWithTimeoutTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="rwt-test-")
        self.log = os.path.join(self.tmp, "timeouts.txt")

    def tearDown(self):
        subprocess.run(["rm", "-rf", self.tmp])

    def test_status_and_stdio_pass_through(self):
        p = helper("5", "--", "sh", "-c", "cat; echo err >&2; exit 3",
                   input="hello\n")
        self.assertEqual(p.returncode, 3)
        self.assertEqual(p.stdout, "hello\n")
        self.assertIn("err", p.stderr)

    def test_signal_death_is_128_plus_n(self):
        p = helper("5", "--", "sh", "-c", "kill -9 $$")
        self.assertEqual(p.returncode, 137)

    def test_timeout_kills_the_whole_group_and_logs(self):
        pids = os.path.join(self.tmp, "pids")
        # A child that ignores SIGTERM and a background grandchild.
        script = ("trap '' TERM; (trap '' TERM; while :; do sleep 1; done) & "
                  "echo $! >> %s; echo $$ >> %s; while :; do sleep 1; done"
                  % (pids, pids))
        start = time.monotonic()
        p = helper("1", "--log", self.log, "--", "sh", "-c", script)
        took = time.monotonic() - start
        self.assertEqual(p.returncode, 124)
        self.assertLess(took, 15)
        with open(self.log) as fh:
            self.assertIn("timed out after 1s: sh -c", fh.read())
        time.sleep(0.5)
        with open(pids) as fh:
            alive = [int(x) for x in fh.read().split() if running(int(x))]
        self.assertEqual(alive, [])

    def test_no_log_line_without_timeout(self):
        p = helper("5", "--log", self.log, "--", "true")
        self.assertEqual(p.returncode, 0)
        self.assertFalse(os.path.exists(self.log))

    def test_bad_arguments_exit_2(self):
        for args in (["x", "--", "true"], ["0", "--", "true"],
                     ["5", "true"], ["5", "--log"], ["5", "--"]):
            self.assertEqual(helper(*args).returncode, 2, args)

    def test_missing_command_is_127(self):
        p = helper("5", "--", os.path.join(self.tmp, "no-such-command"))
        self.assertEqual(p.returncode, 127)


if __name__ == "__main__":
    unittest.main()
