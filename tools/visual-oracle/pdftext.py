#!/usr/bin/env python3
"""Word positions from a pdfTeX PDF, standard library only (oracle tooling).

Reads the reference PDFs that pdflatex produced (`fixtures/real-world/*/`):
xref streams, object streams and FlateDecode are handled with `zlib`; nothing
else is required. For every page the page content stream is replayed
(`BT`/`ET`, `Tf`, `Td`/`TD`/`Tm`/`T*`/`TL`, `Tc`/`Tw`/`Tz`/`Ts`, `Tj`/`TJ`/`'`/`"`,
`q`/`Q`/`cm`) with the fonts' own `/Widths` so every glyph gets its exact
origin in PDF user space. Glyphs are grouped into words by the gap between a
glyph's end and the next glyph's start (a gap wider than `SPACE_FRACTION`
of the font size, or a new text object or a new baseline, starts a word).

Character text is only used to align words with the candidate: codes are
mapped through `/Encoding /Differences` when present, else through the
OT1 (`CM*`) or T1 (`SF*`, `EC*`, `LM*`) tables for the ligature slots and
ASCII otherwise; unknown glyph names become `?`. This is not a text
extractor for prose. Positions are returned in bp with y measured from the
top of the page (the display-list-v2 convention) so both sides compare
directly: 1 bp = 1 PDF user-space unit for these MediaBoxes.

Not a general PDF parser: encrypted files, non-Flate filters, Type 3 fonts,
CID fonts and inline images are reported as `unsupported` on the page
record instead of guessed at.
"""

import re
import zlib

SPACE_FRACTION = 0.16  # gap (in em) that separates two words

_WS = b"\x00\t\n\x0c\r "
_DELIM = b"()<>[]{}/%"

# glyph name -> text, enough to align words; everything else is "?"
_GLYPH_TEXT = {
    "space": " ", "period": ".", "comma": ",", "colon": ":", "semicolon": ";", "hyphen": "-",
    "endash": "--", "emdash": "---", "quoteright": "'", "quoteleft": "`", "quotedblright": "''",
    "quotedblleft": "``", "quotesingle": "'", "parenleft": "(", "parenright": ")",
    "bracketleft": "[", "bracketright": "]", "braceleft": "{", "braceright": "}",
    "exclam": "!", "question": "?", "slash": "/", "equal": "=", "plus": "+", "minus": "-",
    "less": "<", "greater": ">", "asterisk": "*", "ampersand": "&", "percent": "%",
    "dollar": "$", "numbersign": "#", "at": "@", "underscore": "_", "bar": "|",
    "fi": "fi", "fl": "fl", "ff": "ff", "ffi": "ffi", "ffl": "ffl",
    "zero": "0", "one": "1", "two": "2", "three": "3", "four": "4", "five": "5", "six": "6",
    "seven": "7", "eight": "8", "nine": "9",
    "eacute": "é", "egrave": "è", "agrave": "à", "aacute": "á", "ccedilla": "ç", "ntilde": "ñ",
    "odieresis": "ö", "udieresis": "ü", "adieresis": "ä", "ocircumflex": "ô", "ecircumflex": "ê",
    "germandbls": "ß", "oslash": "ø", "aring": "å",
}
for _c in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ":
    _GLYPH_TEXT[_c] = _c

# ligature / dash slots of the two TeX encodings pdfTeX's base fonts use
_OT1 = {11: "ff", 12: "fi", 13: "fl", 14: "ffi", 15: "ffl", 123: "--", 124: "---", 92: "``", 34: "''", 39: "'", 96: "`"}
_T1 = {27: "ff", 28: "fi", 29: "fl", 30: "ffi", 31: "ffl", 21: "--", 22: "---", 16: "``", 17: "''", 39: "'", 96: "`"}


class Name(str):
    """A PDF name object (distinct from a string)."""


class Ref(tuple):
    """An indirect reference (num, gen)."""


