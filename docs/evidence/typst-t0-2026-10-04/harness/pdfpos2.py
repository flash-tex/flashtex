"""T0 evidence (2026-10-04): glyph origins and stroked/filled shapes re-derived
from typst-pdf's content stream, in PDF (stream) space, y up, for pxdiff2.

    pdfpos2.py DUMP.json PAGE.pdf PAGENO OUT.json

Writes OUT.json = the dump, plus
  "pdf_h": MediaBox height,
  "runs_nearest": per run, [gid, X, Y] with each number read as the double
      nearest its decimal (what px/pdfpos.py does),
  "runs_cg": the same with display-list-v3 spec §4.2's viewer arithmetic
      (a number with k fraction digits = m × double(10^-k); products row by
      column, sums left to right),
  "pdf_shapes": every painted path: CTM (PDF numbers), segments, paint,
      line width/cap/join, fill/stroke grey or RGB as the PDF writes them.
Glyph ids and fonts come from the Typst dump: the k-th text object is the
k-th run (as px/pdfpos.py)."""
import json
import sys
from decimal import Decimal

import pikepdf

dump = json.load(open(sys.argv[1]))
pdf = pikepdf.open(sys.argv[2])
page = pdf.pages[int(sys.argv[3]) - 1]
out = sys.argv[4]


def near(o):
    return float(Decimal(str(o)))


def cg(o):
    """Spec §4.2: digits as an integer m times the double nearest 10^-k."""
    d = Decimal(str(o))
    sign, digits, exp = d.as_tuple()
    digits = list(digits)
    while exp < 0 and len(digits) > 1 and digits[-1] == 0:
        digits.pop()
        exp += 1
    if exp >= 0:
        return float(d)
    m = int("".join(map(str, digits)))
    v = m * float("1e-%d" % (-exp))
    return -v if sign else v


def mul(a, b):  # a then b; products row by column, sums left to right
    return [a[0] * b[0] + a[1] * b[2], a[0] * b[1] + a[1] * b[3],
            a[2] * b[0] + a[3] * b[2], a[2] * b[1] + a[3] * b[3],
            a[4] * b[0] + a[5] * b[2] + b[4], a[4] * b[1] + a[5] * b[3] + b[5]]


H = near(page.MediaBox[3])
widths = {}
for name, f in page.Resources.Font.items():
    desc = f.DescendantFonts[0]
    w = {}
    arr = list(desc.get("/W", []))
    i = 0
    while i < len(arr):
        c = int(arr[i])
        if isinstance(arr[i + 1], pikepdf.Array):
            for k, v in enumerate(arr[i + 1]):
                w[c + k] = v
            i += 2
        else:
            for k in range(c, int(arr[i + 1]) + 1):
                w[k] = arr[i + 2]
            i += 3
    widths[str(name)] = (w, desc.get("/DW", 1000))

ops = list(pikepdf.parse_content_stream(page))


def glyph_runs(num):
    ctm = [1.0, 0, 0, 1.0, 0, 0]
    stack = []
    runs = []
    font = size = tm = None
    tc, th = 0.0, 1.0
    for operands, op in ops:
        op = str(op)
        if op == "q":
            stack.append(ctm)
        elif op == "Q":
            ctm = stack.pop()
        elif op == "cm":
            ctm = mul([num(x) for x in operands], ctm)
        elif op == "BT":
            tm = [1.0, 0, 0, 1.0, 0, 0]
            runs.append([])
        elif op == "Tf":
            font = str(operands[0])
            size = num(operands[1])
        elif op == "Tc":
            tc = num(operands[0])
        elif op == "Tz":
            th = num(operands[0]) / 100
        elif op == "Tm":
            tm = [num(x) for x in operands]
        elif op in ("Tj", "TJ"):
            items = [operands[0]] if op == "Tj" else list(operands[0])
            w, dw = widths[font]
            for it in items:
                if isinstance(it, pikepdf.String):
                    b = bytes(it)
                    for k in range(0, len(b), 2):
                        cid = (b[k] << 8) | b[k + 1]
                        trm = mul(tm, ctm)
                        runs[-1].append((trm[4], trm[5]))
                        wv = near(w.get(cid, dw))  # /W: the double nearest
                        tx = ((wv / 1000) * size + tc) * th
                        tm = mul([1.0, 0, 0, 1.0, tx, 0], tm)
                else:
                    tx = ((-near(it) / 1000) * size) * th
                    tm = mul([1.0, 0, 0, 1.0, tx, 0], tm)
    return runs


