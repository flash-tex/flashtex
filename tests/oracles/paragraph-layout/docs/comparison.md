# pdflatex oracle comparison: `wrap-sample.tex`, variant C

The comparison is executable: `cargo test --test oracle_wrap_sample -- --nocapture`
prints the per-word table below and asserts the headline numbers.

## Reference (oracle) declaration

| Item | Value |
|---|---|
| Sample | `tools/native-validation/oracle-samples/wrap-sample.tex` on `origin/agent/mac-validation/native-verification` (`d855ff8894099e92936b5f0b896938073db69993` at the time of writing; the sample is unchanged since the report) |
| Oracle data | `tools/native-validation/reports/oracle-20260912T050958Z.json`, result `wrap-sample` / `C-times12-1in-ragged`, `comparisons[de1020c].deltas[*]` fields `word`, `oracle_x`, `oracle_bottom`, `oracle_line_start` (copied into `tests/oracle_wrap_sample.rs`) |
| Engine | `/Library/TeX/texbin/pdflatex` = pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026, BasicTeX); pdflatex is an oracle only and was **not** run by this crate |
| Preamble | `\documentclass[12pt]{article}`, `\usepackage[T1]{fontenc}`, `\usepackage{times}`, `\usepackage[margin=1in]{geometry}`, `\setlength{\parindent}{0pt}`, `\setcounter{secnumdepth}{0}`, `\pagestyle{empty}`, `\raggedright`, `\hyphenpenalty=10000`, `\exhyphenpenalty=10000` |
| Fonts | psnfss `ptmr8t` / `ptmb8t` / `ptmri8t` (fontinst TFMs from the Adobe Times AFMs; URW Nimbus Roman outlines). TFM `\fontdimen2..4,7` = 0.25 / 0.15 / 0.06 / 0.06 em (read from the TeX Live TFM files with a 40-line TFM parser; no TeX program was run). |
| Word boxes | PDFKit via `oracle_extract.swift`: `x` = glyph-box left, `bottom` = glyph-box bottom (baseline + font descent), from the page's top-left, in PDF points (bp). No rasterisation, so **no DPI** is involved; the comparison is on PDF coordinates. |

## Coordinate conversion

pdflatex computes in TeX points (1 pt = 1/72.27 in) and writes the PDF in big
points (1 bp = 1/72 in). Every oracle coordinate therefore is `tex_pt × 72/72.27`
(`BP_PER_TEX_PT = 0.996264…`) plus the 72 bp left margin. The crate runs the
layout in TeX points with the oracle's `\hsize` = 469.75499 pt (6.5 in), 12 pt
body, 17.28 pt `\Large` bold heading, `\topskip` 12 pt, `\baselineskip` 14.5 pt,
`\parskip` 0 pt plus 1 pt, `\section` skips 3.5 ex / 2.3 ex of the 12 pt body
(18.9 / 12.42 pt), and converts with `tex_pt_to_bp` before comparing. That is
why the oracle's line pitch reads 14.446 bp: it is 14.5 TeX pt (issue #10's
"14.45 vs 14.4" is exactly this unit difference, not a spacing bug).

## What the crate reproduces

Layout: `Algorithm::TotalFit`, `BreakMode::RaggedRight`, `NoHyphenation`,
Core 14 Times adapter with the AFM kern pairs and `fi`/`fl` ligatures, TeX
`\spacefactor` spacing.

Headline (ours − oracle, 102 words):

```
line starts matched: 102/102; mean|dx| 0.001 bp; max|dx| 0.002 bp; mean|dy| 0.016 bp; max|dy| 0.132 bp
```

* **Line starts**: all 8 oracle line starts are reproduced (heading; "A"; "emphasised";
  "This"; "have"; "how"; "longer"; "Short") and no extra breaks are made.
  Paragraph line counts 1 / 2 / 4 / 1.
