# Corpus divergences ranked by blast radius — 2026-09-16T131757Z

Produced by `tools/visual-oracle/cumulative.py` (companion to `rank.py`; same
producer route, same reference reader, same word grouper, same alignment).
pdflatex is an oracle only, never in the product path; nothing below is a
parity claim.

- producer: `/Users/dqi26/flashtex/target-cumcorr/release/flashtex-render` (`1db2e9c051f9f300…`) -> `flashtex-pdf-exact`
- references: pinned files in `fixtures/real-world/*/`
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
| article-twocolumn | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| article-twocolumn | 2 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| conf-paper | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| conf-paper | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| conf-paper | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| conf-paper | 4 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| cv | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| enumitem-worksheet | 1 | - 0.20364 | fin | YES | — | re-run under \tracingoutput; bytes differ from the pinned re |
| enumitem-worksheet | 2 | - 0.16081 | fin | YES | — | re-run under \tracingoutput; bytes differ from the pinned re |
| enumitem-worksheet | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| hw1 | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| hw1 | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| hw1 | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| hw2 | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| hw2 | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| hw2 | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| hyperref-toc | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| hyperref-toc | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| hyperref-toc | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| hyperref-toc | 4 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| inline-math | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| inline-math | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| input-bibliography | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| input-bibliography | 2 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| lab-report | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| lab-report | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| lab-report | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| lecture-notes | 1 | - 0.30122 | fin | YES | — | re-run under \tracingoutput, byte-identical to the pinned re |
| lecture-notes | 2 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| letter | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| listings-manual | 1 | - 0.65112 [] | fin | YES | — | re-run under \tracingoutput; bytes differ from the pinned re |
| listings-manual | 2 | - 0.3748 [] | fin | YES | — | re-run under \tracingoutput; bytes differ from the pinned re |
| listings-manual | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| lmodern-report | 1 | - 0.26439 | fin | YES | — | re-run under \tracingoutput; bytes differ from the pinned re |
| lmodern-report | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| lmodern-report | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| lmodern-report | 4 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| math-sheet | 1 | - 0.74042 | fin | YES | — | re-run under \tracingoutput, byte-identical to the pinned re |
| math-sheet | 2 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |
| natbib-review | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| natbib-review | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| natbib-review | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| plain-article | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| plain-article | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| plain-article | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| ps-calculus | 1 | - 0.02837 | fin | YES | — | re-run under \tracingoutput; bytes differ from the pinned re |
| ps-calculus | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| ps-calculus | 3 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| siunitx-tables | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| siunitx-tables | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| thesis-chapter | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| thesis-chapter | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| thesis-chapter | 3 | - 0.07123 | fin | YES | — | re-run under \tracingoutput; bytes differ from the pinned re |
| thesis-chapter | 4 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| thesis-chapter | 5 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| twelvept-plain | 1 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| twelvept-plain | 2 | 0.0 | fil | no | — | re-run under \tracingoutput; bytes differ from the pinned re |
| twelvept-plain | 3 | - 0.53453 | fin | YES | — | re-run under \tracingoutput; bytes differ from the pinned re |
| unicode-accents | 1 | 0.0 | fil | no | — | re-run under \tracingoutput, byte-identical to the pinned re |

## Ranked causes (vertical)

`lines affected` is the number of reference baselines displaced by steps whose
cause **is** localised — the whole rest of the page for a cumulative step, one
line for a local one. `lines not localised` is the same count for steps on a
page with set glue: displaced, but not by anything this row names.