def shapes(num):
    ctm = [1.0, 0, 0, 1.0, 0, 0]
    stack = []
    gs = {"w": 1.0, "J": 0, "j": 0, "fill": [0.0], "stroke": [0.0]}
    path = []
    res = []
    for operands, op in ops:
        op = str(op)
        a = [num(x) for x in operands if isinstance(x, (int, Decimal)) or type(x).__name__ in ("int", "Decimal")]
        if op == "q":
            stack.append((ctm, dict(gs)))
        elif op == "Q":
            ctm, gs = stack.pop()
        elif op == "cm":
            ctm = mul(a, ctm)
        elif op == "w":
            gs["w"] = a[0]
        elif op == "J":
            gs["J"] = int(a[0])
        elif op == "j":
            gs["j"] = int(a[0])
        elif op in ("scn", "sc", "g", "rg"):
            gs["fill"] = a
        elif op in ("SCN", "SC", "G", "RG"):
            gs["stroke"] = a
        elif op == "m":
            path.append(["m"] + a)
        elif op == "l":
            path.append(["l"] + a)
        elif op == "c":
            path.append(["c"] + a)
        elif op == "re":
            path.append(["re"] + a)
        elif op == "h":
            path.append(["z"])
        elif op in ("S", "s", "f", "F", "f*", "B", "B*", "b", "b*", "n"):
            if op != "n" and path:
                res.append({"ctm": ctm, "segs": path, "paint": op, "w": gs["w"],
                            "J": gs["J"], "j": gs["j"], "fill": gs["fill"],
                            "stroke": gs["stroke"]})
            path = []
    return res


def run_paint():
    """Per text object: the fill colour space, components and ca (spec §11.3)."""
    res = page.Resources
    spaces = {}
    for name, cs in (res.get("/ColorSpace") or {}).items():
        if isinstance(cs, pikepdf.Array) and str(cs[0]) == "/ICCBased":
            st = cs[1]
            spaces[str(name)] = {"kind": "icc", "n": int(st.N), "profile": st.read_bytes().hex()}
        elif isinstance(cs, pikepdf.Array) and str(cs[0]) == "/Separation":
            spaces[str(name)] = {"kind": "separation"}
        else:
            spaces[str(name)] = {"kind": str(cs)}
    gstates = {}
    for name, g in (res.get("/ExtGState") or {}).items():
        gstates[str(name)] = {"ca": float(g.get("/ca", 1)), "CA": float(g.get("/CA", 1)),
                              "other": sorted(str(k) for k in g.keys() if str(k) not in ("/ca", "/CA", "/Type"))}
    stack, cur, out = [], {"cs": "DeviceGray", "comps": [0.0], "ca": 1.0}, []
    for operands, op in ops:
        op = str(op)
        if op == "q":
            stack.append(dict(cur))
        elif op == "Q":
            cur = stack.pop()
        elif op == "cs":
            cur["cs"] = str(operands[0])
        elif op in ("scn", "sc"):
            cur["comps"] = [cg(x) for x in operands if not isinstance(x, pikepdf.Name)]
        elif op == "g":
            cur.update(cs="DeviceGray", comps=[cg(operands[0])])
        elif op == "rg":
            cur.update(cs="DeviceRGB", comps=[cg(x) for x in operands])
        elif op == "k":
            cur.update(cs="DeviceCMYK", comps=[cg(x) for x in operands])
        elif op == "gs":
            cur["ca"] = gstates[str(operands[0])]["ca"]
        elif op == "BT":
            out.append(dict(cur))
    return spaces, gstates, out


rn_all, rc_all = glyph_runs(near), glyph_runs(cg)
spaces, gstates, paints = run_paint()
assert len(paints) == len(rn_all)
paints = [p for p, r in zip(paints, rn_all) if r]
rn, rc = [r for r in rn_all if r], [r for r in rc_all if r]
dump["colorspaces"] = spaces
dump["extgstates"] = gstates
dump["run_paint"] = paints
assert len(rn) == len(dump["runs"]), (len(rn), len(dump["runs"]))
dump["pdf_h"] = H
dump["runs_nearest"] = [[[g[0], x, y] for g, (x, y) in zip(r["g"], p)] for r, p in zip(dump["runs"], rn)]
dump["runs_cg"] = [[[g[0], x, y] for g, (x, y) in zip(r["g"], p)] for r, p in zip(dump["runs"], rc)]
for r, p in zip(dump["runs"], rn):
    assert len(r["g"]) == len(p)
dump["pdf_shapes"] = shapes(cg)
diffs = sum(1 for a, b in zip(sum(rn, []), sum(rc, [])) if a != b)
json.dump(dump, open(out, "w"))
print(json.dumps({"runs": len(rn), "glyphs": sum(len(p) for p in rn),
                  "glyphs_nearest_ne_cg": diffs, "pdf_shapes": len(dump["pdf_shapes"]),
                  "frame_shapes": len(dump["shapes"]), "mediabox_h": H}))
