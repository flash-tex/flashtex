#!/usr/bin/env python3
"""t7.py: the T7 latency gate (DESIGN.md §8 T7, §1.2's targets, §12's P4 exit gate).

usage: t7.py [--build] [--engine NAME] [--docs plain-10,full-1000,...] [--phases P,...]
             [--keys N] [--preamble N] [--reopen N] [--gap-ms MS] [--order fixed|rotate|shuffle]
             [--out DIR] [--baseline FILE] [--write-baseline FILE] [--max-load L]
             [--require-reference] [--wait-load L] [--wait-max S] [--host-args ARGS] [--quick]
       t7.py --check OUT/summary.json [--baseline FILE] [--max-load L] [--require-reference]

Everything goes through the engine host's Unix socket with the display-list-v3 protocol
(docs/protocol/display-list-v3.md), as the app talks to it: `flashtex-host --socket` is started
here as a separate process, and the keystrokes are typed by `dl3-keys` (the harness's socket
client, crates/display-list-v3/src/bin/dl3-keys.rs). One host session per document, as the app
runs one host per open document. The phases:

  letter@start, letter@middle, letter@end   a letter inserted into a word and deleted again, in a
                                            prose line on a page 2 %, 50 % and 98 % into the document
  sentence@middle                           twelve words inserted and deleted (the paragraph reflows)
  newline@middle                            the space after a word becomes a line break and back
                                            (edits.py's `newline`: only later input lines move)
  split@middle                              that space becomes a blank line (edits.py's `split`), and
                                            the blank line a space again (edits.py's `join` of it)
  preamble                                  a \\newcommand line after the \\documentclass line, added
                                            and removed: S0 changes, a full run from the format

The six in-body phases run in `--order` (default `rotate`: document i starts at phase i mod 6, so
no phase always follows the same one: each phase carries the host's state, its checkpoint history
and its RSS, over to the next); the preamble always runs after them (its full runs replace the
history), then `reopen`: the host is stopped, the document is edited on disk, and a new host
process with the same S0 cache opens it (DESIGN.md §5.1).

Measured per keystroke, client side of the socket: COMPILE written to the watched (edited)
page's PAGE frame read (`edited_page_ms`): **the host's share of the gated edit quantity**
(DESIGN.md §1.2, owner decision 1: key event -> preview commit <= 16 ms p95, split host <= 11 ms
and app <= 4 ms); to the first PAGE frame (`first_page_ms`: the first changed page) and to DONE
(`done_ms`: the background re-typesetting); from the host's DONE whether the run converged
(`converged_at`) and how many pages it re-typeset (`typeset_pages`); and whether the client's
pages were kept current: later pages marked stale before they were refreshed, all current at DONE
(dl3-keys' `stale_marked`, `complete`; a missing `complete` counts as not current). Host RSS: the
peak (`wait4` ru_maxrss) of each host.

Samples: the first keystroke of every phase (and the first reopen) is a warm-up and is dropped
from the statistics (kept in raw). `--keys` and `--preamble` are the keystrokes sent, forced even
(every edit is undone, so the next phase sees the document unchanged): 20 keys give 19 samples,
14 preamble edits 13. `--reopen 13` gives 12. A row with fewer than 12 samples is gated on its
maximum, not its p95.

Gate (§1.2):
  in-body edits   host share <= 11 ms p95 (every kind, any size; decision 1)
  preamble        <= 400 ms to the first visible page (page 1, the viewport): host side; §1.2 gives
                  no host/app split for it
  reopen          report-only. Decision 8: a pre-warmed host does not count, and reopen is met by
                  the app's stored pages, which a host-socket harness cannot see. Reported: the
                  host's share of a cold reopen (spawn -> listening -> page 1 from the persisted
                  S0) and, for comparison only, page 1 from a host already listening.
A row passes only when it meets its target: there is no noise margin.
Convergence and background pages are "measured and held" (§5.3, §12): with `--baseline`
(default tools/incr-bench/t7-baseline.json) a row fails when its convergence rate falls more than
0.15 below the baseline's or its median re-typeset pages exceed the baseline's by more than
25 % + 2; a row the baseline lacks is reported as a warning.

Reference conditions: a run is NON-REFERENCE (said in the table and the verdict, and in `--check`)
when it was on battery, in macOS Low Power Mode, under a thermal or CPU speed limit, with a load1
above `--max-load` (default half the cores) before, during or after, or when its power state was
not recorded. Its misses still count; `--require-reference` makes a non-reference run that
otherwise passes exit 3.

Exit: 0 all gated targets met; 1 a target missed (or a page never arrived, or pages were not kept
current, or a held rate fell); 2 the harness failed (an unknown --docs or --phases value, a host
or dl3-keys failure: the summary of what ran is still written); 3 see --require-reference.
Output: OUT/raw/*.jsonl (dl3-keys' lines), OUT/summary.json, OUT/table.md, OUT/environment.txt.
"""
import argparse
import json
import os
import queue
import random
import shutil
import subprocess
import sys
import threading
import time

