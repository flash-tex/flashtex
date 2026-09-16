# Corpus divergences ranked by blast radius — 2026-09-16T112427Z

Produced by `tools/visual-oracle/cumulative.py` (companion to `rank.py`; same
producer route, same reference reader, same word grouper, same alignment).
pdflatex is an oracle only, never in the product path; nothing below is a
parity claim.

- producer: `/Users/dqi26/flashtex/target-gh66/release/flashtex-render` (`cde76936db0d9d89…`) -> `flashtex-pdf-exact`
- references: pinned files in `fixtures/real-world/*/`
- gates: glyph positions 0.5 bp, rules 0.1 bp; step detection floor 0.05 bp
- host: Darwin 25.6.0 arm64

## Ranked causes (vertical)

`lines affected` is the number of reference baselines the step displaces —
the whole rest of the page for a cumulative step, one line for a local one.

| rank | probable cause | kind | median step (bp) | max step (bp) | fixtures | occurrences | lines affected | > 0.5 bp gate |
|---|---|---|---|---|---|---|---|---|
| 1 | `line-break-within-paragraph` | cumulative | 0.1344 | 0.8754 | 5 | 9 | 203 | yes |
| 2 | `paragraph-break` | cumulative | 5.9779 | 5.978 | 2 | 4 | 109 | yes |
| 3 | `display-math boundary (no environment)` | cumulative | 2.7784 | 2.7786 | 2 | 3 | 97 | yes |
| 4 | `before \begin{document}` | cumulative | 0.3444 | 0.5133 | 2 | 2 | 90 | yes |
| 5 | `\section` | cumulative | 0.0666 | 0.0667 | 1 | 3 | 80 | no |
| 6 | `before \begin{align}` | cumulative | 0.4437 | 0.4437 | 1 | 1 | 62 | no |
| 7 | `unknown-boundary` | cumulative | 0.0997 | 5.5445 | 9 | 13 | 60 | yes |
| 8 | `\item` | cumulative | 0.2446 | 0.2454 | 1 | 3 | 60 | no |
| 9 | `row pitch inside {align}` | cumulative | 0.1612 | 0.1612 | 1 | 1 | 38 | no |
| 10 | `before \begin{itemize}` | cumulative | 1.1554 | 5.977 | 3 | 3 | 34 | yes |
| 11 | `after \end{lemma}` | cumulative | 0.1239 | 0.1239 | 1 | 2 | 31 | no |
| 12 | `after \end{align*}` | cumulative | 0.4465 | 0.4465 | 1 | 1 | 23 | no |
| 13 | `after \end{definition}` | cumulative | 0.1239 | 0.1239 | 1 | 1 | 20 | no |
| 14 | `\end{itemize} -> \begin{enumerate}` | cumulative | 6.7324 | 6.7324 | 1 | 1 | 19 | yes |
| 15 | `after \end{example}` | cumulative | 0.1239 | 0.1239 | 1 | 1 | 18 | no |
| 16 | `row pitch inside {enumerate}` | cumulative | 0.2454 | 0.2454 | 1 | 1 | 17 | no |
| 17 | `page-origin` | page-origin | 0.0659 | 0.069 | 2 | 2 | 16 | no |
| 18 | `after \end{enumerate}` | cumulative | 0.9767 | 0.9767 | 1 | 1 | 12 | yes |
| 19 | `before \begin{figure}` | cumulative | 0.0861 | 0.0861 | 1 | 1 | 9 | no |
| 20 | `before \begin{tabular*}` | cumulative | 5.9775 | 5.9775 | 1 | 1 | 8 | yes |
| 21 | `row pitch inside {theorem}` | cumulative | 0.1487 | 0.1487 | 1 | 1 | 8 | no |
| 22 | `after \end{tabular}` | cumulative | 0.056 | 0.056 | 1 | 1 | 8 | no |
| 23 | `after \end{theorem}` | cumulative | 0.1481 | 0.1481 | 1 | 1 | 7 | no |
| 24 | `after \end{proof}` | cumulative | 1.0761 | 1.0761 | 1 | 1 | 2 | yes |

