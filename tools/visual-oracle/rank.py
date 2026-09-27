#!/usr/bin/env python3
"""Per-page geometry + raster ranking of the exact route against pdfLaTeX.

Owner: lane mac-visual-oracle-2 (parent mac-claude-a). Python 3 standard
library only. Nothing here is in the product path: the pinned reference PDFs
under fixtures/real-world/*/ were made by MacTeX 2026 pdflatex (oracle only);
the producer under test is `flashtex-render --v2` (pinned build) followed by
`flashtex-pdf-exact from-v2` (the Mac shell's export route).

For every fixture and page the harness reports

  geometry  word boxes of the rendering-v2 display list (one `glyph_run` per
            word, origin of the first glyph, baseline from the page top, in
            bp) aligned by text (difflib) against the reference PDF's word
            positions, which `pdftext.py` replays from the content stream with
            the fonts' own /Widths, then repaired by position
            (`reanchor_pairs`): text-only alignment has no notion of
            position, so a page with many identical short tokens (a table of
            contents' section/page numbers) can pair a candidate token with a
            same-text reference token on another line while the two tokens'
            true, close partners sit unaligned on both sides; any such pair
            with an unpaired same-text neighbour within 50 bp / 0.6 line
            heights is re-paired to it, and a pair with no such neighbour (a
            genuine reflow) is left as difflib found it. Per page:
            aligned/unaligned counts, the median shift, mean and max
            |dx|/|dy|, and the N largest positional deltas with the candidate
            word's source span (path:start-end) and an excerpt of the .tex
            source;
  pixels    differing pixels at 144 dpi between the two rasters (Ghostscript
            or pdftoppm, whichever exists; the report names it), reusing the
            real-world-corpus comparator;
  owner     the crate to look at first for each top item — a documented
            heuristic (`owner_for_word`): a producer diagnostic whose source
            span overlaps the word's span decides (owner table of
            tools/real-world-corpus/run.py); else a math font means
            crates/math-layout; else a delta shared by the whole page (the
            median shift) means the page builder, an isolated delta the
            paragraph layout — both crates/render-pipeline.

Pages are ranked worst first: missing pages, then pages whose words cannot be
aligned at all, then by the largest positional delta, then by differing
pixels. Whole-PDF byte equality is explicitly not measured. Nothing in the
report is a parity claim; the numbers are a floor to drive down.

Units: both sides are in bp (PDF user space; 1 bp = 1.00375 TeX pt). The
reference word origin is the first glyph's origin, the candidate's is the
first glyph's `origin_x`/`baseline_y`; the two agree by definition of the
from-v2 route, which writes those decimals verbatim.
"""

import argparse
import datetime as _dt
import difflib
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
import thumbs  # noqa: E402
import run as corpus  # noqa: E402  (tools/real-world-corpus/run.py)

DPI = corpus.DPI
Q = float(2 ** 20)  # bp_2pow20 -> bp

REFLOW_DX = 50.0  # |dx| in bp beyond which an aligned word is counted as moved to another line
ANCHOR_DY_LINE_FACTOR = 0.6  # of the page's estimated line height; see reanchor_pairs
DEFAULT_TEXBIN = corpus.DEFAULT_TEXBIN

# pdflatex-lm preamble of tests/visual-corpus/harness (origin/agent/mac-visual-oracle/reference-raster),
# used for --harness-fixtures: the fixtures' own preamble is replaced by this one on both sides.
HARNESS_PREAMBLE = (
    "\\documentclass[12pt]{article}\n\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n"
    "\\usepackage[margin=1in]{geometry}\n\\setlength{\\parindent}{0pt}\n\\setcounter{secnumdepth}{0}\n"
    "\\pagestyle{empty}\n"
)


# ----------------------------------------------------------------------------
# candidate words from the display list


def is_math_font(font):
    name = (font or {}).get("postscript_name", "") or ""
    return "math" in name.lower() or bool(re.search(r"(^|[^a-z])(cmmi|cmsy|cmex|lmmi|lmsy|lmex)", name.lower()))


def v2_page_glyphs(display_list):
    """Per page: every glyph of the rendering-v2 display list, in paint
    order, in the record shape `pdftext.page_glyphs` returns for the
    reference (`text`, `x`, `y_top`, `advance`, `size`, `font`, `bt`), plus
    the per-glyph facts only the candidate has: `math` (drawn from a math
    font), `sources` (the cluster's source spans) and `gid`.

    `text` is the glyph's cluster text; when several glyphs share one
    cluster each carries the whole text, as `v2_words` always had it.
    `cluster` = (run index on the page, cluster index) names the cluster, so
    a consumer that must count each cluster's text once can.
    """
    pl = display_list["payload"]
    unit = pl.get("coordinate_unit")
    if unit != "bp_2pow20":
        raise ValueError(f"unexpected coordinate_unit {unit!r}")
    fonts = {f["font_id"]: f for f in pl.get("fonts", [])}
    pages = []
    for page in pl["pages"]:
        glyphs = []
        for ri, item in enumerate(page.get("items", [])):
            if item.get("kind") != "glyph_run" or not item.get("glyphs"):
                continue
            # Cluster ranges are UTF-8 byte offsets, not str indices: slicing
            # the str drifts after the first non-ASCII character (`∈`, a
            # U+2212 minus) and hands the next glyphs their neighbours' text.
            text = (item.get("text") or "").encode("utf-8")
            font = fonts.get(item.get("font_id"))
            name = (font or {}).get("postscript_name", item.get("font_id", "")[:12])
            math = is_math_font(font)
            size = item.get("font_size", 0) / Q
            clusters = item.get("clusters") or []
            for gi, g in enumerate(item["glyphs"]):
                ci = g.get("cluster", gi)
                cluster = clusters[ci] if ci < len(clusters) else None
                ctext = text[cluster["text_start_byte"]:cluster["text_end_byte"]].decode("utf-8", "replace") if cluster else ""
                glyphs.append({
                    "text": ctext,
                    "x": g["origin_x"] / Q,
                    "y_top": g["baseline_y"] / Q,
                    "advance": g["advance_x"] / Q,
                    "size": size,
                    "font": name,
                    # One text object per page: the candidate has no `BT`
                    # structure to respect, so only the baseline and the gap
                    # decide, which is what this rule is really about.
                    "bt": 0,
                    "math": math,
                    "sources": (cluster or {}).get("sources") or [],
                    "gid": g.get("gid"),
                    "cluster": (ri, ci),
                })
        pages.append({"page": page.get("number"), "glyphs": glyphs,
                      "width": page.get("width", 0) / Q, "height": page.get("height", 0) / Q})
    return pages


