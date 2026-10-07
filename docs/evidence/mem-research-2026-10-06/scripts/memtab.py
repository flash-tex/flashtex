#!/usr/bin/env python3
"""memtab.py OUT.json: per phase, the DONE.mem parts (MB) and the smaps groups at the phase's peak."""
import json, sys
d = json.load(open(sys.argv[1]))
MB = lambda x: x / 2**20
keys = ['rss', 'heap', 'heap_peak', 'malloc_in_use', 'malloc_held', 'sealed_bytes', 'branch_sealed', 'checkpoints',
        'space_touched', 'space_written_after_ckpt', 'space_resident', 'slab_mapped', 'slab_live', 'slab_resident', 'prepared',
        'page_cache', 'form_cache', 'spare_tails', 'records', 'record_lines', 'terminal', 'texts', 'written', 'old_cache']
for ph in d['phases']:
    for m in ph['mem'][-1:]:
        print('==', ph['name'], 'peak RSS', round(MB(d['peak'].get(ph['name'], 0))))
        print('  ' + ' '.join(f'{k}={MB(m[k]):.1f}' if k not in ('checkpoints', 'records', 'record_lines') else f'{k}={m[k]}'
                              for k in keys if k in m))
        tags = {k: MB(v) for k, v in m.items() if k.startswith('heap_') and not k.startswith('heap_peak')}
        if tags:
            print('  heap now: ' + ' '.join(f'{k[5:]}={v:.1f}' for k, v in sorted(tags.items(), key=lambda kv: -kv[1]) if v >= 0.5))
        pk = {k: MB(v) for k, v in m.items() if k.startswith('heap_peak_')}
        if pk:
            print('  heap at peak: ' + ' '.join(f'{k[10:]}={v:.1f}' for k, v in sorted(pk.items(), key=lambda kv: -kv[1]) if v >= 0.5))
        res = {k: MB(v) for k, v in m.items() if k.startswith('res_')}
        print('  word space resident: ' + ' '.join(f'{k[4:]}={v:.1f}' for k, v in sorted(res.items(), key=lambda kv: -kv[1])))
for name, bd in [('peak', max(d['peak_smaps'].values(), key=lambda b: b['rss']) if d.get('peak_smaps') else None),
                 ('steady', {'rss': d.get('steady_rss', 0), 'groups': d.get('steady_smaps', {})})]:
    if not bd:
        continue
    print(f'smaps at {name}: RSS {MB(bd["rss"]):.0f} MB')
    for g, v in list(bd['groups'].items())[:9]:
        print(f'   {g:22} n={v["n"]:5} rss={MB(v["rss"]):7.1f} anon={MB(v.get("anon", 0)):7.1f}')
