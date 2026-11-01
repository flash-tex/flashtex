#!/usr/bin/env python3
"""pdfTeX oracle for image XObjects on the exact route (TEST ONLY).

For every case below, pdflatex (MacTeX, never in the product path) places one
`\\includegraphics` on an otherwise empty US Letter page. From its PDF this
script records:

  reference.ctm      the CTM in force at `/ImN Do` (q/Q/cm replayed)
  reference.xobject  Subtype, Width, Height, BitsPerComponent, ColorSpace,
                     Filter, Decode, SMask (same summary), BBox, Matrix
  reference.samples  SHA-256 of the decoded stream (image samples, or the
                     form's content stream), and of the SMask's samples
  reference.page_group  whether pdfTeX gave the page a /Group

and writes `oracle/NN-name.v2.json`, a `display_list` envelope whose single
image item carries the unit-square transform implied by pdfTeX's CTM (rounded
to 1/1000 pt like `flashtex-render`) and the resource fields of proposal §3.
`tests/images.rs` exports each envelope through `flashtex_pdf::v2` and checks
the CTM (0.01 bp), the XObject summary and the sample hashes against
`oracle/expected.json`; cargo never runs TeX.

With `--exact BIN` it also exports every envelope with that
`flashtex-pdf-exact`, rasterises reference and export with Ghostscript at
150 dpi, and records per-case pixel statistics in `oracle/raster.json`.

Usage: python3 make_oracle.py [--exact path/to/flashtex-pdf-exact]
"""
import hashlib, json, os, re, shutil, subprocess, sys, tempfile, zlib

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "oracle")
PDFLATEX = shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex"
GS = shutil.which("gs") or "/opt/homebrew/bin/gs"
TICKS = 1 << 20

CASES = [
    ("01-rgb8", "rgb8.png", ""),
    ("02-rgb8-adam7-width", "rgb8-adam7.png", "width=5cm"),
    ("03-gray8-144dpi", "gray8-144dpi.png", ""),
    ("04-gray2-height", "gray2.png", "height=2cm"),
    ("05-palette4-scale", "palette4.png", "scale=2"),
    ("06-palette8-trns", "palette8-trns.png", "width=4cm"),
    ("07-rgba8", "rgba8.png", "width=6cm"),
    ("08-graya8-scale", "graya8.png", "scale=3"),
    ("09-rgb16", "rgb16.png", "width=4cm"),
    ("10-rgba16", "rgba16.png", "width=4cm"),
    ("11-jpeg-rgb-96dpi", "rgb-96dpi.jpg", ""),
    ("12-jpeg-progressive", "rgb-progressive.jpg", "width=5cm"),
    ("13-jpeg-gray", "gray.jpg", "height=3cm"),
    ("14-jpeg-cmyk", "cmyk.jpg", "width=4cm"),
    ("15-pdf-cropbox", "page-cropbox.pdf", "width=6cm"),
    ("16-pdf-rotate90", "page-rotate90.pdf", "width=4cm"),
    ("17-png-rotated30", "rgb8.png", "width=4cm,angle=30"),
    ("18-pdf-rotated-45", "page-cropbox.pdf", "width=5cm,angle=-45"),
    ("19-jpeg-angle90", "rgb-96dpi.jpg", "angle=90,width=3cm"),
]

TEX = r"""\pdfobjcompresslevel=0
\documentclass{article}
\usepackage{graphicx}
\pagestyle{empty}
\begin{document}
\vspace*{3cm}\noindent\hspace*{2.5cm}\includegraphics[%s]{%s}
\end{document}
"""

# ----------------------------------------------------------------------------
# A small PDF object parser (uncompressed object syntax; Flate streams).

TOKEN = re.compile(rb"\s*(?:(%[^\r\n]*)|(<<|>>|\[|\])|(/[^\s/\[\]<>()%{}]*)|(\((?:\\.|[^\\)])*\))|(<[0-9A-Fa-f\s]*>)|([+-]?\d*\.?\d+)|([A-Za-z*'\"]+))", re.S)


def tokens(b, pos=0):
    while True:
        m = TOKEN.match(b, pos)
        if not m or m.end() == pos:
            return
        pos = m.end()
        if m.group(1):
            continue
        for k in range(2, 8):
            if m.group(k) is not None:
                yield k, m.group(k), pos
                break


class Ref(tuple):
    pass


