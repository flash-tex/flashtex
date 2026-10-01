"""Re-derive glyph positions from typst-pdf's content stream, the way a PDF
viewer computes them (CTM, Tm, Tf, TJ adjustments, /W widths, all from the
numbers as written), and emit the pxdiff JSON with those positions. Glyph ids
and fonts come from the Typst dump (the k-th text object = the k-th run)."""
import json, sys
from decimal import Decimal
import pikepdf

dump = json.load(open(sys.argv[1]))
pdf = pikepdf.open(sys.argv[2])
pno = int(sys.argv[3]) - 1
out = sys.argv[4]
page = pdf.pages[pno]
mb = [float(Decimal(str(v))) for v in page.MediaBox]
H = mb[3]


def num(o):
    return float(Decimal(str(o)))


def mul(a, b):  # 3x2 affine: a then b
    return [a[0]*b[0] + a[1]*b[2], a[0]*b[1] + a[1]*b[3],
            a[2]*b[0] + a[3]*b[2], a[2]*b[1] + a[3]*b[3],
            a[4]*b[0] + a[5]*b[2] + b[4], a[4]*b[1] + a[5]*b[3] + b[5]]


widths = {}
for name, f in page.Resources.Font.items():
    desc = f.DescendantFonts[0]
    dw = num(desc.get("/DW", 1000))
    w = {}
    arr = list(desc.get("/W", []))
    i = 0
    while i < len(arr):
        c = int(arr[i])
        if isinstance(arr[i + 1], pikepdf.Array):
            for k, v in enumerate(arr[i + 1]):
                w[c + k] = num(v)
            i += 2
        else:
            for k in range(c, int(arr[i + 1]) + 1):
                w[k] = num(arr[i + 2])
            i += 3
    widths[str(name)] = (w, dw)

ctm = [1, 0, 0, 1, 0, 0]
stack = []
runs_pos = []
font = None
size = None
tm = None
for operands, op in pikepdf.parse_content_stream(page):
    op = str(op)
    if op == "q":
        stack.append(ctm)
    elif op == "Q":
        ctm = stack.pop()
    elif op == "cm":
        ctm = mul([num(x) for x in operands], ctm)
    elif op == "BT":
        tm = [1, 0, 0, 1, 0, 0]
        runs_pos.append([])
    elif op == "Tf":
        font = str(operands[0]); size = num(operands[1])
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
                    x, y = trm[4], trm[5]
                    runs_pos[-1].append((x, H - y, trm[:4]))
                    tx = w.get(cid, dw) / 1000.0 * size
                    tm = mul([1, 0, 0, 1, tx, 0], tm)
            else:
                tx = -num(it) / 1000.0 * size
                tm = mul([1, 0, 0, 1, tx, 0], tm)

runs_pos = [r for r in runs_pos if r]
assert len(runs_pos) == len(dump["runs"]), (len(runs_pos), len(dump["runs"]))
maxdx = 0.0
for r, pos in zip(dump["runs"], runs_pos):
    assert len(pos) == len(r["g"])
    m = r["m"]
    newg = []
    for g, (x, y, lin) in zip(r["g"], pos):
        ox, oy = m[4] + g[1], m[5] + g[2]
        maxdx = max(maxdx, abs(ox - x), abs(oy - y))
        if len(sys.argv) > 5 and sys.argv[5] == "sp":
            x = round(x * 65781.76) / 65781.76; y = round(y * 65781.76) / 65781.76
        newg.append([g[0], x, y])
    r["m"] = [1, 0, 0, 1, 0, 0]
    r["g"] = newg
json.dump(dump, open(out, "w"))
print(json.dumps({"runs": len(runs_pos), "glyphs": sum(len(p) for p in runs_pos), "max_abs_pos_diff_pt_typst_vs_pdf": maxdx, "mediabox_h": H}))
