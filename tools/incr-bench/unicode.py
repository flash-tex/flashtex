#!/usr/bin/env python3
"""Unicode mode's benchmark and soundness driver (lane XETEX-INCR;
docs/design/xetex/INCREMENTAL.md): `flashtex-host-unicode` over its socket,
as the app drives it.

usage: unicode.py HOST SRC [--doc main] [--trials N] [--seed S]
                  [--kinds replace,insert,delete,...] [--verify] [--keys N]
                  [--at end|mid|FRAC] [--out JSONL] [--keep]

Copies SRC (a directory, or one .tex file, which becomes DOC.tex) to a work
directory, starts HOST (`flashtex-host-unicode`, its
compiles in its own process by default) and compiles DOC.tex once (cold).

* `--keys N`: N keystrokes, each one letter typed at the same place (`--at`:
  before `\\end{document}`, the middle of the body, or a fraction of it), as
  a user types.
* `--trials N`: N random edits of `--kinds` at prose positions (tools/incr-bench
  edits.py's kinds, as `incr_bench.py` makes them), each followed by its
  revert.

Every compile is timed on the client: the first page received, the edited
page (the first page whose `PAGE` frame differs from the previous compile's
page of the same index; the last page when none differs) and `DONE`; and the
host's instructions retired over the compile (`proc_pid_rusage`: the host
process, its engine thread included; a child process's are not counted).

`--verify`: each compile is compared with a compile from scratch of the same
directory by a second host whose runs are processes of their own
(`FLASHTEX_UNICODE_INPROCESS=0`, no spares): every file the compiles leave
in the output directory (PDF, log, aux, toc, ...) byte for byte, and every
frame the host sent but `STARTED` and `DONE`'s timings (fonts, images,
forms, pages, diagnostics), in order. Both hosts pin the clock
(`FLASHTEX_PIN_CLOCK`). As the app lays a project out, the sources are in
`DIR/src` and the output in `DIR/out`; the reference's DIR is another of
the same length, and its path is read as the measured one's in what it
wrote (the logs name it). The source map (`SOURCES` frames) is also
compared on its own (`source-map`). A mismatch's files go to BASE/mm/.

Prints one JSON line per compile (and to --out), then a summary line.
"""
import argparse
import ctypes
import hashlib
import json
import os
import random
import re
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import edits  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument('host')
ap.add_argument('srcdir')
ap.add_argument('--doc', default='main')
ap.add_argument('--trials', type=int, default=0)
ap.add_argument('--keys', type=int, default=0)
ap.add_argument('--at', default='end')
ap.add_argument('--seed', type=int, default=1)
ap.add_argument('--kinds', default='replace,insert,delete')
ap.add_argument('--verify', action='store_true')
ap.add_argument('--out')
ap.add_argument('--keep', action='store_true')
ap.add_argument('--clock', default='1760000000.5')
ap.add_argument('--host-env', action='append', default=[], help='K=V for the measured host')
ap.add_argument('--interleave', action='store_true',
                help='with --trials: each edit\'s compile is cancelled 0-15 ms after it is sent, a second '
                     'edit saved in between (as an editor saves while the host reads); the second edit is '
                     'then compiled and verified (the classic sweep D: incr_bench.py --interleave)')
a = ap.parse_args()

PAGE, FONT, IMAGE, FORM, DIAG, DONE, STARTED, TOOL, SOURCES = 0x45, 0x43, 0x44, 0x46, 0x48, 0x49, 0x42, 0x4C, 0x47

_libproc = ctypes.CDLL('/usr/lib/libproc.dylib') if sys.platform == 'darwin' else None


class _RU(ctypes.Structure):
    _fields_ = [('uuid', ctypes.c_uint8 * 16)] + [('f%d' % k, ctypes.c_uint64) for k in range(40)]


