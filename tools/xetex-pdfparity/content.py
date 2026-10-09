"""A PDF content-stream tokenizer and interpreter (ISO 32000-1 §7.8, §8, §9).

`tokens(data)` yields `(operands, operator)`; `Interp(doc, fonts).run_page(i)`
runs a page (and the form XObjects it paints, recursively) and returns what
it draws, in page space (default user space, bp):

* glyphs: font (PostScript name without subset tag), glyph id, origin
  (x, y), the text rendering matrix's linear part (a, b, c, d), render mode,
  fill (and for stroking modes stroke) colour, opacity;
* paths: the painting operator, the clip operator, the segments with every
  point transformed to page space, fill/stroke colour, line width, dash,
  cap/join/miter scaled to page space;
* images (the CTM at `Do`, which maps the unit square), inline images,
  forms (CTM at `Do`, /Matrix, /BBox; their content is interpreted too),
  shadings painted with `sh`.

Colours are `(space, components)`: `DeviceGray`/`DeviceRGB`/`DeviceCMYK`, or a
named space resolved through the resources to a canonical description, or
`Pattern` with the pattern's canonical digest.
"""
import math
import re

from pdfdoc import Stream, canon_digest, digest, name, struct

WS = b"\x00\t\n\x0c\r "
_TOKEN = re.compile(rb"""
    (?P<ws>[\x00\t\n\x0c\r ]+|%[^\r\n]*)
  | (?P<num>[+-]?(?:\d+\.?\d*|\.\d+))(?![^\x00\t\n\x0c\r ()<>\[\]{}/%])
  | (?P<name>/[^\x00\t\n\x0c\r ()<>\[\]{}/%]*)
  | (?P<dopen><<) | (?P<dclose>>>)
  | (?P<hex><[0-9A-Fa-f\x00\t\n\x0c\r ]*>)
  | (?P<aopen>\[) | (?P<aclose>\])
  | (?P<lit>\()
  | (?P<op>[^\x00\t\n\x0c\r ()<>\[\]{}/%]+)
  | (?P<other>[{}<>)])
""", re.X)
_EI = re.compile(rb"[\x00\t\n\x0c\r ]EI(?=[\x00\t\n\x0c\r /\[<(%]|$)")
_ESC = {ord("n"): b"\n", ord("r"): b"\r", ord("t"): b"\t", ord("b"): b"\b",
        ord("f"): b"\f", ord("("): b"(", ord(")"): b")", ord("\\"): b"\\"}


class Name(str):
    """A PDF name (without the slash)."""


class Op(str):
    """An operator."""


def _literal(data, i):
    """A literal string starting after its '(' at i: (bytes, next index)."""
    out = bytearray()
    depth = 1
    n = len(data)
    while i < n:
        c = data[i]
        if c == 0x5C:  # backslash
            i += 1
            if i >= n:
                break
            c = data[i]
            if c in _ESC:
                out += _ESC[c]
                i += 1
            elif 0x30 <= c <= 0x37:
                j = i
                while j < n and j < i + 3 and 0x30 <= data[j] <= 0x37:
                    j += 1
                out.append(int(data[i:j], 8) & 0xFF)
                i = j
            elif c == 0x0D:
                i += 2 if i + 1 < n and data[i + 1] == 0x0A else 1
            elif c == 0x0A:
                i += 1
            else:
                out.append(c)
                i += 1
            continue
        if c == 0x28:
            depth += 1
        elif c == 0x29:
            depth -= 1
            if depth == 0:
                return bytes(out), i + 1
        out.append(c)
        i += 1
    return bytes(out), n


def _name(raw):
    s = raw[1:]
    if b"#" in s:
        s = re.sub(rb"#([0-9A-Fa-f]{2})", lambda m: bytes([int(m.group(1), 16)]), s)
    return Name(s.decode("latin-1"))


