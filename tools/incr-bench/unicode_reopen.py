#!/usr/bin/env python3
"""Unicode mode's persisted S0 (lane UNICODE-POLISH; `--s0-cache`,
crates/flashtex-xetex/src/s0file.rs): reopen gates and timings.

usage: unicode_reopen.py HOST SRC [SRC...] [--out JSONL] [--cache DIR]

For each SRC (a .tex file, copied to DIR/src/main.tex; the output goes to
DIR/out, as the app lays a project out):

1. A host with an S0 cache compiles the document (cold) and saves its S0;
   the host ends.
2. Each scenario then starts a new host with the same cache and compiles:
   * `same`: nothing changed while closed: the compile must start from S0;
   * `body`: a sentence added in the body while closed: from S0;
   * `preamble`: a command defined in the preamble, on the line after
     `\\documentclass`, while closed: S0 refused;
   * `preamble_end`: the same, on the line before `\\begin{document}`: S0
     refused, or taken where it holds (an S0 whose read prefix of
     main.tex ends before that line goes on to read the new line itself;
     run 38256375339 saw this in l011, u03 and u04), as long as the
     compile equals a from-scratch one;
   * `font` (when DIR/src has a font file the document loads by file name):
     the font file changed while closed: S0 refused.
   Every such compile is compared with a from-scratch compile of the same
   directory by a host whose runs are processes of their own
   (`FLASHTEX_UNICODE_INPROCESS=0`): every output file byte for byte (a
   log's P-T1 accounting block apart), every frame but STARTED, and DONE's
   status, exit code and pages.
3. Timings: a cold open (a new host without the cache, the outputs of the
   last compile present) against a reopen from S0, each in a new host
   process: the first page and DONE as the client sees them from the
   COMPILE, and the host's instructions retired over the compile.

Exit status 1 when a compile differs or a scenario starts where it must not.
"""
import argparse
import ctypes
import json
import os
import re
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time

ap = argparse.ArgumentParser()
ap.add_argument('host')
ap.add_argument('src', nargs='+')
ap.add_argument('--out')
ap.add_argument('--trials', type=int, default=3, help='timed opens of each kind')
ap.add_argument('--clock', default='1760000000.5')
ap.add_argument('--profile', help='the measured hosts\' performance mode (FLASHTEX_PROFILE)')
ap.add_argument('--cache', help='one S0 cache for every document, as the app has (default: one per document); '
                'the first document of a format makes the base the others are deltas against')
a = ap.parse_args()

PAGE, DIAG, DONE, STARTED, TOOL = 0x45, 0x48, 0x49, 0x42, 0x4C
try:
    _libproc = ctypes.CDLL('/usr/lib/libproc.dylib')
except OSError:
    _libproc = None


class _RU(ctypes.Structure):
    _fields_ = [('uuid', ctypes.c_uint8 * 16)] + [('f%d' % k, ctypes.c_uint64) for k in range(40)]


def rusage(pid):
    if _libproc is None:
        return None
    r = _RU()
    if _libproc.proc_pid_rusage(pid, 4, ctypes.byref(r)) != 0:
        return None
    return r


def write_frame(s, k, body):
    b = body.encode() if isinstance(body, str) else body
    s.sendall(struct.pack('<IB', len(b) + 1, k) + b)


def read_frame(f):
    h = f.read(5)
    if len(h) < 5:
        return None
    n, k = struct.unpack('<IB', h)
    return k, f.read(n - 1)


class Host:
    def __init__(self, env, tag):
        self.sock = os.path.join(tempfile.gettempdir(), 'fxr-%d-%s.sock' % (os.getpid(), tag))
        try:
            os.unlink(self.sock)
        except OSError:
            pass
        self.errpath = os.path.join(tempfile.gettempdir(), 'fxr-%d-%s.err' % (os.getpid(), tag))
        self.err = open(self.errpath, 'w')
        self.p = subprocess.Popen([a.host, '--socket', self.sock, '--once', '--accept-timeout', '600'],
                                  stdout=subprocess.DEVNULL, stderr=self.err, env=env)
        while not os.path.exists(self.sock):
            if self.p.poll() is not None:
                sys.exit('%s: the host ended' % tag)
            time.sleep(0.002)
        # (the socket file exists from bind(), before listen(): a connect
        # in between is refused)
        t0 = time.time()
        while True:
            self.s = socket.socket(socket.AF_UNIX)
            try:
                self.s.connect(self.sock)
                break
            except ConnectionRefusedError:
                self.s.close()
                if self.p.poll() is not None or time.time() - t0 > 10:
                    raise
                time.sleep(0.005)
        self.f = self.s.makefile('rb')
        write_frame(self.s, 0x01, json.dumps({'protocol': 'display-list-v3', 'version': [3, 3],
                                              'client': 'incr-bench-unicode-reopen'}))
        read_frame(self.f)
        self.n = 0

    def compile(self, root):
        self.n += 1
        req = {'id': self.n, 'root': os.path.join(root, 'src'), 'main': 'main.tex', 'format': 'xelatex',
               'shell_escape': 'default', 'output_dir': os.path.join(root, 'out')}
        r0 = rusage(self.p.pid)
        t0 = time.time()
        write_frame(self.s, 0x02, json.dumps(req))
        frames, first = [], None
        while True:
            r = read_frame(self.f)
            if r is None:
                sys.exit('the host closed the connection')
            k, b = r
            if k == PAGE and first is None:
                first = time.time() - t0
            if k == DONE:
                d = json.loads(b)
                if d.get('id') == self.n:
                    break
            frames.append((k, b))
        done = time.time() - t0
        r1 = rusage(self.p.pid)
        ins = (r1.f29 - r0.f29) / 1e6 if r0 and r1 else None
        fp = round(r1.f7 / 2**20, 1) if r1 else None
        return frames, d, dict(first_ms=round(1e3 * (first or done), 1), done_ms=round(1e3 * done, 1),
                               instructions_m=round(ins, 1) if ins is not None else None, footprint_mb=fp)

    def close(self):
        try:
            write_frame(self.s, 0x04, '{}')
        except OSError:
            pass
        try:
            self.p.wait(timeout=30)
        except subprocess.TimeoutExpired:
            self.p.kill()
            self.p.wait()
        try:
            os.unlink(self.sock)
        except OSError:
            pass

    def stderr(self):
        self.err.flush()
        return open(self.errpath).read()


