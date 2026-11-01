"""Tests for tools/parity/wordalign.py and pdftext.py's word grouping (pure Python, no binaries).

    python3 -m unittest discover -s tools/parity -p 'test_*.py' -v

Ported from tools/visual-oracle/test_rank.py when retirement stage S7 deleted
that tool; the cases are unchanged, measured through `aligned_pairs`.
"""

import os
import sys
import tempfile
import unittest
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import pdftext  # noqa: E402
import wordalign  # noqa: E402

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


class AlignTests(unittest.TestCase):
    def test_minus_sign_aligns_with_hyphen_minus(self):
        ref = [{"text": t} for t in ["ad", "-", "bc"]]
        cand = [{"text": t} for t in ["ad", "\u2212", "bc"]]
        self.assertEqual(wordalign.align_words(ref, cand), ([(0, 0), (1, 1), (2, 2)], 0, 0))
        self.assertNotEqual(wordalign.norm("\u2212"), wordalign.norm("="))


class AnchorRepairTests(unittest.TestCase):
    """`reanchor_pairs`: difflib's text-only alignment pairs a candidate `2`
    with a same-text reference `2` several lines away when a table of
    contents' repeated section and page numbers come in another order. The
    pairs `aligned_pairs` returns are repaired by position."""

    def _toc_words(self):
        sec1 = {"text": "2", "x": 72.0, "y_top": 100.0, "size": 10.0, "font": "F"}
        title1 = {"text": "Intro", "x": 90.0, "y_top": 100.0, "size": 10.0, "font": "F"}
        page1 = {"text": "2", "x": 534.6, "y_top": 100.0, "size": 10.0, "font": "F"}
        sec2 = {"text": "3", "x": 72.0, "y_top": 112.0, "size": 10.0, "font": "F"}
        title2 = {"text": "Method", "x": 90.0, "y_top": 112.0, "size": 10.0, "font": "F"}
        page2 = {"text": "3", "x": 534.6, "y_top": 112.0, "size": 10.0, "font": "F"}
        return sec1, title1, page1, sec2, title2, page2

    def _deltas(self, ref, cand, pairs):
        return [(cand[j]["x"] - ref[i]["x"], cand[j]["y_top"] - ref[i]["y_top"]) for i, j in pairs]

    def test_toc_repeated_page_numbers_are_repaired_by_position(self):
        sec1, title1, page1, sec2, title2, page2 = self._toc_words()
        ref = [sec1, title1, page1, sec2, title2, page2]
        cand = [dict(sec1), dict(sec2), dict(title1), dict(title2), dict(page1), dict(page2)]
        raw_pairs, raw_ref_un, raw_cand_un = wordalign.align_words(ref, cand)
        self.assertIn((2, 0), raw_pairs)  # the mispairing this test exists to catch
        self.assertEqual((raw_ref_un, raw_cand_un), (2, 2))
        pairs, ref_free, cand_free = wordalign.aligned_pairs(ref, cand)
        self.assertEqual(len(pairs), 5)
        self.assertEqual((len(ref_free), len(cand_free)), (1, 1))
        self.assertEqual(set(self._deltas(ref, cand, pairs)), {(0.0, 0.0)})

    def test_a_genuine_reflow_has_no_same_text_neighbour_to_anchor_to(self):
        ref = [
            {"text": "Start", "x": 72.0, "y_top": 100.0, "size": 10.0, "font": "F"},
            {"text": "middle", "x": 72.0, "y_top": 112.0, "size": 10.0, "font": "F"},
            {"text": "Conclusion", "x": 300.0, "y_top": 112.0, "size": 10.0, "font": "F"},
        ]
        cand = [
            {"text": "Start", "x": 72.0, "y_top": 100.0, "size": 10.0, "font": "F"},
            {"text": "middle", "x": 72.0, "y_top": 112.0, "size": 10.0, "font": "F"},
            {"text": "Conclusion", "x": 72.0, "y_top": 124.0, "size": 10.0, "font": "F"},
        ]
        pairs, ref_free, cand_free = wordalign.aligned_pairs(ref, cand)
        self.assertEqual(pairs, [(0, 0), (1, 1), (2, 2)])
        self.assertEqual((ref_free, cand_free), ([], []))
        self.assertEqual(self._deltas(ref, cand, pairs)[2], (-228.0, 12.0))

    def test_an_uneven_duplicate_count_leaves_the_extra_one_unaligned(self):
        sec1, title1, page1, sec2, title2, page2 = self._toc_words()
        extra = {"text": "2", "x": 300.0, "y_top": 124.0, "size": 10.0, "font": "F"}
        ref = [sec1, title1, page1, sec2, title2, page2, extra]
        cand = [dict(sec1), dict(sec2), dict(title1), dict(title2), dict(page1), dict(page2)]
        pairs, ref_free, cand_free = wordalign.aligned_pairs(ref, cand)
        self.assertEqual(len(pairs), 5)
        self.assertEqual((len(ref_free), len(cand_free)), (2, 1))  # "Intro" and the extra "2"; "Intro"
        self.assertEqual(set(self._deltas(ref, cand, pairs)), {(0.0, 0.0)})


if __name__ == "__main__":
    unittest.main()
