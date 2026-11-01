# Proposal `path-v0`: vector path items in the display list v2

Status: **proposal** (FT-062, agent `kabir-claude`). Implemented by the render
pipeline on branch `agent/kabir-claude/tikz-min`; **not** part of the frozen
`protocol/rendering-v2.schema.json` and not validated by
`crates/rendering-core`. The schema/validator change is the Commander's
decision; the Mac painter is `mac-claude-a`'s (`apps/`). This document is
the contract both sides can implement against.

It follows `crates/vector-graphics/docs/rendering-v2-integration.md` §2
(`path_fill` / `path_stroke` items, page-space device list, per-item clips)
and makes it concrete for TikZ pictures.

## 1. Where the items come from

A `tikzpicture` in a document is compiled by `flashtex_vector_graphics::tikz`
(PGF geometry, see `crates/vector-graphics/docs/tikz.md`) and placed by the
pipeline as one box on its own line (bottom edge on the baseline, flush left,
centred inside `center`). It produces, in paint order:

- `path_fill` and `path_stroke` items for every drawn/filled path, arrow tip
  and node shape;
- ordinary `glyph_run` items for node text (Latin Modern, TFM metrics, real
  glyph ids) — already painted by the existing glyph-run code.

The items appear in the page's `items` array in the order they must be
painted. A node's text follows its own border.

## 2. Feature negotiation

`payload.required_features` gains, only when such items are present:

| feature | meaning |
| --- | --- |
| `path_fill` | the page contains `path_fill` items |
| `path_stroke` | the page contains `path_stroke` items |
| `clip` | at least one path item carries a non-empty `clips` array |

A consumer that does not implement a listed feature must treat the display
list as unsupported (report it; do not silently skip items), exactly as
rendering-v2 already requires for unknown features. No runtime-v1 message
changes: runtime-v1 items carry no paths (the worker emits the warning
`tikz_display_list_only` once per document with a picture), and the
`--pdf` shim route omits them for now.

## 3. Items

Coordinates are integer ticks (`bp_2pow20`: 2^20 per PDF point), page
space, origin at the page's top-left, y down — identical to `rule` and
`glyph_run`. The page transform is already applied; there are no groups or
transforms on the wire.

### Path data

`path` is an array of commands, each an array whose first element is the
operator:

| command | meaning |
| --- | --- |
| `["m", x, y]` | move to |
| `["l", x, y]` | line to |
| `["c", x1, y1, x2, y2, x, y]` | cubic Bézier to `(x, y)` with controls `(x1, y1)`, `(x2, y2)` |
| `["z"]` | close the current subpath |

A path may contain several subpaths (a grid is one path of many `m`/`l`
pairs). Quadratic segments are never emitted.

### `path_fill`

```json
{"kind":"path_fill",
 "path":[["m",75484004,97213399],["l",134927589,97213399],["l",134927589,67491492],["z"]],
 "fill_rule":"nonzero",
 "paint":{"r":1,"g":0,"b":0,"a":1},
 "clips":[{"kind":"path","path":[...],"fill_rule":"nonzero"}],
 "sources":[{"path":"main.tex","start_byte":14,"end_byte":233}]}
```

- `fill_rule`: `nonzero` | `evenodd`.
- `clips` is optional (absent = unclipped).

### `path_stroke`

```json
{"kind":"path_stroke",
 "path":[["m",75693719,97003684],["l",134398...,67701207]],
 "stroke":{"width":417871,"cap":"butt","join":"miter","miter_limit":10,
           "dash":{"array":[3134035,3134035],"phase":0}},
 "paint":{"r":0,"g":0,"b":0,"a":1},
 "sources":[{"path":"main.tex","start_byte":14,"end_byte":233}]}
```

- `stroke.width`: ticks, ≥ 1, the full line width (centred on the path).
- `cap`: `butt` | `round` | `square`; `join`: `miter` | `round` | `bevel`;
  `miter_limit`: PDF/CoreGraphics miter limit (number).
- `dash`: optional; `array` of on/off lengths in ticks, `phase` in ticks.
  Absent = solid.

### Common fields

- `paint`: unpremultiplied sRGB `{r,g,b,a}` in `0..1`, as for glyph runs and
  rules. xcolor CMYK colours (`cyan`, `magenta`, `yellow`, `olive`) are
  converted naively (`1 - min(1, c + k)`) — the PDF route keeps CMYK, the
  preview may differ slightly in hue for those four.
- Provenance: `sources` (1..128 ranges) or `synthetic_reason`, as for
  `rule`. Paths carry the byte range of the whole `tikzpicture`
  environment; node glyph runs carry the range of the statement that made
  the node (click-to-source lands on the `\node` or `\draw`).
- `clips`: every clip applies (intersection). Each clip is `{"kind":"path",
  "path":[...], "fill_rule": "nonzero"|"evenodd"}` in page space.

## 4. Painting (CoreGraphics reference for the Mac painter)

The display list is y-down top-left like the preview's flipped view, so no
extra flip is needed beyond what glyph runs already use.

