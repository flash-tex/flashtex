import CoreGraphics
import CoreImage
import Foundation
import IOSurface
import FlashTeXDisplayListV3

// High-zoom tiles (DESIGN.md §6.2; lane P3-V3-ZOOM-TILES, from #1228's
// preview-v2 tiles). Above about 3 px/pt a whole-page bitmap is large
// (8 px/pt: 4896×6336 px, 124 MB for US letter) and slow, while the pane
// shows a small part of it. The pane then holds 512 px tiles of the visible
// area instead, each drawn here, off the main thread.
//
// Every tile is a pixel-exact window of the whole-page raster (`rasterize`),
// which is what the zero-tolerance parity gate compares with Core Graphics'
// rendering of the engine's PDF, so the gate covers tiled pages too
// (`TileParityTests` sweeps 3.25–8 px/pt with 0 differing tiles).
//
// A tile context is the page context translated by whole pixels. That alone
// is not exact: Core Graphics rounds some device coordinates at the
// magnitude they have, and a tile's coordinates are smaller than the page's
// (measured in #1228, docs/evidence/preview-smoothing-2026-09-29/README.md §2):
//
// - A glyph's pixel and subpixel phase are `floor((d + 0.001)·N)` per axis
//   (device coordinate d, N = 1, 2 or 4 phases by glyph size). When
//   `d + 0.001` lies within an ulp of a phase boundary the sum rounds one way
//   on the page and the other in the tile. `tileCoordinate` moves such an
//   origin 1e-7 px onto the page's side; every other origin is unchanged.
// - Rule edges are single-precision device coordinates: `tileEdge` hands
//   over the page's single-precision value, which the translation keeps exact.
// - Clipping is not exact near a rule's edge (review of #1287 @c1b698c3b,
//   TileParityRuleEdgeTests): where a filled rule's edge lies within about
//   1 px of the edge of the area Core Graphics clips to — a tile's bitmap,
//   or a clip rect in the page's own context alike — the rule's coverage
//   changes by one level, at its other edge too, so no rounding of the edges
//   undoes it. A tile with a rule's edge within `ruleEdgeMargin` of any of
//   its four edges (`tilesNeedingPageContext`) is therefore drawn with the
//   page's own context (`clippedTiles`), whose clip is the tile grown until
//   every side is clear of rule edges or is the page's edge (`clearClip`);
//   every other tile of the page is still drawn by translation. A clip may
//   grow at most `clipGrowthMax` px a side (inside a dense cluster of rules
//   it would otherwise grow to the whole page, review of #1287): past that,
//   on a page with a rule that is not finite, and where the clipped raster
//   could not be mapped, the tile is cut from the page's whole raster
//   (`DL3PageRaster`, the pane's kept one; `tileRoutes`).
// - Everything else is not tiled by translation. Pages of glyphs and rules
//   (stroked rules included, `clipExact`) are drawn with the page's own
//   context, clipped to the requested tiles (grown clear of rule edges, as
//   above), over memory that is backed only where it is written
//   (`clippedTiles`). Pages with paths, clips, images, forms or stroked
//   text, and pages drawn from the PDF, draw one full-scale page raster per
//   source (`DL3PageRaster`, purgeable anonymous memory, never written to
//   disk), which every tile job of that source cuts from. Both are exact:
//   the page's device coordinates, nothing translated.

/// A rectangle of a page raster in pixels, top-left origin (image rows).
public struct DL3PixelRect: Hashable, Sendable {
    public var x: Int, y: Int, width: Int, height: Int
    public init(x: Int, y: Int, width: Int, height: Int) { self.x = x; self.y = y; self.width = width; self.height = height }
}

extension DL3Renderer {
    /// The context is a tile of the page raster at `scale` px/pt; `visible`
    /// is the tile in the page's user space (bp, y up).
    struct Tile {
        let scale: Double
        let visible: CGRect
        /// Beyond an item's geometric box, what antialiasing can still ink
        /// (bp): at the tiled scales (> 3 px/pt) 2 bp is at least 6 px.
        static let margin = 2.0
    }

    /// Glyph ink boxes for culling, recomputed when the font or glyph matrix changes.
    struct TileInk {
        private var font: DL3RenderFont?
        private var a = 0.0, b = 0.0, c = 0.0, d = 0.0
        private var box: CGRect?

        /// Whether a glyph of `font` drawn with text matrix `tm` may ink `visible`.
        mutating func meets(_ visible: CGRect, font f: DL3RenderFont, matrix tm: CGAffineTransform) -> Bool {
            if font !== f || a != tm.a || b != tm.b || c != tm.c || d != tm.d {
                font = f; a = tm.a; b = tm.b; c = tm.c; d = tm.d
                box = f.inkBox.map {
                    $0.applying(CGAffineTransform(a: tm.a, b: tm.b, c: tm.c, d: tm.d, tx: 0, ty: 0))
                        .insetBy(dx: -Tile.margin, dy: -Tile.margin)
                }
            }
            guard let box, box.width.isFinite, box.height.isFinite else { return true }
            return box.offsetBy(dx: tm.tx, dy: tm.ty).intersects(visible)
        }
    }

    /// Margin around a phase boundary (device px) inside which an origin is
    /// moved to the page's side; the move is 1e-7 px.
    static let tilePhaseMargin = 1e-6

    /// A glyph origin coordinate (user space, bp) for a tile of the page
    /// raster at `scale` (a page whose box starts at the origin).
    static func tileCoordinate(_ v: Double, scale s: Double) -> Double {
        // The page's device coordinate and phase sum as Core Graphics forms
        // them; ×4 is exact and covers the boundaries of N = 1, 2 and 4.
        let d = s * v
        let q = (d + 0.001) * 4
        let k = q.rounded()
        guard abs(q - k) < tilePhaseMargin else { return v }
        return (k / 4 - 0.001 + (q >= k ? 1e-7 : -1e-7)) / s
    }

    /// A rule edge (user space, bp) for a tile at `scale`: the page's
    /// single-precision device coordinate, back in user space.
    static func tileEdge(_ v: Double, scale s: Double) -> Double { Double(Float(s * v)) / s }

