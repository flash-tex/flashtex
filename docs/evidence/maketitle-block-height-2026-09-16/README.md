# `\maketitle` title-block height — WITHDRAWN

The 2026-09-16 corpus sweep (`docs/evidence/corpus-fidelity-2026-09-16T1130Z`,
PR #750, corrected by #753) ranked a cumulative finding as "`\maketitle`
title-block height":

| fixture | step | lines carried |
|---|---|---|
| `math-sheet` p1 | **−0.5132 bp** at the first body line | 63 |
| `lecture-notes` p1 | **+0.1755 bp** at `Contents` | 27 |

90 displaced reference lines, one instance over the 0.5 bp glyph gate, and two
signs that disagreed.

**There is no `\maketitle` defect.** The block is exact at 10, 11 and 12 pt,
with and without an author and a date, and the two steps are the page builder's
*glue set ratio*, not the block.

Everything below is measured on this machine, not inherited from the sweep.

---

## 1. The block itself

pdflatex (TeX Live, `/Library/TeX/texbin/pdflatex`) against
`flashtex-render --v2` → `flashtex-pdf-exact from-v2`; glyph origins read with
PyMuPDF and grouped into baselines at 0.05 bp.

Probe: `article`, `\usepackage[T1]{fontenc}`,
`\usepackage[margin=1in]{geometry}`, `\title{Formula Sheet: Calculus and Linear
Algebra}`, then `\maketitle` and three plain paragraphs.

### `\author{}` `\date{}` (`math-sheet`'s shape), title → first body baseline

| base | pdflatex | pipeline | delta |
|---|---|---|---|
| 10 pt | 65.7460 bp | 65.7462 bp | +0.0002 |
| 11 pt | 71.9940 bp | 71.9937 bp | −0.0003 |
| 12 pt | 80.4400 bp | 80.4398 bp | −0.0002 |

### `\author{Course 21-127 --- Concepts of Mathematics}` `\date{Week 2}` (`lecture-notes`' shape)

| base | title → author | author → date | date → first body |
|---|---|---|---|
| 10 pt pdflatex | 28.8880 | 23.3740 | 36.8591 |
| 10 pt pipeline | 28.8880 | 23.3746 | 36.8582 |
| 11 pt pdflatex | 30.2190 | 24.3960 | 41.7750 |
| 11 pt pipeline | 30.2186 | 24.3963 | 41.7752 |
| 12 pt pdflatex | 35.4880 | 27.9590 | 44.9520 |
| 12 pt pipeline | 35.4877 | 27.9590 | 44.9521 |

The first title baseline is 123.8010 / 126.5710 / 132.2680 bp from the page top
and the pipeline puts it at 123.8008 / 126.5711 / 132.2682. Every title, author
and date line's **x** matches to 0.000 bp, so there is no design-size font
substitution in the block — the concern raised by #724 (11 pt native vs
linearly-scaled) and by #754 (math family 0 is `cmr`, not `rm-lmr`, without
`lmodern`) does not reach here.

Two components are checked on top of the ones the corpus exercises: a second
`\and` author (two `tabular`s sharing one line) and the `\lineskip` branch —
at a 10 pt base the date line's height (8.264 pt of `ecrm1200`) plus the author
`tabular`'s 4.2 pt depth exceeds `\baselineskip`, so pdfTeX falls back to
`\lineskip` there and to ordinary `\baselineskip` at 11 pt. The pipeline
switches at the same point.

### The `em` that makes this work

`\@maketitle`'s four skips are `2em`, `1.5em`, `1em`, `1.5em` of `\normalsize`.
With `[T1]{fontenc}` and no `lmodern`, pdfLaTeX sets the document in the EC
family, whose `\fontdimen6` is **not** its design size:

```
ecrm1000  quad  9.99756pt        ecrm1728  quad 16.23491pt   (\LARGE at 10/11 pt)
ecrm1095  quad 10.88788pt        ecrm1440  quad 13.70703pt   (\large at 12 pt)
ecrm1200  quad 11.74713pt        ecrm2074  quad 19.18753pt   (\LARGE at 12 pt)
```

Taking `em` to be the nominal size instead puts the title baseline 0.124 bp out
at an 11 pt base. The pipeline reads `\fontdimen6` off the resolved TFM
(`Context::text_params`), which is why it lands on the measured value.

---

## 2. What actually moved the 90 lines

