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


class ProgressiveSeedsTest(unittest.TestCase):
    def setUp(self):
        self.seed = jpeg.generate_minimal_jpeg()

    def test_is_progressive(self):
        self.assertFalse(jpeg.is_progressive(self.seed))
        self.assertTrue(
            jpeg.is_progressive(jpeg.make_progressive_variant(self.seed)))
        for blob in (b"", b"\xff", b"\xff\xd8\xff", b"\xff\xd8\xff\xc0"):
            self.assertFalse(jpeg.is_progressive(blob))
            self.assertIsInstance(jpeg.make_progressive_variant(blob), bytes)
            self.assertIsInstance(jpeg.make_12bit_variant(blob), bytes)

    def test_progressive_variant_rewrites_sof_and_sos(self):
        var = jpeg.make_progressive_variant(self.seed, ss=1, se=63,
                                            ah=1, al=13)
        self.assertEqual(len(var), len(self.seed))
        marks = [m for m, _, _, _ in jpeg.parse_segments(var)]
        self.assertIn(0xC2, marks)
        self.assertNotIn(0xC0, marks)
        found = jpeg._sos_spec_tail(var)
        self.assertIsNotNone(found)
        tail = found[3]
        self.assertEqual((var[tail], var[tail + 1], var[tail + 2]),
                         (1, 63, (1 << 4) | 13))

    def test_12bit_variant_sets_precision(self):
        var = jpeg.make_12bit_variant(self.seed)
        self.assertNotEqual(var, self.seed)
        sofs = [(lenoff) for m, _o, lenoff, _l
                in jpeg.parse_segments(var) if m in jpeg.SOF]
        self.assertTrue(sofs)
        self.assertEqual(var[sofs[0] + 2], 12)

    def test_find_progressive_seed_in_tmpdir(self):
        tmp = tempfile.mkdtemp(prefix="jpeg-prog-")
        try:
            with open(os.path.join(tmp, "base.jpg"), "wb") as fh:
                fh.write(self.seed)
            prog = jpeg.make_progressive_variant(self.seed)
            with open(os.path.join(tmp, "prog.jpg"), "wb") as fh:
                fh.write(prog)
            with open(os.path.join(tmp, "note.txt"), "w") as fh:
                fh.write("not a jpeg")
            found = jpeg.find_progressive_seed(dirs=[tmp])
            self.assertIsNotNone(found)
            self.assertEqual(found[0], "prog.jpg")
            self.assertTrue(jpeg.is_progressive(found[1]))
            base_only = os.path.join(tmp, "base-only")
            os.mkdir(base_only)
            with open(os.path.join(base_only, "base.jpg"), "wb") as fh:
                fh.write(self.seed)
            for dead in (base_only, os.path.join(tmp, "missing")):
                self.assertIsNone(jpeg.find_progressive_seed(dirs=[dead]))
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def test_load_seeds_has_progressive_and_12bit(self):
        seeds = jpeg.load_seeds()
        names = [n for n, _d in seeds]
        self.assertTrue(any("progressive" in n for n in names))
        self.assertTrue(any("12bit" in n for n in names))
        for name, data in seeds:
            if "progressive" in name:
                self.assertTrue(jpeg.is_progressive(data), name)

    def test_new_mutation_families_appear_and_shape(self):
        fams = ("sos-spectral", "sos-extra-scan", "dri-insert",
                "rst-insert", "app-inconsistent", "sof-prec=12")
        seen = {}
        for s in range(2000):
            data, desc = jpeg.mutate(self.seed, random.Random(s))
            for fam in fams:
                if desc.startswith(fam):
                    seen.setdefault(fam, (data, desc))
        self.assertFalse(set(fams) - set(seen),
                         "missing draws: %s" % sorted(set(fams) - set(seen)))
        data, _desc = seen["sos-extra-scan"]
        self.assertGreater(len(data), len(self.seed))
        self.assertEqual(data.count(b"\xff\xda"), 2)
        data, _desc = seen["rst-insert"]
        self.assertGreater(len(data), len(self.seed))
        self.assertTrue(any(bytes([0xFF, m]) in data
                            for m in range(0xD0, 0xD8)))
        data, desc = seen["app-inconsistent"]
        parts = desc.split("decl=")[1].split(" actual=")
        self.assertNotEqual(int(parts[0]), int(parts[1]))
        self.assertTrue(b"\xff\xe1" in data or b"\xff\xe2" in data)

    def test_dri_interval_edit_on_seeded_dri(self):
        with_dri = self.seed[:2] + jpeg.make_dri_segment(8) + self.seed[2:]
        for s in range(1000):
            data, desc = jpeg.mutate(with_dri, random.Random(s))
            if desc.startswith("dri-interval"):
                dris = [x for x in jpeg.parse_segments(data) if x[0] == 0xDD]
                self.assertTrue(dris)
                return
        self.fail("no dri-interval draw in 1000 seeds")

    def test_app_segment_declared_differs_from_actual(self):
        seg = jpeg.make_app_segment(0xE1, 0xFFFF, b"Exif\x00\x00AB")
        self.assertTrue(seg.startswith(b"\xff\xe1\xff\xff"))
        self.assertNotEqual((seg[2] << 8) + seg[3], len(seg) - 2)


class ClassifyTest(unittest.TestCase):
    def test_crash(self):
        self.assertEqual(jpeg.classify(101, "panicked at x", False), "crash")
        self.assertEqual(jpeg.classify(-11, "", False), "crash")

    def test_panic_words_are_not_a_crash(self):
        # Transcript text never decides a crash: without a crash return
        # code the words "panicked at" are ok / graceful-error.
        self.assertEqual(jpeg.classify(0, "panicked at y", False), "ok")
        self.assertEqual(
            jpeg.classify(1, "panicked at y", False), "graceful-error")

    def test_hang_ok_graceful(self):
        self.assertEqual(jpeg.classify(None, "", True), "hang")
        self.assertEqual(jpeg.classify(0, "ok", False), "ok")
        self.assertEqual(jpeg.classify(1, "! error", False), "graceful-error")

    def test_panic_location(self):
        loc = jpeg.panic_location("thread 'main' panicked at 'b', "
                                  "src/jpeg.rs:12:5")
        self.assertEqual(loc, "src/jpeg.rs:12")
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
                                 "graceful-error": 0,
                                 "output-flood": 0})
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
