#!/usr/bin/env python3
"""Tests for tools/parity/harness.py pure helpers (no TeX, no rasterizer).

    python3 -m unittest discover -s tools/parity -p 'test_*.py' -v

harness.py had no direct tests: test_parity.py exercises the scoring path
(parity, tiers, capture, engines, nightly) but never imports harness.
These cover the helpers that need no binaries: digests, fixture discovery,
the pdflatex-missing branch, rasterizer selection, the PGM reader,
diagnostic keys, and the page-count fallback scan. generate_reference and
main need a real pdflatex and are not covered here.
"""

import contextlib
import hashlib
import io
import os
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import harness  # noqa: E402


class FilesTests(unittest.TestCase):

    def test_run_reports_a_timeout(self):
        code, _, _, _, timed_out = harness.run(
            [sys.executable, "-c", "import time; time.sleep(10)"], timeout=0.2)
        self.assertEqual((code, timed_out), (None, True))


class FixturesTests(unittest.TestCase):

    def test_entry_prefers_main_and_lists_every_tex(self):
        with tempfile.TemporaryDirectory() as root:
            os.makedirs(os.path.join(root, "a", "sub"))
            for rel in ("a/main.tex", "a/other.tex", "a/sub/nested.tex"):
                with open(os.path.join(root, rel), "w") as f:
                    f.write("x")
            os.makedirs(os.path.join(root, "b"))
            with open(os.path.join(root, "b", "only.tex"), "w") as f:
                f.write("x")
            with open(os.path.join(root, "top.tex"), "w") as f:  # a file, not a fixture
                f.write("x")
            out = harness.discover_fixtures(root)
        self.assertEqual([f["id"] for f in out], ["a", "b"])
        self.assertEqual(out[0]["entry"], "main.tex")
        self.assertEqual(out[0]["documents"], ["main.tex", "other.tex", os.path.join("sub", "nested.tex")])
        self.assertEqual(out[1]["entry"], "only.tex")

    def test_several_tex_files_without_main_and_texless_dirs_are_skipped(self):
        with tempfile.TemporaryDirectory() as root:
            os.makedirs(os.path.join(root, "c"))
            for name in ("one.tex", "two.tex"):
                with open(os.path.join(root, "c", name), "w") as f:
                    f.write("x")
            os.makedirs(os.path.join(root, "d"))  # no .tex at all
            err = io.StringIO()
            with contextlib.redirect_stderr(err):
                out = harness.discover_fixtures(root)
        self.assertEqual(out, [])
        self.assertIn("skip c", err.getvalue())


class RasterizerTests(unittest.TestCase):

    def test_prefer_selects_by_basename(self):
        with mock.patch("shutil.which", return_value=None), \
                mock.patch("os.path.isfile", side_effect=lambda p: p.startswith("/tmp/")):
            self.assertEqual(harness.find_rasterizer("/tmp/pdftoppm-custom"),
                             ("pdftoppm", "/tmp/pdftoppm-custom"))
            self.assertEqual(harness.find_rasterizer("/tmp/gs"), ("gs", "/tmp/gs"))

    def test_sips_is_last_resort_and_cannot_rasterize(self):
        with mock.patch("shutil.which", side_effect=lambda c: "/usr/bin/sips" if c == "sips" else None):
            self.assertEqual(harness.find_rasterizer(), ("sips", "/usr/bin/sips"))
        with tempfile.TemporaryDirectory() as d:
            stale = os.path.join(d, "shot-1.pgm")
            with open(stale, "wb") as f:
                f.write(b"x")
            files, argv, note = harness.rasterize(
                ("sips", "/usr/bin/sips"), os.path.join(d, "in.pdf"), os.path.join(d, "shot"))
            self.assertEqual((files, argv), ([], []))
            self.assertIn("sips cannot", note)
            self.assertFalse(os.path.exists(stale))  # stale pages are swept first


class PgmTests(unittest.TestCase):

    def test_a_comment_and_whitespace_are_skipped(self):
        with tempfile.NamedTemporaryFile(suffix=".pgm", delete=False) as f:
            f.write(b"P5\n# a comment\n2 1\n255\nAB")
        try:
            self.assertEqual(harness.read_pgm(f.name), (2, 1, b"AB"))
        finally:
            os.unlink(f.name)

    def test_a_wrong_magic_maxval_or_truncation_raises(self):
        cases = (b"P6\n2 1\n255\nAB", b"P5\n2 1\n15\nAB", b"P5\n2 1\n255\nA")
        for data in cases:
            with tempfile.NamedTemporaryFile(suffix=".pgm", delete=False) as f:
                f.write(data)
            try:
                with self.assertRaises(ValueError, msg=data[:12]):
                    harness.read_pgm(f.name)
            finally:
                os.unlink(f.name)


class DiagTests(unittest.TestCase):

    def test_each_construct_kind_is_named(self):
        msg = ("Undefined control sequence \\foo* and \\bar, environment 'itemize', "
               "packages foo, bar are recognised, no math glyph for 'x'")
        self.assertEqual(harness.named_constructs(msg),
                         {("cs", "\\foo*"), ("cs", "\\bar"), ("env", "itemize"),
                          ("package", "foo"), ("package", "bar"), ("char", "x")})


class PageCountTests(unittest.TestCase):

    def test_the_byte_scan_counts_pages_but_not_page_trees(self):
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            f.write(b"junk /Type /Page x /Type /Pages y /Type/Page\n")
        try:
            self.assertEqual(harness.pdf_page_count(f.name), 2)
        finally:
            os.unlink(f.name)

    def test_a_file_with_no_pages_counts_as_none(self):
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            f.write(b"no pages here")
        try:
            self.assertIsNone(harness.pdf_page_count(f.name))
        finally:
            os.unlink(f.name)


if __name__ == "__main__":
    unittest.main()
