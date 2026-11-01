#!/usr/bin/env python3
"""Regenerate the KC-105 pdflatex oracle (expected JSON + TFM fixtures).

Oracle only: requires a TeX Live pdflatex (verified with pdfTeX 3.141592653-2.6-1.40.29,
TeX Live 2026).  Cargo tests never run TeX; they read the committed JSON.

Evidence per fixture (``<variant>/<id>``):
  * ``\\showbox`` of ``\\hbox{<input>}`` (max_print_line=100000): top-level interword glue
    in exact scaled points, box dimensions, the node kinds (char/kern/glue/penalty/box).
  * the uncompressed PDF page (``\\pdfcompresslevel=0``): every glyph actually shown,
    its PDF font's /BaseFont, the code, the glyph *name* from /Differences (or the
    Type 1 builtin encoding when pdfTeX writes none), and its position relative to a
    ``\\pdfsavepos`` taken at the box origin (bp -> pt).
  * the TFM name behind each NFSS font identifier (``\\fontname``).
  * TeX/LaTeX errors raised while building the box.

Usage: python3 tools/generate_oracle.py   (from crates/tex-text-encoding)
"""
import json
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)
sys.path.insert(0, HERE)
from fixtures import FIXTURES, VARIANTS  # noqa: E402

OUT_JSON = os.path.join(CRATE, "tests", "oracle", "expected.json")
TFM_DIR = os.path.join(CRATE, "tests", "fixtures", "tfm")

BP_TO_PT = 72.27 / 72.0


def kpsewhich(name):
    r = subprocess.run(["kpsewhich", name], capture_output=True, text=True)
    return r.stdout.strip() or None


def tex_decimal_to_sp(text):
    """TeX's round_decimals (tex.web section 102) + integer part, sign aware."""
    neg = text.startswith("-")
    text = text.lstrip("-")
    if "." in text:
        ip, fp = text.split(".")
    else:
        ip, fp = text, ""
    a = 0
    for d in reversed(fp[:17]):
        a = (a + int(d) * 0o400000) // 10
    sp = int(ip or "0") * 65536 + (a + 1) // 2
    return -sp if neg else sp


