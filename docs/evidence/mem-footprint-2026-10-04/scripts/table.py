#!/usr/bin/env python3
"""table.py TAG... : one row per (tag, doc) from /tmp/mfp/out."""
import glob, json, os, re, sys
rows = []
for tag in sys.argv[1:]:
    for f in sorted(glob.glob(f'/tmp/mfp/out/{tag}-*.jsonl')):
        doc = f[len(f'/tmp/mfp/out/{tag}-'):-6]
        mem = {}
        ed = []
        ins = []
        for l in open(f):
            r = json.loads(l)
            m = (r.get('host') or {}).get('mem')
            if m:
                mem = m
            if r.get('phase', '').startswith('edit') and isinstance(r.get('edited_page_ms'), (int, float)):
                ed.append(r['edited_page_ms'])
                st = (r.get('host') or {}).get('stages') or {}
                if st.get('edited_instr_k') is not None:
                    ins.append(st['edited_instr_k'] / 1e3)
        vm = open(f[:-6] + '.vmmap').read() if os.path.exists(f[:-6] + '.vmmap') else ''
        fp = re.search(r'Physical footprint:\s+(\S+)', vm)
        fpp = re.search(r'Physical footprint \(peak\):\s+(\S+)', vm)
        ed.sort(); ins.sort()
        qi = lambda p: round(ins[min(len(ins) - 1, int(p * len(ins)))], 1) if ins else None
        q = lambda p: round(ed[min(len(ed) - 1, int(p * len(ed)))], 1) if ed else None
        M = lambda k: round(mem.get(k, 0) / 1e6, 1)
        print(f"{tag:10} {doc:10} fp {fp.group(1) if fp else '?':>7} peak {fpp.group(1) if fpp else '?':>7} "
              f"malloc {M('malloc_in_use'):7} held {M('malloc_held'):7} logs {M('sealed_bytes'):6} "
              f"ckpts {mem.get('checkpoints')} edited p50/p95 {q(0.5)}/{q(0.95)} ms, {qi(0.5)}/{qi(0.95)} Minstr n={len(ed)}")
