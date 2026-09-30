#!/usr/bin/env python3
"""L2-L5 benchmark and soundness test driver (P4-L2-L3, extended for P4-L5).

usage: incr_bench.py ENGINE SRCDIR DOC [--edit FILE] [--trials N] [--seed S]
                     [--verify] [--stop] [--kinds replace,insert,delete,...]
                     [--host-args "..."] [--region start|middle|end|any] [--out JSONL]

Copies SRCDIR to a work directory, starts `flashtex-host iserve` on DOC.tex,
settles it (compiles until nothing changes), then for each trial applies one
random edit to FILE (default DOC.tex), compiles, records the report and the
edited page's latency (the host's `edited`: the ship time of the first page the
first pass shipped whose frame changed), and with --verify runs the CLI engine
from scratch on the same inputs (the directory as it was before the compile,
with the edit) and compares the PDF, log, aux, out and toc byte for byte, and
the terminal. Then it reverts the edit and compiles (and verifies) again.

Multi-pass (DESIGN.md 5.5, P4-L5): a host compile runs further passes while
a pass changed a file it read (the .aux), up to 5, stopping on a repeated
state. The reference is the same rule applied to from-scratch runs: run the
CLI engine again while a run changed any file of the directory other than the
.pdf and .log, up to 5 runs, stopping when the files repeat a state an earlier
run started from. Only the final state is compared.

Edit kinds: replace, insert, delete (one letter of a prose word), sentence
(twelve words into a paragraph: reflows), and the structural kinds that
change the .aux/.toc: section (a new \\section before a paragraph), label (a
new \\label after a word), ref (a \\ref/\\pageref to an existing label), cite
(a \\cite of an existing \\bibitem), footnote (a new footnote), unlabel (an
existing \\label removed), unsection (an existing \\section removed).
"""
import argparse
import json
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile
import time

BASE = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')

ap = argparse.ArgumentParser()
ap.add_argument('engine')
ap.add_argument('srcdir')
ap.add_argument('doc')
ap.add_argument('--edit')
ap.add_argument('--trials', type=int, default=10)
ap.add_argument('--seed', type=int, default=1)
ap.add_argument('--verify', action='store_true')
ap.add_argument('--stop', action='store_true', help='compile with stop at the edited page, then finish')
ap.add_argument('--kinds', default='replace,insert,delete')
ap.add_argument('--region', default='any')
ap.add_argument('--host-args', default='')
ap.add_argument('--out')
ap.add_argument('--window', type=int, default=0, help='after the first edit, pick positions within this many bytes of it')
ap.add_argument('--keep', action='store_true')
ap.add_argument('--quiet', action='store_true')
ap.add_argument('--any-letter', action='store_true', help='with few prose positions, edit any letter of the body')
ap.add_argument('--no-revert', action='store_true', help='keep each edit (the next edits build on it)')
ap.add_argument('--interleave', action='store_true',
                help='interrupt each edit\'s compile (pass 1 or 2, after 1-4 pages) with a second edit, '
                     'which is then compiled and verified')
a = ap.parse_args()

import signal  # noqa: E402
# a time limit on the whole session (the host dies with its stdin)
signal.alarm(int(os.environ.get('INCR_BENCH_TIMEOUT', '5400')))
E = f'{BASE}/{a.engine}'
FMT = f'{BASE}/fmt-{a.engine}'
CLOCK = '1700000000.250000'
env = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1',
           FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=FMT, FLASHTEX_PIN_CLOCK=CLOCK, TZ='UTC')
work = tempfile.mkdtemp(prefix=f'incr-{a.doc}.', dir=BASE)
for n in os.listdir(a.srcdir):
    p = os.path.join(a.srcdir, n)
    if os.path.isfile(p):
        shutil.copy(p, work)
editfile = a.edit or f'{a.doc}.tex'
cmdline = ['-fmt=pdflatex', '-interaction=batchmode', f'{a.doc}.tex']
prof = os.environ.get('PROFILE_OUT')
pre = ['samply', 'record', '-s', '--unstable-presymbolicate', '-r', '4000', '-o', prof] if prof else []
host = subprocess.Popen(pre + [f'{E}/flashtex-host', 'iserve'] + a.host_args.split() + ['--'] + cmdline,
                        cwd=work, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                        stderr=sys.stderr if not a.quiet else open(a.out + '.host-stderr', 'a'),
                        text=True, bufsize=1)


def cmd(c):
    host.stdin.write(c + '\n')
    host.stdin.flush()
    line = host.stdout.readline()
    if not line:
        raise SystemExit(f'host died on {c}')
    d = json.loads(line)
    if 'error' in d:
        raise SystemExit(f'host error on {c}: {d["error"]}')
    return d


