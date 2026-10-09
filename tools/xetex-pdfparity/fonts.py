"""Fonts of a PDF: from a content-stream code to the glyph it shows.

What identifies a glyph (docs/design/xetex/PLAN.md §3.5: "font by
PostScript name and face, glyph id"):

* **The font**: the PostScript name (`/BaseFont`, the descendant's for a
  Type0 font) without its six-letter subset tag. In a TrueType collection
  the PostScript name tells the faces apart, so it names the face too.
* **Type0 fonts** (how xdvipdfmx writes XeTeX's native fonts: `/Identity-H`,
  code = CID = the glyph id in the original font file):
  - `CIDFontType2` (TrueType): the glyph is `/CIDToGIDMap` applied to the
    CID (`/Identity`: the CID itself; a stream: the two-byte entry), i.e.
    the glyph id in the embedded program. xdvipdfmx keeps the original
    numbering in its TrueType subsets (VERIFIED, README), so this is the
    original glyph id there; a writer that renumbers its subset and maps
    the CIDs back through a `/CIDToGIDMap` stream to glyphs it kept in place
    compares equal.
  - `CIDFontType0` with a CID-keyed CFF: the glyph is the **CID**. The
    CFF's charset maps the subset's compact glyph indices to CIDs (measured:
    xdvipdfmx's charsets are not the identity), so the CID, not the index in
    the subset, is the stable name of the glyph; the charset is used to
    find the glyph program for the outline digest. A name-keyed CFF: the
    code indexes the CFF's glyphs.
* **Simple fonts** (Type1 from TFM fonts, which xdvipdfmx converts to CFF
  `/FontFile3 /Type1C`; Type1 `/FontFile` in PDFs made by pdfTeX and
  included as images): the glyph **name**, from `/Encoding` (a base
  encoding and `/Differences`), else the embedded program's built-in
  encoding; else the code.

Besides the identity, each glyph has an **outline digest** (the drawn
outline, decomposed, rounded to 1/100 font unit, from the embedded
program), so a renumbered subset can be told from a wrong glyph, and a
**width** (what the PDF says the glyph advances, from `/W` or `/Widths`,
in 1/1000 text space) and its **ToUnicode** text.
"""
import hashlib
import io
import logging
import re

from pdfdoc import Stream, name, raw_bytes

TAG = re.compile(r"^[A-Z]{6}\+")
# Subset fonts carry odd 'head' dates; fontTools warns about each.
logging.getLogger("fontTools").setLevel(logging.ERROR)


def strip_tag(n):
    return TAG.sub("", n or "")


# ---------------------------------------------------------------------------
# Encodings
# ---------------------------------------------------------------------------
def _standard():
    from fontTools.encodings.StandardEncoding import StandardEncoding
    return list(StandardEncoding)


def _macroman():
    from fontTools.encodings.MacRoman import MacRoman
    return list(MacRoman)


def _winansi():
    from fontTools.agl import UV2AGL
    out = []
    for b in range(256):
        try:
            ch = bytes([b]).decode("cp1252")
        except UnicodeDecodeError:
            out.append(".notdef")
            continue
        out.append(UV2AGL.get(ord(ch), ".notdef") if b >= 32 else ".notdef")
    # PDF's WinAnsiEncoding (Annex D): bullet for the unused codes above 127
    # is not needed for identity; the space and hyphen names are AGL's.
    return out


_ENCODINGS = {}


def base_encoding(nm):
    if nm not in _ENCODINGS:
        _ENCODINGS[nm] = {"StandardEncoding": _standard, "MacRomanEncoding": _macroman,
                          "WinAnsiEncoding": _winansi}.get(nm, lambda: None)()
    return _ENCODINGS[nm]


# ---------------------------------------------------------------------------
# CMaps
# ---------------------------------------------------------------------------
_HEX = re.compile(rb"<([0-9A-Fa-f\s]*)>")


def _hex(b):
    return bytes.fromhex(re.sub(rb"\s", b"", b).decode())