* **Horizontal**: every word is within 0.002 bp; the residual is the PDF's
  3-decimal coordinate rounding. The tight case from issue #10 — "and" at the end
  of "have … starts and" — lands at 522.662 bp versus the oracle's 522.663 with a
  right edge of 539.99 bp (0.01 bp inside the 540 bp measure). Without the AFM
  kerns the same line measures 2.0 pt wider and "and" wraps, which is the de1020c
  compiler's 450 pt outlier.
* **Vertical**: body baselines are within 0.013 bp (`\topskip`, `\section`
  afterskip, TeX interline glue across the heading and paragraph boundaries, and
  `\parskip` 0 all modelled). The heading's −0.069 bp and the bold "bold phrase"
  −0.132 bp are not baseline differences: the oracle's `bottom` is a glyph-box
  bottom using PDFKit's descent for the embedded URW *bold* font (≈0.209 em vs the
  Adobe AFM's 0.205 em); the roman rows show the two agree once the descent is
  the same (0.217 em).

Per-word extract (full table from the test; x/bottom in bp):

| word | oracle x | ours x | dx | oracle bottom | ours bottom | dy | line start o/u |
|---|---|---|---|---|---|---|---|
| Wrapping | 72.000 | 72.000 | +0.000 | 87.553 | 87.484 | -0.069 | yes/yes |
| and | 151.879 | 151.880 | +0.001 | 87.553 | 87.484 | -0.069 | no/no |
| A | 72.000 | 72.000 | +0.000 | 113.357 | 113.369 | +0.012 | yes/yes |
| naïve | 83.620 | 83.620 | +0.000 | 113.357 | 113.369 | +0.012 | no/no |
| and | 472.810 | 472.809 | -0.001 | 113.357 | 113.369 | +0.012 | no/no |
| emphasised | 72.000 | 72.000 | +0.000 | 127.802 | 127.815 | +0.013 | yes/yes |
| This | 72.000 | 72.000 | +0.000 | 142.248 | 142.261 | +0.013 | yes/yes |
| both | 496.314 | 496.313 | -0.001 | 142.248 | 142.261 | +0.013 | no/no |
| have | 72.000 | 72.000 | +0.000 | 156.694 | 156.706 | +0.012 | yes/yes |
| and | 522.663 | 522.662 | -0.001 | 156.694 | 156.706 | +0.012 | no/no |
| how | 72.000 | 72.000 | +0.000 | 171.140 | 171.152 | +0.012 | yes/yes |
| few | 495.907 | 495.906 | -0.001 | 171.140 | 171.152 | +0.012 | no/no |
| longer | 72.000 | 72.000 | +0.000 | 185.586 | 185.598 | +0.012 | yes/yes |
| here. | 407.953 | 407.952 | -0.001 | 185.586 | 185.598 | +0.012 | no/no |
| Short | 72.000 | 72.000 | +0.000 | 200.032 | 200.044 | +0.012 | yes/yes |
| coffee | 216.443 | 216.442 | -0.001 | 200.032 | 200.044 | +0.012 | no/no |
| — | 326.024 | 326.023 | -0.001 | 200.032 | 200.044 | +0.012 | no/no |
| done. | 340.968 | 340.967 | -0.001 | 200.032 | 200.044 | +0.012 | no/no |

## First-fit versus total-fit on this sample

`first_fit_ragged_matches_total_fit_on_this_sample` shows the greedy breaker
gives identical output here. That is expected, not luck: under `\raggedright`
every line has infinite stretch, so its badness is 0 and its demerits are
`(\linepenalty + 0)^2 = 100`; total demerits are then 100 × lines, total-fit
minimises the line count, and TeX's `<=` tie rule makes the latest feasible
predecessor win, which recursively is the greedy break. The two would differ
where shrinking a line (finite `\fontdimen4`, still active under
`\raggedright`) saves a whole line, or in justified mode (see
`tests/golden.rs::first_fit_and_total_fit_differ_on_a_loose_line`).

## Known differences from pdflatex that this sample does not exercise

