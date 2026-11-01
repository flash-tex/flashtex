#!/usr/bin/env python3
"""Generate the inputs of tests/pdf_images.rs, deterministically.

    python3 crates/flashtex-engine/tests/images/generate.py

Python 3 standard library only, plus TeX Live's `pdftex` for the three PDFs
made by TeX (fonts to replace; object streams). Every file is written
from code below; none is edited by hand, and the tests compare the engine
with pdfTeX on them (the expected output is pdfTeX's, never stored).

* PNG: every colour type pdfTeX handles, 1 to 16 bits, palettes with and
  without tRNS, alpha (8 and 16 bits), Adam7 interlacing, gAMA (1.0, which
  still allows the IDAT copy, and 0.45455, which does not), sRGB, pHYs, and
  IDAT split over several chunks. Scanlines use all five PNG filters.
* JPEG: baseline grey, RGB and CMYK (Adobe APP14), progressive (SOF2), JFIF
  in dots per inch and per cm, and Exif resolution. The encoder below codes
  8x8 blocks of one colour (DC coefficients only), which is a complete and
  valid JPEG.
* JBIG2: sequential and random-access files with a page-0 symbol dictionary
  that a page refers to. pdfTeX reads only the segment headers, so the
  segment data are synthetic bytes, not decodable bitmaps.
* PDF: a hand-written file covering the object syntax pdftoepdf copies
  (strings, names with #-escapes, reals, nested arrays and dictionaries,
  indirect /Length, content arrays, inherited resources, /Rotate, boxes,
  /Group, /Metadata, named destinations, a non-embedded font, an inline
  font dictionary, an /Info dictionary), a copy with a broken xref table
  (xpdf reconstructs it), a PDF 1.7 file (the version check), and pdfTeX's
  own output with embedded Type 1 subsets, with and without /CharSet, and
  with object streams.

zlib's output can differ between zlib versions, so a regeneration with
another Python may change the compressed bytes of the PNGs and PDFs; the
tests do not care, as both engines read the same committed files.
"""

import os
import struct
import subprocess
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))


def out(name, data):
    with open(os.path.join(HERE, name), "wb") as f:
        f.write(data)


# ---------------------------------------------------------------------------
# PNG
# ---------------------------------------------------------------------------

def png_chunk(t, data):
    return (struct.pack(">I", len(data)) + t + data
            + struct.pack(">I", zlib.crc32(t + data) & 0xFFFFFFFF))


CHANNELS = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}


def pack_row(pixels, depth):
    """Samples of one row (a flat list) as bytes at `depth` bits."""
    if depth == 16:
        return b"".join(struct.pack(">H", s) for s in pixels)
    if depth == 8:
        return bytes(pixels)
    out_, acc, n = bytearray(), 0, 0
    for s in pixels:
        acc = (acc << depth) | s
        n += depth
        if n == 8:
            out_.append(acc)
            acc, n = 0, 0
    if n:
        out_.append(acc << (8 - n))
    return bytes(out_)


def paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    return b if pb <= pc else c


def filter_rows(rows, bpp):
    """Each row with a filter byte, cycling through the five filters."""
    data, prev = bytearray(), bytes(len(rows[0])) if rows else b""
    for i, row in enumerate(rows):
        f = i % 5
        line = bytearray([f])
        for x, v in enumerate(row):
            a = row[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            pred = (0, a, b, (a + b) // 2, paeth(a, b, c))[f]
            line.append((v - pred) & 0xFF)
        data += line
        prev = row
    return bytes(data)


ADAM7 = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)]


