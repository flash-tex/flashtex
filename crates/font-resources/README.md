# Deterministic font resources

Original Rust resource loader for rendering-v2 schema revision `41cacfc`. Its
`FontDescriptor` has exactly the eight wire fields defined there. Paths and
license records belong to a separate resource manifest (`schema_version: 1`),
not to the display-list descriptor. No platform font discovery or fallback occurs.

`FontCollection::load(root, &manifest)` reads explicitly named relative resources,
checks bounded sizes, SHA256 for font and license bytes, declared metrics and
PostScript name, then retains immutable shared bytes. Duplicate IDs, traversal,
escaping symlinks, unsupported formats, missing files, digest mismatches and
metadata mismatches are errors. IDs iterate lexically. Existing resources remain
unchanged if the underlying files or caller buffers later change.

`FontResource::from_bytes` performs the same digest/metadata checks for already
provided bytes. `inspect_static_truetype(&[u8]) -> Result<FontMetadata>` exposes
structural metadata inspection for rendering-core's FontValidator adapter; that
inspection alone does not establish expected identity or license provenance.
`bytes()`, `shared_bytes()` and `table(tag)` expose immutable data for consumers.

The current profile accepts single-face static TrueType sfnt version 1 only,
face index zero. TTC, variable fonts, CFF and WOFF are explicitly unsupported.
Checks cover directory bounds/alignment/overlap, required table presence, head
magic and units, maxp glyph counts, loca offsets and glyph header extents, hmtx
bounds and name record bounds/UTF16 decoding. Raw OS/2 embedding flags are exposed.
This is not complete glyph-program, composite-outline or cmap validation; name
format-1 language-tag metadata is not interpreted. Downstream shaping/rasterizers
must retain their own validation. No shaping or font metrics substitution occurs.

License metadata must identify source, copyright, license identifier, license
text path/hash and operator-declared embedding permission. The loader preserves
this provenance and actual text; it does not adjudicate legal rights, approve
embedding, or override OS/2 restrictions. Export consumers must enforce their
embedding policy before using bytes.

Run `cargo test --offline --manifest-path crates/font-resources/Cargo.toml`.
Tests generate structural fonts without claiming typographic fidelity. The
explicit-path `inspect` example is a smoke tool, not a fallback resolver. On this
Linux host, LiberationSans-Regular.ttf SHA256
`76d04c18ea243f426b7de1f3ad208e927008f961dc5945e5aad352d0dfde8ee8`
(410712 bytes) was accepted with 2048 units/em, 2620 glyphs and name
`LiberationSans`; no installed font is vendored or implicitly selected.

Integration handoff: rendering-core `fe29901` owns display-list cross references
and glyph-ID bounds; adapt its FontValidator using `inspect_static_truetype` and
return units_per_em/glyph_count. No dependency on rendering-core exists here.
Visual identity, PDF byte identity and typing-to-visible latency remain separate,
unmeasured integration gates. No reference LaTeX engine is used in this crate.

Unicode access: `glyph_id(char)` returns the original GID or `None` for .notdef.
It deterministically prefers Unicode format 12 over format 4, then first record
in directory order; unsupported-only cmaps return an explicit error. It checks
all selected subtable groups/segments and resulting GIDs before answering. The validated representation is initialized once per immutable resource and shared
by clones, including cached validation failures. Lookup uses binary search; no
process-global cache or filename-based identity exists. cmap_cache_key() exposes
the verified SHA256/face tuple. A maximum of 65536 ranges retains at most 768KiB
of mapping payload per resource (plus small allocation metadata).
`horizontal_metrics(gid)` returns original unsigned advance and signed bearing,
including the repeated final advance for trailing hmtx bearings.

Composite validation runs during metadata inspection and resource loading. It
bounds component record arguments/transforms/instruction extents, rejects absent
child GIDs and graph cycles, and caps dependency depth at 32 edges and aggregate
component references at 1,000,000. Shared acyclic subgraphs are permitted; cached
subtree heights cannot hide an over-depth ancestor chain. These are explicit
experimental resource limits, not a claim to accept every valid TrueType font.
Simple-outline point data and glyph instructions are still not interpreted.

Cache evidence: 23 normal tests pass; an explicitly run release benchmark on this
Linux host measured 100000 synthetic format-12 lookups at 9.672ms rebuilding
validation versus 0.358ms cached. This is one tiny synthetic font and excludes
initialization, shaping, rendering and UI latency. Reproduce with
`cargo test --offline --release --manifest-path crates/font-resources/Cargo.toml repeated_lookup_benchmark -- --ignored --nocapture`.

`simple_outline(gid)` decodes closed simple-glyph contours into original points
and inclusive contour endpoints. Coordinates are signed 16.16 fixed-point design
units; no scaling or rounding is performed. On/off-curve flags, raw bounding box,
overlap flag and unexecuted instruction bytes are preserved. Limits come from
signed contour counts and 16-bit point endpoints (at most 65536 points), with
checked flag repetitions, instruction/coordinate extents and signed coordinate
accumulation. Missing bytes or invalid endpoints return errors. Composite glyphs
explicitly return unsupported; implied quadratic midpoint expansion, hinting,
composite transforms, shaping and rasterization are not implemented here.

Real-font smoke decoded all 1544 simple/empty glyphs of the LiberationSans digest
above, explicitly skipping 1076 composites. This proves decoder acceptance, not
visual/byte parity. Reproduce by setting FLASHTEX_SMOKE_FONT to that explicit font
path and running the ignored installed_simple_glyph_smoke test with --nocapture.

`expanded_outline(gid)` recursively expands supported composite affine transforms
and explicit XY translations, retaining root font SHA/face/ID and each original
component GID plus point range. Coordinates are normalized exact dyadic rationals
(numerator / 2^shift, accessed through methods), checked within i128 and at most
96 fractional bits. No floating rounding occurs. Expansion caps 4096 visited
instances, 1000000 leaf points and 32 dependency edges. Nonzero grid-rounded offsets and transformed nonzero offsets without an explicit
scaled/unscaled policy return unsupported. Instructions remain unexecuted.

Composite smoke on the pinned LiberationSans font accepted 1679 glyphs and
explicitly rejected 941 requesting nonzero grid-rounded offsets. These are
unsupported pending a hinting policy, not substituted or counted as complete.
`ExpandedOutline::quadratic_path()` yields deterministic MoveTo/LineTo/QuadTo/Close
commands, inserts exact implied midpoints between consecutive off-curve points,
and handles contours whose first/last points are off-curve. Original root/font
identity remains on the owning ExpandedOutline. The iterator validates contour
coverage and materializes a bounded command stream; it does not execute hints,
choose fill/rasterization rules or assert output parity.

