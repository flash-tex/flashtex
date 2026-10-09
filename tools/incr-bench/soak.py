#!/usr/bin/env python3
"""soak.py ENGINE DOC [options]: a long editing session against one resident host, to show that its
memory plateaus instead of growing (DESIGN.md §5.2's retention budget, §1.2; lanes P4-MEMORY and
MEMORY-SAFETY, docs/evidence/mem-soak-2026-10-04/).

Starts `$INCR_BENCH_DIR/ENGINE/flashtex-host --socket` (mkeng.sh's layout, FLASHTEX_MEMSTAT=1) on a
private copy of the document, speaks display-list-v3 to it as the app does (HELLO, then
`incremental` COMPILEs with `edits` and a `viewport`, one after the previous DONE plus --gap-ms),
and sends --edits keystrokes. DOC is a generated document ($INCR_BENCH_DIR/docs/DOC/main.tex,
mkdocs.py) or, with --src DIR --main FILE, a project directory (copied, never edited in place).

The edits are an editor's: at --sites places spread across the body's prose lines (in --files, or
every .tex file of the project), mostly staying at one place for a while, bursts of
  type     1-6 letters typed onto a word one keystroke each, then (85 %) backspaced one by one,
  phrase   3-12 words pasted after a word (the paragraph reflows), then (85 %) deleted again,
and every --revert-every keystrokes whatever is still inserted is taken back, one keystroke per
place (an undo), and the session idles --idle-s seconds (the host's idle work runs then). So the
document keeps returning to states it had before: a host that keeps per-edit state grows, one
that bounds it plateaus.

Samples the host's memory every --sample-s seconds (macOS phys_footprint via proc_pid_rusage, as
memstat::rss; Linux VmRSS) and records each DONE's `mem` (the checkpoint layer's parts). Writes
OUT.jsonl (one line per keystroke: kind, mode, ms, status, mem), OUT-samples.jsonl (t, keystroke,
bytes) and a summary line (also printed). Two measures, each at every DONE:
  footprint   what the system charges the process (Activity Monitor's "Memory"). It moves with
              the undo logs (retention and convergence drop and add checkpoints: +-40 MB on
              plain-120) and, on a machine short of memory, with what the system compresses or
              swaps out, which can hide a leak
  heap        malloc's bytes in use less the undo logs it holds (`mem.malloc_in_use + mem.log_mapped -
              mem.sealed_bytes`: the logs' large blocks are mappings of their own, `logalloc`):
              everything allocated that is not the budgeted logs, Rust's and the C libraries'
              alike, resident or not. It must not grow
For each: warm (the median over the second half of warm-up, --warmup keystrokes, default 10 %),
warm_peak (the highest up to the end of warm-up), end (the median of the last 5 %), and slope (the
least-squares slope over the second half, MB per 100 edits); `growers` are the `mem` parts that
grew the most over the second half.
Gate (exit 1): heap slope > --max-heap-slope or heap end > heap warm + --heap-const-mb; footprint
slope > --max-slope or footprint end > warm_peak * --max-ratio + --const-mb (each when given).
Exit 2: the host died or a compile failed to answer. The host is stopped by its PID (never by name).

  python3 tools/incr-bench/soak.py soak plain-120 --edits 1000
  python3 tools/incr-bench/soak.py soak infdesc --src ~/copy/infdesc --main infdesc.tex \\
      --files 'book/*/*.tex' --edits 1000
"""
import argparse
import ctypes
import glob
import json
import os
import random
import re
import shutil
import signal
import socket
import statistics
import struct
import subprocess
import sys
import tempfile
import threading
import time

BASE = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')

HELLO, COMPILE, BYE = 0x01, 0x02, 0x04
H_DONE, H_ERROR = 0x49, 0x4A

WORDS = ('lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor '
         'incididunt ut labore et dolore magna aliqua enim ad minim veniam quis nostrud').split()
