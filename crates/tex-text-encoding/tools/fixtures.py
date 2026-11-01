"""One-line oracle fixtures for KC-105 (text encodings, accents, symbols, space factor).

Each fixture is typeset as ``\\setbox0\\hbox{<input>}`` under every variant
preamble.  Inputs are plain TeX/LaTeX source (UTF-8).  Keep ids stable: the
committed expected JSON is keyed by ``<variant>/<id>``.
"""

VARIANTS = {
    # LaTeX defaults: OT1 text encoding, Computer Modern (cmr10 / tcrm1000 / cmsy10).
    "ot1-cm": r"\documentclass{article}",
    # [T1]{fontenc} with Computer Modern -> EC / CM-Super (ecrm1000).
    "t1-cm": r"\documentclass{article}\usepackage[T1]{fontenc}",
    # lmodern with the default OT1 encoding (rm-lmr10).
    "ot1-lm": r"\documentclass{article}\usepackage{lmodern}",
    # The common modern preamble (ec-lmr10, ts1-lmr10).
    "t1-lm": r"\documentclass{article}\usepackage[T1]{fontenc}\usepackage{lmodern}",
    # Explicit inputenc/textcomp must be a no-op relative to t1-lm (LaTeX >= 2018/2020).
    "t1-lm-inputenc": r"\documentclass{article}\usepackage[utf8]{inputenc}"
    r"\usepackage[T1]{fontenc}\usepackage{lmodern}\usepackage{textcomp}",
}

ACCENTS = [
    ("acute-e", r"\'e"), ("grave-e", r"\`e"), ("circ-e", r"\^e"), ("uml-o", r"\"o"),
    ("tilde-n", r"\~n"), ("macron-o", r"\=o"), ("dot-z", r"\.z"), ("breve-g", r"\u g"),
    ("caron-s", r"\v s"), ("hung-o", r"\H o"), ("cedilla-c", r"\c c"),
    ("cedilla-C", r"\c C"), ("dotbelow-s", r"\d s"), ("barbelow-b", r"\b b"),
    ("ogonek-a", r"\k a"), ("ogonek-u", r"\k u"), ("ring-u", r"\r u"), ("ring-A", r"\r A"),
    ("tie-oo", r"\t{oo}"), ("acute-dotlessi", r"\'\i"), ("circ-dotlessj", r"\^\j"),
    ("uml-i", r"\"i"), ("dot-i", r"\.i"), ("acute-empty", r"\'{}"), ("circ-empty", r"\^{}"),
    ("acute-E", r"\'E"), ("uml-O", r"\"O"), ("caron-Z", r"\v Z"), ("acute-x", r"\'x"),
    ("caron-h", r"\v h"), ("tilde-e", r"\~e"), ("macron-g", r"\=g"), ("circ-A-word", r"B\^Ac"),
    ("dotbelow-O", r"\d O"), ("barbelow-T", r"\b T"), ("uml-y", r"\"y"),
    ("acute-braced", r"\'{e}t\'{e}"), ("cedilla-S", r"\c S"),
]

SYMBOLS = [
    ("ss", r"\ss"), ("SS", r"\SS"), ("ae", r"\ae"), ("AE", r"\AE"), ("oe", r"\oe"),
    ("OE", r"\OE"), ("o", r"\o"), ("O", r"\O"), ("l", r"\l"), ("L", r"\L"), ("aa", r"\aa"),
    ("AA", r"\AA"), ("i", r"\i"), ("j", r"\j"), ("textbackslash", r"\textbackslash"),
    ("textasciitilde", r"\textasciitilde"), ("textasciicircum", r"\textasciicircum"),
    ("S", r"\S"), ("P", r"\P"), ("dag", r"\dag"), ("ddag", r"\ddag"),
    ("copyright", r"\copyright"), ("pounds", r"\pounds"), ("textbullet", r"\textbullet"),
    ("textendash", r"\textendash"), ("textemdash", r"\textemdash"),
    ("textquoteleft", r"\textquoteleft"), ("textquoteright", r"\textquoteright"),
    ("textquotedblleft", r"\textquotedblleft"), ("textquotedblright", r"\textquotedblright"),
    ("quote-single", r"`a'"), ("quote-double", r"``a''"), ("quote-base", r",,a''"),
    ("quotedblbase", r"\quotedblbase"), ("quotesinglbase", r"\quotesinglbase"),
    ("guillemotleft", r"\guillemotleft"), ("guillemotright", r"\guillemotright"),
    ("guilsinglleft", r"\guilsinglleft"), ("textquotedbl", r"\textquotedbl"),
    ("texteuro", r"\texteuro"), ("textdegree", r"\textdegree"),
    ("textregistered", r"\textregistered"), ("texttrademark", r"\texttrademark"),
    ("textexclamdown", r"\textexclamdown"), ("textquestiondown", r"\textquestiondown"),
    ("exclamdown-lig", r"!`a"), ("th", r"\th"), ("dh", r"\dh"), ("NG", r"\NG"),
    ("textsection", r"\textsection"), ("textless", r"\textless"), ("textbar", r"\textbar"),
    ("ij", r"\ij"), ("endash-lig", r"a--b"), ("emdash-lig", r"a---b"),
    ("textperiodcentered", r"\textperiodcentered"), ("textdagger", r"\textdagger"),
    ("textbraceleft", r"\textbraceleft"),
]

