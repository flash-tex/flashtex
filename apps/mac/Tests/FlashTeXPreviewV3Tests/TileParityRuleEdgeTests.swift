import CoreGraphics
import Foundation
import IOSurface
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// Tiles drawn by translation stay exact windows of the whole page where a
/// filled rule's edge sits within a pixel of a tile boundary (review of
/// #1287 @c1b698c3b: a rule 0.004–0.05 px left of a boundary differed by one
/// coverage level between the tile and the page, which no edge rounding
/// fixes, because the tile's clip changes the other edge's coverage). Zero
/// tolerance, every tile, light and dark, IOSurface and RGBA, 3.25–16 px/pt.
/// Before tiles near a rule's edge came from the page's own context
/// (`tilesNeedingPageContext`), 64 of these 856 tiles differed.
///
/// Also the exact-position paths no checked-in fixture exercises: glyph
/// ORIGINS (spec §4.2, some on a subpixel phase boundary) and RULE_GEOMETRY
/// (spec §4.4, `re` under a CTM translation).
final class TileParityRuleEdgeTests: XCTestCase {
    static let scales: [Double] = [3.25, 4, 6, 8, 12, 16]
    static let tile = TileParityTests.tile
    static let K = DL3.spPerBp

    /// A seeded generator (the same rules every run).
    struct LCG {
        var s: UInt64
        mutating func next() -> Double {
            s = s &* 6364136223846793005 &+ 1442695040888963407
            return Double(s >> 11) / Double(1 << 53)
        }
    }

    /// A rule in page device pixels (y up, the page's bottom-left at 0) at `scale`.
    struct DeviceRule { var x0, y0, x1, y1: Double }

    /// A page of filled rules only (`tilesByTranslation`): rules whose left,
    /// right, top or bottom edge is 0–1.2 px either side of each internal
    /// 512 px tile boundary at `scale` (in the bottom tile row and the left
    /// tile column), rules at the page's edges, and rules anywhere. `geometry`: drawn
    /// from RULE_GEOMETRY (`re` under a translation) instead of sp.
    static func rulePage(widthPt: Double, heightPt: Double, scale s: Double, geometry: Bool) -> (DL3PreparedPage, Int) {
        let (wpx, hpx) = DL3Renderer.pixelSize(widthPt: widthPt, heightPt: heightPt, scale: s)
        let deltas: [Double] = [-1.2, -0.6, -0.3, -0.05, -0.02, -0.004, 0, 0.004, 0.02, 0.05, 0.3, 0.6, 1.2]
        let sizes: [Double] = [0.3, 0.8, 1.6, 7.5, 60, 300]
        var rules: [DeviceRule] = []
        var rng = LCG(s: UInt64(s * 1000))
        var k = 0
        // Vertical boundaries (x = 512 n): a rule's left or right edge just off them.
        for bx in stride(from: tile, to: wpx, by: tile) {
            for d in deltas {
                for leftEdge in [true, false] {
                    let w = sizes[k % sizes.count], h = sizes[(k / 2 + 3) % sizes.count]
                    let y0 = rng.next() * (Double(min(hpx, tile)) - h) // the bottom tile row: the others stay translated
                    k += 1
                    let e = Double(bx) + d
                    rules.append(leftEdge ? DeviceRule(x0: e, y0: y0, x1: e + w, y1: y0 + h) : DeviceRule(x0: e - w, y0: y0, x1: e, y1: y0 + h))
                }
            }
        }
        // Horizontal boundaries (image row 512 n is device y = hpx − 512 n): a top or bottom edge just off them.
        for row in stride(from: tile, to: hpx, by: tile) {
            let by = Double(hpx - row)
            for d in deltas {
                for bottomEdge in [true, false] {
                    let h = sizes[k % sizes.count], w = sizes[(k / 2 + 3) % sizes.count]
                    let x0 = rng.next() * (Double(min(wpx, tile)) - w) // the left tile column
                    k += 1
                    let e = by + d
                    rules.append(bottomEdge ? DeviceRule(x0: x0, y0: e, x1: x0 + w, y1: e + h) : DeviceRule(x0: x0, y0: e - h, x1: x0 + w, y1: e))
                }
            }
        }
        // A rule at each first-column boundary corner: two edges just off two boundaries.
        for bx in stride(from: tile, to: min(wpx, 2 * tile), by: tile) {
            for row in stride(from: tile, to: hpx, by: tile) {
                let d = deltas[k % deltas.count]; k += 1
                rules.append(DeviceRule(x0: Double(bx) + d, y0: Double(hpx - row) - d, x1: Double(bx) + d + 40, y1: Double(hpx - row) - d + 25))
            }
        }
        // The page's own edges: full-width and full-height bars (beamer's
        // headlines), and rules ending just inside the right and top edges.
        let (pw, ph) = (widthPt * s, heightPt * s)
        for i in 0 ..< 3 {
            let y = Double(hpx) * Double(i + 1) / 4 + deltas[(k + i) % deltas.count]
            rules.append(DeviceRule(x0: 0, y0: y, x1: pw, y1: y + sizes[(k + i) % sizes.count]))
            let x = Double(wpx) * Double(i + 1) / 4 + deltas[(k + i + 5) % deltas.count]
            rules.append(DeviceRule(x0: x, y0: 0, x1: x + sizes[(k + i + 1) % sizes.count], y1: ph))
        }
        for d in deltas where d <= 0 {
            let y0 = rng.next() * (Double(hpx) - 80), x0 = rng.next() * (Double(wpx) - 80)
            rules.append(DeviceRule(x0: pw + d - 30, y0: y0, x1: pw + d, y1: y0 + 20))
            rules.append(DeviceRule(x0: x0, y0: ph + d - 30, x1: x0 + 20, y1: ph + d))
        }
        // Anywhere: mostly away from the boundaries (the fast path), some across them.
        for _ in 0 ..< 120 {
            let w = 0.1 + rng.next() * 120, h = 0.1 + rng.next() * 120
            let x0 = rng.next() * (Double(wpx) - w), y0 = rng.next() * (Double(hpx) - h)
            rules.append(DeviceRule(x0: x0, y0: y0, x1: x0 + w, y1: y0 + h))
        }
        var p = DL3Page(kind: .page, index: 0)
        p.box = [0, 0, widthPt, heightPt]
        p.width = Int32((widthPt * K).rounded()); p.height = Int32((heightPt * K).rounded())
        let colors: [[Double]] = [[0], [0.35], [0.8, 0.1, 0.1], [0.1, 0.3, 0.9]]
        let (e, f) = (36.137_42, 18.459_31) // the CTM translation of RULE_GEOMETRY
        for (i, r) in rules.enumerated() {
            if i % 7 == 0 { p.items.append(.fillColor(colors[(i / 7) % colors.count])) }
            let left = r.x0 / s, right = r.x1 / s, bottom = r.y0 / s, top = r.y1 / s
            let x = Int32((left * K).rounded()), y = Int32(((heightPt - top) * K).rounded())
            p.items.append(.rule(kind: .fill, x: x, y: y, w: Int32((right * K).rounded()) - x, h: Int32(((heightPt - bottom) * K).rounded()) - y))
            if geometry { p.ruleGeometry.append([e, f, left - e, bottom - f, right - left, top - bottom, 0]) }
        }
        return (DL3PreparedPage(page: p, fonts: [:], images: [:]), rules.count)
    }

