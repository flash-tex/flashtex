# Vendor re-pin runbook: daniel-parent's queue (GH-REPIN-READINESS)

This is for the integration lane (GoKubar / kabir-claude). It re-pins `crates/render-pipeline/vendor/` past
daniel-parent's queued compiler, pdf, math-layout, tex-expansion and project-files PRs. It also lands the five
drafts that were waiting on that re-pin: #569, #584, #599, #582 and #585.

Branch `agent/daniel-parent/repin-readiness-2` holds the pipeline side, already merged and fixed. Everything below
was run in a scratch copy of that branch, and none of the scratch vendor/ changes were committed. The copy used
vendor crates from `c47e1452`, `rustc 1.98.1`, `CARGO_BUILD_JOBS=4` and the release profile.

## 0. Preconditions

1. Merge the preview queue in the order given in `docs/integration/daniel-parent-preview.md`.
2. Then merge the drafts in this order: #569 → #584 → #599, then #582, then #585.
   - You can merge `agent/daniel-parent/repin-readiness-2` instead; it holds the drafts plus the conflict
     resolutions recorded in the preview notes.
   - #585 carries compiler commit `1b6f03f2`, which is in no other PR.
3. Also land `58118db5`, the one post-merge pipeline fix. It is on the readiness branch: `Inline::HSpace { pt, span, .. }` in `adapter.rs`.
4. Do **not** land the drafts ahead of the vendor copy in a separate CI run.
   - #569/#584/#599's page-control code is not feature-gated.
   - Against today's committed vendor/, `cargo check --release --locked --tests` fails with 23 errors, all of them the new compiler API.
   - Land the drafts and steps 1–4 below as one change.

## 1. Copy the vendor crates

Pick `PIN`, the main commit that contains the whole queue, #585's `1b6f03f2` included. Before copying, confirm that
the six crates carry nothing unexpected since the validated revision:

```sh
cd <repo checkout at PIN>
PIN=$(git rev-parse HEAD)
git merge-base --is-ancestor 1b6f03f2 "$PIN" && echo "has #585's compiler commit"
git diff --stat c47e1452 "$PIN" -- crates/compiler crates/pdf crates/math-layout crates/tex-expansion crates/project-files crates/bibliography
```

Differences since `c47e1452` are fine if they come from later merged PRs, but they are not covered by the results
below. The one known difference is `crates/compiler/tests/amsmath_corpus/oracle.py`, from the preview's `da03ac94`.
It is oracle tooling and is not compiled.

Copy the six crates. Five replace existing mirrors; `bibliography` is a **new** mirror directory:

```sh
V=crates/render-pipeline/vendor
for c in compiler pdf math-layout tex-expansion project-files bibliography; do
  rm -rf "$V/$c"
  mkdir -p "$V/$c"
  git archive "$PIN" "crates/$c" | tar -x -C "$V/$c" --strip-components=2
  echo "$PIN" > "$V/$c/PIN"
done
for c in compiler pdf math-layout tex-expansion project-files bibliography; do
  diff -r --exclude=PIN "crates/$c" "$V/$c" && echo "$c byte-identical"
done
```

Leave these mirrors unchanged: `font-engine`, `document-style`, `tex-text-encoding`, `tex-boxes`, `paragraph-layout`, `microtype`,
`font-resources` and `vector-graphics`.
- The queued compiler builds against them as they are.
- `font-engine` and `document-style` still differ from `crates/` by their own rows' deliberate choices, as before.

## 2. Cargo.toml and Cargo.lock

**No `[dependencies]` line is added for bibliography.** The pipeline does not use `flashtex-bibliography` directly. Its
use is `vendor/compiler/Cargo.toml`'s `flashtex-bibliography = { path = "../bibliography" }` (#558, `5ba97777`), which
resolves once the `vendor/bibliography` sibling from step 1 exists. Without that directory, Cargo cannot load the
`../bibliography` path dependency and the build fails before compiling anything.

Both lockfiles change, because both `crates/render-pipeline` and `crates/flashtex-cli` (which paths into
`../render-pipeline/vendor/{compiler,pdf,project-files}`) resolve these mirrors. CI builds with `--locked`, so regenerate and commit both:

```sh
(cd crates/render-pipeline && cargo build --release)   # no --locked, once
(cd crates/flashtex-cli && cargo build --release)
```

Each lockfile gains the same three packages: `Locking 3 packages to latest compatible versions`, then
`Adding flashtex-bibliography v0.1.0`, `Adding tinyvec v1.13.3` and `Adding unicode-normalization v0.1.25`. The resulting lockfile changes:
- `flashtex-compiler` gains the dependency `flashtex-bibliography`.
- `flashtex-project-files` gains `libc` and `unicode-normalization`, from #518/#539. These are new registry crates for this build.

## 3. Features to flip in `crates/render-pipeline/Cargo.toml`

**Required.** Without these, 7 compile errors remain: 1 for `LineBreak.skip_pt` and 6 for the `Nucleus::{TextRun,SideSet}` matches. The 8th error is the `HSpace` pattern that `58118db5` fixes.

```toml
default = ["amsmath-inline", "request-date", "compiler-package-gating", "par-leading", "math-class-override", "linebreak-skip", "amsmath-sideset", "compiler-text-run"]
```

