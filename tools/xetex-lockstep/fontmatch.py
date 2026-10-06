#!/usr/bin/env python3
"""Font lookup by name: TeX Live's xetex against FlashTeX's port.

Measures `crates/flashtex-xetex/src/fontmgr` (XeTeX's font lookup over the
platform-free font index, docs/design/xetex/PLAN.md §3.1 and §5) against
TeX Live 2026's `xetex -no-pdf` (an oracle only, never in the product
path). For each `\\font` name in the list, in a fresh run of each:

  * xetex runs `\\font\\x="NAME" SIZE \\message{\\fontname\\x}
    \\shipout\\hbox{\\x A}` in INITEX, and the face it chose is read from the
    XDV's define_native_font record (opcode 252: k[4] size[4] flags[2]
    len[1] path[len] index[4] ..., XeTeX_ext.c's makefontdef) in the
    postamble; `\\fontname` gives the name XeTeX left in name_of_file;
    with `\\XeTeXtracingfonts=1` (e-TeX mode) the log's `-> ...` line
    tells a face findFont found but could not load from a name it did not
    find (the line's path itself is garbage on macOS: XeTeXFontMgr.cpp
    prints the c_str() of a temporary std::string after it is destroyed);
  * the port answers through `examples/find_font` (`path#index`, the
    name_of_file).

The face (path and index) is the comparison; the name_of_file and the size
the font is loaded at (the XDV record's, which a `scaled` size takes from
the 'size' feature's design size) are compared too and reported
separately. Every difference is printed with both faces
(PostScript name, full name, family and style per the port's catalog).

Usage:
  fontmatch.py [--names FILE] [--known FILE] [--finder BIN] [--xetex BIN]
               [--jobs N] [--json OUT]

`--names` defaults to fontnames.txt here: one lookup per line, `NAME` or
`NAME<TAB>SIZE` (`12` for `at 12pt`, `scaled 1200`); `#` comments.
`--known` defaults to fontmatch-known.txt here: the measured differences
(`face NAME` or `name NAME`, tab-separated), printed as KNOWN; the run
fails on any other difference and on a listed one that no longer differs.
`--finder` defaults to crates/flashtex-xetex/target/release/examples/
find_font (build it with `cargo build --release --example find_font` in
crates/flashtex-xetex). Stdlib only; MIT, like the rest of tools/.
"""
import argparse
import concurrent.futures
import json
import os
import re
import struct
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
XETEX = "/usr/local/texlive/2026/bin/universal-darwin/xetex"
FINDER = os.path.join(ROOT, "crates", "flashtex-xetex", "target", "release", "examples", "find_font")
NAMES = os.path.join(HERE, "fontnames.txt")
KNOWN = os.path.join(HERE, "fontmatch-known.txt")

# XDV define_native_font flags (XeTeX_ext.c).
XDV_FLAG_COLORED = 0x0200
XDV_FLAG_EXTEND = 0x1000
XDV_FLAG_SLANT = 0x2000
XDV_FLAG_EMBOLDEN = 0x4000


def read_names(path):
    out = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            name, _, size = line.partition("\t")
            out.append((name, size.strip()))
    return out


def read_known(path):
    """{(kind, name, size)} from a known-differences file."""
    out = set()
    if not path or not os.path.exists(path):
        return out
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            kind, _, rest = line.partition("\t")
            name, _, size = rest.partition("\t")
            if kind not in ("face", "name"):
                raise SystemExit("%s: unknown kind %r" % (path, kind))
            out.add((kind, name, size.strip()))
    return out


def tex_size(size):
    if not size:
        return ""
    if size.startswith("scaled"):
        return " scaled " + size[len("scaled"):].strip()
    return " at " + size.rstrip("pt") + "pt"


def native_fonts(xdv):
    """The define_native_font records of an XDV file's postamble:
    [(k, size, flags, path, index)]."""
    end = len(xdv)
    while end > 0 and xdv[end - 1] == 223:
        end -= 1
    # post_post: 249 q[4] i[1]
    if end < 6 or xdv[end - 6] != 249:
        raise ValueError("no post_post")
    q = struct.unpack(">I", xdv[end - 5:end - 1])[0]
    if xdv[q] != 248:
        raise ValueError("no post at %d" % q)
    p = q + 29
    fonts = []
    while True:
        op = xdv[p]
        if op == 249:
            break
        if 243 <= op <= 246:  # fnt_def1..4
            n = op - 242
            p += 1 + n + 12
            a, l = xdv[p], xdv[p + 1]
            p += 2 + a + l
        elif op == 252:
            k, size, flags, ln = struct.unpack(">IiHB", xdv[p + 1:p + 12])
            path = xdv[p + 12:p + 12 + ln].decode("utf-8", "replace")
            index = struct.unpack(">I", xdv[p + 12 + ln:p + 16 + ln])[0]
            p += 16 + ln
            for flag in (XDV_FLAG_COLORED, XDV_FLAG_EXTEND, XDV_FLAG_SLANT, XDV_FLAG_EMBOLDEN):
                if flags & flag:
                    p += 4
            fonts.append((k, size, flags, path, index))
        elif op == 138:  # nop
            p += 1
        else:
            raise ValueError("opcode %d in postamble" % op)
    return fonts