```swift
func point(_ x: Int64, _ y: Int64) -> CGPoint { CGPoint(x: Double(x) / 1048576, y: Double(y) / 1048576) }

func cgPath(_ cmds: [[Any]]) -> CGPath {
    let p = CGMutablePath()
    for c in cmds {
        switch c[0] as! String {
        case "m": p.move(to: point(c[1], c[2]))
        case "l": p.addLine(to: point(c[1], c[2]))
        case "c": p.addCurve(to: point(c[5], c[6]), control1: point(c[1], c[2]), control2: point(c[3], c[4]))
        case "z": p.closeSubpath()
        default: fatalError("unsupported path command")   // reject, never skip
        }
    }
    return p
}

ctx.saveGState()
for clip in item.clips { ctx.addPath(cgPath(clip.path)); ctx.clip(using: clip.fillRule == "evenodd" ? .evenOdd : .winding) }
ctx.setFillColor / setStrokeColor(sRGB r,g,b,a)
switch item.kind {
case "path_fill":   ctx.addPath(path); ctx.fillPath(using: fillRule == "evenodd" ? .evenOdd : .winding)
case "path_stroke": ctx.setLineWidth(width / 2^20); ctx.setLineCap(.butt/.round/.square)
                    ctx.setLineJoin(.miter/.round/.bevel); ctx.setMiterLimit(miter_limit)
                    ctx.setLineDash(phase: phase / 2^20, lengths: array.map { $0 / 2^20 })  // or [] when absent
                    ctx.addPath(path); ctx.strokePath()
}
ctx.restoreGState()
```

Hit-testing (optional): fills by containment under the fill rule, strokes
within `width/2 + tolerance` of the centre line, clips honoured, topmost
(last painted) first — the reference semantics are
`flashtex_vector_graphics::DeviceList::hit`.

## 5. Proposed schema additions (for Commander review)

```json
"path_command": {"type":"array","minItems":1,"maxItems":7,
  "prefixItems":[{"enum":["m","l","c","z"]}],"items":{"$ref":"#/$defs/tick"}},
"path": {"type":"array","items":{"$ref":"#/$defs/path_command"},"minItems":1,"maxItems":1048576},
"clip": {"type":"object","properties":{"kind":{"const":"path"},"path":{"$ref":"#/$defs/path"},
  "fill_rule":{"enum":["nonzero","evenodd"]}},"required":["kind","path","fill_rule"],"additionalProperties":false},
"path_fill": {"type":"object","properties":{"kind":{"const":"path_fill"},"path":{"$ref":"#/$defs/path"},
  "fill_rule":{"enum":["nonzero","evenodd"]},"paint":{"$ref":"#/$defs/paint"},
  "clips":{"type":"array","items":{"$ref":"#/$defs/clip"},"maxItems":64},
  "sources":{"type":"array","items":{"$ref":"#/$defs/source"},"minItems":1,"maxItems":128},
  "synthetic_reason":{"type":"string","minLength":1,"maxLength":1024}},
  "required":["kind","path","fill_rule","paint"],"additionalProperties":false},
"path_stroke": {"type":"object","properties":{"kind":{"const":"path_stroke"},"path":{"$ref":"#/$defs/path"},
  "stroke":{"type":"object","properties":{"width":{"$ref":"#/$defs/positive_tick"},
    "cap":{"enum":["butt","round","square"]},"join":{"enum":["miter","round","bevel"]},
    "miter_limit":{"type":"number","minimum":1},
    "dash":{"type":"object","properties":{"array":{"type":"array","items":{"$ref":"#/$defs/tick"},"minItems":1,"maxItems":64},
      "phase":{"$ref":"#/$defs/tick"}},"required":["array","phase"],"additionalProperties":false}},
    "required":["width","cap","join","miter_limit"],"additionalProperties":false},
  "paint":{"$ref":"#/$defs/paint"},
  "clips":{"type":"array","items":{"$ref":"#/$defs/clip"},"maxItems":64},
  "sources":{"type":"array","items":{"$ref":"#/$defs/source"},"minItems":1,"maxItems":128},
  "synthetic_reason":{"type":"string","minLength":1,"maxLength":1024}},
  "required":["kind","path","stroke","paint"],"additionalProperties":false}
```

plus `path_fill`/`path_stroke` in the page item `oneOf` and the three
feature names in the `required_features` enum. The validator should check
that `c` has 6 coordinates, `m`/`l` 2 and `z` none, that the first command
is `m`, and that every coordinate fits the tick bound.

## 6. Known limits of the producer (stated, not hidden)

- Rotated or scaled node text (`sloped`, node `rotate`, `transform shape`)
  is emitted upright at its origin (glyph runs carry no transform) with the
  warning `tikz_text_transform`; the standalone PDF route draws it rotated.
- Text inside a `\clip` scope is not clipped.
- A picture always occupies its own line; text before and after it in the
  same source paragraph is set as separate paragraphs.
- A picture whose body produces no compiler inline at all is not found by
  the adapter (every fixture and ordinary picture does produce one).

## 7. Evidence

- `crates/render-pipeline/tests/tikz_pipeline.rs`: a document with a picture
  yields strokes, a clipped fill, node glyph runs between the surrounding
  paragraphs, no compiler diagnostics for the body, the three features and
  JSON `path_stroke`/`clips`.
- Geometry against pdflatex: `crates/render-pipeline/docs/evidence/tikz/`
  (30 fixtures, 150 dpi, tolerance documented there). The same items the
  PDF route paints are the ones converted to ticks here.
