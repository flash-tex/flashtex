#!/usr/bin/env python3
"""fpsession.py ENGINE_DIR DOC_DIR OUT [--env K=V ...] [--step AT:KIND:KEYS ...]

One production flashtex-host (FLASHTEX_MEMSTAT=1) on a fresh copy of DOC_DIR (main.tex): a cold
open, then each step (`dl3-keys --at AT --kind KIND --keys KEYS`, 300 ms apart), then 8 s idle and
an unchanged open (the steady record), as the research's memr.py does
(docs/evidence/mem-research-2026-10-06/, allocator A/B). Portable: it samples the host every
100 ms by `proc_pid_rusage` on macOS (`ri_phys_footprint`, what Activity Monitor shows; and the
kernel's lifetime maximum, `ri_lifetime_max_phys_footprint`) and by /proc VmRSS/VmHWM on Linux.
--env sets the host's environment (e.g. FLASHTEX_NO_LOG_MAPS=1 for the A side).

Writes OUT.json (peak sampled and lifetime, steady, per phase the DONE `mem` and the median
edited-page and keystroke instructions, macOS `footprint -p` at the steady state) and prints one
summary line.
"""
import argparse
import json
import os
import shutil
import statistics
import subprocess
import sys
import threading
import time

S = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', '..', 'tools', 'incr-bench')
sys.path.insert(0, os.path.abspath(S))
import t7  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument('eng')
ap.add_argument('doc')
ap.add_argument('out')
ap.add_argument('--env', action='append', default=[])
ap.add_argument('--step', action='append', default=[])
ap.add_argument('--idle', type=float, default=8.0)
a = ap.parse_args()

work = f'{a.out}-work'
shutil.rmtree(work, ignore_errors=True)
shutil.copytree(a.doc, work, symlinks=True)
os.makedirs(f'{work}/out', exist_ok=True)
sock = f'/tmp/fps-{os.getpid()}.sock'
os.environ['FLASHTEX_MEMSTAT'] = '1'
for kv in a.env:
    k, _, v = kv.partition('=')
    os.environ[k] = v

MAC = sys.platform == 'darwin'
if MAC:
    import ctypes
    _libc = ctypes.CDLL(None)
    _buf = (ctypes.c_uint64 * 64)()

    def mem(pid):
        """(phys_footprint, lifetime max) of pid: rusage_info_v4 as u64s, after the 16-byte uuid."""
        if _libc.proc_pid_rusage(pid, 4, ctypes.byref(_buf)) != 0:
            raise OSError('proc_pid_rusage')
        return _buf[9], _buf[30]
else:
    def mem(pid):
        v = {}
        for line in open(f'/proc/{pid}/status'):
            k, _, rest = line.partition(':')
            if k in ('VmRSS', 'VmHWM'):
                v[k] = int(rest.split()[0]) * 1024
        return v['VmRSS'], v['VmHWM']

h = t7.Host(os.path.abspath(a.eng), sock, f'{a.out}-s0', f'{a.out}.host-stderr', [])
pid = h.p.pid
state = dict(phase='start', peak={}, stop=False, life=0)


def sampler():
    while not state['stop']:
        try:
            r, life = mem(pid)
        except (OSError, KeyError):
            break
        ph = state['phase']
        state['peak'][ph] = max(state['peak'].get(ph, 0), r)
        state['life'] = max(state['life'], life)
        time.sleep(0.1)


threading.Thread(target=sampler, daemon=True).start()


def dl3(args, phase):
    state['phase'] = phase
    cmd = [f'{t7.S}/to.sh', '3600', f'{a.eng}/dl3-keys', '--socket', sock, '--root', work,
           '--main', 'main.tex', '--output-dir', f'{work}/out'] + args
    t = time.time()
    p = subprocess.run(cmd, capture_output=True, text=True)
    recs = []
    for line in p.stdout.splitlines():
        if line.startswith('{'):
            try:
                recs.append(json.loads(line))
            except ValueError:
                pass
    st = [(r.get('host') or {}).get('stages') or {} for r in recs]
    ed = [s['edited_instr_k'] for s in st if s.get('edited_instr_k') is not None]
    ik = [s['instr_k'] for s in st if s.get('instr_k') is not None]
    if phase != 'open':  # the first keystroke of a step restarts far away: left out
        ed, ik = ed[1:] or ed, ik[1:] or ik
    mems = [(r.get('host') or {}).get('mem') for r in recs]
    mems = [m for m in mems if m]
    row = dict(name=phase, rc=p.returncode, secs=round(time.time() - t, 1), n=len(recs),
               peak=state['peak'].get(phase, 0),
               edited_instr_m=statistics.median(ed) / 1e3 if ed else None,
               key_instr_m=statistics.median(ik) / 1e3 if ik else None,
               instr_total_g=sum(ik) / 1e6 if ik else None,
               mem=mems[-1] if mems else None)
    print(f"{phase}: rc {p.returncode} {row['secs']}s peak {row['peak'] >> 20} MB", flush=True)
    if p.returncode != 0:
        print(p.stderr[-800:], file=sys.stderr)
    return row


res = dict(env=a.env, doc=a.doc, phases=[])
try:
    res['phases'].append(dl3(['--keys', '0', '--at', '0.5'], 'open'))
    for s in a.step:
        at, kind, keys = s.split(':')
        res['phases'].append(dl3(['--kind', kind, '--at', at, '--keys', keys, '--gap-ms', '300'], s))
    state['phase'] = 'idle'
    time.sleep(a.idle)
    res['phases'].append(dl3(['--keys', '0', '--at', '0.5'], 'steady'))
    time.sleep(1)
    res['steady'], life = mem(pid)
    state['life'] = max(state['life'], life)
    if MAC:
        res['footprint'] = subprocess.run(['footprint', '-p', str(pid)], capture_output=True,
                                          text=True).stdout[-6000:]
finally:
    state['stop'] = True
    h.stop()
res['peak'] = max(state['peak'].values()) if state['peak'] else 0
res['lifetime_peak'] = state['life']
json.dump(res, open(f'{a.out}.json', 'w'), indent=1)
shutil.rmtree(work, ignore_errors=True)
op = res['phases'][0]
eds = ' '.join(f"{p['name']}={p['edited_instr_m']:.1f}M" for p in res['phases'][1:-1]
               if p['edited_instr_m'] is not None)
print(f"SUMMARY {os.path.basename(a.out)} peak {res['peak'] >> 20} MB lifetime {res['lifetime_peak'] >> 20} MB "
      f"steady {res.get('steady', 0) >> 20} MB open {op['instr_total_g'] or 0:.1f} G | edited {eds}", flush=True)
