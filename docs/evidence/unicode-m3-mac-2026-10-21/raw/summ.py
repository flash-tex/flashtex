import glob, json, os, statistics, sys
d = sys.argv[1]
print('| bench | edit at | cold first / DONE ms | keystrokes (status, start) | edited page ms p50 / p95 / max | DONE ms p50 / p95 | reopen in a new host: first page / DONE ms (start, status) |')
print('|---|---|---:|---|---:|---:|---:|')
def p95(xs):
    xs = sorted(xs); return xs[min(len(xs)-1, int(round(0.95*len(xs)))-1)]
for f in sorted(glob.glob(os.path.join(d, '*.jsonl'))):
    rows = [json.loads(l) for l in open(f)]
    by = {r['tag']: r for r in rows}
    keys = [r for r in rows if r['tag'].startswith('key')]
    e = [r['edited_ms'] for r in keys]; dn = [r['done_ms'] for r in keys]
    st = sorted({r['status'] for r in keys}); sts = sorted({(r.get('starts') or ['?'])[0] for r in keys})
    name, at = os.path.basename(f)[:-6].rsplit('-', 1)
    c, ro = by['cold'], by.get('reopen', {})
    print(f"| {name} ({c['pages']} pp, {c['status']}) | {at} | {c['first_ms']} / {c['done_ms']} | {len(keys)} {','.join(st)} {','.join(sts)} | "
          f"{statistics.median(e):.1f} / {p95(e):.1f} / {max(e):.1f} | {statistics.median(dn):.1f} / {p95(dn):.1f} | "
          f"{ro.get('first_ms')} / {ro.get('done_ms')} ({','.join(ro.get('starts') or [])}, {ro.get('status')}) |")
