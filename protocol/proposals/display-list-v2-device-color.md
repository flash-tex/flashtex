# Proposal: `display-list-v2-device-color`

Status: proposal (kabir-claude, xcolor support). Producer: `crates/render-pipeline`.
Consumers: `crates/pdf` exact route (implemented), Mac preview painter (requested).

## Problem

`rendering-v2` paints are straight sRGB `{r, g, b, a}`. pdfTeX does not paint
in sRGB: `pdftex.def` writes the colour tokens xcolor computed in the colour's
own model (`0.05 0.09999 0.15001 0.2 k`, `1 0 0 rg`, `0.8 g`). A float cannot
carry those operands exactly, and a CMYK or gray colour converted to sRGB loses
the model pdfTeX writes, so an exact PDF cannot be produced from the frozen
paint alone.

## Negotiation

A request adds `display-list-v2-device-color` to `layout_capabilities`
together with `display-list-v2`; the producer echoes it when accepted. Without
it nothing changes on the wire (the frozen schema, `additionalProperties:
false`, is respected). `flashtex-render --tex ... --v2 out.json --device-color`
writes the same field for file consumers.

## Wire

Any `paint` object (glyph runs, rules, paths) may carry:

```json
"paint": {"a": 1, "b": 0.8, "device_color": {"space": "cmyk", "values": ["0", "0", "0.2", "0"]}, "g": 1, "r": 1}
```

- `space`: `"gray"` (PDF `g`/`G`), `"rgb"` (`rg`/`RG`) or `"cmyk"` (`k`/`K`).
- `values`: 1, 3 or 4 decimal strings in `[0, 1]`, pdfTeX's operand values
  (shortest decimal form: `0.5`, never `.50`). A consumer that writes PDF sets
  both fill and stroke from them, as `pdftex.def` does.
- `r`, `g`, `b` stay present: the naive preview conversion (gray `g,g,g`;
  CMYK `1 - min(1, c + k)`), identical to `flashtex_vector_graphics::Color::to_rgb`.
- Absent `device_color`: the default colour, as before.
- `required_features` lists `device-color` when negotiated and any paint
  carries one.

Page background (`\pagecolor`) needs no new item: it is the page's first
`rule` covering the whole page, as pdfTeX's `q 0 0 W H re f Q` is.

## Consumers

- `crates/pdf` (`v2.rs`): writes `device_color` as `q <fill> <stroke> ... Q`
  around runs and rules (`Op::FillCmyk`/`StrokeCmyk` added); sRGB paints keep
  the exact `rg`. Measured against pdfLaTeX on the xcolor oracle fixtures:
  every glyph and rule colour operator equal.
- Mac preview (`apps/mac`, not in this lane): may keep painting `r, g, b`; a
  colour-managed painter can use `device_color` (CMYK) instead. No change is
  required for correctness of the preview.
