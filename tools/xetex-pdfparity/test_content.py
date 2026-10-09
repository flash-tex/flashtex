"""Unit tests of the content-stream tokenizer and interpreter, and of the
comparator's matching (no TeX, no PDF files).

    python3 -m unittest discover -s tools/xetex-pdfparity -p 'test_*.py'
"""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import compare as C  # noqa: E402
from content import IDENTITY, Interp, Name, Op, apply, mul, tokens  # noqa: E402
from fonts import Glyph, parse_tounicode, strip_tag  # noqa: E402
from pdfdoc import Stream  # noqa: E402


class FakeFont:
    """One-byte codes, every glyph 500/1000 em wide (code 32: 250)."""
    name = "Fake"
    code_bytes = {1}
    font_matrix = None

    def split(self, s):
        return list(s)

    def glyph(self, code):
        return Glyph("code:%d" % code, None, 250 if code == 32 else 500, chr(code), code)


class FakeFonts:
    def font(self, ref):
        return FakeFont()


class FakeDoc:
    def __init__(self, objs=None):
        self.objs = objs or {}
        self.page_index = {}

    def get(self, x):
        while isinstance(x, str) and x in self.objs:
            x = self.objs[x]
        return x

    def stream(self, x):
        x = self.get(x)
        return x if isinstance(x, Stream) else None

    def canon(self, x, skip=(), depth=0, seen=None):
        x = self.get(x)
        if isinstance(x, Stream):
            return {"dict": self.canon(x.dict), "data": x.data.hex()}
        if isinstance(x, dict):
            return {k: self.canon(v) for k, v in sorted(x.items())}
        if isinstance(x, list):
            return [self.canon(v) for v in x]
        return x


RES = {"/Font": {"/F1": "1 0 R"}}


def run(data, objs=None, res=RES, ctm=IDENTITY):
    return Interp(FakeDoc(objs), FakeFonts()).run_stream(data, res, ctm)


def close(a, b, eps=1e-9):
    return all(abs(x - y) <= eps for x, y in zip(a, b))


class Tokenizer(unittest.TestCase):
    def test_operands_and_operators(self):
        t = list(tokens(b"1 0 0 1 72 720 cm /F1 9.9626 Tf [(a\\(b) -12 <0041>] TJ"))
        self.assertEqual(t[0], ([1, 0, 0, 1, 72, 720], "cm"))
        self.assertIsInstance(t[0][1], Op)
        self.assertEqual(t[1][0], ["F1", 9.9626])
        self.assertIsInstance(t[1][0][0], Name)
        self.assertEqual(t[2], ([[b"a(b", -12, b"\x00A"]], "TJ"))

    def test_numbers(self):
        t = list(tokens(b"-.5 +3 4. -0.001 .25 l"))
        self.assertEqual(t[0][0], [-0.5, 3, 4.0, -0.001, 0.25])

    def test_literal_strings(self):
        t = list(tokens(b"(a (nested) b\\053\\n\\\\x\\\nend) Tj (\\0) Tj"))
        self.assertEqual(t[0][0], [b"a (nested) b+\n\\xend"])
        self.assertEqual(t[1][0], [b"\x00"])

    def test_names_hex_and_comments(self):
        t = list(tokens(b"/A#20B cs % comment ) ( [\n<4 1 4> Tj"))
        self.assertEqual(t[0][0], ["A B"])
        self.assertEqual(t[1][0], [b"A@"])  # an odd digit count: a trailing 0 (0x40)

    def test_dict_operand_and_booleans(self):
        t = list(tokens(b"/P <</MCID 0 /X [1 2] /B true>> BDC EMC"))
        self.assertEqual(t[0][0], ["P", {"MCID": 0, "X": [1, 2], "B": True}])
        self.assertEqual(t[1], ([], "EMC"))

    def test_inline_image(self):
        t = list(tokens(b"q BI /W 2 /H 1 /CS /G /BPC 8 ID \x00\xffEI\xff EI Q"))
        self.assertEqual(t[0], ([], "q"))
        d, data = t[1][0]
        self.assertEqual(t[1][1], "BI")
        self.assertEqual(d, {"W": 2, "H": 1, "CS": "G", "BPC": 8})
        self.assertEqual(data, b"\x00\xffEI\xff")  # 'EI' not after white space is data
        self.assertEqual(t[2], ([], "Q"))