    /// A filled rule in a tile, with the whole page's rounding: its edges
    /// are the page's single-precision device coordinates (`tileEdge`).
    /// (Stroked rules never reach a tile drawn by translation: see
    /// `tilesByTranslation`.)
    static func tileRule(left: Double, right: Double, top: Double, bottom: Double, tile: Tile, in ctx: CGContext) {
        guard CGRect(x: left, y: bottom, width: right - left, height: top - bottom).standardized
            .insetBy(dx: -Tile.margin, dy: -Tile.margin).intersects(tile.visible) else { return }
        let s = tile.scale
        let x0 = tileEdge(left, scale: s), x1 = tileEdge(right, scale: s), y0 = tileEdge(bottom, scale: s), y1 = tileEdge(top, scale: s)
        ctx.beginPath()
        ctx.addRect(CGRect(x: x0, y: y0, width: x1 - x0, height: y1 - y0))
        ctx.fillPath()
    }

    /// How far (device px) a clip's edge, or a tile's, must stay from a
    /// rule's edge. Measured (TileParityRuleEdgeTests, rules 0–1.2 px either
    /// side of the boundaries at 3.25–16 px/pt; review of #1287 @c1b698c3b):
    /// where a filled rule's edge lies within about 1 px of the edge of the
    /// area Core Graphics clips to — a tile's bitmap, or a clip rect in the
    /// page's own context alike — the rule's coverage changes by one level
    /// (at its other edge too, so no rounding of the edges undoes it); away
    /// from it, never. 2 px leaves a margin.
    static let ruleEdgeMargin = 2.0

    /// `prepared`'s rules at `scale` in page pixels (top-left origin, as
    /// `DL3PixelRect`), as `drawStream` places them: a filled rule's
    /// rectangle, a stroked rule's outline. A rule that is not finite is
    /// `.infinite` (then no clip edge is clear of it).
    static func ruleRects(_ prepared: DL3PreparedPage, scale s: Double) -> [CGRect] {
        let page = prepared.page, H = page.box[3], geometry = page.ruleGeometry
        let rows = Double(pixelSize(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: s).height)
        var out: [CGRect] = [], ri = 0
        for item in page.items {
            guard case .rule(let kind, let x, let y, let w, let h) = item else { continue }
            defer { ri += 1 }
            var l, r, b, t: Double
            if ri < geometry.count {
                let g = geometry[ri]
                if kind == .fill {
                    l = g[0] + g[2]; r = l + g[4]; b = g[1] + g[3]; t = b + g[5]
                } else { // `m l S` with butt caps: the line's length, its width across
                    l = g[0] + min(g[2], g[4]); r = g[0] + max(g[2], g[4]); b = g[1] + min(g[3], g[5]); t = g[1] + max(g[3], g[5])
                    if g[3] == g[5] { b -= g[6] / 2; t += g[6] / 2 }
                    if g[2] == g[4] { l -= g[6] / 2; r += g[6] / 2 }
                    if g[3] != g[5] && g[2] != g[4] { l -= g[6] / 2; r += g[6] / 2; b -= g[6] / 2; t += g[6] / 2 }
                }
            } else { // sp: a stroked rule's outline is its box too
                l = Double(x) / K; r = (Double(x) + Double(w)) / K; t = H - Double(y) / K; b = H - (Double(y) + Double(h)) / K
            }
            guard [s * l, s * r, s * b, s * t].allSatisfy(\.isFinite) else { out.append(.infinite); continue }
            let q = CGRect(x: s * l, y: s * b, width: s * (r - l), height: s * (t - b)).standardized
            out.append(CGRect(x: q.minX, y: rows - q.maxY, width: q.width, height: q.height))
        }
        return out
    }

    /// Whether the line `x = v` (`vertical`) or `y = v` between `lo` and `hi`
    /// along it (page pixels, top-left origin) is `ruleEdgeMargin` clear of
    /// the parallel edges of every rule that reaches it.
    static func clear(_ rules: [CGRect], vertical: Bool, at v: Int, from lo: Int, to hi: Int) -> Bool {
        let m = ruleEdgeMargin, v = Double(v), lo = Double(lo) - m, hi = Double(hi) + m
        for q in rules {
            if q.isInfinite { return false }
            if vertical {
                guard q.maxY > lo, q.minY < hi, abs(q.minX - v) < m || abs(q.maxX - v) < m else { continue }
            } else {
                guard q.maxX > lo, q.minX < hi, abs(q.minY - v) < m || abs(q.maxY - v) < m else { continue }
            }
            return false
        }
        return true
    }

    /// For each of `rects`: whether a rule's edge lies within
    /// `ruleEdgeMargin` of one of its four edges (the page's edges included),
    /// so that a translated tile, clipped there by its bitmap, would differ:
    /// such a tile comes from the page's own context (`clippedTiles`).
    static func tilesNeedingPageContext(_ prepared: DL3PreparedPage, scale: Double, rects: [DL3PixelRect]) -> [Bool] {
        let rules = ruleRects(prepared, scale: scale)
        guard !rules.isEmpty else { return rects.map { _ in false } }
        return rects.map { r in
            !(clear(rules, vertical: true, at: r.x, from: r.y, to: r.y + r.height)
                && clear(rules, vertical: true, at: r.x + r.width, from: r.y, to: r.y + r.height)
                && clear(rules, vertical: false, at: r.y, from: r.x, to: r.x + r.width)
                && clear(rules, vertical: false, at: r.y + r.height, from: r.x, to: r.x + r.width))
        }
    }

    /// How far (px) `clearClip` may move each side of a tile's clip. Inside
    /// a cluster of rules whose edges are less than 2 × `ruleEdgeMargin`
    /// apart the clip would grow to the whole page: about 500 MB of scratch
    /// per tile job at 16 px/pt, with up to `tileWorkers` jobs at once
    /// (review of #1287). Capped, a 512 px tile's clip is at most 768 px a
    /// side (2.25× its area) and its clipped raster backs at most
    /// `clipResidentBound` bytes; a tile whose clip would grow further is
    /// cut from the page's raster instead (`tileRoutes`).
    public static let clipGrowthMax = 128

