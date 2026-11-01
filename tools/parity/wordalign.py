"""Word alignment of a reference page with a candidate page (oracle tooling).

`aligned_pairs` pairs the words two pages share: a difflib alignment on the
normalised word text (`align_words`), repaired by position where repeated
short tokens make the text alignment ambiguous (`reanchor_pairs`). The
parity scoreboard measures glyph-position errors (L3) over these pairs.

From tools/visual-oracle/rank.py, the old engine's visual ranking tool, when
retirement stage S7 deleted it (docs/design/engine-v2/retirement-plan.md
§1.8); the functions are unchanged.
"""

import difflib
import re
import statistics

REFLOW_DX = 50.0  # |dx| in bp beyond which an aligned word is counted as moved to another line


ANCHOR_DY_LINE_FACTOR = 0.6  # of the page's estimated line height; see reanchor_pairs


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
