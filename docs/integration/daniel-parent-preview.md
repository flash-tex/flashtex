# daniel-parent integration preview

Branch `agent/daniel-parent/integration-preview`: origin/main `36fe7ec3` plus the 43 open non-draft
`agent/daniel-parent/*` PRs, merged with `git merge --no-ff` (no rebases, vendor/ untouched).
This is a preview for the integration lane (GoKubar / kabir-claude). **Do not merge this branch directly.**
Merge the individual PRs in the order below, and reuse these resolutions when a PR conflicts.

## Recommended merge order

Stacks come from the PR bodies ("stacked on #N") and were confirmed by branch ancestry
(`git merge-base --is-ancestor`). A child PR's branch contains its parent, so merging the child also lands the parent.

1. project-files: #518 → #539 (stacked on #518)
2. pdf: #527 → #533 (stacked on #527) → #546
3. compiler + math-layout: #470 → #581 (stacked on #470; 20 commits behind main)
4. compiler: #517, #526, #529, #542, #535 → #579 (stacked on #535)
5. compiler + tex-expansion: #577 → #592 (stacked on #577) → #601 (stacked on #592), then #587
6. compiler: #568 → #583 (stacked on #568), then #558
7. math-layout: #560 → #589 (stacked on #560)
8. render-pipeline: #524, #525, #528, #530 (needs #529), #536, #537 + #538 → #578 (stacked on both), #543, #545,
   #547 → #556 (stacked on #547), #548, #580, #596, #597
9. docs: #531, #575 (a design proposal awaiting a Commander decision; it merges cleanly, so merge or skip it)
10. cli / protocol / mac: #362, #358 → #363 (stacked on #358)

Skipped (drafts blocked on a vendor re-pin): #569, #584 (stacked on #568/#569), #599 (stacked on #584),
#582 (stacked on #581), #585 (stacked on #470/#581).

## Conflicts and resolutions

12 of the 43 merges conflicted, in 23 files.