def parse_objects(b):
    objs = {}
    for m in re.finditer(rb"(\d+) (\d+) obj\b", b):
        n = int(m.group(1))
        toks = list_tokens(b, m.end())
        objs[n] = toks
    return objs


def list_tokens(b, pos):
    """Tokens of one object up to endobj, with refs folded, plus stream bytes."""
    out = []
    stream = None
    for k, v, end in tokens(b, pos):
        if k == 7 and v == b"endobj":
            break
        if k == 7 and v == b"stream":
            start = end
            if b[start:start + 2] == b"\r\n":
                start += 2
            elif b[start:start + 1] in (b"\n", b"\r"):
                start += 1
            stream = start
            break
        if k == 7 and v == b"R" and len(out) >= 2 and out[-1][0] == 6 and out[-2][0] == 6:
            g = out.pop()
            n = out.pop()
            out.append((8, (int(float(n[1])), int(float(g[1]))), end))
            continue
        out.append((k, v, end))
    return out, stream


def value_from(tok_list):
    it = iter(tok_list)
    return build(it)


def build(it):
    k, v, _ = next(it)
    if k == 8:
        return Ref(v)
    if k == 2 and v == b"<<":
        d = {}
        while True:
            k2, v2, _ = next(it)
            if k2 == 2 and v2 == b">>":
                return d
            d[v2[1:].decode()] = build(it)
    if k == 2 and v == b"[":
        a = []
        while True:
            try:
                x = build(it)
            except StopIteration:
                return a
            if x == "]":
                return a
            a.append(x)
    if k == 2 and v == b"]":
        return "]"
    if k == 3:
        return ("name", v[1:].decode())
    if k == 6:
        return float(v)
    if k == 7:
        return v.decode()
    return v


class Pdf:
    def __init__(self, path):
        self.b = open(path, "rb").read()
        self.raw = parse_objects(self.b)

    def obj(self, n):
        toks, _ = self.raw[n]
        return value_from(toks)

    def resolve(self, v):
        while isinstance(v, Ref):
            v = self.obj(v[0])
        return v

    def stream(self, n):
        toks, start = self.raw[n]
        d = value_from(toks)
        length = int(self.resolve(d["Length"]))
        data = self.b[start:start + length]
        f = self.resolve(d.get("Filter"))
        if f == ("name", "FlateDecode"):
            data = zlib.decompress(data)
        return d, data

    def pages(self):
        cat = next(self.obj(n) for n in self.raw if isinstance(self.obj(n), dict) and self.obj(n).get("Type") == ("name", "Catalog"))
        out = []

        def walk(node):
            node = self.resolve(node)
            if node.get("Type") == ("name", "Page"):
                out.append(node)
            else:
                for k in node["Kids"]:
                    walk(k)

        walk(cat["Pages"])
        return out


def mul(m, n):
    a, b, c, d, e, f = m
    A, B, C, D, E, F = n
    return [a * A + b * C, a * B + b * D, c * A + d * C, c * B + d * D, e * A + f * C + E, e * B + f * D + F]


def placements(pdf, page):
    """(name, ctm) for every Do on the page."""
    contents = pdf.resolve(page["Contents"])
    ref = page["Contents"]
    _, data = pdf.stream(ref[0])
    ctm, stack, operands, out = [1, 0, 0, 1, 0, 0], [], [], []
    for k, v, _ in tokens(data):
        if k in (6,):
            operands.append(float(v))
        elif k == 3:
            operands.append(v[1:].decode())
        elif k == 7:
            op = v.decode()
            if op == "q":
                stack.append(ctm)
            elif op == "Q":
                ctm = stack.pop()
            elif op == "cm":
                ctm = mul(operands[-6:], ctm)
            elif op == "Do":
                out.append((operands[-1], ctm))
            operands = []
        else:
            operands = []
    return out


def name_of(v):
    return v[1] if isinstance(v, tuple) and len(v) == 2 and v[0] == "name" else v


