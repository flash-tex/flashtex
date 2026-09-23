#!/usr/bin/env python3
"""Regenerates crates/font-engine/src/generated.rs.

Inputs (never committed to this repository):
  --afm-dir DIR   directory holding the Adobe Core 14 AFM files
                  (Times-Roman.afm, Times-Bold.afm, Times-Italic.afm,
                  Times-BoldItalic.afm, Helvetica.afm, Courier.afm, Symbol.afm).
                  Adobe distributed these with a licence permitting free
                  copying provided the copyright notices are retained; the
                  matplotlib package ships an unmodified copy under
                  mpl-data/fonts/pdfcorefonts and TeX Live ships the same
                  width tables as fonts/afm/adobe/{times,symbol}/*.afm.
  --glyphlist F   Adobe Glyph List (glyphlist.txt), e.g. from TeX Live
                  texmf-dist/fonts/map/glyphlist/glyphlist.txt.

Everything else comes from the Python standard library `unicodedata` module,
whose Unicode version is recorded in the output header. Running this script
twice on the same inputs produces byte-identical output.
"""
import argparse
import hashlib
import os
import re
import sys
import unicodedata

FACES = [
    ("TIMES_ROMAN", "Times-Roman"),
    ("TIMES_BOLD", "Times-Bold"),
    ("TIMES_ITALIC", "Times-Italic"),
    ("TIMES_BOLD_ITALIC", "Times-BoldItalic"),
    ("HELVETICA", "Helvetica"),
    ("COURIER", "Courier"),
    ("SYMBOL", "Symbol"),
]

# Extra code points that reuse an existing glyph's width. Left = alias,
# right = AFM glyph name. These are deliberate, documented conveniences.
ALIASES = [
    (0x00A0, "space"),        # no-break space
    (0x00AD, "hyphen"),       # soft hyphen
    (0x2010, "hyphen"),       # hyphen
    (0x2011, "hyphen"),       # non-breaking hyphen
    (0x0394, "Delta"),        # Greek capital delta (AGL maps Delta to U+2206)
    (0x03A9, "Omega"),        # Greek capital omega (AGL maps Omega to U+2126)
    (0x03BC, "mu"),           # Greek small mu (AGL maps mu to U+00B5)
    (0x02C9, "macron"),       # modifier letter macron
]


def load_glyphlist(path):
    names = {}
    with open(path, encoding="latin-1") as f:
        for line in f:
            if line.startswith("#") or ";" not in line:
                continue
            name, codes = line.strip().split(";")
            cps = codes.split()
            if len(cps) == 1:
                names[name] = int(cps[0], 16)
    return names


def parse_afm(path):
    header = {}
    chars = []  # (code, width, name)
    kerns = []  # (left_name, right_name, amount)
    with open(path, encoding="latin-1") as f:
        for line in f:
            line = line.rstrip("\n")
            if line.startswith("C "):
                fields = [p.strip() for p in line.split(";")]
                code = int(fields[0].split()[1])
                width = int(fields[1].split()[1])
                name = fields[2].split()[1]
                chars.append((code, width, name))
            elif line.startswith("KPX "):
                _, l, r, amt = line.split()
                kerns.append((l, r, int(amt)))
            elif line and not line.startswith(("Comment", "Start", "End", "KPX")):
                key, _, val = line.partition(" ")
                header[key] = val.strip()
    return header, chars, kerns