    /// The clip for tile `r` in the page's own context (`clippedTiles`): `r`
    /// grown side by side, a pixel at a time, until each side is clear of
    /// every rule's edge or is the page's edge (the whole page is clipped
    /// there too). Usually `r` itself; the tile is copied from inside it.
    /// Nil when a side would move more than `limit` px, or when a rule is
    /// not finite (no clip edge is clear of it).
    static func clearClip(_ r: DL3PixelRect, rules: [CGRect], width W: Int, height H: Int, limit: Int = clipGrowthMax) -> DL3PixelRect? {
        guard !rules.isEmpty else { return r }
        if rules.contains(where: \.isInfinite) { return nil }
        var x0 = r.x, x1 = r.x + r.width, y0 = r.y, y1 = r.y + r.height
        let limit = min(max(0, limit), max(W, H)) // (no overflow below)
        let lx = r.x - limit, hx = r.x + r.width + limit, ly = r.y - limit, hy = r.y + r.height + limit
        var grown = true
        while grown {
            grown = false
            while x0 > 0, !clear(rules, vertical: true, at: x0, from: y0, to: y1) { guard x0 > lx else { return nil }; x0 -= 1; grown = true }
            while x1 < W, !clear(rules, vertical: true, at: x1, from: y0, to: y1) { guard x1 < hx else { return nil }; x1 += 1; grown = true }
            while y0 > 0, !clear(rules, vertical: false, at: y0, from: x0, to: x1) { guard y0 > ly else { return nil }; y0 -= 1; grown = true }
            while y1 < H, !clear(rules, vertical: false, at: y1, from: x0, to: x1) { guard y1 < hy else { return nil }; y1 += 1; grown = true }
        }
        return DL3PixelRect(x: x0, y: y0, width: x1 - x0, height: y1 - y0)
    }

    /// The most a clipped raster for tile `r` (its clip grown at most
    /// `clipGrowthMax` px a side) backs, in a page `stride` bytes a row:
    /// each row of the clip touches at most its own bytes rounded up to
    /// whole VM pages plus one page, and all rows together at most their
    /// span of the raster plus two pages.
    public static func clipResidentBound(_ r: DL3PixelRect, stride: Int) -> Int {
        let page = Int(getpagesize()), g = clipGrowthMax
        let rows = r.height + 2 * g, row = ((r.width + 2 * g) * 4 + page - 1) / page * page + page
        return min(rows * row, (rows * stride + page - 1) / page * page + 2 * page)
    }

    /// How a tile is drawn (`tileRoutes`).
    public enum TileRoute: Equatable, Sendable {
        /// The page context translated by whole pixels (`drawTile`).
        case translate
        /// The page's own context clipped to this rect (`clippedTiles`).
        case clip(DL3PixelRect)
        /// Cut from the page's whole raster (`DL3PageRaster`).
        case pageRaster
        /// Not a rect of the page: no tile.
        case none
    }

    /// The route of each of `rects` at `scale`. Pages drawn whole
    /// (`tilesByTranslation` and `clipExact` both false), and pages with a
    /// rule that is not finite at `scale`, are cut from the page's raster.
    /// On the others a tile near a rule's edge (every tile of a clip-exact
    /// page) is clipped, its clip grown clear of rule edges (`clearClip`),
    /// or cut from the page's raster when the clip would grow past
    /// `clipGrowthMax`; every other tile is drawn by translation.
    public static func tileRoutes(_ prepared: DL3PreparedPage, scale: Double, rects: [DL3PixelRect]) -> [TileRoute] {
        let translated = tilesByTranslation(prepared)
        guard translated || clipExact(prepared) else { return rects.map { _ in .pageRaster } }
        let (W, H) = pixelSize(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale)
        let rules = ruleRects(prepared, scale: scale)
        if rules.contains(where: \.isInfinite) { return rects.map { _ in .pageRaster } }
        let near = translated ? tilesNeedingPageContext(prepared, scale: scale, rects: rects) : rects.map { _ in true }
        return zip(rects, near).map { r, near in
            guard near else { return .translate }
            guard r.width > 0, r.height > 0, r.x >= 0, r.y >= 0, r.x + r.width <= W, r.y + r.height <= H else { return .none }
            return clearClip(r, rules: rules, width: W, height: H).map { .clip($0) } ?? .pageRaster
        }
    }

    /// Tests: the next `n` clipped rasters fail to map, as when the address
    /// space or memory is not available (`clippedPage`).
    public static func failNextClipMapsForTesting(_ n: Int) { residencyLock.lock(); _failMaps = n; residencyLock.unlock() }
    static func takeFailedMap() -> Bool {
        residencyLock.lock(); defer { residencyLock.unlock() }
        guard _failMaps > 0 else { return false }
        _failMaps -= 1
        return true
    }

    /// Whether a page's tiles may come from a raster clipped to them: glyphs
    /// and rules only (filled or stroked). Measured exact (TileParityTests):
    /// all 83 parity fixtures at 3.25–8 px/pt, 9 of them (stroked-rule and
    /// path pages) at 12, 16 and 20 px/pt, the checked-in `tile-text` up to
    /// 20 px/pt. Pages with paths, clips, images, forms or stroked text are
    /// not: a pixel-aligned clip changed 1–17 px of one tile per page of
    /// `tile-paths` (shadings), with clip margins up to 256 px too; they use
    /// a `DL3PageRaster`.
    ///
    /// A page with a rule whose RULE_GEOMETRY is not finite is neither (nor
    /// `tilesByTranslation`): no clip edge is clear of such a rule, so the
    /// page is drawn whole, cut from one `DL3PageRaster`.
    public static func clipExact(_ prepared: DL3PreparedPage) -> Bool {
        let p = prepared.page
        guard p.box[0] == 0, p.box[1] == 0, !prepared.needsPDFFallback, finiteRules(prepared) else { return false }
        return !p.items.contains { switch $0 { case .path, .clip, .image, .form: true; case .textRender(let m): m == 1 || m == 2; default: false } }
    }

    /// Whether every RULE_GEOMETRY number of the page is finite (rules
    /// placed in sp always are). One that is not makes a rule no clip edge
    /// is clear of (`ruleRects`).
    static func finiteRules(_ prepared: DL3PreparedPage) -> Bool {
        prepared.page.ruleGeometry.allSatisfy { $0.allSatisfy(\.isFinite) }
    }