| Already merged | Incoming PR | File | Resolution |
|---|---|---|---|
| #533 (via #527) | #546 | crates/pdf/tests/v2.rs | Both appended tests at the end of the file; kept both (#527/#533's TikZ path/alpha tests, then #546's ellipsis ToUnicode test). |
| main + #470 | #581 (20 commits behind main) | crates/compiler/supported/coverage.md, docs/user/compiler.md (+ supported-latex.json, apps/mac bundled copy) | Generated artefacts; took one side, then regenerated with `sh crates/compiler/scripts/render_supported_latex.sh`. |
| #526 | #587 | crates/compiler/src/parser.rs | Both add `TABLE_LENGTHS` and an `is_table_length` dispatch arm. Kept one const (#526's doc comment) and #587's arm, which passes #587's new `global` flag to `length_assignment`; #526's comment moved onto it. The duplicate two-argument arm was dropped (it would not compile against #587's signature). |
| #601 | #587 | crates/tex-expansion/src/expand.rs | `Checkpoint` gained `last_origin` (#601) and `quad_sp`/`x_height_sp` (#587). Kept all three fields in the struct, in `checkpoint()` and in `restore()`. |
| #601 | #587 | crates/tex-expansion/src/incremental.rs | Checkpoint shift: kept #601's `steps` offset and `last_origin`, plus #587's `quad_sp`/`x_height_sp`. `cargo check --tests` passes for tex-expansion and compiler. |
| #587 | #568 | crates/compiler/src/parser.rs | `parse_project_with`'s `P { .. }` initialiser: kept #587's `length_scopes`/`pending_global` and #568's `parameters`/`parameter_scopes`/`hyphenation`. `cargo check --tests` passes. |
| #581/#587 | #568 | docs/user/compiler.md, crates/compiler/supported/{coverage.md,supported-latex.json}, apps/mac bundled supported-latex.json | Generated; regenerated with the script. |
| #568 (merged, regenerated) | #583 | docs/user/compiler.md | Generated section only; regenerated with the script. |
| #535 (`char_table`) | #558 | crates/compiler/src/lib.rs | Adjacent module declarations; kept `mod char_table;` and `pub mod biblatex;`. |
| #542 | #558 | crates/compiler/src/vocabulary.rs | `command_package`: kept #542's listings/minted split (`lstlistoflistings` is listings, `listoflistings` is minted) and #558's `citeyear` (natbib) and `cite`/`parencite`/`textcite`/`autocite`/`nocite` (biblatex) arms. |
| #583 (merged, regenerated) | #558 | docs/user/compiler.md | Generated section only; regenerated with the script. |
| #581 | #560 | crates/math-layout/src/layout.rs | `Engine::atom`: took #560's signature (new `text_font_pair` argument, doc comment) and kept #581's `left_scripts` early return to `make_sideset` at the top. All call sites already pass four arguments. `cargo check --tests` passes. |
| #537 + #578 | #543 | crates/render-pipeline/src/{toc.rs,lib.rs,adapter.rs}, plus a follow-on edit in listings.rs | This is the one semantic conflict. #537/#578 give each figure/table entry its own `number` (`floats::number`) and merge list entries in reading order (`Labels::reading_order`), and they dropped the adapter's `chapter_starts`. #543 numbers LoF/LoT/LoL entries inside `list_blocks` from `chapter_starts`, adds listing entries (`listed`, `nolol`) and adds `\addvspace` chapter gaps. Resolution: `FloatEntry` keeps both `number` and `listed`. `lib.rs` keeps #578's numbering and reading order, then extends with #543's `listing_entries`. `adapter.rs` keeps #543's `chapter_starts`/`chapter_gaps` and its `list_blocks` call, but now pushes them as reading positions (`reading_position(cmd_doc, cmd.start)`), because chapters can come from `\include`d documents after #538/#578. In `list_blocks`, figures and tables use `f.number` (#578); only LoL entries are counted with `chapter_numbers` (#543), over reading positions; the chapter-gap check compares reading positions. `listings::numbers` takes the reading order so `\thelstlisting` agrees with the LoL; its two unit tests pass `&[]`, where the offset fallback applies. `cargo check --tests` passes. |
| #556 | #596 | crates/render-pipeline/src/adapter.rs | Adjacent edits. #556 dropped `font_declaration`'s italic-correction flag (declaration groups add no `\/`); #596 inserted `text_command_argument_at` (`| #536 | #597 | crates/render-pipeline/tests/math_symbols.rs | Same test assertions. Kept #536's `\Longrightarrow` join assertion (`=` then `⇒y`) and #597's semantic U+2212 minus run text. |
| main (`watch --timing`) | #362 | docs/user/compiler.md (hand-written CLI synopsis, outside the generated section) | Union: `check` gains #362's `--fix` and `--dry-run`, and `watch` keeps main's `--timing`. The generated section was re-rendered with the script. |
| main (structured labels/notes/help folding in `from_compiler`) | #358 (43 commits behind main) | crates/render-pipeline/src/display.rs | #358's side had no change in either hunk; it predates main's folding. Kept main's `sources`/`message` folding. #358's v2-diagnostics capability code auto-merged around it. `cargo check --tests` passes. |

After the last merge, re-running `sh crates/compiler/scripts/render_supported_latex.sh` produced no diff,
so the generated artefacts match the combined compiler.

## Combined test results

Run on the merged tip with `CARGO_BUILD_JOBS=4`. The crate tests used the debug profile with `cargo test --locked --no-fail-fast`;
render-pipeline used `cargo test --release --locked --no-fail-fast`.

| Crate | Passed | Failed | Ignored |
|---|---|---|---|
| compiler | 747 | 1 | 7 |
| tex-expansion | 75 | 0 | 0 |
| project-files | 97 | 0 | 0 |
| pdf | 123 | 0 | 0 |
| math-layout | 74 | 0 | 0 |
| render-pipeline (release) | 439 | 1 | 3 |

### Failures caused by the combination

