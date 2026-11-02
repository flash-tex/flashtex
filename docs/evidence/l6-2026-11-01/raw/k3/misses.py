#!/usr/bin/env python3
"""misses.py DEBUG: a digest of FLASHTEX_BOXMEMO_DEBUG's output for one name: lines by kind, and
how often each key entry differed on a miss."""
import collections
import re
import sys

kinds = collections.Counter()
entries = collections.Counter()
for line in open(sys.argv[1]):
    if not line.startswith('boxmemo: '):
        continue
    if ' recorded ' in line:
        kinds['recorded'] += 1
    elif ' replayed ' in line:
        kinds['replayed (a hit)'] += 1
    elif 'miss, key differs at' in line:
        kinds['miss: a register or eqtb entry the key read differs'] += 1
        for e in re.findall(r'"([^"=]+)=', line):
            entries[e.replace('\\\\', '\\')] += 1
    elif 'key differs:' in line:
        kinds['miss: a meaning the key read differs'] += 1
        for e in re.findall(r'"\\\\([^ ]+) was', line):
            entries['\\' + e + ' (meaning)'] += 1
    else:
        kinds['other: ' + line[9:70].strip()] += 1
print('FLASHTEX_BOXMEMO_DEBUG on full-100 with \\pdfstringdefPreHook the only name (run 41343123964,')
print('every compile of the variant, the settling passes included):')
for k, n in kinds.most_common():
    print(f'  {n:5}  {k}')
print('key entries that differed on a miss (top 20; a miss can list several):')
for k, n in entries.most_common(20):
    print(f'  {n:5}  {k}')
