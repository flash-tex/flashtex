#!/usr/bin/env python3
"""Display-placement oracle: pdflatex word positions vs flashtex-render.

Oracle tooling only; pdflatex never runs in the product path or in cargo.

    python3 gen_fixtures.py
    python3 oracle.py refs --texbin /Library/TeX/texbin [names...]
    python3 oracle.py check --render path/to/flashtex-render [--json out] [names...]

Word formation, reference normalisation and alignment are shared with the
amsmath corpus (`crates/compiler/tests/amsmath_corpus/oracle.py`). Unlike it,
fixtures may span several pages (displays at page breaks): a fixture passes
when page counts agree, every word on every page aligns, every aligned word
origin is within 0.5 bp in x and y, and extension-glyph columns agree.
"""
import argparse, importlib.util, json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "tools", "visual-oracle"))
import fontenv  # noqa: E402
_spec = importlib.util.spec_from_file_location(
    "amsmath_oracle", os.path.join(REPO, "crates", "compiler", "tests", "amsmath_corpus", "oracle.py"))
am = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(am)

FIXTURES = os.path.join(HERE, "fixtures")
REFS = os.path.join(HERE, "refs")
TOL = am.TOL
# The amsmath corpus (12 pt documents) reads a candidate delimiter set at
# 10 bp as a cmex glyph. These fixtures are mostly 10 pt, where that is the
# body size of `(1)`, and none sets a big delimiter or display operator (every
# reference has no extension columns), so no candidate glyph is one.
am.EXT_TEXT = set()


def fixtures(only):
    names = sorted(f[:-4] for f in os.listdir(FIXTURES) if f.endswith(".tex"))
    return [n for n in names if not only or any(o in n for o in only)]


def cmd_refs(args):
    am.FIXTURES, am.REFS = FIXTURES, REFS
    am.cmd_refs(args)


def compare(name, render, env, work):
    pinned = json.load(open(os.path.join(REFS, name + ".json"), encoding="utf-8"))
    ref = pinned["pages"]
    ref_cols = pinned.get("extension_columns") or [[] for _ in ref]
    text = open(os.path.join(FIXTURES, name + ".tex"), encoding="utf-8").read()
    req = {"protocol_version": 1, "id": name, "type": "compile",
           "payload": {"project_id": "display-placement", "revision": 1, "entry_path": "main.tex",
                       "documents": [{"path": "main.tex", "text": text}]}}
    v2 = os.path.join(work, name + ".v2.json")
    rp = subprocess.run([render, "--v2", v2], input=(json.dumps(req) + "\n").encode(), env=env,
                        capture_output=True, timeout=120)
    font_bad = []
    for line in rp.stdout.decode("utf-8", "replace").splitlines():
        try:
            m = json.loads(line)
        except json.JSONDecodeError:
            continue
        if m.get("type") == "compile_result":
            font_bad = fontenv.font_diagnostics(m["payload"].get("diagnostics", []))
    if font_bad:
        fontenv.report_font_failure(name, font_bad, env)
    cand = am.cand_pages(v2) if os.path.isfile(v2) else []
    # pdftext leaves the itemize bullet (lmsy `\textbullet`, a glyph name it
    # does not map) as "?"; FlashTeX's run text is U+2022.
    cand = [([dict(w, text=w["text"].replace("•", "?")) for w in words], cols) for words, cols in cand]
    n = ok = unaligned = 0
    cols_ok = True
    worst = 0.0
    bad = []
    for pi, (rw, rc, (cw, cc)) in enumerate(zip(ref, ref_cols, cand)):
        idx, ur, uc = am.rank.align_words(rw, cw)
        unaligned += ur + uc
        for i, j in idx:
            dx, dy = cw[j]["x"] - rw[i]["x"], cw[j]["y_top"] - rw[i]["y_top"]
            d = max(abs(dx), abs(dy))
            worst, n, ok = max(worst, d), n + 1, ok + (d <= TOL)
            if d > TOL and len(bad) < 6:
                bad.append(f"p{pi + 1} {rw[i]['text']!r} dx {dx:+.2f} dy {dy:+.2f}")
        if len(rc) != len(cc):
            cols_ok = False
        for rx, cx in zip(rc, cc):
            worst = max(worst, abs(rx - cx))
            cols_ok = cols_ok and abs(rx - cx) <= TOL
    good = (len(ref) == len(cand) and n > 0 and ok == n and unaligned == 0
            and cols_ok and not font_bad)
    return {"fixture": name, "pass": good, "pages": [len(ref), len(cand)], "aligned": n, "within_tol": ok,
            "unaligned": unaligned, "worst_bp": round(worst, 3), "sample": bad,
            "font_diagnostics": [(d.get("code"), (d.get("message") or "")[:120]) for d in font_bad]}


def cmd_check(args):
    rows = []
    env = fontenv.render_env(args.fonts, args.tfm_dirs)
    print(fontenv.describe(env))
    with tempfile.TemporaryDirectory() as work:
        for name in fixtures(args.only):
            r = compare(name, args.render, env, work)
            rows.append(r)
            print(f"{'PASS' if r['pass'] else 'FAIL'} {name:34} pages {r['pages'][0]}/{r['pages'][1]} "
                  f"words {r['aligned']:3} ok {r['within_tol']:3} unal {r['unaligned']:3} "
                  f"worst {r['worst_bp']:8.3f} {'; '.join(r['sample'][:3])}")
    passed = sum(r["pass"] for r in rows)
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