| rank | probable cause | kind | median step (bp) | max step (bp) | fixtures | occurrences | lines affected | lines not localised | > 0.5 bp gate |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `paragraph-break` | cumulative | 5.9779 | 5.978 | 2 | 4 | 73 | 36 | yes |
| 2 | `line-break-within-paragraph` | cumulative | 0.1344 | 0.8754 | 5 | 9 | 63 | 140 | yes |
| 3 | `unknown-boundary` | cumulative | 0.0997 | 5.5445 | 9 | 13 | 53 | 7 | yes |
| 4 | `display-math boundary (no environment)` | cumulative | 2.7784 | 2.7786 | 2 | 3 | 49 | 48 | yes |
| 5 | `row pitch inside {align}` | cumulative | 0.1612 | 0.1612 | 1 | 1 | 38 | 0 | no |
| 6 | `page-origin` | page-origin | 0.0659 | 0.069 | 2 | 2 | 16 | 0 | no |
| 7 | `before \begin{itemize}` | cumulative | 1.1554 | 5.977 | 3 | 3 | 10 | 24 | yes |
| 8 | `before \begin{figure}` | cumulative | 0.0861 | 0.0861 | 1 | 1 | 9 | 0 | no |
| 9 | `before \begin{tabular*}` | cumulative | 5.9775 | 5.9775 | 1 | 1 | 8 | 0 | yes |
| 10 | `after \end{tabular}` | cumulative | 0.056 | 0.056 | 1 | 1 | 8 | 0 | no |
| 11 | `before \begin{document}` **(every witness on a page with set glue)** | cumulative | 0.3444 | 0.5133 | 2 | 2 | 0 | 90 | yes |
| 12 | `\section` **(every witness on a page with set glue)** | cumulative | 0.0666 | 0.0667 | 1 | 3 | 0 | 80 | no |
| 13 | `before \begin{align}` **(every witness on a page with set glue)** | cumulative | 0.4437 | 0.4437 | 1 | 1 | 0 | 62 | no |
| 14 | `\item` **(every witness on a page with set glue)** | cumulative | 0.2446 | 0.2454 | 1 | 3 | 0 | 60 | no |
| 15 | `after \end{lemma}` **(every witness on a page with set glue)** | cumulative | 0.1239 | 0.1239 | 1 | 2 | 0 | 31 | no |
| 16 | `after \end{align*}` **(every witness on a page with set glue)** | cumulative | 0.4465 | 0.4465 | 1 | 1 | 0 | 23 | no |
| 17 | `after \end{definition}` **(every witness on a page with set glue)** | cumulative | 0.1239 | 0.1239 | 1 | 1 | 0 | 20 | no |
| 18 | `\end{itemize} -> \begin{enumerate}` **(every witness on a page with set glue)** | cumulative | 6.7324 | 6.7324 | 1 | 1 | 0 | 19 | yes |
| 19 | `after \end{example}` **(every witness on a page with set glue)** | cumulative | 0.1239 | 0.1239 | 1 | 1 | 0 | 18 | no |
| 20 | `row pitch inside {enumerate}` **(every witness on a page with set glue)** | cumulative | 0.2454 | 0.2454 | 1 | 1 | 0 | 17 | no |
| 21 | `after \end{enumerate}` **(every witness on a page with set glue)** | cumulative | 0.9767 | 0.9767 | 1 | 1 | 0 | 12 | yes |
| 22 | `row pitch inside {theorem}` **(every witness on a page with set glue)** | cumulative | 0.1487 | 0.1487 | 1 | 1 | 0 | 8 | no |
| 23 | `after \end{theorem}` **(every witness on a page with set glue)** | cumulative | 0.1481 | 0.1481 | 1 | 1 | 0 | 7 | no |
| 24 | `after \end{proof}` **(every witness on a page with set glue)** | cumulative | 1.0761 | 1.0761 | 1 | 1 | 0 | 2 | yes |

### Witnesses

**1. `paragraph-break`** — cv, twelvept-plain
  - `cv` p1 y=149.58: +5.978 bp, cumulative, 31 line(s) after — “Example University Pittsburgh, PA”
  - `cv` p1 y=244.823: +5.978 bp, cumulative, 26 line(s) after — “Software Engineering Intern Summer 2027”
  - `cv` p1 y=413.758: +5.9778 bp, cumulative, 16 line(s) after — “A”
  - `twelvept-plain` p3 y=281.495: -0.1189 bp, cumulative, 36 line(s) after — CAUSE NOT LOCALISED — “3 Series and closing remarks”

