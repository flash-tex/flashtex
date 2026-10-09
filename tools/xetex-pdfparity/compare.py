#!/usr/bin/env python3
"""Compare a candidate PDF with a reference PDF, structurally and visually.

docs/design/xetex/PLAN.md §3.5: not byte for byte (font subsets, object
order and compression legitimately differ), but what the PDF shows and does:

* pages: count, MediaBox/CropBox (and Bleed/Trim/ArtBox), Rotate;
* glyphs per page: font (PostScript name without subset tag), glyph id
  (fonts.py says how it is recovered), origin within the position
  tolerance, text rendering matrix (size, slant, scale), render mode, fill
  and stroke colour, opacity; per font and glyph: subtype, embedded program
  kind, advance width (/W, /Widths), ToUnicode text, outline digest;
* rules: axis-aligned filled rectangles and butt-capped, undashed
  axis-aligned stroked segments are both the rectangle they cover, so
  `x y w h re f` and `q w 0 J x0 y m x1 y l S Q` compare equal;
* other paths: painting and clipping operator, points in page space,
  colours, line width/dash/cap/join in page space, blend mode, soft mask;
* images: the CTM at `Do`, pixel size, colour space, bits, mask, soft mask,
  decode array, digest of the (decoded where qpdf can) data;
* forms (included PDFs): CTM, /Matrix, /BBox, /Group, and their content
  as above; shadings painted with `sh`: CTM and the shading;
* links (rect, action: URI / GoTo name / explicit destination, border,
  colour, highlight) and other annotations; named destinations (name ->
  page, kind, coordinates); the outline (titles, targets, open/closed,
  depth, order); document information (Title, Author, Subject, Keywords,
  Creator); catalog PageMode/PageLayout/OpenAction/ViewerPreferences/
  PageLabels;
* visual: both PDFs rasterised by Core Graphics (raster.py) at --scale,
  differing pixels per page (0 is the target).

    compare.py REF.pdf CAND.pdf [--tol 0.01] [--rel-tol 0.005] [--scale 2]
               [--no-visual] [--diff-dir DIR] [--json OUT] [--examples 5]
               [--glyph-identity gid|outline]

Exit status 0 when nothing differs.
"""
import argparse
import bisect
import json
import math
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from content import scale_of  # noqa: E402
from model import extract  # noqa: E402
from pdfdoc import Struct, close  # noqa: E402

INF = float("inf")


class Options:
    def __init__(self, tol=0.01, rel_tol=0.005, scale=2.0, visual=True, diff_dir=None,
                 examples=5, glyph_identity="gid", smooth=False, width_tol=1.0):
        self.tol, self.rel_tol, self.scale, self.visual = tol, rel_tol, scale, visual
        self.diff_dir, self.examples, self.glyph_identity, self.smooth = diff_dir, examples, glyph_identity, smooth
        self.width_tol = width_tol


class Report:
    def __init__(self, examples=5):
        self.kinds = defaultdict(lambda: {"count": 0, "examples": []})
        self.examples = examples
        self.stats = {}
        self.notes = []  # informational: not differences

    def add(self, kind, msg):
        k = self.kinds[kind]
        k["count"] += 1
        if len(k["examples"]) < self.examples:
            k["examples"].append(msg)

    @property
    def structural(self):
        return sum(v["count"] for k, v in self.kinds.items() if k != "visual")

    @property
    def visual(self):
        return self.stats.get("visual_pixels", 0)

    def ok(self):
        return not self.kinds

    def as_json(self):
        return {"structural_differences": self.structural,
                "visual_pixels": self.visual,
                "kinds": {k: dict(v) for k, v in sorted(self.kinds.items())},
                "notes": self.notes,
                "stats": self.stats}

    def text(self, indent=""):
        lines = []
        for k, v in sorted(self.kinds.items()):
            lines.append("%s%s: %d" % (indent, k, v["count"]))
            for e in v["examples"]:
                lines.append("%s    %s" % (indent, e))
        for n in self.notes:
            lines.append("%snote (not a difference): %s" % (indent, n))
        return "\n".join(lines)


