#!/usr/bin/env python3
"""TrueType / OpenType (sfnt) font fuzzer for the FlashTeX candidate. MIT.

DESIGN.md section 8, tier T6, lists OpenType among the parser fuzzers;
this is that fuzzer. It drives the engine's port of pdfTeX's
`writettf.c` (crates/flashtex-engine/src/pdftex/writettf.rs): TrueType
subsetting (`writettf`), whole-font TrueType copying and OpenType (CFF)
embedding (`writeotf`).

Each iteration mutates an sfnt seed (real fonts found at run time with
kpsewhich and never copied into the repo, plus a small synthetic
TrueType built here so the fuzzer always has a seed), writes it as
fuzz.ttf or fuzz.otf next to a copy of cmr10.tfm named fuzz.tfm and of
8r.enc, and runs the candidate on a plain-TeX job whose \\pdfmapline
picks one of three embedding modes:

  ttf-subset  +fuzz FuzzFont <8r.enc <fuzz.ttf   (writettf, subset)
  ttf-whole   +fuzz FuzzFont <<fuzz.ttf          (writettf, whole font)
  otf-whole   +fuzz FuzzFont <<fuzz.otf          (writeotf)

An unmutated seed compiles and embeds cleanly under pdfTeX in every
mode, so a font that loads is distinguishable from one the engine
rejects. Classes: ok / font-rejected / graceful-error / crash / hang /
output-flood. Crash, hang and flood inputs are stored under
OUT/<class>/ with a .json sidecar, deduped by panic location or signal.
Stdlib only. Deterministic given --seed (all draws go through one
random.Random).
"""
import argparse
import hashlib
import json
import os
import random
import shutil
import struct
import subprocess
import sys
import tempfile

