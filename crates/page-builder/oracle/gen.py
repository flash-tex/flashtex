#!/usr/bin/env python3
"""pdflatex oracle for crates/page-builder (developer tool; cargo tests never run TeX).

For every fixture it writes oracle/fixtures/<name>.tex and runs pdflatex twice
with oracle/hook.tex inserted after \\documentclass:

1. capture run (\\textheight=8000pt, \\holdinginserts=1): every output only
   happens at a forced break, so the concatenated \\showbox255 dumps give the
   complete main vertical list exactly as TeX's page builder receives it
   (with \\topskip glue and \\@doclearpage push-back removed);
2. real run: \\tracingpages=1 cost lines, \\box255 of every output, and the
   shipped column (\\tracingoutput) from which the expected baselines are
   computed with vlist_out's rounding.

pdftotext -bbox-layout provides each page's first/last text line and an
independent baseline cross-check. Everything lands in oracle/expected/<name>.ftpb
(format: src/format.rs and tests/oracle.rs).

Usage: python3 oracle/gen.py [--only NAME ...] [--workdir DIR]
"""

import argparse
import os
import random
import re
import shutil
import statistics
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
FIXTURES = os.path.join(HERE, "fixtures")
EXPECTED = os.path.join(HERE, "expected")
HOOK = os.path.join(HERE, "hook.tex")

MAXDIMEN = 0x3FFFFFFF
INF_BAD = 10000
AWFUL_BAD = 0x3FFFFFFF

# ---------------------------------------------------------------- TeX numbers


def sp(text):
    """TeX's scan of a decimal constant in points (round_decimals, §102)."""
    text = text.strip()
    for unit in ("pt",):
        if text.endswith(unit):
            text = text[: -len(unit)]
    neg = text.startswith("-")
    if neg:
        text = text[1:]
    if "." in text:
        ip, fp = text.split(".", 1)
    else:
        ip, fp = text, ""
    a = 0
    for ch in reversed(fp[:17]):
        a = (a + int(ch) * 0o400000) // 10
    v = int(ip or "0") * 65536 + (a + 1) // 2
    return -v if neg else v


def order_of(suffix):
    return 0 if not suffix else len(suffix) - 2


SPEC_RE = re.compile(
    r"^(-?[\d.]+)(?:pt)?(?: plus (-?[\d.]+)(?:pt|(fil+))?)?(?: minus (-?[\d.]+)(?:pt|(fil+))?)?$"
)


def parse_spec(text):
    m = SPEC_RE.match(text.strip())
    if not m:
        raise ValueError("bad glue spec %r" % text)
    w, st, sto, sh, sho = m.groups()
    return (sp(w), sp(st) if st else 0, order_of(sto), sp(sh) if sh else 0, order_of(sho))


def spec_str(g):
    return "%d %d %d %d %d" % g


# ---------------------------------------------------------------- box dumps

BOX_RE = re.compile(r"^\\([hv])box\((-?[\d.]+)\+(-?[\d.]+)\)x(-?[\d.]+)(.*)$")
GLUESET_RE = re.compile(r", glue set (- )?(>)?([\d.]+)(fil+)?")
RULE_RE = re.compile(r"^\\rule\((-?[\d.*]+)\+(-?[\d.*]+)\)x(.*)$")
GLUE_RE = re.compile(r"^\\(?:glue|leaders|cleaders|xleaders)(\(\\(\w+)\))? ?(.*)$")
KERN_RE = re.compile(r"^\\kern ?(-?[\d.]+)")
INS_RE = re.compile(r"^\\insert(\d+), natural size (-?[\d.]+); split\((.*),(-?[\d.]+)\); float cost (-?\d+)")


class Item:
    def __init__(self, text):
        self.text = text
        self.children = []


def parse_dump(lines, i):
    """Parses a show_box dump whose root is lines[i]. Returns (root, next)."""
    root = Item(lines[i])
    stack = [root]
    i += 1
    while i < len(lines) and lines[i].startswith("."):
        line = lines[i]
        depth = len(line) - len(line.lstrip("."))
        item = Item(line[depth:])
        while len(stack) > depth:
            stack.pop()
        if len(stack) == depth:
            stack[-1].children.append(item)
            stack.append(item)
        i += 1
    return root, i


