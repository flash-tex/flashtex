# Original pipeline versus established LaTeX

This is a real non-equivalence measurement, not a passing parity fixture.
`manifest.json` pins every input/output, peer commit, and known resource gap.
`reference.pdf` was produced by pdfTeX 1.40.29 (TeX Live 2026), with the exact
engine preamble and SHA in `reference-engine.json`. It comes unchanged from
`tests/visual-corpus/evidence/20260912T064032Z/references/01-plain-paragraph/pdflatex-lm`
on the pinned visual-oracle branch. This is the 06:40 run, not the later Mac
09:27 run or its reported position accuracy.

`original.pdf` was freshly compiled on Linux with the unchanged original
render-pipeline 9bb7b2736f16c96f4ce356fbb89caf9de395402c. The original request
strips the fixture preamble; the reference substitutes the recorded LM preamble.
Both transformations are explicit, so this is not identical full-source input.
The pipeline's `--pdf` uses the exact route (`pdf::write_pdf_exact`), not rendering-core's
exact path exporter. `original-v2.json` separately preserves original GIDs,
source bytes and full LMRoman12 resource identity.

The official existing LM 2.004 OTF archive provided LMRoman12-Regular. Matching
`ec-lmr12.tfm` was absent. Pipeline `fonts.rs` records `tfm_missing`, but no caller
reads it and diagnostics remain empty: OTF metrics were used. This resource
configuration cannot inherit the Mac's matched-TFM accuracy claim. Reported in
https://github.com/flash-tex/flashtex/issues/2#issuecomment-5645061659.

The exact experimental consumer accepts the JSON shape but semantic validation
rejects its `opentype-cff` resource with `unsupported font profile`. Internal CFF
paths exist; this does not authorize widening the negotiated wire profile.
The regression exercises the actual producer bytes and refusal, with no invented
GIDs or silent font substitution.

The existing original PDF reader compares 66 candidate operators against 5
reference operators. Raw bytes and parsed operators differ. Operator-to-source
correspondence is unknown because sequences differ. Numeric placement equality
and raster equality are unmeasured; there is no visual parity claim.

Reproduce the consumer checks from the repository root:

```
cargo test --offline --manifest-path crates/rendering-core/Cargo.toml --test original_reference
cargo run --offline --manifest-path crates/rendering-core/Cargo.toml --example pdf_compare -- crates/rendering-core/tests/fixtures/original-reference/original.pdf crates/rendering-core/tests/fixtures/original-reference/reference.pdf /tmp/original-reference-report.json
```

The second command writes the report and returns 3 for differing bytes. To
regenerate the original output, extract `git archive 9bb7b27 crates/render-pipeline`
into an isolated directory (its pinned vendor crates avoid dependency collision),
build its `flashtex-render` binary offline, then run:

```
FLASHTEX_TFM_DIRS=/path/to/only-ec-lmr10-fixture-directory flashtex-render --font-dir /path/to/extracted-lm2.004otf --secnumdepth 0 --v2 original-v2.json --pdf original.pdf < request.jsonl
```

Keep the missing 12pt TFM condition explicit. Installing the proper TFM changes
the resource configuration and requires a new evidence manifest. Never use the
reference PDF as an input to the original renderer.

## Opt-in CFF consumer and matched metrics rerun

`pipeline_cff::PipelineCff` now offers an explicit additive path: parse the
existing typed display, validate its CFF profile, bind immutable registry
resources and UTF-8 source snapshots, then expand original GIDs with exact
FontMatrix/size/origin, caller-selected hint policy and bounded command counts.
It retains producer advances separately in `display()`; CFF outline advances
never replace supplied glyph origins. Font IDs may differ from raw resource
hashes. The legacy wire validator still rejects CFF. This API is opt-in and does
not negotiate a new protocol or select implicit fonts.

The stronger check exposed a producer defect: the previously declared font SHA
`d0f39b...` is actually SHA256(font bytes + four face-index zero bytes).
The supplied, unmodified LM2.004 font has raw SHA
`e6be218ae83e61aa8a29990d3cdc401c678c1962188cb9a4a8b6359e4f5e5870`.
Thus earlier statements of full-font identity describe the producer's claim,
not successful verification. The adapter refuses `CFF resource metadata mismatch`.
Producer fix requested at
https://github.com/flash-tex/flashtex/issues/2#issuecomment-5645097175.

