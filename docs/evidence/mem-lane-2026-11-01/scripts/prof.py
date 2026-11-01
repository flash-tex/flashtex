#!/usr/bin/env python3
"""prof.py ENGINE NAME OUT.jsonl [options]: one resident host's memory over a user's session (lane
MEMORY, 2026-11-01). Linux (reads /proc). Starts `flashtex-host --socket --s0-cache` on a fresh copy
of the document (--src DIR --main FILE, or $INCR_BENCH_DIR/docs/NAME/main.tex) with FLASHTEX_MEMSTAT=1,
opens it (`dl3-keys --keys 0`), types --keys letters on each of --pages, idles --idle-s (past the
idle trim), opens it once more unchanged, and records:

  open      DONE.mem of the open, and /proc right after it
  typed     DONE.mem of the last keystroke, and /proc right after it
  steady    /proc after the idle (VmRSS, smaps_rollup, smaps grouped by mapping kind)
  again     DONE.mem of the unchanged reopen (the session's parts after the idle)
  disk      bytes of the --s0-cache directory (S0, S1)
  peak      VmHWM, and the 50 ms sampled peak

--hold-s keeps the host alive after the run (several hosts at once).
"""
import argparse
import json
import os
import shutil
import signal
import subprocess
import sys
import threading
import time

IB = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
ap = argparse.ArgumentParser()
ap.add_argument('engine')
ap.add_argument('name')
ap.add_argument('out')
ap.add_argument('--src', default='')
ap.add_argument('--main', default='main.tex')
ap.add_argument('--edit', default='', help='dl3-keys --edit FILE (a chapter of a book)')
ap.add_argument('--pages', default='0')
ap.add_argument('--keys', type=int, default=8)
ap.add_argument('--gap-ms', type=int, default=300)
ap.add_argument('--idle-s', type=float, default=8)
ap.add_argument('--hold-s', type=float, default=0, help='stay alive this long after the run')
ap.add_argument('--profile', default='')
ap.add_argument('--host-args', default='')
ap.add_argument('--limit-gb', type=float, default=8)
ap.add_argument('--timeout', type=int, default=5400)
ap.add_argument('--wrap', default='', help='a command the host runs under (e.g. heaptrack -o F)')
a = ap.parse_args()
signal.alarm(a.timeout)

E = f'{IB}/{a.engine}'
W = f'{IB}/prof/work-{a.name}'
shutil.rmtree(W, ignore_errors=True)
os.makedirs(f'{W}/out')
if a.src:
    subprocess.run(['cp', '-a', f'{a.src}/.', W], check=True)
else:
    shutil.copy(f'{IB}/docs/{a.name}/main.tex', f'{W}/main.tex')
sock = f'{W}/h.sock'
env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{IB}/fmt-{a.engine}',
           SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1', FLASHTEX_MEMSTAT='1')
cmd = a.wrap.split() + [f'{E}/flashtex-host', '--socket', sock, '--s0-cache', f'{W}/s0'] \
    + a.host_args.split() + (['--profile', a.profile] if a.profile else [])
hout = open(f'{W}/h.out', 'w')
host = subprocess.Popen(cmd, env=env, stdout=hout, stderr=open(f'{W}/h.err', 'w'))


def hostpid():
    """The host's PID (under --wrap, the wrapped child)."""
    if not a.wrap:
        return host.pid
    for _ in range(200):
        r = subprocess.run(['pgrep', '-f', f'flashtex-host --socket {sock}'], capture_output=True, text=True)
        p = []
        for x in r.stdout.split():
            try:
                if open(f'/proc/{x}/comm').read().strip() == 'flashtex-host':
                    p.append(int(x))
            except OSError:
                pass
        if p:
            return p[-1]
        time.sleep(0.05)
    return host.pid


def status(pid):
    d = {}
    try:
        for line in open(f'/proc/{pid}/status'):
            k, _, v = line.partition(':')
            if k in ('VmRSS', 'VmHWM', 'RssAnon', 'RssFile', 'RssShmem'):
                d[k] = int(v.split()[0]) * 1024
        for line in open(f'/proc/{pid}/smaps_rollup'):
            k, _, v = line.partition(':')
            if k in ('Pss', 'Private_Dirty', 'Shared_Clean', 'Private_Clean', 'Anonymous'):
                d['roll_' + k] = int(v.split()[0]) * 1024
    except OSError:
        pass
    return d