def instructions(pid):
    """rusage_info_v4's ri_instructions (macOS), or None."""
    if _libproc is None:
        return None
    r = _RU()
    if _libproc.proc_pid_rusage(pid, 4, ctypes.byref(r)) != 0:
        return None
    return r.f29  # ri_instructions


def footprint(pid):
    """(phys_footprint, lifetime max phys_footprint) in MB (macOS), or None."""
    if _libproc is None:
        return None
    r = _RU()
    if _libproc.proc_pid_rusage(pid, 4, ctypes.byref(r)) != 0:
        return None
    return round(r.f7 / 2**20, 1), round(r.f28 / 2**20, 1)


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
    def __init__(self, exe, env, tag):
        self.sock = os.path.join(tempfile.gettempdir(), 'fxu-%d-%s.sock' % (os.getpid(), tag))
        try:
            os.unlink(self.sock)
        except OSError:
            pass
        self.err = open(os.path.join(tempfile.gettempdir(), 'fxu-%d-%s.err' % (os.getpid(), tag)), 'w')
        self.p = subprocess.Popen([exe, '--socket', self.sock, '--once', '--accept-timeout', '600'],
                                  stdout=subprocess.DEVNULL, stderr=self.err, env=env)
        while not os.path.exists(self.sock):
            if self.p.poll() is not None:
                sys.exit('%s: the host ended' % tag)
            time.sleep(0.005)
        self.s = socket.socket(socket.AF_UNIX)
        self.s.connect(self.sock)
        self.f = self.s.makefile('rb')
        write_frame(self.s, 0x01, json.dumps({'protocol': 'display-list-v3', 'version': [3, 3],
                                              'client': 'incr-bench-unicode'}))
        read_frame(self.f)
        self.n = 0

    def compile(self, root, main):
        """One compile: (frames, done, timings)."""
        self.n += 1
        req = {'id': self.n, 'root': os.path.join(root, 'src'), 'main': main, 'format': 'xelatex',
               'shell_escape': 'default', 'output_dir': os.path.join(root, 'out')}
        i0 = instructions(self.p.pid)
        t0 = time.time()
        write_frame(self.s, 0x02, json.dumps(req))
        frames = []
        first = None
        page_times = []
        while True:
            r = read_frame(self.f)
            if r is None:
                sys.exit('the host closed the connection')
            k, b = r
            if k == PAGE:
                page_times.append(time.time() - t0)
                if first is None:
                    first = page_times[-1]
            if k == DONE:
                d = json.loads(b)
                if d.get('id') == self.n:
                    break
            frames.append((k, b))
        done = time.time() - t0
        i1 = instructions(self.p.pid)
        ins = (i1 - i0) if i0 is not None and i1 is not None else None
        return frames, d, dict(first=first, done=done, page_times=page_times, instructions=ins,
                               footprint=footprint(self.p.pid))

    def compile_cancelled(self, root, main, ms, between):
        """A compile cancelled `ms` milliseconds after it is sent, with
        `between()` run just before the cancel: its DONE."""
        self.n += 1
        req = {'id': self.n, 'root': os.path.join(root, 'src'), 'main': main, 'format': 'xelatex',
               'shell_escape': 'default', 'output_dir': os.path.join(root, 'out')}
        write_frame(self.s, 0x02, json.dumps(req))
        time.sleep(ms / 1e3)
        between()
        write_frame(self.s, 0x03, json.dumps({'id': self.n}))
        while True:
            r = read_frame(self.f)
            if r is None:
                sys.exit('the host closed the connection')
            k, b = r
            if k == DONE:
                d = json.loads(b)
                if d.get('id') == self.n:
                    return d

    def close(self):
        try:
            write_frame(self.s, 0x04, '{}')
        except OSError:
            pass
        self.p.kill()
        self.p.wait()
        try:
            os.unlink(self.sock)
        except OSError:
            pass


def snapshot(d):
    """Every file under d, by relative path."""
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


def as_work(b):
    """The reference's bytes with its root's path read as the measured one's."""
    return b.replace(REF_ROOT, WORK_ROOT)


