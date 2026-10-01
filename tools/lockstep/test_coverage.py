#!/usr/bin/env python3
"""Tests for coverage.py: citation parsing, primitive extraction, override order."""
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import coverage as cov  # noqa: E402


class CitedTests(unittest.TestCase):
    def test_ranges_and_two_webs(self):
        c = cov.cited(r"% 9-x: y (tex.web §§656-657, 700; \middle is pdftex.web §1697)")
        self.assertEqual(c["tex.web"], {656, 657, 700})
        self.assertEqual(c["pdftex.web"], {1697})

    def test_uncited_and_non_comment(self):
        self.assertFalse(any(cov.cited("% 001-x: verifies something").values()))
        self.assertFalse(any(cov.cited(r"\input prelude").values()))

    def test_huge_range_ignored(self):
        self.assertFalse(any(cov.cited("% x (tex.web §§1-999)").values()))


class AnalyseTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, name, body, d=None):
        (d or self.d).joinpath(name).write_text(body)

    def test_primitives_skip_comments_and_boilerplate(self):
        self.write("1-a.tex", "% 1-a: x (tex.web §§5)\n\\input prelude\n\\font\\x=cmr10 \\vsize=1pt\n\\hsize=1pt\n"
                              "\\looseness=1 % \\parshape\n\\lsshipbox0\n\\end\n")
        sec, prim, unc = cov.analyse(cov.load_cases([self.d]), {"looseness", "parshape", "hsize", "end"})
        self.assertEqual(set(prim), {"looseness", "hsize", "end"})
        self.assertEqual(sec["tex.web"][5], ["1-a.tex"])
        self.assertEqual(unc, [])

    def test_later_directory_overrides(self):
        other = self.d / "b"
        other.mkdir()
        self.write("1-a.tex", "\\hsize=1pt\n")
        self.write("1-a.tex", "\\vsize=1pt\n", other)
        cases = cov.load_cases([self.d, other])
        self.assertEqual(list(cases), ["1-a.tex"])
        self.assertIn("vsize", cases["1-a.tex"].read_text())


if __name__ == "__main__":
    unittest.main()
