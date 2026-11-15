"""Tests for the display-list oracle tooling's pure logic (no binaries).

    python3 -m unittest discover -s tools/displaylist -p 'test_*.py' -v
"""

import contextlib
import io
import json
import os
import sys
import tempfile
import unittest
from fractions import Fraction as Fr
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import check_positions  # noqa: E402
import preview_parity_summary  # noqa: E402
import synctex_oracle  # noqa: E402


def widths500(font, code):
    return Fr(500)


class RoundHalfAway(unittest.TestCase):

    def test_halves_go_away_from_zero(self):
        self.assertEqual(check_positions.rnd(Fr(3, 2)), 2)
        self.assertEqual(check_positions.rnd(Fr(-3, 2)), -2)
        self.assertEqual(check_positions.rnd(Fr(5, 2)), 3)
        self.assertEqual(check_positions.rnd(Fr(-5, 2)), -3)

    def test_non_halves_round_to_nearest(self):
        self.assertEqual(check_positions.rnd(Fr(13156352, 100)), 131564)  # .52 up
        self.assertEqual(check_positions.rnd(Fr(46047232, 100)), 460472)  # .32 down
        self.assertEqual(check_positions.rnd(Fr(-46047232, 100)), -460472)
        self.assertEqual(check_positions.rnd(Fr(7)), 7)


class ExactNumbers(unittest.TestCase):

    def test_float_uses_its_decimal_spelling(self):
        self.assertEqual(check_positions.fr(0.1), Fr(1, 10))
        self.assertEqual(check_positions.fr(-2.5), Fr(-5, 2))


class Affine(unittest.TestCase):
    I = (Fr(1), Fr(0), Fr(0), Fr(1), Fr(0), Fr(0))

    def test_mul_composes_translations_in_order(self):
        m = check_positions.mul((Fr(1), Fr(0), Fr(0), Fr(1), Fr(2), Fr(3)),
                                (Fr(1), Fr(0), Fr(0), Fr(1), Fr(5), Fr(7)))
        self.assertEqual(m, (Fr(1), Fr(0), Fr(0), Fr(1), Fr(7), Fr(10)))

    def test_apply_scales_then_translates(self):
        self.assertEqual(check_positions.apply((Fr(2), Fr(0), Fr(0), Fr(3), Fr(1), Fr(1)),
                                               Fr(4), Fr(5)), (Fr(9), Fr(16)))


class LiteralStrings(unittest.TestCase):

    def test_nested_parens_balance(self):
        self.assertEqual(check_positions.lit_string(b"(a(b)c)", 0), (b"a(b)c", 7))

    def test_escapes_and_octal(self):
        self.assertEqual(check_positions.lit_string(br"(a\n\101b)", 0)[0], b"a\nAb")

    def test_backslash_newline_continues_and_cr_becomes_lf(self):
        self.assertEqual(check_positions.lit_string(b"(a\\\r\nb\rc)", 0)[0], b"ab\nc")


class Tokenizer(unittest.TestCase):

    def test_numbers_are_exact_fractions(self):
        self.assertEqual(list(check_positions.tokens(b"1.5 -2 .5 +4")),
                         [("num", Fr(3, 2)), ("num", Fr(-2)), ("num", Fr(1, 2)), ("num", Fr(4))])

    def test_names_comments_and_whitespace(self):
        self.assertEqual(list(check_positions.tokens(b"/F1 % a comment\n/F2")),
                         [("name", "F1"), ("name", "F2")])

    def test_hex_string_and_odd_length_pad(self):
        tok = lambda s: list(check_positions.tokens(s))  # noqa: E731
        self.assertEqual(tok(b"<4142>"), [("str", b"AB")])
        self.assertEqual(tok(b"<4>"), [("str", b"@")])

    def test_arrays_yield_brackets(self):
        kinds = [k for k, _ in check_positions.tokens(b"[ (x) -3 ] TJ")]
        self.assertEqual(kinds, ["[", "str", "num", "]", "op"])

    def test_inline_image_data_is_skipped_to_ei(self):
        self.assertEqual(list(check_positions.tokens(b"BI /W 1 ID \xff\xfe EI Q")),
                         [("op", "BI"), ("name", "W"), ("num", Fr(1)), ("op", "Q")])


