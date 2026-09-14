# Experimental Rust rendering core

Original Rust types, bounded JSON parsing and semantic validation for the
experimental `protocol/rendering-v2.schema.json` proposal. This crate does not
change the compiler, native preview, PDF writer or runtime-v1 wire behavior.

```sh
cargo test --manifest-path crates/rendering-core/Cargo.toml
cargo clippy --manifest-path crates/rendering-core/Cargo.toml -- -D warnings
```

`parse(&[u8])` enforces a 32 MiB message limit, exact typed fields, protocol version
and known message/item types. Call `Envelope::validate(Some(&offer))` afterwards:
parsing alone does not negotiate capabilities or validate cross-record references.
Offers and explicit rejections can be validated with no preceding offer. Selection
IDs must match the offer; display IDs identify their own compiler request.

`DisplayList::validate` checks font manifests, original GID bounds, contiguous pages,
finite RGBA, exact fixed-point coordinates and checked geometry sums, explicit rule
geometry, logical UTF-8 clusters, caret/hit regions and source provenance. Logical
cluster text partitions the run and is extracted once per cluster. Multiple glyphs
may refer to one cluster; consumers must not repeat its text once per glyph.

`DisplayList::validate_resources` additionally takes exact `SourceSnapshot` maps,
font byte maps and a `FontValidator` adapter. It verifies SHA256/length before calling
the separately owned font loader, compares units/GID count, and checks source UTF-8
boundaries against the exact document revision. The hook intentionally avoids a
second competing font parser. `font_adapter::StaticTrueTypeLoader` now calls the original `font-resources`
crate's `inspect_static_truetype` API. `validate_with_collection` additionally
requires exact descriptors from a loader-owned immutable, license-bound collection.

All resource evidence retains `paintable: false`: source/font identity checks are
not proof that a consumer safely paints those outlines, that shaping is correct,
or that native/PDF output matches. Production activation requires compiler/PDF/Mac
agreement on the proposal and their acceptance gates.

Coordinates use `Tick(i64)`, bounded to JSON's exact ±(2^53−1) integer range, with
1,048,576 ticks per PDF point. `Tick::from_tex_sp` converts TeX scaled points once
using exact integer arithmetic and round-to-nearest/ties-to-even; TeX and PDF point
sizes are deliberately different. JSON floating-point geometry is rejected.

Tests use declared synthetic glyph IDs and a deliberately fake font-validation
callback to exercise mapping and resource substitution; these are not real fonts
or visual correctness evidence. Fixtures retain the experimental schema provenance
from commit `41cacfc`; no image, network provider, or external TeX engine is run.

`hit_test::PageIndex` builds a per-page spatial index from declared hit rectangles
and rules. A cluster's `hit_rects` are its **laid-out TeX boxes** — the advance
TeX positions the next atom from, and the box's height plus depth — not the
extent of the outlines painted inside them, so a space stays selectable and a
full stop is not a one-point-tall target. Measuring a delimiter reads the same
rectangles; the painted extent is the separate, negotiated `ink_rect`
(`protocol/proposals/display-list-v2-ink-rect.md`), which this crate does not
accept yet. Queries require the matching project/revision, use exact half-open
fixed-point rectangles clipped to page bounds, and resolve overlaps in paint order.
Caret selection uses only supplied caret positions; absent carets return a whole
logical cluster. TeX source ranges and synthetic provenance remain unchanged. Shared
cluster metadata avoids copying caret/source arrays for every rectangle.

`cache::DisplayCache` accepts an authoritative `RenderIdentity` before work starts.
The identity includes project revision, source digests/revisions, font manifests
and a compiler/configuration digest. A `RenderTicket` binds that expectation to a
cache generation. Source/font/config changes invalidate prior tickets immediately;
late frames cannot replace a newer expectation. Installation verifies exact source
and font bytes, keeps immutable font bytes alongside the complete display list,
and rejects conflicting geometry for one immutable identity. It never combines
pages from different revisions. Identical completed installs reuse one `Arc`.

Callers must obtain the current frame through its ticket before painting or querying
its index. Previously borrowed Arcs remain readable but are no longer current after
invalidation. Cache accounting limits retained serialized payload/font bytes;
allocator/index overhead and Arcs retained externally are additional memory.


Real resource integration probe (no rendering or glyph-shaping claim):

```sh
cargo run --manifest-path crates/rendering-core/Cargo.toml --example resource_probe -- /path/to/font.ttf
```

Verified locally with installed LiberationSans-Regular.ttf, SHA256
`76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8`,
2048 units per em and 2620 glyphs. The probe uses a synthetic positioned-text
fixture and keeps `paintable: false`; the font itself is not copied into this crate.
Font license/embedding permission remains explicit loader metadata and is never
inferred from successful parsing or converted from unknown to allowed.

