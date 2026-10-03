#!/usr/bin/env python3
"""ckcpu.py ENGINE DIR DOC REPS: the per-page cost of a checkpoint after every
shipout, from /usr/bin/time -l (CPU time, cycles, instructions: robust on a
busy machine): cold host runs without and with --every-shipout, interleaved."""
import json
import re
import statistics
import subprocess
import sys

E, d, doc, reps = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])


def run(every):
    extra = ['--every-shipout'] if every else []
    p = subprocess.run(['/usr/bin/time', '-l', '/tmp/p4l2/host.sh', E, d, 'bench', '--reps', '0'] + extra + ['--', doc],
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    err = p.stderr
    user = float(re.search(r'([\d.]+) user', err).group(1))
    sys_ = float(re.search(r'([\d.]+) sys', err).group(1))
    real = float(re.search(r'([\d.]+) real', err).group(1))
    ins = int(re.search(r'(\d+)\s+instructions retired', err).group(1))
    cyc = int(re.search(r'(\d+)\s+cycles elapsed', err).group(1))
    rss = int(re.search(r'(\d+)\s+maximum resident set size', err).group(1))
    j = [json.loads(l) for l in p.stdout.splitlines() if l.startswith('{')][-1]
    cs = j.get('checkpoint_stats') or {}
    return dict(cpu=user + sys_, real=real, ins=ins, cyc=cyc, rss=rss, n=cs.get('checkpoints', 0),
                seal_cpu=cs.get('seal_cpu_s', 0), pages=j.get('pages'))


rows = []
for r in range(reps):
    a = run(False)
    b = run(True)
    n = max(1, b['n'] - 1)
    rows.append(dict(cpu_ms=1000 * (b['cpu'] - a['cpu']) / n, cyc_k=(b['cyc'] - a['cyc']) / n / 1000,
                     ins_k=(b['ins'] - a['ins']) / n / 1000, seal_cpu_ms=1000 * b['seal_cpu'] / n,
                     rss_plain_mb=a['rss'] / 2**20, rss_every_mb=b['rss'] / 2**20, n=b['n'],
                     cpu_plain=a['cpu'], cpu_every=b['cpu']))
    print(json.dumps(rows[-1]), flush=True)
med = {k: statistics.median(r[k] for r in rows) for k in rows[0]}
print('median per checkpoint:', json.dumps({k: round(v, 3) for k, v in med.items()}))
