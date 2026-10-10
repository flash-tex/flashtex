"""tab.py PREFIX...: the summary lines whose tag starts with one of PREFIX, as a table."""
import json
import sys

pre = sys.argv[1:]
print('| tag | doc | mode | peak | open | steady (idle) | malloc in use | edited instr p50 M | keystroke instr p50 M | open instr M | edited p50 ms |')
print('|---|---|---|---|---|---|---|---|---|---|---|')
for line in open('/private/tmp/mb/out/summary.jsonl'):
    d = json.loads(line)
    if not any(d['tag'].startswith(p) for p in pre):
        continue
    print(f"| {d['tag']} | {d['doc']} | {d['mode']} | {d['peak_mb']} | {d.get('open_fp_mb')} | {d['steady_fp_mb']} | "
          f"{d['malloc_in_use_mb']} | {d['instr_p50_M']} | {d.get('all_instr_p50_M')} | {d.get('open_instr_M')} | "
          f"{round(d['ed_p50'] or 0, 1)} |")
