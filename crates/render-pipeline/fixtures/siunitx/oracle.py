#!/usr/bin/env python3
"""siunitx oracle (TEST ONLY; pdflatex never runs in the product path).

Runs MacTeX `pdflatex` on every `NN-*.tex` fixture here and writes
`reference/NN-*.json`: page, text, font, size, x, end and baseline in bp of
every maximal run of glyphs in one PDF font with no interword gap. The PDF
reader is `../font-families/oracle.py`, reused unchanged; only the segment
records here also keep `end` (x plus the advance of the last glyph), so the
test can join neighbouring segments whose PDF fonts are one Latin Modern
design (`µ` from TS1 `SFRM1000` beside `Ω` from OT1 `CMR10`).
`tests/siunitx_oracle.rs` compares the pipeline's display list with these
committed files, so cargo never needs TeX.

    python3 crates/render-pipeline/fixtures/siunitx/oracle.py [name-filter ...]
"""
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("ff_oracle", os.path.join(HERE, "..", "font-families", "oracle.py"))
ff = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ff)
ff.HERE = HERE


def segments(glyph_list, page):
    """`ff.segments`, keeping each segment's end."""
    segs = []
    cur = None
    gap = 0.0
    for g in glyph_list:
        if g is None:
            cur = None
            continue
        if g[0] == "kern":
            gap += g[1]
            if cur is not None and gap > 0.15 * cur["size"]:
                cur = None
            continue
        base, size, ch, x, y, adv = g
        if cur is None or cur["font"] != base or abs(cur["size"] - size) > 1e-6 or abs(cur["end"] - x) > 0.15 * size or abs(cur["baseline"] - y) > 0.01:
            cur = {"page": page, "text": "", "font": base, "size": size, "x": x, "baseline": y, "end": x}
            segs.append(cur)
        cur["text"] += ch
        cur["end"] = x + adv
        gap = 0.0
    for s in segs:
        s["x"] = round(s["x"], 3)
        s["end"] = round(s["end"], 3)
        s["baseline"] = round(s["baseline"], 3)
        s["size"] = round(s["size"], 4)
    return [s for s in segs if s["text"].strip()]


ff.segments = segments

if __name__ == "__main__":
    names = sorted(n[:-4] for n in os.listdir(HERE) if re.match(r"\d\d-.*\.tex$", n))
    if len(sys.argv) > 1:
        names = [n for n in names if any(a in n for a in sys.argv[1:])]
    for n in names:
        o = ff.run(n)
        fonts = sorted({s["font"] for s in o["segments"]})
        print(n, "segments", len(o["segments"]), "fonts", fonts)