def smaps(pid):
    """Resident bytes by mapping kind: file mappings by path, anonymous ones by size class."""
    out = {}
    cur = None
    try:
        for line in open(f'/proc/{pid}/smaps'):
            f = line.split()
            if '-' in f[0] and len(f) >= 5 and not f[0].endswith(':'):
                lo, hi = (int(x, 16) for x in f[0].split('-'))
                size = hi - lo
                path = f[5] if len(f) > 5 else ''
                if path:
                    cur = 'file:' + os.path.basename(path)
                elif size >= 256 << 20:
                    cur = 'anon>=256M'
                elif size >= 16 << 20:
                    cur = 'anon16-256M'
                elif size >= 1 << 20:
                    cur = 'anon1-16M'
                else:
                    cur = 'anon<1M'
            elif f[0] == 'Rss:' and cur:
                out[cur] = out.get(cur, 0) + int(f[1]) * 1024
    except OSError:
        pass
    return dict(sorted(((k, v) for k, v in out.items() if v >= 1 << 20), key=lambda kv: -kv[1]))


for _ in range(1200):
    if 'listening' in open(f'{W}/h.out').read():
        break
    time.sleep(0.05)
pid = hostpid()
peak = {'v': 0, 'killed': False}
stop = threading.Event()


def watch():
    while not stop.is_set() and host.poll() is None:
        r = status(pid).get('VmRSS', 0)
        peak['v'] = max(peak['v'], r)
        if r > a.limit_gb * 2**30:
            peak['killed'] = True
            os.kill(pid, signal.SIGKILL)
            break
        time.sleep(0.05)


threading.Thread(target=watch, daemon=True).start()


def keys(page, n):
    args = [f'{E}/dl3-keys', '--socket', sock, '--root', W, '--main', a.main, '--output-dir', f'{W}/out',
            '--keys', str(n), '--gap-ms', str(a.gap_ms), '--no-viewport', '--page', str(page)] \
        + (['--edit', a.edit, '--repeated'] if a.edit and n else [])
    r = subprocess.run(args, capture_output=True, text=True, env=env)
    last, ms = None, []
    for line in r.stdout.splitlines():
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        h = d.get('host') or {}
        if 'mem' in h:
            last = {'mem': dict(h['mem']), 'pages': h.get('pages'), 'stages': h.get('stages')}
            if d.get('edited_page_ms') is not None:
                ms.append(d['edited_page_ms'])
    if last is None:
        sys.stderr.write(f'{a.name}: page {page}: no DONE.mem\n{r.stderr[-2000:]}\n')
    return last, ms


def du(p):
    n = 0
    for dp, _, fs in os.walk(p):
        for f in fs:
            try:
                n += os.path.getsize(os.path.join(dp, f))
            except OSError:
                pass
    return n


rec = {'name': a.name, 'engine': a.engine, 'profile': a.profile or 'balanced'}
t0 = time.time()
rec['open'], _ = keys(0, 0)
rec['open_s'] = round(time.time() - t0, 2)
rec['open_proc'] = status(pid)
ms_all = []
last = rec['open']
for p in (int(x) for x in a.pages.split(',')):
    if a.keys and host.poll() is None:
        r, ms = keys(p, a.keys)
        last = r or last
        ms_all += ms
rec['typed'] = last
rec['typed_proc'] = status(pid)
rec['typed_smaps'] = smaps(pid)
time.sleep(a.idle_s)
rec['steady_proc'] = status(pid)
rec['steady_smaps'] = smaps(pid)
rec['again'], _ = keys(0, 0)
rec['again_proc'] = status(pid)
rec['disk_s0'] = du(f'{W}/s0')
rec['disk_files'] = {f: os.path.getsize(f'{W}/s0/{f}') for f in sorted(os.listdir(f'{W}/s0'))} \
    if os.path.isdir(f'{W}/s0') else {}
if a.hold_s:
    time.sleep(a.hold_s)
stop.set()
rec['peak_sampled'] = peak['v']
rec['peak_hwm'] = status(pid).get('VmHWM')
rec['killed'] = peak['killed']
ms_all.sort()
rec['edited_p50'] = ms_all[len(ms_all) // 2] if ms_all else None
rec['edited_p95'] = ms_all[min(len(ms_all) - 1, int(0.95 * len(ms_all)))] if ms_all else None
rec['seconds'] = round(time.time() - t0, 1)
host.send_signal(signal.SIGTERM)
try:
    host.wait(20)
except subprocess.TimeoutExpired:
    host.kill()
with open(a.out, 'a') as f:
    f.write(json.dumps(rec) + '\n')
mb = lambda v: round((v or 0) / 2**20)  # noqa: E731
print(f"{a.name}: open {mb(rec['open_proc'].get('VmRSS'))} MB, typed {mb(rec['typed_proc'].get('VmRSS'))}, "
      f"steady {mb(rec['steady_proc'].get('VmRSS'))}, peak {mb(rec['peak_hwm'])}, s0 disk {mb(rec['disk_s0'])} MB, "
      f"{rec['seconds']} s")