def parse_tounicode(data):
    """A ToUnicode CMap: {code (int): text}, and the codespace byte widths."""
    out = {}
    spaces = set()
    for m in re.finditer(rb"begincodespacerange(.*?)endcodespacerange", data, re.S):
        for lo, _hi in zip(*[iter(_HEX.findall(m.group(1)))] * 2):
            spaces.add(len(_hex(lo)))
    for m in re.finditer(rb"beginbfchar(.*?)endbfchar", data, re.S):
        toks = _HEX.findall(m.group(1))
        for src, dst in zip(toks[0::2], toks[1::2]):
            out[int.from_bytes(_hex(src), "big")] = _hex(dst).decode("utf-16-be", "replace")
    for m in re.finditer(rb"beginbfrange(.*?)endbfrange", data, re.S):
        body = m.group(1)
        for r in re.finditer(rb"<([0-9A-Fa-f\s]*)>\s*<([0-9A-Fa-f\s]*)>\s*(<[0-9A-Fa-f\s]*>|\[[^\]]*\])", body):
            lo = int.from_bytes(_hex(r.group(1)), "big")
            hi = int.from_bytes(_hex(r.group(2)), "big")
            dst = r.group(3)
            if hi - lo > 65535:
                continue
            if dst.startswith(b"["):
                for i, d in enumerate(_HEX.findall(dst)):
                    out[lo + i] = _hex(d).decode("utf-16-be", "replace")
            else:
                d = bytearray(_hex(dst[1:-1]))
                for c in range(lo, hi + 1):
                    out[c] = bytes(d).decode("utf-16-be", "replace")
                    if d:
                        d[-1] = (d[-1] + 1) & 0xFF  # the last byte increments
    return out, spaces


def parse_cmap_cids(data):
    """An embedded (non-Identity) CMap: code -> CID, plus codespace widths."""
    out = {}
    spaces = set()
    for m in re.finditer(rb"begincodespacerange(.*?)endcodespacerange", data, re.S):
        for lo, _hi in zip(*[iter(_HEX.findall(m.group(1)))] * 2):
            spaces.add(len(_hex(lo)))
    for m in re.finditer(rb"begincidrange(.*?)endcidrange", data, re.S):
        for r in re.finditer(rb"<([0-9A-Fa-f\s]*)>\s*<([0-9A-Fa-f\s]*)>\s*(\d+)", m.group(1)):
            lo = int.from_bytes(_hex(r.group(1)), "big")
            hi = int.from_bytes(_hex(r.group(2)), "big")
            for c in range(lo, min(hi, lo + 65535) + 1):
                out[c] = int(r.group(3)) + c - lo
    for m in re.finditer(rb"begincidchar(.*?)endcidchar", data, re.S):
        for r in re.finditer(rb"<([0-9A-Fa-f\s]*)>\s*(\d+)", m.group(1)):
            out[int.from_bytes(_hex(r.group(1)), "big")] = int(r.group(2))
    return out, spaces


# ---------------------------------------------------------------------------
# Embedded programs
# ---------------------------------------------------------------------------
def _outline_digest(draw):
    from fontTools.pens.recordingPen import RecordingPen
    pen = RecordingPen()
    try:
        draw(pen)
    except Exception as e:  # a broken program: say so rather than guess
        return "error:" + type(e).__name__
    parts = []
    for op, args in pen.value:
        parts.append(op + ":" + ",".join("%.2f,%.2f" % tuple(p) if isinstance(p, tuple)
                                         else str(p) for p in args))
    return hashlib.sha256(";".join(parts).encode()).hexdigest()[:12]


