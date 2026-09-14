#!/usr/bin/env python3
"""Tabular oracle corpus: pdflatex word and rule positions vs flashtex-render.

Oracle tooling only; pdflatex never runs in the product path.

    # regenerate the pinned reference positions (needs TeX Live; oracle only)
    python3 oracle.py refs --texbin /Library/TeX/texbin
    # measure a flashtex-render build against the pinned references
    python3 oracle.py check --render path/to/flashtex-render --fonts apps/mac/Fonts

Words are formed and aligned exactly as in ../amsmath_corpus/oracle.py
(glyph origins, a gap wider than 0.16 em or a new baseline starts a word;
tools/visual-oracle/rank.py:align_words). Rules are the filled rectangles
of the page: on the reference side every `re` path painted by `f`/`F`/`f*`
and every single horizontal or vertical `m`/`l` segment stroked by `S` at
line width `w` (how pdfTeX writes `\\hrule`/`\\vrule`/leaders), under the
current `cm`; on the candidate side every rendering-v2
`rule` item. pdfTeX paints a `|` column rule once per row while a producer
may paint one rule over the table; so on both sides rules that continue one
another (same x and width within 0.01 bp, touching vertically, or same top
and height touching horizontally) are merged before comparison.

A fixture passes when both sides have one page, every word aligns and lies
within 0.5 bp of its reference origin in x and y, math extension glyphs (cmex:
big operators and delimiters, whose painted origin legitimately differs) have
the same sorted distinct x origins within 0.5 bp, the merged rule counts are
equal, and every reference rule is matched by a distinct candidate rule whose
x, top, width and height are all within 0.1 bp.
"""
import argparse, json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "tools", "visual-oracle"))
sys.path.insert(0, os.path.join(REPO, "tools", "real-world-corpus"))
import fontenv  # noqa: E402
import pdftext  # noqa: E402
import rank  # noqa: E402

FIXTURES = os.path.join(HERE, "fixtures")
REFS = os.path.join(HERE, "refs")
TOL = 0.5
RULE_TOL = 0.1
Q = float(2 ** 20)


def regroup(glyphs):
    glyphs = sorted((dict(g, bt=0) for g in glyphs), key=lambda g: (round(g["y_top"], 1), g["x"]))
    return [{"text": w["text"], "x": round(w["x"], 4), "y_top": round(w["y_top"], 4)}
            for w in pdftext.words_from_glyphs(glyphs)]


