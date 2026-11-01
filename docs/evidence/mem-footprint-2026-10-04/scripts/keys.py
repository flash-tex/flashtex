import json, sys
for t in sys.argv[2:]:
    print(t)
    for l in open(f'/tmp/mfp/out/{t}-{sys.argv[1]}.jsonl'):
        r = json.loads(l)
        if not r.get('phase', '').startswith('edit') or 'key' not in r:
            continue
        h = r['host']; s = h.get('stages', {})
        print(' ', r['phase'][5:40], r['key'], 'restart', h.get('restart_page'), 'gap', h.get('restart_gap'),
              'conv', h.get('converged_at'), 'edited Minstr', s.get('edited_instr_k', 0) // 1000,
              'restore', s.get('restore_instr_k', 0) // 1000, 'ms', round(r.get('edited_page_ms') or 0))