`matched-v2.json` and `matched-legacy.pdf` are fresh unchanged pipeline outputs
using official LM2.004 `ec-lmr12.tfm` SHA299021120f0a29ef61278a2363903bd8defbb8faaade458eb79067342aecb56f.
The reference is unchanged. The legacy PDF still differs in bytes/operators;
visual and cross-producer source correspondence remain unknown. Exact export
refuses the mislabeled hash even with correct TFM metrics.

Run the actual refusal from repository root:

```
cargo run --offline --manifest-path crates/rendering-core/Cargo.toml --example pipeline_cff_probe -- crates/rendering-core/tests/fixtures/original-reference/matched-v2.json crates/rendering-core/tests/fixtures/original-reference/request.jsonl crates/rendering-core/tests/fixtures/original-reference/lmroman12-regular.otf crates/rendering-core/tests/fixtures/original-reference/GUST-FONT-LICENSE.txt /tmp/matched-exact
```

The font fixture is unmodified official LM2.004, redistributed with its adjacent
GUST license. The automated hypothetical corrected-contract test changes only
the hash in memory to exercise successful adapter geometry and atomic budget
refusals. That edited fixture is never labeled original output or used to claim
original-versus-reference equality. No producer metadata is silently repaired.

## Current producer 4888a67 framing replay

The three `4888-*.jsonl` fixtures are fresh unchanged producer stdout:
matched metrics accepts a v2 sibling, missing metrics emits `tfm_missing` with
recovered status, and an 8000-byte reply cap explicitly declines the v2 sibling.
`pipeline_frame::pair` binds the optional line to the accepted capability,
request ID, project and revision, rejects unsolicited/missing/oversized siblings,
and optionally refuses missing TeX metrics for reference acceptance. It is
transport validation only, never permission to paint or proof of font identity.

The new producer still mislabels the engine hash as the raw font hash; running
the CFF probe on its actual sibling still returns resource metadata mismatch.
No current exact original PDF can therefore be produced through this adapter.
The fresh `4888-legacy.pdf` and `4888-comparison.json` rerun the legacy route
against the same reference: bytes/operators differ, visual equality unknown.
Earlier producer comparisons remain separately pinned. Current blocker
verification: https://github.com/flash-tex/flashtex/issues/2#issuecomment-5645134718.

## Existing PDF owner subsetter integration

`PipelineCff::export_searchable(max_pdf_bytes)` consumes the unchanged PDF owner's
654f626 `v2::from_v2` and `exact::render_exact`. Verified immutable registry bytes
are staged in a private temporary directory; the returned resolver paths must
match that staging and `HashForm::Bytes`. The raw-hash refusal occurs at binding,
before this API. It does not permit the owner's compatibility interpretation of
an engine hash as a raw hash. No second subsetter, PDF writer or font parser was
added. The temporary paths in its report are audit evidence, not durable assets.

The narrow accepted extraction profile has exactly one glyph per nonempty
cluster and one text value per original GID within each font. Ligatures such as
`fi` are retained as multi-character ToUnicode mappings. Empty/multiple-glyph
clusters or conflicting mappings return explicit errors because the owner API
does not implement marked-content ActualText. Text extraction support is not
proof of global reading order or established-LaTeX visual parity.

The adapter caps 256 pages,100000 glyphs,64MiB staged font bytes and a caller PDF
output cap up to64MiB. The PDF output cap is checked after owner serialization;
it is not a hard ceiling on the owner's transient allocation. Existing opaque
paint and exact numeric refusals remain in the owner implementation. The owned
regression verifies the original-GID CID-CFF subset and `H`/`fi` ToUnicode with
the explicitly hypothetical corrected contract, and rejects ambiguous mappings.
Actual producer4888a67 remains refused until it publishes its raw SHA correctly.

PDF classifier fix654f626 is merged unchanged. The existing independent
unsupported-identical guard and all seven PDF comparison tests continue to pass.

## Historical65dbe7d run: raw hash verified, required metrics unavailable