def classify(item):
    t = item.text
    m = BOX_RE.match(t)
    if m:
        g = GLUESET_RE.search(m.group(5))
        sign, ratio, order = 0, 0.0, 0
        if g:
            sign = -1 if g.group(1) else 1
            # ">20000.0" (print_glue cap, §186): only the glue that gets the
            # excess is affected, and it is the last stretchable node here.
            ratio = sp(g.group(3)) / 65536.0
            order = order_of(g.group(4))
        return {
            "k": m.group(1).upper(),
            "h": sp(m.group(2)),
            "d": sp(m.group(3)),
            "w": sp(m.group(4)),
            "sign": sign,
            "ratio": ratio,
            "order": order,
            "item": item,
        }
    m = RULE_RE.match(t)
    if m:
        h = 0 if m.group(1) == "*" else sp(m.group(1))
        d = 0 if m.group(2) == "*" else sp(m.group(2))
        return {"k": "R", "h": h, "d": d}
    m = INS_RE.match(t)
    if m:
        return {
            "k": "I",
            "n": int(m.group(1)),
            "h": sp(m.group(2)),
            "stsk": parse_spec(m.group(3)),
            "smd": sp(m.group(4)),
            "cost": int(m.group(5)),
            "item": item,
        }
    m = GLUE_RE.match(t)
    if m:
        return {"k": "G", "spec": parse_spec(m.group(3)), "top": m.group(2) == "topskip"}
    m = KERN_RE.match(t)
    if m:
        return {"k": "K", "w": sp(m.group(1))}
    if t.startswith("\\penalty "):
        return {"k": "P", "v": int(t.split()[1])}
    if t.startswith("\\mark"):
        return {"k": "M"}
    if t.startswith("\\"):
        return {"k": "W"}
    raise ValueError("unknown node %r" % t)


def node_lines(n):
    k = n["k"]
    if k in "HV":
        return ["%s %d %d %d" % (k, n["h"], n["d"], n["w"])]
    if k == "R":
        return ["R %d %d" % (n["h"], n["d"])]
    if k == "G":
        return ["G " + spec_str(n["spec"]) + (" T" if n["top"] else "")]
    if k == "K":
        return ["K %d" % n["w"]]
    if k == "P":
        return ["P %d" % n["v"]]
    if k in "MW":
        return [k]
    if k == "I":
        sub = [classify(c) for c in n["item"].children]
        out = ["I %d %d %d %s %d %d" % (n["n"], n["h"], n["smd"], spec_str(n["stsk"]), n["cost"], len(sub))]
        for s in sub:
            out.extend(node_lines(s))
        return out
    raise ValueError(k)


def box_signature(n):
    """Signature as src/format.rs node_line prints it (inserts without contents)."""
    if n["k"] == "I":
        return "I %d %d" % (n["n"], n["h"])
    return node_lines(n)[0]


# ---------------------------------------------------------------- vert_break (h = 0)


def precedes_break(n):
    return n["k"] in ("H", "V", "R", "I", "M", "W")


def badness(t, s):
    if t == 0:
        return 0
    if s <= 0:
        return INF_BAD
    if t <= 7230584:
        r = (t * 297) // s
    elif s >= 1663497:
        r = t // (s // 297)
    else:
        r = t
    return INF_BAD if r > 1290 else (r * r * r + 0x20000) // 0x40000


def vert_break(nodes, h, d):
    least, best = AWFUL_BAD, len(nodes)
    cur, stretch, shrink, prev_dp = 0, [0, 0, 0, 0], 0, 0
    i = 0
    while True:
        upd = False
        pi = None
        if i >= len(nodes):
            pi = -10000
        else:
            n = nodes[i]
            k = n["k"]
            if k in "HVR":
                cur += prev_dp + n["h"]
                prev_dp = n["d"]
            elif k == "G":
                prev = nodes[0] if i == 0 else nodes[i - 1]
                if precedes_break(prev):
                    pi = 0
                else:
                    upd = True
            elif k == "K":
                if i + 1 < len(nodes) and nodes[i + 1]["k"] == "G":
                    pi = 0
                else:
                    upd = True
            elif k == "P":
                pi = n["v"]
        if pi is not None:
            if pi < 10000:
                if cur < h:
                    b = 0 if (stretch[1] or stretch[2] or stretch[3]) else badness(h - cur, stretch[0])
                elif cur - h > shrink:
                    b = AWFUL_BAD
                else:
                    b = badness(cur - h, shrink)
                if b < AWFUL_BAD:
                    b = pi if pi <= -10000 else (b + pi if b < INF_BAD else 100000)
                if b <= least:
                    least, best = b, i
                if b == AWFUL_BAD or pi <= -10000:
                    return best
            if i < len(nodes) and nodes[i]["k"] in "GK":
                upd = True
        if upd:
            n = nodes[i]
            if n["k"] == "G":
                w, st, sto, sh, _ = n["spec"]
                stretch[sto] += st
                shrink += sh
            else:
                w = n["w"]
            cur += prev_dp + w
            prev_dp = 0
        if prev_dp > d:
            cur += prev_dp - d
            prev_dp = d
        i += 1


