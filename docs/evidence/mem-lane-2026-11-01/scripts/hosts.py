#!/usr/bin/env python3
"""hosts.py OUT.json SECONDS: sum every flashtex-host's Rss and Pss (smaps_rollup) every 0.5 s for
SECONDS, or until OUT.json.stop exists; write the peak and last sums with per-host values (lane MEMORY).
Pss divides shared pages among their sharers, so sum(Pss) < sum(Rss) is what the hosts share."""
import json
import os
import sys
import time

out, secs = sys.argv[1], float(sys.argv[2])
best = {'rss': 0, 'pss': 0}
last = {}
t0 = time.time()
while time.time() - t0 < secs and not os.path.exists(out + '.stop'):
    per = {}
    for p in os.listdir('/proc'):
        if not p.isdigit():
            continue
        try:
            if open(f'/proc/{p}/comm').read().strip() != 'flashtex-host':
                continue
            rss = pss = 0
            for line in open(f'/proc/{p}/smaps_rollup'):
                k, _, v = line.partition(':')
                if k == 'Rss':
                    rss = int(v.split()[0]) * 1024
                elif k == 'Pss':
                    pss = int(v.split()[0]) * 1024
            per[p] = (rss, pss)
        except OSError:
            pass
    if per:
        s = {'rss': sum(v[0] for v in per.values()), 'pss': sum(v[1] for v in per.values()), 'n': len(per)}
        if s['rss'] > best['rss']:
            best = dict(s, per=per)
        last = dict(s, per=per)
    time.sleep(0.5)
json.dump({'peak': best, 'last': last}, open(out, 'w'))
print(json.dumps({'peak_rss_sum_mb': best['rss'] >> 20, 'peak_pss_sum_mb': best['pss'] >> 20, 'n': best.get('n'),
                  'last_rss_sum_mb': last.get('rss', 0) >> 20, 'last_pss_sum_mb': last.get('pss', 0) >> 20}))
