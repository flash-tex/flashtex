#!/usr/bin/env python3
"""xcolor oracle: pdflatex glyph and rule positions with their colours.

Oracle tooling only; pdflatex never runs in the product path or in cargo.

    # pin the references (needs TeX Live; oracle only)
    python3 oracle.py refs [--texbin /Library/TeX/texbin] [names...]
    # measure a build: display list (flashtex-render --device-color) and the
    # exact PDF route (flashtex-pdf-exact from-v2) against the references
    python3 oracle.py check --render flashtex-render --pdf-exact flashtex-pdf-exact [--fonts DIR]

References (`refs/<fixture>.json`), per page, in bp with y from the page top:

  glyphs  [text, x, y_top, fill, font]: every shown glyph's origin, the fill
          operator in force when pdfTeX showed it (`1 0 0 rg`, `0 g`, ...) and
          its font. Math extension glyphs (cmex/lmex: big operators and
          delimiters, whose painted origin legitimately differs) are matched
          by colour against the nearest glyph only.
  rules   [x, top, width, height, fill]: every `re` filled by `f` with the
          fill operator in force, and every single horizontal or vertical
          `m`/`l` segment stroked by `S` (how pdfTeX writes `\\hrule` and
          `\\vrule`) as its rectangle, with the stroke operator written as the
          equivalent fill operator (`0 0 1 RG` -> `0 0 1 rg`).

Colour operands are canonical decimals (`0.50` -> `0.5`, `.5` -> `0.5`).
Colour state follows `q`/`Q`. `tests/xcolor_oracle.rs` replays the fixtures
through the pipeline: same page count, a glyph within 0.5 bp of every
reference glyph whose run paints the same fill operator, and for every
reference rule a distinct rule within 0.1 bp with the same operator.
"""
import argparse, json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "tools", "visual-oracle"))
import fontenv  # noqa: E402
import pdftext  # noqa: E402

FIXTURES = os.path.join(HERE, "fixtures")
REFS = os.path.join(HERE, "refs")
TOL, RULE_TOL = 0.5, 0.1
Q = float(2 ** 20)
FILL = {b"g": "g", b"rg": "rg", b"k": "k"}
STROKE = {b"G": "g", b"RG": "rg", b"K": "k"}


def canon(token):
    t = token.decode() if isinstance(token, (bytes, bytearray)) else str(token)
    neg = t.startswith("-")
    t = t.lstrip("-")
    whole, _, frac = t.partition(".")
    whole = whole.lstrip("0") or "0"
    frac = frac.rstrip("0")
    return ("-" if neg else "") + (whole + "." + frac if frac else whole)


def page_data(doc, page):
    """(fills per shown byte in content order, rules with colours)."""
    media = [float(doc.resolve(v)) for v in doc.resolve(page.get("MediaBox", [0, 0, 612, 792]))]
    height = media[3] - media[1]
    contents = page.get("Contents")
    resolved = doc.resolve(contents) if not isinstance(contents, list) else contents
    data = b"\n".join(doc.stream_of(c) for c in resolved) if isinstance(resolved, list) else doc.stream_of(contents)
    lx = pdftext._Lexer(data)
    stack, saved = [], []
    fill, stroke, ctm = "0 g", "0 g", (1, 0, 0, 1, 0, 0)
    path, segment, line_width, in_text = [], [], 1.0, False
    fills, rules = [], []
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
            if op in FILL:
                fill = " ".join(canon(v) for v in stack) + " " + FILL[op]
            elif op in STROKE:
                stroke = " ".join(canon(v) for v in stack) + " " + STROKE[op]
            elif op == b"BT":
                in_text = True
            elif op == b"ET":
                in_text = False
            elif op == b"q":
                saved.append((fill, stroke, ctm))
            elif op == b"Q":
                if saved:
                    fill, stroke, ctm = saved.pop()
            elif op == b"cm":
                ctm = pdftext._mul(tuple(float(v) for v in stack[-6:]), ctm)
            elif op in (b"Tj", b"'", b'"'):
                fills.extend([fill] * len(stack[-1]))
            elif op == b"TJ":
                for el in stack[-1]:
                    if isinstance(el, (bytes, bytearray)):
                        fills.extend([fill] * len(el))
            elif op == b"re" and not in_text:
                x, y, w, h = (float(v) for v in stack[-4:])
                a, _, _, d, e, f = ctm
                x0, y0, x1, y1 = a * x + e, d * y + f, a * (x + w) + e, d * (y + h) + f
                path.append((min(x0, x1), height - max(y0, y1), abs(x1 - x0), abs(y1 - y0)))
            elif op in (b"f", b"F", b"f*"):
                rules.extend(list(r) + [fill] for r in path)
                path, segment = [], []
            elif op == b"w":
                line_width = float(stack[-1])
            elif op in (b"m", b"l") and not in_text:
                a, _, _, d, e, f = ctm
                segment.append((a * float(stack[-2]) + e, d * float(stack[-1]) + f))
            elif op == b"S":
                if len(segment) == 2:
                    (x0, y0), (x1, y1) = segment
                    w = line_width * abs(ctm[0])
                    if abs(y1 - y0) < 1e-6:
                        rules.append([min(x0, x1), height - (y0 + w / 2), abs(x1 - x0), w, stroke])
                    elif abs(x1 - x0) < 1e-6:
                        rules.append([x0 - w / 2, height - max(y0, y1), w, abs(y1 - y0), stroke])
                path, segment = [], []
            elif op in (b"n", b"s"):
                path, segment = [], []
        except (IndexError, TypeError, ValueError):
            pass
        stack = []
    return fills, [[round(v, 4) if isinstance(v, float) else v for v in r] for r in rules]


