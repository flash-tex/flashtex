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

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
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
x.copy_sources(src, cd)
h = x.Host(a.host, a.formats, a.pool, a.out, env_extra=a.env)
req = {"id": 1, "root": cd, "main": main, "output_dir": cd, "external_tools": "auto", "incremental": True}
ev = h.cycle(req)
print("open:", [d.get("mode") for d in ev["dones"]], x.summary_tools(ev))
shutil.copytree(cd, os.path.join(a.out, "cand-open"))
r = {"cite": lambda: x.edit_cite(cd, main, rng, a.texbin), "bib": lambda: x.edit_bib(cd, rng),
     "index": lambda: x.edit_index(cd, main, rng)}[a.kind]()
e, what = r
print("edit:", e, what)
req = dict(req, id=2, edits=[e])
ev = h.cycle(req)
print("edit:", [(d.get("mode"), d.get("restart_page"), d.get("converged_at"), d.get("rerun_pages"))
                for d in ev["dones"]], x.summary_tools(ev))
cand_pages = dict(h.pages)
cand_bodies = dict(h.bodies)
shutil.copytree(cd, os.path.join(a.out, "cand-final"))
h.close()
fd = os.path.join(a.out, "fresh")
x.copy_sources(cd, fd)
os.makedirs(os.path.join(a.out, "fresh-host"))
h2 = x.Host(a.host, a.formats, a.pool, os.path.join(a.out, "fresh-host"), env_extra=a.env)
ev2 = h2.cycle({"id": 1, "root": fd, "main": main, "output_dir": fd, "external_tools": "auto", "incremental": True})
print("fresh:", [d.get("mode") for d in ev2["dones"]], x.summary_tools(ev2))
fresh_pages = dict(h2.pages)
fresh_bodies = dict(h2.bodies)
h2.close()
diff = sorted(i for i in set(fresh_pages) | set(cand_pages) if fresh_pages.get(i) != cand_pages.get(i))
print("pages differing:", diff, "of", len(fresh_pages), len(cand_pages))
fc, ff = x.files_of(os.path.join(a.out, "cand-final")), x.files_of(fd)
print("files differing:", [k for k in sorted(set(fc) | set(ff)) if fc.get(k) != ff.get(k)])

import struct
for i in diff[:3]:
    for tag, (b, fonts) in (("cand", cand_bodies[i]), ("fresh", fresh_bodies[i])):
        (n,) = struct.unpack_from("<I", b, 120)
        at, secs = 124, {}
        for _ in range(n):
            tg, ln = struct.unpack_from("<II", b, at)
            secs[tg] = b[at + 8: at + 8 + ln]
            at += 8 + ln
        fids = sorted({struct.unpack_from("<H", secs[3], j + 1)[0] for j in range(len(secs.get(3, b""))) if False})
        print(i, tag, "hdr", b[:120].hex()[:80], {k: len(v) for k, v in secs.items()},
              "fonts", {k: v.hex()[:12] for k, v in sorted(fonts.items())}[:0] if False else len(fonts))
    ca, fr = cand_bodies[i][0], fresh_bodies[i][0]
    print(i, "header equal:", ca[:120] == fr[:120], "body equal:", ca == fr)
    j = next((k for k in range(min(len(ca), len(fr))) if ca[k] != fr[k]), None)
    print("  first differing byte", j, "lens", len(ca), len(fr))


def sections(b):
    (n,) = struct.unpack_from("<I", b, 120)
    at, secs = 124, {}
    for _ in range(n):
        tg, ln = struct.unpack_from("<II", b, at)
        secs[tg] = b[at + 8: at + 8 + ln]
        at += 8 + ln
    return secs


def links(d):
    (n,) = struct.unpack_from("<I", d, 0)
    at, out = 4, []
    for _ in range(n):
        l, t_, r, bt, span, kind = struct.unpack_from("<iiiiIB", d, at)
        at += 21
        (fl,) = struct.unpack_from("<I", d, at)
        at += 4 + fl
        (dl,) = struct.unpack_from("<I", d, at)
        data = d[at + 4: at + 4 + dl]
        at += 4 + dl
        out.append((l, t_, r, bt, kind, data))
    return out


for i in diff[:2]:
    sc, sf = sections(cand_bodies[i][0]), sections(fresh_bodies[i][0])
    print(i, "links cand ", links(sc[4]) if 4 in sc else None)
    print(i, "links fresh", links(sf[4]) if 4 in sf else None)
    print(i, "matrices", len(sc.get(1, b"")), len(sf.get(1, b"")))