    /// The last-resort bound on a `DL3PageRaster`: 1 GiB, a letter page at
    /// 23.5 px/pt, above the ~19.3 px/pt the pane reaches on a 14-inch
    /// MacBook Pro (a 4:3 beamer frame reaches it at about 52 px/pt). Only a
    /// page drawn whole in a wider pane than that reaches it. Measured
    /// (testPageRasterCostAtTheLastResortBound): 712 ms to draw, +2 MB of
    /// footprint (the pixels are file-backed); 201 ms at 19.3 px/pt.
    public static let wholeRasterMaxBytes = Int(ProcessInfo.processInfo.environment["FLASHTEX_V3_WHOLE_RASTER_MAX_MB"] ?? "").map { $0 << 20 } ?? 1 << 30

    /// The scale a page's tiles are drawn at when the pane shows it at
    /// `pixelsPerPoint`: that scale, except for a page drawn whole whose
    /// raster would exceed `wholeRasterMaxBytes` (last resort: above it the
    /// tiles are stretched on screen, like the backdrop).
    public static func tileScale(widthPt: Double, heightPt: Double, drawnWhole: Bool, pixelsPerPoint ppp: Double) -> Double {
        guard drawnWhole, widthPt > 0, heightPt > 0 else { return ppp }
        let cap = (Double(wholeRasterMaxBytes) / 4 / (widthPt * heightPt)).squareRoot()
        guard ppp > cap else { return ppp }
        // A multiple of 1/64 px/pt below the cap (a stable key for the tile source).
        return (cap * 64).rounded(.down) / 64
    }

    /// The page raster's size at `scale` (what `rasterize` allocates).
    /// 0×0 for a scale or size that is not finite and positive, or beyond
    /// 1 Mpx a side (never a trap; callers then draw nothing).
    public static func pixelSize(widthPt: Double, heightPt: Double, scale: Double) -> (width: Int, height: Int) {
        let w = (widthPt * scale).rounded(.up), h = (heightPt * scale).rounded(.up)
        guard w.isFinite, h.isFinite, w >= 0, h >= 0, w <= Double(1 << 20), h <= Double(1 << 20) else { return (0, 0) }
        return (Int(w), Int(h))
    }

    /// Whether a translated tile context reproduces `prepared`'s whole-page
    /// raster exactly: only glyphs (filled or invisible), filled rules and
    /// state items, on a page whose box starts at the origin.
    ///
    /// Stroked rules (pdfTeX's thin rules: `\hrule`, table lines) are
    /// excluded after measurement (TileParityTests over the 83 parity
    /// fixtures at 3.25–8 px/pt): stroking the translated line differs from
    /// the page in 1–2,000 px per affected page; filling the stroke's outline
    /// with page-rounded edges matched everywhere except one rule (a 1-level
    /// difference along one pixel row, `proof-practice` at 3.75 px/pt, 16
    /// pages), and no rounding of the centre line and width reproduced Core
    /// Graphics' stroker there. Their pages are cut from one raster: exact.
    public static func tilesByTranslation(_ prepared: DL3PreparedPage) -> Bool {
        let p = prepared.page
        guard p.box[0] == 0, p.box[1] == 0, !prepared.needsPDFFallback, finiteRules(prepared) else { return false }
        // Type 3 glyphs (image masks, merged from main after the tile sweeps)
        // were never measured by translation: their pages are cut from one raster.
        if prepared.fonts.values.contains(where: { $0.type3 != nil }) { return false }
        for it in p.items {
            switch it {
            case .glyph, .save, .restore, .fillColor, .strokeColor, .matrix, .span, .unsupported: continue
            case .rule(let kind, _, _, _, _): if kind != .fill { return false }
            case .textRender(let m): if m == 1 || m == 2 { return false }
            case .path, .clip, .image, .form: return false
            }
        }
        return true
    }

    /// The context configuration of `bitmapContext`/`surface`, for a tile:
    /// the page context translated by whole pixels (`origin`: the tile's
    /// bottom-left corner in page pixels, y up).
    static func configureTile(_ ctx: CGContext, width w: Int, height h: Int, scale: Double, origin: (x: Int, y: Int),
                              background: CGColor = CGColor(gray: 1, alpha: 1), smoothFonts: Bool = false) {
        ctx.setFillColor(background)
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        if origin.x != 0 || origin.y != 0 { ctx.translateBy(x: CGFloat(-origin.x), y: CGFloat(-origin.y)) }
        ctx.scaleBy(x: scale, y: scale)
        ctx.setShouldAntialias(true)
        setFontSmoothing(smoothFonts, in: ctx) // Settings > Smooth fonts in preview (#1304), as the whole page
        ctx.setAllowsFontSubpixelPositioning(true)
        ctx.setShouldSubpixelPositionFonts(true)
    }

    static func tileSpace() -> CGColorSpace { CGColorSpace(name: CGColorSpace.sRGB)! }

