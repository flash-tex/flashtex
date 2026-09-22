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

## Coverage (slice 1, 2026-09-21, pdfTeX 1.40.29 / TeX Live 2026, engine at this commit)

Counts are formulas; "declared" is what the sources declare and every one of
them is tested. Failing commands are listed by name in `KNOWN`.

| document | formulas | passing | failing | failing commands |
|---|---:|---:|---:|---:|
| kernel-symbols | 1192 | 776 | 416 | 64 |
| kernel-delimiters | 302 | 89 | 213 | 20 |
| kernel-accents | 42 | 36 | 6 | 2 |
| kernel-composites | 148 | 40 | 108 | 20 |
| spacing | 198 | 198 | 0 | 0 |
| alphabets | 25 | 21 | 4 | 4 |
| latexsym-symbols | 84 | 3 | 81 | 12 |
| amsfonts-symbols | 110 | 64 | 46 | 11 |
| amsfonts-delimiters | 44 | 4 | 40 | 5 |
| amsfonts-composites | 16 | 0 | 16 | 4 |
| amssymb-symbols | 828 | 690 | 138 | 35 |
| amssymb-accents | 8 | 8 | 0 | 0 |
| amsmath-symbols | 92 | 4 | 88 | 12 |
| amsmath-delimiters | 24 | 8 | 16 | 4 |
| amsmath-accents | 3 | 0 | 3 | 1 |
| stmaryrd-symbols | 816 | 0 | 816 | 104 |
| stmaryrd-delimiters | 34 | 0 | 34 | 3 |
| stmaryrd-composites | 64 | 0 | 64 | 9 |
| **total** | **4030** | **1941** | **2089** | |

Failure classes (from the `KNOWN` reasons): unsupported commands the engine
drops (184 commands: all of stmaryrd and latexsym's lasy glyphs, amsmath's
`\varGamma` family, and 54 kernel declarations such as `\clubsuit`, `\flat`,
`\imath`, `\lgroup`, `\lmoustache`, `\braceld`; the compiler drift test
`tests/math_symbols_drift.rs` lists the kernel ones), atom class not carried to
the pipeline (`\gg`, `\ll`, `\asymp`, `\nearrow`, `\uparrow`, the harpoons,
`\amalg`, `\dashv`, `\bigtriangledown` are spaced as Ord), OpenType advances
where pdfTeX uses the TFM's (`\Re`, `\Im`, `\aleph`, `\ell` at scriptscript
size, `\big\backslash`, `\mathit` digits), `\mathhexbox` symbols scaled to the
script size (`\yen`, `\checkmark`, `\circledR`, `\maltese`), and composites the
engine draws as one glyph where pdfTeX builds them from pieces (`\cong`,
`\doteq`, `\bowtie`, `\models`, `\hookrightarrow`, `\angle`, `\rightleftharpoons`).