def run_variant(variant, preamble, fixtures, fontmap_ids, workdir):
    lines = [
        preamble,
        r"\pdfcompresslevel=0 \pdfobjcompresslevel=0",
        r"\pagestyle{empty}",
        r"\showboxdepth=1000 \showboxbreadth=1000000",
        r"\begin{document}",
    ]
    for fid, src in fixtures:
        lines.append(r"\typeout{KC105-BEGIN %s}" % fid)
        lines.append(r"\setbox0\hbox{%s}" % src)
        lines.append(r"\typeout{KC105-SHOW %s}\showbox0" % fid)
        lines.append(r"\typeout{KC105-END %s}" % fid)
        lines.append(
            r"\noindent\hbox{\pdfsavepos\write-1{KC105-POS %s \the\pdflastxpos,\the\pdflastypos}\copy0}\newpage"
            % fid
        )
    for fontid in sorted(fontmap_ids):
        lines.append(
            r"\expandafter\ifx\csname %s\endcsname\relax\else\typeout{KC105-FONT %s=\expandafter\fontname\csname %s\endcsname}\fi"
            % (fontid, fontid, fontid)
        )
    lines.append(r"\end{document}")
    tex = os.path.join(workdir, variant + ".tex")
    with open(tex, "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines) + "\n")
    env = dict(os.environ, max_print_line="100000", error_line="254", half_error_line="238")
    subprocess.run(
        ["pdflatex", "-interaction=nonstopmode", "-halt-on-error=false", variant + ".tex"],
        cwd=workdir, env=env, capture_output=True,
    )
    with open(os.path.join(workdir, variant + ".log"), "rb") as fh:
        log = fh.read().decode("latin-1")
    with open(os.path.join(workdir, variant + ".pdf"), "rb") as fh:
        pdf = fh.read()
    return log, pdf


FONT_ID_RE = re.compile(r"\\((?:OT1|T1|TS1|OMS|OML|OMX|U)/[^ ]+)")


def decode_tex_char(s):
    """Decode the character printed after a font id in a box display."""
    if s.startswith("^^"):
        if len(s) >= 4 and re.match(r"[0-9a-f]{2}", s[2:4]):
            return int(s[2:4], 16), s[4:]
        c = ord(s[2])
        return (c - 64 if c >= 64 else c + 64), s[3:]
    return ord(s[0]), s[1:]


def parse_showbox(block):
    """Return top-level structure from a \\showbox display."""
    res = {"box": None, "nodes": [], "font_ids": set()}
    for raw in block:
        m = re.match(r"^(\.*)(.*)$", raw)
        depth, body = len(m.group(1)), m.group(2)
        for fm in FONT_ID_RE.finditer(body):
            res["font_ids"].add(fm.group(1))
        if depth == 0:
            bm = re.match(r"\\hbox\((-?[\d.]+)\+(-?[\d.]+)\)x(-?[\d.]+)", body)
            if bm and res["box"] is None:
                res["box"] = {
                    "height_sp": tex_decimal_to_sp(bm.group(1)),
                    "depth_sp": tex_decimal_to_sp(bm.group(2)),
                    "width_sp": tex_decimal_to_sp(bm.group(3)),
                }
            continue
        if depth != 1:
            continue
        gm = re.match(r"\\glue(\([^)]*\))? (-?[\d.]+)(?: plus (-?[\d.]+)(fil+)?)?(?: minus (-?[\d.]+)(fil+)?)?$", body)
        if gm:
            res["nodes"].append({
                "kind": "glue",
                "name": (gm.group(1) or "").strip("()"),
                "width_sp": tex_decimal_to_sp(gm.group(2)),
                "stretch_sp": tex_decimal_to_sp(gm.group(3) or "0"),
                "stretch_order": len(gm.group(4) or "f") - 2 if gm.group(4) else 0,
                "shrink_sp": tex_decimal_to_sp(gm.group(5) or "0"),
                "shrink_order": len(gm.group(6) or "f") - 2 if gm.group(6) else 0,
            })
            continue
        km = re.match(r"\\kern ?(-?[\d.]+)( \(for accent\))?$", body)
        if km:
            res["nodes"].append({"kind": "accent_kern" if km.group(2) else "kern",
                                 "width_sp": tex_decimal_to_sp(km.group(1))})
            continue
        pm = re.match(r"\\penalty (-?\d+)$", body)
        if pm:
            res["nodes"].append({"kind": "penalty", "value": int(pm.group(1))})
            continue
        cm = re.match(r"\\((?:OT1|T1|TS1|OMS|OML|OMX|U)/[^ ]+) (.*)$", body)
        if cm:
            code, rest = decode_tex_char(cm.group(2))
            node = {"kind": "char", "font_id": cm.group(1), "code": code}
            lm = re.match(r" \(ligature (.*)\)$", rest)
            if lm:
                node["ligature"] = True
            res["nodes"].append(node)
            continue
        xm = re.match(r"\\([hv]box|rule)", body)
        if xm:
            res["nodes"].append({"kind": xm.group(1)})
            continue
        if body.startswith("\\mathon") or body.startswith("\\mathoff"):
            res["nodes"].append({"kind": "math"})
            continue
        if body.startswith("\\discretionary"):
            res["nodes"].append({"kind": "disc"})
            continue
        res["nodes"].append({"kind": "other", "text": body})
    return res


def parse_log(log):
    per = {}
    fontmap = {}
    lines = log.split("\n")
    cur = None
    mode = None
    for i, line in enumerate(lines):
        m = re.match(r"^KC105-BEGIN (\S+)", line)
        if m:
            cur = m.group(1)
            per[cur] = {"errors": [], "show": [], "missing": []}
            mode = "build"
            continue
        m = re.match(r"^KC105-SHOW (\S+)", line)
        if m:
            mode = "show"
            continue
        m = re.match(r"^KC105-END (\S+)", line)
        if m:
            mode = None
            continue
        m = re.match(r"^KC105-POS (\S+) (-?\d+),(-?\d+)", line)
        if m and m.group(1) in per:
            per[m.group(1)]["pos"] = (int(m.group(2)), int(m.group(3)))
            continue
        m = re.match(r"^KC105-FONT (\S+)=(\S+)", line)
        if m:
            fontmap[m.group(1)] = m.group(2)
            continue
        if cur is None:
            continue
        if mode == "build":
            if line.startswith("! "):
                msg = line[2:].strip()
                # LaTeX continues long messages on following indented lines.
                j = i + 1
                while j < len(lines) and lines[j].startswith(" ") and not lines[j].startswith("See the"):
                    msg += " " + lines[j].strip()
                    j += 1
                    if msg.endswith("."):
                        break
                # The log is read as latin-1; messages quoting input characters are UTF-8.
                msg = msg.encode("latin-1").decode("utf-8", errors="replace")
                per[cur]["errors"].append(re.sub(r"\s+", " ", msg))
            elif line.startswith("Missing character:"):
                per[cur]["missing"].append(line.strip())
        elif mode == "show":
            if line.startswith("> \\box0="):
                per[cur]["show"] = []
                per[cur]["_in"] = True
                continue
            if per[cur].get("_in"):
                if line.strip() == "" or line.startswith("! OK"):
                    per[cur]["_in"] = False
                else:
                    per[cur]["show"].append(line)
    return per, fontmap


# ---------------------------------------------------------------- PDF parsing
def pdf_objects(pdf):
    objs = {}
    for m in re.finditer(rb"(\d+) 0 obj", pdf):
        start = m.end()
        end = pdf.find(b"endobj", start)
        objs[int(m.group(1))] = pdf[start:end]
    return objs


def ref(body, key):
    m = re.search(rb"/" + key + rb"\s+(\d+) 0 R", body)
    return int(m.group(1)) if m else None


def stream_data(body):
    s = body.find(b"stream")
    e = body.rfind(b"endstream")
    data = body[s + 6:e]
    if data.startswith(b"\r\n"):
        data = data[2:]
    elif data.startswith(b"\n"):
        data = data[1:]
    return data


def page_list(objs, pdf):
    root = int(re.search(rb"/Root (\d+) 0 R", pdf).group(1))
    pages = ref(objs[root], b"Pages")
    out = []

    def walk(n):
        body = objs[n]
        if re.search(rb"/Type\s*/Pages", body):
            kids = re.search(rb"/Kids\s*\[([^\]]*)\]", body).group(1)
            for k in re.findall(rb"(\d+) 0 R", kids):
                walk(int(k))
        else:
            out.append(n)

    walk(pages)
    return out


BUILTIN_CACHE = {}


def builtin_encoding(basefont):
    name = basefont.split("+")[-1].lower()
    if name in BUILTIN_CACHE:
        return BUILTIN_CACHE[name]
    enc = {}
    path = kpsewhich(name + ".pfb")
    if path:
        with open(path, "rb") as fh:
            head = fh.read(20000)
        for m in re.finditer(rb"dup (\d+) /([^\s]+) put", head):
            enc[int(m.group(1))] = m.group(2).decode()
    BUILTIN_CACHE[name] = enc
    return enc


def font_info(objs, fobj):
    body = objs[fobj]
    base = re.search(rb"/BaseFont\s*/([^\s/>]+)", body).group(1).decode()
    first = int(re.search(rb"/FirstChar\s+(\d+)", body).group(1))
    wref = ref(body, b"Widths")
    wtxt = objs[wref] if wref else re.search(rb"/Widths\s*\[([^\]]*)\]", body).group(1)
    widths = [float(x) for x in re.findall(rb"-?[\d.]+", wtxt.replace(b"[", b" ").replace(b"]", b" "))]
    names = {}
    eref = ref(body, b"Encoding")
    if eref:
        d = re.search(rb"/Differences\s*\[([^\]]*)\]", objs[eref]).group(1)
        code = 0
        for tok in re.findall(rb"/[^\s/\[\]]+|\d+", d):
            if tok.startswith(b"/"):
                names[code] = tok[1:].decode()
                code += 1
            else:
                code = int(tok)
    else:
        names = builtin_encoding(base)
    return {"base": base, "first": first, "widths": widths, "names": names}


def pdf_string(s, i):
    """Parse a literal string starting at s[i]=='('; return (bytes, next index)."""
    out = bytearray()
    depth = 0
    i += 1
    while True:
        c = s[i]
        if c == 0x5C:  # backslash
            n = s[i + 1]
            if 0x30 <= n <= 0x37:
                j = i + 1
                digs = b""
                while j < len(s) and len(digs) < 3 and 0x30 <= s[j] <= 0x37:
                    digs += bytes([s[j]])
                    j += 1
                out.append(int(digs, 8))
                i = j
                continue
            out.append({ord("n"): 10, ord("r"): 13, ord("t"): 9, ord("b"): 8, ord("f"): 12}.get(n, n))
            i += 2
            continue
        if c == 0x28:
            depth += 1
        elif c == 0x29:
            if depth == 0:
                return bytes(out), i + 1
            depth -= 1
        out.append(c)
        i += 1


def page_glyphs(objs, page):
    body = objs[page]
    res = ref(body, b"Resources")
    resbody = objs[res] if res else body
    fonts = {}
    fm = re.search(rb"/Font\s*<<(.*?)>>", resbody, re.S)
    if fm:
        for name, num in re.findall(rb"/(F\d+)\s+(\d+) 0 R", fm.group(1)):
            fonts[name.decode()] = font_info(objs, int(num))
    content = stream_data(objs[ref(body, b"Contents")])
    glyphs, rules = [], []
    ctm = [0.0, 0.0]
    stack = []
    tlm = [0.0, 0.0]
    tm = [0.0, 0.0]
    font = None
    size = 0.0
    operands = []
    i = 0
    s = content
    while i < len(s):
        c = s[i]
        if c in b" \t\r\n":
            i += 1
            continue
        if c == 0x28:
            val, i = pdf_string(s, i)
            operands.append(val)
            continue
        if c == 0x5B:
            operands.append("[")
            i += 1
            continue
        if c == 0x5D:
            arr = []
            while operands and operands[-1] != "[":
                arr.append(operands.pop())
            operands.pop()
            operands.append(list(reversed(arr)))
            i += 1
            continue
        if c == 0x2F:
            m = re.match(rb"/[^\s/\[\]()<>]+", s[i:])
            operands.append(m.group(0)[1:].decode())
            i += m.end()
            continue
        m = re.match(rb"-?\d*\.?\d+", s[i:])
        if m and (c in b"-." or 0x30 <= c <= 0x39):
            operands.append(float(m.group(0)))
            i += m.end()
            continue
        m = re.match(rb"[A-Za-z'\"*]+", s[i:])
        if not m:
            i += 1
            continue
        op = m.group(0).decode()
        i += m.end()
        if op == "q":
            stack.append(list(ctm))
        elif op == "Q":
            ctm = stack.pop()
        elif op == "cm":
            a, b, cc, d, e, f = operands[-6:]
            ctm = [ctm[0] + e, ctm[1] + f]
        elif op == "BT":
            tlm = [0.0, 0.0]
            tm = [0.0, 0.0]
        elif op == "Tf":
            font, size = operands[-2], operands[-1]
        elif op == "Td":
            tlm = [tlm[0] + operands[-2], tlm[1] + operands[-1]]
            tm = list(tlm)
        elif op == "Tm":
            tlm = [operands[-2], operands[-1]]
            tm = list(tlm)
        elif op in ("TJ", "Tj"):
            items = operands[-1] if op == "TJ" else [operands[-1]]
            fi = fonts[font]
            for it in items:
                if isinstance(it, float):
                    tm[0] -= it * size / 1000.0
                    continue
                for code in it:
                    glyphs.append({
                        "pdf_font": fi["base"].split("+")[-1],
                        "code": code,
                        "glyph": fi["names"].get(code),
                        "x_bp": ctm[0] + tm[0],
                        "y_bp": ctm[1] + tm[1],
                        "size_bp": size,
                    })
                    w = fi["widths"][code - fi["first"]]
                    tm[0] += w * size / 1000.0
        elif op == "re":
            x, y, w, h = operands[-4:]
            rules.append({"x_bp": ctm[0] + x, "y_bp": ctm[1] + y, "w_bp": w, "h_bp": h})
        operands = [] if op not in ("[",) else operands
    return glyphs, rules


# ---------------------------------------------------------------- main
def main():
    fixture_ids = [f for f, _ in FIXTURES]
    assert len(set(fixture_ids)) == len(fixture_ids), "duplicate fixture ids"
    work = tempfile.mkdtemp(prefix="kc105-oracle-")
    result = {
        "generator": "crates/tex-text-encoding/tools/generate_oracle.py",
        "engine": subprocess.run(["pdflatex", "--version"], capture_output=True, text=True).stdout.split("\n")[0],
        "latex_format": None,
        "units": "sp = TeX scaled points (65536 sp = 1pt); x_pt/y_pt relative to box origin, y up",
        "cases": [],
    }
    tfm_names = set()
    for variant, preamble in VARIANTS.items():
        log, _ = run_variant(variant, preamble, FIXTURES, [], work)
        per, _ = parse_log(log)
        ids = set()
        for fid in per:
            ids |= parse_showbox(per[fid]["show"])["font_ids"]
        log, pdf = run_variant(variant, preamble, FIXTURES, ids, work)
        if result["latex_format"] is None:
            m = re.search(r"LaTeX2e <([^>]+)>", log)
            result["latex_format"] = m.group(1) if m else None
        per, fontmap = parse_log(log)
        objs = pdf_objects(pdf)
        pages = page_list(objs, pdf)
        if len(pages) != len(FIXTURES):
            # An empty box still produces a page (\noindent\hbox{} ships something).
            print("warning: %s pages=%d fixtures=%d" % (variant, len(pages), len(FIXTURES)), file=sys.stderr)
        for idx, (fid, src) in enumerate(FIXTURES):
            info = per.get(fid, {"errors": ["fixture missing from log"], "show": [], "missing": []})
            sb = parse_showbox(info["show"])
            glyphs, rules = page_glyphs(objs, pages[idx]) if idx < len(pages) else ([], [])
            ox, oy = info.get("pos", (0, 0))
            ox_bp, oy_bp = ox / 65536.0 * 72.0 / 72.27, oy / 65536.0 * 72.0 / 72.27
            out_glyphs = []
            for g in glyphs:
                out_glyphs.append({
                    "pdf_font": g["pdf_font"],
                    "code": g["code"],
                    "glyph": g["glyph"],
                    "x_pt": round((g["x_bp"] - ox_bp) * BP_TO_PT, 4),
                    "y_pt": round((g["y_bp"] - oy_bp) * BP_TO_PT, 4),
                })
            out_rules = [{
                "x_pt": round((r["x_bp"] - ox_bp) * BP_TO_PT, 4),
                "y_pt": round((r["y_bp"] - oy_bp) * BP_TO_PT, 4),
                "w_pt": round(r["w_bp"] * BP_TO_PT, 4),
                "h_pt": round(r["h_bp"] * BP_TO_PT, 4),
            } for r in rules]
            chars = [n for n in sb["nodes"] if n["kind"] == "char"]
            for n in sb["nodes"]:
                if n["kind"] == "char":
                    n["tfm"] = fontmap.get(n["font_id"])
            used_fonts = sorted({fontmap.get(f, f) for f in sb["font_ids"]})
            tfm_names |= {fontmap[f] for f in sb["font_ids"] if f in fontmap}
            result["cases"].append({
                "key": "%s/%s" % (variant, fid),
                "variant": variant,
                "id": fid,
                "input": src,
                "errors": info["errors"],
                "missing_chars": info["missing"],
                "box": sb["box"],
                "nodes": sb["nodes"],
                "fonts": used_fonts,
                "glyphs": out_glyphs,
                "rules": out_rules,
                "top_level_chars": len(chars),
            })
    os.makedirs(os.path.dirname(OUT_JSON), exist_ok=True)
    with open(OUT_JSON, "w", encoding="utf-8") as fh:
        json.dump(result, fh, ensure_ascii=False, indent=1, sort_keys=True)
        fh.write("\n")
    os.makedirs(TFM_DIR, exist_ok=True)
    manifest = {}
    for name in sorted(tfm_names):
        path = kpsewhich(name + ".tfm")
        if not path:
            continue
        shutil.copyfile(path, os.path.join(TFM_DIR, name + ".tfm"))
        with open(path, "rb") as fh:
            manifest[name + ".tfm"] = {
                "sha256": hashlib.sha256(fh.read()).hexdigest(),
                "source": path.replace("/usr/local/texlive/2026/", "TL2026:"),
            }
    with open(os.path.join(TFM_DIR, "MANIFEST.json"), "w") as fh:
        json.dump(manifest, fh, indent=1, sort_keys=True)
        fh.write("\n")
    print("cases=%d tfms=%d work=%s" % (len(result["cases"]), len(manifest), work))


if __name__ == "__main__":
    main()
