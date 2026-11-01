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

## A step on a page with set glue does not localise its cause

This is the single most important thing to read before using the ranked table.

TeX fits a page by **setting its vertical glue**: every stretchable or
shrinkable skip on the page is scaled by one page-global ratio so the material
comes to exactly `\textheight`. `\tracingoutput` prints that ratio as
`glue set`. A baseline's position on such a page is therefore

    natural position  -  ratio x (shrinkability accumulated above it)

and if the two producers' *natural* page heights differ at all — for any reason,
anywhere on the page, including below the line being looked at — their ratios
differ, and every baseline separates by an amount proportional to the
shrinkability above it. Shrinkable glue is `\abovedisplayskip`, `\topsep`,
`\itemsep`, `\parskip`, the `\@startsection` skips: **exactly the constructs
this tool names its causes after.** So on such a page a step appears at a
`\section`, is proportional to `\section`'s own shrink component, and has
nothing to do with `\section`.

The 2026-09-16 sweep lost a whole finding to this. `\maketitle` was ranked with
90 displaced lines on two fixtures with steps of opposite sign; #755 then
measured the block exact to 0.0009 bp and found both pages shrunk
(`- 0.74042` and `- 0.30122`), each "step" being the difference of two shrink
ratios. Truncating each page so it no longer overflowed collapsed the steps from
-0.5132 bp to +0.0037 and from +0.1755 to +0.0041.

So every page is measured for its glue set and every step on a page with a
**finite-order** set is flagged `cause_not_localised`, ranked separately, and
kept out of the `lines affected` column. A `fil`-order set (a short page whose
`\vfil` absorbs the slack) leaves every finite glue at its natural size and is
*not* flagged — nothing is displaced.

The reference's ratio is pdfTeX's own, read from `\tracingoutput` on a re-run
that is first verified to be the same typesetting as the pinned reference PDF.
**The candidate's ratio is not reported, because the engine does not expose it**
— `render-pipeline`'s `pagebuild::glue_set` computes it and drops it once the
baselines are placed, and the v2 display list carries only the fixed page size.
What the candidate side does expose is its `overfull_vbox` diagnostic, which is
reported instead; a page that overflows on either side is a page whose glue is
being set. Nothing here invents a candidate ratio.

## Honest limits

* Every delta here is measured at `rank.pair_points`: an aligned pair's first
  alphanumeric glyph, which is the content `rank.norm` used to decide the two
  words were the same word. Measuring at the two *word origins* instead reports
  a leading glyph only one producer groups with the word as a placement error —
  that is what made F7 of the 2026-09-16 sweep read as an exact −5.000 bp offset
  on `\bigl(`. It bites the horizontal axis only: `pdftext` starts a new word at
  a new baseline, so a word's alphanumeric anchor is always on the word's own
  baseline and `dy` cannot move with it.
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
* The glue set is read from a **re-run** of the fixture under `\tracingoutput`,
  not from the pinned reference PDF, which carries no such record. The re-run is
  accepted only when it reproduces the pinned PDF byte for byte, or when every
  extracted word sits on the same baseline (`glue_set_source` says which). Where
  `pdflatex` is missing, the fixture fails to build, or the re-run typesets
  differently, the page's glue set is reported `unknown` and its steps are
  flagged conservatively — an unknown glue set is not evidence of a zero one.