    /// Draws tile `rect` of a translatable page into `ctx` (w×h = rect size).
    static func drawTile(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage], scale: Double, rect r: DL3PixelRect, in ctx: CGContext,
                         appearance: DL3Appearance = .light, smoothFonts: Bool = false) {
        let (_, pageHeight) = pixelSize(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale)
        let origin = (x: r.x, y: pageHeight - r.y - r.height)
        configureTile(ctx, width: r.width, height: r.height, scale: scale, origin: origin, background: appearance.background, smoothFonts: smoothFonts)
        let visible = CGRect(x: Double(origin.x) / scale, y: Double(origin.y) / scale, width: Double(r.width) / scale, height: Double(r.height) / scale)
        drawStream(prepared, forms: forms, in: ctx, depth: 0, appearance: appearance, tile: Tile(scale: scale, visible: visible))
    }

    /// One tile as a CGImage in `layout` (tests, evidence). Pages that do
    /// not tile by translation are cut from a whole-page raster.
    public static func rasterizeTile(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double,
                                     rect r: DL3PixelRect, layout: Layout = .rgba, appearance: DL3Appearance = .light,
                                     smoothFonts: Bool = false) -> CGImage? {
        guard tilesByTranslation(prepared) else {
            // What the pane installs (BGRA whatever `layout`: DL3Parity.rgba normalises).
            return rasterizeTiles(prepared, forms: forms, scale: scale, rects: [r], appearance: appearance, smoothFonts: smoothFonts)[0].flatMap { image(of: $0) }
        }
        // The whole page in `layout`, cropped (the RGBA and BGRA rasterisers
        // can differ by a coverage level at a rule's edge).
        func wholePage() -> CGImage? {
            rasterize(prepared, forms: forms, scale: scale, layout: layout, appearance: appearance, smoothFonts: smoothFonts)?
                .cropping(to: CGRect(x: r.x, y: r.y, width: r.width, height: r.height))
        }
        switch tileRoutes(prepared, scale: scale, rects: [r])[0] {
        case .none: return nil
        case .pageRaster: return wholePage()
        case .clip(let clip):
            // Near a filled rule's edge: the page clipped to the tile, in `layout`.
            return clippedTileImage(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, rect: r, clip: clip, layout: layout,
                                    background: appearance.background, smoothFonts: smoothFonts) { draw(prepared, forms: forms, in: $0, appearance: appearance, cull: $1) }
                ?? wholePage()
        case .translate: break
        }
        guard r.width > 0, r.height > 0,
              let ctx = CGContext(data: nil, width: r.width, height: r.height, bitsPerComponent: 8, bytesPerRow: r.width * 4,
                                  space: tileSpace(), bitmapInfo: layout.bitmapInfo) else { return nil }
        drawTile(prepared, forms: forms, scale: scale, rect: r, in: ctx, appearance: appearance, smoothFonts: smoothFonts)
        return ctx.makeImage()
    }

    /// Parallel workers for a tile job.
    public static let tileWorkers = max(1, min(ProcessInfo.processInfo.activeProcessorCount, 8))

    /// Tiles `rects` of `prepared` at `scale`, each an IOSurface (BGRA,
    /// tagged sRGB: a layer shows it without a copy or a colour conversion
    /// in the main thread's commit), in the order given. Translatable pages
    /// draw each tile on its own, in parallel, except tiles near a filled
    /// rule's edge (`tilesNeedingPageContext`), which come from the page
    /// clipped to them; clip-exact pages draw the page clipped to `rects`;
    /// others, and the tiles no clip may draw (`tileRoutes`), are cut from
    /// the page's whole raster: `pageRaster` (the pane's kept
    /// `DL3PageRaster`, on the calling thread) or, without it, one
    /// `DL3PageRaster` drawn for this call. Safe off the main thread.
    public static func rasterizeTiles(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double,
                                      rects: [DL3PixelRect], appearance: DL3Appearance = .light, smoothFonts: Bool = false,
                                      pageRaster: (([DL3PixelRect]) -> [IOSurface?]?)? = nil) -> [IOSurface?] {
        guard !rects.isEmpty else { return [] }
        // A scale that is not finite and positive, or a page too large to address: no tiles (never a trap).
        guard DL3PageRaster.checkedSize(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale) != nil else { return rects.map { _ in nil } }
        let routes = tileRoutes(prepared, scale: scale, rects: rects)
        var out = [IOSurface?](repeating: nil, count: rects.count)
        // Tiles cut from the page's raster: routed there, or their clipped raster could not be mapped.
        var whole = routes.indices.filter { routes[$0] == .pageRaster }
        func clipped(_ ks: [Int]) -> [IOSurface?]? {
            clippedTiles(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, rects: ks.map { rects[$0] },
                         clips: ks.map { k in if case .clip(let c) = routes[k] { c } else { rects[k] } },
                         background: appearance.background, smoothFonts: smoothFonts) { draw(prepared, forms: forms, in: $0, appearance: appearance, cull: $1) }
        }
        if !tilesByTranslation(prepared) {
            // Clip-exact: one raster clipped to all the clipped tiles.
            let ks = routes.indices.filter { if case .clip = routes[$0] { true } else { false } }
            if !ks.isEmpty {
                if let tiles = clipped(ks) { for (k, t) in zip(ks, tiles) { out[k] = t } } else { whole += ks }
            }
        } else {
            // Tiles near a rule's edge: the page's own context, clipped to the
            // tile (grown clear of rule edges); the others by translation. Each
            // tile on its own, in parallel (one rect is the cheap clip: one page
            // context clipped to all of them costs several times as much).
            var unmapped = [Bool](repeating: false, count: rects.count)
            let n = min(tileWorkers, rects.count)
            out.withUnsafeMutableBufferPointer { buffer in
                unmapped.withUnsafeMutableBufferPointer { failed in
                    let base = buffer.baseAddress!, failedBase = failed.baseAddress!
                    DispatchQueue.concurrentPerform(iterations: n) { worker in
                        var k = worker
                        while k < rects.count {
                            let r = rects[k]
                            switch routes[k] {
                            case .translate:
                                base[k] = tileSurface(width: r.width, height: r.height) { drawTile(prepared, forms: forms, scale: scale, rect: r, in: $0, appearance: appearance, smoothFonts: smoothFonts) }
                            case .clip:
                                if let t = clipped([k]) { base[k] = t[0] } else { failedBase[k] = true }
                            case .pageRaster, .none: break
                            }
                            k += n
                        }
                    }
                }
            }
            whole += unmapped.indices.filter { unmapped[$0] }
        }
        if !whole.isEmpty {
            whole.sort()
            let rs = whole.map { rects[$0] }
            let cut = pageRaster.map { $0(rs) } ?? DL3PageRaster(prepared, forms: forms, scale: scale, appearance: appearance, smoothFonts: smoothFonts)?.cut(rs)
            if let cut { for (k, t) in zip(whole, cut) { out[k] = t } }
        }
        return out
    }

    /// Tiles of a PDF page (the fallback for pages the display list cannot
    /// draw exactly), cut from one raster of it.
    public static func rasterizeTiles(pdfPage: CGPDFPage, scale: Double, rects: [DL3PixelRect], appearance: DL3Appearance = .light,
                                      smoothFonts: Bool = false) -> [IOSurface?] {
        guard let raster = DL3PageRaster(pdfPage: pdfPage, scale: scale, smoothFonts: smoothFonts), let cut = raster.cut(rects) else { return rects.map { _ in nil } }
        return pdfTiles(cut, appearance: appearance)
    }

    /// A PDF page's tiles in `appearance`: dark is `rasterizeToSurface(pdfPage:
    /// appearance: .dark)`'s Core Image pass (invert, then rotate hue by half
    /// a turn), applied tile by tile. Both filters work pixel by pixel, so a
    /// tile equals the same window of the dark whole page (TileParityTests).
    public static func pdfTiles(_ light: [IOSurface?], appearance: DL3Appearance) -> [IOSurface?] {
        guard appearance == .dark else { return light }
        return light.map { s in
            guard let s, let out = newSurface(width: s.width, height: s.height) else { return nil }
            let ci = CIImage(ioSurface: s)
                .applyingFilter("CIColorInvert")
                .applyingFilter("CIHueAdjust", parameters: [kCIInputAngleKey: Double.pi])
            ciContext.render(ci, to: out, bounds: CGRect(x: 0, y: 0, width: s.width, height: s.height),
                             colorSpace: CGColorSpace(name: CGColorSpace.sRGB))
            tag(out)
            return out
        }
    }

    /// Tiles `rects` of the page raster `body` draws, drawn with the page's
    /// own context (the configuration of `bitmapContext`, the same device
    /// coordinates) but clipped to `rects`, then copied out. For clip-exact
    /// pages (`clipExact`), and the tiles of a translatable page near a
    /// filled rule's edge (`tilesNeedingPageContext`).
    ///
    /// The context spans the page (its device origin is the page's
    /// bottom-left corner, so nothing is translated), but its memory is an
    /// anonymous mapping, which the system backs only where it is written:
    /// the clip and the white fill cover just the rects, so the resident size
    /// is about the clips' rows (`lastCutResidentBytes`), at any scale.
    /// `clips`: each rect's clip (`tileRoutes`; it contains the rect). Nil
    /// when the raster could not be mapped: the caller cuts the tiles from
    /// the page's raster instead.
    static func clippedTiles(widthPt: Double, heightPt: Double, scale: Double, rects: [DL3PixelRect], clips: [DL3PixelRect],
                             background: CGColor = CGColor(gray: 1, alpha: 1), smoothFonts: Bool = false,
                             _ body: (CGContext, [CGRect]) -> Void) -> [IOSurface?]? {
        clippedPage(widthPt: widthPt, heightPt: heightPt, scale: scale, clips: clips, layout: .screen, background: background,
                    smoothFonts: smoothFonts, body) { base, W, H in copyTiles(rects, from: base, width: W, height: H) }
    }

    /// One tile in `layout`, from the page clipped to `clip` (`clippedTiles`'s
    /// raster in that layout: the RGBA and BGRA rasterisers can differ by a
    /// coverage level at a rule's edge, so a tile keeps its page's layout).
    /// Nil when the raster could not be mapped.
    static func clippedTileImage(widthPt: Double, heightPt: Double, scale: Double, rect r: DL3PixelRect, clip: DL3PixelRect, layout: Layout,
                                 background: CGColor = CGColor(gray: 1, alpha: 1), smoothFonts: Bool = false,
                                 _ body: (CGContext, [CGRect]) -> Void) -> CGImage? {
        let image: CGImage?? = clippedPage(widthPt: widthPt, heightPt: heightPt, scale: scale, clips: [clip], layout: layout, background: background,
                                           smoothFonts: smoothFonts, body) { base, W, _ in
            var bytes = Data(count: r.width * r.height * 4)
            bytes.withUnsafeMutableBytes { o in
                for row in 0 ..< r.height { memcpy(o.baseAddress! + row * r.width * 4, base + (r.y + row) * W * 4 + r.x * 4, r.width * 4) }
            }
            guard let provider = CGDataProvider(data: bytes as CFData) else { return nil }
            return CGImage(width: r.width, height: r.height, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: r.width * 4, space: tileSpace(),
                           bitmapInfo: CGBitmapInfo(rawValue: layout.bitmapInfo), provider: provider, decode: nil, shouldInterpolate: false,
                           intent: .defaultIntent)
        }
        return image ?? nil
    }

    /// The page raster `body` draws, clipped to `clips` (page pixels, top-left
    /// origin, grown clear of rule edges by `tileRoutes`; clips outside the
    /// page are left out), in `layout`, over an anonymous mapping backed only
    /// where it is written; `body` gets the context and the clip in user
    /// space, `read` the raster's base address, width and height. Nil when
    /// the mapping (page-sized address space) or its context failed.
    ///
    /// (Mapping only the clips' rows would shift the context's device
    /// origin, which is a translation and not exact; the address space is
    /// page-sized, the memory behind it only the clips' rows.)
    static func clippedPage<R>(widthPt: Double, heightPt: Double, scale: Double, clips: [DL3PixelRect], layout: Layout,
                               background: CGColor, smoothFonts: Bool, _ body: (CGContext, [CGRect]) -> Void,
                               read: (UnsafePointer<UInt8>, Int, Int) -> R) -> R? {
        guard let (W, H, size) = DL3PageRaster.checkedSize(widthPt: widthPt, heightPt: heightPt, scale: scale) else { return nil }
        let ok = clips.filter { $0.width > 0 && $0.height > 0 && $0.x >= 0 && $0.y >= 0 && $0.x + $0.width <= W && $0.y + $0.height <= H }
        let stride = W * 4
        guard !ok.isEmpty, W > 0, H > 0, !takeFailedMap() else { return nil }
        let mem = mmap(nil, size, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0)
        guard let mem, mem != MAP_FAILED else { return nil }
        defer { munmap(mem, size) }
        guard let ctx = CGContext(data: mem, width: W, height: H, bitsPerComponent: 8, bytesPerRow: stride,
                                  space: tileSpace(), bitmapInfo: layout.bitmapInfo) else { return nil }
        // Device rects (y up) of the requested tiles: clip, then the page's ground.
        let device = ok.map { CGRect(x: $0.x, y: H - $0.y - $0.height, width: $0.width, height: $0.height) }
        ctx.clip(to: device)
        ctx.setFillColor(background)
        ctx.fill(device)
        configurePage(ctx, scale: scale, smoothFonts: smoothFonts)
        // The clip in user space, for `body` to skip glyphs outside it (`draw(cull:)`).
        body(ctx, device.map { CGRect(x: $0.minX / scale, y: $0.minY / scale, width: $0.width / scale, height: $0.height / scale) })
        ctx.flush()
        if measureResidency { recordResidency(resident(mem, size)) }
        return read(mem.assumingMemoryBound(to: UInt8.self), W, H)
    }

    /// The page context configuration after the background (as `bitmapContext`).
    static func configurePage(_ ctx: CGContext, scale: Double, smoothFonts: Bool = false) {
        ctx.scaleBy(x: scale, y: scale)
        ctx.setShouldAntialias(true)
        setFontSmoothing(smoothFonts, in: ctx) // as the whole page (#1304)
        ctx.setAllowsFontSubpixelPositioning(true)
        ctx.setShouldSubpixelPositionFonts(true)
    }

    /// Each rect of a BGRA page raster (top row first) copied into its own surface.
    static func copyTiles(_ rects: [DL3PixelRect], from base: UnsafePointer<UInt8>, width W: Int, height H: Int) -> [IOSurface?] {
        let stride = W * 4
        var out = [IOSurface?](repeating: nil, count: rects.count)
        out.withUnsafeMutableBufferPointer { buffer in
            let dst = buffer.baseAddress!
            DispatchQueue.concurrentPerform(iterations: rects.count) { k in
                let r = rects[k]
                guard r.width > 0, r.height > 0, r.x >= 0, r.y >= 0, r.x + r.width <= W, r.y + r.height <= H,
                      let s = newSurface(width: r.width, height: r.height) else { return }
                s.lock(options: [], seed: nil)
                let o = s.baseAddress.assumingMemoryBound(to: UInt8.self), os = s.bytesPerRow
                for row in 0 ..< r.height { memcpy(o + row * os, base + (r.y + row) * stride + r.x * 4, r.width * 4) }
                s.unlock(options: [], seed: nil)
                tag(s)
                dst[k] = s
            }
        }
        return out
    }

    /// Resident bytes of a mapping (`mincore`).
    static func resident(_ mem: UnsafeMutableRawPointer, _ size: Int) -> Int {
        let page = Int(getpagesize())
        var vec = [CChar](repeating: 0, count: (size + page - 1) / page)
        guard mincore(mem, size, &vec) == 0 else { return 0 }
        return vec.reduce(0) { $0 + (($1 & 1) != 0 ? page : 0) }
    }

    /// Measurement (benches, tests): the resident size of each clipped
    /// raster. Every access is under `residencyLock`, so tile jobs may read
    /// the flag off the main thread.
    private static let residencyLock = NSLock()
    nonisolated(unsafe) private static var _measuring = false, _lastResident = 0, _maxResident = 0, _failMaps = 0
    public static var measureResidency: Bool {
        get { residencyLock.lock(); defer { residencyLock.unlock() }; return _measuring }
        set { residencyLock.lock(); _measuring = newValue; residencyLock.unlock() }
    }
    /// Resident bytes of the last clipped raster, and the largest since `resetResidency`.
    public static var lastCutResidentBytes: Int { residencyLock.lock(); defer { residencyLock.unlock() }; return _lastResident }
    public static var maxCutResidentBytes: Int { residencyLock.lock(); defer { residencyLock.unlock() }; return _maxResident }
    public static func resetResidency() { residencyLock.lock(); _lastResident = 0; _maxResident = 0; residencyLock.unlock() }
    static func recordResidency(_ bytes: Int) {
        residencyLock.lock(); _lastResident = bytes; _maxResident = max(_maxResident, bytes); residencyLock.unlock()
    }

    static func newSurface(width w: Int, height h: Int) -> IOSurface? {
        guard w > 0, h > 0 else { return nil }
        return IOSurface(properties: [.width: w, .height: h, .bytesPerElement: 4, .pixelFormat: 0x4247_5241 /* 'BGRA' */])
    }

    static func tag(_ s: IOSurface) {
        if let plist = tileSpace().copyPropertyList() { IOSurfaceSetValue(s, kIOSurfaceColorSpace, plist) }
    }

    /// A surface of w×h px drawn by `body` into a context over its memory.
    static func tileSurface(width w: Int, height h: Int, _ body: (CGContext) -> Void) -> IOSurface? {
        guard let s = newSurface(width: w, height: h) else { return nil }
        s.lock(options: [], seed: nil)
        defer { s.unlock(options: [], seed: nil) }
        guard let ctx = CGContext(data: s.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: s.bytesPerRow,
                                  space: tileSpace(), bitmapInfo: Layout.screen.bitmapInfo) else { return nil }
        body(ctx)
        ctx.flush()
        tag(s)
        return s
    }
}

