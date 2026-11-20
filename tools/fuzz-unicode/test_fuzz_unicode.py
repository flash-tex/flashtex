#!/usr/bin/env python3
"""Tests for tools/fuzz-unicode. MIT.

    python3 -m unittest discover -s tools/fuzz-unicode -p 'test_*.py'

The pure-logic tests always run. With FUZZ_UNICODE_ENGINE naming a built
flashtex-xetex and TeX Live 2026's xetex on PATH, the fixed seeds must be
clean.
"""
import os
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen  # noqa: E402
import minimize  # noqa: E402

FIXED_SEEDS = [0, 1, 2, 3, 5]


class Generator(unittest.TestCase):
    def test_deterministic(self):
        for s in (0, 7, 123):
            self.assertEqual(gen.generate(s).render(), gen.generate(s).render())
            self.assertEqual(gen.generate(s, latex=True).render(), gen.generate(s, latex=True).render())

    def test_balanced_lines(self):
        # Every line is balanced on its own, so minimize.py's line pass
        # never splits a group, and every document ends in \end.
        for s in range(200):
            lines = gen.generate(s).render().rstrip("\n").split("\n")
            self.assertEqual(lines[-1], "\\end", s)
            for ln in lines:
                body = ln.replace("`\\{", "").replace("`\\}", "")
                self.assertTrue(minimize.balanced(body), (s, ln))

    def test_no_comment_char_in_words(self):
        for w in gen.LATIN_WORDS + gen.HEBREW_WORDS + gen.ARABIC_WORDS + gen.DIGITS:
            self.assertNotIn("%", w)
            self.assertNotIn("&", w)
            self.assertNotIn("#", w)


class DDMin(unittest.TestCase):
    def test_ddmin_finds_pair(self):
        items = list(range(20)) + ["end"]
        got = minimize.ddmin(items, lambda xs: 3 in xs and 11 in xs)
        self.assertEqual(got, [3, 11, "end"])


@unittest.skipUnless(os.environ.get("FUZZ_UNICODE_ENGINE") and shutil.which("xetex"),
                     "FUZZ_UNICODE_ENGINE and xetex needed")
class Oracle(unittest.TestCase):
    def test_fixed_seeds_clean(self):
        import fuzz
        eng = os.path.abspath(os.environ["FUZZ_UNICODE_ENGINE"])
        for s in FIXED_SEEDS:
            work = tempfile.mkdtemp(prefix="fzu-test-")
            try:
                res = fuzz.compare(gen.generate(s).render(), eng, shutil.which("xetex"), work, 120)
            finally:
                shutil.rmtree(work, ignore_errors=True)
            self.assertIn(res["cls"], fuzz.NEUTRAL, (s, res.get("sig")))


if __name__ == "__main__":
    unittest.main()
