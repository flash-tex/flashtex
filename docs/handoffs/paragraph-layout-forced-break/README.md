# paragraph-layout: forced break followed only by discardables (panic) — reviewable handoff

Lane `mac-paragraph-layout-2` (Claude Code subagent of `mac-claude-a`), scoped by
the Commander's authority correction (issue #2 comment 5647125681): nothing in
`crates/paragraph-layout` is committed here. The change is delivered as two
`git format-patch` files against the crate **as it is on `origin/main`**
(`crates/paragraph-layout` tree `d20d55ce`, last crate commit `e69165a8`,
observed at main `0294d2ca` through `d550d6b1`):

| File | Contents |
|---|---|
| `tests.patch` | `tests/forced_break.rs` — 6 tests; 5 fail on main with the panic, 1 contrast test passes before and after |
| `fix.patch` | `src/linebreak.rs` — +28/−2, three private functions; no public signature changes |
| `evidence.json` | hashes of inputs/outputs, the `cargo test --release`, clippy and fmt logs, the dump comparison numbers |

Apply on main (crate owner): `git am docs/handoffs/paragraph-layout-forced-break/tests.patch docs/handoffs/paragraph-layout-forced-break/fix.patch`
(both dry-run cleanly on a fresh `git archive origin/main crates/paragraph-layout`).
They also apply, with line offsets only, to render-pipeline's vendored copy (see
the note at the end).

## Symptom

Real-world corpus run (`docs/evidence/real-world-corpus-2026-09-12T144246Z/report.md`
lines 45–46, `tools/real-world-corpus/README.md` § "Helper misbehaviour"):
`flashtex-render` exits 101 on `article-twocolumn` and `cv` with

```
thread 'main' panicked at vendor/paragraph-layout/src/linebreak.rs:988:21:
slice index starts at 80 but ends at 79      (article-twocolumn)
slice index starts at 50 but ends at 49      (cv)
slice index starts at 6 but ends at 5        (minimal: "Hello \\" + blank line)
```

Line 988 of the vendored copy (= lane tip `0a1ba87d`) and line 882 of main's
copy are the same statement in `set_line`: `for it in &items[start..brk]`.

## Root cause — the index arithmetic

`ParagraphBuilder::line_break()` (`\\`) pushes `Glue(fil)`, `Penalty(-10000)`.
`ParagraphBuilder::finish()` then strips trailing glue and appends TeX's
paragraph end `Penalty(10000)`, `Glue(parfillskip)`, `Penalty(-10000)`. For the
minimal source `Hello \\` + blank line the item list is

```
index:  0          1         2          3            4             5                6
item:   Box(Hello) Glue(sp)  Glue(fil)  Pen(-10000)  Pen(+10000)   Glue(parfill)    Pen(-10000)
                                        ^ forced                                    ^ forced (final)
```

Legal breaks: 1 (glue after a box), 3 (forced), 6 (forced). 4 is `\penalty10000`
(never a break) and 5 is glue after a penalty (not a break).

`line_start(items, Some(3))` computes the next line's first item by skipping
every discardable after the break: 4 (penalty), 5 (glue), 6 (penalty) are all
discardable, so it returns `7 == items.len()`. The next break is 6. Both
`measure(start=7, brk=6)` (reads `p.width[6] - p.width[7]`, a negative width —
no panic, just wrong) and `set_line(start=7, brk=6)` (`&items[7..6]`, panic
"starts at 7 but ends at 6") are then called for that line. In general the
message is `starts at N but ends at N-1` because the final forced penalty is
always at `len - 1` and `line_start` ran to `len`. The same happens for any
break that lies inside the run of discardables after another break (two
consecutive `\\`: "starts at 8 but ends at 4" in the new tests).

Total-fit reaches `set_line` only for the chosen path, but its candidate
evaluation calls `measure` with the same `start > brk` pair, so besides the
panic it silently mis-measured such candidates (negative natural width and
stretch → `INF_BAD`, treated as infeasible) — the four total-fit cases in the
dump comparison below are exactly that.

First-fit did not panic on this input but silently produced one line instead of
two: after `cut(3)` it resumed at `b = start.max(4) = 7` and left the loop, so
the final forced break at 6 was never taken.

## What TeX does