`transform::ViewportTransform` maps canonical ticks to exact rational viewport
coordinates with positive zoom, translation and optional y-axis reversal. It clips
source rectangles before transformation and preserves half-open boundary inclusion
even when the y axis flips. `PageIndex::hit_test_exact` retains fractional query
positions for caret choice; integer floor is used only for exact membership tests
against integer rectangle edges. No pixel rounding changes caret selection.
Rational denominators are bounded to one million; checked distance arithmetic
returns an explicit error if an extreme query exceeds its i128 budget.

`wire::parse_validated` and `wire::serialize_validated` form the opt-in consumer
boundary. They validate capabilities and semantic references, return typed errors
for unsupported versions/messages/primitives, and never emit a partial oversized
frame. The serializer defaults to the caller's requested cap, bounded above by
32 MiB. Resource verification remains separate from wire validity.

```sh
cargo run --manifest-path crates/rendering-core/Cargo.toml --example validate_display -- \
  crates/rendering-core/tests/fixtures/synthetic-display-list.json \
  crates/rendering-core/tests/fixtures/capabilities.json
```

The offline harness validates, serializes, reparses and verifies a stable canonical
roundtrip. It reports `paintable: false`; a synthetic glyph fixture is not a native
rendering or font-shaping test.

`outlines::PreparedOutlines` validates exact display/font descriptors once, then
resolves original glyph IDs through the immutable font loader. It preserves font
SHA, component instances, logical cluster/source ranges and explicit synthetic
provenance. Loader-supplied quadratic paths and implied points are placed with exact
checked rational size/baseline arithmetic; font coordinates point upward and page
coordinates downward. `hinting_applied` remains false. Unsupported loader cases
return errors, never silently empty paths or replacement fonts.

```sh
cargo run --manifest-path crates/rendering-core/Cargo.toml --example outline_probe -- \
  /path/to/font.ttf /path/to/LICENSE.txt A
```

Local LiberationSans `A` probe: original GID 36, 17 points, two contours and
17 placed path commands; exact font hash recorded above. Supplied installed license
SHA256 was `93fed46019c38bbe566b479d22148e2e8a1e85ada614accb0211c37b2c61c19b`.
No glyph shaping, hint execution, raster painting or reference-TeX parity is claimed.

`glyph_cache::GlyphPathCache` retains immutable expanded quadratic paths by font
SHA256, face, original GID and `UnhintedExactComponentsV1` policy. Font aliases share
geometry only when the actual bytes match. LRU limits bound entry count and retained
path/component payload estimates; map/allocator overhead and caller-held Arcs are
additional memory. Unsupported/font-error results and oversized-path budget failures
remain explicit cached outcomes, preventing repeated expansion attempts.
`PreparedOutlines::glyph_cached` applies each glyph's exact size/origin after lookup.

The developer outline probe now reports expansion-cache measurements separately.
One local debug-build LiberationSans `A` run measured one cold expansion at 59,456ns
and 1,000 cache lookups totaling 901,476ns (one expansion, 1,000 hits). These are
single-run font-cache observations, not native paint or edit-to-preview latency.

`batch::PreparedBatchSource` verifies immutable font descriptors and source snapshots
once, then creates atomic consumer-neutral page batches. Batches preserve glyph/rule
paint order, sRGB paint, exact quadratic commands and source/synthetic provenance.
The canonical rectangular clip is intersected with the page; `None` in the returned
`visible_clip` means empty. Consumers must apply that clip when painting curves.
Glyph ink is never culled using source hit boxes. Operation/path limits fail the
whole batch explicitly; immutable cached expansions remain reusable on retry.
Collection validation uses loader-owned verified bytes without copying or reparsing
fonts. This adapter does not activate runtime-v2 or establish native paint parity.

`tex_adapter` binds original 8-bit TFM codes through the font loader's explicit
encoding manifest. Physical and virtual runs retain original GIDs, font/TFM hashes,
exact metrics/kerns and original input intervals. VF rules convert their lower-left
reference to the page's top-edge convention. Callers explicitly select
`ExactRationalNoTexRounding`; exact rational values never imply TeX scaled-point
rounding. Unsupported virtual commands, .notdef, absent bindings and arithmetic
budgets fail explicitly.

