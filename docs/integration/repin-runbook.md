# Re-pinning `crates/render-pipeline/vendor` — runbook

**For the integration lane (the only lane that may move a vendor pin).**
Everything here was measured on `origin/main` at `dbe3cac8` (2026-09-16) in a
scratch worktree; the vendor swap itself was never committed.

`crates/render-pipeline/vendor/*` are frozen `git archive <sha>:crates/<name>`
snapshots plus a one-line `PIN` file (see `vendor/VENDORING.md`). No manifest
rewriting happens: a pinned directory is byte-identical to `crates/<name>` at
its `PIN`, with `PIN` the only extra file. Verified for the current compiler
pin:

```
$ git archive c95977d6:crates/compiler | tar -x -C /tmp/pinref
$ diff -rq /tmp/pinref crates/render-pipeline/vendor/compiler
Only in crates/render-pipeline/vendor/compiler: PIN
```

So a re-pin is a directory swap. What follows is what the *compiler* re-pin
needs on top of that.

## 1. What has to move — it is not `compiler` alone

`vendor/compiler` is at `c95977d6`, **41 `crates/compiler` commits behind
`main`**. Swapping only that directory does not compile: the newer compiler
needs a newer `tex-expansion`, and it has gained a sibling dependency that is
not vendored at all.

| Vendor directory | Action | Why |
| --- | --- | --- |
| `compiler` | re-pin `c95977d6` → `main` | the point of the exercise |
| `tex-expansion` | re-pin `87a513de` → `main` (7 commits) | **required.** The compiler-only swap fails with 21 errors, *all inside `vendor/compiler`*: `tex::FontSwitch`, `tex::is_output_limit`, `tex::output_limit_message`, `flashtex_tex_expansion::scale_internal_dimen`, `Engine::declare_host_assignment`, `Engine::declare_font_switch`, `IncrementalExpander::{input_position,limits,edit_with_limits}`, `FontMetrics::{quad_sp_in,x_height_sp_in}` |
| `bibliography` | **add** (new directory, pinned at `main`) | `crates/compiler/Cargo.toml` on `main` now declares `flashtex-bibliography = { path = "../bibliography" }`. It has no dependencies of its own, so vendoring it is a plain archive |

The remaining siblings do **not** have to move for this re-pin (they are behind
`main` too — `pdf` 8, `project-files` 11, `vector-graphics` 5, `math-layout` 4,
`font-engine` 2, `document-style` 1, `paragraph-layout` 1 — but nothing in the
newer compiler needs them). `tex-boxes`, `tex-text-encoding`, `microtype` and
`font-resources` have had no commits since their pin.

## 2. The pipeline-side half is already on `main`, behind a feature

Swapping the vendor trees is still not enough: the newer compiler adds enum
variants and struct fields that the pipeline's exhaustive matches and node
constructors do not cover, which is 17 further compile errors in
`crates/render-pipeline/src` and one in `tests/layout_cache_keys.rs`.

Those arms are **already committed**, gated behind the off-by-default feature
`compiler-node-surface` (declared in `crates/render-pipeline/Cargo.toml`). They
add no behaviour: every arm is the inert answer the pipeline already gave for a
node the old pin could not produce. Turning the feature on is therefore part of
the swap, not a separate risk.

New compiler surface the feature covers:

| Compiler item | Where the pipeline had to answer |
| --- | --- |
| `parser::Block::Penalty`, `Block::Tabbing` | `adapter::inlines_of`, `adapter::lower_blocks`, the block walk |
| `parser::Inline::ThePage`/`PageNumbering`/`TabStop`/`TabJump`/`Marginpar`/`Penalty`/`PagePenalty`/`Discretionary` | `adapter::inline_span`, the inline hash, `items_from_inlines_styled` |
| `parser::Block::VSpace::{stretch_pt,shrink_pt}` | `adapter::vspace_block` (new cfg'd constructor), the `VSpace` pattern |
| `parser::Inline::HSpace::{space_before_pt,space_after_pt,stretch_pt,stretch_fil,shrink_pt,shrink_fil}` | the `HSpace` pattern |
| `parser::Inline::LineBreak::skip_pt` | `adapter::line_break_inline` — already existed, gated by `linebreak-skip`; the one remaining open-coded `Inline::LineBreak { .. }` (the `\opening` lowering) now goes through it |
| `math::Nucleus::TextRun`/`SideSet`/`Lap` | `incremental::{hash_math,shift_math}`, `typeset::{convert_math_classed,math_grids,math_glue_em,math_approximations}`, `tests/layout_cache_keys.rs` |
| `math::Accent::Mathring` | `typeset::accent_char` |

Two of these arms are deliberate approximations and say so through
`math_approximations`, so they reach the document's limitations rather than
being invisible: `\sideset`'s left scripts (set on an empty box before the
operator, not measured in display style — PR #582 does it properly) and
`\mathllap`/`\mathrlap`/`\mathclap` (set as an ordinary group; math-layout has
no zero-advance lap box). `tabbing` (compiler #551, merged through integration
#642) is lowered to flush-left paragraphs with an `unsupported_block`
limitation, the way `LetterBlock` is, so no row is dropped before a pipeline
half applies `\=` stops and `\>` jumps.

## 3. The re-pin, exactly

From a clean worktree on `main`, with `NEW` the `main` SHA being pinned to:

```sh
NEW=$(git rev-parse origin/main)

for c in compiler tex-expansion; do
  rm -rf crates/render-pipeline/vendor/$c
  mkdir -p crates/render-pipeline/vendor/$c
  git archive "$NEW:crates/$c" | tar -x -C crates/render-pipeline/vendor/$c
  echo "$NEW" > crates/render-pipeline/vendor/$c/PIN
done

# new sibling: crates/compiler now depends on ../bibliography
mkdir -p crates/render-pipeline/vendor/bibliography
git archive "$NEW:crates/bibliography" | tar -x -C crates/render-pipeline/vendor/bibliography
echo "$NEW" > crates/render-pipeline/vendor/bibliography/PIN
```

Then, outside `vendor/`:

1. **`crates/render-pipeline/Cargo.toml`** — move `compiler-node-surface` and
   `linebreak-skip` into `default`:

   ```toml
   default = ["amsmath-inline", "request-date", "compiler-package-gating", "par-leading", "math-class-override", "compiler-node-surface", "linebreak-skip"]
   ```

   (`linebreak-skip`'s own precondition — `parser::Inline::LineBreak::skip_pt`
   — is met by the same re-pin; its feature comment already says turning it on
   is then a one-line default-feature edit. Leaving it off also works: the
   `line_break_inline` constructor compiles either way.)

2. **Lockfiles.** CI runs `cargo build/test --locked`, so both lockfiles that
   see the vendor tree must gain the new crate, or every job fails before
   compiling. Regenerate by building once in each directory; the whole delta is:

   ```
   crates/render-pipeline/Cargo.lock | 5 +++++
   crates/flashtex-cli/Cargo.lock    | 5 +++++
   ```

   (a `[[package]] flashtex-bibliography` stanza and one `dependencies` line
   under `flashtex-compiler`).

3. **Un-ignore the two tests that were waiting for this pin.** Both were
   measured to pass against the re-pinned vendor:

   - `crates/render-pipeline/tests/frame_env.rs:140`
     `#[ignore = "needs vendor/compiler re-pinned past the frame environment change"]`
   - `crates/render-pipeline/tests/math_symbols.rs:449`
     `#[ignore = "needs re-pin past #529 (compiler: \\coloneqq decomposes under mathtools)"]`

   ```
   test frame_draws_a_rule_border_around_its_content ... ok
   test coloneqq_decomposes_like_mathtools ... ok
   ```

4. **`crates/render-pipeline/vendor/VENDORING.md`** — update the `compiler` and
   `tex-expansion` rows and add a `bibliography` row. The `compiler` row still
   claims a temporary pin to the unmerged `engine/letter-class-compiler`
   (PR #316); that is stale twice over, since the current pin is `c95977d6` on
   `main`.

## 4. Expected test deltas

Measured with `CARGO_BUILD_JOBS=4`, debug profile, whole suite,
`--no-fail-fast`, `crates/render-pipeline`:

| | passed | failed |
| --- | --- | --- |
| `origin/main` as it stands (pinned vendor) | 469 | **13** |
| this branch, feature off (pinned vendor) | 469 | **13** — the same 13 names |
| this branch, feature on, vendor re-pinned | **485** | **0** |

The re-pin does not just unblock the stacked PRs: it **fixes 13 tests that are
red on `main` today**. All 13 pass after the swap:

```
every_line_is_where_pdflatex_put_it                    (letter_class_geometry)
letter_paragraph_gaps_are_the_class_parskip            (letter_class_geometry)
opening_lines_are_one_paragraph_not_one_each           (letter_class_geometry)
page_one_texttop_fil_pushes_the_first_baseline_down    (letter_class_geometry)
the_raggedleft_group_closes_no_trivlist                (letter_class_geometry)
shared_colorbox_path_draws_border_with_content_inside  (frame_env)
sout_paints_a_rule                                     (underline_sout)
sout_with_ulem_is_not_an_unsupported_error             (underline_sout)
text_mode_underline_is_not_an_unsupported_error        (underline_sout)
text_mode_underline_paints_a_rule                      (underline_sout)
uline_paints_a_rule_under_the_argument                 (uline)
uline_with_ulem_is_not_an_unsupported_error            (uline)
xcolor_fixtures_match_pdflatex                         (xcolor_oracle)
```

Plus the two `#[ignore]`d tests above, for **487 passing, 0 failing** once they
are un-ignored. No test that passes today fails after the re-pin.

`crates/flashtex-cli`, built against the same re-pinned vendor:

```
test result: ok. 58 passed; 0 failed
```

Two `#[ignore]`s are *not* fixed by this pin and stay as they are:
`adapter::tests::preamble_scan_does_not_see_input_files` (an unrelated
unimplemented path) and the slow release-only `incremental` test.

## 5. What the re-pin unblocks

Merged compiler halves that the pipeline currently cannot see: #568, #581,
#583, #587, #606, #616, plus #529 (the `\coloneqq` `#[ignore]` above), #551
(tabbing, merged through #642), #549 (`\mathllap`/`\mathrlap`/`\mathclap`) and
#555 (`\marginpar`, merged through #645).

Open pipeline PRs whose merge is gated on this pin:

| PR | Branch | What it needs |
| --- | --- | --- |
| #569 | `agent/daniel-parent/page-para-control-pipeline` | #568 — penalty/discretionary/parameter nodes |
| #584 | `agent/daniel-parent/page-control-followups` | #568 + #583 — `\enlargethispage`, `\clearpage` float flush, `\flushbottom`, `\nobreakdash` (stacked on #569) |
| #599 | `agent/daniel-parent/page-control-3` | stacked on #584 — float after a page break, `\pagebreak\section` under `\flushbottom` |
| #607 | `agent/daniel-muse-lead/vskip-glue-pipeline` | #606 — `VSpace` stretch/shrink |
| #617 | `agent/daniel-parent/pagebreak-midparagraph-pipeline` | #616 (stacked on #599) |
| #582 | `agent/daniel-parent/sideset-pipeline` | #581 — `Nucleus::SideSet` |
| #585 | `agent/daniel-parent/tag-placement-pipeline` | #470 — rich `\tag` placement |

Each replaces one or more of the inert `compiler-node-surface` arms with real
layout; none of them has to be rebased *onto* the re-pin first, because the
arms they replace are already on `main`.

Also unblocked, though currently closed rather than open: #557 (place
`\marginpar` in the right margin, GH-505) was closed as "needs vendor re-pin"
and can be reopened once the pin moves; the pipeline half of tabbing (#551's
horizontal `\=`/`\>` handling) has no open PR yet.

## 6. What this runbook does not cover

- `math-layout` (4 commits behind), `pdf` (8), `project-files` (11),
  `vector-graphics` (5): their own re-pins, and the `math-glyph-spans` /
  `math-font-kerns` features waiting on them, are separate and were not
  measured here.
- PR #631 was an earlier readiness attempt on a stale snapshot; it is
  superseded by this document and should be closed rather than revived.
