#!/usr/bin/env python3
"""Launch -> first page from a FlashTeXMac bench log (FLASHTEX_LOG).

usage: startup.py <app.log> [...]
Uses the `launch: process started at <ns>` line (TypingBench.install, monotonic
clock) and the first `preview-v2: blit page` / `paint: revision` stamps.
Also reports the first compile result applied and the first v2 frame published.
"""
import re
import sys

for path in sys.argv[1:]:
    start = first_blit = first_paint = published = applied = None
    stall = None
    for line in open(path, errors="replace"):
        msg = line.split("\t", 1)[-1]
        m = re.search(r"launch: process started at (\d+)", msg)
        if m and start is None:
            start = int(m.group(1))
        m = re.search(r"preview-v2: blit page \d+ \S+ at (\d+)", msg)
        if m and first_blit is None:
            first_blit = int(m.group(1))
        m = re.search(r"^paint: revision \d+ at (\d+)", msg)
        if m and first_paint is None:
            first_paint = int(m.group(1))
        m = re.search(r"preview-v2: published live .* at (\d+)", msg)
        if m and published is None:
            published = int(m.group(1))
        m = re.search(r"compile: applied revision \d+ at (\d+)", msg)
        if m and applied is None:
            applied = int(m.group(1))
        if "first-paint stall" in msg:
            stall = True

    def ms(t):
        return "-" if t is None or start is None else f"{(t - start) / 1e6:.0f} ms"
    print(f"{path}: first live frame published {ms(published)}, first page blit {ms(first_blit)}, first paint {ms(first_paint)}"
          + (" [first-paint stall logged]" if stall else ""))
