#!/usr/bin/env python3
"""LIVE-30MS keystroke stages through the host's socket: one `flashtex-host --socket` per document, dl3-keys
phases (t7.py's Host and keys), every keystroke's line kept (raw JSONL), and p50/p95 of the client times and the
host's DONE stages (wall ms and the engine thread's instruction counts, os::thread_counts).

usage: keyrun.py ENGINE DOC OUT PHASE... [--keys N] [--gap-ms MS] [--host-arg A]...
PHASE is NAME=dl3-keys args separated by commas, e.g. 'iso=--kind,letter,--page,300' or
'typing=--overlap,--gap-ms,100' (a phase's own --gap-ms wins). DOC is $INCR_BENCH_DIR/docs/DOC
(its main file is main.tex)."""
import argparse, json, os, shutil, sys, time

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', '..', 'tools', 'incr-bench'))
import t7  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument('engine')
ap.add_argument('doc')
ap.add_argument('out')
ap.add_argument('phases', nargs='+')
ap.add_argument('--keys', type=int, default=20)
ap.add_argument('--gap-ms', type=int, default=300)
ap.add_argument('--host-arg', action='append', default=[])
ap.add_argument('--timeout', type=int, default=7200)
a = ap.parse_args()


def pct(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, round((len(v) - 1) * p))] if v else None


IB = t7.IB
eng = f'{IB}/{a.engine}'
os.makedirs(a.out, exist_ok=True)
work = f'{IB}/kr-work/{a.engine}-{a.doc}'
s0 = f'{IB}/kr-work/s0-{a.engine}-{a.doc}'
sock = f'/tmp/l30-{a.engine}-{a.doc}.sock'[:100]
shutil.rmtree(work, ignore_errors=True)
shutil.rmtree(s0, ignore_errors=True)
shutil.copytree(f'{IB}/docs/{a.doc}', work, symlinks=True)
os.makedirs(f'{work}/out', exist_ok=True)
h = t7.Host(eng, sock, s0, f'{a.out}/{a.engine}-{a.doc}.host-stderr', a.host_arg)
STAGES = ['queue', 'apply', 'find', 'key', 'changes', 'restore', 'first_page', 'first_page_dl', 'edited_wall',
          'edited_cpu', 'test', 'tests', 'dl', 'send', 'cpu', 'instr_k', 'cycles_k', 'restore_instr_k',
          'edited_instr_k', 'test_instr_k', 'first_page_instr_k']
res = {'engine': a.engine, 'doc': a.doc, 'phases': {}, 'power': t7.power_state()}
try:
    for ph in a.phases:
        name, args = ph.split('=', 1)
        args = [x for x in args.split(',') if x]
        if '--gap-ms' not in args:
            args += ['--gap-ms', str(a.gap_ms)]
        l0 = t7.load1()
        t = time.time()
        recs, where = t7.keys(eng, sock, work, args + ['--keys', str(a.keys)],
                              f'{a.out}/{a.engine}-{a.doc}-{name}.jsonl', a.timeout)
        ks = [r for r in recs if 'key' in r][1:]
        # (overlap: the DONEs come as lines of their own)
        dones = [r['done'] for r in recs if 'done' in r]
        hosts = [k['host'] for k in ks if 'host' in k] or [d for d in dones if d.get('status') != 'cancelled'][1:]
        st = {}
        for s in STAGES:
            v = [x.get('stages', {}).get(s) for x in hosts]
            v = [x for x in v if isinstance(x, (int, float))]
            if v:
                st[s] = dict(p50=pct(v, .5), p95=pct(v, .95), max=max(v), n=len(v))
        ed = [k['edited_page_ms'] for k in ks if k.get('edited_page_ms') is not None]
        dn = [k['done_ms'] for k in ks if 'done_ms' in k]
        res['phases'][name] = dict(args=args, where=where, n=len(ks), stages=st,
                                   edited_page_ms=dict(p50=pct(ed, .5), p95=pct(ed, .95), max=max(ed)) if ed else None,
                                   done_ms=dict(p50=pct(dn, .5), p95=pct(dn, .95), max=max(dn)) if dn else None,
                                   converged=sum(x.get('converged_at') is not None for x in hosts),
                                   cancelled=sum(d.get('status') == 'cancelled' for d in dones),
                                   typeset_pages=sorted(x.get('typeset_pages') or 0 for x in hosts),
                                   load=[l0, t7.load1()], secs=time.time() - t)
        print(name, json.dumps(res['phases'][name]), flush=True)
finally:
    res['rss_peak'] = h.stop()
    res['power_end'] = t7.power_state()
json.dump(res, open(f'{a.out}/summary-{a.engine}-{a.doc}.json', 'w'), indent=1)
