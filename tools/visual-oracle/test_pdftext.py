#!/usr/bin/env python3
"""Tests for tools/visual-oracle/pdftext.py font reader robustness."""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import pdftext  # noqa: E402


def tiny_font_pdf(widths_src):
    """A one-page PDF whose /F1 font dict has /FirstChar 65 and the given
    raw /Widths source (so malformed tokens like ``-40.-9`` pass through
    the lexer untouched)."""
    objs = []
    objs.append("<< /Type /Catalog /Pages 2 0 R >>")
    objs.append("<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
    objs.append("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
                "/Resources << /Font << /F1 4 0 R >> >> >>")
    objs.append("<< /Type /Font /Subtype /Type1 /BaseFont /SFRM1095 "
                "/FirstChar 65 /LastChar 67 /Widths [%s] >>" % widths_src)
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


def font_info(widths_src):
    doc = pdftext.PdfDocument(tiny_font_pdf(widths_src))
    page = doc.pages()[0]
    resources = doc.resolve(page["Resources"])
    fonts = doc.resolve(resources["Font"])
    return doc.font(fonts["F1"])


class MalformedWidthsTests(unittest.TestCase):
    def test_malformed_widths_token_does_not_raise(self):
        # Real pdfTeX output contains the malformed token "-40.-9", which the
        # lexer yields as ("op", b"-40.-9"); float() on it used to raise
        # TypeError and kill the parity scorer.
        info = font_info("500 -40.-9 600")
        self.assertAlmostEqual(info["widths"][65], 0.5)
        self.assertAlmostEqual(info["widths"][67], 0.6)


if __name__ == "__main__":
    unittest.main()
