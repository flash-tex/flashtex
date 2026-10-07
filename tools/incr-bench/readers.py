#!/usr/bin/env python3
"""readers.py ENGINE OUT [--max-mixed R] [--runs DOC:INTERVAL_MS[:KEYS],...]: what another program
reading the output files while the resident host types sees (a viewer that reloads the PDF).

Adapted from the #1613 review's probe (lane P4-PAGE-COST; restores keep the old run's output tail in
the file, `system::Logical`). For each run a host opens DOC, then `dl3-keys` types KEYS letters,
INTERVAL_MS apart (fast enough that compiles are abandoned, slow enough that they converge and jump),
while a thread reads out/main.pdf, .aux and .log in a loop. Each PDF read is one of:
  nul        a run of 4,096 or more zero bytes: a hole (a length cut and then written on)
  truncated  no %%EOF at the end (what a cut file, or one whose end is blanked, reads as)
  eof-ok     ends in %%EOF and every object offset its cross-reference names starts an object
  eof-bad    ends in %%EOF but an offset does not: an old trailer over new bytes ("mixed")
The gate fails on any nul PDF read, any .aux or .log read with a zero run, a final PDF, .aux or .log
(the source is the original again after the even keystrokes) other than the opening compile's, or
eof-bad reads above --max-mixed of all PDF reads. Measured on the NixOS PC, 2026-10-06, 3 runs
each: main (f8e671dca) read a hole or eof-bad in 59 of 22,469 reads on full-100 at 400 ms (0.26 %)
and 21 of 1,117 on full-1000 at 100/250 ms (1.9 %, all holes); kept tails (#1613) 57 of 21,509
(0.26 %) and 19 of about 4,900 (0.4 %, none a hole). The eof-bad reads that remain are a convergence
jump's (it writes the old run's trailer, which the run's end then rewrites; main's too) and reads
torn across an abandon's put-back (the read began before it and ended after).

ENGINE: an engine directory (mkeng.sh, gates.sh's $INCR_BENCH_DIR/gates). OUT: a directory for the
raw records (readers.jsonl). Exit 0: pass; 1: a check failed; 2: the harness failed.
"""
import argparse, json, os, re, shutil, sys, threading, time, zlib

S = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, S)
import t7  # noqa: E402

IB = t7.IB
NUL = b'\0' * 4096
OBJ = re.compile(rb'\d+ \d+ obj')


def unpredict(data, cols):
    out, prev = bytearray(), bytearray(cols)
    for i in range(0, len(data), cols + 1):
        ft, row = data[i], bytearray(data[i + 1:i + 1 + cols])
        if ft == 2:
            for j in range(cols):
                row[j] = (row[j] + prev[j]) & 0xff
        elif ft != 0:
            raise ValueError(f'predictor {ft}')
        out += row
        prev = row
    return bytes(out)


def xref_ok(b):
    """Every in-use object offset of the last cross-reference section starts an object."""
    i = b.rfind(b'startxref')
    m = re.match(rb'startxref\s+(\d+)', b[i:]) if i >= 0 else None
    if not m:
        return False
    off = int(m.group(1))
    if off >= len(b):
        return False
    if b[off:off + 4] == b'xref':
        for line in b[off + 4:i].split(b'\n'):
            f = line.split()
            if f[:1] == [b'trailer']:
                break
            if len(f) == 3 and f[2] == b'n' and not OBJ.match(b, int(f[0])):
                return False
        return True
    if not OBJ.match(b, off):
        return False
    j = b.find(b'stream', off)
    d = b[off:j]
    w = [int(x) for x in re.search(rb'/W\s*\[([^\]]*)\]', d).group(1).split()]
    ln = int(re.search(rb'/Length\s+(\d+)', d).group(1))
    s = j + len(b'stream')
    s += 2 if b[s:s + 2] == b'\r\n' else 1 if b[s:s + 1] == b'\n' else 0
    raw = zlib.decompress(b[s:s + ln])
    cols = sum(w)
    pm = re.search(rb'/Predictor\s+(\d+)', d)
    if pm and int(pm.group(1)) >= 10:
        raw = unpredict(raw, cols)
    for r in range(0, len(raw) - cols + 1, cols):
        e, f, p = raw[r:r + cols], [], 0
        for wi in w:
            f.append(int.from_bytes(e[p:p + wi], 'big') if wi else (1 if not f else 0))
            p += wi
        if f[0] == 1 and not OBJ.match(b, f[1]):
            return False
    return True


def classify(b):
    if not b:
        return 'empty'
    if NUL in b:
        return 'nul'
    if not b.rstrip().endswith(b'%%EOF'):
        return 'truncated'
    try:
        return 'eof-ok' if xref_ok(b) else 'eof-bad'
    except Exception:  # noqa: BLE001
        return 'eof-unparsed'


