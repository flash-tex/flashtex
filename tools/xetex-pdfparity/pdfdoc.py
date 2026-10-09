"""A PDF's objects, read through qpdf's JSON (qpdf parses; this file only walks).

`qpdf --json=2 --json-stream-data=inline --decode-level=specialized` gives
every object of the file, streams decoded where qpdf can (Flate, LZW,
ASCII85/Hex, RunLength, with predictors; DCT and JPX stay encoded and keep
their /Filter). In JSON v2 a string is "u:<text>" or "b:<hex>", a name is
"/Name", a reference is "N G R"; so they never collide.

    doc = Doc.open("a.pdf")
    doc.pages        # [(ref, dict)] in page order
    doc.get(x)       # x resolved if it is a reference
    doc.stream(x)    # (dict, bytes) of a stream object (resolved)
"""
import base64
import hashlib
import json
import re
import shutil
import subprocess

QPDF = shutil.which("qpdf") or "/opt/homebrew/bin/qpdf"
_REF = re.compile(r"^\d+ \d+ R$")


class Stream:
    __slots__ = ("dict", "data", "ref")

    def __init__(self, d, data, ref):
        self.dict, self.data, self.ref = d, data, ref

    def get(self, k, default=None):
        return self.dict.get(k, default)

    def __contains__(self, k):
        return k in self.dict

    def __getitem__(self, k):
        return self.dict[k]


def is_ref(x):
    return isinstance(x, str) and _REF.match(x) is not None


def text(x):
    """A PDF string as Python text ("u:" decoded by qpdf, "b:" as Latin-1)."""
    if not isinstance(x, str):
        return x
    if x.startswith("u:"):
        return x[2:]
    if x.startswith("b:"):
        return bytes.fromhex(x[2:]).decode("latin-1")
    return x


def raw_bytes(x):
    """A PDF string's bytes (text strings re-encoded as UTF-8)."""
    if x.startswith("b:"):
        return bytes.fromhex(x[2:])
    if x.startswith("u:"):
        return x[2:].encode("utf-8")
    return x.encode("latin-1", "replace")


def name(x):
    """'/Name' -> 'Name' (None for anything that is not a name)."""
    if isinstance(x, str) and x.startswith("/"):
        return x[1:]
    if isinstance(x, str) and x.startswith("n:/"):
        return x[3:]
    return None


class Doc:
    def __init__(self, js, path=None):
        self.path = path
        q = js["qpdf"]
        self.header = q[0]
        raw = q[1]
        self.trailer = raw.get("trailer", {}).get("value", {})
        self._raw = raw
        self._cache = {}
        self.page_refs = [p["object"] for p in js.get("pages", [])]
        self.page_index = {r: i for i, r in enumerate(self.page_refs)}

    @classmethod
    def open(cls, path):
        r = subprocess.run([QPDF, "--json=2", "--json-stream-data=inline",
                            "--decode-level=specialized", path],
                           capture_output=True)
        # qpdf exits 3 for warnings (a damaged but readable file).
        if r.returncode not in (0, 3) or not r.stdout:
            raise RuntimeError("qpdf could not read %s: %s" % (
                path, r.stderr.decode("utf-8", "replace").strip()[:500]))
        return cls(json.loads(r.stdout), path)

    def obj(self, ref):
        """The object `ref` ("N G R"): a value, or a Stream."""
        if ref in self._cache:
            return self._cache[ref]
        o = self._raw.get("obj:" + ref)
        if o is None:
            v = None
        elif "stream" in o:
            s = o["stream"]
            data = base64.b64decode(s.get("data", "")) if "data" in s else b""
            v = Stream(s.get("dict", {}), data, ref)
        else:
            v = o.get("value")
        self._cache[ref] = v
        return v

    def get(self, x, depth=0):
        while is_ref(x) and depth < 64:
            x = self.obj(x)
            depth += 1
        return x

    def dget(self, d, key, default=None):
        """d[key] resolved, with d itself resolved first."""
        d = self.get(d)
        if isinstance(d, Stream):
            d = d.dict
        if not isinstance(d, dict):
            return default
        v = self.get(d.get(key))
        return default if v is None else v

    def stream(self, x):
        s = self.get(x)
        return s if isinstance(s, Stream) else None

    @property
    def catalog(self):
        return self.get(self.trailer.get("/Root")) or {}

    @property
    def info(self):
        return self.get(self.trailer.get("/Info")) or {}

    # -- pages ---------------------------------------------------------------
    @property
    def pages(self):
        return [(r, self.get(r)) for r in self.page_refs]

    def inherited(self, page, key):
        """A page attribute, inherited through /Parent (MediaBox, CropBox,
        Rotate, Resources)."""
        seen = 0
        while isinstance(page, dict) and seen < 64:
            if key in page:
                return self.get(page[key])
            page = self.get(page.get("/Parent"))
            seen += 1
        return None

    def page_of(self, x):
        """Index of the page object `x` refers to, or None."""
        return self.page_index.get(x) if isinstance(x, str) else None

    # -- trees ---------------------------------------------------------------
    def name_tree(self, root):
        """A name tree's leaves as {key text: value (unresolved)}."""
        out = {}
        stack = [root]
        n = 0
        while stack and n < 100000:
            node = self.get(stack.pop())
            n += 1
            if not isinstance(node, dict):
                continue
            names = self.get(node.get("/Names"))
            if isinstance(names, list):
                for i in range(0, len(names) - 1, 2):
                    out[text(self.get(names[i]))] = names[i + 1]
            kids = self.get(node.get("/Kids"))
            if isinstance(kids, list):
                stack.extend(reversed(kids))
        return out

    # -- canonical form ------------------------------------------------------
    def canon(self, x, skip=("/Parent", "/P", "/Length"), depth=0, seen=None, text_streams=False):
        """x with references resolved, pages as "page#i", streams as their
        dictionary plus a digest of their (decoded) data: two PDFs that say
        the same thing in different objects get the same canonical value.
        With text_streams, a content stream (a pattern's cell, a form) or a
        PostScript calculator function is its token list instead, numbers
        as numbers, so it can be compared with a tolerance."""
        if seen is None:
            seen = set()
        if depth > 40:
            return "<deep>"
        if is_ref(x):
            if x in self.page_index:
                return "page#%d" % (self.page_index[x] + 1)
            if x in seen:
                return "<cycle>"
            seen = seen | {x}
            x = self.obj(x)
        if isinstance(x, Stream):
            d = x.dict
            textual = text_streams and ("/PatternType" in d or d.get("/FunctionType") == 4
                                        or d.get("/Subtype") == "/Form")
            return {"dict": self.canon(d, skip, depth + 1, seen, text_streams),
                    "data": ps_tokens(x.data) if textual else digest(x.data)}
        if isinstance(x, dict):
            return {k: self.canon(v, skip, depth + 1, seen, text_streams)
                    for k, v in sorted(x.items()) if k not in skip}
        if isinstance(x, list):
            return [self.canon(v, skip, depth + 1, seen, text_streams) for v in x]
        if isinstance(x, float):
            return round(x, 6)
        return x