def summary(pdf, ref):
    d, data = pdf.stream(ref[0])
    s = {}
    for key in ("Subtype", "Width", "Height", "BitsPerComponent", "Filter", "BBox", "Matrix", "Decode"):
        if key in d:
            v = pdf.resolve(d[key])
            s[key] = [name_of(x) for x in v] if isinstance(v, list) else name_of(v)
    if "ColorSpace" in d:
        cs = pdf.resolve(d["ColorSpace"])
        if isinstance(cs, list):
            s["ColorSpace"] = [name_of(cs[0]), name_of(cs[1]), cs[2]]
            _, lookup = pdf.stream(cs[3][0]) if isinstance(cs[3], Ref) else (None, cs[3])
            s["Lookup_sha256"] = hashlib.sha256(lookup).hexdigest()
        else:
            s["ColorSpace"] = name_of(cs)
    if name_of(pdf.resolve(d.get("Filter"))) == "DCTDecode":
        toks, start = pdf.raw[ref[0]]
        data = pdf.b[start:start + int(pdf.resolve(d["Length"]))]
    s["samples_sha256"] = hashlib.sha256(data).hexdigest()
    if "SMask" in d:
        s["SMask"] = summary(pdf, d["SMask"])
    return s


def tick(v):
    return int(round(v * TICKS))


def fmt(v):
    v = round(v * 1000) / 1000 + 0.0
    return int(v) if v == int(v) else v


def build_case(name, image, options, tmp):
    src = os.path.join(HERE, image)
    shutil.copy(src, os.path.join(tmp, image))
    with open(os.path.join(tmp, "case.tex"), "w") as f:
        f.write(TEX % (options, image))
    env = dict(os.environ, SOURCE_DATE_EPOCH="1767225600", FORCE_SOURCE_DATE="1")
    subprocess.run([PDFLATEX, "-interaction=batchmode", "case.tex"], cwd=tmp, env=env, check=True, capture_output=True)
    ref_pdf = os.path.join(OUT, name + ".reference.pdf")
    shutil.copy(os.path.join(tmp, "case.pdf"), ref_pdf)
    pdf = Pdf(ref_pdf)
    page = pdf.pages()[0]
    media = pdf.resolve(page["MediaBox"])
    H = media[3]
    xobjects = pdf.resolve(pdf.resolve(page["Resources"])["XObject"])
    (res, ctm), = placements(pdf, page)
    x = summary(pdf, xobjects[res])
    fmt_name = {"png": "png", "jpg": "jpeg", "pdf": "pdf"}[image.rsplit(".", 1)[1]]
    if x["Subtype"] == "Form":
        llx, lly, urx, ury = x["BBox"]
        w, h = urx - llx, ury - lly
        if "Matrix" in x:
            m = x["Matrix"]
            rotate = {(0, -1, 1, 0): 90, (-1, 0, 0, -1): 180, (0, 1, -1, 0): 270}[tuple(int(v) for v in m[:4])]
            W, Hh = (h, w) if rotate % 180 == 90 else (w, h)
            unit = mul([W, 0, 0, Hh, 0, 0], ctm)
        else:
            rotate = 0
            unit = mul([w, 0, 0, h, llx, lly], ctm)
    else:
        rotate = None
        unit = ctm
    a, b, c, d, e, f = unit
    transform = [fmt(a), fmt(-b), fmt(c), fmt(-d), fmt(e), fmt(H - f)]
    corners = [(transform[4] + transform[0] * u + transform[2] * v, transform[5] + transform[1] * u + transform[3] * v) for u in (0, 1) for v in (0, 1)]
    xs, ys = [p[0] for p in corners], [p[1] for p in corners]
    data = open(src, "rb").read()
    res_json = {
        "image_id": hashlib.sha256(data).hexdigest(),
        "sha256": hashlib.sha256(data).hexdigest(),
        "byte_length": len(data),
        "format": fmt_name,
        "path": image,
    }
    if fmt_name == "pdf":
        res_json.update({"pdf_page": 1, "pdf_box": x["BBox"], "pdf_rotate": rotate})
    else:
        res_json.update({"pixel_width": x["Width"], "pixel_height": x["Height"]})
    envelope = {
        "protocol_version": 2,
        "id": name,
        "type": "display_list",
        "payload": {
            "render_format": "display-list-v2",
            "coordinate_unit": "bp_2pow20",
            "color_space": "srgb",
            "diagnostics": [],
            "fonts": [],
            "pages": [{
                "number": 1,
                "width": tick(media[2]),
                "height": tick(H),
                "items": [{
                    "kind": "image",
                    "x": tick(min(xs)), "top": tick(min(ys)),
                    "width": tick(max(xs) - min(xs)), "height": tick(max(ys) - min(ys)),
                    "transform": transform,
                    "image": res_json,
                    "sources": [{"path": "main.tex", "start_byte": 0, "end_byte": 0}],
                }],
            }],
        },
    }
    with open(os.path.join(OUT, name + ".v2.json"), "w") as f:
        json.dump(envelope, f, indent=1, sort_keys=True)
        f.write("\n")
    return {
        "name": name,
        "image": image,
        "options": options,
        "ctm": [round(v, 6) for v in ctm],
        "xobject": x,
        "page_group": "Group" in page,
    }


