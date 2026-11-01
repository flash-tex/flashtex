#!/usr/bin/env python3
"""launch_summary.py reads the shell's `engine:` label (retirement plan #1236,
S3r) and, from S6 on, attributes only the new engine.

Run: python3 -m unittest discover -s tools/native-validation/mac-live/lib -p 'test_*.py'
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))

NEW_LOG = [
    "2026-10-21T10:00:00Z\tengine-v3: starting /Applications/FlashTeX.app/Contents/Helpers/flashtex-host",
    "2026-10-21T10:00:01Z\tengine: new (classic) main.tex",
    "2026-10-21T10:00:02Z\tengine: new (unicode) main.tex",
]
# A log of the retired old engine's routes: its label is not attributed.
OLD_LOG = [
    "2026-10-06T10:00:00Z\tcompile: applied revision 1 at 1000",
    "2026-10-06T10:00:02Z\tengine: previous (user; fallback from new: no TeX Live is installed) main.tex",
]
NOTES = [
    "FlashTeX running, pid=1",
    "log shows the engine host starting within 1s",
    "flashtex-host started, pid=2 (child of 1)",
    "FlashTeX (pid 1) is still running after its engine host was killed",
    "log shows the host's exit within 5s",
    "log shows the host restarting within 5s",
    "FlashTeX quit cleanly (SIGTERM to pid 1)",
]


def summarise(log, notes=("FlashTeX running, pid=1",)):
    with tempfile.TemporaryDirectory() as d:
        ev = os.path.join(d, "evidence.md")
        open(ev, "w").write("".join("- %s\n" % n for n in notes) + "\n## FLASHTEX_LOG (x)\n\n```\n" + "\n".join(log) + "\n```\n")
        opens = os.path.join(d, "open.log")
        open(opens, "w").write("")
        out = os.path.join(d, "out.json")
        subprocess.run([sys.executable, os.path.join(HERE, "launch_summary.py"), "--evidence", ev,
                        "--open-log", opens, "--exit", "0", "--out", out], check=True, capture_output=True)
        return json.load(open(out))


class EngineLabelTests(unittest.TestCase):
    def test_launch_summary_records_the_new_engine(self):
        s = summarise(NEW_LOG)
        self.assertEqual(s["engine"], "new")
        self.assertEqual(s["log_counts"]["engine:"], 2)

    def test_a_log_with_an_old_route_is_not_attributed(self):
        s = summarise(OLD_LOG)
        self.assertIsNone(s["engine"])

    def test_a_log_without_a_label_has_no_engine(self):
        self.assertIsNone(summarise(NEW_LOG[:1])["engine"])

    def test_the_engine_host_notes_are_summarised(self):
        s = summarise(NEW_LOG, NOTES)
        self.assertEqual(s["pids"], {"FlashTeX": 1, "flashtex-host": 2})
        for k in ("host_start_logged", "host_started", "app_survived_host_kill", "host_exit_logged", "host_restart_logged", "quit_cleanly"):
            self.assertTrue(s[k], k)
        self.assertEqual(s["log_counts"]["engine-v3: starting"], 1)


if __name__ == "__main__":
    unittest.main()
