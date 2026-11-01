#!/usr/bin/env python3
r"""Compare flashtex-math-layout output with a pdfTeX PDF of docs/oracle/compare.tex.

ORACLE TOOLING ONLY: this script reads a PDF that a reference pdflatex produced
in a scratch directory. Nothing here runs in the product. It is the evidence
generator for docs/comparison.md.

Reference geometry comes from two independent sources:
  1. the uncompressed page content stream (`\pdfcompresslevel=0`): text
     operators give every glyph's exact origin (font, code, x, baseline) once
     advances from the PDF's own /Widths arrays are applied; stroked `m … l S`
     segments give every rule's centre line, length and thickness;
  2. `tools/oracle_glyphs.swift` (Apple PDFKit) per-character selection boxes,
     used to cross-check the x positions and advance widths from (1).

Usage:
  cargo run -q --example emit_runs > runs.json
  python3 tools/oracle_compare.py compare.pdf runs.json [pdfkit.json] > report.md
"""
import json
import re
import sys

BP_PER_TEX_PT = 72.0 / 72.27


def load_pdf(path):
    data = open(path, "rb").read()
    objs = {}
    for m in re.finditer(rb"(\d+) 0 obj\s*(.*?)\s*endobj", data, re.S):
        objs[int(m.group(1))] = m.group(2)
    return data, objs


def fonts(objs):
    """resource name -> (base font name, first char, widths in 1/1000 em)."""
    res = next(v for v in objs.values() if b"/Font <<" in v and b"/Type /Page" not in v)
    table = {}
    for name, num in re.findall(rb"/(F\d+) (\d+) 0 R", res):
        fobj = objs[int(num)]
        base = re.search(rb"/BaseFont /(?:[A-Z]{6}\+)?(\w+)", fobj).group(1).decode()
        first = int(re.search(rb"/FirstChar (\d+)", fobj).group(1))
        wref = re.search(rb"/Widths (\d+) 0 R", fobj)
        wsrc = objs[int(wref.group(1))] if wref else re.search(rb"/Widths (\[.*?\])", fobj, re.S).group(1)
        widths = [float(x) for x in re.findall(rb"[-\d.]+", wsrc)]
        table[name.decode()] = (base.lower(), first, widths)
    return table


def pdf_string_bytes(s):
    out = bytearray()
    i = 0
    while i < len(s):
        c = s[i]
        if c == 0x5C:  # backslash
            i += 1
            if s[i:i + 1].isdigit():
                j = i
                while j < len(s) and j < i + 3 and s[j:j + 1].isdigit():
                    j += 1
                out.append(int(s[i:j], 8))
                i = j
                continue
            esc = {b"n": 10, b"r": 13, b"t": 9, b"b": 8, b"f": 12, b"(": 40, b")": 41, b"\\": 92}
            out.append(esc.get(s[i:i + 1], s[i]))
            i += 1
        else:
            out.append(c)
            i += 1
    return bytes(out)