**2. `line-break-within-paragraph`** — enumitem-worksheet, math-sheet, natbib-review, ps-calculus, twelvept-plain
  - `enumitem-worksheet` p1 y=220.557: +0.0798 bp, local, 1 line(s) after — CAUSE NOT LOCALISED — “January 1, 1970”
  - `math-sheet` p1 y=670.484: +0.8754 bp, cumulative, 10 line(s) after — CAUSE NOT LOCALISED — “d”
  - `natbib-review` p1 y=668.605: -0.0611 bp, cumulative, 5 line(s) after — “clamped to an implementation-defined maximum repre”
  - `ps-calculus` p1 y=194.999: +0.1344 bp, cumulative, 49 line(s) after — CAUSE NOT LOCALISED — “October 3, 2025”

**3. `unknown-boundary`** — conf-paper, hyperref-toc, math-sheet, natbib-review, plain-article, ps-calculus, siunitx-tables, thesis-chapter, twelvept-plain
  - `conf-paper` p4 y=767.888: +0.0688 bp, local, 1 line(s) after — “4”
  - `hyperref-toc` p4 y=749.888: -1.155 bp, local, 1 line(s) after — “4”
  - `math-sheet` p2 y=213.489: -2.7778 bp, cumulative, 28 line(s) after — “1 0 0”
  - `math-sheet` p2 y=227.038: -2.7773 bp, cumulative, 25 line(s) after — “0 1 0 I = M =”

**4. `display-math boundary (no environment)`** — math-sheet, twelvept-plain
  - `math-sheet` p2 y=217.485: +2.7786 bp, cumulative, 27 line(s) after — “m m m”
  - `math-sheet` p2 y=250.14: -2.7784 bp, cumulative, 22 line(s) after — “m m m”
  - `twelvept-plain` p3 y=157.236: -0.8586 bp, cumulative, 48 line(s) after — CAUSE NOT LOCALISED — “sin x”

**5. `row pitch inside {align}`** — ps-calculus
  - `ps-calculus` p2 y=242.225: -0.1612 bp, cumulative, 38 line(s) after — “1”

**6. `page-origin`** — conf-paper, lab-report
  - `conf-paper` p4 y=652.082: -0.069 bp, page-origin, 3 line(s) after — “Figure 1: Tail latency (p99) across three producti”
  - `lab-report` p2 y=493.965: -0.0628 bp, page-origin, 13 line(s) after — “2”

**7. `before \begin{itemize}`** — cv, hyperref-toc, twelvept-plain
  - `cv` p1 y=598.977: +5.977 bp, cumulative, 2 line(s) after — “Putnam Competition, top 500 (2026).”
  - `hyperref-toc` p4 y=107.308: +1.1554 bp, cumulative, 8 line(s) after — “Badness: a numeric measure of how far a line or pa”
  - `twelvept-plain` p3 y=391.998: -0.979 bp, cumulative, 24 line(s) after — CAUSE NOT LOCALISED — “Always check the domain of validity before taking ”

**8. `before \begin{figure}`** — thesis-chapter
  - `thesis-chapter` p2 y=604.039: -0.0861 bp, cumulative, 9 line(s) after — “Figure 1.1: Representative sparse sensor deploymen”

**9. `before \begin{tabular*}`** — cv
  - `cv` p1 y=521.506: +5.9775 bp, cumulative, 8 line(s) after — “Languages Rust, Swift, Python, C, Standard ML”

**10. `after \end{tabular}`** — plain-article
  - `plain-article` p2 y=500.42: -0.056 bp, cumulative, 8 line(s) after — “Table 1: Summary of proposed fixes.”

**11. `before \begin{document}`** — lecture-notes, math-sheet
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `lecture-notes` p1 y=225.426: +0.1755 bp, cumulative, 27 line(s) after — CAUSE NOT LOCALISED — “Contents”
  - `math-sheet` p1 y=197.992: -0.5133 bp, cumulative, 63 line(s) after — CAUSE NOT LOCALISED — “Series and sums”

