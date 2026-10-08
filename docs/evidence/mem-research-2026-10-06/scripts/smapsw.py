#!/usr/bin/env python3
"""smapsw.py PATTERN OUT: find the host whose command line contains PATTERN, then sample its RSS
every 100 ms until it exits. Writes OUT.series (t, rss) and OUT.json: the /proc smaps breakdown
taken at the peak (re-taken whenever RSS passes the last breakdown by 8 MB) and at the end."""
import json, os, re, sys, time

pat, out = sys.argv[1], sys.argv[2]


def find():
    for p in os.listdir('/proc'):
        if not p.isdigit():
            continue
        try:
            cmd = open(f'/proc/{p}/cmdline', 'rb').read().replace(b'\0', b' ').decode()
        except OSError:
            continue
        if pat in cmd and 'flashtex-host' in cmd and 'smapsw' not in cmd:
            return int(p)
    return None


def rss(pid):
    for line in open(f'/proc/{pid}/status'):
        if line.startswith('VmRSS:'):
            return int(line.split()[1]) * 1024
    return 0


def breakdown(pid):
    groups = {}
    cur = None
    for line in open(f'/proc/{pid}/smaps'):
        m = re.match(r'^([0-9a-f]+)-([0-9a-f]+) \S+ \S+ \S+ \S+\s*(.*)$', line)
        if m:
            size = int(m.group(2), 16) - int(m.group(1), 16)
            name = m.group(3).strip()
            if not name:
                mb = size >> 20
                name = ('[anon >=256M]' if mb >= 256 else '[anon 64M arena]' if 60 <= mb <= 64
                        else '[anon 1-60M]' if mb >= 1 else '[anon <1M]')
            elif name.startswith('/'):
                name = 'file:' + (os.path.basename(name) if not name.endswith('.fmt') else 'FORMAT')
            cur = groups.setdefault(name, {'n': 0, 'size': 0, 'rss': 0, 'priv_dirty': 0, 'anon': 0})
            cur['n'] += 1
            cur['size'] += size
            continue
        k, _, v = line.partition(':')
        if cur is None or not v.strip().endswith('kB'):
            continue
        val = int(v.split()[0]) * 1024
        if k == 'Rss':
            cur['rss'] += val
        elif k == 'Private_Dirty':
            cur['priv_dirty'] += val
        elif k == 'Anonymous':
            cur['anon'] += val
    return dict(sorted(groups.items(), key=lambda kv: -kv[1]['rss']))


pid = None
for _ in range(600):
    pid = find()
    if pid:
        break
    time.sleep(0.1)
if not pid:
    sys.exit('no host')
t0 = time.time()
peak, peak_bd, last_bd_at, last = 0, None, 0, None
series = open(out + '.series', 'w')
while True:
    try:
        r = rss(pid)
        if r > last_bd_at + (8 << 20):
            peak_bd = {'t': round(time.time() - t0, 2), 'rss': r, 'groups': breakdown(pid)}
            last_bd_at = r
        if r > peak:
            peak = r
        series.write(f'{time.time() - t0:.2f} {r}\n')
        if int((time.time() - t0) * 10) % 50 == 0:
            last = {'t': round(time.time() - t0, 2), 'rss': r, 'groups': breakdown(pid)}
    except (OSError, ValueError):
        break
    time.sleep(0.1)
json.dump({'peak': peak, 'at_peak': peak_bd, 'late': last}, open(out + '.json', 'w'), indent=1)