`EncodedRun::batch` emits existing unhinted exact quadratic draw batches using only
loader-bound immutable fonts and supplied logical UTF-8/source interval mappings.
Source snapshots must match declared revision/digest. Internal batch conversion preserves rational origins/sizes through exact checked
quadratic placement; `ExactRule` retains fractional virtual-rule bounds. No Unicode inference, interval
interpolation, implicit rounding or production runtime activation occurs. Flat/VF
equivalence fixtures are original synthetic data, not a reference-TeX oracle.

`graph_cache::GraphCache` immutably borrows one fully declared resource graph and
caches exact nested packets by resource hash key and encoded character. A cache
cannot be reassigned to another graph: file hashes alone do not identify encoding
declarations or virtual local-font bindings. Cached packets retain the complete
root-to-leaf source chain. LRU entry/payload limits and explicit cached unsupported
resource or oversize outcomes bound retained work; loader limits bound expansion
separately. Payload figures exclude map/allocator overhead and externally held Arcs.
Tests explicitly distinguish identical font/TFM hashes with different encodings.

`place_path_exact` accepts checked rational font sizes and origins, retaining exact
quadratic coordinates with bounded i128/u128 arithmetic. `ExactClip` supports exact
intersection and half-open membership; `batch_with_exact_clip` returns an
`ExactDrawBatch` whose exact clip is authoritative for the consumer. Curves remain
unflattened, and the original integer wire schema is unchanged. Overflow is an
explicit error. Tests cover fractional glyph origins/scales, VF rule bounds, clips,
precision exhaustion and integral-path equivalence.

`tex_adapter::nested_run` consumes cached nested graph packets and verifies every
physical placement against a supplied font/TFM/encoding binding. It produces the
same exact `EncodedRun` API, retaining encoded input intervals and rational nested
scale/offset/rule geometry. `NestedRun::source_chains()` is indexed by the original
run operation index. Prefer `NestedRun::batch`, which attaches the correct chain
to each retained primitive by stable item/glyph identity after culling. Missing or conflicting bindings fail before any partial run is
returned. Synthetic tests compare flat and two-level virtual glyph/rule runs and
verify that differing encoding declarations cannot be substituted at this boundary.

`TracedBatch` keeps source chains attached to primitives, so vector reordering does
not relabel provenance. Primitive identity is scoped by project, revision and page;
`PrimitiveId` retains original item/glyph indices rather than output-vector indices.
The nested run and chain storage are immutable behind accessors. An adversarial
fixture culls an initial rule, keeps a glyph and later rule, reorders the retained
primitives, then applies a fractional clip that removes the later rule. Every
retained chain still points to its original virtual-font command.

`cubic::CffConsumer` is a separate opt-in CFF1 consumer. It verifies an explicitly
supplied raw table digest and encapsulates parsed dictionaries immutably. The
font loader applies exact FontMatrix into font/text space; this adapter then
scales once and flips the baseline into exact page coordinates. Cubic controls
remain cubic. Callers supply `HintPolicy` explicitly; `Unhinted` retains validated
hint metadata while `hinting_applied` remains false. CID/CFF2 and other unsupported
loader operations remain explicit failures. Original GID zero is rejected.

```sh
cargo run --manifest-path crates/rendering-core/Cargo.toml --example cff_probe -- /path/to/raw-table.cff
```

An offline installed STIXTwoText-Regular probe retained 55,177 exact commands across
2,220 non-.notdef glyphs, with zero rejected placements and one skipped .notdef.
CFF table SHA256: `c5d11bab6a95e75a568e1b72fd30fdd5e4c95abe68a72f02c0c4329ee948b532`;
containing installed OTF SHA256: `c4864ca6ec071c2d31d0d8309001faa1ee3517fffb53a31a405a697b71f52ca1`.
The probe used rational size 10,485,761/3 ticks and origin (1/2, 7/4). This verifies
decoding/placement only. OpenType selection, license binding, shaping, rasterization
and native/PDF acceptance remain upstream or downstream gates. No font file is
copied into the repository and no display-list wire primitive is activated.

`mixed::MixedBatch` combines validated upstream traced primitives with explicitly
selected immutable CFF replacements. It preserves quadratic/cubic/rule distinctions,
paint order, stable primitive identity and attached source chains. Each primitive
retains the exact intersection of page, caller and source clips. Repeated identities,
stale project/revision/page context and attempted rule-to-glyph replacement fail.
CFF glyph selection is explicit; this API does not perform shaping or infer that
a replacement glyph represents the upstream logical text.

Construction enforces total command count and a strict serialized-byte cap through
a bounded writer, returning no partial batch on failure. `fixture_bytes()` uses
`flashtex-internal-mixed-v1`, a separate opt-in consumer fixture format. Rational
coordinates use decimal numerator/denominator strings so JSON consumers cannot
round large integers. These limits bound encoded payload, not total process RSS or
font-loader transient allocations. Legacy rendering wire and hinting flags remain
unchanged. Tests combine explicit synthetic quadratic geometry with parsed original
CFF fixture curves, plus exact byte-boundary, command-budget and culling checks.

