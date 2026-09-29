#!/usr/bin/env python3
"""Unit tests for tools/fuzz/parsers/tfm.py. Stdlib unittest only.

Run as `python3 tools/fuzz/parsers/test_tfm.py` from the repo root, or
`python3 -m unittest discover -s tools/fuzz`. The candidate is a fake
shell script that panics when fuzz.tfm contains a marker byte string;
the real candidate is never used.
"""
import json
import os
import random
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import tfm

MARKER = b"TFM-CRASH-MARKER-42"
MARKER_BODY = ("for last do :; done\n"
               "d=$(dirname \"$last\")\n"
               "if grep -q 'TFM-CRASH-MARKER-42' \"$d/fuzz.tfm\" "
               "2>/dev/null; then\n"
               "  echo \"thread 'main' panicked at 'marker hit', "
               "src/tfm.rs:99:3\"\n"
               "  exit 101\n"
               "fi\n"
               "exit 0\n")
SEED = MARKER * 8 + bytes(range(256))


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, 0o755)
    return path


class TfmTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="tfm-test-")
        self.marker = make_engine(self.tmp, "marker.sh", MARKER_BODY)
        self.ok = make_engine(self.tmp, "ok.sh", "exit 0\n")
        self.out = os.path.join(self.tmp, "out")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_mutate_deterministic(self):
        rng = random.Random(7)
        self.assertEqual(tfm.mutate_with_info(SEED, random.Random(7)),
                         tfm.mutate_with_info(SEED, random.Random(7)))
        # Every op kind appears over 200 draws (all draws via rng).
        descs = {tfm.mutate_with_info(SEED, rng)[1].split("@")[0]
                 for _ in range(200)}
        self.assertTrue({"flip", "trunc", "del", "dup", "field"} <= descs)

    def test_run_fuzz_deterministic(self):
        seeds = [("m.tfm", SEED)]
        a = tfm.run_fuzz(self.marker, seeds, os.path.join(self.tmp, "a"),
                         20, 11, 10)
        b = tfm.run_fuzz(self.marker, seeds, os.path.join(self.tmp, "b"),
                         20, 11, 10)
        self.assertEqual(a, b)
        self.assertEqual(sum(a.values()), 20)
        self.assertGreater(a["crash"], 0)

    def test_class_counts(self):
        seeds = [("m.tfm", SEED)]
        counts = tfm.run_fuzz(self.marker, seeds, self.out, 20, 11, 10)
        self.assertEqual(sum(counts.values()), 20)
        self.assertEqual(counts["hang"], 0)
        self.assertGreater(counts["crash"], 0)
        counts = tfm.run_fuzz(self.ok, seeds,
                              os.path.join(self.tmp, "out-ok"), 5, 11, 10)
        self.assertEqual(counts, {"ok": 5, "graceful-error": 0,
                                  "crash": 0, "hang": 0})

    def test_crash_artifacts_and_dedupe(self):
        seeds = [("m.tfm", SEED)]
        counts = tfm.run_fuzz(self.marker, seeds, self.out, 20, 11, 10)
        cls_dir = os.path.join(self.out, "crash")
        tfms = sorted(f for f in os.listdir(cls_dir) if f.endswith(".tfm"))
        # One panic location: exactly one stored case, however many crashed.
        self.assertEqual(len(tfms), 1)
        self.assertEqual(len(tfms[0]), 16 + len(".tfm"))
        with open(os.path.join(cls_dir, tfms[0]), "rb") as fh:
            self.assertIn(MARKER, fh.read())
        with open(os.path.join(cls_dir, tfms[0][:-4] + ".json")) as fh:
            info = json.load(fh)
        for key in ("seed", "mutation", "returncode", "last_line",
                    "panic_location", "signature"):
            self.assertIn(key, info)
        self.assertEqual(info["returncode"], 101)
        self.assertEqual(info["seed"], 11)
        self.assertIn("src/tfm.rs", info["panic_location"])
        # Repeat run: signature already on disk, nothing new stored.
        tfm.run_fuzz(self.marker, seeds, self.out, 20, 11, 10)
        self.assertEqual(sorted(f for f in os.listdir(cls_dir)
                                if f.endswith(".tfm")), tfms)

    def test_hang_artifact(self):
        sleepy = make_engine(self.tmp, "sleep.sh", "sleep 30\nexit 0\n")
        counts = tfm.run_fuzz(sleepy, [("m.tfm", SEED)],
                              os.path.join(self.tmp, "out-hang"),
                              1, 3, 0.2)
        self.assertEqual(counts["hang"], 1)
        hang_dir = os.path.join(self.tmp, "out-hang", "hang")
        tfms = [f for f in os.listdir(hang_dir) if f.endswith(".tfm")]
        self.assertEqual(len(tfms), 1)


if __name__ == "__main__":
    unittest.main()
