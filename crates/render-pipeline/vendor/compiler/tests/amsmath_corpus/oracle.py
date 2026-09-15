#!/usr/bin/env python3
"""FT-061 amsmath oracle corpus: pdflatex word positions vs flashtex-render.

Oracle tooling only; pdflatex never runs in the product path.

    # regenerate the pinned reference positions (needs MacTeX; oracle only)
    python3 oracle.py refs --texbin /Library/TeX/texbin
    # measure a flashtex-render build against the pinned references
    python3 oracle.py check --render path/to/flashtex-render --fonts apps/mac/Fonts

`--fonts` and `--tfm-dirs` both take colon-separated lists, and an exported
`FLASHTEX_FONT_DIRS` / `FLASHTEX_TFM_DIRS` takes precedence over either (the
harness must never discard a correctly configured environment). When the
metrics directories are not given, they are derived by walking
`<fonts>/texmf` for directories holding `.tfm` files -- `--fonts` alone does
not reach the bundled texmf, whose roots are relative to the executable.

Any font diagnostic (`font_unavailable`, `required_metrics_unavailable`,
`ec_metrics_unavailable`, ...) fails the fixture: the renderer used
substituted metrics, so its positions are not comparable to the reference and
scoring the run would report a fallback as a pass.

Words are formed identically on both sides from glyph origins (a gap wider
than 0.16 em or a new baseline starts a word; PDF text objects are ignored),
using tools/visual-oracle/pdftext.py for the reference PDF and the
rendering-v2 display list's glyph_run items for the candidate. Words are
aligned on normalised text (tools/visual-oracle/rank.py:align_words).
A fixture passes when both sides have one page, every word aligns, and
every aligned word's origin is within 0.5 bp in x and y.

Two reference-side facts need normalising before words can pair:

* pdfTeX's math symbol font (cmsy/lmsy, OMS encoding) carries glyph names
  pdftext does not map (`lessequal`, `arrowright`), so those glyphs read as
  `?`; they are given their Unicode text from the OMS code (`OMS_TEXT`).
* Glyphs of the 10 pt math extension font (cmex10/lmex10: big delimiters,
  extensible pieces, display operators) are compared as columns, not words:
  pdfTeX draws the Type 1 glyph from the origin at the top of its TFM box,
  one Type 1 glyph per extensible piece, while FlashTeX paints the Latin
  Modern Math OpenType variant or assembly, whose origin (and piece count)
  legitimately differ. Their horizontal placement is checked instead: the
  sorted distinct x origins of extension glyphs must match within 0.5 bp.
  A candidate glyph counts as an extension glyph when it is a delimiter or
  large-operator character set at the extension font's 10 pt (the fixtures
  are 12 pt documents, where no other math glyph is set at 10 pt).
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
TIE = 0.05
Q = float(2 ** 20)
# cmex10 at its design size, in bp.
EXT_SIZE_BP = 10 * 72 / 72.27
# Characters FlashTeX paints from the math extension role.
EXT_TEXT = set("()[]{}|‖⟨⟩⌊⌋⌈⌉/\\∑∏∐∫∮⋃⋂⨁⨂⨀⨄⨆√")
# OMS (cmsy) code -> text, for the glyph names pdftext leaves as "?".
OMS_TEXT = dict(enumerate(
    "−·×∗÷⋄±∓⊕⊖⊗⊘⊙◯∘∙≍≡⊆⊇≤≥⪯⪰∼≈⊂⊃≪≫≺≻←→↑↓↔↗↘≃⇐⇒⇑⇓⇔↖↙∝′∞∈∋△▽/↦∀∃¬∅ℜℑ⊤⊥ℵ"
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ∪∩⊎∧∨⊢⊣⌊⌋⌈⌉{}⟨⟩|‖↕⇕\\≀√⨿∇∫⊔⊓⊑⊒§†‡¶♣♢♡♠"))


def regroup(glyphs):
    glyphs = sorted((dict(g, bt=0) for g in glyphs), key=lambda g: (round(g["y_top"], 1), g["x"]))
    # Glyphs whose origins coincide within TIE bp (amsmath's `\relbar`
    # and arrow head, 14mu apart by construction) have no reading order:
    # pdfTeX's quantised TJ offsets and FlashTeX's exact arithmetic may put
    # them either way round, so they are ordered by text.
    for i in range(1, len(glyphs)):
        j = i
        while (j > 0 and round(glyphs[j]["y_top"], 1) == round(glyphs[j - 1]["y_top"], 1)
               and glyphs[j]["x"] - glyphs[j - 1]["x"] < TIE and glyphs[j]["text"] < glyphs[j - 1]["text"]):
            glyphs[j], glyphs[j - 1] = glyphs[j - 1], glyphs[j]
            j -= 1
    return [{"text": w["text"], "x": round(w["x"], 4), "y_top": round(w["y_top"], 4)}
            for w in pdftext.words_from_glyphs(glyphs)]


def columns(glyphs):
    """Sorted distinct x origins (0.05 bp) of extension glyphs."""
    xs = []
    for x in sorted(g["x"] for g in glyphs):
        if not xs or x - xs[-1] > 0.05:
            xs.append(x)
    return [round(x, 4) for x in xs]


def ref_page(glyphs):
    words, ext = [], []
    for g in glyphs:
        font = g["font"].upper()
        if "MATHEXTENSION" in font or font.startswith("CMEX") or font.startswith("LMEX"):
            ext.append(g)
            continue
        if g["text"] == "?" and ("MATHSYMBOLS" in font or font.startswith("CMSY") or font.startswith("LMSY")):
            g = dict(g, text=OMS_TEXT.get(g["code"], "?"))
        words.append(g)
    return regroup(words), columns(ext)


def ref_pages(pdf):
    doc = pdftext.PdfDocument.load(pdf)
    return [ref_page(pdftext.page_glyphs(doc, page)[0]) for page in doc.pages()]


def cand_pages(v2path):
    pl = json.load(open(v2path, encoding="utf-8"))["payload"]
    pages = []
    for page in pl["pages"]:
        glyphs, ext = [], []
        for item in page.get("items", []):
            if item.get("kind") != "glyph_run":
                continue
            # Cluster ranges are UTF-8 byte offsets.
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
        pages.append((regroup(glyphs), columns(ext)))
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
                           "unit": "bp, y from page top",
                           "pages": [words for words, _ in pages],
                           "extension_columns": [cols for _, cols in pages]}, f, indent=1)
                f.write("\n")
            print(f"{name}: {len(pages)} page(s), {sum(len(w) for w, _ in pages)} words, "
                  f"{sum(len(c) for _, c in pages)} extension columns")


def cmd_check(args):
    passed, rows = 0, []
    env = fontenv.render_env(args.fonts, args.tfm_dirs)
    print(fontenv.describe(env))
    with tempfile.TemporaryDirectory() as work:
        for name in fixtures(args.only):
            pinned = json.load(open(os.path.join(REFS, name + ".json"), encoding="utf-8"))
            ref = pinned["pages"]
            ref_cols = pinned.get("extension_columns") or [[] for _ in ref]
            text = open(os.path.join(FIXTURES, name + ".tex"), encoding="utf-8").read()
            req = {"protocol_version": 1, "id": name, "type": "compile",
                   "payload": {"project_id": "amsmath-corpus", "revision": 1, "entry_path": "main.tex", "date": "1970-01-01",
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
                             if d.get("severity") == "error" or d.get("code") == "math_limitation"]
            cand = cand_pages(v2) if os.path.isfile(v2) else []
            n = ok = unaligned = 0
            cols_ok = True
            worst = 0.0
            for rw, rc, (cw, cc) in zip(ref, ref_cols, cand):
                idx, ur, uc = rank.align_words(rw, cw)
                unaligned += ur + uc
                for i, j in idx:
                    d = max(abs(cw[j]["x"] - rw[i]["x"]), abs(cw[j]["y_top"] - rw[i]["y_top"]))
                    worst, n, ok = max(worst, d), n + 1, ok + (d <= TOL)
                if len(rc) != len(cc):
                    cols_ok = False
                for rx, cx in zip(rc, cc):
                    worst = max(worst, abs(rx - cx))
                    cols_ok = cols_ok and abs(rx - cx) <= TOL
            good = (len(ref) == len(cand) == 1 and n > 0 and ok == n
                    and unaligned == 0 and cols_ok and not font_bad)
            if font_bad:
                fontenv.report_font_failure(name, font_bad, env)
            passed += good
            ncols = [sum(len(c) for c in ref_cols), sum(len(c) for _, c in cand)]
            row = {"fixture": name, "pass": good, "pages": [len(ref), len(cand)], "aligned": n, "within_tol": ok,
                   "unaligned": unaligned, "extension_columns": ncols, "worst_bp": round(worst, 3),
                   "diagnostics": [(d.get("code"), (d.get("message") or "")[:120]) for d in diags],
                   "font_diagnostics": [(d.get("code"), (d.get("message") or "")[:120]) for d in font_bad]}
            rows.append(row)
            print(f"{'PASS' if good else 'FAIL'} {name:26} words {n:3} ok {ok:3} unaligned {unaligned:3} "
                  f"ext cols {ncols[0]:2}/{ncols[1]:2} worst {worst:8.3f}")
    print(f"TOTAL {passed}/{len(rows)} within {TOL} bp")
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump({"passed": passed, "total": len(rows), "tolerance_bp": TOL,
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