**12. `\section`** — math-sheet
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `math-sheet` p1 y=336.208: -0.0666 bp, cumulative, 43 line(s) after — CAUSE NOT LOCALISED — “Integrals”
  - `math-sheet` p1 y=534.637: -0.0667 bp, cumulative, 22 line(s) after — CAUSE NOT LOCALISED — “Piecewise definitions”
  - `math-sheet` p1 y=621.847: -0.0661 bp, cumulative, 15 line(s) after — CAUSE NOT LOCALISED — “Limits and derivatives”

**13. `before \begin{align}`** — math-sheet
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `math-sheet` p1 y=220.844: -0.4437 bp, cumulative, 62 line(s) after — CAUSE NOT LOCALISED — “n n”

**14. `\item`** — twelvept-plain
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `twelvept-plain` p3 y=415.341: -0.2444 bp, cumulative, 23 line(s) after — CAUSE NOT LOCALISED — “Growable delimiters are a kernel feature—you do no”
  - `twelvept-plain` p3 y=453.131: -0.2454 bp, cumulative, 21 line(s) after — CAUSE NOT LOCALISED — “Nested fractions are the easiest way to stress-tes”
  - `twelvept-plain` p3 y=547.139: -0.2446 bp, cumulative, 16 line(s) after — CAUSE NOT LOCALISED — “sin x”

**15. `after \end{lemma}`** — lecture-notes
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `lecture-notes` p1 y=447.193: +0.1239 bp, cumulative, 17 line(s) after — CAUSE NOT LOCALISED — “Proof. Write b = ka and c = for some k, Then c = =”
  - `lecture-notes` p1 y=503.973: +0.1239 bp, cumulative, 14 line(s) after — CAUSE NOT LOCALISED — “Proof. Write b = ka and c = Then mb + nc = mka + =”

**16. `after \end{align*}`** — math-sheet
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `math-sheet` p1 y=498.606: -0.4465 bp, cumulative, 23 line(s) after — CAUSE NOT LOCALISED — “Integration by parts: u, dv = uv v, du.”

**17. `after \end{definition}`** — lecture-notes
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `lecture-notes` p1 y=391.614: +0.1239 bp, cumulative, 20 line(s) after — CAUSE NOT LOCALISED — “Example 1.2. 3 12 since 12 = 4 3, but 5 12. Every ”

**18. `\end{itemize} -> \begin{enumerate}`** — twelvept-plain
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `twelvept-plain` p3 y=490.713: +6.7324 bp, cumulative, 19 line(s) after — CAUSE NOT LOCALISED — “1. Redo the continued-fraction example above with ”

**19. `after \end{example}`** — lecture-notes
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `lecture-notes` p1 y=426.178: +0.1239 bp, cumulative, 18 line(s) after — CAUSE NOT LOCALISED — “Lemma 1.3. If a b and b c then a c.”

**20. `row pitch inside {enumerate}`** — twelvept-plain
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `twelvept-plain` p3 y=528.503: -0.2454 bp, cumulative, 17 line(s) after — CAUSE NOT LOCALISED — “2. Show that is differentiable wherever ⃗r(t) is, ”

**21. `after \end{enumerate}`** — twelvept-plain
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `twelvept-plain` p3 y=570.645: -0.9767 bp, cumulative, 12 line(s) after — CAUSE NOT LOCALISED — “1”

**22. `row pitch inside {theorem}`** — lecture-notes
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `lecture-notes` p1 y=620.88: +0.1487 bp, cumulative, 8 line(s) after — CAUSE NOT LOCALISED — “a = qb + r and 0 r < b.”

**23. `after \end{theorem}`** — lecture-notes
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `lecture-notes` p1 y=643.588: +0.1481 bp, cumulative, 7 line(s) after — CAUSE NOT LOCALISED — “Proof. Existence. Consider the set S = a kb : k a ”