S = os.path.dirname(os.path.abspath(__file__))
W = os.path.dirname(os.path.dirname(S))
IB = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
DOCS = ['plain-10', 'full-10', 'plain-100', 'full-100', 'plain-300', 'full-300', 'plain-1000', 'full-1000']
# (phase, dl3-keys arguments); the in-body ones are ordered by --order, the preamble runs last
BODY_PHASES = [
    ('letter@start', ['--kind', 'letter', '--at', '0.02']),
    ('letter@middle', ['--kind', 'letter', '--at', '0.5']),
    ('letter@end', ['--kind', 'letter', '--at', '0.98']),
    ('sentence@middle', ['--kind', 'sentence', '--at', '0.5']),
    ('newline@middle', ['--kind', 'newline', '--at', '0.5']),
    ('split@middle', ['--kind', 'split', '--at', '0.5']),
]
PREAMBLE = ('preamble', ['--kind', 'preamble', '--at', '0.5'])
PHASE_NAMES = [p for p, _ in BODY_PHASES] + [PREAMBLE[0], 'reopen']
# §1.2: the host's share of the edit gate (decision 1), the preamble row
TARGET = {'edit': 11.0, 'preamble': 400.0}
REOPEN_APP_TARGET = 100.0  # the app's row (stored pages), for reference only
SMALL_N = 12
CONV_TOL, BG_TOL = 0.15, (1.25, 2)
REOPEN_COLD = "reopen: host's share, cold (spawn -> page 1)"
REOPEN_WARM = 'reopen: host already listening (not counted, decision 8)'


def pct(v, p):
    v = sorted(v)
    return v[round((len(v) - 1) * p)] if v else None


def stats(v):
    return dict(n=len(v), p50=pct(v, .5), p95=pct(v, .95), max=pct(v, 1.0))


def uptime():
    return subprocess.run(['uptime'], capture_output=True, text=True).stdout.strip()


def load1():
    return round(os.getloadavg()[0], 2)


def run(cmd):
    try:
        return subprocess.run(cmd, capture_output=True, text=True).stdout
    except OSError:
        return ''


def power_state():
    """The conditions that make a latency run a reference run or not: power source, battery,
    Low Power Mode, thermal/CPU speed limits (macOS `pmset`; Linux: the mains supply only), load."""
    p = dict(source=None, battery_pct=None, low_power_mode=None, thermal=None, thermal_limited=None,
             load=[round(x, 2) for x in os.getloadavg()], ncpu=os.cpu_count())
    if sys.platform == 'darwin':
        batt = run(['pmset', '-g', 'batt'])
        p['source'] = 'AC' if "'AC Power'" in batt else 'battery' if "'Battery Power'" in batt else None
        for tok in batt.split():
            if tok.endswith('%;') and tok[:-2].isdigit():
                p['battery_pct'] = int(tok[:-2])
        for l in run(['pmset', '-g']).split('\n'):
            if 'lowpowermode' in l:
                p['low_power_mode'] = l.split()[-1] == '1'
        therm = [l.strip() for l in run(['pmset', '-g', 'therm']).split('\n') if l.strip()]
        p['thermal'] = '; '.join(therm)
        limited = False
        for l in therm:
            if '=' in l:
                k, v = [x.strip() for x in l.split('=', 1)]
                if k.endswith('Speed_Limit') and v.isdigit() and int(v) < 100:
                    limited = True
            elif 'level' in l.lower() and not l.startswith('Note: No'):
                limited = True
        p['thermal_limited'] = limited
    elif sys.platform.startswith('linux'):
        base = '/sys/class/power_supply'
        try:
            for d in os.listdir(base):
                if open(f'{base}/{d}/type').read().strip() == 'Mains':
                    p['source'] = 'AC' if open(f'{base}/{d}/online').read().strip() == '1' else 'battery'
        except OSError:
            pass
        if p['source'] is None and os.path.isdir(base) and not os.listdir(base):
            p['source'] = 'AC'  # no supply at all: a desktop or server
    return p


