import sys, re, collections
pat = sys.argv[2]
c = collections.Counter(); n = collections.Counter()
for l in open(sys.argv[1]):
    if pat not in l:
        continue
    m = re.match(r'(\d+) calls? for (\d+) bytes: (.*)', l)
    if not m:
        continue
    fr = [f.split(') ', 1)[-1] for f in m.group(3).split(' | ')]
    i = max(k for k, f in enumerate(fr) if pat in f)
    def short(f):
        parts = re.findall(r'\d+([A-Za-z_][A-Za-z0-9_]*)', f) if f.startswith('_R') else [f]
        return '::'.join(p for p in parts if not p.startswith('Cs'))[-50:]
    key = ' > '.join(short(f) for f in fr[i:i + 5])
    c[key] += int(m.group(2)); n[key] += int(m.group(1))
for k, v in c.most_common(10):
    print(round(v / 1e6, 1), n[k], k)