def png(name, w, h, depth, ctype, pixel, *, interlace=False, palette=None, trns=None,
        gama=None, srgb=False, phys=None, idat_parts=1):
    """Write a PNG whose pixel (x, y) is the tuple `pixel(x, y)`."""
    ch = CHANNELS[ctype]
    bpp = max(1, ch * depth // 8)

    def raw(xs, ys):
        rows = []
        for y in ys:
            samples = []
            for x in xs:
                samples.extend(pixel(x, y))
            rows.append(pack_row(samples, depth))
        return filter_rows(rows, bpp) if rows and rows[0] else b""

    if interlace:
        stream = b""
        for x0, y0, dx, dy in ADAM7:
            xs, ys = list(range(x0, w, dx)), list(range(y0, h, dy))
            if xs and ys:
                stream += raw(xs, ys)
    else:
        stream = raw(range(w), range(h))
    comp = zlib.compress(stream, 9)
    body = b"\x89PNG\r\n\x1a\n"
    body += png_chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, depth, ctype, 0, 0, 1 if interlace else 0))
    if gama is not None:
        body += png_chunk(b"gAMA", struct.pack(">I", gama))
    if srgb:
        body += png_chunk(b"sRGB", b"\x00")
    if phys is not None:
        body += png_chunk(b"pHYs", struct.pack(">IIB", phys[0], phys[1], 1))
    if palette is not None:
        body += png_chunk(b"PLTE", b"".join(bytes(c) for c in palette))
    if trns is not None:
        body += png_chunk(b"tRNS", trns)
    n = len(comp)
    cuts = [n * k // idat_parts for k in range(idat_parts + 1)]
    for k in range(idat_parts):
        body += png_chunk(b"IDAT", comp[cuts[k]:cuts[k + 1]])
    body += png_chunk(b"tEXt", b"Comment\x00after the image data")
    body += png_chunk(b"IEND", b"")
    out(name, body)


def gen_pngs():
    W, H = 37, 23
    png("png-rgb8.png", W, H, 8, 2, lambda x, y: (x * 7 % 256, y * 11 % 256, (x * y) % 256),
        phys=(2835, 2835))
    png("png-gray8.png", W, H, 8, 0, lambda x, y: ((x * 5 + y * 3) % 256,), gama=100000)
    png("png-gray1.png", W, H, 1, 0, lambda x, y: ((x + y) % 2,))
    png("png-gray4.png", W, H, 4, 0, lambda x, y: ((x + 2 * y) % 16,), phys=(3937, 7874))
    pal = [((i * 37) % 256, (i * 91) % 256, (i * 13) % 256) for i in range(200)]
    png("png-pal8.png", W, H, 8, 3, lambda x, y: ((x * 3 + y) % 200,), palette=pal)
    pal16 = [((i * 16) % 256, 255 - i * 16, (i * 40) % 256) for i in range(16)]
    png("png-pal4-trns.png", W, H, 4, 3, lambda x, y: ((x + y) % 16,), palette=pal16,
        trns=bytes(range(0, 256, 32)))
    png("png-rgba8.png", W, H, 8, 6, lambda x, y: (x * 6 % 256, 255 - y * 9, 128, (x * 7 + y * 5) % 256))
    png("png-ga8.png", W, H, 8, 4, lambda x, y: ((x * 3) % 256, (y * 11) % 256))
    png("png-rgb16.png", W, H, 16, 2, lambda x, y: (x * 1771, y * 2843, (x * y * 97) % 65536))
    png("png-rgba16.png", W, H, 16, 6, lambda x, y: (x * 1771, y * 2843, 30000, (x + y) * 1100))
    png("png-rgb8-interlaced.png", W, H, 8, 2, lambda x, y: (x * 7 % 256, y * 11 % 256, 99),
        interlace=True)
    png("png-gray16-interlaced.png", W, H, 16, 0, lambda x, y: ((x * 1000 + y * 700) % 65536,),
        interlace=True)
    png("png-gamma.png", W, H, 8, 2, lambda x, y: (x * 7 % 256, 40, y * 11 % 256), gama=45455)
    png("png-srgb.png", W, H, 8, 2, lambda x, y: (200, x * 7 % 256, y * 11 % 256), srgb=True)
    png("png-multi-idat.png", W, H, 8, 2, lambda x, y: ((x ^ y) * 9 % 256, x * 3, y * 5),
        idat_parts=3)
    png("png-gray-trns.png", W, H, 8, 0, lambda x, y: ((x * 9) % 256,), trns=struct.pack(">H", 0))


# ---------------------------------------------------------------------------
# JPEG
# ---------------------------------------------------------------------------

def huff_codes(bits, vals):
    """Annex C: the code (value, length) of each symbol."""
    codes, code, k = {}, 0, 0
    for length in range(1, 17):
        for _ in range(bits[length - 1]):
            codes[vals[k]] = (code, length)
            code += 1
            k += 1
        code <<= 1
    return codes


DC_BITS = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0]
DC_VALS = list(range(12))
AC_BITS = [1] + [0] * 15  # one code: EOB
AC_VALS = [0x00]