/// One full-scale page raster, kept by the pane for a page drawn whole
/// (paths, clips, images, forms, stroked text; PDF fallbacks) and cut into
/// tiles by every tile job of that page's source: drawn once per page
/// content, scale and appearance, not once per job.
///
/// The pixels are anonymous **purgeable** memory (`mach_vm_allocate` with
/// `VM_FLAGS_PURGABLE`), never a file: nothing is written to disk (a
/// file-backed mapping wrote the whole raster to the SSD on every redraw,
/// review of #1287 @2cd7cfc8e). The raster is nonvolatile while it is drawn,
/// until its first cut, and while tiles are cut (`cut`), and volatile otherwise: volatile
/// pages are not part of the process's footprint, and the kernel may purge
/// them under memory pressure. `cut` reports a purged raster (nil), and its
/// owner draws it again. Its pixels are exactly `rasterizeToSurface`'s (same
/// configuration, ground and coordinates).
public final class DL3PageRaster: @unchecked Sendable {
    public let width: Int, height: Int
    public let scale: Double
    private let address: mach_vm_address_t
    private let size: Int
    private let lock = NSLock()

    /// The page raster's pixel size at `scale`, or nil when the scale is not
    /// finite and positive or the size does not fit (`W·4·H` overflows, or
    /// beyond 1 Mpx a side).
    public static func checkedSize(widthPt: Double, heightPt: Double, scale: Double) -> (width: Int, height: Int, bytes: Int)? {
        guard scale.isFinite, scale > 0, widthPt.isFinite, heightPt.isFinite, widthPt > 0, heightPt > 0 else { return nil }
        let w = (widthPt * scale).rounded(.up), h = (heightPt * scale).rounded(.up)
        guard w >= 1, h >= 1, w <= Double(1 << 20), h <= Double(1 << 20) else { return nil }
        let (row, o1) = Int(w).multipliedReportingOverflow(by: 4)
        let (bytes, o2) = row.multipliedReportingOverflow(by: Int(h))
        guard !o1, !o2 else { return nil }
        return (Int(w), Int(h), bytes)
    }

