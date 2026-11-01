#!/usr/bin/env python3
"""Regenerate the pdfLaTeX oracle for the compiler's text built-ins.

Oracle only: needs a TeX Live pdflatex (verified with pdfTeX
3.141592653-2.6-1.40.29, TeX Live 2026). Cargo tests never run TeX; they read
`expected.json` and the TFMs committed next to it.

Each fixture is set as `\\setbox2\\hbox{<input>}` and recorded through
`\\showbox2`: every node with its exact dimensions (kerns, box shifts, rules,
glue, characters with their NFSS font identifiers). The generator also
records the TFM behind each font identifier (`\\fontname`), `\\textwidth` and
`\\linewidth`, the math fonts in force (for `\\LaTeXe`'s subscript) and any
TeX/LaTeX error. Glyph positions follow from the node list plus the TFM
character widths, so `tests/text_builtins_oracle.rs` compares to the scaled
point, which is stricter than a 0.1pt position tolerance.

Symbol fixtures additionally set the character the kernel's `*.dfu` files
declare for the command (`\\DeclareUnicodeCharacter`, inverted the way
`flashtex_tex_text_encoding::generated::UNICODE_DECLARATIONS` is), or
`\\symbol{n}` for the ASCII symbols, so the test can check that the
compiler's chosen character is the one pdfLaTeX maps back to the command and
typesets to the same nodes.

Usage: python3 crates/compiler/tests/oracle/text_builtins/generate.py
"""
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
COMPILER = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
OUT_JSON = os.path.join(HERE, "expected.json")
TFM_DIR = os.path.join(HERE, "tfm")

VARIANTS = {
    # The render pipeline's faces: Latin Modern in T1 (`ec-lm*`).
    "t1-lm": "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}",
    # pdfLaTeX's default: Computer Modern in OT1.
    "ot1-cm": "",
}

ASCII_SYMBOLS = {
    "textbackslash": 92,
    "textasciitilde": 126,
    "textasciicircum": 94,
    "textunderscore": 95,
    "textbar": 124,
    "textless": 60,
    "textgreater": 62,
    "textbraceleft": 123,
    "textbraceright": 125,
}


def logo(id_, variant, size, input_, logo_name, math_prefix=""):
    return {"id": id_, "variant": variant, "size": size, "kind": "logo", "input": input_,
            "logo": logo_name, "math_prefix": math_prefix}


def rule(id_, variant, size, raise_, width, height, math=False):
    arg = ("[%s]" % raise_ if raise_ is not None else "") + "{%s}{%s}" % (width, height)
    input_ = ("$\\rule%s$" if math else "\\rule%s") % arg
    return {"id": id_, "variant": variant, "size": size, "kind": "rule", "input": input_,
            "rule": {"raise": raise_ or "0pt", "width": width, "height": height, "math": math}}


def spacing(id_, variant, size, input_, command, kind):
    return {"id": id_, "variant": variant, "size": size, "kind": kind, "input": input_,
            "command": command}


