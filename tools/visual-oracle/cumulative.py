#!/usr/bin/env python3
r"""Rank corpus divergences by **blast radius**, not by raw bp.

Companion to `rank.py`, which ranks *pages* by their single largest delta.
That key answers "which page looks worst"; it does not answer "which bug
should be fixed first", and the two are routinely different documents:

* GH-706 (`\addvspace` shared between adjacent lists) was **1.99 bp** per
  boundary — far under `rank.py`'s top item on every page it appeared on —
  yet it moved *every baseline below it*, on 17 fixtures, and the drift
  compounded at each further list.
* GH-717 (`\vdots` as one glyph) was **5.06 pt**, four times larger, but it
  moved one glyph stack inside one matrix and nothing after it.

So this tool measures each divergence's *radius*: a vertical step is
**cumulative** when the page's baselines stay displaced by it for the rest of
the page, and **local** when the following line comes back. It then groups the
steps **by probable cause** — the LaTeX construct standing in the source
between the two lines — so "seventeen fixtures are 1.99 bp low after a list"
arrives as one finding with seventeen witnesses, not as seventeen findings.

Everything measured here reuses `rank.py`: the same producer route
(`flashtex-render --v2` -> `flashtex-pdf-exact from-v2`), the same reference
reader (`pdftext.py` replaying the pinned pdfLaTeX PDF's content stream), the
same word grouper on both sides (`pdftext.words_from_glyphs`) and the same
`difflib` alignment. Nothing here re-implements a measurement that already has
a gate, and nothing here is in the product path: pdflatex is an oracle only.

    python3 tools/visual-oracle/cumulative.py                 # whole corpus
    python3 tools/visual-oracle/cumulative.py --only hw1 --only hw2
    python3 -m unittest discover -s tools/visual-oracle -p 'test_*.py'

Output: `<out>/cumulative.json` (every line profile, step and outlier) and
`<out>/cumulative.md` (the ranked cause table).

## How a step is classified

For every page, aligned word pairs are bucketed into **reference lines** by the
reference baseline. A line's `dy` is the median `dy` of its aligned words, which
is robust to a single mis-placed glyph inside the line. Walking the lines top
to bottom gives a `dy` profile; a **step** is a line whose `dy` differs from the
previous line's by more than `--step-gate` (default 0.05 bp, ten times finer
than the 0.5 bp glyph gate, because a sub-gate step that never comes back is
exactly the failure this tool exists to catch). A step is

* **local** when the next line returns to the previous level (the displacement
  is confined to one line), and
* **cumulative** when it does not: every following line carries it. Its
  `lines_affected` is the count of remaining lines on the page, which is the
  honest size of the defect — a 1.99 bp cumulative step on a 40-line page
  misplaces 40 baselines, a 20 bp local step misplaces one.

Horizontal deltas are reported per line against the line's own median `dx`, so
a whole line shifted right (a wrong indent) is one finding and a single glyph
advance error inside it is another.

## Honest limits

* A page with **reflowed** words (|dx| > 50 bp: the word sits on another line)
  has a different set of lines on the two sides, so its `dy` profile is not a
  measurement of spacing. Those pages are reported as `reflowed`, ranked as
  structural divergences, and excluded from the step table rather than silently
  contributing noise.
* The cause key is the LaTeX source standing between the last word of one line
  and the first word of the next, read from the candidate's own source spans.
  It names the construct at the boundary; it is a triage pointer, not a proof
  of which code is wrong. A boundary with no intervening markup is reported as
  `line-break-within-paragraph`, which usually means the cause is above it.
* Cumulative or local is decided by comparing the median `dy` above the step
  with the median below it, so it needs a few lines on each side to be stable.
  On a page of four or five lines a one-line excursion and a staircase are
  genuinely indistinguishable; such steps are reported with their real
  `lines_affected`, which is small, so they rank low either way.
* Only pages the reference and candidate share are compared; page-count
  differences are reported by `rank.py` and repeated here as `page_count`.
* `dy` is measured from the page top, so a page whose whole body is displaced
  shows one step at its first line. That first-line step is reported as
  `page-origin` and is the drift the previous page handed over.
"""

