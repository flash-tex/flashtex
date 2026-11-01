#!/usr/bin/env python3
"""Would another deflate implementation give pdfTeX's bytes?

usage: zlib_identity.py PDF [--level 9] [--dump DIR]

Takes every FlateDecode stream of PDF (written by pdfTeX's zlib, TeX Live's
1.3.x at \\pdfcompresslevel), inflates it, deflates it again with the zlib
Python links (on macOS the system's libz) at the same level, windowBits and
memLevel as writezip.c's deflateInit, and counts streams whose bytes come out
identical. --dump writes each inflated stream to DIR/N.raw and its pdfTeX
bytes to DIR/N.z, for other implementations (zlib_identity_rs/)."""
import argparse
import os
import re
import zlib

ap = argparse.ArgumentParser()
ap.add_argument('pdf')
ap.add_argument('--level', type=int, default=9)
ap.add_argument('--dump')
a = ap.parse_args()

data = open(a.pdf, 'rb').read()
same = diff = 0
nbytes = 0
if a.dump:
    os.makedirs(a.dump, exist_ok=True)
for i, m in enumerate(re.finditer(rb'/Filter\s*/FlateDecode[^>]*>>\s*stream\r?\n', data)):
    start = m.end()
    d = zlib.decompressobj()
    raw = d.decompress(data[start:])
    used = len(data) - start - len(d.unused_data)
    z = data[start:start + used]
    c = zlib.compressobj(a.level, zlib.DEFLATED, 15, 8)
    z2 = c.compress(raw) + c.flush()
    if z2 == z:
        same += 1
    else:
        diff += 1
    nbytes += len(raw)
    if a.dump:
        open(f'{a.dump}/{i}.raw', 'wb').write(raw)
        open(f'{a.dump}/{i}.z', 'wb').write(z)
print(f'{a.pdf}: {same + diff} streams ({nbytes} bytes inflated); python zlib {zlib.ZLIB_RUNTIME_VERSION} '
      f'level {a.level}: {same} identical, {diff} different')
