#!/usr/bin/env python3
"""mkdocs.py: the generated benchmark documents, under INCR_BENCH_DIR (default /tmp/incr-bench):
src-DOC/DOC.tex (for incr_bench.py) and docs/DOC/main.tex (for the socket drivers), for DOC in
plain-N and full-N (N = 10, 100, 120, 300, 1000; gen.doc, P4-L2-L3's generator: the app's bench
documents are gen.doc(N, False)), and src-refs-30, src-refs-120 (genrefs.py)."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen  # noqa: E402
import genrefs  # noqa: E402

BASE = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
for n in (10, 100, 120, 300, 1000):
    for full in (False, True):
        doc = f"{'full' if full else 'plain'}-{n}"
        text = gen.doc(n, full)
        for d, name in ((f'{BASE}/src-{doc}', f'{doc}.tex'), (f'{BASE}/docs/{doc}', 'main.tex')):
            os.makedirs(d, exist_ok=True)
            open(os.path.join(d, name), 'w').write(text)
sys.argv = ['genrefs.py', BASE]
genrefs.main()
