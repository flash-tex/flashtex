#!/usr/bin/env python3
r"""The exhaustive declared-math oracle: every symbol LaTeX declares, against pdflatex.

ORACLE TOOLING ONLY (pdflatex never runs in the product path; cargo never
runs TeX). The test documents are generated mechanically from the same
declarations `crates/compiler/scripts/gen_math_symbols.py` reads, so the
check is complete by construction:

  1. every `\DeclareMathSymbol` (kernel, latexsym, amsfonts/amssymb,
     stmaryrd, amsmath) in text style, display style, script and
     scriptscript size;
  2. every `\DeclareMathDelimiter` at natural size, `\big`/`\Big`/`\bigg`/
     `\Bigg`, and one tall `\left`/`\right` extensible case;
  3. every `\DeclareMathAccent` over a narrow and a wide nucleus, and the
     radical;
  4. the full 8x8 atom-class spacing matrix (Ord/Op/Bin/Rel/Open/Close/
     Punct/Inner, including the Bin -> Ord demotions) in display, text and
     script style;
  5. every math alphabet over A-Z, a-z and 0-9 where the font has them;
  6. every zero-argument composite the declaring files `\def` (`\cong`,
     `\bowtie`, `\models`, the long arrows, ...).

For every `<doc>.tex` this writes `expected/<doc>.txt`: for each formula
(one per line, labelled by a four-digit number set in text before it) the
label's origin and every glyph pdfTeX placed after it, as
`g <font> <code> <size> <x> <y_top> <text>` in bp from the page's top-left
(the display-list-v2 convention). `<text>` is the character the generated
table gives that font slot (empty when it has none).

It also checks, per symbol formula, that pdfTeX set the symbol from the
font and slot the declaration table records -- the declaration table is
verified against pdflatex here, independently of the engine -- and lists
every mismatch. `tests/declared_math_oracle.rs` then compares the engine's
glyphs with the expected files, without TeX.

Usage: python3 crates/render-pipeline/fixtures/declared-math/generate.py [--only kernel-symbols]
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "crates", "compiler", "scripts"))
sys.path.insert(0, os.path.join(REPO, "tools", "visual-oracle"))
import gen_math_symbols as gms  # noqa: E402
import pdftext  # noqa: E402

PDFLATEX = shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex"

STYLES = [
    ("text", "$a{}b$"),
    ("display", "$\\displaystyle a{}b$"),
    ("script", "$x^{{a{}b}}$"),
    ("scriptscript", "$x^{{y^{{a{}b}}}}$"),
]
DELIM_STYLES = [
    ("natural", "$a{}b$"),
    ("big", "$a\\big{}b$"),
    ("Big", "$a\\Big{}b$"),
    ("bigg", "$a\\bigg{}b$"),
    ("Bigg", "$a\\Bigg{}b$"),
    ("left", "$\\left{}\\frac{{\\frac{{a}}{{b}}}}{{\\frac{{c}}{{d}}}}\\right.$"),
]
ACCENT_STYLES = [
    ("narrow", "${}{{a}}$"),
    ("display", "$\\displaystyle{}{{a}}$"),
    ("wide", "${}{{abc}}$"),
]
# TeXbook Chapter 17 representatives: one atom of each class.
CLASS_REPS = [
    ("Ord", "a"), ("Op", "\\sum "), ("Bin", "+"), ("Rel", "="),
    ("Open", "("), ("Close", ")"), ("Punct", ","), ("Inner", "\\left(b\\right) "),
]
SPACING_STYLES = [
    ("display", "$\\displaystyle x{}y$"),
    ("text", "$x{}y$"),
    ("script", "$z^{{x{}y}}$"),
]
UPPER = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
LOWER = "abcdefghijklmnopqrstuvwxyz"
DIGITS = "0123456789"
# (alphabet, package, groups it has)
ALPHABETS = [
    ("mathrm", None, "ULD"), ("mathnormal", None, "ULD"), ("mathit", None, "ULD"),
    ("mathbf", None, "ULD"), ("mathsf", None, "ULD"), ("mathtt", None, "ULD"),
    ("mathcal", None, "U"),
    ("mathbb", "amssymb", "U"), ("mathfrak", "amssymb", "ULD"),
    ("mathscr", "mathrsfs", "U"),
]
# Zero-argument composites worth a formula of their own (the `fill` and
# `\big` family take arguments or are text-mode leaders).
COMPOSITE_SKIP = {"joinrel", "skew", "rightarrowfill", "leftarrowfill", "downbracefill", "upbracefill",
                  "big", "Big", "bigg", "Bigg", "frak", "Bbb", "bold", "newsymbol", "overrightarrow",
                  "overleftarrow", "overbrace", "underbrace", "widehat", "widetilde", "varcopyright",
                  "int", "oint", "mathunderscore"}

PREAMBLE = """\\documentclass{article}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
"""
# Output encoding only (no layout change), for the oracle's PDF reader; not
# in the committed document, which the engine compiles as well.
PDF_ONLY = "\\pdfcompresslevel=0\n\\pdfobjcompresslevel=0\n"

PROVIDER_PACKAGE = {"Kernel": None, "Latexsym": "latexsym", "Amsfonts": "amsfonts", "Amssymb": "amssymb",
                    "Stmaryrd": "stmaryrd", "Mathrsfs": "mathrsfs", "Amsmath": "amsmath"}


def spelling(r):
    return r["name"] if r["character"] else "\\" + r["name"]


_TFMS = {}


def successors(font, slot):
    """The larger-successor chain of a slot in the font's 10pt TFM, after the
    slot itself: [(font, next), (font, next-next), ...]."""
    if font not in _TFMS:
        _TFMS[font] = gms.gen_cm_tfm.parse(gms.kpse(gms.SYMBOL_FONTS[font][1] + ".tfm"))
    chars = _TFMS[font]["chars"]
    out = []
    cur = chars.get(slot, {}).get("next")
    while cur is not None and cur not in [s for _, s in out] and cur != slot and len(out) < 8:
        out.append((font, cur))
        cur = chars.get(cur, {}).get("next")
    return out


def pieces(font, slot):
    """The extensible recipe of a slot (top, middle, bottom, repeater), as
    (font, slot) pairs, or [] when the slot is not extensible."""
    successors(font, slot)  # loads the TFM
    e = _TFMS[font]["chars"].get(slot, {}).get("ext")
    return [(font, p) for p in e if p] if e else []


def usable(r):
    """A declaration a document can name: no `@` pieces."""
    return "@" not in r["name"]


class Doc:
    def __init__(self, name, packages):
        self.name = name
        self.packages = packages
        self.formulas = []  # (category, name, style, tex, expect_font, expect_slot)

    def add(self, category, name, style, tex, expect=None):
        self.formulas.append((category, name, style, tex, expect))

    def tex(self):
        out = [PREAMBLE]
        for p in self.packages:
            out.append(f"\\usepackage{{{p}}}\n")
        out.append("\\begin{document}\n")
        for i, (_, _, _, tex, _) in enumerate(self.formulas):
            out.append(f"{i:04d} {tex}\\par\n")
        out.append("\\end{document}\n")
        return "".join(out)


def symbol_docs(rows, d):
    docs = {}

    def doc_for(provider, suffix=""):
        pkg = PROVIDER_PACKAGE[provider]
        key = (pkg or "kernel") + suffix
        if key not in docs:
            docs[key] = Doc(key, [pkg] if pkg else [])
        return docs[key]

    # The character's `\mathcode` (a `\DeclareMathSymbol` of the same
    # character) is what a delimiter character sets at natural size: `/` is
    # letters "3D in `$a/b$` and operators "2F only under `\left`/`\big`.
    mathcode = {r["name"]: (r["font"], r["slot"]) for r in rows
                if r["kind"] == "symbol" and r["character"] and r["provider"] == "Kernel"}
    for r in rows:
        if not usable(r):
            continue
        sp = spelling(r)
        # A character with a space after it would not need one, but a control
        # word does; the templates put the symbol directly between `a` and `b`
        # so `\alpha b` tokenises as intended.
        if not r["character"]:
            sp = sp + " "
        expect = [(r["font"], r["slot"])]
        if r["kind"] == "symbol":
            doc = doc_for(r["provider"], "-symbols")
            for style, tmpl in STYLES:
                # A large operator takes its successor in display style
                # (TeX rule 13: cmex "50 `\sum` becomes "58).
                exp = expect + successors(r["font"], r["slot"]) if (style == "display" and r["class"] == "op") else expect
                doc.add("symbol", r["name"], style, tmpl.format(sp), exp)
        elif r["kind"] == "delimiter":
            doc = doc_for(r["provider"], "-delimiters")
            # `\big` is the first large variant, `\Big`/`\bigg`/`\Bigg` the
            # next three of the TFM's larger-successor chain (cmex "00 -> "10
            # -> "12 -> "20 for `(`); `\left` picks by height.
            sizes = [(r["large_font"], r["large_slot"])] + successors(r["large_font"], r["large_slot"])
            # Past the end of the chain TeX builds the extensible from the
            # last slot's pieces (`\big\arrowvert` is repeated cmex "3F).
            ext = pieces(sizes[-1][0], sizes[-1][1])
            for k, (style, tmpl) in enumerate(DELIM_STYLES):
                if style == "natural":
                    exp = [mathcode.get(r["name"], expect[0])] if r["character"] else expect
                elif style == "left":
                    exp = None
                else:
                    # The k-th size, or the pieces once TeX finds no glyph
                    # tall enough (an extensible-only chain goes to pieces
                    # already at `\big`).
                    exp = ([sizes[k - 1]] if k - 1 < len(sizes) else []) + ext or [sizes[-1]]
                doc.add("delimiter", r["name"], style, tmpl.format(sp), exp)
        elif r["kind"] == "accent":
            doc = doc_for(r["provider"], "-accents")
            for style, tmpl in ACCENT_STYLES:
                # A wide accent grows along the successor chain to cover the nucleus.
                exp = expect if style != "wide" else expect + successors(r["font"], r["slot"])
                doc.add("accent", r["name"], style, tmpl.format(sp.rstrip()), exp)
        elif r["kind"] == "radical":
            doc = doc_for(r["provider"], "-accents")
            doc.add("radical", r["name"], "narrow", "$\\sqrt{a}$", expect)
            doc.add("radical", r["name"], "display", "$\\displaystyle\\sqrt{a}$", expect)
            doc.add("radical", r["name"], "tall", "$\\sqrt{\\frac{a}{b}}$", None)
    # amsfonts' wide accents are `\xdef`s, not declarations: test them as accents.
    ams = doc_for("Amssymb", "-accents")
    for name in ("widehat", "widetilde"):
        for style, tmpl in ACCENT_STYLES:
            ams.add("accent", name, style, tmpl.format("\\" + name), None)
        ams.add("accent", name, "extrawide", "$\\%s{abcdefgh}$" % name, None)
    # Composites.
    for c in d["composites"]:
        if c["name"] in COMPOSITE_SKIP or "@" in c["name"]:
            continue
        doc = doc_for(c["provider"], "-composites")
        for style, tmpl in STYLES:
            doc.add("composite", c["name"], style, tmpl.format("\\" + c["name"] + " "), None)
    # Aliases, where the target exists with only the alias's own package
    # loaded (stmaryrd's `\let\oast\circledast` needs amssymb as well).
    declared_by = {}
    for r in rows:
        declared_by.setdefault(r["name"], set()).add(r["provider"])
    for c in d["composites"]:
        declared_by.setdefault(c["name"], set()).add(c["provider"])
    for a in d["aliases"]:
        if not declared_by.get(a["target"], set()) & {"Kernel", a["provider"]}:
            continue
        doc = doc_for(a["provider"], "-symbols")
        for style, tmpl in STYLES[:2]:
            doc.add("alias", a["alias"], style, tmpl.format("\\" + a["alias"] + " "), None)
    # Spacing matrix.
    sp = Doc("spacing", [])
    for lc, l in CLASS_REPS:
        for rc, r in CLASS_REPS:
            for style, tmpl in SPACING_STYLES:
                sp.add("spacing", f"{lc}-{rc}", style, tmpl.format(f"{l}{r}"), None)
    # Bin at the start and end of a list, and after Open/Punct.
    for style, tmpl in SPACING_STYLES:
        sp.add("spacing", "start-Bin", style, tmpl.format("+a").replace("x+a", "+a"), None)
        sp.add("spacing", "Bin-end", style, tmpl.format("a+").replace("a+y", "a+"), None)
    docs["spacing"] = sp
    # Alphabets.
    al = Doc("alphabets", sorted({p for _, p, _ in ALPHABETS if p}))
    for name, _, groups in ALPHABETS:
        for g, letters in (("U", UPPER), ("L", LOWER), ("D", DIGITS)):
            if g in groups:
                al.add("alphabet", name, {"U": "upper", "L": "lower", "D": "digits"}[g],
                       f"$\\{name}{{{letters}}}$", None)
    docs["alphabets"] = al
    return docs


def run_pdflatex(doc, scratch):
    src = os.path.join(scratch, doc.name + ".tex")
    open(src, "w").write(doc.tex().replace("\\pagestyle{empty}\n", PDF_ONLY + "\\pagestyle{empty}\n", 1))
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    for _ in range(2):
        p = subprocess.run([PDFLATEX, "-interaction=batchmode", "-halt-on-error", doc.name + ".tex"],
                           cwd=scratch, env=env, capture_output=True)
    pdf = os.path.join(scratch, doc.name + ".pdf")
    if p.returncode != 0 or not os.path.exists(pdf):
        log = open(os.path.join(scratch, doc.name + ".log"), errors="replace").read()
        m = re.search(r"^! .*$", log, re.M)
        sys.exit(f"pdflatex failed on {doc.name}: {m.group(0) if m else p.returncode}")
    return pdf


def pdf_glyphs(pdf):
    d = pdftext.PdfDocument.load(pdf)
    out = []
    for i, page in enumerate(d.pages()):
        glyphs, notes = pdftext.page_glyphs(d, page)
        for g in glyphs:
            g["page"] = i + 1
            out.append(g)
    return out


def base_font(font):
    """The PDF font name without a subset tag (`ABCDEF+CMR10` -> `CMR10`)."""
    return re.sub(r"^[A-Z]{6}\+", "", font)


def prefix_of(font):
    m = re.match(r"[A-Za-z]+", base_font(font))
    return m.group(0).upper() if m else font


# The Unicode the engine paints for an ASCII math character that has a
# dedicated mathematical code point.
CHAR_EQUIV = {"-": "−", "*": "∗", "'": "′"}


def slot_text(rows):
    """(pdf font prefix, code) -> every text a declaration gives that slot
    (`\mid`, `\vert` and `|` share cmsy "6A with `\arrowvert`'s small variant)."""
    m = {}

    def add(key, text):
        if text:
            m.setdefault(key, [])
            if text not in m[key]:
                m[key].append(text)
            if text in CHAR_EQUIV and CHAR_EQUIV[text] not in m[key]:
                m[key].append(CHAR_EQUIV[text])

    for r in rows:
        add((gms.SYMBOL_FONTS[r["font"]][2], r["slot"]), r["text"])
        if r["kind"] in ("delimiter", "radical"):
            add((gms.SYMBOL_FONTS[r["large_font"]][2], r["large_slot"]), r["text"])
    # Text alphabets: the character itself.
    for p in ("CMTI", "CMBX", "CMSS", "CMTT"):
        for c in UPPER + LOWER + DIGITS:
            add((p, ord(c)), c)
    return m


def main():
    only = None
    if "--only" in sys.argv:
        only = sys.argv[sys.argv.index("--only") + 1]
    d, rows, _ = gms.build()
    docs = symbol_docs(rows, d)
    texts = slot_text(rows)
    os.makedirs(os.path.join(HERE, "expected"), exist_ok=True)
    scratch = tempfile.mkdtemp(prefix="declared-math-")
    version = subprocess.run([PDFLATEX, "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    total_mismatch = 0
    for key in sorted(docs):
        if only and key != only:
            continue
        doc = docs[key]
        open(os.path.join(HERE, doc.name + ".tex"), "w").write(doc.tex())
        pdf = run_pdflatex(doc, scratch)
        glyphs = pdf_glyphs(pdf)
        # Split at the labels: the next label is the four digits of the
        # formula's index set in cmr10, in content order.
        out = [f"# GENERATED by fixtures/declared-math/generate.py; {version}; do not edit",
               f"# doc {doc.name} packages {','.join(doc.packages) or '-'}"]
        i = 0
        n = 0
        mismatches = []
        for fi, (category, name, style, tex, expect) in enumerate(doc.formulas):
            label = f"{fi:04d}"
            # Find the label.
            while i < len(glyphs):
                g = glyphs[i]
                if (base_font(g["font"]).startswith("CMR10") and all(
                        i + k < len(glyphs) and base_font(glyphs[i + k]["font"]).startswith("CMR10")
                        and glyphs[i + k]["code"] == ord(label[k]) for k in range(4))):
                    break
                i += 1
            if i >= len(glyphs):
                sys.exit(f"{doc.name}: label {label} not found in the PDF")
            anchor = glyphs[i]
            i += 4
            body = []
            while i < len(glyphs):
                g = glyphs[i]
                nxt = f"{fi + 1:04d}"
                if fi + 1 < len(doc.formulas) and base_font(g["font"]).startswith("CMR10") and all(
                        i + k < len(glyphs) and base_font(glyphs[i + k]["font"]).startswith("CMR10")
                        and glyphs[i + k]["code"] == ord(nxt[k]) for k in range(4)):
                    break
                body.append(g)
                i += 1
            found = "-"
            if expect is not None:
                wanted = {(gms.SYMBOL_FONTS[f][2], s) for f, s in expect}
                hit = any((prefix_of(g["font"]), g["code"]) in wanted for g in body)
                found = "slot-ok" if hit else "slot-MISSING"
                if not hit:
                    mismatches.append((label, name, style, sorted(wanted),
                                       [(g["font"], g["code"]) for g in body]))
            out.append(f"formula {label} {anchor['page']} {category} {name} {style} {found} "
                       f"{anchor['x']:.4f} {anchor['y_top']:.4f} {tex}")
            # A composite (or an alias of one) may be painted by the engine as
            # one character standing for the whole (`\notin` for `\in` + `/`),
            # so its own Unicode is accepted for every glyph of the formula.
            own = gms.MANUAL.get(name, []) if category in ("composite", "alias") else []
            own = ["".join(chr(int(x, 16)) for x in c.split()) for c in own if c]
            for g in body:
                cands = texts.get((prefix_of(g["font"]), g["code"]), []) + own
                font = base_font(g["font"])
                spelled = "|".join("".join(f"U+{ord(c):04X}" for c in t) for t in cands)
                out.append(f"g {font} {g['code']} {g['size']:.4f} {g['x']:.4f} {g['y_top']:.4f} {spelled or '-'}")
            n += 1
        open(os.path.join(HERE, "expected", doc.name + ".txt"), "w").write("\n".join(out) + "\n")
        print(f"{doc.name}: {n} formulas, {len(glyphs)} glyphs, {len(mismatches)} declaration/pdfTeX slot mismatches")
        for m in mismatches:
            print("   ", m)
        total_mismatch += len(mismatches)
    shutil.rmtree(scratch)
    if total_mismatch:
        sys.exit(1)


if __name__ == "__main__":
    main()
