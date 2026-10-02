#!/usr/bin/env python3
"""t7.py: the T7 latency gate (DESIGN.md §8 T7, §1.2's targets, §12's P4 exit gate).

usage: t7.py [--build] [--engine NAME] [--docs plain-10,full-1000,...] [--keys N]
             [--preamble N] [--reopen N] [--gap-ms MS] [--margin F] [--out DIR]
             [--baseline FILE] [--write-baseline FILE] [--reopen-gate prewarmed|cold|none]
             [--wait-load L] [--wait-max S] [--host-args ARGS] [--quick]
       t7.py --check OUT/summary.json [--margin F] [--baseline FILE] [--reopen-gate ...]

Everything goes through the engine host's Unix socket with the display-list-v3 protocol
(docs/protocol/display-list-v3.md), as the app talks to it: `flashtex-host --socket` is started
here as a separate process, and the keystrokes are typed by `dl3-keys` (the harness's socket
client, crates/display-list-v3/src/bin/dl3-keys.rs). One host session per document, as the app
runs one host per open document, in this order:

  letter@start, letter@middle, letter@end   a letter inserted into a word and deleted again, in a
                                            prose line on a page 2 %, 50 % and 98 % into the document
  sentence@middle                           twelve words inserted and deleted (the paragraph reflows)
  newline@middle                            a line break inserted before a word and deleted (only
                                            later input lines move)
  split@middle                              a blank line inserted (the paragraph splits) and deleted
                                            (it joins again)
  preamble                                  a \\newcommand line after \\documentclass, added and
                                            removed: S0 changes, a full run from the format

then `reopen`: the host is stopped, the document is edited on disk, and a new host process with
the same S0 cache opens it (DESIGN.md §5.1). `prewarmed` is the first COMPILE to page 1 once the
new host listens (its warm-up done); `cold` adds the host's start (spawn to `listening`).

Measured per keystroke, client side of the socket (one of §1.2's four quantities: "socket
client time"): COMPILE to the watched (edited) page's PAGE frame (`edited_page_ms`), to the
first PAGE frame (`first_page_ms`: keystroke to the first changed page) and to DONE (`done_ms`:
the background re-typesetting); from the host's DONE whether the run converged (`converged_at`)
and how many pages it re-typeset (`typeset_pages`); and whether the pages the client holds were
kept current: later pages marked stale before they were refreshed, all current at DONE
(dl3-keys' `stale_marked`, `complete`). Host RSS: the peak (`wait4` ru_maxrss) of each host.

Targets (§1.2): edited page <= 16 ms p95 (every in-body kind, any size); preamble <= 400 ms to
the first visible page (page 1, the viewport); reopen <= 100 ms to the first visible page.
Whether a pre-warmed host counts for reopen is the owner's decision O8 (§5.1):
`--reopen-gate prewarmed` (default) gates the pre-warmed number and reports the cold one.
Convergence and background pages are "measured and held" (§5.3, §12): with `--baseline`
(default tools/incr-bench/t7-baseline.json when present) a row fails when its convergence rate
falls more than 0.15 below the baseline's or its median re-typeset pages exceed the baseline's
by more than 25 % + 2.

Exit: 0 all targets met; 1 a target missed (or a page never arrived, or pages were not kept
current, or a held rate fell); 2 the harness failed.
Output: OUT/raw/*.jsonl (dl3-keys' lines), OUT/summary.json, OUT/table.md, OUT/environment.txt.
"""
import argparse
import json
import os
import queue
import shutil
import subprocess
import sys
import threading
import time

S = os.path.dirname(os.path.abspath(__file__))
W = os.path.dirname(os.path.dirname(S))
IB = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
DOCS = ['plain-10', 'full-10', 'plain-100', 'full-100', 'plain-300', 'full-300', 'plain-1000', 'full-1000']
# (phase, dl3-keys arguments)
PHASES = [
    ('letter@start', ['--kind', 'letter', '--at', '0.02']),
    ('letter@middle', ['--kind', 'letter', '--at', '0.5']),
    ('letter@end', ['--kind', 'letter', '--at', '0.98']),
    ('sentence@middle', ['--kind', 'sentence', '--at', '0.5']),
    ('newline@middle', ['--kind', 'newline', '--at', '0.5']),
    ('split@middle', ['--kind', 'split', '--at', '0.5']),
    ('preamble', ['--kind', 'preamble', '--at', '0.5']),
]
TARGET = {'edit': 16.0, 'preamble': 400.0, 'reopen': 100.0}
CONV_TOL, BG_TOL = 0.15, (1.25, 2)


