import re, sys
# summarise heaptrack_print --print-peaks: each "X consumed over N calls from:" sub-entry, its first
# few frames that are in crates/ (file:line)
skip = re.compile(r'rustc|/alloc/|/core/|/std/|library/')
cur = None
frames = []
out = []
lines = open(sys.argv[1]).read().splitlines()
top = None
for i, l in enumerate(lines):
    m = re.match(r'^([0-9.]+[KMG]?) peak memory consumed over (\d+) calls', l)
    if m:
        top = m.group(1)
        print(f'\n## TOP {top} ({m.group(2)} calls)')
        continue
    m = re.match(r'^([0-9.]+[KMG]?) consumed over (\d+) calls', l)
    if m:
        if cur:
            print(cur, ' <- ', ' | '.join(frames[:int(sys.argv[2]) if len(sys.argv) > 2 else 4]))
        cur = f'  {m.group(1):>8} x{m.group(2)}'
        frames = []
        continue
    m = re.match(r'^\s+(\S.*)$', l)
    if cur and m and i + 1 < len(lines):
        nxt = lines[i + 1].strip()
        if nxt.startswith('at ') and 'crates/' in nxt and not skip.search(nxt):
            fn = m.group(1).split('::')[-1][:40]
            loc = nxt[3:].split('crates/')[-1]
            if not frames or frames[-1] != f'{fn}@{loc}':
                frames.append(f'{fn}@{loc}')
if cur:
    print(cur, ' <- ', ' | '.join(frames[:4]))
