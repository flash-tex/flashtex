#!/usr/bin/env python3
"""How precisely xdvipdfmx writes positions (PLAN.md §3.5's belief to measure).

Two measurements:

1. **Digits**: for every number written as an operand of the position
   operators of the content streams (Td TD Tm cm re m l c v y w, and the
   numbers inside TJ arrays), how many digits follow the decimal point.
2. **Error against TeX**: the glyph origins of the PDF (content.py's
   interpreter) against the exact positions in the XDV the PDF was made
   from (TeX's sp, converted with the DVI preamble's num/den/mag, origin
   1in/1in from the top left of the MediaBox): the largest deviation, in bp
   and in em of the glyph's size. Native glyphs (`set_glyphs`) and TFM
   characters (`set_char`/`set`, widths from the TFM through kpsewhich, as
   DVItype scales them) are walked; glyphs are paired in page order.

    precision.py FILE.pdf [FILE.xdv] [...]       # pairs: a .pdf, then its .xdv
"""
import json
import math
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from content import _TOKEN  # noqa: E402
from model import extract  # noqa: E402
from pdfdoc import Doc, Stream  # noqa: E402

POSITION_OPS = {"Td", "TD", "Tm", "cm", "re", "m", "l", "c", "v", "y", "w", "TJ", "Tf", "Tc", "Tw", "Tz", "TL", "Ts"}


def content_streams(doc):
    """Every content stream: pages' and form XObjects'."""
    seen = set()
    for ref, p in doc.pages:
        c = doc.get(p.get("/Contents"))
        for s in (c if isinstance(c, list) else [p.get("/Contents")]):
            st = doc.stream(s)
            if st is not None and st.ref not in seen:
                seen.add(st.ref)
                yield st.data
    for key in list(doc._raw):
        if key.startswith("obj:"):
            o = doc.obj(key[4:])
            if isinstance(o, Stream) and o.get("/Subtype") == "/Form" and o.ref not in seen:
                seen.add(o.ref)
                yield o.data


def digits(pdf):
    """{operator: Counter(digits after the decimal point)}."""
    doc = Doc.open(pdf)
    out = defaultdict(Counter)
    for data in content_streams(doc):
        nums = []
        i = 0
        n = len(data)
        while i < n:
            m = _TOKEN.match(data, i)
            if m is None:
                i += 1
                continue
            i = m.end()
            k = m.lastgroup
            if k == "num":
                t = m.group()
                nums.append(len(t.split(b".", 1)[1]) if b"." in t else 0)
            elif k == "lit":
                depth = 1
                while i < n and depth:
                    c = data[i]
                    if c == 0x5C:
                        i += 2
                        continue
                    depth += (c == 0x28) - (c == 0x29)
                    i += 1
            elif k == "op":
                op = m.group().decode("latin-1")
                if op in POSITION_OPS:
                    out[op].update(nums)
                if op == "BI":
                    e = re.compile(rb"\sEI\s").search(data, i)
                    i = e.end() if e else n
                nums = []
    return out


