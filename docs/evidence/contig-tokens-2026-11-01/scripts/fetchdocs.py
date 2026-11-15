#!/usr/bin/env python3
"""Print DIR:ENTRY for the 13 arXiv papers P6-HYPEROPT measured (every 12th id of arxiv-2025-01,
sorted; docs/evidence/p6-hyperopt-2026-10-04 raw/cold-arxiv-base.jsonl), fetched and checked against
their pinned SHA-256 by tools/parity/corpus.py.

usage: fetchdocs.py CACHE [K/N]   (K/N: only every N-th paper from the K-th, for sharding)"""
import json, os, sys

W = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..', '..'))
sys.path.insert(0, os.path.join(W, 'tools', 'parity'))
import corpus  # noqa: E402

man = os.path.join(W, 'tools', 'parity', 'corpus', 'arxiv-2025-01.json')
ids = sorted(e['id'] for e in json.load(open(man))['entries'])[::12]
assert len(ids) == 13, ids
if len(sys.argv) > 2:
    k, n = map(int, sys.argv[2].split('/'))
    ids = ids[k::n]
docs = corpus.fetch_manifest(man, sys.argv[1], only={corpus.safe_id(i) for i in ids},
                             log=lambda *m: print(*m, file=sys.stderr))
bad = 0
for d in docs:
    if d['problem']:
        bad += 1
        print(f"{d['id']}: {d['problem']}", file=sys.stderr)
    else:
        print(f"{d['dir']}:{d['entry']}")
sys.exit(1 if bad else 0)