def snapshot(d):
    out = {}
    for n in os.listdir(d):
        p = os.path.join(d, n)
        if os.path.isfile(p):
            with open(p, 'rb') as f:
                out[n] = f.read()
    return out


def frames():
    return [p[0] for p in cmd('pages')['pages']]


def log(msg):
    if not a.quiet:
        print(msg, file=sys.stderr, flush=True)


# settle
t = time.time()
modes = []
for i in range(6):
    r = cmd('compile')
    modes.append(r['mode'] + (f'x{r.get("passes")}' if r.get('passes', 1) > 1 else ''))
    if r['mode'] == 'unchanged':
        break
log(f'settled: {modes} in {time.time()-t:.1f}s, {r["pages"]} pages')
base_frames = frames()

src = open(os.path.join(work, editfile), 'rb').read()
rng = random.Random(a.seed)


def prose_positions(src):
    """Letters inside prose words (not commands, not math) of the body."""
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
            if in_cmd or dollars % 2 == 1 or ch == '{' or ch == '}':
                continue
            if ch.isalpha() and ch.islower() and k > 0 and (chr(line[k - 1]).isalpha() or line[k - 1] == 32):
                cands.append(ls + k)
    if a.any_letter and len(cands) < 50:
        bs = max(body_start, 0)
        end = src.rfind(b'\\end{document}')
        end = end if end > bs else len(src)
        cands = [k for k in range(bs, end) if 97 <= src[k] <= 122]
    return cands