import argparse
import datetime as _dt
import json
import os
import platform
import re
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(REPO, "tools", "real-world-corpus"))

import fontenv  # noqa: E402
import pdftext  # noqa: E402
import rank  # noqa: E402
import run as corpus  # noqa: E402

GLYPH_GATE = 0.5  # project gate: glyph positions, bp
RULE_GATE = 0.1   # project gate: rules, bp
STEP_GATE = 0.05  # detection floor for a change in a line's dy
SAME_LINE = 0.05  # reference baselines within this are one line


# ----------------------------------------------------------------------------
# line profiles


def lines_from_pairs(pairs, ref_words, cand_words):
    """Bucket aligned (ref, cand) word pairs into reference lines, top first.

    The reference decides what a line is: it is the oracle, and if the
    candidate broke the line elsewhere the words are reflowed and the page is
    excluded upstream anyway.
    """
    buckets = []
    for i, j in sorted(pairs, key=lambda p: (ref_words[p[0]]["y_top"], ref_words[p[0]]["x"])):
        r, c = ref_words[i], cand_words[j]
        if buckets and abs(r["y_top"] - buckets[-1]["y"]) < SAME_LINE:
            cur = buckets[-1]
        else:
            cur = {"y": r["y_top"], "words": []}
            buckets.append(cur)
        cur["words"].append({
            "text": c["text"], "dx": c["x"] - r["x"], "dy": c["y_top"] - r["y_top"],
            "ref_x": r["x"], "source": c.get("source"), "math": bool(c.get("math")),
            "font": c["font"], "ref_font": r["font"],
        })
    out = []
    for b in buckets:
        dxs = [w["dx"] for w in b["words"]]
        dys = [w["dy"] for w in b["words"]]
        out.append({
            "ref_y": round(b["y"], 3),
            "words": len(b["words"]),
            "dy": statistics.median(dys),
            "dx": statistics.median(dxs),
            "dx_max": max(abs(d) for d in dxs),
            "math_fraction": sum(1 for w in b["words"] if w["math"]) / len(b["words"]),
            # Words on one line share one baseline on both sides, so their dy
            # must agree. A spread means the line's words were not all paired
            # with words of the same reference line — which happens around
            # cmex big operators, where the reference PDF has no usable
            # ToUnicode and `pdftext` reads `\int` as the OT1 letter `Z` while
            # the candidate reads U+222B. The operator never aligns and a
            # neighbouring limit can pair across the display instead, inventing
            # an 18 bp "step". Such lines are reported as low confidence rather
            # than ranked.
            "dy_spread": max(dys) - min(dys),
            "text": " ".join(w["text"] for w in b["words"])[:70],
            "_words": b["words"],
        })
    return out


# ----------------------------------------------------------------------------
# cause keys


ENV_RE = re.compile(r"\\(begin|end)\s*\{([^}]{1,40})\}")
CMD_RE = re.compile(r"\\([A-Za-z@]+)")
# Constructs that own vertical space. A boundary gap usually holds several
# tokens (`}` `\end{enumerate}` blank line `\section{...}`); the cause is the
# one that carries a skip, so the gap is reduced to these in source order.
VSPACE_CMDS = {
    "par", "item", "section", "subsection", "subsubsection", "paragraph",
    "chapter", "part", "maketitle", "vspace", "vskip", "bigskip", "medskip",
    "smallskip", "addvspace", "newpage", "clearpage", "pagebreak", "hrule",
    "hline", "caption", "footnote", "tableofcontents", "twocolumn", "onecolumn",
    "appendix", "bibliography", "printbibliography", "noindent", "centering",
}


def source_text(fx, path):
    for d in fx["documents"]:
        if d["path"] == path:
            return d["text"]
    return None


