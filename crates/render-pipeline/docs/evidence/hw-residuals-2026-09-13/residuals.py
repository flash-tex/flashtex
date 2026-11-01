"""Per-word residuals of flashtex-render vs pdflatex for HW1/HW2.

usage: residuals.py <repo-root> <out-dir>   (reads <out-dir>/ours/HWn.json, <out-dir>/ref/HWn.pdf)
       residuals.py <repo-root> - <name>=<ours.json>=<ref.pdf> ...  (any documents)

Words come from tools/visual-oracle/pdftext.py (content-stream glyphs, bp).
Lines are matched per page by order of distinct baselines; words on a line are
matched in order by text. dx/dy are ours - ref in TeX pt. A line whose word
lists differ is reported as a line-break/content mismatch instead.
"""
import os
import sys
from collections import defaultdict

R, O = sys.argv[1], sys.argv[2]
sys.path.insert(0, os.path.join(R, "tools", "visual-oracle"))
import pdftext  # noqa: E402

BP2PT = 72.27 / 72.0
THRESH = 0.1


def norm(t):
    return "".join(ch for ch in t if not ch.isspace())


def ours_pages(path):
    import json
    Q = float(2 ** 20)
    pl = json.load(open(path))["payload"]
    pages = []
    for page in pl["pages"]:
        glyphs = []
        for it in page.get("items", []):
            if it.get("kind") != "glyph_run":
                continue
            t = (it.get("text") or "").encode()
            cl = it.get("clusters") or []
            for gi, g in enumerate(it.get("glyphs") or []):
                ci = g.get("cluster", gi)
                ct = t[cl[ci]["text_start_byte"]:cl[ci]["text_end_byte"]].decode("utf-8", "replace") if ci < len(cl) else "?"
                glyphs.append({"x": g["origin_x"] / Q, "y_top": g["baseline_y"] / Q, "size": it["font_size"] / Q,
                               "font": "", "text": ct, "advance": g["advance_x"] / Q, "bt": 0})
        pages.append(pdftext.words_from_glyphs(glyphs))
    return pages


def ref_pages(path):
    return [rec["words"] for rec in pdftext.page_words(path)]


def lines(words):
    by = defaultdict(list)
    for w in words:
        if norm(w["text"]):
            by[round(w["y_top"], 1)].append(w)
    return [(y, sorted(ws, key=lambda w: w["x"])) for y, ws in sorted(by.items())]


def main(docs):
    out = []
    summary = []
    allrows = []
    linedy = {}
    for n, ours_json, ref_pdf in docs:
        ours = [lines(p) for p in ours_pages(ours_json)]
        ref = [lines(p) for p in ref_pages(ref_pdf)]
        out.append(f"\n## {n}: pages ours {len(ours)} ref {len(ref)}\n")
        for pi in range(min(len(ours), len(ref))):
            used = set()
            mism = total = off = 0
            for ry, rws in ref[pi]:
                cands = [(abs(oy - ry), oy, ows) for oy, ows in ours[pi] if oy not in used and abs(oy - ry) < 8]
                rt = [norm(w["text"]) for w in rws]
                best = None
                for _, oy, ows in sorted(cands, key=lambda c: c[0]):
                    ot = [norm(w["text"]) for w in ows]
                    if len(set(ot) & set(rt)) * 2 >= len(rt):
                        best = (oy, ows)
                        break
                if best is None:
                    mism += 1
                    out.append(f"- p{pi+1} y={ry*BP2PT:.1f}: no line in ours: `{' '.join(rt)[:80]}`")
                    continue
                used.add(best[0])
                ows = list(best[1])
                if [norm(w["text"]) for w in ows] != rt:
                    out.append(f"- p{pi+1} y={ry*BP2PT:.1f}: word split differs: ref `{' '.join(rt)[:60]}` ours `{' '.join(norm(w['text']) for w in ows)[:60]}`")
                j = 0
                for li, rw in enumerate(rws):
                    k = j
                    while k < len(ows) and norm(ows[k]["text"]) != norm(rw["text"]):
                        k += 1
                    if k == len(ows):
                        continue
                    ow = ows[k]
                    j = k + 1
                    total += 1
                    dx = (ow["x"] - rw["x"]) * BP2PT
                    dy = (ow["y_top"] - rw["y_top"]) * BP2PT
                    dw = (ow["width"] - rw["width"]) * BP2PT
                    linedy.setdefault((n, pi + 1, round(ry * BP2PT, 1)), []).append(dy)
                    if abs(dx) > THRESH:
                        off += 1
                        allrows.append((n, pi + 1, round(ry * BP2PT, 1), li, rw["text"], dx, dy, dw))
            vlines = sum(1 for (dn, dp, _), v in linedy.items() if dn == n and dp == pi + 1 and abs(sorted(v)[len(v) // 2]) > THRESH)
            summary.append((n, pi + 1, total, off, vlines, mism))
    print("# HW1/HW2 per-word residuals vs pdflatex (threshold 0.1pt)\n")
    print("| doc | page | matched words | words dx >0.1pt | lines with median dy >0.1pt | unmatched ref lines |")
    print("|---|---|---|---|---|---|")
    for s in summary:
        print("| %s | %d | %d | %d | %d | %d |" % s)
    print("\n## Line vertical offsets (median dy of matched words, pt)\n")
    print("| doc | page | ref y | dy |")
    print("|---|---|---|---|")
    for (dn, dp, y), v in sorted(linedy.items()):
        m = sorted(v)[len(v) // 2]
        if abs(m) > THRESH:
            print("| %s | %d | %.1f | %+.2f |" % (dn, dp, y, m))
    print("\n## Words with horizontal residual dx >0.1pt (ours - ref, pt)\n")
    print("| doc | page | y | word# | word | dx | dy | dwidth |")
    print("|---|---|---|---|---|---|---|---|")
    for r in allrows:
        print("| %s | %d | %.1f | %d | `%s` | %+.2f | %+.2f | %+.2f |" % r)
    print("\n## Unmatched lines\n")
    print("\n".join(out))


if __name__ == "__main__":
    if O == "-":
        main([tuple(a.split("=", 2)) for a in sys.argv[3:]])
    else:
        main([(n, f"{O}/ours/{n}.json", f"{O}/ref/{n}.pdf") for n in ("HW1", "HW2")])