class Matrices(unittest.TestCase):
    def test_mul_apply(self):
        t = (1, 0, 0, 1, 10, 20)
        s = (2, 0, 0, 3, 0, 0)
        self.assertEqual(apply(mul(t, s), 1, 1), (22, 63))  # translate, then scale
        self.assertEqual(apply(mul(s, t), 1, 1), (12, 23))


class Text(unittest.TestCase):
    def test_tj_positions(self):
        p = run(b"BT /F1 10 Tf 100 200 Td (ab) Tj [(c) -1000 (d)] TJ ET")
        xs = [(g["glyph"], g["x"], g["y"]) for g in p.glyphs]
        self.assertEqual(xs, [("code:97", 100, 200), ("code:98", 105, 200),
                              ("code:99", 110, 200), ("code:100", 125, 200)])
        self.assertEqual(p.glyphs[0]["m"], (10, 0, 0, 10))
        self.assertEqual(p.glyphs[0]["fill"], ("DeviceGray", (0.0,)))

    def test_text_state(self):
        p = run(b"BT /F1 10 Tf 2 Tc 3 Tw 50 Tz 1 Ts 2 Tr 1 0 0 rg 0 0 1 RG (a b) Tj ET")
        g = p.glyphs
        # Tz scales the advances and the matrix; Tw applies to code 32 only.
        self.assertEqual([x["x"] for x in g], [0, (5 + 2) * 0.5, (5 + 2) * 0.5 + (2.5 + 2 + 3) * 0.5])
        self.assertEqual(g[0]["y"], 1)
        self.assertEqual(g[0]["m"], (5, 0, 0, 10))
        self.assertEqual(g[0]["mode"], 2)
        self.assertEqual(g[0]["fill"], ("DeviceRGB", (1.0, 0.0, 0.0)))
        self.assertEqual(g[0]["stroke"], ("DeviceRGB", (0.0, 0.0, 1.0)))

    def test_lines_and_tm(self):
        p = run(b"BT /F1 10 Tf 12 TL 5 50 Td (a) Tj T* (b) Tj (c) ' 0 -7 TD (d) Tj 2 0 0 2 300 400 Tm (e) Tj ET")
        pos = [(g["x"], g["y"]) for g in p.glyphs]
        self.assertEqual(pos, [(5, 50), (5, 38), (5, 26), (5, 19), (300, 400)])
        self.assertEqual(p.glyphs[-1]["m"], (20, 0, 0, 20))

    def test_ctm_and_q(self):
        p = run(b"q 2 0 0 2 10 10 cm BT /F1 10 Tf 1 1 Td (a) Tj ET Q BT /F1 10 Tf (b) Tj ET")
        self.assertEqual((p.glyphs[0]["x"], p.glyphs[0]["y"]), (12, 12))
        self.assertEqual(p.glyphs[0]["m"], (20, 0, 0, 20))
        self.assertEqual((p.glyphs[1]["x"], p.glyphs[1]["y"]), (0, 0))

    def test_rotated_text(self):
        p = run(b"BT /F1 10 Tf 0 1 -1 0 100 100 Tm (ab) Tj ET")
        self.assertTrue(close((p.glyphs[1]["x"], p.glyphs[1]["y"]), (100, 105)))
        self.assertTrue(close(p.glyphs[0]["m"], (0, 10, -10, 0)))


