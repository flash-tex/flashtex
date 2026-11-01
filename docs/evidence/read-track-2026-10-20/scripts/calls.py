"""calls.py CGFILE SUBSTR...: total call counts (callgrind `calls=` lines) to functions whose
name contains each SUBSTR (names compressed or not)."""
import re
import sys

path, subs = sys.argv[1], sys.argv[2:]
names = {}
totals = {s: 0 for s in subs}
cur_cfn = None
with open(path, errors='replace') as f:
    for line in f:
        if line.startswith(('cfn=', 'cfi=', 'fn=')):
            key, rest = line.split('=', 1)
            m = re.match(r'\((\d+)\)(?: (.*))?', rest.strip())
            if m:
                if m.group(2):
                    names[m.group(1)] = m.group(2)
                name = names.get(m.group(1), '')
            else:
                name = rest.strip()
            if key == 'cfn':
                cur_cfn = name
        elif line.startswith('calls=') and cur_cfn is not None:
            n = int(line.split('=')[1].split()[0])
            for s in subs:
                if s in cur_cfn:
                    totals[s] += n
            cur_cfn = None
for s in subs:
    print(s, totals[s])