# ---------------------------------------------------------------- positions


def positions(box):
    """vlist_out positions of box children: (child, y) with y = baseline for
    boxes/rules, top for glue/kern."""
    cur_v, cur_g, cur_glue = 0, 0, 0.0
    out = []
    for c in box["item"].children:
        n = classify(c)
        k = n["k"]
        if k in "HVR":
            cur_v += n["h"]
            out.append((n, cur_v))
            cur_v += n["d"]
        elif k == "G":
            out.append((n, cur_v))
            w, st, sto, sh, sho = n["spec"]
            rule_ht = w - cur_g
            if box["sign"] == 1 and sto == box["order"]:
                cur_glue += st
                cur_g = int(round_half_away(max(-1e9, min(1e9, box["ratio"] * cur_glue))))
            elif box["sign"] == -1 and sho == box["order"]:
                cur_glue -= sh
                cur_g = int(round_half_away(max(-1e9, min(1e9, box["ratio"] * cur_glue))))
            cur_v += rule_ht + cur_g
        elif k == "K":
            out.append((n, cur_v))
            cur_v += n["w"]
        else:
            out.append((n, cur_v))
    return out


def round_half_away(x):
    return int(x + 0.5) if x >= 0 else -int(-x + 0.5)


def find_column(root, colht):
    """Returns (column box, absolute top in sp from the shipped box top)."""
    rootn = classify(root)
    for child, y in positions(rootn):
        if child["k"] != "V":
            continue
        top = y - child["h"]
        for gc, gy in positions(child):
            if gc["k"] == "V" and gc["h"] == colht:
                return gc, top + gy - gc["h"]
    raise ValueError("no column of height %d" % colht)


def column_lines(col):
    pos = positions(col)
    kinds = [n["k"] for n, _ in pos]
    out = []
    if kinds == ["V", "G"] and pos[1][0]["spec"] == (0, 65536, 1, 65536, 1):
        inner, y = pos[0]
        for n, iy in positions(inner):
            if n["k"] in "HV":
                out.append((n, y - inner["h"] + iy))
        return out
    for n, y in pos:
        if n["k"] in "HV":
            out.append((n, y))
    return out


# ---------------------------------------------------------------- logs


def read_log(path):
    with open(path, encoding="latin-1") as f:
        return f.read().split("\n")


def parse_params(lines):
    for line in lines:
        if line.startswith("FTPB-PARAMS"):
            fields = dict(re.findall(r"(\w+)=((?:-?[\d.]+(?:pt|fil+)?)(?: (?:plus|minus) -?[\d.]+(?:pt|fil+)?)*)", line))
            return fields
    raise ValueError("no FTPB-PARAMS in log")


def parse_events(lines):
    events, pending = [], []
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith("%% goal height=") or line.startswith("% t=") or line.startswith("% split"):
            pending.append(line)
        elif line.startswith("FTPB-OUT penalty="):
            events.append({"penalty": int(line.split("=", 1)[1]), "trace": pending, "box": None, "ship": None})
            pending = []
        elif line == "> \\box255=":
            root, i = parse_dump(lines, i + 1)
            events[-1]["box"] = root
            continue
        elif line.startswith("Completed box being shipped out"):
            root, i = parse_dump(lines, i + 1)
            events[-1]["ship"] = root
            continue
        i += 1
    return events, pending


