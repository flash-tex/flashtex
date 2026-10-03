import gzip, json, sys, os
# split.py DIR: per row, p50 of the edited page's parts (ms)
def pct(v, p):
    v = sorted(v)
    return v[round((len(v) - 1) * p)] if v else float('nan')
f = os.path.join(sys.argv[1], 'summary.json')
s = json.load(gzip.open(f + '.gz')) if not os.path.exists(f) else json.load(open(f))
print('| doc | edit | edited page (client) | request: queue+apply+find (of which S0 key, changes) | restore | engine to the edited shipout, display lists excluded | display list, first page shipped | pages shipped up to the edited one | socket and client |')
print('|---|---|---|---|---|---|---|---|---|')
for doc in s['raw']:
    for ph, p in doc['phases'].items():
        ks = [k for k in p.get('keys', []) if k.get('edited_page_ms') is not None][1:]
        if not ks:
            continue
        rows = []
        for k in ks:
            st = k['host']['stages']
            pre = st['queue'] + st['apply'] + st['move_spans'] + st['find']
            npages = k['edited_page'] - k['first_index'] + 1
            # display-list time up to the edited page: the first page's, plus the later pages' share
            dl_first = st['first_page_dl']
            eng = st['edited_wall'] - pre - st['restore'] - dl_first
            rows.append(dict(client=k['edited_page_ms'], pre=pre, key=st['key'], changes=st['changes'],
                             restore=st['restore'], eng=eng, dl=dl_first, n=npages,
                             sock=k['edited_page_ms'] - st['edited_wall']))
        m = {x: pct([r[x] for r in rows], .5) for x in rows[0]}
        print(f"| {doc['doc']} | {ph} | {m['client']:.1f} | {m['pre']:.1f} ({m['key']:.1f}, {m['changes']:.1f}) | {m['restore']:.1f} | {m['eng']:.1f} | {m['dl']:.1f} | {m['n']:.0f} | {m['sock']:.2f} |")