def unhat(s):
    """TeX's ^^xx printing of the bytes of a UTF-8 name, undone."""
    raw = re.sub(rb"\^\^([0-9a-f]{2})", lambda m: bytes([int(m.group(1), 16)]), s.encode("latin-1", "replace"))
    return raw.decode("utf-8", "replace")


def run_xetex(xetex, name, size):
    with tempfile.TemporaryDirectory(prefix="fm") as d:
        tex = os.path.join(d, "t.tex")
        with open(tex, "w", encoding="utf-8") as f:
            f.write("\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\XeTeXtracingfonts=1\n")
            f.write('\\font\\x="%s"%s\\relax\n' % (name, tex_size(size)))
            f.write("\\message{[[FONTNAME:\\fontname\\x]]}\n")
            f.write("\\shipout\\hbox{\\x A}\n\\end\n")
        env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TZ="UTC")
        subprocess.run(
            [xetex, "-ini", "-etex", "-no-pdf", "-interaction=nonstopmode", "-cnf-line=max_print_line=100000", "t.tex"],
            cwd=d, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=300,
        )
        log = open(os.path.join(d, "t.log"), encoding="latin-1").read()
        # \XeTeXtracingfonts: findFont prints "-> <path>" when the lookup
        # succeeds, before the font is loaded (XeTeXFontMgr.cpp), so a face
        # found but not loadable is told apart from a name not found. The
        # path printed is a dangling pointer on macOS, so only the line's
        # presence is used.
        traced = re.search(r"^ ?-> (?!font not found)", log, re.M) is not None
        if "not loadable" in log:
            return {"face": None, "fontname": None, "traced": traced}
        m = re.search(r"\[\[FONTNAME:(.*?)\]\]", log, re.S)
        fontname = unhat(m.group(1).replace("\n", "")) if m else None
        # \fontname quotes a native font's name (with " or, when the name
        # has a ", with '), then appends " at <size>pt" when the size is
        # not the design size (xetex.web, font_name_code).
        if fontname:
            fontname = re.sub(r" at [0-9.]+pt$", "", fontname)
            if len(fontname) >= 2 and fontname[0] == fontname[-1] and fontname[0] in "\"'":
                fontname = fontname[1:-1]
        xdv = os.path.join(d, "t.xdv")
        if not os.path.exists(xdv):
            return {"face": None, "fontname": fontname, "traced": traced}
        fonts = native_fonts(open(xdv, "rb").read())
        if not fonts:
            return {"face": None, "fontname": fontname, "traced": traced}
        _, size, _, path, index = fonts[0]
        return {"face": "%s#%d" % (path, index), "fontname": fontname, "traced": traced, "size": size}


def run_finder(finder, entries):
    text = "".join("%s\t%s\n" % (n, s) if s else "%s\n" % n for n, s in entries)
    out = subprocess.run([finder], input=text, capture_output=True, text=True, check=True).stdout
    rows = {}
    for (name, size), line in zip(entries, out.splitlines()):
        parts = line.split("\t")
        face = None if parts[1] == "-" else parts[1]
        rows[(name, size)] = {"face": face, "name_of_file": None if parts[2] == "-" else parts[2],
                              "size": None if parts[5] == "-" else int(parts[5])}
    return rows


