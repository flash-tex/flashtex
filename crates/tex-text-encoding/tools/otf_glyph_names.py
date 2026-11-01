#!/usr/bin/env python3
"""List glyph names of the bundled Latin Modern OTFs (apps/mac/Fonts) for KC-105 tests.

Requires fontTools (tooling only, e.g. in a scratch venv).  Output:
tests/oracle/otf-glyph-names.json  {file: {"sha256": ..., "glyphs": [...]}}
"""
import hashlib
import json
import os
import sys

from fontTools.ttLib import TTFont

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)
FONTS = os.path.join(CRATE, "..", "..", "apps", "mac", "Fonts")
FILES = ["lmroman10-regular.otf", "lmroman5-regular.otf"]


def main():
    out = {}
    for name in FILES:
        path = os.path.join(FONTS, name)
        with open(path, "rb") as fh:
            digest = hashlib.sha256(fh.read()).hexdigest()
        font = TTFont(path)
        out[name] = {"sha256": digest, "glyphs": sorted(font.getGlyphOrder())}
    dest = os.path.join(CRATE, "tests", "oracle", "otf-glyph-names.json")
    with open(dest, "w") as fh:
        json.dump(out, fh, indent=0, sort_keys=True)
        fh.write("\n")
    print("wrote", dest, file=sys.stderr)


if __name__ == "__main__":
    main()
