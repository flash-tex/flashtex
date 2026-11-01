import json
import sys

prev = None
for line in open(sys.argv[1] + '/recs.jsonl'):
    r = json.loads(line)
    h = r.get('host') or {}
    m = h.get('mem') or {}
    if 'map_fresh' not in m:
        continue
    cur = {k: m[k] for k in m if k.startswith('map_')}
    if prev:
        d = {k[4:]: (cur[k] - prev[k]) for k in cur}
        print(r.get('phase'), h.get('mode'), {k: (round(v / 2**20, 2) if k.endswith('bytes') else v) for k, v in d.items()})
    prev = cur
