# flashtex-tex-boxes: contract and adoption proposal (KC-107)

TeX's box machinery from *TeX: The Program*, with section numbers cited in the
code: scaled arithmetic, the node model, `hpack`/`vpackage` with TeX's
diagnostics, `\showbox` text, shipout positions, and a primitive box engine.
On top of that sits LaTeX's box layer from `latex.ltx` (TeX Live 2026, with
line numbers cited). The crate has no dependencies, uses integer scaled points,
and changes nothing outside `crates/tex-boxes/`.

## Status: verified vs believed

**Verified against pdflatex** (pdfTeX 3.141592653-2.6-1.40.29, TeX Live 2026).
`tests/oracle/generate.py` runs 151 fixtures in one pdflatex job, and the job's
log contains no TeX errors. `cargo test` replays every fixture through
`BoxEngine` and the LaTeX layer. All 151 fixtures match on:

- `\wd`, `\ht` and `\dp` to the sp, and `\badness`;
- the `\showbox` text, byte for byte;
- the transcript text of every box: over/underfull, tight and loose reports,
  the overfull rule, `\showboxdepth`/`\showboxbreadth` truncation, and LaTeX
  warnings.

All 92 position fixtures match to the sp, measured with `\pdfsavepos`. They cover:

- glue at every order, stretch and shrink;
- 40–50-item glue lines, where accumulated rounding shows;
- nested glue;
- a/c/x leaders;
- `\vtop`, `\boxmaxdepth`, `\baselineskip`/`\lineskip`/`\lineskiplimit`;
- register surgery;
- `\mbox`, `\makebox`, `\framebox`, `\fbox`, `\raisebox`, `\parbox`, `minipage`;
- `\rule`, `\hspace`/`\vspace` and their star forms, struts, phantoms, `\smash`,
  laps, `\sbox`/`\savebox`/`\usebox`, `\settowidth`/`\settoheight`/`\settodepth`,
  and `lrbox`.

Findings worth knowing (all confirmed by the oracle):

- **Glue ratio is a C `double`.** pdfTeX's `glue_set`/`glueratio` is a double.
  If `glue_set` is stored as `f32`, the positions of `g_long_stretch`,
  `g_long_odd` and `g_long_vertical` break. With `f64` they match. `BoxNode::glue_set` is `f64`.
- **LaTeX ≥ 2021-06 para hooks change vertical lists.** The standard
  `\everypar` takes the indent box, ends the empty paragraph, and restarts with
  `\noindent`. It does that inside a group where `\parskip` is 0pt, or
  `-\parskip` under `\if@minipage`. As a result, every paragraph after the
  first in a vlist gets **two** `\glue(\parskip)` nodes. The engine models this
  when `latex_para_hooks` is set; `latex::setup_article_10pt` sets it. Plain
  TeX leaves it off.
- `\strutbox` in article 10pt is 550500sp + 235932sp. `.3\baselineskip` goes
  through `round_decimals`, giving 19661.
- A LaTeX warning's leading blank line depends on pdfTeX's terminal column,
  because `print_nl` in `write_out` checks both `term_offset` and `file_offset`.
  `BoxEngine::term_offset` carries that state.

**Believed (from tex.web, not oracle-checked).**

- Characters and ligatures. There are no fonts in the fixtures, and char
  metrics come from the `CharMetrics` trait.
- Discretionaries inside `\lastbox`.
- `\vadjust`, and insert/mark migration out of `hpack`.
- `\vcenter` inside real math lists. Only `\parbox`'s `$\vcenter$` is exercised.
- Multi-line paragraphs. The default `SingleLineBreaker` is exact only when
  TeX would choose one line.

## Modules