`\\` in LaTeX is `\unskip\penalty10000\hfil\break` (ltspace.dtx `\@gnewline`);
the paragraph end appends `\penalty10000\hskip\parfillskip\penalty-10000`
(TeX §816). In `line_break`, a penalty node is a legal breakpoint wherever it
is (§866/§829), and `break_width` (§837) subtracts the widths of the glue,
penalty and kern nodes that follow a break up to the next box, so the line from
the `\\` to the final penalty is *empty* with zero stretch: badness 10000,
infeasible in passes 1 and 2, accepted with artificial demerits (§854) because
the forced break leaves nothing else; `post_line_break` (§879) prunes the
discardables and packages an empty `\hbox`. That is the well-known
"Underfull \hbox (badness 10000) in paragraph" that pdflatex logs for a
trailing `\\`, and the empty line takes one `\baselineskip` on the page.

## The fix (`fix.patch`)

1. `measure`: `let start = start.min(brk);` — a break inside the discardable
   run after the previous break measures an empty line (natural width
   `left_skip + right_skip`, no stretch), exactly TeX's `break_width`.
2. `set_line`: the same clamp, so the slice is `brk..brk` and the `Line` has
   `runs == []`, `items == brk..brk`, `height == depth == 0`.
3. First-fit: `b = start.max(b + 1)` after a cut became
   `resume_after_cut(items, b, start)`, which returns the first forced penalty
   strictly inside `(at, start)` if there is one, else `start.max(at + 1)` as
   before. Non-forced penalties inside the discarded run are still skipped by
   first-fit (an empty line always "fits", so honouring them would invent
   lines TeX's total-fit would reject as badness 10000); a forced one is set as
   an empty line like total-fit and TeX.
4. A doc comment on `line_start` records the invariant.

`line_start` itself is unchanged; the clamp lives in the two consumers that
index with it, and total-fit's candidate loop needs nothing else because an
empty candidate line is now measured as TeX measures it (badness 10000 →
infeasible unless forced/final pass).

## Tests (`tests.patch`, `tests/forced_break.rs`)

| Test | Input | Expectation (hand-derived) |
|---|---|---|
| `trailing_line_break_yields_an_empty_last_line` | `Hello ` + `line_break()` + `finish` (7 items) | total-fit and first-fit: breaks 3, 6; line 0 `items 0..3`, natural 27.5, set 100, badness 0; line 1 `items 6..6`, no runs, natural 0, badness 10000; baselines 7 and 21.5; height 21.5; total-fit pass 2, `underfull == [(1, 10000)]`, no overfull |
| `mid_paragraph_line_break_is_unchanged` | `Hello \\ world` (8 items) | 2 lines, `world` at x 0, ranges 0..3 / 4..7 (passes before and after: the contrast case) |
| `paragraph_of_only_a_forced_break` | `line_break()` + `finish` (5 items); bare `[Pen(-10000)]`; empty list | 2 empty lines (0..1, 4..4) for total-fit, first-fit, ragged; 1 empty line; 1 empty line |
| `consecutive_forced_breaks_yield_consecutive_empty_lines` | `Hello` + two `line_break()` (8 items) | breaks 2, 4, 7; ranges 0..2, 4..4, 7..7 |
| `forced_break_followed_by_discardable_glue` | `aaaa`, forced, space, kern, space, `bbbb` (9 items); `[Pen(-10000) Glue Pen(-10000)]` | glue/kern discarded, `bbbb` at x 0, ranges 0..1 / 5..8; second: empty line 2..2 with natural 0 |
| `random_item_sequences_ending_in_forced_breaks_never_panic_or_drop_boxes` | 3,000 LCG-generated item sequences × 16 parameter sets (justified/ragged × total-fit/first-fit × 4 measures) | `check_invariants`: one line per break, breaks strictly increasing and ending at the final forced penalty, `items.end == break`, only discardables skipped between lines, every box placed exactly once; asserts > 1,000 empty lines and > 1,000 breaks-inside-discardable-runs were exercised |

The crate has no external dependencies (`proptest` is not available), so the
fuzz test is a fixed-seed 64-bit LCG (`0x5eed_f1a5_47e5_0001`) over words,
spaces, kerns (−3..3 pt), `line_break()`, and penalties from {−10000, +10000,
50, −50, 0, uniform(−10000..10000)}, with four tails: builder `finish`, bare
forced penalty, forced + `fil` + forced, and no tail (`layout_paragraph` appends
`\parfillskip` + forced itself). Every generated sequence is laid out 16 ways;
48,000 layouts per run, 0.06–0.3 s in release.

### Before (main crate + `tests.patch` only)

```
test mid_paragraph_line_break_is_unchanged ... ok
test paragraph_of_only_a_forced_break ... FAILED            slice index starts at 5 but ends at 4
test consecutive_forced_breaks_yield_consecutive_empty_lines ... FAILED   slice index starts at 8 but ends at 4
test forced_break_followed_by_discardable_glue ... FAILED   slice index starts at 3 but ends at 2
test trailing_line_break_yields_an_empty_last_line ... FAILED   slice index starts at 7 but ends at 6
test random_item_sequences_ending_in_forced_breaks_never_panic_or_drop_boxes ... FAILED   slice index starts at 16 but ends at 11
(all at src/linebreak.rs:882:21)
test result: FAILED. 1 passed; 5 failed
```

### After (main crate + both patches), `cargo test --release`

```
unittests src/lib.rs        9 passed
tests/forced_break.rs       6 passed
tests/golden.rs            16 passed
tests/oracle_wrap_sample.rs 2 passed
doc-tests                   0
```

33/33. `cargo clippy --release --all-targets -- -D warnings`: clean.
`cargo fmt --check`: clean. (README's "24 tests" line is stale on main already:
it has 27 before this patch.)

## Golden / oracle byte-identity check

The golden tests assert hand-derived numbers and the oracle test asserts
pdflatex line starts to 0.01 bp; both stay green, which is the primary
evidence. In addition a scratch harness (not shipped) dumped `{:?}` of every
`layout_paragraph` result for 48,576 cases before and after the fix, with
`catch_unwind` so the pre-fix run completes:

- 576 oracle-sample cases: the four `wrap-sample` paragraphs in Core 14
  Times Roman/Bold/Italic, with and without `\-` handling, at 4 measures
  (469.755 / 200 / 120 / 60 pt) × 6 parameter sets (justified, ragged,
  first-fit ×2, emergency stretch 20 pt, `pretolerance = −1`):
  **0 panics before, 0 changed** — byte-identical `Lines` structs.
- 48,000 fuzz cases (seed `0x0badf00d_cafe_0002`, a different seed from the
  committed test, kerns included): 20,969 panicked before, 0 after. Of the
  27,607 that did not panic before, 9,832 changed:
  - 9,828 are first-fit (`p2`/`p3`) and every one of them is exactly the
    insertion of empty lines at forced penalties that first-fit used to jump
    over (16,273 empty lines inserted; all non-empty lines identical in runs,
    widths, ratios, badness, hyphenation, item ranges; only `baseline_y`
    shifts by the inserted lines).
  - 4 are total-fit (`fuzz c317 w12 p4`, `c1141 w1 p1`, `c1141 w1 p4`,
    `c1193 w12 p4`): each has a legal non-forced penalty immediately before a
    forced one, so the candidate line between them was measured with
    `start > brk` (negative width, `INF_BAD`) and the path through it was
    discarded; after the fix it is an empty line with TeX's badness (0 in
    ragged mode, 22 with 20 pt emergency stretch on a 12 pt measure) and
    the lower-demerit path TeX would choose is taken. These inputs (a
    leading `\penalty-50` then a forced break, etc.) do not occur in the
    builder's output for real text.
