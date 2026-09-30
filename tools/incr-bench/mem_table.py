#!/usr/bin/env python3
"""mem_table.py FILE...: a Markdown table of mem.py's summary lines (lane P4-MEMORY): peak RSS and,
from the last keystroke's `mem`, where the memory is. MB = 2^20 bytes. Heap columns need a
`mem-stats` build; the others come from any build with FLASHTEX_MEMSTAT=1."""
import json
import sys

rows = []
for path in sys.argv[1:]:
    for line in open(path):
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        if d.get('summary') is True:
            rows.append(d)


def mb(v):
    return '' if v is None else f'{v / 2**20:.0f}'


cols = [
    ('peak RSS', lambda d, m: d.get('peak_rss')),
    ('RSS end', lambda d, m: m.get('rss')),
    ('heap peak', lambda d, m: m.get('heap_peak')),
    ('side (heap)', lambda d, m: m.get('heap_side')),
    ('logs', lambda d, m: m.get('sealed_bytes')),
    ('slab res.', lambda d, m: m.get('slab_resident')),
    ('space res.', lambda d, m: m.get('space_resident')),
    ('records (heap)', lambda d, m: m.get('heap_record')),
    ('engine (heap)', lambda d, m: m.get('heap_engine')),
    ('branch (heap)', lambda d, m: m.get('heap_branch')),
    ('cow (heap)', lambda d, m: m.get('heap_cow')),
    ('dl+pages (heap)', lambda d, m: m.get('heap_dl')),
    ('page cache', lambda d, m: m.get('page_cache')),
]
print('| doc | tag | edits | ckpts | ' + ' | '.join(c for c, _ in cols) + ' | edited p50/p95 ms |')
print('|' + '---|' * (len(cols) + 5))
for d in rows:
    m = d.get('last') or {}
    lat = ''
    if d.get('edited_p50') is not None:
        lat = f"{d['edited_p50']:.1f} / {d['edited_p95']:.1f}"
    kill = ' (killed)' if d.get('killed_at_limit') else ''
    print(f"| {d['doc']}{kill} | {d['tag']} | {d['keys']} | {m.get('checkpoints', '')} | "
          + ' | '.join(mb(f(d, m)) for _, f in cols) + f' | {lat} |')
