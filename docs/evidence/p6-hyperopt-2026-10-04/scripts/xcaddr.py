#!/usr/bin/env python3
"""Leaf-sample cycle weights by instruction address for functions whose name contains SUBSTR, from an
Instruments cpu-profile export. usage: xcaddr.py EXPORT.xml SUBSTR [load_slide_hex]"""
import collections, sys
import xml.etree.ElementTree as ET

path, sub = sys.argv[1], sys.argv[2]
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


hist, total = collections.Counter(), 0
for ev, row in ET.iterparse(path, events=('end',)):
    if row.tag != 'row':
        continue
    resolve(row)
    w = row.find('cycle-weight')
    if w is None:
        continue
    w = ids[w.get('ref')] if w.get('ref') else w
    bt = row.find('tagged-backtrace')
    if bt is None:
        continue
    bt = ids[bt.get('ref')] if bt.get('ref') else bt
    b = bt.find('backtrace')
    b = ids[b.get('ref')] if b is not None and b.get('ref') else b
    if b is None:
        continue
    fr = next((f for f in b if f.tag == 'frame'), None)
    if fr is None:
        continue
    fr = ids[fr.get('ref')] if fr.get('ref') else fr
    if sub in (fr.get('name') or ''):
        bi = fr.find('binary')
        bi = ids[bi.get('ref')] if bi is not None and bi.get('ref') else bi
        la = int(bi.get('load-addr'), 16) if bi is not None and bi.get('load-addr') else 0
        hist[int(fr.get('addr'), 16) - la] += int(w.text)
        total += int(w.text)
    row.clear()
for a, wt in sorted(hist.items()):
    print(f'{a + 0x100000000:#x} {100 * wt / total:5.1f}')