class PdfError(Exception):
    pass


# ----------------------------------------------------------------------------
# object syntax


class _Lexer:
    def __init__(self, data, pos=0):
        self.data = data
        self.pos = pos

    def skip_ws(self):
        d, n = self.data, len(self.data)
        while self.pos < n:
            c = d[self.pos:self.pos + 1]
            if c in _WS:
                self.pos += 1
            elif c == b"%":
                while self.pos < n and d[self.pos:self.pos + 1] not in (b"\n", b"\r"):
                    self.pos += 1
            else:
                break

    def token(self):
        self.skip_ws()
        d = self.data
        if self.pos >= len(d):
            return None
        c = d[self.pos:self.pos + 1]
        if c == b"<":
            if d[self.pos + 1:self.pos + 2] == b"<":
                self.pos += 2
                return b"<<"
            end = d.index(b">", self.pos)
            hexs = re.sub(rb"\s", b"", d[self.pos + 1:end])
            self.pos = end + 1
            if len(hexs) % 2:
                hexs += b"0"
            return ("str", bytes.fromhex(hexs.decode("ascii")))
        if c == b">":
            self.pos += 2
            return b">>"
        if c == b"(":
            return ("str", self._literal_string())
        if c == b"/":
            self.pos += 1
            start = self.pos
            while self.pos < len(d) and d[self.pos:self.pos + 1] not in _WS and d[self.pos:self.pos + 1] not in _DELIM:
                self.pos += 1
            raw = d[start:self.pos]
            raw = re.sub(rb"#([0-9A-Fa-f]{2})", lambda m: bytes.fromhex(m.group(1).decode()), raw)
            return Name(raw.decode("latin-1"))
        if c in b"[]{}":
            self.pos += 1
            return c
        start = self.pos
        while self.pos < len(d) and d[self.pos:self.pos + 1] not in _WS and d[self.pos:self.pos + 1] not in _DELIM:
            self.pos += 1
        if self.pos == start:
            self.pos += 1
            return d[start:self.pos]
        return d[start:self.pos]

    def _literal_string(self):
        d = self.data
        assert d[self.pos:self.pos + 1] == b"("
        self.pos += 1
        depth = 1
        out = bytearray()
        while self.pos < len(d):
            c = d[self.pos]
            if c == 0x5C:
                self.pos += 1
                e = d[self.pos:self.pos + 1]
                if e.isdigit():
                    j = self.pos
                    while j < len(d) and j < self.pos + 3 and d[j:j + 1].isdigit():
                        j += 1
                    out.append(int(d[self.pos:j], 8) & 0xFF)
                    self.pos = j
                    continue
                out += {b"n": b"\n", b"r": b"\r", b"t": b"\t", b"b": b"\b", b"f": b"\f",
                        b"\n": b"", b"\r": b""}.get(e, e)
                self.pos += 1
                continue
            if c == 0x28:
                depth += 1
            elif c == 0x29:
                depth -= 1
                if depth == 0:
                    self.pos += 1
                    return bytes(out)
            out.append(c)
            self.pos += 1
        return bytes(out)

    def object(self, tok=None):
        """Parses one object; resolves `n g R` references to Ref."""
        if tok is None:
            tok = self.token()
        if tok is None:
            return None
        if tok == b"<<":
            dct = {}
            while True:
                k = self.token()
                if k == b">>" or k is None:
                    break
                dct[str(k)] = self.object()
            return dct
        if tok == b"[":
            arr = []
            while True:
                t = self.token()
                if t == b"]" or t is None:
                    break
                arr.append(self.object(t))
            return arr
        if isinstance(tok, tuple):
            return tok[1]
        if isinstance(tok, Name):
            return tok
        if not isinstance(tok, bytes):
            return tok
        if re.fullmatch(rb"[+-]?\d+", tok):
            save = self.pos
            t2 = self.token()
            if isinstance(t2, bytes) and re.fullmatch(rb"\d+", t2):
                t3 = self.token()
                if t3 == b"R":
                    return Ref((int(tok), int(t2)))
            self.pos = save
            return int(tok)
        if re.fullmatch(rb"[+-]?(\d+\.?\d*|\.\d+)", tok):
            return float(tok)
        if tok == b"true":
            return True
        if tok == b"false":
            return False
        if tok == b"null":
            return None
        return ("op", tok)


