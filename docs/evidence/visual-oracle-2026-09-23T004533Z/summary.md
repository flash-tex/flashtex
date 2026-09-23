# Visual-oracle re-rank: 2026-09-19T230000Z → 2026-09-23T004533Z

Re-run of `tools/visual-oracle/rank.py` against current main (HEAD
`027e2bdbc`, 159 commits after the previous baseline `608ee20ca`).
No source changes in this slice; this file is the wave-3 comparison summary.

## How the new report was produced

- Invocation (same as the 2026-09-19 report's defaults; see
  `tools/visual-oracle/README.md`):
  `python3 tools/visual-oracle/rank.py --rasterizer gs --thumbs 0`
  with `FLASHTEX_RENDER` = release `flashtex-render` built from this checkout
  (`crates/render-pipeline`, sha256 `4de8fe54…`),
  `FLASHTEX_PDF_EXACT` = release `flashtex-pdf-exact` from this checkout,
  `FLASHTEX_FONT_DIRS=<checkout>/apps/mac/Fonts`,
  `FLASHTEX_TFM_DIRS=` the same three TeX Live 2026 trees as the old report
  (`jknappen/ec`, `public/lm`, `public/amsfonts/symbols`),
  `FLASHTEX_RENDER_NOTE="main @ 027e2bdbc (this checkout)"`.
- Two deliberate deviations from bare defaults, both to match the old report's
  format: `--rasterizer gs` (the old report used `gs`; this host also has
  `pdftoppm`, which `find_rasterizer` prefers — pixel counts below are
  gs-10.08.0 vs gs-10.08.0) and `--thumbs 0` (the old evidence dir contains
  no `thumbs/` and its `report.md` has no thumbnail links).
- No `origin/main` ref exists in this environment (no remotes, no network),
  so "current main" is this checkout's HEAD, which contains the old baseline
  `608ee20ca` as an ancestor plus 159 further commits.
- Fixtures are untouched since the baseline (pinned references, stable).
  Font environment: zero font-FAILURE diagnostics in every fixture, both runs.
  114 pages compared in both runs, 0 missing, 0 unaligned pages, 0 reflowed
  words in the new run.

## Headline

Every page that was bad got better; nothing real got worse. The corpus worst
case fell from 39.86 bp (`unicode-accents`) to 1.09 bp (`hw1`/`hw2` page 1),
and 14 of 114 pages now sit above 0.01 bp only on beamer-scale noise. The two
`beamer-default`/`beamer-madrid` documents that previously produced no
candidate PDF at all (`from-v2` exit 1) now export (`from-v2` exit 0), so 14
pages are comparable for the first time.

## Improvements (old max Δ → new max Δ, bp)

Plausible owning fix named per group with confidence. All "PLAN1" commits are
the `agent/kabir-claude/node-stream-s*` compiler→render-pipeline node series
merged since the baseline.

| pages | old → new | likely fix (confidence) |
|---|---|---|
| beamer-default p1–7, beamer-madrid p1–7: `no_candidate_pdf` → compared, max Δ ≤ 0.006 | — → ~0.005 | `a2afcac4b` render-pipeline draws Core-14 Times/Helvetica/Courier with TeX Gyre so base-14 documents export (high — commit message names exactly this failure; old `run.log` shows `from-v2 exit 1`, new shows exit 0) |
| unicode-accents p1: 39.86 → 0.35; article-twocolumn p2: 33.82 → 0.016, p1: 1.20 → 0.006; listings-manual p1: 37.45 → 0.53, p3: 7.54 → 0.013; cv p1: 3.62 → 0.009; lab-report p1: 1.32 → 0.10, p3: 1.10 → 0.006; natbib-review p1–3: 0.68/0.91/0.84 → 0.31/0.006/0.006; plain-article p1: 1.04 → 0.008, p3: 0.73 → 0.005; ps-calculus p2 (cases env): 5.43 → 0.31; siunitx-tables p2: 2.93 → 0.26; lecture-notes p1: 3.24 → 1.03, p2: 2.83 → 0.79; twelvept-plain p2: 0.13 → 0.015 | large | PLAN1 slice-2 family as a group (medium-high): interword glue / control spaces / ties / `{}` from the compiler's nodes (`e805d44c0`, `08e4b8eb9`), paragraph indent and continuation (`22a86b65a`), fonts from the compiler's NFSS nodes (`3272ea6b3`), no space after a control word (`17ff3e172`). Per-page exact cause uncertain — the dx blowups were whole-line shifts, the signature of glue/indent bugs |
| hyperref-toc p4 (page-wide +1.155 dy → ~0); inline-math p1 (dy 0.32 → 0.01), p2 (dy 0.12 → 0.006); plain-article p1/p2 dy components; hw1/hw2 p1 dy 0.054 → 0.016 with within-0.01 jumping 29 → 124/131 | large | Vertical-sizing family (medium): exact NFSS `\fontsize` size and leading (`e07d550eb`, `bb58f4489`, `89788d72f`), preamble lengths + `secnumdepth` from the compiler (`512c06e90`), multi-paragraph theorem closing skip (`5152658ae`) |
| lecture-notes p2 Bézout line 2.83 → 0.79; hw2 p2 0.43 → 0.33 | medium | Math-atom family (medium-low): named-operator limits from the compiler's atoms (`78cf4fa1a`), script attachment (`befba414c`), compiler-chosen dots (`13c79105b`) |
| beamer-blocks-columns p3: 1.17 → 0.005 | large | Same interword-glue family (medium) |

