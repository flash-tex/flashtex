"""Pure-Python tests for tools/visual-oracle (no helper binaries needed).

    python3 -m unittest discover -s tools/visual-oracle -p 'test_*.py' -v
"""

import os
import sys
import tempfile
import unittest
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import pdftext  # noqa: E402
import rank  # noqa: E402
import thumbs  # noqa: E402

REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
HW1_REF = os.path.join(REPO, "fixtures", "real-world", "hw1", "HW1-reference.pdf")


def tiny_pdf(content, widths=None, compress=True, differences=None):
    """A one-page PDF with one Type1 font (/F1) and the given content stream."""
    widths = widths or [500] * 95
    enc = ""
    if differences:
        enc = " /Encoding << /Type /Encoding /Differences [" + differences + "] >>"
    objs = []
    objs.append("<< /Type /Catalog /Pages 2 0 R >>")
    objs.append("<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
    objs.append("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>")
    data = content.encode("latin-1")
    if compress:
        data = zlib.compress(data)
        objs.append(("<< /Length %d /Filter /FlateDecode >>" % len(data), data))
    else:
        objs.append(("<< /Length %d >>" % len(data), data))
    objs.append("<< /Type /Font /Subtype /Type1 /BaseFont /ABCDEF+SFRM1095 /FirstChar 32 /LastChar %d /Widths [%s]%s >>"
                % (32 + len(widths) - 1, " ".join(str(w) for w in widths), enc))
    out = bytearray(b"%PDF-1.5\n")
    offsets = []
    for i, o in enumerate(objs, 1):
        offsets.append(len(out))
        if isinstance(o, tuple):
            out += b"%d 0 obj\n%s\nstream\n" % (i, o[0].encode()) + o[1] + b"\nendstream\nendobj\n"
        else:
            out += b"%d 0 obj\n%s\nendobj\n" % (i, o.encode())
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    for off in offsets:
        out += b"%010d 00000 n \n" % off
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
    return bytes(out)


class PdfTextTests(unittest.TestCase):
    def test_words_positions_and_widths(self):
        # 10 pt font, every glyph 500/1000 em = 5 bp: "ab" then a 300/1000 space gap, then "cd"
        content = "BT /F1 10 Tf 72 700 Td [(ab) -300 (cd)] TJ ET"
        pdf = tiny_pdf(content, compress=True)
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            f.write(pdf)
        pages = pdftext.page_words(f.name)
        os.unlink(f.name)
        self.assertEqual(len(pages), 1)
        words = pages[0]["words"]
        self.assertEqual([w["text"] for w in words], ["ab", "cd"])
        self.assertAlmostEqual(words[0]["x"], 72.0, places=3)
        self.assertAlmostEqual(words[0]["y_top"], 92.0, places=3)  # 792 - 700
        self.assertAlmostEqual(words[0]["width"], 10.0, places=3)
        self.assertAlmostEqual(words[1]["x"], 72 + 10 + 3.0, places=3)  # TJ -300 -> +3 bp at 10 pt

    def test_small_kern_keeps_word_and_td_moves_line(self):
        content = "BT /F1 10 Tf 72 700 Td [(a) -50 (b)] TJ 0 -12 Td (c) Tj ET"
        pdf = tiny_pdf(content, compress=False)
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            f.write(pdf)
        words = pdftext.page_words(f.name)[0]["words"]
        os.unlink(f.name)
        self.assertEqual([w["text"] for w in words], ["ab", "c"])
        self.assertAlmostEqual(words[1]["y_top"], 104.0, places=3)

    def test_differences_map_ligatures(self):
        content = "BT /F1 10 Tf 72 700 Td (\\034x) Tj ET"  # code 28 = fi in T1
        pdf = tiny_pdf(content, differences="28 /fi", widths=[500] * 95)
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            f.write(pdf)
        words = pdftext.page_words(f.name)[0]["words"]
        os.unlink(f.name)
        self.assertEqual(words[0]["text"], "fix")

    @unittest.skipUnless(os.path.isfile(HW1_REF), "HW1-reference.pdf not present")
    def test_hw1_reference_reads_three_pages(self):
        pages = pdftext.page_words(HW1_REF)
        self.assertEqual(len(pages), 3)
        first = pages[0]["words"]
        self.assertGreater(len(first), 200)
        self.assertEqual(pages[0]["notes"], [])
        texts = [w["text"] for w in first[:6]]
        self.assertEqual(texts, ["21-128", "and", "15-151", "Problem", "Sheet", "1"])
        self.assertAlmostEqual(first[3]["y_top"], 99.497, places=2)
        self.assertAlmostEqual(first[3]["x"], 239.322, places=2)


class RankTests(unittest.TestCase):
    def _v2(self, words):
        Q = 2 ** 20
        items = []
        for text, x, y, src in words:
            glyphs, clusters = [], []
            at = 0  # cluster ranges are UTF-8 byte offsets
            for i, ch in enumerate(text):
                n = len(ch.encode("utf-8"))
                glyphs.append({"gid": 1, "origin_x": int((x + 5 * i) * Q), "baseline_y": int(y * Q),
                               "advance_x": 5 * Q, "advance_y": 0, "cluster": i})
                clusters.append({"text_start_byte": at, "text_end_byte": at + n,
                                 "sources": [{"path": "main.tex", "start_byte": src + i, "end_byte": src + i + 1}]})
                at += n
            items.append({"kind": "glyph_run", "font_id": "f", "font_size": 10 * Q, "text": text,
                          "glyphs": glyphs, "clusters": clusters})
        return {"payload": {"coordinate_unit": "bp_2pow20", "fonts": [{"font_id": "f", "postscript_name": "LMRoman10-Regular"}],
                            "pages": [{"number": 1, "width": 612 * Q, "height": 792 * Q, "items": items}]}}

    def test_v2_words_and_geometry(self):
        cand = rank.v2_words(self._v2([("Hello", 72, 100, 10), ("world", 110, 100, 16), ("x", 72, 114.5, 30)]))
        self.assertEqual([w["text"] for w in cand[0]["words"]], ["Hello", "world", "x"])
        self.assertEqual(cand[0]["words"][1]["source"], {"path": "main.tex", "start_byte": 16, "end_byte": 21})
        ref = [{"text": "Hello", "x": 72.0, "y_top": 100.0, "size": 10, "font": "SFRM1095"},
               {"text": "world", "x": 110.0, "y_top": 101.0, "size": 10, "font": "SFRM1095"},
               {"text": "gone", "x": 200.0, "y_top": 101.0, "size": 10, "font": "SFRM1095"},
               {"text": "x", "x": 72.0, "y_top": 114.5, "size": 10, "font": "SFRM1095"}]
        g = rank.geometry_page(ref, cand[0]["words"], [], top_n=3)
        self.assertEqual(g["aligned"], 3)
        self.assertEqual(g["reference_unaligned"], 1)
        self.assertEqual(g["candidate_unaligned"], 0)
        self.assertEqual(g["max_delta"], 1.0)
        self.assertEqual(g["top"][0]["text"], "world")
        self.assertEqual(g["top"][0]["dy"], -1.0)
        self.assertEqual(g["within_0_01"], 2)
        self.assertEqual(g["reflowed"], 0)

    def test_v2_word_text_after_a_multibyte_character(self):
        cand = rank.v2_words(self._v2([("x−y∈K", 72, 100, 0), ("ok", 110, 100, 10)]))
        self.assertEqual([w["text"] for w in cand[0]["words"]], ["x−y∈K", "ok"])

    def test_owner_prefers_overlapping_diagnostic_then_math_then_shift(self):
        w = {"source": {"path": "main.tex", "start_byte": 10, "end_byte": 15}, "math": False}
        d = [{"code": "compiler", "message": "\\foo is not supported", "severity": "error",
              "sources": [{"path": "main.tex", "start_byte": 12, "end_byte": 20}]}]
        self.assertIn("crates/compiler", rank.owner_for_word(w, (1, 1), (0, 0), d))
        self.assertIn("math-layout", rank.owner_for_word(dict(w, math=True), (1, 1), (0, 0), []))
        self.assertIn("page builder", rank.owner_for_word(w, (0.0, -3.0), (0.0, -3.0), []))
        self.assertIn("line breaking", rank.owner_for_word(w, (400.0, -13.0), (0.0, 0.0), []))
        self.assertIn("isolated", rank.owner_for_word(w, (0.3, 0.0), (0.0, 0.0), []))

    def test_rank_key_orders_missing_then_unaligned_then_delta_then_pixels(self):
        pages = [
            ("a", {"page": 1, "result": "compared", "differing": 10, "geometry": {"aligned": 5, "max_delta": 0.01}}),
            ("b", {"page": 1, "result": "compared", "differing": 999, "geometry": {"aligned": 5, "max_delta": 0.01}}),
            ("c", {"page": 2, "result": "missing_in_candidate"}),
            ("d", {"page": 1, "result": "compared", "differing": 1, "geometry": {"aligned": 0, "max_delta": None}}),
            ("e", {"page": 1, "result": "compared", "differing": 1, "geometry": {"aligned": 5, "max_delta": 40.0}}),
        ]
        order = [fid for fid, _ in sorted(pages, key=lambda fp: rank.rank_key(fp[1]))]
        self.assertEqual(order, ["c", "d", "e", "b", "a"])

    def test_minus_sign_aligns_with_hyphen_minus(self):
        ref = [{"text": t} for t in ["ad", "-", "bc"]]
        cand = [{"text": t} for t in ["ad", "−", "bc"]]
        self.assertEqual(rank.align_words(ref, cand), ([(0, 0), (1, 1), (2, 2)], 0, 0))
        self.assertNotEqual(rank.norm("−"), rank.norm("="))


class ThumbTests(unittest.TestCase):
    def test_sheet_writes_png_with_diff_colours(self):
        w, h = 6, 3
        ref = bytes([255] * (w * h))
        cand = bytearray(ref)
        cand[0] = 0  # candidate-only ink at (0,0)
        with tempfile.TemporaryDirectory() as d:
            for name, px in (("r.pgm", ref), ("c.pgm", bytes(cand))):
                with open(os.path.join(d, name), "wb") as f:
                    f.write(b"P5\n%d %d\n255\n" % (w, h) + px)
            out = os.path.join(d, "s.png")
            sw, sh = thumbs.sheet(os.path.join(d, "r.pgm"), os.path.join(d, "c.pgm"), out, scale=1, gutter=1)
            self.assertEqual((sw, sh), (6 * 3 + 2, 3))
            with open(out, "rb") as f:
                data = f.read()
            self.assertTrue(data.startswith(b"\x89PNG"))
        rgb = thumbs.diff_rgb(2, 1, bytes([0, 255]), bytes([255, 255]))
        self.assertEqual(rgb[:3], bytes((30, 60, 220)))  # reference-only ink = blue


class WordGroupingTests(unittest.TestCase):
    """`rank.v2_words` must group candidate glyphs by the same rule the
    reference side uses, or the alignment measures the two halves of the
    tool disagreeing about what a word is."""

    def _runs(self, runs):
        """A one-page display list from (text, font_id, x, y, advance, src)."""
        Q = 2 ** 20
        items = []
        for text, font_id, x, y, adv, src in runs:
            glyphs, clusters, at = [], [], x
            for i, ch in enumerate(text):
                glyphs.append({"gid": 1, "origin_x": int(at * Q), "baseline_y": int(y * Q),
                               "advance_x": int(adv * Q), "advance_y": 0, "cluster": i})
                clusters.append({"text_start_byte": i, "text_end_byte": i + 1,
                                 "sources": [{"path": "main.tex", "start_byte": src + i, "end_byte": src + i + 1}]})
                at += adv
            items.append({"kind": "glyph_run", "font_id": font_id, "font_size": 10 * Q,
                          "text": text, "glyphs": glyphs, "clusters": clusters})
        fonts = [{"font_id": "rm", "postscript_name": "LMRoman10-Regular"},
                 {"font_id": "mi", "postscript_name": "LMMathItalic10-Regular"}]
        return {"payload": {"coordinate_unit": "bp_2pow20", "fonts": fonts,
                            "pages": [{"number": 1, "width": 612 * Q, "height": 792 * Q, "items": items}]}}

    def test_adjacent_runs_of_different_fonts_are_one_word(self):
        # pdfTeX sets a siunitx `S` cell as three Tf-switched runs in one text
        # object; `pdftext.words_from_glyphs` joins them because a font change
        # does not end a word. The candidate must do the same, or the cell
        # reads as three words against the reference's one.
        page = rank.v2_words(self._runs([
            ("1", "rm", 72.0, 100.0, 5.0, 0),
            (".", "mi", 77.0, 100.0, 2.5, 1),
            ("234", "rm", 79.5, 100.0, 5.0, 2),
        ]))[0]
        self.assertEqual([w["text"] for w in page["words"]], ["1.234"])
        w = page["words"][0]
        self.assertAlmostEqual(w["x"], 72.0)
        self.assertAlmostEqual(w["width"], 22.5)
        # Per-glyph facts survive the grouping: the span covers all three runs
        # and a math font anywhere in the word marks it.
        self.assertEqual(w["source"], {"path": "main.tex", "start_byte": 0, "end_byte": 5})
        self.assertTrue(w["math"])

    def test_a_gap_wider_than_the_space_fraction_still_splits(self):
        # 0.16 em at 10 pt = 1.6 bp. Same baseline, same font, 3 bp clear.
        page = rank.v2_words(self._runs([
            ("ab", "rm", 72.0, 100.0, 5.0, 0),
            ("cd", "rm", 85.0, 100.0, 5.0, 2),
        ]))[0]
        self.assertEqual([w["text"] for w in page["words"]], ["ab", "cd"])

    def test_runs_on_different_baselines_are_never_joined(self):
        page = rank.v2_words(self._runs([
            ("ab", "rm", 72.0, 100.0, 5.0, 0),
            ("cd", "rm", 82.0, 112.0, 5.0, 2),
        ]))[0]
        self.assertEqual([w["text"] for w in page["words"]], ["ab", "cd"])

    def test_a_backwards_jump_splits_so_stray_ink_cannot_hide_in_a_word(self):
        # `fixtures/real-world/unicode-accents` has a run whose last five
        # glyphs sit at x = -12321 bp. Grouping by run alone hid that inside
        # one word whose `x` came from its first glyph.
        page = rank.v2_words(self._runs([
            ("ab", "rm", 72.0, 100.0, 5.0, 0),
            ("cd", "rm", -500.0, 100.0, 5.0, 2),
        ]))[0]
        self.assertEqual([w["text"] for w in page["words"]], ["ab", "cd"])
        self.assertAlmostEqual(page["words"][1]["x"], -500.0)

    def test_words_from_glyphs_indexes_its_input(self):
        glyphs = [{"text": t, "x": x, "y_top": 100.0, "advance": 5.0, "size": 10.0,
                   "font": "F", "bt": 0}
                  for t, x in (("a", 72.0), ("b", 77.0), (" ", 82.0), ("c", 87.0))]
        words = pdftext.words_from_glyphs(glyphs)
        self.assertEqual([w["text"] for w in words], ["ab", "c"])
        # A word's glyphs are contiguous in the input, so a caller can carry
        # its own per-glyph data across the grouping.
        self.assertEqual([(w["glyph_index"], w["glyphs"]) for w in words], [(0, 2), (3, 1)])


if __name__ == "__main__":
    unittest.main()
