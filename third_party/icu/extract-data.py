#!/usr/bin/env python3
"""Extract the ICU data items FlashTeX's XeTeX port uses from TeX Live's
`libs/icu/icu-src/source/data/in/icudt78l.dat`, byte for byte.

The .dat is ICU's "common data" package (`CmnD`, format 1): a header, then a
table of contents of (name offset, data offset) pairs, then the items. This
script reads the table of contents and writes each selected item, unchanged,
to `icudt78l/<name>` next to this script; it is what `icupkg -x` does.
`third_party/icu/README.md` lists the items and why; SHA256SUMS pins them.

    python3 third_party/icu/extract-data.py <texlive-source>/libs/icu/icu-src/source/data/in/icudt78l.dat
"""

import fnmatch
import os
import struct
import sys

# The items: the converter alias table (so that every alias of ICU's
# built-in, algorithmic converters resolves) and the whole break-iteration
# tree (line breaking for every locale).
ITEMS = ["icudt78l/cnvalias.icu", "icudt78l/brkitr/*"]


def toc(data):
    header_size, magic1, magic2 = struct.unpack_from("<HBB", data, 0)
    assert (magic1, magic2) == (0xDA, 0x27), "not ICU data"
    assert data[8] == 0, "not little-endian"
    assert data[12:16] == b"CmnD" and data[16] == 1, "not a common-data package"
    count, = struct.unpack_from("<I", data, header_size)
    entries = []
    for i in range(count):
        name_off, data_off = struct.unpack_from("<II", data, header_size + 4 + 8 * i)
        start = header_size + name_off
        name = data[start:data.index(b"\0", start)].decode("ascii")
        entries.append((name, header_size + data_off))
    out = []
    # An item runs to the next item's start (items are padded to 16
    # bytes); `icupkg` reads and extracts items the same way.
    for i, (name, off) in enumerate(entries):
        end = entries[i + 1][1] if i + 1 < count else len(data)
        out.append((name, data[off:end]))
    return out


def main():
    dat = open(sys.argv[1], "rb").read()
    here = os.path.dirname(os.path.abspath(__file__))
    n = 0
    for name, blob in toc(dat):
        if any(fnmatch.fnmatchcase(name, p) for p in ITEMS):
            path = os.path.join(here, name)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "wb") as f:
                f.write(blob)
            n += 1
    print(f"{n} items")


if __name__ == "__main__":
    main()
