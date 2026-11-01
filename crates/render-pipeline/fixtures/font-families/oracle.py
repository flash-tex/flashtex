#!/usr/bin/env python3
"""Font-family oracle (TEST ONLY; pdflatex never runs in the product path).

For every `NN-*.tex` fixture, runs MacTeX `pdflatex` on a copy with
`\\pdfcompresslevel=0 \\pdfobjcompresslevel=0` (output encoding only, no
layout change), reads the PDF's content stream with a small PDF parser
(fonts' /Widths and /ToUnicode, TJ/Tj/Tf/Td/Tm, cm) and writes
`reference/NN-*.json`:

  segments: page, text, font (BaseFont without subset tag), size (bp),
            x and baseline in bp (top-left origin) of every maximal run of
            glyphs in one font with no interword gap
  warnings: every "LaTeX Font Warning: Font shape ..." of the log, one line

`tests/font_families_oracle.rs` compares the pipeline's display list with
these committed files, so cargo never needs TeX.
"""
import json, os, re, shutil, subprocess, sys, tempfile, zlib

HERE = os.path.dirname(os.path.abspath(__file__))
PDFLATEX = shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex"


def objects(pdf):
    out = {}
    for m in re.finditer(rb"(\d+) 0 obj", pdf):
        num = int(m.group(1))
        start = m.end()
        end = pdf.find(b"endobj", start)
        body = pdf[start:end]
        s = body.find(b"stream")
        if s >= 0 and body[:s].rstrip().endswith(b">>"):
            dict_part = body[:s]
            data_start = s + len(b"stream")
            if body[data_start:data_start + 2] == b"\r\n":
                data_start += 2
            elif body[data_start:data_start + 1] == b"\n":
                data_start += 1
            length = re.search(rb"/Length (\d+)(?! 0 R)", dict_part)
            if length:
                data = body[data_start:data_start + int(length.group(1))]
            else:
                data = body[data_start:body.rfind(b"endstream")]
            if b"/FlateDecode" in dict_part:
                data = zlib.decompress(data)
            out[num] = (dict_part, data)
        else:
            out[num] = (body, None)
    return out


def ref(d, key):
    m = re.search(rb"/" + key + rb"\s+(\d+) 0 R", d)
    return int(m.group(1)) if m else None


def resolve_dict(objs, d, key):
    """The dictionary text of /key: inline << >> or an indirect object."""
    r = ref(d, key)
    if r is not None:
        return objs[r][0]
    m = re.search(rb"/" + key + rb"\s*<<", d)
    if not m:
        return b""
    depth, i = 0, m.end() - 2
    while i < len(d):
        if d[i:i + 2] == b"<<":
            depth += 1
            i += 2
        elif d[i:i + 2] == b">>":
            depth -= 1
            i += 2
            if depth == 0:
                return d[m.end():i - 2]
        else:
            i += 1
    return b""


def tounicode(data):
    cmap = {}
    for block in re.findall(rb"beginbfchar(.*?)endbfchar", data, re.S):
        for a, b in re.findall(rb"<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>", block):
            cmap[int(a, 16)] = bytes.fromhex(b.decode()).decode("utf-16-be")
    for block in re.findall(rb"beginbfrange(.*?)endbfrange", data, re.S):
        for a, b, rest in re.findall(rb"<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>\s*(<[0-9A-Fa-f]+>|\[[^\]]*\])", block):
            lo, hi = int(a, 16), int(b, 16)
            if rest.startswith(b"["):
                for k, h in enumerate(re.findall(rb"<([0-9A-Fa-f]+)>", rest)):
                    cmap[lo + k] = bytes.fromhex(h.decode()).decode("utf-16-be")
            else:
                base = bytes.fromhex(rest[1:-1].decode()).decode("utf-16-be")
                for k in range(hi - lo + 1):
                    cmap[lo + k] = base[:-1] + chr(ord(base[-1]) + k)
    return cmap


