#!/usr/bin/env python3
"""Parse an xctrace cpu-profile export once into a pickle of (stack -> cycles), then query it.
usage: xcstk.py X.xml                      (writes X.pkl)
       xcstk.py X.pkl GROUP=substr[,substr...] ...   inclusive share of each group (a sample counts once per group)
       xcstk.py X.pkl --callers FUNC | --callees FUNC"""
import collections, pickle, re, sys
import xml.etree.ElementTree as ET
sys.path.insert(0, '/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a006882406b05a296/docs/evidence/p6-hyperopt-2026-10-04/scripts')

if sys.argv[1].endswith('.xml'):
    sys.argv = [sys.argv[0], sys.argv[1]]
    src = open('/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a006882406b05a296/docs/evidence/p6-hyperopt-2026-10-04/scripts/xcprof.py').read()
    # reuse its demangling helpers only
    ns = {}
    exec(src.split('selfw, incl, total')[0].replace('a = ap.parse_args()', 'a = None'), ns)
    resolve, short, ids = ns['resolve'], ns['short'], ns['ids']
    stacks = collections.Counter()
    for ev, row in ET.iterparse(sys.argv[1], events=('end',)):
        if row.tag != 'row':
            continue
        resolve(row)
        w = row.find('cycle-weight')
        if w is None:
            continue
        w = ids[w.get('ref')] if w.get('ref') else w
        bt = row.find('tagged-backtrace')
        if bt is None:
            continue
        bt = ids[bt.get('ref')] if bt.get('ref') else bt
        b = bt.find('backtrace')
        b = ids[b.get('ref')] if b is not None and b.get('ref') else b
        if b is None:
            continue
        fr = []
        for f in b:
            if f.tag == 'frame':
                f = ids[f.get('ref')] if f.get('ref') else f
                fr.append(short(f.get('name') or f.get('addr') or '?'))
        stacks[tuple(fr)] += int(w.text)
        row.clear()
    pickle.dump(stacks, open(sys.argv[1][:-4] + '.pkl', 'wb'))
    print('stacks', len(stacks), 'cycles', sum(stacks.values()) / 1e6, 'M')
    sys.exit()

stacks = pickle.load(open(sys.argv[1], 'rb'))
total = sum(stacks.values())
args = sys.argv[2:]
if args and args[0] in ('--callers', '--callees'):
    f = args[1]
    c = collections.Counter()
    for st, w in stacks.items():
        for i, fr in enumerate(st):
            if f in fr:
                if args[0] == '--callers':
                    c[st[i + 1] if i + 1 < len(st) else '<root>'] += w
                else:
                    c[st[i - 1] if i > 0 else '<self>'] += w
                break
    for k, w in c.most_common(30):
        print(f'{100 * w / total:6.2f}  {k}')
    sys.exit()
print(f'total {total / 1e9:.1f} G cycles')
for g in args:
    name, subs = g.split('=', 1)
    subs = subs.split(',')
    w = sum(w for st, w in stacks.items() if any(any(s in fr for s in subs) for fr in st))
    print(f'{100 * w / total:6.2f}  {name}')
