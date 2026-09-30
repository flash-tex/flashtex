#!/usr/bin/env python3
"""Write /tmp/p4f/docs/{plain,full}-N/main.tex for N in 10 100 120 300 1000 (gen.doc,
P4-L2-L3's generator; the app bench's plain-N documents are gen.doc(N, False))."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen  # noqa: E402

for n in (10, 100, 120, 300, 1000):
    for full in (False, True):
        d = f"/tmp/p4f/docs/{'full' if full else 'plain'}-{n}"
        os.makedirs(d, exist_ok=True)
        open(os.path.join(d, "main.tex"), "w").write(gen.doc(n, full))