* Justified mode with automatic hyphenation: no pattern hyphenator is shipped
  (only `\-`). TeX's second pass would find more break points than we do.
* `ff`, `ffi`, `ffl`: T1 Times has fontinst composites; we typeset `f`+`f`
  with the AFM `f f` kern (−25), which happens to give the same 641-unit width
  for "coffee" here, but `ffi`/`ffl` would differ by 25 units.
* `\addvspace` merging of consecutive vertical skips, `\looseness`, per-character
  heights/depths (we use font ascender/descender for interline glue), and
  `\flushbottom`.
* Glyph *positions inside* a word were compared only through the next word's
  start; per-glyph offsets follow the same advances + kerns.

## Hyphenating sample (FT-019 rev 2): `docs/hyphen-sample.tex`

Executable: `cargo test --test oracle_hyphen_sample -- --nocapture`.

| Item | Value |
|---|---|
| Sample | `docs/hyphen-sample.tex` (two justified paragraphs, 92 words) |
| Oracle | `/Library/TeX/texbin/pdflatex` = pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026), run once on 2026-09-12 for this measurement; no overfull/underfull boxes in its log |
| Settings | default `times` settings: `[12pt]{article}`, T1, `geometry margin=1in`, `\parindent 0pt`, justified, hyphenation on (`\language` english = Knuth's `hyphen.tex`, `\lefthyphenmin 2`, `\righthyphenmin 3`), `\tolerance 200`, `\pretolerance 100`, `\emergencystretch 0pt` |
| Word boxes | PDFKit via the native-verification branch's `oracle_extract.swift`, PDF points, no DPI |
| Ours | `LiangHyphenator::en_us_subset()` (420 patterns from TeX Live's `hyph-en-us.tex` that are also in `hyphen.tex` and match a sample word; 14 shared exceptions), `Algorithm::TotalFit`, `BreakMode::Justified`, `LineBreakParams::article_12pt_letter_1in()` |

Headline:

```
line starts matched: 92/92; hyphenated line ends: oracle 5, ours 5, matching 5; mean|dx| 0.003 bp; max|dx| 0.007 bp; max|dy| 0.013 bp
paragraph: 6 lines, pass 2, total demerits 25584, hyphenated lines 2
paragraph: 5 lines, pass 2, total demerits 28729, hyphenated lines 3
```

* **Hyphenation match rate: 5/5** hyphenated line ends coincide with pdflatex's
  (`doc-umentation`, `implemen-tation`, `incom-prehensible`,
  `counterproduc-tive`, `poly-syllabic`), with no extra hyphenation on our side,
  and all 11 line starts match. Both paragraphs required TeX's second pass
  (hyphenation), which is what this sample was written to exercise.
* **Word-level hyphenation points: 110/110.** `src/liang.rs` also checks every
  5+-letter word of both oracle samples against pdflatex's `\showhyphens`
  output (`matches_tex_showhyphens_for_every_sample_word`), so the pattern
  algorithm, hyphenmins and exception handling are verified independently of
  the breaker.
* **Horizontal**: mean |dx| 0.003 bp, max 0.007 bp on justified lines — the
  interword glue is stretched/shrunk by the same ratios as TeX's. Getting here
  needed one adapter fix: TeX's T1 encoding maps ASCII `'` to `quoteright`
  (width 333 with its AFM kern pairs, e.g. `quoteright s -55`), whereas
  de1020c's width table holds `quotesingle` (180); the crate now follows TeX.

Representative rows (full table from the test; x/bottom in bp):

| word | oracle x | ours x | dx | oracle bottom | ours bottom | dy | line start o/u |
|---|---|---|---|---|---|---|---|
| Reproducibility | 72.000 | 72.000 | +0.000 | 86.537 | 86.549 | +0.012 | yes/yes |
| doc- | 518.754 | 518.756 | +0.002 | 86.537 | 86.549 | +0.012 | no/no |
| umentation | 72.000 | 72.000 | +0.000 | 100.983 | 100.995 | +0.012 | yes/yes |
| implemen- | 488.196 | 488.198 | +0.002 | 129.875 | 129.887 | +0.012 | no/no |
| tation | 72.000 | 72.000 | +0.000 | 144.320 | 144.333 | +0.013 | yes/yes |
| engine’s | 500.152 | 500.153 | +0.001 | 144.320 | 144.333 | +0.013 | no/no |
| incom- | 506.129 | 506.131 | +0.002 | 173.212 | 173.224 | +0.012 | no/no |
| prehensible | 72.000 | 72.000 | +0.000 | 187.658 | 187.670 | +0.012 | yes/yes |
| counterproduc- | 466.964 | 466.966 | +0.002 | 187.658 | 187.670 | +0.012 | no/no |
| tive | 72.000 | 72.000 | +0.000 | 202.104 | 202.116 | +0.012 | yes/yes |
| poly- | 514.761 | 514.763 | +0.002 | 202.104 | 202.116 | +0.012 | no/no |
| syllabic | 72.000 | 72.000 | +0.000 | 216.550 | 216.562 | +0.012 | yes/yes |
| measurement. | 145.058 | 145.058 | +0.000 | 230.995 | 231.008 | +0.013 | no/no |

Scope of the claim: the embedded pattern set is a documented subset, so words
outside the two samples may receive fewer hyphenation points than TeX
(never wrong ones, since every embedded pattern is Knuth's). Extending the
subset is a data change, not an algorithm change.

## Page geometry (FT-019 rev 3): `docs/pages-default.tex`, `docs/pages-geometry1in.tex`

Executable: `cargo test --test oracle_pages -- --nocapture`.

| Item | Value |
|---|---|
| Sample | 16 justified paragraphs (the two hyphen-sample paragraphs, the wrap-sample long paragraph and its short last paragraph, repeated four times), 668 words, no headings |
| Oracle | `/Library/TeX/texbin/pdflatex` = pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026), `times`, T1, `\pagestyle{empty}`, default `\parindent` (1.5em = 17.62482pt) and default hyphenation, compiled once on 2026-09-12 (oracle only) |
| Variant `default` | article-class default margins — exactly what `flashtex-document-style` computes for `[12pt]{article}` on Letter: text area origin (111.27, 126.27) pt, 390 × 548.5 pt (`\textheight` = 37 × 14.5 + `\topskip` 12), i.e. 38 baselines per page. pdflatex's log reports 4 `Overfull \hbox (1.03694pt too wide)` boxes. |
| Variant `geometry1in` | `\usepackage[margin=1in]{geometry}`: origin (72.27, 72.27) pt, 469.755 × 650.43 pt, 45 baselines per page |
| Ours | `style::ArticleLayout::new(ClassOptions { Letter, Pt12 }, geometry)` → `PageParams`/`LineBreakParams` from document-style, `LiangHyphenator::en_us_subset()`, `document::layout_document` |
| Word boxes | PDFKit (`oracle_extract.swift`), PDF points, top-left origin, no DPI |

Headline:

```
default:     pages [(38, 138.27, 674.77), (38, 138.27, 674.77)] pt; line starts 668/668; page assignment 668/668;
             page 2 starts with oracle "Reproducibility" / ours "Reproducibility"; mean|dx| 0.002 bp; max|dx| 0.007 bp;
             max|dy| 0.013 bp; overfull lines 4
geometry1in: pages [(45, 84.27, 722.27), (19, 84.27, 345.27)] pt; line starts 668/668; page assignment 668/668;
             page 2 starts with oracle "how" / ours "how"; mean|dx| 0.002 bp; max|dx| 0.007 bp; max|dy| 0.013 bp;
             overfull lines 0
```

* **Every baseline agrees**: 76 (default) and 64 (1in) baselines, each within 0.013 bp
  of the oracle's glyph-box bottom minus the AFM descent (the residual is the
  0.217 em AFM descender vs PDFKit's font descent, constant across the document).
  First baseline `\topskip` below the text area (138.27 / 84.27 pt), line pitch
  14.5 pt, `\parskip` 0 pt, `\parindent` 17.62 pt on every paragraph's first line.
* **Page break**: the same word starts page 2 in both variants; every word is on
  the same page as in pdflatex (668/668).
* **Overfull boxes reproduced**: on the 390 pt measure TeX cannot break
  "Deliberately unbalanced paragraphs demonstrate emergency stretchability: ex-"
  feasibly and sets it 1.03694 pt too wide (artificial demerits on the final
  pass); the crate sets the identical line, overfull by 1.041 pt (the 0.004 pt
  is AFM-vs-TFM width rounding), and reports it through `Lines.diagnostics`.
* **Word-rule finding**: this document exposed that TeX skips leading
  non-letters and ignores the character nodes after a word's letter run
  (§896–899), so `engine's` hyphenates as `en-gine's` and `(documentation)` is
  hyphenated; `src/liang.rs` now does the same (verified with `\showhyphens`).

## Preview/PDF agreement (FT-019 rev 3)

Executable: `tools/preview_pdf_agreement.sh [work-dir] [geometry1in]` (macOS;
no pdflatex involved). Method:

1. `examples/emit_runtime_v1.rs` lays out the `pages-default` document with the
   document-style geometry and emits a runtime-v1 `compile_result` (protocol 1,
   `kind: text` items only — **one item per word**, `x_pt`/`baseline_y_pt`/
   `font_size_pt` in PDF points, `source {path, start_byte, end_byte}` exact
   spans; a discretionary hyphen is appended to its fragment and the span covers
   the `\-` marker bytes when present). No new item kinds.
2. The same JSON is rendered twice: by `flashtex-pdf` (`crates/pdf` on this
   checkout, identical to `origin/agent/mac-pdf/pdf-output` 52b3711 for the
   writer, `--default-face times --verify`) and by `tools/coretext_render.swift`,
   a 60-line CoreText/CoreGraphics program that links nothing from the Mac app
   and draws each item with `CTLineDraw` at `(x_pt, height − baseline_y_pt)` in
   macOS `Times-Roman`, platform kerning and ligatures disabled — the placement
   contract rendering-v2 prescribes for the preview (supplied origins, no
   re-measured baselines). This is a stand-in for the app's `PDFExport.render`,
   which is not on main; it replicates the placement rule, not the app's code.
3. PDFKit word boxes of both PDFs are compared with each other and with the
   emitted origins (`tools/compare_word_boxes.py`).

Result (668 words, 2 pages):

```
word sequences identical: True
|x flashtex-pdf - x coretext|:            mean 0.0003  max 0.0005 pt
|bottom flashtex-pdf - bottom coretext|:  mean 0.0003  max 0.0006 pt
|top flashtex-pdf - top coretext|:        mean 0.0003  max 0.0007 pt
|right flashtex-pdf - right coretext|:    mean 0.0065  max 0.0277 pt
|x flashtex-pdf - x item|:                mean 0.0003  max 0.0005 pt
|x coretext - x item|:                    mean 0.0000  max 0.0000 pt
bottom - baseline: flashtex-pdf 2.9882..2.9892 pt; coretext 2.9888 pt (= 0.25 em glyph-box descent at 11.955 bp)
```

Every word origin and baseline agrees between the two renderers to < 0.001 pt
and matches the compiler-supplied origin; word right edges agree to < 0.03 pt
(the two Times programs — Adobe base-14 metrics in the PDF vs macOS Times in
CoreText — differ by up to 0.028 pt over a 20-letter word). All below the
0.05 pt acceptance. What this does *not* show: inside a word, both renderers
use their font's unkerned advances while the layout kerned the glyphs; the
next word still starts at the compiler's `x_pt`, so visible drift is bounded
by one word's kern sum (≤ 0.4 pt in Times for the worst pairs). Per-glyph
origins (rendering-v2) remove even that; the crate already exposes them
(`PositionedGlyph.x_offset`).
