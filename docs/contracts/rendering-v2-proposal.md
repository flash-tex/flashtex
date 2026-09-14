# Rendering v2 proposal: one positioned display list for preview and PDF

Status: **proposal only; not an authoritative wire contract or implementation assignment**.
Owner for review: Commander / FT-001. Author: commander-rendering (hosted Codex).
Review requested from compiler lead, Jaysen/PDF owner, and Mac preview owner through
Sol before any consumer changes. Existing runtime-v1 remains authoritative.

## Evidence and present fidelity blockers

Reviewed source revisions (full SHAs, not assumed integrated):

- Compiler: `01fb13cc65dc6e36f2989b3db06953a7559b0e6c`.
- Rust PDF: `1ea426103b1e24e70de4ee103772021de8b137b9`.
- Mac shell: `983b627b76a60a5aba7b2287797baa74cd1db70f`.
- Initial runtime-v1 contract on proposal base main `342e1e029f80e7a5b472ad202ae87d597274127b`.

| Observed implementation | Consequence | Required information |
|---|---|---|
| `crates/compiler/src/layout.rs` chooses Times-Bold for non-12pt sizes, but `TextItem` and `protocol.rs` serialize no font identity | Bold heading measurements become Roman outlines in PDF/native preview; size cannot encode style | Exact font resource, face, instance and glyph identity |
| `metrics.rs` carries unkerned advances and substitutes 500 units for unsupported scalars | Line breaking and word positions can disagree with Symbol or embedded Unicode glyph advances | Compiler-owned shaping and font metrics; consumers must not remeasure |
| `crates/pdf/src/writer.rs` selects Times-Roman/Symbol/optional embedded fallback and lets `Tj` advance between runs | Positions within a word depend on consumer font choice; fallback substitution can move later glyphs | Explicit per-glyph origins using the same font bytes |
| `embed.rs` optionally discovers a system font independently of compiler layout | Output varies by machine and can embed different metrics from those used for layout | Content-addressed font resources selected upstream |
| `math.rs` emits fraction bars as repeated U+2500 text; PDF recognizes strings of that character and draws rectangles with hardcoded 0.5em widths and 0.06/0.7 thickness | Geometry depends on a text-pattern convention; literal box-drawing text is indistinguishable from a rule | Typed rectangles/rules with explicit dimensions |
| Math script shifts, axis, fraction spacing and script scales are constants | Matching renderers cannot make these approximate layout decisions match reference typography | Font math parameters and compiler layout work, separate from the wire change |
| Native `PreviewView.swift` creates `Font.custom("Times-Roman", ...)`, measures SwiftUI `Text`, and subtracts its measured first baseline | Platform shaping/font resolution can disagree with PDF while each item's nominal baseline still matches | Draw explicit glyphs at compiler coordinates; use supplied hit geometry |
| runtime-v1 has one UTF-8 source span per text item | Ligatures, combining marks, macro expansion and multi-file source selection cannot be represented faithfully | Logical text clusters plus explicit source provenance |

A richer contract is necessary, but does not itself deliver TeX-compatible line
breaking, hyphenation, font loading, math layout or package behavior. Those remain
compiler requirements. No reference TeX engine belongs in the production pipeline.
A separately labeled development oracle may compare outputs without generating the
product's document. Production output must come from the original Rust compiler.

## Proposed contract boundaries

Keep the existing request/revision correlation envelope. Negotiate the display
representation separately: a renderer advertises `render_formats:["display-list-v2"]`
and a compiler returns the selected `render_format`. Do not send new item kinds to
an unmodified v1 consumer: the current PDF reader skips unknown kinds with warnings,
which could otherwise yield a deceptively incomplete document.

The v2 payload contains project ID, revision, document-content digests, font resource
references, pages and diagnostics. Every page and resource belongs to that exact
revision. A renderer publishes a new frame only when all referenced resources are
verified; until then it may retain the old frame with an explicit stale indicator.
A missing font or unsupported primitive is a render failure, not a silent fallback.

One Rust-produced display list drives both PDF export and native preview. The
compiler owns shaping, kerning, ligatures, fallback choice, line breaks, math boxes,
page breaks and placement. Consumers paint the supplied list without reshaping,
adding kerning or choosing a replacement font. Retain semantic text for search,
accessibility and source navigation; semantic text never determines paint geometry.

## Coordinates, baselines and paint order

Proposed canonical unit: signed integer ticks with **1 PDF point (1/72 inch) =
1,048,576 ticks** (`coordinate_unit:"bp_2pow20"`). Coordinates use page top-left;
positive x goes right, positive y goes down. Keep JSON integers within ±(2^53−1)
and use checked arithmetic. Page extents and rule dimensions are nonnegative;
all font sizes are positive. Reject out-of-range coordinates and malformed geometry.

