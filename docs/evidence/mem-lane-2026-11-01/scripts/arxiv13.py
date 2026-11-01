#!/usr/bin/env python3
"""arxiv13.py: fetch (verified, cached) every 12th paper of arxiv-2025-01 (P6-HYPEROPT's 13) and print
DIR<TAB>ENTRY<TAB>ID per paper (lane MEMORY)."""
import os
import sys
W = os.path.abspath(os.path.join(os.path.dirname(__file__), '../../../..'))
sys.path.insert(0, os.path.join(W, 'tools', 'parity'))
import corpus  # noqa: E402
m = os.path.join(W, 'tools', 'parity', 'corpus', 'arxiv-2025-01.json')
ids = corpus.manifest_ids(m)[0::12]
only = set(ids) | {corpus.safe_id(i) for i in ids}
for d in corpus.fetch_manifest(m, corpus.default_cache(), only=only, log=lambda *x: print(*x, file=sys.stderr)):
    if not d['problem']:
        print(f"{d['dir']}\t{d['entry']}\t{d['id']}")
    else:
        print(f"skip {d['id']}: {d['problem']}", file=sys.stderr)