class Program:
    """The embedded font program: glyph lookup by index, CID or name."""

    def __init__(self, kind, data, length1=None):
        self.kind = kind  # FontFile, FontFile2, FontFile3/<Subtype>
        self.cff = None  # CFF top dict
        self.tt = None  # TTFont
        self.t1 = None  # psLib font dict
        self.error = None
        self.cid_keyed = False
        self.cid_to_index = {}
        self.names = []
        try:
            if kind == "FontFile2" or kind == "FontFile3/OpenType":
                from fontTools.ttLib import TTFont
                self.tt = TTFont(io.BytesIO(data), lazy=True)
                if "CFF " in self.tt:
                    self._set_cff(self.tt["CFF "].cff)
            elif kind.startswith("FontFile3"):
                from fontTools.cffLib import CFFFontSet
                fs = CFFFontSet()
                fs.decompile(io.BytesIO(data), None)
                self._set_cff(fs)
            elif kind == "FontFile":
                self._read_type1(data, length1)
        except Exception as e:
            self.error = "%s: %s" % (type(e).__name__, e)

    def _set_cff(self, fs):
        top = fs[fs.fontNames[0]]
        self.cff = top
        self.names = list(top.charset)
        self.cid_keyed = hasattr(top, "ROS")
        if self.cid_keyed:
            for i, g in enumerate(self.names):
                if g.startswith("cid"):
                    try:
                        self.cid_to_index[int(g[3:])] = i
                    except ValueError:
                        pass
                elif g == ".notdef":
                    self.cid_to_index[0] = i

    def _read_type1(self, data, length1):
        from fontTools import t1Lib
        if b"cleartomark" not in data[-1024:]:
            # PDF writers drop the trailer (/Length3 0); t1Lib needs it to
            # find the end of the encrypted part.
            data = data + b"\n" + (b"0" * 64 + b"\n") * 8 + b"cleartomark\n"
        chunks = t1Lib.findEncryptedChunks(data)
        parts = []
        for enc, chunk in chunks:
            parts.append(t1Lib.deHexString(chunk) if enc and t1Lib.isHex(chunk[:4]) else chunk)
        t1 = t1Lib.T1Font.__new__(t1Lib.T1Font)  # T1Font without its file reader
        t1.data = b"".join(parts)
        t1.encoding = "ascii"
        t1.parse()  # sets up the charstrings' subroutines
        self.t1 = t1.font
        self.cleartext = data[:length1] if length1 else data

    def builtin_encoding(self):
        """The program's own encoding as 256 names (or None)."""
        if self.cff is not None and not self.cid_keyed:
            enc = getattr(self.cff, "Encoding", None)
            if enc == "StandardEncoding" or enc is None:
                return base_encoding("StandardEncoding")
            if isinstance(enc, list):
                return list(enc) + [".notdef"] * (256 - len(enc))
            return None
        if self.t1 is not None:
            enc = self.t1.get("Encoding")
            if isinstance(enc, list):
                return [n if isinstance(n, str) else ".notdef" for n in enc] + [".notdef"] * (256 - len(enc))
            return base_encoding("StandardEncoding")
        return None

    def outline(self, index=None, cid=None, gname=None):
        """Digest of a glyph's outline (by index in the program, CID or name)."""
        try:
            if self.cff is not None:
                cs = self.cff.CharStrings
                if gname is None:
                    if cid is not None and self.cid_keyed:
                        index = self.cid_to_index.get(cid)
                    if index is None or index >= len(self.names):
                        return None
                    gname = self.names[index]
                if gname not in cs:
                    return None
                return _outline_digest(lambda pen: cs[gname].draw(pen))
            if self.tt is not None:
                from fontTools.pens.recordingPen import DecomposingRecordingPen
                gs = self.tt.getGlyphSet()
                order = self.tt.getGlyphOrder()
                if gname is None:
                    if index is None or index >= len(order):
                        return None
                    gname = order[index]
                if gname not in gs:
                    return None
                pen = DecomposingRecordingPen(gs)
                gs[gname].draw(pen)
                return _outline_digest(lambda p: pen.replay(p))
            if self.t1 is not None and gname is not None:
                cs = self.t1["CharStrings"]
                if gname not in cs:
                    return None
                return _outline_digest(lambda pen: cs[gname].draw(pen))
        except Exception as e:
            return "error:" + type(e).__name__
        return None


# ---------------------------------------------------------------------------
# Fonts
# ---------------------------------------------------------------------------
class Glyph:
    __slots__ = ("id", "outline", "width", "unicode", "code")

    def __init__(self, gid, outline, width, uni, code):
        self.id, self.outline, self.width, self.unicode, self.code = gid, outline, width, uni, code