`mixed_replay::ReplayBatch` validates the internal fixture and converts its paths
back to typed exact consumer geometry. It rejects duplicate keys recursively,
unknown fields/primitives, mismatched command kinds, noncanonical rationals,
invalid clips and altered total command counts. Full metadata remains available
for lossless canonical replay. The replay does not verify referenced font/source
bytes, and never marks a fixture paintable.

```sh
cargo run --manifest-path crates/rendering-core/Cargo.toml --example replay_mixed -- \
  crates/rendering-core/tests/fixtures/synthetic-mixed.json
```

The checked-in illustrative fixture has three primitives and five commands;
canonical output is 1,932 bytes with SHA256
`aaa78b395c8740a53bb4c363076b0b4263c159287097f943c61591def2f8e7ff`.
Tests also roundtrip numerators beyond 2^100 through typed geometry without float
conversion, retaining exact source metadata and primitive identity.

`CachedCffConsumer` shares the loader's immutable full-font cache behind a mutex.
Its identity retains containing font SHA256, CFF table SHA256, face and validated
range; the existing OpenType reader remains responsible for selecting that range.
Direct and cached providers share exact placement code. Cache status explicitly
distinguishes stored, hit and oversized bypass outcomes. Mixed batches retain the
optional full-font identity and reject provider results that change GID, hint policy
or claim applied hinting. Replay validates the optional identity while accepting
earlier fixtures without it. The merged loader also explicitly rejects stroked CFF
PaintType, which this fill-path consumer cannot implement.

`residency::MixedResidency` keeps immutable prepared pages under explicit total
page, encoded-byte and command budgets. `begin` verifies and captures source
snapshots plus resource/configuration identities in a generation-bound lease;
compile from `lease.snapshots()` and prepare through that same lease. Any changed
source, resource or configuration invalidates prior jobs, even at the same project
revision. Revision rollback is rejected. Preparation checks source UTF-8 boundaries
and requires every retained font identity in the declared resource set.

Installation rejects superseded work, conflicting outputs for one identity and
oversized pages without replacing a good frame. LRU eviction removes residency
while existing caller-held Arcs remain immutable. Encoded-byte and command limits
do not claim to bound all allocator overhead, captured source snapshots or external
Arcs. Source snapshots have their own 32 MiB input cap. A caller must capture its
lease before compilation; a fresh lease cannot prove old output used new sources.

`cff_run::CffRun` consumes the font loader's `BoundCffTfmFont` through the matching
immutable full-font cache. It retains the original 8-bit code, resolved glyph name
and original GID, TFM/encoding/full-font identities and input intervals. TFM widths
and kerns alone advance the pen; the exact transformed charstring advance remains
a separate field. Explicit scale/hint policies carry through rational cubic
placement. Missing mappings, .notdef, wrong cache identity and total glyph/command
overflow fail without returning a partial run. This is explicit encoding, not
Unicode shaping or TeX scaled-point rounding.

`CffRun::fixture_bytes` serializes exact TFM metrics, kerns, input intervals and
full cubic outline evidence through the same bounded writer as mixed batches.
The pinned STIX named-glyph harness checks the installed font and OFL license
hashes, the previously validated CFF range, explicit `A` name -> original GID 3,
and an original synthetic 10-point TFM with half-em advances.

```sh
cargo run --manifest-path crates/rendering-core/Cargo.toml --example cff_tfm_probe -- \
  /usr/share/fonts/stix-fonts/STIXTwoText-Regular.otf /usr/share/licenses/stix-fonts/OFL.txt
```

Two encoded A slots produce 48 exact commands and 6,346 evidence bytes, identical
for cold and warm runs. SHA256:
`f0210698b7d382171727f4768b3fb437a2fb6f2a63ccb492df603dae096e42e1`.
`tests/fixtures/stix-cff-tfm.json` pins the font/license/TFM and records reference
gaps. No matched distribution TFM/encoding pair or TeX rounding/native/PDF oracle
is claimed. The synthetic name-mapping test also compares direct and cached
resolved encodings, preserving independent TFM and outline advances.

`geometry_diff` compares two validated mixed fixtures or two display-list-v2
envelopes with an explicit capability offer. Reports retain raw input/offer hashes,
page and stable primitive identity, original values and exact right-minus-left
rational deltas where representable. Categories distinguish resources/GIDs,
advances/positions, baselines/rules, provenance and membership/order changes. It
does not align pages, normalize geometry, substitute fonts or apply tolerances.
Mixed and display formats are not assumed directly equivalent.

