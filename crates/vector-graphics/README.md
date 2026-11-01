# flashtex-vector-graphics

Original, dependency-free Rust model of explicit display primitives for
FlashTeX: rules, filled and stroked paths, placed images, and groups with
transforms, clips, and opacity; a display list that flattens to device space
with bounds and hit-testing; and two serializers (a PDF content-stream
fragment builder and a hand-written JSON format).

Status: **isolated proposal, not wired into any consumer.** Runtime-v1
(`docs/contracts/runtime-v1.md`) remains text-only; this crate adds no item
kinds to v1. Integration is deferred to rendering-v2 after the ABI is agreed
(see "Integration note" below).

Edition 2024, no external crates. Build and test with

```sh
cd crates/vector-graphics
cargo test
cargo clippy --all-targets
```

## Coordinate conventions

- Units are PostScript points.
- The origin is the **top-left** of the page and `y` grows **downwards**,
  matching runtime-v1 text items (`x_pt`, `baseline_y_pt`) and the Mac
  preview's drawing space.
- `Transform` is the PDF affine `[a b c d e f]` with
  `x' = a·x + c·y + e`, `y' = b·x + d·y + f`. `s.then(&t)` applies `s` first.
  `Transform::rotate` with a positive angle turns clockwise on screen because
  of the y-down space.
- A `Group`'s `transform` maps its children's space into the parent's; its
  `clip` is expressed in the children's space.
- An `Image` occupies `[0, width_pt] × [0, height_pt]` in its own space with
  the first pixel row at `y = 0`; `transform` places that box.
- **The PDF y-flip happens only in the PDF serializer.** It applies
  `F = [1 0 0 -1 0 H]` to every leaf coordinate and emits each group's
  transform as `F·T·F⁻¹`, so a top-level rule comes out exactly as
  `crates/pdf` writes a fraction bar: `x (H - y - h) w h re f`. The JSON
  formats and the device list keep the top-left convention.

## What is here

| Module | Contents |
| --- | --- |
| `geom` | `Point`, `Size`, `Rect` (intersect/union/expand/transformed bounds), `Transform` (translate/scale/rotate/skew, compose, invert). |
| `path` | `Path` (`move_to`/`line_to`/`quad_to`/`cubic_to`/`close`), exact curve bounds, flattening with an analytic tolerance bound, nonzero/even-odd containment, distance to outline, `StrokeStyle` (width, cap, join, miter limit, dash). |
| `color` | `Color` (`Gray`, `Rgb`, `Cmyk`) and `Paint` (colour + straight alpha). |
| `clip` | `Clip::Rect` / `Clip::Path`, `ClipStack` (conjunction), bounds intersection, transformation. |
| `item` | `Rule`, `PathFill`, `PathStroke`, `Image`, `Group`, each with a producer-assigned `ItemId` and optional `SourceRange { path, start_byte, end_byte }` in the runtime-v1 byte-offset convention. |
| `display_list` | `DisplayList { page_size, items }`, `validate()`, `flatten()` to a `DeviceList` of device-space items (transforms and clips applied, groups resolved, opacity multiplied in, ancestors and inherited source recorded), `bounds()`, `hit(point)` and `hit_with_tolerance`, `source_at`. |
| `pdf` | `content_stream(&DisplayList) -> PdfFragment { content, ext_g_states, images }`. Emits `q`/`Q`, `cm`, `re f`, `m l c h f f* S`, `re W n` / `W n` / `W* n`, `gs` via named ExtGStates (alpha), `Do` via named XObjects. A string builder only; page and resource objects are the writer's job. |
| `json` | `write_display_list` / `read_display_list` (exact round trip) and `write_device_list` for a preview consumer; a small strict JSON parser. |
| `diagram` | `arrow`, `polyline`, `circle`, `ellipse` (4 cubics, `KAPPA`), `text_anchor_box` (position/size only, no glyphs), `grid`. |
| `tikz` | A minimal-but-real TikZ subset (FT-062): `find_pictures`, `Tikz::read_preamble`, `Tikz::render` turn a `tikzpicture` into items in picture space plus node-text placements for a caller `TextMeasurer`, with PGF's geometry and bounding-box rules and a diagnostic for everything unsupported. See `docs/tikz.md`; compared against pdflatex in `crates/render-pipeline/docs/evidence/tikz/`. |

