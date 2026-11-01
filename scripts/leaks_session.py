#!/usr/bin/env python3
"""leaks_session.py: the resident host's unreachable memory over an edit session, by macOS `leaks`
(lane MEMORY-SAFETY, 2026-10-04; scripts/sanitizers.sh `leaks`).

    leaks_session.py BENCH_DIR ENGINE DOC [--page P] [--keys A,B,C] [--start-kb N] [--per-edit-b N]
                     [--changing-dir]

Starts BENCH_DIR/ENGINE/flashtex-host (a release build, as tools/incr-bench/mkeng.sh copies it) on a
socket for BENCH_DIR/docs/DOC/main.tex, types A, B and C keystrokes in three connections
(dl3-keys, as the app does), and runs `leaks` on the live host after each. Fails when

* the host has lost more than --start-kb KB after the first connection (default 2048: start-up
  loses ~0.15 MB, all in kpathsea's one-time set-up; the 18 MB lost at every start before
  2026-10-04 fails it), or
* the bytes lost grow by more than --per-edit-b bytes a keystroke between the second connection
  and the third (default 1024: was ~65 KB through zlib and ~18 KB through kpathsea's lookups
  before 2026-10-04; ~0.2 KB after).

--changing-dir: while the keystrokes run, a file is added to (and the one before removed from) the
project directory every 150 ms, so that its signature changes between keystrokes: the lookup memo
looks again, and the resolver forgets kpathsea's `//` expansions of the trees no ls-R covers and
expands them again (`FileResolver::refresh_disk_dirs`), as when the user saves a figure. Those
paths lost ~1.2-1.8 KB (`texmf_nlink_for_leaf`) and ~3.3 KB (`str_list_uniqify`) a keystroke
before 2026-10-09 (review of #1493); the plain leg, an unchanged directory, does not take them.

LeakSanitizer missed all three of those (a stale copy of a pointer kept each "reachable"), so
this check uses Apple's conservative scanner on the release build. The project directory holds
only main.tex: files the harness writes go elsewhere, so the session sees no directory change.

**`leaks` and hashbrown.** A Rust `HashMap` keeps one pointer to its table: `ctrl`, which points
into the table's allocation, past its buckets, never at its start. `leaks` does not credit such a
pointer for a large (VM-backed) block, so a big live map shows up as a ROOT LEAK with every entry
beneath it. Probe (2026-10-04, macOS 26.3): a 50,000-entry map in a `thread_local!`, alive and
used afterwards, was reported as 5.4 MB in 50,001 leaks. On full-120 the engine's `diag` definition
map (`diag.rs`, `St::defs`, live) is reported the same way (~9 MB). Roots allocated by hashbrown's
`RawTableInner::fallible_with_capacity` are therefore counted apart and printed, and the byte
limits apply to the rest: the live map's apparent size swings with its capacity (+-295 KB between
two sessions on full-120, which read as +-2,949 B a keystroke). A map really lost once per
keystroke adds a root each time, so the number of hashbrown roots after the third session must be
no more than after either of the first two (a live map is reported in one scan and not in the next:
2, 1 and 2 roots on full-120).
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
import threading
import time

ap = argparse.ArgumentParser()
ap.add_argument('bench')
ap.add_argument('engine')
ap.add_argument('doc')
ap.add_argument('--page', type=int, default=3)
ap.add_argument('--keys', default='20,20,100')
ap.add_argument('--start-kb', type=int, default=2048)
ap.add_argument('--per-edit-b', type=int, default=1024)
ap.add_argument('--changing-dir', action='store_true',
                help='add and remove a file in the project directory between keystrokes')
ap.add_argument('--work-root', default=None,
                help='where the work directory goes (kept, with the leaks reports, on failure)')
a = ap.parse_args()
if sys.platform != 'darwin':
    print('SKIP leaks_session: macOS `leaks` only')
    sys.exit(0)

E = os.path.join(a.bench, a.engine)
if a.work_root:
    os.makedirs(a.work_root, exist_ok=True)
work = tempfile.mkdtemp(prefix='ftx-leaks-', dir=a.work_root)
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


UNITS = {'': 1, 'K': 1024, 'M': 2**20, 'G': 2**30}


def hashbrown_roots(report):
    """(bytes, roots) under ROOT LEAKs whose block hashbrown allocated (the false positive above)."""
    total = roots = 0
    for blk in re.split(r'\n(?=STACK OF )', report):
        head = re.match(r"STACK OF (\d+) INSTANCES? OF '(.*)'", blk)
        if not head or 'hashbrown' not in head.group(2) or 'RawTable' not in head.group(2):
            continue
        roots += int(head.group(1))
        m = (re.search(r'^\s+\d+ \(([\d.]+)([KMG]?)(?: bytes)?\) << TOTAL >>', blk, re.M)
             or re.search(r'^\s+\d+ \(([\d.]+)([KMG]?)(?: bytes)?\) ROOT', blk, re.M))
        if m:
            total += int(float(m.group(1)) * UNITS[m.group(2)])
    return total, roots


def lost():
    """(leaks, bytes lost, (bytes, roots) under hashbrown roots) of the live host."""
    r = subprocess.run(['leaks', str(host.pid)], capture_output=True, text=True)
    open(os.path.join(logs, f'leaks-{time.time():.0f}.txt'), 'w').write(r.stdout)
    m = re.search(r'(\d+) leaks for (\d+) total leaked bytes', r.stdout)
    if not m:
        raise SystemExit(f'leaks gave no total:\n{r.stdout[-2000:]}{r.stderr[-2000:]}')
    return int(m.group(1)), int(m.group(2)), hashbrown_roots(r.stdout)


# False until the verdict: any exit before it (the host ending, `leaks` giving no total, a
# timeout) keeps the work directory and its logs.
ok = False
try:
    for _ in range(600):
        if os.path.exists(sock):
            break
        time.sleep(0.05)
    totals = []
    for n in [int(x) for x in a.keys.split(',')]:
        stop = threading.Event()

        def churn():
            # a name no lookup of the document asks for
            k = 0
            while not stop.wait(0.15):
                k += 1
                open(os.path.join(proj, f'ftx-churn-{k}.dat'), 'w').close()
                if k > 1:
                    os.remove(os.path.join(proj, f'ftx-churn-{k - 1}.dat'))

        churner = threading.Thread(target=churn, daemon=True)
        if a.changing_dir:
            churner.start()
        r = subprocess.run([f'{E}/dl3-keys', '--socket', sock, '--root', proj, '--main', 'main.tex',
                            '--output-dir', out, '--keys', str(n), '--gap-ms', '100', '--no-viewport',
                            '--page', str(a.page), '--where', 'middle'],
                           env=env_keys, capture_output=True, text=True, timeout=1800)
        stop.set()
        if a.changing_dir:
            churner.join()
        if host.poll() is not None:
            raise SystemExit(f'the host ended during the session ({host.returncode})')
        done = sum(1 for line in r.stdout.splitlines() if '"host"' in line)
        objs, b, (hb, hr) = lost()
        totals.append((n, done, objs, b, hb, hr))
        print(json.dumps({'keys': n, 'compiles': done, 'leaks': objs, 'leaked_bytes': b,
                          'hashbrown_root_bytes': hb, 'hashbrown_roots': hr}))
    (_, _, _, b0, h0, r0), (_, _, _, b1, h1, r1), (n2, _, _, b2, h2, r2) = totals
    ok = True
    if b0 - h0 > a.start_kb * 1024:
        print(f'FAIL {b0 - h0} bytes lost after the first connection, besides {h0} under hashbrown '
              f'tables (limit {a.start_kb} KB)')
        ok = False
    else:
        print(f'PASS {b0 - h0} bytes lost after the first connection, besides {h0} under hashbrown '
              f'tables (limit {a.start_kb} KB)')
    per = ((b2 - h2) - (b1 - h1)) / max(n2, 1)
    verdict = 'PASS' if per <= a.per_edit_b else 'FAIL'
    print(f'{verdict} {per:.0f} bytes lost per keystroke over the last {n2}, besides hashbrown '
          f'tables (limit {a.per_edit_b})')
    ok = ok and per <= a.per_edit_b
    # (a live map is reported in one scan and not in the next: on full-120 the two `diag` maps
    # gave 2, 1, 2 roots; a map lost once a keystroke adds a root per keystroke)
    verdict = 'PASS' if r2 <= max(r0, r1) else 'FAIL'
    print(f'{verdict} hashbrown roots: {r0}, {r1} and {r2} after the sessions (the third no more '
          f'than before)')
    ok = ok and r2 <= max(r0, r1)
finally:
    host.send_signal(signal.SIGTERM)
    try:
        host.wait(10)
    except subprocess.TimeoutExpired:
        host.kill()
    if ok:
        shutil.rmtree(work, ignore_errors=True)
    else:
        # (a socket is no file an artifact upload can take)
        if os.path.exists(sock):
            os.remove(sock)
        print(f'kept {work} (leaks reports in logs/)')
sys.exit(0 if ok else 1)