class Bits:
    def __init__(self):
        self.out, self.acc, self.n = bytearray(), 0, 0

    def put(self, value, length):
        for i in range(length - 1, -1, -1):
            self.acc = (self.acc << 1) | ((value >> i) & 1)
            self.n += 1
            if self.n == 8:
                self.out.append(self.acc)
                if self.acc == 0xFF:
                    self.out.append(0)  # byte stuffing
                self.acc, self.n = 0, 0

    def flush(self):
        if self.n:
            self.put((1 << (8 - self.n)) - 1, 8 - self.n)
        return bytes(self.out)


def seg(marker, payload):
    return bytes([0xFF, marker]) + struct.pack(">H", len(payload) + 2) + payload


def dc_code(bw, codes, diff):
    size = abs(diff).bit_length()
    c, l = codes[size]
    bw.put(c, l)
    if size:
        bw.put(diff if diff >= 0 else diff + (1 << size) - 1, size)


def jpeg(name, w, h, ncomp, value, *, progressive=False, app=b""):
    """A JPEG whose component `k` of the 8x8 block (bx, by) is the sample
    `value(bx, by, k)` (an even number) everywhere in the block."""
    dc, ac = huff_codes(DC_BITS, DC_VALS), huff_codes(AC_BITS, AC_VALS)
    bw_, bh_ = (w + 7) // 8, (h + 7) // 8
    data = b"\xFF\xD8" + app
    data += seg(0xDB, b"\x00" + bytes([16] * 64))
    comps = b"".join(bytes([k + 1, 0x11, 0]) for k in range(ncomp))
    data += seg(0xC2 if progressive else 0xC0, struct.pack(">BHHB", 8, h, w, ncomp) + comps)
    data += seg(0xC4, b"\x00" + bytes(DC_BITS) + bytes(DC_VALS))
    data += seg(0xC4, b"\x10" + bytes(AC_BITS) + bytes(AC_VALS))

    def q(bx, by, k):
        return (value(bx, by, k) - 128) // 2  # DC 8*(v-128), quantised by 16

    if not progressive:
        data += seg(0xDA, bytes([ncomp]) + b"".join(bytes([k + 1, 0x00]) for k in range(ncomp))
                    + b"\x00\x3F\x00")
        bits, pred = Bits(), [0] * ncomp
        for by in range(bh_):
            for bx in range(bw_):
                for k in range(ncomp):
                    v = q(bx, by, k)
                    dc_code(bits, dc, v - pred[k])
                    pred[k] = v
                    bits.put(*ac[0x00])  # EOB
        data += bits.flush()
    else:
        # DC first scan, all components interleaved
        data += seg(0xDA, bytes([ncomp]) + b"".join(bytes([k + 1, 0x00]) for k in range(ncomp))
                    + b"\x00\x00\x00")
        bits, pred = Bits(), [0] * ncomp
        for by in range(bh_):
            for bx in range(bw_):
                for k in range(ncomp):
                    v = q(bx, by, k)
                    dc_code(bits, dc, v - pred[k])
                    pred[k] = v
        data += bits.flush()
        # AC first scans, one component each: every block is EOB0
        for k in range(ncomp):
            data += seg(0xDA, bytes([1, k + 1, 0x00]) + b"\x01\x3F\x00")
            bits = Bits()
            for _ in range(bw_ * bh_):
                bits.put(*ac[0x00])
            data += bits.flush()
    data += b"\xFF\xD9"
    out(name, data)


def jfif(units, xd, yd):
    return seg(0xE0, b"JFIF\x00\x01\x01" + struct.pack(">BHHBB", units, xd, yd, 0, 0))


def exif_le(xres, yres, unit):
    """APP1 Exif (little-endian TIFF) with X/YResolution and ResolutionUnit."""
    ifd_off = 8
    n = 3
    data_off = ifd_off + 2 + n * 12 + 4
    entries = struct.pack("<HHII", 282, 5, 1, data_off)
    entries += struct.pack("<HHII", 283, 5, 1, data_off + 8)
    entries += struct.pack("<HHIHH", 296, 3, 1, unit, 0)
    tiff = b"II*\x00" + struct.pack("<I", ifd_off) + struct.pack("<H", n) + entries
    tiff += struct.pack("<I", 0) + struct.pack("<II", xres, 1) + struct.pack("<II", yres, 1)
    return seg(0xE1, b"Exif\x00\x00" + tiff)


