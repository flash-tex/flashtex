#!/usr/bin/env python3
r"""The oracle for the three renderer halves of the `9b728e90e`/`\smash`/
mathtools re-pin: `\pmb`, `\smash[t|b]`, and the nineteen extensible arrows.

ORACLE TOOLING ONLY (pdflatex never runs in the product path; cargo never
runs TeX). Three documents, each a list of one-line formulas labelled by a
four-digit number set in `cmr10` before them:

  * `pmb`      -- amsbsy `\pmb` (`amsbsy.sty` 49-57) over an ordinary letter,
                  a Bin, a Rel, a large operator and a two-atom body, in four
                  styles, plus the neighbour spacing `\binrel@` decides and
                  the height the raised middle copy adds;
  * `smash`    -- `\smash`, `\smash[t]`, `\smash[b]` and an option amsmath
                  does not know, over a tall/deep body, in four styles, next
                  to neighbours that reveal the zeroed box (a `\frac`
                  numerator, a scripted smash, `\fbox`-free row stacking);
  * `xarrows`  -- all nineteen `\ext@arrow`s (amsmath's two, mathtools'
                  seventeen), each with no label, an `{above}` only, and both
                  labels, in text, display and script style.

The output format is `fixtures/declared-math/expected/*.txt`'s: per formula
one `formula <label> <page> <category> <name> <style> - <x> <y_top> <tex>`
line and then one `g <font> <code> <size> <x> <y_top> <U+...|...>` line per
glyph pdfTeX placed, in bp from the page's top-left (the display-list-v2
convention). `tests/math_halves_oracle.rs` compares the engine's glyphs with
these, within 0.5bp, without running TeX.

Usage: python3 crates/render-pipeline/fixtures/math-halves/generate.py [--only pmb]
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

PREAMBLE = """\\documentclass{article}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
"""
# Output encoding only (no layout change), for the oracle's PDF reader; not
# in the committed document, which the engine compiles as well.
PDF_ONLY = "\\pdfcompresslevel=0\n\\pdfobjcompresslevel=0\n"

# (name, template) -- `{}` is the construct under test.
STYLES = [
    ("text", "$a{}b$"),
    ("display", "$\\displaystyle a{}b$"),
    ("script", "$x^{{a{}b}}$"),
    ("scriptscript", "$x^{{y^{{a{}b}}}}$"),
]

# `\pmb` bodies: an Ord letter, a Bin, a Rel, a large operator, a Greek
# letter (the amsbsy example) and a two-atom body, so `\binrel@`'s three
# answers and the raised copy's extra height are all exercised.
PMB_BODIES = [
    ("ord", "x"),
    ("bin", "+"),
    ("rel", "="),
    ("greek", "\\alpha"),
    ("op", "\\sum"),
    ("pair", "a+b"),
]

# `\smash` bodies and options. An option amsmath does not know leaves
# `\csname mb@..\endcsname` `\relax`, so the natural box ships with no error.
SMASH_CASES = [
    ("tb", "\\smash{\\int}"),
    ("t", "\\smash[t]{\\int}"),
    ("b", "\\smash[b]{\\int}"),
    ("unknown", "\\smash[x]{\\int}"),
    ("frac", "\\smash{\\frac{a}{b}}"),
    ("scripted", "\\smash{\\int}^{2}"),
    ("paren", "\\smash{\\left(\\frac{a}{b}\\right)}"),
]

# Every `\ext@arrow`. `package` is the one that defines it.
ARROWS = [
    ("xrightarrow", "amsmath"),
    ("xleftarrow", "amsmath"),
    ("xleftrightarrow", "mathtools"),
    ("xmapsto", "mathtools"),
    ("xhookleftarrow", "mathtools"),
    ("xhookrightarrow", "mathtools"),
    ("xLeftarrow", "mathtools"),
    ("xRightarrow", "mathtools"),
    ("xLeftrightarrow", "mathtools"),
    ("xLongleftarrow", "mathtools"),
    ("xLongrightarrow", "mathtools"),
    ("xlongleftarrow", "mathtools"),
    ("xlongrightarrow", "mathtools"),
    ("xleftharpoonup", "mathtools"),
    ("xleftharpoondown", "mathtools"),
    ("xrightharpoonup", "mathtools"),
    ("xrightharpoondown", "mathtools"),
    ("xleftrightharpoons", "mathtools"),
    ("xrightleftharpoons", "mathtools"),
]
# (name, argument spelling) -- no label, an `{above}`, and both labels.
ARROW_LABELS = [
    ("bare", "{}"),
    ("above", "{n}"),
    ("both", "[m]{n}"),
]
ARROW_STYLES = STYLES[:3]


class Doc:
    def __init__(self, name, packages):
        self.name = name
        self.packages = packages
        self.formulas = []  # (category, name, style, tex)

    def add(self, category, name, style, tex):
        self.formulas.append((category, name, style, tex))

    def tex(self):
        out = [PREAMBLE]
        for p in self.packages:
            out.append(f"\\usepackage{{{p}}}\n")
        out.append("\\begin{document}\n")
        for i, (_, _, _, tex) in enumerate(self.formulas):
            out.append(f"{i:04d} {tex}\\par\n")
        out.append("\\end{document}\n")
        return "".join(out)


def docs():
    pmb = Doc("pmb", ["amsmath"])
    for name, body in PMB_BODIES:
        for style, tmpl in STYLES:
            pmb.add("pmb", name, style, tmpl.format(f"\\pmb{{{body}}}"))
    # The spacing `\binrel@` decides, against the same body set plain: a
    # `\pmb{+}` must keep the `\medmuskip` a `+` has, `\pmb{=}` the
    # `\thickmuskip` of `=`, `\pmb{x}` neither.
    for name, body in PMB_BODIES:
        # The trailing space ends a control word (`\alpha b`, not `\alphab`)
        # and is then discarded, math mode ignoring space tokens.
        pmb.add("pmb-plain", name, "text", f"$a{body} b$")

    smash = Doc("smash", ["amsmath"])
    for name, body in SMASH_CASES:
        for style, tmpl in STYLES:
            smash.add("smash", name, style, tmpl.format(body))
    # The unsmashed bodies, so the test can see that only the box changed.
    for name, body in SMASH_CASES:
        plain = body.replace("\\smash[t]", "").replace("\\smash[b]", "")
        plain = plain.replace("\\smash[x]", "").replace("\\smash", "")
        smash.add("smash-plain", name, "text", f"$a{plain}b$")

    xa = Doc("xarrows", ["amsmath", "mathtools"])
    for name, _ in ARROWS:
        for label, arg in ARROW_LABELS:
            for style, tmpl in ARROW_STYLES:
                xa.add("arrow", name, f"{label}-{style}", tmpl.format(f"\\{name}{arg}"))
    return {d.name: d for d in (pmb, smash, xa)}


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
        glyphs, _ = pdftext.page_glyphs(d, page)
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


CHAR_EQUIV = {"-": "\u2212", "*": "\u2217", "'": "\u2032"}


def slot_text(rows):
    """(pdf font prefix, code) -> every text a declaration gives that slot."""
    m = {}

    def add(key, text):
        if not text:
            return
        m.setdefault(key, [])
        if text not in m[key]:
            m[key].append(text)
        if text in CHAR_EQUIV and CHAR_EQUIV[text] not in m[key]:
            m[key].append(CHAR_EQUIV[text])

    for r in rows:
        add((gms.SYMBOL_FONTS[r["font"]][2], r["slot"]), r["text"])
        if r["kind"] in ("delimiter", "radical"):
            add((gms.SYMBOL_FONTS[r["large_font"]][2], r["large_slot"]), r["text"])
    for p in ("CMR", "CMMI", "CMTI", "CMBX", "CMSS", "CMTT"):
        for c in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789":
            add((p, ord(c)), c)
    return m


def main():
    only = None
    if "--only" in sys.argv:
        only = sys.argv[sys.argv.index("--only") + 1]
    _, rows, _ = gms.build()
    texts = slot_text(rows)
    all_docs = docs()
    os.makedirs(os.path.join(HERE, "expected"), exist_ok=True)
    scratch = tempfile.mkdtemp(prefix="math-halves-")
    version = subprocess.run([PDFLATEX, "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    for key in sorted(all_docs):
        if only and key != only:
            continue
        doc = all_docs[key]
        open(os.path.join(HERE, doc.name + ".tex"), "w").write(doc.tex())
        pdf = run_pdflatex(doc, scratch)
        glyphs = pdf_glyphs(pdf)
        out = [f"# GENERATED by fixtures/math-halves/generate.py; {version}; do not edit",
               f"# doc {doc.name} packages {','.join(doc.packages) or '-'}"]
        i = 0
        for fi, (category, name, style, tex) in enumerate(doc.formulas):
            label = f"{fi:04d}"
            while i < len(glyphs):
                if (base_font(glyphs[i]["font"]).startswith("CMR10") and all(
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
            out.append(f"formula {label} {anchor['page']} {category} {name} {style} - "
                       f"{anchor['x']:.4f} {anchor['y_top']:.4f} {tex}")
            for g in body:
                cands = texts.get((prefix_of(g["font"]), g["code"]), [])
                spelled = "|".join("".join(f"U+{ord(c):04X}" for c in t) for t in cands)
                out.append(f"g {base_font(g['font'])} {g['code']} {g['size']:.4f} "
                           f"{g['x']:.4f} {g['y_top']:.4f} {spelled or '-'}")
        open(os.path.join(HERE, "expected", doc.name + ".txt"), "w").write("\n".join(out) + "\n")
        print(f"{doc.name}: {len(doc.formulas)} formulas, {len(glyphs)} glyphs")
    shutil.rmtree(scratch)


if __name__ == "__main__":
    main()
