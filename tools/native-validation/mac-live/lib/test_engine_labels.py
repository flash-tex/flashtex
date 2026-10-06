#!/usr/bin/env python3
"""The attribution tools read the shell's `engine:` label (retirement plan #1236, S3r).

Run: python3 -m unittest discover -s tools/native-validation/mac-live/lib -p 'test_*.py'
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import typing_attribution as ta  # noqa: E402

LOG = [
    "2026-10-06T10:00:00Z\tstatus: attached: flashtex-compiler",
    "2026-10-06T10:00:01Z\tengine: new (user) main.tex",
    "2026-10-06T10:00:02Z\tengine: previous (user; fallback from new: no TeX Live is installed) main.tex",
]


class EngineLabelTests(unittest.TestCase):
    def test_the_last_engine_line_wins_with_its_reason(self):
        self.assertEqual(ta.engine_of(LOG), {"engine": "previous", "why": "user; fallback from new: no TeX Live is installed"})

    def test_a_log_without_a_label_has_no_engine(self):
        self.assertIsNone(ta.engine_of(LOG[:1]))

    def test_launch_summary_records_the_engine(self):
        with tempfile.TemporaryDirectory() as d:
            ev = os.path.join(d, "evidence.md")
            open(ev, "w").write("- FlashTeX running, pid=1\n\n## FLASHTEX_LOG (x)\n\n```\n" + "\n".join(LOG) + "\n```\n")
            opens = os.path.join(d, "open.log")
            open(opens, "w").write("")
            out = os.path.join(d, "out.json")
            subprocess.run([sys.executable, os.path.join(HERE, "launch_summary.py"), "--evidence", ev,
                            "--open-log", opens, "--exit", "0", "--out", out], check=True, capture_output=True)
            s = json.load(open(out))
            self.assertEqual(s["engine"], "previous")
            self.assertEqual(s["log_counts"]["engine:"], 2)


if __name__ == "__main__":
    unittest.main()