def gen_jpegs():
    jpeg("jpg-gray.jpg", 40, 24, 1, lambda bx, by, k: (bx * 40 + by * 20) % 256 & ~1,
         app=jfif(1, 72, 72))
    jpeg("jpg-rgb.jpg", 43, 21, 3, lambda bx, by, k: (60 + bx * 30 + k * 50 + by * 10) % 256 & ~1,
         app=jfif(2, 118, 118))
    jpeg("jpg-progressive.jpg", 40, 32, 3, lambda bx, by, k: (100 + bx * 20 - by * 10 + k * 30) % 256 & ~1,
         progressive=True, app=jfif(1, 150, 0))
    adobe = seg(0xEE, b"Adobe" + struct.pack(">HHHB", 100, 0, 0, 0))
    jpeg("jpg-cmyk.jpg", 24, 16, 4, lambda bx, by, k: (bx * 60 + by * 30 + k * 40) % 256 & ~1,
         app=adobe)
    jpeg("jpg-exif.jpg", 32, 16, 3, lambda bx, by, k: (128 + (bx - by) * 16 + k * 8) % 256 & ~1,
         app=exif_le(300, 300, 2))


# ---------------------------------------------------------------------------
# JBIG2 (headers exact, segment data synthetic)
# ---------------------------------------------------------------------------

def jb_seg_header(num, typ, refs, page, length):
    h = struct.pack(">IB", num, typ) + bytes([len(refs) << 5]) + bytes(refs)
    return h + bytes([page]) + struct.pack(">I", length)


def jb_segments():
    """(number, type, referred-to, page, data) in file order."""
    pageinfo = lambda w, h: struct.pack(">IIIIBH", w, h, 11811, 11811, 0, 0)
    return [
        (0, 0, [], 0, bytes(range(20))),                   # symbol dictionary, page 0
        (1, 0, [], 0, bytes(range(40, 52))),               # unreferred symbol dictionary
        (2, 48, [], 1, pageinfo(64, 32)),                  # page information
        (3, 6, [0], 1, bytes((i * 7) % 256 for i in range(30))),  # text region -> 0
        (4, 38, [], 1, bytes((i * 13) % 256 for i in range(25))),  # generic region
        (5, 49, [], 1, b""),                               # end of page
        (6, 48, [], 2, pageinfo(48, 40)),
        (7, 38, [], 2, bytes((i * 5) % 256 for i in range(17))),
        (8, 49, [], 2, b""),
        (9, 51, [], 0, b""),                               # end of file
    ]


def gen_jbig2():
    head = b"\x97JB2\r\n\x1a\n"
    segs = jb_segments()
    seq = head + b"\x01" + struct.pack(">I", 2)
    for num, typ, refs, page, data in segs:
        seq += jb_seg_header(num, typ, refs, page, len(data)) + data
    out("jbig2-sequential.jb2", seq)
    ra = head + b"\x00" + struct.pack(">I", 2)
    for num, typ, refs, page, data in segs:
        ra += jb_seg_header(num, typ, refs, page, len(data))
    for num, typ, refs, page, data in segs:
        ra += data
    out("jbig2-random.jbig2", ra)


# ---------------------------------------------------------------------------
# PDF
# ---------------------------------------------------------------------------

def pdf_file(objs, root, info=None, version="1.4", broken_xref=False):
    """A classic PDF: `objs` maps object number to its body (bytes)."""
    data = b"%PDF-" + version.encode() + b"\n%\xe2\xe3\xcf\xd3\n"
    offsets = {}
    for num in sorted(objs):
        offsets[num] = len(data)
        data += b"%d 0 obj\n" % num + objs[num] + b"\nendobj\n"
    size = max(objs) + 1
    xref = len(data)
    data += b"xref\n0 %d\n0000000000 65535 f \n" % size
    for num in range(1, size):
        off = offsets.get(num, 0) + (7 if broken_xref else 0)
        data += b"%010d 00000 n \n" % off if num in offsets else b"0000000000 65535 f \n"
    data += b"trailer\n<< /Size %d /Root %d 0 R" % (size, root)
    if info:
        data += b" /Info %d 0 R" % info
    data += b" >>\nstartxref\n%d\n%%%%EOF\n" % xref
    return data


def stream(dict_extra, body, compress=False):
    if compress:
        body = zlib.compress(body, 9)
        dict_extra = b"/Filter /FlateDecode " + dict_extra
    return b"<< " + dict_extra + b"/Length %d >>\nstream\n" % len(body) + body + b"\nendstream"