FIXTURES = [
    logo("tex-10", "t1-lm", "10pt", "\\TeX", "TeX"),
    logo("latex-10", "t1-lm", "10pt", "\\LaTeX", "LaTeX"),
    logo("latexe-10", "t1-lm", "10pt", "\\LaTeXe", "LaTeXe"),
    logo("tex-11", "t1-lm", "11pt", "\\TeX", "TeX"),
    logo("latex-11", "t1-lm", "11pt", "\\LaTeX", "LaTeX"),
    logo("tex-12", "t1-lm", "12pt", "\\TeX", "TeX"),
    logo("latex-12", "t1-lm", "12pt", "\\LaTeX", "LaTeX"),
    logo("latexe-12", "t1-lm", "12pt", "\\LaTeXe", "LaTeXe"),
    logo("latex-bf-10", "t1-lm", "10pt", "\\textbf{\\LaTeX}", "LaTeX"),
    logo("latexe-bf-10", "t1-lm", "10pt", "\\textbf{\\LaTeXe}", "LaTeXe", "\\bfseries\\boldmath"),
    logo("latex-sf-10", "t1-lm", "10pt", "\\textsf{\\LaTeX}", "LaTeX"),
    logo("tex-it-10", "t1-lm", "10pt", "\\textit{\\TeX}", "TeX"),
    logo("latex-large-10", "t1-lm", "10pt", "\\Large\\LaTeX", "LaTeX", "\\Large"),
    logo("latex-small-12", "t1-lm", "12pt", "\\small\\LaTeX", "LaTeX", "\\small"),
    logo("tex-ot1-10", "ot1-cm", "10pt", "\\TeX", "TeX"),
    logo("latex-ot1-10", "ot1-cm", "10pt", "\\LaTeX", "LaTeX"),
    logo("latexe-ot1-10", "ot1-cm", "10pt", "\\LaTeXe", "LaTeXe"),
    logo("latex-ot1-12", "ot1-cm", "12pt", "\\LaTeX", "LaTeX"),
    rule("rule-basic", "t1-lm", "10pt", None, "2pt", "3pt"),
    rule("rule-lowered", "t1-lm", "10pt", "-1pt", "2pt", "3pt"),
    rule("rule-raised", "t1-lm", "10pt", "1.5pt", "10pt", "0.4pt"),
    rule("rule-strut", "t1-lm", "10pt", None, "0pt", "2ex"),
    rule("rule-em", "t1-lm", "10pt", None, "1em", ".5ex"),
    rule("rule-textwidth-cv", "t1-lm", "11pt", None, "\\textwidth", "0.6pt"),
    rule("rule-half-linewidth", "t1-lm", "12pt", None, ".5\\linewidth", "1pt"),
    rule("rule-negative-width", "t1-lm", "10pt", None, "-1pt", "1pt"),
    rule("rule-units", "t1-lm", "10pt", "-.1in", "1cm", "2mm"),
    rule("rule-math", "t1-lm", "10pt", None, "1pt", "2pt", math=True),
    rule("rule-ot1-em", "ot1-cm", "12pt", "-.5ex", "2em", "1ex"),
    spacing("kern-thin", "t1-lm", "10pt", "a\\,b", ",", "kern"),
    spacing("kern-negthin", "t1-lm", "10pt", "a\\!b", "!", "kern"),
    spacing("kern-med", "t1-lm", "10pt", "a\\:b", ":", "kern"),
    spacing("kern-med-gt", "t1-lm", "10pt", "a\\>b", ">", "kern"),
    spacing("kern-thick", "t1-lm", "10pt", "a\\;b", ";", "kern"),
    spacing("kern-thinspace", "t1-lm", "12pt", "a\\thinspace b", "thinspace", "kern"),
    spacing("kern-negthinspace", "t1-lm", "10pt", "a\\negthinspace b", "negthinspace", "kern"),
    spacing("kern-negmedspace", "t1-lm", "10pt", "a\\negmedspace b", "negmedspace", "kern"),
    spacing("kern-negthickspace", "t1-lm", "10pt", "a\\negthickspace b", "negthickspace", "kern"),
    spacing("kern-enspace", "t1-lm", "11pt", "a\\enspace b", "enspace", "kern"),
    spacing("kern-thin-ot1", "ot1-cm", "10pt", "25\\,\\%", ",", "kern"),
    spacing("glue-quad", "t1-lm", "10pt", "a\\quad b", "quad", "glue"),
    spacing("glue-qquad", "t1-lm", "10pt", "a\\qquad b", "qquad", "glue"),
    spacing("glue-enskip", "t1-lm", "10pt", "a\\enskip b", "enskip", "glue"),
    spacing("glue-control-space", "t1-lm", "10pt", "a.\\ b", " ", "glue"),
    spacing("glue-tie", "t1-lm", "10pt", "a~b", "~", "glue"),
]


def symbol_table():
    """TEXT_SYMBOLS from the compiler source (name, text command)."""
    src = open(os.path.join(COMPILER, "src", "text_builtins.rs"), encoding="utf-8").read()
    body = src[src.index("pub const TEXT_SYMBOLS"):]
    body = body[:body.index("];")]
    return re.findall(r'\("(\w+)", "\\\\(\w+)"\)', body)


def kpsewhich(name):
    r = subprocess.run(["kpsewhich", name], capture_output=True, text=True)
    return r.stdout.strip() or None