class TextPositioning(unittest.TestCase):

    def test_tm_origin_and_advance_through_string(self):
        g, _, _ = check_positions.interpret(b"BT /F1 10 Tf 1 0 0 1 2 3 Tm (AB) Tj ET",
                                            Fr(10), widths500)
        self.assertEqual(g, [("F1", 65, 131564, 460472), ("F1", 66, 460472, 460472)])

    def test_td_moves_and_t_star_applies_leading(self):
        g, _, _ = check_positions.interpret(b"BT /F1 10 Tf 12 TL 2 3 Td (A) Tj T* (B) Tj ET",
                                            Fr(10), widths500)
        self.assertEqual([p[2:] for p in g], [(131564, 460472), (131564, 1249853)])

    def test_td_sets_leading_from_ty(self):
        g, _, _ = check_positions.interpret(b"BT /F1 10 Tf 2 3 TD (A) Tj T* (B) Tj ET",
                                            Fr(10), widths500)
        self.assertEqual([p[2:] for p in g], [(131564, 460472), (131564, 263127)])

    def test_tj_adjustment_shifts_the_text_matrix(self):
        g, _, _ = check_positions.interpret(b"BT /F1 10 Tf 1 0 0 1 2 3 Tm [(A) -500 (B)] TJ ET",
                                            Fr(10), widths500)
        self.assertEqual([p[2:] for p in g], [(131564, 460472), (789381, 460472)])

    def test_ts_rise_lifts_the_glyph(self):
        g, _, _ = check_positions.interpret(b"BT /F1 10 Tf 1 0 0 1 0 0 Tm 2 Ts (A) Tj ET",
                                            Fr(10), widths500)
        self.assertEqual([p[2:] for p in g], [(0, 526254)])

    def test_tc_tw_tz_widen_the_advance(self):
        g, _, _ = check_positions.interpret(
            b"BT /F1 10 Tf 1 0 0 1 0 0 Tm 100 Tc 200 Tw 50 Tz (A ) Tj ET", Fr(10), widths500)
        self.assertEqual([p[2:] for p in g], [(0, 657818), (3453542, 657818)])


class Rules(unittest.TestCase):

    def test_filled_rect_is_kind_zero(self):
        _, r, _ = check_positions.interpret(b"0 0 10 20 re f", Fr(100), widths500)
        self.assertEqual(r, [(0, 0, 5262541, 657818, 1315635)])

    def test_horizontal_stroke_is_kind_one(self):
        _, r, _ = check_positions.interpret(b"2 w 0 5 m 10 5 l S", Fr(100), widths500)
        self.assertEqual(r, [(1, 0, 6183485, 657818, 131564)])

    def test_vertical_stroke_is_kind_two(self):
        _, r, _ = check_positions.interpret(b"2 w 0 5 m 0 15 l S", Fr(100), widths500)
        self.assertEqual(r, [(2, -65782, 5591450, 131564, 657817)])

    def test_non_axis_strokes_are_not_rules(self):
        for content in (b"2 w 0 5 m 10 6 l S",       # diagonal
                        b"[1 1] 0 d 2 w 0 5 m 10 5 l S",  # dashed
                        b"1 J 2 w 0 5 m 10 5 l S"):      # round cap
            self.assertEqual(check_positions.interpret(content, Fr(100), widths500)[1], [])

    def test_fill_needs_unclipped_axis_aligned_single_rect(self):
        for content in (b"0 0 10 20 re W f",            # clipped
                        b"0 1 -1 0 0 0 cm 0 0 10 20 re f",  # rotated CTM
                        b"0 0 m 0 0 10 20 re f",         # compound path
                        b"0 0 5 5 re 0 0 10 20 re f"):   # two rects
            self.assertEqual(check_positions.interpret(content, Fr(100), widths500)[1], [])

    def test_translated_ctm_shifts_rule_edges(self):
        _, r, _ = check_positions.interpret(b"1 0 0 1 5 5 cm 0 0 10 20 re f",
                                            Fr(100), widths500)
        self.assertEqual(r, [(0, 328909, 4933632, 657817, 1315635)])


class GraphicsState(unittest.TestCase):
    HEAD = b"BT /F1 10 Tf 1 0 0 1 0 0 Tm (A) Tj ET"

    def test_q_q_restores_the_ctm(self):
        g, _, _ = check_positions.interpret(b"q 1 0 0 1 5 0 cm Q " + self.HEAD,
                                            Fr(10), widths500)
        self.assertEqual([p[2:] for p in g], [(0, 657818)])

    def test_cm_without_restore_moves_glyphs(self):
        g, _, _ = check_positions.interpret(b"1 0 0 1 5 0 cm " + self.HEAD, Fr(10), widths500)
        self.assertEqual([p[2:] for p in g], [(328909, 657818)])

    def test_do_records_the_placement_matrix(self):
        _, _, x = check_positions.interpret(b"q 1 0 0 1 5 0 cm /Fm1 Do Q", Fr(10), widths500)
        self.assertEqual(x, [("Fm1", (Fr(1), Fr(0), Fr(0), Fr(1), Fr(5), Fr(0)))])