def digest(data):
    return hashlib.sha256(data).hexdigest()[:16]


def canon_digest(doc, x, **kw):
    return digest(json.dumps(doc.canon(x, **kw), sort_keys=True).encode())


_PS = re.compile(rb"[+-]?(?:\d+\.?\d*|\.\d+)(?![^\s()<>\[\]{}/%])|/[^\s()<>\[\]{}/%]*|<<|>>"
                 rb"|<[0-9A-Fa-f\s]*>|\((?:[^()\\]|\\.)*\)|[\[\]{}]|[^\s()<>\[\]{}/%]+")


def ps_tokens(data):
    """A content stream or PostScript function as tokens: numbers as numbers
    (0.0 == 0), everything else as its text; comments dropped."""
    data = re.sub(rb"%[^\r\n]*", b"", data)
    out = []
    for m in _PS.finditer(data):
        t = m.group()
        if t[:1].isdigit() or t[:1] in b"+-." and len(t) > 1 and t[1:2] not in b"-+":
            try:
                out.append(round(float(t), 6))
                continue
            except ValueError:
                pass
        out.append(t.decode("latin-1"))
    return out


def close(a, b, tol):
    """a and b equal, numbers within tol (booleans and strings exactly)."""
    num = (int, float)
    if isinstance(a, num) and isinstance(b, num) and not isinstance(a, bool) and not isinstance(b, bool):
        return abs(a - b) <= tol + 1e-9
    if isinstance(a, (list, tuple)) and isinstance(b, (list, tuple)):
        return len(a) == len(b) and all(close(x, y, tol) for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(close(a[k], b[k], tol) for k in a)
    return a == b


def first_diff(a, b, tol, path=""):
    """Where a and b first differ (close's rules), as 'path: a vs b'."""
    num = (int, float)
    if isinstance(a, num) and isinstance(b, num) and not isinstance(a, bool) and not isinstance(b, bool):
        return None if abs(a - b) <= tol + 1e-9 else "%s: %r vs %r" % (path or ".", a, b)
    if isinstance(a, (list, tuple)) and isinstance(b, (list, tuple)):
        for i, (x, y) in enumerate(zip(a, b)):
            d = first_diff(x, y, tol, "%s[%d]" % (path, i))
            if d:
                return d
        return None if len(a) == len(b) else "%s: %d items vs %d" % (path or ".", len(a), len(b))
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                return "%s%s: %s vs %s" % (path, k, "-" if k not in a else "present",
                                           "-" if k not in b else "present")
            d = first_diff(a[k], b[k], tol, path + k)
            if d:
                return d
        return None
    return None if a == b else "%s: %s vs %s" % (path or ".", str(a)[:60], str(b)[:60])


def _skeleton(v):
    if isinstance(v, bool) or v is None or isinstance(v, str):
        return v
    if isinstance(v, (int, float)):
        return "#"
    if isinstance(v, (list, tuple)):
        return [_skeleton(x) for x in v]
    if isinstance(v, dict):
        return {k: _skeleton(x) for k, x in v.items()}
    return str(v)


class Struct:
    """A canonical object (Doc.canon with text_streams) that equals another
    when close() says so, numbers within Struct.tol (compare.py sets it from
    --tol). Hashed by its shape without the numbers, so it can be a key."""
    tol = 0.01
    __slots__ = ("v", "_h")

    def __init__(self, v):
        self.v = v
        self._h = hash(json.dumps(_skeleton(v), sort_keys=True, default=str))

    def __hash__(self):
        return self._h

    def __eq__(self, other):
        return isinstance(other, Struct) and self._h == other._h and close(self.v, other.v, Struct.tol)

    def __ne__(self, other):
        return not self.__eq__(other)

    def __repr__(self):
        return "{%s}" % digest(json.dumps(self.v, sort_keys=True, default=str).encode())[:8]

    def diff(self, other):
        return first_diff(self.v, other.v, Struct.tol)


def struct(doc, x):
    return Struct(doc.canon(x, text_streams=True))
