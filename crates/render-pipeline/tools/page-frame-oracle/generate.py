#!/usr/bin/env python3
"""Page-frame oracle: twoside margins, two-column frames, page numbers and
running heads, report/book chapter openers and \\noindent.

pdflatex is the ORACLE ONLY (never in the product path, never in cargo
tests). This script

  1. writes fixtures/page-frame/<name>.tex (deterministic text),
  2. runs pdflatex on each with \\pdfcompresslevel=0 \\pdfobjcompresslevel=0
     given on the command line (changes nothing typeset, only how the PDF is
     stored), and
  3. reads every word's origin and baseline straight from the content
     streams (Td/TD/Tm/T*/Tf/TJ/Tj with the font /Widths, q/Q/cm and `re`
     rules), writing fixtures/page-frame/expected/<name>.json in PDF big
     points with a top-left origin.

A word is a run of TJ glyphs between adjustments of <= -150/1000 em (the
interword glue pdfTeX writes as a kern; letter kerns are far smaller).

Usage: generate.py [--keep DIR] [NAME ...]
"""
import argparse
import json
import os
import random
import re
import shutil
import sys
import tempfile
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(os.path.dirname(HERE))
REPO = os.path.dirname(os.path.dirname(CRATE))
FIXTURES = os.path.join(CRATE, "fixtures", "page-frame")
EXPECTED = os.path.join(FIXTURES, "expected")

sys.path.insert(0, REPO)
from tools.oracle import pdflatex as oracle  # noqa: E402

WORDS = (
    "the of and to in is was for on are with as by at be this from that or "
    "not but all were when can there an which their if has more one would "
    "some time will each about many then them these so other into only two "
    "could its now over most such where after may well also back even here "
    "much good same both between under never last while might great old "
    "year off come since move part place long made live round keep plan "
    "line page text book word note case mark head foot side left right top "
    "low set run hold form draw box rule glue space break small large wide"
).split()


def sentences(seed, n_words):
    rng = random.Random(seed)
    out, count = [], 0
    while count < n_words:
        k = rng.randint(6, 14)
        ws = [rng.choice(WORDS) for _ in range(k)]
        ws[0] = ws[0].capitalize()
        out.append(" ".join(ws) + ".")
        count += k
    return " ".join(out)


def paras(seed, count, words=90):
    return "\n\n".join(sentences(seed * 100 + i, words) for i in range(count))


def doc(cls, opts, body, preamble=""):
    o = f"[{opts}]" if opts else ""
    # Latin Modern in T1: the metrics (ec-lm*.tfm) the pipeline lays out with.
    fonts = "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n"
    return f"\\documentclass{o}{{{cls}}}\n{fonts}{preamble}\\begin{{document}}\n{body}\n\\end{{document}}\n"


def sectioned(seed, sections, per, words=90, sub=False):
    parts = []
    names = ["Introduction", "Method", "Results", "Discussion", "Summary", "Outlook"]
    for s in range(sections):
        parts.append(f"\\section{{{names[s % len(names)]}}}")
        parts.append(paras(seed + s, per, words))
        if sub:
            parts.append(f"\\subsection{{Details {s + 1}}}")
            parts.append(paras(seed + 50 + s, per, words))
    return "\n\n".join(parts)