class FontNumber(unittest.TestCase):

    def test_f_names_number(self):
        self.assertEqual(check_positions.font_number("F1"), 1)
        self.assertEqual(check_positions.font_number("F12"), 12)

    def test_other_names_are_none(self):
        self.assertIsNone(check_positions.font_number("Fm1"))
        self.assertIsNone(check_positions.font_number(""))
        self.assertIsNone(check_positions.font_number(None))


class Compare(unittest.TestCase):

    def test_equal_is_none(self):
        ref = {"glyphs": [("F1", 65, 1, 2)], "rules": [(0, 1, 2, 3, 4)]}
        dl = {"glyphs": [(1, 65, 1, 2)], "rules": [(0, 1, 2, 3, 4)]}
        self.assertIsNone(check_positions.compare(ref, dl, "page 1"))

    def test_first_glyph_difference(self):
        ref, dl = {"glyphs": [("F1", 65, 1, 2)], "rules": []}, {"glyphs": [(1, 66, 1, 2)], "rules": []}
        self.assertEqual(check_positions.compare(ref, dl, "page 1"),
                         {"where": "page 1", "glyph": 0,
                          "oracle": (1, 65, 1, 2), "display_list": (1, 66, 1, 2)})

    def test_glyph_count_mismatch(self):
        ref = {"glyphs": [("F1", 65, 1, 2)], "rules": []}
        self.assertEqual(check_positions.compare(ref, {"glyphs": [], "rules": []}, "page 1"),
                         {"where": "page 1", "glyphs": [1, 0]})

    def test_first_rule_difference(self):
        ref, dl = {"glyphs": [], "rules": [(0, 1, 2, 3, 4)]}, {"glyphs": [], "rules": [(1, 1, 2, 3, 4)]}
        self.assertEqual(check_positions.compare(ref, dl, "form 2"),
                         {"where": "form 2", "rule": 0,
                          "oracle": (0, 1, 2, 3, 4), "display_list": (1, 1, 2, 3, 4)})

    def test_rule_count_mismatch(self):
        dl = {"glyphs": [], "rules": [(1, 1, 2, 3, 4)]}
        self.assertEqual(check_positions.compare({"glyphs": [], "rules": []}, dl, "page 1"),
                         {"where": "page 1", "rules": [0, 1]})


def sweep_file(tmp, rows):
    path = os.path.join(tmp, "sweep.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(rows, f)
    return path


def run_summary(*argv):
    buf = io.StringIO()
    with mock.patch.object(sys, "argv", ["preview_parity_summary.py", *argv]), \
            contextlib.redirect_stdout(buf):
        rc = preview_parity_summary.main()
    return rc, buf.getvalue()


def sweep_row(fixture="fx", differing=0, incomplete=False, scale=1, page=0):
    pg = {"page": page, "scale": scale, "incomplete": incomplete,
          "differingPixels": differing, "maxChannelDelta": 0}
    return {"fixture": fixture, "pdf": "oracle.pdf", "pages": [pg]}


class SweepSummary(unittest.TestCase):

    def test_differing_render_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            rc, out = run_summary("--sweep", sweep_file(tmp, [sweep_row(differing=7)]))
        self.assertEqual(rc, 1)
        self.assertIn("FAIL", out.splitlines()[0])
        self.assertIn("7 px", out)

    def test_missing_sweep_report_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            rc, out = run_summary("--sweep", os.path.join(tmp, "absent.json"))
        self.assertEqual(rc, 1)
        self.assertIn("no sweep report", out)


class MainTex(unittest.TestCase):

    def test_picks_tex_with_matching_pdf(self):
        with tempfile.TemporaryDirectory() as tmp:
            for name in ("main.tex", "main.pdf", "other.tex"):
                open(os.path.join(tmp, name), "w").close()
            self.assertEqual(synctex_oracle.main_tex(tmp), "main.tex")

    def test_none_without_engine_pdf(self):
        with tempfile.TemporaryDirectory() as tmp:
            open(os.path.join(tmp, "a.tex"), "w").close()
            self.assertIsNone(synctex_oracle.main_tex(tmp))


if __name__ == "__main__":
    unittest.main()
