import glob, json, sys
for d in sys.argv[1:]:
    for f in sorted(glob.glob(d + '/*.jsonl')):
        n = mm = conv = inc = intr = acc = 0
        for l in open(f):
            try:
                r = json.loads(l)
            except json.JSONDecodeError:
                continue
            n += 1
            if r.get('mismatch'):
                mm += 1
            if r.get('accounting_only'):
                acc += 1
            if r.get('converged_at') is not None or r.get('converged'):
                conv += 1
            if r.get('interrupted'):
                intr += 1
        print(f'{f}: {n} compiles, {mm} mismatches, {conv} converged, {intr} interrupted, {acc} accounting-only')
