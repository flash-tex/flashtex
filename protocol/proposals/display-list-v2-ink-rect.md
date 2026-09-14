# Proposal: `display-list-v2-ink-rect`

Status: proposal. Producer: `crates/render-pipeline` (implemented, off unless
negotiated). Consumers: Mac preview `V2Geometry.formulaBox` / `MathHoverPreview`
(requested, `apps/mac`, not this lane), `crates/rendering-core` (must learn the
field before the capability can be accepted).

## Problem

A cluster ships exactly one rectangle, `hit_rects`, and the repository never
wrote down what it measures. The producer answered differently on different
paths: body text reported the **TeX box** it was shaped into, a stacked
extensible delimiter reported the **cmex boxes TeX stacked**, and a single math
glyph reported the **ink of the OpenType outline painted inside its box**.

Those two measurements are not close, and they disagree in both directions:

| formula | TeX box (pdfTeX `\showbox`) | painted ink | reported before |
| --- | --- | --- | --- |
| `\Bigg(` at 10 pt | 30.00029 pt | 29.90 pt | 29.90 |
| `\Bigg[` at 10 pt | 30.00029 pt | 30.00 pt | 30.00 |
| one cmex extension piece | 6.00006 pt | 12.02 pt | 12.02 |
| `\Bigg|` (three stacked pieces) | 30.00031 pt | 30.00031 pt | 30.00031 |

Three lanes measured a delimiter's height from `hit_rects`, got the ink, and
opened engine defects for boxes that were already exact (PRs #265 and #266 and
their #2 comment). `[` and `\{` looked correct in the same runs only because
their flat outlines happen to fill their boxes; `(`'s round ends do not, and a
cmex bar is deliberately drawn past its box so stacked copies overlap.

`hit_rects` is now the TeX box on every path. That is the right answer for its
own consumers — a pointer target and a caret highlight want the box, so a space
stays clickable and a selection band has line height — and it is the answer a
lane measuring against `\showbox` needs. But it removes the only ink the Mac
preview had: `V2Geometry.formulaBox` unions cluster rectangles into the outline
drawn at `PreviewV2View.swift:1218` and the crop
`MathHoverPreview` pads by 4 pt, and both of those want the extent of what is
actually painted.

## Negotiation

A request adds `display-list-v2-ink-rect` to `layout_capabilities` together
with `display-list-v2`; the producer echoes it when accepted. Without it nothing
changes on the wire — the cluster object is byte-identical to today's, so the
frozen schema's `additionalProperties: false`, `crates/rendering-core`'s
`deny_unknown_fields` and `scripts/check_rendering_v2.py` all keep passing
unchanged. `display::Wire { ink_rects }` is the producer-side switch.

## Wire

A cluster may carry one additional key:

```json
{
  "text_start_byte": 0,
  "text_end_byte": 3,
  "hit_rects": [{"height": 31459744, "top": 153372672, "width": 8300577, "x": 310382592}],
  "ink_rect": {"height": 31350272, "top": 153427968, "width": 7993856, "x": 310460416},
  "carets": [...],
  "sources": [...]
}
```

- `hit_rects`: unchanged in shape and required. Each rectangle is the cluster's
  **laid-out TeX box** — `width` is the advance TeX positions the next atom
  from (the TFM width, pdfTeX's `/Widths`), `top` is `baseline - box height`
  and `height` is `box height + box depth`.
- `ink_rect`: optional, at most one, the same `$defs/rect` shape. The tight
  bounding box of the outline the producer actually paints for the cluster: the
  union over its glyphs of each outline's bounding box placed at that glyph's
  origin and baseline.
- `ink_rect` is **absent**, not zero-sized, when the producer has no ink to
  report: every text run (nothing measures outlines there) and any cluster
  whose glyphs are all blank.
- `ink_rect` may fall outside its `hit_rects` on any side. That is not an
  error; it is the fact the field exists to carry.
- `required_features` lists `ink-rect` when negotiated and some cluster carries
  one.

## What must not read it

`ink_rect` is not a measurement of the box and must never be substituted for
one. Reporting a delimiter's size, comparing against pdfTeX's `\showbox`,
positioning a following atom, and every hit test and caret rectangle read
`hit_rects`. `ink_rect` exists to frame what is on the page — a formula
outline, a hover crop, a trim.

## Consumers

- `crates/rendering-core` must add `ink_rect: Option<HitRect>` to its `Cluster`
  before this can be accepted end to end: its `#[serde(deny_unknown_fields)]`
  rejects the key today. Validation is `$defs/rect`'s: both dimensions
  non-negative and `x + width` / `top + height` in tick range. Unlike
  `hit_rects`, an empty or zero-area `ink_rect` is meaningless rather than
  merely unselectable, so producers omit the key instead.
- Mac preview (`apps/mac`, not this lane): `V2Geometry.formulaBox` and
  `MathHoverPreview` are the two sites that want it. `V2Geometry.hit`, the
  whole-cluster caret fallback and `CaretFollow`'s scroll target must keep
  reading `hitRects` — ink makes a space unhittable and a zero-height scroll
  target, which `CaretFollowTests` already asserts against.
- `crates/pdf`: no change. It paints from glyph origins, not cluster rectangles.

## Delta digests

`dl2-canon-1` (`docs/proposals/display-list-v2-delta.md`) encodes `hit_rects`
and does not cover `ink_rect`, so a delta could reuse a page whose ink moved
while its boxes did not. A delta that carries the field needs `dl2-canon-2`;
until that exists the producer **refuses `display-list-v2-ink-rect` when
`display-list-v2-delta` is also requested** (`v1::Capabilities::negotiate`) —
the ink capability is simply not echoed, and the reply is a plain delta.