UTF8 = [
    ("u-eacute", "é"), ("u-agrave", "à"), ("u-uuml", "ü"), ("u-ntilde", "ñ"),
    ("u-ccedil", "ç"), ("u-Ccedil", "Ç"), ("u-oslash", "ø"), ("u-szlig", "ß"),
    ("u-Aring", "Å"), ("u-aring", "å"), ("u-lstroke", "ł"), ("u-Lstroke", "Ł"),
    ("u-oe", "œ"), ("u-ae", "æ"), ("u-odblac", "ő"), ("u-aogonek", "ą"), ("u-eogonek", "ę"),
    ("u-zcaron", "ž"), ("u-ccaron", "č"), ("u-gbreve", "ğ"), ("u-dotlessi", "ı"),
    ("u-iexcl", "¡"), ("u-iquest", "¿"), ("u-bdquo", "„a“"), ("u-laquo", "«a»"),
    ("u-lsaquo", "‹"), ("u-ndash", "a–b"), ("u-mdash", "a—b"), ("u-lsquo", "‘a’"),
    ("u-ldquo", "“a”"), ("u-bull", "•"), ("u-sect", "§"), ("u-para", "¶"),
    ("u-dagger", "†"), ("u-Dagger", "‡"), ("u-copy", "©"), ("u-reg", "®"), ("u-pound", "£"),
    ("u-euro", "€"), ("u-deg", "°"), ("u-plusmn", "±"), ("u-times", "×"), ("u-micro", "µ"),
    ("u-frac12", "½"), ("u-hellip-undeclared", "…"), ("u-cyrillic-undeclared", "Ж"),
    ("u-emoji-undeclared", "😀"), ("u-nbsp", "a b"), ("u-capital-sharp-s", "ẞ"),
    ("u-scommabelow", "ș"), ("u-ecircumflex-word", "fête"), ("u-idiaeresis", "ï"),
    ("u-Yacute", "Ý"), ("u-thorn", "þ"), ("u-eng", "ŋ"), ("u-mixed", "Ångström"),
]

SPACE_FACTOR = [
    ("sf-sentence", r"a. b"), ("sf-upper-period", r"A. B"), ("sf-paren", r"a.) b"),
    ("sf-quote", r"a.'' b"), ("sf-at", r"A\@. B"), ("sf-comma", r"a, b"),
    ("sf-semicolon", r"a; b"), ("sf-colon", r"a: b"), ("sf-question", r"a? b"),
    ("sf-exclam", r"a! b"), ("sf-french", r"\frenchspacing a. b: c"),
    ("sf-accent-upper", r"\'E. b"), ("sf-utf8-upper", "É. b"), ("sf-lstroke-upper", r"\L. b"),
    ("sf-control-space", r"a.\ b"), ("sf-eg", r"e.g.\ b"), ("sf-ss", r"\ss. b"),
    ("sf-AE", r"\AE. b"), ("sf-at-after-lower", r"a\@ b"), ("sf-bracket", r"a.] b"),
    ("sf-plain", r"a b"), ("sf-nonfrench-reset", r"\frenchspacing\nonfrenchspacing a. b"),
    ("sf-accent-lower", r"\'e. b"), ("sf-digit", r"1. b"), ("sf-two-spaces", r"a.  b"),
]

FIXTURES = ACCENTS + SYMBOLS + UTF8 + SPACE_FACTOR
