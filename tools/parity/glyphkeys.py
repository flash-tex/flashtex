"""Glyph identity for the parity scoreboard: one comparable key per glyph.

Oracle tooling only (standard library). The two producers describe a glyph
differently. pdfTeX's PDF knows a font and a code, and through the font's
encoding a PostScript glyph name (`pdftext` records it as `name`: from
`/Differences`, or from the embedded Type 1 program's built-in encoding for
the `cm*`/`msbm`/`cmex` fonts that carry no `/Encoding`). FlashTeX's display
list knows an OpenType glyph id and the Unicode text of the glyph's cluster.
Neither side's native identity means anything to the other, so both are
reduced to **Unicode characters**, normalised the same way:

* reference: glyph name -> Unicode (`NAME_TO_TEXT`, then `uniXXXX`/`uXXXXX`,
  then the one-letter names); a name with no mapping becomes the token
  `⟪name⟫`, which can never match a candidate character and is counted as a
  *measurement* gap (`unmapped_names`), not an engine divergence;
* candidate: the cluster text;
* both: NFKD (math alphanumerics `𝑥` -> `x`, ligatures `ﬁ` -> `fi`, `é` ->
  `e` + U+0301), then `FOLD` (typographic variants a PDF text layer treats as
  one character: U+2212 and `-`, curly and straight quotes, …), whitespace
  dropped.

A glyph's key is a string of zero or more characters: the `fi` ligature is
one glyph and two characters on both sides; pdfTeX's OT1 `\\'e` is two
glyphs (`e`, `acute` -> U+0301) where an OpenType font draws one precomposed
`é` (NFKD: `e` + U+0301) -- the character multisets agree, the glyph origins
do not (the accent sits elsewhere), and the scoreboard's L2/L3 split says
exactly that.
"""

import re
import unicodedata

