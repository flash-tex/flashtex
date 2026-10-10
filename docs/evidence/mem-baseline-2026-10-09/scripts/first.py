import json
import sys

for d in sys.argv[1:]:
    r = json.loads(open(f'/private/tmp/mb/w/{d}/recs.jsonl').readline())
    h = r['host']
    print(d, h.get('passes'), h.get('pass_modes'), h.get('pass_s'), h.get('s0_error'), h['stages'].get('cpu'),
          h['stages'].get('instr_k'))