def comparable(frames, fix=lambda b: b):
    """The frames a from-scratch compile must send alike: all but STARTED
    (pid, argv) and TOOL; DIAGNOSTIC without its id."""
    out = []
    for k, b in frames:
        if k in (STARTED, TOOL):
            continue
        b = fix(b)
        if k == DIAG:
            j = json.loads(b)
            j.pop('id', None)
            b = json.dumps(j, sort_keys=True).encode()
        out.append((k, b))
    return out


ACCOUNTING_BLOCK = b"Here is how much of TeX's memory you used:"


def pt1(b):
    """A log with DESIGN.md §1.1's P-T1 normalisation (ruled 2026-09-29):
    the end-of-run memory block, `\\tracingstats` memory-usage lines and the
    output's byte count are accounting, compared apart (non-gating)."""
    out, skip = [], False
    for line in b.split(b'\n'):
        if line.startswith(ACCOUNTING_BLOCK):
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
    return {k: d.get(k) for k in ('status', 'exit_code', 'pages', 'passes')}


def prose_positions(src):
    """Letters inside prose words (not commands, not math) of the body
    (incr_bench.py's rule)."""
    cands = []
    body_start = src.find(b'\\begin{document}')
    for m in re.finditer(rb'[^\n]*\n', src):
        line = m.group(0)
        ls = m.start()
        if ls < body_start or not line[:1].isalpha():
            continue
        dollars = 0
        in_cmd = False
        for k, c in enumerate(line):
            ch = chr(c)
            if ch == '$':
                dollars += 1
            if ch == '\\':
                in_cmd = True
                continue
            if in_cmd and not ch.isalpha():
                in_cmd = False
            if in_cmd or dollars % 2 == 1 or ch in '{}':
                continue
            if ch.isalpha() and ch.islower() and k > 0 and (chr(line[k - 1]).isalpha() or line[k - 1] == 32):
                cands.append(ls + k)
    return cands


def paragraph_starts(src):
    body = src.find(b'\\begin{document}')
    end = src.rfind(b'\\end{document}')
    return [m.start() + 1 for m in re.finditer(rb'\n\n(?=[A-Z])', src) if body < m.start() < end]


def word_end(src, p):
    while p < len(src) and chr(src[p]).isalpha():
        p += 1
    return p


def structural(kind, src, p, i, rng):
    """incr_bench.py's structural edits near p (None where none applies)."""
    if kind == 'section':
        starts = paragraph_starts(src)
        if not starts:
            return None
        q = min(starts, key=lambda s: abs(s - p))
        return src[:q] + ('\\section{Inserted %d}\\label{sec:ins%d}\n\n' % (i, i)).encode() + src[q:]
    if kind == 'label':
        q = word_end(src, p)
        return src[:q] + ('\\label{lab:new%d}' % i).encode() + src[q:]
    if kind == 'footnote':
        q = word_end(src, p)
        return src[:q] + ('\\footnote{An inserted note %d.}' % i).encode() + src[q:]
    if kind == 'ref':
        labs = re.findall(rb'\\label\{([^}]*)\}', src)
        if not labs:
            return None
        q = word_end(src, p)
        how = rng.choice(['ref', 'pageref'])
        return src[:q] + (' (see \\%s{%s})' % (how, rng.choice(labs).decode())).encode() + src[q:]
    if kind in ('unlabel', 'unsection'):
        pat = rb'\\label\{[^}]*\}' if kind == 'unlabel' else rb'\\section\{[^}\n]*\}'
        ms = list(re.finditer(pat, src))
        if not ms:
            return None
        m = min(ms, key=lambda m: abs(m.start() - p))
        return src[:m.start()] + src[m.end():]
    return None


