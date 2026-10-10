"""ev.py EVENTS [MIN]: malloc_history -allEvents summary: allocated bytes by size bucket,
and by the innermost flashtex frames (for blocks >= MIN, default 64 KB)."""
import collections
import re
import sys

path = sys.argv[1]
MIN = int(sys.argv[2]) if len(sys.argv) > 2 else 65536
kinds = collections.Counter()
buckets = collections.Counter()
nb = collections.Counter()
owner = collections.Counter()
ownern = collections.Counter()
rx = re.compile(r'^(\w+) 0x[0-9a-f]+(?:-0x[0-9a-f]+)? \[size=(\d+)\]:(.*)$')
SKIP = ('alloc', 'raw_vec', 'hashbrown', 'core', 'std', 'libsystem')


def short(fr):
    m = re.search(r'\(flashtex-host\) (\S+)', fr)
    if not m:
        return None
    s = m.group(1)
    # crude demangle: keep identifiers of length > 2 from v0 mangling
    ids = re.findall(r'\d+([A-Za-z_][A-Za-z0-9_]*)', s)
    ids = [i for i in ids if len(i) > 2 and not i.startswith('Cs')]
    return '::'.join(ids[-3:])


for line in open(path, errors='replace'):
    m = rx.match(line)
    if not m:
        continue
    k, size, stack = m.group(1), int(m.group(2)), m.group(3)
    kinds[k] += 1
    if k not in ('ALLOC', 'REALLOC'):
        continue
    b = 1 << max(0, (size - 1).bit_length())
    buckets[b] += size
    nb[b] += 1
    if size >= MIN:
        frames = [short(f) for f in stack.split(' | ')]
        frames = [f for f in frames if f and not any(f.startswith(s) for s in SKIP)]
        o = ' < '.join(reversed(frames[-3:])) if frames else '?'
        owner[o] += size
        ownern[o] += 1
print(kinds)
tot = sum(buckets.values())
print(f'total allocated {tot / 2**20:.1f} MB')
for b in sorted(buckets):
    print(f'  <= {b:>10}: {buckets[b] / 2**20:8.1f} MB  {nb[b]:>8} blocks')
print(f'blocks >= {MIN}:')
for o, v in owner.most_common(40):
    print(f'  {v / 2**20:8.1f} MB {ownern[o]:>6}  {o}')
