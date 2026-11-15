"""Virtual fonts in the XDV check (precision.VirtualFonts): the fixture of
crates/flashtex-xetex/tests/vf, whose XDV (TeX Live 2026 `xetex -no-pdf`)
and expected.json (what TeX Live 2026's xdvipdfmx drew of the same files)
are both the oracle's. The XDV's glyphs, with the virtual fonts' packets
run as precision.py runs them, are where xdvipdfmx drew them, within its
own rounding. No TeX Live is needed: the fixture's files stand in for
kpsewhich. (python3 -m unittest discover -s tools/xetex-pdfparity -p
'test_vf.py'; needs fontTools, as precision.py's imports do.)"""
import json
import os
import re
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import precision as P  # noqa: E402

FIXTURE = os.path.normpath(os.path.join(HERE, "..", "..", "crates", "flashtex-xetex", "tests", "vf"))


def fixture_file(name):
    p = os.path.join(FIXTURE, name)
    return p if os.path.isfile(p) else None


class SqxfwTest(unittest.TestCase):
    def test_numbers_c(self):
        """The table the Rust port is tested with, from numbers.c itself."""
        path = os.path.join(FIXTURE, "..", "..", "src", "out", "testdata", "sqxfw_oracle.rs")
        with open(path) as f:
            rows = re.findall(r"\((-?\d+), (-?\d+), (-?\d+)\)", f.read())
        self.assertGreater(len(rows), 1000)
        for sq, fw, want in rows:
            self.assertEqual(P.sqxfw(int(sq), int(fw)), int(want), (sq, fw))


class VirtualFontTest(unittest.TestCase):
    def setUp(self):
        self.kpsewhich = P.kpsewhich
        P.kpsewhich = fixture_file

    def tearDown(self):
        P.kpsewhich = self.kpsewhich

    def test_fixture_glyphs_are_where_xdvipdfmx_drew_them(self):
        with open(os.path.join(FIXTURE, "expected.json")) as f:
            want = json.load(f)
        pages, conv = P.xdv_glyphs(os.path.join(FIXTURE, "vf.xdv"))
        self.assertEqual(len(pages), 1)
        got = pages[0]
        self.assertEqual(len(got), len(want["glyphs"]))
        height = want["media_box"][3]
        worst = 0.0
        for (h, v, c, native, size, _px, _py), w in zip(got, want["glyphs"]):
            self.assertFalse(native)
            self.assertEqual(c, w["code"])
            em = size * conv
            # the size of the font the packet set it in
            self.assertAlmostEqual(em, w["m"][0] / 1000.0, delta=2e-4)
            x, y = 72.0 + h * conv, height - 72.0 - v * conv
            tol = 0.01 + 0.005 * em
            self.assertLessEqual(abs(x - w["x"]), tol, (c, x, w))
            self.assertLessEqual(abs(y - w["y"]), tol, (c, y, w))
            worst = max(worst, abs(x - w["x"]) / em, abs(y - w["y"]) / em)
        self.assertLess(worst, 0.005)

    def test_a_font_with_a_map_entry_is_not_virtual(self):
        vfs = P.VirtualFonts()
        self.assertTrue(vfs.mapped("fxvfbase"))
        self.assertFalse(vfs.mapped("fxvf"))
        self.assertIsNone(vfs.locate("fxvfbase", 655360)["vf"])
        f = vfs.locate("fxvfn", 655360)
        # its first font is `fxvf` at half its size, a virtual font again
        (n0, f0), (n1, f1) = f["vf"][0]
        self.assertEqual((n0, f0["name"], f0["size"]), (0, "fxvf", 327680))
        self.assertIsNotNone(f0["vf"])
        self.assertEqual((n1, f1["name"], f1["size"], f1["vf"]), (1, "fxvfbase", 655360, None))


if __name__ == "__main__":
    unittest.main()