| Module | tex.web / latex.ltx | Contents |
|---|---|---|
| `scaled` | §§99–109, 453–458 | `round_decimals`, `print_scaled`, `xn_over_d`, units, `<factor><internal dimen>` |
| `node` | §§133–161 | char/ligature, box, rule, glue (4 orders), kern, margin kern, penalty, math, disc, whatsit, mark, ins, adjust |
| `pack` | §§108, 644–678 | `badness`, `hpack`, `vpackage`, reports, `ExpansionHook` |
| `display` | §§173–198 | `show_box`, `short_display`, a printer with `file_offset` |
| `shipout` | §§619–637 | `ship_out` → `ShipEvent`s (chars, rules, whatsits, boxes) with `cur_g`/`cur_glue` rounding and leaders |
| `engine` | §§211–283, 1055–1110, 1167–1196 | `BoxEngine`: the nest, save stack, registers, box contexts, `\lastbox`, un-box, `\vadjust`, `\vcenter`, paragraphs |
| `latex` | latex.ltx (ltboxes, ltspace) | the box commands listed above, `setup_article_10pt` |

## Adoption: `crates/compiler` and `crates/render-pipeline`

Today `crates/compiler/src/layout.rs` places words greedily in points.
`crates/render-pipeline` carries its own float page builder
(`pagebuild.rs`) and a styled block model (`adapter.rs`). Neither has TeX's
box semantics. The proposed path, in order:

1. **Dependency.** `flashtex-tex-boxes = { path = "../tex-boxes" }` in the
   crate that owns typesetting (render-pipeline today).
2. **Metrics.** Implement `CharMetrics` over font-resources' TFM data. It needs
   `char_dims(font, ch)` and `font_identifier(font)`, the latter e.g.
   `\OT1/cmr/m/n/10`, which `\showbox` and the overfull reports print.
3. **Boxes as the layout IR.** Replace ad-hoc float boxes with `node::Node`
   and `BoxNode` lists: characters via `BoxEngine::append_char`, glue via
   `hskip`/`vskip` using TeX's param-glue identities, and LaTeX constructs via
   `latex::*`. Nothing changes for callers that only want pixels.
4. **Output.** `shipout::ship_out(&page_box, h0, v0, metrics)` yields
   positions in sp, identical to pdfTeX. Convert them to PDF user space once,
   at the end, as `bp = sp / 65781.76`, instead of accumulating floats.
   `Whatsit` events carry tags for source-span mapping: the IDE's
   click-to-source can attach a span id to a whatsit, the way the oracle uses
   `\pdfsavepos`.
5. **Diagnostics.** `BoxEngine::log()` and `reports()` give pdflatex's exact
   Overfull/Underfull text, so the IDE problem list can show the same messages
   as pdflatex. Set `line` from the expansion span as boxes open (TeX reports
   `pack_begin_line`).

## Plug-in points for sibling crates

### KC-101 `tex-expansion` (the "mouth") → `BoxEngine` (the "stomach")

`tex-expansion` leaves unrecognized control sequences in its output stream for
the typesetting layer. The adoption diff is a main-control loop that feeds that
stream into `BoxEngine`. `tests/dsl/mod.rs` is a working, oracle-verified
miniature of that loop. Mapping (§1045 `main_control` cases):

| Token / primitive | `BoxEngine` call |
|---|---|
| `{` / `}` (simple group), `\begingroup`/`\endgroup` | `begin_group` / `end_group` |
| `\hbox`/`\vbox`/`\vtop` [`to`/`spread`] `{` … `}` | `begin_box(kind, PackSpec, BoxContext)` … `end_box` |
| `\setbox n=`, `\global\setbox`, `\raise`/`\lower`/`\moveleft`/`\moveright`, `\leaders`/`\cleaders`/`\xleaders`, `\shipout` | the `BoxContext` passed to `begin_box` / `use_box` / `copy_box` / `last_box` |
| `\box n`, `\copy n`, `\lastbox`, `\unhbox`/`\unhcopy`/`\unvbox`/`\unvcopy` | `use_box`, `copy_box`, `last_box`, `unpackage` |
| `\wd`/`\ht`/`\dp n` (read and assign) | `box_dimen` / `set_box_dimen` |
| `\hskip`/`\vskip`/`\hfil`/…/`\kern`/`\penalty`/`\vrule`/`\hrule` | `hskip`, `vskip`, `kern`, `penalty`, `vrule`, `hrule` |
| `\unskip`/`\unkern`/`\unpenalty`, `\lastskip`/`\lastkern`/`\lastpenalty` | same-named methods |
| `\indent`/`\noindent`/`\par`, a letter in vertical mode | `indent`, `noindent`, `par`, `leavevmode` |
| `\vadjust`, `\vcenter`, `$` | `begin_vadjust`/`end_vadjust`, `begin_vcenter`/`end_vcenter`, `begin_math`/`end_math` |
| characters | `append_char(font, ch)` (math lists belong to math-layout) |

