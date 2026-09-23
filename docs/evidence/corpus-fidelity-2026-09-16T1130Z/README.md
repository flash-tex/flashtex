# Corpus fidelity against pdfLaTeX, ranked by blast radius — 2026-09-16

**GH-66.** Lane `daniel-parent` / `agent/daniel-parent/corpus-fidelity-66`, machine
`mac-m5pro-dq222`. Read-only audit: **no rendering behaviour is changed by this
evidence directory or by the PR that carries it.**

> ## Correction 2 — 2026-09-16, supersedes #750's *and* #753's ranking
>
> **Every cumulative step on a page whose glue pdfTeX set has been withdrawn from
> the ranking.** 704 of the sweep's 1031 displaced baselines — 68% — sat on ten
> such pages and were credited to the construct standing at their position. They
> are not that construct's. Two more findings are withdrawn (**F3**, **F11**) and
> a third (**F5**) drops to a single witness. Read
> [Why a step on a shrunk page proves nothing about its position](#why-a-step-on-a-shrunk-page-proves-nothing-about-its-position)
> before using any number below. The measurement itself did not move: every
> page's `dy_profile` and every `step_bp` in `cumulative.json` is byte-identical
> to #753's. Only the attribution changed.
>
> ## Correction 1 — 2026-09-16, supersedes the original numbers
>
> One of the twelve findings below, **F7**, was an artefact of the oracle and is
> **withdrawn**. `pdftext` splits a word at a new baseline, so pdfTeX's cmex10
> `\bigl(` — which carries its own origin 8.836 bp above the line — made the
> reference read `(` and `A…` as two words where we read `(A…` as one. `rank.norm`
> matched them on their alphanumeric content and the comparison then measured the
> two **word origins**, which differ by exactly the delimiter's advance. The
> delimiter's ink agreed with pdfLaTeX's to 0.083 bp all along. #752 fixed
> `rank.py` (`rank.pair_points` anchors an aligned pair at its first alphanumeric
> glyph, the content `norm` used to decide it *was* a pair); this PR applies the
> same anchor to `cumulative.py`, which measured word origins in
> `lines_from_pairs` and in its reflow test, and re-runs the full 22-fixture
> sweep. `cumulative.json`, `cumulative.md` and `probes-report/` in this directory
> are the re-run; every number below is the corrected one.
>
> **What moved.** F7 is withdrawn — all **eight** of its occurrences, across four
> fixtures, measure 0 once anchored. F8 shrinks from 173 to 164 words. Nothing
> else in the twelve changed.
>
> **What did not move, and could not have.** Every vertical finding — F1, F2, F3,
> F4, F5, F6, F11, F12 — is **byte-identical** before and after: the whole
> `causes` table, every page's `steps`, `dy_profile`, `lines` and `reflowed_words`
> across all 22 fixtures and 59 compared pages. That is not luck. A word never
> spans two baselines (`pdftext.words_from_glyphs` starts a new word at a baseline
> 0.05 bp away), so a word's first alphanumeric glyph always sits on the word's
> own baseline and re-anchoring can move a measured `x` but never a measured `y`.
> Measured over the corpus: the anchor moves `x` on 546 of 16 496 reference words
> and `y` on **0** of them. The `dy` profile the vertical findings rest on never
> touched the flawed comparison.

Producer: `flashtex-render --v2` -> `flashtex-pdf-exact from-v2` built from
`origin/main` `9c5b028d`, fonts `apps/mac/Fonts`. Reference: the pinned
pdfLaTeX PDFs in `fixtures/real-world/*/` (MacTeX 2026 / TeX Live 2025, see each
`reference.json`); the probe references were rendered now with
`/Library/TeX/texbin/pdflatex` (pdfTeX 1.40.29, TeX Live 2026), two passes,
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`. **pdflatex is an oracle only, never
in the product path, and nothing here is a parity claim.**

- `cumulative.md` / `cumulative.json` — the full sweep: 22 fixtures, 59 compared
  pages, every line's `dy`, every step, every horizontal outlier.
- `probes/` — 20 minimal documents that isolate the causes below, and
  `probes-report/cumulative.md` with their measured result. The reference PDFs
  are not committed; regenerate one with `cd probes/<id> && SOURCE_DATE_EPOCH=0
  FORCE_SOURCE_DATE=1 pdflatex -interaction=batchmode main.tex` (twice), then
  `mv main.pdf reference.pdf`.
- `tools/visual-oracle/cumulative.py` — the reusable script. It is a companion
  to `rank.py` and reuses its producer route, reference reader, word grouper and
  `difflib` alignment unchanged; what it adds is the classification
  **local or cumulative** and the grouping **by probable cause**.

## Why blast radius and not bp

`rank.py` ranks pages by their single largest delta. That answers "which page
looks worst"; it does not answer "what should be fixed first", and on this
corpus those are different documents. Today's own record is the argument: #706
was **1.99 bp** per list boundary and moved every baseline below it on 17
fixtures; #717 was **5.06 pt**, four times larger, and moved one glyph stack
inside one matrix. So every divergence here is measured by how many reference
baselines it displaces:

* a step is **cumulative** when the page's prevailing level changes at it — the
  median `dy` below it differs from the median above it. Its size is
  (step) x (lines below it);
* **local** when the page comes back. Its size is one line.

Project gates: **0.5 bp** glyph positions, **0.1 bp** rules. A cumulative step
under both gates still ruins the page, which is what the `lines` column is for.

## Reproduce

```sh
cargo build --release --manifest-path crates/render-pipeline/Cargo.toml --bin flashtex-render
cargo build --release --manifest-path crates/pdf/Cargo.toml --bin flashtex-pdf-exact
export FLASHTEX_RENDER=.../flashtex-render FLASHTEX_PDF_EXACT=.../flashtex-pdf-exact
python3 tools/visual-oracle/cumulative.py --out <evidence-dir>
python3 tools/visual-oracle/cumulative.py --fixtures <evidence-dir>/probes --out <evidence-dir>/probes-report
python3 -m unittest discover -s tools/visual-oracle -p 'test_*.py'   # 53 pure-python tests
```

The glue-set pass re-runs each fixture through `/Library/TeX/texbin/pdflatex`
under `\tracingoutput` (`--texbin` to point elsewhere, `--no-glue-set` to skip
it — which flags *every* step, because an unmeasured glue set is not a zero
one). The re-run is only believed after it is shown to be the pinned
reference's typesetting; `glue_set_source` on each page records which check
passed.

## Where the corpus stands

**Accumulated vertical drift, worst displacement reached on each page** (the 12
pages that reach anything; the other 36 measurable pages are under 0.08 bp):

| fixture | page | worst `dy` | pdfTeX's glue set | recovered by the page foot? |
|---|---|---|---|---|
| cv | 1 | **29.888 bp** | 0 (fil) | no — the page ends 29.888 bp low |
| twelvept-plain | 2 | 18.850 bp | 0 (fil) | yes — **and it is an artefact, not a defect**: see below |
| math-sheet | 2 | 5.554 bp | 0 (fil) | at the folio only |
| twelvept-plain | 3 | 5.019 bp | **− 0.53453** | at the folio only |
| math-sheet | 1 | 2.552 bp | **− 0.74042** | at the folio only |
| hyperref-toc | 4 | 1.155 bp | 0 (fil) | at the folio only |
| lecture-notes | 1 | 1.077 bp | **− 0.30122** | at the folio only |
| siunitx-tables | 1 | 0.413 bp | 0 (fil) | at the folio only |
| ps-calculus | 2 | 0.158 bp | 0 (fil) | at the folio only |
| ps-calculus | 1 | 0.151 bp | **− 0.02837** | at the folio only |
| thesis-chapter | 2 | 0.085 bp | 0 (fil) | at the folio only |
| enumitem-worksheet | 1 | 0.080 bp | **− 0.20364** | at the folio only |

The displacement on the four **bold** pages is real — those baselines are where
the table says — but on those pages it is *not* localised to the construct the
step sits at. The three largest numbers in this table are all on unshrunk pages,
so `cv` p1, `twelvept-plain` p2 and `math-sheet` p2 are unaffected.

"At the folio only" means the page number is placed correctly by the page
builder while the text block above it has drifted — so the drift is real body
text displacement, not a page-origin offset. `cv` sets `\pagestyle{empty}`, so
it has no folio to hide behind.

`twelvept-plain` page 2's 18.850 bp is the **one number in this sweep that is an
alignment artefact rather than a layout defect**, and it is worth stating
because it is the biggest number on the page: the reference PDF's cmex `\int`
extracts as the OT1 letter `Z` where the candidate emits `U+222B`, so the big
operator never aligns and the integral's upper limit `b` paired across the
display instead. `cumulative.py` marks such lines low confidence and excludes
them from the ranked table (`cumulative.md`, "Low-confidence steps"); probe
`v1-lim-sum` reproduces the alignment failure and shows the real displacement
for that display is **−0.066 bp** (F2).

### What is already good

* **All 22 fixtures now produce the reference page count.** The 2026-09-13 sweep
  (`docs/evidence/corpus-fidelity-2026-09-13T1548Z/`) had five that did not
  (`lab-report` 2/3, `conf-paper` 2/4, `thesis-chapter` 3/5, `twelvept-plain`
  4/3, `article-twocolumn` 1/2).
* **HW1 and HW2 have zero vertical steps above 0.05 bp on all six pages.** Every
  baseline on both documents is within 0.055 bp of pdfLaTeX's, and neither has a
  reflowed word. Their remaining divergences are horizontal only (F8, F10);
  `rank.py`'s worst horizontal delta on them is 1.086 bp (hw1 p1 / hw2 p1),
  0.611 bp (hw1 p2) and 0.432 bp (hw2 p2) — the 5.000 bp that used to head this
  list was F7's measurement artefact.
* **The vertical skip constants are right.** Probes `p5-12pt-display` (plain
  `\[x=a+b\]` at 12 pt), `p8-align` (plain `align`/`align*` at 11 pt),
  `p4-parskip` (`\parskip 4pt`), `q1-vspace` (`\vspace{6pt}` between
  paragraphs), `p2-negbreak` / `t1-endpar-neg` / `s2-large-only` (`\\[-6pt]`)
  and `p3-rule` (`\rule`) each measure **0.000 bp**. Every vertical finding
  below is a *box height*, an *environment boundary* or a *macro expansion*
  defect — not a wrong skip.

## Why a step on a shrunk page proves nothing about its position

TeX fits a page by **setting its vertical glue**: every stretchable or shrinkable
skip on the page is scaled by one page-global ratio, which `\tracingoutput`
prints as `glue set`. A baseline on such a page therefore sits at

```
natural position  -  ratio x (shrinkability accumulated above it)
```

If the two producers' *natural* page heights differ at all — for any reason,
anywhere on the page, **including below the line being looked at** — their ratios
differ and every baseline separates by an amount proportional to the shrinkable
glue above it. Shrinkable glue is `\abovedisplayskip`, `\topsep`, `\itemsep`,
`\parskip` and the `\@startsection` skips: exactly the constructs `cumulative.py`
names its causes after. So a shrunk page manufactures a step at every list, every
display and every heading, each sized by that construct's own shrink component
and none of them caused by it.

**This is how 90 lines got double-counted.** #755 measured `\maketitle` exact to
0.0009 bp, found both F3 pages shrunk (`- 0.74042`, `- 0.30122`), and collapsed
the steps to +0.0037 and +0.0041 bp by truncating each page so it no longer
overflowed. `cumulative.py` now measures the glue set of every page and flags
every step on a finitely-set one as **cause not localised**.

### The corpus's ten set-glue pages

Read from pdfTeX's own `\tracingoutput` on a re-run of each fixture that is first
verified to be the pinned reference's typesetting (8 of 22 re-runs are
byte-identical to the pinned PDF; the other 14 were pinned under TeX Live 2025
and differ in bytes with **every baseline within 0.001 bp**).

| fixture | page | glue set | ranked steps on it | lines they carried |
|---|---|---|---|---|
| math-sheet | 1 | **− 0.74042** | 11 | 238 |
| twelvept-plain | 3 | **− 0.53453** | 13 | 294 |
| lecture-notes | 1 | **− 0.30122** | 8 | 113 |
| ps-calculus | 1 | **− 0.02837** | 2 | 59 |
| listings-manual | 1 | − 0.65112 | 0 (page reflowed) | 0 |
| listings-manual | 2 | − 0.37480 | 0 (page reflowed) | 0 |
| lmodern-report | 1 | − 0.26439 | 0 | 0 |
| enumitem-worksheet | 1 | − 0.20364 | 1, local | 0 |
| enumitem-worksheet | 2 | − 0.16081 | 0 | 0 |
| thesis-chapter | 3 | − 0.07123 | 0 | 0 |

Every other compared page has a **fil**-order set or none: a short page whose
`\vfil` absorbs the slack leaves every finite glue at its natural size and
displaces nothing, so it is not flagged. That is why `cv` page 1 — the corpus's
largest cumulative defect — is untouched by this correction. **All 20 probes are
unshrunk too**, so every probe number in this document stands.

### One ratio explains a whole page

Not asserted — fitted. For each shrunk page, take each step and divide it by the
shrink component of the glue standing at its boundary, read from that page's own
`\showboxbreadth=500` dump. If the mechanism is right, every step returns the
same ratio difference `Δr`.

**`math-sheet` page 1** (`\abovedisplayskip`/`\belowdisplayskip` = `11.0 plus 3.0
minus 6.0`, the `\@startsection` skip `16.49693 plus 4.71341 minus 0.94266`,
`\@topsepadd` `12.0 plus 4.0 minus 6.0`):

| step, as labelled | bp | shrink at that boundary | implied `Δr` |
|---|---|---|---|
| `before \begin{document}` (F3) | −0.5133 | 6.9167 bp (`\@topsepadd` + section) | 0.0742 |
| `before \begin{align}` (F2) | −0.4437 | 5.9776 bp | 0.0742 |
| `after \end{align*}` (F2) | −0.4465 | 5.9776 bp | 0.0747 |
| `\section` x3 (F11) | −0.0666 / −0.0667 / −0.0661 | 0.9391 bp | 0.0709 / 0.0710 / 0.0704 |

#755 derived the implied candidate ratio 0.81514 against pdfTeX's 0.74042 —
`Δr` = **0.07472** — from the page builder alone. The display boundaries return
0.0742–0.0747 independently.

**`twelvept-plain` page 3** (`\abovedisplayskip` `12.0 plus 3.0 minus 7.0`,
`\topsep`+`\partopsep` `13.0 plus 6.0 minus 8.0`, `\itemsep`+`\parsep` 2 pt):

| step, as labelled | bp | shrink | implied `Δr` |
|---|---|---|---|
| `line-break-within-paragraph` x2, `display-math boundary` (F2) | −0.8575 / −0.8577 / −0.8586 | 6.9738 bp | 0.1230 / 0.1230 / 0.1231 |
| `before \begin{itemize}`, `after \end{enumerate}` (F4) | −0.9790 / −0.9767 | 7.9701 bp | 0.1228 / 0.1226 |
| `\item` x3, `row pitch inside {enumerate}` (F4) | −0.2444 / −0.2454 / −0.2446 / −0.2454 | 1.9925 bp | 0.1227 / 0.1232 / 0.1228 / 0.1232 |
| `paragraph-break` (F11) | −0.1189 | 0.9963 bp (`\parskip`) | 0.1193 |
| **`\end{itemize}` -> `\begin{enumerate}` (F4)** | **+6.7324** | would need **54.8 bp**, and the sign is wrong | **not the artefact** |

Nine steps, three different constructs, `Δr` = **0.1229 ± 0.0003**.

**`lecture-notes` page 1** (amsthm `\topsep` `9.0 plus 3.0 minus 5.0`,
`\abovedisplayskip` `11.0 plus 3.0 minus 6.0`):

| step, as labelled | bp | shrink | implied `Δr` |
|---|---|---|---|
| `after \end{lemma}` x2, `\end{definition}`, `\end{example}` (F5) | +0.1239 x4 | 4.9813 bp | 0.02487 |
| `row pitch inside {theorem}`, `after \end{theorem}` (F5) | +0.1487 / +0.1481 | 5.9776 bp | 0.02488 / 0.02478 |
| `before \begin{document}` (F3) | +0.1755 | ~7.06 bp | 0.02485 |
| **`after \end{proof}` (F5)** | **−1.0761** | would need **43 bp**, and the sign is wrong | **not the artefact** |

`Δr` = **0.02485 ± 0.00005**. #755's independently implied ratio was 0.27638
against 0.30122 — `Δr` = **0.02484**. Four significant figures, from two
unrelated derivations.

`ps-calculus` page 1's two steps are +0.1344 and −0.1587, **opposite signs**, so
at most one of them can be this page's artefact and the ratio cannot be fitted.
Both stay flagged; neither is resolved.

## Ranked findings — re-attributed

`lines` = reference baselines displaced by steps whose cause **is** localised.
`double-counted` = baselines displaced on a set-glue page, which belong to
whatever made that page's natural height differ, not to the construct named.
`fx` = fixtures out of 22 that show it.

Two rows here are more generous than the tool. `cumulative.md`'s machine count
is **327** attributable lines: it flags *every* step on a set-glue page, with no
exception. This table restores 21 of them — F4's +6.732 bp (19 lines) and F5's
−1.076 bp (2) — because both have the wrong sign for their page's artefact and
are an order of magnitude larger than any shrink available on it, so the flag is
conservative there and the evidence is not. **351 attributable, 680 withdrawn.**
Nothing else is restored by hand.

| # | finding | size | fx | lines | double-counted | > 0.5 bp gate | status |
|---|---|---|---|---|---|---|---|
| F6 | two matrices in one display fall out of register | ±2.778 bp, 4x (−5.55 bp net) | 1 | **102** | 0 | yes (5x) | open, unfiled — **unaffected**, `math-sheet` p2 is unshrunk |
| F2 | display-math **box height** whenever the display holds a `\frac`, `\sqrt` or an operator with limits | 0.05 – 0.88 bp per display | 3 of 5 | **101** | 273 | 1 of 15 | **#754** — survives on 6 unshrunk probes and 4 unshrunk pages; corpus line count was inflated 3.7x |
| F1 | `\\[<len>]` inside a `\newcommand` body drops its skip | **6.00 pt** (5.978 bp) each, 5x on one page | 1 | **83** | 0 | yes (12x) | **already fixed; behind `linebreak-skip`, invisible until the re-pin (#711)** — unaffected |
| F4 | `\end{itemize}` -> `\begin{enumerate}` shares one `\addvspace`; entering a list | +6.73 bp, +1.16 bp | 2 | **27** | 113 | 2 of 2 | **#706 + open PR stack #712 -> #722 -> #728 -> #738 -> #741** — the two big steps survive; see below |
| F12 | float and `tabular` boundaries | −0.056 / −0.086 / −0.069 bp | 3 | **20** | 0 | no (over the rule gate) | open, unfiled — unaffected |
| — | page origin handed over by the previous page (`lab-report` p2; `conf-paper` p4's is counted in F12) | −0.063 bp | 1 | **13** | 0 | no | open, unfiled |
| F5 | `\end{proof}` closing skip | **−1.076 bp** | 1 | **2** | 84 | yes | plausibly #722. **The four `\end{definition}`/`\end{example}`/`\end{lemma}` +0.1239 bp steps and the two `theorem` +0.148 bp steps are withdrawn** |
| ~~F3~~ | ~~`\maketitle` title-block height~~ | ~~−0.513 / +0.176 bp~~ | 0 | **0** | 90 | — | **WITHDRAWN (#755).** `\maketitle` is exact to 0.0009 bp; both steps are `Δr` x the shrink above the first body line |
| ~~F11~~ | ~~section-heading skip (`\section*` at 11 pt, `\section` at 12 pt)~~ | ~~−0.067 x3, −0.119 bp~~ | 0 | **0** | 116 | no | **WITHDRAWN (this correction).** All four steps are `Δr` x `\@startsection`'s or `\parskip`'s own shrink, on two shrunk pages, with no unshrunk witness and no probe |
| ~~F7~~ | ~~`\bigl(` / `\Bigl[` opening delimiter beside other math~~ | ~~"exactly −5.000"~~ | 0 | **0** | — | no | **WITHDRAWN (#752/#753).** Not a defect: the oracle measured two word origins the producers segment differently |

The horizontal and structural findings are unchanged: a page's glue set is a
**vertical** quantity, so it cannot move a word's `x` or change where a line
breaks. F8, F9 and F10 stand exactly as measured.

| # | finding | size | local / structural | fx | words / pages | > 0.5 bp gate | status |
|---|---|---|---|---|---|---|---|
| F8 | interword glue distribution inside a justified line | ~0.20 bp per space, ±1.1 to ±2.0 bp by line end | local, grows *along* the line | 12 | 164 words | yes | partly #710 (font metrics) |
| F9 | pages where the two sides break lines differently | 1 to 52 reflowed words | structural | 9 | 11 pages | n/a | mixed; 2 of the 9 are the known `ec_metrics_unavailable` pair |
| F10 | math Ord kerns (TFM lig/kern) and the `\normalfont` heading space | −0.911 bp / +0.604 bp | local (horizontal) | 2 | 5 words | yes | **already fixed; behind `math-font-kerns`, invisible until the re-pin (#711)** |

### What the re-attribution costs the totals

1031 displaced baselines were ranked; **351 survive as attributable and 680 are
withdrawn** (the tool's own conservative count is 327/704; see the note above the
ranked table). Three of the original twelve findings are now withdrawn entirely
(F7 as an instrument artefact, F3 and F11 as double-counts) and a fourth (F5)
keeps one of its seven witnesses. The two largest surviving vertical findings,
**F6 and F1, are untouched** — both sit on pages pdfTeX did not shrink.

### F1 — `\\[<len>]` inside a `\newcommand` body drops its skip

`fixtures/real-world/cv` defines

```tex
\newcommand{\cvsection}[1]{\vspace{6pt}{\large\bfseries #1}\\[-6pt]\rule{\textwidth}{0.6pt}\vspace{2pt}}
```

and calls it five times. The `dy` profile of page 1 is a clean staircase, one
tread per `\cvsection`, each exactly 6.00 TeX pt (5.9776 bp):

```
0.000 0.000 0.000 | 5.978 x5 | 11.955 x10 | 17.933 x8 | 23.910 x6 | 29.887 x2
```

By the foot of the page every baseline is **29.89 bp (10.5 mm) too low** — by a
wide margin the largest body displacement in the corpus, and the only page that
does not recover.

The probes isolate it to the optional argument, not to the rest of the macro:

| probe | body | result |
|---|---|---|
| `r1-cvsec-blank` | the `cv` macro verbatim | **+5.978 bp per call** |
| `r2-cvsec-norule` | same, `\rule` removed | +5.978 |
| `r3-cvsec-novspace` | same, `\vspace{6pt}` removed | +5.977 |
| `r4-cvsec-plainbreak` | same, `\\[-6pt]` -> `\\` | **0.000** |
| `t3-macro-mid` | a bare `\newcommand{\head}[1]{{\large\bfseries #1}\\[-6pt]\rule{...}}` | +5.978 |
| `t1-endpar-neg` | `Heading\\[-6pt]` in body text, no macro | **0.000** |
| `s2-large-only` | `{\large Heading}\\[-6pt]` in body text, no macro | **0.000** |

**Already fixed on the compiler side.** `crates/render-pipeline/Cargo.toml`
carries a `linebreak-skip` feature whose comment names this fixture and this
number: *"the byte scan cannot see the argument of a `\\` that came from a macro
body … which silently drops every `\\[-6pt]` inside a `\newcommand`
(`fixtures/real-world/cv`: +5.98bp per `\cvsection`,
`tests/line_break_skip_macro.rs`) … off until `vendor/` is re-pinned."` This is
re-pin work (#711), not new engineering — the measurement above is what that
re-pin is worth.

### F2 — display-math box height

> **Re-attributed.** The finding stands; the corpus line count does not. All
> **eight** probes below are on unshrunk pages, so the probe column is the
> measurement to trust. Of F2's 374 corpus lines, **273 sit on set-glue pages**
> (`twelvept-plain` p3 119, `math-sheet` p1 95, `ps-calculus` p1 59) and are
> withdrawn — on those pages the "per display" steps are `Δr` x the display
> skip's own 6 or 7 pt shrink, to three decimal places. **101 lines survive**
> (`ps-calculus` p2 38 and p3 28, `twelvept-plain` p2 30, `natbib-review` p1 5).
> The lane behind **#754 is not chasing a double-count** — the probes are
> independent evidence — but the "374 lines, worst in the corpus" framing was.

The display *skips* are exact (`p5-12pt-display` and `p8-align` both 0.000 bp).
What is not exact is how tall the display box is, and the error scales with the
construct inside it:

| probe | display | step |
|---|---|---|
| `p5-12pt-display` | `\[ x = a + b \]` | **0.000** |
| `p8-align` | `align` / `align*`, text only | **0.000** |
| `q4-sum-int` | `\sum_{n=0}^{\infty} x^n = \frac{1}{1-x}` | −0.060 |
| `q5-bigdelim` | `\left(\frac{a}{b}\right)`, `\sqrt{\frac{a}{b}}` | −0.067 |
| `u2-manual-delims` | `\bigl(…\bigr), \Bigl[…\Bigr], \biggl\{…\biggr\}, \Biggl\langle…\Biggr\rangle` | −0.067 |
| `v1-lim-sum` | `\int_a^b f\,dx = \lim_{n\to\infty}\sum_{i=1}^{n} f\,\Delta x` | −0.066 |
| `q6-contfrac` | 3-level nested `\frac` | −0.176 |
| `u1-lim-frac12` | `\lim_{x\to 0}\frac{\sin x}{x} = 1` at 12 pt | **−0.406** |

On the corpus they compound. `twelvept-plain` page 3 is the worst run —

```
0.00  −0.86  −0.86  −0.86  −0.86  −1.72 … −1.84 … −2.69 … −3.55 … −4.53  −4.77  −5.02
```

— nine display and inline-display lines, each taking 0.06 to 0.98 bp, reaching
**−5.02 bp** before the lists even start. `math-sheet` page 1 loses 0.444 bp
entering each `align` and 0.447 leaving it, four times. `ps-calculus`
contributes 0.05–0.16 bp steps on all three pages.

Every one of these except one is inside the 0.5 bp glyph gate, which is exactly
why none has been filed. Owner: `crates/math-layout` box metrics (height and
depth of `Fraction`, `Radical`, and `Operator`-with-limits), then
`crates/render-pipeline` display placement.

### F4 — list boundaries (covered by open work)

> **Re-attributed, and the PR stack's lane should read this row.** Only two of
> the five boundaries survive. `twelvept-plain` page 3 is shrunk by `- 0.53453`,
> and every *negative* step on it is `Δr` = 0.1229 x that boundary's own shrink,
> to within 0.3%. The **+6.732 bp** step is not: it has the wrong sign for the
> page's artefact and would need 54.8 bp of shrinkable glue where the largest on
> the page is 7.97 bp. So the #706 shape is real and the stack is right to chase
> it — but **the −0.979 / −0.977 entry/exit steps and all four `\item` -> `\item`
> −0.245 bp steps are not defects at all**, and 113 of F4's 140 lines are
> withdrawn. Do not use `\itemsep` as evidence: `−0.245 bp = 0.1229 x
> (\itemsep + \parsep)`'s 2 pt.

| boundary | step | fixture | lines below | verdict |
|---|---|---|---|---|
| `\end{itemize}` -> `\begin{enumerate}` | **+6.732 bp** | twelvept-plain p3 | 19 | **real** — wrong sign and 7x too large for the page's glue set |
| paragraph -> `\begin{itemize}` | **+1.155 bp** | hyperref-toc p4 | 8 | **real** — the page is unshrunk |
| paragraph -> `\begin{itemize}` | −0.979 bp | twelvept-plain p3 | 24 | **withdrawn** — `Δr` x 7.970 bp (`\topsep`+`\partopsep`) |
| `\end{enumerate}` -> paragraph | −0.977 bp | twelvept-plain p3 | 12 | **withdrawn** — same glue |
| `\item` -> `\item` | −0.245 bp x4 | twelvept-plain p3 | 23 + 21 + 17 + 16 | **withdrawn** — `Δr` x 1.993 bp (`\itemsep`+`\parsep`) |

The first four are the #706 shape (adjacent list environments sharing one
`\addvspace`) and belong to the open PR stack #712 -> #722 -> #728 -> #738 ->
#741, **none of which is on `main` yet** — this sweep measures `main`, so they
are expected to still be here. Re-measure this table once that stack merges.
~~The `\item` -> `\item` −0.245 bp is `\itemsep`, not a boundary, and is not
obviously inside that stack's scope.~~ It is not a defect; see the box above.

### ~~F11~~ — WITHDRAWN: the section-heading skip was the page's shrink ratio

**This finding does not exist.** Its original text is kept below the line.

`math-sheet` (`\section*` at 11 pt): −0.0666, −0.0667, −0.0661 bp, carrying 43,
22 and 15 lines. `twelvept-plain` (`\section` at 12 pt): −0.1189 bp over 36
lines. **Both fixtures' pages are shrunk** (`- 0.74042`, `- 0.53453`), and the
four steps are that page's ratio difference times the shrink component of the
skip standing at the boundary — `\@startsection`'s `\@minus.2ex` = 0.94266 pt on
`math-sheet`, `\parskip`'s 1 pt on `twelvept-plain`:

| fixture | step | boundary shrink | implied `Δr` | page's `Δr` from its *other* boundaries |
|---|---|---|---|---|
| math-sheet | −0.0666 / −0.0667 / −0.0661 | 0.9391 bp | 0.0709 / 0.0710 / 0.0704 | 0.0742 – 0.0747 |
| twelvept-plain | −0.1189 | 0.9963 bp | 0.1193 | 0.1226 – 0.1232 |

F11 had **no unshrunk witness and no probe.** Its 116 lines belong to whatever
made those two pages' natural heights differ — on `math-sheet` that is F2 (the
only step on the page the ratio does not explain), on `twelvept-plain` it is F4's
+6.732 bp and F2. The `\section` numbers are consistent with the page ratio to
within 5%, not to within 0.3% as the same-page display and list boundaries are;
that residue is the honest limit of this decomposition and is *not* an argument
for a defect — it is smaller than the withdrawn step itself. **Re-measure
`\section` on a probe before filing anything.** The probe set does not contain
one; `p5`, `p8`, `q1`, `p2`, `t1`, `s2` and `p3` all measure 0.000 bp for the
other vertical skips.

### F6 — two matrices in one display

`math-sheet` page 2 sets `I = \begin{pmatrix}…\end{pmatrix}` and
`M = \begin{pmatrix}…\end{pmatrix}` in one display. The rows fall out of
register by ±2.778 bp four times, leaving the last 22 lines of the page −5.545
bp out — which the folio step at the page foot reads back exactly (+5.5445 bp).

**Caveat that bounds the claim:** the bracket extension glyphs never align (the
reference PDF gives cmex the PUA codepoint `U+F8EE` where the candidate emits
`⎡`), so 2.778 bp is the *row-to-row* register between the two matrices, not the
bracket position. The bracket pieces themselves sit 16.4 bp apart on the two
sides and word alignment cannot measure them at all.

### ~~F3~~ — WITHDRAWN by #755: `\maketitle` is exact

**This finding does not exist.** `\maketitle`'s block is pdfLaTeX's to within
0.0009 bp at 10, 11 and 12 pt, with and without an author and a date
(`crates/render-pipeline/tests/maketitle_block_height.rs`,
`docs/evidence/maketitle-block-height-2026-09-16/`). Its original text:

> `math-sheet` (`\title{…}\author{}\date{}`, `\maketitle`,
> `\thispagestyle{empty}`): the first body line is **−0.513 bp** and the whole
> 63-line page carries it. `lecture-notes` (`\maketitle` + `\tableofcontents`):
> **+0.176 bp** over 27 lines. Opposite signs, so it is not one constant.

The opposite signs were the tell, and this correction now places F3 in a family:
it is the *first* step on each of two shrunk pages, so it is `Δr` x the whole
shrinkability standing above the first body line — 0.0742 x 6.92 bp and 0.02485
x ~7.06 bp. Every other step on both pages is the same `Δr` x its own boundary.

### F5 — `\end{proof}` closing skip (six of its seven steps withdrawn)

> **Re-attributed.** `lecture-notes` page 1 is shrunk by `- 0.30122`, and six of
> F5's seven steps are `Δr` = 0.02485 x the amsthm `\topsep`'s 5 pt or the
> display skip's 6 pt shrink — 0.02478 to 0.02488, a 0.4% spread. `\end{proof}`
> **−1.0761 bp** is not: wrong sign for the page's artefact, and it would need
> 43 bp of shrinkable glue where the largest on the page is 5.98 bp. So
> `\end{proof}` is the finding and the other six are its shadow — which also
> sharpens #755, whose text attributed the page's height difference to "the
> amsthm closing skips" in general: it is **`\end{proof}` specifically**.

`lecture-notes`, seven cumulative steps: ~~`\end{definition}` +0.1239 (20 lines),
`\end{example}` +0.1239 (18), `\end{lemma}` +0.1239 (17 and 14), inside
`theorem` +0.1487 (8), `\end{theorem}` +0.1481 (7)~~, `\end{proof}` **−1.0761**
(2 lines). `\end{proof}` is over both gates. `proof` is a `\trivlist`, so it is
plausibly inside PR #722's scope; **the four theorem-like environments are no
longer a finding.**

### F12 — float and `tabular` boundaries

`plain-article` p2 `\end{tabular}` −0.0560 bp (8 lines), `thesis-chapter` p2
`\begin{figure}` −0.0861 bp (9 lines), `conf-paper` p4 page origin −0.0690 bp
(3 lines). All over the 0.1 bp rule gate only marginally; listed so the next
sweep can tell whether they move. **All three pages are unshrunk** —
`thesis-chapter` p3 is the shrunk one and carries no ranked step — so F12 is
untouched by this correction.

### ~~F7~~ — WITHDRAWN: `\bigl(` was never misplaced; the oracle measured the wrong point

**This finding does not exist.** The original text is kept below the line for the
record; the correction is here.

The number was a constant because it *was* a constant: a glyph advance. pdfTeX
sets `\bigl(` from **cmex10**, whose variant glyph carries its own origin, and
emits it on a baseline 8.836 bp above the line; ours is a LatinModernMath variant
on the math baseline. `pdftext.words_from_glyphs` starts a new word at a new
baseline, so the **reference** reads `(` and `A…` as two words while **we** read
`(A…` as one. `rank.norm` strips punctuation for identity, so `(A` still matched
`A` — and the comparison then measured the two **word origins**, which differ by
exactly the delimiter's advance: 4.996 bp for `\big(`, 5.149 bp for `\Big[`.

The engine was exact. Both PDFs put the delimiter's origin at x = 243.037 and the
following `A` at x = 248.037, and the delimiter's **ink** agrees to 0.083 bp
vertically.

Anchoring each aligned pair at its first alphanumeric glyph — the content `norm`
used to decide it was a pair — is `rank.pair_points` (#752 for `rank.py`, this PR
for `cumulative.py`). Every F7 occurrence in the sweep then measures 0:

| fixture | word | deviation, word origins | deviation, anchored |
|---|---|---|---|
| hw1 p2 | `(P(x)` | −5.0001 | **none — under the 0.5 bp gate** |
| hw2 p1 | `(A` | −5.0000 | **none** |
| hw2 p1 | `(A` | −5.0000 | **none** |
| hw2 p2 | `(C` | −4.9980 | **none** |
| math-sheet p1 | `)a` | −4.9994 | **none** |
| math-sheet p1 | `(f(x)g(x))` | −4.9986 | **none** |
| ps-calculus p2 | `(4` | −4.9975 | **none** |
| ps-calculus p2 | `[arctan` | −5.1499 | **none** |

Eight occurrences across four fixtures, all gone; the horizontal cause group
`math glyph advance` drops from 86 occurrences / 21.089 bp worst to 78 / 12.891
bp, and the corpus's total horizontal outliers from 565 to 553. `rank.py`'s
page maxima move with it: hw1 p2 `dx_max` 5.000 → 0.611, hw2 p1 5.000 → 1.086,
hw2 p2's `(C` pair 4.998 → 0.432.

Probe `u2-manual-delims` was right after all: it sets all four manual sizes on
their own and showed **no** horizontal outlier, because with nothing beside them
both producers segment the words the same way. That the artefact needed "a
relation, a big operator or `\displaystyle`" beside it was the tell — those are
what put a cmex glyph on its own baseline.

<details><summary>Original text of F7, superseded</summary>

> ### F7 — `\bigl(` / `\Bigl[` beside other math: exactly −5.000 bp
>
> | fixture | source | deviation from the line |
> |---|---|---|
> | hw2 p1 | `\bigl(A\cap B\ne\varnothing\bigr) \Longrightarrow \bigl(A\setminus B\subsetneq A\bigr)` | **−5.000** (x2) |
> | hw2 p2 | `\bigl(C…` | −4.998 |
> | ps-calculus p2 | `$\displaystyle\int \sqrt{\bigl(4 - x^2\bigr)}\,dx$` | −4.997 |
> | ps-calculus p2 | `= \lim_{b\to\infty}\Bigl[\arctan x\Bigr]` | −5.150 |
> | math-sheet p1 | `$(a+b)^n = \sum_{k=0}^{n}\binom{n}{k}…$` | −4.999 |
>
> A repeated, exact 5.000 bp is a constant, not an accumulation of metrics. Probe
> `u2-manual-delims` sets all four manual sizes on their own and shows **no**
> horizontal outlier, so the plain delimiter is right; the offset appears only
> when the delimited group sits beside a relation, a big operator or
> `\displaystyle`. **I did not find the cause and did not guess one** — this needs
> a narrower probe before it is filed.

</details>

### F8 — interword glue inside a justified line

The most widespread finding: **164** words over the 0.5 bp gate across the 12
fixtures with measurable pages, with a characteristic shape — the deviation
grows monotonically along the line and resets on the next one:

```
lecture-notes  p1 line 23:  Consider +0.561   the +0.757   set +0.944   S +1.131
listings-manual p1 line 36:  the −0.514   toolchain −0.742   with −0.961   ftxc −1.183
```

about 0.19 to 0.22 bp per interword space. Local — no line inherits the previous
line's error — but present on every prose fixture: lecture-notes 45,
listings-manual 27, math-sheet 24, unicode-accents 16, article-twocolumn 15,
natbib-review 11, hw1 7, ps-calculus 7, hw2 5, cv 3, lab-report 3,
siunitx-tables 1. Partly the font-metric work already measured in #710.

The nine words this count lost to the correction were not interword glue: eight
were F7's delimiter pairs (hw1 −1, hw2 −3, math-sheet −2, ps-calculus −2) and one
was a word whose line median moved once those pairs were anchored, taking it back
under the gate (`listings-manual` p1 `--%s",`, −0.5323). The *shape* of F8 —
monotone growth along a line, reset on the next — is unchanged, and so are the
example lines above to the fourth decimal.

### F9 — pages that break lines differently

No `dy` profile is measurable on these, so they rank on their own:

| fixture | page | reflowed words | aligned / reference words |
|---|---|---|---|
| listings-manual | 3 | 52 | 227/236 |
| listings-manual | 2 | 12 | 271/278 |
| lab-report | 1 | 3 | 301/311 |
| letter | 1 | 3 | 227/232 |
| inline-math | 1 | 2 | 727/855 |
| article-twocolumn | 1 | 2 | 506/527 |
| conf-paper | 1 | 2 | 643/652 |
| plain-article | 1 | 2 | 213/220 |
| inline-math | 2 | 1 | 307/358 |
| hyperref-toc | 1 | 1 | 493/505 |
| input-bibliography | 2 | 1 | 233/235 |

`listings-manual` and `hyperref-toc` are the two fixtures the corpus README
already flags as `ec_metrics_unavailable` (no `ectt*.tfm` is bundled, so
`\texttt` falls back to `ec-lmr*` metrics): **their line breaks are not a
measurement of this engine.** `inline-math` also has the lowest word alignment
in the corpus (727/855 and 307/358), which is upstream text divergence rather
than line breaking.

### F10 — math Ord kerns and the heading space (covered by the re-pin)

`hw1` page 2 `$a + r,$` is −0.911 bp, and `hw1`/`hw2` page 1 `This assignment`
is +0.604 / +0.508 bp. Both are already named in issue #66's own measurement
comment (causes 2 and 3). The kern half is **fixed behind the `math-font-kerns`
feature** in `crates/render-pipeline/Cargo.toml`, whose comment cites *"cmmi `r`
`,` = -0.0556 em: HW1's `$r,a,b$`, #66 … off until `vendor/math-layout` is
re-pinned."*

## Blocked on the re-pin (#711), not on engineering

Three feature flags in `crates/render-pipeline/Cargo.toml` are off "until
`vendor/` is re-pinned". Two of them account for findings measured above; the
third this sweep cannot separate from F2 and F11:

| feature | what it fixes | finding |
|---|---|---|
| `linebreak-skip` | `\\[<dimen>]` read from `Inline::LineBreak::skip_pt` instead of a byte scan | **F1** — 29.89 bp of drift on `cv` page 1 |
| `math-font-kerns` | TeX `make_ord` math kerns and roman math ligatures | **F10** |
| `par-leading` | a paragraph's `\baselineskip` follows the size in force at its `\par` (`\begin{quote}\small …`) | not separable here; `twelvept-plain` p3 holds the corpus's one `quote` |

**Do not count F1 or F10 as open engineering work.** They are the re-pin's
payload — and F1 is the largest cumulative defect in the corpus.

## What I could not measure

* **Anything on the 11 reflowed pages** (F9): the two sides do not share lines
  there, so no vertical profile exists. Two of those fixtures are additionally
  running on substituted `ectt` metrics.
* **Rules, leaders, and bracket extension pieces.** Word alignment only sees
  glyphs that extract as text on *both* sides. cmex big operators and growable
  bracket pieces do not (F6), so **the 0.1 bp rule gate is not exercised by this
  sweep at all.** `rank.py`'s pixel comparison is the nearest existing check.
* **Whether PRs #712–#741 close F4.** They are not on `main`, and building each
  branch is the integration lane's call, not a read-only audit's.
* ~~**F7's cause.**~~ **Found, and it was the instrument.** The number was exact
  and repeatable and the minimal probe did not reproduce it because there was no
  defect to reproduce — the comparison was anchored at two word origins the two
  producers had segmented differently. See the withdrawn F7 above. The right
  lesson is the general one: *a divergence that is exactly constant across
  documents is more likely a metric of the measurement than a metric of the
  layout*, and this sweep now anchors identity and position on the same content.
* **Whether any other finding hides the same shape.** Checked, not assumed: the
  correction was applied and the whole sweep re-run. The vertical half is
  byte-identical (it cannot move — see the Correction at the top), and the
  horizontal half lost only F7 and one word that followed it under the gate.
* **`\vdots`/`\ddots` (#717) and `\twocolumn` (#743/#746).** No fixture in this
  corpus exercises them at a measurable boundary, so this sweep neither
  confirms nor contradicts those fixes.
* **The candidate's own glue set.** The engine does not expose it.
  `crates/render-pipeline/src/pagebuild.rs:1475` (`fn glue_set`) computes exactly
  the right quantity — "positive stretches by `ratio * stretch`, negative shrinks
  by `-ratio * shrink` (capped at the available shrink, TeX §676/§677)" — and the
  value dies as a local at `pagebuild.rs:657` once each line's baseline is
  placed. `BuiltPage` keeps only `lines` and `overfull_by`, and the v2
  display-list `Page` (`display.rs:524`) carries `number`, `width`, `height`,
  `content`, where width and height are the fixed paper size. So a consumer
  cannot re-derive the ratio, and **this document reports the reference's ratio
  and the engine's own overflow record rather than inventing a candidate
  number.** Adding a page-level `glue_set` (and its natural height) to the v2
  page record would make this whole analysis a comparison instead of an
  inference; it is the single highest-value instrument change this sweep found.
* **Whether the candidate shrinks these pages too.** Its one published signal is
  the `overfull_vbox` diagnostic (`typeset.rs:8412`), and **it fired on none of
  the ten pages pdfTeX shrinks** — there is no `underfull_vbox` counterpart and
  no diagnostic at all for a page that merely had to shrink. So the candidate
  column of the glue-set table in `cumulative.md` is empty on every page, which
  is a fact about the instrument, not evidence that our pages fit.
* **How much of a flagged step is real.** A step on a set-glue page is
  `δ - Δr x (shrinkability above it)` and the two terms cannot be separated from
  one page's profile. Where a step's size matches `Δr x (its own boundary's
  shrink)` to a fraction of a percent *and* matches the page's sign, it is
  accounted for; where it does not (F4's +6.732, F5's −1.076), a real `δ` is
  present but its *position* still does not localise it. The only way to recover
  the rest is #755's: truncate the page so it no longer overflows and re-measure.
  That was done for two pages, not for `twelvept-plain` p3 or `ps-calculus` p1.
* **`ps-calculus` page 1.** Its two steps are +0.1344 and −0.1587 — opposite
  signs — so no single `Δr` fits and the page is unresolved. Both stay flagged.
* **Whether the pinned references' own pdfTeX shrinks identically.** Eight of the
  22 fixtures re-run byte-identical; the other fourteen were pinned under TeX
  Live 2025 and re-run here under 2026, differing in bytes but with every
  extracted baseline within **0.001 bp**. A vertical measurement on an identical
  baseline set is the same measurement, but the glue set is read from the re-run,
  not from the pinned file, and that is the assumption.

### One probe-only divergence, recorded here rather than filed

`\vspace` in horizontal mode starts a new paragraph on our side. In pdfLaTeX
`Text.\vspace{6pt}More text` stays one paragraph and `More` continues the same
line; the same input put the following word on its own line 27.5 bp lower here
(probe `q3-cvsec-norule`, which calls the `cv` macro with no blank line before
it). No corpus fixture writes `\vspace` mid-paragraph, so its measured blast
radius today is **zero** — recorded so the next sweep does not rediscover it as
new work.