def dfu_inverse():
    """expansion -> smallest code point declaring it (last declaration wins per code point)."""
    decl = {}
    pattern = re.compile(r"^\\DeclareUnicodeCharacter\{([0-9A-Fa-f]+)\}\{(.*)\}\s*(%.*)?$")
    for name in ["omsenc.dfu", "ot1enc.dfu", "t1enc.dfu", "ts1enc.dfu", "utf8.def"]:
        path = kpsewhich(name)
        for line in open(path, encoding="latin-1"):
            m = pattern.match(line.strip())
            if m:
                decl[int(m.group(1), 16)] = m.group(2).strip()
    inverse = {}
    for cp in sorted(decl):
        inverse.setdefault(decl[cp], cp)
    return inverse


def symbol_fixtures():
    inverse = dfu_inverse()
    out = []
    for variant in VARIANTS:
        for name, command in symbol_table():
            key = {"aa": "\\r a", "AA": "\\r A"}.get(name, "\\" + command)
            if name in ASCII_SYMBOLS:
                char_input, cp = "\\symbol{%d}" % ASCII_SYMBOLS[name], ASCII_SYMBOLS[name]
            else:
                cp = inverse.get(key)
                char_input = chr(cp) if cp is not None else None
            out.append({"id": "sym-%s-%s" % (variant, name), "variant": variant, "size": "10pt",
                        "kind": "symbol", "input": "\\" + name,
                        "symbol": {"name": name, "command": "\\" + command,
                                   "code_point": cp, "char_input": char_input}})
    return out


def tex_decimal_to_sp(text):
    """TeX's round_decimals (tex.web section 102) plus the integer part."""
    neg = text.startswith("-")
    text = text.lstrip("-").rstrip("pt")
    ip, _, fp = text.partition(".")
    a = 0
    for d in reversed(fp[:17]):
        a = (a + int(d) * 0o400000) // 10
    sp = int(ip or "0") * 65536 + (a + 1) // 2
    return -sp if neg else sp


def document(variant, size, fixtures, font_ids):
    lines = [
        "\\documentclass[%s]{article}" % size,
        "\\usepackage[utf8]{inputenc}",
        VARIANTS[variant],
        "\\pagestyle{empty}",
        "\\showboxdepth=1000 \\showboxbreadth=1000000",
        "\\begin{document}",
    ]
    for fx in fixtures:
        fid = fx["id"]
        lines.append("\\typeout{FTX-BEGIN %s}" % fid)
        lines.append("\\typeout{FTX-TEXTFONT %s \\fontname\\font}" % fid)
        lines.append("\\setbox2\\hbox{%s}\\showbox2" % fx["input"])
        if fx["kind"] == "symbol" and fx["symbol"]["char_input"] is not None:
            lines.append("\\typeout{FTX-CHAR %s}" % fid)
            lines.append("\\setbox2\\hbox{%s}\\showbox2" % fx["symbol"]["char_input"])
        lines.append("\\typeout{FTX-END %s}" % fid)
        lines.append("\\typeout{FTX-DIMS %s \\the\\textwidth\\space\\the\\linewidth}" % fid)
        if fx["kind"] == "logo":
            lines.append(
                "\\sbox4{%s$\\typeout{FTX-MATH %s \\fontname\\textfont1|\\fontname\\textfont2|\\fontname\\scriptfont2}$}"
                % (fx.get("math_prefix", ""), fid))
    for fontid in sorted(font_ids):
        lines.append(
            "\\expandafter\\ifx\\csname %s\\endcsname\\relax\\else"
            "\\typeout{FTX-FONT %s=\\expandafter\\fontname\\csname %s\\endcsname}\\fi" % (fontid, fontid, fontid))
    lines.append("\\end{document}")
    return "\n".join(lines) + "\n"


def run_tex(tex, workdir):
    path = os.path.join(workdir, "oracle.tex")
    with open(path, "w", encoding="utf-8") as f:
        f.write(tex)
    env = dict(os.environ, max_print_line="100000", error_line="254", half_error_line="238")
    subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error=false", "oracle.tex"],
                   cwd=workdir, env=env, capture_output=True, text=True)
    return open(os.path.join(workdir, "oracle.log"), encoding="latin-1").read()