- Every other non-panicking case (17,775 total-fit/first-fit cases with no
  break inside a discardable run) is byte-identical.

Dump hashes are in `evidence.json`; the harness is 100 lines and can be
re-created from its description if a reviewer wants to re-run it.

## Vendored copy — note for the render-pipeline owner

`crates/render-pipeline/vendor/paragraph-layout` on
`origin/agent/mac-render-pipeline/unified` is pinned (`PIN`) at `70209e2c`; its
`src/linebreak.rs` is byte-identical to the lane tip `0a1ba87d`
(sha256 `79786f90…8249`, 1,014 lines) and is *ahead* of main's copy (908 lines:
main lacks Liang hyphenation, document/style/runtime-v1 modules, incremental
relayout, page-metric goldens and the two-page oracle — main's only crate
commit not on the lane branch is `e69165a8`, per-glyph height/depth). Both
patches apply to the vendored copy with `patch -p3` (offsets only: the clamps
land at its lines 427 and 963, `resume_after_cut` at 746). With them applied
the vendored crate's full suite passes, 46/46: 10 unit, 6 new, 19 golden, 2
incremental, 1 `oracle_hyphen_sample`, 2 `oracle_pages` (668/668 line starts
and page assignment on both variants), 2 `oracle_wrap_sample`, 4
`page_metrics`; clippy `-D warnings` clean. The vendored copy's TeX-style
diagnostics will now print `Underfull \hbox (badness 10000) in paragraph` for
the empty line, as pdflatex does.