def fixtures():
    f = {}
    f["01-article-plain"] = doc("article", "", paras(1, 11))
    f["02-article-empty"] = doc("article", "", paras(2, 11), "\\pagestyle{empty}\n")
    f["03-article-twoside-plain"] = doc("article", "twoside", paras(3, 16))
    f["04-article-twoside-headings"] = doc("article", "twoside", sectioned(4, 4, 3), "\\pagestyle{headings}\n")
    f["05-article-oneside-headings"] = doc("article", "", sectioned(5, 4, 3), "\\pagestyle{headings}\n")
    f["06-article-twoside-myheadings"] = doc(
        "article", "twoside", "\\markboth{Left Head Text}{Right Head Text}\n" + paras(6, 16), "\\pagestyle{myheadings}\n"
    )
    f["07-article-oneside-myheadings"] = doc("article", "", "\\markright{Only Right Head}\n" + paras(7, 11), "\\pagestyle{myheadings}\n")
    f["08-article-twocolumn"] = doc("article", "twocolumn", paras(8, 20))
    f["09-article-twocolumn-rule"] = doc("article", "twocolumn", paras(9, 20), "\\setlength{\\columnseprule}{0.4pt}\n")
    f["10-article-twocolumn-twoside-headings"] = doc("article", "twocolumn,twoside", sectioned(10, 4, 5), "\\pagestyle{headings}\n")
    f["11-report-chapter"] = doc("report", "", "\\chapter{Getting Started}\n" + paras(11, 10))
    f["12-report-chapter-headings"] = doc("report", "", "\\chapter{First Steps}\n" + paras(12, 12), "\\pagestyle{headings}\n")
    f["13-book-chapter"] = doc("book", "", "\\chapter{Opening}\n" + paras(13, 14))
    f["14-article-noindent"] = doc(
        "article",
        "",
        "\n\n".join(
            [paras(14, 1), "\\noindent " + sentences(1401, 80), paras(15, 1), "\\noindent\n" + sentences(1402, 80), paras(16, 1)]
        ),
    )
    f["15-article-11pt-twoside-plain"] = doc("article", "11pt,twoside", paras(17, 14))
    f["16-article-12pt-a4-geometry-plain"] = doc("article", "12pt,a4paper", paras(18, 9), "\\usepackage[margin=1in]{geometry}\n")
    f["17-article-thispagestyle-empty"] = doc("article", "", "\\thispagestyle{empty}\n" + paras(19, 11))
    f["18-article-geometry-twoside-asymmetric"] = doc(
        "article", "twoside", paras(20, 12), "\\usepackage[inner=1in,outer=2in,top=1in,bottom=1in]{geometry}\n"
    )
    f["19-article-twocolumn-geometry-12pt"] = doc(
        "article", "12pt,twocolumn", paras(21, 16), "\\usepackage[margin=0.75in,columnsep=0.25in]{geometry}\n"
    )
    f["20-article-twoside-headings-subsections"] = doc("article", "twoside", sectioned(22, 3, 2, sub=True), "\\pagestyle{headings}\n")
    f["21-report-twoside-headings"] = doc("report", "twoside", "\\chapter{Main Part}\n" + sectioned(23, 3, 3), "\\pagestyle{headings}\n")
    f["22-article-pagestyle-in-body"] = doc("article", "", paras(24, 5) + "\n\n\\pagestyle{empty}\n\n" + paras(25, 6))
    f["23-article-roman-then-arabic"] = doc(
        "article", "", "\\pagenumbering{roman}\n" + paras(26, 8) + "\n\n\\clearpage\n\\pagenumbering{arabic}\n" + paras(27, 8)
    )
    f["24-article-Roman-twoside-headings"] = doc(
        "article", "twoside", "\\pagenumbering{Roman}\n" + sectioned(28, 3, 3), "\\pagestyle{headings}\n"
    )
    f["25-article-setcounter-page"] = doc("article", "", "\\setcounter{page}{7}\n" + paras(29, 11))
    f["26-article-twoside-setcounter-even"] = doc("article", "twoside", "\\setcounter{page}{4}\n" + paras(30, 12))
    f["27-article-alph-Alph"] = doc(
        "article", "", "\\pagenumbering{alph}\n" + paras(31, 8) + "\n\n\\clearpage\n\\pagenumbering{Alph}\n" + paras(32, 8)
    )
    f["28-book-openright"] = doc("book", "", "\\chapter{Opening}\n" + paras(33, 3) + "\n\n\\chapter{Second}\n" + paras(34, 6))
    f["29-book-openright-headings"] = doc(
        "book", "", "\\chapter{Opening}\n" + paras(35, 3) + "\n\n\\chapter{Second}\n" + paras(36, 6), "\\pagestyle{headings}\n"
    )
    f["30-report-twoside-openright"] = doc(
        "report", "twoside,openright", "\\chapter{Alpha}\n" + paras(37, 3) + "\n\n\\chapter{Beta}\n" + paras(38, 5)
    )
    f["31-article-maketitle-headings"] = doc(
        "article", "", "\\maketitle\n" + sectioned(39, 3, 3), "\\pagestyle{headings}\n\\title{A Title}\n\\author{An Author}\n\\date{1 May 2020}\n"
    )
    f["32-article-maketitle-empty"] = doc(
        "article", "", "\\maketitle\n" + paras(40, 11), "\\pagestyle{empty}\n\\title{A Title}\n\\author{An Author}\n\\date{1 May 2020}\n"
    )
    # \maketitle layout (article.cls/report.cls/book.cls \@maketitle and
    # titlepage; book \frontmatter/\mainmatter/\backmatter). Every fixture
    # gives \date explicitly: pdflatex's \today is the run date.
    f["33-article-maketitle-and"] = doc(
        "article",
        "",
        "\\maketitle\n" + sectioned(41, 3, 3),
        "\\title{A Longer Title for Testing}\n\\author{First Author \\and Second Writer \\and Third Person}\n\\date{2 June 2021}\n",
    )
    f["34-article-maketitle-author-lines-nodate"] = doc(
        "article",
        "",
        "\\maketitle\n" + paras(42, 11),
        "\\title{Stacked Authors}\n\\author{Ann Author\\\\ University of Somewhere \\and Bob Writer\\\\ Institute Two\\\\ Some City}\n\\date{}\n",
    )
    f["35-article-maketitle-thanks"] = doc(
        "article",
        "",
        "\\maketitle\n" + sectioned(43, 3, 3),
        "\\title{A Title\\thanks{Supported by a grant.}}\n\\author{An Author\\thanks{Corresponding author.}}\n\\date{3 July 2022}\n",
    )
    f["36-article-titlepage"] = doc(
        "article", "titlepage", "\\maketitle\n" + sectioned(44, 3, 3), "\\title{A Title Page}\n\\author{An Author}\n\\date{4 August 2023}\n"
    )
    f["37-report-maketitle"] = doc(
        "report", "", "\\maketitle\n\\chapter{Getting Started}\n" + paras(45, 10), "\\title{A Report}\n\\author{One Author \\and Two Author}\n\\date{5 May 2024}\n"
    )
    f["38-book-maketitle"] = doc(
        "book", "", "\\maketitle\n\\chapter{Opening}\n" + paras(46, 10), "\\title{A Book}\n\\author{Book Author}\n\\date{6 June 2025}\n"
    )
    f["39-report-notitlepage"] = doc(
        "report", "notitlepage", "\\maketitle\n" + paras(47, 10), "\\title{A Compact Report}\n\\author{Report Author}\n\\date{7 July 2019}\n"
    )
    f["40-article-twocolumn-maketitle"] = doc(
        "article", "twocolumn", "\\maketitle\n" + sectioned(48, 4, 4), "\\title{A Two Column Title}\n\\author{Left Author \\and Right Author}\n\\date{8 March 2018}\n"
    )
    f["41-article-11pt-twoside-maketitle-nodate"] = doc(
        "article", "11pt,twoside", "\\maketitle\n" + sectioned(49, 3, 4), "\\pagestyle{headings}\n\\title{Eleven Point Title}\n\\author{Some Author}\n\\date{}\n"
    )
    f["42-article-12pt-maketitle-long-title"] = doc(
        "article",
        "12pt",
        "\\maketitle\n" + paras(50, 10),
        "\\title{A Considerably Longer Title That Has to Wrap Across More Than One Line of the Page\\\\ With a Forced Second Part}\n\\author{An Author}\n\\date{9 September 2017}\n",
    )
    f["43-book-frontmatter-mainmatter-backmatter"] = doc(
        "book",
        "",
        "\\frontmatter\n\\chapter{Preface}\n"
        + paras(51, 3)
        + "\n\n\\mainmatter\n\\chapter{Introduction}\n"
        + paras(52, 4)
        + "\n\n\\chapter{Development}\n"
        + paras(53, 3)
        + "\n\n\\backmatter\n\\chapter{Notes}\n"
        + paras(54, 3),
        "\\pagestyle{headings}\n",
    )
    f["44-article-twoside-titlepage"] = doc(
        "article", "twoside,titlepage", "\\maketitle\n" + paras(55, 14), "\\title{Two Sided Title Page}\n\\author{First Author \\and Second Author}\n\\date{10 October 2016}\n"
    )
    return f