def tokens(data):
    """(operands, operator) for each operator of a content stream. Inline
    images come as ([dict, data], Op('BI'))."""
    stack = [[]]  # operand list, then open arrays/dicts
    kinds = []  # "a" or "d" for each open container
    i = 0
    n = len(data)
    while i < n:
        m = _TOKEN.match(data, i)
        if m is None:
            i += 1
            continue
        i = m.end()
        k = m.lastgroup
        if k == "ws" or k == "other":
            continue
        if k == "num":
            t = m.group()
            v = float(t) if (b"." in t) else int(t)
            stack[-1].append(v)
        elif k == "name":
            stack[-1].append(_name(m.group()))
        elif k == "hex":
            h = re.sub(rb"[^0-9A-Fa-f]", b"", m.group()[1:-1])
            if len(h) % 2:
                h += b"0"
            stack[-1].append(bytes.fromhex(h.decode()))
        elif k == "lit":
            s, i = _literal(data, i)
            stack[-1].append(s)
        elif k == "aopen" or k == "dopen":
            stack.append([])
            kinds.append("a" if k == "aopen" else "d")
        elif k == "aclose" or k == "dclose":
            if len(stack) > 1:
                items = stack.pop()
                kind = kinds.pop()
                if kind == "d":
                    items = {items[j]: items[j + 1] for j in range(0, len(items) - 1, 2)}
                stack[-1].append(items)
        elif k == "op":
            word = m.group().decode("latin-1")
            if word in ("true", "false"):
                stack[-1].append(word == "true")
                continue
            if word == "null":
                stack[-1].append(None)
                continue
            if len(stack) > 1:  # an operator inside an array: malformed, drop
                continue
            if word == "BI":
                d = {}
                vals = []
                while i < n:
                    m2 = _TOKEN.match(data, i)
                    if m2 is None:
                        i += 1
                        continue
                    if m2.lastgroup == "ws":
                        i = m2.end()
                        continue
                    if m2.lastgroup == "op" and m2.group() == b"ID":
                        i = m2.end()
                        break
                    # parse one object with the same machinery
                    sub, i = _one_object(data, i)
                    if sub is not _NOTHING:
                        vals.append(sub)
                for j in range(0, len(vals) - 1, 2):
                    d[vals[j]] = vals[j + 1]
                if i < n and data[i] in WS:
                    i += 1
                e = _EI.search(data, i)
                end = e.start() if e else n
                img = data[i:end]
                i = e.end() if e else n
                yield [d, img], Op("BI")
                stack = [[]]
                kinds = []
                continue
            yield stack[0], Op(word)
            stack = [[]]
            kinds = []


_NOTHING = object()


def _one_object(data, i):
    """One object at i (for inline image dictionaries)."""
    stack = [[]]
    kinds = []
    n = len(data)
    while i < n:
        m = _TOKEN.match(data, i)
        if m is None:
            i += 1
            continue
        i = m.end()
        k = m.lastgroup
        if k == "ws" or k == "other":
            continue
        if k == "num":
            t = m.group()
            stack[-1].append(float(t) if b"." in t else int(t))
        elif k == "name":
            stack[-1].append(_name(m.group()))
        elif k == "hex":
            h = re.sub(rb"[^0-9A-Fa-f]", b"", m.group()[1:-1])
            stack[-1].append(bytes.fromhex((h + b"0" * (len(h) % 2)).decode()))
        elif k == "lit":
            s, i = _literal(data, i)
            stack[-1].append(s)
        elif k in ("aopen", "dopen"):
            stack.append([])
            kinds.append(k)
            continue
        elif k in ("aclose", "dclose"):
            items = stack.pop()
            kind = kinds.pop() if kinds else "aopen"
            if kind == "dopen":
                items = {items[j]: items[j + 1] for j in range(0, len(items) - 1, 2)}
            stack[-1].append(items)
        elif k == "op":
            w = m.group()
            stack[-1].append({b"true": True, b"false": False, b"null": None}.get(w, Name(w.decode("latin-1"))))
        if len(stack) == 1 and stack[0]:
            return stack[0][0], i
    return _NOTHING, i


