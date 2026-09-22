#!/usr/bin/env python3
r"""Generate the declared math symbol tables from LaTeX's own declarations.

Development-time extraction only (no TeX at runtime; the generated files are
committed and the tests never need TeX). Sources, resolved with `kpsewhich`:

* `fontmath.ltx` (the LaTeX2e kernel): every `\DeclareSymbolFont`,
  `\DeclareMathAlphabet`, `\DeclareSymbolFontAlphabet`, `\DeclareMathSymbol`,
  `\DeclareMathDelimiter`, `\DeclareMathAccent` and `\DeclareMathRadical`,
  plus the `\DeclareRobustCommand` composites (`\cong`, `\bowtie`, `\models`,
  the long arrows, `\hbar`, `\surd`, `\angle`, `\neq`, `\dots`, ...), which
  are recorded with their definition body so that the engine's hand-written
  arms can be checked against the list rather than against memory.
* `latexsym.sty` (lasy), `amsfonts.sty` / `amssymb.sty` (AMSa msam, AMSb
  msbm), `stmaryrd.sty` (stmry), `mathrsfs.sty` (rsfs): the same declaration
  forms in their package spellings (`\DeclareMathSymbol\mho{\mathord}...`,
  `\ams@DeclareMathSymbol{...}`, `\stmry@if\DeclareMathSymbol\x\mathrel...`),
  and their `\let` aliases.
* `unicode-math-table.tex`: the Unicode character standing for each command,
  where it names one; `MANUAL` below chooses the rest.
* The 10pt TFM of every symbol font, for the slot's existence and its width.
* `apps/mac/Fonts/latinmodern-math.otf` / `NewCMMath-Regular.otf`: which
  bundled face carries each character (Latin Modern Math first).

Outputs (each `GENERATED`; do not edit by hand):

* `crates/compiler/src/math_symbols.rs`: the full table (`SYMBOLS`),
  aliases, composites, symbol fonts and math alphabets.
* `crates/math-layout/src/cm_slots.rs`: the Unicode character -> Computer
  Modern (family, slot) map for the four kernel symbol fonts.

Usage:
    python3 crates/compiler/scripts/gen_math_symbols.py            # write both
    python3 crates/compiler/scripts/gen_math_symbols.py --check    # diff only
    python3 crates/compiler/scripts/gen_math_symbols.py --json     # declarations as JSON
"""
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "crates", "math-layout", "tools"))
sys.path.insert(0, HERE)
import gen_cm_tfm  # noqa: E402
import gen_amssymb  # noqa: E402

FONTS_DIR = os.path.join(REPO, "apps", "mac", "Fonts")
COMPILER_OUT = os.path.join(REPO, "crates", "compiler", "src", "math_symbols.rs")
LAYOUT_OUT = os.path.join(REPO, "crates", "math-layout", "src", "cm_slots.rs")


def kpse(name):
    return subprocess.check_output(["kpsewhich", name]).decode().strip()


# (file, provider) in the order pdflatex would see them: later declarations of
# the same name in the same provider win; across providers every declaration
# is kept (the engine gates on the loaded package).
SOURCES = [
    ("fontmath.ltx", "Kernel"),
    ("latexsym.sty", "Latexsym"),
    ("amsfonts.sty", "Amsfonts"),
    ("amssymb.sty", "Amssymb"),
    ("stmaryrd.sty", "Stmaryrd"),
    ("mathrsfs.sty", "Mathrsfs"),
    ("amsmath.sty", "Amsmath"),
]
# Files whose `\def`s/`\DeclareRobustCommand`s are symbol composites worth
# recording; amsmath.sty is read for its declarations only (its hundreds of
# macros are the package's machinery, not symbols).
COMPOSITE_SOURCES = {"fontmath.ltx", "latexsym.sty", "amsfonts.sty", "amssymb.sty", "stmaryrd.sty"}

# Symbol font name -> (Rust variant, 10pt TFM, pdfTeX PDF font prefix).
SYMBOL_FONTS = {
    "operators": ("Operators", "cmr10", "CMR"),
    "letters": ("Letters", "cmmi10", "CMMI"),
    "symbols": ("Symbols", "cmsy10", "CMSY"),
    "largesymbols": ("LargeSymbols", "cmex10", "CMEX"),
    "AMSa": ("AMSa", "msam10", "MSAM"),
    "AMSb": ("AMSb", "msbm10", "MSBM"),
    "lasy": ("Lasy", "lasy10", "LASY"),
    "stmry": ("Stmry", "stmary10", "STMARY"),
    "rsfs": ("Rsfs", "rsfs10", "RSFS"),
}

CLASSES = {"ord": "Ord", "op": "Op", "bin": "Bin", "rel": "Rel", "open": "Open",
           "close": "Close", "punct": "Punct", "alpha": "Alpha"}