* `\showboxdepth` is 3, which reaches the page body box under every class in
  this corpus (depth 2 plainly, depth 3 under `cv`'s `geometry` wrapper). A
  document nesting the body deeper would report its glue set as 0; the box
  heights are printed in `cumulative.json` so that is auditable.
"""

import argparse
import datetime as _dt
import hashlib
import json
import os
import platform
import re
import shutil
import statistics
import subprocess
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
TEXBIN = "/Library/TeX/texbin"


# ----------------------------------------------------------------------------
# page glue set
#
# See the module docstring: a step on a page whose glue is set does not localise
# its cause, so the ratio decides whether a step may be ranked at all.


# `..\vbox(650.43001+0.0)x469.75502, glue set - 0.74042`
BOX_RE = re.compile(r"^(\.*)\\(vbox|hbox)\(([-\d.]+)\+([-\d.]+)\)x([-\d.]+)(?:, glue set ([^\n]*))?$")
TRACE_FIRST_LINE = (r"\tracingoutput=1 \showboxbreadth=5 \showboxdepth=3 \tracingonline=0 \input{%s}")


def parse_glue_set(val):
    r"""`- 0.74042` / `215.83968fil` / `>20000.0fil` / `- 0.65112 []` -> ratio, order.

    pdfTeX writes a shrinking set as `- r` and a stretching one as `r`, with the
    order appended for infinite glue. Only a **finite** set moves the page's
    ordinary skips: when the order is `fil` the `\vfil` at the foot of a short
    page absorbs the whole excess and every finite glue keeps its natural size.

    The trailing ` []` is pdfTeX saying the box's contents were cut off by
    `\showboxdepth`, so it appears on exactly the deepest boxes shown — which
    for a class that nests the body one level further down (`listings-manual`,
    `cv`) is the page body box itself. Rejecting those lines reports those pages
    as unset, which is the wrong answer in the one direction that matters.
    """
    v = re.sub(r"\s*\[\]\s*$", "", val.strip())
    sign = -1.0 if v.startswith("- ") else 1.0
    if sign < 0:
        v = v[2:].strip()
    m = re.match(r"^>?([-\d.]+)(fil+|)$", v)
    if not m:
        return None
    return {"ratio": sign * float(m.group(1)), "order": m.group(2) or "fin", "raw": val.strip()}


def glue_sets_from_log(log):
    r"""Per shipped page, the ratio of the tallest `\vbox` with a finite set.

    The page body box is `\vbox to \textheight`; the other box beside it is the
    12 pt head/foot line, whose own `12.0fil` set places the folio and moves no
    body baseline. Which nesting depth the body sits at depends on the class and
    on `geometry` (2 plainly, 3 under `cv`'s wrapper), so it is selected by
    height, not by depth — and among *finitely* set boxes only, because a
    fil-order set displaces nothing.
    """
    pages, cur = [], None
    for line in log.splitlines():
        if line.startswith("Completed box being shipped out"):
            cur = []
            pages.append(cur)
            continue
        if cur is None:
            continue
        m = BOX_RE.match(line.rstrip())
        if m:
            cur.append({"depth": len(m.group(1)), "kind": m.group(2), "height": float(m.group(3)),
                        "width": float(m.group(5)),
                        "glue": parse_glue_set(m.group(6)) if m.group(6) else None})
    out = []
    for boxes in pages:
        vb = [b for b in boxes if b["kind"] == "vbox" and b["depth"] >= 1]
        fin = [b for b in vb if b["glue"] and b["glue"]["order"] == "fin" and b["glue"]["ratio"]]
        pick = max(fin, key=lambda b: b["height"]) if fin else None
        tallest = max(vb, key=lambda b: b["height"]) if vb else None
        out.append({
            "glue_set": round(pick["glue"]["ratio"], 6) if pick else 0.0,
            "glue_order": pick["glue"]["order"] if pick else ("fil" if any(
                b["glue"] and b["glue"]["order"] != "fin" for b in vb) else "none"),
            "glue_raw": pick["glue"]["raw"] if pick else None,
            "set_box_height": pick["height"] if pick else None,
            "page_box_height": tallest["height"] if tallest else None,
        })
    return out


def _sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1 << 16), b""):
            h.update(b)
    return h.hexdigest()


def _same_typesetting(a, b):
    """Every extracted word on the same baseline, or a reason it is not.

    A pinned reference generated by another TeX Live than the one installed here
    differs in bytes (`/Producer`, object order) without differing in layout.
    That is fine for a *vertical* measurement, so the check is on positions.
    """
    try:
        pa, pb = pdftext.page_words(a), pdftext.page_words(b)
    except Exception as exc:  # noqa: BLE001 - any reader failure is a refusal
        return None, f"pdftext could not read both PDFs: {exc}"
    if len(pa) != len(pb):
        return None, f"page count differs: pinned {len(pa)}, re-run {len(pb)}"
    worst_y = 0.0
    for x, y in zip(pa, pb):
        if len(x["words"]) != len(y["words"]):
            return None, "word count differs on a page"
        for u, v in zip(x["words"], y["words"]):
            if u["text"] != v["text"]:
                return None, f"text differs ({u['text']!r} vs {v['text']!r})"
            worst_y = max(worst_y, abs(u["y_top"] - v["y_top"]))
    if worst_y > SAME_LINE:
        return None, f"baselines differ by up to {worst_y:.4f} bp"
    return worst_y, None


def reference_glue_sets(fx, texbin, work, log):
    r"""pdfTeX's own `glue set` per page for `fx`'s pinned reference.

    The pinned PDF records no glue set, so the fixture is re-run under
    `\tracingoutput` and the log is read — but only after the re-run is shown to
    be the *same typesetting* as the pinned PDF. Returns
    `(pages, source, error)`; `pages` is None whenever anything at all was off,
    because an unknown glue set must not read as a zero one.
    """
    pdflatex = os.path.join(texbin, "pdflatex")
    if not os.path.isfile(pdflatex):
        return None, None, f"no pdflatex at {pdflatex}"
    if not fx.get("reference"):
        return None, None, "no reference PDF"
    wd = os.path.join(work, "glueset", fx["id"])
    if os.path.isdir(wd):
        shutil.rmtree(wd)
    shutil.copytree(fx["dir"], wd)
    pinned = os.path.join(work, "glueset", fx["id"] + "-pinned.pdf")
    shutil.copy(fx["reference"], pinned)
    for name in os.listdir(wd):
        if name.endswith(".pdf"):
            os.remove(os.path.join(wd, name))
    stem = fx["entry"][:-4] if fx["entry"].endswith(".tex") else fx["entry"]
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    argv = [pdflatex, "-interaction=nonstopmode", "-file-line-error", "-jobname=" + stem,
            TRACE_FIRST_LINE % fx["entry"]]
    code = None
    for _ in range(3):  # \tableofcontents and \label need the aux to settle
        try:
            pr = subprocess.run(argv, cwd=wd, env=env, capture_output=True, timeout=300)
        except (OSError, subprocess.SubprocessError) as exc:
            return None, None, f"pdflatex failed to run: {exc}"
        code = pr.returncode
    logp, pdfp = os.path.join(wd, stem + ".log"), os.path.join(wd, stem + ".pdf")
    if not os.path.isfile(logp) or not os.path.isfile(pdfp):
        return None, None, f"pdflatex produced no log or PDF (exit {code})"
    with open(logp, encoding="utf-8", errors="replace") as f:
        pages = glue_sets_from_log(f.read())
    if not pages:
        return None, None, "the log records no shipped page"
    if _sha256(pinned) == _sha256(pdfp):
        return pages, "re-run under \\tracingoutput, byte-identical to the pinned reference PDF", None
    worst, why = _same_typesetting(pinned, pdfp)
    if why:
        return None, None, "the re-run is not the pinned typesetting: " + why
    log(f"  glue set {fx['id']}: re-run differs in bytes, every baseline within {worst:.4f} bp")
    return pages, (f"re-run under \\tracingoutput; bytes differ from the pinned reference "
                   f"(another TeX Live), every baseline within {worst:.4f} bp"), None


OVERFULL_PAGE_RE = re.compile(r"page (\d+):")


def candidate_overfull_pages(diagnostics):
    """`{page: [message]}` from the engine's own `overfull_vbox` diagnostics.

    The engine computes a page's glue set (`render-pipeline`
    `pagebuild::glue_set`) and discards it; the display list carries only the
    fixed page size, so there is no candidate ratio to report. This is the one
    signal it does publish that a page's material did not fit, and it covers the
    overfull direction only — there is no `underfull_vbox`.
    """
    out = {}
    for d in diagnostics or []:
        if d.get("code") != "overfull_vbox":
            continue
        m = OVERFULL_PAGE_RE.search(d.get("message") or "")
        if m:
            out.setdefault(int(m.group(1)), []).append(d.get("message"))
    return out


# ----------------------------------------------------------------------------
# line profiles


def measured_pairs(pairs, ref_words, cand_words):
    """`(ref_index, cand_index, (rx, ry), (cx, cy))` for every aligned pair.

    The two points come from `rank.pair_points`, which anchors both sides at
    their first alphanumeric glyph — the content `rank.norm` used to decide the
    pair was a pair in the first place. Measuring at the two *word origins*
    instead reports a leading glyph only one side carries as a placement error:
    pdfTeX emits `\\bigl(` from cmex10 on its own raised baseline, so
    `pdftext.words_from_glyphs` reads the reference as `(` + `A...` and the
    candidate as one `(A...`, and the two origins are a delimiter's advance
    apart. That is what made F7 of the 2026-09-16 sweep read as an exact
    -5.000 bp offset (see `rank.anchor`).

    This matters here on both axes, not just `dx`: a word's `y_top` is its
    *first* glyph's baseline, so a pair segmented differently was also being
    bucketed into a line, and having its `dy` measured, at a baseline the
    aligned content never sat on.
    """
    out = []
    for i, j in pairs:
        rp, cp = rank.pair_points(ref_words[i], cand_words[j])
        out.append((i, j, rp, cp))
    return out


def lines_from_pairs(pairs, ref_words, cand_words):
    """Bucket aligned (ref, cand) word pairs into reference lines, top first.

    The reference decides what a line is: it is the oracle, and if the
    candidate broke the line elsewhere the words are reflowed and the page is
    excluded upstream anyway.

    Both the bucketing and the deltas use `measured_pairs`' anchored points, so
    a line's `ref_y` and its words' `dy` are on one coordinate and cannot
    disagree.
    """
    buckets = []
    measured = measured_pairs(pairs, ref_words, cand_words)
    for i, j, (rx, ry), (cx, cy) in sorted(measured, key=lambda m: (m[2][1], m[2][0])):
        r, c = ref_words[i], cand_words[j]
        if buckets and abs(ry - buckets[-1]["y"]) < SAME_LINE:
            cur = buckets[-1]
        else:
            cur = {"y": ry, "words": []}
            buckets.append(cur)
        cur["words"].append({
            "text": c["text"], "dx": cx - rx, "dy": cy - ry,
            "ref_x": rx, "source": c.get("source"), "math": bool(c.get("math")),
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


def localisation(glue):
    r"""Whether a step on this page may be attributed to its own position.

    `glue` is the page's measured glue-set record, or None when it could not be
    measured. Returns `(not_localised, why)`.
    """
    if glue is None:
        return True, ("the page's glue set could not be measured, so it is not known "
                      "whether the page's baselines were displaced page-globally")
    if glue.get("glue_order") == "fin" and glue.get("glue_set"):
        return True, (f"pdfTeX set this page's glue by {glue['glue_raw']}: the ratio is page-global, "
                      "so a step here is the difference of two shrink ratios wherever the two sides' "
                      "natural page heights diverge — above this line as readily as at it")
    return False, None


def steps_for_page(fx, lines, step_gate, glue=None):
    """Classify every change in the line dy profile as local or cumulative."""
    not_localised, why = localisation(glue)
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
            # The cause label above names the construct standing at this
            # position. On a page whose glue is set, position does not imply
            # cause — see the module docstring — so the label is kept (it is
            # still where the step surfaced) and disclaimed here.
            "cause_not_localised": not_localised,
            "cause_not_localised_why": why,
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
    """One row per probable cause, over the whole corpus.

    `lines_affected` counts only steps whose cause **is** localised. A step on a
    page with set glue displaces just as many baselines, but it does not say
    which construct displaced them, so counting it here would credit the
    construct standing at its position with a defect that may sit anywhere on
    the page — which is exactly how the 2026-09-16 sweep ranked `\\maketitle`
    fifth with 90 lines that belonged to two other findings. Those lines are
    kept, separately, in `lines_not_localised`.
    """
    causes = {}
    for doc in documents:
        for page in doc.get("pages", []):
            for s in page.get("steps", []):
                if s.get("confidence") == "low":
                    continue
                flagged = bool(s.get("cause_not_localised"))
                c = causes.setdefault(s["cause"], {
                    "cause": s["cause"], "kind": set(), "fixtures": set(), "occurrences": 0,
                    "steps": [], "clean_steps": [], "lines_affected": 0,
                    "lines_not_localised": 0, "not_localised_occurrences": 0,
                    "not_localised_fixtures": set(), "examples": [],
                })
                c["kind"].add(s["kind"])
                c["fixtures"].add(doc["id"])
                c["occurrences"] += 1
                c["steps"].append(s["step_bp"])
                if flagged:
                    c["not_localised_occurrences"] += 1
                    c["not_localised_fixtures"].add(doc["id"])
                else:
                    c["clean_steps"].append(s["step_bp"])
                if s["kind"] != "local":
                    if flagged:
                        c["lines_not_localised"] += s["lines_affected"]
                    else:
                        c["lines_affected"] += s["lines_affected"]
                if len(c["examples"]) < 4:
                    c["examples"].append({"fixture": doc["id"], "page": page["page"], "ref_y": s["ref_y"],
                                          "step_bp": s["step_bp"], "kind": s["kind"],
                                          "lines_affected": s["lines_affected"],
                                          "cause_not_localised": flagged,
                                          "after_text": s["after_text"][:50]})
    rows = []
    for c in causes.values():
        mags = [abs(v) for v in c["steps"]]
        clean = [abs(v) for v in c["clean_steps"]]
        rows.append({
            "cause": c["cause"],
            "kind": "cumulative" if "cumulative" in c["kind"] else ("page-origin" if "page-origin" in c["kind"] else "local"),
            "fixtures": sorted(c["fixtures"]),
            "fixture_count": len(c["fixtures"]),
            "occurrences": c["occurrences"],
            "lines_affected": c["lines_affected"],
            "lines_not_localised": c["lines_not_localised"],
            "not_localised_occurrences": c["not_localised_occurrences"],
            "not_localised_fixtures": sorted(c["not_localised_fixtures"]),
            # A cause every one of whose witnesses is on a page with set glue is
            # not evidence about that cause at all.
            "only_on_set_glue_pages": c["not_localised_occurrences"] == c["occurrences"],
            "attributable_fixture_count": len(c["fixtures"] - c["not_localised_fixtures"]),
            "median_step_bp": round(statistics.median(mags), 4),
            "max_step_bp": round(max(mags), 4),
            "median_attributable_step_bp": round(statistics.median(clean), 4) if clean else None,
            "max_attributable_step_bp": round(max(clean), 4) if clean else None,
            "crosses_glyph_gate": max(mags) > GLYPH_GATE,
            "crosses_rule_gate": max(mags) > RULE_GATE,
            "examples": c["examples"],
        })
    # Impact first: baselines moved *by this cause*, then how many documents
    # show it there, then the size. A cause that only ever moves one line ranks
    # under one that moves a hundred, whatever the bp — and a cause whose every
    # witness sits on a page with set glue has moved no baseline it can be
    # shown to own, so it ranks at the bottom whatever its raw numbers.
    rows.sort(key=lambda r: (-r["lines_affected"], -r["attributable_fixture_count"],
                             -(r["max_attributable_step_bp"] or 0.0), -r["lines_not_localised"]))
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
    A(f"- page glue set: read from `{meta['texbin']}/pdflatex` under `\\tracingoutput` "
      "on a re-run verified against the pinned reference")
    A(f"- host: {meta['platform']}")
    A("")
    A("## Page glue set, and what it does to attribution")
    A("")
    A("TeX fits a page by scaling every stretchable or shrinkable skip on it by one")
    A("**page-global** ratio. On a page with a finite-order set, a baseline sits at")
    A("its natural position minus `ratio x (shrinkability above it)`, so if the two")
    A("producers' natural page heights differ *anywhere*, every baseline separates in")
    A("proportion to the shrinkable glue above it — and shrinkable glue is")
    A("`\\abovedisplayskip`, `\\topsep`, `\\itemsep`, `\\parskip`, the `\\@startsection`")
    A("skips, which is precisely what this tool names its causes after.")
    A("")
    A("**A step on such a page therefore does not localise its cause.** Those steps")
    A("are listed separately below and excluded from `lines affected`; they are real")
    A("displacements, but the construct at their position is not shown to have caused")
    A("them. A `fil`-order set (a short page whose `\\vfil` absorbs the slack) leaves")
    A("every finite glue at natural size and is **not** flagged.")
    A("")
    A("The reference ratio is pdfTeX's own `\\tracingoutput` figure. **The candidate's")
    A("is not reported: the engine does not expose it** — `render-pipeline`'s")
    A("`pagebuild::glue_set` computes the ratio and drops it once baselines are placed,")
    A("and the v2 display list carries only the fixed page size. The engine's own")
    A("`overfull_vbox` diagnostic is reported in its place. No candidate ratio is")
    A("invented here.")
    A("")
    A("| fixture | page | reference glue set | order | displaces baselines? | candidate overfull | source |")
    A("|---|---|---|---|---|---|---|")
    for d in documents:
        for p in d.get("pages", []):
            if "reference_glue_set" not in p:
                continue
            g = p["reference_glue_set"]
            A(f"| {d['id']} | {p['page']} | {p['reference_glue_raw'] or (g if g is not None else 'unknown')} | "
              f"{p['reference_glue_order'] or '—'} | {'YES' if p.get('cause_not_localised') else 'no'} | "
              f"{len(p.get('candidate_overfull') or []) or '—'} | {rank.md((p.get('glue_set_source') or '')[:60])} |")
    A("")
    A("## Ranked causes (vertical)")
    A("")
    A("`lines affected` is the number of reference baselines displaced by steps whose")
    A("cause **is** localised — the whole rest of the page for a cumulative step, one")
    A("line for a local one. `lines not localised` is the same count for steps on a")
    A("page with set glue: displaced, but not by anything this row names.")
    A("")
    A("| rank | probable cause | kind | median step (bp) | max step (bp) | fixtures | occurrences | lines affected | lines not localised | > 0.5 bp gate |")
    A("|---|---|---|---|---|---|---|---|---|---|")
    for i, r in enumerate(causes, 1):
        flag = " **(every witness on a page with set glue)**" if r["only_on_set_glue_pages"] else ""
        A(f"| {i} | `{rank.md(r['cause'])}`{flag} | {r['kind']} | {r['median_step_bp']} | {r['max_step_bp']} | "
          f"{r['fixture_count']} | {r['occurrences']} | {r['lines_affected']} | {r['lines_not_localised']} | "
          f"{'yes' if r['crosses_glyph_gate'] else 'no'} |")
    A("")
    A("### Witnesses")
    A("")
    for i, r in enumerate(causes, 1):
        A(f"**{i}. `{rank.md(r['cause'])}`** — {', '.join(r['fixtures'])}")
        if r["only_on_set_glue_pages"]:
            A("  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so "
              "this row is not evidence that this construct is where the defect is.")
        for e in r["examples"]:
            A(f"  - `{e['fixture']}` p{e['page']} y={e['ref_y']}: {e['step_bp']:+} bp, {e['kind']}, "
              f"{e['lines_affected']} line(s) after{' — CAUSE NOT LOCALISED' if e['cause_not_localised'] else ''}"
              f" — “{rank.md(e['after_text'])}”")
        A("")
    A("## Steps whose cause is not localised (excluded from the ranking above)")
    A("")
    A("Each of these displaces the baselines it says it does. What it does **not** do")
    A("is name the construct responsible: the page's glue set is global, so the step")
    A("surfaces where the two sides' ratios diverge, not where the underlying error")
    A("is. Re-measure any of these on a page that does not overflow before filing it.")
    A("")
    A("| fixture | page | glue set | step (bp) | kind | lines | cause as labelled | line text |")
    A("|---|---|---|---|---|---|---|---|")
    for d in documents:
        for p in d.get("pages", []):
            for s in p.get("steps", []):
                if not s.get("cause_not_localised") or s.get("confidence") == "low":
                    continue
                A(f"| {d['id']} | {p['page']} | {p.get('reference_glue_raw') or 'unknown'} | {s['step_bp']:+} | "
                  f"{s['kind']} | {s['lines_affected']} | `{rank.md(s['cause'])}` | {rank.md(s['after_text'][:40])} |")
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
    A("| fixture | pages ref/cand | status | reflowed pages | pages with set glue | cumulative steps | of those, not localised | local steps | worst localised cumulative (bp) |")
    A("|---|---|---|---|---|---|---|---|---|")
    for d in documents:
        cum = [s for p in d.get("pages", []) for s in p.get("steps", [])
               if s["kind"] == "cumulative" and s.get("confidence") != "low"]
        loc = [s for p in d.get("pages", []) for s in p.get("steps", [])
               if s["kind"] == "local" and s.get("confidence") != "low"]
        nl = [s for s in cum if s.get("cause_not_localised")]
        refl = sum(1 for p in d.get("pages", []) if p.get("reflowed_words", 0))
        setg = sum(1 for p in d.get("pages", []) if p.get("cause_not_localised"))
        worst = max((abs(s["step_bp"]) for s in cum if not s.get("cause_not_localised")), default=0.0)
        A(f"| {d['id']} | {d.get('reference_pages', '—')}/{d.get('candidate_pages', '—')} | "
          f"{d.get('status', '—')} | {refl} | {setg} | {len(cum)} | {len(nl)} | {len(loc)} | {round(worst, 3)} |")
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
    ap.add_argument("--texbin", default=os.environ.get("FLASHTEX_TEXBIN", TEXBIN),
                    help="directory holding the pdflatex that reads back each page's glue set")
    ap.add_argument("--no-glue-set", action="store_true",
                    help="skip the glue-set re-runs; every step is then flagged as not localised, "
                         "because an unmeasured glue set is not a zero one")
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
        "step_gate": args.step_gate, "glyph_gate": args.glyph_gate, "texbin": args.texbin,
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
        if args.no_glue_set:
            glue_pages, glue_source, glue_error = None, None, "--no-glue-set"
        else:
            glue_pages, glue_source, glue_error = reference_glue_sets(fx, args.texbin, work, log)
        d["glue_set_source"] = glue_source
        d["glue_set_error"] = glue_error
        overfull = candidate_overfull_pages(pr["diagnostics"])
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
            glue = glue_pages[i] if glue_pages and i < len(glue_pages) else None
            page["reference_glue_set"] = glue["glue_set"] if glue else None
            page["reference_glue_order"] = glue["glue_order"] if glue else None
            page["reference_glue_raw"] = glue["glue_raw"] if glue else None
            page["reference_glue_box"] = ({"set_box_height": glue["set_box_height"],
                                           "page_box_height": glue["page_box_height"]} if glue else None)
            page["glue_set_source"] = glue_source or ("unavailable: " + str(glue_error))
            # No candidate ratio exists to report: `pagebuild::glue_set`
            # computes one and drops it, and the v2 display list carries only
            # the fixed page size. Reporting the reference's and the engine's
            # own overflow record is the honest substitute for inventing one.
            page["candidate_glue_set"] = None
            page["candidate_glue_set_note"] = (
                "not exposed by the engine: render-pipeline `pagebuild::glue_set` computes the page "
                "ratio and discards it once baselines are placed, and the v2 display list carries "
                "only the fixed page size")
            page["candidate_overfull"] = overfull.get(i + 1, [])
            page["cause_not_localised"], page["cause_not_localised_why"] = localisation(glue)
            if not pairs:
                page["note"] = "no word aligns; the producer typeset different text"
                d["pages"].append(page)
                continue
            # Anchored points, as everywhere else here: a word origin that only
            # one side carries a leading delimiter at is not a position.
            reflowed = sum(1 for _i, _j, rp, cp in measured_pairs(pairs, rw, cw)
                           if abs(cp[0] - rp[0]) > rank.REFLOW_DX)
            page["reflowed_words"] = reflowed
            lines = lines_from_pairs(pairs, rw, cw)
            page["lines"] = len(lines)
            if reflowed:
                page["note"] = (f"{reflowed} reflowed word(s): the two sides break lines differently, so the "
                                "dy profile is not a spacing measurement")
            else:
                page["steps"] = steps_for_page(fx, lines, args.step_gate, glue)
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