def v2_words(display_list):
    """Per page: the candidate's words, grouped by **the same rule the
    reference side uses** (`pdftext.words_from_glyphs`).

    The display list's unit of output is the `glyph_run`, which is a
    typesetting artefact, not a word: the pipeline starts a new run whenever
    the face changes, so one siunitx `S` cell arrives as three runs
    (`1`, `.`, `234`). This function used to emit one word per run and split
    only *within* a run, never joining across them — while `pdftext` joined
    freely across `Tf` switches, because a font change does not end a word.
    pdfTeX sets the same cell as three `Tf`-switched runs in one text object
    and the reference therefore read `1.234`, one word, against our three.

    That is a disagreement between the two halves of one measurement, and it
    is what made PR #331 (siunitx `S`/`s` columns) read as a regression: on
    `lab-report` page 2 the candidate word count went 78 -> 118 and aligned
    70 -> 50 with the ink unchanged and correctly placed. Feeding the
    candidate's glyphs through the reference's own grouper removes the
    disagreement by construction — there is now one definition of a word in
    this tool, in one function.

    `math` and the source span are per-glyph facts that the grouper does not
    know about, so they are carried across it through `glyph_index`. The
    glyphs themselves come from `v2_page_glyphs`, which the parity
    scoreboard (`tools/parity`) reads too.
    """
    pages = []
    for gp in v2_page_glyphs(display_list):
        glyphs = [{k: g[k] for k in ("text", "x", "y_top", "advance", "size", "font", "bt")} for g in gp["glyphs"]]
        meta = [(g["math"], g["sources"]) for g in gp["glyphs"]]
        words = pdftext.words_from_glyphs(glyphs)
        for w in words:
            i, n = w.pop("glyph_index"), w["glyphs"]
            run = meta[i:i + n]
            w["math"] = any(m for m, _ in run)
            srcs = [s[0] for _, s in run if s]
            w["source"] = None
            if srcs:
                w["source"] = {"path": srcs[0]["path"],
                               "start_byte": min(s["start_byte"] for s in srcs),
                               "end_byte": max(s["end_byte"] for s in srcs)}
        pages.append({"page": gp["page"], "words": words, "width": gp["width"], "height": gp["height"]})
    return pages


# ----------------------------------------------------------------------------
# alignment


def norm(text):
    # U+2212 MINUS SIGN and U+002D HYPHEN-MINUS are one word for alignment.
    # pdfTeX's own PDFs extract a math minus as U+2212 (PyMuPDF), and so does
    # the candidate; pdftext's glyph-name table maps `minus` to "-", and the
    # pinned references were made with it.
    folded = text.lower().replace("−", "-")
    t = re.sub(r"[^\w]", "", folded.replace("?", ""))
    return t or folded


def align_words(ref_words, cand_words):
    """difflib alignment on normalised word text; returns list of
    (ref_index, cand_index) pairs and the unaligned counts."""
    a = [norm(w["text"]) for w in ref_words]
    b = [norm(w["text"]) for w in cand_words]
    sm = difflib.SequenceMatcher(None, a, b, autojunk=False)
    pairs = []
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == "equal":
            pairs.extend((i, j1 + (i - i1)) for i in range(i1, i2))
    return pairs, len(ref_words) - len(pairs), len(cand_words) - len(pairs)


def overlaps(span, sources):
    if not span:
        return False
    for s in sources or []:
        if s.get("path") == span["path"] and s.get("start_byte", 0) < span["end_byte"] and s.get("end_byte", 0) > span["start_byte"]:
            return True
    return False


def owner_for_word(word, delta, page_median, diags):
    """Heuristic owner per top item (see module docstring)."""
    span = word.get("source")
    for d in diags:
        srcs = d.get("sources") or ([d["source"]] if d.get("source") else [])
        if overlaps(span, srcs):
            return corpus.owner_for(d) + f" — diagnostic `{d.get('message', '')[:80]}`"
    if word.get("math"):
        return "crates/math-layout (FT-020) via crates/render-pipeline (mac-claude-a) — math word"
    mdx, mdy = page_median
    if abs(delta[0] - mdx) < 0.05 and abs(delta[1] - mdy) < 0.05 and (abs(mdx) > 0.05 or abs(mdy) > 0.05):
        return "crates/render-pipeline (mac-claude-a) — page builder / document-style: delta equals the page's median shift"
    if abs(delta[0]) > REFLOW_DX:
        return "crates/render-pipeline (mac-claude-a) — paragraph-layout line breaking: word sits on a different line than in the reference"
    return "crates/render-pipeline (mac-claude-a) — paragraph/line layout: isolated delta"


def anchor(word):
    """The point a word's position is measured at: its first alphanumeric
    glyph, else its first glyph.

    `norm` decides *identity* on a word's alphanumeric content, so position
    has to be measured there too. Otherwise a pair the two sides segment
    differently — `(A` here against a bare `A` in the reference, because
    pdfTeX emits the cmex10 `\\bigl(` on its own baseline while our
    LatinModernMath variant sits on the math baseline — is compared at two
    points that are a delimiter's advance apart, and reports that advance as
    a placement error. That is what made F7 of the 2026-09-16 corpus sweep
    read as an exact −5.000 bp offset on `\\bigl(`/`\\Bigl[`: 4.996 bp is the
    width of `\\big(` at 10.95 pt and 5.149 bp the width of `\\Big[`, and the
    delimiter's ink is in fact within 0.09 bp of the reference's.

    When both sides carry the same leading punctuation the anchor moves both
    by the same amount and `dx`/`dy` are unchanged, so this only touches
    pairs the two producers segment differently.
    """
    x, y = word.get("x_alnum"), word.get("y_alnum")
    if x is None or y is None:
        return word["x"], word["y_top"]
    return x, y


def pair_points(ref_word, cand_word):
    """The two points an aligned pair is compared at.

    Both sides are anchored the same way. A pair where only one side has an
    alphanumeric glyph falls back to the word origin on *both*, which is
    also what `norm` falls back to when it has no alphanumeric content.
    """
    if (ref_word.get("x_alnum") is None) != (cand_word.get("x_alnum") is None):
        return (ref_word["x"], ref_word["y_top"]), (cand_word["x"], cand_word["y_top"])
    return anchor(ref_word), anchor(cand_word)


