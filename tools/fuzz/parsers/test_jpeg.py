#!/usr/bin/env python3
"""Unit tests for tools/fuzz/parsers/jpeg.py. Stdlib unittest only.

Run as `python3 tools/fuzz/parsers/test_jpeg.py` from the repo root, or
via `python3 -m unittest discover -s tools/fuzz`. The candidate is always
a fake shell script in a temp dir: it panics (exit 101) when fuzz.jpg
contains a marker byte string, sleeps when it contains another, and
exits 0 otherwise. The real engine is never used.
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
sys.path.insert(0, os.path.dirname(HERE))

import jpeg

CRASH_MARK = b"CRASHMARKXYZ"
HANG_MARK = b"HANGMARKXYZ"

FAKE_BODY = ("""for last do :; done
dir=$(dirname "$last")
if grep -qa 'CRASHMARKXYZ' "$dir"/fuzz.jpg 2>/dev/null; then
  echo "thread 'main' panicked at 'jpeg boom', src/jpeg.rs:12:5" >&2
  exit 101
fi
if grep -qa 'HANGMARKXYZ' "$dir"/fuzz.jpg 2>/dev/null; then
  sleep 30
fi
exit 0
""")
SLEEP_BODY = "sleep 30\nexit 0\n"


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, 0o755)
    return path


def stored_files(out_dir):
    found = []
    for root, _dirs, files in os.walk(out_dir):
        found.extend(os.path.join(root, f) for f in files)
    return sorted(found)


class MutateTest(unittest.TestCase):
    def setUp(self):
        self.seed = jpeg.generate_minimal_jpeg()

    def test_deterministic(self):
        self.assertEqual(jpeg.mutate(self.seed, random.Random(42)),
                         jpeg.mutate(self.seed, random.Random(42)))

    def test_returns_bytes_and_desc(self):
        for s in range(30):
            data, desc = jpeg.mutate(self.seed, random.Random(s))
            self.assertIsInstance(data, bytes)
            self.assertTrue(desc)

    def test_minimal_seed_parses(self):
        self.assertTrue(self.seed.startswith(b"\xff\xd8"))
        self.assertTrue(self.seed.endswith(b"\xff\xd9"))
        marks = [m for m, _, _, _ in jpeg.parse_segments(self.seed)]
        self.assertIn(0xC0, marks)
        self.assertIn(0xDB, marks)
        self.assertIn(0xC4, marks)

    def test_sof_edit_changes_dims(self):
        # sof-prec may redraw the existing value (a no-op); require a
        # sof draw that actually changes bytes (kind/width/height do).
        for s in range(500):
            data, desc = jpeg.mutate(self.seed, random.Random(s))
            if desc.startswith("sof-") and data != self.seed:
                self.assertEqual(len(data), len(self.seed))
                return
        self.fail("no differing sof draw in 500 seeds")

    def test_trunc_before_eoi_drops_eoi(self):
        for s in range(200):
            data, desc = jpeg.mutate(self.seed, random.Random(s))
            if desc.startswith("trunc-before-eoi"):
                self.assertFalse(data.endswith(b"\xff\xd9"))
                return
        self.fail("no trunc-before-eoi draw in 200 seeds")


class ClassifyTest(unittest.TestCase):
    def test_crash(self):
        self.assertEqual(jpeg.classify(101, "panicked at x", False), "crash")
        self.assertEqual(jpeg.classify(-11, "", False), "crash")
        self.assertEqual(jpeg.classify(1, "panicked at y", False), "crash")

    def test_hang_ok_graceful(self):
        self.assertEqual(jpeg.classify(None, "", True), "hang")
        self.assertEqual(jpeg.classify(0, "ok", False), "ok")
        self.assertEqual(jpeg.classify(1, "! error", False), "graceful-error")

    def test_panic_location(self):
        loc = jpeg.panic_location("thread 'main' panicked at 'b', "
                                  "src/jpeg.rs:12:5")
        self.assertEqual(loc, "'b', src/jpeg.rs")
        self.assertIsNone(jpeg.panic_location("no panic here"))


class FuzzLoopTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="jpeg-test-")
        self.out = os.path.join(self.tmp, "out")
        self.fake = make_engine(self.tmp, "fake.sh", FAKE_BODY)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def args(self, cand=None, n=12, seed=7, timeout=10):
        return ["--candidate", cand or self.fake, "--out", self.out,
                "--iterations", str(n), "--seed", str(seed),
                "--timeout", str(timeout)]

    def stub_mutate(self, pattern):
        state = {"i": 0}

        def stub(_base, _rng):
            i = state["i"]
            state["i"] += 1
            mark = pattern(i)
            return mark + bytes([i % 256]), "stub-%d" % i
        return stub

    def test_counts_and_artifacts(self):
        real = jpeg.mutate
        jpeg.mutate = self.stub_mutate(
            lambda i: CRASH_MARK if i % 3 == 0 else b"clean")
        try:
            counts = jpeg.main(self.args(n=12))
        finally:
            jpeg.mutate = real
        self.assertEqual(counts, {"crash": 4, "hang": 0, "ok": 8,
                                 "graceful-error": 0})
        jpgs = [f for f in stored_files(self.out) if f.endswith(".jpg")]
        # One panic location: deduped to a single stored crash input.
        self.assertEqual(len(jpgs), 1)
        with open(jpgs[0], "rb") as fh:
            self.assertIn(CRASH_MARK, fh.read())
        with open(jpgs[0][:-4] + ".json") as fh:
            info = json.load(fh)
        for key in ("seed", "mutation", "returncode", "last_stderr_line",
                    "panic_location", "signature"):
            self.assertIn(key, info)
        self.assertEqual(info["returncode"], 101)
        self.assertIn("src/jpeg.rs", info["panic_location"])

    def test_deterministic_loop(self):
        real = jpeg.mutate
        jpeg.mutate = self.stub_mutate(
            lambda i: CRASH_MARK if i % 2 == 0 else b"clean")
        try:
            first = jpeg.main(self.args(n=6, seed=3))
        finally:
            jpeg.mutate = real
        names_before = stored_files(self.out)
        shutil.rmtree(self.out, ignore_errors=True)
        jpeg.mutate = self.stub_mutate(
            lambda i: CRASH_MARK if i % 2 == 0 else b"clean")
        try:
            second = jpeg.main(self.args(n=6, seed=3))
        finally:
            jpeg.mutate = real
        self.assertEqual(first, second)
        self.assertEqual(stored_files(self.out), names_before)

    def test_hang_class_and_artifact(self):
        real = jpeg.mutate
        jpeg.mutate = self.stub_mutate(lambda _i: HANG_MARK)
        try:
            counts = jpeg.main(self.args(n=2, timeout=0.3))
        finally:
            jpeg.mutate = real
        self.assertEqual(counts["hang"], 2)
        hang_dir = os.path.join(self.out, "hang")
        jpgs = [f for f in os.listdir(hang_dir) if f.endswith(".jpg")]
        # Shared "timeout" signature: one stored input.
        self.assertEqual(len(jpgs), 1)


class SleepEngineTest(unittest.TestCase):
    def test_all_hang(self):
        tmp = tempfile.mkdtemp(prefix="jpeg-sleep-")
        try:
            sleepy = make_engine(tmp, "sleep.sh", SLEEP_BODY)
            out = os.path.join(tmp, "out")
            counts = jpeg.main(["--candidate", sleepy, "--out", out,
                                "--iterations", "2", "--seed", "1",
                                "--timeout", "0.2"])
            self.assertEqual(counts["hang"], 2)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