# Unicode for commands unicode-math does not name, or names with a different
# design. A base plus U+0338 where Unicode has no precomposed negation. The
# kernel entries are the characters the compiler already emits for them
# (`COMMAND_GLYPHS`), so the switch-over changes no painted text.
MANUAL = dict(gen_amssymb.MANUAL)
MANUAL.update({
    # fontmath.ltx: ASCII single-character declarations keep their character.
    # Kernel names unicode-math spells differently or does not have.
    # `\phi` is cmmi "1E (the straight form) and `\varphi` "27 (the loopy one);
    # the engine has always spelt them U+03C6/U+03D5 in that order and paints
    # the TFM slot, so the text keeps that convention (unicode-math swaps them).
    "epsilon": ["03F5"], "varepsilon": ["03B5"], "phi": ["03C6"], "varphi": ["03D5"],
    "varrho": ["03F1"], "varsigma": ["03C2"], "vartheta": ["03D1"], "varpi": ["03D6"],
    # amsmath's italic capitals (cmmi "00-"0A): Mathematical Italic Capital letters.
    "varGamma": ["1D6E4"], "varDelta": ["1D6E5"], "varTheta": ["1D6E9"], "varLambda": ["1D6EC"],
    "varXi": ["1D6EF"], "varPi": ["1D6F1"], "varSigma": ["1D6F4"], "varUpsilon": ["1D6F6"],
    "varPhi": ["1D6F7"], "varPsi": ["1D6F9"], "varOmega": ["1D6FA"],
    # amsfonts' `\mathhexbox` text symbols (msam).
    "yen": ["00A5"], "checkmark": ["2713"], "circledR": ["00AE"], "maltese": ["2720"],
    "imath": ["1D6A4", "0131"], "jmath": ["1D6A5", "0237"],
    "prime": ["2032"], "emptyset": ["2205"], "surd": ["221A"], "top": ["22A4"],
    "bot": ["22A5"], "perp": ["22A5"], "wp": ["2118"], "ell": ["2113"],
    "Re": ["211C"], "Im": ["2111"], "mho": ["2127"],
    "triangle": ["25B3"], "bigtriangleup": ["25B3"], "bigtriangledown": ["25BD"],
    "triangleleft": ["25C1"], "triangleright": ["25B7"],
    "lhd": ["22B2"], "rhd": ["22B3"], "unlhd": ["22B4"], "unrhd": ["22B5"],
    "Box": ["25A1"], "Diamond": ["25CA"], "Join": ["2A1D"], "leadsto": ["21DD"],
    "sqsubset": ["228F"], "sqsupset": ["2290"],
    "backslash": ["2216"], "setminus": ["2216"], "vert": ["2223"], "mid": ["2223"],
    "Vert": ["2016"], "parallel": ["2225"], "|": ["2016"],
    "lvert": ["2223"], "rvert": ["2223"], "lVert": ["2016"], "rVert": ["2016"],
    "cdot": ["22C5"], "cdotp": ["22C5"], "ldotp": ["002E"], "colon": ["003A"],
    "dagger": ["2020"], "ddagger": ["2021"], "ast": ["2217"], "star": ["22C6"],
    "circ": ["2218"], "bullet": ["2219"], "diamond": ["22C4"],
    "lbrace": ["007B"], "rbrace": ["007D"], "langle": ["27E8"], "rangle": ["27E9"],
    "lfloor": ["230A"], "rfloor": ["230B"], "lceil": ["2308"], "rceil": ["2309"],
    "lgroup": ["27EE"], "rgroup": ["27EF"], "lmoustache": ["23B0"], "rmoustache": ["23B1"],
    # `\Arrowvert` has no Unicode of its own (U+2016 is `\Vert`, whose large
    # variant differs), so it carries no text: nothing keyed by character can
    # stand for it.
    "arrowvert": ["23D0"], "Arrowvert": [""], "bracevert": ["23AA"],
    "uparrow": ["2191"], "downarrow": ["2193"], "updownarrow": ["2195"],
    "Uparrow": ["21D1"], "Downarrow": ["21D3"], "Updownarrow": ["21D5"],
    "mapstochar": ["F8FE"], "not": ["0338"], "lhook": [""], "rhook": [""],
    "braceld": ["23DF"], "bracerd": ["23DF"], "bracelu": ["23DE"], "braceru": ["23DE"],
    "sqrtsign": ["221A"],
    "mathsection": ["00A7"], "mathparagraph": ["00B6"], "mathdollar": ["0024"],
    "acute": ["00B4"], "grave": ["0060"], "ddot": ["00A8"], "tilde": ["02DC"],
    "bar": ["00AF"], "breve": ["02D8"], "check": ["02C7"], "hat": ["02C6"],
    "vec": ["20D7"], "dot": ["02D9"], "widetilde": ["0303"], "widehat": ["0302"],
    "mathring": ["02DA"],
    "int": ["222B"], "intop": ["222B"], "oint": ["222E"], "ointop": ["222E"],
    "smallint": ["222B"],
    "hbar": ["210F"], "angle": ["2220"],
    "flat": ["266D"], "natural": ["266E"], "sharp": ["266F"],
    "clubsuit": ["2663"], "diamondsuit": ["2662"], "heartsuit": ["2661"], "spadesuit": ["2660"],
    "amalg": ["2A3F"], "uplus": ["228E"], "wr": ["2240"], "bigcirc": ["25EF"],
    "sqcup": ["2294"], "sqcap": ["2293"], "sqsubseteq": ["2291"], "sqsupseteq": ["2292"],
    "propto": ["221D"], "asymp": ["224D"], "smile": ["2323"], "frown": ["2322"],
    "ni": ["220B"], "owns": ["220B"],
    "leq": ["2264"], "le": ["2264"], "geq": ["2265"], "ge": ["2265"],
    "gets": ["2190"], "to": ["2192"], "land": ["2227"], "lor": ["2228"], "lnot": ["00AC"],
    "neg": ["00AC"],
    "leftharpoonup": ["21BC"], "leftharpoondown": ["21BD"],
    "rightharpoonup": ["21C0"], "rightharpoondown": ["21C1"],
    "aleph": ["2135"], "infty": ["221E"], "nabla": ["2207"], "partial": ["2202"],
    "forall": ["2200"], "exists": ["2203"], "flat": ["266D"],
    "coprod": ["2210"], "bigvee": ["22C1"], "bigwedge": ["22C0"], "biguplus": ["2A04"],
    "bigcap": ["22C2"], "bigcup": ["22C3"], "prod": ["220F"], "sum": ["2211"],
    "bigotimes": ["2A02"], "bigoplus": ["2A01"], "bigodot": ["2A00"], "bigsqcup": ["2A06"],
    "ominus": ["2296"], "oslash": ["2298"], "odot": ["2299"], "oplus": ["2295"],
    "otimes": ["2297"], "pm": ["00B1"], "mp": ["2213"], "times": ["00D7"], "div": ["00F7"],
    "cup": ["222A"], "cap": ["2229"], "vee": ["2228"], "wedge": ["2227"],
    "equiv": ["2261"], "sim": ["223C"], "simeq": ["2243"], "approx": ["2248"],
    "subset": ["2282"], "supset": ["2283"], "subseteq": ["2286"], "supseteq": ["2287"],
    "ll": ["226A"], "gg": ["226B"], "prec": ["227A"], "succ": ["227B"],
    "preceq": ["2AAF"], "succeq": ["2AB0"], "in": ["2208"],
    "leftarrow": ["2190"], "rightarrow": ["2192"], "leftrightarrow": ["2194"],
    "Leftarrow": ["21D0"], "Rightarrow": ["21D2"], "Leftrightarrow": ["21D4"],
    "nearrow": ["2197"], "searrow": ["2198"], "swarrow": ["2199"], "nwarrow": ["2196"],
    "dashv": ["22A3"], "vdash": ["22A2"], "models": ["22A8"],
    "Relbar": ["003D"], "relbar": ["2212"],
    # stmaryrd names unicode-math lacks.
    "shortleftarrow": ["2190"], "shortrightarrow": ["2192"], "shortuparrow": ["2191"],
    "shortdownarrow": ["2193"], "Yup": ["2144"], "Ydown": ["2144"], "Yleft": ["2144"],
    "Yright": ["2144"],
    "llbracket": ["27E6"], "rrbracket": ["27E7"],
    # fontmath.ltx 268-269 (2020/02/02): `\varbigtriangleup`/`\varbigtriangledown`
    # share `\bigtriangleup`/`\bigtriangledown`'s cmsy slots.
    "varbigtriangleup": ["25B3"], "varbigtriangledown": ["25BD"],
    # Pieces (`@` names): what they draw, or nothing of their own.
    "@backslashchar": ["005C"], "dabar@": [""],
})
# unicode-math names Greek letters `\mupalpha`/`\mitalpha`; LaTeX's `\alpha` is
# the cmmi italic lower case and `\Gamma` the cmr upright capital.
GREEK_LOWER = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
               "lambda", "mu", "nu", "xi", "omicron", "pi", "rho", "varsigma", "sigma", "tau", "upsilon",
               "phi", "chi", "psi", "omega"]