    struct Tally { var tiles = 0, differing = 0, pixels = 0 }

    /// Every tile of `page` at `scale` against the whole page: the pane's
    /// IOSurfaces (`rasterizeTiles`) against `rasterizeToSurface`, and RGBA
    /// tiles (`rasterizeTile`) against `rasterize`, in `appearance`.
    func compare(_ page: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double, appearance: DL3Appearance,
                 rgba: Bool = true, _ label: String) throws -> Tally {
        var t = Tally()
        try autoreleasepool {
            let (w, h) = DL3Renderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: scale)
            let rects = TileParityTests.rects(width: w, height: h)
            let surface = try XCTUnwrap(DL3Renderer.rasterizeToSurface(page, forms: forms, scale: scale, appearance: appearance).flatMap { DL3Renderer.image(of: $0) })
            let wholeSurface = DL3Parity.rgba(surface)
            let tiles = DL3Renderer.rasterizeTiles(page, forms: forms, scale: scale, rects: rects, appearance: appearance)
            XCTAssertEqual(tiles.count, rects.count)
            // All at once, then each on its own (its own clip: the pane's jobs ask for part of a page).
            let single = rects.map { DL3Renderer.rasterizeTiles(page, forms: forms, scale: scale, rects: [$0], appearance: appearance)[0] }
            for (kind, surfaces) in [("IOSurface", tiles), ("IOSurface single", single)] {
                for (s, r) in zip(surfaces, rects) {
                    let img = try XCTUnwrap(s.flatMap { DL3Renderer.image(of: $0) }, "\(label) \(r)")
                    let d = DL3Parity.diff(DL3Parity.rgba(img), TileParityTests.window(wholeSurface, pageWidth: w, r))
                    t.tiles += 1
                    if d.pixels > 0 { t.differing += 1; t.pixels += d.pixels; print("differs: \(label) \(kind) tile \(r): \(d.pixels) px, max \(d.maxDelta)") }
                }
            }
            guard rgba else { return }
            let whole = DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterize(page, forms: forms, scale: scale, layout: .rgba, appearance: appearance)))
            for r in rects {
                let img = try XCTUnwrap(DL3Renderer.rasterizeTile(page, forms: forms, scale: scale, rect: r, layout: .rgba, appearance: appearance))
                let d = DL3Parity.diff(DL3Parity.rgba(img), TileParityTests.window(whole, pageWidth: w, r))
                t.tiles += 1
                if d.pixels > 0 { t.differing += 1; t.pixels += d.pixels; print("differs: \(label) RGBA tile \(r): \(d.pixels) px, max \(d.maxDelta)") }
            }
        }
        return t
    }

    /// Filled rules with an edge 0–1.2 px off a tile boundary (left, right,
    /// top, bottom, corners), from sp and from RULE_GEOMETRY.
    func testFilledRulesNearTileBoundariesAreExact() throws {
        var total = Tally(), rules = 0, translated = 0, pageContext = 0
        for scale in Self.scales {
            for geometry in [false, true] {
                let (page, n) = Self.rulePage(widthPt: 240, heightPt: 180, scale: scale, geometry: geometry)
                rules += n
                XCTAssertTrue(DL3Renderer.tilesByTranslation(page), "a page of filled rules tiles by translation")
                let (w, h) = DL3Renderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: scale)
                let routes = DL3Renderer.tilesNeedingPageContext(page, scale: scale, rects: TileParityTests.rects(width: w, height: h))
                pageContext += routes.filter { $0 }.count; translated += routes.filter { !$0 }.count
                for appearance in [DL3Appearance.light, .dark] {
                    let label = "rules\(geometry ? " (RULE_GEOMETRY)" : "") \(appearance) at \(scale) px/pt"
                    let t = try compare(page, scale: scale, appearance: appearance, label)
                    XCTAssertEqual(t.differing, 0, "\(label): \(t.differing) of \(t.tiles) tiles differ (\(t.pixels) px)")
                    total.tiles += t.tiles; total.differing += t.differing; total.pixels += t.pixels
                }
            }
        }
        // Both routes are exercised: tiles near a rule's edge, and tiles by translation.
        XCTAssertGreaterThan(pageContext, 0); XCTAssertGreaterThan(translated, 0)
        print("rule-edge tiles: \(rules) rules, \(total.tiles) tiles compared, \(total.differing) differing (\(total.pixels) px); per page×scale \(pageContext) tiles from the page's context, \(translated) by translation")
    }

    /// `beamer-overlays` (glyphs and filled rules, tiled by translation) with
    /// exact glyph ORIGINS — every third x and every fifth y on a subpixel
    /// phase boundary at that scale, the rest off the sp grid — and
    /// RULE_GEOMETRY under a CTM translation.
    func testExactOriginsAndRuleGeometryTilesAreExact() throws {
        let doc = try DL3Document(frames: Array(try Data(contentsOf: TileParityTests.fixtures.appendingPathComponent("beamer-overlays.dl3"))))
        let pages = Array(doc.orderedPages.prefix(3))
        var total = Tally()
        for scale in Self.scales {
            for (n, base) in pages.enumerated() {
                var page = base
                var rng = LCG(s: UInt64(scale * 100) + UInt64(n))
                let H = page.page.box[3]
                // On a phase boundary: the device coordinate plus 0.001 is a multiple of ¼ px.
                func onBoundary(_ v: Double) -> Double { (((v * scale + 0.001) * 4).rounded() / 4 - 0.001) / scale }
                var gi = 0
                page.page.origins = page.page.items.compactMap { item -> DL3Origin? in
                    guard case .glyph(_, _, let x, let y, _) = item else { return nil }
                    defer { gi += 1 }
                    let ox = Double(x) / Self.K, oy = H - Double(y) / Self.K
                    return DL3Origin(x: gi % 3 == 0 ? onBoundary(ox) : ox + (rng.next() - 0.5) * 0.002,
                                     y: gi % 5 == 0 ? onBoundary(oy) : oy + (rng.next() - 0.5) * 0.002)
                }
                let (e, f) = (28.3465 + rng.next(), 14.1732 + rng.next())
                page.page.ruleGeometry = page.page.items.compactMap { item -> [Double]? in
                    guard case .rule(_, let x, let y, let w, let h) = item else { return nil }
                    let left = Double(x) / Self.K + (rng.next() - 0.5) * 0.002, top = H - Double(y) / Self.K
                    let width = Double(w) / Self.K, height = Double(h) / Self.K
                    return [e, f, left - e, top - height - f, width, height, 0]
                }
                XCTAssertGreaterThan(page.page.origins.count, 0)
                XCTAssertTrue(DL3Renderer.tilesByTranslation(page), "beamer-overlays p\(n + 1) tiles by translation")
                for appearance in [DL3Appearance.light, .dark] {
                    let label = "beamer-overlays p\(n + 1) ORIGINS+RULE_GEOMETRY \(appearance) at \(scale) px/pt"
                    let t = try compare(page, forms: doc.forms, scale: scale, appearance: appearance, rgba: appearance == .light, label)
                    XCTAssertEqual(t.differing, 0, "\(label): \(t.differing) of \(t.tiles) tiles differ (\(t.pixels) px)")
                    total.tiles += t.tiles; total.differing += t.differing; total.pixels += t.pixels
                }
            }
        }
        print("ORIGINS+RULE_GEOMETRY tiles: \(total.tiles) tiles compared, \(total.differing) differing (\(total.pixels) px)")
    }
}