def _line_height(words):
    """A rough estimate of this page's line height (bp), used only to scale
    `reanchor_pairs`'s vertical threshold. TeX's default `\\baselineskip` is
    about 1.2x the font size; a page with no usable `size` (an empty page, or
    a synthetic test fixture) falls back to a plain 10 pt document's 12 bp."""
    sizes = [w["size"] for w in words if w.get("size")]
    if not sizes:
        return 12.0
    return statistics.median(sizes) * 1.2


def reanchor_pairs(ref_words, cand_words, pairs, ref_free, cand_free, dx_threshold, dy_threshold):
    """Fix difflib's blind pairing of interchangeable repeated short tokens.

    `align_words` aligns on text alone (a difflib longest-common-subsequence
    over the normalised word strings); it has no notion of position. On a
    table of contents, page numbers and section numbers are short, heavily
    repeated tokens (`2`, `3`, ...) interleaved with titles the two sides may
    segment or order slightly differently, and the LCS then has many
    equally-valid alignments to choose from. It can pair a candidate token
    with a same-text reference token several lines away while the two
    tokens' true, adjacent partners are left out of `pairs` entirely (in
    `ref_free`/`cand_free`) — this is what made `hyperref-toc` page 1 read as
    a −446 bp/+13.57 bp outlier when every one of its 505 words was in fact
    within 0.02 bp of the reference (docs/evidence/visual-oracle-
    2026-09-19T210411Z, rank 2): the trailing `2` of one TOC line (the page
    number, x=534.6) was paired with the leading `2` of the next line's
    section number (x=88.3, one line down).

    This runs after `align_words` and repairs exactly that: for every pair
    whose measured points (`pair_points`) disagree by more than
    `dx_threshold` in x or `dy_threshold` in y ("bad"), gather every
    same-normalised-text word that is either the ref/cand half of another bad
    pair, or sits unpaired in `ref_free`/`cand_free`. Within each such
    text-keyed group, greedily match ref/cand words nearest first, accepting
    only matches within (dx_threshold, dy_threshold) — a small per-text
    nearest-position assignment, not a free-for-all reshuffle.

    A bad pair whose original two words find no better within-threshold
    partner is left exactly as it was: this is what keeps a genuine reflow
    (the same word moved to another line because a line break differs, with
    no duplicate nearby to anchor it) reported as reflowed rather than
    silently dropped. A word that *is* freed by another word claiming its old
    partner, but itself finds no new partner, becomes unaligned — a leftover
    duplicate the two sides could not reconcile, not a placement claim.

    Untouched: any pair that was already within threshold, and any word whose
    text does not appear in a bad pair. Returns (pairs, ref_free, cand_free),
    same shapes as the inputs.
    """
    def bad(i, j):
        (rx, ry), (cx, cy) = pair_points(ref_words[i], cand_words[j])
        return abs(cx - rx) > dx_threshold or abs(cy - ry) > dy_threshold

    bad_pairs = [(i, j) for i, j in pairs if bad(i, j)]
    if not bad_pairs:
        return list(pairs), list(ref_free), list(cand_free)

    texts = {norm(ref_words[i]["text"]) for i, _ in bad_pairs}
    group_i = {t: [] for t in texts}
    group_j = {t: [] for t in texts}
    for i, j in bad_pairs:
        t = norm(ref_words[i]["text"])
        group_i[t].append(i)
        group_j[t].append(j)
    for i in ref_free:
        t = norm(ref_words[i]["text"])
        if t in group_i:
            group_i[t].append(i)
    for j in cand_free:
        t = norm(cand_words[j]["text"])
        if t in group_j:
            group_j[t].append(j)

    # Greedy nearest-position matching per text group, closest pair first,
    # accepting only within-threshold matches. `resolved_j` is kept as the
    # exact inverse of `resolved_i`: both are filled together below.
    resolved_i, resolved_j = {}, {}
    for t in texts:
        scored = []
        for i in group_i[t]:
            for j in group_j[t]:
                (rx, ry), (cx, cy) = pair_points(ref_words[i], cand_words[j])
                dx, dy = cx - rx, cy - ry
                if abs(dx) <= dx_threshold and abs(dy) <= dy_threshold:
                    scored.append((abs(dx) + abs(dy), i, j))
        scored.sort(key=lambda s: s[0])
        for _, i, j in scored:
            if i in resolved_i or j in resolved_j:
                continue
            resolved_i[i] = j
            resolved_j[j] = i

    new_pairs = []
    new_ref_free = set(ref_free) - set(resolved_i)
    new_cand_free = set(cand_free) - set(resolved_j)
    for i, j in pairs:
        if i in resolved_i or j in resolved_j:
            # This pair's ref and/or cand half moved to a better partner
            # (added below); whichever half did *not* find one of its own is
            # now a leftover duplicate with no partner — unaligned, not
            # silently re-paired with the old one that just left it.
            if i in resolved_i and j not in resolved_j:
                new_cand_free.add(j)
            if j in resolved_j and i not in resolved_i:
                new_ref_free.add(i)
            continue
        new_pairs.append((i, j))  # neither half found a better match: unchanged (e.g. a genuine reflow)
    for i, j in resolved_i.items():
        new_pairs.append((i, j))
    new_pairs.sort()
    return new_pairs, sorted(new_ref_free), sorted(new_cand_free)


def aligned_pairs(ref_words, cand_words):
    """The word pairs this tool measures: `align_words` (text) repaired by
    `reanchor_pairs` (position). Returns (pairs, ref_free, cand_free)."""
    pairs, _, _ = align_words(ref_words, cand_words)
    paired_i = {i for i, _ in pairs}
    paired_j = {j for _, j in pairs}
    ref_free = [i for i in range(len(ref_words)) if i not in paired_i]
    cand_free = [j for j in range(len(cand_words)) if j not in paired_j]
    dy_threshold = ANCHOR_DY_LINE_FACTOR * _line_height(ref_words + cand_words)
    return reanchor_pairs(ref_words, cand_words, pairs, ref_free, cand_free, REFLOW_DX, dy_threshold)