# ---------------------------------------------------------------------------
# Matching
# ---------------------------------------------------------------------------
def match(A, B, key, anchor, dist, radius, ia=None, ib=None):
    """Pair items of A with items of B that have the same key and are within
    `radius(a)` of each other (dist(a, b)); nearest first, greedily in A's
    order. `anchor(x)` is a coordinate with |anchor(a) - anchor(b)| <=
    dist(a, b), used to search. Returns (pairs [(i, j, d)], rest of A
    indices, rest of B indices)."""
    ia = list(range(len(A))) if ia is None else ia
    ib = list(range(len(B))) if ib is None else ib
    buckets = defaultdict(list)
    for j in ib:
        buckets[key(B[j])].append((anchor(B[j]), j))
    for b in buckets.values():
        b.sort()
    used = set()
    pairs = []
    rest_a = []
    for i in ia:
        a = A[i]
        bucket = buckets.get(key(a))
        if not bucket:
            rest_a.append(i)
            continue
        r = radius(a)
        x = anchor(a)
        lo = bisect.bisect_left(bucket, (x - r - 1e-9, -1))
        best, bd = None, INF
        k = lo
        while k < len(bucket) and bucket[k][0] <= x + r + 1e-9:
            j = bucket[k][1]
            if j not in used:
                d = dist(a, B[j])
                if d <= r + 1e-9 and d < bd:
                    best, bd = j, d
            k += 1
        if best is None:
            rest_a.append(i)
        else:
            used.add(best)
            pairs.append((i, best, bd))
    rest_b = [j for j in ib if j not in used]
    return pairs, rest_a, rest_b


def fmt(v, nd=3):
    if isinstance(v, float):
        return ("%.*f" % (nd, v)).rstrip("0").rstrip(".") if abs(v) < 1e9 else repr(v)
    if isinstance(v, (list, tuple)):
        return "[" + " ".join(fmt(x, nd) for x in v) + "]"
    return str(v)


def colour_str(c):
    if c is None:
        return "-"
    sp, comps = c
    return "%s%s" % (sp, fmt(list(comps), 4))


# ---------------------------------------------------------------------------
# Glyphs
# ---------------------------------------------------------------------------
def glyph_size(g):
    return math.sqrt(abs(g["m"][0] * g["m"][3] - g["m"][1] * g["m"][2]))