# ---------------------------------------------------------------------------
# XDV
# ---------------------------------------------------------------------------
def tfm_widths(name, scaled):
    """A TFM font's character widths in DVI units, as DVItype computes them."""
    path = subprocess.run(["kpsewhich", name + ".tfm"], capture_output=True, text=True).stdout.strip()
    if not path:
        return {}
    with open(path, "rb") as f:
        b = f.read()
    h = [int.from_bytes(b[2 * i:2 * i + 2], "big") for i in range(12)]
    lf, lh, bc, ec, nw = h[0], h[1], h[2], h[3], h[4]
    ci = 24 + 4 * lh
    wbase = ci + 4 * (ec - bc + 1)
    z = scaled
    alpha = 16
    while z >= 0o40000000:
        z //= 2
        alpha += alpha
    beta = 256 // alpha
    alpha *= z
    widths = []
    for k in range(nw):
        b0, b1, b2, b3 = b[wbase + 4 * k:wbase + 4 * k + 4]
        w = (((b3 * z) // 256 + b2 * z) // 256 + b1 * z) // beta
        widths.append(w - alpha if b0 == 255 else w)
    out = {}
    for c in range(bc, ec + 1):
        wi = b[ci + 4 * (c - bc)]
        if wi:
            out[c] = widths[wi]
    return out


class Specials:
    """The transformations dvipdfmx's specials set up, interpreted as
    FlashTeX's writer does (crates/flashtex-xetex/src/out/special.rs and
    content.rs, #1712): a CTM M (page space = M applied to the point, plus
    the page origin 72 bp, H - 72 bp), q/Q and its own save stack, and
    `pdf:bcontent`'s coordinate stack. Points are dvipdfmx's user space: bp
    from the DVI origin, y up.

    * `pdf:code` / `pdf:direct` / `pdf:literal direct`: the operators, with
      `cm` concatenated and `q`/`Q` saving and restoring M;
    * `pdf:literal` (not direct): the same with the origin moved to the
      current point, and back;
    * `pdf:content`: the same, inside a save and restore;
    * `pdf:bcontent` / `pdf:econtent`: a save, and a translation to the
      current point relative to the coordinate stack's top, which the point
      is pushed on; a pop and restore;
    * `pdf:btrans` / `pdf:begintransform` (scale, xscale, yscale, rotate,
      matrix), `x:scale`, `x:rotate`: about the current point;
      `pdf:etrans`, `x:gsave`, `x:grestore`: save and restore."""

    def __init__(self):
        self.m = IDENT
        self.stack = []
        self.coords = []

    def top(self):
        return self.coords[-1] if self.coords else (0.0, 0.0)

    def rel(self, x, y):
        tx, ty = self.top()
        return x - tx, y - ty

    def point(self, x, y):
        """User point (x, y) to page space without the page origin."""
        return apply_m(self.m, *self.rel(x, y))

    def save(self):
        self.stack.append(self.m)

    def restore(self):
        if self.stack:
            self.m = self.stack.pop()

    def about(self, t, x, y):
        x, y = self.rel(x, y)
        self.m = mul_m(mul_m(mul_m((1, 0, 0, 1, -x, -y), t), (1, 0, 0, 1, x, y)), self.m)

    def literal(self, data):
        from content import tokens
        for ops, op in tokens(data.encode("latin-1")):
            if op == "q":
                self.save()
            elif op == "Q":
                self.restore()
            elif op == "cm" and len(ops) == 6:
                try:
                    self.m = mul_m(tuple(float(v) for v in ops), self.m)
                except (TypeError, ValueError):
                    pass

    def special(self, text, x, y):
        text = text.lstrip()
        if text.startswith("pdf:"):
            words = text[4:].split(None, 1)
            if not words:
                return
            cmd, rest = words[0], (words[1] if len(words) > 1 else "")
            if cmd in ("code", "direct"):
                self.literal(rest)
            elif cmd == "literal":
                r = rest.split(None, 1)
                if r and r[0] == "direct":
                    self.literal(r[1] if len(r) > 1 else "")
                    return
                if r and r[0] == "reverse":
                    rest = r[1] if len(r) > 1 else ""
                ux, uy = self.rel(x, y)
                self.m = mul_m((1, 0, 0, 1, ux, uy), self.m)
                self.literal(rest)
                self.m = mul_m((1, 0, 0, 1, -ux, -uy), self.m)
            elif cmd == "content":
                ux, uy = self.rel(x, y)
                self.save()
                self.m = mul_m((1, 0, 0, 1, ux, uy), self.m)
                self.literal(rest)
                self.restore()
            elif cmd == "bcontent":
                tx, ty = self.top()
                self.save()
                self.coords.append((x, y))
                self.m = mul_m((1, 0, 0, 1, x - tx, y - ty), self.m)
            elif cmd == "econtent":
                if self.coords:
                    self.coords.pop()
                self.restore()
            elif cmd in ("btrans", "begintransform"):
                t = read_transform(rest)
                self.save()
                self.about(t, x, y)
            elif cmd in ("etrans", "endtransform"):
                self.restore()
        elif text.startswith("x:"):
            words = text[2:].split()
            if not words:
                return
            if words[0] == "gsave":
                self.save()
            elif words[0] == "grestore":
                self.restore()
            elif words[0] == "scale" and len(words) >= 3:
                try:
                    self.about((float(words[1]), 0, 0, float(words[2]), 0, 0), x, y)
                except ValueError:
                    pass
            elif words[0] == "rotate" and len(words) >= 2:
                try:
                    self.about(rotation(float(words[1])), x, y)
                except ValueError:
                    pass


IDENT = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)


def mul_m(m, n):
    """m then n."""
    a, b, c, d, e, f = m
    A, B, C, D, E, F = n
    return (a * A + b * C, a * B + b * D, c * A + d * C, c * B + d * D,
            e * A + f * C + E, e * B + f * D + F)


def apply_m(m, x, y):
    return m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]