def raster(pdf, prefix, dpi=150):
    subprocess.run([GS, "-q", "-dNOPAUSE", "-dBATCH", "-dSAFER", "-sDEVICE=png16m", f"-r{dpi}", "-dTextAlphaBits=1", "-dGraphicsAlphaBits=1", "-o", f"{prefix}-{dpi}-%d.png", pdf], check=True, capture_output=True)
    return f"{prefix}-{dpi}-1.png"


def compare(a, b):
    from PIL import Image, ImageChops
    ia, ib = Image.open(a).convert("RGB"), Image.open(b).convert("RGB")
    if ia.size != ib.size:
        return {"size_a": ia.size, "size_b": ib.size}
    diff = ImageChops.difference(ia, ib)
    hist = diff.convert("L").histogram()
    total = ia.size[0] * ia.size[1]
    over = sum(hist[3:])
    extrema = diff.getextrema()
    painted = sum(1 for p in ia.convert("L").getdata() if p < 250)
    return {"pixels": total, "painted_reference_pixels": painted, "pixels_over_2": over, "max_channel_diff": max(e[1] for e in extrema)}


def main():
    exact = None
    if "--exact" in sys.argv:
        exact = os.path.abspath(sys.argv[sys.argv.index("--exact") + 1])
    os.makedirs(OUT, exist_ok=True)
    version = subprocess.run([PDFLATEX, "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    cases = []
    for name, image, options in CASES:
        with tempfile.TemporaryDirectory() as tmp:
            cases.append(build_case(name, image, options, tmp))
    with open(os.path.join(OUT, "expected.json"), "w") as f:
        json.dump({"pdflatex": version, "tolerance_bp": 0.01, "cases": cases}, f, indent=1, sort_keys=True)
        f.write("\n")
    print(f"wrote {len(cases)} cases ({version})")
    if not exact:
        return
    results = {"gs": subprocess.run([GS, "--version"], capture_output=True, text=True).stdout.strip(), "dpi": 150, "cases": []}
    with tempfile.TemporaryDirectory() as tmp:
        for c in cases:
            name = c["name"]
            out_pdf = os.path.join(tmp, name + ".pdf")
            p = subprocess.run([exact, "from-v2", os.path.join(OUT, name + ".v2.json"), "--out", out_pdf, "--project-root", HERE], capture_output=True, text=True)
            if p.returncode != 0:
                results["cases"].append({"name": name, "error": p.stderr.strip()[-400:]})
                print(name, "EXPORT FAILED", p.stderr.strip()[-400:])
                continue
            ours = Pdf(out_pdf)
            page = ours.pages()[0]
            (res, ctm), = placements(ours, page)
            xs = ours.resolve(ours.resolve(page["Resources"])["XObject"])
            ctm_err = max(abs(u - v) for u, v in zip(ctm, c["ctm"]))
            # Compare the placed unit square's corners (what the CTM does to the image) in bp.
            r = compare(raster(os.path.join(OUT, name + ".reference.pdf"), os.path.join(tmp, name + "-ref")), raster(out_pdf, os.path.join(tmp, name + "-ours")))
            if r.get("pixels_over_2"):
                # An edge exactly on a pixel boundary flips a whole row or
                # column under a sub-1e-4 bp CTM difference (the producer
                # rounds the transform to 1/1000 pt); a second resolution
                # separates that from a content difference.
                again = compare(raster(os.path.join(OUT, name + ".reference.pdf"), os.path.join(tmp, name + "-ref"), 300), raster(out_pdf, os.path.join(tmp, name + "-ours"), 300))
                r["pixels_over_2_at_300dpi"] = again.get("pixels_over_2")
            same_xobject = summary(ours, xs[res]) == c["xobject"]
            r.update({"name": name, "ctm_max_abs_diff": round(ctm_err, 6), "xobject_summary_identical": same_xobject, "page_group": "Group" in page})
            results["cases"].append(r)
            print(name, r)
    with open(os.path.join(OUT, "raster.json"), "w") as f:
        json.dump(results, f, indent=1, sort_keys=True)
        f.write("\n")


if __name__ == "__main__":
    main()