def parse_content(stream, font_table):
    """Returns (glyphs, rules, labels) in PDF bp with a bottom-left origin."""
    glyphs, rules = [], []
    tokens = re.findall(rb"\[(?:[^\]]*)\]|\((?:\\.|[^\\)])*\)|/[A-Za-z0-9]+|[-+.\d]+|[A-Za-z*']+", stream)
    lx = ly = 0.0
    cx = cy = 0.0
    font = None
    size = 0.0
    stack = []
    cm = (0.0, 0.0)
    pending_line = None
    for t in tokens:
        if t.startswith(b"["):
            # TJ array
            parts = re.findall(rb"\((?:\\.|[^\\)])*\)|[-+.\d]+", t[1:-1])
            for p in parts:
                if p.startswith(b"("):
                    for code in pdf_string_bytes(p[1:-1]):
                        base, first, widths = font_table[font]
                        w = widths[code - first] / 1000.0 * size
                        glyphs.append({"font": base, "code": code, "x": cx, "y": cy, "size": size, "adv": w})
                        cx += w
                else:
                    cx -= float(p) / 1000.0 * size
            continue
        if t.startswith(b"/"):
            stack.append(t[1:].decode())
            continue
        if re.fullmatch(rb"[-+.\d]+", t):
            stack.append(float(t))
            continue
        op = t.decode()
        if op == "Tf":
            font, size = stack[-2], stack[-1]
        elif op == "Td":
            lx += stack[-2]
            ly += stack[-1]
            cx, cy = lx, ly
        elif op == "Tm":
            lx, ly = stack[-2], stack[-1]
            cx, cy = lx, ly
        elif op == "BT":
            lx = ly = cx = cy = 0.0
        elif op == "cm":
            cm = (stack[-2], stack[-1])
        elif op == "w":
            pending_line = {"w": stack[-1]}
        elif op == "l":
            if pending_line is not None:
                pending_line["len"] = stack[-2]
        elif op == "S":
            if pending_line is not None and "len" in pending_line:
                rules.append({"x": cm[0], "y_center": cm[1], "w": pending_line["len"], "h": pending_line["w"]})
            pending_line = None
        stack = [] if op not in ("Tf",) else stack
    return glyphs, rules


def split_formulas(glyphs, rules):
    """Formula k = glyphs after the k-th 'Formula X:' label, up to the next."""
    # Labels are cmr10 runs spelling F o r m u l a ... :
    label_idx = [i for i, g in enumerate(glyphs) if g["font"] == "cmr10" and g["code"] == ord("F")]
    chunks = []
    for k, start in enumerate(label_idx):
        end = label_idx[k + 1] if k + 1 < len(label_idx) else len(glyphs)
        # skip the label itself: "Formula A:" is 9 glyphs (no space glyph)
        chunks.append(glyphs[start + 9:end])
    # A rule belongs to the formula whose glyph baselines are nearest to it.
    centers = [sum(g["y"] for g in c) / len(c) for c in chunks]
    out = [(c, []) for c in chunks]
    for r in rules:
        k = min(range(len(chunks)), key=lambda k: abs(centers[k] - r["y_center"]))
        out[k][1].append(r)
    return out


