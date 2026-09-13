#!/usr/bin/env python3
r"""Generate src/amssymb.rs: the amssymb/amsfonts math symbol table.

Development-time extraction only (no TeX at runtime). Sources, resolved with
`kpsewhich`:

* `amssymb.sty` / `amsfonts.sty`: every `\DeclareMathSymbol` and
  `\DeclareMathDelimiter` of the `AMSa` (msam) / `AMSb` (msbm) symbol fonts,
  giving the command, its math class and its font slot (cited by file:line);
  plus the `\mathhexbox` symbols (`\yen`, `\checkmark`, `\circledR`,
  `\maltese`, amsfonts.sty 64-75) and the pieces of the dashed arrows
  (`\dabar@` "39 with heads "4B/"4C, amsfonts.sty 87-95) and of the extra-wide
  accents (msbm "5B/"5D, amsfonts.sty 78-86), which are not commands.
* `msam10.tfm` / `msbm10.tfm`: the character width, for the compiler's own
  (non-TFM) layout only; render-pipeline sets every size from the TFMs.
* `unicode-math-table.tex`: the Unicode character standing for each command,
  where it names one; `MANUAL` below chooses the rest.
* `apps/mac/Fonts/latinmodern-math.otf` / `NewCMMath-Regular.otf`: which face
  carries every character (Latin Modern Math first) and its advance.

Nothing declared by those two files is skipped. An earlier revision dropped
`\angle`, `\hbar`, `\mho`, `\sqsubset`, `\sqsupset` and `\rightleftharpoons`
as "kernel commands amsfonts only redefines", which was wrong twice over,
checked against pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025):

* `\mho`, `\sqsubset` and `\sqsupset` are not kernel commands at all.
  `latex.ltx` 14145/14150/14151 define them as `\not@base` stubs, so base
  LaTeX2e answers `! LaTeX Error: Command \sqsubset not provided in base
  LaTeX2e.`; `amsfonts.sty` 99-101 is what actually provides them.
* `\angle`, `\hbar` and `\rightleftharpoons` are kernel commands
  (`fontmath.ltx` 243, 241, 361), but they are kernel *composites* --
  `\angle` an `\ialign` of rules, `\hbar` `\mathchar'26\mkern-9mu h`,
  `\rightleftharpoons` a `\mathpalette` stack -- and amsfonts replaces each
  with a single msam/msbm glyph of different metrics. Measured at 10pt,
  base vs amssymb: `\angle` 6.37344pt vs 7.22223pt, `\hbar` 5.76172pt vs
  5.40280pt, `\rightleftharpoons` the same 10.00002pt width but height
  5.33438pt/depth 0.33437pt vs 5.22394pt/0.13539pt. So "keeps its kernel
  glyph" was not a description of what pdfLaTeX does under amssymb.

Usage: python3 scripts/gen_amssymb.py > src/amssymb.rs
"""
import os
import re
import struct
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "crates", "math-layout", "tools"))
import gen_cm_tfm  # noqa: E402

FONTS = os.path.join(REPO, "apps", "mac", "Fonts")


def kpse(name):
    return subprocess.check_output(["kpsewhich", name]).decode().strip()


# Commands unicode-math does not name, or names with a different design
# (a base plus U+0338 where Unicode has no precomposed negation).
MANUAL = {
    "square": ["25A1"], "blacksquare": ["25A0"], "centerdot": ["2B1D", "22C5"], "lozenge": ["25CA"],
    "blacklozenge": ["29EB", "2B27"], "circlearrowright": ["21BB"], "circlearrowleft": ["21BA"],
    "doteqdot": ["2251"], "varpropto": ["221D"], "smallsmile": ["2323"], "smallfrown": ["2322"],
    "circledS": ["24C8"], "lvertneqq": ["2268"], "gvertneqq": ["2269"],
    "nleqslant": ["2A7D 0338"], "ngeqslant": ["2A7E 0338"], "npreceq": ["22E0"], "nsucceq": ["22E1"],
    "nleqq": ["2266 0338"], "ngeqq": ["2267 0338"], "varsubsetneq": ["228A"], "varsupsetneq": ["228B"],
    "nsubseteqq": ["2AC5 0338"], "nsupseteqq": ["2AC6 0338"], "varsubsetneqq": ["2ACB"],
    "varsupsetneqq": ["2ACC"], "nshortmid": ["2224"], "nshortparallel": ["2226"],
    "ntriangleleft": ["22EA"], "ntriangleright": ["22EB"], "eth": ["00F0"], "shortmid": ["2223"],
    "shortparallel": ["2225"], "smallsetminus": ["2216"], "thicksim": ["223C"], "thickapprox": ["2248"],
    "digamma": ["03DD"], "varkappa": ["03F0"], "backepsilon": ["03F6"],
    # amsfonts.sty 98 puts `\hbar` on msbm "7E, the slot amssymb.sty also
    # gives `\hslash`; unicode-math names that character only as `\hslash`.
    "hbar": ["210F"],
}
EXTRA = [
    ("yen", "ord", "msam", 0x55, ["00A5"], "amsfonts.sty:64"),
    ("checkmark", "ord", "msam", 0x58, ["2713"], "amsfonts.sty:67"),
    ("circledR", "ord", "msam", 0x72, ["00AE"], "amsfonts.sty:70"),
    ("maltese", "ord", "msam", 0x7A, ["2720"], "amsfonts.sty:73"),
    # Not commands (names with `@`): pieces other constructions place.
    ("dabar@", "ord", "msam", 0x39, [""], "amsfonts.sty:89"),
    ("dashrightarrow@", "ord", "msam", 0x4B, ["21E2"], "amsfonts.sty:90"),
    ("dashleftarrow@", "ord", "msam", 0x4C, ["21E0"], "amsfonts.sty:92"),
    ("widehat@", "ord", "msbm", 0x5B, ["0302"], "amsfonts.sty:81"),
    ("widetilde@", "ord", "msbm", 0x5D, ["0303"], "amsfonts.sty:85"),
    # msbm10.tfm successors of "5B/"5D (`\mathaccent` grows along them).
    ("widehat@@", "ord", "msbm", 0x5C, ["0302"], "msbm10.tfm"),
    ("widetilde@@", "ord", "msbm", 0x5E, ["0303"], "msbm10.tfm"),
]
ALIASES = [("restriction", "upharpoonright"), ("Doteq", "doteqdot"), ("doublecup", "Cup"),
           ("doublecap", "Cap"), ("llless", "lll"), ("gggtr", "ggg")]


