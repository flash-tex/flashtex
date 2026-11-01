#!/usr/bin/env python3
"""Generates crates/font-engine/fixtures/tfm/ (a real, licensed TFM fixture
bound to Latin Modern Roman 10 in the shape of
crates/font-resources `encoding::EncodingManifest`).

Inputs (all from a TeX Live / BasicTeX tree; nothing proprietary):
  --tfm      ec-lmr10.tfm           (T1/EC-encoded Latin Modern Roman 10)
  --font     lmroman10-regular.otf  (OpenType CFF, face 0)
  --enc      lm-ec.enc              (LM's own EC encoding vector: slot -> glyph name)
  --glyphlist glyphlist.txt         (Adobe Glyph List, for the cmap cross-check)
  --license  GUST-FONT-LICENSE.txt
  --python-with-fonttools PATH      (interpreter that can `import fontTools`)

Glyph identities come from TWO independent readers and must agree:
  1. the CFF charset of the OTF, read with fontTools (name -> original GID);
  2. this crate's own cmap parser (`cargo run --example gid_for`), reached
     through glyph name -> Unicode (Adobe Glyph List / uniXXXX names).
TFM metrics and the lig/kern program are read by the small parser below
(TeX: The Program, part 30, TFM format), not by any TeX engine.

Outputs (deterministic for fixed inputs):
  fixtures/tfm/ec-lmr10.tfm                 copy (GUST FL permits redistribution)
  fixtures/tfm/lm-ec.enc                    copy of the encoding vector (GUST FL)
  fixtures/tfm/GUST-FONT-LICENSE.txt        copy
  fixtures/tfm/ec-lmr10.encoding.json       EncodingManifest (font-resources shape)
  fixtures/tfm/ec-lmr10.metrics.json        reference metrics for the README
  fixtures/tfm/README.md                    provenance, metrics, regeneration
"""
import argparse
import hashlib
import json
import os
import shutil
import struct
import subprocess
import sys


