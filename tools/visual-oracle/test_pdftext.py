#!/usr/bin/env python3
"""Tests for tools/visual-oracle/pdftext.py font reader robustness."""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import pdftext  # noqa: E402


def tiny_font_pdf(widths_src, missing_width=None):
    """A one-page PDF whose /F1 font dict has /FirstChar 65 and the given
    raw /Widths source (so malformed tokens like ``-40.-9`` pass through
    the lexer untouched)."""
    objs = []
    objs.append("<< /Type /Catalog /Pages 2 0 R >>")
    objs.append("<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
    objs.append("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
                "/Resources << /Font << /F1 4 0 R >> >> >>")
    desc = ""
    if missing_width is not None:
        desc = "/FontDescriptor 5 0 R "
    objs.append("<< /Type /Font /Subtype /Type1 /BaseFont /SFRM1095 "
                "/FirstChar 65 /LastChar 67 %s/Widths [%s] >>" % (desc, widths_src))
    if missing_width is not None:
        objs.append("<< /Type /FontDescriptor /FontName /SFRM1095 /MissingWidth %s >>" % missing_width)
    out = bytearray(b"%PDF-1.5\n")
    offsets = []
    for i, o in enumerate(objs, 1):
        offsets.append(len(out))
        out += b"%d 0 obj\n%s\nendobj\n" % (i, o.encode())
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    for off in offsets:
        out += b"%010d 00000 n \n" % off
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
    return bytes(out)


def font_info(widths_src, missing_width=None):
    doc = pdftext.PdfDocument(tiny_font_pdf(widths_src, missing_width))
    page = doc.pages()[0]
    resources = doc.resolve(page["Resources"])
    fonts = doc.resolve(resources["Font"])
    return doc.font(fonts["F1"])


class MalformedWidthsTests(unittest.TestCase):
    def test_stray_minus_token_is_read_as_the_number(self):
        # Real pdfTeX output contains the malformed token "-40.-9", which the
        # lexer yields as ("op", b"-40.-9"); float() on it used to raise
        # TypeError and kill the parity scorer. pdf.js and Adobe read it as
        # -40.9 (the stray minus is ignored); poppler splits it into two
        # numbers, which would shift every later width by one slot.
        info = font_info("500 -40.-9 600")
        self.assertAlmostEqual(info["widths"][65], 0.5)
        self.assertAlmostEqual(info["widths"][66], -0.0409)
        self.assertAlmostEqual(info["widths"][67], 0.6)
        self.assertEqual(len(info["notes"]), 1)
        self.assertIn("read as -40.9", info["notes"][0])

    def test_unreadable_op_token_uses_nonzero_missing_width(self):
        info = font_info("500 5..5 600", missing_width=250)
        self.assertAlmostEqual(info["widths"][66], 0.25)
        self.assertAlmostEqual(info["widths"][67], 0.6)
        self.assertIn("used MissingWidth", info["notes"][0])

    def test_clean_widths_leave_no_notes(self):
        info = font_info("500 510 600")
        self.assertNotIn("notes", info)

    def test_null_and_array_elements_still_raise(self):
        with self.assertRaises(TypeError):
            font_info("500 null 600")
        with self.assertRaises(TypeError):
            font_info("500 [1 2] 600")

    def test_malformed_missing_width_still_raises(self):
        with self.assertRaises((TypeError, ValueError)):
            font_info("500 510 600", missing_width="(abc)")


if __name__ == "__main__":
    unittest.main()