End-to-end check (evidence only; nothing in render-pipeline is changed by this
handoff): `flashtex-render` built with `cargo build --release --offline` from a
`git archive` of `crates/render-pipeline` at `9aaec57a` (the corpus lane's
binary revision), fed one `compile` request on stdin with `FLASHTEX_NO_ACTIVATE=1`:

| Source body | unpatched vendored copy | vendored copy + both patches |
|---|---|---|
| `Hello` (control) | exit 0, `Hello` at (148.712, 134.765) bp | identical |
| `Hello \\` + blank line | **exit 101**, `panicked at vendor/paragraph-layout/src/linebreak.rs:988:21: slice index starts at 6 but ends at 5` | exit 0, status `ok`, 1 page, `Hello` at (148.712, 134.765) — same as control |
| `Hello \\ world` | exit 0 | exit 0, `Hello` (148.712, 134.765), `world` (133.768, 146.72) |
| `Hello \\` + blank + `Next` | exit 101 | exit 0, `Next` at (148.712, **158.675**) vs 146.72 in the `Hello` + `Next` control: one extra line pitch (11.955 bp = 12 pt), which is the blank line pdflatex sets |
| `Hello \\\\` + blank line | exit 101 | exit 0, `Hello` at the control position |

The helper emits no diagnostic for the underfull empty line (its diagnostics
policy is the pipeline owner's; the vendored crate's own `Lines.diagnostics`
does carry the `Underfull \hbox (badness 10000)` entry).

The render-pipeline call-site guard (not panicking the whole helper on a
layout error) belongs to the text-gaps lane's follow-up 1 and is not part of
this handoff. Consumers of `Lines` should expect lines with `runs.is_empty()`
and `items.start == items.end` (render-pipeline's `assemble` loop already
handles this: it pairs boxes in `line.items` with `line.runs` and emits an
empty display line; `label_pages` falls back to the last line for a label
inside a discarded run).

## Follow-up 1 — oracle cases that still differ (findings only)

Source: `docs/comparison.md` on the lane tip / vendored copy (main's copy has
only the wrap-sample section) plus the `0a1ba87d` commit message. pdflatex was
not run in this lane.

| Oracle case | Metric | Status on lane tip | Remaining difference |
|---|---|---|---|
| `wrap-sample` C-times12-1in-ragged (102 words) | line starts | 102/102 | none |
| | horizontal | max\|dx\| 0.002 bp | PDF 3-decimal rounding only |
| | vertical | max\|dy\| 0.132 bp | not a baseline difference: PDFKit glyph-box descent of the embedded URW **bold** (≈0.209 em) vs AFM 0.205 em; roman rows 0.013 bp |
| `hyphen-sample` (92 words, justified, hyphenation) | line starts / hyphens | 92/92, 5/5 | none |
| | horizontal | max\|dx\| 0.007 bp | pdfTeX places glyphs at integer 1/1000 em inside `TJ` (0.012 bp at 12 pt) |
| `pages-default` (668 words, 2 pages) | line starts / page assignment | 668/668 | none |
| | overfull lines | 4 / 4 reproduced | excess 1.041 pt vs TeX's 1.03694 pt: 0.004 pt AFM-vs-TFM width rounding |
| | glue set (592 gaps) | mean 0.0034 bp, max 0.0110 bp | 8/592 gaps beyond 0.01 bp, all ≤ 1/1000 em = the TJ floor |
| `pages-geometry1in` | glue set (604 gaps) | mean 0.0039 bp, max 0.0115 bp | 8/604 beyond 0.01 bp, same floor |
| demerit-parameter goldens (4 cases, `\linepenalty`, `\adjdemerits`, `\doublehyphendemerits`, `\finalhyphendemerits`) | chosen breaks | hand-enumerated | **not oracle-confirmed** — awaits a pdflatex regeneration (stated in `0a1ba87d`) |
| trailing `\\` (this handoff) | line count | panicked | fixed at the crate level; **not oracle-confirmed** — the expected two-line/underfull result is from TeX §837/§879 and the LaTeX `\\` definition |
| `ff`/`ffi`/`ffl` | width | not exercised | T1 Times composites vs `f`+`f` with the AFM kern: `ffi`/`ffl` would differ by 25 units |
| preview vs PDF right edges | max 0.0277 pt | within 0.05 pt gate | Adobe base-14 vs macOS Times advances inside a word, not a layout difference |