# PostScript glyph names used by pdfTeX's Type 1 fonts (cm*, ams, ec/lm via
# .enc files) that are not a single ASCII letter. Extensible-delimiter
# pieces map to the delimiter they build, so a big `\left(` compares as `(`.
NAME_TO_TEXT = {
    # ASCII punctuation / digits
    "space": " ", "exclam": "!", "quotedbl": '"', "numbersign": "#", "dollar": "$", "percent": "%",
    "ampersand": "&", "quoteright": "\u2019", "quotesingle": "'", "parenleft": "(", "parenright": ")",
    "asterisk": "*", "plus": "+", "comma": ",", "hyphen": "-", "sfthyphen": "-", "hyphenchar": "-",
    "period": ".", "slash": "/", "zero": "0", "one": "1", "two": "2", "three": "3", "four": "4",
    "five": "5", "six": "6", "seven": "7", "eight": "8", "nine": "9", "colon": ":", "semicolon": ";",
    "less": "<", "equal": "=", "greater": ">", "question": "?", "at": "@", "bracketleft": "[",
    "backslash": "\\", "bracketright": "]", "asciicircum": "^", "underscore": "_", "quoteleft": "\u2018",
    "grave": "\u0300", "braceleft": "{", "bar": "|", "braceright": "}", "asciitilde": "~",
    "exclamdown": "¡", "questiondown": "¿", "visiblespace": "␣",
    # ligatures and dashes
    "ff": "ff", "fi": "fi", "fl": "fl", "ffi": "ffi", "ffl": "ffl",
    "endash": "–", "emdash": "—", "quotedblleft": "“", "quotedblright": "”", "quotedblbase": "„",
    "quotesinglbase": "‚", "guillemotleft": "«", "guillemotright": "»", "guilsinglleft": "‹",
    "guilsinglright": "›", "compwordmark": "", "perthousandzero": "0",
    # accents (spacing forms -> combining, matching NFKD of a precomposed letter)
    "acute": "́", "circumflex": "̂", "tilde": "̃", "macron": "̄", "breve": "̆",
    "dotaccent": "̇", "dieresis": "̈", "ring": "̊", "hungarumlaut": "̋",
    "caron": "̌", "cedilla": "̧", "ogonek": "̨", "tie": "͡",
    # Latin letters beyond ASCII
    "dotlessi": "ı", "dotlessj": "ȷ", "germandbls": "ß", "ae": "æ", "AE": "Æ", "oe": "œ", "OE": "Œ",
    "oslash": "ø", "Oslash": "Ø", "aring": "å", "Aring": "Å", "lslash": "ł", "Lslash": "Ł",
    "eth": "ð", "Eth": "Ð", "thorn": "þ", "Thorn": "Þ", "eng": "ŋ", "Eng": "Ŋ", "dcroat": "đ",
    "Dcroat": "Đ", "Germandbls": "SS", "SS": "SS", "ij": "ij", "IJ": "IJ",
    "sterling": "£", "section": "§", "paragraph": "¶", "dagger": "†", "daggerdbl": "‡",
    "bullet": "•", "copyright": "©", "registered": "®", "trademark": "™", "degree": "°",
    "ellipsis": "…", "periodcentered": "·", "multiply": "×", "divide": "÷", "plusminus": "±",
    "minusplus": "∓", "logicalnot": "¬", "mu": "μ", "currency": "¤", "yen": "¥", "cent": "¢",
    "Euro": "€", "brokenbar": "¦", "ordfeminine": "ª", "ordmasculine": "º", "onesuperior": "1",
    "twosuperior": "2", "threesuperior": "3", "onehalf": "1/2", "onequarter": "1/4",
    "threequarters": "3/4", "florin": "ƒ",
    # Greek (cmmi / cmr)
    "alpha": "α", "beta": "β", "gamma": "γ", "delta": "δ", "epsilon": "ϵ", "epsilon1": "ε",
    "zeta": "ζ", "eta": "η", "theta": "θ", "theta1": "ϑ", "iota": "ι", "kappa": "κ", "lambda": "λ",
    "nu": "ν", "xi": "ξ", "pi": "π", "pi1": "ϖ", "rho": "ρ", "rho1": "ϱ", "sigma": "σ", "sigma1": "ς",
    "tau": "τ", "upsilon": "υ", "phi": "ϕ", "phi1": "φ", "chi": "χ", "psi": "ψ", "omega": "ω",
    "Gamma": "Γ", "Delta": "Δ", "Theta": "Θ", "Lambda": "Λ", "Xi": "Ξ", "Pi": "Π", "Sigma": "Σ",
    "Upsilon": "Υ", "Upsilon1": "Υ", "Phi": "Φ", "Psi": "Ψ", "Omega": "Ω", "digamma": "ϝ", "kappa1": "ϰ",
    # cmmi specials
    "harpoonleftup": "↼", "harpoonleftdown": "↽", "harpoonrightup": "⇀", "harpoonrightdown": "⇁",
    "arrowhookleft": "↩", "arrowhookright": "↪", "triangleright": "▹", "triangleleft": "◃",
    "star": "⋆", "partialdiff": "∂", "flat": "♭", "natural": "♮", "sharp": "♯", "slurbelow": "⌣",
    "slurabove": "⌢", "lscript": "ℓ", "weierstrass": "℘", "tie1": "͡", "vector": "⃗",
    "dotlessi1": "ı",
    # cmsy
    "minus": "−", "asteriskmath": "∗", "circleplus": "⊕", "circleminus": "⊖", "circlemultiply": "⊗",
    "circledivide": "⊘", "circledot": "⊙", "circlecopyrt": "◯", "openbullet": "∘", "equivasymptotic": "≍",
    "equivalence": "≡", "reflexsubset": "⊆", "reflexsuperset": "⊇", "lessequal": "≤", "greaterequal": "≥",
    "precedesequal": "⪯", "followsequal": "⪰", "similar": "∼", "approxequal": "≈", "propersubset": "⊂",
    "propersuperset": "⊃", "lessmuch": "≪", "greatermuch": "≫", "precedes": "≺", "follows": "≻",
    "arrowleft": "←", "arrowright": "→", "arrowup": "↑", "arrowdown": "↓", "arrowboth": "↔",
    "arrownortheast": "↗", "arrowsoutheast": "↘", "similarequal": "≃", "arrowdblleft": "⇐",
    "arrowdblright": "⇒", "arrowdblup": "⇑", "arrowdbldown": "⇓", "arrowdblboth": "⇔",
    "arrownorthwest": "↖", "arrowsouthwest": "↙", "proportional": "∝", "prime": "′", "infinity": "∞",
    "element": "∈", "owner": "∋", "triangle": "△", "triangleinv": "▽", "negationslash": "̸",
    "mapsto": "↦", "universal": "∀", "existential": "∃", "emptyset": "∅", "Rfractur": "ℜ",
    "Ifractur": "ℑ", "latticetop": "⊤", "perpendicular": "⊥", "aleph": "ℵ", "union": "∪",
    "intersection": "∩", "unionmulti": "⊎", "logicaland": "∧", "logicalor": "∨", "turnstileleft": "⊢",
    "turnstileright": "⊣", "floorleft": "⌊", "floorright": "⌋", "ceilingleft": "⌈", "ceilingright": "⌉",
    "angbracketleft": "⟨", "angbracketright": "⟩", "bardbl": "‖", "arrowbothv": "↕", "arrowdblbothv": "⇕",
    "radical": "√", "coproduct": "∐", "nabla": "∇", "integral": "∫", "unionsq": "⊔", "intersectionsq": "⊓",
    "subsetsqequal": "⊑", "supersetsqequal": "⊒", "club": "♣", "diamond": "◇", "heart": "♡", "spade": "♠",
    "diamondmath": "⋄", "wreathproduct": "≀", "dagger1": "†",
    # cmex (sizes and extensible pieces fold to the delimiter they build)
    "summationtext": "∑", "summationdisplay": "∑", "producttext": "∏", "productdisplay": "∏",
    "integraltext": "∫", "integraldisplay": "∫", "contintegraltext": "∮", "contintegraldisplay": "∮",
    "uniontext": "⋃", "uniondisplay": "⋃", "intersectiontext": "⋂", "intersectiondisplay": "⋂",
    "unionmultitext": "⨄", "unionmultidisplay": "⨄", "logicalandtext": "⋀", "logicalanddisplay": "⋀",
    "logicalortext": "⋁", "logicalordisplay": "⋁", "coproducttext": "∐", "coproductdisplay": "∐",
    "circleplustext": "⨁", "circleplusdisplay": "⨁", "circlemultiplytext": "⨂", "circlemultiplydisplay": "⨂",
    "circledottext": "⨀", "circledotdisplay": "⨀", "unionsqtext": "⨆", "unionsqdisplay": "⨆",
    "hatwide": "̂", "hatwider": "̂", "hatwidest": "̂",
    "tildewide": "̃", "tildewider": "̃", "tildewidest": "̃",
}