# ---------------------------------------------------------------------------
# Matrices: [a b c d e f], row vectors (x' = a x + c y + e)
# ---------------------------------------------------------------------------
IDENTITY = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)


def mul(m, n):
    """m then n (m × n in PDF's notation)."""
    a, b, c, d, e, f = m
    A, B, C, D, E, F = n
    return (a * A + b * C, a * B + b * D, c * A + d * C, c * B + d * D,
            e * A + f * C + E, e * B + f * D + F)


def apply(m, x, y):
    return m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]


def scale_of(m):
    return math.sqrt(abs(m[0] * m[3] - m[1] * m[2]))


class GState:
    __slots__ = ("ctm", "fill_cs", "fill", "stroke_cs", "stroke", "lw", "dash",
                 "cap", "join", "miter", "fill_alpha", "stroke_alpha", "blend",
                 "smask", "font", "size", "tc", "tw", "th", "tl", "tr", "ts")

    def __init__(self):
        self.ctm = IDENTITY
        self.fill_cs = self.stroke_cs = "DeviceGray"
        self.fill = self.stroke = ("DeviceGray", (0.0,))
        self.lw = 1.0
        self.dash = ((), 0)
        self.cap = self.join = 0
        self.miter = 10.0
        self.fill_alpha = self.stroke_alpha = 1.0
        self.blend = "Normal"
        self.smask = None
        self.font = None
        self.size = 0.0
        self.tc = self.tw = 0.0
        self.th = 1.0
        self.tl = self.ts = 0.0
        self.tr = 0

    def copy(self):
        g = GState.__new__(GState)
        for s in GState.__slots__:
            setattr(g, s, getattr(self, s))
        return g


_DEFAULT_COMPONENTS = {"DeviceGray": (0.0,), "DeviceRGB": (0.0, 0.0, 0.0),
                       "DeviceCMYK": (0.0, 0.0, 0.0, 1.0)}


def _r(v, nd=4):
    return round(float(v), nd) + 0.0


class Page:
    """What a page draws."""

    def __init__(self):
        self.glyphs = []
        self.paths = []
        self.images = []
        self.forms = []
        self.shadings = []
        self.unknown = {}
        self.warnings = []