Correction: the first run below used the old FLASHTEX_TFM_DIRS layout and
contained an error diagnostic. Its211-pixel result is fallback-metrics evidence,
not matched-metrics acceptance. The corrected zero-diagnostic run is documented
next; searchable export now refuses error diagnostics.

The owner published the raw digest correction in919ad8b, followed by65dbe7d.
An untouched archived build of65dbe7d now emits valid raw font identities and
passes PipelineCff with the actual font/source snapshots. No producer output was
rewritten. The regression recreates `65dbe7d-searchable.pdf` byte-for-byte through
the existing owner subsetter/writer. Earlier refusal and scratch-candidate records
remain historical evidence, not current blockers.

`65dbe7d-measurement.json` pins the published commit, resources and every output.
The same request, recorded preamble transformation and established pdfTeX reference
are used. Poppler26.01.0 extracts identical UTF-8 text from the original PDF and
reference. At144DPI both rasterizations are1224×1584 RGB and211 pixels differ.
Thus text extraction equality is observed, visual equality is false at that
configuration, and raw bytes/parsed operators differ. No threshold was relaxed.

Poppler reports13 equal word strings in order. Maximum xMin/xMax box differences
are0.064160/0.064083bp. yMin/yMax differ5.236173/1.028306bp; these are extractor
font-metric boxes, not verified glyph baselines or outline distances. Raster
and box evidence must not be substituted for each other. The existing PDF
comparison preserves unknown cross-producer operator/source correspondence.

Reproduce export by passing `65dbe7d-v2.json`, unchanged `request.jsonl`, the
pinned font/license and an output prefix to `pipeline_cff_probe --searchable`.
This mode emits the owner's CID-CFF PDF and a separate verified-input evidence
sidecar. It does not use the outline-only exporter. Run `pdf_compare` without
passing that sidecar (the comparison's outline-span evidence format is distinct),
then `pdftotext -enc UTF-8` and `pdftoppm -r 144 -singlefile -png` on both PDFs.

## Corrected rooted-assets run and source-text semantics

The new producer requires four pinned TFM files and the exact license under a
rooted layout: `fonts/tfm/public/lm/{ec-lmr12,rm-lmr12,rm-lmr8,rm-lmr6}.tfm` and
`doc/fonts/lm/GUST-FONT-LICENSE.TXT`. FLASHTEX_TFM_DIRS must name that full TFM
directory, not the old flat download directory. With these unchanged official
assets the producer reports zero diagnostics. `65dbe7d-clean-*` pins this run.

The original searchable PDF SHA6308c8a95726a980ec341ad53af15a0b12654d27c9021d5a74306b8b807c9cb1
reproduces exactly from unmodified producer JSON. Poppler26.01 extracted text
matches the established reference, and144DPI1224×1584 RGB pixels are identical.
This establishes one fixture/configuration's raster and extraction equality;
it is not all-document, all-resolution or PDF byte/operator equality. The latter
still differ. Word-box x differences are at most0.007259bp; y box differences
remain font-metric extraction differences, not proven baseline differences.

The historical error-bearing display is retained as
`65dbe7d-required-unavailable.json` and now exercises a refusal before searchable
export. Partial geometry remains available through the separate page API. The
reference frame gate refuses both old `tfm_missing` and new
`required_metrics_unavailable`/error diagnostics, preventing this setup mistake
from being labeled successful reference acceptance again.

Actual published-producer `escaped-*` fixtures retain visible `% _ & # { }`
cluster strings while their source spans include the original backslash escapes.
Actual exported PDF text is `Escaped % _ & # { } and office fi.`; no extraction
text is inferred from source spelling or reverse-mapped from a GID. `ffi`/`fi`
clusters retain their logical strings. Therefore no new policy refusal is needed
for these demonstrated cases. Source ranges remain navigation provenance.

The actual math emitter appends its laid-out `g.ch` into logical cluster text,
while source ranges can cover the enclosing TeX expression. No matching
`latinmodern-math.otf` is installed on this host, so a real matching-font math
extraction probe is still unavailable; there is no new math-text parity claim.
The ActualText proposal and existing ambiguous-mapping refusals remain in force.