NODE_BOX = re.compile(r"^\\([hv])box\((-?[\d.]+)\+(-?[\d.]+)\)x(-?[\d.]+)(.*)$")
NODE_RULE = re.compile(r"^\\rule\((-?[\d.*]+)\+(-?[\d.*]+)\)x(-?[\d.*]+)$")
# `\kern 1.0` (with a space) is an explicit kern; `\kern1.0` a font (TFM
# lig/kern program) kern; `\kern 1.0 (for accent)` an accent kern (§191).
NODE_KERN = re.compile(r"^\\kern( ?)(-?[\d.]+)( \(for accent\))?$")
NODE_GLUE = re.compile(r"^\\glue(\(\\\w+\))? (-?[\d.]+)(.*)$")
NODE_PENALTY = re.compile(r"^\\penalty (-?\d+)$")
NODE_MATH = re.compile(r"^\\math(on|off)")
NODE_CHAR = re.compile(r"^\\([A-Za-z0-9]+/[^ ]+) (.+)$")


def char_code(text):
    token = text.split(" (ligature")[0]
    if token.startswith("^^") and len(token) >= 4 and re.fullmatch(r"[0-9a-f]{2}", token[2:4]):
        return int(token[2:4], 16)
    if token.startswith("^^") and len(token) >= 3:
        c = ord(token[2])
        return c + 64 if c < 64 else c - 64
    return ord(token[0])


def parse_node(text):
    m = NODE_BOX.match(text)
    if m:
        rest = m.group(5)
        shift = re.search(r"shifted (-?[\d.]+)", rest)
        return {"kind": m.group(1) + "box", "height_sp": tex_decimal_to_sp(m.group(2)),
                "depth_sp": tex_decimal_to_sp(m.group(3)), "width_sp": tex_decimal_to_sp(m.group(4)),
                "shift_sp": tex_decimal_to_sp(shift.group(1)) if shift else 0,
                "glue_set": "glue set" in rest, "children": []}
    m = NODE_RULE.match(text)
    if m:
        dim = lambda s: None if s == "*" else tex_decimal_to_sp(s)
        return {"kind": "rule", "height_sp": dim(m.group(1)), "depth_sp": dim(m.group(2)),
                "width_sp": dim(m.group(3))}
    m = NODE_KERN.match(text)
    if m:
        return {"kind": "kern", "width_sp": tex_decimal_to_sp(m.group(2)),
                "explicit": m.group(1) == " " and not m.group(3), "accent": bool(m.group(3))}
    m = NODE_GLUE.match(text)
    if m:
        return {"kind": "glue", "width_sp": tex_decimal_to_sp(m.group(2)), "spec": text}
    m = NODE_PENALTY.match(text)
    if m:
        return {"kind": "penalty", "value": int(m.group(1))}
    m = NODE_MATH.match(text)
    if m:
        return {"kind": "math" + m.group(1)}
    m = NODE_CHAR.match(text)
    if m:
        return {"kind": "char", "font_id": m.group(1), "code": char_code(m.group(2))}
    return {"kind": "other", "text": text}


def parse_showboxes(block):
    """Every `> \\box2=` tree in a log block, in order."""
    trees = []
    lines = block.splitlines()
    i = 0
    while i < len(lines):
        if lines[i].startswith("> \\box2="):
            stack = []
            i += 1
            while i < len(lines) and lines[i].strip() and not lines[i].startswith("! OK"):
                raw = lines[i]
                depth = len(raw) - len(raw.lstrip("."))
                node = parse_node(raw[depth:])
                if depth == 0:
                    trees.append(node)
                    stack = [node]
                else:
                    stack = stack[:depth]
                    stack[-1]["children"].append(node)
                    stack.append(node)
                i += 1
        i += 1
    return trees


def fontname(text):
    m = re.match(r"^(\S+?)(?: at (-?[\d.]+)pt)?$", text.strip())
    return {"tfm": m.group(1), "at_sp": tex_decimal_to_sp(m.group(2)) if m.group(2) else None}


def collect_font_ids(node, out):
    if node.get("kind") == "char":
        out.add(node["font_id"])
    for child in node.get("children", []):
        collect_font_ids(child, out)