for _i, _n in enumerate(GREEK_LOWER):
    MANUAL.setdefault(_n, [f"{0x03B1 + _i:04X}"])
    MANUAL.setdefault(_n.capitalize(), [f"{0x0391 + _i:04X}"])

# Aliases (`\let`) the package files make, in the spelling gen_amssymb records
# plus the ones outside the AMS files. (alias, target, provider, source)
STATIC_ALIASES = []


def strip_comment(line):
    out = []
    i = 0
    while i < len(line):
        c = line[i]
        if c == "\\":
            out.append(line[i:i + 2])
            i += 2
            continue
        if c == "%":
            break
        out.append(c)
        i += 1
    return "".join(out)


NAME = r"(?:\\([A-Za-z@|]+)|([^\\{}\s]))"  # \cmd or a single character
ARG_NAME = r"\{?\s*" + NAME + r"\s*\}?"
CLASS = r"\{?\\math(ord|op|bin|rel|open|close|punct|alpha)\}?"
FONT = r"\{(operators|letters|symbols|largesymbols|AMSa|AMSb|lasy|stmry|rsfs)\}"
SLOT = r"""\{\s*(?:"([0-9A-Fa-f]+)|'([0-7]+)|`\\?(.))\s*\}"""

RE_SYMBOL = re.compile(r"\\(?:ams@|stmry@if\\)?DeclareMathSymbol\s*" + ARG_NAME + r"\s*" + CLASS + r"\s*" + FONT + SLOT)
RE_DELIM = re.compile(r"\\(?:ams@|stmry@if\\)?DeclareMathDelimiter\s*" + ARG_NAME + r"\s*" + CLASS + r"\s*" + FONT + SLOT + r"\s*" + FONT + SLOT)
RE_ACCENT = re.compile(r"\\DeclareMathAccent\s*" + ARG_NAME + r"\s*" + CLASS + r"\s*" + FONT + SLOT)
RE_RADICAL = re.compile(r"\\DeclareMathRadical\s*" + ARG_NAME + r"\s*" + FONT + SLOT + r"\s*" + FONT + SLOT)
RE_SYMFONT = re.compile(r"\\DeclareSymbolFont\s*\{(\w+)\}\s*\{(\w+)\}\s*\{(\w+)\}\s*\{(\w+)\}\s*\{(\w+)\}")
RE_ALPHABET = re.compile(r"\\DeclareMathAlphabet\s*\{\\(\w+)\}\s*\{(\w+)\}\s*\{(\w+)\}\s*\{(\w+)\}\s*\{(\w+)\}")
RE_SYMALPHA = re.compile(r"\\DeclareSymbolFontAlphabet\s*\{\\(\w+)\}\s*\{(\w+)\}")
RE_LET = re.compile(r"\\(?:global\\)?let\s*\\([A-Za-z@]+)\s*=?\s*\\([A-Za-z@]+)")
RE_ROBUST = re.compile(r"\\DeclareRobustCommand\s*\{?\\([A-Za-z@]+)\}?\s*(?:\[\d\])?\s*\{")
# amsfonts.sty 62-71: `\edef\yen{\noexpand\mathhexbox{\hexnumber@\symAMSa}55}`,
# a `\mathchar"0<family><slot>` in an `\mbox` (latex.ltx 630): class 0, usable
# in text too.
RE_HEXBOX = re.compile(r"\\edef\\([A-Za-z]+)\{\\noexpand\\mathhexbox\{\\hexnumber@\\sym(AMSa|AMSb)\}([0-9A-F][0-9A-F])\}")
RE_DEF = re.compile(r"\\(?:stmry@if\\)?(?:x?def|gdef)\s*\\([A-Za-z@]+)\s*(?:#\d)*\s*\{")


