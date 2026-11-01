# `\text{...}` in math — the six HW1 probes before/after

Replay of the renderer's acceptance packet (main f261b36c,
`crates/rendering-core/docs/handoffs/hw1-text-producer/actual-candidate/`,
owner packet `owner-acceptance.md` on main a5e7fa7c) on mac-m1max-a
(rustc 1.99.0-nightly 1a833e165, `cargo build --release --offline`).

Both binaries are scratch builds of this crate with the **isolated compiler
candidate** applied to `vendor/compiler` — `candidate.patch` (sha256
`af8fd219…`, = source-audit `compiler_patch_sha256`) then `comment-fix.patch`
(`51dbbd0d…`, = `comment_patch_sha256`) — because neither `main` nor the
vendored compiler carries `Nucleus::Text`. Nothing under `vendor/` is changed
on the branch; the compiler adoption/repin is a cross-owner request (see
`coordination/mac-render-text-gaps.md`).

- `before/`: producer base 9aaec57a + the renderer's three-arm `adapter.patch`
  (incremental.rs hash/shift arms, typeset.rs `Nucleus::Text` → math-layout
  per-character `text_glyph`). This is the evidence binary's configuration.
- `after/`: this branch (`mathtext.rs`, `typeset.rs`) built with
  `--features compiler-text-nucleus` + only the incremental.rs half of
  `adapter.patch` (the typeset arm is the feature-gated one in the crate).

Resources: the same isolated flat stage as the evidence — `stage-sha256.txt`
lists the nine files; every hash equals `evidence.json`'s `resource_hashes`
(lmroman12/10-regular.otf, latinmodern-math.otf, rm-lmr12/8/6, ec-lmr12/10,
GUST-FONT-LICENSE.TXT), copied from MacTeX 2026 texmf-dist. `replay.py`
sends the unchanged `request.jsonl` of each case and writes `--v2`.

## Before (base + adapter patch) vs the evidence

| case | v2 identical to evidence | note |
| --- | --- | --- |
| comment | yes | |
| escaped | yes | `{`/`}` placed by rm-lmr12 OT1 slots 123/125 |
| five | yes | |
| ligature | yes | `ffi` = gids 55, 55, 66 |
| scripts | no — environment only | host MacTeX supplies `lmroman8-regular.otf`, which the Linux stage lacked: `2` is drawn from LMRoman8 (gid 107) instead of latinmodern-math (gid 19) and the `lmr8` resource-profile warning is absent; the `and` run and `lmmi8` warning are identical. Font discovery always falls through to the host TeX directories, there is no switch to hide them. |
| space | yes | `b` at 144198207 = `a` + rm-lmr12 slot 32 |

## After (this branch)

| case | change | ticks (2^20 per bp; 1 TeX pt = 1044480.7 ticks) |
| --- | --- | --- |
| comment | none | `and` = gids 28, 77, 47 at 134651073 / 140788438 / 147607965 (identical) |
| five | none | all 19 glyphs identical: labels `(a)…(d)`, `and` keep their Roman12 gids and positions; Ord/Bin spacing around `+` unchanged |
| scripts | none beyond the environment note above | `and` identical; `lmmi8` warning retained |
| space | interword glue = `\fontdimen2` of ec-lmr12 | `b` at 144879963 (was 144198207): `b − a` = 10228890 ticks = 9.7917 pt = TFM width of `a` (0.4896 em = 5.875 pt) + `\fontdimen2` (342239/2^20 em × 12 = 3.9166 pt, the evidence README's SPACE fontdimen); the run is split into `a` and `b` items like paragraph words, no space glyph. (`advance_x` 6142592 is the OTF hmtx advance the display list reports per glyph, not the TFM width.) |
| escaped | T1 brace slots | `x` at 140919024 (was 140788438), `}` at 147397482 (was 147266896): `{` advances by its own T1 width 6267951 = its OTF advance, not the OT1 slot-123 width 6137365 |
| ligature | TFM ligature program | one glyph, gid 123 (= `cmap` U+FB03 in LMRoman12-Regular), advance 10229296, cluster text `ffi` (text_start 0, text_end 3) — was gids 55, 55, 66 |

`after/summary.json` and `before/summary.json` hold the per-case runs, the
diagnostics and the binaries' sha256. No diagnostics are added or removed by
the change in any case (`escaped`, `ligature`, `space`: none; `scripts`:
`math_resource_profile lmmi8`).

This is bounded transport/GID/geometry evidence against TeX's own metrics; it
is not a raster comparison with pdflatex (the oracle harness on this branch
has no `\text` fixture yet).