Point-attachment placement now supports existing parent/child contour-point
indices, decoded as unsigned byte/word values. The child affine transform is
applied before computing the exact translation that makes the points coincide.
First-component attachment, missing indices and phantom-point references fail;
this API does not synthesize phantom points or execute child/parent instructions.
The semantics are explicitly unhinted design-space geometry, not the rasterizer's
post-hint placement. Source: [OpenType glyf specification](https://learn.microsoft.com/en-us/typography/opentype/spec/glyf).
Acceptance tests cover transformed attachment, byte/word indices, malformed
indices, exact scaled versus unscaled offsets and refusal to guess grid rounding.
No ppem, device grid or hint state is represented here, so grid-rounded nonzero
offsets remain unsupported. Real-font smoke remains 1679 accepted / 941 unsupported.

## Exact TeX font metrics

`tfm::Tfm::parse` is an original bounded parser for the documented TFM format.
It checks exact file/table lengths, dimensions, character indices, next-larger
cycles, extensible pieces and ligature/kern program targets and actions. Checksum
is retained as an external font identity value, not recomputed from TFM contents;
source_sha256 binds the actual bytes. `FixWord(i32)` preserves signed 12.20 values;
`at_design_size` returns an exact numerator over 2^40 in TeX points, without
pretending that this implements TeX's separate scaled-point rounding algorithm.
`char_metrics`, one-based `parameter`, and bounded `pair_action` expose typed
values and ligature retention/advance semantics. TFM codes are 8-bit encoding
slots, not Unicode or TrueType GIDs; consumers need an explicit encoding binding.
Specification reference: [TeX Live tex.web TFM format documentation](https://github.com/TeX-Live/texlive-source/blob/trunk/texk/web2c/tex.web).
No existing TeX engine is linked or invoked. No installed TFM oracle was found;
current tests use declared synthetic format fixtures, not measured font parity.

`apply_ligatures_kerns` interprets already encoded runs with exact kern FixWords
and ligature keep-left/keep-right/advance semantics. Each output glyph retains
its contributing input index interval; inserted kerns stay separate typed items.
Input is limited to 4096 bytes, output to 8192 items and execution to 65536 steps;
cyclic ligature programs fail rather than hanging. Fonts declaring boundary
programs are explicitly unsupported by this run interpreter (pair inspection
remains available). It does not perform Unicode encoding, hyphenation,
discretionaries, TeX scaled-point rounding or TrueType glyph selection.

## Explicit TFM encoding adapter

`encoding::EncodingManifest` is a typed JSON declaration (not a PostScript `.enc`
interpreter). It binds exact TFM SHA256, font SHA256 and face index to at most 256
code/name entries and 65536 declared name/original-GID entries. Names are literal
JSON strings; JSON escapes decode normally, while PostScript/PDF escape syntax is
not guessed. Duplicate codes/names, absent declarations, invalid GIDs and hash or
face mismatches fail. Distinct encoding slots may intentionally share a glyph.
`.notdef` yields the explicit GlyphIdentity::Notdef value; other names cannot map
to GID zero. A declaration is not proof that a font's name table uses those names.

`BoundTfmFont::new(tfm,font,manifest)` retains immutable borrowed resources and a
validated map. `map_code` returns original glyph identity plus exact TFM metrics;
`map_run` applies the bounded TFM interpreter then resolves all output slots,
retaining input intervals and separate exact kerns. No Unicode casting, font
fallback, hidden `.notdef` drawing, or TrueType metric substitution occurs.
The caller must handle Notdef explicitly and establish the declared encoding's
provenance. This adapter does not establish TFM-to-outline visual equivalence.

## Virtual font packets

`vf::VirtualFont::parse` preserves VF checksum/design size, raw font names/areas,
local font definitions, short/long character packets, exact signed movement/rule
units and typed glyph/font/register/stack commands. Names never trigger disk or
network resolution. Missing font references, malformed packet lengths, duplicate
IDs/codes, prohibited opcodes and unbalanced/deep stacks fail. Bounds: 16MiB file,
4096 fonts, 65536 packets, 1MiB/100000 commands per packet, 1000000 commands total,
and 64 stack entries. Specials remain explicit typed byte payloads; parsing does
not execute or silently discard them. [VF format documentation](https://github.com/TeX-Live/texlive-source/blob/trunk/texk/web2c/vftovp.web)
was consulted; no existing VF/DVI implementation is used in production.

`VirtualFont::expand_packet(code, virtual_tfm, physical_bindings)` validates the
virtual TFM checksum/design-size/packet width and each local TFM definition against
explicit BoundTfmFont resources. It emits original GID/font+TFM hashes, local font
ID and exact dyadic placement coordinates relative to the virtual font's size.
Glyph scale is a 12.20 fraction of that size. DVI y is positive downward; rule
position is its lower-left reference point and positive height extends upward.
Set/put advances, w/x/y/z registers and push/pop positions are interpreted without
float/device rounding. Font selection is not a pushed register. Specials, missing
bindings, explicit Notdef and physical codes above the supported 8-bit encoding
fail. No special-handler execution, nested VF resolver, font discovery, ligature
reshaping inside packets or silent substitution occurs. Current packet limits
bound execution and output; actual nested resource recursion is not implemented.
Synthetic integration tests establish arithmetic/binding behavior only, not
real-VF/font visual parity.

## Explicit nested VF graph

`vf_graph::ResourceGraph` extends the single-packet API with recursive virtual
resources and explicit local-font edges. Physical keys bind font SHA/face plus
TFM SHA; virtual keys bind VF SHA plus TFM SHA. Duplicate identities and malformed
or undeclared local edges fail; no pathname lookup or recursive fallback occurs.
One explicit encoding binding is accepted per physical key (conflicting duplicate
bindings are rejected). Each output preserves the root-to-leaf resource/code and
command-index chain, original physical GID, exact dyadic position and composed
scale. Child glyph advances use the child's TFM packet width at the declared
local scale; rules scale and translate through the same exact arithmetic.

Graph bounds are 4096 resources, 4096 expanded nodes, 32 nested edges, 1000000
executed commands and 100000 output placements across the whole expansion.
Repeated (resource,character) on the active stack is a cycle; acyclic reuse is
allowed. Precision/overflow errors propagate without partial output. Specials
remain explicitly unsupported. Flat-versus-nested synthetic glyph and rule
fixtures compare exact geometry and retain intentionally distinct provenance;
cycle, missing-resource, depth, node and global-output cap tests also pass.
Real licensed VF/TFM/outline oracle agreement remains pending.

## Staged CFF1 support (not production activated)

`cff::Cff::parse` accepts raw CFF table bytes from the existing public
font-engine `TrueTypeFace::cff_table()` or PDF `TrueTypeFont::cff_table()` accessor.
Reviewed peers: font-engine `2d6954b923340c788cd31f663fa8e7a845326dec`, PDF
`52b371171ee497a52ce0529dbe6bf22cda4bfe04`. Their private numeric readers are not
re-exported; existing local bounded readers are reused. No second OpenType font
selector, shaper or sfnt directory parser is introduced.

This first stage validates bounded CFF1 INDEX/DICT syntax, one-font Name/Top DICT,
CharStrings, custom charset/encoding and Private/Local/Global Subrs offsets.
Integer and decimal DICT values remain exact (decimals are preserved strings).
CID-keyed/CFF2/Expert predefined charsets are explicitly unsupported. Semantic
validation of every optional DICT operator is not claimed. Metadata acceptance
alone does not prove charstring safety or rendering support. Production font
resource schema remains static-truetype; no fallback or wire activation occurs.
References: [CFF specification](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5176.CFF.pdf)
and [Type 2 specification](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5177.Type2.pdf).

`Cff::cubic_outline(gid)` adds staged exact Type2 charstring-space MoveTo, LineTo,
CurveTo(control1,control2,end) and Close commands. It handles integer/16.16
operands, width extraction, relative/alternating line and curve forms, local/global
subroutine bias/calls/returns, and explicit closure. Limits: 48 operands, 10 active
subroutine calls, 100000 instructions and 100000 commands; recursion, malformed
arity and out-of-range calls fail. Hints/masks, escaped flex/arithmetic and seac
are explicitly unsupported. FontMatrix stays in Top DICT and is not silently
applied; cubic commands are not reinterpreted as quadratic rendering-v2 paths.

Real CFF smoke used the unchanged font-engine accessor at `2d6954b` in an isolated
/tmp harness, against installed STIXTwoText-Regular.otf font SHA
`c4864ca6ec071c2d31d0d8309001faa1ee3517fffb53a31a405a697b71f52ca1`.
CFF SHA `c5d11bab6a95e75a568e1b72fd30fdd5e4c95abe68a72f02c0c4329ee948b532`:
2221 glyphs, 824 global and 704 local subroutines; 16 glyphs accepted and 2205
explicitly unsupported due to hints/masks. No invalid-font failures occurred.
This does not establish useful visible coverage, Latin Modern support or parity;
no font file was copied into the repository and no schema activation occurred.

## Explicit unhinted CFF geometry and FontMatrix

`cubic_outline_with_policy(gid, HintPolicy::Unhinted)` validates/records stem
operands and hint/counter masks without applying grid fitting. It enforces the
96-stem limit, exact ceil(stems/8) mask payload length, zero unused bits, stem
ordering and operand counts. HintMetadata records policy, raw stem deltas/widths,
mask bytes and flex depths. The default `cubic_outline` retains Reject policy.
All four flex forms emit their two exact cubic curves under Unhinted policy;
no device-dependent flattening is performed. Arithmetic/seac/CID/CFF2 remain
explicitly unsupported. This supersedes the earlier stage's blanket hint/flex
rejection only when Unhinted is deliberately selected.

`matrix_outline(gid, policy)` applies the exact six-term FontMatrix and returns
separate MatrixCommand/MatrixOutline types. Coordinates use normalized checked
i128 Rational values with positive denominators, so decimal 0.001 is exactly
1/1000, not a binary approximation. The default matrix is applied explicitly.
Translation affects points but not the advance vector. Decimal scale/mantissa
budgets and arithmetic overflow return errors. Output is CFF font/text space,
not raw charstring units or rendering-v2 page ticks. Rasterizers must make their
size/grid policy separately; no schema or production font-profile switch occurs.

Under Unhinted plus matrix application, the same pinned STIX font above now
accepts all 2221 glyphs, with zero unsupported/invalid decoder outcomes. The old
Reject-policy results remain valid and unchanged in meaning. This proves bounded
parser/geometry acceptance only, not outline equality against an oracle, hinted
raster fidelity, Latin Modern coverage or PDF parity. Synthetic tests cover exact
masks, malformed operands, all flex forms, decimal matrices and overflow.

## Pinned CFF regression and immutable cache

`fixtures/stix-cff.json` records the installed font/OFL hashes, exact CFF table
range verified through the existing font-engine accessor, and a canonical exact
geometry regression hash. No font/license bytes are vendored. Run the opt-in
`cff_regression` test in release mode with `--ignored --nocapture`; optional
FLASHTEX_STIX_FONT and FLASHTEX_STIX_LICENSE override paths, never expected hashes.
Missing resources mean this gate remains pending, not that fidelity passed.
The canonical v1 hash streams GID u16be, advance rationals (signed numerator and
positive denominator i128be), command count u32be, then command tag 0/1/2/3 and
point rationals for move/line/curve/close. It is an implementation regression
baseline, not an independent visual oracle. Concrete hardening also rejects
stroked PaintType in the filled-outline API; mask/subroutine/matrix overflow
regressions run through the matrix-applied API.

`CffOutlineCache::from_font_table(full_bytes, face0, validated_table_range, limits)`
computes the full-font and CFF hashes, reparses immutable table bytes and retains
an internal Cff value. The existing OpenType accessor remains range/selection
authority. Keys include that fixed full identity, original GID and HintPolicy.
`lookup` returns CacheOutcome { result: Result<Arc<MatrixOutline>>, status };
status is Hit, Stored or BypassedOversize, including explicit negative results.
LRU entry/byte limits are enforced, with 512 conservative bookkeeping bytes per
entry plus actual retained Vec/String capacities and object sizes. The budget
covers cache-owned references/payload, not external Arc holders or allocator-wide
accounting. Source CFF storage has its separate 64MiB input cap. Oversized entries
are returned without retention; identities and decoded state cannot mutate through
the cache API. Max configured limits are 4096 entries and 256MiB.

Measured replay evidence is in `fixtures/stix-cff-replay.json`, with tested source
hashes. All 2221 STIX glyphs produce 55206 commands; cached/direct outputs match.
One Linux release observation: direct decoding 103.106ms, cache cold 108.647ms,
full warm replay 0.251ms, 20641359 charged retained bytes within a 32MiB budget.
These measurements exclude native rendering and are not typing-visible latency
or hinted/visual/PDF parity claims.

## CFF names and exact TFM encoding bindings

`Cff::glyph_names()` validates every glyph's charset SID against the 391 standard
SID strings or the custom String INDEX, then rejects duplicate or absent names.
The standard SID registry is format data transcribed from Adobe CFF Appendix A;
no third-party name resolver implementation is imported. Glyph names are literal
bounded ASCII PostScript names (127 bytes here), not Unicode or escape-decoded
aliases. SID strings used only as other metadata are not treated as glyph names.
GID0 resolves explicitly to Notdef. The immutable name index is shared by the CFF
outline cache and has its own conservative 16MiB metadata budget, separate from
decoded-outline retention.

`BoundCffTfmFont::new(tfm, outline_cache, CffEncodingManifest)` resolves explicit
TFM slot/name entries against actual CFF charset names, checking full-font/CFF/TFM
hashes and face. It returns exact TFM metrics and original GIDs/Notdef through
map_code/map_run, retaining input intervals. `validate_cache` checks the immutable
identity before using another outline cache. No invented named-GID declaration
is needed for CFF. Stroked/hinted/unsupported outline policies remain independent.

Opt-in `cff_names` regression verifies every pinned STIX name round-trips to its
original GID. The 2221-name canonical hash and resource hashes are recorded in
`fixtures/stix-cff-names.json`; A is GID3 and the literal name fi is absent.
A missing alias fails rather than being guessed. This is charset/name evidence,
not TeX encoding correctness or visual/raster parity.

`CffEncodingCache::new(&outline_cache, CacheLimits)` pins an immutable full-font,
CFF table range and face identity. `lookup(tfm, manifest)` validates identity before
reuse and returns `EncodingCacheOutcome { encoding: Arc<ResolvedCffEncoding>,
status }`. Canonical sorted slot/name declarations share entries regardless of
input order; different TFM hashes or declarations do not. Missing names and invalid
identities are errors, not negative fallback entries. `BoundCffTfmFont::from_resolved`
consumes the shared binding; `validate_cache` checks the eventual outline consumer.
LRU retention permits at most 128 entries and 8MiB of conservatively charged entry
payload/bookkeeping, with explicit oversize bypass and zero-budget operation.
The shared name index has its separate 16MiB bound; caller-retained Arc references
and allocator-wide memory are outside cache ownership. No shaping or native render
performance claim follows from an encoding cache hit.

Type2 arithmetic follows Adobe Technical Note5177 sections4.4–4.6:
https://adobe-type-tools.github.io/font-tech-notes/pdfs/5177.Type2.pdf
The original decoder now supports exact add/subtract/multiply, absolute/negation,
logic/comparison/ifelse, dup/exch/index/roll/drop, and 32 transient slots shared only
within one glyph and its subroutines. Reads before writes fail. Division accepts
exact dyadic quotients and sqrt accepts exact dyadic roots; non-dyadic quotients,
irrational roots and random return `UnsupportedFont`, never approximate geometry.
The existing Coordinate profile remains unchanged. Checked numeric overflow,
invalid indices, zero division, operand underflow, 48-stack and 100000-instruction
budgets fail explicitly. Subroutine cycle/depth limits remain active. This is not
an emulation of device arithmetic rounding, hinting, or a raster-fidelity claim.

`FontResource::expanded_outline_with_grid(gid, CompositeDeviceGrid { ppem_x,
ppem_y, tie_rule })` adds explicit device-context rounding of composite XY offsets.
It returns `DeviceExpandedOutline { outline, grid, units_per_em }`; the inner
outline retains original font SHA, GIDs and component point provenance and can
produce its existing quadratic path. The default `expanded_outline` retains its
unsupported outcome when nonzero rounded offsets lack context.

The official glyf specification requires offset transformation before nearest-pixel
rounding: https://learn.microsoft.com/en-us/typography/opentype/spec/glyf .
This implementation assembles children before applying the parent component
transform, and ignores the round flag on point attachments. Callers explicitly
choose `AwayFromZero` or `TowardPositive` half ties; neither is advertised as a
complete rasterizer's instruction-controlled rounding state. Integer ppem values
1..65536 are supported per axis. Exact rational scaling/rounding is converted back
to the existing dyadic design-coordinate API; a non-dyadic final offset is explicit
`UnsupportedFont`, never approximated. Ambiguous default scaled-offset policy stays
unsupported. No TrueType instruction interpreter, phantom-point generation, or
hinted/visual parity is implied.

A device outline cache MUST bind font SHA/face, GID, units-per-em, both ppem values,
tie rule, offset policy and decoder build/profile identity. Do not place this
output into a size-independent outline cache. Preserve the wrapper until the
consumer checks context; `outline.quadratic_path()` alone intentionally carries no
device-context field. Current font caches are process-local; persistent artifacts
also need exact build identity. A pinned LiberationSans replay at ppem16 accepts
2620 glyphs versus default1679 plus941 explicit grid-context rejections.

`engine_adapter::EngineFontAdapter` reuses the original sibling `font-engine`
parser and shaper (reviewed peer `f418238899d1218b17cff9a33601790a3057949a`, default
features disabled). `from_resource` binds an immutable licensed TrueType resource;
`from_cff(bytes, &outline_cache)` verifies full-font and selected CFF-table identity.
No font lookup, fallback, GSUB/GPOS implementation, or TFM code conversion is added.
`ShapeRequest` requires the matching raw font SHA/face, full source text/hash,
path/revision and selected UTF8 byte range. Only explicit static Unicode context
is accepted; variation axes/selectors, TeX eight-bit encoding, missing glyphs and
non-strict requested features fail. Engine unsupported notes remain on output.

`BoundShapedRun` exposes immutable identity/source/shaped views. The engine's
FontId SHA hashes bytes PLUS big-endian face index; the raw resource SHA hashes
bytes alone. Both are verified and retained. Original GIDs, integer font-unit
advances/offsets, exact cluster text/ranges, ligature counts and composition source
ranges are preserved unchanged. Cluster ranges remain relative to the selection;
`absolute_cluster_range` adds its source base. No subset GIDs or invented source
ranges enter this API. Page conversion requires a separate exact size/UPEM and
y-axis policy; TFM spacing/encoding and device-rounded outlines remain separate.

The returned cache key binds both font identities, source hash/path/revision/range,
all shaping flags, adapter source fingerprint and the actual linked peer source
fingerprint generated by build.rs. `ENGINE_PEER_REVISION` is the reviewed baseline;
`ENGINE_SOURCE_SHA256` reflects the current compiled sibling source. Keys are not
a cache implementation or whole-toolchain binary provenance: persistent artifacts
must additionally bind the exact compiler/build configuration.

Opt-in `--test engine_adapter -- --ignored --nocapture` checks pinned STIX and
LiberationSans bytes/licenses against direct peer shaping. STIX mapping-only:
18 clusters/glyphs,7704 font units; default liga explicitly rejects unsupported
GSUB lookup14 type6. LiberationSans default:10 clusters/glyphs,1 ligature,9752 font
units with combining composition. These are adapter equivalence observations,
not reference-engine, hinted-raster, PDF-byte or sub200ms typing-visible evidence.

`registry::ProjectFontRegistry::load(&ProjectRoot, "fonts.json", RegistryLimits)`
loads an explicit project font manifest through the original `project-files`
rooted read API (reviewed tree d92db378f25cc0b882a7a45e0a83a96f58ca1306), refusing
symlinks at the file and parent-directory levels. Registry paths must be canonical
project-relative names with no parent traversal. There is no recursive discovery,
family-name guessing, case folding, system-font fallback or font-byte vendoring.

Schema1 contains `entries: [{ binding: { family, weight, style }, resource: ... }]`;
`resource` is the existing `ManifestEntry`, declaring font ID/path/full SHA/face,
metrics and license path/hash/provenance. Style is `upright`, `italic` or `oblique`,
weight1..1000, and family lookup is exact. Registry resources accept `static-truetype` and `static-cff`, both face0; their
backends remain explicit and separate from rendering-v2 wire negotiation.
Declared style is an application binding, not an inferred OpenType style.

Typed outcomes distinguish missing files/bindings, ambiguous duplicate bindings,
rooted-read refusal, resource mismatch, budget failure and stale generation.
Hard caps are128 entries,257 reads including manifest,1MiB manifest and256MiB total
read bytes, with existing64MiB/font and license limits. Repeated declarations are
charged for each read and immutable retained copy; no unbounded cache is hidden.

`get` returns a shared immutable `Arc<FontResource>` suitable for the existing
shape/outline adapters. `discovery()` exposes sorted serializable declarations
without font bytes. `generation()` hashes canonical sorted binding/resource/license
metadata after verifying all declared bytes. JSON whitespace and entry order do
not change generation; source identity or style/provenance changes do. Reloading
builds a fresh snapshot, and `require_generation` checks a consumer's expected
version. Prior snapshots remain valid immutable objects after external changes;
callers must reload/check at project-update boundaries. Reads are individually
rooted, not a transactional snapshot of all project files. Consumer caches should
bind project-instance identity plus registry generation and their existing exact
shape/device/build keys; equal generations do not authorize cross-project access.


`ProjectFontRegistry::resource` returns `RegistryResource::{TrueType,Cff}` with
shared immutable backend resources. The existing `get` accessor stays TrueType-only
and explicitly refuses CFF. `CffFontResource` exposes verified descriptor/license,
full font bytes, exact CFF table SHA/range/face identity, `outline_cache(limits)` and
`shape_adapter()`. The original font-engine accessor selects the CFF table and
validates metadata; the existing CFF parser validates its container/glyph count.
No sfnt/CFF parser is duplicated. Cache construction retains explicit budgets and
Unhinted policy requirements. Registry generation covers both backend declarations
and full-font/license hashes. Synthetic and pinned mixed STIX/Liberation tests
verify stale-generation rejection and unchanged held CFF geometry after project
font bytes change. No fonts are copied into the repository; installed-font tests
use disposable temporary project directories.

Registry transport version2 adds `generation` and per-entry `cff_table`:
`{ sha256, offset, byte_length }` for CFF, `null` for TrueType. `export_manifest()`
returns typed data; `export_json(max_bytes)` produces deterministic compact JSON
through a bounded writer (hard1MiB cap). Export includes declarations and identities,
never font/license bytes. Callers may save through the existing rooted atomic-save
API; serialization itself has no filesystem side effects. Existing schema1 inputs
remain accepted. Version2 import uses the same rooted load and resource validators,
then checks the claimed generation and exact selected CFF table against those
verified bytes. Unknown fields and duplicate JSON keys are rejected by direct
strict-struct deserialization, without a map intermediary that overwrites keys.

`enumerate(expected_generation, MetadataFilter { family_prefix, weight, style },
offset, limit)` returns a serializable `MetadataPage` with generation, total matches,
next offset and typed export entries including CFF table identity. It scans only
the bounded declared registry: exact case-sensitive prefix/style/weight filtering,
1..64 results per page, offset<=128, prefix<=256bytes. Stale generation is refused
before enumeration. No filesystem discovery or implicit fallback occurs. Roundtrip
and pagination tests cover both synthetic resources and the pinned mixed licensed
STIX/Liberation registry; no native wire negotiation or visual parity is implied.

TFM run interpretation now honors implicit left/right boundaries from the first
and last lig/kern marker records. The authoritative format rules are in TeX's
TFM specification, `tex.web` sections on the lig/kern array:
https://raw.githubusercontent.com/TeX-Live/texlive-source/trunk/texk/web2c/tex.web
The original interpreter uses distinct invisible boundary sentinels; these are
never emitted as glyphs or cast to Unicode/GIDs. Left-boundary programs and
right-boundary matching support exact ligature retention/advance and signed kerns.
A real encoded glyph matching the boundary byte remains a real glyph. Replacement
intervals cover participating real input; boundary sentinels contribute only
zero-length start/end intervals. Missing input glyphs are rejected.

`apply_ligatures_kerns_with_boundaries(input, BoundaryOptions { left, right })`
allows explicit suppression; the existing method enables both. Both
`BoundTfmFont` and `BoundCffTfmFont` expose `map_run_with_boundaries` and preserve
explicit encoding mappings, missing-map errors, metrics and input intervals.
Empty runs produce nothing.4096 input bytes,8192 working items (including
sentinels), and65536 combined run/program steps bound malformed cycles and scans.
Cache consumers must bind TFM hash, explicit encoding/font identity, boundary
options, input bytes and implementation version. No TeX token scanning, automatic
font-run segmentation, hyphenation/discretionary reconstruction, or scaled-point
rounding is added.

Boundary-specific fixtures are hand-checked synthetic programs for left/right
kerns and ligatures, retention, suppression, cycles, invalid addresses and encoded
mapping. The existing licensed peer `ec-lmr10.tfm` (SHA cd13479f463b9a575d053dd7bf0884daa46bfdeffe4b7f537c193861652ac9e5)
provides real fi->slot28 and AV kern(-116509 fix_word) regression evidence, but it
has no boundary marker/program. No real-font boundary oracle is claimed.

`registry::vf_project::ResolvedVfProject` resolves explicit VF dependencies through
the same rooted reader and immutable project font registry. No published real VF
fixture was available; the only special fixture contains `ps`, so no special
semantics were guessed. All specials remain explicit unsupported expansion results.

Dependency schema1 binds `registry_generation`, a root node ID, and physical or
virtual nodes. Each TFM/VF asset declares path/full SHA/license provenance and
license hash; physical nodes declare a registry StyleBinding and explicit encoding
manifest. Virtual nodes map every local font ID to an explicit target node ID.
There is no path/name/font fallback. Load verifies bytes/licenses, TFM/VF headers,
local checksum/design size, complete local-ID bindings, unique resource identities,
missing nodes and dependency cycles before returning an immutable resolved project.

Limits reuse RegistryLimits (128nodes,257reads,1MiB manifest,256MiB total maximum),
plus131068bytes per TFM,16MiB per VF and32 graph-depth limit. `expand(code,
registry_generation)` refuses stale context and builds borrowed bindings for the
existing ResourceGraph exact expansion/source-chain implementation. It does not
reimplement packet execution. Physical endpoints explicitly select TrueType BoundTfmFont or CFF BoundCffTfmFont
through separate node variants; no backend inference occurs.

`generation`, `manifest`, `loaded_bytes` and retained license texts provide
recoverable provenance. Node/local-ID ordering is normalized before generation
hashing; explicit encoding declarations are preserved. Caller caches must bind
project instance, registry/dependency generations, character and implementation
profile. Synthetic rooted-project tests cover originalGID/source-chain expansion,
missing/duplicate/cyclic dependencies, changed bytes, missing licenses, symlinks,
read budgets, retained snapshots and explicit unknown-special rejection. These do
not establish real VF special or visual reference equivalence.


VF graphs now accept `Resource::CffPhysical(&BoundCffTfmFont)` with a distinct
`ResourceKey::CffPhysical` binding full-font SHA, CFF SHA, TFM SHA, canonical encoding
SHA and face. Existing TrueType keys/API stay intact. Physical mapping reuses the
bound CFF encoding and exact TFM dimensions; missing mappings and `.notdef` fail.
Nested packets preserve original GIDs, exact scaling and complete source chains.
Rooted dependency schema1 adds explicit `kind: "cff_physical"` nodes with a CFF
encoding manifest and registry StyleBinding. Assets retain the same license/read
bounds; a wrong backend/hash cannot silently resolve to another resource.

99tests and strictclippy cover flat/nested CFF equivalence, mixed TrueType/CFF
packets, explicit missing/.notdef errors and rooted CFF binding mismatch. Specials
remain unsupported. Renderer adoption requires new exhaustive-match handling:
read-only compile of rendering-core5c5e3f89dd51cc0c1517b915b95c9a65a654bd89 against
this candidate identified `tex_adapter.rs:602`, `graph_cache.rs:149` and
`mixed.rs:380`. These consumer files were not edited; no integrated renderer
compatibility claim is made until its owner publishes that adaptation.

### Registry-bound MATH

`math_adapter::BoundMathFont::from_registry` reuses font-engine's existing MATH
parser, binding immutable registry generation, explicit style, full declaration
(including project font path and license), original engine/full-font/face identity,
and MATH table SHA/length. `constants` preserves all 56 exact integer fields;
`glyphs` accepts at most 256 original GIDs and rejects out-of-range IDs.
`MathPolicy::UnhintedDesignUnits` is mandatory. Device adjustments (including their
validation), variants, math kerns and extended-shape coverage remain typed
unsupported capabilities. Missing accent data stays `None`; italic absence uses
the peer's zero default. No font selection fallback or new MATH parser is added.

`scale_design_units` takes a positive exact rational size and returns exact lengths;
`percent_ratio` treats percentage constants as dimensionless. Neither performs
hinting, device rounding, TeX layout, or proves visual/PDF-byte parity.

The opt-in `math_adapter` integration test pins installed STIXTwoMath-Regular.otf
SHA `3a5f3f26f40d5698b3c62dd085d48d6663696a3f80825aab8b553d5097518e8c`
and its OFL license. Observed MATH table SHA
`0af4bf095e9d3a968b460b83d589b5c0a6dc58762fe1f3cb3dece03fd5344c4a`,
27408 bytes: all 6760 GIDs agree exactly with the reused peer parser. This is
adapter-equivalence evidence, not an independent layout oracle. Synthetic tests
cover missing MATH, signed/unsigned metric boundaries, lookup bounds, stale
registry generations, and exact scaling/overflow.

`BoundMathFont::variants()` separately parses the previously unsupported variants
subtable and returns `BoundMathVariants` retaining its complete parent identity.
The constants-only `require(Variants)` remains unsupported; use this explicit
fallible extension. `MathVariants` preserves original GIDs, variant advances,
assembly italic correction, connector lengths, extender flags and direction.
Limits: 4 MiB table, 4096 constructions, 65536 aggregate variant/part records.
Coverage formats 1/2 are checked locally because the peer helper is private.

`select` returns the first ready-made variant meeting a requested integer advance,
`AssemblyRequired`, or `Unavailable`. `assemble` accepts caller-selected equal
extender repetitions and exact per-join overlaps; it enforces connector bounds
and returns exact design-unit positions and original part/instance provenance.
It caps repetitions at1024 and output at4096 parts. This is not automatic target
fitting or baseline placement. Device corrections remain explicitly unevaluated;
nonzero device offsets are range-checked, but device table bodies are not parsed.
Pinned STIX Math replay observes165 constructions,633 variants and69 assemblies.
Connector lengths are preserved even when longer than advance: these are distinct
font measurements. Semantics reference:
https://learn.microsoft.com/en-us/typography/opentype/spec/math

`BoundMathVariants::fit` adds exact target fitting with the explicit strategy
`EqualExtendersProportionalConnectorFlexibility`. It chooses a sufficient
ready-made variant first. Otherwise it tries equal extender counts in increasing
order, computes each valid extent interval, and distributes overlap reduction
proportionally to available connector flexibility using checked rationals.
A successful assembly reaches the target exactly; a variant can exceed it.
No overlap exceeds either connector or falls below the font's minimum overlap.
Caller limits may only tighten the1024 repetition/4096 part ceilings. Outcomes
separate invalid inputs, absent construction, unrepresentable size, exhausted
budget and arithmetic failure. Repetition budgets do not imply a font is invalid.

Bound results retain the full MATH resource identity, direction, requested target,
base GID, original part indices and repeat-instance provenance. Source-document
selection and vertical baseline placement remain the consumer's responsibility.
No hinting or general TeX delimiter parity is claimed. Pinned STIXMath at5000.5
units gives66 accepted/3 refused assembly-bearing constructions (horizontal
GIDs1510,1514,1532 exhaust this strategy's budget). At5000 units, parentheses,
brackets and integral GIDs1064–1067/1698 all fit. Synthetic tests hand-check exact
fractional and unequal connector overlaps, extent boundaries, gaps, impossible
joins, output/repetition limits and arithmetic overflow.

`BoundMathFont::kerns()` independently decodes MathKernInfo and returns immutable
`BoundMathKern` with the same full resource identity. The original constants/glyph
consumer is unchanged. Original-GID/corner queries use exact rational heights and
binary upper-bound lookup: equality at a correction height selects the following
kern interval. An absent glyph/corner entry returns the specified zero correction;
out-of-font GIDs fail. Values remain signed design units. Parsing caps coverage at
4096 glyphs and aggregate correction/kern records at65536, with strict increasing
heights and bounded table/coverage/device offsets. Device adjustment bodies remain
explicitly unevaluated and unvalidated; presence is exposed with each value.

Pinned STIX Math has219 corner tables and217 correction heights, with no device
records. Replay tests every exact boundary and half a unit below it. Synthetic
fixtures cover negative values, missing corners, truncated prefixes, invalid GIDs,
counts/offsets/heights, device-offset bounds and rational comparison overflow.
This establishes data/lookup behavior, not a script-placement or visual oracle.

`math_device::DeviceTable` decodes OpenType packed signed2/4/8-bit pixel deltas,
validating the entire declared payload and zero padding before answering a query.
VariationIndex0x8000 and unknown formats return typed unsupported errors, including
when the requested ppem would otherwise lie outside the range. `DeviceContext`
requires an explicit positive integer ppem. Out-of-range valid Device data gives
zero as specified. No implicit point-size/axis/pixel conversion is performed.

`BoundMathFont::constant_device(ConstantDeviceRecord,DeviceContext)` exposes this
additively for the51 MathValueRecords in constants, reusing the peer's constants
parser and original raw table selection. A result retains parent identity, record
index, context, pixel delta and exact referenced device-table hash/offset. The
unhinted API remains unchanged; other MATH value families are not automatically
device-adjusted. Synthetic tests establish signed packing, word boundaries,
truncation, padding, parent-relative offsets and variation rejection. Installed
STIX/Noto Math constant inventory observes no positive device records; exact
font/table/license hashes are in `fixtures/math-device-inventory.json`.
OpenType contract: https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2#device-and-variationindex-tables

`glyph_device` and `kern_device` add identity-bound corrections for glyph italics,
top accents and corner kerns. Raw design units remain separate from pixel deltas.
Glyph value extraction is checked against the existing peer MATH parser. Kern
queries reuse the original parsed corner records and Device decoder; horizontal
and vertical ppem are explicit separately. Correction-height deltas are converted
exactly into design units using vertical ppem/UPEM before upper-bound selection;
selected kern deltas use horizontal ppem. Crossed corrected heights are refused,
not sorted into a different font program. At most4096 correction heights are
visited per query. VariationIndex remains unsupported even outside the size range.

The expanded pinned STIX inventory finds four actual glyph device records: top
accent GIDs3309/3316/3326 and italic GID4010. At12ppem, GID3326 has a +1pixel accent
correction; all four exact device hashes are retained in the inventory manifest.
All6760 glyph italic/accent values still match the peer's raw metrics. Synthetic
fixtures additionally verify negative pixel deltas, corrected-height equality,
outside-range zero, variation refusal and nonmonotone corrected-height refusal.
This is metric correction only; it neither grid-fits outlines nor proves raster
or TeX script-layout parity.

`math_cache::MathQueryCache` borrows one immutable `BoundMathFont`. Its namespace
is the complete registry/style/declaration/font/engine/MATH identity, checked on
every call; query keys include record/GID/corner, rational height/target, ppem,
strategy and fitting limits. Replies retain that identity and share immutable
`Arc<Result<Value,QueryError>>`, including typed negative outcomes. Stale identity
is refused before lookup. The caller must supply the new identity after a registry
reload; this cache does not discover external file changes.

Variants and kerns reuse one bounded parsed table per cache when budget allows.
LRU query retention has separate entry/byte caps; oversized values bypass storage.
Defaults are128 entries,8MiB query charge and8MiB parsed charge; hard limits4096
entries/64MiB/16MiB. Conservative charges include preallocated queue storage,
result payloads and parsed vector/tree storage. Borrowed font bytes and externally
retained result Arcs are outside the cache's ownership/budget. These caches are
process-local and cannot outlive their borrowed font or cross decoder builds.

Pinned STIX GID3/TopRight height0 ppem12 replay observed100 identical queries
reduce from100 kern parses to1 parse/99 hits, with exactly equal outputs. One Linux
debug run measured56.27ms uncached versus0.618ms cached; host-load variation was
substantial, so tests assert no timing threshold or native performance claim.
`fixtures/math-cache-replay.json` preserves measured source/hash/context evidence.
Tests also cover device-context changes, exact fitted-shape equality, stale
identity, eviction, disabled budgets, and cached VariationIndex refusal.

### Explicit collection consumer and remaining resolver dependency

`registry::collections::CollectionRegistry::load` is a separate opt-in consumer
requiring a `VerifiedCollectionResolver`. No built-in resolver is supplied and
no existing rendering-v2 registry format is silently broadened. The resolver must
provide a complete, structurally verified collection layout; completeness is a
trusted parser contract, not something this consumer can infer from partial data.
The consumer bounds128 faces/256 tables per face, validates supplied directory and
table bounds/overlaps, permits exact same-tag shared tables across distinct faces,
and compares every supplied selected-face table range with the existing peer
accessor. Rooted reads, full collection SHA/face index, original engine ID, exact
table SHA/ranges, descriptor, license and stale-generation gates are retained.

Reviewed peer `8080c90` exposes `parse_with_source` and `table(tag)` but lacks
complete validated directory enumeration (and does not reject duplicate tags or
partial overlaps itself). Its `truetype.rs` is byte-identical to the currently
linked original reader: SHA
`2aca31389307f63cb8df88388dea0810b291d44ba0702b56fa5cbc0db78e0b77`.
The concrete dependency is an original-engine API returning every face directory
and table range after checking TTC header/version/counts, duplicate records and
shared-table rules. This crate does not add another directory parser to fill it.
Collection resources expose the original selected `TrueTypeFace`; existing
single-face outline/cache adapters remain unchanged.

Synthetic two-face fixtures establish explicit face identity, table matching,
shared-table versus partial-overlap checks, bounds and immutable stale snapshots.
The installed NotoSansCJK-VF.ttc inventory (font SHA
`d3d8256cdec8dbcb3552284bc6b20c734dd60c2ee9df83b5758e34807c4bac32`)
currently yields peer `MissingTable("CFF ")` for face0. Its license hash and exact
refusal are pinned in `tests/collection_inventory.rs`; this is not successful
collection-rendering evidence. Production collection activation awaits the
verified resolver and downstream outline support.

### Literal .enc files

`enc_file::EncFile` parses one bounded declaration:
`/EncodingName [ /glyph0 ... /glyph255 ] [readonly] def`, with ASCII whitespace
and `%` line comments. The brackets around `readonly` here mean optional grammar,
not extra file tokens. There must be exactly256 positional names; duplicate array
definitions, extra/missing slots, numeric assignment, strings and unknown operators
are rejected. Repeated names (especially `.notdef`) are valid and do not duplicate
slots. Names are preserved literally; no escape decoding, execution or evaluation
occurs. Input is capped at64KiB and names at256 bytes.

`load` uses rooted no-symlink reads with an expected file SHA. Binding wrappers
preserve the encoding-file SHA/name/project path alongside existing exact
`BoundTfmFont` or `BoundCffTfmFont`. TrueType mappings still require explicit named
GID declarations; CFF uses its already validated original-name index. Missing
names and `.notdef` semantics come from those binders, with no Unicode casts or
fallback. Consumers must include the wrapper's file identity in provenance/cache
keys when source-file identity matters, even if two files declare equal slots.

The peer has no runtime `.enc` parser; its fixture-generation Python utility has
a narrow source-extraction helper. The published licensed `lm-ec.enc` (SHA
`7f9932c402d22a937b853406cfdf4166b80260e3ff21a03fe9a4c05105a2918c`)
is parsed as256 slots and compared against all253 mapped slots in the peer's
encoding manifest. The existing GUST license is hash-verified by the test. No
reference bytes are copied and no production PostScript engine is introduced.

Rooted TFM/VF dependency schema2 adds `physical_encoding_asset` and
`cff_physical_encoding_asset` node kinds. Each specifies the existing licensed
`Asset { path, sha256, license }` as `encoding_asset`, plus TFM asset and explicit
registry binding. TrueType also requires `declared_glyphs`; CFF uses its verified
name index. Slot maps are loaded through `EncFile` and existing binders. Schema1
inline nodes remain unchanged; new node kinds are rejected under schema1.
Asset reads share existing byte/file/license budgets. Project generation includes
the exact encoding asset path/SHA/license; ResourceKey variants are unchanged, so
consumers must retain the project generation when caching across loaded projects.

`ResolvedVfProject::physical_run` exposes the existing bounded TFM ligature/kern
mapping for one loaded physical endpoint, returning project/registry generation,
node ID, exact TFM words and original8bit input intervals. Virtual endpoints remain
explicitly unsupported by this run API; their existing packet expansion is intact.
Tests exercise both TT/CFF asset nodes, nested expansion, file-hash refusal and
schema gating with synthetic fonts. Existing real Latin Modern TFM and encoding
fixtures are hash-pinned, including `fi`->slot28 and AV kern-116509 evidence.

The full matching-font test is deliberately opt-in:
`FLASHTEX_LM_FONT=/exact/lmroman10-regular.otf cargo test --offline --manifest-path crates/font-resources/Cargo.toml --test rooted_lm_encoding -- --ignored`.
It requires font SHA `1aa18cfefa58132c52ce5de70db1fd1154201c19cd2b2cdaffba4906a33e6852`
and verifies exact TFM/encoding/license hashes, rooted loading, fi interval0..2,
AV intervals0..1/1..2 and exact kern. The exact matching font was subsequently obtained from the official GUST
distribution into a temporary evidence directory; the test now passes with the
explicit declarations described below. No substitute font is used.


CFF encoding-asset nodes may explicitly opt into `declarations`, binding the
encoding-file/full-font/CFF hashes and face index. Each alias declares its literal
source name, literal font target name and exact original GID. Duplicate, conflicting,
unknown or `.notdef` aliases are refused. Explicit unavailable slots must match the
literal vector and actually lack a font name; requesting one fails, never substitutes.
The default without declarations remains strict. Resolved mappings retain literal
source names; their semantic digest includes the declaration hash, and rooted
project generation retains the entire declaration and licensed asset provenance.

The official GUST archive supplied exact font SHA1aa18cf...; the initially failing
full rooted test exposed the documented five LM ligature aliases and three missing
slots. With those exact fixture-bound declarations it now passes fi->originalGID125,
input0..2, AV kern-116509 with intervals0..1/1..2, and all three unavailable-slot
refusals. `fixtures/rooted-lm-download.json` preserves official URLs, archive/font/
license hashes and the initial failure. The freshly downloaded license differs in
line endings/URL spelling/whitespace from the pinned fixture; its normalized terms
agree, but the two raw hashes remain distinct in evidence. Assets are temporary,
not installed or copied into the repository.

The bounded `rooted-lm-replay.json` now records five real ligatures (`fi`, `ff`,
`fl`, `ffi`, `ffl`), three signed kern pairs (`AV`, `To`, `WA`), exact metric
fix-words, original GIDs, code intervals and rejected unavailable slots. The opt-in
matching-font test compares the complete generated value against this artifact
and checks identical repeated runs. `rooted-lm-replay-provenance.json` pins source
and artifact hashes plus the reproduction command. Regeneration writes only to
an explicitly supplied `FLASHTEX_LM_REPLAY_OUTPUT` path for review; ordinary replay
never updates expectations. This measures actual resource mapping, not independent
TeX layout agreement or visual fidelity.

Collection API followup reviewed Daniel supervisor tip
`e61c176dc71048e1fcbba2ae5df187f07cbd5ba4` and font-engine tip
`73422451ec367269d8ce2ba4554ba5103224a262`: no complete layout resolver is published.
Daniel's `src/truetype.rs` still hashes
`2aca31389307f63cb8df88388dea0810b291d44ba0702b56fa5cbc0db78e0b77`, identical to the
linked file. Lines 83–90 select one TTC directory, and lines 109–126 validate
selected table ends then insert into a map; they do not enumerate every face,
reject duplicate tags or establish collection-wide overlap safety. The explicit
`VerifiedCollectionResolver::complete_layout` dependency remains necessary.
A peer implementation must validate the full TTC header/face list and all directory
and table ranges, distinguish allowed exact sharing from partial overlaps, and
expose that verified layout without losing full collection identity. No second
parser or unverified production adapter has been added.

Fresh TTC review: `f2fdb08` (float-layout report `4422cff`) adds
`collection_layout`, but the exact compiled peer accepts unsupported TTC/sfnt
versions and table payloads overlapping the TTC header or face directory.
`tools/ttc_peer_contract.rs` reproduces the five tiny directory probes; actual
outcomes/source hashes are in `fixtures/ttc-peer-contract.json`. Consumer tests
independently enforce header/directory/table separation, same-tag exact sharing
and128face/256table caps. Those post-parse caps cannot bound the peer's prior work.
The resolver gate therefore remains: the peer must return a validated header range
and directory ranges, reject unsupported versions/DSIG as appropriate, and enforce
hard caller-supplied face/table/work caps before allocation and cross-face scans.
No header decoder was duplicated and no peer dependency was activated prematurely.