_DELIMS = {
    "parenleft": "(", "parenright": ")", "bracketleft": "[", "bracketright": "]", "braceleft": "{",
    "braceright": "}", "floorleft": "⌊", "floorright": "⌋", "ceilingleft": "⌈", "ceilingright": "⌉",
    "angbracketleft": "⟨", "angbracketright": "⟩", "slash": "/", "backslash": "\\", "radical": "√",
    "bar": "|", "bardbl": "‖", "arrowup": "↑", "arrowdown": "↓", "arrowdblup": "⇑", "arrowdbldown": "⇓",
}
for _base, _t in _DELIMS.items():
    for _suffix in ("big", "Big", "bigg", "Bigg", "BIG", "bt", "tp", "ex", "mid", "vertical", "verticalbt",
                    "verticaltp", "bigBig", "Bigg1"):
        NAME_TO_TEXT.setdefault(_base + _suffix, _t)
for _n in ("radicalbig", "radicalBig", "radicalbigg", "radicalBigg", "radicalbt", "radicalvertical", "radicaltp"):
    NAME_TO_TEXT[_n] = "√"
for _n in ("braceleftmid", "bracerightmid", "braceex", "braceleftbt", "bracerighttp", "bracelefttp", "bracerightbt"):
    NAME_TO_TEXT.setdefault(_n, "{" if "left" in _n else "}")
NAME_TO_TEXT["braceex"] = "{"
for _n in ("parenleftex", "parenrightex", "bracketleftex", "bracketrightex", "barex", "bardblex"):
    NAME_TO_TEXT.setdefault(_n, NAME_TO_TEXT.get(_n[:-2], "|"))