`\@maketitle`'s *only* stretchable/shrinkable glue is `center`'s closing
`\@endparenv` skip, `\addvspace{\@topsepadd}` = `\topsep` + `\partopsep`
= `12.0 plus 4.0 minus 6.0` at an 11 pt base. `\vskip 2em`, `\vskip 1.5em`,
`\vskip 1em` and every `\baselineskip` are rigid, and `center`'s opening
`\addvspace{\@topsep}` loses to the rigid `\vskip 2em` it is compared against.

Both offending pages are **shrunk** pages. pdfTeX's own `\tracingoutput`:

```
math-sheet    p1  ..\vbox(650.43001+0.0)x469.75502, glue set - 0.74042
lecture-notes p1  ..\vbox(650.43001+0.0)x469.75502, glue set - 0.30122
```

Shrinkable glue above the first body line is `\@topsepadd`'s 6 pt plus
`\section*`'s `\@minus.2ex` (0.94266 pt at 11 pt) = 6.94266 pt, so the
reference's first body line is lifted by

```
math-sheet     0.74042 x 6.94266 = 5.1405 pt = 5.1213 bp
lecture-notes  0.30122 x 6.94266 = 2.0912 pt = 2.0834 bp
```

Measured directly: truncating each fixture so page 1 no longer overflows moves
the reference's first body line down by **5.1210 bp** and **2.0830 bp**
respectively — the predicted amounts — and the reported step collapses:

| fixture | full page 1 | truncated page 1 |
|---|---|---|
| `math-sheet`, `\section*{Series and sums}` | −0.5132 bp | **+0.0037 bp** |
| `lecture-notes`, `Contents` | +0.1755 bp | **+0.0041 bp** |

So each step is the *difference of two shrink ratios*: 0.74042 vs an implied
0.81514 on `math-sheet`, 0.30122 vs an implied 0.27638 on `lecture-notes`. The
ratios differ only because the two sides' page-1 natural heights differ — from
the display-math box heights on `math-sheet` (the sweep's own F2, the live lane
behind PR #754) and from the amsthm closing skips on `lecture-notes` (its F5).
The opposite signs are simply which side's page came out taller.

### The ratio arithmetic is right

Checked, not assumed. A plain-text page whose content is identical on both
sides and which pdfTeX shrinks:

| probe | pdfTeX glue set | worst baseline delta over the whole page |
|---|---|---|
| 10 pt, 16 sections + unbreakable tail | `- 0.92087` | −0.0084 bp (29 baselines) |
| 11 pt, 10 sections + unbreakable tail | `- 0.91649` | −0.0126 bp (27 baselines) |
| 12 pt, 11 sections + unbreakable tail | `- 0.59448` | −0.0136 bp (24 baselines) |
| 11 pt, toc + 4 sections x 2 paragraphs | `- 0.23944` | −0.0155 bp (29 baselines) |
| 11 pt, toc + 5 sections x 3 paragraphs | `- 0.91246` | −0.0125 bp (31 baselines) |

The fourth of these — 11 pt, toc + 4 sections x 2 paragraphs, glue set
`- 0.23944`, 29 baselines — is pinned as
`crates/render-pipeline/tests/maketitle_block_height.rs::a_shrunk_page_keeps_every_baseline`.

---

## 3. Consequences for the sweep

1. **F3 (`\maketitle` title-block height) is withdrawn**, like F7 was in #753.
   Its 90 lines are not separately recoverable: they are already counted by F2
   on `math-sheet` and by F5 on `lecture-notes`. Fixing those fixes these.
2. **On a shrunk page a cumulative step does not localise its cause.** The
   page's glue set ratio is global, so a divergence anywhere on the page moves
   every line *above* it as well as below. `cumulative.py` labels a step with
   the construct standing between the two source lines, which is the right
   label for an unshrunk page and a misleading one here. A future sweep could
   read the ratio off `\tracingoutput` and report it per page, or flag the
   first step on any page whose reference is shrunk.

## 4. Reproducing

```
python3 tools/visual-oracle/rank.py --only math-sheet --only lecture-notes \
  --render <flashtex-render> --pdf-exact <flashtex-pdf-exact> \
  --out <out> --work <work> --keep-work --thumbs 0
```

then read glyph origins out of `fixtures/real-world/<id>/reference.pdf` and
`<work>/<id>/render.exact.pdf`. For the reference's glue set, add
`\tracingoutput=1 \showboxbreadth=8 \showboxdepth=2 \tracingonline=0` after
`\begin{document}` and read the second `glue set` line of the `.log`.