```sh
cargo run --manifest-path crates/rendering-core/Cargo.toml --example geometry_diff -- \
  left.json right.json [--offer capabilities.json]
```

CLI exit codes: 0 for complete equality, 1 for complete differences, 2 for an
incomplete/unsupported comparison. API limits bound traversal, difference count
and serialized report bytes. `equal` is null whenever truncated or unsupported,
including exact delta arithmetic that exceeds its i128/u128 representation budget.
Source/font bytes are not verified by comparison, and equality is not a visual
or reference-engine parity claim. Identical illustrative mixed inputs visit 141
comparison nodes and produce an empty complete difference report.

The Type2 arithmetic dependency checkpoint (`2f770fd`) preserves the pinned STIX
run SHA above. A synthetic add-operated curve matches literal-coordinate output
exactly through direct and cached placement, while retaining distinct input hashes.
Non-dyadic Type2 division stays an explicit unsupported result. This exercises the
new arithmetic without changing default hint, wire or device-grid policies.

`device_grid::DevicePathCache` is a separate opt-in cache for composites requiring
an explicit device grid. Keys retain font SHA/face/GID, ppem X/Y, tie rule, declared
outline-policy/build hash and transform order (declared offset transform before
grid rounding; child assembly before parent transform). Missing context fails;
changing context clears device entries. The size-independent unhinted cache and
its unsupported outcomes remain unchanged. Retained byte charges exclude map
overhead and external Arcs; entry and payload caps stay explicit.

Device paths serialize only to `flashtex-internal-device-v1` comparison fixtures.
The geometry-diff tool validates this opt-in format and reports device policy changes
as resource-context differences; it does not infer equivalence to mixed/display
formats. Fractional placement remains exact, and `hinting_applied` remains false.

```sh
cargo run --manifest-path crates/rendering-core/Cargo.toml --example device_grid_probe -- \
  /usr/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf \
  /usr/share/licenses/liberation-sans-fonts/LICENSE
```

Pinned 16-ppem/AwayFromZero replay: 2,619 non-.notdef device glyphs, 63,782 commands,
2,619 direct/cache matches and warm hits. Default expansion remains 1,678 accepted
and 941 explicitly unsupported non-.notdef glyphs. Geometry SHA256:
`a33a8836d80b2bd9fa89ba017c60df0ef175d563d930b5620e0b7e75f9dfa3b5`.
The fixture records font/license/policy pins. Device policy is not TrueType
instruction execution or a hinted raster/native/PDF parity claim.

`shaped_run::PlacedShapedRun::prepare` consumes the original font engine's
`BoundShapedRun` and a current source snapshot into exact internal quadratic or
cubic paths. It retains the complete immutable shaping record, original GIDs,
engine face identity and distinct full-font SHA, absolute UTF-8 cluster ranges,
empty clusters, ligature counts and feature notes. Integer advances already
include shaping kerning; exact size/UPEM scales them separately from outline
advances. Glyph offsets flip the font y-axis once. The caller supplies the exact
clip and an item identity; no hit positions are invented inside a cluster.

The source digest/revision and outline resource identity must agree before
expansion. Glyph/command/charged-payload limits fail atomically without returning
a partial run. Charged payload includes hint records; it is not allocator RSS or
shared cache residency. TrueType uses the unchanged unhinted cache; device-grid
paths require the separate explicit device API. CFF requires an immutable cache
bound to the full font and an explicit hint policy. No wire/native activation.

`cargo run --offline --manifest-path crates/rendering-core/Cargo.toml --example
shaped_run_probe -- /path/STIXTwoText-Regular.otf /path/OFL.txt` checks existing
font/license pins and exact warm-cache placement for a Unicode source slice.
The observed pinned run has 11 glyphs/clusters, 165 commands and advance
9762243491/600 canonical ticks. This demonstrates consumer consistency, not
reference shaping completeness, TeX metrics, hinting or native visual parity.

`PlacedShapedRun::replay_bytes` produces `flashtex-internal-shaped-v1` offline
fixtures with exact rational geometry, both font identities, source revision/hash,
shaping options/notes, full cluster coverage (including empty clusters), original
GIDs and stable primitive IDs. `shaped_replay::ShapedReplay::parse` rejects
unknown fields/commands, duplicate JSON keys, noncanonical fractions, malformed
cluster coverage, inconsistent exact metric placement and exceeded limits.
`verify_source(path, snapshot)` separately checks current source contents and
revision; parsing alone cannot verify font bytes, the shape cache key's provenance,
or the truth of externally supplied outline geometry.