# Characters a PDF text layer treats as one (both sides are folded).
FOLD = {
    "\u02c6": "\u0302", "\u02c7": "\u030c", "\u02dc": "\u0303", "\u02c9": "\u0304",
    "−": "-",   # minus sign / hyphen-minus (rank.norm folds these too)
    "‐": "-", "‑": "-", "­": "-",
    "‘": "'", "’": "'", "ʼ": "'", "`": "'",
    "“": '"', "”": '"',
    "∗": "*",
    "ı": "i", "ȷ": "j",  # dotless i/j carry an accent in OT1: same letter to a reader
    "ϵ": "ε", "ε": "ε",  # lunate and straight epsilon: cmmi vs Unicode math naming
    "ϕ": "φ", "φ": "φ",
    "′": "'",
    "∣": "|", "∥": "‖", "‖": "‖",
    "⟨": "⟨", "〈": "⟨", "⟩": "⟩", "〉": "⟩",
    "∅": "∅", "⌀": "∅",
    "∖": "\\", "⧵": "\\",
    "⋅": "·", "·": "·",
    "∶": ":",
    "→": "→", "⟶": "→",
    "←": "←", "⟵": "←",
    "⇒": "⇒", "⟹": "⇒",
    "⇐": "⇐", "⟸": "⇐",
    "⇔": "⇔", "⟺": "⇔",
    "↔": "↔", "⟷": "↔",
    "↦": "↦", "⟼": "↦",
}

# AMS msam/msbm and a few other symbol-font names seen in pdfTeX output.
NAME_TO_TEXT.update({
    "square": "□", "squaresolid": "■", "trianglerightsld": "▶", "triangleleftsld": "◀", "triangleright1": "▷",
    "triangleleft1": "◁", "triangleup": "△", "triangledown": "▽", "trianglesolid": "▲", "triangleinvsolid": "▼",
    "lozenge": "◊", "lozengesolid": "⧫", "checkmark": "✓", "maltesecross": "✠", "circleR": "®", "circleS": "Ⓢ",
    "subsetnoteql": "⊊", "supersetnoteql": "⊋", "subsetnotequal": "⊊", "supersetnotequal": "⊋",
    "notbar": "∤", "notparallel": "∦", "notsubseteql": "⊈", "notsupersetequal": "⊉", "lessorequalslant": "⩽",
    "greaterorequalslant": "⩾", "notlessequal": "≰", "notgreaterequal": "≱",
    "notless": "≮", "notgreater": "≯", "notsimilar": "≁", "notequal": "≠",
    "therefore": "∴", "because": "∵", "emptyset1": "∅", "emptysetalt": "∅", "nexists": "∄", "backprime": "‵",
    "complement": "∁", "hbar": "ℏ", "planckover2pi": "ℏ", "planckover2pi1": "ℏ", "Finv": "Ⅎ", "Game": "⅁",
    "beth": "ℶ", "gimel": "ℷ", "daleth": "ℸ", "angle": "∠", "measuredangle": "∡", "sphericalangle": "∢",
    "multicloseleft": "⋉", "multicloseright": "⋊", "dotplus": "∔", "intercal": "⊺", "barwedge": "⊼",
    "doublebarwedge": "⩞", "circleasterisk": "⊛", "circlering": "⊚", "boxplus": "⊞",
    "boxminus": "⊟", "boxmultiply": "⊠", "boxdot": "⊡", "equalsdots": "≑", "defines": "≜",
    "subsetdbl": "⋐", "supersetdbl": "⋑", "uniondbl": "⋓", "intersectiondbl": "⋒",
    "harpoonleftright": "⇌", "harpoonrightleft": "⇋",
    "arrowtailright": "↣", "arrowtailleft": "↢", "Lsh": "↰", "Rsh": "↱", "curlyveeuprise": "⋎",
    "curlywedgeuprise": "⋏", "blacksquare": "■",
    "visualspace": "␣", "vextendsingle": "|", "vextenddouble": "‖", "radicalvertex": "√", "arrowvertex": "↑",
    "arrowtp": "↑", "arrowbt": "↓", "arrowdblvertex": "⇑", "arrowdbltp": "⇑", "arrowdblbt": "⇓",
    "mapsto": "↦", "mapstochar": "↦", "arrowhookleft": "↪", "arrowhookright": "↩",
    "careof": "℅", "numero": "№", "cwm": "", "Ng": "Ŋ", "ng": "ŋ",
})

