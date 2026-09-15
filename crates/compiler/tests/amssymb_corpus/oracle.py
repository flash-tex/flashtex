#!/usr/bin/env python3
"""amssymb/over-under corpus: pdflatex word positions vs flashtex-render.

Oracle tooling only; pdflatex never runs in the product path. Same method as
`../amsmath_corpus/oracle.py` (words from glyph origins, aligned on text, a
fixture passes when every aligned word is within 0.5 bp in x and y and
nothing is unaligned), with these reference/candidate normalisations:

* AMS symbol fonts (MSAM*/MSBM*): pdftext cannot name their glyphs, so each
  is given the Unicode text `crates/compiler/src/amssymb.rs` assigns to its
  slot (the same text FlashTeX extracts). Slots with empty text (`\\dabar@`)
  are dropped on both sides.
* Math accents of the OT1 roman font (`\\hat` "5E, `\\tilde` "7E, ...) read
  as `?`; they are given the spacing accent characters FlashTeX sets.
* cmex glyphs are compared as sorted distinct x columns, as in the amsmath
  oracle, now including the wide accents (U+0302/U+0303 on the candidate).
* `\\overbrace`/`\\underbrace` pieces (cmex "7A-"7D) are compared by the
  extent of each brace row (leftmost origin, rightmost origin + advance):
  pdfTeX sets four cmex pieces, FlashTeX paints Latin Modern Math's three
  assembly parts (left end, middle, right end) aligned to the same extent.
* The dashed-arrow heads (U+21E2/U+21E0) are not position-checked: FlashTeX
  paints a 1em New Computer Modern arrow right-aligned in the 1.875em msam box
  (the box, and so everything after it, is exact).
* A negation overlay glyph (U+0338 painted over its base in the same
  cluster) is not a separate glyph of the word.

    python3 oracle.py refs --texbin /Library/TeX/texbin
    python3 oracle.py check --render path/to/flashtex-render --fonts apps/mac/Fonts
"""
import argparse, json, os, re, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "tools", "visual-oracle"))
sys.path.insert(0, os.path.join(REPO, "tools", "real-world-corpus"))
sys.path.insert(0, os.path.join(REPO, "crates", "compiler", "tests", "amsmath_corpus"))
import fontenv  # noqa: E402
import pdftext  # noqa: E402
import rank  # noqa: E402
import oracle as ams_oracle  # noqa: E402  (the amsmath corpus oracle)

FIXTURES = os.path.join(HERE, "fixtures")
REFS = os.path.join(HERE, "refs")
TOL = ams_oracle.TOL
Q = ams_oracle.Q
EXT_SIZE_BP = ams_oracle.EXT_SIZE_BP
EXT_TEXT = ams_oracle.EXT_TEXT | set("\u0302\u0303")
BRACES = set("\u23de\u23df")
UNCHECKED = set("\u21e2\u21e0")
ROMAN_ACCENTS = {0x5E: "\u02c6", 0x7E: "\u02dc", 0x16: "\u00af", 0x5F: "\u02d9", 0x7F: "\u00a8",
                 0x13: "\u00b4", 0x12: "`", 0x14: "\u02c7", 0x15: "\u02d8"}


def ams_text():
    src = open(os.path.join(REPO, "crates", "compiler", "src", "amssymb.rs"), encoding="utf-8").read()
    out = {}
    for m in re.finditer(r'text: "((?:[^"\\]|\\u\{[0-9A-F]+\})*)", class: SymbolClass::\w+, '
                         r'font: SymbolFont::(Msam|Msbm), slot: 0x([0-9A-F]{2})', src):
        text = re.sub(r"\\u\{([0-9A-F]+)\}", lambda u: chr(int(u.group(1), 16)), m.group(1))
        out[(m.group(2).upper(), int(m.group(3), 16))] = text
    return out


AMS_TEXT = ams_text()