def slot_value(hexs, octs, chars):
    if hexs is not None:
        return int(hexs, 16)
    if octs is not None:
        return int(octs, 8)
    return ord(chars)


def name_of(m, i):
    return m.group(i) if m.group(i) is not None else m.group(i + 1)


def balanced_body(text, start):
    """The text inside the brace group opening at `text[start] == '{'`."""
    depth = 0
    i = start
    while i < len(text):
        c = text[i]
        if c == "\\":
            i += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return text[start + 1:i], i + 1
        i += 1
    raise ValueError("unbalanced group")


def parse_file(fname, provider):
    path = kpse(fname)
    raw = open(path, encoding="latin-1").read().split("\n")
    lines = [strip_comment(l) for l in raw]
    # Joined text for multi-line declarations, with a line index per offset.
    joined = "\n".join(lines)
    offsets = []
    pos = 0
    for i, l in enumerate(lines):
        offsets.append(pos)
        pos += len(l) + 1

    def line_of(off):
        lo, hi = 0, len(offsets) - 1
        while lo < hi:
            mid = (lo + hi + 1) // 2
            if offsets[mid] <= off:
                lo = mid
            else:
                hi = mid - 1
        return lo + 1

    recs = []
    for m in RE_SYMBOL.finditer(joined):
        recs.append({"kind": "symbol", "name": name_of(m, 1), "character": m.group(1) is None, "class": m.group(3), "font": m.group(4),
                     "slot": slot_value(m.group(5), m.group(6), m.group(7)),
                     "provider": provider, "source": f"{fname}:{line_of(m.start())}"})
    for m in RE_HEXBOX.finditer(joined):
        recs.append({"kind": "symbol", "name": m.group(1), "character": False, "class": "ord", "font": m.group(2),
                     "slot": int(m.group(3), 16),
                     "provider": provider, "source": f"{fname}:{line_of(m.start())}"})
    for m in RE_DELIM.finditer(joined):
        recs.append({"kind": "delimiter", "name": name_of(m, 1), "character": m.group(1) is None, "class": m.group(3), "font": m.group(4),
                     "slot": slot_value(m.group(5), m.group(6), m.group(7)),
                     "large_font": m.group(8), "large_slot": slot_value(m.group(9), m.group(10), m.group(11)),
                     "provider": provider, "source": f"{fname}:{line_of(m.start())}"})
    for m in RE_ACCENT.finditer(joined):
        recs.append({"kind": "accent", "name": name_of(m, 1), "character": m.group(1) is None, "class": m.group(3), "font": m.group(4),
                     "slot": slot_value(m.group(5), m.group(6), m.group(7)),
                     "provider": provider, "source": f"{fname}:{line_of(m.start())}"})
    for m in RE_RADICAL.finditer(joined):
        recs.append({"kind": "radical", "name": name_of(m, 1), "character": m.group(1) is None, "class": "ord", "font": m.group(3),
                     "slot": slot_value(m.group(4), m.group(5), m.group(6)),
                     "large_font": m.group(7), "large_slot": slot_value(m.group(8), m.group(9), m.group(10)),
                     "provider": provider, "source": f"{fname}:{line_of(m.start())}"})
    fonts, alphabets, aliases, composites = [], [], [], []
    for m in RE_SYMFONT.finditer(joined):
        fonts.append({"name": m.group(1), "encoding": m.group(2), "family": m.group(3), "series": m.group(4),
                      "shape": m.group(5), "provider": provider, "source": f"{fname}:{line_of(m.start())}"})
    for m in RE_ALPHABET.finditer(joined):
        alphabets.append({"name": m.group(1), "encoding": m.group(2), "family": m.group(3),
                          "series": m.group(4), "shape": m.group(5), "symbol_font": None,
                          "provider": provider, "source": f"{fname}:{line_of(m.start())}"})
    for m in RE_SYMALPHA.finditer(joined):
        alphabets.append({"name": m.group(1), "encoding": None, "family": None, "series": None,
                          "shape": None, "symbol_font": m.group(2),
                          "provider": provider, "source": f"{fname}:{line_of(m.start())}"})
    # amssymb.sty 40-42 `\let\square\relax \let\rightsquigarrow\square ...`
    # are placeholders (the names are declared below them), not aliases: a
    # `\let` to a name the file has `\let` to `\relax` is skipped.
    relaxed = set()
    for m in RE_LET.finditer(joined):
        alias, target = m.group(1), m.group(2)
        if target == "relax":
            relaxed.add(alias)
            continue
        if "@" in alias or target == "undefined" or "@" in target or target in relaxed:
            continue
        aliases.append({"alias": alias, "target": target, "provider": provider,
                        "source": f"{fname}:{line_of(m.start())}"})
    macros = list(RE_ROBUST.finditer(joined)) + list(RE_DEF.finditer(joined)) if fname in COMPOSITE_SOURCES else []
    for m in macros:
        name = m.group(1)
        if "@" in name:
            continue
        try:
            body, _ = balanced_body(joined, m.end() - 1)
        except ValueError:
            continue
        body = re.sub(r"\s+", " ", body.strip())
        # amsfonts.sty 111-118: `\frak`, `\Bbb`, `\bold`, `\newsymbol` are
        # `\@obsolete` shims, not symbols.
        if "obsolete" in body:
            continue
        composites.append({"name": name, "body": body, "provider": provider,
                           "source": f"{fname}:{line_of(m.start())}"})
    return recs, fonts, alphabets, aliases, composites


