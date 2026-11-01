#!/usr/bin/env python3
"""Keep both sides of a soundness comparison for inspection: an incremental
host after one edit (`--kind cite|bib|index`), and a fresh host on a copy of
the same sources, each copied as it stood when `settled` arrived (before any
export). Prints which pages and files differ.

    debug_pair.py --host BIN --formats DIR --pool FILE --doc DIR:MAIN --kind bib --out DIR
"""
import argparse
import os
import random
import shutil
import sys

sys.path.insert(0, "/Users/dqi26/flashtex/.claude/worktrees/agent-a9b0d190d583b912c/tools/external-tools")
import xtools as x  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument("--host", required=True)
ap.add_argument("--formats", required=True)
ap.add_argument("--pool", required=True)
ap.add_argument("--texbin", default=os.path.dirname(shutil.which("latexmk") or "/usr/bin/latexmk"))
ap.add_argument("--doc", required=True)
ap.add_argument("--kind", default="cite")
ap.add_argument("--seed", type=int, default=1)
ap.add_argument("--out", required=True)
ap.add_argument("--env", action="append", default=[])
a = ap.parse_args()
a.env = dict(kv.split("=", 1) for kv in a.env)
src, main = a.doc.split(":")
name = os.path.basename(os.path.normpath(src))
rng = random.Random(a.seed + sum(map(ord, name)))
shutil.rmtree(a.out, ignore_errors=True)
cd = os.path.join(a.out, "cand")
x.copy_sources(src, cd, main)
h = x.Host(a.host, a.formats, a.pool, a.out, env_extra=a.env)
req = {"id": 1, "root": cd, "main": main, "output_dir": cd, "external_tools": "auto", "incremental": True}
ev = h.cycle(req)
print("open:", [d.get("mode") for d in ev["dones"]], x.summary_tools(ev))
shutil.copytree(cd, os.path.join(a.out, "cand-open"))
r = {"cite": lambda: x.edit_cite(cd, main, rng, a.texbin), "bib": lambda: x.edit_bib(cd, rng, src),
     "index": lambda: x.edit_index(cd, main, rng)}[a.kind]()
e, what = r
print("edit:", e, what)
req = dict(req, id=2, edits=[e])
ev = h.cycle(req)
print("edit:", [(d.get("mode"), d.get("restart_page"), d.get("converged_at"), d.get("rerun_pages"))
                for d in ev["dones"]], x.summary_tools(ev))
cand_pages = dict(h.pages)
cand_bodies = dict(h.bodies)
cand_final = {i: x.page_digest(b, h.fonts, h.images, h.forms) for i, (b, _) in h.bodies.items()}
cand_forms = dict(h.forms)
shutil.copytree(cd, os.path.join(a.out, "cand-final"))
h.close()
fd = os.path.join(a.out, "fresh")
x.copy_sources(src, fd, main)
x.apply_edit(fd, e)
os.makedirs(os.path.join(a.out, "fresh-host"))
h2 = x.Host(a.host, a.formats, a.pool, os.path.join(a.out, "fresh-host"), env_extra=a.env)
ev2 = h2.cycle({"id": 1, "root": fd, "main": main, "output_dir": fd, "external_tools": "auto", "incremental": True})
print("fresh:", [d.get("mode") for d in ev2["dones"]], x.summary_tools(ev2))
fresh_pages = dict(h2.pages)
fresh_bodies = dict(h2.bodies)
fresh_final = {i: x.page_digest(b, h2.fonts, h2.images, h2.forms) for i, (b, _) in h2.bodies.items()}
fresh_forms = dict(h2.forms)
h2.close()
diff = sorted(i for i in set(fresh_pages) | set(cand_pages) if fresh_pages.get(i) != cand_pages.get(i))
print("pages differing:", diff, "of", len(fresh_pages), len(cand_pages))
fc, ff = x.files_of(os.path.join(a.out, "cand-final")), x.files_of(fd)
print("files differing:", [k for k in sorted(set(fc) | set(ff)) if fc.get(k) != ff.get(k)])

import struct


def glyph_fonts(b):
    (n,) = struct.unpack_from("<I", b, 120)
    at, items = 124, b""
    for _ in range(n):
        tg, ln = struct.unpack_from("<II", b, at)
        if tg == 3:
            items = b[at + 8: at + 8 + ln]
        at += 8 + ln
    i, ids, forms = 0, set(), set()
    while i < len(items):
        op = items[i]
        i += 1
        if op in (0x09, 0x0A):
            i += 1 + 8 * items[i]
            continue
        if op == 0x01:
            ids.add(struct.unpack_from("<H", items, i)[0])
        if op == 0x06:
            forms.add(struct.unpack_from("<I", items, i)[0])
        i += x.ITEM_LEN[op]
    return ids, forms


for i in diff[:3]:
    (cb, cf), (fb, ff) = cand_bodies[i], fresh_bodies[i]
    ci, cfm = glyph_fonts(cb)
    fi, ffm = glyph_fonts(fb)
    print(i, "body equal", cb == fb, "font ids", sorted(ci), sorted(fi), "forms", sorted(cfm), sorted(ffm))
    for f in sorted(ci | fi):
        a, b2 = cf.get(f), ff.get(f)
        print("   font", f, "cand", a.hex()[:16] if a else None, "fresh", b2.hex()[:16] if b2 else None,
              "same" if a == b2 else "DIFF")
print("final-state digests differing:",
      sorted(i for i in set(cand_final) | set(fresh_final) if cand_final.get(i) != fresh_final.get(i)))
for i in diff[:3]:
    _, fm = glyph_fonts(cand_bodies[i][0])
    print(i, "forms (final) differing:", [f for f in sorted(fm) if cand_forms.get(f) != fresh_forms.get(f)])