class Font:
    def __init__(self, doc, fdict):
        self.doc = doc
        g = doc.get
        self.subtype = name(g(fdict.get("/Subtype"))) or "?"
        self.base = name(g(fdict.get("/BaseFont"))) or ""
        self.cid_subtype = None
        self.two_byte = False
        self.cmap = None  # code -> CID for a non-Identity CMap
        self.code_bytes = {1}
        self.c2g = None  # CIDToGIDMap stream
        self.warnings = []
        desc = fdict
        if self.subtype == "Type0":
            dfs = g(fdict.get("/DescendantFonts")) or []
            desc = g(dfs[0]) if dfs else {}
            self.base = name(g(desc.get("/BaseFont"))) or self.base
            self.cid_subtype = name(g(desc.get("/Subtype")))
            enc = g(fdict.get("/Encoding"))
            self.code_bytes = {2}
            if isinstance(enc, Stream):
                self.cmap, spaces = parse_cmap_cids(enc.data)
                self.code_bytes = spaces or {2}
            elif name(enc) not in ("Identity-H", "Identity-V"):
                self.warnings.append("CMap %s read as Identity" % enc)
            m = g(desc.get("/CIDToGIDMap"))
            if isinstance(m, Stream):
                self.c2g = m.data
            self.dw = g(desc.get("/DW"))
            self.dw = 1000 if self.dw is None else self.dw
            self.w = self._parse_w(g(desc.get("/W")) or [])
        else:
            self.first = g(fdict.get("/FirstChar")) or 0
            self.widths = [g(x) for x in (g(fdict.get("/Widths")) or [])]
        self.name = strip_tag(self.base)
        fd = g(desc.get("/FontDescriptor")) or {}
        self.missing_width = g(fd.get("/MissingWidth")) or 0
        self.font_matrix = None
        if self.subtype == "Type3":
            self.font_matrix = [g(x) for x in g(fdict.get("/FontMatrix"))]
            self.name = self.name or "Type3"
        self.program = None
        for k in ("/FontFile", "/FontFile2", "/FontFile3"):
            s = doc.stream(fd.get(k))
            if s is not None:
                kind = k[1:]
                if k == "/FontFile3":
                    kind += "/" + (name(g(s.get("/Subtype"))) or "?")
                self.program = Program(kind, s.data, g(s.get("/Length1")))
                if self.program.error:
                    self.warnings.append("font program: " + self.program.error)
                break
        self.program_kind = self.program.kind if self.program else "none"
        tu = doc.stream(fdict.get("/ToUnicode"))
        self.tounicode = parse_tounicode(tu.data)[0] if tu is not None else {}
        self.encoding = None
        if self.subtype != "Type0":
            self.encoding = self._simple_encoding(g(fdict.get("/Encoding")))
        self._glyphs = {}

    def _parse_w(self, w):
        g = self.doc.get
        out = {}
        i = 0
        w = [g(x) for x in w]
        while i < len(w):
            c = w[i]
            nxt = g(w[i + 1]) if i + 1 < len(w) else None
            if isinstance(nxt, list):
                for k, x in enumerate(nxt):
                    out[int(c) + k] = g(x)
                i += 2
            elif i + 2 < len(w):
                for cid in range(int(c), int(nxt) + 1):
                    out[cid] = w[i + 2]
                i += 3
            else:
                break
        return out

    def _simple_encoding(self, enc):
        g = self.doc.get
        base = None
        diffs = None
        if isinstance(enc, str) and name(enc):
            base = base_encoding(name(enc))
        elif isinstance(enc, dict):
            b = name(g(enc.get("/BaseEncoding")))
            base = base_encoding(b) if b else None
            diffs = g(enc.get("/Differences"))
        if base is None and self.program is not None and self.subtype != "TrueType":
            base = self.program.builtin_encoding()
        if base is None and self.subtype in ("Type1", "MMType1") and self.program is None:
            base = base_encoding("StandardEncoding")
        names = list(base) if base else [None] * 256
        if isinstance(diffs, list):
            code = 0
            for x in diffs:
                x = g(x)
                if isinstance(x, (int, float)):
                    code = int(x)
                elif name(x) is not None:
                    if 0 <= code < 256:
                        names[code] = name(x)
                    code += 1
        return names

    # -- codes ---------------------------------------------------------------
    def split(self, s):
        """A shown string's bytes as codes."""
        if self.code_bytes == {1}:
            return list(s)
        if self.code_bytes == {2}:
            return [int.from_bytes(s[i:i + 2], "big") for i in range(0, len(s) - 1, 2)]
        # Mixed codespaces: take the shortest width that the CMap maps.
        out = []
        i = 0
        widths = sorted(self.code_bytes)
        while i < len(s):
            for n in widths:
                c = int.from_bytes(s[i:i + n], "big")
                if n == widths[-1] or (self.cmap and c in self.cmap):
                    out.append(c)
                    i += n
                    break
        return out

    def glyph(self, code):
        g = self._glyphs.get(code)
        if g is None:
            g = self._glyphs[code] = self._make_glyph(code)
        return g

    def _make_glyph(self, code):
        uni = self.tounicode.get(code)
        p = self.program
        if self.subtype == "Type0":
            cid = self.cmap.get(code, code) if self.cmap is not None else code
            width = self.w.get(cid, self.dw)
            if self.cid_subtype == "CIDFontType2":
                if self.c2g is not None:
                    gid = int.from_bytes(self.c2g[2 * cid:2 * cid + 2], "big") if 2 * cid + 2 <= len(self.c2g) else 0
                else:
                    gid = cid
                outline = p.outline(index=gid) if p else None
            else:
                gid = cid
                if p is not None and p.cff is not None and not p.cid_keyed:
                    outline = p.outline(index=cid)
                else:
                    outline = p.outline(cid=cid) if p else None
            return Glyph("gid:%d" % gid, outline, width, uni, code)
        # simple fonts
        i = code - int(self.first)
        width = self.widths[i] if 0 <= i < len(self.widths) else self.missing_width
        gname = self.encoding[code] if self.encoding and code < len(self.encoding) else None
        if gname and gname != ".notdef":
            outline = p.outline(gname=gname) if p else None
            return Glyph("name:" + gname, outline, width, uni, code)
        return Glyph("code:%d" % code, None, width, uni, code)

    def summary(self):
        return {"subtype": self.subtype, "cid_subtype": self.cid_subtype,
                "program": self.program_kind}


class FontCache:
    def __init__(self, doc):
        self.doc = doc
        self._fonts = {}

    def font(self, ref_or_dict):
        key = ref_or_dict if isinstance(ref_or_dict, str) else id(ref_or_dict)
        f = self._fonts.get(key)
        if f is None:
            d = self.doc.get(ref_or_dict)
            f = self._fonts[key] = Font(self.doc, d if isinstance(d, dict) else {})
        return f

    def all(self):
        return list(self._fonts.values())


__all__ = ["Font", "FontCache", "Glyph", "strip_tag", "parse_tounicode", "raw_bytes"]