def geometry_page(ref_words, cand_words, diags, top_n):
    pairs, ref_free, cand_free = aligned_pairs(ref_words, cand_words)
    ref_un, cand_un = len(ref_words) - len(pairs), len(cand_words) - len(pairs)
    rec = {"reference_words": len(ref_words), "candidate_words": len(cand_words), "aligned": len(pairs),
           "reference_unaligned": ref_un, "candidate_unaligned": cand_un}
    if not pairs:
        rec.update({"max_delta": None, "top": []})
        return rec
    deltas = []
    for i, j in pairs:
        r, c = ref_words[i], cand_words[j]
        (rx, ry), (cx, cy) = pair_points(r, c)
        deltas.append((cx - rx, cy - ry, i, j))
    mdx = statistics.median(d[0] for d in deltas)
    mdy = statistics.median(d[1] for d in deltas)
    absx = [abs(d[0]) for d in deltas]
    absy = [abs(d[1]) for d in deltas]
    rec.update({
        "median_shift": [round(mdx, 3), round(mdy, 3)],
        "dx_mean": round(sum(absx) / len(absx), 3), "dx_max": round(max(absx), 3),
        "dy_mean": round(sum(absy) / len(absy), 3), "dy_max": round(max(absy), 3),
        "max_delta": round(max(max(absx), max(absy)), 3),
        "within_0_01": sum(1 for d in deltas if abs(d[0]) <= 0.01 and abs(d[1]) <= 0.01),
        "within_0_5": sum(1 for d in deltas if abs(d[0]) <= 0.5 and abs(d[1]) <= 0.5),
        # The combined figure hides which axis is wrong, and on this corpus the
        # two are very different: horizontal placement (line breaking, glyph
        # advances, inter-word glue) is near-exact on most documents while the
        # vertical position of the same word drifts, because one earlier
        # vertical-space difference moves everything below it. Reporting them
        # apart stops a page-wide `dy` from being read as a line-breaking miss.
        "within_0_5_x": sum(1 for d in deltas if abs(d[0]) <= 0.5),
        "within_0_5_y": sum(1 for d in deltas if abs(d[1]) <= 0.5),
        # `dy` measured against the page's own median vertical shift: how well
        # the *spacing within* the page matches once a single rigid offset is
        # taken out.
        "within_0_5_y_derigidified": sum(1 for d in deltas if abs(d[1] - mdy) <= 0.5),
        "reflowed": sum(1 for d in deltas if abs(d[0]) > REFLOW_DX),
    })
    if abs(mdx) > 2.0 or abs(mdy) > 2.0:
        rec["page_owner"] = (f"page-wide shift ({mdx:+.2f}, {mdy:+.2f}) bp shared by the median word: "
                             "crates/render-pipeline (mac-claude-a) page builder / vendored document-style"
                             + (" — a diagnostic names an unimplemented class/package/environment in this document"
                                if any(re.search(r"not implemented|not supported", d.get("message", "")) for d in diags) else ""))
    ranked = sorted(deltas, key=lambda d: max(abs(d[0]), abs(d[1])), reverse=True)
    top = []
    for dx, dy, i, j in ranked[:top_n]:
        r, c = ref_words[i], cand_words[j]
        # The measured points, not the word origins: `reference` + (dx, dy)
        # must equal `candidate`, or the report cannot be checked by hand.
        (rx, ry), (cx, cy) = pair_points(r, c)
        top.append({
            "text": c["text"], "ref_text": r["text"],
            "dx": round(dx, 3), "dy": round(dy, 3),
            "reference": [round(rx, 3), round(ry, 3)],
            "candidate": [round(cx, 3), round(cy, 3)],
            "anchor": "alnum" if (rx, ry) != (r["x"], r["y_top"]) or (cx, cy) != (c["x"], c["y_top"]) else "word",
            "font": c["font"], "ref_font": r["font"],
            "source": c.get("source"),
            "owner": owner_for_word(c, (dx, dy), (mdx, mdy), diags),
        })
    rec["top"] = top
    return rec


# ----------------------------------------------------------------------------
# fixtures


def corpus_fixtures(root, only):
    out = []
    for fx in corpus.discover_fixtures(root):
        if only and fx["id"] not in only:
            continue
        ref = corpus.find_reference(fx)
        docs = []
        for name in fx["documents"]:
            with open(os.path.join(fx["dir"], name), encoding="utf-8") as f:
                docs.append({"path": name.replace(os.sep, "/"), "text": f.read()})
        out.append({"id": fx["id"], "dir": fx["dir"], "entry": fx["entry"], "documents": docs, "reference": ref,
                    "reference_note": "pinned reference in fixtures/real-world (see reference.json / README)"})
    return out


def harness_fixtures(root, only, texbin, work, log):
    """tests/visual-corpus/harness fixtures: the body after \\begin{document}
    under the pdflatex-lm preamble; the reference is rendered now with
    pdflatex (oracle) into `work`, deterministic env."""
    out = []
    pdflatex = os.path.join(texbin, "pdflatex")
    for name in sorted(os.listdir(root)):
        if not name.endswith(".tex"):
            continue
        fid = name[:-4]
        if only and fid not in only:
            continue
        with open(os.path.join(root, name), encoding="utf-8") as f:
            src = f.read()
        m = re.search(r"(?m)^[ \t]*\\begin\{document\}", src)  # not the one quoted in a % comment
        body = src[m.start():] if m else src
        doc = HARNESS_PREAMBLE + body
        fdir = os.path.join(work, "harness", fid)
        os.makedirs(fdir, exist_ok=True)
        with open(os.path.join(fdir, "main.tex"), "w", encoding="utf-8") as f:
            f.write(doc)
        ref = os.path.join(fdir, "main.pdf")
        note = "missing pdflatex"
        if os.path.isfile(pdflatex):
            env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TEXMFVAR=os.path.join(work, "texmf-var"))
            argv = [pdflatex, "-interaction=batchmode", "-halt-on-error", "-file-line-error", "main.tex"]
            code = 0
            for _ in range(2):
                code, out_, err_, secs, to = corpus.run(argv, env=env, timeout=120, cwd=fdir)
                if code != 0:
                    break
            note = f"rendered by {pdflatex} (exit {code}) with SOURCE_DATE_EPOCH=0, two passes, preamble pdflatex-lm"
            log(f"  reference {fid}: pdflatex exit {code}")
            if code != 0 or not os.path.isfile(ref):
                ref = None
        else:
            ref = None
        out.append({"id": fid, "dir": fdir, "entry": "main.tex", "documents": [{"path": "main.tex", "text": doc}],
                    "reference": ref, "reference_note": note})
    return out


# ----------------------------------------------------------------------------
# producer