def run_pdflatex(tex_path, workdir):
    # Kept (same name, same behavior) for the generators that import this
    # module (`pf.run_pdflatex`); the logic lives in tools/oracle/pdflatex.py.
    return oracle.run_pdflatex(tex_path, workdir, error_tail=2000)


# ---- minimal PDF reader (uncompressed pdfTeX output) ----

def read_objects(data):
    objs = {}
    for m in re.finditer(rb"(\d+) 0 obj\s*(.*?)endobj", data, re.S):
        objs[int(m.group(1))] = m.group(2)
    return objs


def ref(body, key):
    m = re.search(rb"/" + key + rb"\s+(\d+) 0 R", body)
    return int(m.group(1)) if m else None


def stream_of(body):
    m = re.search(rb"stream\r?\n(.*)endstream", body, re.S)
    raw = m.group(1)
    if b"/FlateDecode" in body:
        return zlib.decompress(raw)
    return raw


def pages_in_order(objs):
    root = next(b for b in objs.values() if re.search(rb"/Type\s*/Catalog", b))
    out = []

    def walk(n):
        b = objs[n]
        if re.search(rb"/Type\s*/Pages", b):
            kids = re.search(rb"/Kids\s*\[(.*?)\]", b, re.S).group(1)
            for k in re.findall(rb"(\d+) 0 R", kids):
                walk(int(k))
        else:
            out.append(n)

    walk(ref(root, b"Pages"))
    return out