### Witnesses

**1. `line-break-within-paragraph`** — enumitem-worksheet, math-sheet, natbib-review, ps-calculus, twelvept-plain
  - `enumitem-worksheet` p1 y=220.557: +0.0798 bp, local, 1 line(s) after — “January 1, 1970”
  - `math-sheet` p1 y=670.484: +0.8754 bp, cumulative, 10 line(s) after — “d”
  - `natbib-review` p1 y=668.605: -0.0611 bp, cumulative, 5 line(s) after — “clamped to an implementation-defined maximum repre”
  - `ps-calculus` p1 y=194.999: +0.1344 bp, cumulative, 49 line(s) after — “October 3, 2025”

**2. `paragraph-break`** — cv, twelvept-plain
  - `cv` p1 y=149.58: +5.978 bp, cumulative, 31 line(s) after — “Example University Pittsburgh, PA”
  - `cv` p1 y=244.823: +5.978 bp, cumulative, 26 line(s) after — “Software Engineering Intern Summer 2027”
  - `cv` p1 y=413.758: +5.9778 bp, cumulative, 16 line(s) after — “A”
  - `twelvept-plain` p3 y=281.495: -0.1189 bp, cumulative, 36 line(s) after — “3 Series and closing remarks”

**3. `display-math boundary (no environment)`** — math-sheet, twelvept-plain
  - `math-sheet` p2 y=217.485: +2.7786 bp, cumulative, 27 line(s) after — “m m m”
  - `math-sheet` p2 y=250.14: -2.7784 bp, cumulative, 22 line(s) after — “m m m”
  - `twelvept-plain` p3 y=157.236: -0.8586 bp, cumulative, 48 line(s) after — “sin x”

**4. `before \begin{document}`** — lecture-notes, math-sheet
  - `lecture-notes` p1 y=225.426: +0.1755 bp, cumulative, 27 line(s) after — “Contents”
  - `math-sheet` p1 y=197.992: -0.5133 bp, cumulative, 63 line(s) after — “Series and sums”

**5. `\section`** — math-sheet
  - `math-sheet` p1 y=336.208: -0.0666 bp, cumulative, 43 line(s) after — “Integrals”
  - `math-sheet` p1 y=534.637: -0.0667 bp, cumulative, 22 line(s) after — “Piecewise definitions”
  - `math-sheet` p1 y=621.847: -0.0661 bp, cumulative, 15 line(s) after — “Limits and derivatives”

**6. `before \begin{align}`** — math-sheet
  - `math-sheet` p1 y=220.844: -0.4437 bp, cumulative, 62 line(s) after — “n n”

**7. `unknown-boundary`** — conf-paper, hyperref-toc, math-sheet, natbib-review, plain-article, ps-calculus, siunitx-tables, thesis-chapter, twelvept-plain
  - `conf-paper` p4 y=767.888: +0.0688 bp, local, 1 line(s) after — “4”
  - `hyperref-toc` p4 y=749.888: -1.155 bp, local, 1 line(s) after — “4”
  - `math-sheet` p2 y=213.489: -2.7778 bp, cumulative, 28 line(s) after — “1 0 0”
  - `math-sheet` p2 y=227.038: -2.7773 bp, cumulative, 25 line(s) after — “0 1 0 I = M =”

**8. `\item`** — twelvept-plain
  - `twelvept-plain` p3 y=415.341: -0.2444 bp, cumulative, 23 line(s) after — “Growable delimiters are a kernel feature—you do no”
  - `twelvept-plain` p3 y=453.131: -0.2454 bp, cumulative, 21 line(s) after — “Nested fractions are the easiest way to stress-tes”
  - `twelvept-plain` p3 y=547.139: -0.2446 bp, cumulative, 16 line(s) after — “sin x”

**9. `row pitch inside {align}`** — ps-calculus
  - `ps-calculus` p2 y=242.225: -0.1612 bp, cumulative, 38 line(s) after — “1”

