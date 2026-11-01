"""Outline digests: equal within 1/100 font unit's resolution, different
outlines still different. (python3 -m unittest discover -s
tools/xetex-pdfparity -p 'test_outline.py'; needs fontTools.)"""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from fonts import OUTLINE_TOL, _outline_digest, same_outline  # noqa: E402


def outline(points):
    def draw(pen):
        pen.moveTo(points[0])
        for p in points[1:]:
            pen.lineTo(p)
        pen.closePath()
    return _outline_digest(draw)


class OutlineTest(unittest.TestCase):
    def test_rounding_boundary_is_the_same_outline(self):
        # 12.345 - 2^-16 and 12.345 + 2^-16 round to different hundredths
        a = outline([(0, 0), (12.345 - 2 ** -16, 7), (20, 0)])
        b = outline([(0, 0), (12.345 + 2 ** -16, 7), (20, 0)])
        self.assertNotEqual(a, b)  # the digests differ
        self.assertTrue(same_outline(a, b))  # the outline does not

    def test_a_real_change_is_found(self):
        a = outline([(0, 0), (12, 7), (20, 0)])
        self.assertFalse(same_outline(a, outline([(0, 0), (12 + 2 * OUTLINE_TOL, 7), (20, 0)])))
        self.assertFalse(same_outline(a, outline([(0, 0), (12, 7), (20, 0), (5, 5)])))

    def test_equal_digests(self):
        a = outline([(0, 0), (1, 1)])
        self.assertTrue(same_outline(a, outline([(0, 0), (1, 1)])))
        self.assertTrue(same_outline("abc", "abc"))
        self.assertFalse(same_outline("abc", "abd"))


if __name__ == "__main__":
    unittest.main()