def dict_entries(body):
    return {k.decode(): int(v) for k, v in re.findall(rb"/(\w+)\s+(\d+) 0 R", body)}


def fonts_of(objs, page):
    b = objs[page]
    res = re.search(rb"/Resources\s*(\d+) 0 R", b)
    rb_ = objs[int(res.group(1))] if res else b
    fm = re.search(rb"/Font\s*<<(.*?)>>", rb_, re.S)
    if fm:
        entries = dict_entries(fm.group(1))
    else:
        entries = dict_entries(objs[ref(rb_, b"Font")])
    fonts = {}
    for name, n in entries.items():
        fb = objs[n]
        first = int(re.search(rb"/FirstChar\s+(\d+)", fb).group(1))
        wm = re.search(rb"/Widths\s*(\d+) 0 R", fb)
        warr = objs[int(wm.group(1))] if wm else re.search(rb"/Widths\s*(\[.*?\])", fb, re.S).group(1)
        widths = [float(x) for x in re.findall(rb"-?[\d.]+", warr)]
        base = re.search(rb"/BaseFont\s*/([\w+-]+)", fb).group(1).decode()
        fonts[name] = (first, widths, base.split("+")[-1])
    return fonts


def media_box(objs, page):
    b = objs[page]
    m = re.search(rb"/MediaBox\s*\[\s*([-\d.\s]+)\]", b)
    if not m:
        pages = next(v for v in objs.values() if re.search(rb"/Type\s*/Pages", v) and b"/MediaBox" in v)
        m = re.search(rb"/MediaBox\s*\[\s*([-\d.\s]+)\]", pages)
    return [float(x) for x in m.group(1).split()]


T1 = {0x1B: "ff", 0x1C: "fi", 0x1D: "fl", 0x1E: "ffi", 0x1F: "ffl"}