def extent(glyphs, per):
    """Brace extents [left, right]: glyphs banded by baseline (within 3 bp;
    FlashTeX re-centres each painted part on its TFM box, so part baselines
    differ slightly), sorted by x, `per` glyphs to a brace (4 cmex pieces on
    the reference side, 3 assembly parts on the candidate side)."""
    bands = []
    for g in sorted(glyphs, key=lambda g: g["y_top"]):
        if bands and abs(bands[-1][-1]["y_top"] - g["y_top"]) <= 3.0:
            bands[-1].append(g)
        else:
            bands.append([g])
    out = []
    for band in bands:
        band.sort(key=lambda g: g["x"])
        for i in range(0, len(band), per):
            chunk = band[i:i + per]
            out.append([round(min(g["x"] for g in chunk), 4), round(max(g["x"] + g["advance"] for g in chunk), 4)])
    return sorted(out)


def ref_page(glyphs):
    words, ext, braces = [], [], []
    for g in glyphs:
        font = g["font"].upper()
        if "MATHEXTENSION" in font or font.startswith("CMEX") or font.startswith("LMEX"):
            (braces if 0x7A <= g["code"] <= 0x7D else ext).append(g)
            continue
        fam = "MSAM" if "MSAM" in font else "MSBM" if "MSBM" in font else None
        if fam:
            text = AMS_TEXT.get((fam, g["code"]), "?")
            if text == "" or text in UNCHECKED:
                continue
            g = dict(g, text=text)
        elif g["text"] == "?" and ("MATHSYMBOLS" in font or font.startswith("CMSY") or font.startswith("LMSY")):
            g = dict(g, text=ams_oracle.OMS_TEXT.get(g["code"], "?"))
        elif g["text"] == "?" and "ROMAN" in font and g["code"] in ROMAN_ACCENTS:
            g = dict(g, text=ROMAN_ACCENTS[g["code"]])
        words.append(g)
    return ams_oracle.regroup(words), ams_oracle.columns(ext), extent(braces, 4)


def cand_pages(v2path):
    pl = json.load(open(v2path, encoding="utf-8"))["payload"]
    pages = []
    for page in pl["pages"]:
        glyphs, ext, braces = [], [], []
        for item in page.get("items", []):
            if item.get("kind") != "glyph_run":
                continue
            text = (item.get("text") or "").encode("utf-8")
            clusters = item.get("clusters") or []
            size = item.get("font_size", 0) / Q
            seen = set()
            for gi, g in enumerate(item.get("glyphs") or []):
                ci = g.get("cluster", gi)
                if ci in seen:
                    continue
                seen.add(ci)
                ct = (text[clusters[ci]["text_start_byte"]:clusters[ci]["text_end_byte"]].decode("utf-8", "replace")
                      if ci < len(clusters) else "?")
                if ct in UNCHECKED:
                    continue
                glyph = {"text": ct, "x": g["origin_x"] / Q, "y_top": g["baseline_y"] / Q,
                         "advance": g["advance_x"] / Q, "size": size, "font": ""}
                if ct in BRACES:
                    braces.append(glyph)
                elif abs(size - EXT_SIZE_BP) < 0.01 and ct in EXT_TEXT:
                    ext.append(glyph)
                else:
                    glyphs.append(glyph)
        pages.append((ams_oracle.regroup(glyphs), ams_oracle.columns(ext), extent(braces, 3)))
    return pages


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
            doc = pdftext.PdfDocument.load(os.path.join(work, name + ".pdf"))
            pages = [ref_page(pdftext.page_glyphs(doc, p)[0]) for p in doc.pages()]
            with open(os.path.join(REFS, name + ".json"), "w", encoding="utf-8") as f:
                json.dump({"fixture": name + ".tex", "reference_engine": version,
                           "invocation": "pdflatex -interaction=batchmode, two passes, SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1",
                           "unit": "bp, y from page top",
                           "pages": [w for w, _, _ in pages],
                           "extension_columns": [c for _, c, _ in pages],
                           "brace_extents": [b for _, _, b in pages]}, f, indent=1, ensure_ascii=False)
                f.write("\n")
            print(f"{name}: {len(pages)} page(s), {sum(len(w) for w, _, _ in pages)} words, "
                  f"{sum(len(c) for _, c, _ in pages)} extension columns, {sum(len(b) for _, _, b in pages)} braces")


