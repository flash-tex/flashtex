"""LOC of the engine by category, and the largest hand-written functions.
Usage: python3 loc.py REPO_ROOT"""
import os, re, sys

root = sys.argv[1]
E = os.path.join(root, 'crates/flashtex-engine')


def lines(p):
    with open(p, 'rb') as f:
        return f.read().count(b'\n')


def walk(d, exts):
    out = []
    for dp, dn, fn in os.walk(d):
        if '/target' in dp:
            continue
        for f in fn:
            if f.endswith(exts):
                out.append(os.path.join(dp, f))
    return out


cats = {}
for p in walk(os.path.join(E, 'src'), ('.rs',)):
    rel = os.path.relpath(p, E)
    if rel.startswith('src/generated/'):
        k = 'generated (web2rust output)'
    else:
        parts = rel.split('/')
        k = 'hand: ' + ('/'.join(parts[:2]) if len(parts) > 2 else rel)
    cats[k] = cats.get(k, 0) + lines(p)
cats['change files (crates/flashtex-engine/changes)'] = sum(lines(p) for p in walk(os.path.join(E, 'changes'), ('.ch',)))
cats['C/C++ shims (csrc, kpathsea-config)'] = sum(lines(p) for p in walk(os.path.join(E, 'csrc'), ('.c', '.cc', '.h'))) + sum(
    lines(p) for p in walk(os.path.join(E, 'kpathsea-config'), ('.c', '.h')))
cats['engine tests (tests/*.rs)'] = sum(lines(p) for p in walk(os.path.join(E, 'tests'), ('.rs',)))
cats['build.rs + build-id'] = lines(os.path.join(E, 'build.rs')) + sum(lines(p) for p in walk(os.path.join(E, 'build-id'), ('.rs',)))
cats['tools/web2rust (src)'] = sum(lines(p) for p in walk(os.path.join(root, 'tools/web2rust/src'), ('.rs',)))
for t in ('tools/lockstep', 'tools/parity', 'tools/latex-suites', 'tools/fuzz', 'tools/displaylist', 'crates/display-list-v3', 'tools/snapshot-bench'):
    d = os.path.join(root, t)
    if os.path.isdir(d):
        cats[t + ' (code)'] = sum(lines(p) for p in walk(d, ('.rs', '.py', '.sh', '.swift')))
hand = sum(v for k, v in cats.items() if k.startswith('hand: '))
for k in sorted(cats, key=lambda k: -cats[k]):
    print(f'{cats[k]:8d}  {k}')
print(f'{hand:8d}  == hand-written engine Rust total')

# largest functions in hand-written engine Rust (brace matching on `fn`)
fnre = re.compile(r'^\s*(pub(\([^)]*\))?\s+)?(unsafe\s+)?(extern\s+"C"\s+)?fn\s+(\w+)')
sizes = []
for p in walk(os.path.join(E, 'src'), ('.rs',)):
    rel = os.path.relpath(p, E)
    gen = rel.startswith('src/generated/')
    src = open(p, encoding='utf-8', errors='replace').read().split('\n')
    i = 0
    while i < len(src):
        m = fnre.match(src[i])
        if m:
            depth = 0
            started = False
            j = i
            while j < len(src):
                s = re.sub(r'"(\\.|[^"\\])*"', '""', src[j])
                s = re.sub(r"'(\\.|[^'\\])'", "''", s)
                s = s.split('//')[0]
                depth += s.count('{') - s.count('}')
                if s.count('{'):
                    started = True
                if started and depth <= 0:
                    break
                if not started and s.rstrip().endswith(';'):
                    break
                j += 1
            sizes.append((j - i + 1, gen, rel, i + 1, m.group(5)))
            i = j + 1 if started else i + 1
        else:
            i += 1
print('\nLargest hand-written functions:')
for n, gen, rel, ln, name in sorted([s for s in sizes if not s[1]], reverse=True)[:15]:
    print(f'{n:6d}  {rel}:{ln}  {name}')
print('\nLargest generated functions:')
for n, gen, rel, ln, name in sorted([s for s in sizes if s[1]], reverse=True)[:8]:
    print(f'{n:6d}  {rel}:{ln}  {name}')
hw = [s for s in sizes if not s[1]]
print(f'\nhand-written fns: {len(hw)}; >100 lines: {sum(1 for s in hw if s[0] > 100)}; >200 lines: {sum(1 for s in hw if s[0] > 200)}')
