"""chcheck.py MASTER.web ARGSFILE REPO: for each change file of ARGSFILE, in order,
count the @x blocks whose text is found neither in MASTER nor in the @y text of an
earlier change (an approximation of TANGLE/tie's sequential matching)."""
import re, sys, os

master, argsfile, repo = sys.argv[1:4]
text = open(master, encoding='latin1').read()
ys = []
tot = bad = 0
for line in open(argsfile):
    line = line.split('#')[0].strip()
    if not line.startswith('--change'):
        continue
    ch = os.path.join(repo, line.split()[1])
    src = open(ch, encoding='latin1').read()
    blocks = re.findall(r'^@x[^\n]*\n(.*?)^@y[^\n]*\n(.*?)^@z', src, re.S | re.M)
    nb = 0
    for x, y in blocks:
        tot += 1
        xs = '\n'.join(l.rstrip() for l in x.rstrip('\n').split('\n'))
        hay = '\n'.join(l.rstrip() for l in text.split('\n'))
        if xs not in hay and not any(xs in yy for yy in ys):
            nb += 1
            bad += 1
            print(f'  NO MATCH {os.path.basename(ch)}: {x.strip().splitlines()[0][:90]!r}')
        ys.append('\n'.join(l.rstrip() for l in y.split('\n')))
    print(f'{os.path.basename(ch):28s} {len(blocks):4d} hunks, {nb} not matching')
print(f'TOTAL {tot} hunks, {bad} not matching')