# ----------------------------------------------------------------------------
# document


class PdfDocument:
    def __init__(self, data):
        self.data = data
        self.objects = {}   # num -> (dict/obj, stream bytes or None)
        self.trailer = {}
        self.unsupported = []
        self._scan()

    @classmethod
    def load(cls, path):
        with open(path, "rb") as f:
            return cls(f.read())

    def _scan(self):
        d = self.data
        for m in re.finditer(rb"(?<![0-9])(\d+)\s+(\d+)\s+obj\b", d):
            num = int(m.group(1))
            lx = _Lexer(d, m.end())
            obj = lx.object()
            stream = None
            lx.skip_ws()
            if d[lx.pos:lx.pos + 6] == b"stream":
                p = lx.pos + 6
                if d[p:p + 2] == b"\r\n":
                    p += 2
                elif d[p:p + 1] in (b"\n", b"\r"):
                    p += 1
                length = obj.get("Length") if isinstance(obj, dict) else None
                if isinstance(length, Ref):
                    length = self._direct_length(length)
                if isinstance(length, int) and d[p + length:p + length + 20].lstrip().startswith(b"endstream"):
                    stream = d[p:p + length]
                else:
                    end = d.find(b"endstream", p)
                    stream = d[p:end].rstrip(b"\r\n")
            self.objects[num] = (obj, stream)
        for m in re.finditer(rb"trailer\s*<<", d):
            lx = _Lexer(d, m.end() - 2)
            t = lx.object()
            if isinstance(t, dict):
                self.trailer.update({k: v for k, v in t.items() if k not in self.trailer})
        # object streams and xref-stream trailers
        for num in sorted(self.objects):
            obj, stream = self.objects[num]
            if not isinstance(obj, dict):
                continue
            if obj.get("Type") == "XRef":
                for k in ("Root", "Info", "ID"):
                    if k in obj and k not in self.trailer:
                        self.trailer[k] = obj[k]
            if obj.get("Type") == "ObjStm" and stream is not None:
                self._expand_objstm(obj, stream)

    def _direct_length(self, ref):
        m = re.search(rb"(?<![0-9])%d\s+%d\s+obj\s+(\d+)" % ref, self.data)
        return int(m.group(1)) if m else None

    def _expand_objstm(self, obj, stream):
        raw = self.decode(obj, stream)
        n, first = int(obj.get("N", 0)), int(obj.get("First", 0))
        head = raw[:first].split()
        for i in range(n):
            num, off = int(head[2 * i]), int(head[2 * i + 1])
            lx = _Lexer(raw, first + off)
            if num not in self.objects:
                self.objects[num] = (lx.object(), None)

    def decode(self, obj, stream):
        filt = obj.get("Filter")
        if filt is None:
            return stream
        filters = filt if isinstance(filt, list) else [filt]
        for f in filters:
            if f == "FlateDecode":
                stream = zlib.decompress(stream)
            else:
                raise PdfError(f"unsupported filter {f}")
        parms = obj.get("DecodeParms")
        if isinstance(parms, dict) and parms.get("Predictor", 1) > 1:
            raise PdfError("PNG predictors are not supported")
        return stream

    def resolve(self, x, depth=0):
        while isinstance(x, Ref) and depth < 32:
            x = self.objects.get(x[0], (None, None))[0]
            depth += 1
        return x

    def stream_of(self, ref):
        obj, stream = self.objects.get(ref[0], (None, None))
        if stream is None:
            raise PdfError(f"object {ref[0]} has no stream")
        return self.decode(obj, stream)

    def pages(self):
        root = self.resolve(self.trailer.get("Root"))
        if not isinstance(root, dict):
            # fall back: any /Type /Pages without a parent
            root = {"Pages": next((Ref((n, 0)) for n, (o, _) in self.objects.items()
                                   if isinstance(o, dict) and o.get("Type") == "Pages" and "Parent" not in o), None)}
        out = []

        def walk(ref, inherited):
            node = self.resolve(ref)
            if not isinstance(node, dict):
                return
            inh = dict(inherited)
            for k in ("Resources", "MediaBox"):
                if k in node:
                    inh[k] = node[k]
            if node.get("Type") == "Pages" or "Kids" in node:
                for kid in self.resolve(node.get("Kids", [])) or []:
                    walk(kid, inh)
            else:
                page = dict(node)
                page.update({k: v for k, v in inh.items() if k not in page})
                out.append(page)

        walk(root.get("Pages"), {})
        return out

    # ------------------------------------------------------------------
    # fonts

    def font(self, fdict):
        fdict = self.resolve(fdict)
        sub = fdict.get("Subtype")
        base = str(fdict.get("BaseFont", ""))
        base_plain = re.sub(r"^[A-Z]{6}\+", "", base)
        info = {"subtype": str(sub), "base": base_plain, "widths": {}, "text": {}, "missing_width": 0.0,
                "unsupported": None}
        if sub not in ("Type1", "TrueType", "MMType1"):
            info["unsupported"] = f"font subtype {sub}"
            return info
        first = self.resolve(fdict.get("FirstChar"))
        widths = self.resolve(fdict.get("Widths"))
        if isinstance(first, int) and isinstance(widths, list):
            for i, w in enumerate(widths):
                w = self.resolve(w)
                info["widths"][first + i] = float(w) / 1000.0
        desc = self.resolve(fdict.get("FontDescriptor"))
        if isinstance(desc, dict) and "MissingWidth" in desc:
            info["missing_width"] = float(self.resolve(desc["MissingWidth"])) / 1000.0
        table = _OT1 if base_plain.upper().startswith("CM") else _T1
        for code in range(256):
            if 32 <= code < 127:
                info["text"][code] = chr(code)
        for code, t in table.items():
            info["text"][code] = t
        enc = self.resolve(fdict.get("Encoding"))
        if isinstance(enc, dict):
            diffs = self.resolve(enc.get("Differences"))
            if isinstance(diffs, list):
                code = 0
                for item in diffs:
                    item = self.resolve(item)
                    if isinstance(item, (int, float)):
                        code = int(item)
                    else:
                        name = str(item)
                        info["text"][code] = _GLYPH_TEXT.get(name) or _uni_name(name) or "?"
                        code += 1
        return info