def compare_glyphs(pno, A, B, opt, rep, stats):
    tol = opt.tol

    def radius(g):
        # xdvipdfmx's own error, measured (README): up to 0.005 em.
        return tol + opt.rel_tol * glyph_size(g)

    if opt.glyph_identity == "outline":
        def key(g):
            return (g["font"], g["outline"] or g["glyph"])
    else:
        def key(g):
            return (g["font"], g["glyph"])

    def anchor(g):
        return g["x"]

    def dist(a, b):
        return max(abs(a["x"] - b["x"]), abs(a["y"] - b["y"]))

    pairs, ra, rb = match(A, B, key, anchor, dist, radius)
    # Moved glyphs: same glyph within 5 bp.
    moved, ra, rb = match(A, B, key, anchor, dist, lambda g: 5.0, ra, rb)
    # Same place, different glyph.
    swapped, ra, rb = match(A, B, lambda g: 0, anchor, dist, radius, ra, rb)

    def where(g):
        return "p%d (%s,%s) %s %s" % (pno, fmt(g["x"]), fmt(g["y"]), g["font"], g["glyph"])

    for i, j, d in pairs:
        a, b = A[i], B[j]
        dx, dy = abs(a["x"] - b["x"]), abs(a["y"] - b["y"])
        sz = glyph_size(a) or 1.0
        stats["glyph_max_dev"] = max(stats.get("glyph_max_dev", 0.0), dx, dy)
        stats["glyph_max_dev_em"] = max(stats.get("glyph_max_dev_em", 0.0), max(dx, dy) / sz)
        mt = 1e-4 * sz + 1e-6
        if any(abs(x - y) > mt for x, y in zip(a["m"], b["m"])):
            rep.add("glyph-matrix", "%s: %s vs %s" % (where(a), fmt(list(a["m"]), 5), fmt(list(b["m"]), 5)))
        if a["mode"] != b["mode"]:
            rep.add("glyph-mode", "%s: Tr %s vs %s" % (where(a), a["mode"], b["mode"]))
        for k in ("fill", "stroke"):
            if a.get(k) != b.get(k):
                rep.add("glyph-colour", "%s: %s %s vs %s" % (where(a), k, colour_str(a.get(k)), colour_str(b.get(k))))
        for k in ("fill_alpha", "stroke_alpha"):
            if a.get(k) != b.get(k):
                rep.add("glyph-colour", "%s: %s %s vs %s" % (where(a), k, a.get(k), b.get(k)))
        if "lw" in a and abs(a["lw"] - b.get("lw", 0)) > tol:
            rep.add("glyph-colour", "%s: stroke width %s vs %s" % (where(a), fmt(a["lw"]), fmt(b.get("lw"))))
    # Content order: the paired glyphs in the reference's order should come
    # in the candidate's order too.
    last = -1
    for i, j, d in sorted(pairs):
        if j < last:
            rep.add("glyph-order", "%s comes earlier in the candidate's content" % where(A[i]))
        last = max(last, j)
    for i, j, d in moved:
        a, b = A[i], B[j]
        stats["glyph_moved_max"] = max(stats.get("glyph_moved_max", 0.0), d)
        rep.add("glyph-position", "%s moved by (%s, %s) bp (tolerance %s)" % (
            where(a), fmt(b["x"] - a["x"], 4), fmt(b["y"] - a["y"], 4), fmt(radius(a), 4)))
    for i, j, d in swapped:
        a, b = A[i], B[j]
        if a["font"] != b["font"]:
            rep.add("glyph-font", "%s: candidate has %s %s" % (where(a), b["font"], b["glyph"]))
        elif a["outline"] and a["outline"] == b["outline"]:
            rep.add("glyph-id", "%s: candidate %s, same outline (renumbered)" % (where(a), b["glyph"]))
        else:
            rep.add("glyph-id", "%s: candidate %s (different outline)" % (where(a), b["glyph"]))
    for i in ra:
        rep.add("glyph-missing", where(A[i]))
    for j in rb:
        rep.add("glyph-extra", where(B[j]))
    stats["glyphs_ref"] = stats.get("glyphs_ref", 0) + len(A)
    stats["glyphs_cand"] = stats.get("glyphs_cand", 0) + len(B)
    stats["glyphs_matched"] = stats.get("glyphs_matched", 0) + len(pairs)


def compare_fonts(ma, mb, opt, rep):
    """Per font: subtype (a difference) and embedded program kind (a note:
    PLAN §3.5 names a font by PostScript name, face and glyph id, so a TFM
    font embedded as Type 1 /FontFile rather than xdvipdfmx's /FontFile3
    /Type1C is not a parity difference); per glyph used: width (within
    --width-tol: xdvipdfmx writes integers), ToUnicode, outline."""
    def collect(m):
        fonts = {}
        glyphs = {}
        for p in m.pages:
            for g in p.content.glyphs:
                f = g["fontobj"]
                fonts.setdefault(f.name, set()).add(tuple(sorted(f.summary().items())))
                gl = f.glyph(g["code"])
                glyphs.setdefault((f.name, gl.id), set()).add((gl.width, gl.unicode, gl.outline))
        return fonts, glyphs

    fa, ga = collect(ma)
    fb, gb = collect(mb)
    for n in sorted(set(fa) & set(fb)):
        ta = {(d["subtype"], d["cid_subtype"]) for d in map(dict, fa[n])}
        tb = {(d["subtype"], d["cid_subtype"]) for d in map(dict, fb[n])}
        if ta != tb:
            rep.add("font", "%s: %s vs %s" % (n, sorted(ta, key=str), sorted(tb, key=str)))
        pa = sorted({dict(d)["program"] for d in fa[n]})
        pb = sorted({dict(d)["program"] for d in fb[n]})
        if pa != pb:
            rep.notes.append("font program %s: %s vs %s" % (n, "/".join(pa), "/".join(pb)))
    for k in sorted(set(ga) & set(gb)):
        a, b = sorted(ga[k], key=str), sorted(gb[k], key=str)
        wa, wb = sorted({float(x[0]) for x in a}), sorted({float(x[0]) for x in b})
        if len(wa) != len(wb) or any(abs(x - y) > opt.width_tol + 1e-9 for x, y in zip(wa, wb)):
            rep.add("glyph-width", "%s %s: width %s vs %s" % (k[0], k[1], wa, wb))
        ua, ub = {x[1] for x in a}, {x[1] for x in b}
        if ua != ub:
            rep.add("tounicode", "%s %s: %r vs %r" % (k[0], k[1], sorted(ua, key=str), sorted(ub, key=str)))
        oa, ob = {x[2] for x in a} - {None}, {x[2] for x in b} - {None}
        if oa and ob and oa != ob:
            rep.add("glyph-outline", "%s %s: outline %s vs %s" % (k[0], k[1], sorted(oa), sorted(ob)))