class Interp:
    MAX_DEPTH = 12

    def __init__(self, doc, fonts):
        self.doc = doc
        self.fonts = fonts
        self._cs_cache = {}

    def run_page(self, index):
        ref, page = self.doc.pages[index]
        out = Page()
        res = self.doc.inherited(page, "/Resources") or {}
        contents = self.doc.get(page.get("/Contents"))
        if isinstance(contents, list):
            data = b"\n".join((self.doc.stream(c).data if self.doc.stream(c) else b"") for c in contents)
        elif isinstance(contents, Stream):
            data = contents.data
        else:
            data = b""
        self.run(data, res, GState(), out, 0, "page")
        return out

    def run_stream(self, data, resources=None, ctm=IDENTITY):
        """Interpret bare content (tests)."""
        out = Page()
        g = GState()
        g.ctm = ctm
        self.run(data, resources or {}, g, out, 0, "page")
        return out

    # -- resources -------------------------------------------------------------
    def _res(self, res, cat, key):
        d = self.doc.get(res.get("/" + cat)) if isinstance(res, dict) else None
        if not isinstance(d, dict):
            return None, None
        ref = d.get("/" + key)
        return ref, self.doc.get(ref)

    def _cs(self, res, nm):
        """A colour space operand -> (description, number of components)."""
        if nm in ("DeviceGray", "G"):
            return "DeviceGray", 1
        if nm in ("DeviceRGB", "RGB"):
            return "DeviceRGB", 3
        if nm in ("DeviceCMYK", "CMYK"):
            return "DeviceCMYK", 4
        if nm == "Pattern":
            return "Pattern", 0
        ref, obj = self._res(res, "ColorSpace", nm)
        if obj is None:
            return "Unknown:" + str(nm), 0
        return self.cs_desc(obj)

    def cs_desc(self, obj):
        """A colour space object -> (canonical description, components)."""
        doc = self.doc
        obj = doc.get(obj)
        if name(obj):
            return self._cs({}, name(obj))
        if not isinstance(obj, list) or not obj:
            return "Unknown", 0
        fam = name(doc.get(obj[0]))
        if fam == "ICCBased":
            s = doc.stream(obj[1])
            n = doc.get(s.get("/N")) if s else 0
            return "ICCBased(%s,%s)" % (n, digest(s.data) if s else "?"), n or 0
        if fam in ("CalRGB", "CalGray", "Lab"):
            return (fam, struct(doc, obj[1])), {"CalRGB": 3, "CalGray": 1, "Lab": 3}[fam]
        if fam == "Separation":
            alt = self.cs_desc(obj[2])[0]
            return ("Separation", name(doc.get(obj[1])), alt, struct(doc, obj[3])), 1
        if fam == "DeviceN":
            names = [name(doc.get(x)) for x in doc.get(obj[1])]
            alt = self.cs_desc(obj[2])[0]
            return ("DeviceN", tuple(names), alt, struct(doc, obj[3])), len(names)
        if fam == "Indexed":
            base = self.cs_desc(obj[1])[0]
            lk = doc.get(obj[3])
            lkd = digest(lk.data) if isinstance(lk, Stream) else digest(str(lk).encode())
            return "Indexed(%s,%s,%s)" % (base, doc.get(obj[2]), lkd), 1
        if fam == "Pattern":
            return "Pattern", 0
        return "Unknown:" + str(fam), 0

    def _image_cs(self, cs, res):
        if cs is None:
            return None
        if isinstance(cs, Name):
            return self._cs(res, str(cs))[0]
        if isinstance(cs, str) and name(cs):
            return self._cs(res, name(cs))[0]
        return self.cs_desc(cs)[0]

    # -- the interpreter -------------------------------------------------------
    def run(self, data, res, g, out, depth, where):
        doc = self.doc
        stack = []
        path = []  # segments in user space, transformed at paint time
        clip = None
        cur = start = None
        tm = tlm = IDENTITY
        font = None
        for ops, op in tokens(data):
            try:
                if op in ("m", "l", "c", "v", "y", "re", "h"):
                    if op == "m":
                        cur = start = (ops[0], ops[1])
                        path.append(("m", g.ctm, cur))
                    elif op == "l":
                        cur = (ops[0], ops[1])
                        path.append(("l", g.ctm, cur))
                    elif op == "c":
                        path.append(("c", g.ctm, (ops[0], ops[1]), (ops[2], ops[3]), (ops[4], ops[5])))
                        cur = (ops[4], ops[5])
                    elif op == "v":
                        path.append(("c", g.ctm, cur, (ops[0], ops[1]), (ops[2], ops[3])))
                        cur = (ops[2], ops[3])
                    elif op == "y":
                        path.append(("c", g.ctm, (ops[0], ops[1]), (ops[2], ops[3]), (ops[2], ops[3])))
                        cur = (ops[2], ops[3])
                    elif op == "re":
                        x, y, w, h = ops[:4]
                        path += [("m", g.ctm, (x, y)), ("l", g.ctm, (x + w, y)),
                                 ("l", g.ctm, (x + w, y + h)), ("l", g.ctm, (x, y + h)), ("h",)]
                        cur = start = (x, y)
                    else:
                        path.append(("h",))
                        cur = start
                elif op in ("S", "s", "f", "F", "f*", "B", "B*", "b", "b*", "n"):
                    if op in ("s", "b", "b*"):
                        path.append(("h",))
                    paint = {"s": "S", "b": "B", "b*": "B*", "F": "f"}.get(op, op)
                    if path:
                        self._path(out, g, path, paint, clip, where)
                    path = []
                    clip = None
                elif op in ("W", "W*"):
                    clip = op
                elif op == "q":
                    stack.append(g)
                    g = g.copy()
                elif op == "Q":
                    if stack:
                        g = stack.pop()
                elif op == "cm":
                    g.ctm = mul(tuple(float(x) for x in ops[:6]), g.ctm)
                elif op == "w":
                    g.lw = float(ops[0])
                elif op == "J":
                    g.cap = int(ops[0])
                elif op == "j":
                    g.join = int(ops[0])
                elif op == "M":
                    g.miter = float(ops[0])
                elif op == "d":
                    g.dash = (tuple(float(x) for x in ops[0]), float(ops[1]))
                elif op in ("g", "G", "rg", "RG", "k", "K"):
                    sp = {"g": "DeviceGray", "rg": "DeviceRGB", "k": "DeviceCMYK"}[op.lower()]
                    col = (sp, tuple(_r(x) for x in ops))
                    if op.islower():
                        g.fill_cs, g.fill = sp, col
                    else:
                        g.stroke_cs, g.stroke = sp, col
                elif op in ("cs", "CS"):
                    desc, n = self._cs(res, str(ops[0]))
                    comps = _DEFAULT_COMPONENTS.get(desc, (0.0,) * n if n else ())
                    if op == "cs":
                        g.fill_cs, g.fill = desc, (desc, comps)
                    else:
                        g.stroke_cs, g.stroke = desc, (desc, comps)
                elif op in ("sc", "scn", "SC", "SCN"):
                    sp = g.fill_cs if op.islower() else g.stroke_cs
                    if ops and isinstance(ops[-1], Name):
                        ref, pat = self._res(res, "Pattern", str(ops[-1]))
                        col = ("Pattern", (struct(doc, ref),) + tuple(_r(x) for x in ops[:-1]))
                    else:
                        col = (sp, tuple(_r(x) for x in ops))
                    if op.islower():
                        g.fill = col
                    else:
                        g.stroke = col
                elif op == "gs":
                    ref, gsd = self._res(res, "ExtGState", str(ops[0]))
                    if isinstance(gsd, dict):
                        self._extgstate(g, gsd)
                elif op == "BT":
                    tm = tlm = IDENTITY
                elif op == "ET":
                    pass
                elif op == "Tf":
                    ref, fd = self._res(res, "Font", str(ops[0]))
                    font = self.fonts.font(ref) if ref is not None else None
                    if font is None:
                        out.warnings.append("font %s not in resources" % ops[0])
                    g.font = font
                    g.size = float(ops[1])
                elif op == "Tc":
                    g.tc = float(ops[0])
                elif op == "Tw":
                    g.tw = float(ops[0])
                elif op == "Tz":
                    g.th = float(ops[0]) / 100.0
                elif op == "TL":
                    g.tl = float(ops[0])
                elif op == "Tr":
                    g.tr = int(ops[0])
                elif op == "Ts":
                    g.ts = float(ops[0])
                elif op == "Td":
                    tm = tlm = mul((1, 0, 0, 1, float(ops[0]), float(ops[1])), tlm)
                elif op == "TD":
                    g.tl = -float(ops[1])
                    tm = tlm = mul((1, 0, 0, 1, float(ops[0]), float(ops[1])), tlm)
                elif op == "Tm":
                    tm = tlm = tuple(float(x) for x in ops[:6])
                elif op == "T*":
                    tm = tlm = mul((1, 0, 0, 1, 0, -g.tl), tlm)
                elif op in ("Tj", "TJ", "'", '"'):
                    if op == "'":
                        tm = tlm = mul((1, 0, 0, 1, 0, -g.tl), tlm)
                        items = [ops[0]]
                    elif op == '"':
                        g.tw, g.tc = float(ops[0]), float(ops[1])
                        tm = tlm = mul((1, 0, 0, 1, 0, -g.tl), tlm)
                        items = [ops[2]]
                    elif op == "Tj":
                        items = [ops[0]]
                    else:
                        items = ops[0]
                    tm = self._show(out, g, tm, items, where)
                elif op == "Do":
                    self._do(out, g, res, str(ops[0]), depth, where)
                elif op == "BI":
                    self._inline_image(out, g, res, ops[0], ops[1], where)
                elif op == "sh":
                    ref, sh = self._res(res, "Shading", str(ops[0]))
                    out.shadings.append({"ctm": g.ctm, "shading": self._shading_desc(ref),
                                         "alpha": g.fill_alpha, "where": where})
                elif op in ("BMC", "BDC", "EMC", "MP", "DP", "BX", "EX", "ri", "i", "d0", "d1"):
                    pass
                else:
                    out.unknown[op] = out.unknown.get(op, 0) + 1
            except (IndexError, TypeError, ValueError) as e:
                out.warnings.append("%s: bad operands %r (%s)" % (op, ops[:6], e))

    def _extgstate(self, g, d):
        doc = self.doc
        for k, v in d.items():
            v = doc.get(v)
            if k == "/LW":
                g.lw = float(v)
            elif k == "/LC":
                g.cap = int(v)
            elif k == "/LJ":
                g.join = int(v)
            elif k == "/ML":
                g.miter = float(v)
            elif k == "/D":
                g.dash = (tuple(float(doc.get(x)) for x in doc.get(v[0])), float(doc.get(v[1])))
            elif k == "/CA":
                g.stroke_alpha = _r(v)
            elif k == "/ca":
                g.fill_alpha = _r(v)
            elif k == "/BM":
                g.blend = name(v) if name(v) else str(v)
            elif k == "/SMask":
                g.smask = None if name(v) == "None" else struct(doc, d.get(k))

    def _shading_desc(self, ref):
        doc = self.doc
        sh = doc.get(ref)
        d = sh.dict if isinstance(sh, Stream) else sh
        if not isinstance(d, dict):
            return "missing"
        # The whole dictionary (function, coords, extend, colour space, ...)
        # with numbers as numbers: compared within --tol.
        return {"type": doc.get(d.get("/ShadingType")), "struct": struct(doc, ref)}

    def _path(self, out, g, path, paint, clip, where):
        segs = []
        for s in path:
            if s[0] == "h":
                segs.append(("h",))
            else:
                m = s[1]
                segs.append((s[0],) + tuple(apply(m, float(p[0]), float(p[1])) for p in s[2:]))
        # The CTM of the paint applies to line width and dashes.
        sc = scale_of(g.ctm)
        e = {"op": paint, "clip": clip, "segs": segs, "where": where}
        if paint in ("f", "f*", "B", "B*"):
            e["fill"] = g.fill
            e["fill_alpha"] = g.fill_alpha
        if paint in ("S", "B", "B*"):
            e["stroke"] = g.stroke
            e["stroke_alpha"] = g.stroke_alpha
            e["lw"] = g.lw * sc
            e["dash"] = (tuple(x * sc for x in g.dash[0]), g.dash[1] * sc)
            e["cap"] = g.cap
            e["join"] = g.join
            e["miter"] = g.miter if g.join == 0 else None
        if paint != "n":
            e["blend"] = g.blend
            e["smask"] = g.smask
        out.paths.append(e)

    def _show(self, out, g, tm, items, where):
        f = g.font
        if f is None:
            return tm
        th = g.th
        fm = f.font_matrix
        for it in items:
            if isinstance(it, (int, float)):
                tm = mul((1, 0, 0, 1, -float(it) / 1000.0 * g.size * th, 0), tm)
                continue
            if not isinstance(it, bytes):
                continue
            for code in f.split(it):
                gl = f.glyph(code)
                trm = mul((g.size * th, 0, 0, g.size, 0, g.ts), mul(tm, g.ctm))
                if fm:
                    trm = mul(tuple(float(x) for x in fm), trm)
                e = {"font": f.name, "glyph": gl.id, "x": trm[4], "y": trm[5],
                     "m": trm[:4], "mode": g.tr, "code": code, "outline": gl.outline,
                     "fontobj": f, "where": where}
                if g.tr in (0, 2, 4, 6):
                    e["fill"] = g.fill
                    e["fill_alpha"] = g.fill_alpha
                if g.tr in (1, 2, 5, 6):
                    e["stroke"] = g.stroke
                    e["stroke_alpha"] = g.stroke_alpha
                    e["lw"] = g.lw * scale_of(g.ctm)
                out.glyphs.append(e)
                w0 = float(gl.width) * (float(fm[0]) if fm else 0.001)
                tx = (w0 * g.size + g.tc + (g.tw if (code == 32 and f.code_bytes == {1}) else 0.0)) * th
                tm = mul((1, 0, 0, 1, tx, 0), tm)
        return tm

    def _image_entry(self, g, d, data, res, where, filt=None):
        doc = self.doc
        gd = lambda k, alt=None: doc.get(d.get(k, d.get(alt) if alt else None))  # noqa: E731
        mask = gd("/ImageMask", "/IM")
        smask = d.get("/SMask")
        sm = doc.stream(smask)
        return {"ctm": g.ctm, "w": gd("/Width", "/W"), "h": gd("/Height", "/H"),
                "cs": None if mask else self._image_cs(gd("/ColorSpace", "/CS"), res),
                "bpc": gd("/BitsPerComponent", "/BPC"), "mask": bool(mask),
                "decode": doc.canon(gd("/Decode", "/D")),
                "smask": None if sm is None else {"w": doc.get(sm.get("/Width")), "h": doc.get(sm.get("/Height")),
                                                  "data": digest(sm.data)},
                "filter": filt, "data": digest(data),
                "fill": g.fill if mask else None, "alpha": g.fill_alpha, "where": where}

    def _do(self, out, g, res, nm, depth, where):
        doc = self.doc
        ref, x = self._res(res, "XObject", nm)
        if not isinstance(x, Stream):
            out.warnings.append("XObject %s missing" % nm)
            return
        sub = name(doc.get(x.get("/Subtype")))
        if sub == "Image":
            filt = doc.canon(x.get("/Filter"))
            out.images.append(self._image_entry(g, x.dict, x.data, res, where, filt))
        elif sub == "Form":
            if depth >= self.MAX_DEPTH:
                out.warnings.append("forms nested deeper than %d" % self.MAX_DEPTH)
                return
            matrix = doc.get(x.get("/Matrix"))
            matrix = tuple(float(doc.get(v)) for v in matrix) if isinstance(matrix, list) else IDENTITY
            bbox = [float(doc.get(v)) for v in (doc.get(x.get("/BBox")) or [])]
            fid = len(out.forms)
            out.forms.append({"ctm": g.ctm, "matrix": matrix, "bbox": bbox,
                              "group": struct(doc, x.get("/Group")), "where": where,
                              "id": "%s/form%d" % (where, fid)})
            g2 = g.copy()
            g2.ctm = mul(matrix, g.ctm)
            fres = doc.get(x.get("/Resources"))
            if not isinstance(fres, dict):
                fres = res
            self.run(x.data, fres, g2, out, depth + 1, "%s/form%d" % (where, fid))
        else:
            out.warnings.append("XObject %s of subtype %s ignored" % (nm, sub))

    def _inline_image(self, out, g, res, d, data, where):
        abbrev = {"BPC": "/BitsPerComponent", "CS": "/ColorSpace", "D": "/Decode", "DP": "/DecodeParms",
                  "F": "/Filter", "H": "/Height", "IM": "/ImageMask", "I": "/Interpolate", "W": "/Width"}
        dd = {}
        for k, v in d.items():
            kk = abbrev.get(str(k), "/" + str(k))
            if isinstance(v, Name):
                v = "/" + {"G": "DeviceGray", "RGB": "DeviceRGB", "CMYK": "DeviceCMYK"}.get(str(v), str(v))
            dd[kk] = v
        cs = dd.get("/ColorSpace")
        if isinstance(cs, str) and cs.startswith("/") and cs[1:] not in ("DeviceGray", "DeviceRGB", "DeviceCMYK"):
            dd["/ColorSpace"] = Name(cs[1:])
        e = self._image_entry(g, dd, data, res, where, str(dd.get("/Filter")))
        e["inline"] = True
        out.images.append(e)
