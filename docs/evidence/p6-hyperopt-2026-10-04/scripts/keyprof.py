#!/usr/bin/env python3
"""P6-HYPEROPT keystroke profile: one `flashtex-host --socket` session per document, dl3-keys phases (t7.py's
Host and keys), and the host's DONE stages per keystroke, including the engine thread's instruction counts
(`instr_k`, `restore_instr_k`, `edited_instr_k`, `test_instr_k`; os::thread_counts, macOS).

usage: keyprof.py ENGINE DOC OUT PHASE... [--keys N] [--gap-ms MS] [--host-arg A]...
PHASE is NAME=dl3-keys args separated by commas, e.g. 'mid=--kind,letter,--line,198,--page,62'.
DOC is $INCR_BENCH_DIR/docs/DOC (a directory with the main file)."""
import argparse, json, os, shutil, statistics, sys, time

W = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', '..')), 'tools', 'incr-bench'))
import t7  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument('engine')
ap.add_argument('doc')
ap.add_argument('out')
ap.add_argument('phases', nargs='+')
ap.add_argument('--keys', type=int, default=12)
ap.add_argument('--gap-ms', type=int, default=300)
ap.add_argument('--host-arg', action='append', default=[])
ap.add_argument('--timeout', type=int, default=7200)
a = ap.parse_args()

IB = t7.IB
eng = f'{IB}/{a.engine}'
os.makedirs(a.out, exist_ok=True)
work = f'{IB}/kp-work/{a.engine}-{a.doc}'
s0 = f'{IB}/kp-work/s0-{a.engine}-{a.doc}'
sock = f'/tmp/p6h-{a.engine}-{a.doc}.sock'[:100]
shutil.rmtree(work, ignore_errors=True)
shutil.rmtree(s0, ignore_errors=True)
shutil.copytree(f'{IB}/docs/{a.doc}', work, symlinks=True)
os.makedirs(f'{work}/out', exist_ok=True)
h = t7.Host(eng, sock, s0, f'{a.out}/{a.engine}-{a.doc}.host-stderr', a.host_arg)
STAGES = ['restore', 'first_page', 'first_page_dl', 'edited_wall', 'test', 'cpu', 'instr_k', 'cycles_k',
          'restore_instr_k', 'edited_instr_k', 'test_instr_k', 'first_page_instr_k']
res = {'engine': a.engine, 'doc': a.doc, 'phases': {}}
try:
    for ph in a.phases:
        name, args = ph.split('=', 1)
        args = args.split(',')
        l0 = t7.load1()
        t = time.time()
        recs, where = t7.keys(eng, sock, work, args + ['--keys', str(a.keys), '--gap-ms', str(a.gap_ms)],
                              f'{a.out}/{a.engine}-{a.doc}-{name}.jsonl', a.timeout)
        ks = [r for r in recs if 'key' in r][1:]
        st = {}
        for s in STAGES:
            v = [k['host'].get('stages', {}).get(s) for k in ks]
            v = [x for x in v if isinstance(x, (int, float))]
            if v:
                st[s] = dict(p50=statistics.median(v), min=min(v), max=max(v), n=len(v))
        ed = [k['edited_page_ms'] for k in ks if k.get('edited_page_ms') is not None]
        dn = [k['done_ms'] for k in ks]
        res['phases'][name] = dict(where=where, n=len(ks), stages=st,
                                   edited_page_ms=dict(p50=statistics.median(ed), max=max(ed)) if ed else None,
                                   done_ms=dict(p50=statistics.median(dn), max=max(dn)) if dn else None,
                                   converged=sum(k['host'].get('converged_at') is not None for k in ks),
                                   typeset_pages=sorted(k['host'].get('typeset_pages') or 0 for k in ks),
                                   load=[l0, t7.load1()], secs=time.time() - t)
        print(name, json.dumps(res['phases'][name]), flush=True)
finally:
    res['rss_peak'] = h.stop()
json.dump(res, open(f'{a.out}/summary-{a.engine}-{a.doc}.json', 'w'), indent=1)