# ---------------------------------------------------------------------------
# Paths, rules, images, forms, shadings
# ---------------------------------------------------------------------------
def normalise_segs(segs):
    """Drop moves that draw nothing (a move followed by a move, a trailing
    move), as TikZ's `re`-less rectangles leave them."""
    out = []
    for s in segs:
        if s[0] == "m" and out and out[-1][0] == "m":
            out[-1] = s
        else:
            out.append(s)
    while out and out[-1][0] == "m":
        out.pop()
    return out


def as_rule(p, tol):
    """The rectangle a path covers if it is a rule: an axis-aligned filled
    rectangle, or an axis-aligned, undashed, butt-capped stroked segment."""
    if p["clip"]:
        return None
    segs = normalise_segs(p["segs"])
    if p["op"] in ("f", "f*") and len(segs) in (5, 4):
        if segs[-1][0] == "h":
            segs = segs[:-1]
        if len(segs) != 4 or segs[0][0] != "m" or any(s[0] != "l" for s in segs[1:]):
            return None
        pts = [s[1] for s in segs]
        xs = sorted({round(x, 6) for x, _ in pts})
        ys = sorted({round(y, 6) for _, y in pts})
        for k in range(4):
            (x0, y0), (x1, y1) = pts[k], pts[(k + 1) % 4]
            if abs(x0 - x1) > 1e-6 and abs(y0 - y1) > 1e-6:
                return None
        return {"rect": (min(xs), min(ys), max(xs), max(ys)), "colour": p["fill"],
                "alpha": p.get("fill_alpha"), "blend": p.get("blend"), "smask": p.get("smask"), "src": "fill"}
    if p["op"] == "S" and len(segs) == 2 and segs[0][0] == "m" and segs[1][0] == "l" \
            and p.get("cap") == 0 and not p["dash"][0]:
        (x0, y0), (x1, y1) = segs[0][1], segs[1][1]
        h = p["lw"] / 2.0
        if abs(y0 - y1) <= 1e-6:
            r = (min(x0, x1), y0 - h, max(x0, x1), y0 + h)
        elif abs(x0 - x1) <= 1e-6:
            r = (x0 - h, min(y0, y1), x0 + h, max(y0, y1))
        else:
            return None
        return {"rect": r, "colour": p["stroke"], "alpha": p.get("stroke_alpha"),
                "blend": p.get("blend"), "smask": p.get("smask"), "src": "stroke"}
    return None


def path_points(p):
    return [pt for s in p["segs"] for pt in s[1:]]


def path_sig(p):
    return (p["op"], p["clip"], "".join(s[0] for s in p["segs"]))


def path_attrs(p):
    return (p.get("fill"), p.get("fill_alpha"), p.get("stroke"), p.get("stroke_alpha"),
            p.get("blend"), p.get("smask"), p.get("cap"), p.get("join"),
            round(p["miter"], 4) if p.get("miter") is not None else None)


def path_dist(a, b):
    pa, pb = path_points(a), path_points(b)
    if len(pa) != len(pb):
        return INF
    return max([max(abs(x - u), abs(y - v)) for (x, y), (u, v) in zip(pa, pb)] or [0.0])


def path_str(pno, p):
    pts = path_points(p)
    return "p%d %s%s %d points from (%s,%s)" % (pno, p["op"], ("+" + p["clip"]) if p["clip"] else "",
                                                len(pts), fmt(pts[0][0]) if pts else "-", fmt(pts[0][1]) if pts else "-")


