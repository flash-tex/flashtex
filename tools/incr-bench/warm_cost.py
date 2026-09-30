#!/usr/bin/env python3
"""warm_cost.py ENGINE DOC [--keys N] [--gap-ms MS] [--idle-s S] [--host-args "..."]: what the host
costs while the user types and while idle (a powermetrics-free energy proxy: the host process's CPU
seconds per minute of wall time), and the edited page's latency, through the socket as the app
types (page 1, no viewport). One host (no --once) on $INCR_BENCH_DIR/docs/DOC; dl3-keys types N
keystrokes GAP ms apart (the typing time starts at the first keystroke's answer: the opening
compiles are not typing); then 3 s (the keep-warm tail) and S seconds of idle are measured apart. Prints one JSON line."""
import argparse
import json
import os
import re
import shutil
import subprocess
import time

ap = argparse.ArgumentParser()
ap.add_argument('engine')
ap.add_argument('doc')
ap.add_argument('--keys', type=int, default=200)
ap.add_argument('--gap-ms', type=int, default=300)
ap.add_argument('--idle-s', type=float, default=30)
ap.add_argument('--host-args', default='')
ap.add_argument('--tag', default='')
a = ap.parse_args()
BASE = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
E = f'{BASE}/{a.engine}'
D = f'{BASE}/warm'
W = f'{D}/work-{a.doc}'
shutil.rmtree(W, ignore_errors=True)
os.makedirs(W + '/out')
shutil.copy(f'{BASE}/docs/{a.doc}/main.tex', W)
sock = f'{D}/h.sock'
if os.path.exists(sock):
    os.remove(sock)
shutil.rmtree(f'{D}/s0-{a.doc}', ignore_errors=True)
env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{BASE}/fmt-{a.engine}',
           SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1')
host = subprocess.Popen([f'{E}/flashtex-host', '--socket', sock, '--s0-cache', f'{D}/s0-{a.doc}']
                        + a.host_args.split(), env=env, stdout=open(f'{D}/h.out', 'w'),
                        stderr=open(f'{D}/h.stderr', 'w'))


def cpu():
    t = subprocess.run(['ps', '-o', 'cputime=', '-p', str(host.pid)],
                       capture_output=True, text=True).stdout.strip()
    s = 0.0
    for p in re.split(':', t):
        s = s * 60 + float(p)
    return s


try:
    for _ in range(600):
        if 'listening' in open(f'{D}/h.out').read():
            break
        time.sleep(0.05)
    kp = subprocess.Popen([f'{E}/dl3-keys', '--socket', sock, '--root', W, '--main', 'main.tex',
                           '--output-dir', W + '/out', '--keys', str(a.keys), '--gap-ms', str(a.gap_ms),
                           '--no-viewport', '--page', '0'], env=env, stdout=subprocess.PIPE,
                          stderr=subprocess.DEVNULL, text=True)
    lines, c0, t0 = [], None, None
    for line in kp.stdout:
        if not line.startswith('{'):
            continue
        j = json.loads(line)
        lines.append(j)
        if c0 is None and 'key' in j:
            c0, t0 = cpu(), time.time()
    kp.wait(timeout=1800)
    t1 = time.time()
    c1 = cpu()
    # the keep-warm tail after the last keystroke, then idle
    time.sleep(3)
    c1b = cpu()
    time.sleep(a.idle_s)
    c2 = cpu()
    summ = [j for j in lines if 'summary' in j]
    typing_s = t1 - t0
    print(json.dumps(dict(
        engine=a.engine, doc=a.doc, tag=a.tag, host_args=a.host_args, keys=a.keys, gap_ms=a.gap_ms,
        typing_s=round(typing_s, 1), idle_s=a.idle_s,
        host_cpu_typing_s=round(c1 - c0, 2), host_cpu_tail_3s=round(c1b - c1, 2),
        host_cpu_idle_s=round(c2 - c1b, 2),
        cpu_s_per_min_typing=round((c1 - c0) / typing_s * 60, 1),
        cpu_s_per_min_idle=round((c2 - c1b) / a.idle_s * 60, 2),
        first_page_ms=summ[0]['first_page_ms'] if summ else None,
        load=round(os.getloadavg()[0], 2))), flush=True)
finally:
    host.terminate()
    host.wait(timeout=30)
