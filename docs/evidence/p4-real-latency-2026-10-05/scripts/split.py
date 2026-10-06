#!/usr/bin/env python3
"""split.py FILE.jsonl...: one key's latency split into stages (p50 / p95), wall ms and engine
instructions (M), for dl3-keys output (iso: one line per key with `host`; interval: `key`/`by`
lines and `done` lines). Key 0 of a file is dropped (warm-up)."""
import json, sys


def pct(v, p):
    v = sorted(x for x in v if x is not None)
    return v[min(len(v) - 1, round((len(v) - 1) * p))] if v else None


def f(x, nd=1):
    return '-' if x is None else f'{x:.{nd}f}'


def rows_of(path):
    recs = [json.loads(l) for l in open(path) if l.startswith('{')]
    dones = {r['done']['id']: r['done'] for r in recs if 'done' in r}
    keyl = [r for r in recs if 'key' in r]
    summ = [r for r in recs if 'interval_ms' in r]
    out = []
    if summ:
        iv = summ[-1]['interval_ms']
        first = min(dones)  # key k's compile is first + k
        for r in keyl:
            if r['key'] == 0:
                continue
            by = dones.get(r['by'], {})
            out.append((r, by, (r['by'] - (first + r['key'])) * iv))
    else:
        for r in keyl:
            if r['key'] == 0:
                continue
            out.append((r, r.get('host', {}), 0.0))
    return out


def stage(rows):
    S = {}
    def add(k, v):
        S.setdefault(k, []).append(v)
    for r, d, later in rows:
        st = d.get('stages', {})
        lat = r.get('edited_page_ms')
        add('client key -> edited PAGE', lat)
        add('  waiting for a later key\'s compile (own one superseded)', later)
        q = st.get('queue')
        add('  queue (engine thread busy)', q)
        qb = st.get('queue_by', {}) or {}
        for k in ('typeset', 'test', 'jump', 'restore', 'paused', 'prepare', 'done', 'other', 'request', 'idle'):
            add(f'    queue_by {k}', qb.get(k, 0.0))
        fp = d.get('first_page_ms')
        add('  host: arrival -> first PAGE written', fp)
        add('    apply', st.get('apply'))
        add('    move_spans', st.get('move_spans'))
        add('    find (S0 key + changes)', st.get('find'))
        add('    restore', st.get('restore'))
        dl, snd = st.get('first_page_dl'), st.get('first_page_send')
        parts = [q, st.get('apply'), st.get('move_spans'), st.get('find'), st.get('restore'), dl, snd]
        add('    typeset to the page (rest)', fp - sum(x or 0 for x in parts) if fp is not None else None)
        add('    display list (to the first page)', dl)
        add('    send (to the first page)', snd)
        add('    engine CPU (compile start -> first page)', st.get('first_page_cpu'))
        add('    off-CPU (wall - queue - CPU)', fp - (q or 0) - st['first_page_cpu'] if fp is not None and st.get('first_page_cpu') is not None else None)
        add('  socket + client (client - later - host)', lat - later - fp if fp is not None and lat is not None else None)
        m = lambda x: None if x is None else x / 1000.0
        add('instr: compile start -> first page (M)', m(st.get('first_page_instr_k')))
        add('instr: restore (M)', m(st.get('restore_instr_k')))
        add('instr: restore start -> edited page (M)', m(st.get('edited_instr_k')))
        add('instr: convergence tests (M)', m(st.get('test_instr_k')))
        a, b = st.get('arrival_mark_kc'), st.get('first_page_mark_kc')
        add('cycles: arrival -> first page (M)', None if a is None or b is None else (b - a) / 1000.0)
    return S


def main():
    for p in sys.argv[1:]:
        rows = rows_of(p)
        S = stage(rows)
        print(f'== {p}  (n {len(rows)})')
        for k, v in S.items():
            v2 = [x for x in v if x is not None]
            if not v2:
                continue
            print(f'{k:62s} p50 {f(pct(v2, .5)):>8s}  p95 {f(pct(v2, .95)):>8s}  n {len(v2)}')


main()