def run_tex(workdir, name, source, huge):
    job = name + ("-capture" if huge else "-real")
    lines = source.split("\n")
    hook = "\\input{%s}" % HOOK
    out = [lines[0], ("\\def\\ftpbhuge{1}" if huge else "") + hook] + lines[1:]
    tex = os.path.join(workdir, job + ".tex")
    with open(tex, "w") as f:
        f.write("\n".join(out))
    env = dict(os.environ, max_print_line="1000000", error_line="254", half_error_line="238")
    for _ in range(1):
        subprocess.run(
            ["pdflatex", "-interaction=batchmode", job + ".tex"],
            cwd=workdir, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False,
        )
    return os.path.join(workdir, job + ".log"), os.path.join(workdir, job + ".pdf")


def children_nodes(root):
    return [classify(c) for c in root.children]


def capture_stream(events):
    stream = []
    skip = 0
    prev_push = []
    for k, ev in enumerate(events):
        if ev["penalty"] > -10000:
            raise ValueError("capture run broke at a non-forced penalty %d (fixture too long?)" % ev["penalty"])
        nodes = children_nodes(ev["box"])
        head = [box_signature(n) for n in nodes[:skip]]
        if head != prev_push:
            raise ValueError("push-back mismatch at output %d: %r vs %r" % (k, head, prev_push))
        stream.extend(n for n in nodes[skip:] if not (n["k"] == "G" and n["top"]))
        stream.append({"k": "P", "v": ev["penalty"]})
        skip, prev_push = 0, []
        if -10002 < ev["penalty"] < -10000:
            q = vert_break(nodes, 0, MAXDIMEN)
            skip = q
            prev_push = [box_signature(n) for n in nodes[:q]]
    return stream


def pdftotext_pages(pdf):
    xml = subprocess.run(["pdftotext", "-bbox-layout", pdf, "-"], capture_output=True, text=True, check=True).stdout
    pages = []
    for page in re.findall(r"<page [^>]*>(.*?)</page>", xml, re.S):
        lines = []
        for ymin, ymax, body in re.findall(r'<line xMin="[\d.]+" yMin="([\d.]+)" xMax="[\d.]+" yMax="([\d.]+)">(.*?)</line>', page, re.S):
            words = re.findall(r"<word [^>]*>(.*?)</word>", body)
            lines.append((float(ymin), float(ymax), " ".join(words)))
        lines.sort()
        # Merge fragments on the same baseline (pdftotext splits displays).
        merged = []
        for l in lines:
            if merged and abs(merged[-1][1] - l[1]) < 0.5:
                merged[-1] = (min(merged[-1][0], l[0]), max(merged[-1][1], l[1]), merged[-1][2] + " " + l[2])
            else:
                merged.append(l)
        pages.append(merged)
    return pages


