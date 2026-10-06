#!/usr/bin/env python3
"""sweeps.py: the soundness sweeps of gates.sh, cut into shards for GitHub-hosted runners
(.github/workflows/sweeps.yml, lane CI-SWEEPS-HOSTED).

A sweep is a list of runs of soundness.py (or dlspan.py, readers.py) over documents. Each run of
soundness.py over one document is independent of the others: soundness.py seeds each document's
edits from its name alone (crc32), so `--only DOC` gives that document exactly the edits the whole
sweep gives it. A unit is one (run, document) pair; a shard is a set of units, run J at a time.
Shards are packed from measured unit costs (sweeps-costs.json, `costs` below) so that each takes
about --target minutes.

The run definitions (GATES) mirror gates.sh's sweeps; change both together.

  sweeps.py gates                              the gate names, in gates.sh's order
  sweeps.py plan  --tree T --gates G,.. [--side S --sha X] [--target MIN] [-j J]
                                               the shards, as JSON (the workflow's matrix)
  sweeps.py run   --tree T --gate G --shard K/N --out DIR [-j J]
                                               one shard: INCR_BENCH_DIR must hold the engine
                                               `gates` and fmt-gates (gates.sh's build step);
                                               writes DIR/result.json and the raw records
  sweeps.py summary DIR... [--plan P] [--md F] [--json F]
                                               one table of every result.json under DIR; exit 1
                                               if a gate of side `ref` has a bad, wrong or failed
                                               unit, or a shard of the plan is missing
  sweeps.py costs DIR...                       measured unit seconds, as sweeps-costs.json

T is the checkout under test: its soundness.py, incr_bench.py, edits.py, dlspan.py, readers.py,
generators and fixtures are the ones run. The engine is the one built from it.
"""
import argparse
import concurrent.futures
import glob
import json
import math
import os
import resource
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
COSTS = os.path.join(HERE, 'sweeps-costs.json')
ENGINE = 'gates'  # gates.sh's engine name under INCR_BENCH_DIR

KD = 'replace,insert,sentence,section,label,ref,unlabel'  # sweep D's interleaved kinds
BUDGET = '--budget 4194304'
TIMED = '--timed 0.0002'
LINES = 'newline,split,join'
BIG = ('plain-120', 'full-100')
REFS = ('refs-30', 'refs-120', 'full-100')
VOL = ('vol-closed', 'vol-open')


def S(tag, trials, extras=(), fixtures=True, kinds=None, interleave=False, host=None, toggle=False,
      allow_no_trials=False):
    """One soundness.py run (gates.sh: `soundness.py gates ...`)."""
    return dict(tool='soundness', tag=tag, trials=trials, extras=list(extras), fixtures=fixtures,
                kinds=kinds, interleave=interleave, host=host, toggle=toggle,
                allow_no_trials=allow_no_trials)


def SPAN(doc, edits, seed, frac=None, kinds=None, eol=None):
    """One dlspan.py run (gates.sh's run_span)."""
    tag = f'{doc}-s{seed}' + (f'-{eol}' if eol else '')
    return dict(tool='span', tag=tag, doc=doc, edits=edits, seed=seed, frac=frac, kinds=kinds, eol=eol)