def run_group(variant, size, fixtures, workdir):
    log = run_tex(document(variant, size, fixtures, set()), workdir)
    font_ids = set()
    for tree in parse_showboxes(log):
        collect_font_ids(tree, font_ids)
    log = run_tex(document(variant, size, fixtures, font_ids), workdir)
    fonts = {}
    for m in re.finditer(r"^FTX-FONT (\S+)=(.*)$", log, re.M):
        fonts[m.group(1)] = fontname(m.group(2))
    results = []
    for fx in fixtures:
        fid = fx["id"]
        begin = log.index("FTX-BEGIN %s\n" % fid)
        end = log.index("FTX-END %s\n" % fid, begin)
        block = log[begin:end]
        trees = parse_showboxes(block)
        errors = [line[2:] for line in block.splitlines()
                  if line.startswith("! ") and not line.startswith("! OK")]
        dims = re.search(r"^FTX-DIMS %s (\S+) (\S+)$" % re.escape(fid), log, re.M)
        rec = dict(fx)
        rec["box"] = trees[0] if trees else None
        if fx["kind"] == "symbol":
            rec["char_box"] = trees[1] if len(trees) > 1 else None
        rec["errors"] = errors
        rec["textwidth_sp"] = tex_decimal_to_sp(dims.group(1))
        rec["linewidth_sp"] = tex_decimal_to_sp(dims.group(2))
        text_font = re.search(r"^FTX-TEXTFONT %s (.*)$" % re.escape(fid), log, re.M)
        rec["text_font"] = fontname(text_font.group(1))
        used = set()
        for tree in trees:
            collect_font_ids(tree, used)
        rec["fonts"] = {fid_: fonts[fid_] for fid_ in sorted(used) if fid_ in fonts}
        if fx["kind"] == "logo":
            math = re.search(r"^FTX-MATH %s (.*)\|(.*)\|(.*)$" % re.escape(fid), log, re.M)
            rec["math_fonts"] = {"textfont1": fontname(math.group(1)),
                                 "textfont2": fontname(math.group(2)),
                                 "scriptfont2": fontname(math.group(3))}
        results.append(rec)
    return results


def main():
    version = subprocess.run(["pdflatex", "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    fixtures = FIXTURES + symbol_fixtures()
    groups = {}
    for fx in fixtures:
        groups.setdefault((fx["variant"], fx["size"]), []).append(fx)
    results = []
    with tempfile.TemporaryDirectory() as workdir:
        for (variant, size), group in sorted(groups.items()):
            results.extend(run_group(variant, size, group, workdir))
    order = {fx["id"]: i for i, fx in enumerate(fixtures)}
    results.sort(key=lambda r: order[r["id"]])

    os.makedirs(TFM_DIR, exist_ok=True)
    manifest = {}
    names = set()
    for r in results:
        names.update(f["tfm"] for f in r["fonts"].values())
        names.add(r["text_font"]["tfm"])
        names.update(f["tfm"] for f in r.get("math_fonts", {}).values())
    for name in sorted(names):
        src = kpsewhich(name + ".tfm")
        if src is None:
            sys.exit("kpsewhich cannot find %s.tfm" % name)
        dst = os.path.join(TFM_DIR, name + ".tfm")
        shutil.copyfile(src, dst)
        data = open(dst, "rb").read()
        manifest[name + ".tfm"] = {"sha256": hashlib.sha256(data).hexdigest(),
                                   "source": "TL2026:" + src.split("texmf-dist/", 1)[-1]}
    with open(os.path.join(TFM_DIR, "MANIFEST.json"), "w") as f:
        json.dump(manifest, f, indent=1, sort_keys=True)
        f.write("\n")
    with open(OUT_JSON, "w", encoding="utf-8") as f:
        json.dump({"tex": version, "generator": "crates/compiler/tests/oracle/text_builtins/generate.py",
                   "fixtures": results}, f, indent=1, sort_keys=True, ensure_ascii=False)
        f.write("\n")
    errors = sum(1 for r in results if r["errors"])
    print("%d fixtures (%d with TeX errors), %d TFMs -> %s" % (len(results), errors, len(manifest), OUT_JSON))


if __name__ == "__main__":
    main()
