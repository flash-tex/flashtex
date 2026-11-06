#!/usr/bin/env python3
"""Summarise a cachegrind output file by *source* function (lane CONTIG-TOKENS).

usage: cgsum.py CACHEGRIND_OUT SRC_DIR [--detail FN,FN,...] [--top N]

Cachegrind charges an inlined routine's instructions to the function it was inlined into, but its
line table still names the routine's own source lines. With SRC_DIR (the variant's
crates/flashtex-engine/src, generated/ included) every counted line of a file found there is charged
to the `fn` that encloses it in that source, so `get_avail` and `flush_list` (always inlined) get
their own totals. Prints: the totals; the top source functions by instructions and by D1 read misses;
then, for each --detail function, every counted line with its events, its source text and the
functions it was inlined into."""
import argparse, collections, os, re, sys

ap = argparse.ArgumentParser()
ap.add_argument('cg')
ap.add_argument('src')
ap.add_argument('--detail', default='get_next,get_avail,flush_list,end_token_list,begin_token_list,'
                'back_input,macro_call,delete_token_ref,get_token,expand,scan_toks,the_toks,str_toks,'
                'show_token_list,flush_node_list,get_node,free_node')
ap.add_argument('--top', type=int, default=45)
a = ap.parse_args()

FN_RE = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?fn\s+(\w+)')
sources = {}
for root, _, files in os.walk(a.src):
    for f in files:
        if f.endswith('.rs'):
            sources.setdefault(f, os.path.join(root, f))
fnmap, texts = {}, {}


def load(base):
    if base in fnmap:
        return
    path = sources.get(base)
    if not path:
        fnmap[base] = texts[base] = None
        return
    lines = open(path, encoding='utf-8', errors='replace').read().split('\n')
    cur, owners = None, [None]
    for l in lines:
        m = FN_RE.match(l)
        if m:
            cur = m.group(1)
        owners.append(cur)
    fnmap[base], texts[base] = owners, [''] + lines


events = None
per_line = collections.defaultdict(lambda: None)  # (base, line) -> counts
per_line_outer = collections.defaultdict(collections.Counter)  # (base, line) -> outer fn -> Ir
fl = fn = None
for raw in open(a.cg, encoding='utf-8', errors='replace'):
    if raw.startswith('events:'):
        events = raw.split()[1:]
        continue
    if raw.startswith('fl='):
        fl = os.path.basename(raw[3:].strip())
        continue
    if raw.startswith('fn='):
        fn = raw[3:].strip()
        continue
    if not raw[:1].isdigit():
        continue
    v = raw.split()
    line, c = int(v[0]), [int(x) for x in v[1:]]
    c += [0] * (len(events) - len(c))
    k = (fl, line)
    old = per_line[k]
    per_line[k] = c if old is None else [x + y for x, y in zip(old, c)]
    short = re.sub(r'.*::', '', fn)
    per_line_outer[k][short] += c[0]

E = {e: i for i, e in enumerate(events)}
tot = [0] * len(events)
by_src = collections.defaultdict(lambda: [0] * len(events))
for (base, line), c in per_line.items():
    tot = [x + y for x, y in zip(tot, c)]
    load(base)
    owners = fnmap.get(base)
    name = owners[line] if owners and line < len(owners) and owners[line] else None
    key = f'{name} ({base})' if name else f'? ({base})'
    by_src[key] = [x + y for x, y in zip(by_src[key], c)]


def fmt(c):
    return ' '.join(f'{e}={c[i]:,}' for i, e in enumerate(events))


print(f'# {a.cg}')
print('totals:', fmt(tot))
cols = events
print('\n## top source functions by Ir (share of total Ir)')
print('fn\t' + '\t'.join(cols))
for k, c in sorted(by_src.items(), key=lambda kv: -kv[1][0])[:a.top]:
    print(f'{k}\t' + '\t'.join(str(x) for x in c) + f'\t{100.0 * c[0] / max(tot[0], 1):.2f}%')
if 'D1mr' in E:
    print('\n## top source functions by D1mr')
    for k, c in sorted(by_src.items(), key=lambda kv: -kv[1][E['D1mr']])[:a.top]:
        print(f'{k}\t' + '\t'.join(str(x) for x in c))

want = set(a.detail.split(','))
print('\n## per-line detail (line, events, outer functions by Ir, source)')
for name in a.detail.split(','):
    rows = []
    for (base, line), c in per_line.items():
        owners = fnmap.get(base)
        if owners and line < len(owners) and owners[line] == name:
            rows.append((base, line, c))
    if not rows:
        continue
    s = [0] * len(events)
    for _, _, c in rows:
        s = [x + y for x, y in zip(s, c)]
    print(f'\n### {name}: {fmt(s)}')
    for base, line, c in sorted(rows):
        if c[0] == 0:
            continue
        outer = ', '.join(f'{f}:{n}' for f, n in per_line_outer[(base, line)].most_common(4))
        text = texts[base][line].strip()[:110] if texts.get(base) else ''
        print(f'{base}:{line}\t' + '\t'.join(str(x) for x in c) + f'\t[{outer}]\t{text}')