**24. `after \end{proof}`** — lecture-notes
  - **cause not localised:** every occurrence is on a page whose glue pdfTeX set, so this row is not evidence that this construct is where the defect is.
  - `lecture-notes` p1 y=720.0: -1.0761 bp, cumulative, 2 line(s) after — CAUSE NOT LOCALISED — “Corollary 2.2. Every integer is either even or odd”

## Steps whose cause is not localised (excluded from the ranking above)

Each of these displaces the baselines it says it does. What it does **not** do
is name the construct responsible: the page's glue set is global, so the step
surfaces where the two sides' ratios diverge, not where the underlying error
is. Re-measure any of these on a page that does not overflow before filing it.

| fixture | page | glue set | step (bp) | kind | lines | cause as labelled | line text |
|---|---|---|---|---|---|---|---|
| enumitem-worksheet | 1 | - 0.20364 | +0.0798 | local | 1 | `line-break-within-paragraph` | January 1, 1970 |
| lecture-notes | 1 | - 0.30122 | +0.1755 | cumulative | 27 | `before \begin{document}` | Contents |
| lecture-notes | 1 | - 0.30122 | +0.1239 | cumulative | 20 | `after \end{definition}` | Example 1.2. 3 12 since 12 = 4 3, but 5  |
| lecture-notes | 1 | - 0.30122 | +0.1239 | cumulative | 18 | `after \end{example}` | Lemma 1.3. If a b and b c then a c. |
| lecture-notes | 1 | - 0.30122 | +0.1239 | cumulative | 17 | `after \end{lemma}` | Proof. Write b = ka and c = for some k,  |
| lecture-notes | 1 | - 0.30122 | +0.1239 | cumulative | 14 | `after \end{lemma}` | Proof. Write b = ka and c = Then mb + nc |
| lecture-notes | 1 | - 0.30122 | +0.1487 | cumulative | 8 | `row pitch inside {theorem}` | a = qb + r and 0 r < b. |
| lecture-notes | 1 | - 0.30122 | +0.1481 | cumulative | 7 | `after \end{theorem}` | Proof. Existence. Consider the set S = a |
| lecture-notes | 1 | - 0.30122 | -1.0761 | cumulative | 2 | `after \end{proof}` | Corollary 2.2. Every integer is either e |
| math-sheet | 1 | - 0.74042 | -0.5133 | cumulative | 63 | `before \begin{document}` | Series and sums |
| math-sheet | 1 | - 0.74042 | -0.4437 | cumulative | 62 | `before \begin{align}` | n n |
| math-sheet | 1 | - 0.74042 | -0.0666 | cumulative | 43 | `\section` | Integrals |
| math-sheet | 1 | - 0.74042 | -0.4465 | cumulative | 23 | `after \end{align*}` | Integration by parts: u, dv = uv v, du. |
| math-sheet | 1 | - 0.74042 | -0.0667 | cumulative | 22 | `\section` | Piecewise definitions |
| math-sheet | 1 | - 0.74042 | -0.0661 | cumulative | 15 | `\section` | Limits and derivatives |
| math-sheet | 1 | - 0.74042 | +0.8754 | cumulative | 10 | `line-break-within-paragraph` | d |
| ps-calculus | 1 | - 0.02837 | +0.1344 | cumulative | 49 | `line-break-within-paragraph` | October 3, 2025 |
| ps-calculus | 1 | - 0.02837 | -0.1587 | cumulative | 10 | `line-break-within-paragraph` | 13 |
| twelvept-plain | 3 | - 0.53453 | -0.8586 | cumulative | 48 | `display-math boundary (no environment)` | sin x |
| twelvept-plain | 3 | - 0.53453 | -0.8575 | cumulative | 44 | `line-break-within-paragraph` | a fact that can be proved geometrically  |
| twelvept-plain | 3 | - 0.53453 | -0.1189 | cumulative | 36 | `paragraph-break` | 3 Series and closing remarks |
| twelvept-plain | 3 | - 0.53453 | -0.8577 | cumulative | 27 | `line-break-within-paragraph` | 1 |
| twelvept-plain | 3 | - 0.53453 | -0.979 | cumulative | 24 | `before \begin{itemize}` | Always check the domain of validity befo |
| twelvept-plain | 3 | - 0.53453 | -0.2444 | cumulative | 23 | `\item` | Growable delimiters are a kernel feature |
| twelvept-plain | 3 | - 0.53453 | -0.2454 | cumulative | 21 | `\item` | Nested fractions are the easiest way to  |
| twelvept-plain | 3 | - 0.53453 | +6.7324 | cumulative | 19 | `\end{itemize} -> \begin{enumerate}` | 1. Redo the continued-fraction example a |
| twelvept-plain | 3 | - 0.53453 | -0.2454 | cumulative | 17 | `row pitch inside {enumerate}` | 2. Show that is differentiable wherever  |
| twelvept-plain | 3 | - 0.53453 | -0.2446 | cumulative | 16 | `\item` | sin x |
| twelvept-plain | 3 | - 0.53453 | -0.9767 | cumulative | 12 | `after \end{enumerate}` | 1 |
| twelvept-plain | 3 | - 0.53453 | -0.2411 | cumulative | 7 | `unknown-boundary` | 1 |