def fonts_of(objs, page_dict):
    res = resolve_dict(objs, page_dict, b"Resources")
    fdict = resolve_dict(objs, res, b"Font")
    fonts = {}
    for name, num in re.findall(rb"/([A-Za-z0-9]+)\s+(\d+) 0 R", fdict):
        d = objs[int(num)][0]
        base = re.search(rb"/BaseFont\s*/([^\s/>]+)", d).group(1).decode()
        base = base.split("+", 1)[-1]
        first = int(re.search(rb"/FirstChar\s+(\d+)", d).group(1))
        wref = ref(d, b"Widths")
        warr = objs[wref][0] if wref is not None else re.search(rb"/Widths\s*(\[[^\]]*\])", d).group(1)
        widths = [float(x) for x in re.findall(rb"-?[\d.]+", warr[warr.find(b"["):warr.find(b"]")])]
        tu = ref(d, b"ToUnicode")
        cmap = tounicode(objs[tu][1]) if tu is not None else {}
        fonts[name.decode()] = (base, first, widths, cmap)
    return fonts


TOKEN = re.compile(rb"\s*(?:(\()|(\[)|(\])|(/[^\s/\[\]()<>]+)|(<[0-9A-Fa-f\s]*>)|(-?\d*\.?\d+)|([A-Za-z'\"*]+))")


def read_string(data, i):
    depth, out = 1, bytearray()
    while i < len(data):
        c = data[i]
        if c == 0x5C:  # backslash
            n = data[i + 1]
            if n in b"01234567":
                j = i + 1
                while j < i + 4 and data[j] in b"01234567":
                    j += 1
                out.append(int(data[i + 1:j], 8) & 0xFF)
                i = j
                continue
            out.append({ord("n"): 10, ord("r"): 13, ord("t"): 9, ord("b"): 8, ord("f"): 12}.get(n, n))
            i += 2
            continue
        if c == 0x28:
            depth += 1
        elif c == 0x29:
            depth -= 1
            if depth == 0:
                return bytes(out), i + 1
        out.append(c)
        i += 1
    return bytes(out), i


def glyphs(content, fonts, page_h):
    """(font, size, char, x, y_top, advance) for every glyph, plus None at
    every positioning operator (Td/Tm/BT) as a segment break."""
    stack, operands = [], []
    ctm = [1, 0, 0, 1, 0, 0]
    tm = [0, 0]
    tlm = [0, 0]
    font, size = None, 0
    i = 0
    out = []
    while i < len(content):
        m = TOKEN.match(content, i)
        if not m:
            i += 1
            continue
        i = m.end()
        if m.group(1):
            s, i = read_string(content, i)
            operands.append(s)
        elif m.group(2):
            operands.append("[")
        elif m.group(3):
            arr = []
            while operands and operands[-1] != "[":
                arr.append(operands.pop())
            if operands:
                operands.pop()
            operands.append(list(reversed(arr)))
        elif m.group(4):
            operands.append(m.group(4)[1:].decode())
        elif m.group(5):
            operands.append(bytes.fromhex(re.sub(rb"\s", b"", m.group(5)[1:-1]).decode()))
        elif m.group(6):
            operands.append(float(m.group(6)))
        else:
            op = m.group(7).decode()
            if op == "q":
                stack.append(list(ctm))
            elif op == "Q" and stack:
                ctm = stack.pop()
            elif op == "cm" and len(operands) >= 6:
                a, b, c, d, e, f = operands[-6:]
                ctm = [a * ctm[0] + b * ctm[2], a * ctm[1] + b * ctm[3], c * ctm[0] + d * ctm[2],
                       c * ctm[1] + d * ctm[3], e * ctm[0] + f * ctm[2] + ctm[4], e * ctm[1] + f * ctm[3] + ctm[5]]
            elif op == "BT":
                tm, tlm = [0, 0], [0, 0]
                out.append(None)
            elif op in ("Td", "TD") and len(operands) >= 2:
                tlm = [tlm[0] + operands[-2], tlm[1] + operands[-1]]
                tm = list(tlm)
                out.append(None)
            elif op == "Tm" and len(operands) >= 6:
                tlm = [operands[-2], operands[-1]]
                tm = list(tlm)
                out.append(None)
            elif op == "Tf" and len(operands) >= 2:
                font, size = operands[-2], operands[-1]
            elif op in ("TJ", "Tj") and operands:
                items = operands[-1] if op == "TJ" else [operands[-1]]
                base, first, widths, cmap = fonts[font]
                for it in items:
                    if isinstance(it, float):
                        tm[0] -= it / 1000 * size
                        out.append(("kern", -it / 1000 * size))
                        continue
                    for code in it:
                        w = widths[code - first] if 0 <= code - first < len(widths) else 0
                        x = tm[0] * ctm[0] + ctm[4]
                        y = tm[1] * ctm[3] + ctm[5]
                        out.append((base, size, cmap.get(code, chr(code)), x, page_h - y, w / 1000 * size))
                        tm[0] += w / 1000 * size
            operands = []
    return out


