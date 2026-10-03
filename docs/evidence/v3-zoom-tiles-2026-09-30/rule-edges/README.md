# Rule edges at tile boundaries (review of #1287 @c1b698c3b)

**Blocker.** A filled rule whose edge lies within about 1 px of a 512 px tile
boundary differed by one coverage level between a translated tile and the
whole page. Rounding the edges cannot fix it. Clipping near the rule's edge
changes Core Graphics' coverage of the rule, at its other edge too.

**Finding.** The same happens in the page's own context: a single tile drawn
by `clippedTiles` (one clip rect) differed in the same pixels (142 px at
6 px/pt, 278 px at 12 px/pt, max 1 level). It only looked exact before
because the sweeps request whole rows, so adjacent tiles' clips merge and no
clip edge falls inside the row. The cause is the edge of any clipped area (a
bitmap's bounds or a clip rect) lying near a rule's edge.

**Fix (`DL3Tiles.swift`).**

- `tilesNeedingPageContext`: a tile of a translatable page with a rule's edge
  within `ruleEdgeMargin` (2 px) of any of its four edges is drawn with the
  page's own context, one tile per clip, in the same worker pool. Every other
  tile is still drawn by translation.
- `clearClip`: every page-context clip, for these tiles and for clip-exact
  pages alike, is the tile grown a pixel at a time until each side is 2 px
  clear of every rule's edge (filled, or a stroked rule's outline) or is the
  page's edge. The tile is then copied from inside the clip.
- `rasterizeTile(layout: .rgba)` draws such a tile in the requested layout
  (`clippedTileImage`).
- Glyphs whose ink misses the clip are skipped in the page-context draw
  (`draw(cull:)`, exact by construction).

**Tests (`TileParityRuleEdgeTests`).** Zero tolerance; light and dark;
IOSurface tiles (all at once, and each tile on its own) and RGBA tiles; at
3.25, 4, 6, 8, 12 and 16 px/pt.

- **Rule edges.** 3,478 filled rules, from sp and from RULE_GEOMETRY. Their
  edges sit 0–1.2 px either side of the left, right, top and bottom tile
  boundaries and the corners, plus page-edge bars and random rules. Before
  the fix, 66 of 1,284 tiles differed (max 1 level). After: 0 of 1,284.
- **Exact positions.** `beamer-overlays` pages 1–3 with exact glyph ORIGINS
  (every third x and every fifth y on a subpixel phase boundary) and
  RULE_GEOMETRY under a CTM translation: 0 of 3,540 tiles differ.

**Cost.** `bench-before/`, `bench-after/`: `testTileBenchmark`, release
build, median of 9, 25 viewport tiles of a 1100×900 pt pane at 2×. The
`viewport_tiles_page_context` field in `bench-before/` is a −1 placeholder.

| Page | Tiles from the page's context | Viewport, before → after | Scroll row | One tile |
|---|---|---|---|---|
| beamer-overlays p1–3 at 8 px/pt | 13 of 25 | 2.3 → 4.7–4.9 ms | 0.3 → 0.7 ms | 0.2 ms (same) |
| beamer-overlays p1–3 at 16 px/pt | 9 of 25 | 2.4–2.6 → 6.3–6.6 ms | 0.6 → 1.3–1.4 ms | 0.2 ms (same) |
| tile-text (clip-exact) at 8 / 16 px/pt | — | 8.7 / 10.2 → 9.2 / 9.8 ms | unchanged | unchanged |

A single page-context draw clipped to all the near tiles at once took
7.2–11.1 ms; one tile per clip, in parallel, takes the numbers above.

**Memory.** Tile bytes are unchanged. The page-context rasters are the same
anonymous mappings as before, backed only where written. A clip grows only
where a rule's edge lies near a tile's side. The residency checks
(`testCutRasterMemoryIsBoundedByTheTiles`) are byte-identical to the
previous round: one tile 9.37 / 9.31 / 9.40 MB and the 5×4 block
40.1 / 50.3 / 54.0 MB at 8 / 16 / 20 px/pt.
