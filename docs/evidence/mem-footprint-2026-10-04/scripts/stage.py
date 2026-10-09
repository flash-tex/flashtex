import json, statistics, sys
for doc in ('full-100', 'long-deck', 'infdesc'):
    for t in sys.argv[1:]:
        r_ins, r_ms, e_ms = [], [], []
        for l in open(f'/tmp/mfp/out/{t}-{doc}.jsonl'):
            r = json.loads(l)
            if not r.get('phase', '').startswith('edit') or 'key' not in r:
                continue
            s = r['host'].get('stages', {})
            r_ins.append(s.get('restore_instr_k', 0) / 1e3)
            r_ms.append(s.get('restore', 0))
        print(f'{doc:10} {t:8} restore median {statistics.median(r_ins):.1f} M instr, {statistics.median(r_ms):.2f} ms; max {max(r_ins):.1f} M')