# cmex10 at its design size, in bp, and the characters FlashTeX paints from
# the math extension role (compared as columns, ../amsmath_corpus/oracle.py).
EXT_SIZE_BP = 10 * 72 / 72.27
EXT_TEXT = set("()[]{}|‖⟨⟩⌊⌋⌈⌉/\\∑∏∐∫∮⋃⋂⨁⨂⨀⨄⨆√")
# OMS (cmsy) and OML (cmmi Greek) codes whose glyph names pdftext leaves as "?".
OMS_TEXT = dict(enumerate(
    "−·×∗÷⋄±∓⊕⊖⊗⊘⊙◯∘∙≍≡⊆⊇≤≥⪯⪰∼≈⊂⊃≪≫≺≻←→↑↓↔↗↘≃⇐⇒⇑⇓⇔↖↙∝′∞∈∋△▽/↦∀∃¬∅ℜℑ⊤⊥ℵ"
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ∪∩⊎∧∨⊢⊣⌊⌋⌈⌉{}⟨⟩|‖↕⇕\\≀√⨿∇∫⊔⊓⊑⊒§†‡¶♣♢♡♠"))
OML_TEXT = dict(enumerate("ΓΔΘΛΞΠΣΥΦΨΩαβγδϵζηθικλμνξπρστυϕχψωεϑϖϱςφ"))


def columns(glyphs):
    """Sorted distinct x origins (0.05 bp) of extension glyphs."""
    xs = []
    for x in sorted(g["x"] for g in glyphs):
        if not xs or x - xs[-1] > 0.05:
            xs.append(x)
    return [round(x, 4) for x in xs]


def ref_words(glyphs):
    words, ext = [], []
    for g in glyphs:
        font = g["font"].upper()
        if "MATHEXTENSION" in font or font.startswith("CMEX") or font.startswith("LMEX"):
            ext.append(g)
            continue
        if g["text"] == "?":
            if "MATHSYMBOLS" in font or font.startswith("CMSY") or font.startswith("LMSY"):
                g = dict(g, text=OMS_TEXT.get(g["code"], "?"))
            elif "MATHITALIC" in font or font.startswith("CMMI") or font.startswith("LMMI"):
                g = dict(g, text=OML_TEXT.get(g["code"], "?"))
        words.append(g)
    return regroup(words), columns(ext)


def merge_rules(rules):
    """Merges rules that continue one another (see module docstring)."""
    rules = [list(r) for r in rules]
    changed = True
    while changed:
        changed = False
        rules.sort(key=lambda r: (round(r[0], 2), round(r[1], 2)))
        for i in range(len(rules)):
            for j in range(len(rules)):
                if i == j:
                    continue
                a, b = rules[i], rules[j]
                vertical = abs(a[0] - b[0]) < 0.01 and abs(a[2] - b[2]) < 0.01 and abs(a[1] + a[3] - b[1]) < 0.01
                horizontal = abs(a[1] - b[1]) < 0.01 and abs(a[3] - b[3]) < 0.01 and abs(a[0] + a[2] - b[0]) < 0.01
                if vertical:
                    a[3] = b[1] + b[3] - a[1]
                elif horizontal:
                    a[2] = b[0] + b[2] - a[0]
                else:
                    continue
                del rules[j]
                changed = True
                break
            if changed:
                break
    return [[round(v, 4) for v in r] for r in sorted(rules, key=lambda r: (round(r[1], 2), round(r[0], 2)))]


def ref_rules(doc, page):
    """Filled rectangles (x, top, width, height) in bp from the page top."""
    media = [float(doc.resolve(v)) for v in doc.resolve(page.get("MediaBox", [0, 0, 612, 792]))]
    height = media[3] - media[1]
    contents = page.get("Contents")
    resolved = doc.resolve(contents) if not isinstance(contents, list) else contents
    if isinstance(resolved, list):
        data = b"\n".join(doc.stream_of(c) for c in resolved)
    else:
        data = doc.stream_of(contents)
    lx = pdftext._Lexer(data)
    stack, gs, ctm, path, out = [], [], (1, 0, 0, 1, 0, 0), [], []
    segment, line_width = [], 1.0
    in_text = False
    while True:
        tok = lx.token()
        if tok is None:
            break
        obj = lx.object(tok)
        if not (isinstance(obj, tuple) and obj and obj[0] == "op"):
            stack.append(obj)
            continue
        op = obj[1]
        try:
            if op == b"BT":
                in_text = True
            elif op == b"ET":
                in_text = False
            elif op == b"q":
                gs.append(ctm)
            elif op == b"Q":
                ctm = gs.pop() if gs else ctm
            elif op == b"cm":
                ctm = pdftext._mul(tuple(float(v) for v in stack[-6:]), ctm)
            elif op == b"re" and not in_text:
                x, y, w, h = (float(v) for v in stack[-4:])
                a, _, _, d, e, f = ctm
                x0, y0 = a * x + e, d * y + f
                x1, y1 = a * (x + w) + e, d * (y + h) + f
                path.append((min(x0, x1), height - max(y0, y1), abs(x1 - x0), abs(y1 - y0)))
            elif op in (b"f", b"F", b"f*", b"B", b"B*"):
                out.extend(path)
                path = []
                segment = []
            elif op == b"w":
                line_width = float(stack[-1])
            elif op in (b"m", b"l") and not in_text:
                x, y = float(stack[-2]), float(stack[-1])
                a, _, _, d, e, f = ctm
                segment.append((a * x + e, d * y + f))
            elif op == b"S":
                # pdfTeX strokes `\hrule`/`\vrule` as one `m`/`l` segment
                # of line width equal to the rule thickness.
                if len(segment) == 2:
                    (x0, y0), (x1, y1) = segment
                    w = line_width * abs(ctm[0])
                    if abs(y1 - y0) < 1e-6:
                        out.append((min(x0, x1), height - (y0 + w / 2), abs(x1 - x0), w))
                    elif abs(x1 - x0) < 1e-6:
                        out.append((x0 - w / 2, height - max(y0, y1), w, abs(y1 - y0)))
                path, segment = [], []
            elif op in (b"n", b"s"):
                path, segment = [], []
        except (IndexError, TypeError, ValueError):
            pass
        stack = []
    return merge_rules(out)


def ref_pages(pdf):
    doc = pdftext.PdfDocument.load(pdf)
    return [ref_words(pdftext.page_glyphs(doc, page)[0]) + (ref_rules(doc, page),) for page in doc.pages()]


def cand_pages(v2path):
    pl = json.load(open(v2path, encoding="utf-8"))["payload"]
    pages = []
    for page in pl["pages"]:
        glyphs, ext, rules = [], [], []
        for item in page.get("items", []):
            if item.get("kind") == "rule":
                rules.append((item["x"] / Q, item["top"] / Q, item["width"] / Q, item["height"] / Q))
                continue
            if item.get("kind") != "glyph_run":
                continue
            text = (item.get("text") or "").encode("utf-8")
            clusters = item.get("clusters") or []
            size = item.get("font_size", 0) / Q
            for gi, g in enumerate(item.get("glyphs") or []):
                ci = g.get("cluster", gi)
                ct = (text[clusters[ci]["text_start_byte"]:clusters[ci]["text_end_byte"]].decode("utf-8", "replace")
                      if ci < len(clusters) else "?")
                glyph = {"text": ct, "x": g["origin_x"] / Q, "y_top": g["baseline_y"] / Q,
                         "advance": g["advance_x"] / Q, "size": size, "font": ""}
                (ext if abs(size - EXT_SIZE_BP) < 0.01 and ct in EXT_TEXT else glyphs).append(glyph)
        pages.append((regroup(glyphs), columns(ext), merge_rules(rules)))
    return pages


def match_rules(ref, cand):
    """(matched within tolerance, worst delta) pairing each reference rule
    with the nearest unused candidate rule."""
    used, ok, worst = set(), 0, 0.0
    for r in ref:
        best, bj = None, None
        for j, c in enumerate(cand):
            if j in used:
                continue
            d = max(abs(r[k] - c[k]) for k in range(4))
            if best is None or d < best:
                best, bj = d, j
        if bj is None:
            continue
        used.add(bj)
        worst = max(worst, best)
        ok += best <= RULE_TOL
    return ok, worst


def fixtures(only):
    names = sorted(f[:-4] for f in os.listdir(FIXTURES) if f.endswith(".tex"))
    return [n for n in names if not only or any(o in n for o in only)]


def cmd_refs(args):
    os.makedirs(REFS, exist_ok=True)
    pdflatex = os.path.join(args.texbin, "pdflatex")
    version = subprocess.run([pdflatex, "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    with tempfile.TemporaryDirectory() as work:
        for name in fixtures(args.only):
            src = open(os.path.join(FIXTURES, name + ".tex"), encoding="utf-8").read()
            with open(os.path.join(work, name + ".tex"), "w", encoding="utf-8") as f:
                f.write(src)
            for _ in range(2):
                subprocess.run([pdflatex, "-interaction=batchmode", name + ".tex"], cwd=work, env=env,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            pages = ref_pages(os.path.join(work, name + ".pdf"))
            with open(os.path.join(REFS, name + ".json"), "w", encoding="utf-8") as f:
                json.dump({"fixture": name + ".tex", "reference_engine": version,
                           "invocation": "pdflatex -interaction=batchmode, two passes, SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1",
                           "unit": "bp, y from page top; rules [x, top, width, height]",
                           "pages": [words for words, _, _ in pages],
                           "extension_columns": [cols for _, cols, _ in pages],
                           "rules": [rules for _, _, rules in pages]}, f, indent=1)
                f.write("\n")
            print(f"{name}: {len(pages)} page(s), {sum(len(w) for w, _, _ in pages)} words, "
                  f"{sum(len(c) for _, c, _ in pages)} extension columns, {sum(len(r) for _, _, r in pages)} rules")


def cmd_check(args):
    passed, rows = 0, []
    env = fontenv.render_env(args.fonts, args.tfm_dirs)
    print(fontenv.describe(env))
    with tempfile.TemporaryDirectory() as work:
        for name in fixtures(args.only):
            pinned = json.load(open(os.path.join(REFS, name + ".json"), encoding="utf-8"))
            ref = pinned["pages"]
            ref_r = pinned.get("rules") or [[] for _ in ref]
            ref_c = pinned.get("extension_columns") or [[] for _ in ref]
            text = open(os.path.join(FIXTURES, name + ".tex"), encoding="utf-8").read()
            req = {"protocol_version": 1, "id": name, "type": "compile",
                   "payload": {"project_id": "tabular-corpus", "revision": 1, "entry_path": "main.tex", "date": "1970-01-01",
                               "documents": [{"path": "main.tex", "text": text}]}}
            v2 = os.path.join(work, name + ".v2.json")
            p = subprocess.run([args.render, "--v2", v2], input=(json.dumps(req) + "\n").encode(), env=env,
                               capture_output=True, timeout=120)
            diags, font_bad = [], []
            for line in p.stdout.decode("utf-8", "replace").splitlines():
                try:
                    m = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if m.get("type") == "compile_result":
                    all_diags = m["payload"].get("diagnostics", [])
                    font_bad = fontenv.font_diagnostics(all_diags)
                    diags = [d for d in all_diags
                             if d.get("severity") == "error" or d.get("code") in ("unsupported_block", "table_limitation")]
            cand = cand_pages(v2) if os.path.isfile(v2) else []
            n = ok = unaligned = 0
            rn = rok = 0
            rules_equal = True
            worst = rworst = 0.0
            cols_ok = True
            for rw, rc, rr, (cw, cc, cr) in zip(ref, ref_c, ref_r, cand):
                cols_ok = cols_ok and len(rc) == len(cc) and all(abs(a - b) <= TOL for a, b in zip(rc, cc))
                for a, b in zip(rc, cc):
                    worst = max(worst, abs(a - b))
                idx, ur, uc = rank.align_words(rw, cw)
                unaligned += ur + uc
                for i, j in idx:
                    d = max(abs(cw[j]["x"] - rw[i]["x"]), abs(cw[j]["y_top"] - rw[i]["y_top"]))
                    worst, n, ok = max(worst, d), n + 1, ok + (d <= TOL)
                rules_equal = rules_equal and len(rr) == len(cr)
                m_ok, m_worst = match_rules(rr, cr)
                rn, rok, rworst = rn + len(rr), rok + m_ok, max(rworst, m_worst)
            ncand_rules = sum(len(r) for _, _, r in cand)
            good = (len(ref) == len(cand) == 1 and n > 0 and ok == n and unaligned == 0 and cols_ok
                    and rules_equal and rok == rn and not font_bad)
            if font_bad:
                fontenv.report_font_failure(name, font_bad, env)
            passed += good
            row = {"fixture": name, "pass": good, "pages": [len(ref), len(cand)], "aligned": n, "within_tol": ok,
                   "unaligned": unaligned, "extension_columns_match": cols_ok, "worst_bp": round(worst, 3),
                   "rules": [rn, ncand_rules], "rules_within_tol": rok, "rule_worst_bp": round(rworst, 3),
                   "diagnostics": [(d.get("code"), (d.get("message") or "")[:120]) for d in diags],
                   "font_diagnostics": [(d.get("code"), (d.get("message") or "")[:120]) for d in font_bad]}
            rows.append(row)
            print(f"{'PASS' if good else 'FAIL'} {name:30} words {n:3} ok {ok:3} unal {unaligned:3} worst {worst:7.3f}"
                  f" | rules {rn:2}/{ncand_rules:2} ok {rok:2} worst {rworst:7.3f}")
    print(f"TOTAL {passed}/{len(rows)} (words {TOL} bp, rules {RULE_TOL} bp)")
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump({"passed": passed, "total": len(rows), "tolerance_bp": TOL, "rule_tolerance_bp": RULE_TOL,
                       "font_dirs": env.get("FLASHTEX_FONT_DIRS"), "tfm_dirs": env.get("FLASHTEX_TFM_DIRS"),
                       "fixtures": rows}, f, indent=1)
            f.write("\n")
    return 0 if passed == len(rows) else 1


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("refs")
    r.add_argument("--texbin", default="/Library/TeX/texbin")
    r.add_argument("only", nargs="*")
    c = sub.add_parser("check")
    c.add_argument("--render", required=True)
    fontenv.add_font_arguments(c, REPO)
    c.add_argument("--json")
    c.add_argument("only", nargs="*")
    args = ap.parse_args()
    return cmd_refs(args) if args.cmd == "refs" else cmd_check(args)


if __name__ == "__main__":
    sys.exit(main() or 0)
