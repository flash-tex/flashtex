#!/usr/bin/env python3
"""Unit tests for tools/fuzz/parsers/pdfinc.py. Stdlib unittest only.

Run as `python3 tools/fuzz/parsers/test_pdfinc.py` from the repo root.
Engines are small shell scripts written to a temp dir (in the style of
tools/fuzz/test_fuzz.py); the real candidate is never used.
"""
import importlib.util
import json
import os
import random
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "pdfinc", os.path.join(HERE, "pdfinc.py"))
pdfinc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pdfinc)

MINIPDF = (b"%PDF-1.4\n"
           b"1 0 obj\n<< /Type /Pages /Kids[2 0 R] /Count 1 /N 3 >>\nendobj\n"
           b"2 0 obj\n<< /Type /Page /Parent 1 0 R /Length 11 >>\n"
           b"stream\nhello world\nendstream\nendobj\n"
           b"xref\n0 5\n0000000000 65535 f \n0000000009 00000 n \n"
           b"0000000100 00000 n \n"
           b"trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n200\n%%EOF\n")
MARKED = MINIPDF + b"% MARKERXYZ\n"

OK_BODY = "for last do :; done\nexit 0\n"
FAIL_BODY = "for last do :; done\nexit 1\n"
PANIC_BODY = ("for last do :; done\n"
              "echo \"thread 'main' panicked at 'boom', src/pdf.rs:42:7\"\n"
              "exit 101\n")
MARKER_BODY = ("for last do :; done\n"
               "d=${last%/*}\n[ \"$d\" = \"$last\" ] && d=.\n"
               "if grep -q 'MARKERXYZ' \"$d/fuzz.pdf\" 2>/dev/null; then\n"
               "  echo \"thread 'main' panicked at 'boom', src/pdf.rs:42:7\"\n"
               "  exit 101\nfi\nexit 0\n")
SLEEP_BODY = "sleep 30\nexit 0\n"


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, 0o755)
    return path


class MutateTest(unittest.TestCase):
    def test_deterministic(self):
        self.assertEqual(pdfinc.mutate_pdf(MINIPDF, random.Random(7)),
                         pdfinc.mutate_pdf(MINIPDF, random.Random(7)))

    def test_structure_ops_apply(self):
        rng = random.Random(3)
        self.assertEqual(pdfinc._m_byteflip(MINIPDF, rng)[1], "byteflip")
        out, desc = pdfinc._m_truncate(MINIPDF, rng)
        self.assertTrue(desc.startswith("truncate@") and len(out) < len(MINIPDF))
        for op in (pdfinc._m_chunkdel, pdfinc._m_chunkdup, pdfinc._m_xref,
                   pdfinc._m_startxref, pdfinc._m_trailer, pdfinc._m_length,
                   pdfinc._m_count, pdfinc._m_objstm, pdfinc._m_kids,
                   pdfinc._m_cycle, pdfinc._m_streamcut):
            out, desc = op(MINIPDF, rng)
            self.assertIsNotNone(out, op.__name__)
        self.assertNotIn(b"trailer", pdfinc._m_trailer(MINIPDF, rng)[0])
        self.assertIn(b"/Kids[0 0 R 0 0 R]",
                      pdfinc._m_kids(MINIPDF, rng)[0])
        self.assertIn(b"/FzCycle", pdfinc._m_cycle(MINIPDF, rng)[0])


class RunCaseTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="pdfinc-test-")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_classes(self):
        ok = make_engine(self.tmp, "ok.sh", OK_BODY)
        fail = make_engine(self.tmp, "fail.sh", FAIL_BODY)
        marker = make_engine(self.tmp, "marker.sh", MARKER_BODY)
        sleepy = make_engine(self.tmp, "sleep.sh", SLEEP_BODY)
        self.assertEqual(pdfinc.run_case(MINIPDF, ok, 10)[0], "ok")
        self.assertEqual(pdfinc.run_case(MINIPDF, fail, 10)[0],
                         "graceful-error")
        cls, rc, log = pdfinc.run_case(MARKED, marker, 10)
        self.assertEqual(cls, "crash")
        self.assertEqual(rc, 101)
        self.assertEqual(pdfinc.panic_location(log), "src/pdf.rs:42")
        self.assertEqual(pdfinc.run_case(MINIPDF, marker, 10)[0], "ok")
        self.assertEqual(pdfinc.run_case(MINIPDF, sleepy, 0.2)[0], "hang")


class FuzzTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="pdfinc-fuzz-")
        self.seeds = [("s%d" % i, MARKED) for i in range(3)]

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_determinism_counts_and_artifacts(self):
        panic = make_engine(self.tmp, "panic.sh", PANIC_BODY)
        outs = [os.path.join(self.tmp, "out%d" % i) for i in (1, 2)]
        counts = [pdfinc.run_fuzz(panic, self.seeds, out, 10, 99, 10)
                  for out in outs]
        self.assertEqual(counts[0], counts[1])
        self.assertEqual(sum(counts[0].values()), 10)
        self.assertEqual(counts[0]["crash"], 10)
        files = [sorted(os.listdir(os.path.join(o, "crash"))) for o in outs]
        self.assertEqual(files[0], files[1])
        # One panic location: deduped to a single stored case.
        pdfs = [f for f in files[0] if f.endswith(".pdf")]
        self.assertEqual(len(pdfs), 1)
        with open(os.path.join(outs[0], "crash",
                               pdfs[0][:-4] + ".json")) as fh:
            info = json.load(fh)
        for key in ("seed", "mutation", "returncode", "last_stderr_line",
                    "panic_location"):
            self.assertIn(key, info)
        self.assertEqual(info["returncode"], 101)

    def test_ok_stores_nothing(self):
        ok = make_engine(self.tmp, "ok.sh", OK_BODY)
        out = os.path.join(self.tmp, "out-ok")
        counts = pdfinc.run_fuzz(ok, self.seeds, out, 5, 7, 10)
        self.assertEqual(counts["ok"], 5)
        self.assertFalse(os.path.exists(out))


if __name__ == "__main__":
    unittest.main()