    init?(widthPt: Double, heightPt: Double, scale: Double, background: CGColor = CGColor(gray: 1, alpha: 1), _ body: (CGContext) -> Void) {
        guard let (W, H, size) = Self.checkedSize(widthPt: widthPt, heightPt: heightPt, scale: scale) else { return nil }
        var addr: mach_vm_address_t = 0
        guard mach_vm_allocate(mach_task_self_, &addr, mach_vm_size_t(size), VM_FLAGS_ANYWHERE | VM_FLAGS_PURGABLE) == KERN_SUCCESS,
              let mem = UnsafeMutableRawPointer(bitPattern: UInt(addr)) else { return nil }
        guard let ctx = CGContext(data: mem, width: W, height: H, bitsPerComponent: 8, bytesPerRow: W * 4,
                                  space: DL3Renderer.tileSpace(), bitmapInfo: DL3Renderer.Layout.screen.bitmapInfo) else {
            mach_vm_deallocate(mach_task_self_, addr, mach_vm_size_t(size)); return nil
        }
        ctx.setFillColor(background)
        ctx.fill(CGRect(x: 0, y: 0, width: W, height: H))
        DL3Renderer.configurePage(ctx, scale: scale)
        body(ctx)
        ctx.flush()
        // Nonvolatile until the first `cut` (which leaves it volatile): a raster
        // is never purged between being drawn and being used.
        self.width = W; self.height = H; self.scale = scale; self.address = addr; self.size = size
    }