def boundary_gap(fx, prev_line, line):
    """The .tex source standing between two consecutive reference lines."""
    prev_src = next((w["source"] for w in reversed(prev_line["_words"]) if w.get("source")), None)
    next_src = next((w["source"] for w in line["_words"] if w.get("source")), None)
    if not prev_src or not next_src or prev_src.get("path") != next_src.get("path"):
        return None, None, next_src
    text = source_text(fx, prev_src["path"])
    if text is None:
        return None, None, next_src
    a, b = prev_src.get("end_byte", 0), next_src.get("start_byte", 0)
    if b <= a or b - a > 4000:
        return None, prev_src["path"], next_src
    raw = text.encode("utf-8")[a:b].decode("utf-8", "replace")
    return raw, prev_src["path"], next_src


def enclosing_env(fx, path, byte):
    r"""The innermost environment still open at `byte` in `path`.

    Needed because a boundary gap is empty or runs backwards whenever the two
    lines come from source that is not laid out in reading order — the rows of
    a `pmatrix`, the two sides of an `align`, a `\left(…\right)` split over
    lines. Those boundaries carried no cause at all until this fallback, and
    they are precisely the row-pitch defects worth ranking.
    """
    text = source_text(fx, path)
    if text is None:
        return None
    head = text.encode("utf-8")[:byte].decode("utf-8", "replace")
    stack = []
    for m in ENV_RE.finditer(head):
        if m.group(1) == "begin":
            stack.append(m.group(2))
        elif stack and stack[-1] == m.group(2):
            stack.pop()
        elif m.group(2) in stack:
            while stack and stack.pop() != m.group(2):
                pass
    while stack and stack[-1] == "document":
        stack.pop()
    return stack[-1] if stack else None


def cause_key(gap):
    """Reduce a boundary gap to a stable, groupable cause label."""
    if gap is None:
        return "unknown-boundary", []
    tokens = []
    for m in re.finditer(r"\\(begin|end)\s*\{([^}]{1,40})\}|\\([A-Za-z@]+)|\n[ \t]*\n", gap):
        if m.group(1):
            tokens.append(f"\\{m.group(1)}{{{m.group(2)}}}")
        elif m.group(3):
            name = m.group(3)
            if name in VSPACE_CMDS:
                tokens.append("\\" + name)
        else:
            tokens.append("blankline")
    if not tokens:
        return "line-break-within-paragraph", []
    ends = [t for t in tokens if t.startswith("\\end{")]
    begins = [t for t in tokens if t.startswith("\\begin{")]
    if ends and begins:
        key = f"{ends[-1]} -> {begins[0]}"
    elif ends:
        key = f"after {ends[-1]}"
    elif begins:
        key = f"before {begins[0]}"
    else:
        named = [t for t in tokens if t != "blankline"]
        key = named[0] if named else "paragraph-break"
    return key, tokens


# ----------------------------------------------------------------------------
# steps


