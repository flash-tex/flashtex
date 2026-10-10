"""cpucmp.py [ROUNDS_DIR] [BASE PR]: per doc, the summed thread CPU ms (stages.cpu) and cycles
(stages.cycles_k) of every keystroke's compile, BASE against PR."""
import glob
import json
import statistics
import sys

base = sys.argv[1] if len(sys.argv) > 1 else '/private/tmp/mb/instr-rounds'
B, P = (sys.argv[2], sys.argv[3]) if len(sys.argv) > 3 else ('m2', 'a6')
res = {}
for path in sorted(glob.glob(f'{base}/*/*/recs.jsonl')):
    rnd = path.split('/')[-3]
    name = path.split('/')[-2]
    tag, rest = name.split('-', 1)[1].split('-', 1)
    doc = rest.rsplit('-balanced', 1)[0]
    cpu = cyc = ins = 0
    for line in open(path):
        r = json.loads(line)
        if not r.get('phase', '').startswith('edit'):
            continue
        h = r.get('host') or {}
        st = h.get('stages') or {}
        if h.get('mode') == 'unchanged' or 'instr_k' not in st:
            continue
        cpu += st.get('cpu', 0)
        cyc += st.get('cycles_k', 0)
        ins += st['instr_k']
    res.setdefault((doc, tag), []).append((cpu, cyc / 1e3, ins / 1e3))
for doc in sorted({d for d, _ in res}):
    b, p = res.get((doc, B), []), res.get((doc, P), [])
    if not b or not p:
        continue
    for k, name in ((0, 'cpu ms'), (1, 'cycles M'), (2, 'instr M')):
        mb, mp = statistics.median(x[k] for x in b), statistics.median(x[k] for x in p)
        print(f'{doc:16} {name:9} {mb:12,.0f} -> {mp:12,.0f} ({(mp / mb - 1) * 100:+.2f} %)  rounds {[round(x[k]) for x in b]} / {[round(x[k]) for x in p]}')