def pct(v, p):
    v = sorted(v)
    return v[round((len(v) - 1) * p)] if v else None


def stats(v):
    return dict(n=len(v), p50=pct(v, .5), p95=pct(v, .95), max=pct(v, 1.0))


def uptime():
    return subprocess.run(['uptime'], capture_output=True, text=True).stdout.strip()


def power():
    """macOS: the power source, battery, Low Power Mode (it lowers the clocks: every number with it on
    is pessimistic) and sleeps, recorded with the run."""
    out = []
    batt = subprocess.run(['pmset', '-g', 'batt'], capture_output=True, text=True).stdout.split('\n')
    out.append('power: ' + ' '.join(l.strip() for l in batt if l.strip()))
    lpm = [l.split()[-1] for l in subprocess.run(['pmset', '-g'], capture_output=True, text=True).stdout.split('\n')
           if 'lowpowermode' in l]
    out.append(f'lowpowermode: {lpm[0] if lpm else "?"}')
    if lpm and lpm[0] == '1':
        print('t7: WARNING: macOS Low Power Mode is on; latencies will be pessimistic', file=sys.stderr)
    return out


def load1():
    return round(os.getloadavg()[0], 2)


class Host:
    """flashtex-host --socket, our own child; stopped by its handle (never by name)."""

    def __init__(self, eng, sock, s0, log, extra):
        for p in (sock,):
            if os.path.exists(p):
                os.unlink(p)
        env = dict(os.environ, FLASHTEX_POOL=f'{eng}/pdftex.pool', FLASHTEX_FORMATS=os.path.join(os.path.dirname(eng), 'fmt-' + os.path.basename(eng)),
                   SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1')
        self.sock = sock
        self.lines = queue.Queue()
        self.saved = 0
        self.startup = None
        t0 = time.perf_counter()
        self.p = subprocess.Popen([f'{eng}/flashtex-host', '--socket', sock, '--s0-cache', s0] + extra,
                                  stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=open(log, 'a'), text=True, env=env)
        threading.Thread(target=self._read, daemon=True).start()
        deadline = time.time() + 600
        while True:
            try:
                l = self.lines.get(timeout=max(0.01, deadline - time.time()))
            except queue.Empty:
                self.stop()
                raise RuntimeError(f'flashtex-host did not listen (see {log})')
            if l is None:
                raise RuntimeError(f'flashtex-host exited before listening (see {log})')
            if l.startswith('flashtex-host: listening'):
                break
            if l.startswith('flashtex-host: {') and self.startup is None:
                self.startup = l[len('flashtex-host: '):]
        self.ready_ms = (time.perf_counter() - t0) * 1e3

    def _read(self):
        for l in self.p.stdout:
            l = l.rstrip('\n')
            if '"saved_s0"' in l:
                self.saved += 1
            self.lines.put(l)
        self.lines.put(None)

    def settle_saves(self, quiet=1.0, limit=30.0):
        """Wait until no S0 save has been reported for `quiet` seconds."""
        end, n, since = time.time() + limit, self.saved, time.time()
        while time.time() < end and time.time() - since < quiet:
            time.sleep(0.1)
            if self.saved != n:
                n, since = self.saved, time.time()

    def stop(self):
        """Stop the host; its peak RSS in bytes (ru_maxrss: bytes on macOS, KiB on Linux)."""
        if self.p.returncode is not None:
            return None
        self.p.terminate()
        _, _, ru = os.wait4(self.p.pid, 0)
        self.p.returncode = -15
        if os.path.exists(self.sock):
            os.unlink(self.sock)
        return ru.ru_maxrss if sys.platform == 'darwin' else ru.ru_maxrss * 1024


def keys(eng, sock, work, args, out, timeout):
    """One dl3-keys connection; its JSON lines."""
    cmd = [f'{S}/to.sh', str(timeout), f'{eng}/dl3-keys', '--socket', sock, '--root', work, '--main', 'main.tex',
           '--output-dir', f'{work}/out'] + args
    p = subprocess.run(cmd, capture_output=True, text=True)
    with open(out, 'a') as f:
        f.write(p.stdout)
    recs = [json.loads(l) for l in p.stdout.splitlines() if l.startswith('{')]
    if p.returncode != 0:
        raise RuntimeError(f'dl3-keys {" ".join(args)}: exit {p.returncode}: {p.stderr.strip()[-400:]}')
    return recs, p.stderr.strip()


def body_line(text, frac):
    """The byte offset inside the second word of the prose line about `frac` into the body."""
    lines = text.split('\n')
    prose = [i for i, l in enumerate(lines) if len(l.split(' ')) > 40]
    i = prose[min(len(prose) - 1, int(len(prose) * frac))]
    off = sum(len(l) + 1 for l in lines[:i])
    return off + lines[i].index(' ') + 3


def run_doc(a, eng, doc, out):
    src = f'{IB}/docs/{doc}/main.tex'
    work = f'{IB}/t7-work/{doc}'
    s0 = f'{IB}/t7-work/s0-{doc}'
    sock = f'{IB}/t7-work/{doc}.sock'
    shutil.rmtree(work, ignore_errors=True)
    shutil.rmtree(s0, ignore_errors=True)
    os.makedirs(f'{work}/out')
    shutil.copy(src, f'{work}/main.tex')
    log = f'{out}/raw/{doc}.host-stderr'
    hostargs = a.host_args.split()
    res = dict(doc=doc, phases={}, reopen=[], load=[])
    h = Host(eng, sock, s0, log, hostargs)
    res['host_startup'], res['host_ready_ms'] = h.startup, round(h.ready_ms, 1)
    rss = []
    try:
        for name, args in PHASES:
            if a.phases and name not in a.phases:
                continue
            n = a.preamble if name == 'preamble' else a.keys
            l0 = load1()
            t = time.time()
            recs, err = keys(eng, sock, work, args + ['--keys', str(n), '--gap-ms', str(a.gap_ms)],
                             f'{out}/raw/{doc}-{name}.jsonl', a.timeout)
            opens = [r for r in recs if 'open' in r]
            if 'open_ms' not in res and opens:
                res['open_ms'] = round(opens[0]['done_ms'], 1)
                res['pages'] = opens[-1]['host'].get('pages')
            res['phases'][name] = dict(keys=[r for r in recs if 'key' in r], where=err, load=[l0, load1()],
                                       wall_s=round(time.time() - t, 1))
            print(f'  {doc} {name}: {len(res["phases"][name]["keys"])} keys, {time.time() - t:.1f} s, load {l0}->{load1()}',
                  flush=True)
        h.settle_saves()
    finally:
        rss.append(h.stop())
    # reopen: edit on disk, a new host from the persisted S0, page 1
    if a.reopen and (not a.phases or 'reopen' in a.phases):
        at = body_line(open(src).read(), 0.5)
        for i in range(a.reopen):
            text = open(f'{work}/main.tex').read()
            text = text[:at] + 'x' + text[at:] if i % 2 == 0 else text[:at] + text[at + 1:]
            open(f'{work}/main.tex', 'w').write(text)
            l0 = load1()
            h = Host(eng, sock, s0, log, hostargs)
            try:
                recs, _ = keys(eng, sock, work, ['--keys', '0'], f'{out}/raw/{doc}-reopen.jsonl', a.timeout)
                h.settle_saves(quiet=0.5)
            finally:
                rss.append(h.stop())
            first = recs[0] if recs else {}
            fp = first.get('first_page_ms')
            res['reopen'].append(dict(ready_ms=round(h.ready_ms, 1), first_page_ms=fp,
                                      cold_ms=None if fp is None else round(h.ready_ms + fp, 1),
                                      mode=first.get('host', {}).get('mode'), load=[l0, load1()]))
        print(f'  {doc} reopen: {[r["mode"] for r in res["reopen"]]}', flush=True)
    res['rss_peak'] = max((r for r in rss if r), default=None)
    return res


def summarise(raw):
    """Rows from the per-document results."""
    rows = []
    for d in raw:
        for name, ph in d['phases'].items():
            ks = ph['keys']
            pre = name == 'preamble'
            v = [k['first_page_ms'] if pre else k['edited_page_ms'] for k in ks]
            got = [x for x in v if x is not None]
            hosts = [k.get('host', {}) for k in ks]
            rows.append(dict(
                doc=d['doc'], edit=name, metric='first visible page' if pre else 'edited page',
                target=TARGET['preamble' if pre else 'edit'], n=len(ks), missing=len(v) - len(got), **{'ms': stats(got)},
                first_page=stats([k['first_page_ms'] for k in ks if k.get('first_page_ms') is not None]),
                done=stats([k['done_ms'] for k in ks if 'done_ms' in k]),
                converged=sum(1 for x in hosts if x.get('converged_at') is not None),
                typeset_pages=stats([x.get('typeset_pages') or 0 for x in hosts]),
                later_pages=stats([k.get('later_pages', 0) for k in ks]),
                stale_unmarked=sum(1 for k in ks if k.get('later_pages') and not k.get('stale_marked')),
                incomplete=sum(1 for k in ks if k.get('complete') is False),
                modes=sorted({x.get('mode') for x in hosts if x.get('mode')}),
                status=sorted({x.get('status') for x in hosts if x.get('status')}),
                load=ph['load'], pages=d.get('pages'), rss=d.get('rss_peak')))
        if d['reopen']:
            ro = d['reopen']
            for kind, key in (('reopen (pre-warmed host)', 'first_page_ms'), ('reopen (cold: host start + page 1)', 'cold_ms')):
                v = [r[key] for r in ro if r[key] is not None]
                rows.append(dict(doc=d['doc'], edit=kind, metric='first visible page', target=TARGET['reopen'], n=len(ro),
                                 missing=len(ro) - len(v), ms=stats(v), modes=sorted({r['mode'] for r in ro if r['mode']}),
                                 load=[min(r['load'][0] for r in ro), max(r['load'][1] for r in ro)],
                                 pages=d.get('pages'), rss=d.get('rss_peak')))
    return rows


def verdict(rows, margin, baseline, reopen_gate):
    fails = []
    for r in rows:
        tag = f"{r['doc']} {r['edit']}"
        gated = True
        if r['edit'].startswith('reopen (pre'):
            gated = reopen_gate == 'prewarmed'
        elif r['edit'].startswith('reopen (cold'):
            gated = reopen_gate == 'cold'
        r['gated'] = gated
        why = []
        if r['missing']:
            why.append(f"{r['missing']} of {r['n']} without the watched page")
        p95 = r['ms']['p95']
        if p95 is not None and p95 > r['target'] * (1 + margin):
            why.append(f"p95 {p95:.1f} ms > {r['target']:.0f} ms")
        if r.get('stale_unmarked'):
            why.append(f"{r['stale_unmarked']} compiles refreshed later pages without marking them stale")
        if r.get('incomplete'):
            why.append(f"{r['incomplete']} compiles ended with pages not current")
        if r.get('modes') and r['edit'].startswith('reopen') and r['modes'] != ['open']:
            why.append(f"reopen modes {r['modes']} (expected open: from S0)")
        b = (baseline or {}).get(tag)
        if b and 'converged' in r and r['n']:
            rate, brate = r['converged'] / r['n'], b['converged_rate']
            if rate < brate - CONV_TOL:
                why.append(f'convergence {rate:.2f} < baseline {brate:.2f}')
            bp = b['typeset_pages_p50']
            if r['typeset_pages']['p50'] is not None and r['typeset_pages']['p50'] > bp * BG_TOL[0] + BG_TOL[1]:
                why.append(f"re-typeset pages p50 {r['typeset_pages']['p50']} > baseline {bp}")
        r['why'] = why
        r['pass'] = not why
        if why and gated:
            fails.append(f'{tag}: ' + '; '.join(why))
    return fails


def f(x, nd=1):
    return '-' if x is None else (f'{x:.{nd}f}' if isinstance(x, float) else str(x))


def table(rows, margin):
    out = ['| doc | pages | edit | n | ms p50 / p95 / max | first changed page p95 | target p95 | DONE p95 ms '
           '| converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |',
           '|---|---|---|---|---|---|---|---|---|---|---|---|---|']
    for r in rows:
        m = r['ms']
        conv = f"{r['converged']}/{r['n']}" if 'converged' in r else '-'
        tp = f"{f(r['typeset_pages']['p50'])} / {f(r['typeset_pages']['max'])}" if 'typeset_pages' in r else '-'
        fp = f(r['first_page']['p95']) if 'first_page' in r else '-'
        dn = f(r['done']['p95']) if 'done' in r and r['done']['n'] else '-'
        rss = f"{r['rss'] / 2**20:.0f} MB" if r.get('rss') else '-'
        v = ('PASS' if r['pass'] else 'MISS: ' + '; '.join(r['why'])) if r['gated'] else ('(not gated) ' + ('ok' if r['pass'] else '; '.join(r['why'])))
        out.append(f"| {r['doc']} | {f(r.get('pages'))} | {r['edit']} | {r['n']} | {f(m['p50'])} / {f(m['p95'])} / {f(m['max'])} "
                   f"| {fp} | {r['target']:.0f} | {dn} | {conv} | {tp} | {rss} | {r['load'][0]}-{r['load'][1]} | {v} |")
    out.append(f'\nms: COMPILE to the watched page\'s PAGE frame on the socket (preamble, reopen: page 1); '
               f'a p95 may exceed its target by {margin:.0%} (noise margin) before it misses.')
    return '\n'.join(out)


def baseline_of(rows):
    return {f"{r['doc']} {r['edit']}": dict(converged_rate=round(r['converged'] / r['n'], 3),
                                            typeset_pages_p50=r['typeset_pages']['p50'])
            for r in rows if 'converged' in r and r['n']}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--engine', default='t7')
    ap.add_argument('--build', action='store_true', help='cargo build --release, mkeng.sh, mkdocs.py')
    ap.add_argument('--docs', default=','.join(DOCS))
    ap.add_argument('--phases', default='', help='comma list of phases (and/or reopen); default all')
    ap.add_argument('--keys', type=int, default=20)
    ap.add_argument('--preamble', type=int, default=6)
    ap.add_argument('--reopen', type=int, default=6)
    ap.add_argument('--gap-ms', type=int, default=300)
    ap.add_argument('--timeout', type=int, default=3600, help='per dl3-keys connection, s')
    ap.add_argument('--margin', type=float, default=0.10)
    ap.add_argument('--host-args', default='')
    ap.add_argument('--out')
    ap.add_argument('--baseline', default=os.path.join(S, 't7-baseline.json'))
    ap.add_argument('--write-baseline')
    ap.add_argument('--reopen-gate', choices=['prewarmed', 'cold', 'none'], default='prewarmed')
    ap.add_argument('--wait-load', type=float, help='wait until the 1-minute load is below this')
    ap.add_argument('--wait-max', type=int, default=1800)
    ap.add_argument('--quick', action='store_true', help='plain-10,full-100; 8 keys, 2 preamble, 2 reopen')
    ap.add_argument('--check', help='re-evaluate an existing summary.json')
    a = ap.parse_args()
    a.preamble += a.preamble % 2  # even: the S0 left behind is the document's own
    baseline = json.load(open(a.baseline)) if a.baseline and os.path.exists(a.baseline) else None
    if a.check:
        summ = json.load(open(a.check))
        rows = summarise(summ['raw'])
        fails = verdict(rows, a.margin, baseline, a.reopen_gate)
        print(table(rows, a.margin))
        print('\nT7: ' + ('PASS' if not fails else f'{len(fails)} MISSES\n  ' + '\n  '.join(fails)))
        return 1 if fails else 0
    if a.quick:
        a.docs, a.keys, a.preamble, a.reopen = 'plain-10,full-100', 8, 2, 2
    a.phases = [p for p in a.phases.split(',') if p]
    eng = f'{IB}/{a.engine}'
    out = a.out or f'{IB}/t7/{time.strftime("%Y%m%dT%H%M%S")}'
    os.makedirs(f'{out}/raw', exist_ok=True)
    env_lines = [f'start {time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())} {uptime()}',
                 subprocess.run(['uname', '-srm'], capture_output=True, text=True).stdout.strip()]
    if sys.platform == 'darwin':
        env_lines.append(subprocess.run(['sysctl', '-n', 'machdep.cpu.brand_string', 'hw.ncpu', 'hw.memsize'],
                                        capture_output=True, text=True).stdout.replace('\n', ' ').strip())
        env_lines += power()
        # no idle sleep while measuring (a sleeping Mac stops the clocks the keystrokes are timed by)
        subprocess.Popen(['caffeinate', '-i', '-w', str(os.getpid())])
    if a.build:
        jobs = os.environ.get('CARGO_BUILD_JOBS', '6')
        b = subprocess.run(['cargo', 'build', '--release', '-p', 'flashtex-engine', '-p', 'flashtex-display-list'], cwd=W,
                           env=dict(os.environ, CARGO_BUILD_JOBS=jobs))
        if b.returncode or subprocess.run([f'{S}/mkeng.sh', a.engine], cwd=W).returncode:
            print('t7: build failed', file=sys.stderr)
            return 2
    if not os.path.exists(f'{IB}/docs/full-1000/main.tex'):
        subprocess.run([sys.executable, f'{S}/mkdocs.py'], check=True)
    for need in ('flashtex-host', 'dl3-keys', 'pdftex.pool'):
        if not os.path.exists(f'{eng}/{need}'):
            print(f't7: {eng}/{need} missing (run with --build, or mkeng.sh {a.engine})', file=sys.stderr)
            return 2
    env_lines.append(f'engine {eng} HEAD {open(f"{eng}/HEAD").read().strip() if os.path.exists(f"{eng}/HEAD") else "?"}; '
                     f'checkout {subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=W, capture_output=True, text=True).stdout.strip()}')
    if a.wait_load:
        end = time.time() + a.wait_max
        while load1() >= a.wait_load and time.time() < end:
            time.sleep(15)
        env_lines.append(f'waited for load1 < {a.wait_load}: now {load1()}')
    os.makedirs(f'{IB}/t7-work', exist_ok=True)
    raw = []
    error = None
    try:
        for doc in [d for d in a.docs.split(',') if d]:
            print(f'{doc}:', flush=True)
            raw.append(run_doc(a, eng, doc, out))
    except RuntimeError as e:
        error = str(e)
        print(f't7: {e} (the documents before it are summarised)', file=sys.stderr)
    env_lines.append(f'end {time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())} {uptime()}')
    if sys.platform == 'darwin':
        env_lines += power()
    open(f'{out}/environment.txt', 'w').write('\n'.join(env_lines) + '\n')
    rows = summarise(raw)
    fails = verdict(rows, a.margin, baseline, a.reopen_gate)
    t = table(rows, a.margin)
    summ = dict(environment=env_lines, args={k: v for k, v in vars(a).items()}, rows=rows, fails=fails, raw=raw, error=error)
    json.dump(summ, open(f'{out}/summary.json', 'w'), indent=1)
    open(f'{out}/table.md', 'w').write(t + '\n')
    if a.write_baseline and not error:
        json.dump(baseline_of(rows), open(a.write_baseline, 'w'), indent=1, sort_keys=True)
    print('\n'.join(env_lines))
    print(t)
    print(f'\nT7: ' + ('PASS' if not fails else f'{len(fails)} MISSES\n  ' + '\n  '.join(fails)))
    print(f'output: {out}')
    return 2 if error else 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