def steps_for_page(fx, lines, step_gate):
    """Classify every change in the line dy profile as local or cumulative."""
    steps = []
    n = len(lines)
    for i in range(n):
        prev_dy = lines[i - 1]["dy"] if i else 0.0
        step = lines[i]["dy"] - prev_dy
        if abs(step) <= step_gate:
            continue
        if i == 0:
            kind, affected = "page-origin", n
            gap, path, tokens, key = None, None, [], "page-origin"
        else:
            # Cumulative iff the page's *prevailing level* changes here: the
            # median dy below the step differs from the median above it.
            # Comparing only with the neighbouring line calls an alternating
            # row pitch inside a matrix "cumulative" three times over (each
            # row's neighbour is another row, not a return to the paragraph
            # level) and calls the step back down from a one-line excursion a
            # second, opposite cumulative step. Medians on both sides are
            # immune to both.
            head = statistics.median(ln["dy"] for ln in lines[:i])
            tail = statistics.median(ln["dy"] for ln in lines[i:])
            kind = "cumulative" if (i < n - 1 and abs(tail - head) > step_gate) else "local"
            affected = 1 if kind == "local" else n - i
            gap, path, next_src = boundary_gap(fx, lines[i - 1], lines[i])
            key, tokens = cause_key(gap)
            # A boundary with no readable markup between the two lines is not a
            # cause; name the construct the following line sits *inside*
            # instead, which is what a matrix/align row pitch looks like.
            if key in ("unknown-boundary", "line-break-within-paragraph") and next_src:
                env = enclosing_env(fx, next_src.get("path"), next_src.get("start_byte", 0))
                if env:
                    key = f"row pitch inside {{{env}}}"
                elif lines[i]["math_fraction"] >= 0.5:
                    key = "display-math boundary (no environment)"
                elif key == "unknown-boundary" and lines[i - 1]["math_fraction"] >= 0.5:
                    key = "display-math boundary (no environment)"
        low = (lines[i]["dy_spread"] > 5 * step_gate
               or (i and lines[i - 1]["dy_spread"] > 5 * step_gate)
               or (lines[i]["words"] < 2 and lines[i]["math_fraction"] >= 0.5))
        steps.append({
            "confidence": "low" if low else "ok",
            "line_index": i, "ref_y": lines[i]["ref_y"], "step_bp": round(step, 4),
            "dy_before": round(prev_dy, 4), "dy_after": round(lines[i]["dy"], 4),
            "kind": kind, "lines_affected": affected,
            "cause": key, "tokens": tokens[:8],
            "path": path,
            "gap": (gap or "")[:160],
            "before_text": lines[i - 1]["text"] if i else "",
            "after_text": lines[i]["text"],
            "crosses_glyph_gate": abs(step) > GLYPH_GATE,
            "crosses_rule_gate": abs(step) > RULE_GATE,
        })
    return steps


def horizontal_findings(lines, gate):
    """Per-line horizontal shift, and per-word deviation from the line's own dx."""
    line_shifts, word_outliers = [], []
    for idx, ln in enumerate(lines):
        if abs(ln["dx"]) > gate:
            line_shifts.append({"line_index": idx, "ref_y": ln["ref_y"], "dx_bp": round(ln["dx"], 4),
                                "words": ln["words"], "text": ln["text"]})
        for w in ln["_words"]:
            dev = w["dx"] - ln["dx"]
            if abs(dev) > gate:
                word_outliers.append({"line_index": idx, "ref_y": ln["ref_y"], "text": w["text"],
                                      "dx_bp": round(w["dx"], 4), "deviation_bp": round(dev, 4),
                                      "math": w["math"], "font": w["font"], "ref_font": w["ref_font"],
                                      "source": w["source"]})
    return line_shifts, word_outliers


# ----------------------------------------------------------------------------
# aggregation


def aggregate(documents):
    """One row per probable cause, over the whole corpus."""
    causes = {}
    for doc in documents:
        for page in doc.get("pages", []):
            for s in page.get("steps", []):
                if s.get("confidence") == "low":
                    continue
                c = causes.setdefault(s["cause"], {
                    "cause": s["cause"], "kind": set(), "fixtures": set(), "occurrences": 0,
                    "steps": [], "lines_affected": 0, "examples": [],
                })
                c["kind"].add(s["kind"])
                c["fixtures"].add(doc["id"])
                c["occurrences"] += 1
                c["steps"].append(s["step_bp"])
                if s["kind"] != "local":
                    c["lines_affected"] += s["lines_affected"]
                if len(c["examples"]) < 4:
                    c["examples"].append({"fixture": doc["id"], "page": page["page"], "ref_y": s["ref_y"],
                                          "step_bp": s["step_bp"], "kind": s["kind"],
                                          "lines_affected": s["lines_affected"],
                                          "after_text": s["after_text"][:50]})
    rows = []
    for c in causes.values():
        mags = [abs(v) for v in c["steps"]]
        rows.append({
            "cause": c["cause"],
            "kind": "cumulative" if "cumulative" in c["kind"] else ("page-origin" if "page-origin" in c["kind"] else "local"),
            "fixtures": sorted(c["fixtures"]),
            "fixture_count": len(c["fixtures"]),
            "occurrences": c["occurrences"],
            "lines_affected": c["lines_affected"],
            "median_step_bp": round(statistics.median(mags), 4),
            "max_step_bp": round(max(mags), 4),
            "crosses_glyph_gate": max(mags) > GLYPH_GATE,
            "crosses_rule_gate": max(mags) > RULE_GATE,
            "examples": c["examples"],
        })
    # Impact first: total baselines moved, then how many documents show it,
    # then the size. A cause that only ever moves one line ranks under one that
    # moves a hundred, whatever the bp.
    rows.sort(key=lambda r: (-r["lines_affected"], -r["fixture_count"], -r["max_step_bp"]))
    return rows


