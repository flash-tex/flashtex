#!/usr/bin/env python3
"""Unit tests for tools/fuzz/parsers/fontfiles.py. Stdlib unittest only.

Run as `python3 tools/fuzz/parsers/test_fontfiles.py`. The candidate is a
fake shell script; the real engine is never used, and the TeX tree is not
needed (the synthetic seeds stand in when kpsewhich finds nothing).
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

import fontfiles  # noqa: E402


def make_engine(tmpdir, body):
    path = os.path.join(tmpdir, "eng")
    with open(path, "w") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, 0o755)
    return path


# Panics when the job folder's fuzz.vf names itself (`fuzz` in a fnt_def).
SELF_REF_PANIC = ("if grep -q 'fuzz' fuzz.vf 2>/dev/null; then\n"
                  "  echo \"thread 'main' panicked at src/vf.rs:7:1:\"\n"
                  "  exit 101\nfi\nexit 0\n")


class VfTests(unittest.TestCase):
    def test_parse_the_synthetic_vf(self):
        vf = fontfiles.synthetic_vf()
        pre_end, defs, pkts = fontfiles.parse_vf(vf)
        self.assertEqual(pre_end, 11)
        self.assertEqual(len(defs), 1)
        self.assertEqual(vf[defs[0][2]:defs[0][3]], b"cmr10")
        self.assertEqual(len(pkts), 1)

    def test_self_reference_rewrites_the_font_name(self):
        vf = fontfiles.synthetic_vf()
        out, name = fontfiles._vf_self_reference(vf, random.Random(1))
        self.assertEqual(name, "vf-self-reference")
        _, defs, pkts = fontfiles.parse_vf(out)
        self.assertEqual(out[defs[0][2]:defs[0][3]], b"fuzz")
        self.assertEqual(len(pkts), 1, "the rest of the file is intact")

    def test_dvi_insert_keeps_the_packet_length_consistent(self):
        vf = fontfiles.synthetic_vf()
        rng = random.Random(3)
        for _ in range(50):
            out, name = fontfiles._vf_dvi(vf, rng)
            if name == "vf-dvi-insert":
                self.assertIsNotNone(fontfiles.parse_vf(out), name)

    def test_every_mutation_is_deterministic(self):
        for kind in fontfiles.KINDS:
            base = {"vf": fontfiles.synthetic_vf(), "enc": fontfiles.synthetic_enc(),
                    "map": None}[kind]
            a = [fontfiles.mutate_with_info(kind, base, random.Random(9)) for _ in range(3)]
            b = [fontfiles.mutate_with_info(kind, base, random.Random(9)) for _ in range(3)]
            self.assertEqual(a, b, kind)


class RunTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="test-fontfiles-")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_classes(self):
        self.assertEqual(fontfiles.classify(0, ""), "ok")
        self.assertEqual(fontfiles.classify(101, "panicked at"), "crash")
        self.assertEqual(fontfiles.classify(-11, ""), "crash")
        self.assertEqual(fontfiles.classify(1, "!pdfTeX error: bad"), "rejected")
        self.assertEqual(fontfiles.classify(1, "something"), "graceful-error")

    def test_job_files(self):
        for kind in fontfiles.KINDS:
            files = fontfiles.job_files(kind, b"data", b"tfm")
            self.assertIn("job.tex", files)
            self.assertEqual(files["fuzz." + kind], b"data")

    def test_a_crash_is_stored_once_with_its_sidecar(self):
        eng = make_engine(self.tmp, SELF_REF_PANIC)
        seeds = [("synthetic.vf", fontfiles.synthetic_vf(), b"")]
        out = os.path.join(self.tmp, "out")
        counts = fontfiles.run_fuzz("vf", eng, seeds, out, 40, 5, 5.0)
        self.assertGreater(counts["crash"], 0)
        stored = os.listdir(os.path.join(out, "crash"))
        self.assertEqual(sorted(n.rsplit(".", 1)[1] for n in stored), ["json", "vf"],
                         "one stored case per distinct signature")
        side = [n for n in stored if n.endswith(".json")][0]
        with open(os.path.join(out, "crash", side)) as fh:
            info = json.load(fh)
        self.assertEqual(info["kind"], "vf")
        self.assertIn("vf.rs:7", info["signature"])

    def test_a_hang_is_classified(self):
        eng = make_engine(self.tmp, "sleep 5\n")
        cls, rc, _ = fontfiles.run_one("enc", fontfiles.synthetic_enc(), None, eng, 0.5)
        self.assertEqual(cls, "hang")
        self.assertIsNone(rc)


if __name__ == "__main__":
    unittest.main()