### Colour and alpha semantics

Components are `0.0..=1.0` and are clamped by the serializers. Alpha is
straight (non-premultiplied) and lives on `Paint`/`Image::alpha`/`Group::opacity`.
RGB is assumed sRGB, gray is sRGB gray, CMYK is device CMYK. **No colour
management is implemented**; `Color::to_rgb` is a naive preview conversion.

### Hit-testing semantics

`hit` returns leaf ids topmost (last painted) first. A point must satisfy every
clip on the item's clip stack. Fills use exact containment under the item's
fill rule on the outline flattened at 0.05 pt; strokes hit within
`width / 2 + tolerance` of the centre line (dashes are ignored, the whole path
is hittable); rules and images use closed containment (images through the
inverse of their resolved transform). Stroke widths in the device list are
scaled by the geometric mean scale of the resolved transform, which is exact
for uniform scale and rotation and approximate for anisotropic scale or skew.
The PDF serializer is unaffected because it keeps strokes in local space via
`cm`.

### Group opacity

Flattening multiplies a group's opacity into each descendant's alpha, and the
PDF serializer does the same via per-leaf `gs`. This is *not* a PDF
transparency group: overlapping children of a translucent group show through
each other. Real knockout/isolated groups are out of scope.

### Nesting limits

Groups may nest at most `display_list::MAX_GROUP_DEPTH` (64) deep and the JSON
reader accepts at most `json::MAX_JSON_DEPTH` (256) nested arrays/objects.
Deeper input returns `json::JsonError::NestingTooDeep { depth, limit }` and a
deeper in-memory tree is reported by `validate()` as
`ValidationError::NestingTooDeep`; neither recurses past its limit, so
adversarial documents produce an error instead of a stack-overflow abort
(GH46). The display-list contract states no bound of its own; the rationale
for these values is on the constants.

## Unsupported

- Gradients and shading, tiling patterns, blend modes, soft masks.
- Text as paths (glyph outlines); text remains runtime-v1 text items and the
  anchor-box helper only positions a box.
- Stroke outlining (converting a stroke to a fill), arc segments, and
  transparency groups.
- Image decoding or pixel data: `Image` carries only a content hash and
  placement.
- Colour management and ICC profiles.
- Non-affine transforms.

## Proposed formats (not v1)

The PDF fragment contract and the two JSON documents are documented in the
`pdf` and `json` module docs. Both are **proposals** for later consumer
integration. The JSON format is versioned (`"version": 0`) and readers reject
unknown formats, versions, item kinds, and path operators.

## Integration note (rendering-v2)

Rendering-v2 is under Commander review. A field-level mapping proposal
against the draft schema lives in `docs/rendering-v2-integration.md`.
Until an ABI is agreed:

- `crates/pdf` is not changed. When adopted, its page writer would append
  `PdfFragment::content` to the page stream (wrapped in `q … Q`), register
  each `ExtGState::dictionary()` under its name in `/Resources /ExtGState`,
  and create one image XObject per `ImageResource` keyed by content hash.
  Number formatting (`pdf::num`) already matches the writer's `num()`.
- The Mac preview would consume `write_device_list` output (top-left points,
  transforms already resolved) and use `hit`/`source_at` for click-to-source
  with `SourceRange` converted from UTF-8 byte offsets exactly as runtime-v1
  text sources are today.
- The compiler would populate a `DisplayList` per page alongside the existing
  text items; no runtime-v1 message changes are proposed here.