def compare_paths(pno, A, B, opt, rep, stats):
    tol = opt.tol
    for lst in (A, B):
        for p in lst:
            p["segs"] = normalise_segs(p["segs"])
    ra_rules = [(i, as_rule(p, tol)) for i, p in enumerate(A)]
    rb_rules = [(j, as_rule(p, tol)) for j, p in enumerate(B)]
    RA = [r for _, r in ra_rules if r]
    RB = [r for _, r in rb_rules if r]
    PA = [A[i] for i, r in ra_rules if not r]
    PB = [B[j] for j, r in rb_rules if not r]

    # Rules: by the rectangle they cover.
    def rdist(a, b):
        return max(abs(x - y) for x, y in zip(a["rect"], b["rect"]))

    def rkey(r):
        return (r["colour"], r["alpha"], r["blend"], r["smask"])

    def rstr(r):
        return "p%d rule %s (%s)" % (pno, fmt(list(r["rect"])), colour_str(r["colour"]))

    pairs, ra, rb = match(RA, RB, rkey, lambda r: r["rect"][0], rdist, lambda r: tol)
    for i, j, d in pairs:
        stats["path_max_dev"] = max(stats.get("path_max_dev", 0.0), d)
    cpairs, ra, rb = match(RA, RB, lambda r: 0, lambda r: r["rect"][0], rdist, lambda r: tol, ra, rb)
    for i, j, d in cpairs:
        rep.add("rule-colour", "%s: candidate %s alpha %s" % (rstr(RA[i]), colour_str(RB[j]["colour"]), RB[j]["alpha"]))
    gpairs, ra, rb = match(RA, RB, rkey, lambda r: r["rect"][0], rdist, lambda r: 5.0, ra, rb)
    for i, j, d in gpairs:
        rep.add("rule-geometry", "%s: candidate %s (max deviation %s bp)" % (rstr(RA[i]), fmt(list(RB[j]["rect"])), fmt(d, 4)))
    for i in ra:
        rep.add("rule-missing", rstr(RA[i]))
    for j in rb:
        rep.add("rule-extra", rstr(RB[j]))

    # Other paths: same operator and segment kinds, points within tolerance.
    def anchor(p):
        pts = path_points(p)
        return pts[0][0] if pts else 0.0

    def full_key(p):
        return path_sig(p) + (path_attrs(p),)

    def style_ok(a, b):
        msgs = []
        for k in ("lw",):
            if k in a or k in b:
                if abs(a.get(k, 0) - b.get(k, 0)) > tol:
                    msgs.append("line width %s vs %s" % (fmt(a.get(k), 4), fmt(b.get(k), 4)))
        if "dash" in a or "dash" in b:
            da, db = a.get("dash", ((), 0)), b.get("dash", ((), 0))
            if len(da[0]) != len(db[0]) or any(abs(x - y) > tol for x, y in zip(da[0] + (da[1],), db[0] + (db[1],))):
                msgs.append("dash %s vs %s" % (fmt(list(da[0]) + [da[1]]), fmt(list(db[0]) + [db[1]])))
        return msgs

    pairs, pa, pb = match(PA, PB, full_key, anchor, path_dist, lambda p: tol)
    for i, j, d in pairs:
        stats["path_max_dev"] = max(stats.get("path_max_dev", 0.0), d)
        for m in style_ok(PA[i], PB[j]):
            rep.add("path-style", "%s: %s" % (path_str(pno, PA[i]), m))
    apairs, pa, pb = match(PA, PB, path_sig, anchor, path_dist, lambda p: tol, pa, pb)
    for i, j, d in apairs:
        a, b = PA[i], PB[j]
        names = ("fill", "fill_alpha", "stroke", "stroke_alpha", "blend", "smask", "cap", "join", "miter")
        diffs = ["%s %s vs %s" % (n, colour_str(x) if n in ("fill", "stroke") else x,
                                  colour_str(y) if n in ("fill", "stroke") else y)
                 for n, x, y in zip(names, path_attrs(a), path_attrs(b)) if x != y]
        kind = "path-colour" if any(d.startswith(("fill", "stroke")) for d in diffs) else "path-style"
        rep.add(kind, "%s: %s" % (path_str(pno, a), "; ".join(diffs)))
        for m in style_ok(a, b):
            rep.add("path-style", "%s: %s" % (path_str(pno, a), m))
    gpairs, pa, pb = match(PA, PB, path_sig, anchor, path_dist, lambda p: 5.0, pa, pb)
    for i, j, d in gpairs:
        rep.add("path-geometry", "%s: points differ by up to %s bp" % (path_str(pno, PA[i]), fmt(d, 4)))
    for i in pa:
        rep.add("path-missing", path_str(pno, PA[i]))
    for j in pb:
        rep.add("path-extra", path_str(pno, PB[j]))
    stats["paths_ref"] = stats.get("paths_ref", 0) + len(A)
    stats["paths_cand"] = stats.get("paths_cand", 0) + len(B)


