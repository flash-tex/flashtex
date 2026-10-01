#!/usr/bin/env python3
"""restore_parts.py FILE.jsonl...: where a keystroke's restore goes (review of #1300, item 3). For
mem.py runs made with FLASHTEX_INCR_DEBUG=1: DONE's stages per keystroke (restore, find, the
edited page) and, from the host's stderr next to each file, the checkpoint layer's split of each
restore ([ckpt] restore: drop pending, output tails, terminal and spill, undo, host state) and how
the undo was done ([arena]: prepared, with its chunk count, or rewound). Medians and p95 per file."""
import json
import re
import statistics
import sys


def pct(v, q):
    v = sorted(v)
    return v[min(len(v) - 1, int(q * len(v)))] if v else float('nan')


for path in sys.argv[1:]:
    st = {'restore': [], 'find': [], 'edited': []}
    for line in open(path):
        d = json.loads(line)
        if d.get('summary') or d.get('key') is None:
            continue
        s = d.get('stages') or {}
        for k in ('restore', 'find'):
            if s.get(k) is not None:
                st[k].append(s[k])
        if d.get('edited_ms') is not None:
            st['edited'].append(d['edited_ms'])
    parts = {'drop': [], 'tails': [], 'spill': [], 'undo': [], 'host': []}
    prepared, rewound, chunks = 0, 0, []
    try:
        err = open(path + '.host-stderr').read()
    except OSError:
        err = ''
    for m in re.finditer(r'\[ckpt\] restore \d+: drop pending ([\d.]+), output tails ([\d.]+), '
                         r'terminal and spill ([\d.]+), undo ([\d.]+) \(\d+ logs\), host state ([\d.]+) ms', err):
        for k, v in zip(parts, m.groups()):
            parts[k].append(float(v))
    for m in re.finditer(r'\[arena\] restore to \d+: (prepared, (\d+) chunks|rewound)', err):
        if m.group(2):
            prepared += 1
            chunks.append(int(m.group(2)))
        else:
            rewound += 1
    out = {k: f'{statistics.median(v):.2f}/{pct(v, .95):.2f}' for k, v in {**st, **parts}.items() if v}
    out['prepared'] = prepared
    out['rewound'] = rewound
    if chunks:
        out['prepared_chunks_med'] = statistics.median(chunks)
    print(path.split('/')[-1], json.dumps(out))