The smallest layout-level difference is the one this handoff fixes (a wrong
line count / crash on `\\` at paragraph end); its pinned before/after is the
test output above. Every other row is either at the oracle format's floor
(1/1000 em), a measurement artifact, or awaiting an oracle run. Two known
builder deviations from LaTeX's `\\` that no oracle sample exercises and that
are **not** changed here: `line_break()` does not `\unskip` the preceding
interword glue (a trailing space before `\\` stays on the line, invisible
under `\hfil`) and does not insert the `\nobreak` before `\hfil` (the glue
after a box is a legal break in the crate, always infeasible next to the
forced break because its stretch is excluded from the line).

## Follow-up 2 — migration note for the compiler owner (Kabir, FT-002)

The adoption the compiler owner made and then reverted (`8b348a8` → `ee423c3`
on `origin/agent/claude/compiler-foundation`, issue #27) and the
render-pipeline branch both consume the same public surface:
`ParagraphBuilder::{new, text, word, space, glue, penalty, kern, line_break,
finish, items}`, `Item::{Box, Glue, Penalty, Kern}`, `Item::penalty`,
`Glue::{fil, fixed, finite}`, `FORCED_BREAK`, `INFINITE_PENALTY`,
`GlyphRun::from_shaped`, `ShapedGlyph`, `FontId`, `FontMetricsSource`,
`LineBreakParams`, `Algorithm`, `BreakMode`, `layout_paragraph`, `Lines`,
`Line`, `PositionedRun`, `PositionedGlyph`, `Stats`, `ParagraphBlock`,
`layout_pages`, `Pages`, `Page`, `PlacedLine`, `PageOverflow`.

**No signature, type, field or re-export changes.** Behavioural changes only:

1. A paragraph whose item list ends in a forced break before the paragraph-end
   tail (`\\` then blank line, or a raw `Penalty(-10000)` followed by
   glue/penalties) no longer panics; it yields **one more `Line`** with
   `runs == []`, `items == brk..brk`, `height == depth == 0.0`,
   `natural_width == left_skip + right_skip`, `badness == 10000` in justified
   mode (0 in ragged mode, where the `fil` right skip absorbs the shortfall),
   listed in `stats.underfull` for total-fit, and `stats.pass == 2` for
   ordinary text (both tolerances reject it; the final pass accepts it with
   artificial demerits). Page layout gives it one `\baselineskip`.
2. Consecutive forced breaks yield consecutive empty lines.
3. First-fit now sets those empty lines too (it used to skip the forced break
   and return fewer lines than breaks the input contained).
4. `Line.items` may be empty; do not assume `runs.len() >= 1` or
   `items.start < items.end`. Item-to-line lookups for an item inside a
   discarded run find no line (as before for glue between lines).
5. The compiler's own fuzz tests
   (`no_generated_source_panics_the_parser_or_layout`,
   `no_generated_source_panics_incremental_reuse`) that failed against the
   `8b348a8` adoption were, by the commit's description, hitting this panic
   class; they should be re-run against the patched crate before the
   adoption is re-proposed. The byte-exact fixtures that changed under that
   adoption changed because line breaking moved from the placeholder to
   Knuth–Plass, not because of this fix — this fix leaves every layout that
   did not previously panic or silently mis-measure unchanged.

If the compiler maps `\\` to `line_break()`, source documents with a trailing
`\\` (tabular rows typeset as text in `cv` and `article-twocolumn`) will now
render with an extra blank line per row end, which is what pdflatex does for
that source.

## How this was verified (commands)

```
git archive origin/main crates/paragraph-layout | tar -x -C .scratch      # tree d20d55ce
cd .scratch/crates/paragraph-layout && cargo test --release                 # 27 passed (baseline)
# + tests.patch:  cargo test --release --test forced_break                  # 1 passed, 5 failed (panic)
# + fix.patch:    cargo test --release                                      # 33 passed
cargo clippy --release --all-targets -- -D warnings                         # clean
cargo fmt --check                                                           # clean
# vendored copy (origin/agent/mac-render-pipeline/unified) + both patches:
cargo test --release                                                        # 46 passed; clippy clean
```

Machine: mac-m1max-a, `cargo 1.99.0-nightly (3efb1f477 2026-07-17)`, 1-min
load 26–61 during the run (no timing tests involved).

## Limitations

- The end-to-end check above covers the minimal reproduction only; the full
  corpus fixtures (`cv`, `article-twocolumn`, tabular rows ending in `\\`)
  were not re-run through the harness (render-pipeline is not this lane's
  crate and its owner re-pins the vendored copy after review).
- No pdflatex run in this lane: the TeX behaviour cited is from the TeX
  program sections named above and the LaTeX kernel's `\\`; the existing
  oracle tests are the regression evidence.