def run_render(fx, render, pdf_exact, font_dirs, tfm_dirs, work, log):
    fdir = os.path.join(work, fx["id"])
    os.makedirs(fdir, exist_ok=True)
    req = {"protocol_version": 1, "id": "vo2-" + fx["id"], "type": "compile",
           "payload": {"project_id": "visual-oracle-" + fx["id"], "revision": 1, "entry_path": fx["entry"],
                       "documents": fx["documents"],
                       # The directory `\includegraphics` reads image files
                       # from. Document snapshots carry text only; without a
                       # root the renderer answers every \includegraphics with
                       # the hard error "no project root was supplied with the
                       # request", which is a property of the harness and not
                       # of the engine under test. Absolute and resolved: the
                       # worker rejects relative or symlinked roots
                       # (document-runtime::validate_project_root).
                       "project_root": os.path.realpath(fx["dir"]),
                       # Pinned civil date for \today, matching the
                       # SOURCE_DATE_EPOCH=0 references on the other side; see
                       # the same pin in tools/real-world-corpus/run.py.
                       "date": corpus.EPOCH_DATE,
                       # Image items are opt-in: without the negotiated
                       # capability the renderer places the graphic but never
                       # serialises it (display.rs `Wire.images`), so the PDF
                       # the exact route exports has no image on it.
                       "layout_capabilities": ["display-list-v2", "display-list-v2-images"]}}
    v2 = os.path.join(fdir, "render.v2.json")
    env = dict(os.environ, FLASHTEX_FONT_DIRS=font_dirs, FLASHTEX_TFM_DIRS=tfm_dirs)
    line = (json.dumps(req, ensure_ascii=False) + "\n").encode("utf-8")
    # `--images`: flashtex-render hardcoded `Wire { images: false }` for the
    # `--v2` side output, so the display list never carried an image item even
    # when the file resolved.
    # `--device-color`: paints carry the exact `rg`/`k`/`g` operands
    # (proposal `display-list-v2-device-color`), which `from-v2` reads first;
    # without it a non-dyadic component such as beamer's structure colour
    # (0.2, 0.2, 0.7) reaches `from-v2` as an f64 and is refused ("0.2 needs
    # more than 20 decimal digits"), so no candidate PDF is made at all.
    code, out, err, secs, to = corpus.run([render, "--v2", v2, "--images", "--device-color"], stdin_bytes=line, env=env, timeout=180)
    rec = {"exit": code, "timed_out": to, "seconds": round(secs, 3), "status": "no_reply", "diagnostics": [],
           "stderr_tail": err.decode("utf-8", "replace")[-600:], "pdf": None, "pdf_exit": None, "v2": None}
    for raw in out.decode("utf-8", "replace").splitlines():
        try:
            msg = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if msg.get("type") == "compile_result":
            p = msg.get("payload", {})
            rec["status"] = p.get("status")
            rec["diagnostics"] = p.get("diagnostics", [])
            rec["v1_pages"] = len(p.get("pages", []))
            break
    if rec["status"] in ("ok", "recovered") and os.path.isfile(v2):
        rec["v2"] = v2
        pdf = os.path.join(fdir, "render.exact.pdf")
        # `--project-root`: the display list names image files by
        # project-relative path and the exporter reads the bytes itself.
        argv = [pdf_exact, "from-v2", v2, "--out", pdf, "--font-dir", font_dirs,
                "--project-root", os.path.realpath(fx["dir"])]
        code2, out2, err2, secs2, to2 = corpus.run(argv, timeout=180)
        rec["pdf_exit"] = code2
        rec["pdf_stderr_tail"] = err2.decode("utf-8", "replace")[-400:]
        if code2 == 0 and os.path.isfile(pdf):
            rec["pdf"] = pdf
    log(f"  render {fx['id']}: {rec['status']} exit {code} {rec['seconds']}s; from-v2 exit {rec['pdf_exit']}")
    return rec


# ----------------------------------------------------------------------------
# report


def rank_key(page):
    """Worst first: missing page, unalignable page, largest delta, pixels."""
    g = page.get("geometry") or {}
    missing = 0 if page.get("result", "").startswith("missing") or page.get("result") == "size_mismatch" else 1
    unalign = 0 if (missing == 1 and g.get("aligned", 0) == 0) else 1
    delta = g.get("max_delta") if g.get("max_delta") is not None else -1.0
    px = page.get("differing", 0) or 0
    return (missing, unalign, -delta, -px)


def md(s):
    return str(s).replace("|", "\\|").replace("\n", " ")


def span_str(span):
    if not span:
        return "—"
    return f"{span['path']}:{span['start_byte']}-{span['end_byte']}"