# ----------------------------------------------------------------------------
# report


def write_report(out_dir, meta, documents, causes, hgroups):
    L = []
    A = L.append
    A(f"# Corpus divergences ranked by blast radius — {meta['stamp']}")
    A("")
    A("Produced by `tools/visual-oracle/cumulative.py` (companion to `rank.py`; same")
    A("producer route, same reference reader, same word grouper, same alignment).")
    A("pdflatex is an oracle only, never in the product path; nothing below is a")
    A("parity claim.")
    A("")
    A(f"- producer: `{meta['render']}` (`{meta['render_sha256'][:16]}…`) -> `{os.path.basename(meta['pdf_exact'])}`")
    A(f"- references: {meta['reference_mode']}")
    A(f"- gates: glyph positions {GLYPH_GATE} bp, rules {RULE_GATE} bp; step detection floor {meta['step_gate']} bp")
    A(f"- host: {meta['platform']}")
    A("")
    A("## Ranked causes (vertical)")
    A("")
    A("`lines affected` is the number of reference baselines the step displaces —")
    A("the whole rest of the page for a cumulative step, one line for a local one.")
    A("")
    A("| rank | probable cause | kind | median step (bp) | max step (bp) | fixtures | occurrences | lines affected | > 0.5 bp gate |")
    A("|---|---|---|---|---|---|---|---|---|")
    for i, r in enumerate(causes, 1):
        A(f"| {i} | `{rank.md(r['cause'])}` | {r['kind']} | {r['median_step_bp']} | {r['max_step_bp']} | "
          f"{r['fixture_count']} | {r['occurrences']} | {r['lines_affected']} | {'yes' if r['crosses_glyph_gate'] else 'no'} |")
    A("")
    A("### Witnesses")
    A("")
    for i, r in enumerate(causes, 1):
        A(f"**{i}. `{rank.md(r['cause'])}`** — {', '.join(r['fixtures'])}")
        for e in r["examples"]:
            A(f"  - `{e['fixture']}` p{e['page']} y={e['ref_y']}: {e['step_bp']:+} bp, {e['kind']}, "
              f"{e['lines_affected']} line(s) after — “{rank.md(e['after_text'])}”")
        A("")
    A("## Horizontal findings")
    A("")
    A("| rank | probable cause | fixtures | occurrences | median |dx| (bp) | max |dx| (bp) | kind |")
    A("|---|---|---|---|---|---|---|")
    for i, r in enumerate(hgroups, 1):
        A(f"| {i} | `{rank.md(r['cause'])}` | {r['fixture_count']} | {r['occurrences']} | "
          f"{r['median_bp']} | {r['max_bp']} | {r['kind']} |")
    A("")
    A("## Low-confidence steps (not ranked)")
    A("")
    A("A line whose aligned words disagree about `dy` was not all paired with one")
    A("reference line. The reference PDF gives cmex big operators no usable text, so")
    A("`\\int`/`\\sum`/`\\prod` and large delimiters never align and a neighbouring")
    A("limit can pair across the display. These are alignment artefacts, not layout.")
    A("")
    A("| fixture | page | step (bp) | cause | line text |")
    A("|---|---|---|---|---|")
    for d in documents:
        for p in d.get("pages", []):
            for s in p.get("steps", []):
                if s.get("confidence") == "low" and abs(s["step_bp"]) > GLYPH_GATE:
                    A(f"| {d['id']} | {p['page']} | {s['step_bp']:+} | `{rank.md(s['cause'])}` | {rank.md(s['after_text'][:40])} |")
    A("")
    A("## Pages that break lines differently (excluded from the step table)")
    A("")
    A("A reflowed word (|dx| > 50 bp) sits on another line than in the reference, so")
    A("the two sides no longer have the same lines and no `dy` profile is measurable.")
    A("These are structural divergences and rank on their own.")
    A("")
    A("| fixture | page | reflowed words | aligned / reference words |")
    A("|---|---|---|---|")
    for d in documents:
        for p in d.get("pages", []):
            if p.get("reflowed_words"):
                A(f"| {d['id']} | {p['page']} | {p['reflowed_words']} | {p['aligned']}/{p['reference_words']} |")
    A("")
    A("## Per document")
    A("")
    A("| fixture | pages ref/cand | status | reflowed pages | cumulative steps | local steps | worst cumulative (bp) |")
    A("|---|---|---|---|---|---|---|")
    for d in documents:
        cum = [s for p in d.get("pages", []) for s in p.get("steps", [])
               if s["kind"] == "cumulative" and s.get("confidence") != "low"]
        loc = [s for p in d.get("pages", []) for s in p.get("steps", [])
               if s["kind"] == "local" and s.get("confidence") != "low"]
        refl = sum(1 for p in d.get("pages", []) if p.get("reflowed_words", 0))
        worst = max((abs(s["step_bp"]) for s in cum), default=0.0)
        A(f"| {d['id']} | {d.get('reference_pages', '—')}/{d.get('candidate_pages', '—')} | "
          f"{d.get('status', '—')} | {refl} | {len(cum)} | {len(loc)} | {round(worst, 3)} |")
    A("")
    with open(os.path.join(out_dir, "cumulative.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(L) + "\n")


def aggregate_horizontal(documents):
    groups = {}
    for doc in documents:
        for page in doc.get("pages", []):
            for s in page.get("line_shifts", []):
                key = "line-indent-or-margin"
                g = groups.setdefault(key, {"cause": key, "kind": "line", "fixtures": set(), "vals": []})
                g["fixtures"].add(doc["id"])
                g["vals"].append(abs(s["dx_bp"]))
            for w in page.get("word_outliers", []):
                key = ("math glyph advance" if w["math"] else
                       ("font substitution " + str(w["ref_font"]) + "->" + str(w["font"])
                        if w["ref_font"] and w["font"] and w["ref_font"][:2] != w["font"][:2] else "text advance / interword glue"))
                g = groups.setdefault(key, {"cause": key, "kind": "word", "fixtures": set(), "vals": []})
                g["fixtures"].add(doc["id"])
                g["vals"].append(abs(w["deviation_bp"]))
    rows = []
    for g in groups.values():
        rows.append({"cause": g["cause"], "kind": g["kind"], "fixtures": sorted(g["fixtures"]),
                     "fixture_count": len(g["fixtures"]), "occurrences": len(g["vals"]),
                     "median_bp": round(statistics.median(g["vals"]), 4), "max_bp": round(max(g["vals"]), 4)})
    rows.sort(key=lambda r: (-r["fixture_count"], -r["occurrences"], -r["max_bp"]))
    return rows


# ----------------------------------------------------------------------------


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--fixtures", default=corpus.DEFAULT_FIXTURES)
    ap.add_argument("--only", action="append", default=[])
    ap.add_argument("--render", default=os.environ.get("FLASHTEX_RENDER",
                    os.path.join(REPO, "crates", "render-pipeline", "target", "release", "flashtex-render")))
    ap.add_argument("--pdf-exact", default=os.environ.get("FLASHTEX_PDF_EXACT",
                    os.path.join(REPO, "crates", "pdf", "target", "release", "flashtex-pdf-exact")))
    ap.add_argument("--font-dirs", default=None)
    ap.add_argument("--tfm-dirs", default=None)
    ap.add_argument("--out", default=None)
    ap.add_argument("--work", default=None)
    ap.add_argument("--step-gate", type=float, default=STEP_GATE)
    ap.add_argument("--glyph-gate", type=float, default=GLYPH_GATE)
    args = ap.parse_args(argv)

    args.font_dirs, args.tfm_dirs = fontenv.resolve_dirs(
        args.font_dirs, args.tfm_dirs, os.path.join(REPO, "apps", "mac", "Fonts"))
    stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")
    out_dir = args.out or os.path.join(REPO, "docs", "evidence", "corpus-fidelity-" + stamp)
    os.makedirs(out_dir, exist_ok=True)
    work = args.work or os.path.join(out_dir, "work")
    os.makedirs(work, exist_ok=True)

    def log(msg):
        print(msg, file=sys.stderr)

    fixtures = rank.corpus_fixtures(args.fixtures, set(args.only))
    meta = {
        "stamp": stamp, "host": platform.node(),
        "platform": f"{platform.system()} {platform.release()} {platform.machine()}",
        "render": args.render, "render_sha256": corpus.sha256_file(args.render),
        "pdf_exact": args.pdf_exact, "pdf_exact_sha256": corpus.sha256_file(args.pdf_exact),
        "font_dirs": args.font_dirs, "tfm_dirs": args.tfm_dirs,
        "step_gate": args.step_gate, "glyph_gate": args.glyph_gate,
        "reference_mode": f"pinned files in `{os.path.relpath(args.fixtures, REPO)}/*/`",
    }

    documents = []
    for fx in fixtures:
        log(fx["id"])
        d = {"id": fx["id"], "pages": []}
        pr = rank.run_render(fx, args.render, args.pdf_exact, args.font_dirs, args.tfm_dirs, work, log)
        d["status"] = pr["status"]
        d["diagnostics"] = len(pr["diagnostics"])
        bad = fontenv.font_diagnostics(pr["diagnostics"])
        if bad:
            d["font_env_failure"] = [b.get("code") for b in bad]
        if not fx["reference"]:
            d["note"] = "no reference PDF"
            documents.append(d)
            continue
        ref_pages = pdftext.page_words(fx["reference"])
        cand_pages = []
        if pr["v2"]:
            with open(pr["v2"], encoding="utf-8") as f:
                cand_pages = rank.v2_words(json.load(f))
        d["reference_pages"] = len(ref_pages)
        d["candidate_pages"] = len(cand_pages)
        for i in range(min(len(ref_pages), len(cand_pages))):
            rw, cw = ref_pages[i]["words"], cand_pages[i]["words"]
            pairs, ref_un, cand_un = rank.align_words(rw, cw)
            page = {"page": i + 1, "aligned": len(pairs), "reference_words": len(rw),
                    "candidate_words": len(cw), "reference_unaligned": ref_un}
            if not pairs:
                page["note"] = "no word aligns; the producer typeset different text"
                d["pages"].append(page)
                continue
            reflowed = sum(1 for a, b in pairs if abs(cw[b]["x"] - rw[a]["x"]) > rank.REFLOW_DX)
            page["reflowed_words"] = reflowed
            lines = lines_from_pairs(pairs, rw, cw)
            page["lines"] = len(lines)
            if reflowed:
                page["note"] = (f"{reflowed} reflowed word(s): the two sides break lines differently, so the "
                                "dy profile is not a spacing measurement")
            else:
                page["steps"] = steps_for_page(fx, lines, args.step_gate)
            ls, wo = horizontal_findings(lines, args.glyph_gate)
            page["line_shifts"] = ls
            page["word_outliers"] = wo[:40]
            page["word_outlier_total"] = len(wo)
            page["dy_profile"] = [round(ln["dy"], 4) for ln in lines]
            d["pages"].append(page)
        documents.append(d)

    causes = aggregate(documents)
    hgroups = aggregate_horizontal(documents)
    with open(os.path.join(out_dir, "cumulative.json"), "w", encoding="utf-8") as f:
        json.dump({"meta": meta, "causes": causes, "horizontal": hgroups, "documents": documents},
                  f, indent=1, ensure_ascii=False)
    write_report(out_dir, meta, documents, causes, hgroups)
    log(f"report: {os.path.join(out_dir, 'cumulative.md')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