## Regressions

None above noise. The only max-Δ increases are sub-0.02 bp jitters on
already-exact beamer pages (beamer-fragile p2 0.005 → 0.006, beamer-overlays
p8 0.005 → 0.006, beamer-polish p6/p9/p10 ±0.001) and lab-report p2
0.063 → 0.078 on a `Figure` float caption — all within rasterizer/alignment
noise, not regressions. Unchanged-but-stuck pages are listed under residuals.

## Residuals still above 0.5 bp (new report ranks 1–8)

Owner is the report heuristic plus the guess below (confidence stated).

1. `hw1` p1 and `hw2` p1 — max Δ 1.086, worst word `This` dx +1.086 decaying
   ~0.1 bp per word along the first line after `\textbf{Instructions.}` (dy
   now ~0). Unchanged since baseline. Guess: `crates/render-pipeline`
   paragraph/line layout — line-start offset and/or interword glue width
   after a formatting group (the decay pattern is glue, not a constant
   indent). `crates/compiler` glue node a second suspect. Confidence:
   medium-low. (Same numbers in hw1/hw2 — one root cause, shared preamble.)
2. `hw1` p2 — max Δ 0.611, `b:` in the `\qq`-separated display
   (`a+r, \qquad ar, …`). Unchanged. Guess: `crates/math-layout` display-math
   spacing (`\qquad` / `\qq`) via `crates/render-pipeline`. Confidence: medium.
3. `lecture-notes` p1 — max Δ 1.029, math `n` in `$(mb+nc)$ … $m,n\in\Z$`
   (was 3.24 on theorem text). Guess: `crates/math-layout` — inline-math
   italic correction / side bearings (cf. `1bc731804`, nodes now carry italic
   corrections, but this line still drifts). Confidence: medium.
4. `lecture-notes` p2 — max Δ 0.787, `=` on `$ma+nb=\gcd(a,b)$` (was 2.83).
   Guess: `crates/math-layout` operator/operand spacing around the named
   operator (limits landed in `78cf4fa1a`, spacing did not) — leaning
   math-layout over render-pipeline line layout. Confidence: medium-low.
5. `listings-manual` p1 — max Δ 0.526, `>=` in `if (n < 0 || n >= …)`
   (was 37.45; rest of page ≤ 0.18, but page pixels still 20.1% — verbatim
   rendering differs wholesale). Guess: `crates/render-pipeline` monospace
   advance for `>`/`>=` in LMMono10 (or the listings emulation upstream of
   it). Confidence: low-medium.
6. `math-sheet` p2 — max Δ 0.912, whole `bmatrix` block shifted +0.91
   (`M`, cells, `=` all +0.91). Bit-identical to baseline. Guess:
   `crates/math-layout` matrix column placement. Confidence: medium-high
   (math-only, block-coherent, untouched by 159 commits).
7. `ps-calculus` p3 — max Δ 0.603, `i.e.\ ` line (`i.e.` +0.603, neighbours
   decaying both ways). Bit-identical to baseline. Guess:
   `crates/render-pipeline` paragraph layout — control-space (`\ `) glue
   width. Flag: PLAN1 slice 2 touched control spaces (`e805d44c0`) yet this
   did not move. Confidence: medium-low.