def make_edit(kind, src, p, rng, i=0):
    if kind == 'replace':
        c = src[p]
        return src[:p] + bytes([rng.choice([x for x in b'abcdefghijklmnopqrstuvwxyz' if x != c])]) + src[p + 1:]
    if kind == 'insert':
        return src[:p] + bytes([rng.choice(b'abcdefghijklmnopqrstuvwxyz')]) + src[p:]
    if kind == 'delete':
        return src[:p] + src[p + 1:]
    if kind == 'sentence':
        return src[:p] + b' lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor' + src[p:]
    if kind in edits.STRUCTURAL_KINDS:
        return structural(kind, src, p, i, rng)
    if kind in edits.ALL_KINDS and hasattr(edits, kind):
        return edits.apply(kind, src, p)
    return None


def main():
    # (short paths: the log wraps lines at 79 columns, and the reference's
    # path is replaced by the measured one's in what it wrote)
    base = tempfile.mkdtemp(prefix='fxu-', dir='/tmp')
    work = os.path.join(base, 'fxuwrk')
    ref = os.path.join(base, 'fxuref')  # as long as the other
    if os.path.isdir(a.srcdir):
        shutil.copytree(a.srcdir, os.path.join(work, 'src'))
    else:
        os.makedirs(os.path.join(work, 'src'))
        shutil.copy(a.srcdir, os.path.join(work, 'src', a.doc + '.tex'))
    global WORK_ROOT, REF_ROOT
    WORK_ROOT, REF_ROOT = work.encode(), ref.encode()
    env = dict(os.environ, FLASHTEX_PIN_CLOCK=a.clock)
    env.pop('SOURCE_DATE_EPOCH', None)
    henv = dict(env)
    for kv in a.host_env:
        k, v = kv.split('=', 1)
        henv[k] = v
    host = Host(a.host, henv, 'm')
    refhost = None
    if a.verify:
        renv = dict(env, FLASHTEX_UNICODE_INPROCESS='0', FLASHTEX_UNICODE_SPARES='0')
        refhost = Host(a.host, renv, 'r')
    out = open(a.out, 'a') if a.out else None
    main_tex = a.doc + '.tex'
    results = []
    prev_pages = {}

    def one(content, tag):
        nonlocal prev_pages
        open(os.path.join(work, 'src', main_tex), 'wb').write(content)
        pre = snapshot(work)
        frames, d, t = host.compile(work, main_tex)
        pages = [b for k, b in frames if k == PAGE]
        n = d.get('pages') or 0
        last = pages[-n:] if n else []
        # the edited page: the first page received whose frame differs from
        # the previous compile's page of the same number (a pass sends its
        # pages from the first, so page `idx` of the stream is page
        # `idx mod n` of its pass)
        edited = None
        for idx, b in enumerate(pages):
            if prev_pages.get(idx % max(n, 1)) != hashlib.sha256(b).hexdigest():
                edited = t['page_times'][idx]
                break
        if edited is None:
            edited = t['page_times'][-1] if t['page_times'] else t['done']
        prev_pages = {i: hashlib.sha256(b).hexdigest() for i, b in enumerate(last)}
        r = dict(tag=tag, status=d.get('status'), passes=d.get('passes'), pages=n,
                 first_ms=round(1e3 * (t['first'] or 0), 1), edited_ms=round(1e3 * edited, 1),
                 done_ms=round(1e3 * t['done'], 1),
                 instructions_m=round(t['instructions'] / 1e6, 1) if t['instructions'] is not None else None,
                 mode=d.get('mode'), starts=d.get('starts'),
                 footprint_mb=(t['footprint'] or (None, None))[0], peak_mb=(t['footprint'] or (None, None))[1],
                 sources=sum(1 for k, _ in frames if k == SOURCES))
        if refhost is not None:
            got = snapshot(work)
            restore(ref, pre)
            rframes, rd, _ = refhost.compile(ref, main_tex)
            want = {n: as_work(b) for n, b in snapshot(ref).items()}
            bad = []
            accounting = []
            for name in sorted(set(got) | set(want)):
                a, b = got.get(name), want.get(name)
                if a != b and name.endswith('.log') and a is not None and b is not None \
                        and pt1(a) == pt1(b):
                    accounting.append(name)
                    continue
                if a != b:
                    bad.append(name)
                    os.makedirs(os.path.join(base, 'mm'), exist_ok=True)
                    safe = re.sub(r'[^A-Za-z0-9_.-]', '_', tag)
                    for side, snap in (('got', got), ('ref', want)):
                        if name in snap:
                            open(os.path.join(base, 'mm', '%s.%s.%s' % (safe, side, name.replace('/', '_'))), 'wb').write(snap[name])
            # the source map on its own (each page's spans and columns are
            # in its PAGE frame; the files and lines in the SOURCES before)
            src = [b for k, b in comparable(frames) if k == SOURCES]
            rsrc = [b for k, b in comparable(rframes, as_work) if k == SOURCES]
            if src != rsrc:
                bad.append('source-map')
            if comparable(frames) != comparable(rframes, as_work):
                bad.append('display-list')
            if done_fields(d) != done_fields(rd):
                bad.append('done %s vs %s' % (done_fields(d), done_fields(rd)))
            r['mismatch'] = bad
            if accounting:
                r['accounting'] = accounting
        results.append(r)
        line = json.dumps(r)
        print(line, flush=True)
        if out:
            out.write(line + '\n')
            out.flush()
        return r

    src = open(os.path.join(work, 'src', main_tex), 'rb').read()
    one(src, 'cold')
    if a.keys:
        end = src.index(b'\\end{document}')
        if a.at == 'end':
            anchor = end
        else:
            frac = 0.5 if a.at == 'mid' else float(a.at)
            begin = src.index(b'\\begin{document}') + len(b'\\begin{document}')
            anchor = src.index(b'\n', begin + int((end - begin) * frac)) + 1
        cur = src
        for i in range(a.keys):
            cur = cur[:anchor + i] + b'x' + cur[anchor + i:]
            one(cur, 'key%d' % (i + 1))
        src = cur
    if a.trials:
        rng = random.Random(a.seed)
        kinds = edits.parse_kinds(a.kinds)
        cands = prose_positions(src)
        for i in range(a.trials):
            if not cands:
                break
            p = rng.choice(cands)
            kind = rng.choice(kinds)
            new = make_edit(kind, src, p, rng, i)
            if new is None:
                continue
            if a.interleave:
                # the second edit: another kind, anywhere (before the first
                # one about half the time), saved while the first compiles
                q = rng.choice(cands)
                second = make_edit(rng.choice(kinds), new, min(q, len(new) - 1), rng, i + 1000)
                if second is None:
                    second = new
                ms = rng.uniform(0, 15)
                path = os.path.join(work, 'src', main_tex)
                open(path, 'wb').write(new)
                d = host.compile_cancelled(work, main_tex, ms,
                                           lambda: open(path, 'wb').write(second))
                print(json.dumps(dict(tag='%d:%s@%d interrupted after %.1f ms' % (i, kind, p, ms),
                                      status=d.get('status'), starts=d.get('starts'))), flush=True)
                one(second, '%d:second-after-interrupt' % i)
            else:
                one(new, '%d:%s@%d' % (i, kind, p))
            one(src, '%d:revert' % i)
    host.close()
    if refhost is not None:
        refhost.close()
    summary = dict(doc=a.srcdir, compiles=len(results),
                   verified=sum(1 for r in results if r.get('mismatch') == []),
                   mismatches=sum(1 for r in results if r.get('mismatch')),
                   accounting=sum(1 for r in results if r.get('accounting')))
    print(json.dumps(summary), flush=True)
    if not a.keep and summary['mismatches'] == 0:
        shutil.rmtree(base)
    else:
        print('work directory: %s' % base, file=sys.stderr)
    sys.exit(1 if summary['mismatches'] else 0)


main()
