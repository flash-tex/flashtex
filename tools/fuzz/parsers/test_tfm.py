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
import struct
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


REJECT_BODY = ("echo '! Font \\x=fuzz not loadable: "
               "Bad metric (TFM) file.'\nexit 1\n")


def mini_tfm(no_exten=False, no_ligkern=False):
    """A tiny self-consistent TFM: lh=2, bc=0, ec=1, one entry per
    table, nl=2, nk=1, ne=1, np=1 (76 bytes, lf=19)."""
    nl = 0 if no_ligkern else 2
    ne = 0 if no_exten else 1
    words = 6 + 2 + 2 + 1 + 1 + 1 + 1 + nl + 1 + ne + 1
    head = struct.pack(">12H", words, 2, 0, 1, 1, 1, 1, 1, nl, 1,
                       ne, 1)
    body = (b"\x00" * 8                  # lh header words
            + b"\x00\x00\x00\x00" * 2    # char_info x2
            + b"\x00\x00\x10\x00"        # width
            + b"\x00\x00\x10\x00"        # height
            + b"\x00\x00\x10\x00"        # depth
            + b"\x00\x00\x10\x00"        # italic
            + (b"" if no_ligkern else b"\x00\x41\x00\x05"  # ligkern
               + b"\x80\x00\x00\x00")
            + b"\x00\x00\x10\x00"        # kern
            + (b"" if no_exten else b"\x01\x02\x01\x02")  # exten
            + b"\x00\x00\x10\x00")       # param
    blob = head + body
    assert len(blob) == 4 * words
    return blob


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
        self.assertEqual(counts, {"ok": 5, "tfm-rejected": 0,
                                  "graceful-error": 0, "crash": 0,
                                  "hang": 0})

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

    def test_job_maps_fuzz_font(self):
        self.assertIn("\\pdfmapline{+fuzz fuzz <cmr10.pfb}", tfm.JOB_TEX)
        self.assertIn("\\font\\x=fuzz", tfm.JOB_TEX)

    def test_classify(self):
        self.assertEqual(tfm.classify(101, "thread panicked at x"), "crash")
        self.assertEqual(tfm.classify(1, "! Font \\x=fuzz not loadable: "
                                         "Bad metric (TFM) file."),
                         "tfm-rejected")
        self.assertEqual(tfm.classify(1, "not loadable: Metric (TFM) "
                                         "file not found"),
                         "tfm-rejected")
        self.assertEqual(tfm.classify(0, ""), "ok")
        self.assertEqual(tfm.classify(1, "some other error"), "graceful-error")

    def test_tfm_rejected_run(self):
        reject = make_engine(self.tmp, "reject.sh", REJECT_BODY)
        cls, rc, _log = tfm.run_one(SEED, reject, 10)
        self.assertEqual((cls, rc), ("tfm-rejected", 1))
        counts = tfm.run_fuzz(reject, [("m.tfm", SEED)],
                              os.path.join(self.tmp, "out-reject"),
                              5, 11, 10)
        self.assertEqual(counts["tfm-rejected"], 5)

    def test_parse_tfm(self):
        blob = mini_tfm()
        info = tfm.parse_tfm(blob)
        self.assertEqual([info[k] for k in tfm.FIELD_NAMES],
                         [19, 2, 0, 1, 1, 1, 1, 1, 2, 1, 1, 1])
        self.assertEqual(info["nchars"], 2)
        self.assertEqual(info["total"], len(blob))
        self.assertEqual(info["total"], 4 * info["lf"])
        start, end = info["spans"]["char_info"]
        self.assertTrue(0 <= start < end <= len(blob))
        self.assertIsNone(tfm.parse_tfm(b"short"))
        # ec < bc means an empty char_info table, not a huge one.
        bad = struct.pack(">12H", 7, 0, 5, 3, 0, 0, 0, 0, 0, 0, 0, 1)
        self.assertEqual(tfm.parse_tfm(bad + b"\x00" * 4)["nchars"], 0)

    def test_structure_mutations(self):
        blob = mini_tfm()
        for fn, prefix in ((tfm._charinfo, "charinfo@"),
                           (tfm._index, "index@"),
                           (tfm._ligkern, "ligkern@"),
                           (tfm._exten, "exten@"),
                           (tfm._tablen, "tablen@")):
            got, desc = fn(blob, random.Random(4))
            self.assertNotEqual(got, blob)
            self.assertTrue(desc.startswith(prefix), desc)
        # Empty tables opt out so the generic mutations still apply.
        self.assertIsNone(tfm._exten(mini_tfm(no_exten=True),
                                     random.Random(4)))
        self.assertIsNone(tfm._ligkern(mini_tfm(no_ligkern=True),
                                       random.Random(4)))

    def test_tablen_breaks_lf(self):
        blob = mini_tfm()
        got, desc = tfm._tablen(blob, random.Random(0))
        info = tfm.parse_tfm(got)
        self.assertNotEqual(got[:24], blob[:24])
        self.assertNotEqual(info["total"], 4 * info["lf"])

    def test_structure_ops_appear(self):
        rng = random.Random(7)
        descs = {tfm.mutate_with_info(mini_tfm(), rng)[1].split("@")[0]
                 for _ in range(400)}
        self.assertTrue({"charinfo", "index", "ligkern", "exten",
                         "tablen"} <= descs)

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