def snapshot(d):
    out = {}
    for dp, _, fs in os.walk(d):
        for n in fs:
            p = os.path.join(dp, n)
            out[os.path.relpath(p, d)] = open(p, 'rb').read()
    return dict(sorted(out.items()))


def restore(d, snap):
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d)
    for n, b in snap.items():
        os.makedirs(os.path.dirname(os.path.join(d, n)), exist_ok=True)
        open(os.path.join(d, n), 'wb').write(b)


def comparable(frames):
    out = []
    for k, b in frames:
        if k in (STARTED, TOOL):
            continue
        if k == DIAG:
            j = json.loads(b)
            j.pop('id', None)
            b = json.dumps(j, sort_keys=True).encode()
        out.append((k, b))
    return out


def pt1(b):
    out, skip = [], False
    for line in b.split(b'\n'):
        if line.startswith(b"Here is how much of TeX's memory you used:"):
            skip = True
            out.append(line)
            continue
        if skip:
            if line.startswith(b' '):
                continue
            skip = False
        if line.startswith(b'Memory usage before') or b'still untouched' in line:
            continue
        line = re.sub(rb'(Output written on .* \(\d+ pages?), \d+ bytes\)', rb'\1)', line)
        out.append(line)
    return b'\n'.join(out)


def done_fields(d):
    return {k: d.get(k) for k in ('status', 'exit_code', 'pages')}


def env(cache=None, **extra):
    e = dict(os.environ, FLASHTEX_PIN_CLOCK=a.clock, FLASHTEX_UNICODE_DEBUG='1')
    e.pop('SOURCE_DATE_EPOCH', None)
    e.pop('FLASHTEX_S0_CACHE', None)
    if cache:
        e['FLASHTEX_S0_CACHE'] = cache
    if a.profile and extra.get('FLASHTEX_UNICODE_INPROCESS') != '0':
        e['FLASHTEX_PROFILE'] = a.profile
    e.update(extra)
    return e


def kpsewhich(name):
    return subprocess.run(['kpsewhich', name], capture_output=True, text=True, check=True).stdout.strip()


def wait_saved(cache, host, timeout=60):
    """The host's S0 is written (its writer says so)."""
    t = time.time()
    while time.time() - t < timeout:
        if 'saved_s0' in host.stderr() or 'saving S0' in host.stderr():
            return 'saved_s0' in host.stderr()
        time.sleep(0.05)
    return False


def verify(work, measured_env, tag, out, bad_box):
    """A new host (measured_env) compiles `work`; the reference compiles the
    same directory from scratch; both compared."""
    pre = snapshot(work)
    h = Host(measured_env, 'm')
    frames, d, t = h.compile(work)
    h.close()
    note = [l for l in h.stderr().splitlines() if 'run from' in l]
    got = snapshot(work)
    restore(work, pre)
    r = Host(env(FLASHTEX_UNICODE_INPROCESS='0', FLASHTEX_UNICODE_SPARES='0'), 'r')
    rframes, rd, _ = r.compile(work)
    r.close()
    want = snapshot(work)
    bad = []
    for n in sorted(set(got) | set(want)):
        x, y = got.get(n), want.get(n)
        if x != y and not (n.endswith('.log') and x and y and pt1(x) == pt1(y)):
            bad.append(n)
    if comparable(frames) != comparable(rframes):
        bad.append('frames')
    if done_fields(d) != done_fields(rd):
        bad.append('done %s vs %s' % (done_fields(d), done_fields(rd)))
    restore(work, got)
    row = dict(tag=tag, starts=d.get('starts'), mismatch=bad, note=note[:1], **t)
    print(json.dumps(row), flush=True)
    if out:
        out.write(json.dumps(row) + '\n')
    if bad:
        bad_box[0] += 1
    return row


