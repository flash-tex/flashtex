#!/usr/bin/env python3
"""mm.py ENGINE DOC MODE [--src DIR --main F] [--edits SPEC;SPEC] [--keys N] [--idle S]

MEM-MODES driver: start ENGINE's flashtex-host --profile MODE on a fresh copy of DOC, open it,
type --keys letters (inserted, 300 ms apart) at each edit spec (page[:line[:file]]), idle
--idle s (past the mode's keep-warm + trim windows), then a final unchanged open. Records:
phys_footprint peak (DONE.mem.rss_peak, the kernel's lifetime max) and steady (`footprint -p`
after the idle, and DONE.mem.rss of the final open), malloc in use / held, and the edited
page's host latency p50/p95 per key. One host at a time. Appends a JSON line to
/private/tmp/mb/out/summary.jsonl."""
import argparse, json, os, re, signal, statistics, subprocess, sys, threading, time

ap = argparse.ArgumentParser()
ap.add_argument('engine'); ap.add_argument('doc'); ap.add_argument('mode')
ap.add_argument('--src', default=''); ap.add_argument('--main', default='main.tex')
ap.add_argument('--edits', default='0')
ap.add_argument('--keys', type=int, default=8)
ap.add_argument('--gap-ms', type=int, default=300)
ap.add_argument('--idle', type=float, default=7.0)
ap.add_argument('--tag', default='')
ap.add_argument('--host-args', default='')
ap.add_argument('--limit-gb', type=float, default=8.0)
ap.add_argument('--env', default='')
ap.add_argument('--hook', default='')
ap.add_argument('--s0', default='')
a = ap.parse_args()
IB = '/private/tmp/mb/ib'
E = f'{IB}/{a.engine}'
OUT = '/private/tmp/mb/out'; os.makedirs(OUT, exist_ok=True)
tag = a.tag or a.engine
W = f'/private/tmp/mb/w/{tag}-{a.doc}-{a.mode}'
if not a.s0:
    subprocess.run(['rm', '-rf', W], check=True)
os.makedirs(f'{W}/out', exist_ok=True)
os.makedirs(f'{W}/src', exist_ok=True)
if a.src:
    subprocess.run(['cp', '-a', f'{a.src}/.', f'{W}/src'], check=True)
else:
    subprocess.run(['cp', '-a', f'/private/tmp/mb/src/{a.doc}/.', f'{W}/src'], check=True)
sock = f'{W}/h.sock'
env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{IB}/fmt-{a.engine}',
           SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1', FLASHTEX_MEMSTAT='1', TMPDIR='/private/tmp/mb/tmp')
for kv in [x for x in a.env.split(',') if x]:
    k, v = kv.split('=', 1); env[k] = v
herr = open(f'{W}/host-stderr', 'w')
hout = open(f'{W}/h.out', 'w')
host = subprocess.Popen([f'{E}/flashtex-host', '--socket', sock, '--s0-cache', a.s0 or f'{W}/s0',
                         '--profile', a.mode] + a.host_args.split(), env=env, stdout=hout, stderr=herr)
killed = {'v': False}
stop = threading.Event()


def watch():
    while not stop.is_set() and host.poll() is None:
        r = subprocess.run(['ps', '-o', 'rss=', '-p', str(host.pid)], capture_output=True, text=True)
        try:
            v = int(r.stdout.strip() or 0) * 1024
        except ValueError:
            v = 0
        if v > a.limit_gb * 2**30:
            killed['v'] = True
            os.kill(host.pid, signal.SIGKILL)
            break
        time.sleep(0.5)


threading.Thread(target=watch, daemon=True).start()
for _ in range(1200):
    if 'listening' in open(f'{W}/h.out').read():
        break
    time.sleep(0.05)


def keys(extra):
    args = [f'{E}/dl3-keys', '--socket', sock, '--root', f'{W}/src', '--main', a.main,
            '--output-dir', f'{W}/out', '--gap-ms', str(a.gap_ms), '--no-viewport'] + extra
    r = subprocess.run(args, capture_output=True, text=True)
    out = []
    for line in r.stdout.splitlines():
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    if r.returncode != 0:
        print('dl3-keys failed', extra, r.stderr[-400:], file=sys.stderr)
    return out


