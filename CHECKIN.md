# CHECKIN — lane `phantom-review-fix-4`, slice 1

## 1. Merge-conflict resolutions (`git merge origin/main`, 4 files)

All four conflicts are the same shape: this branch added text-mode
`\phantom` support while `origin/main` independently added kernel
`\textsuperscript`/`\textsubscript`, both extending the same `match`
sites / impl blocks. Every site now keeps **both** arms; verified by
`cargo check`, the `textscript` suite (8 passed, main's feature intact)
and the `text_phantom` suite (21 passed, this branch's feature intact).

- `crates/compiler/src/incremental.rs` (`shift_inlines`): kept HEAD's
  `Inline::Phantom` arm (map span + shift content) **and** main's
  `Inline::TextScript` arm (map span + shift content).
- `crates/compiler/src/layout.rs`, `visit_inline_references`: kept HEAD's
  `Inline::Phantom` arm (refs inside a phantom still warn) **and** main's
  `Inline::TextScript` arm.
- `crates/compiler/src/layout.rs`, `emit`: kept HEAD's full
  `Inline::Phantom` arm (unbroken-box measurement, overflow wrap check,
  vertical extents, underline-depth reserve) **and** main's full
  `Inline::TextScript` arm (local `\sf@size` resolution, mark-size emit,
  raise/lower shift).
- `crates/compiler/src/parser.rs`: kept HEAD's `text_phantom` method
  **and** main's `text_script` method (with its outer-size clearing).
  Command dispatch already contained both (`phantom|hphantom|vphantom`
  and `textsuperscript|textsubscript`); the `Inline` enum and
  `supported.rs` inventory had auto-merged with both variants/entries.
- `docs/user/compiler.md`: took **main's side**, then regenerated via
  `crates/compiler/scripts/render_supported_latex.sh` (which also
  re-synced `supported-latex.json`, `coverage.md` and the Mac bundled
  copy). Regenerated header reads **363 text-mode** (= main's 360 + this
  branch's 3 phantom commands), 570 math-mode, 25 packages — both sides'
  counts survive. `cargo test --test supported_latex` passes, so the
  drift gate confirms the doc is current.

No conflict markers remain anywhere (`git grep "<<<<<<<|>>>>>>>"`
empty; `git status` reports "All conflicts fixed").

## 2. Caption test vs real pdflatex behavior

The fixture `\caption{AAAAAAAAAAAAAAAAAAAAAAAA\\B}` (24 A's, explicit
`\\`) was asserted to break into two lines with the first centred on its
own width. That never matches pdflatex: article.cls `\@makecaption`
measures the caption in an `\sbox` — restricted horizontal mode, so `\\`
(`\@xnewline`: unskip before, ignore spaces after) is glue, never a
break — and centres the **whole single line** when it fits `\hsize`.
Measured in this engine: single-line width ≈ 262.6pt (`Figure 1:`
43.668 + space 3 + 24 A's 207.936 + `B` 8.004 at 12pt) against a 468pt
measure — it fits, so pdflatex sets it on **one** centred line.

Compiler fix (`crates/compiler/src/layout.rs`, `FigureCaption` arm):
measure the break-joined run (`caption_single_line`: drops top-level
`LineBreak`s, clears the following `space_before`, mirroring
unskip+`\ignorespaces`); if that single line fits the measure, emit it
whole on one centred line; otherwise keep the old paragraph path
(`\\` breaks, lines wrap from the margin). New helpers
`caption_single_line` / `clear_space_before` live next to
`caption_box_width`; captions without `\\` take a byte-identical path
to before.

Test fix (`crates/compiler/tests/references_and_figures.rs`):
replaced `figure_caption_centering_measures_the_broken_line` with
`figure_caption_short_explicit_break_stays_on_one_centred_line` —
asserts one shared baseline, `B` glued to the A-run with zero break
width (line edges equal the joined oracles `\caption{AAAA…AB}` and,
for the spaced case, `\caption{A \\ B}` ≡ `\caption{AB}`), and whole-line
centering — plus `figure_caption_overwide_break_opens_at_the_margin`
for the overflow branch (breaks, opens at the margin, i.e. paragraph
mode). Probe (since removed) showed the old code emitting y=84 then
y=98.4 (two lines, first line offset for its own width) and the new
code emitting all three items at y=84 with `B` at 429.3 = A-run end
exactly, left/right gaps equal at 102.7pt.

## 3. `place` empty-result callers

`place`'s actual current contract (`crates/compiler/src/layout.rs:1005`,
`fn place(&mut self, text: String, size: f64, span: Span, font: Font,
space_before: bool)`): it always advances the cursor and reserves
extents, but pushes a `TextItem` **only when `text` is non-empty**
(round-3's no-empty-items change; `push_item` has the same early return).
So after `place("")` there is no new item to adjust.

Audit of every `place`/`push_item` caller (12 `place` sites, 3
`push_item` sites): the ONLY site that touches the just-placed item is
the footnote-mark raise in `crates/compiler/src/layout/footnotes.rs:279`
(the finding's `footnotes.rs:225` is a stale path — the file has always
lived at `layout/footnotes.rs`, and its line 225 is unrelated). That
site was already defensive since the file's creation and is now
load-bearing under the new contract:

```rust
if let Some(item) = self.pages.last_mut().and_then(|page| page.items.last_mut()) {
    item.baseline_y_pt = round2(item.baseline_y_pt - raise);
}
```

No change needed there. All other callers (`emit` text/reference/
verbatim arms, heading numbers, verbatim-block lines, TOC entries, page
numbers) never index the just-placed item — verified arm by arm — so no
other site can panic or mis-adjust on an empty result. The `None` arm is
currently unreachable via public input (footnote numbers are never
empty), so it stands as cheap insurance, consistent with the new
contract.

Tests: the empty-result path already had end-to-end coverage
(`box_trailing_space_emits_no_empty_text_item` in `text_phantom.rs`:
`place("")` via box-edge markers pushes no `""` item while geometry is
preserved). Added contract-level coverage alongside it: new unit test
`layout::tests::place_empty_text_pushes_no_item_but_still_advances`
(empty place pushes nothing on an empty page, cursor still advances,
guarded adjust finds nothing, non-empty place still pushes). It fails
under the old unconditional-push contract and passes now.

## 4. Test commands and output (all from `crates/compiler`)

- `cargo test --locked --test text_phantom` — **21 passed, 0 failed**
  (includes round-3's `box_trailing_space_emits_no_empty_text_item`).
- `cargo test --locked --test references_and_figures` — **19 passed,
  0 failed** (rewritten one-line test, new overflow test, diagnostics-once
  guard, and all pre-existing tests).
- `cargo test --locked --test footnote_counters` — 8 passed;
  `--test footnote_long_argument` — 2 passed;
  `cargo test --locked -p flashtex-compiler --lib
  layout::tests::place_empty` — new unit test passes.
- `cargo test --locked --test supported_latex` — 8 passed (drift gate
  green after regeneration); `--test textscript` — 8 passed.
- `cargo test --locked` (full crate suite) — **1042 passed, 0 failed,
  10 ignored (pre-existing `#[ignore]`s; none added), exit 0** across
  all 85 test targets, including the 461 lib unit tests. Zero
  regressions.