The geometry-diff CLI accepts these fixtures directly. It preserves raw input
hashes, reports exact rational changes and refuses equality claims on truncated
reports. `tests/fixtures/synthetic-shaped.json` is an original synthetic-font
fixture containing `A`, `é` and an invisible zero-width-space cluster, not a real
font rendering reference. It can be regenerated with
`FLASHTEX_RECORD_SHAPED_FIXTURE=1 cargo test --offline --manifest-path
crates/rendering-core/Cargo.toml --test shaped_run`.

The pinned STIX probe also verifies source-aware replay, explicit warm-cache hit
status and 100 identical placement/serialization repetitions. Timings are labelled
as local debug measurements; they exclude native painting and are not a preview
latency claim. Replay includes the shaping implementation cache key, so a source
revision may legitimately change the replay hash without changing geometry.

`registry_binding::RegistryRenderer` binds explicit family/weight/style choices
from an immutable `ProjectFontRegistry`. Every lease retains the project instance,
registry generation, full resource declaration (including file and licensing
provenance), verified engine identity and exact CFF table identity when applicable.
Shaping and placement check the active lease; no nearest-style or system-font
fallback occurs. Per-binding cache allocations share the configured total charged
cache payload budget. Verified font storage, engine copies and caller-held frames
or leases are outside that cache budget.

Replacing a registry with a different semantic generation clears active bindings
and rejects prior leases, even if a later replacement returns to an earlier
hash. Existing frames and their exact paths remain immutable. Identical semantic
generations preserve active caches. Renderer instances have separate local lease
identities even when their caller-supplied project labels match.

Registry frame replay wraps shaped evidence with the complete binding.
`verify_replay` checks the current lease, source snapshot, exact declaration,
engine/raw font identities, backend and CFF table range/hash. This establishes
resource provenance, not that arbitrary externally supplied outline commands
match the font; it does not grant painting or activate a native wire.

Synthetic tests exercise replacement and retained-frame behavior. Run the separate
pinned TTF/CFF acceptance when those licensed installed resources are available:
`cargo test --offline --manifest-path crates/rendering-core/Cargo.toml --test
registry_binding pinned_mixed_backend_registry_replay_and_replacement -- --ignored
--nocapture`. It pins both fonts/licenses before creating a temporary project.

`registry_binding::selection` provides source-aware historical hit inspection and
current insertion destinations. The caller supplies exact bounds and start/end
carets for every shaped cluster; glyph advances are never treated as ink bounds.
Clusters without hit bounds, including invisible characters, remain addressable
through their complete source metadata. Ligatures expose their indivisible source
range and supplied edge carets. RTL/reordered caret layouts are explicitly rejected
until the shaping engine supports that ordering.

Selection creation requires the current registry lease and exact source snapshot.
Historical `inspect` remains available after replacement, but `destination` and
`validate_destination` reject old epochs, foreign frames, source revisions/hashes
or paths. Call `validate_destination` at the edit boundary: an earlier destination
is not a permanent authorization to edit a changed document. Exact nearest-caret
comparison can return a precision-bound error rather than round coordinates.

`RegistryRenderer::export_manifest(expected_generation, max_bytes)` consumes the
font owner's version2 manifest API without rewriting its format. Bounded
`metadata_page` exposes exact declared styles and CFF table provenance for an
explicit user choice. Importing the exported manifest through the rooted registry
loader preserves the semantic generation and active rendering leases. Saving is
left to the caller's existing rooted file layer. No system discovery or native
protocol change is introduced.

The opt-in `pdf_stream` adapter emits validated exact PDF path operators and a
provenance sidecar. It refuses unsupported decimal rounding/transparency and does
not write another PDF container. See [PDF-INTEGRATION.md](PDF-INTEGRATION.md) for
the reviewed backend limitation, missing owner API, exact geometry gates and
executable synthetic/real-font stream checks.

`registry_binding::nested` consumes immutable VF graph-cache packets with explicit
registry leases and independently bound TFM encodings for both TrueType and CFF
endpoints. Original GIDs, full font/CFF/TFM/encoding hashes, face identity and source
chains survive mixed batch serialization. Exact VF coordinates and TFM advances
use the caller-selected rational policy; they do not claim TeX scaled-point
rounding equivalence. Unbound endpoints and stale registry leases reject before a
partial frame is returned. The older TrueType-only `nested_run` explicitly rejects
CFF roots; use the mixed registry consumer instead.