Convert internal layout coordinates once when emitting the display list, with a
documented round-to-nearest/ties-to-even rule. Do not independently round every
consumer transform. TeX points and PDF points are different: when the compiler
uses TeX scaled points, the conversion is
`PDF points = tex_sp × 7200 / (7227 × 65536)` before conversion to ticks.
The current v1 numeric `_pt` fields mean PDF points; migration must state that
explicitly rather than introducing a 72 versus 72.27 mismatch.

A glyph origin is its font baseline origin in page coordinates. PDF uses
`x_pdf=x`, `y_pdf=page_height−baseline_y`; a rectangle uses
`y_pdf=page_height−top−height`. Native rendering applies the same scale and page
transform once. Font outline coordinates remain in the font's own units with its
normal positive-up axis; the renderer handles that axis flip when drawing outlines.
Do not use a platform's measured first baseline to reposition the supplied origin.

Display-list order defines paint order. Initial v2 covers glyph runs and filled
rules/rectangles, each with explicit RGBA paint. Clipping is the page bounds unless
an explicit clip primitive is later negotiated. Rotation, arbitrary paths, images,
patterns and transparency groups require declared extensions; do not approximate
or silently ignore them. This staged proposal does not remove those product goals.

## Immutable font resources

Each `font_id` identifies exact font bytes plus face index and instance. The resource
manifest includes SHA-256, byte length, format, face index, units per em, PostScript
name for diagnostics, and glyph count. Use a bundled, redistributable font asset
with a recorded license for reproducible tests. Platform font names alone are not
identities. The compiler chooses any fallback font and emits a separate run for it.

For the first implementation, limit negotiated resources to static TrueType outlines.
If variable fonts are used, produce and hash a static instance before layout and
rendering; do not independently instantiate axes in each consumer. CFF, color fonts
and other formats require additional capability negotiation. Resource transport is
a separate bounded local interface: only approved resource IDs can resolve files;
no arbitrary paths, remote URLs or network font discovery from document content.

Display glyph IDs index the original manifest font's glyph order. PDF subsetting
must retain an explicit original-GID → subset-GID mapping. CID and GID are separate
namespaces; `/CIDToGIDMap /Identity` is valid only if the writer deliberately makes
that relationship true. Multiple CIDs may refer to the same glyph with different
logical text mappings. A font resource mismatch invalidates the page cache.

## Glyph runs, logical clusters and source maps

A `glyph_run` contains `font_id`, size in ticks, logical UTF-8 `text`, ordered glyph
records, and clusters. Every glyph supplies its font glyph ID, **absolute** baseline
origin, advance vector and cluster index. Absolute origins are authoritative for
painting; advances support caret/layout diagnostics and must not be added again.
Kerning and combining-mark offsets are already incorporated. Combining glyphs may
have zero advance; ligatures may cover multiple Unicode scalars. Glyph zero is an
explicit missing-glyph diagnostic unless intentionally used by the font contract.

Clusters identify end-exclusive UTF-8 byte ranges in the run's logical text and a
list of source ranges `{path,start_byte,end_byte}` into declared document revisions.
Multiple glyphs may share a cluster; multiple source ranges may describe expansion.
Generated content uses an explicit synthetic provenance reason. Do not fabricate
source offsets by equating rendered text with the TeX substring. Logical cluster
order is independent of visual glyph order, allowing future RTL support.

Each cluster supplies hit/caret geometry from the compiler. The Mac uses those
bounds and source ranges to navigate, after confirming the source revision or a
validated rebase. Ligature carets need explicit positions or a documented whole-
cluster selection fallback; dividing glyph width by character count is not exact.