def hand_pdf():
    c1 = (b"q 0.5 0 0 RG 2 w 10 10 m 190 90 l S Q\n"
          b"/GS1 gs /Fm1 Do BT /F1 12 Tf 20 40 Td (Hand \\(made\\)) Tj ET\n"
          b"BT /F2 9 Tf 20 60 Td (inline font) Tj ET\n")
    c2a = b"q 0 0 1 rg 0 0 50 50 re f Q\n"
    c2b = b"q 1 0 0 rg 50 50 50 50 re f Q\n/Im0 Do\n"
    objs = {
        1: b"<< /Type /Catalog /Pages 2 0 R /Dests 30 0 R /Metadata 20 0 R >>",
        # inherited: MediaBox, Resources (page 3 uses them), Rotate
        2: (b"<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 /MediaBox [0 0 200 100] "
            b"/Resources << /ProcSet [/PDF /Text] /Font << /F1 10 0 R >> >> >>"),
        3: (b"<< /Type /Page /Parent 2 0 R /CropBox [5 5.5 195 95.25] /BleedBox [1 1 199 99] "
            b"/TrimBox [2 2 198 98] /ArtBox [3 3 197.125 97] "
            b"/Contents 6 0 R /LastModified (D:20260101000000Z) "
            b"/PieceInfo << /FlashTeX << /Private (x\\051y) /LastModified (D:2026) >> >> "
            b"/Resources << /ProcSet [/PDF /Text /ImageB] /Font << /F1 10 0 R "
            b"/F2 << /Type /Font /Subtype /Type1 /BaseFont /Courier >> >> "
            b"/ExtGState << /GS1 << /Type /ExtGState /CA 0.333333333 /ca 1e-7 /LW 2.50000049 >> >> "
            b"/XObject << /Fm1 11 0 R >> "
            b"/Properties << /MC0 << /S (esc \\\\ \\( \\) \\n \\377 end) /H <00ff10> "
            b"/N /A#20B#2Fc /R [ -12.5 100000.25 0.000001 3 [true false null] << /K /V >> ] >> >> "
            b"/Shading 12 0 R >> >>"),
        4: (b"<< /Type /Page /Parent 2 0 R /Rotate 90 /Contents [7 0 R 8 0 R] "
            b"/Group << /S /Transparency /CS /DeviceRGB /I true >> /Metadata 20 0 R "
            b"/Resources << /XObject << /Im0 13 0 R >> /ColorSpace << /CS0 [/ICCBased 14 0 R] >> "
            b"/Subtype /NotAName >> >>"),
        5: b"<< /Type /Page /Parent 2 0 R /Rotate 270 /SeparationInfo << /Pages [3 0 R] >> >>",
        6: stream(b"", c1).replace(b"/Length %d" % len(c1), b"/Length 9 0 R"),
        7: stream(b"", c2a, compress=True),
        8: stream(b"", c2b),
        9: b"%d" % len(c1),
        10: b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        11: stream(b"/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << /ExtGState << /G 15 0 R >> >> ",
                   b"0 0 10 10 re f\n", compress=True),
        12: b"<< /Sh1 << /ShadingType 2 /ColorSpace /DeviceGray /Coords [0 0 1 0] /Function 16 0 R >> >>",
        13: stream(b"/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceGray "
                   b"/BitsPerComponent 8 ", b"\x00\x80\xc0\xff"),
        14: stream(b"/N 1 ", b"not really a profile"),
        15: b"<< /Type /ExtGState /LW 0.1 >>",
        16: b"<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>",
        20: stream(b"/Type /Metadata /Subtype /XML ", b"<?xpacket?><x:xmpmeta/>"),
        21: b"<< /Title (Hand-made test PDF) /Producer (generate.py) >>",
        30: b"<< /pagetwo [4 0 R /Fit] /pageone [3 0 R /XYZ 0 0 0] >>",
    }
    return objs


def pdftex(name, body):
    """Typeset `body` with plain TeX into `name` (TeX Live's pdftex)."""
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        with open(os.path.join(d, "doc.tex"), "w") as f:
            f.write(body)
        env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
        subprocess.run(["pdftex", "-interaction=batchmode", "doc.tex"], cwd=d, env=env, check=True,
                       stdout=subprocess.DEVNULL)
        with open(os.path.join(d, "doc.pdf"), "rb") as f:
            out(name, f.read())


