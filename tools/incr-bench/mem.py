#!/usr/bin/env python3
"""mem.py ENGINE DOC [options]: the socket host's memory while a user types (DESIGN.md §5.2's
retention budget; lane P4-MEMORY). Starts `flashtex-host --socket` on $INCR_BENCH_DIR/docs/DOC
with FLASHTEX_MEMSTAT=1 (DONE then carries `mem`: the process's resident bytes and the checkpoint
layer's parts, `incr::Session::mem_stats`; with a `mem-stats` build also the heap by tag), and
types with `dl3-keys` on each of --pages in turn (--keys keystrokes each, --gap-ms apart, letters or
--sentence). Samples the host's resident memory every 50 ms meanwhile and kills it (SIGKILL, by
PID) if it passes --limit-gb, so that a runaway cannot take the machine down.

Prints one JSON line per keystroke's `mem` (to --out, default $INCR_BENCH_DIR/mem/DOC-TAG.jsonl)
and a summary line: peak resident bytes (sampled, and the kernel's own peak), the last `mem`, the
edited-page latency p50/p95, and whether the limit was hit. Exit status 1 if the host's peak passed
--gate-gb (the memory gate), 2 if the limit killed it.

  python3 tools/incr-bench/mem.py m0 plain-1000 --pages 0,500,999 --keys 20 --gate-gb 1.5
"""
import argparse
import json
import os
import signal
import statistics
import subprocess
import sys
import threading
import time

BASE = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
S = os.path.dirname(os.path.abspath(__file__))

ap = argparse.ArgumentParser()
ap.add_argument('engine')
ap.add_argument('doc')
ap.add_argument('--pages', default='0', help='0-based pages to type on, in turn')
ap.add_argument('--keys', type=int, default=20, help='keystrokes per page')
ap.add_argument('--gap-ms', type=int, default=300)
ap.add_argument('--sentence', action='store_true')
ap.add_argument('--where', default='middle')
ap.add_argument('--host-args', default=os.environ.get('MEM_HOSTARGS', ''),
                help="the host's options (default: $MEM_HOSTARGS)")
ap.add_argument('--limit-gb', type=float, default=12.0, help='kill the host above this RSS')
ap.add_argument('--gate-gb', type=float, default=0.0, help='fail (exit 1) above this peak RSS')
ap.add_argument('--tag', default='')
ap.add_argument('--out', default='')
ap.add_argument('--timeout', type=int, default=3600, help='seconds for the whole run')
a = ap.parse_args()

signal.alarm(a.timeout)
E = f'{BASE}/{a.engine}'
tag = a.tag or a.engine
outdir = f'{BASE}/mem'
os.makedirs(outdir, exist_ok=True)
out = a.out or f'{outdir}/{a.doc}-{tag}.jsonl'
W = f'{outdir}/work-{a.doc}-{tag}'
subprocess.run(['rm', '-rf', W], check=True)
os.makedirs(f'{W}/out')
subprocess.run(['cp', f'{BASE}/docs/{a.doc}/main.tex', f'{W}/main.tex'], check=True)
sock = f'{W}/h.sock'
env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{BASE}/fmt-{a.engine}',
           SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1', FLASHTEX_MEMSTAT='1')
herr = open(f'{out}.host-stderr', 'w')
hout = open(f'{W}/h.out', 'w')
host = subprocess.Popen([f'{E}/flashtex-host', '--socket', sock, '--s0-cache', f'{W}/s0']
                        + a.host_args.split(), env=env, stdout=hout, stderr=herr)


def rss_of(pid):
    """Resident bytes of `pid` now (Linux /proc; macOS ps, coarser)."""
    try:
        with open(f'/proc/{pid}/status') as f:
            for line in f:
                if line.startswith('VmRSS:'):
                    return int(line.split()[1]) * 1024
    except FileNotFoundError:
        pass
    except OSError:
        return 0
    try:
        r = subprocess.run(['ps', '-o', 'rss=', '-p', str(pid)], capture_output=True, text=True)
        return int(r.stdout.strip() or 0) * 1024
    except (OSError, ValueError):
        return 0


samples = {'peak': 0, 'killed': False}
stop = threading.Event()


def watch():
    period = 0.05 if os.path.exists('/proc') else 0.25
    while not stop.is_set() and host.poll() is None:
        r = rss_of(host.pid)
        samples['peak'] = max(samples['peak'], r)
        if r > a.limit_gb * 2**30:
            samples['killed'] = True
            os.kill(host.pid, signal.SIGKILL)
            break
        time.sleep(period)


threading.Thread(target=watch, daemon=True).start()
for _ in range(600):
    if 'listening' in open(f'{W}/h.out').read():
        break
    time.sleep(0.05)

lines = []
edited = []
t0 = time.time()
for p in [int(x) for x in a.pages.split(',')]:
    if host.poll() is not None:
        break
    args = [f'{E}/dl3-keys', '--socket', sock, '--root', W, '--main', 'main.tex', '--output-dir',
            f'{W}/out', '--keys', str(a.keys), '--gap-ms', str(a.gap_ms), '--no-viewport',
            '--page', str(p), '--where', a.where] + (['--sentence'] if a.sentence else [])
    r = subprocess.run([f'{S}/to.sh', str(a.timeout)] + args, capture_output=True, text=True)
    for line in r.stdout.splitlines():
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        done = d.get('host') or {}
        if 'mem' in done:
            rec = {'page': p, 'key': d.get('key'), 'mode': done.get('mode'),
                   'pages': done.get('pages'), 'rerun_pages': done.get('rerun_pages'),
                   'converged_at': done.get('converged_at'), 'restart_page': done.get('restart_page'),
                   'restart_gap': done.get('restart_gap'), 'edited_ms': d.get('edited_page_ms'),
                   'done_ms': d.get('done_ms'), 'stages': done.get('stages'), 'mem': done['mem']}
            lines.append(rec)
            if d.get('edited_page_ms') is not None:
                edited.append(d['edited_page_ms'])
stop.set()
kernel_peak = None
try:
    with open(f'/proc/{host.pid}/status') as f:
        for line in f:
            if line.startswith('VmHWM:'):
                kernel_peak = int(line.split()[1]) * 1024
except OSError:
    pass
if kernel_peak is None and lines:
    kernel_peak = lines[-1]['mem'].get('rss_peak')
if host.poll() is None:
    host.terminate()
    try:
        host.wait(10)
    except subprocess.TimeoutExpired:
        host.kill()
with open(out, 'w') as f:
    for rec in lines:
        f.write(json.dumps(rec) + '\n')
peak = max(samples['peak'], kernel_peak or 0)


def pct(v, q):
    v = sorted(v)
    return v[min(len(v) - 1, int(q * len(v)))] if v else None


summary = {
    'summary': True, 'doc': a.doc, 'engine': a.engine, 'tag': tag, 'pages_typed': a.pages,
    'keys': len(lines), 'peak_rss': peak, 'peak_rss_sampled': samples['peak'],
    'peak_rss_kernel': kernel_peak, 'killed_at_limit': samples['killed'],
    'edited_p50': statistics.median(edited) if edited else None, 'edited_p95': pct(edited, 0.95),
    'seconds': round(time.time() - t0, 1), 'edited': [round(v, 3) for v in edited],
    'last': lines[-1]['mem'] if lines else None,
}
print(json.dumps(summary))
with open(out, 'a') as f:
    f.write(json.dumps(summary) + '\n')
if samples['killed']:
    sys.exit(2)
if a.gate_gb and peak > a.gate_gb * 2**30:
    sys.exit(1)