def tokens(content):
    i, n = 0, len(content)
    while i < n:
        c = content[i:i + 1]
        if c.isspace():
            i += 1
        elif c == b"%":
            while i < n and content[i:i + 1] not in (b"\n", b"\r"):
                i += 1
        elif c == b"(":
            depth, j, s = 1, i + 1, bytearray()
            while depth:
                ch = content[j]
                if ch == 0x5C:
                    nx = content[j + 1]
                    if 0x30 <= nx <= 0x37:
                        k = j + 1
                        while k < j + 4 and 0x30 <= content[k] <= 0x37:
                            k += 1
                        s.append(int(content[j + 1:k], 8) & 0xFF)
                        j = k
                        continue
                    s.append({0x6E: 10, 0x72: 13, 0x74: 9, 0x62: 8, 0x66: 12}.get(nx, nx))
                    j += 2
                    continue
                if ch == 0x28:
                    depth += 1
                elif ch == 0x29:
                    depth -= 1
                    if depth == 0:
                        break
                s.append(ch)
                j += 1
            yield ("str", bytes(s))
            i = j + 1
        elif c in (b"[", b"]"):
            yield ("op", c.decode())
            i += 1
        elif c == b"/":
            m = re.match(rb"/[^\s/\[\]()<>]+", content[i:])
            yield ("name", m.group(0)[1:].decode())
            i += len(m.group(0))
        elif c == b"<":
            j = content.index(b">", i)
            yield ("hex", bytes.fromhex(content[i + 1:j].decode()))
            i = j + 1
        else:
            m = re.match(rb"[^\s/\[\]()<>%]+", content[i:])
            t = m.group(0)
            i += len(t)
            try:
                yield ("num", float(t))
            except ValueError:
                yield ("op", t.decode())


def mul(a, b):
    return [
        a[0] * b[0] + a[1] * b[2], a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2], a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4], a[4] * b[1] + a[5] * b[3] + b[5],
    ]


def page_words(content, fonts, height):
    words, rules = [], []
    ctm, stack = [1, 0, 0, 1, 0, 0], []
    tm = tlm = [1, 0, 0, 1, 0, 0]
    font, size, lead = None, 0.0, 0.0
    operands = []
    line_width, path = 1.0, []
    cur = None  # (text, x, y, font, size)

    def flush():
        nonlocal cur
        if cur and cur[0].strip():
            words.append({"text": cur[0], "x": round(cur[1], 4), "baseline": round(height - cur[2], 4), "font": cur[3], "size": round(cur[4], 4)})
        cur = None

    def show(items):
        nonlocal tm, cur
        first, widths, base = fonts[font]
        for it in items:
            if isinstance(it, float):
                if it <= -150:
                    flush()
                tm = mul([1, 0, 0, 1, -it / 1000 * size, 0], tm)
                continue
            for code in it:
                m = mul(tm, ctm)
                ch = T1.get(code) or (chr(code) if 32 < code < 127 else "?")
                if code == 32:
                    flush()
                elif cur is None:
                    cur = [ch, m[4], m[5], base, size * abs(m[0])]
                else:
                    cur[0] += ch
                w = widths[code - first] if 0 <= code - first < len(widths) else 0.0
                tm = mul([1, 0, 0, 1, w / 1000 * size, 0], tm)

    arr = None
    for kind, v in tokens(content):
        if kind == "op" and v == "[":
            arr = []
            continue
        if kind == "op" and v == "]":
            operands.append(arr)
            arr = None
            continue
        if arr is not None:
            arr.append(v)
            continue
        if kind != "op":
            operands.append(v)
            continue
        o = operands
        if v == "q":
            stack.append(ctm[:])
        elif v == "Q":
            ctm = stack.pop()
        elif v == "cm":
            ctm = mul(o[-6:], ctm)
        elif v == "BT":
            tm = tlm = [1, 0, 0, 1, 0, 0]
        elif v == "ET":
            flush()
        elif v == "Tf":
            flush()
            font, size = o[-2], o[-1]
        elif v == "TL":
            lead = o[-1]
        elif v in ("Td", "TD"):
            flush()
            if v == "TD":
                lead = -o[-1]
            tlm = mul([1, 0, 0, 1, o[-2], o[-1]], tlm)
            tm = tlm[:]
        elif v == "Tm":
            flush()
            tlm = o[-6:]
            tm = tlm[:]
        elif v == "T*":
            flush()
            tlm = mul([1, 0, 0, 1, 0, -lead], tlm)
            tm = tlm[:]
        elif v == "TJ":
            show([x if isinstance(x, float) else x for x in o[-1]])
        elif v == "Tj":
            show([o[-1]])
        elif v == "w":
            line_width = o[-1]
        elif v == "m":
            path = [mul([1, 0, 0, 1, o[-2], o[-1]], ctm)[4:]]
        elif v == "l":
            path.append(mul([1, 0, 0, 1, o[-2], o[-1]], ctm)[4:])
        elif v == "S" and len(path) == 2 and abs(path[0][0] - path[1][0]) < 1e-6:
            # A vertical stroked rule (pdfTeX's \vrule in \@outputdblcol):
            # centred on x, `line_width` wide.
            (x, y0), (_, y1) = path
            lo, hi = sorted([y0, y1])
            rules.append({"x": round(x - line_width / 2, 4), "top": round(height - hi, 4), "width": round(line_width, 4), "height": round(hi - lo, 4)})
            path = []
        elif v == "re":
            x, y, w, h = o[-4:]
            p0 = mul([1, 0, 0, 1, x, y], ctm)
            p1 = mul([1, 0, 0, 1, x + w, y + h], ctm)
            x0, x1 = sorted([p0[4], p1[4]])
            y0, y1 = sorted([p0[5], p1[5]])
            rules.append({"x": round(x0, 4), "top": round(height - y1, 4), "width": round(x1 - x0, 4), "height": round(y1 - y0, 4)})
        operands = []
    flush()
    return words, rules


