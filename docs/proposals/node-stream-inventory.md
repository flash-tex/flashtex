# Node-stream inventory (PLAN1 slice 1)

Slice 1 of item 1 in
[generated-data-and-maintainability.md §3.1](generated-data-and-maintainability.md#31-one-node-stream-contract--rewrite-of-the-compilerpipeline-boundary-p0).
It lists every site in `crates/render-pipeline` that re-derives a layout fact
from source bytes (`source[span]`, gap bytes between spans, whole-source
re-scans, byte masking) instead of reading the compiler's node tree. Each
site has one falsifier in
[`crates/render-pipeline/tests/node_stream_falsifiers.rs`](../../crates/render-pipeline/tests/node_stream_falsifiers.rs).
Together they are the acceptance list for slices 2 onwards. Slice 1
changes no behaviour. Line numbers are from `origin/main` at `1bc69f66d`.

## Method

**Falsifier.** Each test typesets one construct twice: written directly, and
produced by a user macro (`\newcommand`, `\def`, `\let`, `\newenvironment`,
or a project `.sty`). The compiler gives every token of a replacement text
the invocation's span, so the bytes at the span are not the tokens it
expanded. The test compares the two display lists glyph by glyph (font,
size, gid, and origin to 0.01 bp) and fails if they differ.

**What was measured.**

- *pdfLaTeX sets both forms identically.* All 46 pairs were compiled with
  `pdflatex` (pdfTeX 3.141592653-2.6-1.40.29, TeX Live 2026, MacTeX;
  `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, two passes). Their glyph lists
  (font, size, character, x, y to 0.01 bp), read with
  `tools/visual-oracle/pdftext.py`, are identical. So "macro form == direct
  form" is the pdflatex expectation, and the tests need no TeX at run time.
  To reproduce, run the suite with `NODE_STREAM_DUMP=<dir>`, which writes
  each pair's documents out for the oracle.
- *All 46 fail today.* Every test is `#[ignore = "PLAN1 site N: …"]`, and
  each one fails under
  `cargo test --release --test node_stream_falsifiers -- --ignored`. The
  run reports 0 passed, 46 failed.
- *Tree column.* It records whether the compiler's block trees for the two
  forms are equal once spans are dropped.
  - **same** (39 sites): the compiler already expands the macro correctly,
    so the difference on the page comes from the pipeline alone. The test
    asserts this as a precondition.
  - **differs** (7 sites): the node stream itself is wrong or missing for
    the macro form, so fixing the site also needs a compiler change.

**Node field.** Whether the compiler's output already carries the fact.
Values are *yes (field)*, *partial*, or *no*. Each was checked by printing
the direct form's tree, not assumed.

## Inventory

`IFIS` = `adapter::items_from_inlines_styled` (adapter.rs 10950–12090).
Unless a path is given, lines are in `crates/render-pipeline/src/adapter.rs`.

### A. Style scope (6)

| # | site (file:line) | fact re-derived | node field today | tree | falsifier |
|---:|---|---|---|---|---|
| 1 | `source_style_intervals` 8959–9061, via `style_intervals` 8827 / `Styles::at` | bold/italic/family from the source brace groups at the span | **yes**: `Inline::Text.style` (`TextStyle` bold/italic/slanted/small_caps/family/size) is already right for the macro form | same | `site01_declaration_from_def_body` (`\def\B{\bfseries}`) |
| 2 | same scanner: a depth-0 declaration in the preamble | a `\let\B\bfseries` in the preamble is read as `\bfseries` in force to the end of the document | yes (`TextStyle`) | same | `site02_preamble_let_leaks_declaration` |
| 3 | `macro_argument_intervals_defined_in` 8861–8933 | fonts a macro body wraps around `#k` (only one level deep) | yes (`TextStyle`) | same | `site03_nested_argument_wrapper` |
| 4 | IFIS 10994–11003 `foreign`/`body_of` + `Styles::in_body` 10470 | declaration macros defined in a project `.sty` | yes (`TextStyle`) | same | `site04_declaration_macro_from_project_sty` |
| 5 | `glue_size` 12204–12220, `space_size` 12130–12142 | the size of `\quad`/interword glue, from `\Large`… bytes in the gap | partial: `Text.style.size` is carried; `Inline::TextGlue` has `em` but no size/style | same | `site05_quad_size_after_size_macro` |
| 6 | `text_command_argument_at` 8570–8584, IFIS 11785–11789 | `\check@icl` italic correction before an `\eqref`/`\textup` argument | no: no "text-command argument" or italic-correction marker | same | `site06_italic_correction_before_eqref` |

### B. Interword glue and input conventions (9)

| # | site | fact | node field | tree | falsifier |
|---:|---|---|---|---|---|
| 7 | `token_gap` 9409–9550, `gap_has_space` 10573, `ligature_char_sources` 12347–12376 | whether a gap holds a space; which bytes made a character | **yes**: `Text.space_before` (`false` here, correctly); the pipeline sets a space anyway | differs (segmentation only) | `site07_gap_before_macro_dash` |
| 8 | IFIS 11643 | a control space `\ ` (span bytes `== "\\ "`) | partial: a `Text " "` node, not marked as a control space | same | `site08_control_space` |
| 9 | `push_segment_in` 12299–12339 | `{}` breaks a ligature/kern (literal `{}` between spans) | no: a macro boundary also splits the text, so a split is not a boundary | same | `site09_empty_group_breaks_ligature` |
| 10 | IFIS 11960–11972 | `~` is a tie (source byte `~`) | no: the compiler emits the text `Figure~7.` | same | `site10_tie_from_macro` |
| 11 | IFIS 11677–11681 | accent composition (`\'e` as a 2-byte command at the span) | no: the compiler emits `Caf`, `’`, `e` | same | `site11_accent_from_macro` |
| 12 | `citation_label_run` / `generated_citation` 7691–7725 | a run was generated by `\cite` (so blanks and `\emph` flips apply) | no: no origin marker on `Text` | same | `site12_citation_through_macro` |
| 13 | `footnote_command_end` 12095–12123, IFIS 11124–11154 | the gap after `\footnote[n]{..}` | no: `Footnote.span` is the command token only | same | `site13_footnote_in_two_argument_macro` |
| 14 | `is_control_word` 9081–9084 (used at IFIS 11399) | `\hfil` vs `\hfill` | no: `HFill { leader }` has no stretch order | same | `site14_hfil_from_macro` |
| 15 | `url_run_at` 8460–8465 | a run is `\url` (url break penalties) | partial: `family: Mono`, no url flag | differs (the compiler also mangles `\url` expanded from a macro; see [compiler-side sites](#compiler-side-re-scans-found-on-the-way)) | `site15_url_through_macro` |

### C. Paragraph and vertical structure (17)

| # | site | fact | node field | tree | falsifier |
|---:|---|---|---|---|---|
| 16 | `body_commands` 9829–9962 (`\noindent` arm), `noindent_reaches` 3820, 2638–2641, 2936–2939 | `\noindent` suppresses the next paragraph's indent | no: `Block::Paragraph` has no indent flag (the parser tracks `paragraph_started` internally) | same | `site16_noindent_from_macro` |
| 17 | `body_commands` (`\markboth`/`\markright` arms) | running-head marks | no: the mark arguments arrive as body `Text` (`L`, `R`) | same | `site17_markboth_from_macro` |
| 18 | `body_commands` (`\chapter`/`\part`), `strip_command_text` 10052 | chapter/part headings in report/book | no: the compiler emits a bold `Paragraph`, not a heading | same | `site18_chapter_from_macro` |
| 19 | 2367–2373 `plain_text(&t[first.start..last.end])`, `plain_text` 10321 | the heading title for `\markboth`/running heads | partial: `Block::Heading.content` holds the right inlines; the mark text is re-read from bytes | same | `site19_heading_mark_from_title_macro` |
| 20 | `run_in_heading_at` 10225–10293 | `\paragraph`/`\subparagraph` run-in head (bold, `\hskip 1em`) | no: the compiler emits a plain `Text "Head"` | same | `site20_run_in_heading_from_macro` |
| 21 | `list_stack_at` 7472–7509, `SourceIndex::new` 7577–7635, `list_seps_from` 7089–7179, `list_margins` 7925–8015 | open list environments and their options (topsep, itemsep, margins) | **yes**: `ListItem.lists: [ListFrame { environment, kind_depth, options, begin_span }]` | same | `site21_list_opened_by_macro` |
| 22 | 4978–4989 (`after_env`), `gap_has_list_end`/`list_env_ends` 7301–7320, `list_end_adjust` 7235 | `\@endpe`: no indent after `\end{list}`, plus the closing topsep | no | same | `site22_list_closed_by_macro` |
| 23 | 4956–4974 (`env_open`), `gap_has_trivlist_end` 7409, floats.rs `body_blocks` 870–918 | a trivlist env (center/quote) opens here, so topsep/partopsep apply | partial: `Block::Styled` exists; its topsep context is re-read from gap bytes | same | `site23_center_opened_by_macro` |
| 24 | 2769–2781 | a display is numbered (`\begin{equation}` bytes), `\[` vs `displaymath` | partial: `Inline::Math.number`/`number_span` exist; the adapter re-reads the env | same | `site24_equation_opened_by_macro` |
| 25 | `strip_tag` 5337–5385, `rich_tag_of` 5437–5446 | `\tag`/`\tag*` atoms in a formula | partial: `Math.number` | differs (the compiler's own `custom_tag_text` re-scans too) | `site25_tag_from_macro` |
| 26 | `is_qedhere_marker` 5471–5478, IFIS 11085–11113 | the `\qedhere` box position | no | same | `site26_qedhere_from_macro` |
| 27 | 2690–2694 (proof start), amsthm.rs `head_separator` 62–155 | a proof/theorem head and its head separator | no | same | `site27_proof_opened_by_macro` |
| 28 | `theorem_environments` 8144–8162, `opens_theorem_item` 8175, `SourceIndex::in_theorem` 5028 | the set of `\newtheorem` environment names | no | same | `site28_newtheorem_through_macro` |
| 29 | `gap_continues` 5557–5564 | a display continues its paragraph (no blank line or `\par` bytes) | no | same | `site29_par_after_display_from_macro` |
| 30 | `vspace_in_gap` 9681–9696, `split_at_page_breaks` 4663–4674 | a `\vspace{..em}` re-evaluated at the local size | partial: `Block::VSpace` carries points at a fixed em | same | `site30_vspace_em_from_macro` |
| 31 | `setlist_calls` 7432–7454 | enumitem `\setlist` keys (counted even inside a definition that is never called) | partial: `ListFrame.options` carries per-environment keys; no global `\setlist` state | same | `site31_setlist_in_uncalled_definition` |
| 32 | `body_commands` (`\pagestyle`/`\thispagestyle` arms) | the page style per page | yes: `Inline::PageStyle` | differs (the compiler drops `PageStyle` when `\pagestyle` comes from a macro) | `site32_pagestyle_from_macro` |

### D. Preamble facts (8)

| # | site | fact | node field | tree | falsifier |
|---:|---|---|---|---|---|
| 33 | `apply_preamble_lengths` 5776–5855, `setlength`/`setlength_in` 6463–6502 | preamble `\setlength` of page parameters, `\parindent`, `\parskip` | partial: `Parsed.parskip_pt` only | same | `site33_preamble_setlength_from_macro` |
| 34 | `counter` 6413–6429 | `\setcounter{secnumdepth}` (the last match wins, even inside an uncalled definition) | no | same | `site34_setcounter_in_uncalled_definition` |
| 35 | `document_setup` 5704–5726 → `class-geometry` `DocumentSetup::from_preamble` | `\geometry{..}`, page style, class kind | no | same | `site35_geometry_from_macro` |
| 36 | `document_sloppy` 8076–8105 | `\sloppy` (tolerance, emergency stretch) | no | same | `site36_sloppy_from_macro` |
| 37 | columns.rs `switches` 101–153 / `ColumnMode::scan` 447 (called at adapter 1730) | `\twocolumn`/`\onecolumn` state by position | no | same | `site37_twocolumn_from_macro` |
| 38 | `author_groups` 1531–1545 | `\and` splits `\author` into groups | partial: `Block::TitleBlock` authors; the split is decided from bytes | same | `site38_author_and_from_macro` |
| 39 | `body_commands` (toc arms), toc.rs `superseded_commands`/`has_lists` 454–495 | `\tableofcontents`/`\listof…` | partial: `Block::TableOfContents` exists; the adapter decides from bytes | same | `site39_tableofcontents_from_macro` |
| 40 | abstractenv.rs `ranges` 157–195, `page_ranges` 513–536 | the abstract's begin/body/end | no | same | `site40_abstract_opened_by_macro` |

### E. Floats, multicols, TikZ, math (6)

| # | site | fact | node field | tree | falsifier |
|---:|---|---|---|---|---|
| 41 | lib.rs 231–235 (`floats::scan` + `floats::mask`), floats.rs `scan` 139–193, `mask` 448–478 | which environments are floats. They are blanked out of the source before the compiler runs. | no: floats never reach the compiler (`Block::FigureCaption` only) | differs (by construction: the direct form's float is masked) | `site41_figure_opened_by_macro` |
| 42 | floats.rs `pieces` 289–437 | `\caption`/`\label`/`\includegraphics` at the float body's top level | no | same | `site42_caption_from_macro` |
| 43 | lib.rs 238–240 (`multicol::scan` + `Scan::masked`), typeset/multicol.rs `scan` 241–448, `masked` 126–140 | `multicols` regions. Literal bytes are masked, including a `\begin{multicols}` inside a preamble `\newcommand`, which opens a region there. | no | differs (masking precedes the compiler) | `site43_multicols_opened_by_macro` |
| 44 | adapter 4479 / tikz.rs `find_pictures`, lib.rs 281–285, typeset.rs `picture_block` 7128–7147 | `tikzpicture` extents and bodies, re-lexed from bytes | no | same | `site44_tikzpicture_from_macro` |
| 45 | typeset.rs `operator_limits_of` 8896 (and the atom-span family 1952–1999, 8689–9075: `fence_of`, `style_switch_of`, `class_override_of`, …) | display limits of `\lim`, fences, style switches, atom class | partial: `MathAtom` nucleus; limits and class re-read from bytes | same | `site45_operator_limits_from_macro` |
| 46 | `RowsEnv::at` 422–434 (called 2724, 2758) | align/gather/multline/eqnarray kind | partial: `Inline::MathRows { aligned }`; the kind is read from `\begin{..}` bytes | differs (the compiler does not expand `\gather` yet) | `site46_gather_through_environment` |

**Count by category.**

| category | sites |
|---|---:|
| style scope | 6 |
| interword glue and input conventions | 9 |
| paragraph and vertical structure | 17 |
| preamble facts | 8 |
| floats | 2 |
| multicols | 1 |
| TikZ | 1 |
| math | 2 |
| **total** | **46** |

Of these, 39 have a compiler tree that is the same for both forms, and 7
have one that differs.

### Sites probed that pass today (no falsifier committed)

The same macro-versus-direct probe passed for the following. In each case
a node field or an existing macro-aware path already covers the shape
tried:

- `\\[<dimen>]` from a macro body: `line_break_skip` 9700 prefers
  `LineBreak.skip_pt`, and the byte scan is only the fallback.
- Argument wrappers defined in a `.sty` (`\newcommand\Bb[1]{\textbf{#1}}`).
- One-level `\emph`/`\texttt`/`\textbf{\color{red}#1}` wrappers
  (`Styles::in_body`, `macro_argument_intervals`).
- Plain and nested text macros (`token_gap`'s `BodyCursor`).
- `\newcommand\mysec{\section}`.
- `\subsection*{#1 \hfill #2}` (`tests/heading_macros.rs`).
- A called `\setcounter` or `\setlist` in the preamble.
- `\newpage`, which reaches the adapter through the compiler's `PageBreak`
  block.
- `\vspace{2em}` and `\hspace{1em}` at the body size.
- `\begin{small}`, `{\Large x}` spaces, `\lstset`, `\input` through a macro,
  `\left|..\right|` and `\mathrm{tr}` through a macro, `\clearpage` in two
  columns, and `\newblock`.
- A `twocolumn` class option.
- An uncalled `\usepackage[T1]{fontenc}` definition: 0.01 bp, too small to
  call a falsifier.

These rows of the helper inventories were not given a test:

- Sites with no pdflatex falsifier:
  - fontspec.rs `commands`/`scan`, which is XeLaTeX/LuaLaTeX only.
  - `verb_span`/`verbatim_environment_span`: `\verb` is illegal in a macro
    argument.
  - links.rs `scan`, which affects annotations, not layout.
- Sites that already follow macros: `length_at`/`length_assignments`
  6514–6630 and `macro_length_assignments`.
- The long tail, which reuses a scanner already covered above:
  - `microtype_setup`, `t1_encoding`, `natbib_options`, `package_options`,
    `graphics::mode`, `InputSetup::for_project`, `toc::Settings::read`.
  - `clear_page_blocks`, `size_environments`, `size_env_par_leading`,
    `space_style`, `kern_command_text`.
  - `cjk_gap_spaces`.
  - floats.rs `number`/`isolate`/`prepare`, multicol.rs lengths and
    `outer_doc`, `abstract_name`/`inside_vspace`.
  - listings.rs `scan`, `lstset_spans`.
  - typeset.rs `display_block` 7421 (split, leqno), `GridSpec::from_source`,
    `is_eqref_text`, `input_filtered`, `resolve_table_colors`,
    `math_fonts`.

Each of these reads bytes the same way as a site above, so the site's
migration should retire them too.

### Compiler-side re-scans found on the way

These are outside `render-pipeline`, but they block sites 15 and 25:

- `vendor/compiler/src/parser.rs` 11045 `custom_tag_text` reads `\tag` from
  the source bytes at each atom's span.
- `vendor/compiler/src/expansion.rs` 367 handles `\url`/`\href` in the lexer,
  and a macro that expands to `\url{..}` comes out as a space.

## Ranking by parity-scoreboard reach

Causes 2 and 4 of
[parity-2026-09-22/report.md](../evidence/parity-2026-09-22/report.md):

- **Cause 2**: project `.sty`/`.cls` read incompletely. 51 documents.
- **Cause 4**: project macro defined in the document. 37 documents.

The two causes cover 78 distinct documents. All of them are at L0 today:
each is held there by an unsupported construct, not by one of these sites.
Once the compiler expands those macros, their output goes through the
sites listed here.

**This is an estimate, not a measurement.** For each of the 78 documents,
every `\newcommand`, `\renewcommand`, `\providecommand`,
`\DeclareRobustCommand`, `\def`, `\let` and `\newenvironment` body in its
`.tex`, `.sty` and `.cls` files (cached under
`~/.cache/flashtex-parity/src`) was matched against each site's construct.
A match means a macro body contains the construct, not that the macro is
invoked. Class files inflate the cause-2 column.

| site(s) | documents (of 78) | cause 2 | cause 4 |
|---|---:|---:|---:|
| 1–4 style declarations and wrappers | **65** | 49 | 26 |
| 9 `{}` | 56 | 44 | 21 |
| 5 size macro before glue | 54 | 47 | 16 |
| 29 `\par` | 51 | 45 | 14 |
| 13 `\footnote` | 47 | 44 | 11 |
| 23 trivlist env / `\centering` | 47 | 42 | 14 |
| 11 accents | 46 | 41 | 13 |
| 45 math operators and fences | 44 | 29 | 21 |
| 14 `\hfil` | 43 | 42 | 7 |
| 16 `\noindent` | 43 | 40 | 9 |
| 33 `\setlength` | 43 | 41 | 9 |
| 21–22 list environments / `\item` | 41 | 35 | 12 |
| 34 `\setcounter` | 39 | 39 | 8 |
| 8 control space | 38 | 33 | 11 |
| 19 sectioning in a body | 38 | 38 | 6 |
| 32 `\pagestyle` | 38 | 37 | 6 |
| 30 `\vspace` | 37 | 33 | 10 |
| 7 dashes | 35 | 32 | 7 |
| 37 `\twocolumn` | 32 | 32 | 7 |
| 38 `\author`/`\and` | 31 | 31 | 5 |
| 10 `~` | 30 | 28 | 8 |
| 15 `\url`/`\href` | 28 | 25 | 8 |
| 36 `\sloppy` | 27 | 27 | 4 |
| 12 `\cite` | 20 | 18 | 7 |
| 17 marks | 20 | 19 | 5 |
| 41–42 floats / `\caption` | 13 | 11 | 5 |
| 27–28 proof / `\newtheorem` | 12 | 9 | 6 |
| 24 `equation` | 11 | 5 | 7 |
| 46 rows environments | 10 | 6 | 5 |
| 20 `\paragraph` | 8 | 8 | 2 |
| 39 toc lists | 6 | 5 | 1 |
| 44 TikZ | 5 | 5 | 1 |
| 18 `\chapter`/`\part` | 4 | 4 | 0 |
| 40 abstract | 3 | 2 | 1 |
| 26 `\qedhere` | 2 | 2 | 0 |
| 6 `\eqref`; 25 `\tag`; 35 `\geometry`; 43 `multicols` | 1 each | | |
| 31 `\setlist` | 0 | 0 | 0 |

Floats (41–43) and TikZ (44) rarely come from a macro. Their cost is the
byte masking itself, which the macro counts above do not measure: masking
runs on every document with a float. They stay slice 3 and 4 in §3.1, where
they are migrated as blocks rather than for macro reach.

## Proposed slice 2 scope

Slice 2 migrates the three highest-reach sites for which the compiler's tree
is already right (tree **same**), so each is a pipeline-only change plus one
new node field. The acceptance test is that these falsifiers pass with their
`#[ignore]` removed, and every other test in the crate still passes.

1. **Style scope: sites 1–4 (65 of 78 documents).**
   - Consume `Inline::Text.style`. Delete `source_style_intervals`,
     `macro_argument_intervals_defined_in` and `Styles::in_body`'s body scan
     as the source of bold/italic/slanted/caps/family.
   - New field: the adapter's style has three properties `TextStyle` lacks.
     Add them to the compiler's `TextStyle`:
     - `medium: bool`: `\normalfont`/`\mdseries` inside a heading.
     - `literal: bool`: verbatim text, which suppresses ligatures and makes
       blanks rigid.
     - `italic_correction: ItalicCorrection { before, after }`: the
       `\maybe@ic`/`\check@icl` decisions. Adding this also retires site 6.
   - Falsifiers: `site01`–`site04`, plus `site06`.
2. **Interword glue: sites 7, 8, 10 (plus 9).**
   - Consume `Text.space_before` instead of `token_gap`'s gap bytes.
   - New field: `Inline::Text.glue_before: Option<InterwordGlue>`, with
     `InterwordGlue { style: TextStyle, kind: Normal | ControlSpace | Tie }`.
     This is the §3.2 request in `rendering-abi.md`; the style gives the
     space's font (sites 5, 7).
   - Resolve the tie to a no-break glue in the parser (site 10), and add
     `Inline::Text.boundary_before: bool` for an explicit `{}`/`\relax`
     boundary (site 9).
   - Falsifiers: `site07`–`site10`, plus `site05` if `TextGlue` gains the
     same `style`.
3. **Paragraph indent: sites 16 and 22 (43 documents for `\noindent`).**
   - New field: `Block::Paragraph` gains `indent: bool`. The parser already
     tracks this state (`paragraph_started`, parser.rs 6593) and `\@endpe`.
   - Delete the `\noindent` arm of `body_commands`, `noindent_reaches`, and
     the `after_env` gap scan.
   - Falsifiers: `site16`, `site22`, and `site29`, which uses the same
     "paragraph continues" fact.

Site 21 (the list stack) is the one site where the field already exists
(`ListItem.lists`) and only the adapter must change. It is a good fourth
item if slice 2 has room. Sites 15, 25, 32 and 46 need the compiler fixes
named above first.

## Slice 2 status

- **Style scope (sites 1–4 and 6): migrated.** The compiler's
  `TextStyle::font` (the `nfss` state, now `flashtex_compiler::nfss`),
  `medium`, `literal` and `italic_correction`, `Inline::Text::glue_before`
  for the font of the space in front, and `style` on references, `\verb`
  and horizontal glue replace `source_style_intervals`,
  `macro_argument_intervals_defined_in`, `Styles::in_body`/`at`,
  `space_style` and `text_command_argument_at`. Falsifiers `site01`–`site04`
  and `site06` run un-ignored. The sizes of glue (`space_size`,
  `glue_size`, site 5) are still read from the gap.
- **Interword glue (sites 7–10): migrated.** Whether a run has glue in
  front is the compiler's `Inline::Text::glue_before` (horizontal mode, not
  after a control word, a control space, a line break or a display), a
  control space is a node with `GlueKind::ControlSpace`, a tie is U+00A0 in
  the text, and `boundary_before` marks an empty group or `\relax`
  between two words. `token_gap` still runs for the macro-body cursor, for
  `\newblock`, for CJK environment boundaries, for the space at the end of
  an `\input` file without a final newline, and for the gaps in front of
  non-text inlines (boxes, notes, glue, logos), which carry no
  `glue_before` yet. Falsifiers `site07`–`site10` run un-ignored.
- **Paragraph indent (sites 16, 22 and 29): migrated.** The compiler's
  `Parsed::block_par_starts` (one `ParStart { indent, par_before }` per
  block, kept like `block_par_leading`) replaces the `\noindent` arm of
  `body_commands`, `noindent_reaches`, the `after_env` gap scan and
  `gap_continues`; the lists a paragraph closes come from the previous
  `\item`'s `ListItem.lists` frames instead of `\end{..}` bytes in the gap
  (`gap_has_list_end`, `list_env_ends`). Each list's keys and `\setlist`
  state are still read from the source (site 21). Falsifiers `site16`,
  `site22` and `site29` run un-ignored.