def footprint(pid):
    t = subprocess.run(['footprint', '-p', str(pid)], capture_output=True, text=True).stdout
    m = re.search(r'Footprint:\s*([\d.]+)\s*([KMG]B)', t)
    if not m:
        return None, t
    f = {'KB': 2**10, 'MB': 2**20, 'GB': 2**30}[m.group(2)]
    return int(float(m.group(1)) * f), t


t0 = time.time()
recs = []
o = keys(['--keys', '0'])
open_s = time.time() - t0
recs += [dict(r, phase='open') for r in o]
fp_open, _ = footprint(host.pid)
edited = []
instr = []
instr_all = []
cpu_all = []
for spec in [x for x in a.edits.split(';') if x]:
    p = spec.split(':')
    extra = ['--keys', str(a.keys), '--page', p[0], '--kind', 'letter']
    if len(p) > 1 and p[1]:
        extra += ['--line', p[1]]
    if len(p) > 2:
        extra += ['--edit', p[2], '--repeated']
    o = keys(extra)
    recs += [dict(r, phase=f'edit {spec}') for r in o]
    for r in o:
        if isinstance(r.get('edited_page_ms'), (int, float)):
            edited.append(r['edited_page_ms'])
        st = ((r.get('host') or {}).get('stages') or {})
        if 'edited_instr_k' in st:
            instr.append(st['edited_instr_k'] / 1000)
        if 'instr_k' in st and (r.get('host') or {}).get('mode') != 'unchanged':
            instr_all.append(st['instr_k'] / 1000)
            cpu_all.append(st.get('cpu'))
mem_after_keys = {}
for r in recs:
    m = (r.get('host') or {}).get('mem')
    if m:
        mem_after_keys = m
time.sleep(a.idle)
if a.hook:
    subprocess.run(a.hook.replace("{pid}", str(host.pid)).replace("{w}", W), shell=True)
fp_idle, fpt = footprint(host.pid)
open(f'{W}/footprint.txt', 'w').write(fpt)
vm = subprocess.run(['vmmap', '-summary', str(host.pid)], capture_output=True, text=True).stdout
open(f'{W}/vmmap.txt', 'w').write(vm)
vmf = subprocess.run(['vmmap', str(host.pid)], capture_output=True, text=True).stdout
open(f'{W}/vmmap-full.txt', 'w').write(vmf)
o = keys(['--keys', '0'])
recs += [dict(r, phase='final') for r in o]
mem = {}
for r in recs:
    m = (r.get('host') or {}).get('mem')
    if m:
        mem = m
stop.set()
host.terminate()
try:
    host.wait(10)
except subprocess.TimeoutExpired:
    host.kill()
with open(f'{W}/recs.jsonl', 'w') as f:
    for r in recs:
        f.write(json.dumps(r) + '\n')


def pct(v, q):
    v = sorted(v)
    return v[min(len(v) - 1, int(q * len(v)))] if v else None


MB = lambda x: round(x / 2**20, 1) if isinstance(x, (int, float)) else None
s = {'tag': tag, 'doc': a.doc, 'mode': a.mode, 'peak_mb': MB(mem.get('rss_peak')),
     'open_fp_mb': MB(fp_open), 'steady_fp_mb': MB(fp_idle), 'steady_final_mb': MB(mem.get('rss')),
     'after_keys_mb': MB(mem_after_keys.get('rss')),
     'malloc_in_use_mb': MB(mem.get('malloc_in_use')), 'malloc_held_mb': MB(mem.get('malloc_held')),
     'sealed_mb': MB(mem.get('sealed_bytes')), 'checkpoints': mem.get('checkpoints'),
     'pages': mem.get('pages'), 'n': len(edited), 'ed_p50': pct(edited, 0.5), 'ed_p95': pct(edited, 0.95),
     'instr_p50_M': pct(instr, 0.5), 'instr_p95_M': pct(instr, 0.95),
     'all_instr_p50_M': pct(instr_all, 0.5), 'all_cpu_p50_ms': pct([c for c in cpu_all if c is not None], 0.5),
     'open_instr_M': round(sum(((r.get('host') or {}).get('stages') or {}).get('instr_k', 0) for r in recs if r.get('phase') == 'open') / 1000, 1),
     'open_s': round(open_s, 2), 'res': {k[4:]: MB(v) for k, v in mem.items() if k.startswith('res_')}, 'killed': killed['v'], 'secs': round(time.time() - t0, 1)}
print(json.dumps(s))
open(f'{OUT}/summary.jsonl', 'a').write(json.dumps(s) + '\n')
