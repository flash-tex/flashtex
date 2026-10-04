#!/usr/bin/env python3
"""leaks_session.py: the resident host's unreachable memory over an edit session, by macOS `leaks`
(lane MEMORY-SAFETY, 2026-10-04; scripts/sanitizers.sh `leaks`).

    leaks_session.py BENCH_DIR ENGINE DOC [--page P] [--keys A,B,C] [--start-kb N] [--per-edit-b N]

Starts BENCH_DIR/ENGINE/flashtex-host (a release build, as tools/incr-bench/mkeng.sh copies it) on a
socket for BENCH_DIR/docs/DOC/main.tex, types A, B and C keystrokes in three connections
(dl3-keys, as the app does), and runs `leaks` on the live host after each. Fails when

* the host has lost more than --start-kb KB after the first connection (default 2048: start-up
  loses ~0.15 MB, all in kpathsea's one-time set-up; the 18 MB lost at every start before
  2026-10-04 fails it), or
* the bytes lost grow by more than --per-edit-b bytes a keystroke between the second connection
  and the third (default 1024: was ~65 KB through zlib and ~18 KB through kpathsea's lookups
  before 2026-10-04; ~0.2 KB after).

LeakSanitizer missed all three of those (a stale copy of a pointer kept each "reachable"), so
this check uses Apple's conservative scanner on the release build. The project directory holds
only main.tex: files the harness writes go elsewhere, so the session sees no directory change.
"""
import argparse
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time

ap = argparse.ArgumentParser()
ap.add_argument('bench')
ap.add_argument('engine')
ap.add_argument('doc')
ap.add_argument('--page', type=int, default=3)
ap.add_argument('--keys', default='20,20,100')
ap.add_argument('--start-kb', type=int, default=2048)
ap.add_argument('--per-edit-b', type=int, default=1024)
a = ap.parse_args()
if sys.platform != 'darwin':
    print('SKIP leaks_session: macOS `leaks` only')
    sys.exit(0)

E = os.path.join(a.bench, a.engine)
work = tempfile.mkdtemp(prefix='ftx-leaks-')
proj = os.path.join(work, 'proj')
out = os.path.join(work, 'out')
logs = os.path.join(work, 'logs')
for d in (proj, out, logs):
    os.makedirs(d)
shutil.copy(os.path.join(a.bench, 'docs', a.doc, 'main.tex'), proj)
sock = os.path.join(work, 'h.sock')
env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{a.bench}/fmt-{a.engine}',
           SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1', MallocStackLogging='lite')
hout = open(os.path.join(logs, 'h.out'), 'w')
host = subprocess.Popen([f'{E}/flashtex-host', '--socket', sock, '--s0-cache', os.path.join(work, 's0')],
                        env=env, stdout=hout, stderr=subprocess.STDOUT, cwd=work)
env_keys = {k: v for k, v in env.items() if k != 'MallocStackLogging'}


def lost():
    r = subprocess.run(['leaks', str(host.pid)], capture_output=True, text=True)
    open(os.path.join(logs, f'leaks-{time.time():.0f}.txt'), 'w').write(r.stdout)
    m = re.search(r'(\d+) leaks for (\d+) total leaked bytes', r.stdout)
    if not m:
        raise SystemExit(f'leaks gave no total:\n{r.stdout[-2000:]}{r.stderr[-2000:]}')
    return int(m.group(1)), int(m.group(2))


ok = True
try:
    for _ in range(600):
        if os.path.exists(sock):
            break
        time.sleep(0.05)
    totals = []
    for n in [int(x) for x in a.keys.split(',')]:
        r = subprocess.run([f'{E}/dl3-keys', '--socket', sock, '--root', proj, '--main', 'main.tex',
                            '--output-dir', out, '--keys', str(n), '--gap-ms', '100', '--no-viewport',
                            '--page', str(a.page), '--where', 'middle'],
                           env=env_keys, capture_output=True, text=True, timeout=1800)
        if host.poll() is not None:
            raise SystemExit(f'the host ended during the session ({host.returncode})')
        done = sum(1 for line in r.stdout.splitlines() if '"host"' in line)
        objs, b = lost()
        totals.append((n, done, objs, b))
        print(json.dumps({'keys': n, 'compiles': done, 'leaks': objs, 'leaked_bytes': b}))
    (_, _, _, b0), (_, _, _, b1), (n2, _, _, b2) = totals[0], totals[1], totals[2]
    if b0 > a.start_kb * 1024:
        print(f'FAIL {b0} bytes lost after the first connection (limit {a.start_kb} KB)')
        ok = False
    per = (b2 - b1) / max(n2, 1)
    verdict = 'PASS' if per <= a.per_edit_b else 'FAIL'
    print(f'{verdict} {per:.0f} bytes lost per keystroke over the last {n2} (limit {a.per_edit_b})')
    ok = ok and per <= a.per_edit_b
finally:
    host.send_signal(signal.SIGTERM)
    try:
        host.wait(10)
    except subprocess.TimeoutExpired:
        host.kill()
    if ok:
        shutil.rmtree(work, ignore_errors=True)
    else:
        print(f'kept {work} (leaks reports in logs/)')
sys.exit(0 if ok else 1)
