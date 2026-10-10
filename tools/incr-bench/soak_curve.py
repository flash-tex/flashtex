#!/usr/bin/env python3
"""soak_curve.py OUT.jsonl... [--every N]: soak.py's runs as small curves (lane MEMORY-SAFETY,
docs/evidence/mem-soak-2026-10-04/). For each run prints its summary (start, warm-up peak, middle,
end in MB, the second-half slope in MB per 100 edits) and writes OUT.csv beside it: every N-th
keystroke (default 20) with the host's footprint, the undo logs, malloc's bytes in use less the logs,
the checkpoints and the position-correction entries, all from that keystroke's DONE `mem`."""
import json
import sys

args = sys.argv[1:]
every = 20
if '--every' in args:
    i = args.index('--every')
    every = int(args[i + 1])
    del args[i:i + 2]
MB = 2**20
for path in args:
    rows = [json.loads(line) for line in open(path)]
    summ = next((r for r in rows if r.get('summary')), {})
    recs = [r for r in rows if 'i' in r and r.get('mem')]
    out = path.rsplit('.jsonl', 1)[0] + '.csv'
    with open(out, 'w') as f:
        f.write('edit,footprint_mb,logs_mb,malloc_less_logs_mb,checkpoints,reloc\n')
        for r in recs:
            if r['i'] % every and r is not recs[-1]:
                continue
            m = r['mem']
            heap = m.get('malloc_in_use')
            heap = None if heap is None else heap + m.get('mapped_bytes', m.get('log_mapped', 0)) - m.get('sealed_bytes', 0)
            f.write(f"{r['i']},{m.get('rss', 0) / MB:.1f},{m.get('sealed_bytes', 0) / MB:.1f},"
                    f"{'' if heap is None else f'{heap / MB:.1f}'},{m.get('checkpoints', '')},"
                    f"{m.get('reloc', '')}\n")
    keys = ('doc', 'engine', 'edits', 'open_mb', 'warm_peak_mb', 'mid_mb', 'end_mb', 'peak_mb',
            'slope_mb_per_100', 'heap_open_mb', 'heap_warm_mb', 'heap_end_mb', 'heap_max_mb',
            'heap_slope_mb_per_100', 'edit_ms_p50', 'edit_ms_p95', 'pass', 'failed_checks')
    print(json.dumps({k: summ.get(k) for k in keys}), '->', out)