def segments(glyph_list, page):
    segs = []
    cur = None
    gap = 0.0
    for g in glyph_list:
        if g is None:
            cur = None
            continue
        if g[0] == "kern":
            gap += g[1]
            if cur is not None and gap > 0.15 * cur["size"]:
                cur = None
            continue
        base, size, ch, x, y, adv = g
        if cur is None or cur["font"] != base or abs(cur["size"] - size) > 1e-6 or abs(cur["end"] - x) > 0.15 * size or abs(cur["baseline"] - y) > 0.01:
            cur = {"page": page, "text": "", "font": base, "size": size, "x": x, "baseline": y, "end": x}
            segs.append(cur)
        cur["text"] += ch
        cur["end"] = x + adv
        gap = 0.0
    for s in segs:
        s["x"] = round(s["x"], 3)
        s["baseline"] = round(s["baseline"], 3)
        s["size"] = round(s["size"], 4)
        del s["end"]
    return [s for s in segs if s["text"].strip()]


def pages_of(objs):
    root = next(d for d, _ in objs.values() if re.search(rb"/Type\s*/Catalog", d))
    order = []

    def walk(num):
        d = objs[num][0]
        if re.search(rb"/Type\s*/Pages", d):
            kids = re.search(rb"/Kids\s*\[([^\]]*)\]", d).group(1)
            for k in re.findall(rb"(\d+) 0 R", kids):
                walk(int(k))
        else:
            order.append(d)

    walk(ref(root, b"Pages"))
    return order


def warnings_of(log):
    out = []
    text = re.sub(r"\n\(Font\)\s+", " ", log)
    for m in re.finditer(r"LaTeX Font Warning: (Font shape .*?)(?: on input line \d+)?\.\n", text):
        w = re.sub(r"\s+", " ", m.group(1)).strip()
        w = w.replace("' undefined using", "' undefined, using").replace(" not available Font shape", " not available, Font shape")
        if w not in out:
            out.append(w)
    return out


def run(name):
    tex = open(os.path.join(HERE, name + ".tex")).read()
    with tempfile.TemporaryDirectory() as tmp:
        open(os.path.join(tmp, "doc.tex"), "w").write("\\pdfcompresslevel=0 \\pdfobjcompresslevel=0\n" + tex)
        r = subprocess.run([PDFLATEX, "-interaction=batchmode", "-halt-on-error", "doc.tex"], cwd=tmp, capture_output=True)
        if r.returncode != 0:
            sys.exit(f"{name}: pdflatex failed\n" + open(os.path.join(tmp, "doc.log"), errors="replace").read()[-3000:])
        pdf = open(os.path.join(tmp, "doc.pdf"), "rb").read()
        log = open(os.path.join(tmp, "doc.log"), errors="replace").read()
    objs = objects(pdf)
    segs = []
    for n, page in enumerate(pages_of(objs), 1):
        mb = [float(v) for v in re.search(rb"/MediaBox\s*\[([^\]]*)\]", page).group(1).split()]
        fonts = fonts_of(objs, page)
        cref = re.search(rb"/Contents\s+(\d+) 0 R", page)
        content = objs[int(cref.group(1))][1]
        segs += segments(glyphs(content, fonts, mb[3]), n)
    out = {
        "fixture": name + ".tex",
        "engine": subprocess.run([PDFLATEX, "--version"], capture_output=True, text=True).stdout.split("\n")[0],
        "pages": len(pages_of(objs)),
        "segments": segs,
        "warnings": warnings_of(log),
    }
    os.makedirs(os.path.join(HERE, "reference"), exist_ok=True)
    json.dump(out, open(os.path.join(HERE, "reference", name + ".json"), "w"), indent=1, ensure_ascii=False)
    return out


if __name__ == "__main__":
    names = sorted(n[:-4] for n in os.listdir(HERE) if re.match(r"\d\d-.*\.tex$", n))
    if len(sys.argv) > 1:
        names = [n for n in names if any(a in n for a in sys.argv[1:])]
    for n in names:
        o = run(n)
        fonts = sorted({s["font"] for s in o["segments"]})
        print(n, "segments", len(o["segments"]), "fonts", fonts, "warnings", len(o["warnings"]))
