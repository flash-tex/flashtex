#!/usr/bin/env python3
"""Unit tests for tools/fuzz/parsers/ttf.py. Stdlib unittest only.

Run as `python3 tools/fuzz/parsers/test_ttf.py` from the repo root, or
`python3 -m unittest discover -s tools/fuzz/parsers`. The candidate is a
fake shell script that panics when the fuzz font contains a marker byte
string; the real candidate is never used.
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

import ttf

MARKER = b"TTF-CRASH-MARKER-42"
MARKER_BODY = ("for last do :; done\n"
               "d=$(dirname \"$last\")\n"
               "if cat \"$d\"/fuzz.ttf \"$d\"/fuzz.otf 2>/dev/null "
               "| grep -q 'TTF-CRASH-MARKER-42'; then\n"
               "  echo \"thread 'main' panicked at 'marker hit', "
               "src/ttf.rs:99:3\"\n"
               "  exit 101\n"
               "fi\n"
               "exit 0\n")
REJECT_BODY = ("echo '!pdfTeX error: pdftex (file fuzz.ttf): "
               "unexpected EOF'\nexit 1\n")
# Records the job and the files beside it, then succeeds.
SPY_BODY = ("for last do :; done\n"
            "d=$(dirname \"$last\")\n"
            "cat \"$d/job.tex\" > \"$SPY_OUT\"\n"
            "ls \"$d\" >> \"$SPY_OUT\"\n"
            "exit 0\n")


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, 0o755)
    return path


def marker_seed():
    # The marker sits after the last table, where it survives most
    # structure-aware mutations.
    return ttf.synthetic_ttf() + MARKER * 8


class SyntheticFontTest(unittest.TestCase):
    def setUp(self):
        self.font = ttf.synthetic_ttf()

    def test_directory(self):
        info = ttf.parse_sfnt(self.font)
        self.assertEqual(info["num_tables"], 10)
        self.assertEqual(
            sorted(info["tables"]),
            [b"OS/2", b"cmap", b"glyf", b"head", b"hhea", b"hmtx",
             b"loca", b"maxp", b"name", b"post"])
        for _rec, tag, off, length in info["records"]:
            self.assertLessEqual(off + length, len(self.font), tag)
            self.assertEqual(off % 4, 0, tag)

    def test_checksums(self):
        info = ttf.parse_sfnt(self.font)
        for rec, tag, off, length in info["records"]:
            want = struct.unpack(">I", self.font[rec + 4:rec + 8])[0]
            data = bytearray(self.font[off:off + length])
            if tag == b"head":
                data[8:12] = b"\x00" * 4
            self.assertEqual(ttf._checksum(bytes(data)), want, tag)
        self.assertEqual(ttf._checksum(self.font), 0xB1B0AFBA)

    def test_glyphs_and_loca(self):
        n = ttf.num_glyphs(self.font)
        self.assertEqual(n, 26)
        entries = ttf.loca_entries(self.font)
        self.assertEqual(len(entries), n + 1)
        offsets = [v for _p, v, _f in entries]
        self.assertEqual(offsets, sorted(offsets))
        self.assertEqual(offsets[-1], ttf._table(self.font, b"glyf")[1])
        # The last glyph, the "fi" ligature, is a composite.
        glyf = ttf._table(self.font, b"glyf")[0]
        start = glyf + offsets[-2]
        self.assertEqual(struct.unpack(">h", self.font[start:start + 2])[0],
                         -1)

    def test_post_names_match_8r(self):
        t = ttf._table(self.font, b"post")
        base = t[0]
        self.assertEqual(struct.unpack(">I", self.font[base:base + 4])[0],
                         0x00020000)
        n = struct.unpack(">H", self.font[base + 32:base + 34])[0]
        self.assertEqual(n, ttf.num_glyphs(self.font))
        p = base + 34 + 2 * n
        names = []
        while p < base + t[1]:
            ln = self.font[p]
            names.append(self.font[p + 1:p + 1 + ln].decode("ascii"))
            p += 1 + ln
        for name in ("H", "e", "l", "o", "comma", "exclam", "zero", "nine",
                     "f", "i", "fi"):
            self.assertIn(name, names)

    def test_deterministic(self):
        self.assertEqual(self.font, ttf.synthetic_ttf())

    def test_parse_short(self):
        self.assertIsNone(ttf.parse_sfnt(b"short"))
        # A directory that claims more tables than the file holds.
        blob = struct.pack(">IHHHH", 0x00010000, 50, 0, 0, 0)
        self.assertEqual(ttf.parse_sfnt(blob)["records"], [])


class TtfFuzzTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="ttf-test-")
        self.marker = make_engine(self.tmp, "marker.sh", MARKER_BODY)
        self.ok = make_engine(self.tmp, "ok.sh", "exit 0\n")
        self.out = os.path.join(self.tmp, "out")
        self.seeds = [("m.ttf", "ttf", marker_seed())]

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_mutate_deterministic(self):
        font = ttf.synthetic_ttf()
        self.assertEqual(ttf.mutate_with_info(font, random.Random(7)),
                         ttf.mutate_with_info(font, random.Random(7)))

    def test_every_op_appears(self):
        rng = random.Random(7)
        font = ttf.synthetic_ttf()
        descs = {ttf.mutate_with_info(font, rng)[1].split("@")[0]
                     .split(":")[0].split("=")[0]
                 for _ in range(600)}
        self.assertTrue({"flip", "trunc", "del", "dup", "numtables",
                         "record", "tag", "head", "maxp", "hhea", "loca",
                         "glyph", "composite", "post", "cmap",
                         "name"} <= descs, descs)

    def test_structure_mutations_change_bytes(self):
        font = ttf.synthetic_ttf()
        for fn in ttf.MUTATIONS:
            if fn is ttf._cff:
                continue
            for s in range(20):
                got = fn(font, random.Random(s))
                self.assertIsNotNone(got, fn.__name__)
                self.assertIsNotNone(got[0], fn.__name__)
                self.assertNotEqual(got[0], font,
                                    "%s seed %d: %s" % (fn.__name__, s,
                                                        got[1]))

    def test_cff_needs_cff_table(self):
        self.assertIsNone(ttf._cff(ttf.synthetic_ttf(), random.Random(0)))
        # A fake OpenType: one CFF table of 64 bytes.
        body = bytes(range(64))
        blob = (struct.pack(">IHHHH", 0x4F54544F, 1, 16, 0, 0)
                + struct.pack(">4sIII", b"CFF ", 0, 28, len(body)) + body)
        got, desc = ttf._cff(blob, random.Random(1))
        self.assertEqual(desc, "cff-head")
        self.assertEqual(got[:28], blob[:28])
        self.assertNotEqual(got, blob)

    def test_maxp_targets_numglyphs(self):
        font = ttf.synthetic_ttf()
        got, desc = ttf._maxp(font, random.Random(3))
        self.assertTrue(desc.startswith("maxp:numglyphs="), desc)
        self.assertEqual(ttf.num_glyphs(got), int(desc.split("=")[1]))
        self.assertNotEqual(ttf.num_glyphs(got), ttf.num_glyphs(font))

    def test_composite_kinds(self):
        font = ttf.synthetic_ttf()
        kinds = set()
        for s in range(40):
            got, desc = ttf._composite(font, random.Random(s))
            kinds.add(desc.split(":")[1].split("->")[0])
            self.assertNotEqual(got, font)
        self.assertEqual(kinds, {"self", "past", "unterminated", "cycle"})

    def test_job_modes(self):
        self.assertEqual(ttf.MODES["ttf"], ("ttf-subset", "ttf-whole"))
        self.assertEqual(ttf.MODES["otf"], ("otf-whole",))
        self.assertIn("<8r.enc <fuzz.ttf", ttf.job_tex("ttf-subset"))
        self.assertIn("<<fuzz.ttf", ttf.job_tex("ttf-whole"))
        self.assertIn("<<fuzz.otf", ttf.job_tex("otf-whole"))
        for mode in ttf.MAPLINES:
            self.assertIn("\\font\\x=fuzz", ttf.job_tex(mode))
        self.assertEqual(ttf.font_file("otf-whole"), "fuzz.otf")
        self.assertEqual(ttf.font_file("ttf-subset"), "fuzz.ttf")

    def test_run_one_writes_job_and_font(self):
        spy_out = os.path.join(self.tmp, "spy.txt")
        spy = make_engine(self.tmp, "spy.sh", SPY_BODY)
        os.environ["SPY_OUT"] = spy_out
        try:
            cls, rc, _ = ttf.run_one(b"\x00\x01\x00\x00", "otf-whole", spy,
                                     10)
        finally:
            del os.environ["SPY_OUT"]
        self.assertEqual((cls, rc), ("ok", 0))
        with open(spy_out) as fh:
            seen = fh.read()
        self.assertIn("<<fuzz.otf", seen)
        self.assertIn("fuzz.otf", seen.split())

    def test_run_fuzz_deterministic(self):
        a = ttf.run_fuzz(self.marker, self.seeds,
                         os.path.join(self.tmp, "a"), 20, 11, 10)
        b = ttf.run_fuzz(self.marker, self.seeds,
                         os.path.join(self.tmp, "b"), 20, 11, 10)
        self.assertEqual(a, b)
        self.assertEqual(sum(a.values()), 20)
        self.assertGreater(a["crash"], 0)

    def test_class_counts(self):
        counts = ttf.run_fuzz(self.ok, self.seeds, self.out, 5, 11, 10)
        self.assertEqual(counts, {"ok": 5, "font-rejected": 0,
                                  "graceful-error": 0, "crash": 0,
                                  "hang": 0, "output-flood": 0})

    def test_crash_artifacts_and_dedupe(self):
        ttf.run_fuzz(self.marker, self.seeds, self.out, 20, 11, 10)
        cls_dir = os.path.join(self.out, "crash")
        fonts = sorted(f for f in os.listdir(cls_dir)
                       if f.endswith((".ttf", ".otf")))
        # One panic location: exactly one stored case.
        self.assertEqual(len(fonts), 1)
        self.assertEqual(len(fonts[0]), 16 + len(".ttf"))
        with open(os.path.join(cls_dir, fonts[0]), "rb") as fh:
            self.assertIn(MARKER, fh.read())
        with open(os.path.join(cls_dir, fonts[0][:-4] + ".json")) as fh:
            info = json.load(fh)
        for key in ("seed", "origin", "mode", "maplines", "mutation",
                    "returncode", "last_line", "panic_location",
                    "signature"):
            self.assertIn(key, info)
        self.assertEqual(info["returncode"], 101)
        self.assertIn(info["mode"], ("ttf-subset", "ttf-whole"))
        self.assertIn("src/ttf.rs", info["panic_location"])
        # Repeat run: signature already on disk, nothing new stored.
        ttf.run_fuzz(self.marker, self.seeds, self.out, 20, 11, 10)
        self.assertEqual(sorted(f for f in os.listdir(cls_dir)
                                if f.endswith((".ttf", ".otf"))), fonts)

    def test_classify(self):
        self.assertEqual(ttf.classify(101, "panicked at x"), "crash")
        self.assertEqual(ttf.classify(-11, ""), "crash")
        self.assertEqual(
            ttf.classify(1, "!pdfTeX error: pdftex (file fuzz.ttf): "
                            "unexpected EOF"), "font-rejected")
        self.assertEqual(
            ttf.classify(1, "!pdfTeX error: pdftex (file fuzz.otf): "
                            "OTF fonts must be included entirely"),
            "font-rejected")
        self.assertEqual(ttf.classify(0, "(file fuzz.ttf)"), "ok")
        self.assertEqual(ttf.classify(1, "some other error"),
                         "graceful-error")

    def test_font_rejected_run(self):
        reject = make_engine(self.tmp, "reject.sh", REJECT_BODY)
        counts = ttf.run_fuzz(reject, self.seeds,
                              os.path.join(self.tmp, "out-reject"),
                              5, 11, 10)
        self.assertEqual(counts["font-rejected"], 5)

    def test_hang_artifact(self):
        sleepy = make_engine(self.tmp, "sleep.sh", "sleep 30\nexit 0\n")
        counts = ttf.run_fuzz(sleepy, self.seeds,
                              os.path.join(self.tmp, "out-hang"),
                              1, 3, 0.2)
        self.assertEqual(counts["hang"], 1)
        hang_dir = os.path.join(self.tmp, "out-hang", "hang")
        fonts = [f for f in os.listdir(hang_dir) if f.endswith(".ttf")]
        self.assertEqual(len(fonts), 1)

    def test_load_seeds_always_has_synthetic(self):
        seeds = ttf.load_seeds()
        self.assertEqual(seeds[0][:2], ("synthetic.ttf", "ttf"))
        for name, kind, blob in seeds:
            self.assertIn(kind, ttf.MODES)
            self.assertTrue(blob, name)


if __name__ == "__main__":
    unittest.main()