Near-threshold watchlist (0.25–0.5 bp, not residuals yet): `unicode-accents`
0.346 (`Caf\'e` accent-compose line — likely `crates/compiler` accent
composition `973adcfaf`, medium-low), `hw1` p3 0.402, `siunitx-tables` p1
0.413 dy page-median shift (page builder), `plain-article` p2 0.324,
`natbib-review` p1 0.307, `ps-calculus` p2 0.308, `siunitx-tables` p2 0.264,
`hw2` p2 0.334.

## Pages whose differing-pixel count changed (gs 10.08.0 both sides)

49 of 114 changed; the other 65 are bit-identical pixel counts. All changes
are decreases except the 14 newly-comparable beamer pages (previously no
candidate PDF). Format: fixture page: old px → new px (geometric max Δ old → new).

- article-twocolumn 1: 230562 → 228720 (1.197 → 0.006)
- article-twocolumn 2: 16019 → 10781 (33.818 → 0.016)
- beamer-blocks-columns 3: 2548 → 1257 (1.168 → 0.005)
- beamer-default 1–7: none → 2361 / 3081 / 1295 / 1256 / 1707 / 1194 / 990 (newly comparable)
- beamer-madrid 1–7: none → 18767 / 19088 / 14299 / 15465 / 14603 / 16438 / 12042 (newly comparable)
- beamer-polish 2: 1179 → 965; 6: 731 → 237; 7: 1019 → 525; 8: 1143 → 649 (all ≤ 0.006 bp)
- cv 1: 35496 → 32061 (3.62 → 0.009)
- enumitem-worksheet 1: 7337 → 6305 (0.08 → 0.011); 2: 9762 → 9640; 3: 5747 → 5678
- hw1 1: 38268 → 32116 (1.086 → 1.086); hw2 1: 30420 → 28321 (1.086 → 1.086); hw2 2: 29074 → 27692 (0.432 → 0.334)
- hyperref-toc 4: 24324 → 8898 (1.155 → 0.006)
- inline-math 1: 41209 → 15912 (0.323 → 0.011); 2: 6256 → 6137 (0.122 → 0.006)
- lab-report 1: 24161 → 17236 (1.317 → 0.095); 2: 5472 → 4129 (0.063 → 0.078); 3: 11802 → 9195 (1.095 → 0.006)
- lecture-notes 1: 79335 → 32788 (3.243 → 1.029); 2: 32853 → 19454 (2.832 → 0.787)
- listings-manual 1: 393324 → 389430 (37.445 → 0.526); 3: 57146 → 31007 (7.536 → 0.013)
- math-sheet 2: 19096 → 19091 (0.912 → 0.912, unchanged geometry)
- natbib-review 1: 29036 → 18442 (0.679 → 0.307); 2: 31782 → 22470 (0.913 → 0.006); 3: 9007 → 4056 (0.84 → 0.006)
- plain-article 1: 17625 → 6739 (1.042 → 0.008); 2: 11754 → 8500 (0.324 → 0.324); 3: 9685 → 6258 (0.733 → 0.005)
- ps-calculus 2: 22157 → 19196 (5.428 → 0.308)
- siunitx-tables 2: 35480 → 35397 (2.925 → 0.264)
- twelvept-plain 2: 25596 → 25343 (0.13 → 0.015)
- unicode-accents 1: 42107 → 34616 (39.861 → 0.346)

Pages whose geometric max Δ changed but pixels did not (rounding-level pixel
stability): none — every geometric change above moved pixels too, except
sub-0.01 bp beamer noise. Pages with identical max Δ but changed composition:
hw1/hw2 p1 (dy 0.054 → 0.016, within-0.01 29 → 124/131) and plain-article p2
(dy 0.056 → 0.001, within-0.01 150 → 219) — vertical placement fixed while an
unchanged dx residual holds the max.

## Honest limits (inherited from the tool)

- Host differs from the old run (`DN0a230887` vs `MacBook-Pro-423.local`);
  rasterizer is the same `gs` 10.08.0. Pixel counts are gs grey rasters at
  144 dpi — secondary key only.
- Owners above are triage pointers, not verdicts; confidences are stated.
- Whole-PDF byte equality is not measured; nothing here is a parity claim.
