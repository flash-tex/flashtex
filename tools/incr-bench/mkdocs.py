#!/usr/bin/env python3
"""mkdocs.py: the generated benchmark documents, under INCR_BENCH_DIR (default /tmp/incr-bench):
src-DOC/DOC.tex (for incr_bench.py) and docs/DOC/main.tex (for the socket drivers), for DOC in
plain-N and full-N (N = 10, 100, 120, 300, 1000; gen.doc, P4-L2-L3's generator: the app's bench
documents are gen.doc(N, False)), full-100t (gen.titled: front matter after the last package), src-refs-30, src-refs-120 (genrefs.py), and the beamer decks
docs/beamer-5 and docs/beamer-30 with their edit locations (genbeamer.py)."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen  # noqa: E402
import genbeamer  # noqa: E402
import genrefs  # noqa: E402

BASE = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
for n in (10, 100, 120, 300, 1000):
    for full in (False, True):
        doc = f"{'full' if full else 'plain'}-{n}"
        text = gen.doc(n, full)
        for d, name in ((f'{BASE}/src-{doc}', f'{doc}.tex'), (f'{BASE}/docs/{doc}', 'main.tex')):
            os.makedirs(d, exist_ok=True)
            open(os.path.join(d, name), 'w').write(text)
# full-100 with front matter right after its last package (`gen.titled`: sound-pre's mid-line restarts)
for d, name in ((f'{BASE}/src-full-100t', 'full-100t.tex'), (f'{BASE}/docs/full-100t', 'main.tex')):
    os.makedirs(d, exist_ok=True)
    open(os.path.join(d, name), 'w').write(gen.titled(100, True))
sys.argv = ['genrefs.py', BASE]
genrefs.main()
sys.argv = ['genbeamer.py', BASE]
genbeamer.main()