def declarations():
    """Every declaration of every source, in file order."""
    symbols, fonts, alphabets, aliases, composites = [], [], [], [], []
    for fname, provider in SOURCES:
        r, f, a, l, c = parse_file(fname, provider)
        symbols += r
        fonts += f
        alphabets += a
        aliases += l
        composites += c
    return {"symbols": symbols, "fonts": fonts, "alphabets": alphabets, "aliases": aliases,
            "composites": composites}


def unicode_names():
    um = {}
    for m in re.finditer(r'\\UnicodeMathSymbol\{"([0-9A-F]+)\}\{\\([A-Za-z]+) *\}',
                         open(kpse("unicode-math-table.tex"), encoding="utf-8").read()):
        um.setdefault(m.group(2), m.group(1).lstrip("0"))
    return um


def candidates(name, um, character=False):
    """Unicode candidates for a command, or for a character declaration
    (`\\DeclareMathSymbol{+}`), which is spelt as itself."""
    if character:
        return [f"{ord(name):04X}"]
    if name in MANUAL:
        return MANUAL[name]
    if name in um:
        return [um[name]]
    return None


def resolve_text(cands, lm, nc):
    """(text, face) for the first candidate a bundled face carries."""
    for c in cands:
        chars = [int(x, 16) for x in c.split()]
        if not chars:
            return "", "None"
        if all(x in lm for x in chars):
            return "".join(map(chr, chars)), "LatinModernMath"
        if all(x in nc for x in chars):
            return "".join(map(chr, chars)), "NewComputerModernMath"
    # Private-use or text-only characters: keep the first choice, unbound.
    chars = [int(x, 16) for x in cands[0].split()]
    return "".join(map(chr, chars)), "None"