def generate(name, source, workdir):
    with open(os.path.join(FIXTURES, name + ".tex"), "w") as f:
        f.write(source)
    cap_log, _ = run_tex(workdir, name, source, True)
    real_log, real_pdf = run_tex(workdir, name, source, False)
    cap_lines = read_log(cap_log)
    real_lines = read_log(real_log)
    params = parse_params(real_lines)
    cap_events, _ = parse_events(cap_lines)
    stream = capture_stream(cap_events)
    events, tail = parse_events(real_lines)
    colht = sp(params["colht"])
    pdf_pages = pdftotext_pages(real_pdf)

    out = ["ftpb 1", "name " + name]
    for key in ("vsize", "maxdepth", "colht", "hsize", "splitmaxdepth", "footdimen"):
        out.append("param %s %d" % (key, sp(params[key])))
    for key in ("kludgeins", "footins", "footcount", "ragged"):
        out.append("param %s %d" % (key, int(params[key])))
    for key in ("topskip", "splittopskip", "footskip"):
        out.append("glue %s %s" % (key, spec_str(parse_spec(params[key]))))
    body = []
    for n in stream:
        body.extend(node_lines(n))
    out.append("stream %d" % len(stream))
    out.extend(body)
    residuals = []
    page_no = 0
    for ev in events:
        out.append("event %d" % ev["penalty"])
        for t in ev["trace"]:
            out.append("trace " + t)
        nodes = children_nodes(ev["box"])
        out.append("box %d" % len(nodes))
        out.extend(box_signature(n) for n in nodes)
        if ev["ship"] is not None:
            col, col_top = find_column(ev["ship"], colht)
            lines = column_lines(col)
            out.append("column %d" % len(lines))
            for n, y in lines:
                out.append("line %d %d %d" % (n["h"], n["d"], y))
            # pdftotext cross-check: absolute baseline (bp from page top).
            if page_no < len(pdf_pages):
                ptl = [l for l in pdf_pages[page_no] if l[2].strip()]
                if ptl and re.fullmatch(r"\d+", ptl[-1][2].strip()):
                    ptl = ptl[:-1]
                expected_bp = [(72.27 * 65536 + col_top + y) / 65536 * 72 / 72.27 for n, y in lines if n["h"] > 0]
                for ymin, ymax, text in ptl:
                    est = ymax - (ymax - ymin) * 194.0 / 888.0
                    if expected_bp:
                        near = min(expected_bp, key=lambda e: abs(e - est))
                        residuals.append((est - near) * 72.27 / 72)
                if ptl:
                    out.append("text first %s" % ptl[0][2][:60])
                    out.append("text last %s" % ptl[-1][2][:60])
            page_no += 1
    for t in tail:
        out.append("tailtrace " + t)
    good = [abs(r) for r in residuals if abs(r) < 1.0]
    out.insert(2, "# pdftotext baseline check: %d/%d lines within 1pt, median |dev| %.3fpt, max %.3fpt" % (
        len(good), len(residuals), statistics.median(good) if good else 0.0, max(good) if good else 0.0))
    out.append("end")
    with open(os.path.join(EXPECTED, name + ".ftpb"), "w") as f:
        f.write("\n".join(out) + "\n")
    shipped = sum(1 for e in events if e["ship"] is not None)
    return "%s: %d nodes, %d outputs, %d pages, pdftotext %d/%d within 1pt (max %.3fpt)" % (
        name, len(stream), len(events), shipped, len(good), len(residuals), max(good) if good else 0.0)


# ---------------------------------------------------------------- fixtures

WORDS = (
    "the of and to in is that for it as was with be by on not he this are or his from at which but have an they "
    "you were her she there one all we their has been if more when will would who so no page break builder "
    "vertical glue penalty paragraph insertion typography baseline measure between remaining document "
    "consideration extraordinary representation characteristically incomprehensibility nevertheless "
    "throughout mathematics algorithm fidelity composition discretionary hyphenation approximately "
    "especially interesting particular structure arrangement environment independently simultaneously "
    "a an I it we us me go do so up at by of on or"
).split()


def words(rng, n):
    w = [rng.choice(WORDS) for _ in range(n)]
    w[0] = w[0].capitalize()
    return " ".join(w) + "."


def paragraph(rng, lo=30, hi=140):
    return words(rng, rng.randint(lo, hi))


def document(options="", preamble="", body=""):
    cls = "\\documentclass%s{article}" % ("[%s]" % options if options else "")
    return "%s\n%s\n\\begin{document}\n%s\n\\end{document}\n" % (cls, preamble, body)


def plain_body(rng, paras, lo=30, hi=140):
    return "\n\n".join(paragraph(rng, lo, hi) for _ in range(paras))


def headings_body(rng, paras, density=0.35, sub=True):
    out = []
    n = 0
    for i in range(paras):
        if rng.random() < density:
            n += 1
            kind = rng.choice(["section", "subsection"] if sub else ["section"])
            out.append("\\%s{%s}" % (kind, words(rng, rng.randint(1, 4))[:-1]))
        out.append(paragraph(rng, 10, 90))
    return "\n\n".join(out)


def display_body(rng, paras, env="bracket"):
    out = []
    for i in range(paras):
        p = paragraph(rng, 15, 80)
        r = rng.random()
        if r < 0.45:
            eq = "a_{%d} + b^{2} = c + \\sum_{i=1}^{n} x_i" % i
            if env == "equation" or (env == "mixed" and rng.random() < 0.5):
                p += "\n\\begin{equation}\n%s\n\\end{equation}\n%s" % (eq, words(rng, rng.randint(3, 40)))
            else:
                p += "\n\\[ %s \\]\n%s" % (eq, words(rng, rng.randint(3, 40)))
        out.append(p)
    return "\n\n".join(out)


