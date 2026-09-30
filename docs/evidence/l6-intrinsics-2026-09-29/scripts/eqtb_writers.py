#!/usr/bin/env python3
"""List the generated routines that write eqtb or xeq_level, and how often.

The guarded intrinsics' watch (crates/flashtex-engine/src/intrinsics.rs) must
see every write to a watched eqtb entry; this lists the write sites so that
each one is either a hooked routine (eq_define, geq_define, eq_word_define,
geq_word_define, unsave's restore) or provably outside a recorded or watched
execution. usage: eqtb_writers.py [GENERATED_DIR]"""
import glob
import os
import re
import sys

d = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
    os.path.dirname(__file__), '../../../../crates/flashtex-engine/src/generated')
pat = re.compile(r'self\.(eqtb|xeq_level)\[[^\]]*\]\.set_|self\.(eqtb|xeq_level)\[[^\]]*\] = |self\.(eqtb|xeq_level)\[\(__ix')
counts = {}
for f in sorted(glob.glob(os.path.join(d, 'body_*.rs')) + [os.path.join(d, 'main_body.rs')]):
    fn = '?'
    for line in open(f):
        m = re.search(r'pub fn ([a-z_0-9]+)', line)
        if m:
            fn = m.group(1)
        if pat.search(line):
            k = (fn, 'xeq_level' if 'xeq_level' in line else 'eqtb')
            counts[k] = counts.get(k, 0) + 1
for (fn, arr), n in sorted(counts.items()):
    print(f'{n:4} {arr:9} {fn}')