class Paths(unittest.TestCase):
    def test_rect_fill_in_page_space(self):
        p = run(b"q 1 0 0 1 72 720 cm 0.2 0.4 0.6 rg 10 20 30 40 re f Q")
        e = p.paths[0]
        self.assertEqual(e["op"], "f")
        self.assertEqual([s[0] for s in e["segs"]], ["m", "l", "l", "l", "h"])
        self.assertEqual(e["segs"][0][1], (82, 740))
        self.assertEqual(e["segs"][2][1], (112, 780))
        self.assertEqual(e["fill"], ("DeviceRGB", (0.2, 0.4, 0.6)))

    def test_stroke_state_scales_and_restores(self):
        p = run(b"q 2 0 0 2 0 0 cm 1 0 0 RG 3 w [2 1] 0.5 d 1 J 0 0 m 10 0 l S Q 0 0 m 1 1 l S")
        a, b = p.paths
        self.assertEqual(a["lw"], 6)
        self.assertEqual(a["dash"], ((4, 2), 1))
        self.assertEqual(a["cap"], 1)
        self.assertEqual(a["stroke"], ("DeviceRGB", (1.0, 0.0, 0.0)))
        self.assertEqual(a["segs"][1][1], (20, 0))
        self.assertEqual(b["stroke"], ("DeviceGray", (0.0,)))
        self.assertEqual(b["lw"], 1)

    def test_curves_close_and_clip(self):
        p = run(b"0 0 m 1 1 2 2 3 3 c 4 4 5 5 v 6 6 7 7 y h W n 0 0 m 1 0 l b")
        c = p.paths[0]
        self.assertEqual(c["op"], "n")
        self.assertEqual(c["clip"], "W")
        self.assertEqual(c["segs"][2], ("c", (3, 3), (4, 4), (5, 5)))  # v: first control = current point
        self.assertEqual(c["segs"][3], ("c", (6, 6), (7, 7), (7, 7)))  # y: second control = end
        self.assertEqual(p.paths[1]["op"], "B")
        self.assertEqual(p.paths[1]["segs"][-1], ("h",))

    def test_colour_spaces(self):
        objs = {"5 0 R": ["/Separation", "/Spot", "/DeviceCMYK", {"/FunctionType": 2}]}
        res = {"/ColorSpace": {"/CS0": "5 0 R"}}
        p = run(b"/CS0 cs 0.3 scn 0 0 1 1 re f 0.1 0.2 0.3 0.4 k 0 0 1 1 re f 0.5 g 0 0 1 1 re f", objs, res)
        self.assertTrue(p.paths[0]["fill"][0].startswith("Separation(Spot,DeviceCMYK,"))
        self.assertEqual(p.paths[0]["fill"][1], (0.3,))
        self.assertEqual(p.paths[1]["fill"], ("DeviceCMYK", (0.1, 0.2, 0.3, 0.4)))
        self.assertEqual(p.paths[2]["fill"], ("DeviceGray", (0.5,)))

    def test_extgstate_opacity(self):
        res = {"/ExtGState": {"/G1": {"/ca": 0.5, "/CA": 0.25}}}
        p = run(b"/G1 gs 0 0 1 1 re B", res=res)
        self.assertEqual((p.paths[0]["fill_alpha"], p.paths[0]["stroke_alpha"]), (0.5, 0.25))


class XObjects(unittest.TestCase):
    def test_image_and_form(self):
        img = Stream({"/Subtype": "/Image", "/Width": 4, "/Height": 2, "/ColorSpace": "/DeviceRGB",
                      "/BitsPerComponent": 8}, b"\x00" * 24, "7 0 R")
        form = Stream({"/Subtype": "/Form", "/BBox": [0, 0, 10, 10], "/Matrix": [1, 0, 0, 1, 5, 5],
                       "/Resources": {"/XObject": {"/Im": "7 0 R"}}},
                      b"q 10 0 0 10 0 0 cm /Im Do Q 0 0 1 1 re f", "8 0 R")
        objs = {"7 0 R": img, "8 0 R": form}
        p = run(b"q 2 0 0 2 100 100 cm /Fm Do Q", objs, {"/XObject": {"/Fm": "8 0 R"}})
        self.assertEqual(len(p.forms), 1)
        self.assertEqual(p.forms[0]["ctm"], (2, 0, 0, 2, 100, 100))
        self.assertEqual(p.images[0]["ctm"], (20, 0, 0, 20, 110, 110))
        self.assertEqual((p.images[0]["w"], p.images[0]["h"], p.images[0]["cs"]), (4, 2, "DeviceRGB"))
        self.assertEqual(p.paths[0]["segs"][0][1], (110, 110))
        self.assertEqual(p.paths[0]["where"], "page/form0")

    def test_inline_image(self):
        p = run(b"q 8 0 0 4 1 2 cm BI /W 2 /H 1 /CS /RGB /BPC 8 ID abcdef EI Q")
        i = p.images[0]
        self.assertEqual((i["w"], i["h"], i["cs"], i["bpc"], i["ctm"]), (2, 1, "DeviceRGB", 8, (8, 0, 0, 4, 1, 2)))


class Fonts(unittest.TestCase):
    def test_strip_tag(self):
        self.assertEqual(strip_tag("ABCDEF+LMRoman10-Regular"), "LMRoman10-Regular")
        self.assertEqual(strip_tag("LMRoman10-Regular"), "LMRoman10-Regular")

    def test_tounicode(self):
        cmap = (b"1 begincodespacerange <0000> <FFFF> endcodespacerange "
                b"2 beginbfchar <0003> <0020> <0010> <D835DC00> endbfchar "
                b"1 beginbfrange <0020> <0022> <0041> endbfrange "
                b"1 beginbfrange <0030> <0031> [<0066006C> <0078>] endbfrange")
        m, spaces = parse_tounicode(cmap)
        self.assertEqual(spaces, {2})
        self.assertEqual(m[3], " ")
        self.assertEqual(m[0x10], "\U0001D400")
        self.assertEqual([m[0x20], m[0x22]], ["A", "C"])
        self.assertEqual([m[0x30], m[0x31]], ["fl", "x"])


