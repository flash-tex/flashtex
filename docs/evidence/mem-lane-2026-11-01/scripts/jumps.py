#!/usr/bin/env python3
"""jumps.py ENGINE NAME OUT.jsonl [options]: a far jump's first keystroke, in milliseconds (lane
MEMORY's VIEWPORT gate, macOS or Linux; no /proc).

One resident host (flashtex-host --socket, as the app runs it) on $INCR_BENCH_DIR/docs/NAME (or
--src): the document opened, KEYS keystrokes on the first of --pages (the retention cursor goes
there), then for each later page a jump: with --jump-ms MS the client first sends VIEWPORT (spec
§6.3.1) with the page and waits MS ms (0: types at once), then KEYS keystrokes there. Each
keystroke's `edited_page_ms` (COMPILE written to the watched page's PAGE read, client side: the
T7 quantity, DESIGN.md §1.2's host share) is recorded, the jump's first apart. --edit F1,F2,... types
in those files of the project, cycled per page (a book's chapters). FLASHTEX_FAR in the environment
reaches the host. One JSON line per page to OUT.jsonl."""
import argparse, json, os, shutil, signal, subprocess, sys, time

IB = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
ap = argparse.ArgumentParser()
ap.add_argument('engine')
ap.add_argument('name')
ap.add_argument('out')
ap.add_argument('--src', default='')
ap.add_argument('--main', default='main.tex')
ap.add_argument('--pages', default='0,300,600')
ap.add_argument('--edit', default='')
ap.add_argument('--keys', type=int, default=6)
ap.add_argument('--gap-ms', type=int, default=300)
ap.add_argument('--jump-ms', type=int, default=-1)
ap.add_argument('--cfg', default='')
ap.add_argument('--round', type=int, default=0)
ap.add_argument('--timeout', type=int, default=3600)
a = ap.parse_args()
signal.alarm(a.timeout)

E = f'{IB}/{a.engine}'
# (one client for every engine: the protocol is the same, and the lane's has --jump-viewport)
CLIENT = os.environ.get('JUMPS_CLIENT', f'{E}/dl3-keys')
W = os.environ.get('JUMPS_WORK', f'{IB}/jw-{a.name}')
shutil.rmtree(W, ignore_errors=True)
if a.src:
    shutil.copytree(a.src, W)
else:
    os.makedirs(W)
    shutil.copy(f'{IB}/docs/{a.name}/main.tex', f'{W}/main.tex')
os.makedirs(f'{W}/out', exist_ok=True)
sock = f'{W}/h.sock'
env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{IB}/fmt-{a.engine}',
           SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1')
host = subprocess.Popen([f'{E}/flashtex-host', '--socket', sock, '--s0-cache', f'{W}/s0'], env=env,
                        stdout=open(f'{W}/h.out', 'w'), stderr=open(f'{W}/h.err', 'w'))
for _ in range(600):
    if os.path.exists(sock):
        break
    time.sleep(0.05)
EDITS = [e for e in a.edit.split(',') if e]


def keys(i, page, n):
    args = [CLIENT, '--socket', sock, '--root', W, '--main', a.main, '--output-dir', f'{W}/out',
            '--keys', str(n), '--gap-ms', str(a.gap_ms), '--no-viewport', '--page', str(page)]
    if n:
        args += ['--repeated', '--min-words', '8']
        if EDITS:
            args += ['--edit', EDITS[i % len(EDITS)]]
        if a.jump_ms >= 0 and i > 0:
            args += ['--jump-viewport', str(a.jump_ms)]
    r = subprocess.run(args, capture_output=True, text=True, env=env)
    if os.environ.get('JUMPS_LOG'):
        # (the client's `[kw]` lines, FLASHTEX_KEYWAIT_DEBUG, beside the host's)
        with open(os.environ['JUMPS_LOG'], 'a') as lf:
            lf.write(f'=== client {a.cfg} r{a.round} step {i} page {page}\n')
            lf.write(''.join(l + '\n' for l in r.stderr.splitlines() if l.startswith('[kw]')))
    ms, instr, stages = [], [], []
    for line in r.stdout.splitlines():
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        # (one line a keystroke; the last line's `edited_page_ms` is the summary's object)
        if isinstance(d.get('edited_page_ms'), (int, float)):
            ms.append(round(d['edited_page_ms'], 2))
            st = (d.get('host') or {}).get('stages') or {}
            instr.append(st.get('edited_instr_k'))
            # (where a key's time goes: `queue` = COMPILE's arrival to the engine taking it,
            # `queue_by` = what the engine thread did meanwhile, `paused_how`/`paused` = what it
            # did with a run newer work stopped)
            stages.append({k: st.get(k) for k in ('queue', 'queue_by', 'paused_how', 'paused', 'find', 'key',
                                                  'changes', 'restore', 'first_page', 'edited_page',
                                                  'queue_by_instr_k', 'queue_by_kc', 'paused_instr_k')
                           if k in st})
    if n and not ms:
        sys.stderr.write(f'{a.name} page {page}: no keystroke timed (exit {r.returncode})\n{r.stderr[-1500:]}\n')
    return ms, instr, stages


t0 = time.time()
keys(0, 0, 0)  # the open
open_s = round(time.time() - t0, 1)
with open(a.out, 'a') as f:
    for i, p in enumerate(int(x) for x in a.pages.split(',')):
        if host.poll() is not None:
            break
        ms, instr, stages = keys(i, p, a.keys)
        rec = {'doc': a.name, 'cfg': a.cfg or a.engine, 'engine': a.engine, 'round': a.round,
               'far': os.environ.get('FLASHTEX_FAR'), 'jump_ms': a.jump_ms, 'step': i, 'page': p,
               'edit': EDITS[i % len(EDITS)] if EDITS else None, 'open_s': open_s,
               'first_ms': ms[0] if ms else None, 'rest_ms': ms[1:], 'instr_k': instr,
               'first_stages': stages[0] if stages else None}
        f.write(json.dumps(rec) + '\n')
        f.flush()
        print(f"{a.name} {rec['cfg']} r{a.round} step {i} page {p}: first {rec['first_ms']} ms, "
              f"rest {sorted(ms[1:])[len(ms[1:]) // 2] if ms[1:] else None} ms", flush=True)
host.send_signal(signal.SIGTERM)
try:
    host.wait(20)
except subprocess.TimeoutExpired:
    host.kill()
if os.environ.get('JUMPS_LOG'):
    with open(os.environ['JUMPS_LOG'], 'a') as lf:
        lf.write(f'=== host {a.cfg} r{a.round}\n')
        lf.write(''.join(l for l in open(f'{W}/h.err') if l.startswith('[kw]')))