def build():
    d = declarations()
    um = unicode_names()
    lm = gen_amssymb.otf(os.path.join(FONTS_DIR, "latinmodern-math.otf"))
    nc = gen_amssymb.otf(os.path.join(FONTS_DIR, "NewCMMath-Regular.otf"))
    tfms = {}
    for font, (_, tfm, _) in SYMBOL_FONTS.items():
        tfms[font] = gen_cm_tfm.parse(kpse(tfm + ".tfm"))
    rows = []
    missing = []
    # Within one provider the last declaration of a name wins (pdflatex
    # semantics); across providers each is kept, gated by the engine.
    seen = {}
    for r in d["symbols"]:
        key = (r["provider"], r["name"], r["character"], r["kind"])
        seen[key] = r
    for r in seen.values():
        cands = candidates(r["name"], um, r["character"])
        if cands is None:
            missing.append(r)
            cands = [""]
        text, face = resolve_text(cands, lm, nc)
        tfm = tfms[r["font"]]
        ch = tfm["chars"].get(r["slot"])
        if ch is None:
            sys.exit(f"{r['source']}: \\{r['name']} slot {r['slot']:#04x} is not in {SYMBOL_FONTS[r['font']][1]}")
        width = ch["w"] / 2 ** 20
        rows.append(dict(r, text=text, face=face, width_em=width))
    order = {k: i for i, k in enumerate(SYMBOL_FONTS)}
    prov = {p: i for i, (_, p) in enumerate(SOURCES)}
    rows.sort(key=lambda r: (prov[r["provider"]], order[r["font"]], r["slot"], r["kind"], r["name"]))
    return d, rows, missing


def rs(s):
    return "".join(ch if 0x20 <= ord(ch) < 0x7F and ch not in '"\\' else f"\\u{{{ord(ch):04X}}}" for ch in s)


