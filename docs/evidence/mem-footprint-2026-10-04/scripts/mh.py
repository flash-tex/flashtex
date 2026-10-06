#!/usr/bin/env python3
"""mh.py FILE.mh [depth]: live malloc bytes by the innermost meaningful flashtex/C frame."""
import collections, re, sys
SKIP = re.compile(r'alloc|raw_vec|RawVec|malloc|realloc|reserve|grow|memstat|_ZN5alloc|clone|Clone|'
                  r'to_vec|to_owned|from_iter|FromIterator|SpecFrom|SpecExtend|extend|collect|Extend|'
                  r'push|insert|with_capacity|from_elem|hashbrown|String|vec|Vec|Box|Rc|Arc|calloc|'
                  r'xmalloc|xrealloc|xcalloc|xstrdup|strdup|concat|new_uninit|io..Read|read_to_end|'
                  r'Write|write_all|BufWriter|BufReader|format|fmt', re.I)
depth = int(sys.argv[2]) if len(sys.argv) > 2 else 1
by = collections.Counter()
calls = collections.Counter()
tot = 0
for line in open(sys.argv[1]):
    m = re.match(r'(\d+) calls? for (\d+) bytes: (.*)', line)
    if not m:
        continue
    n, b, st = int(m.group(1)), int(m.group(2)), m.group(3)
    frames = [f.strip() for f in st.split('|')]
    if any('mmap' in f or 'vm_allocate' in f or 'mach_vm' in f for f in frames[-3:]):
        continue
    tot += b
    names = []
    for f in reversed(frames):
        mm = re.match(r'0x[0-9a-f]+ \((\S+)\) (.*)', f)
        if not mm:
            continue
        img, sym = mm.groups()
        if img != 'flashtex-host':
            continue
        if SKIP.search(sym):
            continue
        # shorten a mangled Rust name to its last path parts
        parts = re.findall(r'\d+([A-Za-z_][A-Za-z0-9_]*)', sym)
        short = '::'.join(p for p in parts if not p.startswith('Cs'))[-90:] if sym.startswith('_R') else sym
        names.append(short)
        if len(names) >= depth:
            break
    key = ' <- '.join(names) or '(no flashtex frame)'
    by[key] += b
    calls[key] += n
print(f'total live malloc bytes (non-VM): {tot/1e6:.1f} MB')
for k, b in by.most_common(45):
    print(f'{b/1e6:8.1f} MB {calls[k]:8d}  {k}')