def otf_cff(name):
    """The 'CFF ' table of TeX Live's OpenType font `name` (a bare CFF
    font program, what PDF calls /Type1C)."""
    path = subprocess.run(["kpsewhich", name], capture_output=True, text=True, check=True).stdout.strip()
    data = open(path, "rb").read()
    num = struct.unpack(">H", data[4:6])[0]
    for i in range(num):
        tag, _, off, length = struct.unpack(">4sIII", data[12 + 16 * i:28 + 16 * i])
        if tag == b"CFF ":
            return data[off:off + length]
    raise SystemExit(f"{name}: no CFF table")


def type1c_pdf():
    """Two Type 1C fonts that pdfTeX replaces through the font map: one with
    a /CharSet (subset), one without (whole font)."""
    content = (b"BT /F1 24 Tf 10 50 Td (Hello, CFF) Tj ET\n"
               b"BT /F2 12 Tf 10 20 Td (Bold, whole font) Tj ET\n")
    objs = {
        1: b"<< /Type /Catalog /Pages 2 0 R >>",
        2: b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        3: (b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 80] /Contents 4 0 R "
            b"/Resources << /Font << /F1 5 0 R /F2 8 0 R >> /ProcSet [/PDF /Text] >> >>"),
        4: stream(b"", content, compress=True),
        5: (b"<< /Type /Font /Subtype /Type1 /BaseFont /LMRoman10-Regular /FirstChar 32 "
            b"/LastChar 126 /Widths [" + b" 500" * 95 + b"] /Encoding /WinAnsiEncoding "
            b"/FontDescriptor 6 0 R >>"),
        6: (b"<< /Type /FontDescriptor /FontName /LMRoman10-Regular /Flags 34 "
            b"/FontBBox [-430 -290 1417 1127] /ItalicAngle 0 /Ascent 1127 /Descent -290 "
            b"/CapHeight 683 /StemV 69.5 /CharSet (/H/e/l/o/comma/space/C/F) /FontFile3 7 0 R >>"),
        7: stream(b"/Subtype /Type1C ", otf_cff("lmroman10-regular.otf"), compress=True),
        8: (b"<< /Type /Font /Subtype /Type1 /BaseFont /LMRoman10-Bold /FirstChar 32 "
            b"/LastChar 126 /Widths [" + b" 575" * 95 + b"] "
            b"/Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [65 /B /o] >> "
            b"/FontDescriptor 9 0 R >>"),
        9: (b"<< /Type /FontDescriptor /FontName /LMRoman10-Bold /Flags 262178 "
            b"/FontBBox [-480 -290 1535 1147] /ItalicAngle 0 /Ascent 1147 /Descent -290 "
            b"/CapHeight 686 /StemV 114 /FontFile3 10 0 R >>"),
        10: stream(b"/Subtype /Type1C ", otf_cff("lmroman10-bold.otf"), compress=True),
    }
    return pdf_file(objs, 1)


def gen_pdfs():
    out("pdf-type1c.pdf", type1c_pdf())
    objs = hand_pdf()
    out("pdf-hand.pdf", pdf_file(objs, 1, info=21))
    out("pdf-broken-xref.pdf", pdf_file(objs, 1, info=21, broken_xref=True))
    out("pdf-17.pdf", pdf_file(objs, 1, version="1.7"))
    setup = ("\\pdfoutput=1 \\pdfsuppressptexinfo=-1 \\pdftrailerid{} \\pdfpagewidth=3in "
             "\\pdfpageheight=2in \\hsize=2.5in \\parindent=0pt \\pdfhorigin=.25in \\pdfvorigin=.25in ")
    pdftex("pdf-fonts.pdf", setup + "\\pdfcompresslevel=9 \\pdfobjcompresslevel=0 "
           "First page, {\\bf bold} and $x^2+\\alpha$.\\vfill\\eject "
           "Second page {\\it italic} shares fonts.\\bye\n")
    pdftex("pdf-fonts-whole.pdf", setup + "\\pdfomitcharset=1 \\pdfmapline{=cmtt10 CMTT10 <cmtt10.pfb}"
           "\\pdfcompresslevel=0 {\\tt Whole typewriter font} and roman.\\bye\n")
    pdftex("pdf-objstm.pdf", setup + "\\pdfminorversion=5 \\pdfobjcompresslevel=2 "
           "Object streams: \\TeX\\ $\\sum_i y_i$.\\vfill\\eject Page two.\\bye\n")


if __name__ == "__main__":
    gen_pngs()
    gen_jpegs()
    gen_jbig2()
    if "--no-tex" not in sys.argv:
        gen_pdfs()