def rotation(deg):
    r = math.radians(deg)
    s, c = math.sin(r), math.cos(r)
    snap = lambda v: 0.0 if abs(v) < 1e-12 else v  # noqa: E731
    return (snap(c), snap(s), snap(-s), snap(c), 0.0, 0.0)


def read_transform(rest):
    """`pdf:btrans`'s keywords: scale, then rotate, then matrix."""
    w = rest.split()
    sc = xs = ys = 1.0
    rot = None
    mat = None
    k = 0
    try:
        while k < len(w):
            key = w[k]
            if key in ("scale", "xscale", "yscale", "rotate"):
                val = float(w[k + 1])
                if key == "scale":
                    sc = val
                elif key == "xscale":
                    xs = val
                elif key == "yscale":
                    ys = val
                else:
                    rot = val
                k += 2
            elif key == "matrix":
                mat = tuple(float(v) for v in w[k + 1:k + 7])
                k += 7
            elif key in ("width", "height", "depth", "clip", "page", "pagebox"):
                k += 2
            elif key == "bbox":
                k += 5
            else:
                k += 1
    except (IndexError, ValueError):
        pass
    m = (sc * xs, 0.0, 0.0, sc * ys, 0.0, 0.0)
    if rot is not None:
        m = mul_m(m, rotation(rot))
    if mat is not None and len(mat) == 6:
        m = mul_m(m, mat)
    return m