GATES = {
    'sound-a': [S('a', 50, BIG)],
    'sound-budget': [S('budget', 20, BIG, host=BUDGET)],
    'sound-budget-d': [S('budget-d', 8, REFS, kinds=KD, interleave=True, host=BUDGET)],
    'sound-timed': [S('timed', 10, host=TIMED)],
    'sound-vol': [S('vol-timed', 20, VOL, fixtures=False, host=TIMED),
                  S('vol', 20, VOL, fixtures=False),
                  S('vol-d', 12, VOL, fixtures=False, kinds=KD, interleave=True)],
    'sound-lookup': [S('lookup', 30, ('lookup',), fixtures=False, kinds='replace,insert,delete,sentence', toggle=True),
                     S('lookup-d', 12, ('lookup',), fixtures=False, kinds='replace,insert,delete,sentence',
                       interleave=True, toggle=True)],
    'sound-lines': [S('lines', 20, BIG, kinds=LINES, allow_no_trials=True),
                    S('lines-timed', 10, kinds=LINES, host=TIMED, allow_no_trials=True),
                    S('lines-d', 8, BIG, kinds=LINES + ',replace,sentence', interleave=True, allow_no_trials=True)],
    'span': [SPAN('plain-10', 15, 1), SPAN('plain-10', 15, 2), SPAN('plain-10', 15, 3),
             SPAN('plain-120', 12, 1, 0.85), SPAN('plain-120', 12, 2, 0.3),
             SPAN('full-10', 15, 1), SPAN('full-10', 15, 2), SPAN('full-10', 15, 3),
             SPAN('full-100', 10, 1, 0.85),
             SPAN('plain-120', 12, 4, 0.3, 'letter,newline,split'),
             SPAN('full-100', 10, 4, 0.3, 'letter,newline,split'),
             SPAN('plain-120', 12, 5, 0.3, 'letter,newline,split', 'cr'),
             SPAN('full-100', 10, 5, 0.3, 'letter,newline,split,wedge', 'mixed')],
    # readers.py (#1613): run when the tree under test has it
    'readers': [dict(tool='readers', tag='readers')],
    'sound-c': [S('c', 20, REFS, kinds='sentence,section,label,ref,cite,footnote,unlabel,unsection')],
    'sound-d': [S('d', 12, REFS, kinds=KD, interleave=True)],
}
# gates.sh's span check: every one of its 13 dlspan runs printed a summary
SPAN_RUNS = len(GATES['span'])

# Seconds per unit before any measurement (sweeps-costs.json overrides): per trial (an edit and its
# revert, each verified), by document size; interleaved trials compile twice.
PER_TRIAL = {'plain-120': 30, 'full-100': 30, 'refs-120': 30, 'refs-30': 8, 'lookup': 6,
             'vol-closed': 4, 'vol-open': 4}
SPAN_PER_EDIT = {'plain-10': 3, 'full-10': 4, 'plain-120': 25, 'full-100': 30}


def fixtures(tree):
    """soundness.py's fixtures(): the documents a sweep without --no-fixtures runs."""
    out = []
    for tier in ('fixtures/real-world', 'fixtures/divergence-probes'):
        base = os.path.join(tree, tier)
        if not os.path.isdir(base):
            continue
        for name in sorted(os.listdir(base)):
            d = os.path.join(base, name)
            if not os.path.isdir(d):
                continue
            texs = [f for f in os.listdir(d) if f.endswith('.tex')]
            if 'main.tex' in texs or len(texs) == 1:
                out.append(name)
    return out


def units(tree, gate):
    """The units of GATE: dicts with a stable key."""
    out = []
    for run in GATES[gate]:
        if run['tool'] == 'soundness':
            docs = list(run['extras']) + (fixtures(tree) if run['fixtures'] else [])
            for doc in docs:
                out.append(dict(run, doc=doc, extra=doc in run['extras'], key=f"{gate}|{run['tag']}|{doc}"))
        elif run['tool'] == 'span':
            out.append(dict(run, key=f"{gate}|{run['tag']}|{run['doc']}"))
        else:
            out.append(dict(run, doc='-', key=f"{gate}|{run['tag']}|-"))
    return out


def load_costs():
    try:
        return json.load(open(COSTS))['seconds']
    except (OSError, ValueError, KeyError):
        return {}


def estimate(u, costs):
    if u['key'] in costs:
        return float(costs[u['key']])
    if u['tool'] == 'span':
        return 30 + u['edits'] * SPAN_PER_EDIT.get(u['doc'], 20)
    if u['tool'] == 'readers':
        return 1200
    t = PER_TRIAL.get(u['doc'], 3) * u['trials'] * (2 if u['interleave'] else 1)
    return 20 + t


