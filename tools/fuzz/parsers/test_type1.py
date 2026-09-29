#!/usr/bin/env python3
"""Unit tests for tools/fuzz/parsers/type1.py. Stdlib unittest only.

Run as `python3 -m unittest discover -s tools/fuzz` from the repo root.
Engines are small shell scripts written to a temp dir; the real candidate
is never used.
"""
import json
import os
import random
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import type1

MARKER = b"TYPE1MARKER123"
SEED_M = ("mark.pfb", b"\x80\x01\x10\x00\x00\x00hello " + MARKER + b"!!"
          + b"\x80\x03\x00\x00\x00\x00")
SEED_P = ("plain.pfb", b"\x80\x01\x05\x00\x00\x00hello"
          + b"\x80\x03\x00\x00\x00\x00")
SEED_E = ("eexec.pfb", b"\x80\x01\x05\x00\x00\x00hello"
          + b"\x80\x02\x04\x00\x00\x00\xde\xad\xbe\xef"
          + b"\x80\x03\x00\x00\x00\x00")
TFM = b"fake-tfm"

PANIC_BODY = ("#!/bin/sh\n"
              "if grep -q 'TYPE1MARKER123' fuzz.pfb 2>/dev/null; then\n"
              "  echo \"thread 'main' panicked at 't1 boom', src/t1.rs:42:7\""
              " >&2\n"
              "  exit 101\n"
              "fi\n"
              "exit 0\n")
ALWAYS_CRASH = ("#!/bin/sh\necho \"thread 'main' panicked at src/x.rs:1:2\""
                " >&2\nexit 101\n")
ALWAYS_OK = "#!/bin/sh\nexit 0\n"
ALWAYS_FAIL = "#!/bin/sh\nexit 1\n"
SLEEPER = "#!/bin/sh\nsleep 30\nexit 0\n"


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write(body)
    os.chmod(path, 0o755)
    return path


class Type1Test(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="t1test-")
        self.out = os.path.join(self.tmp, "out")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_mutate_deterministic(self):
        data = SEED_M[1]
        self.assertEqual(type1.mutate_with_info(data, random.Random(9)),
                         type1.mutate_with_info(data, random.Random(9)))

    def test_format_aware_edits_cover_headers_and_eexec(self):
        kinds = set()
        for s in range(60):
            _b, desc = type1.mutate_with_info(SEED_E[1], random.Random(s))
            kinds.add(desc.split("@")[0].split(":")[0])
        self.assertTrue({"segtype", "seglen", "eexec-trunc"} <= kinds)

    def test_deterministic_counts_and_files(self):
        cand = make_engine(self.tmp, "cand", PANIC_BODY)
        kw = dict(seeds=[SEED_M, SEED_P], tfm=TFM, timeout=10)
        c1 = type1.run_fuzz(cand, 12, 1234, self.out + "1", **kw)
        c2 = type1.run_fuzz(cand, 12, 1234, self.out + "2", **kw)
        self.assertEqual(c1, c2)
        self.assertEqual(sum(c1.values()), 12)
        snap = lambda d: sorted(os.listdir(os.path.join(d, "crash"))) \
            if os.path.isdir(os.path.join(d, "crash")) else []
        self.assertEqual(snap(self.out + "1"), snap(self.out + "2"))
        self.assertGreater(c1["crash"], 0)

    def test_class_counts(self):
        ok = make_engine(self.tmp, "ok", ALWAYS_OK)
        self.assertEqual(type1.run_fuzz(
            ok, 4, 1, self.out, timeout=10, seeds=[SEED_P], tfm=TFM),
            {"crash": 0, "hang": 0, "ok": 4, "graceful-error": 0})
        fail = make_engine(self.tmp, "fail", ALWAYS_FAIL)
        self.assertEqual(type1.run_fuzz(
            fail, 3, 1, self.out, timeout=10, seeds=[SEED_P], tfm=TFM)
            ["graceful-error"], 3)
        crash = make_engine(self.tmp, "crash", ALWAYS_CRASH)
        self.assertEqual(type1.run_fuzz(
            crash, 3, 1, self.out, timeout=10, seeds=[SEED_P], tfm=TFM)
            ["crash"], 3)

    def test_hang(self):
        sleepy = make_engine(self.tmp, "sleep", SLEEPER)
        counts = type1.run_fuzz(sleepy, 1, 1, self.out, timeout=0.2,
                                seeds=[SEED_P], tfm=TFM)
        self.assertEqual(counts["hang"], 1)
        names = os.listdir(os.path.join(self.out, "hang"))
        self.assertTrue(any(n.endswith(".pfb") for n in names))
        self.assertTrue(any(n.endswith(".json") for n in names))

    def test_artifacts(self):
        crash = make_engine(self.tmp, "crash", ALWAYS_CRASH)
        type1.run_fuzz(crash, 6, 77, self.out, timeout=10, seeds=[SEED_P],
                       tfm=TFM)
        cdir = os.path.join(self.out, "crash")
        blobs = sorted(n for n in os.listdir(cdir) if n.endswith(".pfb"))
        self.assertTrue(blobs)
        for blob in blobs:
            with open(os.path.join(cdir, blob[:-4] + ".json")) as fh:
                info = json.load(fh)
            for key in ("seed", "mutation", "returncode",
                        "last_stderr_line", "panic_location"):
                self.assertIn(key, info)
            self.assertEqual(info["returncode"], 101)
            self.assertIn("src/x.rs", info["panic_location"])
        with open(os.path.join(self.out, "signatures.json")) as fh:
            sigs = json.load(fh)
        self.assertIn("panic:src/x.rs", sigs)


if __name__ == "__main__":
    unittest.main()
