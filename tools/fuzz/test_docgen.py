#!/usr/bin/env python3
"""Unit tests for tools/fuzz/docgen.py. Stdlib unittest only.

Run as `python3 -m unittest discover -s tools/fuzz` from the repo root.
Engines are small shell scripts written to a temp dir (in the style of
tools/lockstep/test_capture.py); the real candidate is never used.
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

import docgen


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, 0o755)
    return path


def log_engine(fonts=5):
    # Writes $job.log, identical except the font count; exit 0.
    return ("for last do :; done\n"
            "job=${last##*/}; job=${job%%.tex}\n"
            "cat > \"$job.log\" <<'EOF'\n"
            "This is pdfTeX\n"
            "Here is how much of TeX's memory you used:\n"
            " 10 words of font info for %d fonts, out of 8000000 for 9000\n"
            "Output written on doc.pdf (1 page, 100 bytes).\n"
            "EOF\n"
            "exit 0\n" % fonts)


SAME_BODY = log_engine(5)
FEW_BODY = log_engine(3)
CRASH_BODY = ("for last do :; done\n"
              "job=${last##*/}; job=${job%%.tex}\n"
              "echo \"thread 'main' panicked at 'boom', src/x.rs:1:2\" "
              "> \"$job.log\"\n"
              "cat \"$job.log\"\n"
              "exit 101\n")


class GenerateDocTest(unittest.TestCase):
    def test_deterministic(self):
        self.assertEqual(docgen.generate_doc(random.Random(7)),
                         docgen.generate_doc(random.Random(7)))

    def test_complete_article(self):
        for s in range(20):
            text, _desc = docgen.generate_doc(random.Random(s))
            self.assertIn("\\documentclass{article}", text)
            self.assertIn("\\begin{document}", text)
            self.assertTrue(text.rstrip().endswith("\\end{document}"))

    def test_desc_matches_text(self):
        text, desc = docgen.generate_doc(random.Random(3))
        if "microtype=none" in desc:
            self.assertNotIn("microtype", text)
        else:
            self.assertIn("microtype", text)

    def test_missing_packages_never_selected(self):
        docgen._kpse_cache.clear()
        real = docgen.package_present
        try:
            docgen.package_present = lambda sty: sty == "microtype"
            for s in range(20):
                text, _desc = docgen.generate_doc(random.Random(s))
                self.assertNotIn("lmodern", text)
                self.assertNotIn("hyperref", text)
                self.assertNotIn("geometry", text)
        finally:
            docgen.package_present = real
            docgen._kpse_cache.clear()


class FontCountTest(unittest.TestCase):
    def test_extract(self):
        self.assertEqual(
            docgen.font_count(" 10 words of font info for 5 fonts,"
                              " out of 8000000 for 9000"), 5)
        self.assertEqual(
            docgen.font_count(" 3 words of font info for 1 font,"
                              " out of 10 for 20"), 1)
        self.assertIsNone(docgen.font_count("no accounting here"))

    HEAD = "Here is how much of TeX's memory you used:\n"

    def test_equal_counts_stay_equal(self):
        log = (self.HEAD
               + " 10 words of font info for 5 fonts, out of 8000000 for 9000")
        self.assertEqual(docgen.classify(0, log, 0, log, False), "equal")

    def test_different_counts_are_fontcount_diff(self):
        cand = (self.HEAD
                + " 10 words of font info for 3 fonts, out of 8000000"
                " for 9000")
        orc = (self.HEAD
               + " 10 words of font info for 5 fonts, out of 8000000"
               " for 9000")
        self.assertEqual(
            docgen.classify(0, cand, 0, orc, False), "fontcount-diff")

    def test_missing_line_stays_equal(self):
        self.assertEqual(docgen.classify(0, "ok", 0, "ok", False), "equal")

    def test_crash_not_masked(self):
        cand = "panicked at 'boom', src/x.rs:1:2"
        orc = " 10 words of font info for 5 fonts, out of 8000000 for 9000"
        self.assertEqual(
            docgen.classify(101, cand, 0, orc, False), "candidate-crash")

    def test_panic_words_never_decide(self):
        # Transcript text never decides a crash: identical logs carrying
        # the words "panicked at" with rc 0 are equal, not both-crash.
        log = ("This is pdfTeX\n\\message{panicked at}\npanicked at\n"
               " 10 words of font info for 5 fonts, out of 8000000 for 9000")
        self.assertEqual(docgen.classify(0, log, 0, log, False), "equal")
        # A real candidate exit 101 is candidate-crash even when the
        # oracle log carries the same words (never both-crash).
        self.assertEqual(
            docgen.classify(101, "panicked at x", 0, log, False),
            "candidate-crash")

    def test_signature(self):
        cand = " 10 words of font info for 3 fonts, out of 8000000 for 9000"
        orc = " 12 words of font info for 5 fonts, out of 8000000 for 9000"
        self.assertEqual(
            docgen.signature("fontcount-diff", 0, cand, 0, orc, "d"),
            "fontcount-diff:candidate-3-oracle-5")


class RunOneTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="docgen-test-")
        self.same = make_engine(self.tmp, "same.sh", SAME_BODY)
        self.few = make_engine(self.tmp, "few.sh", FEW_BODY)
        self.crash = make_engine(self.tmp, "crash.sh", CRASH_BODY)
        self.out = os.path.join(self.tmp, "out")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_identical_engines_equal(self):
        cls, _, _, _ = docgen.run_one("x", self.same, self.same, 10)
        self.assertEqual(cls, "equal")

    def test_font_count_diff(self):
        cls, _, _, diff, _, _ = docgen.run_one(
            "x", self.few, self.same, 10, return_logs=True)
        self.assertEqual(cls, "fontcount-diff")
        self.assertIn("candidate=3", diff)
        self.assertIn("oracle=5", diff)

    def test_candidate_crash(self):
        cls, cand_rc, _, _ = docgen.run_one(
            "x", self.crash, self.same, 10)
        self.assertEqual(cls, "candidate-crash")
        self.assertEqual(cand_rc, 101)

    def test_run_docs_stores_fontcount_diff(self):
        counts = docgen.main(["--candidate", self.few,
                              "--oracle", self.same,
                              "--out", self.out, "--iterations", "5",
                              "--seed", "1", "--timeout", "10"])
        self.assertEqual(sum(counts.values()), 5)
        self.assertEqual(counts["fontcount-diff"], 5)
        cls_dir = os.path.join(self.out, "fontcount-diff")
        texs = sorted(f for f in os.listdir(cls_dir) if f.endswith(".tex"))
        # One signature for every fontcount pair: exactly one stored case.
        self.assertEqual(len(texs), 1)
        with open(os.path.join(cls_dir, texs[0][:-4] + ".json")) as fh:
            info = json.load(fh)
        for key in ("options", "candidate_returncode", "oracle_returncode",
                    "first_diff", "signature"):
            self.assertIn(key, info)
        with open(os.path.join(self.out, "signatures.json")) as fh:
            sigs = json.load(fh)
        self.assertEqual(sum(sigs.values()), 5)

    def test_run_docs_equal_stores_nothing(self):
        counts = docgen.main(["--candidate", self.same,
                              "--oracle", self.same,
                              "--out", self.out, "--iterations", "3",
                              "--seed", "1", "--timeout", "10"])
        self.assertEqual(counts["equal"], 3)
        self.assertFalse(os.path.exists(self.out))


def desc_map(desc):
    return dict(kv.split("=", 1) for kv in desc.split(";"))


class HeaderShapesTest(unittest.TestCase):
    def _docs(self, seeds):
        return [docgen.generate_doc(random.Random(s)) for s in seeds]

    def test_header_shapes_cover_lone_space_boxes(self):
        seen = set()
        for text, desc in self._docs(range(300)):
            seen.add(desc_map(desc)["header"])
        for box in ("hbox-space", "hbox-0pt", "hbox-bs"):
            self.assertTrue(
                any(box in h for h in seen), "box never generated: " + box)
        for shape in ("headings-oddhead", "headings-oddfoot",
                      "headings-both", "mine"):
            self.assertTrue(
                any(h.startswith(shape) for h in seen),
                "shape never generated: " + shape)
        if docgen.package_present("fancyhdr"):
            self.assertTrue(
                any(h.startswith("fancy:") for h in seen),
                "fancy header never generated")

    def test_microtype_sets_cover_named_options(self):
        seen = set()
        for _text, desc in self._docs(range(300)):
            seen.add(desc_map(desc)["microtype"])
        self.assertIn("spacing=true", seen)
        for opt in ("expansion", "protrusion", "tracking",
                    "letterspace=120", "final"):
            self.assertTrue(
                any(opt in m for m in seen),
                "microtype option never generated: " + opt)

    def test_header_desc_matches_text(self):
        for text, desc in self._docs(range(50)):
            header = desc_map(desc)["header"]
            if header.startswith("none/"):
                self.assertNotIn("\\ps@", text)
                self.assertNotIn("fancyhdr", text)
            elif header.startswith("fancy:"):
                self.assertIn("fancyhdr", text)
            else:
                self.assertIn("\\ps@", text)
            if "hbox-space" in header:
                self.assertIn("\\hbox{ }", text)
            if "hbox-0pt" in header:
                self.assertIn("\\hbox to 0pt{ }", text)
            if "hbox-bs" in header:
                self.assertIn("\\hbox{\\ }", text)

    def test_body_blocks_match_desc(self):
        seen_null = seen_title = seen_switch = False
        seen_empty_float = False
        for text, desc in self._docs(range(100)):
            m = desc_map(desc)
            if m["nullpage"] == "yes":
                seen_null = True
                self.assertIn("\\null", text)
                self.assertIn("\\newpage", text)
            else:
                self.assertNotIn("\\null", text)
            if m["title"] == "none":
                self.assertNotIn("\\maketitle", text)
                self.assertNotIn("Probe Title", text)
            else:
                seen_title = True
            if m["pageswitch"] == "none":
                self.assertEqual(text.count("\\pagestyle"), 1)
            else:
                seen_switch = True
                self.assertGreaterEqual(text.count("\\pagestyle"), 2)
            if m.get("table") == "empty" or m.get("figure") == "emptybox":
                seen_empty_float = True
        self.assertTrue(seen_null, "null page never generated")
        self.assertTrue(seen_title, "title block never generated")
        self.assertTrue(seen_switch, "pagestyle switch never generated")
        self.assertTrue(seen_empty_float, "empty float never generated")

    def test_no_fancyhdr_no_fancy(self):
        real = docgen.package_present
        try:
            docgen.package_present = lambda sty: sty != "fancyhdr"
            for text, _desc in self._docs(range(20)):
                self.assertNotIn("fancyhdr", text)
        finally:
            docgen.package_present = real

    def test_trigger_shape_appears(self):
        # The known panic needs microtype spacing plus a lone-space
        # header box; that joint shape must occur within 300 seeds.
        found = False
        for _text, desc in self._docs(range(300)):
            m = desc_map(desc)
            if ("spacing" in m["microtype"]
                    and ("hbox-space" in m["header"]
                         or "hbox-bs" in m["header"])):
                found = True
        self.assertTrue(found, "no trigger-shaped doc in 300 seeds")


if __name__ == "__main__":
    unittest.main()
