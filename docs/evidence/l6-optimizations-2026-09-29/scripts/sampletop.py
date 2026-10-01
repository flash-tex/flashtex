#!/usr/bin/env python3
"""Inclusive and self sample counts per function from macOS `sample` output.

usage: sampletop.py SAMPLE.txt... [--top N] [--under FUNC]

Reads the "Call graph" tree of one or more `sample` reports (summed), and
prints for each function its inclusive count (samples with the function
anywhere on the stack, counted once when recursive) and self count (samples
with it on top). --under FUNC restricts to stacks through FUNC. Rust symbols
are shortened to their last path component."""
import argparse
import re
from collections import Counter

ap = argparse.ArgumentParser()
ap.add_argument('files', nargs='+')
ap.add_argument('--top', type=int, default=40)
ap.add_argument('--under')
a = ap.parse_args()

LINE = re.compile(r'^(?P<pre>[\s+!:|]*?)(?P<n>\d+) (?P<sym>\S.*?)\s+\(in (?P<lib>[^)]*)\)')


def short(sym):
    # _RNvMNt...Globals8get_next -> get_next ; keep C names as they are
    if sym.startswith('_R'):
        parts = re.findall(r'(\d+)([A-Za-z_][A-Za-z0-9_]*)', sym)
        name = ''
        # take the last length-prefixed identifier
        for ln, rest in re.findall(r'(\d+)([A-Za-z_].*?)(?=\d+[A-Za-z_]|$)', sym):
            pass
        m = list(re.finditer(r'(\d+)([A-Za-z_])', sym))
        out = []
        i = 0
        while i < len(sym):
            mm = re.match(r'(\d+)', sym[i:])
            if mm:
                ln = int(mm.group(1))
                j = i + len(mm.group(1))
                ident = sym[j:j + ln]
                if ident and (ident[0].isalpha() or ident[0] == '_'):
                    out.append(ident)
                    i = j + ln
                    continue
            i += 1
        cand = [o for o in out if o not in ('generated', 'globals', 'Globals', 'flashtex_engine') and not o.startswith('body_')]
        return cand[-1] if cand else sym
    return sym


incl = Counter()
selfc = Counter()
total = 0
for fn in a.files:
    lines = open(fn, errors='replace').read().split('\n')
    try:
        start = next(i for i, l in enumerate(lines) if l.startswith('Call graph:'))
    except StopIteration:
        continue
    stack = []  # (depth, name, count)
    nodes = []
    for l in lines[start + 1:]:
        if not l.strip():
            break
        m = LINE.match(l)
        if not m:
            continue
        depth = len(m.group('pre'))
        n = int(m.group('n'))
        name = short(m.group('sym'))
        while stack and stack[-1][0] >= depth:
            stack.pop()
        stack.append((depth, name, n))
        nodes.append(([s[1] for s in stack], n))
    # self count of a node = its count minus its children's counts
    for idx, (path, n) in enumerate(nodes):
        child = 0
        d = len(path)
        for p2, n2 in nodes[idx + 1:]:
            if len(p2) <= d:
                break
            if len(p2) == d + 1:
                child += n2
        s = n - child
        if a.under and a.under not in path:
            continue
        if s > 0:
            total += s
            selfc[path[-1]] += s
            for f in set(path):
                incl[f] += s
print(f'total samples {total}' + (f' under {a.under}' if a.under else ''))
print(f'{"function":60} {"incl":>7} {"incl%":>6} {"self":>7} {"self%":>6}')
for f, c in incl.most_common(a.top):
    print(f'{f[:60]:60} {c:7} {100 * c / total:6.1f} {selfc[f]:7} {100 * selfc[f] / total:6.1f}')
print('--- top self')
for f, c in selfc.most_common(a.top):
    print(f'{f[:60]:60} {incl[f]:7} {100 * incl[f] / total:6.1f} {c:7} {100 * c / total:6.1f}')