def catalog(finder):
    out = subprocess.run([finder, "--list"], capture_output=True, text=True, check=True).stdout
    faces = {}
    for line in out.splitlines():
        face, ps, full, fam, style = (line.split("\t") + [""] * 5)[:5]
        faces[face] = "%s (full %r, family %r, style %r)" % (ps, full.split("|")[0], fam.split("|")[0], style.split("|")[0])
    return faces


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--names", default=NAMES)
    ap.add_argument("--known", default=KNOWN)
    ap.add_argument("--finder", default=FINDER)
    ap.add_argument("--xetex", default=XETEX)
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--json")
    a = ap.parse_args()
    for exe in (a.xetex, a.finder):
        if not os.access(exe, os.X_OK):
            print("missing %s" % exe, file=sys.stderr)
            return 2
    entries = read_names(a.names)
    known = read_known(a.known)
    seen_known = set()
    unexpected = []
    ours = run_finder(a.finder, entries)
    faces = catalog(a.finder)
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
        theirs = dict(zip(entries, ex.map(lambda e: run_xetex(a.xetex, *e), entries)))
    same = 0
    name_same = 0
    results = []
    lookup_only = 0
    size_same = size_cmp = size_aat = 0
    for e in entries:
        o, t = ours[e], theirs[e]
        t_face = t["face"]
        o_face = o["face"]
        is_file = e[0].startswith("[")
        note = ""
        if t_face is None and t["traced"] and not is_file:
            # xetex's lookup found a face but loading it failed: which face
            # is not observable, so this lookup is not counted either way.
            t_face = "(found, not loadable)"
            equal = None
            note = "  (unverifiable: xetex found a face but could not load it)"
            lookup_only += 1
        else:
            equal = o_face == t_face
        same += bool(equal)
        label = e[0] + ("  [%s]" % e[1] if e[1] else "")
        t_name = t["fontname"]
        o_name = o["name_of_file"] if not is_file else (e[0] if o_face else None)
        name_equal = (t_name == o_name) if (t["face"] or o_face) and not note else True
        name_same += name_equal
        size_equal = None
        if equal and t.get("size") is not None:
            size_cmp += 1
            # A font macOS's xetex shapes with Core Text (AAT: no GSUB/GPOS
            # and no /OT) records its size in big points: loadAATfont makes
            # the CTFont at TeXtoPSPoints(Fix2D(scaled_size)) and makefontdef
            # writes D2Fix(CTFontGetSize) (XeTeX_mac.c, XeTeX_ext.c).
            aat = int(o["size"] / 65536.0 * 72.0 / 72.27 * 65536.0 + 0.5)
            size_equal = t["size"] == o["size"] or t["size"] == aat
            size_aat += t["size"] != o["size"] and t["size"] == aat
            size_same += size_equal
        results.append({"name": e[0], "size": e[1], "xetex": t_face, "port": o_face, "equal": equal,
                        "xetex_fontname": t_name, "port_name_of_file": o_name, "name_equal": name_equal,
                        "xetex_size": t.get("size"), "port_size": o["size"], "size_equal": size_equal})
        mark = "same" if equal else ("n/a" if equal is None else "DIFF")
        diffs = [k for k, d in (("face", equal is False), ("name", not name_equal)) if d]
        listed = [k for k in diffs if (k, e[0], e[1]) in known]
        seen_known.update((k, e[0], e[1]) for k in listed)
        unexpected += [(k, label) for k in diffs if k not in listed]
        if diffs:
            mark = "KNOWN" if len(listed) == len(diffs) else "DIFF"
        print("%-5s %-44s %s%s" % (mark, label, t_face or "(not found)", note))
        if equal is False:
            print("        xetex: %s -> %s" % (t_face or "(not found)", faces.get(t_face, "(not in the port's catalog)") if t_face else ""))
            print("        port:  %s -> %s" % (o_face or "(not found)", faces.get(o_face, "") if o_face else ""))
        if size_equal is False:
            print("        size: xetex %s sp, port %s sp" % (t["size"], o["size"]))
        if not name_equal:
            print("        name_of_file: xetex %r, port %r" % (t_name, o_name))
    print()
    print("faces equal: %d of %d (and %d unverifiable: found by xetex's lookup, not loadable)" % (same, len(entries) - lookup_only, lookup_only))
    print("name_of_file equal: %d of %d" % (name_same, len(entries)))
    print("size equal (XDV define_native_font size, faces equal and loaded): %d of %d (%d in big points: AAT-loaded)" % (size_same, size_cmp, size_aat))
    stale = sorted(known - seen_known)
    print("known differences: %d of %d listed seen; %d not listed" % (len(seen_known), len(known), len(unexpected)))
    for kind, label in unexpected:
        print("  not listed: %s %s" % (kind, label))
    for kind, name, size in stale:
        print("  listed, no longer differs (or not in --names): %s %s" % (kind, name + ("  [%s]" % size if size else "")))
    if a.json:
        with open(a.json, "w", encoding="utf-8") as f:
            json.dump(results, f, indent=1, ensure_ascii=False)
    return 0 if not unexpected and not stale and size_same == size_cmp else 1


if __name__ == "__main__":
    sys.exit(main())