def xdv_glyphs(path):
    """Per page: [(x_dvi, y_dvi, glyph or char code, native?, size, px, py)]
    and conv (bp per DVI unit). (px, py) is the glyph's page position less
    the page origin (72 bp, H - 72 bp), with the transformations of the
    specials applied (Specials)."""
    with open(path, "rb") as f:
        b = f.read()
    i = 0
    num = den = mag = None
    pages = []
    fonts = {}
    f = None
    h = v = w = x = y = z = 0
    stack = []
    sp = Specials()
    conv = 1.0

    def at(hh, vv):
        return sp.point(hh * conv, -vv * conv)

    def s(n):
        nonlocal i
        val = int.from_bytes(b[i:i + n], "big", signed=True)
        i += n
        return val

    def u(n):
        nonlocal i
        val = int.from_bytes(b[i:i + n], "big")
        i += n
        return val

    while i < len(b):
        op = b[i]
        i += 1
        if op <= 127 or 128 <= op <= 131 or 133 <= op <= 136:
            if op <= 127:
                c = op
            else:
                c = u(op - 127 if op <= 131 else op - 132)
            fo = fonts.get(f)
            pages[-1].append((h, v, c, False, fo["size"] if fo else 0) + at(h, v))
            if op <= 131 and fo is not None:
                h += fo["widths"].get(c, 0)
        elif op in (132, 137):
            a_, b_ = s(4), s(4)
            if op == 132:
                h += b_
        elif op == 138:
            pass
        elif op == 139:
            i += 44
            pages.append([])
            h = v = w = x = y = z = 0
            stack = []
            sp = Specials()
        elif op == 140:
            pass
        elif op == 141:
            stack.append((h, v, w, x, y, z))
        elif op == 142:
            h, v, w, x, y, z = stack.pop()
        elif 143 <= op <= 146:
            h += s(op - 142)
        elif 147 <= op <= 151:
            if op > 147:
                w = s(op - 147)
            h += w
        elif 152 <= op <= 156:
            if op > 152:
                x = s(op - 152)
            h += x
        elif 157 <= op <= 160:
            v += s(op - 156)
        elif 161 <= op <= 165:
            if op > 161:
                y = s(op - 161)
            v += y
        elif 166 <= op <= 170:
            if op > 166:
                z = s(op - 166)
            v += z
        elif 171 <= op <= 234:
            f = op - 171
        elif 235 <= op <= 238:
            f = u(op - 234)
        elif 239 <= op <= 242:
            k = u(op - 238)  # not `i += u(...)`: that reads i before u moves it
            if pages:
                sp.special(b[i:i + k].decode("latin-1"), h * conv, -v * conv)
            i += k
        elif 243 <= op <= 246:
            k = u(op - 242)
            i += 4
            sc = u(4)
            i += 4
            a, l = b[i], b[i + 1]
            i += 2
            nm = b[i + a:i + a + l].decode("latin-1")
            i += a + l
            fonts[k] = {"widths": tfm_widths(nm, sc), "size": sc, "native": False}
        elif op == 247:
            i += 1
            num, den, mag = u(4), u(4), u(4)
            conv = num / den * mag / 1000.0 * 1e-7 / 0.0254 * 72.0
            k = b[i]
            i += 1 + k
        elif op == 248:
            break
        elif op == 252:
            k = u(4)
            size = u(4)
            flags = u(2)
            ln = b[i]
            i += 1 + ln + 4
            for bit in (0x0200, 0x1000, 0x2000, 0x4000):
                if flags & bit:
                    i += 4
            fonts[k] = {"widths": {}, "size": size, "native": True}
        elif op == 253:
            wd = s(4)
            n = u(2)
            pos = [(s(4), s(4)) for _ in range(n)]
            gl = [u(2) for _ in range(n)]
            fo = fonts.get(f)
            for (dx, dy), g in zip(pos, gl):
                pages[-1].append((h + dx, v + dy, g, True, fo["size"] if fo else 0) + at(h + dx, v + dy))
            h += wd
        else:
            raise ValueError("XDV opcode %d at %d" % (op, i - 1))
    return pages, conv


def deviation(pdf, xdv):
    """Glyph origins of the PDF against the XDV's: stats."""
    m = extract(pdf)
    pages, conv = xdv_glyphs(xdv)
    out = {"glyphs": 0, "unpaired_pages": [], "max_bp": 0.0, "max_em": 0.0, "worst": None,
           "hist_bp": Counter(), "transformed": 0, "by_program": {}}
    for pm, xp in zip(m.pages, pages):
        H = pm.box["MediaBox"][3]
        # Only glyphs drawn by the page itself (not inside included PDFs).
        gl = [g for g in pm.content.glyphs if g["where"] == "page"]
        if len(gl) != len(xp):
            out["unpaired_pages"].append((pm.index + 1, len(gl), len(xp)))
            continue
        for g, (h, v, c, native, size, _px, _py) in zip(gl, xp):
            ex = h * conv + 72.0
            ey = H - (v * conv + 72.0)
            d = max(abs(g["x"] - ex), abs(g["y"] - ey))
            em = d / (size * conv) if size else 0.0
            if d > 1.0:
                # Placed inside a transformation the XDV's specials set up
                # (TikZ nodes, rotated boxes): not a precision question.
                out["transformed"] += 1
                continue
            out["glyphs"] += 1
            kind = g["fontobj"].program_kind
            bp, bem = out["by_program"].get(kind, (0.0, 0.0))
            out["by_program"][kind] = (max(bp, d), max(bem, em))
            out["hist_bp"]["%.3f" % (round(d, 3))] += 1
            if d > out["max_bp"]:
                out["max_bp"] = d
                out["worst"] = {"page": pm.index + 1, "glyph": g["glyph"], "font": g["font"],
                                "pdf": (round(g["x"], 6), round(g["y"], 6)), "xdv": (round(ex, 6), round(ey, 6)),
                                "size_bp": round(size * conv, 4)}
            out["max_em"] = max(out["max_em"], em)
    return out


