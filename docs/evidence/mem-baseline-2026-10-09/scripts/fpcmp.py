"""fpcmp.py DIR: the final DONE's footprint (after the idle) and peak per run."""
import glob
import json
import sys

for path in sorted(glob.glob(f'{sys.argv[1]}/*/*/recs.jsonl')):
    last = None
    for line in open(path):
        r = json.loads(line)
        m = (r.get('host') or {}).get('mem')
        if m:
            last = m
    if last is None:
        continue
    print(f"{path.split('/')[-3]} {path.split('/')[-2]:60} steady {last['rss'] / 2**20:6.1f}  peak {last['rss_peak'] / 2**20:6.1f}")