sys.path.insert(0,
                os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import run as fuzz_run

# (kpsewhich name, kind). Missing ones are skipped; the synthetic seed
# is always present. emo-lingchi.ttf is tiny; DejaVuSans.ttf exercises
# a real hinted font with composites; the two .otf are CFF OpenType.
SEED_NAMES = (("emo-lingchi.ttf", "ttf"), ("DejaVuSans.ttf", "ttf"),
              ("lmroman10-regular.otf", "otf"),
              ("texgyretermes-regular.otf", "otf"))
MODES = {
    "ttf": ("ttf-subset", "ttf-whole"),
    "otf": ("otf-whole",),
}
MAPLINES = {
    "ttf-subset": "+fuzz FuzzFont <8r.enc <fuzz.ttf",
    "ttf-whole": "+fuzz FuzzFont <<fuzz.ttf",
    "otf-whole": "+fuzz FuzzFont <<fuzz.otf",
}
# Characters set in the fuzz font: through 8r.enc they reach glyphs by
# name (post table) in subset mode, covering letters, digits and
# punctuation so that several glyf/loca entries and composites are read.
JOB_TEXT = "Hello, World! 0123456789 fi AV"
# Log text of pdfTeX's fatal font errors, e.g.
# "!pdfTeX error: pdftex (file fuzz.ttf): ..." The file name is only
# printed for errors raised while reading the font.
REJECT_HINTS = ("(file fuzz.ttf)", "(file fuzz.otf)")
CLASSES = ("ok", "font-rejected", "graceful-error", "crash", "hang",
           "output-flood")
STORE = ("crash", "hang", "output-flood")
SUPPORT_FILES = ("cmr10.tfm", "8r.enc")
_SUPPORT_CACHE = {}

U32_VALUES = (0, 1, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF)
U16_VALUES = (0, 1, 0x7FFF, 0x8000, 0xFFFF)


# ---------------------------------------------------------------------
# Synthetic seed: a small, valid TrueType font built from scratch.

def _checksum(blob):
    blob = blob + b"\x00" * (-len(blob) % 4)
    return sum(struct.unpack(">%dI" % (len(blob) // 4), blob)) & 0xFFFFFFFF


def _simple_glyph(x0, y0, x1, y1):
    """One closed rectangle contour as a simple glyf entry."""
    pts = ((x0, y0), (x0, y1), (x1, y1), (x1, y0))
    out = struct.pack(">hhhhh", 1, x0, y0, x1, y1)
    out += struct.pack(">H", 3)            # endPtsOfContours[0]
    out += struct.pack(">H", 0)            # instructionLength
    out += bytes([0x01] * 4)               # flags: on-curve, i16 coords
    px = py = 0
    xs = ys = b""
    for x, y in pts:
        xs += struct.pack(">h", x - px)
        ys += struct.pack(">h", y - py)
        px, py = x, y
    return out + xs + ys


def _composite_glyph(parts, bbox):
    """A composite glyf entry referencing `parts` [(glyph, dx, dy)]."""
    out = struct.pack(">hhhhh", -1, *bbox)
    for i, (gid, dx, dy) in enumerate(parts):
        flags = 0x0001 | 0x0002            # ARG_1_AND_2_ARE_WORDS, XY
        if i + 1 < len(parts):
            flags |= 0x0020                # MORE_COMPONENTS
        out += struct.pack(">HHhh", flags, gid, dx, dy)
    return out


def synthetic_ttf():
    """A minimal valid TrueType font (bytes).

    Glyphs: .notdef, then one rectangle per character of JOB_TEXT that
    8r.enc names (H e l o comma W r d exclam zero..nine f i A V space),
    and the ligature 'fi' as a composite of 'f' and 'i'. Tables: cmap
    (format 4), glyf, head, hhea, hmtx, loca (long), maxp, name, OS/2,
    post (format 2 with names), all checksummed.
    """
    names = [".notdef", "space", "H", "e", "l", "o", "comma", "W", "r",
             "d", "exclam", "zero", "one", "two", "three", "four",
             "five", "six", "seven", "eight", "nine", "f", "i", "A",
             "V", "fi"]
    codes = {"space": 0x20, "H": 0x48, "e": 0x65, "l": 0x6C, "o": 0x6F,
             "comma": 0x2C, "W": 0x57, "r": 0x72, "d": 0x64,
             "exclam": 0x21, "f": 0x66, "i": 0x69, "A": 0x41,
             "V": 0x56}
    for k, n in enumerate(("zero", "one", "two", "three", "four", "five",
                           "six", "seven", "eight", "nine")):
        codes[n] = 0x30 + k
    glyphs = []
    for gid, name in enumerate(names):
        if name == "space":
            glyphs.append(b"")
        elif name == "fi":
            glyphs.append(_composite_glyph(
                [(names.index("f"), 0, 0), (names.index("i"), 300, 0)],
                (50, 0, 650, 700)))
        else:
            w = 100 + 10 * gid
            glyphs.append(_simple_glyph(50, 0, 50 + w, 700))
    glyf = b""
    loca = []
    for g in glyphs:
        loca.append(len(glyf))
        glyf += g + b"\x00" * (-len(g) % 4)
    loca.append(len(glyf))
    n = len(names)
    tables = {}
    tables[b"glyf"] = glyf
    tables[b"loca"] = struct.pack(">%dI" % len(loca), *loca)
    tables[b"head"] = struct.pack(
        ">IIIIHHqqhhhhHHhhh", 0x00010000, 0x00010000, 0, 0x5F0F3CF5,
        0x000B, 1000, 0, 0, 0, -200, 1000, 800, 0, 8, 2, 1, 0)
    tables[b"hhea"] = struct.pack(
        ">IhhhHhhhhhhhhhhhH", 0x00010000, 800, -200, 0, 1000, 0, 0, 900,
        1, 0, 0, 0, 0, 0, 0, 0, n)
    tables[b"hmtx"] = b"".join(struct.pack(">Hh", 600, 50)
                               for _ in range(n))
    tables[b"maxp"] = struct.pack(">IHHHHHHHHHHHHHH", 0x00010000, n,
                                  4, 1, 4, 2, 2, 0, 0, 0, 0, 0, 0, 2, 1)
    # post format 2: standard Mac names for indices < 258 are not used;
    # every glyph but .notdef gets a custom Pascal-string name.
    post = struct.pack(">IIhhIIIII", 0x00020000, 0, -100, 50, 0, 0, 0,
                       0, 0)
    post += struct.pack(">H", n)
    custom = []
    for name in names:
        if name == ".notdef":
            post += struct.pack(">H", 0)
        else:
            post += struct.pack(">H", 258 + len(custom))
            custom.append(name)
    for name in custom:
        raw = name.encode("ascii")
        post += bytes([len(raw)]) + raw
    tables[b"post"] = post
    # cmap: one format 4 subtable (3,1), one segment per mapped char.
    pairs = sorted((codes[nm], names.index(nm)) for nm in codes)
    segs = [(c, c, g) for c, g in pairs] + [(0xFFFF, 0xFFFF, 0)]
    seg_x2 = 2 * len(segs)
    sub = struct.pack(">HHHHHHH", 4, 0, 0, seg_x2, 0, 0, 0)
    sub += b"".join(struct.pack(">H", e) for _s, e, _g in segs)
    sub += struct.pack(">H", 0)
    sub += b"".join(struct.pack(">H", s) for s, _e, _g in segs)
    sub += b"".join(struct.pack(">h", ((g - s) + 0x8000) % 0x10000
                                - 0x8000) if s != 0xFFFF
                    else struct.pack(">h", 1) for s, _e, g in segs)
    sub += b"".join(struct.pack(">H", 0) for _ in segs)
    sub = sub[:2] + struct.pack(">H", len(sub)) + sub[4:]
    tables[b"cmap"] = struct.pack(">HHHHI", 0, 1, 3, 1, 12) + sub
    family = "FuzzSynth".encode("utf-16-be")
    recs = ((1, family), (2, "Regular".encode("utf-16-be")),
            (4, family), (6, family))
    name = struct.pack(">HHH", 0, len(recs), 6 + 12 * len(recs))
    strings = b""
    for nid, raw in recs:
        name += struct.pack(">HHHHHH", 3, 1, 0x409, nid, len(raw),
                            len(strings))
        strings += raw
    tables[b"name"] = name + strings
    tables[b"OS/2"] = (struct.pack(">HhHHH", 3, 500, 400, 5, 0)
                       + b"\x00" * 20 + struct.pack(">h", 0)
                       + b"\x00" * 10 + b"\x00" * 16 + b"FUZZ"
                       + struct.pack(">HHH", 0x40, 0x20, 0x7A)
                       + struct.pack(">hhhHH", 800, -200, 0, 1000, 200)
                       + b"\x00" * 8
                       + struct.pack(">hhHHH", 500, 700, 0, 0x20, 0))
    tags = sorted(tables)
    ntab = len(tags)
    es = max(k for k in range(8) if 1 << k <= ntab)
    header = struct.pack(">IHHHH", 0x00010000, ntab, 16 << es, es,
                         16 * ntab - (16 << es))
    offset = 12 + 16 * ntab
    directory = b""
    body = b""
    for tag in tags:
        data = tables[tag]
        directory += struct.pack(">4sIII", tag, _checksum(data),
                                 offset + len(body), len(data))
        body += data + b"\x00" * (-len(data) % 4)
    font = bytearray(header + directory + body)
    head_off = parse_sfnt(bytes(font))["tables"][b"head"][0]
    adj = (0xB1B0AFBA - _checksum(bytes(font))) & 0xFFFFFFFF
    font[head_off + 8:head_off + 12] = struct.pack(">I", adj)
    return bytes(font)


# ---------------------------------------------------------------------
# sfnt parsing (just enough for structure-aware mutations).

def parse_sfnt(data):
    """Parse the table directory; return a dict or None when too short.

    Returns {"num_tables", "records": [(record_offset, tag, offset,
    length)], "tables": {tag: (offset, length)}}. Records whose 16 bytes
    lie past the end are dropped; table spans are not bounds-checked
    (callers use _table).
    """
    if len(data) < 12:
        return None
    num = struct.unpack(">H", data[4:6])[0]
    records = []
    tables = {}
    for i in range(num):
        rec = 12 + 16 * i
        if rec + 16 > len(data):
            break
        tag, _sum, off, length = struct.unpack(">4sIII",
                                               data[rec:rec + 16])
        records.append((rec, tag, off, length))
        tables.setdefault(tag, (off, length))
    return {"num_tables": num, "records": records, "tables": tables}


def _table(data, tag, minimum=1):
    """(offset, length) of `tag` if it lies fully inside `data`."""
    info = parse_sfnt(data)
    if info is None or tag not in info["tables"]:
        return None
    off, length = info["tables"][tag]
    if length < minimum or off + length > len(data):
        return None
    return off, length


def _put(data, off, fmt, val):
    buf = bytearray(data)
    raw = struct.pack(fmt, val)
    if off + len(raw) > len(buf):
        return None
    buf[off:off + len(raw)] = raw
    return bytes(buf)


def num_glyphs(data):
    t = _table(data, b"maxp", 6)
    if t is None:
        return None
    return struct.unpack(">H", data[t[0] + 4:t[0] + 6])[0]


def loca_entries(data):
    """[(entry_offset, value_in_bytes, fmt)] for the loca table, or []."""
    head = _table(data, b"head", 54)
    loca = _table(data, b"loca")
    if head is None or loca is None:
        return []
    long_fmt = struct.unpack(">h", data[head[0] + 50:head[0] + 52])[0]
    size, fmt, scale = (4, ">I", 1) if long_fmt else (2, ">H", 2)
    out = []
    for k in range(loca[1] // size):
        p = loca[0] + size * k
        out.append((p, struct.unpack(fmt, data[p:p + size])[0] * scale,
                    fmt))
    return out


# ---------------------------------------------------------------------
# Mutations. Each returns (bytes, description) or None when it does not
# apply to this input.

def _flip(data, rng):
    if not data:
        return None
    buf = bytearray(data)
    for _ in range(1 + rng.randrange(8)):
        buf[rng.randrange(len(buf))] = rng.randrange(256)
    return bytes(buf), "flip"


def _trunc(data, rng):
    if len(data) < 2:
        return None
    off = rng.randrange(len(data))
    return data[:off], "trunc@%d/%d" % (off, len(data))


def _delete(data, rng):
    if len(data) < 4:
        return None
    i = rng.randrange(len(data))
    j = min(len(data), i + 1 + rng.randrange(min(64, len(data) - i)))
    return data[:i] + data[j:], "del@%d+%d" % (i, j - i)


def _dup(data, rng):
    if len(data) < 2:
        return None
    i = rng.randrange(len(data))
    j = min(len(data), i + 1 + rng.randrange(min(64, len(data) - i)))
    return data[:j] + data[i:j] + data[j:], "dup@%d+%d" % (i, j - i)


def _numtables(data, rng):
    """Set the directory's numTables past, below or far past the truth."""
    info = parse_sfnt(data)
    if info is None:
        return None
    n = info["num_tables"]
    val = rng.choice((0, 1, max(0, n - 1), n + 1, 0x7FFF, 0xFFFF))
    if val == n:
        val = n + 1
    return _put(data, 4, ">H", val), "numtables=%d" % val


def _record(data, rng):
    """Point one table record's offset or length out of bounds."""
    info = parse_sfnt(data)
    if not info or not info["records"]:
        return None
    rec, tag, off, length = rng.choice(info["records"])
    field = rng.choice(("offset", "length"))
    cands = [len(data), len(data) + 1, len(data) - 1, 0xFFFFFFFF,
             0x80000000, 0, rng.randrange(len(data) + 1)]
    if field == "offset":
        others = [r[2] for r in info["records"] if r[0] != rec]
        if others:
            cands.append(rng.choice(others))
        cands.append(max(0, len(data) - length // 2))
        pos = rec + 8
    else:
        cands += [length + 1, length * 2 + 4, max(0, length - 1)]
        pos = rec + 12
    val = rng.choice(cands) & 0xFFFFFFFF
    old = off if field == "offset" else length
    if val == old:
        val = (val + 1) & 0xFFFFFFFF
    return (_put(data, pos, ">I", val),
            "record@%s:%s=0x%X" % (tag.decode("latin-1"), field, val))


def _tag(data, rng):
    """Rename one table: drop a required table or duplicate another."""
    info = parse_sfnt(data)
    if not info or not info["records"]:
        return None
    rec, tag, _off, _len = rng.choice(info["records"])
    cands = [t for _r, t, _o, _l in info["records"] if t != tag]
    cands.append(b"zzzz")
    new = rng.choice(cands)
    buf = bytearray(data)
    buf[rec:rec + 4] = new
    return bytes(buf), "tag@%s=%s" % (tag.decode("latin-1"),
                                      new.decode("latin-1"))


def _head(data, rng):
    """head: indexToLocFormat, unitsPerEm, magic or bbox edits."""
    t = _table(data, b"head", 54)
    if t is None:
        return None
    base = t[0]
    field = rng.choice(("loca-format", "units", "magic", "bbox"))
    if field == "loca-format":
        val = rng.choice((0, 1, 2, -1, 0x7FFF))
        old = struct.unpack(">h", data[base + 50:base + 52])[0]
        if val == old:
            val = 1 - old if old in (0, 1) else 0
        return _put(data, base + 50, ">h", val), "head:loca-format=%d" % val
    if field == "units":
        val = rng.choice((0, 1, 16, 0xFFFF))
        return _put(data, base + 18, ">H", val), "head:units=%d" % val
    if field == "magic":
        return _put(data, base + 12, ">I", 0), "head:magic=0"
    k = rng.randrange(4)
    val = rng.choice((-32768, 32767, 0))
    return (_put(data, base + 36 + 2 * k, ">h", val),
            "head:bbox%d=%d" % (k, val))


def _maxp(data, rng):
    """maxp.numGlyphs: 0, 1, one past or far past the loca table."""
    t = _table(data, b"maxp", 6)
    if t is None:
        return None
    n = num_glyphs(data)
    val = rng.choice((0, 1, max(0, n - 1), n + 1, 0x7FFF, 0xFFFF))
    if val == n:
        val = n + 1
    return _put(data, t[0] + 4, ">H", val), "maxp:numglyphs=%d" % val


def _hhea(data, rng):
    """hhea.numberOfHMetrics: 0, past numGlyphs or past the hmtx table."""
    t = _table(data, b"hhea", 36)
    if t is None:
        return None
    n = num_glyphs(data) or 0
    old = struct.unpack(">H", data[t[0] + 34:t[0] + 36])[0]
    val = rng.choice((0, 1, n + 1, n + 100, 0xFFFF))
    if val == old:
        val = (val + 1) & 0xFFFF
    return _put(data, t[0] + 34, ">H", val), "hhea:nhmetrics=%d" % val


def _loca(data, rng):
    """One loca entry: past glyf, before its predecessor, or odd."""
    entries = loca_entries(data)
    if len(entries) < 2:
        return None
    glyf = _table(data, b"glyf", 0)
    glyf_len = glyf[1] if glyf else 0
    k = rng.randrange(len(entries))
    pos, old, fmt = entries[k]
    long_fmt = fmt == ">I"
    prev = entries[k - 1][1] if k else 0
    cands = [glyf_len + 2, glyf_len + 1000, max(0, prev - 2), 0,
             old + 1, 0xFFFFFFFF if long_fmt else 0x1FFFE]
    val = rng.choice(cands)
    if not long_fmt:
        val = (val // 2) & 0xFFFF
    else:
        val &= 0xFFFFFFFF
    raw = val if long_fmt else val
    return (_put(data, pos, fmt, raw),
            "loca@%d=%d" % (k, val if long_fmt else 2 * val))


def _glyph_span(data, rng):
    """(start, end, gid) of a non-empty glyf entry inside the data."""
    glyf = _table(data, b"glyf")
    entries = loca_entries(data)
    if glyf is None or len(entries) < 2:
        return None
    spans = []
    for k in range(len(entries) - 1):
        a, b = entries[k][1], entries[k + 1][1]
        if 10 <= b - a and b <= glyf[1]:
            spans.append((glyf[0] + a, glyf[0] + b, k))
    if not spans:
        return None
    return rng.choice(spans)


def _glyph(data, rng):
    """Edit one glyph: contour count, endPts, instruction length."""
    span = _glyph_span(data, rng)
    if span is None:
        return None
    start, end, gid = span
    field = rng.choice(("contours", "endpts", "instr"))
    ncont = struct.unpack(">h", data[start:start + 2])[0]
    if field == "contours" or ncont < 0:
        val = rng.choice((-1, -2, 0, 0x7FFF, ncont + 1))
        if val == ncont:
            val = ncont + 1
        return (_put(data, start, ">h", val),
                "glyph@%d:contours=%d" % (gid, val))
    if field == "endpts" and ncont > 0 and start + 12 <= end:
        val = rng.choice((0xFFFF, 0, 0x7FFF))
        k = rng.randrange(ncont)
        if start + 12 + 2 * k <= end:
            return (_put(data, start + 10 + 2 * k, ">H", val),
                    "glyph@%d:endpt%d=%d" % (gid, k, val))
    ins = start + 10 + 2 * max(0, ncont)
    if ins + 2 > end:
        return None
    val = rng.choice((0xFFFF, end - ins, end - ins + 1))
    return (_put(data, ins, ">H", val & 0xFFFF),
            "glyph@%d:instr=%d" % (gid, val & 0xFFFF))


def _composite(data, rng):
    """Turn a glyph into a composite pointing at itself, a later glyph,
    a glyph past numGlyphs, or a chain whose MORE_COMPONENTS flag never
    clears before the glyph's end."""
    span = _glyph_span(data, rng)
    if span is None:
        return None
    start, end, gid = span
    n = num_glyphs(data) or 1
    kind = rng.choice(("self", "past", "unterminated", "cycle"))
    if kind == "self":
        target = gid
    elif kind == "past":
        target = rng.choice((n, n + 1, 0xFFFF))
    elif kind == "cycle":
        target = (gid + 1) % max(1, n)
    else:
        target = 0
    flags = 0x0001 | 0x0002
    if kind == "unterminated":
        flags |= 0x0020
    comp = struct.pack(">h", -1) + data[start + 2:start + 10]
    comp += struct.pack(">HHhh", flags, target, 0, 0)
    if len(comp) > end - start:
        return None
    buf = bytearray(data)
    buf[start:start + len(comp)] = comp
    if kind == "unterminated":
        # Fill the rest with more MORE_COMPONENTS records so the reader
        # runs to the glyph's end (and past it) looking for the last one.
        p = start + len(comp)
        while p + 8 <= end:
            buf[p:p + 8] = struct.pack(">HHhh", flags, 0, 0, 0)
            p += 8
    return bytes(buf), "composite@%d:%s->%d" % (gid, kind, target)


def _post(data, rng):
    """post: version, glyph count or name indices/lengths out of range."""
    t = _table(data, b"post", 32)
    if t is None:
        return None
    base, length = t
    version = struct.unpack(">I", data[base:base + 4])[0]
    field = rng.choice(("version", "count", "index", "pascal"))
    if field == "version" or version != 0x00020000 or length < 34:
        val = rng.choice((0x00010000, 0x00020000, 0x00025000,
                          0x00030000, 0x00040000, 0))
        if val == version:
            val = 0x00020000 if version != 0x00020000 else 0x00030000
        return _put(data, base, ">I", val), "post:version=0x%X" % val
    n = struct.unpack(">H", data[base + 32:base + 34])[0]
    if field == "count":
        val = rng.choice((0, n + 1, 0xFFFF, max(0, n - 1)))
        if val == n:
            val = n + 1
        return _put(data, base + 32, ">H", val), "post:count=%d" % val
    if field == "index" and n and base + 34 + 2 * n <= base + length:
        k = rng.randrange(n)
        val = rng.choice((258 + n, 0x7FFF, 0xFFFF, 257, 258 + 0x100))
        return (_put(data, base + 34 + 2 * k, ">H", val),
                "post:index%d=%d" % (k, val))
    names = base + 34 + 2 * n
    if names >= base + length:
        return None
    # Pascal string lengths: make one run past the end of the table.
    buf = bytearray(data)
    p = names
    hops = rng.randrange(8)
    while hops and p < base + length:
        p += 1 + buf[p]
        hops -= 1
    if p >= base + length:
        p = names
    buf[p] = rng.choice((255, 254, (base + length - p) & 0xFF))
    return bytes(buf), "post:pascal@%d=%d" % (p - names, buf[p])


def _cmap(data, rng):
    """cmap: subtable count/offset, format 4 segCountX2 or a range."""
    t = _table(data, b"cmap", 4)
    if t is None:
        return None
    base, length = t
    n = struct.unpack(">H", data[base + 2:base + 4])[0]
    field = rng.choice(("count", "offset", "seg", "length"))
    if field == "count" or n == 0 or base + 4 + 8 * n > base + length:
        val = rng.choice((0, n + 1, 0xFFFF))
        if val == n:
            val = n + 1
        return _put(data, base + 2, ">H", val), "cmap:count=%d" % val
    k = rng.randrange(n)
    rec = base + 4 + 8 * k
    sub = struct.unpack(">I", data[rec + 4:rec + 8])[0]
    if field == "offset" or base + sub + 8 > base + length:
        val = rng.choice((length, length + 4, 0xFFFFFFF0, 0, 1))
        return (_put(data, rec + 4, ">I", val),
                "cmap:offset%d=0x%X" % (k, val))
    s = base + sub
    fmt = struct.unpack(">H", data[s:s + 2])[0]
    if field == "length" or fmt != 4 or s + 14 > base + length:
        val = rng.choice((0, 2, 0xFFFF, length))
        return (_put(data, s + 2, ">H", val & 0xFFFF),
                "cmap:sub%d.length=%d" % (k, val & 0xFFFF))
    val = rng.choice((0, 1, 3, 0xFFFE, 0xFFFF))
    return _put(data, s + 6, ">H", val), "cmap:segx2=%d" % val


def _name(data, rng):
    """name: record count, string storage offset or one string span."""
    t = _table(data, b"name", 6)
    if t is None:
        return None
    base, length = t
    count = struct.unpack(">H", data[base + 2:base + 4])[0]
    field = rng.choice(("count", "storage", "string"))
    if field == "count" or count == 0:
        val = rng.choice((0, count + 1, 0xFFFF))
        if val == count:
            val = count + 1
        return _put(data, base + 2, ">H", val), "name:count=%d" % val
    if field == "storage":
        val = rng.choice((0, length, length + 1, 0xFFFF))
        return _put(data, base + 4, ">H", val), "name:storage=%d" % val
    k = rng.randrange(count)
    rec = base + 6 + 12 * k
    if rec + 12 > base + length:
        return None
    sub = rng.choice(("len", "off"))
    val = rng.choice((0xFFFF, length, 0xFFFE))
    pos = rec + 8 if sub == "len" else rec + 10
    return _put(data, pos, ">H", val), "name:%s%d=%d" % (sub, k, val)


def _cff(data, rng):
    """CFF table (OpenType): flip bytes in its header and INDEX counts."""
    t = _table(data, b"CFF ", 8)
    if t is None:
        return None
    base, length = t
    buf = bytearray(data)
    span = min(length, 64)
    for _ in range(1 + rng.randrange(4)):
        buf[base + rng.randrange(span)] = rng.choice(
            (0, 0xFF, rng.randrange(256)))
    return bytes(buf), "cff-head"


MUTATIONS = (_flip, _trunc, _delete, _dup, _numtables, _record, _tag,
             _head, _maxp, _hhea, _loca, _glyph, _composite, _post,
             _cmap, _name, _cff)


def mutate_with_info(data, rng):
    """Apply one mutation; return (new_bytes, description). Deterministic."""
    order = list(MUTATIONS)
    rng.shuffle(order)
    for fn in order:
        got = fn(data, rng)
        if got is not None and got[0] is not None and (
                got[0] != data or len(data) == 0):
            return got
    buf = bytearray(data or b"\x00")
    buf[0] ^= 0xFF
    return bytes(buf), "flip-fallback"


# ---------------------------------------------------------------------
# Seeds, running and classification.

def _kpse_bytes(name):
    try:
        cap = subprocess.run(["kpsewhich", name], capture_output=True,
                             text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return None
    path = (cap.stdout or "").strip().splitlines()
    if not path:
        return None
    try:
        with open(path[0], "rb") as fh:
            return fh.read()
    except OSError:
        return None


def load_seeds():
    """[(name, kind, bytes)]: kpsewhich-resolved fonts plus the synthetic."""
    seeds = [("synthetic.ttf", "ttf", synthetic_ttf())]
    for name, kind in SEED_NAMES:
        blob = _kpse_bytes(name)
        if blob:
            seeds.append((name, kind, blob))
    return seeds


def support_bytes(name):
    """cmr10.tfm / 8r.enc from the TeX Live tree (cached; None if lost)."""
    if name not in _SUPPORT_CACHE:
        _SUPPORT_CACHE[name] = _kpse_bytes(name)
    return _SUPPORT_CACHE[name]


def job_tex(mode):
    return ("\\nopagenumbers\\pdfmapline{%s}\\font\\x=fuzz \\x %s\\bye\n"
            % (MAPLINES[mode], JOB_TEXT))


def font_file(mode):
    return "fuzz.otf" if mode.startswith("otf") else "fuzz.ttf"


def is_crash(returncode, log=None):
    # Return code only: log text never decides a crash.
    return (returncode is not None
            and (returncode < 0 or returncode == 101))


def classify(returncode, log):
    """ok / font-rejected / graceful-error / crash for a finished run."""
    if fuzz_run.is_output_flood(returncode):
        return "output-flood"
    if is_crash(returncode, log):
        return "crash"
    if returncode != 0 and any(h in (log or "") for h in REJECT_HINTS):
        return "font-rejected"
    return "ok" if returncode == 0 else "graceful-error"


def signature(cls, returncode, log):
    if cls == "hang":
        return "hang"
    if cls == "output-flood":
        return "output-flood"
    return fuzz_run.crash_signature(returncode, log)


def panic_location(log):
    return fuzz_run.panic_location(log)


def candidate_env():
    env = dict(os.environ)  # FLASHTEX_POOL / FLASHTEX_FORMATS from caller
    env["SOURCE_DATE_EPOCH"] = "0"
    return env


def run_one(font_bytes, mode, candidate, timeout):
    """Run one mutated font through the candidate; (class, rc, log)."""
    workdir = tempfile.mkdtemp(prefix="ttf-fuzz-")
    try:
        with open(os.path.join(workdir, font_file(mode)), "wb") as fh:
            fh.write(font_bytes)
        with open(os.path.join(workdir, "job.tex"), "w") as fh:
            fh.write(job_tex(mode))
        for name in SUPPORT_FILES:
            blob = support_bytes(name)
            if blob:
                dest = "fuzz.tfm" if name == "cmr10.tfm" else name
                with open(os.path.join(workdir, dest), "wb") as fh:
                    fh.write(blob)
        try:
            rc, out = fuzz_run.run_capped(
                [candidate] + fuzz_run.FUZZ_SHELL_ESCAPE_FLAGS
                + ["-fmt=pdftex", "-interaction=nonstopmode", "job.tex"],
                cwd=workdir, env=candidate_env(), timeout=timeout)
            log = out.decode("utf-8", "replace")
        except subprocess.TimeoutExpired as exc:
            out = (exc.stdout or b"") + (exc.stderr or b"")
            return "hang", None, out.decode("utf-8", "replace")
        return classify(rc, log), rc, log
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def load_known_signatures(out_dir):
    known = set()
    for root, _dirs, files in os.walk(out_dir):
        for name in files:
            if not name.endswith(".json") or name == "signatures.json":
                continue
            try:
                with open(os.path.join(root, name)) as fh:
                    info = json.load(fh)
                if isinstance(info, dict) and info.get("signature"):
                    known.add(info["signature"])
            except (OSError, ValueError):
                continue
    return known


def last_line(log):
    lines = [ln for ln in (log or "").splitlines() if ln.strip()]
    return lines[-1][:200] if lines else ""


def run_fuzz(candidate, seeds, out_dir, iterations, seed, timeout):
    rng = random.Random(seed)
    known = load_known_signatures(out_dir)
    seen = set()
    counts = {c: 0 for c in CLASSES}
    for _ in range(iterations):
        name, kind, base = seeds[rng.randrange(len(seeds))]
        mode = rng.choice(MODES[kind])
        mutated, mutation = mutate_with_info(base, rng)
        cls, rc, log = run_one(mutated, mode, candidate, timeout)
        counts[cls] += 1
        if cls in STORE:
            sig = signature(cls, rc, log)
            if sig not in seen and sig not in known:
                seen.add(sig)
                digest = hashlib.sha256(mutated).hexdigest()[:16]
                cls_dir = os.path.join(out_dir, cls)
                os.makedirs(cls_dir, exist_ok=True)
                ext = os.path.splitext(font_file(mode))[1]
                with open(os.path.join(cls_dir, digest + ext), "wb") as fh:
                    fh.write(mutated)
                with open(os.path.join(cls_dir, digest + ".json"),
                          "w") as fh:
                    json.dump({"seed": seed, "origin": name, "mode": mode,
                               "maplines": MAPLINES[mode],
                               "mutation": mutation, "returncode": rc,
                               "last_line": last_line(log),
                               "panic_location": panic_location(log),
                               "signature": sig}, fh, indent=2)
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="TrueType/OpenType fuzzer")
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--iterations", type=int, default=100)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--out", default="ttf-out")
    ap.add_argument("--timeout", type=float, default=10.0)
    args = ap.parse_args(argv)
    fuzz_run.apply_fsize_limit()
    if args.iterations < 0:
        print("error: --iterations must be >= 0", file=sys.stderr)
        return 2
    seeds = load_seeds()
    counts = run_fuzz(args.candidate, seeds, args.out, args.iterations,
                      args.seed, args.timeout)
    print("done: %d iterations: %s"
          % (args.iterations,
             " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    return counts


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