A cluster's `hit_rects` are its **laid-out TeX boxes**, not the extent of the
outlines painted inside them: `width` is the advance TeX positions the next atom
from (the TFM width, pdfTeX's `/Widths`), `top` is the baseline minus the box
height, and `height` is the box height plus its depth. That is the definition
`$defs.cluster.hit_rects` now carries in `protocol/rendering-v2.schema.json`,
and it is what "the agreed cluster geometry" below means. It matters in both
directions: a consumer must be able to hit-test a space and draw a selection
band of line height, and a lane measuring a delimiter against pdfTeX's
`\showbox` must get the box. The painted outline's own bounding box is a
different measurement — it can fall short of the box or run past it — and
travels, when negotiated, as the separate `ink_rect`
(`protocol/proposals/display-list-v2-ink-rect.md`).
PDF text extraction uses a ToUnicode mapping for the cluster's Unicode sequence.
Combining sequences and duplicate outlines need extraction tests, not just visible
rendering tests. A PDF embedding implementation that maps one glyph to only one
Unicode scalar cannot claim this gate passed.

## Illustrative examples

These are schema sketches, **not font fixtures or authoritative sample glyph IDs**.
The IDs below must be replaced by IDs derived from a checked-in test font before
using the examples as executable acceptance fixtures. `demo-roman` is a resource
reference, not permission to resolve a platform font by name.

```json
{
  "kind": "glyph_run",
  "font_id": "demo-roman",
  "font_size": 12582912,
  "text": "AV",
  "glyphs": [
    {"gid": 36, "origin_x": 75497472, "baseline_y": 88080384, "advance_x": 7340032, "advance_y": 0, "cluster": 0},
    {"gid": 57, "origin_x": 82837504, "baseline_y": 88080384, "advance_x": 8388608, "advance_y": 0, "cluster": 1}
  ],
  "clusters": [
    {"text_start_byte": 0, "text_end_byte": 1, "sources": [{"path": "main.tex", "start_byte": 0, "end_byte": 1}]},
    {"text_start_byte": 1, "text_end_byte": 2, "sources": [{"path": "main.tex", "start_byte": 1, "end_byte": 2}]}
  ],
  "paint": {"r": 0, "g": 0, "b": 0, "a": 1}
}
```

The sketch omits hit/caret rectangles to keep the positioning example readable;
a complete fixture must include the agreed cluster geometry. Here 72 points is
75,497,472 ticks, and the baseline is 84 points. The second origin already contains
whatever kerning the compiler selected. A renderer must not kern `AV` again.

```json
{
  "kind": "rule",
  "x": 75497472,
  "top": 88080384,
  "width": 25165824,
  "height": 524288,
  "paint": {"r": 0, "g": 0, "b": 0, "a": 1},
  "sources": [{"path": "main.tex", "start_byte": 0, "end_byte": 11}]
}
```

This draws a 24-point-wide, half-point-thick rectangle. It cannot be confused with
literal `──` text. Fraction layout chooses its actual top/width/thickness upstream.

## Migration and proposed ownership gates

1. Commander gets compiler, PDF and Mac owners' agreement on units, resource
   transport, cluster schema and supported font format. Publish an authoritative
   versioned contract and real licensed fixtures; this proposal alone is no grant.
2. Compiler owner emits font identity and typed rules behind an opt-in representation.
   Preserve v1 output for existing consumers, explicitly labeled approximate.
3. PDF owner consumes exact font resources and rule geometry, then positioned glyph
   IDs with subset remapping and ToUnicode. Reject incompatible resources/features.
4. Mac owner draws the same glyph IDs/resources/positions using a low-level glyph
   drawing API and compiler-provided hit regions. Keep editing in source and stale
   revision protection. A consumer must not quietly choose a system fallback.
5. Compiler owner implements shaping/math-font/layout improvements independently;
   consumer parity tests detect placement regressions. Coordinate interface ownership
   before splitting tightly coupled shaping and font-resource code among agents.
6. Only switch the default when real source → compiler → both consumers passes the
   shared corpus. Remove temporary U+2500 rule inference from the v2 path. V1 remains
   a negotiated compatibility path until its consumers migrate.

## Consumer and fidelity acceptance tests

| Gate | Exact evidence required |
|---|---|
| Font identity | Same manifest SHA/face/instance in compiler, native and exported PDF; incorrect/missing resource yields an explicit failure |
| Heading/style | Same-size Roman/Bold and differently sized Roman examples render the intended faces; no size-based font inference |
| Kerning/ligatures | `AV`, `To`, `office`, combining `e` + U+0301: exact glyph IDs/origins/clusters from a pinned font, with no consumer reshaping |
| Unicode/fallback | Greek/math/non-BMP sequence with declared fallback runs; unsupported glyphs are diagnosed, never silently replaced or measured with a different face |
| Rules/math | Fraction, nested fraction, radical rule and literal U+2500 text remain distinct; rectangle dimensions match numerically |
| Baselines | Superscript/subscript, accents and mixed font sizes share specified baselines; PDF y-flip and native scale tests cover multiple page sizes |
| Source/extraction | UTF-8 multibyte spans, ligature clusters, macro expansion and generated symbols navigate correctly; PDF extraction preserves intended logical Unicode |
| Subsetting | Subset remaps original GIDs correctly, includes composite dependencies, and supports multiple Unicode sequences sharing an outline |
| Revision/cache | Stale display list, missing resource, same filename with changed font bytes, and source edit while rendering cannot overwrite the current verified frame |
| Determinism | Identical source/font/configuration inputs yield identical display-list geometry; export geometry agrees to one canonical tick or tighter documented serializer precision |
| Pixel comparison | Render native-generated and exported artifacts through a pinned rasterizer, fixed DPI/color settings/font resources, and compare golden images with a declared tolerance |

“Pixel-perfect” needs a named reference and rendering setup. PDFKit, CoreGraphics
and other rasterizers may differ in antialiasing despite identical outlines and
coordinates. Use geometry/resource assertions to prove consumer agreement, and a
pinned rasterizer for reproducible pixel assertions. Record the chosen tolerance
before accepting results; do not explain away layout errors as antialiasing.
Native screenshot checks remain useful but do not substitute for source geometry.

Reference-TeX comparison, where authorized as a development oracle, is a separate
compiler fidelity gate with exact engine/font/package/version fixtures. Passing
preview-to-PDF parity alone is not proof of full LaTeX compatibility.
