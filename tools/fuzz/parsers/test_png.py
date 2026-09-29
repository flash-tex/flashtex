#!/usr/bin/env python3
"""Unit tests for tools/fuzz/parsers/png.py. Stdlib unittest only.

Run as `python3 -m unittest discover -s tools/fuzz/parsers` from the repo
root (tools/fuzz/parsers has no __init__.py, so plain `discover -s tools/fuzz`
does not descend into it). Engines are small shell scripts written to a temp
dir; the real candidate is never used.
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
    "pngfuzz", os.path.join(HERE, "png.py"))
pngfuzz = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pngfuzz)

CRASH_BODY = ("echo \"thread 'main' panicked at 'idat boom', "
              "src/png/idat.rs:42:7\" >&2\nexit 101\n")
MARKER_BODY = ("for last do :; done\n"
               "f=\"$(dirname \"$last\")/fuzz.png\"\n"
               "if grep -q 'PNGMARKER' \"$f\" 2>/dev/null; then\n"
               "  echo \"thread 'main' panicked at 'marker hit', "
               "src/png.rs:7:3\" >&2\n"
               "  exit 101\n"
               "fi\nexit 1\n")
ECHO_BODY = "exit 0\n"
FAIL_BODY = "exit 1\n"
SLEEP_BODY = "sleep 30\nexit 0\n"


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, 0o755)
    return path


def stored_pngs(out):
    found = []
    for root, _dirs, files in os.walk(out):
        found.extend(os.path.join(root, f) for f in files
                     if f.endswith(".png"))
    return sorted(found)


class MutateTest(unittest.TestCase):
    def test_deterministic(self):
        seed = pngfuzz.make_gray()
        rng = random.Random(42)
        first = [pngfuzz.mutate_png(seed, rng) for _ in range(50)]
        rng = random.Random(42)
        second = [pngfuzz.mutate_png(seed, rng) for _ in range(50)]
        self.assertEqual(first, second)

    def test_ops_cover_flip_trunc_and_chunk_edits(self):
        seed = pngfuzz.make_gray()
        rng = random.Random(7)
        descs = [pngfuzz.mutate_png(seed, rng)[1] for _ in range(200)]
        kinds = {d.split("=")[0].split("@")[0].rstrip("0123456789")
                 for d in descs}
        for want in ("flip", "trunc", "del-", "dup-", "len-", "crc-",
                     "add-", "ihdr-"):
            self.assertTrue(any(k.startswith(want) for k in kinds), want)

    def test_synthetic_seeds_are_valid_png(self):
        gray = pngfuzz.make_gray(1, 1, 0x80, 1)
        pal = pngfuzz.make_palette()
        for data in (gray, pal):
            self.assertTrue(data.startswith(pngfuzz.SIG))
            self.assertIsNotNone(pngfuzz.parse(data))
        self.assertEqual(pngfuzz.parse(gray)[0][1][9:10], b"\x00")
        self.assertEqual(pngfuzz.parse(pal)[0][1][9:10], b"\x03")

    def test_load_seeds_always_has_synthetics(self):
        seeds = pngfuzz.load_seeds()
        names = [n for n, _ in seeds]
        self.assertIn("synth-gray-interlaced", names)
        self.assertIn("synth-palette", names)
        for _n, data in seeds:
            self.assertTrue(data.startswith(pngfuzz.SIG))


class ClassifyTest(unittest.TestCase):
    def test_classes(self):
        panic = b"thread 'main' panicked at 'x', src/a.rs:1:2"
        self.assertEqual(
            pngfuzz.classify(101, panic), "crash")
        self.assertEqual(pngfuzz.classify(-11, b""), "crash")
        self.assertEqual(pngfuzz.classify(1, b"oops", True), "hang")
        self.assertEqual(pngfuzz.classify(0, b"ok"), "ok")
        self.assertEqual(
            pngfuzz.classify(1, b"libpng error"), "graceful-error")

    def test_panic_location(self):
        self.assertEqual(
            pngfuzz.panic_location(
                b"thread 'main' panicked at 'boom', src/png.rs:7:3"),
            "'boom', src/png.rs")
        self.assertIsNone(pngfuzz.panic_location(b"plain error"))


class FuzzRunTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="pngfuzz-test-")
        self.out = os.path.join(self.tmp, "out")
        self.echo = make_engine(self.tmp, "echo.sh", ECHO_BODY)
        self.seeds = [("synth", pngfuzz.make_gray()),
                      ("pal", pngfuzz.make_palette())]

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def dorun(self, cand, n=10, seed=42, timeout=10, seeds=None):
        return pngfuzz.run_fuzz(cand, self.out, n, seed, timeout,
                                seeds if seeds is not None else self.seeds)

    def test_all_ok_stores_nothing(self):
        counts = self.dorun(self.echo, n=5)
        self.assertEqual(counts, {"crash": 0, "hang": 0, "ok": 5,
                                 "graceful-error": 0})
        self.assertFalse(os.path.exists(self.out))

    def test_graceful_error_counts_but_stores_nothing(self):
        fail = make_engine(self.tmp, "fail.sh", FAIL_BODY)
        counts = self.dorun(fail, n=3)
        self.assertEqual(counts["graceful-error"], 3)
        self.assertFalse(os.path.exists(self.out))

    def test_crash_artifacts_and_dedupe(self):
        crash = make_engine(self.tmp, "crash.sh", CRASH_BODY)
        counts = self.dorun(crash)
        self.assertEqual(counts["crash"], 10)
        # One panic location: deduped to a single stored case.
        found = stored_pngs(self.out)
        self.assertEqual(len(found), 1)
        with open(found[0][:-4] + ".json") as fh:
            info = json.load(fh)
        for key in ("seed", "mutation", "returncode",
                    "last_output_line", "panic_location", "signature"):
            self.assertIn(key, info)
        self.assertEqual(info["returncode"], 101)
        self.assertIn("src/png/idat.rs", info["panic_location"])
        self.assertIn("panicked at", info["last_output_line"])

    def test_hang_artifact(self):
        sleepy = make_engine(self.tmp, "sleep.sh", SLEEP_BODY)
        counts = self.dorun(sleepy, n=2, timeout=0.2)
        self.assertEqual(counts["hang"], 2)
        found = stored_pngs(self.out)
        self.assertEqual(len(found), 1)
        with open(found[0][:-4] + ".json") as fh:
            info = json.load(fh)
        self.assertIsNone(info["returncode"])
        self.assertIsNone(info["panic_location"])
        self.assertEqual(info["signature"], "hang")

    def test_marker_fake_panics_on_marker_input(self):
        marker = make_engine(self.tmp, "marker.sh", MARKER_BODY)
        png = pngfuzz.make_gray() + b"PNGMARKER"
        cls, rc, _out = pngfuzz.run_once(png, marker, 10)
        self.assertEqual(cls, "crash")
        self.assertEqual(rc, 101)
        cls, _rc, _out = pngfuzz.run_once(pngfuzz.make_gray(), marker, 10)
        self.assertEqual(cls, "graceful-error")

    @staticmethod
    def _snap(out):
        snap = []
        for path in stored_pngs(out):
            with open(path, "rb") as fh:
                snap.append((os.path.basename(path), fh.read()))
        return snap

    def test_full_run_deterministic(self):
        crash = make_engine(self.tmp, "crash.sh", CRASH_BODY)
        counts1 = pngfuzz.run_fuzz(crash, self.out, 10, 99, 10,
                                   self.seeds)
        first = self._snap(self.out)
        shutil.rmtree(self.out)
        out2 = os.path.join(self.tmp, "out2")
        counts2 = pngfuzz.run_fuzz(crash, out2, 10, 99, 10, self.seeds)
        self.assertEqual(counts1, counts2)
        self.assertEqual(first, self._snap(out2))


if __name__ == "__main__":
    unittest.main()