def excerpt(fx, span, width=40):
    if not span:
        return ""
    for d in fx["documents"]:
        if d["path"] == span["path"]:
            b = d["text"].encode("utf-8")
            s, e = span["start_byte"], span["end_byte"]
            lo, hi = max(0, s - width // 2), min(len(b), e + width // 2)
            return b[lo:hi].decode("utf-8", "replace").replace("\n", "⏎")
    return ""


def write_report(out_dir, meta, results, ranked, top_pages):
    L = []
    L.append(f"# Visual-oracle geometry ranking — {meta['stamp']}")
    L.append("")
    L.append(f"Generated by `tools/visual-oracle/rank.py` on `{meta['host']}` ({meta['platform']}). "
             f"`uptime` at start: `{meta['uptime_start']}`; at end: `{meta['uptime_end']}`.")
    L.append("")
    L.append("Nothing here is a parity claim. Every page of every document is ranked worst first by "
             "(missing page, no aligned words, largest word-origin delta in bp, differing pixels at 144 dpi); "
             "whole-PDF byte equality is explicitly not a goal and not measured. Owners are the heuristic in "
             "`rank.py:owner_for_word` (a diagnostic overlapping the word's span wins, then math font, then "
             "page-wide vs isolated shift) — a pointer for triage, not a verdict.")
    L.append("")
    L.append("## Producers and tools")
    L.append("")
    L.append(f"- `render`: `{meta['render']}` (sha256 `{meta['render_sha256'][:16]}…`, {meta['render_note']})")
    L.append(f"- PDF route: `{meta['pdf_exact']}` `from-v2 --font-dir {meta['font_dirs']}` (sha256 `{meta['pdf_exact_sha256'][:16]}…`)")
    L.append(f"- Reference PDFs: {meta['reference_mode']} (pdflatex is an oracle only, never in the product path)")
    L.append(f"- Rasterizer: {meta['rasterizer']}, 8-bit grey at {DPI} dpi")
    L.append(f"- Reference word positions: `tools/visual-oracle/pdftext.py` (stdlib content-stream replay); candidate word "
             f"positions: rendering-v2 `glyph_run` origins (`bp_2pow20`)")
    L.append(f"- Fonts for the producer: `FLASHTEX_FONT_DIRS={meta['font_dirs']}`, `FLASHTEX_TFM_DIRS={meta['tfm_dirs']}`")
    L.append("")
    L.append("## Ranked pages (worst first)")
    L.append("")
    L.append("| rank | fixture | page | result | ref/cand words (aligned) | median shift dx,dy | max \\|dx\\| / \\|dy\\| (bp) | ≤0.01 / ≤0.5 bp | reflowed | differing px | worst word | span | owner |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for n, (fid, page) in enumerate(ranked, 1):
        g = page.get("geometry") or {}
        top = (g.get("top") or [{}])[0]
        px = (f"{page['differing']} ({page['differing_fraction'] * 100:.1f}%)"
              if page.get("result") == "compared" and "differing" in page else "—")
        words = f"{g.get('reference_words', '—')}/{g.get('candidate_words', '—')} ({g.get('aligned', 0)})"
        ms = f"{g['median_shift'][0]:+.2f},{g['median_shift'][1]:+.2f}" if g.get("median_shift") else "—"
        mx = f"{g['dx_max']:.2f} / {g['dy_max']:.2f}" if g.get("max_delta") is not None else "—"
        within = f"{g['within_0_01']} / {g['within_0_5']}" if g.get("max_delta") is not None else "—"
        worst = f"`{md(top.get('text', ''))}` dx {top['dx']:+.2f} dy {top['dy']:+.2f}" if top else "—"
        refl = g.get("reflowed", "—") if g.get("max_delta") is not None else "—"
        L.append(f"| {n} | {fid} | {page['page']} | {page.get('result', '')} | {words} | {ms} | {mx} | {within} | {refl} | {px} | {worst} | `{span_str(top.get('source'))}` | {md(top.get('owner', page.get('owner', '')))} |")
    L.append("")
    L.append("## Per-page detail (top deltas)")
    L.append("")
    for n, (fid, page) in enumerate(ranked, 1):
        g = page.get("geometry") or {}
        L.append(f"### {n}. {fid} page {page['page']} — {page.get('result', '')}")
        L.append("")
        if page.get("result") != "compared":
            L.append(f"- {page.get('note', page.get('result'))}; owner: {page.get('owner', '')}")
            L.append("")
            continue
        if page.get("thumbnail"):
            L.append(f"- thumbnail sheet: `{page['thumbnail']}` (reference | candidate | diff: red = candidate-only ink, blue = reference-only ink)")
        if "differing" in page:
            L.append(f"- pixels: {page['differing']} differing of {page['pixels']} ({page['differing_fraction'] * 100:.2f}%), "
                     f"max grey Δ {page['max_delta']}, ink px ref/cand {page['ink_pixels_reference']}/{page['ink_pixels_candidate']}")
        else:
            L.append("- pixels: not compared (no rasterizer on this host); geometry only")
        if g.get("max_delta") is None:
            L.append(f"- geometry: no aligned words (reference {g.get('reference_words')}, candidate {g.get('candidate_words')}); owner: {page.get('owner', '')}")
            L.append("")
            continue
        L.append(f"- geometry: {g['aligned']} aligned of ref {g['reference_words']} / cand {g['candidate_words']} "
                 f"(unaligned ref {g['reference_unaligned']}, cand {g['candidate_unaligned']}); median shift "
                 f"({g['median_shift'][0]:+.3f}, {g['median_shift'][1]:+.3f}); |dx| mean/max {g['dx_mean']}/{g['dx_max']}; "
                 f"|dy| mean/max {g['dy_mean']}/{g['dy_max']}; within 0.01 bp: {g['within_0_01']}, within 0.5 bp: {g['within_0_5']}")
        L.append(f"- reflowed words (|dx| > {REFLOW_DX:.0f} bp): {g['reflowed']}")
        if g.get("page_owner"):
            L.append(f"- page-level owner: {g['page_owner']}")
        if page.get("reference_notes"):
            L.append(f"- reference reader notes: {', '.join(page['reference_notes'])}")
        L.append("")
        L.append("| word (cand / ref) | dx | dy | reference x,y | candidate x,y | fonts cand / ref | span | source excerpt | owner |")
        L.append("|---|---|---|---|---|---|---|---|---|")
        for t in g["top"]:
            L.append(f"| `{md(t['text'])}` / `{md(t['ref_text'])}` | {t['dx']:+.3f} | {t['dy']:+.3f} | {t['reference'][0]:.3f},{t['reference'][1]:.3f} "
                     f"| {t['candidate'][0]:.3f},{t['candidate'][1]:.3f} | {md(t['font'])} / {md(t['ref_font'])} | `{span_str(t['source'])}` "
                     f"| `{md(t.get('excerpt', ''))}` | {md(t['owner'])} |")
        L.append("")
    L.append("## Documents")
    L.append("")
    L.append("| fixture | reference | ref pages | render status | diags err/warn | v2 pages | from-v2 exit | PDF bytes |")
    L.append("|---|---|---|---|---|---|---|---|")
    for r in results:
        pr = r["producer"]
        errs = sum(1 for d in pr["diagnostics"] if d.get("severity") == "error")
        warns = len(pr["diagnostics"]) - errs
        refp = os.path.relpath(r["reference"], REPO) if r["reference"] and r["reference"].startswith(REPO) else (r["reference"] or "none")
        L.append(f"| {r['id']} | `{refp}` `{r.get('reference_sha256', '')[:12]}` | {r.get('reference_pages', '—')} | {pr['status']} | {errs}/{warns} | "
                 f"{r.get('candidate_pages', '—')} | {pr['pdf_exit']} | {r.get('pdf_bytes', '—')} |")
    L.append("")
    failures = [r for r in results if r["producer"]["status"] not in ("ok", "recovered") or not r["producer"]["pdf"]]
    if failures:
        L.append("## Producer failures (reported, not patched)")
        L.append("")
        for r in failures:
            pr = r["producer"]
            L.append(f"- `{r['id']}`: status `{pr['status']}`, exit {pr['exit']}, from-v2 exit {pr['pdf_exit']} — stderr: "
                     f"{md(pr['stderr_tail'][-300:])} {md(pr.get('pdf_stderr_tail', ''))}")
        L.append("")
    L.append("## Honest limits")
    L.append("")
    L.append("- Word alignment is by normalised text (difflib); a page where the producer typesets different text "
             "(unsupported environments, dropped math) aligns fewer words and the deltas of the aligned ones can be "
             "dominated by that upstream difference. `aligned` and the unaligned counts say how much of the page the "
             "geometry numbers cover.")
    L.append("- Reference positions are decoded from the content stream with the fonts' /Widths; the text used for "
             "alignment maps OT1/T1 ligature slots and /Differences names, everything else becomes `?` and is dropped from "
             "the alignment key. Math words in the reference are per-glyph runs and rarely align.")
    L.append("- The rasterizer anti-aliases; differing-pixel counts on a page whose text differs are large by "
             "construction and are only the secondary ranking key.")
    L.append("- Owners are heuristic pointers; the crate named for a delta caused by an upstream diagnostic is the "
             "diagnostic's owner, not the crate that positioned the word.")
    with open(os.path.join(out_dir, "report.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(L) + "\n")


# ----------------------------------------------------------------------------
# main


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--fixtures", default=corpus.DEFAULT_FIXTURES, help="fixtures/real-world root")
    ap.add_argument("--harness-fixtures", default=None,
                    help="directory of tests/visual-corpus/harness fixtures (*.tex): bodies under the pdflatex-lm preamble, references rendered now with pdflatex")
    ap.add_argument("--harness-note", default="", help="provenance note for --harness-fixtures (branch/SHA the fixtures came from)")
    ap.add_argument("--only", action="append", default=[], help="fixture id (repeatable)")
    ap.add_argument("--render", default=os.environ.get("FLASHTEX_RENDER"), required=False)
    # Never a *default* provenance claim. This used to default to
    # "origin/agent/mac-render-pipeline/unified @ 9aaec57a (git-archive scratch
    # build)" whatever binary --render actually pointed at, so every report
    # asserted a branch and SHA nobody had checked. The report always carries
    # the binary's own sha256 (`meta.render_sha256`); the note is only the
    # human sentence, and when nobody supplies one it must say so.
    ap.add_argument("--render-note", default=os.environ.get(
        "FLASHTEX_RENDER_NOTE", "no provenance note given (see meta.render_sha256 for what actually ran)"))
    ap.add_argument("--pdf-exact", default=os.environ.get("FLASHTEX_PDF_EXACT", os.path.join(REPO, "crates", "pdf", "target", "release", "flashtex-pdf-exact")))
    ap.add_argument("--font-dirs", default=None, help="outline roots (default: $FLASHTEX_FONT_DIRS, else apps/mac/Fonts)")
    # Not a single hardcoded metrics directory: the old default named
    # `texmf/fonts/tfm/public/lm` alone and dropped the bundled `jknappen/ec`
    # and `public/amsfonts/symbols` trees, so T1 fixtures were measured on
    # substituted widths. fontenv.resolve_dirs derives them instead.
    ap.add_argument("--tfm-dirs", default=None, help="metrics roots (default: $FLASHTEX_TFM_DIRS, else derived from --font-dirs)")
    ap.add_argument("--texbin", default=DEFAULT_TEXBIN)
    ap.add_argument("--out", default=None, help="output dir (default docs/evidence/visual-oracle-<UTC>)")
    ap.add_argument("--work", default=None, help="scratch dir for PDFs/rasters (default <out>/work, deleted unless --keep-work)")
    ap.add_argument("--keep-work", action="store_true")
    ap.add_argument("--top", type=int, default=8, help="largest deltas listed per page")
    ap.add_argument("--thumbs", type=int, default=10, help="thumbnail sheets for the N worst compared pages (0 = none)")
    ap.add_argument("--rasterizer", default=None)
    args = ap.parse_args(argv)
    args.font_dirs, args.tfm_dirs = fontenv.resolve_dirs(
        args.font_dirs, args.tfm_dirs, os.path.join(REPO, "apps", "mac", "Fonts"))

    if not args.render or not os.path.isfile(args.render):
        print("--render (or FLASHTEX_RENDER) must point at a flashtex-render binary", file=sys.stderr)
        return 2
    if not os.path.isfile(args.pdf_exact):
        print(f"flashtex-pdf-exact not found at {args.pdf_exact}", file=sys.stderr)
        return 2

    stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")
    out_dir = args.out or os.path.join(REPO, "docs", "evidence", f"visual-oracle-{stamp}")
    work = args.work or os.path.join(out_dir, "work")
    os.makedirs(out_dir, exist_ok=True)
    os.makedirs(work, exist_ok=True)
    logf = open(os.path.join(out_dir, "run.log"), "w", encoding="utf-8")

    def log(msg):
        print(msg, file=sys.stderr)
        logf.write(msg + "\n")
        logf.flush()

    meta = {"stamp": stamp, "host": platform.node(), "platform": f"{platform.system()} {platform.release()} {platform.machine()}",
            "uptime_start": corpus.uptime(), "render": args.render, "render_sha256": corpus.sha256_file(args.render),
            "render_note": args.render_note, "pdf_exact": args.pdf_exact, "pdf_exact_sha256": corpus.sha256_file(args.pdf_exact),
            "font_dirs": args.font_dirs, "tfm_dirs": args.tfm_dirs, "dpi": DPI}
    rast = corpus.find_rasterizer(args.rasterizer)
    meta["rasterizer"] = f"`{rast[0]}` at `{rast[1]}`" if rast[0] else "none found (no pixel comparison)"

    if args.harness_fixtures:
        fixtures = harness_fixtures(args.harness_fixtures, set(args.only), args.texbin, work, log)
        ver = corpus.pdflatex_version(args.texbin)
        ver = ver[1] if isinstance(ver, tuple) else ver
        meta["reference_mode"] = (f"harness fixtures from `{args.harness_fixtures}`{(' (' + args.harness_note + ')') if args.harness_note else ''}: "
                                  f"bodies under the pdflatex-lm preamble, rendered now by `{os.path.join(args.texbin, 'pdflatex')}` ({ver}) "
                                  "with SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1, two passes")
    else:
        fixtures = corpus_fixtures(args.fixtures, set(args.only))
        meta["reference_mode"] = f"pinned files in `{os.path.relpath(args.fixtures, REPO)}/*/` (`reference.pdf` / `*-reference.pdf`, see each `reference.json`)"

    results = []
    all_pages = []
    font_failures = []
    for fx in fixtures:
        log(f"{fx['id']}")
        r = {"id": fx["id"], "reference": fx["reference"], "reference_note": fx["reference_note"], "pages": []}
        if fx["reference"]:
            r["reference_sha256"] = corpus.sha256_file(fx["reference"])
            r["reference_pages"] = corpus.pdf_page_count(fx["reference"])
        pr = run_render(fx, args.render, args.pdf_exact, args.font_dirs, args.tfm_dirs, work, log)
        r["producer"] = pr
        # A run that fell back to substituted metrics is not a measurement of
        # this engine's geometry, so say so in the report instead of scoring it
        # silently (tools/visual-oracle/fontenv.py). run.py already gates on
        # this; rank.py did not, which is how a mis-set FLASHTEX_TFM_DIRS could
        # produce a full ranked report of meaningless deltas.
        font_bad = fontenv.font_diagnostics(pr["diagnostics"])
        if font_bad:
            r["font_env_failure"] = [{"code": d.get("code"), "message": d.get("message")} for d in font_bad]
            font_failures.append((fx["id"], font_bad))
            log(f"  FONT-ENV FAILURE {fx['id']}: {fontenv.format_font_diagnostics(font_bad)}")
        if not fx["reference"]:
            r["note"] = "no reference PDF"
            results.append(r)
            continue
        ref_words = pdftext.page_words(fx["reference"])
        cand_words = []
        if pr["v2"]:
            with open(pr["v2"], encoding="utf-8") as f:
                cand_words = v2_words(json.load(f))
        r["candidate_pages"] = len(cand_words)
        if pr["pdf"]:
            r["pdf_bytes"] = os.path.getsize(pr["pdf"])
        px_pages = []
        ref_files = cand_files = []
        if rast[0] and pr["pdf"]:
            ref_files, _, note_r = corpus.rasterize(rast, fx["reference"], os.path.join(work, fx["id"], "ref"))
            cand_files, _, note_c = corpus.rasterize(rast, pr["pdf"], os.path.join(work, fx["id"], "cand"))
            if note_r or note_c:
                log(f"  raster note: {note_r} {note_c}")
            px_pages = corpus.compare_pages(ref_files, cand_files)
        n = max(len(ref_words), len(cand_words), len(px_pages))
        for i in range(n):
            page = {"page": i + 1}
            if i < len(px_pages):
                page.update(px_pages[i])
            elif not pr["pdf"]:
                page["result"] = "missing_in_candidate" if i >= len(cand_words) else "no_candidate_pdf"
                page["note"] = f"producer status {pr['status']}, from-v2 exit {pr['pdf_exit']}"
            else:
                page["result"] = "compared" if i < len(ref_words) and i < len(cand_words) else ("missing_in_candidate" if i >= len(cand_words) else "missing_in_reference")
            if i < len(ref_words):
                page["reference_notes"] = ref_words[i]["notes"]
            if i < len(ref_words) and i < len(cand_words):
                g = geometry_page(ref_words[i]["words"], cand_words[i]["words"], pr["diagnostics"], args.top)
                for t in g["top"]:
                    t["excerpt"] = excerpt(fx, t["source"])
                page["geometry"] = g
                if g["aligned"] == 0:
                    page["owner"] = ("crates/compiler (parser/expansion; Commander/main) — no word of the page aligns: the producer typeset different text "
                                     "(see the document's diagnostics)")
            if page.get("result", "").startswith("missing") or page.get("result") == "no_candidate_pdf":
                page.setdefault("owner", "crates/render-pipeline (mac-claude-a) — page count differs from the reference"
                                if page["result"] == "missing_in_candidate" and pr["pdf"] else
                                "crates/paragraph-layout / render-pipeline — producer produced no PDF (see failures)")
            if i < len(ref_files) and i < len(cand_files):
                page["_ref_pgm"], page["_cand_pgm"] = ref_files[i], cand_files[i]
            r["pages"].append(page)
            all_pages.append((fx["id"], page))
        results.append(r)

    ranked = sorted(all_pages, key=lambda fp: rank_key(fp[1]))
    # thumbnails for the worst compared pages
    if args.thumbs > 0:
        tdir = os.path.join(out_dir, "thumbs")
        os.makedirs(tdir, exist_ok=True)
        made = 0
        for fid, page in ranked:
            if made >= args.thumbs:
                break
            if page.get("result") != "compared" or "_ref_pgm" not in page:
                continue
            name = f"{made + 1:02d}-{fid}-p{page['page']}.png"
            thumbs.sheet(page["_ref_pgm"], page["_cand_pgm"], os.path.join(tdir, name), scale=3)
            page["thumbnail"] = os.path.join("thumbs", name)
            made += 1
    for _, page in all_pages:
        page.pop("_ref_pgm", None)
        page.pop("_cand_pgm", None)

    meta["uptime_end"] = corpus.uptime()
    meta["font_env_failures"] = [{"fixture": fid, "codes": sorted({d.get("code") for d in bad})}
                                 for fid, bad in font_failures]
    if font_failures:
        for fid, bad in font_failures:
            fontenv.report_font_failure(fid, bad, env=os.environ)
        log(f"FONT-ENV FAILURE in {len(font_failures)} fixture(s); their word positions are not a "
            "measurement of this engine's line breaking")
    else:
        log("font environment: zero font-FAILURE diagnostics in every fixture")
    ranked_out = [{"rank": n, "fixture": fid, "page": p["page"], "result": p.get("result"),
                   "max_delta": (p.get("geometry") or {}).get("max_delta"), "differing": p.get("differing"),
                   "owner": ((p.get("geometry") or {}).get("top") or [{}])[0].get("owner", p.get("owner")),
                   "thumbnail": p.get("thumbnail")} for n, (fid, p) in enumerate(ranked, 1)]
    with open(os.path.join(out_dir, "report.json"), "w", encoding="utf-8") as f:
        json.dump({"meta": meta, "ranked": ranked_out, "documents": results}, f, indent=1, ensure_ascii=False)
    write_report(out_dir, meta, results, ranked, args.thumbs)
    if not args.keep_work and not args.work:
        import shutil
        shutil.rmtree(work, ignore_errors=True)
    log(f"report: {os.path.join(out_dir, 'report.md')}")
    logf.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