The contract has two registers and two save stacks: the expansion engine's
integer, dimen and skip registers, and the box engine's `int`/`dimen`/`skip`,
which are named parameters. One of them must own the values, so both cannot
keep their own copies. Recommended: `tex-expansion` owns every register, and
`BoxEngine` reads `\hbadness`, `\baselineskip` and the rest through a small
trait replacing its internal `eq` table. Box registers stay in `BoxEngine`.
Grouping must drive both save stacks together: one `begin_group` is issued to
both, and `end_group` checks the group kind. Until that refactor, mirror the
assignments into `BoxEngine::set_*`, which is what the DSL does.

### KC-102 `microtype` / Knuth–Plass

- Line breaking: implement `engine::LineBreaker` with
  `break_paragraph(list, &LineBreakParams, metrics) -> Vec<BrokenLine>`. The
  engine already does §816, the final glue → `\penalty10000` plus
  `\parfillskip`. It also does post_line_break's packaging with
  `\leftskip`/`\rightskip`, `adjust_tail` and interline glue. The breaker
  returns the lines, whether each broke at glue, and the `\parshape`/hanging
  width and indent. Hyphenation and `\parshape` live in the breaker.
- Protrusion: the breaker inserts `Node::MarginKern { width, side }`, which
  `hpack` and `ship_out` treat as a kern and `\showbox` prints as
  `\kern… (left margin)` or `(right margin)`.
- Expansion: **not wired yet.** `pack::ExpansionHook` is a placeholder whose
  only method is `enabled() -> false`, and `hpack` does not take a hook. So
  `\pdfadjustspacing` lines are packed without expansion today. Adoption work:
  1. Extend the trait so it can compute pdfTeX's `cal_expand_ratio` from the
     line's char stretch and shrink totals, and return per-char expanded widths
     (microtype's `pdftex` module already computes both).
  2. Pass the hook into `hpack` from `post_line_break`.
  3. Add an expanded-glyph marker to `Node::Char` for `ship_out`.

  None of this is oracle-checked here; microtype's own oracle covers the
  arithmetic.

### KC-106 `page-builder`

- `engine::PageBuilder::build_page(&mut contributions)` is called wherever TeX
  calls `build_page`: after `\par` in outer vertical mode, `new_graf` at
  nest level 1, and box appends. The page builder takes contributions and
  returns nothing; output routines re-enter `BoxEngine` with
  `output_active = true`.
- Duplication to resolve at integration: page-builder has its own `scaled`,
  `node` and `pack` (vpackage, vlist_out). Both copies are oracle-verified on
  disjoint fixtures. Proposal: page-builder depends on `tex-boxes` for
  `node`/`pack`/`shipout` and keeps `page`/`split`/`vlist`/`latex`. The
  vpackage and vlist_out rounding here is exercised by `g_long_vertical`,
  `g_long_vshrink`, `v_*` and `l_vertical`.

## Regenerating the oracle

```sh
python3 crates/tex-boxes/tests/oracle/generate.py [--keep]
CARGO_BUILD_JOBS=3 cargo test --manifest-path crates/tex-boxes/Cargo.toml
```

With `--keep`, the pdflatex work directory is kept. Its log must contain no `^! `
lines other than `! OK.`. `cargo test` never runs TeX.