def shards(tree, gate, target_s, j, costs=None):
    """GATE's units packed into shards (longest first onto the least loaded shard), each about
    TARGET_S seconds of J parallel slots. Deterministic: plan and run compute the same."""
    costs = load_costs() if costs is None else costs
    us = sorted(units(tree, gate), key=lambda u: (-estimate(u, costs), u['key']))
    if not us:
        return []
    total = sum(estimate(u, costs) for u in us)
    n = max(1, math.ceil(total / (target_s * j)))
    n = min(n, len(us))
    load = [0.0] * n
    out = [[] for _ in range(n)]
    for u in us:
        k = min(range(n), key=lambda i: (load[i], i))
        out[k].append(u)
        load[k] += estimate(u, costs)
    return [(s, l) for s, l in zip(out, load)]


# --------------------------------------------------------------------------- run

def env():
    e = dict(os.environ, PYTHONHASHSEED='0', FLASHTEX_VERIFY_JUMP='1', FLASHTEX_VERIFY_OLDCACHE='1',
             FLASHTEX_VERIFY_PREPARED='1', FLASHTEX_VERIFY_RELOC='1')
    return e


def last_json(text, pred=lambda r: True):
    for line in reversed(text.splitlines()):
        if line.startswith('{'):
            try:
                r = json.loads(line)
            except ValueError:
                continue
            if pred(r):
                return r
    return None


def prepare(tree, bench, gate):
    """The generated documents GATE reads (gates.sh's build step and the vol/lookup cases)."""
    ib = os.path.join(tree, 'tools/incr-bench')
    e = dict(env(), INCR_BENCH_DIR=bench)
    subprocess.run([sys.executable, f'{ib}/mkdocs.py'], env=e, check=True, stdout=subprocess.DEVNULL)
    if gate == 'sound-vol':
        subprocess.run([sys.executable, f'{ib}/genvol.py', bench], env=e, check=True, stdout=subprocess.DEVNULL)
    if gate == 'sound-lookup':
        subprocess.run([sys.executable, f'{ib}/genlookup.py', bench], env=e, check=True, stdout=subprocess.DEVNULL)