def reference_issues(summ, rows, max_load=None):
    """Why a run is not a reference run ([] when it is)."""
    pw = summ.get('power') or {}
    before, after = pw.get('before'), pw.get('after')
    out = [] if before else ['power state not recorded']
    for when, s in (('start', before or {}), ('end', after or {})):
        if s.get('source') == 'battery':
            out.append(f'on battery at {when} ({s.get("battery_pct")} %)')
        elif s.get('source') is None and s:
            out.append(f'power source unknown at {when}')
        if s.get('low_power_mode'):
            out.append(f'Low Power Mode on at {when}')
        if s.get('thermal_limited'):
            out.append(f'thermal limit at {when}: {s.get("thermal")}')
    limit = max_load if max_load is not None else ((before or {}).get('ncpu') or os.cpu_count() or 2) / 2
    loads = [x['load'][0] for x in (before, after) if x] + [x for r in rows for x in r.get('load', [])]
    if loads and max(loads) > limit:
        out.append(f'load1 up to {max(loads):.1f} (> {limit:.1f})')
    return list(dict.fromkeys(out))


class Host:
    """flashtex-host --socket, our own child; stopped by its handle (never by name)."""

    def __init__(self, eng, sock, s0, log, extra):
        if os.path.exists(sock):
            os.unlink(sock)
        env = dict(os.environ, FLASHTEX_POOL=f'{eng}/pdftex.pool',
                   FLASHTEX_FORMATS=os.path.join(os.path.dirname(eng), 'fmt-' + os.path.basename(eng)),
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


def phase_order(order, i, seed):
    """The in-body phases for the i-th document."""
    ph = list(BODY_PHASES)
    if order == 'rotate':
        k = i % len(ph)
        ph = ph[k:] + ph[:k]
    elif order == 'shuffle':
        random.Random(seed + i).shuffle(ph)
    return ph + [PREAMBLE]


def run_doc(a, eng, doc, out, i):
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
    res = dict(doc=doc, phases={}, reopen=[], order=[])
    h = Host(eng, sock, s0, log, hostargs)
    res['host_startup'], res['host_ready_ms'] = h.startup, round(h.ready_ms, 1)
    rss = []
    try:
        for name, args in phase_order(a.order, i, a.seed):
            if name not in a.phases:
                continue
            res['order'].append(name)
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
    if a.reopen and 'reopen' in a.phases:
        at = body_line(open(src).read(), 0.5)
        for k in range(a.reopen):
            text = open(f'{work}/main.tex').read()
            text = text[:at] + 'x' + text[at:] if k % 2 == 0 else text[:at] + text[at + 1:]
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


def gated_stat(m):
    """('p95', value), or ('max', value) for fewer than SMALL_N samples."""
    return ('p95', m['p95']) if m['n'] >= SMALL_N else ('max', m['max'])


def summarise(raw):
    """Rows from the per-document results (the first keystroke of each phase and the first reopen
    are warm-ups and are left out)."""
    rows = []
    for d in raw:
        for name, ph in d['phases'].items():
            ks = ph['keys'][1:]
            pre = name == 'preamble'
            v = [k['first_page_ms'] if pre else k['edited_page_ms'] for k in ks]
            got = [x for x in v if x is not None]
            hosts = [k.get('host', {}) for k in ks]
            rows.append(dict(
                doc=d['doc'], edit=name, metric='first visible page' if pre else 'edited page (host share)',
                target=TARGET['preamble' if pre else 'edit'], gated=True, n=len(ks), missing=len(v) - len(got),
                ms=stats(got),
                first_page=stats([k['first_page_ms'] for k in ks if k.get('first_page_ms') is not None]),
                done=stats([k['done_ms'] for k in ks if 'done_ms' in k]),
                converged=sum(1 for x in hosts if x.get('converged_at') is not None),
                typeset_pages=stats([x.get('typeset_pages') or 0 for x in hosts]),
                later_pages=stats([k.get('later_pages', 0) for k in ks]),
                stale_unmarked=sum(1 for k in ks if k.get('later_pages') and not k.get('stale_marked')),
                incomplete=sum(1 for k in ks if k.get('complete') is not True),
                modes=sorted({x.get('mode') for x in hosts if x.get('mode')}),
                status=sorted({x.get('status') for x in hosts if x.get('status')}),
                load=ph['load'], pages=d.get('pages'), rss=d.get('rss_peak')))
        ro = d.get('reopen', [])[1:]
        if ro:
            for kind, key in ((REOPEN_COLD, 'cold_ms'), (REOPEN_WARM, 'first_page_ms')):
                v = [r[key] for r in ro if r[key] is not None]
                rows.append(dict(doc=d['doc'], edit=kind, metric='first visible page', target=REOPEN_APP_TARGET,
                                 gated=False, n=len(ro), missing=len(ro) - len(v), ms=stats(v),
                                 modes=sorted({r['mode'] for r in ro if r['mode']}),
                                 load=[min(r['load'][0] for r in ro), max(r['load'][1] for r in ro)],
                                 pages=d.get('pages'), rss=d.get('rss_peak')))
    return rows


def verdict(rows, baseline):
    """The gated misses; sets each row's `why`, `pass`, `stat`. Rows the baseline lacks get a
    `warning`."""
    fails = []
    for r in rows:
        tag = f"{r['doc']} {r['edit']}"
        why = []
        if r['missing']:
            why.append(f"{r['missing']} of {r['n']} without the watched page")
        if not r['n']:
            why.append('no samples')
        name, val = gated_stat(r['ms'])
        r['stat'] = name
        if val is not None and val > r['target']:
            why.append(f"{name} {val:.1f} ms > {r['target']:.0f} ms")
        if r.get('stale_unmarked'):
            why.append(f"{r['stale_unmarked']} compiles refreshed later pages without marking them stale")
        if r.get('incomplete'):
            why.append(f"{r['incomplete']} compiles ended without all pages current")
        if r.get('modes') and r['edit'].startswith('reopen') and r['modes'] != ['open']:
            why.append(f"reopen modes {r['modes']} (expected open: from S0)")
        if baseline is not None and 'converged' in r and r['n']:
            b = baseline.get(tag)
            if b is None:
                r['warning'] = 'no baseline row'
            else:
                rate, brate = r['converged'] / r['n'], b['converged_rate']
                if rate < brate - CONV_TOL:
                    why.append(f'convergence {rate:.2f} < baseline {brate:.2f}')
                bp = b['typeset_pages_p50']
                if r['typeset_pages']['p50'] is not None and r['typeset_pages']['p50'] > bp * BG_TOL[0] + BG_TOL[1]:
                    why.append(f"re-typeset pages p50 {r['typeset_pages']['p50']} > baseline {bp}")
        r['why'] = why
        r['pass'] = not why
        if why and r['gated']:
            fails.append(f'{tag}: ' + '; '.join(why))
    return fails


def f(x, nd=1):
    return '-' if x is None else (f'{x:.{nd}f}' if isinstance(x, float) else str(x))


def table(rows, issues):
    ref = ('**NON-REFERENCE run**: ' + '; '.join(issues)) if issues else 'Reference run.'
    out = [ref, '',
           '| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms '
           '| converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |',
           '|---|---|---|---|---|---|---|---|---|---|---|---|---|---|']
    for r in rows:
        m = r['ms']
        conv = f"{r['converged']}/{r['n']}" if 'converged' in r else '-'
        tp = f"{f(r['typeset_pages']['p50'])} / {f(r['typeset_pages']['max'])}" if 'typeset_pages' in r else '-'
        fp = f(r['first_page']['p95']) if 'first_page' in r else '-'
        dn = f(r['done']['p95']) if 'done' in r and r['done']['n'] else '-'
        rss = f"{r['rss'] / 2**20:.0f} MB" if r.get('rss') else '-'
        if r['gated']:
            v = 'PASS' if r['pass'] else 'MISS: ' + '; '.join(r['why'])
            tgt = f"{r['target']:.0f}"
        else:
            v = 'report-only' + ('' if r['pass'] else ': ' + '; '.join(r['why']))
            tgt = f"({r['target']:.0f}, app)"
        if r.get('warning'):
            v += f" ({r['warning']})"
        out.append(f"| {r['doc']} | {f(r.get('pages'))} | {r['edit']} | {r['n']} | {f(m['p50'])} / {f(m['p95'])} / {f(m['max'])} "
                   f"| {r.get('stat', '-')} | {tgt} | {fp} | {dn} | {conv} | {tp} | {rss} | {r['load'][0]}-{r['load'][1]} | {v} |")
    out.append("\nms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). "
               f'The first sample of each phase is a warm-up, left out; rows with n < {SMALL_N} are gated on their '
               'maximum. No noise margin: a row passes only when it meets its target.')
    return '\n'.join(out)


def baseline_of(rows):
    return {f"{r['doc']} {r['edit']}": dict(converged_rate=round(r['converged'] / r['n'], 3),
                                            typeset_pages_p50=r['typeset_pages']['p50'])
            for r in rows if 'converged' in r and r['n']}


def report(summ, baseline, max_load, require_reference):
    """Evaluate a summary (fresh or `--check`): print the table and the verdict; the exit code."""
    rows = summarise(summ.get('raw', []))
    fails = verdict(rows, baseline)
    issues = reference_issues(summ, rows, max_load)
    t = table(rows, issues)
    print(t)
    warn = [f"{r['doc']} {r['edit']}: {r['warning']}" for r in rows if r.get('warning')]
    for w in warn:
        print(f'warning: {w}')
    gated = sum(1 for r in rows if r['gated'])
    ref = 'NON-REFERENCE (' + '; '.join(issues) + ')' if issues else 'reference'
    head = (f'{len(fails)} MISSES' if fails else 'INCOMPLETE (harness error), no miss so far' if summ.get('error')
            else 'NOTHING GATED' if not gated else 'PASS')
    print(f'\nT7: {head} ({gated - len(fails)} of {gated} gated rows pass; {ref} run)')
    for x in fails:
        print('  ' + x)
    if summ.get('error'):
        print(f"T7: HARNESS ERROR: {summ['error']}")
    code = 2 if summ.get('error') else 1 if fails else 3 if (issues and require_reference) else 0
    return code, rows, fails, issues, t


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--engine', default='t7')
    ap.add_argument('--build', action='store_true', help='cargo build --release, mkeng.sh, mkdocs.py')
    ap.add_argument('--docs')
    ap.add_argument('--phases', help='comma list of ' + ','.join(PHASE_NAMES) + '; default all')
    ap.add_argument('--keys', type=int, help='keystrokes per in-body phase, forced even (default 20)')
    ap.add_argument('--preamble', type=int, help='preamble edits, forced even (default 14)')
    ap.add_argument('--reopen', type=int, help='reopens (default 13)')
    ap.add_argument('--gap-ms', type=int, default=300)
    ap.add_argument('--order', choices=['fixed', 'rotate', 'shuffle'], default='rotate')
    ap.add_argument('--seed', type=int, default=7)
    ap.add_argument('--timeout', type=int, default=3600, help='per dl3-keys connection, s')
    ap.add_argument('--host-args', default='')
    ap.add_argument('--out')
    ap.add_argument('--baseline', default=os.path.join(S, 't7-baseline.json'))
    ap.add_argument('--write-baseline')
    ap.add_argument('--max-load', type=float, help='load1 above this makes a run non-reference (default: half the cores)')
    ap.add_argument('--require-reference', action='store_true', help='a non-reference run that passes exits 3')
    ap.add_argument('--wait-load', type=float, help='wait until the 1-minute load is below this')
    ap.add_argument('--wait-max', type=int, default=1800)
    ap.add_argument('--quick', action='store_true', help='defaults plain-10,full-100; 8 keys, 2 preamble, 3 reopen')
    ap.add_argument('--check', help='re-evaluate an existing summary.json')
    a = ap.parse_args()
    baseline = json.load(open(a.baseline)) if a.baseline and os.path.exists(a.baseline) else None
    if a.check:
        summ = json.load(open(a.check))
        return report(summ, baseline, a.max_load, a.require_reference)[0]
    quick = dict(docs='plain-10,full-100', keys=8, preamble=2, reopen=3)
    full = dict(docs=','.join(DOCS), keys=20, preamble=14, reopen=13)
    for k, v in (quick if a.quick else full).items():
        if getattr(a, k) is None:
            setattr(a, k, v)
    a.keys += a.keys % 2  # every edit undone: the next phase sees the document unchanged
    a.preamble += a.preamble % 2  # and the S0 left behind is the document's own
    eng = f'{IB}/{a.engine}'
    out = a.out or f'{IB}/t7/{time.strftime("%Y%m%dT%H%M%S")}'
    os.makedirs(f'{out}/raw', exist_ok=True)
    error = None
    a.phases = PHASE_NAMES if a.phases is None else [p.strip() for p in a.phases.split(',') if p.strip()]
    bad = [p for p in a.phases if p not in PHASE_NAMES]
    if bad or not a.phases:
        error = f'unknown or empty --phases {bad or a.phases}; known: {",".join(PHASE_NAMES)}'
    docs = [d.strip() for d in a.docs.split(',') if d.strip()]
    known = set(DOCS) | (set(os.listdir(f'{IB}/docs')) if os.path.isdir(f'{IB}/docs') else set())
    if not error and (not docs or [d for d in docs if d not in known]):
        error = f'unknown or empty --docs {[d for d in docs if d not in known] or docs}; known: {",".join(sorted(known))}'
    env_lines = [f'start {time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())} {uptime()}',
                 run(['uname', '-srm']).strip()]
    if sys.platform == 'darwin':
        env_lines.append(run(['sysctl', '-n', 'machdep.cpu.brand_string', 'hw.ncpu', 'hw.memsize']).replace('\n', ' ').strip())
    raw = []
    power = dict(before=None, after=None)
    if not error:
        if a.build:
            jobs = os.environ.get('CARGO_BUILD_JOBS', '6')
            b = subprocess.run(['cargo', 'build', '--release', '-p', 'flashtex-engine', '-p', 'flashtex-display-list'], cwd=W,
                               env=dict(os.environ, CARGO_BUILD_JOBS=jobs))
            if b.returncode or subprocess.run([f'{S}/mkeng.sh', a.engine], cwd=W).returncode:
                error = 'build failed'
        if not error and not os.path.exists(f'{IB}/docs/full-1000/main.tex'):
            subprocess.run([sys.executable, f'{S}/mkdocs.py'], check=True)
        for need in ('flashtex-host', 'dl3-keys', 'pdftex.pool'):
            if not error and not os.path.exists(f'{eng}/{need}'):
                error = f'{eng}/{need} missing (run with --build, or mkeng.sh {a.engine})'
    if not error:
        env_lines.append(f'engine {eng} HEAD {open(f"{eng}/HEAD").read().strip() if os.path.exists(f"{eng}/HEAD") else "?"}; '
                         f'checkout {run(["git", "-C", W, "rev-parse", "--short", "HEAD"]).strip()}')
        if a.wait_load:
            end = time.time() + a.wait_max
            while load1() >= a.wait_load and time.time() < end:
                time.sleep(15)
            env_lines.append(f'waited for load1 < {a.wait_load}: now {load1()}')
        if sys.platform == 'darwin':
            # no idle sleep while measuring (a sleeping Mac stops the clock the keystrokes are timed by)
            subprocess.Popen(['caffeinate', '-i', '-w', str(os.getpid())])
        power['before'] = power_state()
        if power['before'].get('low_power_mode'):
            print('t7: WARNING: macOS Low Power Mode is on: a non-reference run', file=sys.stderr)
        os.makedirs(f'{IB}/t7-work', exist_ok=True)
        try:
            for i, doc in enumerate(docs):
                print(f'{doc}:', flush=True)
                raw.append(run_doc(a, eng, doc, out, i))
        except RuntimeError as e:
            error = str(e)
            print(f't7: {e} (the documents before it are summarised)', file=sys.stderr)
        power['after'] = power_state()
    env_lines.append(f'end {time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())} {uptime()}')
    open(f'{out}/environment.txt', 'w').write('\n'.join(env_lines) + '\n')
    summ = dict(environment=env_lines, power=power, args=dict(vars(a)), raw=raw, error=error)
    print('\n'.join(env_lines))
    code, rows, fails, issues, t = report(summ, baseline, a.max_load, a.require_reference)
    summ.update(rows=rows, fails=fails, reference_issues=issues)
    json.dump(summ, open(f'{out}/summary.json', 'w'), indent=1)
    open(f'{out}/table.md', 'w').write(t + '\n')
    if a.write_baseline and not error:
        json.dump(baseline_of(rows), open(a.write_baseline, 'w'), indent=1, sort_keys=True)
    print(f'output: {out}')
    return code


if __name__ == '__main__':
    sys.exit(main())