def mdist(a, b):
    return max(abs(x - y) for x, y in zip(a, b))


def compare_placed(pno, kind, A, B, key, placement, describe, opt, rep):
    """Images, forms, shadings: same attributes (key) at the same placement."""
    def d(a, b):
        return mdist(placement(a), placement(b))

    pairs, ra, rb = match(A, B, key, lambda x: placement(x)[4], d, lambda x: opt.tol)
    apairs, ra, rb = match(A, B, lambda x: 0, lambda x: placement(x)[4], d, lambda x: opt.tol, ra, rb)
    for i, j, _ in apairs:
        ka, kb = key(A[i]), key(B[j])
        diffs = [("%s %s" % (n, x.diff(y))) if isinstance(x, Struct) and isinstance(y, Struct)
                 else "%s %s vs %s" % (n, x, y) for (n, x), (_, y) in zip(ka, kb) if x != y]
        rep.add(kind + "-attributes", "%s: %s" % (describe(A[i]), "; ".join(diffs)))
    gpairs, ra, rb = match(A, B, key, lambda x: 0, d, lambda x: INF, ra, rb)
    for i, j, dd in gpairs:
        rep.add(kind + "-placement", "%s: candidate at %s (max deviation %s)" % (
            describe(A[i]), fmt(list(placement(B[j]))), fmt(dd, 4)))
    for i in ra:
        rep.add(kind + "-missing", describe(A[i]))
    for j in rb:
        rep.add(kind + "-extra", describe(B[j]))


def image_key(x):
    return (("size", (x["w"], x["h"])), ("cs", x["cs"]), ("bpc", x["bpc"]), ("mask", x["mask"]),
            ("decode", json.dumps(x["decode"])), ("smask", json.dumps(x["smask"], sort_keys=True)),
            ("data", x["data"]), ("fill", x["fill"]), ("alpha", x["alpha"]))


def form_key(x):
    return (("bbox", tuple(round(v, 3) for v in x["bbox"])), ("group", x["group"]))


def form_place(x):
    from content import mul
    return mul(x["matrix"], x["ctm"])


def shading_key(x):
    return (("shading", x["shading"]["struct"]), ("alpha", x["alpha"]))


# ---------------------------------------------------------------------------
# Document level
# ---------------------------------------------------------------------------
def js(x):
    return json.dumps(x, sort_keys=True, ensure_ascii=False)


def compare_links(pno, A, B, opt, rep):
    def key(a):
        return js({k: v for k, v in a.items() if k != "rect"})

    def rd(a, b):
        if not a["rect"] or not b["rect"]:
            return 0.0 if a["rect"] == b["rect"] else INF
        return mdist(a["rect"], b["rect"])

    def anchor(a):
        return a["rect"][0] if a["rect"] else 0.0

    # Explicit destinations carry coordinates: compare those with the tolerance.
    def key_loose(a):
        return js({k: v for k, v in a.items() if k not in ("rect", "action")})

    pairs, ra, rb = match(A, B, key, anchor, rd, lambda a: opt.tol)
    lp, ra, rb = match(A, B, key_loose, anchor, rd, lambda a: opt.tol, ra, rb)
    for i, j, _ in lp:
        if not close(A[i].get("action"), B[j].get("action"), opt.tol):
            rep.add("link", "p%d %s: action %s vs %s" % (pno, fmt(A[i]["rect"]), js(A[i].get("action")), js(B[j].get("action"))))
    ap, ra, rb = match(A, B, lambda a: 0, anchor, rd, lambda a: opt.tol, ra, rb)
    for i, j, _ in ap:
        a, b = A[i], B[j]
        diffs = ["%s %s vs %s" % (k, js(a.get(k)), js(b.get(k))) for k in sorted(set(a) | set(b))
                 if k != "rect" and not close(a.get(k), b.get(k), opt.tol)]
        rep.add("link", "p%d %s: %s" % (pno, fmt(a["rect"]), "; ".join(diffs)))
    gp, ra, rb = match(A, B, key, anchor, rd, lambda a: INF, ra, rb)
    for i, j, d in gp:
        rep.add("link-rect", "p%d %s %s: candidate rect %s" % (pno, A[i]["subtype"], fmt(A[i]["rect"]), fmt(B[j]["rect"])))
    for i in ra:
        rep.add("link-missing", "p%d %s %s %s" % (pno, A[i]["subtype"], fmt(A[i]["rect"]), js(A[i].get("action"))))
    for j in rb:
        rep.add("link-extra", "p%d %s %s %s" % (pno, B[j]["subtype"], fmt(B[j]["rect"]), js(B[j].get("action"))))