class Matching(unittest.TestCase):
    def test_rules_compare_by_covered_rectangle(self):
        filled = run(b"0 0 1 rg 10 20 30 2 re f").paths[0]
        stroked = run(b"0 0 1 RG 2 w 10 21 m 40 21 l S").paths[0]
        a, b = C.as_rule(filled, 0.01), C.as_rule(stroked, 0.01)
        self.assertEqual(a["rect"], b["rect"])
        self.assertEqual(a["colour"], b["colour"])
        rep = C.Report()
        C.compare_paths(1, [filled], [stroked], C.Options(), rep, {})
        self.assertTrue(rep.ok(), rep.text())
        # A round cap makes the stroke cover more: not a rule then.
        self.assertIsNone(C.as_rule(run(b"1 J 2 w 10 21 m 40 21 l S").paths[0], 0.01))

    def test_glyph_tolerance_and_moves(self):
        a = run(b"BT /F1 10 Tf 100 200 Td (ab) Tj ET").glyphs
        near = run(b"BT /F1 10 Tf 100.009 200 Td (ab) Tj ET").glyphs
        far = run(b"BT /F1 10 Tf 100.02 200 Td (ab) Tj ET").glyphs
        exact = C.Options(rel_tol=0)  # --tol only: 0.01 bp
        stats = {}
        rep = C.Report()
        C.compare_glyphs(1, a, near, exact, rep, stats)
        self.assertTrue(rep.ok(), rep.text())
        self.assertAlmostEqual(stats["glyph_max_dev"], 0.009, places=9)
        rep = C.Report()
        C.compare_glyphs(1, a, far, exact, rep, {})
        self.assertEqual(rep.kinds["glyph-position"]["count"], 2)
        # The default adds 0.005 em (xdvipdfmx's measured error): 0.06 bp at 10 bp.
        rep = C.Report()
        C.compare_glyphs(1, a, far, C.Options(), rep, {})
        self.assertTrue(rep.ok(), rep.text())
        for dx, ok in ((b"100.059", True), (b"100.061", False)):
            b = run(b"BT /F1 10 Tf " + dx + b" 200 Td (ab) Tj ET").glyphs
            rep = C.Report()
            C.compare_glyphs(1, a, b, C.Options(), rep, {})
            self.assertEqual(rep.ok(), ok, (dx, rep.text()))

    def test_glyph_order(self):
        a = run(b"BT /F1 10 Tf 100 200 Td (ab) Tj ET").glyphs
        b = run(b"BT /F1 10 Tf 105 200 Td (b) Tj -5 0 Td (a) Tj ET").glyphs
        rep = C.Report()
        C.compare_glyphs(1, a, b, C.Options(), rep, {})
        self.assertEqual(list(rep.kinds), ["glyph-order"], rep.text())

    def test_glyph_swapped_missing_extra(self):
        a = run(b"BT /F1 10 Tf 100 200 Td (ab) Tj 0 100 Td (z) Tj ET").glyphs
        b = run(b"BT /F1 10 Tf 100 200 Td (ac) Tj 0 -100 Td (z) Tj ET").glyphs
        rep = C.Report()
        C.compare_glyphs(1, a, b, C.Options(), rep, {})
        self.assertEqual(rep.kinds["glyph-id"]["count"], 1)
        self.assertEqual(rep.kinds["glyph-missing"]["count"], 1)
        self.assertEqual(rep.kinds["glyph-extra"]["count"], 1)

    def test_colour_and_path_geometry(self):
        a = run(b"1 0 0 RG 0 0 m 10 5 l 20 0 l S").paths
        b = run(b"0 1 0 RG 0 0 m 10 5 l 20 0 l S").paths
        c = run(b"1 0 0 RG 0 0 m 10 5.5 l 20 0 l S").paths
        rep = C.Report()
        C.compare_paths(1, a, b, C.Options(), rep, {})
        self.assertEqual(list(rep.kinds), ["path-colour"])
        rep = C.Report()
        C.compare_paths(1, a, c, C.Options(), rep, {})
        self.assertEqual(list(rep.kinds), ["path-geometry"])


if __name__ == "__main__":
    unittest.main()