# Extensible-delimiter pieces: pdfTeX stacks them (top, repeat, middle,
# bottom) where an OpenType MATH font draws one cluster. `reference_atoms`
# merges a vertical run of pieces into one character at the first piece.
PIECE_RE = re.compile(r"(tp|bt|ex|mid|vertex|vertical)$|^vextend|^braceex$")

# Candidate characters pdfTeX never draws as one glyph: LaTeX builds them
# from pieces (`\longrightarrow` = `\relbar\joinrel\rightarrow`, `\mapsto`
# = `\mapstochar\rightarrow`, `\cdots` = three `\cdotp`, ...), so the
# candidate's one glyph is expanded into the reference's characters.
CANDIDATE_EXPAND = {
    "↦": "↦→", "⟼": "↦-→", "⟶": "-→", "⟵": "←-", "⟷": "←→", "⟹": "=⇒", "⟸": "⇐=", "⟺": "⇐⇒",
    "↪": "↪→", "↩": "←↩", "⋯": "···", "⋮": "...", "⋱": "...", "⊨": "|=", "≅": "∼=", "≐": ".=",
}

UNMAPPED_OPEN, UNMAPPED_CLOSE = "⟪", "⟫"
_ACCENT_SUFFIX = {"acute": "\u0301", "grave": "\u0300", "circumflex": "\u0302", "tilde": "\u0303",
                  "dieresis": "\u0308", "ring": "\u030a", "cedilla": "\u0327", "caron": "\u030c",
                  "breve": "\u0306", "macron": "\u0304", "ogonek": "\u0328", "dotaccent": "\u0307",
                  "hungarumlaut": "\u030b", "commaaccent": "\u0326"}
_ACCENTED = re.compile(r"([A-Za-z]|dotlessi|dotlessj|ae|AE|oe|OE|oslash|Oslash)(" + "|".join(_ACCENT_SUFFIX) + r")")


def name_text(name):
    """Unicode text for a PostScript glyph name, or None."""
    if name is None:
        return None
    if name in NAME_TO_TEXT:
        return NAME_TO_TEXT[name]
    if len(name) == 1 and name.isascii() and name.isalpha():
        return name
    m = re.fullmatch(r"uni([0-9A-Fa-f]{4})+", name)
    if m:
        hexes = re.findall(r"[0-9A-Fa-f]{4}", name[3:])
        return "".join(chr(int(h, 16)) for h in hexes)
    m = re.fullmatch(r"u([0-9A-Fa-f]{4,6})", name)
    if m:
        return chr(int(m.group(1), 16))
    m = _ACCENTED.fullmatch(name)
    if m:
        base = name_text(m.group(1)) if len(m.group(1)) > 1 else m.group(1)
        return (base or m.group(1)) + _ACCENT_SUFFIX[m.group(2)]
    # suffixed variants: `a.sc`, `one.oldstyle`, `A.swash`, `alpha1`
    base = name.split(".", 1)[0]
    if base != name:
        return name_text(base)
    return None


def normalize(text):
    """NFKD, fold, drop whitespace. Returns a str (possibly empty)."""
    out = []
    for ch in unicodedata.normalize("NFKD", text or ""):
        ch = FOLD.get(ch, ch)
        if ch.isspace():
            continue
        out.append(ch)
    # A second pass: FOLD may produce characters NFKD would split further.
    return "".join(FOLD.get(c, c) for c in unicodedata.normalize("NFKD", "".join(out)))


def reference_key(glyph):
    """Key for a `pdftext.page_glyphs` record."""
    name = glyph.get("name")
    t = name_text(name)
    if t is None and name is None:
        t = glyph.get("text")
        if t == "?":
            t = None
    if t is None:
        return UNMAPPED_OPEN + (name or f"{glyph.get('font')}#{glyph.get('code')}") + UNMAPPED_CLOSE
    return normalize(t)


def candidate_key(glyph):
    """Key for a `rank.v2_page_glyphs` record (cluster text)."""
    text = glyph.get("text") or ""
    if any(c in CANDIDATE_EXPAND for c in text):
        text = "".join(CANDIDATE_EXPAND.get(c, c) for c in text)
    return normalize(text)


def is_piece(name):
    return bool(name) and bool(PIECE_RE.search(name))


def chars(key):
    """The characters of a key; an unmapped token is one indivisible char."""
    if key.startswith(UNMAPPED_OPEN):
        return [key]
    return list(key)