## Horizontal findings

| rank | probable cause | fixtures | occurrences | median |dx| (bp) | max |dx| (bp) | kind |
|---|---|---|---|---|---|---|
| 1 | `line-indent-or-margin` | 15 | 117 | 4.3994 | 356.6595 | line |
| 2 | `font substitution SFRM1095->LMRoman10-Regular` | 10 | 112 | 2.219 | 446.314 | word |
| 3 | `math glyph advance` | 7 | 78 | 1.8184 | 12.8912 | word |
| 4 | `font substitution CMR10->LMRoman10-Regular` | 7 | 71 | 2.2811 | 409.6507 | word |
| 5 | `font substitution SFRM1000->LMRoman10-Regular` | 2 | 88 | 2.7721 | 246.2029 | word |
| 6 | `text advance / interword glue` | 1 | 38 | 5.0685 | 471.6361 | word |
| 7 | `font substitution SFRM1200->LMRoman12-Regular` | 1 | 26 | 4.0532 | 23.0127 | word |
| 8 | `font substitution SFTT1000->LMMono10-Regular` | 1 | 13 | 3.2341 | 572.0926 | word |
| 9 | `font substitution CMR9->LMRoman9-Regular` | 1 | 11 | 1.5271 | 297.6041 | word |
| 10 | `font substitution SFBX1095->LMRoman10-Bold` | 1 | 8 | 1.192 | 2.0413 | word |
| 11 | `font substitution SFTI1095->LMRoman10-Italic` | 1 | 8 | 0.8912 | 1.7731 | word |
| 12 | `font substitution SFTT1095->LMMono10-Regular` | 1 | 5 | 6.231 | 12.4542 | word |
| 13 | `font substitution CMTI10->LMRoman10-Italic` | 1 | 4 | 0.7502 | 0.8273 | word |
| 14 | `font substitution SFTT1095->LMRoman10-Bold` | 1 | 3 | 2.067 | 3.4901 | word |
| 15 | `font substitution CMR12->LMRoman12-Regular` | 1 | 3 | 0.6495 | 0.6541 | word |
| 16 | `font substitution SFTI1200->LMRoman12-Italic` | 1 | 2 | 227.2877 | 423.1411 | word |
| 17 | `font substitution SFTI1000->LMRoman10-Italic` | 1 | 2 | 6.1222 | 7.3429 | word |
| 18 | `font substitution SFTI1095->LMRoman10-Regular` | 1 | 1 | 0.5536 | 0.5536 | word |

## Low-confidence steps (not ranked)

A line whose aligned words disagree about `dy` was not all paired with one
reference line. The reference PDF gives cmex big operators no usable text, so
`\int`/`\sum`/`\prod` and large delimiters never align and a neighbouring
limit can pair across the display. These are alignment artefacts, not layout.