cands = prose_positions(src)
if a.region != 'any':
    n = len(cands)
    third = {'start': (0, n // 10), 'middle': (n * 45 // 100, n * 55 // 100), 'end': (n * 9 // 10, n)}[a.region]
    cands = cands[third[0]:third[1]]
kinds = a.kinds.split(',')
results = []
out = open(a.out, 'a') if a.out else None

# DESIGN.md 5.5: the reference is from-scratch runs repeated like the host's passes
MAX_PASSES = 5
SKIP = ('.pdf', '.log', '.synctex', '.synctex.gz', '.fls')


def state(d):
    return tuple(sorted((n, open(os.path.join(d, n), 'rb').read()) for n in os.listdir(d)
                        if os.path.isfile(os.path.join(d, n)) and not n.endswith(SKIP)))


def run_cli(pre, content, tag):
    d = tempfile.mkdtemp(prefix=f'scr-{tag}.', dir=BASE)
    for n, b in pre.items():
        with open(os.path.join(d, n), 'wb') as f:
            f.write(b)
    with open(os.path.join(d, editfile), 'wb') as f:
        f.write(content)
    e2 = dict(env, FLASHTEX_PREVIEW='1')
    seen = []
    runs = 0
    while True:
        before = state(d)
        seen.append(before)
        p = subprocess.run([f'{E}/pdftex'] + cmdline, cwd=d, env=e2, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=900)
        runs += 1
        after = state(d)
        if runs >= MAX_PASSES or after in seen:
            break
    res = snapshot(d)
    res['<stdout>'] = p.stdout
    res['<runs>'] = runs
    shutil.rmtree(d)
    return res


sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../../../tools/parity'))
from capture import split_accounting  # noqa: E402
ACCT = []
REFRUNS = {}


def compare(tag, pre, content):
    if not a.verify:
        return None
    got = snapshot(work)
    ref = run_cli(pre, content, tag)
    REFRUNS[tag] = ref['<runs>']
    bad = []
    for ext in ('pdf', 'log', 'aux', 'out', 'toc'):
        n = f'{a.doc}.{ext}'
        if n in ref or n in got:
            if ref.get(n) != got.get(n):
                if ext == 'log' and got.get(n) is not None and ref.get(n) is not None:
                    # DESIGN §1.1 P-T1 ruling N2: end-of-run capacity and
                    # output-size accounting is reported, not gated
                    sg, ag = split_accounting(got[n].decode('latin-1'))
                    sr, ar = split_accounting(ref[n].decode('latin-1'))
                    if sg == sr:
                        ACCT.append(tag)
                        continue
                bad.append(f'{n} ({len(got.get(n) or b"")} vs {len(ref.get(n) or b"")} bytes)')
                os.makedirs(BASE + '/mm', exist_ok=True)
                safe = tag.replace(':', '_').replace('@', '_')
                for k, v in (('got', got.get(n)), ('ref', ref.get(n))):
                    if v is not None:
                        open(f'{BASE}/mm/{a.doc}.{safe}.{k}.{ext}', 'wb').write(v)
    term = cmd('terminal')['terminal'].encode('utf-8', 'surrogateescape')
    if term.decode('utf-8', 'replace') != ref['<stdout>'].decode('utf-8', 'replace'):
        bad.append('terminal')
    return bad


def one(content, tag, interrupt=None, pre=None):
    """Write `content`, compile and verify. With `interrupt=(pass, pages)`
    the compile is preempted there (a newer edit arrives): a compile that
    stopped is not verified. `pre`: the directory the reference starts
    from (after an interrupted compile: as the last complete one left it,
    the files the interrupted run was rewriting being its inputs as read)."""
    pre = pre if pre is not None else snapshot(work)
    with open(os.path.join(work, editfile), 'wb') as f:
        f.write(content)
    old_frames = frames()
    t0 = time.time()
    if interrupt:
        r = cmd(f'compile-interrupt {interrupt[0]} {interrupt[1]}')
    elif a.stop:
        r = cmd('compile 1000000')
    else:
        r = cmd('compile')
    wall = time.time() - t0
    if r.get('paused'):
        rec = dict(tag=tag, mode=r['mode'], interrupted=True, interrupt=interrupt, mismatch=None,
                   pass_modes=r.get('pass_modes'), pages=r['pages'], restart_pages=r['restart_pages'],
                   edited_page_s=None, edited_page_cpu=None, converged_at=None, passes=r.get('passes'))
        results.append(rec)
        if out:
            out.write(json.dumps(rec) + '\n')
            out.flush()
        log(f'{tag}: interrupted in pass {interrupt[0]} after {interrupt[1]} pages ({r.get("pass_modes")})')
        return r
    new_frames = frames()
    changed_pages = sum(1 for x, y in zip(old_frames, new_frames) if x != y) + abs(len(old_frames) - len(new_frames))
    first = next((i + 1 for i, (x, y) in enumerate(zip(old_frames, new_frames)) if x != y),
                 None if len(old_frames) == len(new_frames) else min(len(old_frames), len(new_frames)) + 1)
    ed = r.get('edited')
    if ed is not None:
        edited_page, edited_s, edited_cpu = ed
    else:
        # (an engine before P4-L5: the ship time of the first changed page)
        pt = dict((x[0], x[1]) for x in r['page_times'])
        pc = dict((x[0], x[2]) for x in r['page_times'])
        edited_page = first
        edited_s = pt.get(first) if first else None
        edited_cpu = pc.get(first) if first else None
    bad = compare(tag, pre, content)
    rec = dict(tag=tag, mode=r['mode'], restart_pages=r['restart_pages'], converged_at=r['converged_at'],
               rerun_pages=r['rerun_pages'], pages=r['pages'], first_changed=first, edited_page=edited_page,
               edited_page_s=edited_s, edited_page_cpu=edited_cpu,
               restore_s=r['restore_s'], find_s=r['find_s'], key_s=r.get('key_s'), changes_s=r.get('changes_s'), total_s=r['total_s'], wall=wall, tests=r['tests'],
               test_s=r['test_s'], log_bytes=r['log_bytes'], checkpoints=r['checkpoints'],
               diffs=r['diffs'][:3], mismatch=bad, cold_reason=r['cold_reason'],
               accounting_only=tag in ACCT, changed_pages=changed_pages,
               passes=r.get('passes', 1), pass_modes=r.get('pass_modes'), pass_s=r.get('pass_s'),
               oscillation=r.get('oscillation'), ref_runs=REFRUNS.get(tag),
               restart_mid_page=r.get('restart_mid_page'), restart_gap=r.get('restart_gap'),
               l5=r.get('l5'), rs_events=r.get('rs_events'), ck_stats=r.get('ck_stats'))
    results.append(rec)
    if out:
        out.write(json.dumps(rec) + '\n')
        out.flush()
    ms = lambda x: 'None' if x is None else f'{1000*x:.1f}'
    log(f'{tag}: {r["mode"]} restart@{r["restart_pages"]}{"m" if r.get("restart_mid_page") else ""}/gap{r.get("restart_gap")} conv@{r["converged_at"]} rerun={r["rerun_pages"]} '
        f'edited={edited_page}:{ms(edited_s)}ms cpu={ms(edited_cpu)} first={first} total={ms(r["total_s"])}ms restore={ms(r["restore_s"])} '
        f'find={ms(r["find_s"])}(key {ms(r.get("key_s"))}) tests={r["tests"]}/{ms(r["test_s"])} logs={r["log_bytes"]>>20}MB '
        f'passes={r.get("pass_modes")} ref_runs={REFRUNS.get(tag)} '
        f'{"OK" if bad == [] else ("MISMATCH " + str(bad)) if bad else ""}'
        + (f' diff={r["diffs"][:1]}' if r['diffs'] and r['converged_at'] is None else ''))
    return rec


def paragraph_starts(src):
    body = src.find(b'\\begin{document}')
    end = src.rfind(b'\\end{document}')
    return [m.start() + 1 for m in re.finditer(rb'\n\n(?=[A-Z])', src) if body < m.start() < end]


def word_end(src, p):
    while p < len(src) and chr(src[p]).isalpha():
        p += 1
    return p


def structural(kind, src, p, i):
    """A structural edit near position p (None if the document has nothing
    to apply it to)."""
    if kind == 'section':
        starts = paragraph_starts(src)
        if not starts:
            return None
        q = min(starts, key=lambda s: abs(s - p))
        return src[:q] + f'\\section{{Inserted {i}}}\\label{{sec:ins{i}}}\n\n'.encode() + src[q:]
    if kind == 'label':
        q = word_end(src, p)
        return src[:q] + f'\\label{{lab:new{i}}}'.encode() + src[q:]
    if kind == 'footnote':
        q = word_end(src, p)
        return src[:q] + f'\\footnote{{An inserted note {i}.}}'.encode() + src[q:]
    if kind == 'ref':
        labs = re.findall(rb'\\label\{([^}]*)\}', src)
        if not labs:
            return None
        lab = rng.choice(labs).decode()
        q = word_end(src, p)
        how = rng.choice(['ref', 'pageref'])
        return src[:q] + f' (see \\{how}{{{lab}}})'.encode() + src[q:]
    if kind == 'cite':
        keys = re.findall(rb'\\bibitem\{([^}]*)\}', src)
        if not keys:
            return None
        q = word_end(src, p)
        return src[:q] + f'~\\cite{{{rng.choice(keys).decode()}}}'.encode() + src[q:]
    if kind == 'unlabel':
        ms = [m for m in re.finditer(rb'\\label\{[^}]*\}', src)]
        if not ms:
            return None
        m = min(ms, key=lambda m: abs(m.start() - p))
        return src[:m.start()] + src[m.end():]
    if kind == 'unsection':
        ms = [m for m in re.finditer(rb'\\section\{[^}\n]*\}', src)]
        if not ms:
            return None
        m = min(ms, key=lambda m: abs(m.start() - p))
        return src[:m.start()] + src[m.end():]
    return None


anchor = None
for i in range(a.trials):
    if not cands:
        break
    if a.window and anchor is not None:
        near = [c for c in cands if abs(c - anchor) <= a.window]
        p = rng.choice(near)
    else:
        p = rng.choice(cands)
        anchor = p
    kind = rng.choice(kinds)
    if kind == 'replace':
        c = src[p]
        nc = rng.choice([x for x in b'abcdefghijklmnopqrstuvwxyz' if x != c])
        new = src[:p] + bytes([nc]) + src[p + 1:]
    elif kind == 'sentence':
        # a reflowing edit: a dozen words into the paragraph
        new = src[:p] + b' lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor' + src[p:]
    elif kind == 'insert':
        new = src[:p] + bytes([rng.choice(b'abcdefghijklmnopqrstuvwxyz')]) + src[p:]
    elif kind == 'delete':
        new = src[:p] + src[p + 1:]
    else:
        new = structural(kind, src, p, i)
        if new is None:
            continue
    if a.interleave:
        pre_c = snapshot(work)
        r1 = one(new, f'{i}:{kind}@{p}', interrupt=(rng.choice([1, 1, 2]), rng.randint(1, 4)))
        if r1.get('paused'):
            # the second edit: the revert, or one more letter near the first
            if rng.random() < 0.5:
                one(src, f'{i}:revert-after-interrupt', pre=pre_c)
            else:
                q = min(len(new) - 1, p + rng.randint(-40, 40))
                second = new[:q] + bytes([rng.choice(b'abcdefghijklmnopqrstuvwxyz')]) + new[q:]
                one(second, f'{i}:second-after-interrupt', pre=pre_c)
                one(src, f'{i}:revert')
        else:
            one(src, f'{i}:revert')
        continue
    one(new, f'{i}:{kind}@{p}')
    if a.no_revert:
        src = new
        cands = prose_positions(src)
    else:
        one(src, f'{i}:revert')

host.stdin.write('quit\n')
host.stdin.flush()
host.wait()
ok = sum(1 for r in results if r['mismatch'] == [])
bad = sum(1 for r in results if r['mismatch'])
interrupted = sum(1 for r in results if r.get('interrupted'))
print(json.dumps(dict(doc=a.doc, compiles=len(results), verified_ok=ok, mismatches=bad, interrupted=interrupted,
                      converged=sum(1 for r in results if r['converged_at']),
                      multipass=sum(1 for r in results if (r.get('passes') or 1) > 1),
                      ref_multirun=sum(1 for r in results if (r.get('ref_runs') or 1) > 1),
                      accounting_only=sum(1 for r in results if r.get('accounting_only')),
                      modes={m: sum(1 for r in results if r['mode'] == m) for m in set(r['mode'] for r in results)})))
if not a.keep:
    shutil.rmtree(work)
