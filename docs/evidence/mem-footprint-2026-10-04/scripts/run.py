#!/usr/bin/env python3
"""run.py TAG ENGINE DOC [--edits FILE:PAGE,...] [--keys N] [--host-args ...]

MEM-FOOTPRINT phase 1 driver: start ENGINE's flashtex-host on a fresh copy of
/tmp/mfp/docs/DOC (FLASHTEX_MEMSTAT=1), open it (dl3-keys --keys 0), type --keys
keystrokes at each FILE:PAGE in turn, idle --idle s, then record the steady state:
`footprint -p`, `vmmap -summary`, and the host's `mem` (from a final unchanged open).
Writes /tmp/mfp/out/TAG-DOC.{jsonl,footprint,vmmap}; prints a summary line.
"""
import argparse, json, os, signal, subprocess, sys, threading, time

ap = argparse.ArgumentParser()
ap.add_argument('tag'); ap.add_argument('engine'); ap.add_argument('doc')
ap.add_argument('--main', default='main.tex')
ap.add_argument('--edits', default='')
ap.add_argument('--keys', type=int, default=8)
ap.add_argument('--gap-ms', type=int, default=300)
ap.add_argument('--idle', type=float, default=8.0)
ap.add_argument('--host-args', default='')
ap.add_argument('--kind', default='letter')
ap.add_argument('--limit-gb', type=float, default=8.0)
a = ap.parse_args()

IB = '/tmp/mfp/ib'
E = f'{IB}/{a.engine}'
OUT = '/tmp/mfp/out'; os.makedirs(OUT, exist_ok=True)
W = f'/tmp/mfp/w/{a.tag}-{a.doc}'
subprocess.run(['rm', '-rf', W], check=True)
os.makedirs(f'{W}/out')
subprocess.run(['cp', '-R', f'/tmp/mfp/docs/{a.doc}', f'{W}/src'], check=True)
sock = f'{W}/h.sock'
env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{IB}/fmt-{a.engine}',
           SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1', FLASHTEX_MEMSTAT='1')
herr = open(f'{OUT}/{a.tag}-{a.doc}.host-stderr', 'w')
hout = open(f'{W}/h.out', 'w')
host = subprocess.Popen([f'{E}/flashtex-host', '--socket', sock, '--s0-cache', f'{W}/s0']
                        + a.host_args.split(), env=env, stdout=hout, stderr=herr)
peak = {'v': 0, 'killed': False}
stop = threading.Event()


def fp(pid):
    r = subprocess.run(['ps', '-o', 'rss=', '-p', str(pid)], capture_output=True, text=True)
    try:
        return int(r.stdout.strip() or 0) * 1024
    except ValueError:
        return 0


def watch():
    while not stop.is_set() and host.poll() is None:
        r = fp(host.pid)
        peak['v'] = max(peak['v'], r)
        if r > a.limit_gb * 2**30:
            peak['killed'] = True
            os.kill(host.pid, signal.SIGKILL)
            break
        time.sleep(0.5)


threading.Thread(target=watch, daemon=True).start()
for _ in range(600):
    if 'listening' in open(f'{W}/h.out').read():
        break
    time.sleep(0.05)

recs = []


def keys(extra):
    args = [f'{E}/dl3-keys', '--socket', sock, '--root', f'{W}/src', '--main', a.main,
            '--output-dir', f'{W}/out', '--gap-ms', str(a.gap_ms), '--no-viewport'] + extra
    t = time.time()
    r = subprocess.run(args, capture_output=True, text=True)
    out = []
    for line in r.stdout.splitlines():
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    if r.returncode != 0:
        print('dl3-keys failed', extra, r.stderr[-500:], file=sys.stderr)
    return out, time.time() - t


t0 = time.time()
o, dt = keys(['--keys', '0'])
recs += [dict(r, phase='open') for r in o]
print(f'open: {dt:.1f}s, {len(o)} compiles', file=sys.stderr)
edited = []
for e in [x for x in a.edits.split(',') if x]:
    parts = e.split(':')
    f, p = parts[0], parts[1]
    extra = ['--keys', str(a.keys), '--page', p, '--kind', a.kind]
    if len(parts) > 2:
        extra += ['--line', parts[2]]
    if f != a.main:
        extra += ['--edit', f]
    o, dt = keys(extra)
    recs += [dict(r, phase=f'edit {e}') for r in o]
    edited += [r['edited_page_ms'] for r in o if isinstance(r.get('edited_page_ms'), (int, float))]
    print(f'edit {e}: {dt:.1f}s', file=sys.stderr)
time.sleep(a.idle)
if os.environ.get('FLASHTEX_DUMP_LOGS'):
    open(os.environ['FLASHTEX_DUMP_LOGS'] + '.go', 'w').close()
o, _ = keys(['--keys', '0'])
recs += [dict(r, phase='final') for r in o]
mem = {}
for r in recs:
    m = (r.get('host') or {}).get('mem')
    if m:
        mem = m
pid = host.pid
fpt = subprocess.run(['footprint', '-p', str(pid)], capture_output=True, text=True).stdout
vm = subprocess.run(['vmmap', '-summary', str(pid)], capture_output=True, text=True).stdout
open(f'{OUT}/{a.tag}-{a.doc}.footprint', 'w').write(fpt)
open(f'{OUT}/{a.tag}-{a.doc}.vmmap', 'w').write(vm)
if os.environ.get('PROBE'):
    subprocess.run(os.environ['PROBE'].replace('{pid}', str(pid)).replace('{out}', f'{OUT}/{a.tag}-{a.doc}'), shell=True)
stop.set()
host.terminate()
try:
    host.wait(10)
except subprocess.TimeoutExpired:
    host.kill()
with open(f'{OUT}/{a.tag}-{a.doc}.jsonl', 'w') as f:
    for r in recs:
        f.write(json.dumps(r) + '\n')
edited.sort()
pct = lambda q: edited[min(len(edited) - 1, int(q * len(edited)))] if edited else None
s = {'tag': a.tag, 'doc': a.doc, 'rss_steady': mem.get('rss'), 'rss_peak': mem.get('rss_peak'),
     'ps_peak': peak['v'], 'killed': peak['killed'], 'checkpoints': mem.get('checkpoints'),
     'pages': mem.get('pages'), 'edited_p50': pct(0.5), 'edited_p95': pct(0.95), 'n': len(edited),
     'secs': round(time.time() - t0, 1)}
print(json.dumps(s))
open(f'{OUT}/summary.jsonl', 'a').write(json.dumps(s) + '\n')
