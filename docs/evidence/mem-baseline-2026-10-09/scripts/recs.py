import json
import sys

for l in open(sys.argv[1] + '/recs.jsonl'):
    r = json.loads(l)
    h = r.get('host') or {}
    m = h.get('mem') or {}
    st = h.get('stages') or {}
    print(r.get('phase'), h.get('mode'), {k: round(m.get(k, 0) / 2**20, 1) for k in
          ['rss', 'rss_peak', 'malloc_in_use', 'malloc_held', 'mapped_bytes', 'spare_bytes', 'spare_dirty', 'sealed_bytes']},
          'instr', st.get('edited_instr_k'), 'cpu', st.get('edited_cpu'), 'wall', st.get('edited_wall'))
