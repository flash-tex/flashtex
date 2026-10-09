"""Rasterise PDF pages with Core Graphics, as the app's preview gate does.

The configuration is `DL3Renderer.bitmapContext` + `rasterize(pdfPage:)`
(apps/mac/Sources/FlashTeXPreviewV3/DL3Renderer.swift), which
PreviewParityTests uses for its reference PDF: an 8-bit RGBA bitmap,
premultiplied-last, sRGB, `ceil(width × scale)` × `ceil(height × scale)`
pixels, filled white, scaled, antialiasing on, font smoothing off,
subpixel positioning and subpixel-positioned fonts on, translated by the
MediaBox origin, then `CGContextDrawPDFPage`. Both PDFs go through this one
function, so a difference is a difference in the PDFs.

The pixel diff is done with Pillow (no numpy here): a pixel differs when any
of its four bytes does.
"""
import math
import os
import threading

import Quartz
from PIL import Image, ImageChops

# pyobjc resolves Quartz's names lazily, and two threads resolving at once
# can fail (measured: KeyError 'CGContextSetShouldSmoothFonts' with 4 jobs).
# Resolve every name used here once, at import, under one lock.
_NAMES = ("CFURLCreateFromFileSystemRepresentation", "CGPDFDocumentCreateWithURL",
          "CGPDFDocumentGetNumberOfPages", "CGPDFDocumentGetPage", "CGPDFPageGetBoxRect",
          "kCGPDFMediaBox", "CGColorSpaceCreateWithName", "kCGColorSpaceSRGB",
          "CGBitmapContextCreate", "kCGImageAlphaPremultipliedLast", "CGContextSetFillColorWithColor",
          "CGColorCreateGenericGray", "CGContextFillRect", "CGRectMake", "CGContextScaleCTM",
          "CGContextSetShouldAntialias", "CGContextSetAllowsFontSmoothing", "CGContextSetShouldSmoothFonts",
          "CGContextSetAllowsFontSubpixelPositioning", "CGContextSetShouldSubpixelPositionFonts",
          "CGContextTranslateCTM", "CGContextDrawPDFPage", "CGContextFlush")
_LOCK = threading.Lock()
with _LOCK:
    for _n in _NAMES:
        getattr(Quartz, _n)


def open_pdf(path):
    url = Quartz.CFURLCreateFromFileSystemRepresentation(None, os.fsencode(path), len(os.fsencode(path)), False)
    doc = Quartz.CGPDFDocumentCreateWithURL(url)
    if doc is None:
        raise RuntimeError("Core Graphics cannot open %s" % path)
    return doc


def page_count(doc):
    return Quartz.CGPDFDocumentGetNumberOfPages(doc)


def render(doc, index, scale=2.0, smooth=False):
    """Page `index` (0-based) as a Pillow RGBA image (premultiplied bytes;
    opaque on a white ground, so the same as straight alpha)."""
    page = Quartz.CGPDFDocumentGetPage(doc, index + 1)
    box = Quartz.CGPDFPageGetBoxRect(page, Quartz.kCGPDFMediaBox)
    w = int(math.ceil(box.size.width * scale))
    h = int(math.ceil(box.size.height * scale))
    buf = bytearray(w * h * 4)
    cs = Quartz.CGColorSpaceCreateWithName(Quartz.kCGColorSpaceSRGB)
    ctx = Quartz.CGBitmapContextCreate(buf, w, h, 8, w * 4, cs, Quartz.kCGImageAlphaPremultipliedLast)
    if ctx is None:
        raise RuntimeError("no bitmap context %dx%d" % (w, h))
    Quartz.CGContextSetFillColorWithColor(ctx, Quartz.CGColorCreateGenericGray(1.0, 1.0))
    Quartz.CGContextFillRect(ctx, Quartz.CGRectMake(0, 0, w, h))
    Quartz.CGContextScaleCTM(ctx, scale, scale)
    Quartz.CGContextSetShouldAntialias(ctx, True)
    if smooth:
        Quartz.CGContextSetAllowsFontSmoothing(ctx, True)
    Quartz.CGContextSetShouldSmoothFonts(ctx, smooth)
    Quartz.CGContextSetAllowsFontSubpixelPositioning(ctx, True)
    Quartz.CGContextSetShouldSubpixelPositionFonts(ctx, True)
    Quartz.CGContextTranslateCTM(ctx, -box.origin.x, -box.origin.y)
    Quartz.CGContextDrawPDFPage(ctx, page)
    Quartz.CGContextFlush(ctx)
    del ctx
    # Row 0 of the buffer is the top of the page (bitmap contexts are top-down in memory).
    return Image.frombuffer("RGBA", (w, h), bytes(buf), "raw", "RGBA", 0, 1)


def diff(a, b):
    """(differing pixels, bbox (x0, y0, x1, y1) or None, max channel delta, mask)."""
    if a.size != b.size:
        return max(a.size[0] * a.size[1], b.size[0] * b.size[1]), None, 255, None
    if a.tobytes() == b.tobytes():
        return 0, None, 0, None
    d = ImageChops.difference(a, b)
    r, g, bl, al = d.split()
    m = ImageChops.lighter(ImageChops.lighter(r, g), ImageChops.lighter(bl, al))
    maxd = m.getextrema()[1]
    mask = m.point(lambda v: 255 if v else 0)
    n = mask.histogram()[255]
    return n, mask.getbbox(), maxd, mask


def diff_image(a, b, mask):
    """The reference, faded, with the differing pixels in red."""
    base = Image.blend(a.convert("RGB"), Image.new("RGB", a.size, "white"), 0.75)
    red = Image.new("RGB", a.size, (230, 0, 0))
    base.paste(red, mask=mask)
    return base


def compare_pdfs(ref, cand, scale=2.0, smooth=False, diff_dir=None, tag="page"):
    """Per page: {"page", "pixels", "bbox", "max_delta", "size"} (pages of
    both PDFs only; a page count difference is the structural report's)."""
    da, db = open_pdf(ref), open_pdf(cand)
    out = []
    for i in range(min(page_count(da), page_count(db))):
        a = render(da, i, scale, smooth)
        b = render(db, i, scale, smooth)
        n, bbox, maxd, mask = diff(a, b)
        row = {"page": i + 1, "pixels": n, "bbox": list(bbox) if bbox else None,
               "max_delta": maxd, "size": [a.size, b.size]}
        if diff_dir and n and mask is not None:
            os.makedirs(diff_dir, exist_ok=True)
            p = os.path.join(diff_dir, "%s-p%d-%gx.png" % (tag, i + 1, scale))
            diff_image(a, b, mask).save(p)
            a.save(p[:-4] + "-ref.png")
            b.save(p[:-4] + "-cand.png")
            row["diff_png"] = p
        out.append(row)
    return out