| fixture | page | step (bp) | cause | line text |
|---|---|---|---|---|
| math-sheet | 1 | +1.6746 | `display-math boundary (no environment)` | n |
| twelvept-plain | 2 | +18.9292 | `display-math boundary (no environment)` | n→∞ |
| twelvept-plain | 2 | -18.9293 | `display-math boundary (no environment)` | b |
| twelvept-plain | 3 | -0.8572 | `display-math boundary (no environment)` | n |

## Pages that break lines differently (excluded from the step table)

A reflowed word (|dx| > 50 bp) sits on another line than in the reference, so
the two sides no longer have the same lines and no `dy` profile is measurable.
These are structural divergences and rank on their own.

| fixture | page | reflowed words | aligned / reference words |
|---|---|---|---|
| article-twocolumn | 1 | 2 | 506/527 |
| conf-paper | 1 | 2 | 643/652 |
| hyperref-toc | 1 | 1 | 493/505 |
| inline-math | 1 | 2 | 727/855 |
| inline-math | 2 | 1 | 307/358 |
| input-bibliography | 2 | 1 | 233/235 |
| lab-report | 1 | 3 | 301/311 |
| letter | 1 | 3 | 227/232 |
| listings-manual | 2 | 12 | 271/278 |
| listings-manual | 3 | 52 | 227/236 |
| plain-article | 1 | 2 | 213/220 |

## Per document

| fixture | pages ref/cand | status | reflowed pages | pages with set glue | cumulative steps | of those, not localised | local steps | worst localised cumulative (bp) |
|---|---|---|---|---|---|---|---|---|
| article-twocolumn | 2/2 | recovered | 1 | 0 | 0 | 0 | 0 | 0.0 |
| conf-paper | 4/4 | recovered | 1 | 0 | 0 | 0 | 1 | 0.0 |
| cv | 1/1 | recovered | 0 | 0 | 5 | 0 | 0 | 5.978 |
| enumitem-worksheet | 3/3 | recovered | 0 | 2 | 0 | 0 | 1 | 0.0 |
| hw1 | 3/3 | recovered | 0 | 0 | 0 | 0 | 0 | 0.0 |
| hw2 | 3/3 | recovered | 0 | 0 | 0 | 0 | 0 | 0.0 |
| hyperref-toc | 4/4 | recovered | 1 | 0 | 1 | 0 | 1 | 1.155 |
| inline-math | 2/2 | recovered | 2 | 0 | 0 | 0 | 0 | 0.0 |
| input-bibliography | 2/2 | recovered | 1 | 0 | 0 | 0 | 0 | 0.0 |
| lab-report | 3/3 | recovered | 1 | 0 | 0 | 0 | 0 | 0.0 |
| lecture-notes | 2/2 | recovered | 0 | 1 | 8 | 8 | 0 | 0.0 |
| letter | 1/1 | recovered | 1 | 0 | 0 | 0 | 0 | 0.0 |
| listings-manual | 3/3 | recovered | 2 | 2 | 0 | 0 | 0 | 0.0 |
| lmodern-report | 4/4 | recovered | 0 | 1 | 0 | 0 | 0 | 0.0 |
| math-sheet | 2/2 | recovered | 0 | 1 | 11 | 7 | 1 | 2.779 |
| natbib-review | 3/3 | recovered | 0 | 0 | 1 | 0 | 1 | 0.061 |
| plain-article | 3/3 | recovered | 1 | 0 | 1 | 0 | 1 | 0.056 |
| ps-calculus | 3/3 | recovered | 0 | 1 | 4 | 2 | 2 | 0.161 |
| siunitx-tables | 2/2 | recovered | 0 | 0 | 0 | 0 | 1 | 0.0 |
| thesis-chapter | 5/5 | recovered | 0 | 1 | 1 | 0 | 1 | 0.086 |
| twelvept-plain | 3/3 | recovered | 0 | 1 | 13 | 12 | 1 | 0.088 |
| unicode-accents | 1/1 | recovered | 0 | 0 | 0 | 0 | 0 | 0.0 |