def otf(path):
    b = open(path, "rb").read()
    n = struct.unpack(">H", b[4:6])[0]
    t = {}
    for i in range(n):
        tag, _, off, ln = struct.unpack(">4sIII", b[12 + 16 * i:28 + 16 * i])
        t[tag.decode()] = off
    off = t["cmap"]
    cmap = {}
    for i in range(struct.unpack(">H", b[off + 2:off + 4])[0]):
        _, _, so = struct.unpack(">HHI", b[off + 4 + 8 * i:off + 12 + 8 * i])
        if struct.unpack(">H", b[off + so:off + so + 2])[0] == 12:
            at = off + so
            for k in range(struct.unpack(">I", b[at + 12:at + 16])[0]):
                s, e, g = struct.unpack(">III", b[at + 16 + 12 * k:at + 28 + 12 * k])
                for c in range(s, e + 1):
                    cmap[c] = g + c - s
    nh = struct.unpack(">H", b[t["hhea"] + 34:t["hhea"] + 36])[0]
    adv = [struct.unpack(">H", b[t["hmtx"] + 4 * i:t["hmtx"] + 4 * i + 2])[0] for i in range(nh)]
    return {c: adv[min(g, nh - 1)] for c, g in cmap.items()}


def main():
    af = os.path.dirname(kpse("amsfonts.sty"))
    rows = []
    for fname in ("amssymb.sty", "amsfonts.sty"):
        for ln, line in enumerate(open(os.path.join(af, fname), encoding="latin-1"), 1):
            if line.lstrip().startswith("%"):
                continue
            m = (re.search(r'DeclareMathSymbol\{\\(\w+@?)\}\s*\{\\math(\w+)\}\s*\{AMS(a|b)\}\{"(\w\w)\}', line)
                 or re.search(r'DeclareMathDelimiter\{\\(\w+)\}\{\\math(\w+)\}\s*\{AMS(a|b)\}\{"(\w\w)\}', line))
            if m:
                rows.append((m.group(1), m.group(2), "msam" if m.group(3) == "a" else "msbm",
                             int(m.group(4), 16), f"{fname}:{ln}"))
    um = {}
    for m in re.finditer(r'\\UnicodeMathSymbol\{"([0-9A-F]+)\}\{\\([A-Za-z]+) *\}',
                         open(kpse("unicode-math-table.tex"), encoding="utf-8").read()):
        um.setdefault(m.group(2), m.group(1).lstrip("0"))
    lm, nc = otf(os.path.join(FONTS, "latinmodern-math.otf")), otf(os.path.join(FONTS, "NewCMMath-Regular.otf"))
    tfm = {f: gen_cm_tfm.parse(kpse(f + ".tfm")) for f in ("msam10", "msbm10")}

    seen, table = set(), []
    for name, cls, font, slot, where in rows:
        if name in seen or "@" in name:
            continue
        # amsfonts.sty 141-160 repeat amssymb's `\square`..`\trianglelefteq`
        # (and `\lhd`..`\unrhd` under latexsym compatibility).
        if where.startswith("amsfonts") and name in ("square", "lozenge", "lhd", "unlhd", "rhd", "unrhd"):
            continue
        seen.add(name)
        cands = MANUAL.get(name) or ([um[name]] if name in um else [])
        table.append((name, cls, font, slot, cands, where))
    table += EXTRA

    out_rows, lm_adv, nc_adv = [], {}, {}
    for name, cls, font, slot, cands, where in table:
        chosen = None
        for c in cands:
            chars = [int(x, 16) for x in c.split()]
            if not chars:
                chosen = ("", "None")
                break
            if all(x in lm for x in chars):
                chosen = ("".join(map(chr, chars)), "LatinModernMath")
                for x in chars:
                    lm_adv[x] = lm[x]
                break
            if all(x in nc for x in chars):
                chosen = ("".join(map(chr, chars)), "NewComputerModernMath")
                for x in chars:
                    nc_adv[x] = nc[x]
                break
        if chosen is None:
            sys.exit(f"no face carries \\{name} {cands}")
        width = tfm[font + "10"]["chars"][slot]["w"] / 2 ** 20
        out_rows.append((name, chosen[0], cls, font, slot, width, chosen[1], where))
    for x in list(nc_adv):
        if x in lm_adv:
            del nc_adv[x]

    def rs(s):
        return "".join(ch if 0x20 <= ord(ch) < 0x7F and ch not in '"\\' else f"\\u{{{ord(ch):04X}}}" for ch in s)

    o = []
    o.append("//! amssymb/amsfonts math symbols: the `AMSa` (msam) and `AMSb` (msbm)")
    o.append("//! declarations of `amssymb.sty`/`amsfonts.sty` (TeX Live 2026, v3.01), with")
    o.append("//! each command's math class and font slot, the Unicode text standing for it")
    o.append("//! and the OpenType face that carries that text.")
    o.append("//!")
    o.append("//! GENERATED by `scripts/gen_amssymb.py`; do not edit by hand. The TFM slot is")
    o.append("//! what pdfLaTeX sets: render-pipeline boxes every symbol from the msam/msbm")
    o.append("//! TFMs at its math size (math-layout `ams`) and paints `text` from the face;")
    o.append("//! `width_em` (msam10/msbm10) only feeds the compiler's own layout.")
    o.append("")
    o.append("/// `AMSa` (`U/msa/m/n`, msam) or `AMSb` (`U/msb/m/n`, msbm).")
    o.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    o.append("pub enum SymbolFont {\n    Msam,\n    Msbm,\n}")
    o.append("")
    o.append("/// The `\\math<class>` of the declaration.")
    o.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    o.append("pub enum SymbolClass {\n    Ord,\n    Bin,\n    Rel,\n    Open,\n    Close,\n}")
    o.append("")
    o.append("/// The face whose `cmap` carries every character of a symbol's `text`.")
    o.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    o.append("pub enum Face {\n    LatinModernMath,\n    NewComputerModernMath,\n    /// An empty `text`: a piece that paints nothing of its own.\n    None,\n}")
    o.append("")
    o.append("#[derive(Debug, PartialEq)]")
    o.append("pub struct AmsSymbol {")
    o.append("    /// The command without its backslash; names containing `@` are pieces")
    o.append("    /// other constructions place, not commands.")
    o.append("    pub name: &'static str,")
    o.append("    /// Unicode text: one character, or a base followed by U+0338 when Unicode")
    o.append("    /// has no precomposed negation.")
    o.append("    pub text: &'static str,")
    o.append("    pub class: SymbolClass,")
    o.append("    pub font: SymbolFont,")
    o.append("    pub slot: u8,")
    o.append("    /// Character width in ems of msam10/msbm10.")
    o.append("    pub width_em: f64,")
    o.append("    pub face: Face,")
    o.append("    /// The declaring `file:line`.")
    o.append("    pub source: &'static str,")
    o.append("}")
    o.append("")
    o.append("pub const SYMBOLS: &[AmsSymbol] = &[")
    for name, text, cls, font, slot, width, face, where in out_rows:
        o.append(f"    AmsSymbol {{ name: \"{name}\", text: \"{rs(text)}\", class: SymbolClass::{cls.capitalize()}, "
                 f"font: SymbolFont::{font.capitalize()}, slot: 0x{slot:02X}, width_em: {width:.6f}, "
                 f"face: Face::{face}, source: \"{where}\" }},")
    o.append("];")
    o.append("")
    o.append("/// `\\global\\let` aliases of `amssymb.sty` (68, 90, 145, 147, 157, 159).")
    o.append("pub const ALIASES: &[(&str, &str)] = &[")
    for a, t in ALIASES:
        if t:
            o.append(f"    (\"{a}\", \"{t}\"),")
    o.append("];")
    o.append("")
    o.append("/// Advances (font units of 1000) of the Latin Modern Math characters above.")
    o.append("pub const LM_ADVANCES: &[(char, u16)] = &[")
    for x in sorted(lm_adv):
        o.append(f"    ('\\u{{{x:04X}}}', {lm_adv[x]}),")
    o.append("];")
    o.append("")
    o.append("/// Advances of the characters only New Computer Modern Math carries.")
    o.append("pub const NEWCM_ADVANCES: &[(char, u16)] = &[")
    for x in sorted(nc_adv):
        o.append(f"    ('\\u{{{x:04X}}}', {nc_adv[x]}),")
    o.append("];")
    o.append(open(os.path.join(HERE, "amssymb_api.rs.in"), encoding="utf-8").read().rstrip("\n"))
    sys.stdout.write("\n".join(o) + "\n")


if __name__ == "__main__":
    main()
