#!/usr/bin/env python3
"""Side-by-side thumbnail sheet (reference | candidate | diff) from two
same-size 8-bit PGM rasters, written as RGB PNG with the standard library.

The diff panel paints ink present only in the candidate red, ink present only
in the reference blue, ink on both sides grey; `scale` is the integer box
downsample (3 at 144 dpi gives 48 dpi thumbnails). Evidence only.
"""

import struct
import zlib

import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "real-world-corpus"))
from run import read_pgm  # noqa: E402

INK = 128  # grey level below which a pixel counts as ink


def downsample(w, h, px, scale):
    """Box-average an 8-bit grey buffer by an integer factor."""
    ow, oh = w // scale, h // scale
    out = bytearray(ow * oh)
    n = scale * scale
    for oy in range(oh):
        rows = [px[(oy * scale + k) * w:(oy * scale + k + 1) * w] for k in range(scale)]
        base = oy * ow
        for ox in range(ow):
            x0 = ox * scale
            s = 0
            for r in rows:
                s += sum(r[x0:x0 + scale])
            out[base + ox] = s // n
    return ow, oh, bytes(out)


def diff_rgb(w, h, ref, cand):
    """RGB bytes: red = candidate-only ink, blue = reference-only ink, grey = both."""
    out = bytearray(w * h * 3)
    for i in range(w * h):
        r_ink, c_ink = ref[i] < INK, cand[i] < INK
        if r_ink and c_ink:
            v = min(ref[i], cand[i])
            out[3 * i:3 * i + 3] = bytes((v, v, v))
        elif c_ink:
            out[3 * i:3 * i + 3] = bytes((220, 30, 30))
        elif r_ink:
            out[3 * i:3 * i + 3] = bytes((30, 60, 220))
        else:
            out[3 * i:3 * i + 3] = b"\xff\xff\xff"
    return bytes(out)


def write_png(path, w, h, rgb):
    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + rgb[y * w * 3:(y + 1) * w * 3] for y in range(h))
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(raw, 6)))
        f.write(chunk(b"IEND", b""))


def sheet(ref_pgm, cand_pgm, out_png, scale=3, gutter=8):
    rw, rh, rp = read_pgm(ref_pgm)
    cw, ch, cp = read_pgm(cand_pgm)
    if (rw, rh) != (cw, ch):
        raise ValueError(f"size mismatch {rw}x{rh} vs {cw}x{ch}")
    # diff at full resolution, then downsample each channel
    w, h = rw, rh
    full = diff_rgb(w, h, rp, cp)
    ow, oh, ref_s = downsample(w, h, rp, scale)
    _, _, cand_s = downsample(w, h, cp, scale)
    chans = [downsample(w, h, full[k::3], scale)[2] for k in range(3)]
    sw = ow * 3 + gutter * 2
    rgb = bytearray(b"\x80" * (sw * oh * 3))
    for y in range(oh):
        row = y * sw * 3
        for x in range(ow):
            v = ref_s[y * ow + x]
            rgb[row + 3 * x:row + 3 * x + 3] = bytes((v, v, v))
            v = cand_s[y * ow + x]
            o = row + 3 * (ow + gutter + x)
            rgb[o:o + 3] = bytes((v, v, v))
            o = row + 3 * (2 * ow + 2 * gutter + x)
            rgb[o:o + 3] = bytes((chans[0][y * ow + x], chans[1][y * ow + x], chans[2][y * ow + x]))
    write_png(out_png, sw, oh, bytes(rgb))
    return sw, oh


if __name__ == "__main__":  # pragma: no cover
    print(sheet(sys.argv[1], sys.argv[2], sys.argv[3], scale=int(sys.argv[4]) if len(sys.argv) > 4 else 3))