1. **compiler, `tests/robustness.rs::nested_sub_parses_hit_tex_grouping_capacity_instead_of_the_stack`, debug only.**
   `thread '...' has overflowed its stack` / `fatal runtime error: stack overflow, aborting`.
   - The abort also killed the rest of that test binary; re-run with `--skip`, the other 13 pass.
   - The test passes in `--release`, which is what CI runs.
   - #601 alone (with #577/#592) passes. Bisecting the preview's first-parent merges in debug, the test passes after #583 (`59311b2b`) and overflows after **#558** (`a1ce8b4c`).
   - #558 (basic biblatex) makes the parser's recursive dispatch larger (the `\printbibliography`, `\addbibresource` and biblatex citation arms). #577 set `STREAM_DEPTH_LIMIT = 32` to stay "below what a 2 MiB debug test thread holds", and the larger frames now exceed that.
   - Fix options, for the #558 or #577 owner: move the new biblatex arms into `#[inline(never)]` helpers; lower `STREAM_DEPTH_LIMIT`; or run the test on a thread with an explicit stack size.
   - **Fixed on #558** (`16dd331f`, merged here): the limit stays 32. `\addbibresource` and `\printbibliography` moved out of `P::command` into `#[inline(never)]` helpers, and the biblatex `\citeauthor`/`\citeyear` check moved into `natbib_cite`. Debug stack per nesting level (tabular / footnote): #601 57232 / 31424 bytes; #601+#558 before 58672 / 32864; after 57360 / 31552. The test passes in debug on #558, on #601+#558 and here.
   - Margin warning: this preview's debug frame is 65136 / 39104 bytes per level (other merged PRs add about 7.8 KB per level), so 32 nested tabulars use about 2.0 MiB. The test passes, but a small further growth of `P::command` will overflow the 2 MiB debug test thread again.
2. **render-pipeline, `tests/math_symbols.rs::common_math_glyph_runs_use_semantic_unicode`.**
   - `left: "…↦=⇒−→↪…"`, `right: "…↦⟹⟶↪…"`.
   - #536 sets `\Longrightarrow`/`\longrightarrow` as pdfTeX joins (`=`+`⇒`, `−`+`→`), so the math runs' text is the pieces. #597 added this test expecting the semantic single characters `⟹` and `⟶`. Each PR is self-consistent; together their intents conflict.
   - Not resolved here. The decision was to keep #536's two-piece drawing and give the join the semantic text. That is blocked as specified; see the comment on #536:
     - A cluster with empty text is invalid display-list-v2: `crates/rendering-core` and the Mac decoder require non-empty clusters that partition the run text.
     - `\Longrightarrow`'s `=` is an LM Roman glyph and its `⇒` an LM Math glyph, so they are in different runs and cannot share one cluster.
     - The exact PDF route writes one ToUnicode entry per glyph id, first use wins. A prototype that put `⟹`/`⟶` on the join extracted `x ⟹⇒y ⟶⟶z and a ⟹b ⇒c ⟶d ⟶e` from `$x\Longrightarrow y \longrightarrow z$ and $a=b\Rightarrow c - d \to e$`, so every later real `=`, `−` and `→` extracted as the arrow.

No other failures.

## Vendor re-pins needed after merging

render-pipeline builds against the frozen mirrors in `crates/render-pipeline/vendor/`; see `vendor/VENDORING.md`.
These PRs change crates that are mirrored there. Their pipeline effect appears only after re-pinning.

| Mirror | PRs to re-pin past | Blocked pipeline work |
|---|---|---|
| `vendor/compiler` | #470, #517, #525, #526, #529, #535, #542, #558, #568, #577, #579, #581, #583, #587, #592, #601 (#524 only touches a compiler test) | drafts #569, #584, #599 (need #568/#583); #582, #585 (need #470/#581); #530's `\coloneqq` composition needs #529 |
| `vendor/math-layout` | #470, #560, #581, #589 | #560/#589's `math-font-kerns` feature stays off until then; #582, #585 |
| `vendor/tex-expansion` | #577, #587, #592, #601 | none |
| `vendor/pdf` | #527, #533, #546 | none |
| `vendor/project-files` | #518, #539 | none |
| bibliography | none; no merged PR touches `crates/bibliography` (#558's biblatex is in `crates/compiler`) | none |