def sha256(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def fix_word_pt(fw, design_pt):
    return fw / float(1 << 20) * design_pt


class Tfm:
    def __init__(self, data):
        (lf, lh, bc, ec, nw, nh, nd, ni, nl, nk, ne, np) = struct.unpack(">12H", data[:24])
        assert lf * 4 == len(data), "TFM length"
        self.bc, self.ec = bc, ec
        pos = 24
        header = data[pos:pos + 4 * lh]
        self.checksum = struct.unpack(">I", header[:4])[0]
        self.design_size = struct.unpack(">i", header[4:8])[0]
        pos += 4 * lh
        n = ec - bc + 1
        self.char_info = [data[pos + 4 * i:pos + 4 * i + 4] for i in range(n)]
        pos += 4 * n
        def words(count):
            nonlocal pos
            out = list(struct.unpack(">%di" % count, data[pos:pos + 4 * count]))
            pos += 4 * count
            return out
        self.width, self.height, self.depth, self.italic = words(nw), words(nh), words(nd), words(ni)
        self.lig_kern = [data[pos + 4 * i:pos + 4 * i + 4] for i in range(nl)]
        pos += 4 * nl
        self.kern = words(nk)
        self.exten = data[pos:pos + 4 * ne]
        pos += 4 * ne
        self.param = words(np)

    def metrics(self, code):
        if not (self.bc <= code <= self.ec):
            return None
        ci = self.char_info[code - self.bc]
        wi = ci[0]
        if wi == 0:
            return None
        hi, di = ci[1] >> 4, ci[1] & 15
        ii, tag = ci[2] >> 2, ci[2] & 3
        return {
            "width": self.width[wi], "height": self.height[hi], "depth": self.depth[di],
            "italic": self.italic[ii], "tag": tag, "remainder": ci[3],
        }

    def lig_kern_program(self, code):
        """Yields (next_char, op_byte, remainder) for the char's program."""
        m = self.metrics(code)
        if not m or m["tag"] != 1:
            return
        i = m["remainder"]
        skip, nxt, op, rem = self.lig_kern[i]
        if skip > 128:
            i = 256 * op + rem
        while True:
            skip, nxt, op, rem = self.lig_kern[i]
            yield nxt, op, rem
            if skip >= 128:
                return
            i += skip + 1

    def pair(self, left, right):
        for nxt, op, rem in self.lig_kern_program(left):
            if nxt == right:
                if op >= 128:
                    return ("kern", self.kern[256 * (op - 128) + rem])
                return ("lig", op, rem)
        return None


def load_enc(path):
    names = []
    for line in open(path, encoding="latin-1"):
        line = line.split("%")[0]
        for tok in line.split():
            if tok.startswith("/") and not tok.startswith("/enc"):
                names.append(tok[1:])
            elif tok.startswith("/enc"):
                continue
    assert len(names) == 256, len(names)
    return names


def load_glyphlist(path):
    m = {}
    for line in open(path, encoding="latin-1"):
        if line.startswith("#") or ";" not in line:
            continue
        name, codes = line.strip().split(";")
        cps = codes.split()
        if len(cps) == 1:
            m[name] = int(cps[0], 16)
    return m


def name_to_unicode(name, agl):
    if name in agl:
        return agl[name]
    if name.startswith("uni") and len(name) == 7:
        return int(name[3:], 16)
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tfm", required=True)
    ap.add_argument("--font", required=True)
    ap.add_argument("--enc", required=True)
    ap.add_argument("--glyphlist", required=True)
    ap.add_argument("--license", required=True)
    ap.add_argument("--python-with-fonttools", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--crate", required=True, help="crates/font-engine directory (for cargo)")
    a = ap.parse_args()

    os.makedirs(a.out, exist_ok=True)
    tfm = Tfm(open(a.tfm, "rb").read())
    names = load_enc(a.enc)
    agl = load_glyphlist(a.glyphlist)

    # 1. CFF charset via fontTools (independent of this crate).
    probe = (
        "import sys, json\n"
        "from fontTools.ttLib import TTFont\n"
        "f = TTFont(sys.argv[1])\n"
        "order = f.getGlyphOrder()\n"
        "print(json.dumps({n: i for i, n in enumerate(order)}))\n"
    )
    charset = json.loads(subprocess.check_output([a.python_with_fonttools, "-c", probe, a.font]))

    # 2. cmap route via this crate's parser.
    wanted = sorted({n for n in names if n != ".notdef"})
    cps = {}
    for n in wanted:
        cp = name_to_unicode(n, agl)
        if cp is not None:
            cps[n] = cp
    out = subprocess.check_output(
        ["cargo", "run", "--quiet", "--example", "gid_for", "--", a.font]
        + ["%04X" % cp for cp in cps.values()],
        cwd=a.crate,
    ).decode()
    cmap_gid = {}
    for line in out.strip().splitlines():
        hexcp, gid = line.split()
        cmap_gid[int(hexcp, 16)] = None if gid == "-" else int(gid)

    # The encoding vector uses AFM-style ligature names; the OTF charset uses
    # AGL-style underscore names for the same glyphs. Declared under the
    # encoding's name, with the alias recorded in the metrics/README.
    aliases = {"ff": "f_f", "fi": "f_i", "fl": "f_l", "ffi": "f_f_i", "ffl": "f_f_l"}
    used_aliases = {}
    declared = []
    disagreements = []
    missing_in_font = []
    cross_checked = 0
    for n in wanted:
        gid = charset.get(n)
        if gid is None and n in aliases and aliases[n] in charset:
            gid = charset[aliases[n]]
            used_aliases[n] = aliases[n]
        if gid is None:
            missing_in_font.append(n)
            continue
        cp = cps.get(n)
        if cp is not None and cmap_gid.get(cp) is not None:
            cross_checked += 1
            if cmap_gid[cp] != gid:
                disagreements.append((n, cp, gid, cmap_gid[cp]))
        declared.append({"glyph_name": n, "glyph_id": gid})
    declared.sort(key=lambda d: d["glyph_name"])
    declared.insert(0, {"glyph_name": ".notdef", "glyph_id": 0})
    if disagreements:
        print("charset/cmap disagreement:", disagreements, file=sys.stderr)
        sys.exit(1)

    encoding = []
    for code, n in enumerate(names):
        if n == ".notdef":
            continue
        if n in missing_in_font:
            continue  # slot stays undeclared; the adapter rejects it explicitly
        encoding.append({"code": code, "glyph_name": n})

    manifest = {
        "tfm_sha256": sha256(a.tfm),
        "font_sha256": sha256(a.font),
        "face_index": 0,
        "encoding": encoding,
        "declared_glyphs": declared,
    }
    with open(os.path.join(a.out, "ec-lmr10.encoding.json"), "w") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")

    shutil.copyfile(a.tfm, os.path.join(a.out, "ec-lmr10.tfm"))
    shutil.copyfile(a.enc, os.path.join(a.out, "lm-ec.enc"))
    shutil.copyfile(a.license, os.path.join(a.out, "GUST-FONT-LICENSE.txt"))

    design_pt = tfm.design_size / float(1 << 20)
    def slot_of(name):
        return names.index(name)
    ref = {"design_size_fix_word": tfm.design_size, "design_size_pt": design_pt, "checksum": tfm.checksum, "slots": {}}
    for name in ["A", "V", "a", "eacute", "f", "i", "fi"]:
        code = slot_of(name)
        m = tfm.metrics(code)
        ref["slots"][name] = {
            "code": code,
            "glyph_id": charset.get(name, charset.get(aliases.get(name, ""))),
            "width": m["width"], "height": m["height"], "depth": m["depth"], "italic": m["italic"],
            "width_pt": fix_word_pt(m["width"], design_pt),
            "height_pt": fix_word_pt(m["height"], design_pt),
            "depth_pt": fix_word_pt(m["depth"], design_pt),
        }
    ref["ligature_f_i"] = tfm.pair(slot_of("f"), slot_of("i"))
    ref["kern_A_V"] = tfm.pair(slot_of("A"), slot_of("V"))
    kern_av = ref["kern_A_V"]
    ref["kern_A_V_pt"] = fix_word_pt(kern_av[1], design_pt) if kern_av and kern_av[0] == "kern" else None
    ref["cross_checked_names"] = cross_checked
    ref["charset_name_aliases"] = used_aliases
    ref["names_absent_from_font"] = missing_in_font
    ref["params"] = tfm.param
    with open(os.path.join(a.out, "ec-lmr10.metrics.json"), "w") as f:
        json.dump(ref, f, indent=2)
        f.write("\n")
    print(json.dumps(ref, indent=2))


if __name__ == "__main__":
    main()