def rust_str(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def write_or_check(path, text, check):
    """Write `text` to `path`, or with `--check` compare instead.

    `--check` exit codes (the same for every generator; see
    scripts/check-generated.py): 0 the committed file is what this generator
    produces now, 1 it is stale (a diff excerpt is printed), 2 it cannot be
    checked here (a tool or input is missing; the message says which).
    """
    data = text.encode("utf-8")
    rel = os.path.relpath(path)
    if not check:
        with open(path, "wb") as fh:
            fh.write(data)
        return 0
    try:
        with open(path, "rb") as fh:
            old = fh.read()
    except FileNotFoundError:
        print("STALE: %s does not exist" % rel)
        return 1
    if old == data:
        print("up to date: %s" % rel)
        return 0
    import difflib
    diff = list(difflib.unified_diff(old.decode("utf-8", "replace").splitlines(),
                                     text.splitlines(), "committed", "regenerated",
                                     lineterm="", n=0))
    for line in diff[:40]:
        print(line)
    if len(diff) > 40:
        print("... %d more diff lines" % (len(diff) - 40))
    print("STALE: %s differs from what %s generates now" % (rel, os.path.relpath(sys.argv[0])))
    return 1


def require_tools(*tools):
    """Exit 2 (cannot check here) when a TeX tool is not on PATH."""
    import shutil
    missing = [t for t in tools if not shutil.which(t)]
    if missing:
        print("CANNOT CHECK: %s not found on PATH (needs TeX Live 2026)" % ", ".join(missing))
        sys.exit(2)


def check_environment(args):
    """`--check` preconditions: the inputs and the Unicode version the committed
    file records. Anything else would report a difference that says nothing
    about staleness, so it exits 2 (cannot check here) instead."""
    try:
        with open(args.out, encoding="utf-8") as fh:
            header = fh.read(4096)
    except FileNotFoundError:
        return
    m = re.search(r"Python unicodedata (\S+) for", header)
    if m and m.group(1) != unicodedata.unidata_version:
        print("CANNOT CHECK: %s was generated with Unicode %s; this Python has %s"
              % (os.path.relpath(args.out), m.group(1), unicodedata.unidata_version))
        sys.exit(2)
    recorded = dict(re.findall(r"^//   (\S+)\s+([0-9a-f]{64})$", header, re.M))
    files = [("%s.afm" % afm, os.path.join(args.afm_dir, afm + ".afm")) for _, afm in FACES]
    files.append(("glyphlist.txt", args.glyphlist))
    for name, path in files:
        try:
            digest = hashlib.sha256(open(path, "rb").read()).hexdigest()
        except OSError as e:
            print("CANNOT CHECK: %s" % e)
            sys.exit(2)
        if name in recorded and recorded[name] != digest:
            print("CANNOT CHECK: %s has SHA-256 %s, but the committed file was generated from %s"
                  % (path, digest, recorded[name]))
            sys.exit(2)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--afm-dir", required=True)
    ap.add_argument("--glyphlist", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--check", action="store_true",
                    help="compare with --out instead of writing it")
    args = ap.parse_args()
    if args.check:
        check_environment(args)

    agl = load_glyphlist(args.glyphlist)
    out = []
    w = out.append
    w("// GENERATED by tools/gen_tables.py -- do not edit by hand.")
    w("// Regenerate with the command recorded in README.md (section \"Reproducibility\").")
    w("//")
    w("// Sources: Adobe Core 14 AFM files (widths in 1/1000 em, kerning pairs in")
    w("// 1/1000 em), the Adobe Glyph List for glyph name -> Unicode mapping, and")
    w("// Python unicodedata %s for canonical compositions." % unicodedata.unidata_version)
    w("// Input SHA-256 digests:")
    digests = []
    for _, afm in FACES:
        p = "%s/%s.afm" % (args.afm_dir, afm)
        digests.append((afm + ".afm", hashlib.sha256(open(p, "rb").read()).hexdigest()))
    digests.append(("glyphlist.txt", hashlib.sha256(open(args.glyphlist, "rb").read()).hexdigest()))
    for name, d in digests:
        w("//   %-22s %s" % (name, d))
    w("")
    w("pub const UNICODE_VERSION: &str = %s;" % rust_str(unicodedata.unidata_version))
    w("")
    w("/// Header metrics of one AFM file, all in 1/1000 em (angles in degrees).")
    w("#[derive(Debug, Clone, Copy, PartialEq)]")
    w("pub struct AfmHeader {")
    w("    pub font_name: &'static str,")
    w("    pub family_name: &'static str,")
    w("    pub weight: &'static str,")
    w("    pub italic_angle: f64,")
    w("    pub is_fixed_pitch: bool,")
    w("    pub bbox: [i16; 4],")
    w("    pub cap_height: i16,")
    w("    pub x_height: i16,")
    w("    pub ascender: i16,")
    w("    pub descender: i16,")
    w("    pub underline_position: i16,")
    w("    pub underline_thickness: i16,")
    w("    pub std_vw: i16,")
    w("    pub encoding_scheme: &'static str,")
    w("}")
    w("")
    unmapped_all = {}
    for const, afm in FACES:
        header, chars, kerns = parse_afm("%s/%s.afm" % (args.afm_dir, afm))
        name_to_width = {n: wd for _, wd, n in chars}
        widths = {}
        unmapped = []
        for code, width, name in chars:
            cp = agl.get(name)
            if cp is None:
                unmapped.append(name)
                continue
            if cp in widths and widths[cp] != width:
                print("conflict %s U+%04X %s" % (afm, cp, name), file=sys.stderr)
            widths.setdefault(cp, width)
        for cp, name in ALIASES:
            if cp not in widths and name in name_to_width:
                widths[cp] = name_to_width[name]
        unmapped_all[afm] = unmapped
        bbox = [int(x) for x in header["FontBBox"].split()]
        w("pub const %s_HEADER: AfmHeader = AfmHeader {" % const)
        w("    font_name: %s," % rust_str(header["FontName"]))
        w("    family_name: %s," % rust_str(header["FamilyName"]))
        w("    weight: %s," % rust_str(header["Weight"]))
        w("    italic_angle: %s_f64," % header["ItalicAngle"])
        w("    is_fixed_pitch: %s," % header["IsFixedPitch"])
        w("    bbox: [%d, %d, %d, %d]," % tuple(bbox))
        w("    cap_height: %s," % header.get("CapHeight", "0"))
        w("    x_height: %s," % header.get("XHeight", "0"))
        w("    ascender: %s," % header.get("Ascender", "0"))
        w("    descender: %s," % header.get("Descender", "0"))
        w("    underline_position: %s," % header["UnderlinePosition"])
        w("    underline_thickness: %s," % header["UnderlineThickness"])
        w("    std_vw: %s," % header.get("StdVW", "0"))
        w("    encoding_scheme: %s," % rust_str(header["EncodingScheme"]))
        w("};")
        w("")
        w("/// (code point, advance width) sorted by code point. %d entries." % len(widths))
        w("pub static %s_WIDTHS: [(u32, u16); %d] = [" % (const, len(widths)))
        for cp in sorted(widths):
            w("    (0x%04X, %d)," % (cp, widths[cp]))
        w("];")
        w("")
        pairs = []
        for l, r, amt in kerns:
            lc, rc = agl.get(l), agl.get(r)
            if lc is None or rc is None or lc > 0xFFFF or rc > 0xFFFF:
                continue
            pairs.append((lc, rc, amt))
        pairs.sort()
        w("/// (left, right, adjustment) sorted by (left, right). %d entries." % len(pairs))
        w("pub static %s_KERNS: [(u16, u16, i16); %d] = [" % (const, len(pairs)))
        for lc, rc, amt in pairs:
            w("    (0x%04X, 0x%04X, %d)," % (lc, rc, amt))
        w("];")
        w("")
    w("/// AFM glyph names with no single-code-point Adobe Glyph List entry; these")
    w("/// have no width in the tables above and shape as missing glyphs.")
    w("pub static UNMAPPED_GLYPH_NAMES: &[(&str, &[&str])] = &[")
    for afm in sorted(unmapped_all):
        w("    (%s, &[%s])," % (rust_str(afm), ", ".join(rust_str(n) for n in sorted(unmapped_all[afm]))))
    w("];")
    w("")
    # Canonical pairwise compositions (primary composites only).
    comps = []
    for cp in range(0x00C0, 0x3000):
        ch = chr(cp)
        d = unicodedata.decomposition(ch)
        if not d or d.startswith("<"):
            continue
        parts = d.split()
        if len(parts) != 2:
            continue
        base, mark = int(parts[0], 16), int(parts[1], 16)
        if unicodedata.category(chr(mark)) not in ("Mn", "Mc"):
            continue
        # Skip composition exclusions: NFC must recompose the pair to cp.
        if unicodedata.normalize("NFC", chr(base) + chr(mark)) != ch:
            continue
        comps.append((base, mark, cp))
    comps.sort()
    w("/// Canonical pairwise compositions (base, combining mark) -> precomposed,")
    w("/// sorted by (base, mark). Only primary composites; %d entries." % len(comps))
    w("pub static COMPOSITIONS: [(u32, u32, u32); %d] = [" % len(comps))
    for base, mark, cp in comps:
        w("    (0x%04X, 0x%04X, 0x%04X)," % (base, mark, cp))
    w("];")
    w("")
    marks = [cp for cp in range(0x0300, 0x0370) if unicodedata.category(chr(cp)) == "Mn"]
    marks += [cp for cp in range(0x1AB0, 0x1B00) if unicodedata.category(chr(cp)) == "Mn"]
    marks += [cp for cp in range(0x1DC0, 0x1E00) if unicodedata.category(chr(cp)) == "Mn"]
    marks += [cp for cp in range(0x20D0, 0x2100) if unicodedata.category(chr(cp)) == "Mn"]
    marks += [cp for cp in range(0xFE20, 0xFE30) if unicodedata.category(chr(cp)) == "Mn"]
    w("/// Non-spacing combining marks (General Category Mn) in the blocks the")
    w("/// shaper treats as marks: Combining Diacritical Marks (+Extended,")
    w("/// Supplement, for Symbols, Half Marks). %d entries, sorted." % len(marks))
    w("pub static COMBINING_MARKS: [u32; %d] = [" % len(marks))
    for cp in marks:
        w("    0x%04X," % cp)
    w("];")
    return write_or_check(args.out, "\n".join(out) + "\n", args.check)


if __name__ == "__main__":
    sys.exit(main())
