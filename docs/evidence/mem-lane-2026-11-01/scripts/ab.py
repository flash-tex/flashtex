import json, statistics, sys
mb = lambda v: round((v or 0) / 2**20, 1)
rows = {}
for l in open(sys.argv[1]):
    r = json.loads(l)
    rows.setdefault((r['name'], r['profile']), {})[r['engine']] = r
for (name, prof), e in rows.items():
    print(f'== {name} {prof}')
    for eng, r in e.items():
        m = r['again']['mem']
        tags = {k[5:]: mb(v) for k, v in m.items() if k.startswith('heap_') and not k.startswith('heap_peak')}
        ins = [x for x in r.get('instr', []) if x[0] is not None]
        ed = [x[0] / 1e3 for x in ins]
        ks = [x[1] / 1e3 for x in ins if x[1] is not None]
        print(f"  {eng:5} open {mb(r['open_proc'].get('VmRSS'))} steady {mb(r['steady_proc'].get('VmRSS'))} "
              f"peak {mb(r['peak_hwm'])} log {tags.get('log')} dl {tags.get('dl')} other {tags.get('other')} "
              f"page_cache {mb(m.get('page_cache'))} sealed {mb(m.get('sealed_bytes'))} | edited instr M p50 "
              f"{round(statistics.median(ed), 1) if ed else None} key instr M p50 {round(statistics.median(ks), 1) if ks else None} "
              f"n={len(ins)} edited ms p50/p95 {r['edited_p50']}/{r['edited_p95']}")