**10. `before \begin{itemize}`** — cv, hyperref-toc, twelvept-plain
  - `cv` p1 y=598.977: +5.977 bp, cumulative, 2 line(s) after — “Putnam Competition, top 500 (2026).”
  - `hyperref-toc` p4 y=107.308: +1.1554 bp, cumulative, 8 line(s) after — “Badness: a numeric measure of how far a line or pa”
  - `twelvept-plain` p3 y=391.998: -0.979 bp, cumulative, 24 line(s) after — “Always check the domain of validity before taking ”

**11. `after \end{lemma}`** — lecture-notes
  - `lecture-notes` p1 y=447.193: +0.1239 bp, cumulative, 17 line(s) after — “Proof. Write b = ka and c = for some k, Then c = =”
  - `lecture-notes` p1 y=503.973: +0.1239 bp, cumulative, 14 line(s) after — “Proof. Write b = ka and c = Then mb + nc = mka + =”

**12. `after \end{align*}`** — math-sheet
  - `math-sheet` p1 y=498.606: -0.4465 bp, cumulative, 23 line(s) after — “Integration by parts: u, dv = uv v, du.”

**13. `after \end{definition}`** — lecture-notes
  - `lecture-notes` p1 y=391.614: +0.1239 bp, cumulative, 20 line(s) after — “Example 1.2. 3 12 since 12 = 4 3, but 5 12. Every ”

**14. `\end{itemize} -> \begin{enumerate}`** — twelvept-plain
  - `twelvept-plain` p3 y=490.713: +6.7324 bp, cumulative, 19 line(s) after — “1. Redo the continued-fraction example above with ”

**15. `after \end{example}`** — lecture-notes
  - `lecture-notes` p1 y=426.178: +0.1239 bp, cumulative, 18 line(s) after — “Lemma 1.3. If a b and b c then a c.”

**16. `row pitch inside {enumerate}`** — twelvept-plain
  - `twelvept-plain` p3 y=528.503: -0.2454 bp, cumulative, 17 line(s) after — “2. Show that is differentiable wherever ⃗r(t) is, ”

**17. `page-origin`** — conf-paper, lab-report
  - `conf-paper` p4 y=652.082: -0.069 bp, page-origin, 3 line(s) after — “Figure 1: Tail latency (p99) across three producti”
  - `lab-report` p2 y=493.965: -0.0628 bp, page-origin, 13 line(s) after — “2”

**18. `after \end{enumerate}`** — twelvept-plain
  - `twelvept-plain` p3 y=570.645: -0.9767 bp, cumulative, 12 line(s) after — “1”

**19. `before \begin{figure}`** — thesis-chapter
  - `thesis-chapter` p2 y=604.039: -0.0861 bp, cumulative, 9 line(s) after — “Figure 1.1: Representative sparse sensor deploymen”

**20. `before \begin{tabular*}`** — cv
  - `cv` p1 y=521.506: +5.9775 bp, cumulative, 8 line(s) after — “Languages Rust, Swift, Python, C, Standard ML”

**21. `row pitch inside {theorem}`** — lecture-notes
  - `lecture-notes` p1 y=620.88: +0.1487 bp, cumulative, 8 line(s) after — “a = qb + r and 0 r < b.”

**22. `after \end{tabular}`** — plain-article
  - `plain-article` p2 y=500.42: -0.056 bp, cumulative, 8 line(s) after — “Table 1: Summary of proposed fixes.”

**23. `after \end{theorem}`** — lecture-notes
  - `lecture-notes` p1 y=643.588: +0.1481 bp, cumulative, 7 line(s) after — “Proof. Existence. Consider the set S = a kb : k a ”

**24. `after \end{proof}`** — lecture-notes
  - `lecture-notes` p1 y=720.0: -1.0761 bp, cumulative, 2 line(s) after — “Corollary 2.2. Every integer is either even or odd”

## Horizontal findings