def rs_str(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def compiler_source(d, rows):
    o = []
    o.append("//! Every math symbol LaTeX declares, derived from the declarations themselves:")
    o.append("//! `fontmath.ltx` (kernel), `latexsym.sty`, `amsfonts.sty`/`amssymb.sty`,")
    o.append("//! `stmaryrd.sty` and `mathrsfs.sty` (TeX Live 2026), each command with its")
    o.append("//! math class, symbol font, slot, the Unicode text the engine paints for it")
    o.append("//! and the bundled face that carries that text.")
    o.append("//!")
    o.append("//! GENERATED by `scripts/gen_math_symbols.py`; do not edit by hand. The")
    o.append("//! (font, slot) pair is what pdfLaTeX sets from the TFM; `text` is what the")
    o.append("//! engine's OpenType route paints. `COMPOSITES` are the kernel's `\\def`s,")
    o.append("//! which have no single slot and are implemented by hand (`math.rs`).")
    o.append("")
    o.append("/// A `\\DeclareSymbolFont` name: the four kernel families and the package fonts.")
    o.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    o.append("pub enum SymbolFont {")
    for font, (variant, tfm, _) in SYMBOL_FONTS.items():
        o.append(f"    /// `{font}` ({tfm} at 10pt).")
        o.append(f"    {variant},")
    o.append("}")
    o.append("")
    o.append("impl SymbolFont {")
    o.append("    /// The `\\DeclareSymbolFont` name.")
    o.append("    pub fn latex_name(self) -> &'static str {")
    o.append("        match self {")
    for font, (variant, _, _) in SYMBOL_FONTS.items():
        o.append(f"            SymbolFont::{variant} => \"{font}\",")
    o.append("        }")
    o.append("    }")
    o.append("")
    o.append("    /// The 10pt TFM the font is set from.")
    o.append("    pub fn tfm10(self) -> &'static str {")
    o.append("        match self {")
    for font, (variant, tfm, _) in SYMBOL_FONTS.items():
        o.append(f"            SymbolFont::{variant} => \"{tfm}\",")
    o.append("        }")
    o.append("    }")
    o.append("")
    o.append("    /// The prefix of the font's name in a pdfTeX PDF (`CMSY10`, `MSAM7`).")
    o.append("    pub fn pdf_prefix(self) -> &'static str {")
    o.append("        match self {")
    for font, (variant, _, prefix) in SYMBOL_FONTS.items():
        o.append(f"            SymbolFont::{variant} => \"{prefix}\",")
    o.append("        }")
    o.append("    }")
    o.append("}")
    o.append("")
    o.append("/// The `\\math<class>` of the declaration (`Alpha` is `\\mathalpha`: an")
    o.append("/// ordinary symbol that math alphabets may change).")
    o.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    o.append("pub enum SymbolClass {")
    for v in CLASSES.values():
        o.append(f"    {v},")
    o.append("}")
    o.append("")
    o.append("/// Which file declares a command: the kernel, or the package that has to be")
    o.append("/// loaded for it to exist.")
    o.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    o.append("pub enum Provider {")
    for _, p in SOURCES:
        o.append(f"    {p},")
    o.append("}")
    o.append("")
    o.append("/// The declaration form.")
    o.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    o.append("pub enum Kind {")
    o.append("    /// `\\DeclareMathSymbol`.")
    o.append("    Symbol,")
    o.append("    /// `\\DeclareMathDelimiter`: the small variant, then `large` in cmex-like fonts.")
    o.append("    Delimiter,")
    o.append("    /// `\\DeclareMathAccent`.")
    o.append("    Accent,")
    o.append("    /// `\\DeclareMathRadical`.")
    o.append("    Radical,")
    o.append("}")
    o.append("")
    o.append("/// The bundled face whose `cmap` carries every character of a symbol's `text`.")
    o.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    o.append("pub enum Face {")
    o.append("    LatinModernMath,")
    o.append("    NewComputerModernMath,")
    o.append("    /// An empty or private-use `text`: nothing a face paints as such.")
    o.append("    None,")
    o.append("}")
    o.append("")
    o.append("#[derive(Debug, PartialEq)]")
    o.append("pub struct MathSymbol {")
    o.append("    /// The command without its backslash, or the single character")
    o.append("    /// (`\\DeclareMathSymbol{+}...`) whose `\\mathcode` the declaration sets.")
    o.append("    pub name: &'static str,")
    o.append("    /// `name` is a single character whose `\\mathcode` the declaration sets,")
    o.append("    /// not a control sequence (`\\P` is a command; `P` the letter).")
    o.append("    pub character: bool,")
    o.append("    pub kind: Kind,")
    o.append("    pub class: SymbolClass,")
    o.append("    pub font: SymbolFont,")
    o.append("    pub slot: u8,")
    o.append("    /// The large variant of a delimiter or radical.")
    o.append("    pub large: Option<(SymbolFont, u8)>,")
    o.append("    /// Unicode text: one character, or a base followed by U+0338 when Unicode")
    o.append("    /// has no precomposed negation; empty for pieces without a character.")
    o.append("    pub text: &'static str,")
    o.append("    /// Character width in ems of the 10pt TFM.")
    o.append("    pub width_em: f64,")
    o.append("    pub face: Face,")
    o.append("    pub provider: Provider,")
    o.append("    /// The declaring `file:line`.")
    o.append("    pub source: &'static str,")
    o.append("}")
    o.append("")
    o.append("pub const SYMBOLS: &[MathSymbol] = &[")
    for r in rows:
        large = "None"
        if r["kind"] in ("delimiter", "radical"):
            large = f"Some((SymbolFont::{SYMBOL_FONTS[r['large_font']][0]}, 0x{r['large_slot']:02X}))"
        character = "true" if r["character"] else "false"
        o.append(f"    MathSymbol {{ name: {rs_str(r['name'])}, character: {character}, "
                 f"kind: Kind::{r['kind'].capitalize()}, "
                 f"class: SymbolClass::{CLASSES[r['class']]}, font: SymbolFont::{SYMBOL_FONTS[r['font']][0]}, "
                 f"slot: 0x{r['slot']:02X}, large: {large}, text: \"{rs(r['text'])}\", "
                 f"width_em: {r['width_em']:.6f}, face: Face::{r['face']}, provider: Provider::{r['provider']}, "
                 f"source: \"{r['source']}\" }},")
    o.append("];")
    o.append("")
    o.append("/// `\\let` aliases the declaring files make: (alias, target, provider, source).")
    o.append("pub const ALIASES: &[(&str, &str, Provider, &str)] = &[")
    for a in d["aliases"]:
        o.append(f"    (\"{a['alias']}\", \"{a['target']}\", Provider::{a['provider']}, \"{a['source']}\"),")
    o.append("];")
    o.append("")
    o.append("/// Commands the declaring files define as macros over other symbols (the")
    o.append("/// `\\mathrel` joins, `\\not` overlays, `\\mathpalette` stacks, ...): (name,")
    o.append("/// definition body, provider, source). Each needs a hand-written arm.")
    o.append("pub const COMPOSITES: &[(&str, &str, Provider, &str)] = &[")
    for c in d["composites"]:
        o.append(f"    (\"{c['name']}\", {rs_str(c['body'])}, Provider::{c['provider']}, \"{c['source']}\"),")
    o.append("];")
    o.append("")
    o.append("/// `\\DeclareSymbolFont`: (name, encoding, family, series, shape, provider, source).")
    o.append("pub const SYMBOL_FONTS: &[(&str, &str, &str, &str, &str, Provider, &str)] = &[")
    for f in d["fonts"]:
        o.append(f"    (\"{f['name']}\", \"{f['encoding']}\", \"{f['family']}\", \"{f['series']}\", "
                 f"\"{f['shape']}\", Provider::{f['provider']}, \"{f['source']}\"),")
    o.append("];")
    o.append("")
    o.append("/// `\\DeclareMathAlphabet` / `\\DeclareSymbolFontAlphabet`: (command, NFSS")
    o.append("/// encoding/family/series/shape or the symbol font it shares, provider, source).")
    o.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    o.append("pub enum AlphabetFont {")
    o.append("    Nfss { encoding: &'static str, family: &'static str, series: &'static str, shape: &'static str },")
    o.append("    SymbolFont(&'static str),")
    o.append("}")
    o.append("")
    o.append("pub const MATH_ALPHABETS: &[(&str, AlphabetFont, Provider, &str)] = &[")
    for a in d["alphabets"]:
        if a["symbol_font"]:
            font = f"AlphabetFont::SymbolFont(\"{a['symbol_font']}\")"
        else:
            font = (f"AlphabetFont::Nfss {{ encoding: \"{a['encoding']}\", family: \"{a['family']}\", "
                    f"series: \"{a['series']}\", shape: \"{a['shape']}\" }}")
        o.append(f"    (\"{a['name']}\", {font}, Provider::{a['provider']}, \"{a['source']}\"),")
    o.append("];")
    o.append(open(os.path.join(HERE, "math_symbols_api.rs.in")).read())
    return "\n".join(o) + "\n"


def layout_source(rows):
    fam = {"operators": "Roman", "letters": "Italic", "symbols": "Symbol", "largesymbols": "Extension"}
    o = []
    o.append("//! The Computer Modern (family, slot) of every character the kernel's math")
    o.append("//! declarations set from the four `fontmath.ltx` symbol fonts (`operators`")
    o.append("//! cmr, `letters` cmmi, `symbols` cmsy, `largesymbols` cmex), keyed by the")
    o.append("//! Unicode text the compiler emits for the command.")
    o.append("//!")
    o.append("//! GENERATED by `crates/compiler/scripts/gen_math_symbols.py`; do not edit by")
    o.append("//! hand. Where two commands share a character but not a slot (`\\vec` cmmi")
    o.append("//! \"7E against `\\rightarrow` cmsy \"21 are different characters; `\\int` and")
    o.append("//! `\\smallint` are not), the first declaration in `fontmath.ltx` order is")
    o.append("//! listed and the rest are in `SHARED_TEXT`.")
    o.append("")
    o.append("use crate::cm::Family;")
    o.append("")
    o.append("/// (character, family, slot, command) for every kernel `\\DeclareMathSymbol`,")
    o.append("/// `\\DeclareMathAccent` and the small variant of every `\\DeclareMathDelimiter`.")
    o.append("pub const DECLARED_SLOTS: &[(char, Family, u8, &str)] = &[")
    first = {}
    shared = []
    # `\DeclareMathSymbol` sets the character's `\mathcode`; a
    # `\DeclareMathDelimiter` of a single character sets it too, but a later
    # `\DeclareMathSymbol` of the same character wins (`/` is letters "3D as
    # a symbol and operators "2F only as a delimiter, fontmath.ltx 170-171).
    # So symbols and accents first, in file order, then the small variants of
    # delimiters for characters nothing else maps.
    kernel = [r for r in rows if r["provider"] == "Kernel" and r["font"] in fam and len(r["text"]) == 1
              and r["face"] != "None" and not (r["kind"] == "accent" and r["name"] in ("widehat", "widetilde"))]
    kernel.sort(key=lambda r: ({"symbol": 0, "accent": 0, "radical": 0, "delimiter": 1}[r["kind"]],
                               int(r["source"].split(":")[1])))
    for r in kernel:
        ch = r["text"]
        key = (fam[r["font"]], r["slot"])
        if ch in first:
            if first[ch] != key and r["kind"] != "delimiter":
                shared.append((ch, r))
            continue
        first[ch] = key
        o.append(f"    ('{rs(ch)}', Family::{fam[r['font']]}, 0x{r['slot']:02X}, {rs_str(r['name'])}),")
    o.append("];")
    o.append("")
    o.append("/// Commands whose text another command already maps to a different slot;")
    o.append("/// the engine keeps these apart by the command, not the character.")
    o.append("pub const SHARED_TEXT: &[(char, Family, u8, &str)] = &[")
    for ch, r in shared:
        o.append(f"    ('{rs(ch)}', Family::{fam[r['font']]}, 0x{r['slot']:02X}, {rs_str(r['name'])}),")
    o.append("];")
    o.append("")
    o.append("/// The large (cmex) variant of every kernel `\\DeclareMathDelimiter`, keyed by")
    o.append("/// the character: ((small family, small slot), large slot).")
    o.append("pub const DECLARED_DELIMITERS: &[(char, Family, u8, u8, &str)] = &[")
    seen = set()
    for r in rows:
        if r["provider"] != "Kernel" or r["kind"] != "delimiter" or len(r["text"]) != 1:
            continue
        if r["text"] in seen:
            continue
        seen.add(r["text"])
        o.append(f"    ('{rs(r['text'])}', Family::{fam[r['font']]}, 0x{r['slot']:02X}, "
                 f"0x{r['large_slot']:02X}, {rs_str(r['name'])}),")
    o.append("];")
    return "\n".join(o) + "\n"


def main():
    d, rows, missing = build()
    if "--json" in sys.argv:
        json.dump({"rows": rows, **{k: v for k, v in d.items() if k != "symbols"}}, sys.stdout, indent=1,
                  ensure_ascii=False)
        return
    if missing:
        # stmaryrd names Unicode never encoded: the rows keep an empty text
        # (the engine can only set them from the TFM slot).
        print(f"{len(missing)} declarations without a Unicode character: "
              + " ".join("\\" + r["name"] for r in missing), file=sys.stderr)
    outs = {COMPILER_OUT: compiler_source(d, rows), LAYOUT_OUT: layout_source(rows)}
    if "--check" in sys.argv:
        stale = [p for p, s in outs.items() if open(p).read() != s]
        for p in stale:
            print(f"stale: {p}", file=sys.stderr)
        sys.exit(1 if stale else 0)
    for p, s in outs.items():
        open(p, "w").write(s)
        print(f"wrote {p} ({s.count(chr(10))} lines)")


if __name__ == "__main__":
    main()
