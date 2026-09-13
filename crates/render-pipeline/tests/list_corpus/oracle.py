#!/usr/bin/env python3
"""List-layout oracle corpus: pdflatex word positions vs flashtex-render.

Oracle tooling only; pdflatex never runs in the product path or in cargo.

    # regenerate the pinned reference positions (needs TeX Live; oracle only)
    python3 oracle.py refs --texbin /Library/TeX/texbin
    # measure a flashtex-render build against the pinned references
    python3 oracle.py check --render path/to/flashtex-render [-v] [name ...]

Words (and list labels, which are words of their own) are formed, read and
aligned exactly as in ../../../compiler/tests/tabular_corpus/oracle.py,
whose readers this script imports. A fixture passes when both sides have
the same page count, every word aligns and its origin lies within 0.5 bp of
the reference in x and y. `tests/list_oracle.rs` replays the same check in
cargo from the pinned `refs/`.
"""
import argparse, json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "crates", "compiler", "tests", "tabular_corpus"))
import oracle as tab  # noqa: E402

FIXTURES = os.path.join(HERE, "fixtures")
REFS = os.path.join(HERE, "refs")
TOL = 0.5


# Label glyphs whose text the two readers spell differently: pdftext leaves
# cmsy/lmsy symbols it has no name for as "?" (bullet, asterisk, centred
# dot) and reads the OT1 en dash as "--"; the candidate carries Unicode.
CANON = {"•": "?", "∗": "?", "·": "?", "⋅": "?", "∙": "?", "–": "--"}


def canon(words):
    return [dict(w, text=CANON.get(w["text"], w["text"])) for w in words]


def cand_pages(v2path):
    """Candidate words as in the tabular oracle, except that a run whose
    glyphs all share one source cluster (an `\\item` label, every character
    of which points at the `\\item` command) takes one character of the
    run's text per glyph instead of the cluster's bytes."""
    pl = json.load(open(v2path, encoding="utf-8"))["payload"]
    pages = []
    for page in pl["pages"]:
        glyphs = []
        for item in page.get("items", []):
            if item.get("kind") != "glyph_run":
                continue
            text = item.get("text") or ""
            raw = text.encode("utf-8")
            clusters = item.get("clusters") or []
            gl = item.get("glyphs") or []
            per_glyph = len(gl) == len(text) and len({g.get("cluster", i) for i, g in enumerate(gl)}) < len(gl)
            for gi, g in enumerate(gl):
                ci = g.get("cluster", gi)
                if per_glyph:
                    ct = text[gi]
                elif ci < len(clusters):
                    ct = raw[clusters[ci]["text_start_byte"]:clusters[ci]["text_end_byte"]].decode("utf-8", "replace")
                else:
                    ct = "?"
                glyphs.append({"text": ct, "x": g["origin_x"] / tab.Q, "y_top": g["baseline_y"] / tab.Q,
                               "advance": g["advance_x"] / tab.Q, "size": item.get("font_size", 0) / tab.Q, "font": ""})
        pages.append(canon(tab.regroup(glyphs)))
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
            pages = [words for words, _, _ in tab.ref_pages(os.path.join(work, name + ".pdf"))]
            with open(os.path.join(REFS, name + ".json"), "w", encoding="utf-8") as f:
                json.dump({"fixture": name + ".tex", "reference_engine": version,
                           "invocation": "pdflatex -interaction=batchmode, two passes, SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1",
                           "unit": "bp, word origin, y from page top", "pages": pages}, f, indent=1)
                f.write("\n")
            print(f"{name}: {len(pages)} page(s), {sum(len(w) for w in pages)} words")


def cmd_check(args):
    passed, rows = 0, []
    with tempfile.TemporaryDirectory() as work:
        for name in fixtures(args.only):
            ref = json.load(open(os.path.join(REFS, name + ".json"), encoding="utf-8"))["pages"]
            text = open(os.path.join(FIXTURES, name + ".tex"), encoding="utf-8").read()
            req = {"protocol_version": 1, "id": name, "type": "compile",
                   "payload": {"project_id": "list-corpus", "revision": 1, "entry_path": "main.tex",
                               "documents": [{"path": "main.tex", "text": text}]}}
            v2 = os.path.join(work, name + ".v2.json")
            p = subprocess.run([args.render, "--v2", v2], input=(json.dumps(req) + "\n").encode(),
                               capture_output=True, timeout=120)
            cand = cand_pages(v2) if os.path.isfile(v2) else []
            n = ok = unaligned = 0
            worst = 0.0
            bad = []
            for pi, (rw, cw) in enumerate(zip(ref, cand)):
                idx, ur, uc = tab.rank.align_words(rw, cw)
                unaligned += ur + uc
                for i, j in idx:
                    dx, dy = cw[j]["x"] - rw[i]["x"], cw[j]["y_top"] - rw[i]["y_top"]
                    d = max(abs(dx), abs(dy))
                    worst, n, ok = max(worst, d), n + 1, ok + (d <= TOL)
                    if d > TOL:
                        bad.append((pi, rw[i]["text"], round(rw[i]["x"], 2), round(rw[i]["y_top"], 2), round(dx, 2), round(dy, 2)))
            good = len(ref) == len(cand) and n > 0 and ok == n and unaligned == 0
            passed += good
            rows.append({"fixture": name, "pass": good, "pages": [len(ref), len(cand)], "aligned": n,
                         "within_tol": ok, "unaligned": unaligned, "worst_bp": round(worst, 3)})
            print(f"{'PASS' if good else 'FAIL'} {name:36} pages {len(ref)}/{len(cand)} words {n:3} ok {ok:3} "
                  f"unal {unaligned:3} worst {worst:7.3f}")
            if args.verbose and not good:
                for b in bad[:12]:
                    print("     page {} {!r:14} ref ({}, {}) dx {} dy {}".format(*b))
                if unaligned:
                    rt = " ".join(w["text"] for pg in ref for w in pg)
                    ct = " ".join(w["text"] for pg in cand for w in pg)
                    print("     ref :", rt[:300])
                    print("     cand:", ct[:300])
    print(f"TOTAL {passed}/{len(rows)} (words {TOL} bp)")
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump({"passed": passed, "total": len(rows), "tolerance_bp": TOL, "fixtures": rows}, f, indent=1)
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
    c.add_argument("--json")
    c.add_argument("-v", "--verbose", action="store_true")
    c.add_argument("only", nargs="*")
    args = ap.parse_args()
    return cmd_refs(args) if args.cmd == "refs" else cmd_check(args)


if __name__ == "__main__":
    sys.exit(main() or 0)