def pdf_pages(pdf):
    doc = pdftext.PdfDocument.load(pdf)
    out = []
    for page in doc.pages():
        glyphs, _ = pdftext.page_glyphs(doc, page)
        fills, rules = page_data(doc, page)
        if len(fills) != len(glyphs):
            raise SystemExit(f"{pdf}: {len(glyphs)} glyphs but {len(fills)} shown bytes")
        out.append({
            "glyphs": [[g["text"], round(g["x"], 4), round(g["y_top"], 4), f, g["font"]] for g, f in zip(glyphs, fills)],
            "rules": rules,
        })
    return out


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
            with open(os.path.join(work, name + ".tex"), "w", encoding="utf-8") as f:
                f.write(open(os.path.join(FIXTURES, name + ".tex"), encoding="utf-8").read())
            for _ in range(2):
                subprocess.run([pdflatex, "-interaction=batchmode", name + ".tex"], cwd=work, env=env,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            errors = [l for l in open(os.path.join(work, name + ".log"), encoding="latin-1") if l.startswith("!")]
            if errors:
                raise SystemExit(f"{name}: pdflatex errors {errors[:3]}")
            pages = pdf_pages(os.path.join(work, name + ".pdf"))
            with open(os.path.join(REFS, name + ".json"), "w", encoding="utf-8") as f:
                json.dump({"fixture": name + ".tex", "reference_engine": version,
                           "invocation": "pdflatex -interaction=batchmode, two passes, SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1",
                           "unit": "bp, y from page top; glyphs [text, x, y_top, fill]; rules [x, top, width, height, fill]",
                           "pages": pages}, f, indent=1)
                f.write("\n")
            print(f"{name}: {len(pages)} page(s), {sum(len(p['glyphs']) for p in pages)} glyphs, "
                  f"{sum(len(p['rules']) for p in pages)} rules")


def is_extension(font):
    f = font.upper()
    return "MATHEXTENSION" in f or "CMEX" in f or "LMEX" in f


def match(ref_pages, cand_pages):
    """(glyphs ok, glyphs, rules ok, rules, worst glyph bp, worst rule bp, colour mismatches)."""
    gok = gn = rok = rn = 0
    gworst = rworst = 0.0
    colour = []
    for rp, cp in zip(ref_pages, cand_pages):
        for text, x, y, fill, *font in rp["glyphs"]:
            gn += 1
            if is_extension(font[0] if font else "") and cp["glyphs"]:
                best = min(cp["glyphs"], key=lambda c: abs(c[1] - x) + abs(c[2] - y))
                gok += best[3] == fill
                continue
            near = [c for c in cp["glyphs"] if abs(c[1] - x) <= TOL and abs(c[2] - y) <= TOL]
            if near:
                best = min(near, key=lambda c: max(abs(c[1] - x), abs(c[2] - y)))
                gworst = max(gworst, max(abs(best[1] - x), abs(best[2] - y)))
                if best[3] == fill:
                    gok += 1
                else:
                    colour.append(f"glyph {text!r}: {best[3]} vs {fill}")
        used = set()
        for r in rp["rules"]:
            rn += 1
            cands = [(max(abs(c[k] - r[k]) for k in range(4)), j) for j, c in enumerate(cp["rules"]) if j not in used]
            if not cands:
                continue
            d, j = min(cands)
            if d <= RULE_TOL:
                used.add(j)
                rworst = max(rworst, d)
                if cp["rules"][j][4] == r[4]:
                    rok += 1
                else:
                    colour.append(f"rule {r[:4]}: {cp['rules'][j][4]} vs {r[4]}")
    return gok, gn, rok, rn, gworst, rworst, colour


def cand_v2(path):
    pages = []
    for page in json.load(open(path, encoding="utf-8"))["payload"]["pages"]:
        glyphs, rules = [], []
        for it in page.get("items", []):
            dc = (it.get("paint") or {}).get("device_color")
            fill = "0 g"
            if dc:
                fill = " ".join(canon(v) for v in dc["values"]) + " " + {"gray": "g", "rgb": "rg", "cmyk": "k"}[dc["space"]]
            if it.get("kind") == "rule":
                rules.append([it["x"] / Q, it["top"] / Q, it["width"] / Q, it["height"] / Q, fill])
            elif it.get("kind") == "glyph_run":
                glyphs.extend(["", g["origin_x"] / Q, g["baseline_y"] / Q, fill, ""] for g in it["glyphs"])
        pages.append({"glyphs": glyphs, "rules": rules})
    return pages


def cmd_check(args):
    passed = 0
    names = fixtures(args.only)
    env = fontenv.render_env(args.fonts, args.tfm_dirs)
    print(fontenv.describe(env))
    font_dirs = fontenv.split(env.get("FLASHTEX_FONT_DIRS"))
    with tempfile.TemporaryDirectory() as work:
        for name in names:
            ref = json.load(open(os.path.join(REFS, name + ".json"), encoding="utf-8"))["pages"]
            tex = os.path.join(FIXTURES, name + ".tex")
            v2 = os.path.join(work, name + ".v2.json")
            cmd = [args.render, "--tex", tex, "--v2", v2, "--device-color"]
            for d in font_dirs:
                cmd += ["--font-dir", d]
            rp = subprocess.run(cmd, env=env, capture_output=True, timeout=120)
            font_bad = []
            for line in rp.stdout.decode("utf-8", "replace").splitlines():
                try:
                    m = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if m.get("type") == "compile_result":
                    font_bad = fontenv.font_diagnostics(m["payload"].get("diagnostics", []))
            cand = cand_v2(v2) if os.path.isfile(v2) else []
            gok, gn, rok, rn, gw, rw, colour = match(ref, cand)
            row = f"{name:32} pages {len(ref)}/{len(cand)} glyphs {gok}/{gn} (worst {gw:.3f}) rules {rok}/{rn} (worst {rw:.3f})"
            exact = ""
            if args.pdf_exact and os.path.isfile(v2):
                pdf = os.path.join(work, name + ".pdf")
                ecmd = [args.pdf_exact, "from-v2", v2, "--out", pdf]
                for d in font_dirs:
                    ecmd += ["--font-dir", d]
                p = subprocess.run(ecmd, env=env, capture_output=True, text=True, timeout=120)
                if os.path.isfile(pdf):
                    egok, egn, erok, ern, _, _, ecol = match(ref, pdf_pages(pdf))
                    exact = f" | exact pdf glyph colours {egok}/{egn} rules {erok}/{ern}"
                    colour += [f"exact: {c}" for c in ecol[:3]]
                else:
                    exact = f" | exact pdf failed: {(p.stderr or p.stdout).strip()[:120]}"
            good = len(ref) == len(cand) and gok == gn and rok == rn and not font_bad
            if font_bad:
                fontenv.report_font_failure(name, font_bad, env)
            passed += good
            print(("PASS " if good else "FAIL ") + row + exact)
            for c in colour[:4]:
                print("     " + c)
    print(f"TOTAL {passed}/{len(names)}")
    return 0 if passed == len(names) else 1


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("refs")
    r.add_argument("--texbin", default="/Library/TeX/texbin")
    r.add_argument("only", nargs="*")
    c = sub.add_parser("check")
    c.add_argument("--render", required=True)
    c.add_argument("--pdf-exact")
    fontenv.add_font_arguments(c, REPO)
    c.add_argument("only", nargs="*")
    args = ap.parse_args()
    return cmd_refs(args) if args.cmd == "refs" else cmd_check(args)


if __name__ == "__main__":
    sys.exit(main() or 0)
