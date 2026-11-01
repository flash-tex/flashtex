import json, sys
rows = [json.loads(l) for l in open(sys.argv[1])]
same = diff = 0
for r in rows:
    if r['result'] != 'PASS':
        continue
    h = set()
    for x in r['host']['rules']:
        tool, f = x.split(None, 1)
        h.add(tool + ' ' + f.rsplit('.', 1)[0])
    o = set()
    for x in r['oracle']['rules']:
        if x == 'pdflatex':
            continue
        tool, f = x.split(None, 1)
        if f.endswith('.idx'):
            f = f[:-4]
        o.add(tool + ' ' + f)
    if h == o:
        same += 1
    else:
        diff += 1
        print(r['doc'], sorted(h), sorted(o))
print('same', same, 'differ', diff)