def extract(pdf):
    data = open(pdf, "rb").read()
    objs = read_objects(data)
    pages = []
    for i, p in enumerate(pages_in_order(objs)):
        mb = media_box(objs, p)
        height = mb[3] - mb[1]
        body = objs[p]
        cm = re.search(rb"/Contents\s*(\d+) 0 R", body)
        content = stream_of(objs[int(cm.group(1))])
        words, rules = page_words(content, fonts_of(objs, p), height)
        pages.append({"number": i + 1, "width": mb[2] - mb[0], "height": height, "words": words, "rules": rules})
    return pages


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--keep", help="copy the pdflatex PDFs here")
    ap.add_argument("names", nargs="*")
    args = ap.parse_args()
    os.makedirs(EXPECTED, exist_ok=True)
    version = oracle.pdftex_version()
    for name, src in fixtures().items():
        if args.names and name not in args.names:
            continue
        tex = os.path.join(FIXTURES, name + ".tex")
        with open(tex, "w") as fh:
            fh.write(src)
        with tempfile.TemporaryDirectory() as tmp:
            pdf = run_pdflatex(tex, tmp)
            if args.keep:
                os.makedirs(args.keep, exist_ok=True)
                shutil.copy(pdf, args.keep)
            pages = extract(pdf)
        # One record per line (bp, top-left origin):
        #   page <n> <width> <height>
        #   word <page> <x> <baseline> <font> <size> <text>
        #   rule <page> <x> <top> <width> <height>
        lines = [f"# {version}", f"# source fixtures/page-frame/{name}.tex (tools/page-frame-oracle/generate.py)"]
        for p in pages:
            lines.append(f"page {p['number']} {p['width']:.4f} {p['height']:.4f}")
            for w in p["words"]:
                lines.append(f"word {p['number']} {w['x']:.4f} {w['baseline']:.4f} {w['font']} {w['size']:.4f} {w['text']}")
            for r in p["rules"]:
                lines.append(f"rule {p['number']} {r['x']:.4f} {r['top']:.4f} {r['width']:.4f} {r['height']:.4f}")
        with open(os.path.join(EXPECTED, name + ".txt"), "w") as fh:
            fh.write("\n".join(lines) + "\n")
        stale = os.path.join(EXPECTED, name + ".json")
        if os.path.exists(stale):
            os.remove(stale)
        print(name, len(pages), "pages", sum(len(p["words"]) for p in pages), "words")


if __name__ == "__main__":
    main()