def cmd_check(args):
    passed, rows = 0, []
    env = fontenv.render_env(args.fonts, args.tfm_dirs)
    print(fontenv.describe(env))
    with tempfile.TemporaryDirectory() as work:
        for name in fixtures(args.only):
            pinned = json.load(open(os.path.join(REFS, name + ".json"), encoding="utf-8"))
            ref = pinned["pages"]
            ref_cols = pinned["extension_columns"]
            ref_braces = pinned["brace_extents"]
            text = open(os.path.join(FIXTURES, name + ".tex"), encoding="utf-8").read()
            req = {"protocol_version": 1, "id": name, "type": "compile",
                   "payload": {"project_id": "amssymb-corpus", "revision": 1, "entry_path": "main.tex", "date": "1970-01-01",
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
                             if d.get("severity") == "error" or d.get("code") in ("math_limitation", "math_glyph_unmapped")]
            cand = cand_pages(v2) if os.path.isfile(v2) else []
            n = ok = unaligned = 0
            cols_ok = True
            worst = 0.0
            for rw, rc, rb, (cw, cc, cb) in zip(ref, ref_cols, ref_braces, cand):
                idx, ur, uc = rank.align_words(rw, cw)
                unaligned += ur + uc
                for i, j in idx:
                    d = max(abs(cw[j]["x"] - rw[i]["x"]), abs(cw[j]["y_top"] - rw[i]["y_top"]))
                    worst, n, ok = max(worst, d), n + 1, ok + (d <= TOL)
                if len(rc) != len(cc) or len(rb) != len(cb):
                    cols_ok = False
                for rx, cx in zip(rc, cc):
                    worst = max(worst, abs(rx - cx))
                    cols_ok = cols_ok and abs(rx - cx) <= TOL
                for (rl, rr), (cl, cr) in zip(rb, cb):
                    d = max(abs(rl - cl), abs(rr - cr))
                    worst = max(worst, d)
                    cols_ok = cols_ok and d <= TOL
            good = (len(ref) == len(cand) == 1 and n > 0 and ok == n
                    and unaligned == 0 and cols_ok and not font_bad)
            if font_bad:
                fontenv.report_font_failure(name, font_bad, env)
            passed += good
            row = {"fixture": name, "pass": good, "pages": [len(ref), len(cand)], "aligned": n, "within_tol": ok,
                   "unaligned": unaligned,
                   "extension_columns": [sum(len(c) for c in ref_cols), sum(len(c) for _, c, _ in cand)],
                   "braces": [sum(len(b) for b in ref_braces), sum(len(b) for _, _, b in cand)],
                   "worst_bp": round(worst, 3),
                   "diagnostics": [(d.get("code"), (d.get("message") or "")[:120]) for d in diags],
                   "font_diagnostics": [(d.get("code"), (d.get("message") or "")[:120]) for d in font_bad]}
            rows.append(row)
            print(f"{'PASS' if good else 'FAIL'} {name:28} words {n:3} ok {ok:3} unaligned {unaligned:3} "
                  f"ext {row['extension_columns'][0]:2}/{row['extension_columns'][1]:2} "
                  f"braces {row['braces'][0]}/{row['braces'][1]} worst {worst:8.3f}")
    print(f"TOTAL {passed}/{len(rows)} within {TOL} bp")
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump({"passed": passed, "total": len(rows), "tolerance_bp": TOL,
                       "font_dirs": env.get("FLASHTEX_FONT_DIRS"), "tfm_dirs": env.get("FLASHTEX_TFM_DIRS"),
                       "fixtures": rows}, f, indent=1,
                      ensure_ascii=False)
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