def run_unit(tree, bench, out, u, timeout):
    ib = os.path.join(tree, 'tools/incr-bench')
    gate, tag, doc = u['key'].split('|')
    stem = f"{out}/raw/{tag}.{doc}"
    os.makedirs(os.path.dirname(stem), exist_ok=True)
    r = dict(key=u['key'], tool=u['tool'], doc=doc)
    if u['tool'] == 'soundness':
        cmd = [sys.executable, f'{ib}/soundness.py', ENGINE, '-j', '1', '--trials', str(u['trials']), '--only', doc,
               '--dir', f'{bench}/sound-{tag}', '--out', f'{stem}.jsonl']
        if u['extra']:
            cmd += ['--no-fixtures', '--extra', f'{bench}/src-{doc}:{doc}']
        if u['kinds']:
            cmd += ['--kinds', u['kinds']]
        if u['interleave']:
            cmd += ['--interleave']
        if u['host']:
            cmd += ['--host-args=' + u['host']]
        if u['toggle']:
            names = subprocess.run([sys.executable, f'{ib}/genlookup.py', '--names'], stdout=subprocess.PIPE,
                                   text=True, check=True).stdout.strip()
            cmd += ['--toggle-files', names]
        if u['allow_no_trials']:
            cmd += ['--allow-no-trials']
    elif u['tool'] == 'span':
        cmd = [sys.executable, f'{ib}/dlspan.py', ENGINE, u['doc'], '--edits', str(u['edits']), '--seed', str(u['seed']),
               '--timeout', '7000']
        if u['frac'] is not None:
            cmd += ['--from', str(u['frac'])]
        if u['kinds']:
            cmd += ['--kinds', u['kinds']]
        if u['eol']:
            cmd += ['--eol', u['eol']]
    else:
        if not os.path.exists(f'{ib}/readers.py'):
            r.update(status='absent', seconds=0.0, exit=0)
            return r
        cmd = [sys.executable, f'{ib}/readers.py', f'{bench}/{ENGINE}', f'{out}/raw/readers']
    r['cmd'] = ' '.join(cmd)
    t0 = time.monotonic()
    try:
        p = subprocess.run(cmd, cwd=tree, env=dict(env(), INCR_BENCH_DIR=bench), stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, text=True, errors='replace', timeout=timeout)
        code, so, se = p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as e:
        so = e.stdout.decode('latin-1') if isinstance(e.stdout, bytes) else (e.stdout or '')
        code, se = 'timeout', f'timed out after {timeout} s'
    r['seconds'] = round(time.monotonic() - t0, 1)
    r['exit'] = code
    open(f'{stem}.out', 'w').write(so + ('\n--- stderr\n' + se if se else ''))
    if u['tool'] == 'soundness':
        tot = last_json(so, lambda x: 'compiles' in x and 'ok' in x)
        if tot is None:
            r.update(status='error', note=(se or so)[-600:])
            return r
        for k in ('compiles', 'ok', 'bad', 'conv', 'err', 'interrupted', 'skipped'):
            r[k] = tot.get(k, 0)
        if tot['err'] or tot['bad'] or code != 0:
            r['status'] = 'bad' if tot['bad'] else 'error'
            r['note'] = '\n'.join(l for l in so.splitlines() if 'ERROR' in l or 'mismatches' in l)[-600:]
        elif tot['compiles'] == 0 and not tot['skipped']:
            # no such document in the tree: soundness.py and fixtures() above disagree
            r.update(status='error', note=f'soundness.py ran no document named {doc}')
        else:
            r['status'] = 'ok'
    elif u['tool'] == 'span':
        s = last_json(so, lambda x: x.get('summary'))
        if s is None:
            r.update(status='error', note=(se or so)[-600:])
            return r
        r['edits'], r['glyphs'] = s.get('edits', 0), s.get('glyphs', 0)
        r['wrong'] = s.get('line_bad', 0) + s.get('col_bad', 0) + s.get('glyph_count_bad', 0)
        r['status'] = 'ok' if code == 0 and not r['wrong'] else ('bad' if r['wrong'] else 'error')
        if r['status'] != 'ok':
            r['note'] = (se or so)[-600:]
    else:
        r['status'] = {0: 'ok', 1: 'bad'}.get(code, 'error')
        r['note'] = so[-1500:] if r['status'] != 'ok' else so[-400:]
    return r


def cmd_run(a):
    k, n = map(int, a.shard.split('/'))
    plan = shards(a.tree, a.gate, a.target * 60, a.j)
    if n != len(plan):
        sys.exit(f'sweeps.py: the plan has {len(plan)} shards for {a.gate}, not {n} (a different tree or costs?)')
    us, est = plan[k]
    bench = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
    os.makedirs(a.out, exist_ok=True)
    prepare(a.tree, bench, a.gate)
    t0 = time.monotonic()
    results = []
    with concurrent.futures.ThreadPoolExecutor(a.j) as ex:
        futs = {ex.submit(run_unit, a.tree, bench, a.out, u, a.timeout): u for u in us}
        for f in concurrent.futures.as_completed(futs):
            u = futs[f]
            try:
                r = f.result()
            except Exception as e:  # the harness itself
                r = dict(key=u['key'], tool=u['tool'], status='error', note=repr(e), seconds=0)
            results.append(r)
            print(f"{r['status']:6} {r.get('seconds', 0):7.0f}s {r['key']}"
                  + (f"  {r.get('compiles', '')} compiles, {r.get('bad', 0)} bad" if r['tool'] == 'soundness' else '')
                  + (f"  {r.get('edits', '')} edits, {r.get('wrong', 0)} wrong" if r['tool'] == 'span' else ''),
                  flush=True)
            if r['status'] not in ('ok', 'absent') and r.get('note'):
                print('       ' + r['note'].replace('\n', '\n       '), flush=True)
    wall = time.monotonic() - t0
    ru = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpus = os.cpu_count() or 1
    res = dict(side=a.side, sha=a.sha, gate=a.gate, shard=k, of=n, jobs=a.j, estimate=round(est),
               seconds=round(wall, 1), cpu_seconds=round(ru.ru_utime + ru.ru_stime), cpus=cpus,
               max_rss_mb=round(ru.ru_maxrss / 1024), units=sorted(results, key=lambda r: r['key']))
    json.dump(res, open(os.path.join(a.out, 'result.json'), 'w'), indent=1)
    bad = [r for r in results if r['status'] not in ('ok', 'absent')]
    print(f"{a.gate} {k + 1}/{n}: {len(results)} units in {wall:.0f}s (estimated {est:.0f}s), {len(bad)} not ok; "
          f"CPU {res['cpu_seconds']}s = {res['cpu_seconds'] / max(wall, 1) / cpus:.0%} of {cpus} cpus; "
          f"largest child {res['max_rss_mb']} MB")
    sys.exit(1 if bad else 0)