Synthetic sfnt/TFM/VF acceptance proves flat/nested mixed geometry equality,
fractional origins/advances, explicit encoding provenance and immutable retained
frames after registry/source changes. The graph cache is scoped to one immutable
graph; callers still own dependency reload detection. `require_current` checks
registry/source state, not external VF file freshness. No real VF oracle or native
wire integration is claimed. Resource cache charging includes the expanded CFF
identity fields; caller-held frames/Arcs remain outside current cache residency.

`registry_binding::math` consumes the existing original MATH parser through
`BoundMathFont`; it adds no parser or layout algorithm. Bind a `MathLease` from a
current rendering lease with explicit `UnhintedDesignUnits`, then request a
`MathMetricsSnapshot` with source path/revision/range, exact font size and at most
256 original GIDs. All56 constants retain raw values and a typed dimension:
53 lengths scale exactly to canonical ticks, while3 percentages become independent
dimensionless ratios. Italic corrections and optional top-accent attachments
retain original design units alongside scaled values. Absent accents stay absent.

Snapshots expose complete registry/style/declaration/license, raw font and engine
face identities, MATH table SHA/length and parser source SHA. Replay also records
the consumer source SHA and exact source snapshot identity. `verify_replay`
compares all supplied values against current verified bound metrics and rejects
duplicate/malformed/tampered or stale evidence. Retained snapshots stay immutable
after replacement. Device adjustments, variants, math kerning and extended-shape
coverage remain explicit unsupported capabilities in this stage.

Synthetic acceptance covers signed lengths, unsigned minimum heights, percentages,
glyph corrections, absent accents, exact scaling and stale font/source refusal.
The separately run pinned `pinned_stix_math_metric_consumer_replay` test uses the
installed STIXTwoMath resource SHA
`3a5f3f26f40d5698b3c62dd085d48d6663696a3f80825aab8b553d5097518e8c`
and the already pinned OFL license. Its MATH table is27408 bytes, SHA
`0af4bf095e9d3a968b460b83d589b5c0a6dc58762fe1f3cb3dece03fd5344c4a`.
It verifies56 constants,6 original GIDs and deterministic exact replay at a
fractional size; this is consumer consistency, not a mathematical layout oracle
or native painting claim.

### Exact fitted MATH construction placement

`registry_binding::math::assembly::RegistryRenderer::math_assembly` consumes the
original font resource fitter through an explicit strategy and limits. The caller
supplies target extent in design units, page font size, baseline origin, direction,
clip, source range and CFF unhinted policy. Horizontal assembly offsets advance
right; vertical offsets advance upward (one conversion to downward page y).
No automatic baseline alignment or TeX delimiter policy is inferred.

`MathAssemblyFrame` retains immutable bound MATH metrics, the complete fitted
result (part/instance identity, exact offsets/overlaps, italic correction and
strategy), and mixed quadratic/cubic batch. Stable primitive index corresponds to
the fitted part index; the original GID and source range remain on every primitive.
`require_current` gates reuse against registry lease and source revision/hash;
retained geometry remains readable. Device-adjusted assemblies are explicitly
unsupported. Budget failures return no partial frame. The mixed fixture is still
an internal geometry format: retain the frame's fit/metrics provenance alongside
it, since it does not serialize new MATH wire fields.

The synthetic test proves exact fractional origins in both directions, overlaps,
ready variants and primitive/command/byte/fit budget rejection. The pinned STIX
acceptance exercises 32 vertical and 34 horizontal assemblies at target5000.5
with cache/direct byte equality and stale-source refusal. Three other STIX
constructions refuse the declared fit budget. These are consistency tests;
TeX layout, native paint and visual parity are not established.

### Source-bound MathKern metrics replay

`RegistryRenderer::math_kerns` accepts up to256 explicit original-GID, corner and
rational design-unit height queries. It consumes the original bounded parser's
upper-bound tie policy, preserves raw kern values and scales them exactly to the
requested page font size. Absent corners return the parser's zero value; invalid
GIDs fail. Height-record and selected-value device flags remain separate and no
device correction or script layout occurs.

`MathKernSnapshot` retains the source/registry/MATH snapshot. Its bounded replay
records query heights, tie policy, scaled values and consumer source hash;
verification rejects tampering, duplicate JSON keys, stale source or stale lease.
Synthetic checks cover negative heights/values, exact ties, missing corners and
request/output limits. Pinned STIX replay compares every available corner table
around its correction-height boundaries against the bound resource API. This
is exact consumer consistency rather than an independent typesetting oracle.

### Explicit MATH device metrics

`math_devices` is an opt-in query path for constant, glyph and kerning Device
records. Every request supplies ppem; `PixelScale` separately supplies exact
horizontal and vertical canonical ticks per pixel. Constants/glyph requests name
the axis explicitly. Kern selection uses the resource adapter's corrected-height
policy and retains its selected interval. Nothing infers a device scale from zoom
or font size.