def _uni_name(name):
    m = re.fullmatch(r"uni([0-9A-Fa-f]{4})", name)
    if m:
        return chr(int(m.group(1), 16))
    return None


# ----------------------------------------------------------------------------
# content replay


def _mul(a, b):
    """3x3 affine (as 6-tuples) product a×b."""
    a0, a1, a2, a3, a4, a5 = a
    b0, b1, b2, b3, b4, b5 = b
    return (a0 * b0 + a1 * b2, a0 * b1 + a1 * b3, a2 * b0 + a3 * b2, a2 * b1 + a3 * b3,
            a4 * b0 + a5 * b2 + b4, a4 * b1 + a5 * b3 + b5)


def page_glyphs(doc, page):
    """Returns (glyphs, notes). Each glyph: dict(x, y_top, size, font, text,
    advance, bt) in bp; `bt` counts text objects so words never span one."""
    notes = []
    media = [float(doc.resolve(v)) for v in doc.resolve(page.get("MediaBox", [0, 0, 612, 792]))]
    height = media[3] - media[1]
    resources = doc.resolve(page.get("Resources", {})) or {}
    font_dicts = doc.resolve(resources.get("Font", {})) or {}
    fonts = {}
    for name, ref in font_dicts.items():
        fonts[name] = doc.font(ref)
        if fonts[name]["unsupported"]:
            notes.append(f"/{name}: {fonts[name]['unsupported']}")
    contents = page.get("Contents")
    contents = doc.resolve(contents) if not isinstance(contents, list) else contents
    if isinstance(contents, list):
        data = b"\n".join(doc.stream_of(c) for c in contents)
    elif isinstance(page.get("Contents"), Ref):
        data = doc.stream_of(page["Contents"])
    else:
        return [], notes + ["page has no content stream"]
    if "XObject" in resources:
        notes.append("page uses XObjects (their text, if any, is not read)")

    lx = _Lexer(data)
    stack = []
    ctm = (1, 0, 0, 1, 0, 0)
    gs_stack = []
    tm = tlm = (1, 0, 0, 1, 0, 0)
    font = None
    size = 0.0
    tc = tw = 0.0
    th = 1.0
    tl = 0.0
    rise = 0.0
    bt = 0
    glyphs = []

    def show(s):
        nonlocal tm
        if font is None:
            notes.append("text shown before Tf")
            return
        for code in s:
            w0 = font["widths"].get(code, font["missing_width"])
            m = _mul(_mul((size * th, 0, 0, size, 0, rise), tm), ctm)
            x, y = m[4], m[5]
            adv = (w0 * size + tc + (tw if code == 32 else 0.0)) * th
            scale = m[0] / (size * th) if size * th else 1.0
            glyphs.append({"x": x, "y_top": height - y, "size": m[3], "font": font["base"],
                           "text": font["text"].get(code, "?"), "advance": adv * scale, "bt": bt, "code": code})
            tm = _mul((1, 0, 0, 1, adv, 0), tm)

    def adjust(n):
        nonlocal tm
        tx = -n / 1000.0 * size * th
        tm = _mul((1, 0, 0, 1, tx, 0), tm)

    while True:
        tok = lx.token()
        if tok is None:
            break
        obj = lx.object(tok)
        if not (isinstance(obj, tuple) and obj and obj[0] == "op"):
            stack.append(obj)
            continue
        op = obj[1]
        try:
            if op == b"BT":
                tm = tlm = (1, 0, 0, 1, 0, 0)
                bt += 1
            elif op == b"ET":
                pass
            elif op == b"Tf":
                fname, size = str(stack[-2]), float(stack[-1])
                font = fonts.get(fname)
                if font is None:
                    notes.append(f"unknown font /{fname}")
            elif op == b"Td":
                tlm = _mul((1, 0, 0, 1, float(stack[-2]), float(stack[-1])), tlm)
                tm = tlm
            elif op == b"TD":
                tl = -float(stack[-1])
                tlm = _mul((1, 0, 0, 1, float(stack[-2]), float(stack[-1])), tlm)
                tm = tlm
            elif op == b"Tm":
                tlm = tuple(float(v) for v in stack[-6:])
                tm = tlm
            elif op == b"T*":
                tlm = _mul((1, 0, 0, 1, 0, -tl), tlm)
                tm = tlm
            elif op == b"TL":
                tl = float(stack[-1])
            elif op == b"Tc":
                tc = float(stack[-1])
            elif op == b"Tw":
                tw = float(stack[-1])
            elif op == b"Tz":
                th = float(stack[-1]) / 100.0
            elif op == b"Ts":
                rise = float(stack[-1])
            elif op == b"Tj":
                show(stack[-1])
            elif op == b"'":
                tlm = _mul((1, 0, 0, 1, 0, -tl), tlm)
                tm = tlm
                show(stack[-1])
            elif op == b'"':
                tw, tc = float(stack[-3]), float(stack[-2])
                tlm = _mul((1, 0, 0, 1, 0, -tl), tlm)
                tm = tlm
                show(stack[-1])
            elif op == b"TJ":
                for el in stack[-1]:
                    if isinstance(el, bytes):
                        show(el)
                    else:
                        adjust(float(el))
            elif op == b"q":
                gs_stack.append(ctm)
            elif op == b"Q":
                if gs_stack:
                    ctm = gs_stack.pop()
            elif op == b"cm":
                ctm = _mul(tuple(float(v) for v in stack[-6:]), ctm)
            elif op == b"BI":
                notes.append("inline image skipped")
                end = data.find(b"EI", lx.pos)
                lx.pos = end + 2 if end >= 0 else len(data)
            elif op == b"Do":
                notes.append("XObject drawn (not read)")
        except (IndexError, TypeError, ValueError) as e:
            notes.append(f"operator {op!r}: {e}")
        stack = []
    return glyphs, sorted(set(notes))