# Lines that hold more than prose: an edit there could change a label, a file name or verbatim.
SKIP = re.compile(r'\\(label|ref|eqref|cref|Cref|cite|index|begin|end|input|include|verb|lstinline|'
                  r'url|href|item|section|subsection|chapter|part|caption|def|newcommand|renewcommand|'
                  r'usepackage|includegraphics|draw|node|path|foreach|hypersetup|bookmark)\b|[#&@~]|\\\\')


def footprint_fn():
    """bytes(pid): the process's footprint now (macOS phys_footprint; Linux VmRSS)."""
    if sys.platform == 'darwin':
        lib = ctypes.CDLL('/usr/lib/libproc.dylib')
        lib.proc_pid_rusage.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_void_p]

        def fp(pid):
            buf = (ctypes.c_uint64 * 64)()
            if lib.proc_pid_rusage(pid, 4, buf) != 0:  # RUSAGE_INFO_V4
                return 0
            return buf[9]  # ri_phys_footprint (memstat::rss)
        return fp

    def rss(pid):
        try:
            with open(f'/proc/{pid}/status') as f:
                for line in f:
                    if line.startswith('VmRSS:'):
                        return int(line.split()[1]) * 1024
        except OSError:
            pass
        return 0
    return rss


class Conn:
    def __init__(self, path):
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.connect(path)
        self.buf = b''

    def send(self, kind, obj):
        body = json.dumps(obj).encode()
        self.s.sendall(struct.pack('<IB', len(body) + 1, kind) + body)

    def _need(self, n):
        while len(self.buf) < n:
            chunk = self.s.recv(1 << 20)
            if not chunk:
                raise EOFError('host closed the connection')
            self.buf += chunk

    def frame(self):
        self._need(5)
        n, kind = struct.unpack('<IB', self.buf[:5])
        self._need(4 + n)
        body = self.buf[5:4 + n]
        self.buf = self.buf[4 + n:]
        return kind, body

    def until_done(self, cid):
        while True:
            kind, body = self.frame()
            if kind == H_DONE:
                d = json.loads(body)
                if d.get('id') == cid and d.get('cause') is None:
                    return d
            elif kind == H_ERROR:
                raise RuntimeError(f'host ERROR: {body[:300]!r}')


class Site:
    """A place in a file where the session types: the text there is original[:pos] + extra +
    original[pos:]."""

    def __init__(self, path, pos, frac):
        self.path, self.pos, self.frac = path, pos, frac
        self.extra = ''
        self.page = None  # learnt from the first DONE's restart_page