Each result retains raw design units, base ticks, signed pixel delta, correction
ticks, optional combined value and Device table hash/offset. Absent accents remain
absent. The retained base metrics still declare their original unhinted policy;
this API does not hint outlines or position scripts. Replay binds both ppem axes
and pixel scales and rejects changed context/provenance. VariationIndex and
unsupported device data preserve typed resource errors.

Synthetic tests check constant/italic/accent/kern corrections, exact unequal axis
scales, ppem changes, absent accents and request/output limits. Pinned STIX GID3326
has a +1pixel top-accent correction at12ppem: the consumer preserves its exact
7/3 canonical-tick correction and original Device table identity. This does not
establish device-aware paint or native cache integration.

### Fitted geometry replay and cache consistency

`MathAssemblyFrame::replay_bytes` now joins the exact mixed geometry fixture with
its source/registry/MATH metrics, part-instance mapping, offsets/overlaps, target,
strategy, limits, explicit origin and CFF policy. This bounded internal evidence
has a consumer source hash and geometry hash. `verify_replay` first checks the
live lease/source and then compares every field with the immutable frame; it does
not accept an arbitrary font outline just because JSON is well formed.

Synthetic and pinned STIX acceptance compare this complete evidence after fresh
resource binding and repeated path-cache reuse, with no geometry tolerance.
Pinned tests print fresh/reused-path elapsed times for one horizontal and one
vertical construction. These include consumer work and are diagnostic samples,
not native paint latency or a performance threshold. Source revisions, fit
metadata and declared origins remain part of the replay identity.

### Registry-bound parsed MATH query cache

`RegistryRenderer::math_cache` creates `RegistryMathCache` borrowing one immutable
`MathLease`. Its `assembly`, `kerns` and `devices` methods validate the live renderer
lease before querying the original font cache, then rebuild exact source-bound
frames through the same conversion paths used by direct calls. Unhinted kerning
has its own cache query and never invents ppem. Device contexts, rational heights,
fit strategy and limits remain query keys; page pixel scales and source revisions
are recomposed and verified on every result. A registry A→B→A transition cannot
revive an old cache. No persistent cache or native wire activation is introduced.

`MathAssemblyFrame::fit()` now returns the local immutable `AssemblyFit` wrapper,
with the same `identity()` and `fit()` getters; it preserves the verified parent
MathIdentity without fabricating a private resource-library BoundMathFit. Consumers
that explicitly named that former return type must use AssemblyFit instead.

Caller limits bound entry storage, retained query results and parsed tables
separately. `stats()` exposes the original cache's charged bytes, computations,
hits and parse counts; oversized outcomes may bypass residency. Caller-retained
frames and transient parsing allocations are outside these residency counters.
Synthetic checks exercise eviction, zero parsed-table residency, cached failures,
changed pixel scales/source, and stale A→B→A leases with retained geometry intact.

The pinned STIX workload compares exact direct/cached assembly, unhinted kerning
and device replay: one kern parse, one variants parse,945 hits and946 computations
in the recorded run (178624 query bytes,130528 parsed bytes). These counts prove
eliminated repeated parsing; they do not establish native paint latency or an
end-to-end speedup.

### Original PDF backend export

`pdf_export::export` now connects exact mixed/shaped path streams to the PDF
owner's unchanged `ExactDocument`/`Content::Verbatim` API. It produces actual
PDF bytes plus bounded provenance evidence, verifies xref/page sizes/content
readback, and never calls the legacy rounded-number text route. Glyphs remain
resolved outlines; original GID/font/source identities stay in evidence rather
than embedded searchable text. Empty glyphs preserve spans without invalid fills.
Nonterminating decimals and alpha remain explicit unsupported conversions.

`examples/pdf_export.rs` is an executable single-page fixture exporter.
`shaped_run_probe` additionally emits and checks a real pinned STIX PDF and accepts
an optional output directory after its font/license arguments. Checked-in
synthetic and STIX PDF fixtures support deterministic byte/operator replay.
See `PDF-INTEGRATION.md` for hashes, limits and remaining text/native/oracle gaps.

`pdf_compare` and its matching CLI now compare actual candidate/reference PDF
bytes through the owner's reader/classifier. Reports separate raw equality,
parsed operator equality and unknown visual equality; exact geometry/paint
changes can carry regenerated candidate primitive/font/source provenance.
Reference correspondence stays unknown unless independently established. Limits,
unsupported operators and truncated reports cannot produce an equality claim.
See `PDF-INTEGRATION.md` for the explicit evidence and reference-fixture scope.
