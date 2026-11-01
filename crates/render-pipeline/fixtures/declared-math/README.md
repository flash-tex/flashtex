# Declared math: the exhaustive symbol oracle

Owner: lane SYMBOL-TABLES (kabir-claude, mac-m5pro-kabir). Created 2026-09-21.
**pdflatex is an oracle only, never in the product path; cargo never runs TeX.**

Every math symbol LaTeX declares, through the pipeline, against pdfTeX's glyph
origins. The documents are generated from the same declarations the compiler's
`math_symbols` table is generated from (`crates/compiler/scripts/gen_math_symbols.py`
reads `fontmath.ltx`, `latexsym.sty`, `amsfonts.sty`/`amssymb.sty`, `stmaryrd.sty`,
`mathrsfs.sty`, `amsmath.sty` via `kpsewhich`), so the coverage is complete by
construction rather than by memory:

| document | what it holds |
|---|---|
| `<provider>-symbols` | every `\DeclareMathSymbol` in text style, display style, script and scriptscript size (`$a\sym b$`, `$\displaystyle a\sym b$`, `$x^{a\sym b}$`, `$x^{y^{a\sym b}}$`), plus the file's `\let` aliases |
| `<provider>-delimiters` | every `\DeclareMathDelimiter` at natural size, `\big`/`\Big`/`\bigg`/`\Bigg`, and a tall `\left…\right.` around a double fraction |
| `<provider>-accents` | every `\DeclareMathAccent` over `a` (text and display) and over `abc`; the radical; amsfonts' extra-wide `\widehat`/`\widetilde` |
| `<provider>-composites` | every zero-argument macro the file `\def`s (`\cong`, `\bowtie`, `\models`, `\hookrightarrow`, the long arrows, `\mapsto`, `\notin`, `\hbar`, `\surd`, `\angle`, `\dots`, stmaryrd's `\Mapsto` family, …) |
| `spacing` | the 8x8 atom-class matrix (Ord `a`, Op `\sum`, Bin `+`, Rel `=`, Open `(`, Close `)`, Punct `,`, Inner `\left(b\right)`) as `$x L R y$` in display, text and script style, plus a Bin at the start and at the end of a list |
| `alphabets` | `\mathrm` `\mathnormal` `\mathit` `\mathbf` `\mathsf` `\mathtt` over A–Z, a–z, 0–9; `\mathcal`, `\mathbb`, `\mathscr` over A–Z; `\mathfrak` over all three |

## Files

* `generate.py` — writes every `<doc>.tex`, runs `pdflatex` on a copy (with
  `\pdfcompresslevel=0` prepended, output encoding only), reads the PDF's content
  stream with `tools/visual-oracle/pdftext.py`, and writes `expected/<doc>.txt`.
  It also checks, for every symbol formula, that pdfTeX set the symbol from the
  font and slot the declaration table records (`slot-ok` in the `formula` line);
  the run that produced the committed data reported **0 mismatches** over every
  document, so the table's (font, slot) pairs are verified against pdfTeX
  independently of the engine.
* `expected/<doc>.txt` — per formula: `formula <label> <page> <category> <name>
  <style> <slot-check> <label x> <label y> <tex>`, then one `g <font> <code> <size>
  <x> <y_top> <texts>` per glyph pdfTeX placed after the label (bp, top-left
  origin, the display-list-v2 convention). `<texts>` lists every character the
  declarations give that font slot, `|`-separated (`-` when none).
* `tests/declared_math_oracle.rs` — renders each `.tex` with the pipeline and
  compares per formula relative to the label's origin: each pdfTeX glyph needs an
  engine glyph of the same size within 0.5 bp in x and y (x only for cmex glyphs,
  whose Type 1 origin sits at the top of the TFM box) painting one of the slot's
  declared characters, and the engine may paint nothing pdfTeX did not. `KNOWN`
  lists the failing commands per document with a reason; an entry that starts
  passing fails the test until it is removed.

Regenerate after a change to the declaration sources or the templates:

```sh
python3 crates/render-pipeline/fixtures/declared-math/generate.py            # all documents
python3 crates/render-pipeline/fixtures/declared-math/generate.py --only spacing
cd crates/render-pipeline && cargo test --release --test declared_math_oracle
DECLARED_MATH_VERBOSE=1 cargo test --release --test declared_math_oracle kernel_symbols
```

## Coverage (pdfTeX 1.40.29 / TeX Live 2026)

Counts are formulas; "declared" is what the sources declare and every one of
them is tested. Failing commands are listed by name in `KNOWN`. Slice 1 is the
engine before the switch-over to the generated table; slice 2 is after it
(kernel commands resolved through `math_symbols`, math-layout's slots and
classes from `cm_slots`, the kernel joins `\bowtie`/`\relbar`/`\Relbar`/
`\joinrel`/`\surd`/`\mathellipsis` and amsfonts' `\Join` as arms).

| document | formulas | slice 1 passing | slice 2 passing | slice 2 failing commands |
|---|---:|---:|---:|---:|
| kernel-symbols | 1016 | 760 | 964 | 14 |
| kernel-delimiters | 198 | 76 | 102 | 20 |
| kernel-accents | 42 | 36 | 39 | 1 |
| kernel-composites | 116 | 32 | 53 | 17 |
| spacing | 198 | 198 | 198 | 0 |
| alphabets | 24 | 21 | 21 | 4 |
| latexsym-symbols | 44 | 3 | 3 | 12 |
| amsfonts-symbols | 92 | 64 | 64 | 11 |
| amsfonts-delimiters | 24 | 4 | 4 | 5 |
| amsfonts-composites | 12 | 0 | 4 | 2 |
| amssymb-symbols | 828 | 690 | 690 | 35 |
| amssymb-accents | 8 | 8 | 8 | 0 |
| amsmath-symbols | 48 | 4 | 4 | 12 |
| amsmath-delimiters | 24 | 2 | 8 | 4 |
| amsmath-accents | 3 | 0 | 3 | 0 |
| stmaryrd-symbols | 412 | 0 | 0 | 104 |
| stmaryrd-delimiters | 12 | 0 | 0 | 3 |
| stmaryrd-composites | 32 | 0 | 0 | 9 |
| **total** | **3133** | **1898** | **2165** | |

(The slice-1 column counts formulas the way slice 2 does; the slice-1 commit
message's "1941 of 4030" counted every diagnostic and alias line as well.)

Failure classes after slice 2 (from the `KNOWN` reasons): unsupported
commands the engine drops (136 commands: all of stmaryrd, latexsym's lasy
glyphs, amsmath's `\varGamma` family, and the kernel pieces with no glyph
in a bundled face -- `\lhook`, `\rhook`, `\mapstochar`, `\Arrowvert`, cmex
"7A-"7D `\lmoustache`/`\rmoustache`/`\braceld`..`\braceru`); composites the
engine draws as one glyph where pdfTeX builds them from pieces (`\cong`,
`\doteq`, `\models`, `\hookrightarrow`, `\angle`, `\hbar`, `\mapsto`,
`\notin`, `\ne`, `\rightleftharpoons`, `\surd`, `\mathellipsis`,
`\mathsterling`); the arrow delimiters (`\uparrow` family, `\vert`,
`\Vert`, `\arrowvert`, `\bracevert`, `\lgroup`/`\rgroup`, `\backslash`,
`<`/`>`) under `\big`..`\Bigg` and `\left`/`\right`; OpenType advances
where pdfTeX uses the TFM's (`\Re`, `\Im`, `\smallint`, `\not`, `\vec`,
`\mathbb`, `\mathit` digits, `\mathscr`); `\mathhexbox` symbols scaled to
the script size (`\yen`, `\checkmark`, `\circledR`, `\maltese`); and
`\phi`/`\varphi`, whose slots the engine paints with each other's character.