| rank | probable cause | fixtures | occurrences | median |dx| (bp) | max |dx| (bp) | kind |
|---|---|---|---|---|---|---|
| 1 | `line-indent-or-margin` | 15 | 117 | 4.3994 | 356.6595 | line |
| 2 | `font substitution SFRM1095->LMRoman10-Regular` | 10 | 113 | 2.1453 | 446.314 | word |
| 3 | `math glyph advance` | 7 | 86 | 2.6374 | 21.0889 | word |
| 4 | `font substitution CMR10->LMRoman10-Regular` | 7 | 71 | 2.2811 | 409.6507 | word |
| 5 | `font substitution SFRM1000->LMRoman10-Regular` | 2 | 88 | 2.7721 | 246.2029 | word |
| 6 | `text advance / interword glue` | 1 | 38 | 4.9371 | 471.6361 | word |
| 7 | `font substitution SFRM1200->LMRoman12-Regular` | 1 | 26 | 4.0532 | 23.0127 | word |
| 8 | `font substitution SFTT1000->LMMono10-Regular` | 1 | 14 | 3.0316 | 572.0926 | word |
| 9 | `font substitution CMR9->LMRoman9-Regular` | 1 | 11 | 1.5271 | 297.6041 | word |
| 10 | `font substitution SFBX1095->LMRoman10-Bold` | 1 | 8 | 1.192 | 2.0413 | word |
| 11 | `font substitution SFTI1095->LMRoman10-Italic` | 1 | 8 | 0.8912 | 1.7731 | word |
| 12 | `font substitution SFTT1095->LMMono10-Regular` | 1 | 4 | 9.3375 | 12.4542 | word |
| 13 | `font substitution CMTI10->LMRoman10-Italic` | 1 | 4 | 0.7502 | 0.8273 | word |
| 14 | `font substitution SFTT1095->LMRoman10-Bold` | 1 | 3 | 2.5189 | 5.9367 | word |
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

| fixture | pages ref/cand | status | reflowed pages | cumulative steps | local steps | worst cumulative (bp) |
|---|---|---|---|---|---|---|
| article-twocolumn | 2/2 | recovered | 1 | 0 | 0 | 0.0 |
| conf-paper | 4/4 | recovered | 1 | 0 | 1 | 0.0 |
| cv | 1/1 | recovered | 0 | 5 | 0 | 5.978 |
| enumitem-worksheet | 3/3 | recovered | 0 | 0 | 1 | 0.0 |
| hw1 | 3/3 | recovered | 0 | 0 | 0 | 0.0 |
| hw2 | 3/3 | recovered | 0 | 0 | 0 | 0.0 |
| hyperref-toc | 4/4 | recovered | 1 | 1 | 1 | 1.155 |
| inline-math | 2/2 | recovered | 2 | 0 | 0 | 0.0 |
| input-bibliography | 2/2 | recovered | 1 | 0 | 0 | 0.0 |
| lab-report | 3/3 | recovered | 1 | 0 | 0 | 0.0 |
| lecture-notes | 2/2 | recovered | 0 | 8 | 0 | 1.076 |
| letter | 1/1 | recovered | 1 | 0 | 0 | 0.0 |
| listings-manual | 3/3 | recovered | 2 | 0 | 0 | 0.0 |
| lmodern-report | 4/4 | recovered | 0 | 0 | 0 | 0.0 |
| math-sheet | 2/2 | recovered | 0 | 11 | 1 | 2.779 |
| natbib-review | 3/3 | recovered | 0 | 1 | 1 | 0.061 |
| plain-article | 3/3 | recovered | 1 | 1 | 1 | 0.056 |
| ps-calculus | 3/3 | recovered | 0 | 4 | 2 | 0.161 |
| siunitx-tables | 2/2 | recovered | 0 | 0 | 1 | 0.0 |
| thesis-chapter | 5/5 | recovered | 0 | 1 | 1 | 0.086 |
| twelvept-plain | 3/3 | recovered | 0 | 13 | 1 | 6.732 |
| unicode-accents | 1/1 | recovered | 0 | 0 | 0 | 0.0 |

