# Corpus divergences ranked by blast radius — 2026-09-16T131649Z

Produced by `tools/visual-oracle/cumulative.py` (companion to `rank.py`; same
producer route, same reference reader, same word grouper, same alignment).
pdflatex is an oracle only, never in the product path; nothing below is a
parity claim.

- producer: `/Users/dqi26/flashtex/target-cumcorr/release/flashtex-render` (`1db2e9c051f9f300…`) -> `flashtex-pdf-exact`
- references: pinned files in `../../../../private/tmp/claude-503/-Users-dqi26-flashtex/ad5c1298-8c05-46e3-95c6-f8f6b313317c/scratchpad/probes/*/`
- gates: glyph positions 0.5 bp, rules 0.1 bp; step detection floor 0.05 bp
- page glue set: read from `/Library/TeX/texbin/pdflatex` under `\tracingoutput` on a re-run verified against the pinned reference
- host: Darwin 25.6.0 arm64

## Page glue set, and what it does to attribution

TeX fits a page by scaling every stretchable or shrinkable skip on it by one
**page-global** ratio. On a page with a finite-order set, a baseline sits at
its natural position minus `ratio x (shrinkability above it)`, so if the two
producers' natural page heights differ *anywhere*, every baseline separates in
proportion to the shrinkable glue above it — and shrinkable glue is
`\abovedisplayskip`, `\topsep`, `\itemsep`, `\parskip`, the `\@startsection`
skips, which is precisely what this tool names its causes after.

**A step on such a page therefore does not localise its cause.** Those steps
are listed separately below and excluded from `lines affected`; they are real
displacements, but the construct at their position is not shown to have caused
them. A `fil`-order set (a short page whose `\vfil` absorbs the slack) leaves
every finite glue at natural size and is **not** flagged.

The reference ratio is pdfTeX's own `\tracingoutput` figure. **The candidate's
is not reported: the engine does not expose it** — `render-pipeline`'s
`pagebuild::glue_set` computes the ratio and drops it once baselines are placed,
and the v2 display list carries only the fixed page size. The engine's own
`overfull_vbox` diagnostic is reported in its place. No candidate ratio is
invented here.

| fixture | page | reference glue set | order | displaces baselines? | candidate overfull | source |
|---|---|---|---|---|---|---|
| p2-negbreak | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| p3-rule | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| p4-parskip | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| p5-12pt-display | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| p8-align | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| q1-vspace | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| q3-cvsec-norule | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| q4-sum-int | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| q5-bigdelim | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| q6-contfrac | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| r1-cvsec-blank | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| r2-cvsec-norule | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| r3-cvsec-novspace | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| r4-cvsec-plainbreak | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| s2-large-only | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| t1-endpar-neg | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| t3-macro-mid | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| u1-lim-frac12 | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| u2-manual-delims | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| v1-lim-sum | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |

## Ranked causes (vertical)

`lines affected` is the number of reference baselines displaced by steps whose
cause **is** localised — the whole rest of the page for a cumulative step, one
line for a local one. `lines not localised` is the same count for steps on a
page with set glue: displaced, but not by anything this row names.

| rank | probable cause | kind | median step (bp) | max step (bp) | fixtures | occurrences | lines affected | lines not localised | > 0.5 bp gate |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `line-break-within-paragraph` | cumulative | 0.0669 | 5.9774 | 7 | 13 | 34 | 0 | yes |
| 2 | `display-math boundary (no environment)` | cumulative | 0.4064 | 0.4064 | 1 | 1 | 11 | 0 | no |
| 3 | `paragraph-break` | cumulative | 5.978 | 5.978 | 3 | 6 | 9 | 0 | yes |

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

## Steps whose cause is not localised (excluded from the ranking above)

Each of these displaces the baselines it says it does. What it does **not** do
is name the construct responsible: the page's glue set is global, so the step
surfaces where the two sides' ratios diverge, not where the underlying error
is. Re-measure any of these on a page that does not overflow before filing it.

| fixture | page | glue set | step (bp) | kind | lines | cause as labelled | line text |
|---|---|---|---|---|---|---|---|

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

| fixture | pages ref/cand | status | reflowed pages | pages with set glue | cumulative steps | of those, not localised | local steps | worst localised cumulative (bp) |
|---|---|---|---|---|---|---|---|---|
| p2-negbreak | 1/1 | ok | 0 | 0 | 0 | 0 | 0 | 0.0 |
| p3-rule | 1/1 | recovered | 0 | 0 | 0 | 0 | 0 | 0.0 |
| p4-parskip | 1/1 | ok | 0 | 0 | 0 | 0 | 0 | 0.0 |
| p5-12pt-display | 1/1 | recovered | 0 | 0 | 0 | 0 | 0 | 0.0 |
| p8-align | 1/1 | recovered | 0 | 0 | 0 | 0 | 0 | 0.0 |
| q1-vspace | 1/1 | ok | 0 | 0 | 0 | 0 | 0 | 0.0 |
| q3-cvsec-norule | 1/1 | recovered | 1 | 0 | 0 | 0 | 0 | 0.0 |
| q4-sum-int | 1/1 | recovered | 0 | 0 | 1 | 0 | 1 | 0.06 |
| q5-bigdelim | 1/1 | recovered | 0 | 0 | 1 | 0 | 2 | 0.067 |
| q6-contfrac | 1/1 | recovered | 0 | 0 | 1 | 0 | 1 | 0.176 |
| r1-cvsec-blank | 1/1 | recovered | 0 | 0 | 1 | 0 | 1 | 5.978 |
| r2-cvsec-norule | 1/1 | ok | 0 | 0 | 1 | 0 | 1 | 5.978 |
| r3-cvsec-novspace | 1/1 | recovered | 0 | 0 | 1 | 0 | 1 | 5.977 |
| r4-cvsec-plainbreak | 1/1 | recovered | 0 | 0 | 0 | 0 | 0 | 0.0 |
| s2-large-only | 1/1 | ok | 0 | 0 | 0 | 0 | 0 | 0.0 |
| t1-endpar-neg | 1/1 | ok | 0 | 0 | 0 | 0 | 0 | 0.0 |
| t3-macro-mid | 1/1 | recovered | 0 | 0 | 1 | 0 | 1 | 5.977 |
| u1-lim-frac12 | 1/1 | recovered | 0 | 0 | 2 | 0 | 1 | 0.406 |
| u2-manual-delims | 1/1 | recovered | 0 | 0 | 0 | 0 | 1 | 0.0 |
| v1-lim-sum | 1/1 | recovered | 0 | 0 | 0 | 0 | 1 | 0.0 |