def words_from_glyphs(glyphs):
    """Groups glyphs into words: same text object, same baseline (0.05 bp),
    and no gap wider than SPACE_FRACTION em between glyphs.

    A *font change does not break a word*. That matters: pdfTeX sets one
    siunitx `S` cell as three `Tf`-switched runs (CMR10 digits, CMMI10
    decimal marker, CMR10 again) inside one text object, and this rule joins
    them back into `1.234` — which is the word a reader sees. Any other
    producer of word boxes has to use this same function, or the two sides
    disagree about what a word is and the alignment measures the
    disagreement instead of the geometry (see `rank.v2_words`).

    Each word carries `glyph_index`, the index in `glyphs` of its first
    glyph. A word's glyphs are always contiguous there (a space glyph ends
    the current word and is itself dropped), so `glyphs[i:i + w["glyphs"]]`
    is exactly the run the word was built from, and a caller can carry its
    own per-glyph data across the grouping.
    """
    words = []
    cur = None
    for i, g in enumerate(glyphs):
        if g["text"] == " ":
            cur = None
            continue
        if cur is not None:
            gap = g["x"] - cur["_end"]
            same_line = abs(g["y_top"] - cur["y_top"]) < 0.05 and g["bt"] == cur["_bt"]
            if not same_line or gap > SPACE_FRACTION * max(g["size"], 1e-6) or gap < -0.5 * max(g["size"], 1e-6):
                cur = None
        if cur is None:
            cur = {"text": "", "x": g["x"], "y_top": g["y_top"], "size": g["size"], "font": g["font"],
                   "_end": g["x"], "_bt": g["bt"], "glyphs": 0, "glyph_index": i}
            words.append(cur)
        cur["text"] += g["text"]
        cur["_end"] = g["x"] + g["advance"]
        cur["glyphs"] += 1
    for w in words:
        w["width"] = w.pop("_end") - w["x"]
        w.pop("_bt")
    return words


def page_words(path):
    """Word boxes for every page of `path`: list of dict(page, words, notes,
    media_box)."""
    doc = PdfDocument.load(path)
    out = []
    for i, page in enumerate(doc.pages()):
        rec = {"page": i + 1, "words": [], "notes": [], "media_box": None}
        try:
            rec["media_box"] = [float(doc.resolve(v)) for v in doc.resolve(page.get("MediaBox", [0, 0, 612, 792]))]
            glyphs, notes = page_glyphs(doc, page)
            rec["words"] = words_from_glyphs(glyphs)
            rec["notes"] = notes
        except PdfError as e:
            rec["notes"] = [f"unsupported: {e}"]
        out.append(rec)
    return out


if __name__ == "__main__":  # pragma: no cover - manual inspection
    import json
    import sys

    for rec in page_words(sys.argv[1]):
        print(json.dumps({"page": rec["page"], "notes": rec["notes"], "words": len(rec["words"])}))
        for w in rec["words"][: int(sys.argv[2]) if len(sys.argv) > 2 else 12]:
            print(f"  {w['x']:8.3f} {w['y_top']:8.3f} {w['size']:6.2f} {w['font']:12} {w['text']}")