def read(p):
    try:
        with open(p, 'rb') as f:
            return f.read()
    except OSError:
        return b''


class Reader(threading.Thread):
    def __init__(self, pdf, others, trace):
        super().__init__(daemon=True)
        self.pdf, self.others, self.trace = pdf, others, trace
        self.stop = False
        self.counts, self.other_nul, self.nuls, self.mixed = {}, 0, [], []

    def run(self):
        while not self.stop:
            b = read(self.pdf)
            c = classify(b)
            self.counts[c] = self.counts.get(c, 0) + 1
            if c == 'nul':
                i = b.find(NUL)
                j = i
                while j < len(b) and b[j] == 0:
                    j += 1
                self.nuls.append([len(b), i, j - i, self.at()])
            elif c == 'eof-bad':
                self.mixed.append([len(b), self.at()])
            self.other_nul += sum(NUL in read(p) for p in self.others)
            time.sleep(0.0005)

    def at(self):
        """Where the file trace was (--trace), to line a read up with the engine's file operations."""
        return os.path.getsize(self.trace) if self.trace and os.path.exists(self.trace) else None


def run(eng, out, doc, interval, nkeys, trace):
    tag = f'{doc}-{interval}'
    trace = f'{os.path.abspath(out)}/{tag}.trace' if trace else None
    if trace:
        os.environ['FLASHTEX_FILE_TRACE'] = trace
    work, s0 = f'{IB}/readers-work/{tag}', f'{IB}/readers-work/s0-{tag}'
    for p in (work, s0):
        shutil.rmtree(p, ignore_errors=True)
    shutil.copytree(f'{IB}/docs/{doc}', work, symlinks=True)
    os.makedirs(f'{work}/out', exist_ok=True)
    files = [f'{work}/out/main.{x}' for x in ('pdf', 'aux', 'log')]
    sock = f'/tmp/readers-{os.getpid()}.sock'
    h = t7.Host(eng, sock, s0, f'{out}/{tag}.host-stderr', [])
    res = dict(doc=doc, interval=interval, keys=nkeys)
    try:
        t7.keys(eng, sock, work, ['--kind', 'letter', '--at', '0.5', '--keys', '0'], f'{out}/{tag}-open.jsonl', 1800)
        base = [read(p) for p in files]
        r = Reader(files[0], files[1:], trace)
        r.start()
        t7.keys(eng, sock, work, ['--kind', 'letter', '--at', '0.5', '--interval-ms', str(interval), '--keys',
                                  str(nkeys), '--gap-ms', '300'], f'{out}/{tag}.jsonl', 1800)
        r.stop = True
        r.join()
        res.update(reads=r.counts, nul_reads=r.nuls[:20], mixed_reads=r.mixed[:20], aux_log_nul_reads=r.other_nul,
                   final_equals_open=[read(p) == b for p, b in zip(files, base)])
    finally:
        h.stop()
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('engine')
    ap.add_argument('out')
    ap.add_argument('--max-mixed', type=float, default=0.005, help='eof-bad PDF reads, of all (default 0.5 %%)')
    ap.add_argument('--runs', default='full-100:400,full-1000:100,full-1000:250,full-10:50:60')
    ap.add_argument('--trace', action='store_true', help='FLASHTEX_FILE_TRACE per run, and where each bad read fell in it')
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    bad, total, mixed = [], 0, 0
    try:
        for spec in a.runs.split(','):
            doc, interval, *k = spec.split(':')
            res = run(os.path.abspath(a.engine), a.out, doc, int(interval), int(k[0]) if k else 40, a.trace)
            print(json.dumps(res), flush=True)
            with open(f'{a.out}/readers.jsonl', 'a') as f:
                f.write(json.dumps(res) + '\n')
            reads = res['reads']
            total += sum(reads.values())
            mixed += reads.get('eof-bad', 0)
            if reads.get('nul'):
                bad.append(f'{spec}: {reads["nul"]} PDF reads with a hole')
            if res['aux_log_nul_reads']:
                bad.append(f'{spec}: {res["aux_log_nul_reads"]} .aux/.log reads with a zero run')
            if not all(res['final_equals_open']):
                bad.append(f'{spec}: final pdf/aux/log equal the opening compile\'s: {res["final_equals_open"]}')
    except Exception as e:  # noqa: BLE001
        print(f'readers: harness failed: {e}', file=sys.stderr)
        return 2
    if total and mixed / total > a.max_mixed:
        bad.append(f'eof-bad in {mixed} of {total} PDF reads ({100 * mixed / total:.2f} % > {100 * a.max_mixed:.2f} %)')
    print(f'readers: {total} PDF reads, {mixed} eof-bad; ' + ('; '.join(bad) if bad else 'pass'))
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
