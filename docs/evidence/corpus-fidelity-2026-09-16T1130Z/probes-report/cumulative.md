# Corpus divergences ranked by blast radius — 2026-09-16T112436Z

Produced by `tools/visual-oracle/cumulative.py` (companion to `rank.py`; same
producer route, same reference reader, same word grouper, same alignment).
pdflatex is an oracle only, never in the product path; nothing below is a
parity claim.

- producer: `/Users/dqi26/flashtex/target-gh66/release/flashtex-render` (`cde76936db0d9d89…`) -> `flashtex-pdf-exact`
- references: pinned files in `docs/evidence/corpus-fidelity-2026-09-16T1130Z/probes/*/`
- gates: glyph positions 0.5 bp, rules 0.1 bp; step detection floor 0.05 bp
- host: Darwin 25.6.0 arm64

## Ranked causes (vertical)

`lines affected` is the number of reference baselines the step displaces —
the whole rest of the page for a cumulative step, one line for a local one.

| rank | probable cause | kind | median step (bp) | max step (bp) | fixtures | occurrences | lines affected | > 0.5 bp gate |
|---|---|---|---|---|---|---|---|---|
| 1 | `line-break-within-paragraph` | cumulative | 0.0669 | 5.9774 | 7 | 13 | 34 | yes |
| 2 | `display-math boundary (no environment)` | cumulative | 0.4064 | 0.4064 | 1 | 1 | 11 | no |
| 3 | `paragraph-break` | cumulative | 5.978 | 5.978 | 3 | 6 | 9 | yes |

### Witnesses

**1. `line-break-within-paragraph`** — q4-sum-int, q5-bigdelim, q6-contfrac, t3-macro-mid, u1-lim-frac12, u2-manual-delims, v1-lim-sum
  - `q4-sum-int` p1 y=133.328: -0.0601 bp, cumulative, 9 line(s) after — “Beta beta beta beta beta.”
  - `q4-sum-int` p1 y=176.372: +0.06 bp, local, 1 line(s) after — “Gamma gamma gamma gamma.”
  - `q5-bigdelim` p1 y=126.961: -0.0666 bp, cumulative, 7 line(s) after — “Beta beta beta beta beta.”
  - `q5-bigdelim` p1 y=173.487: +0.0662 bp, local, 1 line(s) after — “Gamma gamma gamma gamma.”

**2. `display-math boundary (no environment)`** — u1-lim-frac12
  - `u1-lim-frac12` p1 y=107.166: -0.4064 bp, cumulative, 11 line(s) after — “sin x”

**3. `paragraph-break`** — r1-cvsec-blank, r2-cvsec-norule, r3-cvsec-novspace
  - `r1-cvsec-blank` p1 y=133.569: +5.978 bp, cumulative, 3 line(s) after — “Beta beta beta beta.”
  - `r1-cvsec-blank` p1 y=184.179: +5.978 bp, local, 1 line(s) after — “Gamma gamma gamma gamma.”
  - `r2-cvsec-norule` p1 y=133.569: +5.978 bp, cumulative, 3 line(s) after — “Beta beta beta beta.”
  - `r2-cvsec-norule` p1 y=184.179: +5.978 bp, local, 1 line(s) after — “Gamma gamma gamma gamma.”

## Horizontal findings

| rank | probable cause | fixtures | occurrences | median |dx| (bp) | max |dx| (bp) | kind |
|---|---|---|---|---|---|---|
| 1 | `font substitution CMBX12->LMRoman12-Bold` | 1 | 1 | 125.7561 | 125.7561 | word |
| 2 | `line-indent-or-margin` | 1 | 1 | 28.5688 | 28.5688 | line |

## Low-confidence steps (not ranked)

A line whose aligned words disagree about `dy` was not all paired with one
reference line. The reference PDF gives cmex big operators no usable text, so
`\int`/`\sum`/`\prod` and large delimiters never align and a neighbouring
limit can pair across the display. These are alignment artefacts, not layout.

| fixture | page | step (bp) | cause | line text |
|---|---|---|---|---|
| v1-lim-sum | 1 | +18.9289 | `display-math boundary (no environment)` | n→∞ |
| v1-lim-sum | 1 | -18.9293 | `display-math boundary (no environment)` | b |

## Pages that break lines differently (excluded from the step table)

A reflowed word (|dx| > 50 bp) sits on another line than in the reference, so
the two sides no longer have the same lines and no `dy` profile is measurable.
These are structural divergences and rank on their own.

| fixture | page | reflowed words | aligned / reference words |
|---|---|---|---|
| q3-cvsec-norule | 1 | 1 | 9/9 |

## Per document

| fixture | pages ref/cand | status | reflowed pages | cumulative steps | local steps | worst cumulative (bp) |
|---|---|---|---|---|---|---|
| p2-negbreak | 1/1 | ok | 0 | 0 | 0 | 0.0 |
| p3-rule | 1/1 | recovered | 0 | 0 | 0 | 0.0 |
| p4-parskip | 1/1 | ok | 0 | 0 | 0 | 0.0 |
| p5-12pt-display | 1/1 | recovered | 0 | 0 | 0 | 0.0 |
| p8-align | 1/1 | recovered | 0 | 0 | 0 | 0.0 |
| q1-vspace | 1/1 | ok | 0 | 0 | 0 | 0.0 |
| q3-cvsec-norule | 1/1 | recovered | 1 | 0 | 0 | 0.0 |
| q4-sum-int | 1/1 | recovered | 0 | 1 | 1 | 0.06 |
| q5-bigdelim | 1/1 | recovered | 0 | 1 | 2 | 0.067 |
| q6-contfrac | 1/1 | recovered | 0 | 1 | 1 | 0.176 |
| r1-cvsec-blank | 1/1 | recovered | 0 | 1 | 1 | 5.978 |
| r2-cvsec-norule | 1/1 | ok | 0 | 1 | 1 | 5.978 |
| r3-cvsec-novspace | 1/1 | recovered | 0 | 1 | 1 | 5.977 |
| r4-cvsec-plainbreak | 1/1 | recovered | 0 | 0 | 0 | 0.0 |
| s2-large-only | 1/1 | ok | 0 | 0 | 0 | 0.0 |
| t1-endpar-neg | 1/1 | ok | 0 | 0 | 0 | 0.0 |
| t3-macro-mid | 1/1 | recovered | 0 | 1 | 1 | 5.977 |
| u1-lim-frac12 | 1/1 | recovered | 0 | 2 | 1 | 0.406 |
| u2-manual-delims | 1/1 | recovered | 0 | 0 | 1 | 0.0 |
| v1-lim-sum | 1/1 | recovered | 0 | 0 | 1 | 0.0 |