    /// `prepared` drawn whole at `scale` in `appearance` (as `rasterizeToSurface`).
    public convenience init?(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double, appearance: DL3Appearance = .light,
                             smoothFonts: Bool = false) {
        self.init(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, background: appearance.background) {
            // Settings > Smooth fonts in preview (#1304); off leaves the context as it was.
            if smoothFonts { DL3Renderer.setFontSmoothing(true, in: $0) }
            DL3Renderer.draw(prepared, forms: forms, in: $0, appearance: appearance)
        }
    }

    /// A PDF page (the fallback) drawn whole at `scale`, light: its grid is
    /// its media box's (`gridSize(pdfPage:scale:)`); a dark pane passes its
    /// tiles through `DL3Renderer.pdfTiles`.
    public convenience init?(pdfPage: CGPDFPage, scale: Double, smoothFonts: Bool = false) {
        let box = pdfPage.getBoxRect(.mediaBox)
        self.init(widthPt: box.width, heightPt: box.height, scale: scale) { ctx in
            if smoothFonts { DL3Renderer.setFontSmoothing(true, in: ctx) }
            ctx.translateBy(x: -box.minX, y: -box.minY)
            ctx.drawPDFPage(pdfPage)
        }
    }

    /// The pixel grid a PDF page's raster (and so its tiles) has at `scale`.
    public static func gridSize(pdfPage: CGPDFPage, scale: Double) -> (width: Int, height: Int) {
        let box = pdfPage.getBoxRect(.mediaBox)
        return DL3Renderer.pixelSize(widthPt: box.width, heightPt: box.height, scale: scale)
    }

    deinit { mach_vm_deallocate(mach_task_self_, address, mach_vm_size_t(size)) }

    /// Sets the purgeable state; returns the previous one.
    private func setState(_ s: Int32) -> Int32 {
        var state = s
        guard mach_vm_purgable_control(mach_task_self_, address, VM_PURGABLE_SET_STATE, &state) == KERN_SUCCESS else { return -1 }
        return state
    }

    /// Bytes of the raster.
    public var bytes: Int { size }
    /// Whether the kernel purged it (tests).
    public var purged: Bool {
        lock.lock(); defer { lock.unlock() }
        var state: Int32 = 0
        guard mach_vm_purgable_control(mach_task_self_, address, VM_PURGABLE_GET_STATE, &state) == KERN_SUCCESS else { return true }
        return state == VM_PURGABLE_EMPTY
    }
    /// Bytes of it resident now (`mincore`).
    public var residentBytes: Int {
        guard let mem = UnsafeMutableRawPointer(bitPattern: UInt(address)) else { return 0 }
        return DL3Renderer.resident(mem, size)
    }

    /// Purges it now, as the kernel may under memory pressure (tests).
    public func purgeForTesting() {
        lock.lock(); defer { lock.unlock() }
        var state: Int32 = VM_PURGABLE_EMPTY
        _ = mach_vm_purgable_control(mach_task_self_, address, VM_PURGABLE_SET_STATE, &state)
    }

    /// Tiles `rects` (page pixels, top-left origin), each copied into its own
    /// surface; nil when the kernel purged the raster (draw it again).
    public func cut(_ rects: [DL3PixelRect]) -> [IOSurface?]? {
        lock.lock(); defer { lock.unlock() }
        guard let mem = UnsafeMutableRawPointer(bitPattern: UInt(address)) else { return nil }
        let previous = setState(VM_PURGABLE_NONVOLATILE)
        defer { _ = setState(VM_PURGABLE_VOLATILE) }
        guard previous != VM_PURGABLE_EMPTY, previous >= 0 else { return nil }
        return DL3Renderer.copyTiles(rects, from: mem.assumingMemoryBound(to: UInt8.self), width: width, height: height)
    }
}
