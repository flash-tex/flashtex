# flashtex-render-pipeline

Original Rust rendering pipeline for FlashTeX and the `flashtex-render` worker
binary. It turns the compiler's parse tree into positioned pages through the
sibling crates (font-engine shaping, paragraph-layout Knuth–Plass line
breaking, math-layout Appendix G boxes, document-style geometry) plus this
crate's own TeX page builder, and emits the rendering-v2 display list with a
runtime-v1 `compile_result` fallback. No TeX engine runs anywhere in the
product path; pdflatex is used only as a test oracle (see
`docs/oracle-evidence.md`).

```sh
cd crates/render-pipeline
cargo build --release
cargo test --release            # 57 tests; the Latin Modern ones skip (loudly) without the fonts
FLASHTEX_COMPILER=$PWD/target/release/flashtex-render   # drop-in worker for the Mac app
```

## Pipeline

```
compiler::parser::parse_project      exact byte spans per document (Span.document)
  -> adapter.rs                      styles/gaps/ligatures/accents re-derived from source bytes,
                                     heading numbers, \ref/\pageref, \newpage, secnumdepth
  -> shape.rs (tfm.rs / font-engine) TFM widths, kerns, ligatures, heights (ec-lm*.tfm, 2^20
                                     units per em) with glyph ids from the OTF cmap; font-engine
                                     GSUB/GPOS for text outside T1; clusters keep source bytes
  -> typeset.rs (paragraph-layout)   total-fit Knuth–Plass, TeX space factor (§1034), \parindent,
                                     \quad, \/ italic correction, glue sized by the font at the space
  -> mathtex.rs / math-layout        TeX's math metrics (lmmi/lmsy/lmex = CM TFMs, rm-lmr*.tfm),
                                     Appendix G, explicit rules; Latin Modern Math glyphs painted
                                     (mathfont.rs: OpenType MATH fallback when no TFMs are installed)
  -> mathtext.rs                     \text{...}: an hbox in the T1 text face (ec-lm* ligature/kern
                                     program, \fontdimen2 glue with the space factor) at the math
                                     style's size, entered through math-layout's Nucleus::Text
  -> pagebuild.rs                    TeX §980–1028 page builder (penalty costs, \topskip, \maxdepth,
                                     per-block \baselineskip, \nointerlineskip, \raggedbottom)
  -> display.rs                      display list v2 (ticks, original GIDs, clusters, carets, rules)
  -> v1.rs                           runtime-v1 fallback with negotiated layout capabilities
```

Distinct identifier types (`ids.rs`): `EncodingCode` (a T1/OT1 slot),
`char`, and `GlyphId` are separate types; TFM codes are never cast to glyph
ids. Every glyph in the display list carries the font's original glyph id.

## Fonts