def main():
    pdf_path, runs_path = sys.argv[1], sys.argv[2]
    pdfkit = json.load(open(sys.argv[3])) if len(sys.argv) > 3 else None
    data, objs = load_pdf(pdf_path)
    table = fonts(objs)
    stream = re.search(rb"stream\r?\n(.*?)\r?\nendstream", data, re.S).group(1)
    glyphs, rules = parse_content(stream, table)
    ref_formulas = split_formulas(glyphs, rules)
    ours = json.load(open(runs_path))
    lines = []
    overall = 0.0
    for (name_ours), (ref_glyphs, ref_rules) in zip(ours, ref_formulas):
        name = name_ours["name"]
        # Our runs: TeX pt, y down. Convert to bp and align on the first glyph.
        og = name_ours["glyphs"]
        first = og[0]
        # find the matching reference glyph (same font/code, first occurrence)
        ref_first = next(g for g in ref_glyphs if g["font"] == first["font"] and g["code"] == first["gid"])
        ax, ay = ref_first["x"], ref_first["y"]
        used = set()
        lines.append(f"\n### Formula {name}\n")
        lines.append("| glyph | font | ours x | ref x | Δx | ours baseline | ref baseline | Δy |")
        lines.append("|---|---|---|---|---|---|---|---|")
        worst = 0.0
        for g in og:
            ox = (g["x"] - first["x"]) * BP_PER_TEX_PT
            oy = -(g["baseline"] - first["baseline"]) * BP_PER_TEX_PT
            cands = [(i, r) for i, r in enumerate(ref_glyphs)
                     if i not in used and r["font"] == g["font"] and r["code"] == g["gid"]]
            if not cands:
                lines.append(f"| {g['ch']} | {g['font']} | {ox:.3f} | — | missing | {oy:.3f} | — | missing |")
                continue
            i, r = min(cands, key=lambda c: abs(c[1]["x"] - ax - ox) + abs(c[1]["y"] - ay - oy))
            used.add(i)
            rx, ry = r["x"] - ax, r["y"] - ay
            dx, dy = ox - rx, oy - ry
            worst = max(worst, abs(dx), abs(dy))
            lines.append(f"| {g['ch']} | {g['font']} gid {g['gid']} | {ox:.3f} | {rx:.3f} | {dx:+.3f} | {oy:.3f} | {ry:.3f} | {dy:+.3f} |")
        unmatched = [r for i, r in enumerate(ref_glyphs) if i not in used]
        if unmatched:
            lines.append(f"\nReference glyphs not produced by ours: {[(r['font'], r['code']) for r in unmatched]}")
        lines.append("")
        lines.append("| rule | ours x | ref x | Δx | ours centre y | ref centre y | Δy | ours w | ref w | Δw | ours h | ref h | Δh |")
        lines.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
        ref_rules_sorted = sorted(ref_rules, key=lambda r: (-r["y_center"], r["x"]))
        our_rules = sorted(name_ours["rules"], key=lambda r: (r["y"], r["x"]))
        for k, (o, r) in enumerate(zip(our_rules, ref_rules_sorted)):
            ox = (o["x"] - first["x"]) * BP_PER_TEX_PT
            oyc = -((o["y"] + o["h"] / 2.0) - first["baseline"]) * BP_PER_TEX_PT
            ow, oh = o["w"] * BP_PER_TEX_PT, o["h"] * BP_PER_TEX_PT
            rx, ryc = r["x"] - ax, r["y_center"] - ay
            d = (ox - rx, oyc - ryc, ow - r["w"], oh - r["h"])
            worst = max(worst, *[abs(v) for v in d])
            lines.append(f"| {k} | {ox:.3f} | {rx:.3f} | {d[0]:+.3f} | {oyc:.3f} | {ryc:.3f} | {d[1]:+.3f} | {ow:.3f} | {r['w']:.3f} | {d[2]:+.3f} | {oh:.3f} | {r['h']:.3f} | {d[3]:+.3f} |")
        if len(our_rules) != len(ref_rules_sorted):
            lines.append(f"\nRule count differs: ours {len(our_rules)}, reference {len(ref_rules_sorted)}")
        lines.append(f"\nLargest absolute deviation in formula {name}: {worst:.3f} bp; limitations: {name_ours['limitations'] or 'none'}")
        overall = max(overall, worst)
        if pdfkit:
            # Cross-check: PDFKit character boxes (top-left origin, bp) vs the
            # content-stream x positions for this formula's glyphs.
            page = pdfkit["pages"][0]
            lines.append("\nPDFKit cross-check (content-stream x vs PDFKit box left edge, same page):")
            lines.append("| ref glyph | stream x | PDFKit left | Δ | PDFKit width | stream advance | Δ |")
            lines.append("|---|---|---|---|---|---|---|")
            for r in ref_glyphs:
                near = [c for c in page["chars"] if abs(c["x_pt"] - r["x"]) < 0.6 and abs((page["height_pt"] - c["bottom_pt"]) - r["y"]) < 12]
                if not near:
                    continue
                c = min(near, key=lambda c: abs(c["x_pt"] - r["x"]))
                lines.append(f"| {r['font']} {r['code']} | {r['x']:.3f} | {c['x_pt']:.3f} | {c['x_pt'] - r['x']:+.3f} | {c['right_pt'] - c['x_pt']:.3f} | {r['adv']:.3f} | {(c['right_pt'] - c['x_pt']) - r['adv']:+.3f} |")
    print(f"Largest absolute deviation over all formulas: {overall:.3f} bp")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
