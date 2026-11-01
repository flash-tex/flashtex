# Replace the pipeline TFM reader

Reviewed original pipeline `9bb7b27:crates/render-pipeline/src/tfm.rs`. All table
access is already available; new `Tfm::glyph_run` supplies the consumer's attached
kern representation without reimplementing parsing or ligature execution.

| Pipeline API | Shared replacement |
| --- | --- |
| `Tfm::load(path)` | Read through the caller's rooted asset loader, verify SHA, then `Tfm::parse(&bytes)` |
| `design_size_pt` | `design_size.0` in exact fix_word points (denominator 1048576) |
| `metrics(code)` | `char_metrics(code)`; each metric's `.0` is the signed integer |
| `param(n)` | `parameter(n).map(|v| v.0)`; one-based |
| `pair(l,r)` | `pair_action(l,r)?`; rename replacement/retain_left/retain_right/advance fields |
| `has_boundary()` | `boundary_character.is_some() || left_boundary_program.is_some()` |
| `ligkern(codes)` | `glyph_run(codes, BoundaryOptions::default())?` |

`GlyphRun` retains `tfm_sha256`, `leading_kern: FixWord` and `glyphs` containing
`code`, `input_start`, `input_end`, `kern_after: FixWord`. The consumer must advance
by `leading_kern` before the first glyph, including a glyphless run. Kern addition
is checked. Errors propagate; do not turn budget or malformed-program errors into
empty/partial successful output. Same-font input is bounded to 4096 codes. Do not
split a long run arbitrarily: doing so changes ligatures and boundary semantics.
Explicit `BoundaryOptions {left:false,right:false}` suppresses both boundaries
only when required by the actual caller semantics.

Keep TFM code-to-original-GID mapping through the declared encoding separate from
metrics. Keep exact integers through layout; `FixWord::at_design_size` returns an
exact product with denominator 2^40. Arbitrary rational font sizes require explicit
consumer scaling/rounding. This adapter does not mandate the pipeline's prior f64
conversion or make a TeX scaled-point rounding claim.

`tools/replay_pipeline_tfm.py` retrieves the exact hashed peer source from Git,
compiles it in a temporary directory and checks its outputs against the committed
artifact. With rustc on PATH, run it from any directory. Then run
`cargo test --offline --manifest-path crates/font-resources/Cargo.toml --test tfm_consumer`.
The shared adapter matches all present ec-lmr10 metric fields, parameters1..32 and
ten runs including `office`, five ligatures and signed kern pairs. Provenance is
in `fixtures/pipeline-tfm-replay.json`. rm-lmr fixtures were not found locally and
are not claimed equivalent. The enclosing crate still has serde/sha2 dependencies;
this is separate from parser compatibility. No pipeline files or CM/MATH glyph
mapping were modified.

## Required metric failure contract and 12pt evidence

`required_tfm::RequiredMetrics::load(root, &Manifest)` loads at most16 explicitly
named assets atomically (128KiB/TFM,16KiB/license). The version1 typed manifest
example is `fixtures/lm-required-metrics.json`. Each path and license is rooted and
SHA-bound; duplicate declarations, invalid/noncanonical paths, absent files,
changed bytes and malformed TFMs fail. `get` also refuses an undeclared name.
This is a required-metrics mode: propagate its typed `Missing`/`Digest`/`Read`/
`Parse` result as a blocking layout diagnostic. Do not convert it into `tfm: None`
and then use OTF widths. A separate explicitly selected OTF layout mode is outside
this contract; resource failure never implicitly selects it. The immutable result
retains each exact asset declaration; file changes require reloading/revalidation.

The official `lm2.004bas.zip` supplied ec-lmr12 and rm-lmr12/8/6; all four were
actually replayed against the pinned pipeline reader, covering every present
metric, parameters1..32 and ten ligature/kern inputs. Its ec-lmr10 SHA matches our
existing fixture. The current official2.007 release has different metric hashes
and is explicitly excluded from this replay. Archive/file/license/output hashes
and the temporary directory are recorded in `lm-required-metrics-provenance.json`.
No metric or font bytes were installed or copied into this repository.

Run the peer reproduction tool and shared test with
`FLASHTEX_LM_TFM_DIR=/path/to/extracted2.004metrics` (including the declared LICENSE):

```
python3 crates/font-resources/tools/replay_pipeline_tfm.py
cargo test --offline --manifest-path crates/font-resources/Cargo.toml --test required_tfm -- --include-ignored
```

Set that environment variable for both commands. This verifies metrics and encoded
code intervals; rm-lmr code-to-MATH-GID mapping and visual parity remain separate.