The default face is Latin Modern (`lmroman<size>-{regular,bold,italic}.otf`
by optical size following `t1lmr.fd`, `latinmodern-math.otf` for math).
Layout uses the TeX font metrics pdfLaTeX uses (`ec-lm*.tfm` for text,
`rm-lmr*.tfm` for the math roman family, the CM-identical `lmmi`/`lmsy`/
`lmex` values embedded in math-layout), found next to the OTFs
(`fonts/opentype/...` → `fonts/tfm/...`) or in `FLASHTEX_TFM_DIRS`; the
OTFs supply the outlines and glyph ids. Without the TFMs the OpenType
metrics are used and a `math_metrics_opentype` / `tfm_missing` diagnostic
says so (`tfm_missing` is a warning per face).
Times is used only when the document selects it (`\usepackage{times}`) and is
metric-only (`core14-afm`, no bytes). Fonts are read at run time from, in
order (`fonts::Discovery`): `FLASHTEX_FONT_DIRS` (colon separated),
`--font-dir`, `FLASHTEX_LM_DIR`, the bundled texmf trees
`<exe>/../Resources/texmf` (an app bundle's `Contents/Resources/texmf`) and
`<exe>/texmf` (`fonts/opentype/public/{lm,lm-math}`), a flat `Fonts`
directory next to the executable or in the bundle's `Resources`, then
MacTeX/BasicTeX 2025/2026 and Debian TeX Live paths
(`fonts::DEFAULT_FONT_DIRS`). TFMs: `FLASHTEX_TFM_DIRS`, the bundled trees'
`fonts/tfm/public/lm`, then each font directory's `/tfm/` sibling and the
directory itself. The pinned LM 2.004 set (`ec-lmr12`, `rm-lmr12/8/6`,
`GUST-FONT-LICENSE.TXT`) loads digest-bound from a texmf root
(`<root>/fonts/tfm/public/lm` + `<root>/doc/fonts/lm/GUST-FONT-LICENSE.TXT`)
or from a flat directory holding the four TFMs and the licence; a missing or
mismatched asset is the blocking `required_metrics_unavailable` diagnostic,
never a silent host-TeX or OpenType fallback. Explicit overrides always
precede the bundle; no environment variable is written.
A missing Latin Modern face is an **error
diagnostic** with Times metrics substituted, never a silent fallback; a
missing math face reports `math_font_unavailable` and typesets no math.

## `flashtex-render` (runtime-v1 worker)

Reads `compile` envelopes on stdin (JSON Lines), answers one `compile_result`
per line. Unknown `protocol_version`/`type`, oversized lines, invalid UTF-8
and unsafe paths get the compiler's error envelopes. Options: `--v2 out.json`
(display list of the last request), `--pdf out.pdf` (through the pdf
sibling's negotiated route), `--font-dir DIR`, `--class-options OPTS`
(body-only input; default `12pt` like the compiler's implicit preamble),
`--secnumdepth N` (default 2; the visual-oracle preamble is 0), `--timing`.

### Layout-capability negotiation (`docs/contracts/runtime-v1-layout-capabilities.md`)

`payload.layout_capabilities` is validated (≤16 unique non-empty strings of
≤64 bytes, else the request fails, never the worker); the accepted subset
(`rules-v1`, `font-hints-v1`, request order, never unrequested) is echoed in
the reply, also on failures. `rules-v1` turns every v2 rule (fraction bars,
radical rules) into `{"kind":"rule",x_pt,y_pt,width_pt,
height_pt,source}` with top-left semantics; without it the legacy U+2500
approximation is emitted (run of box-drawing characters at the size whose
0.0857 em equals the rule height). `font-hints-v1` adds
`font:{family,weight,style}` (`Latin Modern Roman`/`Latin Modern Math`/
`Times`) to text items. `display-list-v2` (mac-preview-v2's proposal,
`docs/contracts/runtime-v1-display-list-v2.md` on its branch, ACKed here)
makes the worker write the rendering-v2 `display_list` envelope as one
sibling line right after the `compile_result` (same `id`, project,
revision, document digests) for `ok`/`recovered` results; a line over the
16 MiB reply limit declines the capability for that request with a
`display-list-v2 declined:` warning. The v1 payload is derived from the
immutable v2 display list per request, so the same source with the same
accepted set is byte-identical whether or not a previous request warmed the
worker. Replies larger than 16 MiB (the Mac reader's line limit) fail the
request explicitly rather than being cut off (`FLASHTEX_MAX_REPLY_BYTES`
lowers the limit for tests).

## Runtime-v1 items

One text item per glyph run (a styled word segment) with the exact UTF-8
source range including the document path (multi-file `\input` projects keep
each item's own file); one item per glyph for math, since each has its own
position. Coordinates are PDF points, y downward, rounded to 1/1000 pt.

## What is implemented, honestly

Paragraphs (justified, `\parindent`, `\\`, `~`, TeX ligatures `--`/`---`/
quotes, `\'e`-style accents composed to precomposed characters), `\textbf`,
`\emph`, `\textit`, `\section`/`\subsection` with LaTeX numbering
(`secnumdepth`), `\label`/`\ref`/`\pageref` (bounded 3-pass convergence),
inline and display math (`$`, `\[`, `$$`, `equation` with `(n)` flush right,
`\frac`, `\sqrt`, scripts, operators with display limits), `\newpage`/
`\clearpage`/`\pagebreak`, page breaking with TeX's cost model, the
`article`/`report`/`book` page frame at 10/11/12pt on every class paper
with the complete `geometry` algorithm (see "Page frame" below). Diagnostics carry source
ranges and codes (`font_unavailable`, `missing_glyph`, `overfull_hbox`,
`overfull_vbox`, `unsupported_script`, `math_limitation`, `labels_unstable`).

Floats and images (FT-063; `src/floats.rs`, `src/graphics.rs`,
`src/typeset/floatpage.rs`, `tests/floats_oracle.rs`,
`docs/evidence/floats/`): `figure`/`table` environments are found in the
source and blanked (same byte length) before the compiler parse, then set as
float boxes (`\includegraphics` lines, `\@makecaption` with `Figure~N:`/
`Table~N:`, `\label`/`\ref`) and placed with LaTeX's `\@addtocurcol`/
`\@addtonextcol`/`\@tryfcolumn` rules (`[htbp!]`, top/bottom/here, float
pages, `\end{document}` flush). `\includegraphics` sizes PNG/JPEG/PDF from
their headers like pdfTeX and applies graphicx's `width`/`height`/
`totalheight`/`scale`/`angle`/`keepaspectratio`/`page`. Image files are read
from `RenderOptions::project_root` (request `project_root`, or
`--project-root DIR`). Image items reach the `display_list` line only when
`display-list-v2-images` is negotiated
(`protocol/proposals/display-list-v2-image.md`); runtime-v1 has none.
10 fixtures match pdfLaTeX within 0.05 bp.

`\text{...}` in math (`src/mathtext.rs`, `tests/math_text.rs`,
`docs/evidence/hw1-text/`): the argument is an `\hbox` in the text face at the
math style's size (12/8/6 pt), shaped like a paragraph word — T1 `ec-lm*`
ligatures (`ffi` is one glyph), kerns, braces at their T1 slots, interword
glue `\fontdimen2`(+7) with TeX's space factor at natural width — and laid
out as an Ord atom. The compiler side (`Nucleus::Text`) is an isolated
candidate (`crates/preview-controller/docs/handoffs/hw1-text-candidate/`), so
the conversion arm is behind the `compiler-text-nucleus` feature until the
compiler adopts it and `vendor/compiler` is re-pinned; without it `\text` is
still the compiler's "not supported in math mode" error.

Math symbols (`src/mathtex.rs`, `tests/math_symbols.rs`,
`fixtures/math-symbols/`, `docs/evidence/math-symbols/`): every control word
the compiler pin `49e6eb43` recognises (Greek, `\times`…`\cap`, arrows,
quantifiers, `\sum`/`\int`/`\prod` with display limits) is placed with its
plain.tex family/slot metrics and painted from Latin Modern Math (the
optical `lmroman` faces for the roman family); `\neq`/`\notin` are TeX's
`\not`+relation composites (zero-width cmsy `"36`), `\perp` is cmsy `"3F`,
`\left`/`\right` fences are re-derived from the source bytes before each
delimiter (the compiler flattens them) and sized through the `lmex` chain.
Against pdflatex+lmodern the trailing word after each fixture formula lands
within 0.01 bp; `\angle` is LaTeX's constructed `\not`+rule macro, not a
glyph, and is the one typed `math_limitation` left.

TikZ pictures (`src/tikz.rs`, FT-062): a `tikzpicture` is found from the
source bytes (the compiler reports it as an unknown environment; those
diagnostics and the body text are dropped), compiled by
`flashtex_vector_graphics::tikz` at the body size with node text shaped in
Latin Modern with TFM metrics, and set as one box on its own line (bottom
edge on the baseline; centred inside `center`). The display list v2 gets
`path_fill`/`path_stroke` items (with `clips`) and glyph runs for node text
— proposal `path-v0`, `docs/proposals/display-list-paths.md`, not in the
frozen schema; runtime-v1 and `--pdf` omit the paths (warning
`tikz_display_list_only`). `flashtex-tikz-pdf in.tex out.pdf` writes a
standalone picture PDF (paths through vector-graphics' content-stream
serializer, whole Latin Modern OTFs embedded as CID fonts); against
pdflatex+TikZ at 150 dpi all 30 fixtures in `fixtures/tikz/` pass the
documented tolerance (`tools/tikz-oracle/compare.py`,
`docs/evidence/tikz/README.md`). Test: `tests/tikz_pipeline.rs`.

Not implemented (reported, not approximated silently): hyphenation, lists
(`\item` markers are set as plain paragraphs, no hanging indent), figures
(`\includegraphics` is dropped by the compiler; captions are plain
paragraphs), `\angle`, `\bigl`/`\bigr`, `\mathbb`, `\mid`, `\setminus`,
`\quad`/`\qquad` and `array` in math (the compiler's math parser rejects
them; see `coordination/mac-math-symbols.md`), tables, footnotes,
non-Latin scripts (`unsupported_script`), RTL.

## Page frame (`flashtex-class-geometry`)

`adapter::document_setup` reads the preamble with
`DocumentSetup::from_preamble` (`\documentclass` options, every
`\usepackage[..]{geometry}` option, `\geometry{..}` calls, `\pagestyle`) and
`Stylesheet::from_resolved` takes the MediaBox, text block, `\textheight`,
`\topskip`, `\maxdepth` and `\parindent` from the resolved frame (exact to
the sp against pdflatex in that crate's 96 fixtures). Body-only input keeps
the compiler's implicit preamble: article, `--class-options` (default
`12pt`), `\usepackage[margin=1in]{geometry}`. A non-standard class
(`amsart`, ...) is laid out as article with its options and geometry.
Heading skips, display skips, `\parskip` and list glue still come from
document-style (class-geometry CONTRACT step 6). Frame lengths enter the f64
layout as the nearest 0.001 pt decimal when that is within 2 sp of the exact
value (`1in` = 4736286 sp as 72.27 pt), so pre-adoption display lists stay
byte-identical (HW1/HW2 verified) and every length stays within 2 sp.

**`a4paper` without `geometry` is a US Letter page.** pdfTeX only changes
`\pdfpagewidth`/`\pdfpageheight` when a package (geometry's pdftex driver)
sets them; the class option alone changes `\paperwidth`/`\textwidth`/
`\textheight`, not the MediaBox, which stays MacTeX's `pdftexconfig.tex`
default of 8.5in x 11in. The pipeline reproduces that: the page is 612 x
792 bp and the A4 text block (345 pt x 598 pt at 10pt) is placed from the
top-left corner (`tests/class_geometry_frame.rs`, pdflatex `\pdfsavepos`
readings). With geometry the page is the paper (`595.276 x 841.89` bp).

### Two-sided pages, two columns, headers and footers (CONTRACT steps 4–5)

- **Left edge per page.** Blocks are assembled at `\oddsidemargin`; every
  placed line carries an x offset (`Laid::line_dx`: `frame.text_left(page)`
  minus that edge, plus the column offset), applied in `assemble`.
- **Two columns.** The page builder fills columns of `\textheight`; pairs
  become one page, the second column `\columnwidth + \columnsep` to the right
  (LaTeX's order, no balancing). `\columnseprule` (class default or a
  preamble `\setlength`) is a rule centred in `\columnsep`, as tall as the
  column boxes, on every page. Two-column documents get `\parindent 1em` and
  `\sloppy` (`\tolerance 9999`, `\emergencystretch 3em`).
- **`\flushbottom`.** The standard classes keep the kernel's `\flushbottom`
  for two-sided or two-column documents: a page ended at an ordinary break
  is `\vbox to\textheight` with its glue stretched/shrunk (`pagebuild`);
  `\newpage`/`\clearpage` pages and the last page stay natural.
- **Page styles.** `\@oddhead`/`\@evenhead`/`\@oddfoot`/`\@evenfoot` from
  class-geometry's `StyleMacros` (class default, preamble `\pagestyle`),
  changed by body `\pagestyle` (in force when the page ships) and
  `\thispagestyle` (that page only). Each line is `\hb@xt@\textwidth{L\hfil
  C\hfil R}` in the `\normalsize` body face at `head_baseline`/
  `foot_baseline`; `\thepage` upright, marks in `\slshape`
  (`lmromanslant*`, `ec-lmro*` metrics).
- **Marks.** `\sectionmark`/`\subsectionmark`/`\chaptermark` per the class's
  `\ps@headings` (number + `\quad`, or `Chapter n.` and `n.` followed by a
  space-factor-3000 space and `\ `; uppercased where the class does);
  `\markboth`/`\markright` in the body (their arguments, which the compiler
  sets as text, are dropped). `\leftmark` is the page's last mark,
  `\rightmark` its first, the previous page's last when it has none.
- **`\chapter` (report/book).** `\clearpage`, `\thispagestyle{plain}`,
  `\vspace*{50pt}` (a zero-height box at `\topskip`), `\huge` bold
  `Chapter n`, 20pt, `\Huge` bold title (`\raggedright`), 40pt, first
  paragraph unindented. Sections number `chapter.section`.
- **`\noindent`** directly before a paragraph's first material removes its
  indent (the compiler treats the command as a no-op).
- Body-only input and non-standard classes keep `\pagestyle{empty}`.

Oracle: `tests/page_frame.rs` over 22 fixtures in `fixtures/page-frame/`
(article/report/book; oneside, twoside, twocolumn with and without rule,
plain/empty/headings/myheadings, `\thispagestyle`, body `\pagestyle`,
geometry, 10/11/12pt, `\noindent`). Expected word origins and baselines come
from pdflatex's content streams (`tools/page-frame-oracle/generate.py`); the
test gates header/footer words, rules and per-page/per-column left edges
within 0.1pt, and every matched line start's x and baseline within 0.1pt
(line starts whose first words differ, i.e. a different line break, are
counted, not failed).

Implemented and gated against pdflatex (`tests/page_frame.rs`, 32
fixtures): `\pagenumbering{arabic|roman|Roman|alph|Alph}` (resets
`\c@page` to 1), `\setcounter{page}{n}` (page parity follows the counter
for margins and heads), `\cleardoublepage`'s empty page (current page
style) before an `openright` chapter on an even page, article
`\maketitle`'s `\thispagestyle{plain}`.

Not implemented: `\@maketitle`'s vertical skips and tabular author block
(body baselines under a title are reported, not gated), report/book
`titlepage` (`\maketitle` on its own empty-style page, `\c@page` reset),
`\frontmatter`/`\mainmatter`, `\cleardoublepage` in two-column documents
and explicit `\cleardoublepage` commands, two-column `\chapter`
(`\@topnewpage`), float and footnote placement, commands and marks inside
`\input` files, macros inside mark/chapter titles (their source text is
used).

## Sibling pins and requested API changes

The crate builds against vendored copies of the sibling crates (see
`vendor/VENDORING.md`; each directory has a `PIN` file). Requested changes,
also listed in `docs/proposals/rendering-abi.md`:

- **compiler**: expose style scopes (`\textbf`/`\emph`), interword gaps,
  class options, `\parindent`, `secnumdepth` and page-break commands in the
  parse tree instead of leaving them to source re-scanning; number only
  `equation` displays (today every closed `\[`/`$$` increments the counter);
  keep `\newpage` as a block boundary instead of an unsupported-command error.
- **rendering-core / schema**: accept `format: "opentype-cff"` (face 0) as a
  font profile — Latin Modern ships as OpenType CFF; every other rule of the
  schema validates on all 18 fixtures (see `docs/oracle-evidence.md`).
- **font-resources**: `inspect_opentype_cff` (OTTO wrapper over the existing
  `cff` module) so rendering-core's validator/outlines can consume Latin
  Modern; this crate's `cff.rs` (Type 2 bounds) then goes away.
- **paragraph-layout**: penalty-based page builder (this crate's
  `pagebuild.rs` would move there), per-block `\baselineskip`.
- **math-layout**: `Nucleus::HBox(MathBox)` — a pre-typeset box as a nucleus
  (TeX §1076, `\hbox` in math is an Ord). `mathtext.rs` currently passes a
  placeholder through `Nucleus::Text` + `text_glyph` and substitutes the hbox
  after layout; the variant removes that indirection.
- **math-layout**: re-pin `vendor/math-layout` to PR #161 (`SourceTag` on
  atoms and placed glyphs/rules), then enable the `math-glyph-spans` feature
  (and promote it to `default`, as `amsmath-inline` was): math clusters and
  rules then carry the source range of the atom that produced them (scripts,
  fraction parts, `\left`/`\right` each to its own command, radical signs,
  accents, `\text` runs, grid fences) instead of the whole formula, and
  `MathRec::span_paints` paints leaves per source range (the hook for xcolor
  ranges from #150/#158). Positions are unchanged; `tests/math_glyph_spans.rs`.
- **compiler**: adopt the `Nucleus::Text` candidate (hw1-text-candidate +
  comment-fix) so `\text{...}` reaches the pipeline; then re-pin
  `vendor/compiler` and drop the `compiler-text-nucleus` feature gate.
- **pdf**: a glyph-run entry point (font id + original GIDs + tick
  positions) so `--pdf` stops re-encoding text by character.
- **font-engine**: none new; the preview JSON export overlaps with `--v2`.

## Incremental reuse

The worker keeps a block cache across requests (`src/incremental.rs`):
each paragraph part, heading and display is keyed by its items with source
offsets relative to the block, the page-builder flags and a stylesheet
fingerprint; a hit clones the typeset block back with offsets relocated
and its diagnostics replayed, so the output is byte-identical to a fresh
compile (`tests/incremental.rs` checks 200 edits on a 27-page document and
30 edits on 107 pages). Blocks spanning two documents are always rebuilt;
the cache is bounded (50 000 blocks).

## Latency

Warm worker, Apple M1 Max, release build, per request including JSON in and
out (see `docs/oracle-evidence.md`): 0.2 ms (one line), 7 ms (three dense
pages, 1 860 items), about 40 ms for a 27-page document after a one-word
edit (45 ms of worker CPU including the 4.3 MB reply). The first request
adds 5–8 ms of font loading. This is the compile side of the Commander's
typing-to-visible gate (< 200 ms including layout); the Mac paint side is
measured separately by the Mac shell.