def cmd_plan(a):
    out = []
    for g in a.gates.split(','):
        g = g.strip()
        if not g:
            continue
        if g not in GATES:
            sys.exit(f'sweeps.py: no gate {g!r} (have: {", ".join(GATES)})')
        p = shards(a.tree, g, a.target * 60, a.j)
        for k, (us, est) in enumerate(p):
            out.append(dict(side=a.side, sha=a.sha, gate=g, shard=f'{k}/{len(p)}',
                            name=f'{a.side} {g} {k + 1}/{len(p)}', id=f'{a.side}-{g}-{k}', units=len(us),
                            estimate_min=round(est / a.j / 60, 1)))
    print(json.dumps(out))


# --------------------------------------------------------------------------- summary

def collect(dirs):
    rs = []
    for d in dirs:
        for f in sorted(glob.glob(os.path.join(d, '**', 'result.json'), recursive=True)):
            rs.append(json.load(open(f)))
    return rs


def cmd_summary(a):
    rs = collect(a.dirs)
    plan = json.load(open(a.plan)) if a.plan else []
    order = list(GATES)
    rows, failed = [], False
    sides = sorted({r['side'] for r in rs} | {p['side'] for p in plan}, key=lambda s: (s != 'ref', s))
    for side in sides:
        gates = sorted({r['gate'] for r in rs if r['side'] == side} | {p['gate'] for p in plan if p['side'] == side},
                       key=lambda g: order.index(g) if g in order else 99)
        for g in gates:
            got = [r for r in rs if r['side'] == side and r['gate'] == g]
            want = [p for p in plan if p['side'] == side and p['gate'] == g]
            us = [u for r in got for u in r['units']]
            row = dict(side=side, sha=(got or want or [{}])[0].get('sha', ''), gate=g, shards=len(got),
                       planned=len(want) or (got[0]['of'] if got else 0), units=len(us),
                       compiles=sum(u.get('compiles', 0) for u in us), ok=sum(u.get('ok', 0) for u in us),
                       bad=sum(u.get('bad', 0) for u in us), wrong=sum(u.get('wrong', 0) for u in us),
                       edits=sum(u.get('edits', 0) for u in us),
                       aborts=sum(1 for u in us if u['status'] == 'error'),
                       skipped=sum(u.get('skipped', 0) for u in us),
                       absent=all(u['status'] == 'absent' for u in us) and bool(us),
                       slowest_shard_min=round(max([r['seconds'] for r in got] or [0]) / 60, 1),
                       slowest_unit=max(us, key=lambda u: u.get('seconds', 0))['key'] if us else '')
            missing = row['planned'] - row['shards']
            bad_units = [u for u in us if u['status'] not in ('ok', 'absent')]
            if g == 'span' and row['units'] and row['units'] < SPAN_RUNS:
                missing = max(missing, 1)
            row['missing'] = missing
            row['status'] = 'n/a' if row['absent'] else ('FAIL' if bad_units or missing or not us else 'pass')
            row['failures'] = [dict(key=u['key'], status=u['status'], note=u.get('note', '')[-300:]) for u in bad_units][:20]
            if side == 'ref' and row['status'] == 'FAIL':
                failed = True
            rows.append(row)
    md = ['| side | gate | status | shards | units | compiles | ok | bad | wrong (span) | aborts | skipped | slowest shard (min) |',
          '|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|']
    for r in rows:
        md.append(f"| {r['side']} `{r['sha'][:9]}` | {r['gate']} | **{r['status']}** | {r['shards']}/{r['planned']} | "
                  f"{r['units']} | {r['compiles']} | {r['ok']} | {r['bad']} | {r['wrong'] if r['gate'] == 'span' else ''} | "
                  f"{r['aborts']} | {r['skipped']} | {r['slowest_shard_min']} |")
    fails = [(r, f) for r in rows for f in r['failures']]
    if fails:
        md += ['', '**Not ok:**', '']
        for r, f in fails:
            md.append(f"- {r['side']} `{f['key']}`: {f['status']}" + (f" -- `{f['note'][-200:].strip().replace(chr(10), ' / ')}`" if f['note'] else ''))
    if any(r['missing'] for r in rows):
        md += ['', 'Missing shards: ' + ', '.join(f"{r['side']} {r['gate']} ({r['missing']})" for r in rows if r['missing'])]
    text = '\n'.join(md) + '\n'
    print(text)
    if a.md:
        open(a.md, 'w').write(text)
    if a.json:
        json.dump(dict(failed=failed, rows=rows), open(a.json, 'w'), indent=1)
    sys.exit(1 if failed else 0)


