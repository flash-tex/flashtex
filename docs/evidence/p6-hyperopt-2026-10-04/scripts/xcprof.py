#!/usr/bin/env python3
"""Summarise an Instruments 'CPU Profiler' cpu-profile export (cycle-weighted PMU samples):
self and inclusive cycle share per function, optionally only samples whose stack contains --under FUNC
and only the thread named --thread.

usage: xcprof.py EXPORT.xml [--top N] [--under SUBSTR] [--thread SUBSTR] [--callers FUNC]"""
import argparse, collections, re, sys
import xml.etree.ElementTree as ET

ap = argparse.ArgumentParser()
ap.add_argument('xml')
ap.add_argument('--top', type=int, default=40)
ap.add_argument('--under', action='append', default=[])
ap.add_argument('--thread')
ap.add_argument('--not', dest='nott', action='append', default=[])
ap.add_argument('--callers')
ap.add_argument('--callees')
a = ap.parse_args()

ids = {}


def resolve(e):
    r = e.get('ref')
    if r is not None:
        return ids[r]
    i = e.get('id')
    if i is not None:
        ids[i] = e
    for c in e:
        resolve(c)
    return e


def idents(n):
    out, i = [], 0
    while i < len(n):
        m = re.match(r'(\d+)', n[i:])
        if not m:
            i += 1
            continue
        k = int(m.group(1))
        j = i + len(m.group(1))
        if n[j:j + 1] == '_':
            j += 1
        s = n[j:j + k]
        if k > 0 and len(s) == k and re.match(r'^[A-Za-z_][A-Za-z0-9_]*$', s):
            out.append(s)
            i = j + k
        else:
            i += 1
    return out


def short(n):
    if n.startswith('_R'):
        ids_ = [x for x in idents(n[2:]) if not re.match(r'^[a-z]?[A-Za-z0-9]{8,}_$', x)]
        ids_ = [x for x in ids_ if x not in ('flashtex_engine', 'generated', 'globals', 'Globals', 'std', 'core', 'alloc')]
        return '::'.join(ids_[-2:]) if ids_ else n
    n = re.sub(r'::h[0-9a-f]{16}$', '', n)
    return n


selfw, incl, total = collections.Counter(), collections.Counter(), 0
callers, callees = collections.Counter(), collections.Counter()
nsamp = 0
for ev, row in ET.iterparse(a.xml, events=('end',)):
    if row.tag != 'row':
        continue
    resolve(row)
    th = row.find('thread')
    th = ids.get(th.get('ref')) if th is not None and th.get('ref') else th
    if a.thread and (th is None or a.thread not in (th.get('fmt') or '')):
        continue
    w = row.find('cycle-weight')
    if w is None:
        continue
    w = ids[w.get('ref')] if w.get('ref') else w
    wt = int(w.text)
    bt = row.find('tagged-backtrace')
    if bt is None:
        continue
    bt = ids[bt.get('ref')] if bt.get('ref') else bt
    b = bt.find('backtrace')
    b = ids[b.get('ref')] if b is not None and b.get('ref') else b
    if b is None:
        continue
    frames = []
    for f in b:
        if f.tag != 'frame':
            continue
        f = ids[f.get('ref')] if f.get('ref') else f
        frames.append(short(f.get('name') or f.get('addr') or '?'))
    if a.under and not all(any(u in fr for fr in frames) for u in a.under):
        continue
    if a.nott and any(any(u in fr for fr in frames) for u in a.nott):
        continue
    nsamp += 1
    total += wt
    if frames:
        selfw[frames[0]] += wt
    for fr in set(frames):
        incl[fr] += wt
    if a.callers:
        for i, fr in enumerate(frames):
            if fr == a.callers or (a.callers in fr):
                callers[frames[i + 1] if i + 1 < len(frames) else '<root>'] += wt
                break
    if a.callees:
        for i, fr in enumerate(frames):
            if a.callees in fr:
                callees[frames[i - 1] if i > 0 else '<self>'] += wt
                break
    row.clear()

print(f'samples {nsamp}, cycles {total / 1e6:.1f} M')
print(f'{"self%":>6} {"incl%":>6}  function')
for fn, w in selfw.most_common(a.top):
    print(f'{100 * w / total:6.2f} {100 * incl[fn] / total:6.2f}  {fn[:140]}')
print('\n-- inclusive --')
for fn, w in incl.most_common(a.top):
    print(f'{100 * w / total:6.2f}  {fn[:140]}')
if a.callers:
    print(f'\n-- callers of {a.callers} --')
    for fn, w in callers.most_common(25):
        print(f'{100 * w / total:6.2f}  {fn[:140]}')
if a.callees:
    print(f'\n-- callees of {a.callees} --')
    for fn, w in callees.most_common(25):
        print(f'{100 * w / total:6.2f}  {fn[:140]}')