def xdv_check(model, xdv, tol=0.001, radius=1.0, examples=5):
    """A PDF's glyph origins against TeX's exact positions in the XDV of the
    same document: what proves a writer that places glyphs exactly. Glyphs
    are paired by glyph id (native fonts; TFM characters by position only)
    and nearest position within `radius` bp, not by order. Each XDV glyph's
    position has the transformations of the specials applied (Specials:
    TikZ's `pdf:bcontent` and `cm`, graphicx's `x:rotate`, `pdf:btrans`).
    Glyphs drawn inside included PDFs are not in the XDV and are left out."""
    import compare as C
    pages, conv = xdv_glyphs(xdv)
    out = {"tol": tol, "paired": 0, "unpaired_pdf": 0, "unpaired_xdv": 0, "over": 0,
           "max_bp": 0.0, "max_em": 0.0, "examples": []}
    if len(pages) != len(model.pages):
        out["examples"].append("pages: XDV %d, PDF %d" % (len(pages), len(model.pages)))
        out["over"] += 1
    for pm, xp in zip(model.pages, pages):
        H = pm.box["MediaBox"][3]
        A = []
        for g in pm.content.glyphs:
            if g["where"] != "page":
                continue
            gid = g["glyph"]
            A.append({"k": int(gid[4:]) if gid.startswith("gid:") else None, "x": g["x"], "y": g["y"],
                      "size": math.sqrt(abs(g["m"][0] * g["m"][3] - g["m"][1] * g["m"][2])), "g": g})
        B = [{"k": c if native else None, "x": px + 72.0, "y": H - 72.0 + py}
             for h, v, c, native, size, px, py in xp]
        pairs, ra, rb = C.match(A, B, lambda e: e["k"], lambda e: e["x"],
                                lambda a, b: max(abs(a["x"] - b["x"]), abs(a["y"] - b["y"])),
                                lambda e: radius)
        out["paired"] += len(pairs)
        out["unpaired_pdf"] += len(ra)
        out["unpaired_xdv"] += len(rb)
        for i, j, d in pairs:
            a, b = A[i], B[j]
            out["max_bp"] = max(out["max_bp"], d)
            if a["size"]:
                out["max_em"] = max(out["max_em"], d / a["size"])
            if d > tol + 1e-9:
                out["over"] += 1
                if len(out["examples"]) < examples:
                    out["examples"].append("p%d %s %s at (%.6f, %.6f), TeX (%.6f, %.6f): off by (%.6f, %.6f) bp" % (
                        pm.index + 1, a["g"]["font"], a["g"]["glyph"], a["x"], a["y"], b["x"], b["y"],
                        a["x"] - b["x"], a["y"] - b["y"]))
    return out


def main(argv=None):
    argv = argv or sys.argv[1:]
    total = defaultdict(Counter)
    devs = {}
    k = 0
    while k < len(argv):
        pdf = argv[k]
        xdv = argv[k + 1] if k + 1 < len(argv) and argv[k + 1].endswith(".xdv") else None
        k += 2 if xdv else 1
        for op, c in digits(pdf).items():
            total[op].update(c)
        if xdv:
            devs[os.path.basename(pdf)] = deviation(pdf, xdv)
    print("digits after the decimal point, per operator (count of operands):")
    for op in sorted(total):
        print("  %-3s %s" % (op, " ".join("%d:%d" % kv for kv in sorted(total[op].items()))))
    for name, d in devs.items():
        top = sorted(d["hist_bp"].items(), key=lambda kv: -float(kv[0]))[:3]
        print("%s: %d glyphs, max deviation from the XDV %.6f bp (%.6f em)%s; largest %s" % (
            name, d["glyphs"], d["max_bp"], d["max_em"],
            (", pages not paired %s" % d["unpaired_pages"]) if d["unpaired_pages"] else "", top))
        if d["worst"]:
            print("    worst: %s" % json.dumps(d["worst"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