def cmd_costs(a):
    """Measured seconds per unit, the larger where two results measured one (both sides count)."""
    measured = {}
    for r in collect(a.dirs):
        for u in r['units']:
            if u['status'] != 'absent':
                measured[u['key']] = max(round(u.get('seconds', 0)), measured.get(u['key'], 0))
    out = dict(load_costs(), **measured) if a.merge else measured
    json.dump(dict(note='seconds per unit on a GitHub-hosted ubuntu-latest runner, %d units at a time '
                        '(sweeps.py costs)' % a.j, seconds=dict(sorted(out.items()))), sys.stdout, indent=0)
    print()


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest='cmd', required=True)
    sub.add_parser('gates')
    for name in ('plan', 'run'):
        p = sub.add_parser(name)
        p.add_argument('--tree', required=True)
        p.add_argument('--target', type=float, default=20, help='minutes per shard (default 20)')
        p.add_argument('-j', type=int, default=4, help='units at a time per shard (default 4)')
        p.add_argument('--side', default='ref')
        p.add_argument('--sha', default='')
    sub.choices['plan'].add_argument('--gates', default=','.join(GATES))
    r = sub.choices['run']
    r.add_argument('--gate', required=True)
    r.add_argument('--shard', required=True, help='K/N, 0 <= K < N')
    r.add_argument('--out', required=True)
    r.add_argument('--timeout', type=int, default=6000, help='seconds per unit')
    s = sub.add_parser('summary')
    s.add_argument('dirs', nargs='+')
    s.add_argument('--plan')
    s.add_argument('--md')
    s.add_argument('--json')
    c = sub.add_parser('costs')
    c.add_argument('dirs', nargs='+')
    c.add_argument('-j', type=int, default=4)
    c.add_argument('--merge', action='store_true', help='keep the units sweeps-costs.json has and DIRs do not')
    a = ap.parse_args()
    if a.cmd == 'gates':
        print(' '.join(GATES))
    elif a.cmd == 'plan':
        cmd_plan(a)
    elif a.cmd == 'run':
        cmd_run(a)
    elif a.cmd == 'summary':
        cmd_summary(a)
    else:
        cmd_costs(a)


if __name__ == '__main__':
    main()