def list_body(rng, paras):
    out = []
    for i in range(paras):
        out.append(paragraph(rng, 20, 80))
        if rng.random() < 0.5:
            env = rng.choice(["itemize", "enumerate", "description"])
            items = []
            for j in range(rng.randint(2, 7)):
                label = "[%s]" % words(rng, 1)[:-1] if env == "description" else ""
                it = "\\item%s %s" % (label, words(rng, rng.randint(5, 60)))
                if rng.random() < 0.15:
                    it += "\n\\begin{itemize}\n\\item %s\n\\item %s\n\\end{itemize}" % (words(rng, 12), words(rng, 20))
                items.append(it)
            out.append("\\begin{%s}\n%s\n\\end{%s}" % (env, "\n".join(items), env))
    return "\n\n".join(out)


def fixtures():
    fx = {}

    def add(name, src):
        assert name not in fx
        fx[name] = src

    # A: plain paragraphs, natural club/widow decisions.
    for i, opts in enumerate(["", "11pt", "12pt", "a4paper", "", "10pt,a4paper", "12pt,a4paper", "11pt"]):
        rng = random.Random(100 + i)
        add("plain-%02d" % i, document(opts, "", plain_body(rng, 18 + i * 2)))
    # B: widow/club parameter variants.
    variants = [
        ("clubwidow-inf", "\\clubpenalty=10000 \\widowpenalty=10000"),
        ("clubwidow-zero", "\\clubpenalty=0 \\widowpenalty=0"),
        ("widow-5000", "\\widowpenalty=5000"),
        ("club-3000", "\\clubpenalty=3000"),
        ("interline-100", "\\interlinepenalty=100"),
        ("broken-inf", "\\brokenpenalty=10000 \\clubpenalty=500"),
    ]
    for i, (name, pre) in enumerate(variants):
        rng = random.Random(200 + i)
        add(name, document("", "\\AtBeginDocument{%s}" % pre, plain_body(rng, 26, 20, 110)))
    # C: headings.
    for i in range(6):
        rng = random.Random(300 + i)
        add("headings-%02d" % i, document(["", "11pt", "", "12pt", "a4paper", ""][i], "", headings_body(rng, 30 + 3 * i, 0.3 + 0.05 * i)))
    # D: displays.
    for i, (env, pre) in enumerate([
        ("bracket", ""), ("equation", ""), ("mixed", ""), ("bracket", "\\AtBeginDocument{\\predisplaypenalty=0}"),
        ("mixed", "\\AtBeginDocument{\\displaywidowpenalty=3000 \\postdisplaypenalty=200}"), ("equation", "\\usepackage{amsmath}"),
    ]):
        rng = random.Random(400 + i)
        add("display-%02d" % i, document("" if i % 2 == 0 else "11pt", pre, display_body(rng, 34, env)))
    # E: raggedbottom vs flushbottom.
    for i, (opts, pre) in enumerate([("", "\\flushbottom"), ("twoside", ""), ("twoside", "\\raggedbottom"), ("", "\\flushbottom")]):
        rng = random.Random(500 + i)
        body = headings_body(rng, 34, 0.3) if i % 2 else display_body(rng, 34, "mixed")
        add("bottom-%02d" % i, document(opts, pre, body))
    # F: \topskip with tall first lines.
    for i in range(4):
        rng = random.Random(600 + i)
        paras = []
        for j in range(30):
            p = paragraph(rng, 20, 100)
            if rng.random() < 0.4:
                p = "\\rule{0pt}{%dpt}%s" % (rng.choice([12, 15, 18, 24]), p)
            if rng.random() < 0.15:
                p = "{\\Large %s\\par}" % p
            paras.append(p)
        pre = ["\\setlength\\topskip{20pt}", "", "\\setlength\\topskip{12pt plus 2pt}", "\\setlength\\topskip{30pt}"][i]
        add("topskip-%02d" % i, document("", pre, "\n\n".join(paras)))
    # G: \parskip stretch.
    for i, ps in enumerate(["6pt plus 2pt minus 1pt", "0pt plus 10pt", "12pt", "3pt plus 1fil"]):
        rng = random.Random(700 + i)
        add("parskip-%02d" % i, document("", "\\setlength\\parskip{%s}" % ps, headings_body(rng, 36, 0.2)))
    # H: \enlargethispage.
    for i, cmd in enumerate([
        "\\enlargethispage{2\\baselineskip}", "\\enlargethispage*{2\\baselineskip}", "\\enlargethispage{-3\\baselineskip}",
        "\\enlargethispage*{0pt}", "\\enlargethispage{1in}",
    ]):
        rng = random.Random(800 + i)
        paras = [paragraph(rng, 20, 100) for _ in range(28)]
        for j in rng.sample(range(2, 26), 2):
            paras[j] = cmd + "\n" + paras[j] if i % 2 == 0 else paras[j].replace(" ", " %s " % cmd, 1)
        add("enlarge-%02d" % i, document("", "", "\n\n".join(paras)))
    # I: explicit breaks.
    for i in range(6):
        rng = random.Random(900 + i)
        paras = [paragraph(rng, 15, 90) for _ in range(30)]
        cmds = [
            ["\\pagebreak[%d]" % n for n in range(5)],
            ["\\nopagebreak[%d]" % n for n in range(5)],
            ["\\newpage", "\\pagebreak", "\\clearpage"],
            ["\\pagebreak[2]", "\\nopagebreak", "\\linebreak[1]"],
            ["\\samepage"],
            ["\\clearpage", "\\newpage\\mbox{}\\newpage"],
        ][i]
        for j in rng.sample(range(1, 29), 8):
            c = rng.choice(cmds)
            if c == "\\samepage":
                paras[j] = "{\\samepage %s\n\n%s\\par}" % (paras[j], paragraph(rng, 20, 40))
            elif rng.random() < 0.5:
                paras[j] = paras[j] + "\n" + c
            else:
                ws = paras[j].split(" ")
                k = rng.randint(1, len(ws) - 1)
                paras[j] = " ".join(ws[:k] + [c] + ws[k:])
        add("breaks-%02d" % i, document("", "", "\n\n".join(paras)))
    # J: lists.
    for i in range(4):
        rng = random.Random(1000 + i)
        add("lists-%02d" % i, document(["", "11pt", "12pt", ""][i], "", list_body(rng, 30)))
    # K: vertical spacing commands.
    for i in range(4):
        rng = random.Random(1100 + i)
        paras = []
        for j in range(30):
            paras.append(paragraph(rng, 15, 90))
            r = rng.random()
            if r < 0.25:
                paras.append(rng.choice(["\\bigskip", "\\medskip", "\\smallskip", "\\vspace{1cm}", "\\vspace*{8pt}", "\\vspace{-4pt}"]))
            elif r < 0.3:
                paras.append("\\vfill")
        pre = ["", "\\linespread{1.3}", "\\setlength\\maxdepth{1pt}", "\\AtBeginDocument{\\setlength\\maxdepth{10pt}}"][i]
        add("vspace-%02d" % i, document("", pre, "\n\n".join(paras)))
    # L: footnotes (insertion interface).
    for i in range(4):
        rng = random.Random(1200 + i)
        paras = []
        for j in range(28):
            p = paragraph(rng, 20, 100)
            for _ in range(rng.randint(0, 2 if i < 3 else 1)):
                ws = p.split(" ")
                k = rng.randint(1, len(ws) - 1)
                note = words(rng, rng.randint(5, 30 if i < 3 else 180))
                ws[k] = ws[k] + "\\footnote{%s}" % note
                p = " ".join(ws)
            paras.append(p)
        add("footnotes-%02d" % i, document("", "", "\n\n".join(paras)))
    return fx


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--workdir")
    args = ap.parse_args()
    os.makedirs(FIXTURES, exist_ok=True)
    os.makedirs(EXPECTED, exist_ok=True)
    workdir = args.workdir or tempfile.mkdtemp(prefix="ftpb-oracle-")
    os.makedirs(workdir, exist_ok=True)
    fx = fixtures()
    names = args.only or sorted(fx)
    failures = 0
    for name in names:
        try:
            print(generate(name, fx[name], workdir), flush=True)
        except Exception as e:  # noqa: BLE001 - report and continue
            failures += 1
            print("%s: FAILED %s" % (name, e), flush=True)
    if not args.workdir:
        shutil.rmtree(workdir, ignore_errors=True)
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