def compare_docs(ma, mb, opt, rep):
    stats = rep.stats
    na, nb = len(ma.pages), len(mb.pages)
    stats["pages"] = [na, nb]
    if na != nb:
        rep.add("pages", "page count %d vs %d" % (na, nb))
    for pa, pb in zip(ma.pages, mb.pages):
        n = pa.index + 1
        if not close(pa.box, pb.box, opt.tol):
            rep.add("page-box", "p%d: %s vs %s" % (n, js(pa.box), js(pb.box)))
        ca, cb = pa.content, pb.content
        compare_glyphs(n, ca.glyphs, cb.glyphs, opt, rep, stats)
        compare_paths(n, ca.paths, cb.paths, opt, rep, stats)
        compare_placed(n, "image", ca.images, cb.images, image_key, lambda x: x["ctm"],
                       lambda x: "p%d image %sx%s %s at %s" % (n, x["w"], x["h"], x["cs"], fmt(list(x["ctm"]))), opt, rep)
        compare_placed(n, "form", ca.forms, cb.forms, form_key, form_place,
                       lambda x: "p%d form bbox %s at %s" % (n, fmt(x["bbox"]), fmt(list(form_place(x)))), opt, rep)
        compare_placed(n, "shading", ca.shadings, cb.shadings, shading_key, lambda x: x["ctm"],
                       lambda x: "p%d shading type %s at %s" % (n, x["shading"].get("type") if isinstance(x["shading"], dict) else "?",
                                                                fmt(list(x["ctm"]))), opt, rep)
        compare_links(n, pa.annots, pb.annots, opt, rep)
        for w in ca.warnings[:3]:
            stats.setdefault("warnings_ref", []).append("p%d %s" % (n, w))
        for w in cb.warnings[:3]:
            stats.setdefault("warnings_cand", []).append("p%d %s" % (n, w))
        for who, c in (("ref", ca), ("cand", cb)):
            for op, k in c.unknown.items():
                u = stats.setdefault("unknown_operators_" + who, {})
                u[op] = u.get(op, 0) + k
    compare_fonts(ma, mb, opt, rep)
    # Named destinations.
    for k in sorted(set(ma.dests) - set(mb.dests)):
        rep.add("dest-missing", "%s -> %s" % (k, js(ma.dests[k])))
    for k in sorted(set(mb.dests) - set(ma.dests)):
        rep.add("dest-extra", "%s -> %s" % (k, js(mb.dests[k])))
    for k in sorted(set(ma.dests) & set(mb.dests)):
        if not close(ma.dests[k], mb.dests[k], opt.tol):
            rep.add("dest", "%s: %s vs %s" % (k, js(ma.dests[k]), js(mb.dests[k])))
    # Outline, in order.
    oa, ob = ma.outline, mb.outline
    if len(oa) != len(ob):
        rep.add("outline", "%d entries vs %d" % (len(oa), len(ob)))
    for i, (a, b) in enumerate(zip(oa, ob)):
        for k in sorted(set(a) | set(b)):
            if not close(a.get(k), b.get(k), opt.tol):
                rep.add("outline", "entry %d (%r): %s %s vs %s" % (i + 1, a.get("title"), k, js(a.get(k)), js(b.get(k))))
    for k in sorted(set(ma.info) | set(mb.info)):
        if ma.info.get(k) != mb.info.get(k):
            rep.add("info", "%s: %r vs %r" % (k, ma.info.get(k), mb.info.get(k)))
    for k in sorted(ma.catalog):
        if not close(ma.catalog[k], mb.catalog.get(k), opt.tol):
            rep.add("catalog", "%s: %s vs %s" % (k, js(ma.catalog[k]), js(mb.catalog.get(k))))
    fw = [w for f in ma.fonts.all() for w in f.warnings] + [w for f in mb.fonts.all() for w in f.warnings]
    if fw:
        stats["font_warnings"] = sorted(set(fw))[:10]