def main():
    out = open(a.out, 'a') if a.out else None
    bad = [0]
    for src in a.src:
        name = os.path.basename(src)
        base = tempfile.mkdtemp(prefix='fxr-', dir='/tmp')
        work = os.path.join(base, 'w')
        cache = a.cache or os.path.join(base, 'cache')
        os.makedirs(cache, exist_ok=True)
        before = set(os.listdir(cache))
        os.makedirs(os.path.join(work, 'src'))
        os.makedirs(os.path.join(work, 'out'))
        shutil.copy(src, os.path.join(work, 'src', 'main.tex'))
        # a document that loads `myfont.otf` gets TeX Live's Latin Modern
        # Roman 10 under that name in its directory (the `font` scenario
        # puts Latin Modern Roman 12 there while it is closed)
        fonts = []
        if b'myfont.otf' in open(src, 'rb').read():
            shutil.copy(kpsewhich('lmroman10-regular.otf'), os.path.join(work, 'src', 'myfont.otf'))
            fonts.append('myfont.otf')
        print('==', name, flush=True)
        # 1. the cold compile, which saves S0
        h = Host(env(cache), 'c')
        _, d, t = h.compile(work)
        saved = wait_saved(cache, h)
        h.close()
        print(json.dumps(dict(tag='cold', saved=saved, starts=d.get('starts'), **t)), flush=True)
        if not saved:
            print('   no S0 saved:', [l for l in h.stderr().splitlines() if 'S0' in l][-3:], flush=True)
            bad[0] += 1
            continue
        # S0's own file (this document's), and the bases (S0's pages equal
        # to the state after the format load are references to a base,
        # one per format, which every document of the format shares)
        own = [f for f in os.listdir(cache) if f.endswith('.xs0') and f not in before]
        size = sum(os.path.getsize(os.path.join(cache, f)) for f in own)
        bases = [f for f in os.listdir(cache) if f.endswith('.xs0base')]
        # the writer's report: the engine thread's time taking S0 out (and
        # comparing it with the base), the writer's
        saved_line = [l for l in h.stderr().splitlines() if '"saved_s0"' in l]
        try:
            w = json.loads(saved_line[-1].split(': ', 1)[1])
        except (IndexError, ValueError):
            w = {}
        print(json.dumps(dict(tag='s0_file', bytes=size, base_bytes=sum(os.path.getsize(os.path.join(cache, f)) for f in bases),
                              base_made=sorted(set(bases) - before), engine_ms=w.get('engine_ms'),
                              write_ms=w.get('write_ms'))), flush=True)
        # 2. timings: cold open against reopen, each a new host
        for _ in range(a.trials):
            for kind, e in (('cold_open', env()), ('reopen', env(cache))):
                h = Host(e, kind)
                _, d, t = h.compile(work)
                h.close()
                row = dict(tag=kind, starts=d.get('starts'), **t)
                print(json.dumps(row), flush=True)
                if out:
                    out.write(json.dumps(dict(row, doc=name)) + '\n')
        # 3. the scenarios
        r = verify(work, env(cache), 'same', out, bad)
        if (r['starts'] or [None])[0] != 's0':
            print('   UNEXPECTED: an unchanged document did not start from S0', flush=True)
            bad[0] += 1
        main_tex = os.path.join(work, 'src', 'main.tex')
        text = open(main_tex, 'rb').read()
        body = text.replace(b'\\end{document}', b'Added while the document was closed.\n\\end{document}')
        open(main_tex, 'wb').write(body)
        r = verify(work, env(cache), 'body', out, bad)
        if (r['starts'] or [None])[0] != 's0':
            print('   UNEXPECTED: a body edit did not start from S0', flush=True)
            bad[0] += 1
        pre_end = body.replace(b'\\begin{document}', b'\\newcommand\\addedwhileclosed{x}\n\\begin{document}', 1)
        open(main_tex, 'wb').write(pre_end)
        verify(work, env(cache), 'preamble_end', out, bad)
        # (the line after `\documentclass`: read before any S0)
        dc = body.find(b'\\documentclass')
        nl = body.find(b'\n', dc) + 1 if dc >= 0 else 0
        pre = body[:nl] + b'\\newcommand\\addedwhileclosed{x}\n' + body[nl:]
        open(main_tex, 'wb').write(pre)
        r = verify(work, env(cache), 'preamble', out, bad)
        if (r['starts'] or [None])[0] == 's0':
            print('   UNEXPECTED: a preamble edit started from S0', flush=True)
            bad[0] += 1
        open(main_tex, 'wb').write(text)
        if fonts:
            # the S0 of the text as it was, then the font changed
            h = Host(env(cache), 'c2')
            h.compile(work)
            wait_saved(cache, h)
            h.close()
            shutil.copy(kpsewhich('lmroman12-regular.otf'), os.path.join(work, 'src', fonts[0]))
            r = verify(work, env(cache), 'font', out, bad)
            if (r['starts'] or [None])[0] == 's0':
                print('   UNEXPECTED: a changed font file started from S0', flush=True)
                bad[0] += 1
        shutil.rmtree(base, ignore_errors=True)
    print('TOTAL: %d bad' % bad[0], flush=True)
    sys.exit(1 if bad[0] else 0)


main()