def find_sites(root, files, n, rng):
    cands = []
    for rel in files:
        raw = open(os.path.join(root, rel), 'rb').read()
        try:
            text = raw.decode('utf-8')
        except UnicodeDecodeError:
            continue
        off = 0
        body = '\\begin{document}' not in text  # an \input file is all body
        for line in text.split('\n'):
            lb = len(line.encode()) + 1
            if '\\begin{document}' in line:
                body = True
            s = line.strip()
            if (body and len(s) >= 40 and s[0] not in '\\%{}[]$' and not SKIP.search(line)
                    and '%' not in line and line.count('$') % 2 == 0):
                # the end of a lowercase word outside inline math, followed by a space
                dollars = 0
                for i, ch in enumerate(line):
                    if ch == '$':
                        dollars += 1
                    m = re.match(r'[a-z]{4,}(?= [a-z])', line[i:])
                    if (m and dollars % 2 == 0 and (i == 0 or not line[i - 1].isalpha())
                            and (i == 0 or line[i - 1] != '\\')):
                        end = i + m.end()
                        cands.append((rel, off + len(line[:end].encode())))
                        break
            off += lb
    if not cands:
        sys.exit('soak: no prose lines to edit')
    # spread over the document: n evenly spaced candidates (files in the given order)
    step = len(cands) / n
    picked = [cands[int(i * step + rng.random() * step * 0.5)] for i in range(n)]
    return [Site(p, o, i / n) for i, (p, o) in enumerate(picked)]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('engine')
    ap.add_argument('doc')
    ap.add_argument('--src', help='a project directory to copy (default $INCR_BENCH_DIR/docs/DOC)')
    ap.add_argument('--main', default='main.tex')
    ap.add_argument('--files', default='', help='globs (comma-separated, relative to the project) of '
                    'the files to edit, in document order (default: the main file)')
    ap.add_argument('--edits', type=int, default=1000)
    ap.add_argument('--sites', type=int, default=16)
    ap.add_argument('--gap-ms', type=int, default=120)
    ap.add_argument('--revert-every', type=int, default=100)
    ap.add_argument('--idle-s', type=float, default=5.0)
    ap.add_argument('--sample-s', type=float, default=1.0)
    ap.add_argument('--warmup', type=int, default=0, help='keystrokes of warm-up (default 10 %%)')
    ap.add_argument('--seed', type=int, default=1)
    ap.add_argument('--host-args', default=os.environ.get('SOAK_HOSTARGS', ''))
    ap.add_argument('--limit-gb', type=float, default=8.0, help='kill the host above this footprint')
    ap.add_argument('--max-slope', type=float, default=0.0, help='gate: MB per 100 edits (0: no gate)')
    ap.add_argument('--max-ratio', type=float, default=1.15)
    ap.add_argument('--const-mb', type=float, default=0.0)
    ap.add_argument('--max-heap-slope', type=float, default=0.0,
                    help='gate: heap MB per 100 edits over the second half (0: no gate)')
    ap.add_argument('--heap-const-mb', type=float, default=0.0,
                    help='gate: heap end at most warm + this (needs --max-heap-slope)')
    ap.add_argument('--out', default='')
    ap.add_argument('--timeout', type=int, default=7200, help='seconds for the whole run')
    ap.add_argument('--keep', action='store_true', help='keep the work directory')
    ap.add_argument('--vmmap-every', type=int, default=0,
                    help="macOS: save `vmmap -summary` of the host every N keystrokes (diagnosis)")
    a = ap.parse_args()

    def timed_out(*_):
        raise TimeoutError(f'{a.timeout} s')

    signal.signal(signal.SIGALRM, timed_out)  # (so that the host is stopped below)
    signal.alarm(a.timeout)
    E = f'{BASE}/{a.engine}'
    outdir = f'{BASE}/soak'
    os.makedirs(outdir, exist_ok=True)
    out = a.out or f'{outdir}/{a.doc}-{a.engine}'
    W = f'{outdir}/work-{a.doc}-{a.engine}-{os.getpid()}'
    shutil.rmtree(W, ignore_errors=True)
    src = a.src or f'{BASE}/docs/{a.doc}'
    shutil.copytree(src, f'{W}/doc', ignore=shutil.ignore_patterns('.git', '.flashtex'))
    root = f'{W}/doc'
    os.makedirs(f'{W}/out')
    files = []
    for g in [x for x in a.files.split(',') if x] or [a.main]:
        files += sorted(os.path.relpath(p, root) for p in glob.glob(os.path.join(root, g)))
    rng = random.Random(a.seed)
    sites = find_sites(root, files, a.sites, rng)
    orig = {s.path: open(os.path.join(root, s.path), 'rb').read() for s in sites}

    sock = f'{W}/h.sock'
    sockdir = None
    if len(sock) > 100:  # sun_path holds 104 bytes on macOS
        sockdir = tempfile.mkdtemp(prefix='soak.', dir='/tmp')
        sock = f'{sockdir}/h.sock'
    env = dict(os.environ, FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{BASE}/fmt-{a.engine}',
               SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1', FLASHTEX_MEMSTAT='1')
    herr = open(f'{out}.host-stderr', 'w')
    hout = open(f'{W}/h.out', 'w')
    host = subprocess.Popen([f'{E}/flashtex-host', '--socket', sock, '--s0-cache', f'{W}/s0']
                            + a.host_args.split(), env=env, stdout=hout, stderr=herr)
    fp = footprint_fn()
    state = {'edits': 0, 'killed': False, 'samples': []}
    stop = threading.Event()
    t0 = time.time()

    def watch():
        while not stop.is_set() and host.poll() is None:
            b = fp(host.pid)
            state['samples'].append((round(time.time() - t0, 2), state['edits'], b))
            if b > a.limit_gb * 2**30:
                state['killed'] = True
                os.kill(host.pid, signal.SIGKILL)
                break
            stop.wait(a.sample_s)

    threading.Thread(target=watch, daemon=True).start()
    for _ in range(1200):
        if 'listening' in open(f'{W}/h.out').read() or host.poll() is not None:
            break
        time.sleep(0.05)
    recs = []
    failed = None
    try:
        c = Conn(sock)
        c.send(HELLO, {'protocol': 'display-list-v3', 'version': [3, 2], 'client': 'soak.py'})
        cid = 0

        def compile_(edits, viewport, kind):
            nonlocal cid
            cid += 1
            req = {'id': cid, 'root': root, 'main': a.main, 'output_dir': f'{W}/out',
                   'incremental': True, 'edits': edits}
            if viewport is not None:
                req['viewport'] = viewport
            ts = time.time()
            c.send(COMPILE, req)
            d = c.until_done(cid)
            ms = (time.time() - ts) * 1e3
            return d, ms

        d, ms = compile_([], None, 'open')
        pages = d.get('pages') or 1
        recs.append({'i': 0, 'kind': 'open', 'ms': round(ms, 1), 'mode': d.get('mode'),
                     'status': d.get('status'), 'pages': pages, 'mem': d.get('mem')})
        print(f'soak: opened {a.doc}: {pages} pages, {ms / 1e3:.1f} s, status {d.get("status")}',
              file=sys.stderr)

        def cur(s):
            return s.pos + sum(len(t.extra.encode()) for t in sites if t.path == s.path and t.pos < s.pos)

        def splice(s, ins='', dele=0):
            o = cur(s) + len(s.extra.encode())
            if dele:
                cut = s.extra[-dele:]
                s.extra = s.extra[:-dele]
                return {'path': s.path, 'offset': o - len(cut.encode()), 'delete': len(cut.encode()),
                        'insert': ''}
            s.extra += ins
            return {'path': s.path, 'offset': o, 'insert': ins, 'delete': 0}

        def plan():
            """The keystrokes, as (site, kind, insert, delete) lazily, one burst at a time."""
            si = 0
            n = 0
            while True:
                if rng.random() < 0.3:
                    si = rng.randrange(len(sites))
                s = sites[si]
                if rng.random() < 0.8:
                    k = rng.randint(1, 6)
                    for _ in range(k):
                        yield s, 'type', rng.choice('etaoinshrdlu'), 0
                    if rng.random() < 0.85:
                        for _ in range(k):
                            yield s, 'backspace', '', 1
                else:
                    ph = ' ' + ' '.join(rng.choice(WORDS) for _ in range(rng.randint(3, 12)))
                    yield s, 'phrase', ph, 0
                    if rng.random() < 0.85:
                        yield s, 'unphrase', '', len(ph)
                n += 1

        gen = plan()
        i = 0
        while i < a.edits:
            batch = []
            if a.revert_every and i and i % a.revert_every == 0:
                for s in sites:
                    if s.extra:
                        batch.append((s, 'revert', '', len(s.extra)))
            batch = batch or [next(gen)]
            for s, kind, ins, dele in batch:
                if dele > len(s.extra):  # a revert took it back already
                    continue
                e = splice(s, ins, dele)
                vp = s.page if s.page is not None else min(pages - 1, int(s.frac * pages))
                d, ms = compile_([e], vp, kind)
                i += 1
                state['edits'] = i
                if s.page is None and d.get('restart_page') is not None:
                    s.page = min(d['restart_page'], (d.get('pages') or pages) - 1)
                pages = d.get('pages') or pages
                recs.append({'i': i, 'kind': kind, 'site': sites.index(s), 'ms': round(ms, 1),
                             'mode': d.get('mode'), 'status': d.get('status'), 'pages': d.get('pages'),
                             'restart_page': d.get('restart_page'), 'converged_at': d.get('converged_at'),
                             'typeset_pages': d.get('typeset_pages'), 'mem': d.get('mem')})
                if a.vmmap_every and i % a.vmmap_every == 0:
                    with open(f'{out}-vmmap-{i}.txt', 'w') as f:
                        subprocess.run(['vmmap', '-summary', str(host.pid)], stdout=f,
                                       stderr=subprocess.STDOUT)
                if i % 50 == 0:
                    m = d.get('mem') or {}
                    print(f'soak: {i} edits, {fp(host.pid) / 2**20:.0f} MB, logs '
                          f'{(m.get("sealed_bytes") or 0) / 2**20:.0f} MB, {ms:.0f} ms', file=sys.stderr)
                time.sleep(a.gap_ms / 1e3)
                if i >= a.edits:
                    break
            if kind == 'revert':
                time.sleep(a.idle_s)
        # the files must hold what this driver thinks it typed
        for p, o in orig.items():
            want = o
            for s in sorted((s for s in sites if s.path == p), key=lambda s: -s.pos):
                want = want[:s.pos] + s.extra.encode() + want[s.pos:]
            if open(os.path.join(root, p), 'rb').read() != want:
                failed = f'{p}: the file differs from the edits sent'
        c.send(BYE, {})
    except (EOFError, RuntimeError, OSError, TimeoutError) as e:
        failed = f'{type(e).__name__}: {e}'
    stop.set()
    if host.poll() is None:
        host.send_signal(signal.SIGTERM)
        try:
            host.wait(10)
        except subprocess.TimeoutExpired:
            host.kill()
            host.wait()
    with open(f'{out}.jsonl', 'w') as f:
        for r in recs:
            f.write(json.dumps(r) + '\n')
    with open(f'{out}-samples.jsonl', 'w') as f:
        for t, k, b in state['samples']:
            f.write(json.dumps({'t': t, 'edits': k, 'bytes': b}) + '\n')
    summ = summarise(a, recs, state['samples'])
    summ.update({'doc': a.doc, 'engine': a.engine, 'killed_at_limit': state['killed'], 'failed': failed,
                 'seconds': round(time.time() - t0, 1),
                 'status': {k: sum(1 for r in recs if r['status'] == k) for k in {r['status'] for r in recs}},
                 'modes': {k: sum(1 for r in recs if r['mode'] == k) for k in {r['mode'] for r in recs}}})
    print(json.dumps(summ))
    with open(f'{out}.jsonl', 'a') as f:
        f.write(json.dumps(summ) + '\n')
    if sockdir:
        shutil.rmtree(sockdir, ignore_errors=True)
    if not a.keep:
        shutil.rmtree(W, ignore_errors=True)
    if failed or state['killed']:
        sys.exit(2)
    if not summ.get('pass', True):
        sys.exit(1)


def slope(xs, ys):
    mx, my = statistics.fmean(xs), statistics.fmean(ys)
    den = sum((x - mx) ** 2 for x in xs)
    return sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / den if den else 0.0


def summarise(a, recs, samples):
    MB = 2**20
    ed = [r for r in recs if r['i'] > 0 and r.get('mem') and r['mem'].get('rss')]
    n = len(ed)
    s = {'summary': True, 'edits': n}
    if n < 10:
        s['pass'] = False
        return s
    warm = a.warmup or max(1, n // 10)
    fps = [b for _, k, b in samples if b]
    warm_s = [b for _, k, b in samples if b and k <= warm] or fps[:1]
    second = [r for r in ed if r['i'] > n // 2]
    tail = [r['mem']['rss'] for r in ed[-max(1, n // 20):]]
    s.update({
        'warmup_edits': warm,
        'open_mb': round(recs[0]['mem']['rss'] / MB, 1) if recs[0].get('mem') else None,
        'warm_peak_mb': round(max(warm_s) / MB, 1),
        'mid_mb': round(statistics.median(r['mem']['rss'] for r in ed[n // 2 - n // 40: n // 2 + 1]) / MB, 1),
        'end_mb': round(statistics.median(tail) / MB, 1),
        'peak_mb': round(max(fps) / MB, 1),
        'lifetime_peak_mb': round(max(r['mem'].get('rss_peak', 0) for r in ed) / MB, 1),
        'slope_mb_per_100': round(slope([r['i'] for r in second], [r['mem']['rss'] for r in second]) / MB * 100, 3),
        'slope_mb_per_100_all': round(slope([r['i'] for r in ed if r['i'] > warm],
                                            [r['mem']['rss'] for r in ed if r['i'] > warm]) / MB * 100, 3),
        # the footprint less the undo logs (bounded by the budget, and up and down with
        # retention and convergence): what is left should not grow at all
        'slope_nolog_mb_per_100': round(slope([r['i'] for r in second],
                                              [r['mem']['rss'] - r['mem'].get('sealed_bytes', 0)
                                               for r in second]) / MB * 100, 3),
        'edit_ms_p50': statistics.median(r['ms'] for r in ed),
        'edit_ms_p95': sorted(r['ms'] for r in ed)[int(0.95 * (n - 1))],
    })
    def nolog(m):
        return m['malloc_in_use'] + m.get('log_mapped', 0) - m.get('sealed_bytes', 0)

    heap = [(r['i'], nolog(r['mem']) / MB) for r in ed if r['mem'].get('malloc_in_use')]
    if len(heap) == n:
        hw = [v for i, v in heap if warm // 2 < i <= warm] or [heap[0][1]]
        h2 = [(i, v) for i, v in heap if i > n // 2]
        s.update({
            'heap_open_mb': round(nolog(recs[0]['mem']) / MB, 1)
            if recs[0].get('mem', {}).get('malloc_in_use') else None,
            'heap_warm_mb': round(statistics.median(hw), 1),
            'heap_end_mb': round(statistics.median(v for _, v in heap[-max(1, n // 20):]), 1),
            'heap_max_mb': round(max(v for _, v in heap), 1),
            'heap_slope_mb_per_100': round(slope([i for i, _ in h2], [v for _, v in h2]) * 100, 3),
        })
    # the parts that grew over the second half
    a0, a1 = second[0]['mem'], ed[-1]['mem']
    grow = sorted(((k, a1[k] - a0.get(k, 0)) for k in a1 if isinstance(a1[k], int)), key=lambda kv: -abs(kv[1]))
    s['growers'] = {k: v for k, v in grow[:12] if v}
    s['last_mem'] = a1
    fails = []
    if a.max_slope and s['slope_mb_per_100'] > a.max_slope:
        fails.append('footprint slope')
    if a.max_slope and s['end_mb'] > s['warm_peak_mb'] * a.max_ratio + a.const_mb:
        fails.append('footprint end')
    if a.max_heap_slope:
        if 'heap_slope_mb_per_100' not in s:
            fails.append('no malloc_in_use in DONE.mem')
        else:
            if s['heap_slope_mb_per_100'] > a.max_heap_slope:
                fails.append('heap slope')
            if s['heap_end_mb'] > s['heap_warm_mb'] + a.heap_const_mb:
                fails.append('heap end')
    s['pass'] = not fails
    s['failed_checks'] = fails
    s['gate'] = {k: v for k, v in (('max_slope', a.max_slope), ('max_ratio', a.max_ratio),
                                   ('const_mb', a.const_mb), ('max_heap_slope', a.max_heap_slope),
                                   ('heap_const_mb', a.heap_const_mb)) if v} or None
    return s


if __name__ == '__main__':
    main()
