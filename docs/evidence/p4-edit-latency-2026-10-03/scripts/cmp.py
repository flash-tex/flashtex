import gzip, json, sys, os
# cmp.py DIR...: per (doc, phase): p50 / p95 of the edited page (client side) and the p50 of the
# host's restore stage, one column per t7.py output directory (summary.json or summary.json.gz)
def pct(v, p):
    v = sorted(v)
    return v[round((len(v) - 1) * p)] if v else None
def load(d):
    f = os.path.join(d, 'summary.json')
    s = json.load(gzip.open(f + '.gz')) if not os.path.exists(f) else json.load(open(f))
    out = {}
    for doc in s['raw']:
        for ph, p in doc['phases'].items():
            ks = [k for k in p.get('keys', []) if k.get('edited_page_ms') is not None][1:]
            if not ks:
                continue
            st = lambda n: [k['host']['stages'][n] for k in ks if isinstance(k['host'].get('stages', {}).get(n), (int, float))]
            out[(doc['doc'], ph)] = dict(n=len(ks), p50=pct([k['edited_page_ms'] for k in ks], .5),
                                          p95=pct([k['edited_page_ms'] for k in ks], .95),
                                          restore=pct(st('restore'), .5))
    return out
runs = [(d, load(d)) for d in sys.argv[1:]]
keys = sorted({k for _, r in runs for k in r})
print('| doc | edit | ' + ' | '.join(os.path.basename(d) for d, _ in runs) + ' |')
print('|---|---|' + '---|' * len(runs))
for k in keys:
    cells = []
    for _, r in runs:
        x = r.get(k)
        cells.append('' if not x else f"{x['p50']:.1f} / {x['p95']:.1f} (restore {x['restore']:.1f})")
    print(f'| {k[0]} | {k[1]} | ' + ' | '.join(cells) + ' |')