| Feature | From | Needs | Without it |
|---|---|---|---|
| `linebreak-skip` | #479 | compiler `Inline::LineBreak::skip_pt` | `Inline::LineBreak { span }` initialiser misses `skip_pt` (1 error); cv's `\\[-6pt]` in `\cvsection` stays +5.98 bp |
| `amsmath-sideset` | #582 | compiler `Nucleus::SideSet`, math-layout `Atom::left_scripts` | the 6 exhaustive `Nucleus` matches (`hash_math`, `shift_math`, `convert_math_classed`, `math_grids`, `math_glue_em`, `math_approximations`) fail to compile; shared with `compiler-text-run` |
| `compiler-text-run` | #585 | compiler `Nucleus::TextRun`/`TextPiece` (#470), math-layout `TextRun` | same 6 matches; display-placement 37–48 are not checked |

Update each flipped feature's comment the way `amsmath-inline`'s was: its precondition is met as of this re-pin.

**Optional.** These build against the re-pinned vendor, but their effect on the full suite was not measured:
- `math-font-kerns` (#560/#589): `cargo test --release --features math-font-kerns --test math_font_kerns` gives `test result: ok. 1 passed`. It changes math spacing corpus-wide, so run the amsmath/amssymb/real-world oracles before making it a default.
- `math-glyph-spans`: `cargo check --release --features math-glyph-spans` gives `Finished`.
- `compiler-text-nucleus` has no `cfg` users left in the crate. It is dead and can be deleted.

Un-ignore `tests/math_symbols.rs::coloneqq_decomposes_like_mathtools`. Its `#[ignore = "needs re-pin past #529 ..."]`
precondition is met: `cargo test --release --test math_symbols -- --ignored coloneqq_decomposes_like_mathtools` gives
`test result: ok. 1 passed; 0 failed`.

## 4. Update `vendor/VENDORING.md`

- Point the `compiler`, `pdf`, `math-layout`, `tex-expansion` and `project-files` rows at `PIN`. Move each old text into the row's "Previously" notes.
- The `compiler` row's "Temporary pin to an unmerged branch" wording goes, because `PIN` is on main.
- Add a `bibliography` row: branch `main`, commit `PIN`, owner the compiler lead. Its cause, like `tex-text-encoding`'s: `crates/compiler` depends on `../bibliography` since #558.
- Record the step 5 results in the rows.

## 5. Expected results

Compile errors in `crates/render-pipeline/src`, against the re-pinned vendor:

| State | Errors |
|---|---|
| preview src (`5d1eb06c`), default features | 13 (`skip_pt` ×2, `HSpace` fields, `Block::Penalty` ×2, `Inline::{Penalty,PagePenalty,Discretionary}` ×2, `Nucleus::{TextRun,SideSet}` ×6) |
| + #569/#584/#599/#582/#585 merged, default features | 8 |
| + the three features flipped | 1 (`Inline::HSpace` fields) |
| + `58118db5` | **0**; `cargo test --release --no-run` builds every test target |

Tests in the scratch re-pin:

- `crates/render-pipeline`, `cargo test --release --no-fail-fast`: every binary `ok`, totalling **466 passed, 0 failed, 3 ignored**.
  - The preview gave 439 passed / 1 failed / 3 ignored before #597's fix.
  - The 3 ignored:
    - `adapter::tests::preamble_scan_does_not_see_input_files` (known unimplemented);
    - `incremental_output_is_byte_identical_over_30_edits_of_a_107_page_document` (slow);
    - `coloneqq_decomposes_like_mathtools`, which passes once un-ignored (step 3).
  - Oracle binaries:
    - `page_para_control`: `test result: ok. 19 passed; 0 failed`
    - `display_placement`: `test result: ok. 1 passed` (37–48 are checked with `compiler-text-run`)
    - `toc_oracle`: `test result: ok. 1 passed`
    - `tabular_oracle`: `test result: ok. 1 passed`
    - `amsmath_inline`: `test result: ok. 11 passed`
    - `amsmath_sideset`: `test result: ok. 3 passed`
- `crates/flashtex-cli`, `cargo test --release --no-fail-fast`: `31 passed; 0 failed` and `26 passed; 0 failed; 1 ignored`.
- The oracle scripts, run with the scratch `flashtex-render` and `--fonts apps/mac/Fonts`:
  - `python3 crates/compiler/tests/amsmath_corpus/oracle.py check`: `TOTAL 59/59 within 0.5 bp`.
  - `python3 crates/compiler/tests/amssymb_corpus/oracle.py check`: `TOTAL 38/39 within 0.5 bp`.
  - `python3 crates/compiler/tests/tabular_corpus/oracle.py check`: `TOTAL 128/128 (words 0.5 bp, rules 0.1 bp)`.
  - `python3 crates/render-pipeline/fixtures/display-placement/oracle.py check`: `TOTAL 47/48 within 0.5 bp`. This was 35/36 on the preview; the 12 new fixtures, 37–48, all pass.

## 6. Known remaining failures

| Failure | Since | Owner |
|---|---|---|
| display-placement `15-align-tag-notag-eqref` (23 words ok, 2 unaligned): `\tag{$*$}` is placed exactly, but it extracts as `*` where pdfTeX's cmsy glyph reads `∗` | same on main and on the preview | display-placement lane (daniel-parent, #441 follow-up) |
| amssymb `32-braces-narrow` (worst 1.012 bp) | same on main and on the preview | unassigned; amssymb braces area (math-layout `Atom::brace`, #127) |
| compiler debug stack margin: 32 nested tabulars use about 2.0 MiB of a 2 MiB debug test thread (the test passes; release is unaffected) | preview | daniel-parent, GH-PARSER-STACK-FRAME |
| `math-font-kerns` not yet measured corpus-wide | — | math-layout lane (#560/#589) |

Real-world corpus page counts and `rank.py` geometry were **not** re-run on the scratch re-pin. Run them after the
re-pin lands and compare against the sweep-2 numbers in the preview notes. `cv` should improve, because
`linebreak-skip` removes the +5.98 bp per `\cvsection`.
