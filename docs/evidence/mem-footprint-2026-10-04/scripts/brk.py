#!/usr/bin/env python3
"""brk.py OUT/TAG-DOC: the steady-state breakdown from run.py's files (MB)."""
import json, re, sys
b = sys.argv[1]
mem = {}
for l in open(b + '.jsonl'):
    m = (json.loads(l).get('host') or {}).get('mem')
    if m:
        mem = m
M = lambda v: round(v / 1e6, 1)
vm = open(b + '.vmmap').read()
fp = re.search(r'Physical footprint:\s+(\S+)', vm).group(1)
fpp = re.search(r'Physical footprint \(peak\):\s+(\S+)', vm).group(1)
print(f'footprint {fp} (peak {fpp})')
for k in ['rss', 'rss_peak', 'malloc_in_use', 'malloc_held', 'heap', 'space_touched', 'space_resident',
          'slab_resident', 'slab_live', 'sealed_bytes', 'chain_words', 'chain_deltas', 'prepared',
          'bookkeeping', 'page_cache', 'form_cache', 'written', 'terminal', 'record_lines', 'spare_tails',
          'checkpoints', 'pages']:
    if k in mem:
        v = mem[k]
        print(f'  {k:16} {M(v) if k not in ("checkpoints","pages","chain_deltas","chain_words") else v}')
for k, v in mem.items():
    if k.startswith('res_') or (k.startswith('heap_') and not k.startswith('heap_peak')):
        if v > 5e5:
            print(f'  {k:24} {M(v)}')
for line in vm.splitlines():
    if re.match(r'(MALLOC_|VM_ALLOCATE|mapped file|__DATA|Stack )', line):
        print('  vm:', ' '.join(line.split()[:4]))