def compare(ref, cand, opt=None, tag=None):
    """Compare two PDF files; returns a Report."""
    opt = opt or Options()
    Struct.tol = opt.tol
    rep = Report(opt.examples)
    ma, mb = extract(ref), extract(cand)
    rep.models = (ma, mb)
    compare_docs(ma, mb, opt, rep)
    if opt.visual:
        import raster
        rows = raster.compare_pdfs(ref, cand, opt.scale, opt.smooth, opt.diff_dir,
                                   tag or os.path.splitext(os.path.basename(cand))[0])
        rep.stats["visual"] = rows
        rep.stats["visual_pixels"] = sum(r["pixels"] for r in rows)
        rep.stats["visual_scale"] = opt.scale
        for r in rows:
            if r["pixels"]:
                rep.add("visual", "p%d: %d px differ at %gx, bbox %s, max channel delta %d" % (
                    r["page"], r["pixels"], opt.scale, r["bbox"], r["max_delta"]))
    return rep


def add_options(ap):
    ap.add_argument("--tol", type=float, default=0.01,
                    help="position tolerance in bp (default 0.01)")
    ap.add_argument("--rel-tol", type=float, default=0.005,
                    help="glyph origins: tolerance tol + rel_tol*size (em), default 0.005, "
                         "xdvipdfmx's measured error (0: tol only)")
    ap.add_argument("--scale", type=float, default=2.0, help="raster scale (px per bp), default 2")
    ap.add_argument("--smooth", action="store_true", help="rasterise with font smoothing on")
    ap.add_argument("--no-visual", action="store_true")
    ap.add_argument("--diff-dir", help="write diff PNGs of differing pages here")
    ap.add_argument("--examples", type=int, default=5, help="examples shown per kind")
    ap.add_argument("--width-tol", type=float, default=1.0,
                    help="glyph widths (/W, /Widths) tolerance in glyph-space units (default 1: "
                         "xdvipdfmx writes integers)")
    ap.add_argument("--glyph-identity", choices=("gid", "outline"), default="gid",
                    help="match glyphs by glyph id (default) or by outline digest")


def options_from(a):
    return Options(a.tol, a.rel_tol, a.scale, not a.no_visual, a.diff_dir, a.examples, a.glyph_identity, a.smooth,
                   a.width_tol)


def summary_line(rep):
    s = rep.stats
    return ("%d structural differences, %s; glyphs %s/%s matched, max deviation %s bp (%s em)" % (
        rep.structural, ("%d px differ" % rep.visual) if "visual" in s else "visual not run",
        s.get("glyphs_matched", 0), s.get("glyphs_ref", 0),
        fmt(s.get("glyph_max_dev", 0.0), 6), fmt(s.get("glyph_max_dev_em", 0.0), 6)))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("reference")
    ap.add_argument("candidate")
    add_options(ap)
    ap.add_argument("--json", help="write the report as JSON here ('-': stdout)")
    a = ap.parse_args(argv)
    rep = compare(a.reference, a.candidate, options_from(a))
    if a.json == "-":
        json.dump(rep.as_json(), sys.stdout, indent=1, default=str)
        print()
    else:
        print(summary_line(rep))
        t = rep.text("  ")
        if t:
            print(t)
        if a.json:
            with open(a.json, "w") as f:
                json.dump(rep.as_json(), f, indent=1, default=str)
    return 0 if rep.ok() else 1


if __name__ == "__main__":
    sys.exit(main())
