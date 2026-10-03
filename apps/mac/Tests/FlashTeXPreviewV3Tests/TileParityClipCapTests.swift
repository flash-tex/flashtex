import CoreGraphics
import Foundation
import IOSurface
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// Follow-ups of the review of #1287 (`DL3Tiles.swift`, `tileRoutes`):
///
/// - Inside a dense cluster of rules (edges less than 4 px apart) a
///   page-context clip grew to the whole page. It now grows at most
///   `clipGrowthMax` px a side; past that the tile is cut from the page's
///   raster. A rule that is not finite sends its page to the raster.
/// - Stroked rules near a clip's edge on a clip-exact page.
/// - Tiles drawn by translation (the fast path) on a page where most tiles
///   have no rule edge nearby.
/// - A clipped raster that cannot be mapped falls back to the page raster.
///
/// Zero tolerance, every tile, light and dark, IOSurface (all at once and one
/// by one) and RGBA, at 3.25–16 px/pt (`TileParityRuleEdgeTests.compare`).
final class TileParityClipCapTests: XCTestCase {
    typealias Rule = TileParityRuleEdgeTests.DeviceRule
    static let scales = TileParityRuleEdgeTests.scales
    static let tile = TileParityTests.tile

    /// Rules with edges `pitch` px apart (`width` px wide): vertical ones
    /// across `xs` spanning `ys`, horizontal ones across `ys` spanning `xs`.
    static func mesh(xs: ClosedRange<Double>?, ys: ClosedRange<Double>?, span: (x: ClosedRange<Double>, y: ClosedRange<Double>),
                     pitch: Double = 3.1, width: Double = 1.3) -> [Rule] {
        var out: [Rule] = []
        if let xs { for x in stride(from: xs.lowerBound, to: xs.upperBound, by: pitch) { out.append(Rule(x0: x, y0: span.y.lowerBound, x1: x + width, y1: span.y.upperBound)) } }
        if let ys { for y in stride(from: ys.lowerBound, to: ys.upperBound, by: pitch) { out.append(Rule(x0: span.x.lowerBound, y0: y, x1: span.x.upperBound, y1: y + width)) } }
        return out
    }

    func routes(_ page: DL3PreparedPage, scale: Double) -> [DL3Renderer.TileRoute] {
        let (w, h) = DL3Renderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: scale)
        return DL3Renderer.tileRoutes(page, scale: scale, rects: TileParityTests.rects(width: w, height: h))
    }

    func count(_ routes: [DL3Renderer.TileRoute]) -> (translate: Int, clip: Int, raster: Int) {
        routes.reduce((0, 0, 0)) { t, r in
            switch r {
            case .translate: (t.0 + 1, t.1, t.2)
            case .clip: (t.0, t.1 + 1, t.2)
            case .pageRaster: (t.0, t.1, t.2 + 1)
            case .none: t
            }
        }
    }

    /// A page-wide mesh of rules 3.1 px apart (every tile's clip would grow
    /// to the whole page), and a band of them across the page's middle
    /// (some tiles clipped or translated, the band's cut from the raster);
    /// filled (by translation) and half stroked (clip-exact). Every tile is
    /// exact, and no clipped raster backs more than `clipResidentBound`.
    func testDenseRuleClustersAreExactAndBounded() throws {
        let (wPt, hPt) = (160.0, 120.0)
        var total = TileParityRuleEdgeTests.Tally(), routed = (translate: 0, clip: 0, raster: 0)
        DL3Renderer.measureResidency = true
        defer { DL3Renderer.measureResidency = false }
        for (n, scale) in Self.scales.enumerated() {
            let (W, H) = (wPt * scale, hPt * scale)
            let full = (x: 0.0 ... W, y: 0.0 ... H)
            let cases: [(String, [Rule])] = [
                ("page-wide mesh", Self.mesh(xs: 0.4 ... W - 2, ys: 0.4 ... H - 2, span: full)),
                ("band", Self.mesh(xs: nil, ys: H * 0.35 ... H * 0.65, span: full)
                    + [Rule(x0: W * 0.2 + 0.37, y0: H * 0.1, x1: W * 0.6, y1: H * 0.2 + 0.61)]),
            ]
            for (name, rules) in cases {
                for stroked in [false, true] {
                    let geometry = n % 2 == 1
                    let page = TileParityRuleEdgeTests.page(rules, widthPt: wPt, heightPt: hPt, scale: scale, geometry: geometry, stroked: stroked)
                    XCTAssertEqual(DL3Renderer.tilesByTranslation(page), !stroked); XCTAssertTrue(DL3Renderer.clipExact(page))
                    let rs = routes(page, scale: scale), r = count(rs)
                    if name == "page-wide mesh" {
                        // Every tile from the page raster, or (a page within `clipGrowthMax` of one tile) clipped to the whole page.
                        let (w, h) = DL3Renderer.pixelSize(widthPt: wPt, heightPt: hPt, scale: scale)
                        let all = DL3PixelRect(x: 0, y: 0, width: w, height: h)
                        XCTAssertTrue(rs.allSatisfy { $0 == .pageRaster || $0 == .clip(all) }, "\(name) at \(scale): \(rs)")
                        if w > Self.tile + DL3Renderer.clipGrowthMax { XCTAssertEqual(r.clip, 0, "\(name) at \(scale)") }
                    }
                    routed.translate += r.translate; routed.clip += r.clip; routed.raster += r.raster
                    for appearance in [DL3Appearance.light, .dark] {
                        let label = "\(name)\(stroked ? " stroked" : "")\(geometry ? " (RULE_GEOMETRY)" : "") \(appearance) at \(scale) px/pt"
                        DL3Renderer.resetResidency()
                        let t = try TileParityRuleEdgeTests.compare(page, scale: scale, appearance: appearance, label)
                        XCTAssertEqual(t.differing, 0, "\(label): \(t.differing) of \(t.tiles) tiles differ (\(t.pixels) px)")
                        total.tiles += t.tiles; total.differing += t.differing; total.pixels += t.pixels
                        // One clip per raster (translated pages); a clip-exact job's raster holds all its clips.
                        let (w, _) = DL3Renderer.pixelSize(widthPt: wPt, heightPt: hPt, scale: scale)
                        let bound = DL3Renderer.clipResidentBound(DL3PixelRect(x: 0, y: 0, width: Self.tile, height: Self.tile), stride: w * 4)
                        XCTAssertLessThanOrEqual(DL3Renderer.maxCutResidentBytes, bound * (stroked ? max(1, r.clip) : 1), label)
                    }
                }
            }
        }
        XCTAssertGreaterThan(routed.raster, 0); XCTAssertGreaterThan(routed.clip, 0); XCTAssertGreaterThan(routed.translate, 0)
        print("dense rule clusters: \(total.tiles) tiles compared, \(total.differing) differing (\(total.pixels) px); routes \(routed)")
    }

    /// The cap takes no tile of the checked-in pages off its route: at
    /// 3.25–20 px/pt every tile of a translatable or clip-exact page is
    /// still translated or clipped (never cut from a page raster).
    func testCheckedInPagesKeepTheirRoutes() throws {
        var routed = (translate: 0, clip: 0, raster: 0)
        for name in ["beamer-overlays", "tile-text", "tile-paths"] {
            let doc = try DL3Document(frames: Array(try Data(contentsOf: TileParityTests.fixtures.appendingPathComponent("\(name).dl3"))))
            for page in doc.orderedPages where DL3Renderer.tilesByTranslation(page) || DL3Renderer.clipExact(page) {
                for scale in Self.scales + [20] {
                    let r = count(routes(page, scale: scale))
                    XCTAssertEqual(r.raster, 0, "\(name) p\(page.page.index + 1) at \(scale)")
                    routed.translate += r.translate; routed.clip += r.clip; routed.raster += r.raster
                }
            }
        }
        print("checked-in pages: routes \(routed)")
    }

    /// The memory the review measured: a letter page at 16 px/pt (a 496 MB
    /// raster) with a band of rules 3.1 px apart across it. Before the cap
    /// each clip in the band grew to the page's full width and the band's
    /// full height; now no clipped raster backs more than
    /// `clipResidentBound`, the band's tiles come from one page raster, and
    /// a 3×4 block across the band's edge is exact.
    func testDenseClusterMemoryAtSixteenPxPerPt() throws {
        let (wPt, hPt, scale) = (612.0, 792.0, 16.0)
        let (W, H) = (wPt * scale, hPt * scale)
        // Horizontal rules across the page and vertical ones across the band's rows.
        let band = Self.mesh(xs: 0.4 ... W - 2, ys: H * 0.4 ... H * 0.6, span: (0 ... W, H * 0.4 ... H * 0.6))
        let page = TileParityRuleEdgeTests.page(band, widthPt: wPt, heightPt: hPt, scale: scale, geometry: false)
        let (w, h) = DL3Renderer.pixelSize(widthPt: wPt, heightPt: hPt, scale: scale)
        // Image rows: the band is rows 0.4–0.6 h; the block's top row of tiles is above it.
        let top = (Int(Double(h) * 0.4) / Self.tile - 1) * Self.tile, left = 3 * Self.tile
        let block = TileParityTests.rects(width: w, height: h).filter { $0.y >= top && $0.y < top + 4 * Self.tile && $0.x >= left && $0.x < left + 3 * Self.tile }
        XCTAssertEqual(block.count, 12)
        let rules = DL3Renderer.ruleRects(page, scale: scale)
        let inBand = try XCTUnwrap(block.last)
        let uncapped = try XCTUnwrap(DL3Renderer.clearClip(inBand, rules: rules, width: w, height: h, limit: .max))
        XCTAssertNil(DL3Renderer.clearClip(inBand, rules: rules, width: w, height: h))
        let r = count(DL3Renderer.tileRoutes(page, scale: scale, rects: block))
        XCTAssertGreaterThan(r.raster, 0); XCTAssertGreaterThan(r.translate, 0)
        DL3Renderer.measureResidency = true
        defer { DL3Renderer.measureResidency = false }
        DL3Renderer.resetResidency()
        var rasterCalls = 0
        let surfaces = DL3Renderer.rasterizeTiles(page, scale: scale, rects: block, pageRaster: { rs in
            rasterCalls += 1
            return DL3PageRaster(page, scale: scale)?.cut(rs)
        })
        let bound = DL3Renderer.clipResidentBound(DL3PixelRect(x: 0, y: 0, width: Self.tile, height: Self.tile), stride: w * 4)
        XCTAssertEqual(rasterCalls, 1, "one page raster for the job")
        XCTAssertLessThanOrEqual(DL3Renderer.maxCutResidentBytes, bound)
        let whole = try XCTUnwrap(DL3Renderer.rasterizeToSurface(page, scale: scale).flatMap { DL3Renderer.image(of: $0) })
        let pixels = DL3Parity.rgba(whole)
        for (s, rect) in zip(surfaces, block) {
            let img = try XCTUnwrap(s.flatMap { DL3Renderer.image(of: $0) }, "\(rect)")
            XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(img), TileParityTests.window(pixels, pageWidth: w, rect)).pixels, 0, "\(rect)")
        }
        print("dense band at 16 px/pt: routes \(r); uncapped clip of tile \(inBand) was \(uncapped) (\(uncapped.width * uncapped.height * 4) bytes of raster); clipped rasters now back at most \(DL3Renderer.maxCutResidentBytes) bytes (bound \(bound)); page raster \(w * h * 4) bytes")
    }

    /// A rule whose RULE_GEOMETRY is not finite (NaN, ±∞): the page is drawn
    /// whole (`tilesByTranslation` and `clipExact` false, so the pane keeps
    /// its raster). One that overflows only at a scale (1e308) sends every
    /// tile at that scale to the page raster. Both exact.
    func testNonFiniteRulesSendThePageToItsRaster() throws {
        for scale in [3.25, 8, 16] {
            let (base, _) = TileParityRuleEdgeTests.rulePage(widthPt: 240, heightPt: 180, scale: scale, geometry: true)
            for (label, bad) in [("NaN", [Double.nan, 0, 10, 10, 20, 20, 0]), ("inf", [0, 0, .infinity, 10, 20, 20, 0]),
                                 ("-inf width", [0, 0, 10, 10, -.infinity, 20, 0]), ("overflow", [1e308, 0, 1e308, 10, 20, 20, 0])] {
                var page = base
                page.page.ruleGeometry[page.page.ruleGeometry.count / 2] = bad
                let finite = label == "overflow"
                XCTAssertEqual(DL3Renderer.tilesByTranslation(page), finite, label); XCTAssertEqual(DL3Renderer.clipExact(page), finite, label)
                XCTAssertTrue(routes(page, scale: scale).allSatisfy { $0 == .pageRaster }, "\(label) at \(scale)")
                XCTAssertTrue(DL3Renderer.ruleRects(page, scale: scale).contains { $0.isInfinite }, label)
                XCTAssertNil(DL3Renderer.clearClip(DL3PixelRect(x: 0, y: 0, width: 512, height: 512), rules: DL3Renderer.ruleRects(page, scale: scale),
                                                   width: 4096, height: 4096), label)
                for appearance in [DL3Appearance.light, .dark] {
                    let t = try TileParityRuleEdgeTests.compare(page, scale: scale, appearance: appearance, "\(label) \(appearance) at \(scale)")
                    XCTAssertEqual(t.differing, 0, "\(label) \(appearance) at \(scale): \(t.differing) of \(t.tiles) tiles differ")
                }
            }
        }
    }

    /// Stroked rules (table lines) on a clip-exact page with an outline edge
    /// 0–1.2 px either side of each tile boundary, from sp and from
    /// RULE_GEOMETRY (`m l S`): the grown clips keep every tile exact.
    func testStrokedRulesNearClipEdgesOnAClipExactPage() throws {
        var total = TileParityRuleEdgeTests.Tally(), clipped = 0, grown = 0
        for scale in Self.scales {
            for geometry in [false, true] {
                // The rule-edge page's rules, every other one stroked.
                let (filled, _) = TileParityRuleEdgeTests.rulePage(widthPt: 240, heightPt: 180, scale: scale, geometry: false)
                let H = 180.0, K = TileParityRuleEdgeTests.K
                let rules = filled.page.items.compactMap { item -> Rule? in
                    guard case .rule(_, let x, let y, let w, let h) = item else { return nil }
                    return Rule(x0: Double(x) / K * scale, y0: (H - Double(y + h) / K) * scale, x1: Double(x + w) / K * scale, y1: (H - Double(y) / K) * scale)
                }
                let page = TileParityRuleEdgeTests.page(rules, widthPt: 240, heightPt: H, scale: scale, geometry: geometry, stroked: true)
                XCTAssertFalse(DL3Renderer.tilesByTranslation(page)); XCTAssertTrue(DL3Renderer.clipExact(page))
                let (w, h) = DL3Renderer.pixelSize(widthPt: 240, heightPt: H, scale: scale)
                for (r, route) in zip(TileParityTests.rects(width: w, height: h), routes(page, scale: scale)) {
                    guard case .clip(let c) = route else { XCTFail("\(route) at \(scale)"); continue }
                    clipped += 1
                    if c != r { grown += 1 }
                }
                for appearance in [DL3Appearance.light, .dark] {
                    let label = "stroked rules\(geometry ? " (RULE_GEOMETRY)" : "") \(appearance) at \(scale) px/pt"
                    let t = try TileParityRuleEdgeTests.compare(page, scale: scale, appearance: appearance, label)
                    XCTAssertEqual(t.differing, 0, "\(label): \(t.differing) of \(t.tiles) tiles differ (\(t.pixels) px)")
                    total.tiles += t.tiles; total.differing += t.differing; total.pixels += t.pixels
                }
            }
        }
        XCTAssertGreaterThan(grown, 0, "some clips grew clear of a stroked rule's edge")
        print("stroked rules near clip edges: \(total.tiles) tiles compared, \(total.differing) differing (\(total.pixels) px); \(grown) of \(clipped) clips grown")
    }

    /// The fast path: rules everywhere (many across tile boundaries), their
    /// edges at least 3 px from every boundary and the page's edges, except
    /// a few: most tiles are drawn by translation, all exactly. Alone, and
    /// over `beamer-overlays` p1 (glyphs).
    func testTranslatedTilesAwayFromRuleEdgesAreExact() throws {
        let doc = try DL3Document(frames: Array(try Data(contentsOf: TileParityTests.fixtures.appendingPathComponent("beamer-overlays.dl3"))))
        let beamer = try XCTUnwrap(doc.orderedPages.first)
        var total = TileParityRuleEdgeTests.Tally(), routed = (translate: 0, clip: 0, raster: 0), beamerRouted = (translate: 0, clip: 0)
        for scale in Self.scales {
            for base in [nil, beamer] {
                let (wPt, hPt) = base.map { ($0.widthPt, $0.heightPt) } ?? (240, 180)
                let (w, h) = DL3Renderer.pixelSize(widthPt: wPt, heightPt: hPt, scale: scale)
                var rng = TileParityRuleEdgeTests.LCG(s: UInt64(scale * 977) + (base == nil ? 0 : 1))
                // Away from every 512 px boundary (and so from the page's own edges, kept 4 px clear).
                func away(_ v: Double, limit: Int) -> Double {
                    let m = v.truncatingRemainder(dividingBy: Double(Self.tile))
                    let u = m < 3 ? v + 3.5 - m : m > Double(Self.tile) - 3 ? v + Double(Self.tile) - m + 3.5 : v
                    return min(max(u, 4), Double(limit) - 4)
                }
                var rules: [Rule] = []
                for k in 0 ..< 400 {
                    let wide = k % 3 == 0
                    let rw = wide ? 50 + rng.next() * 900 : 0.2 + rng.next() * 40, rh = wide ? 0.2 + rng.next() * 12 : 0.2 + rng.next() * 900
                    // x in columns; y in image rows (tile row boundaries are rows 512 n, device y = h − 512 n).
                    let x0 = away(rng.next() * Double(w), limit: w), r0 = away(rng.next() * Double(h), limit: h)
                    let x1 = away(x0 + rw, limit: w), r1 = away(r0 + rh, limit: h)
                    guard x1 - x0 >= 0.2, r1 - r0 >= 0.2 else { continue }
                    rules.append(Rule(x0: x0, y0: Double(h) - r1, x1: x1, y1: Double(h) - r0))
                }
                // A few right on a boundary: those tiles come from the page's context.
                rules.append(Rule(x0: Double(Self.tile) - 0.02, y0: 40, x1: Double(Self.tile) + 30, y1: 52))
                rules.append(Rule(x0: 40, y0: Double(h - Self.tile) + 0.3, x1: 300, y1: Double(h - Self.tile) + 4))
                let page = TileParityRuleEdgeTests.page(rules, widthPt: wPt, heightPt: hPt, scale: scale, geometry: base == nil, base: base)
                XCTAssertTrue(DL3Renderer.tilesByTranslation(page))
                let r = count(routes(page, scale: scale))
                XCTAssertEqual(r.raster, 0, "at \(scale)")
                if base == nil {
                    // Only the tiles the two rules on a boundary reach (two each at most) leave the fast path.
                    XCTAssertLessThanOrEqual(r.clip, 4, "at \(scale): \(r)")
                    routed.translate += r.translate; routed.clip += r.clip; routed.raster += r.raster
                } else { beamerRouted.translate += r.translate; beamerRouted.clip += r.clip }
                for appearance in [DL3Appearance.light, .dark] {
                    let label = "\(base == nil ? "rules" : "beamer-overlays p1 + rules") \(appearance) at \(scale) px/pt"
                    let t = try TileParityRuleEdgeTests.compare(page, forms: doc.forms, scale: scale, appearance: appearance, label)
                    XCTAssertEqual(t.differing, 0, "\(label): \(t.differing) of \(t.tiles) tiles differ (\(t.pixels) px)")
                    total.tiles += t.tiles; total.differing += t.differing; total.pixels += t.pixels
                }
            }
        }
        // The synthetic page: most tiles by translation (every one away from the two rules on a boundary).
        XCTAssertGreaterThan(routed.clip, 0)
        XCTAssertGreaterThanOrEqual(Double(routed.translate), 0.75 * Double(routed.translate + routed.clip), "most tiles by translation: \(routed)")
        XCTAssertGreaterThan(beamerRouted.translate, 0)
        print("translated tiles: \(total.tiles) tiles compared, \(total.differing) differing (\(total.pixels) px); routes: rules \(routed), beamer-overlays p1 + rules \(beamerRouted)")
    }

    /// A clipped raster that cannot be mapped (address space, memory) is not
    /// a missing tile: it is cut from the page's raster, exactly. Near-edge
    /// tiles of a translated page, and a clip-exact page (`tile-text`).
    func testUnmappedClipRastersFallBackToThePageRaster() throws {
        defer { DL3Renderer.failNextClipMapsForTesting(0) }
        let doc = try DL3Document(frames: Array(try Data(contentsOf: TileParityTests.fixtures.appendingPathComponent("tile-text.dl3"))))
        let text = try XCTUnwrap(doc.orderedPages.first)
        let (rules, _) = TileParityRuleEdgeTests.rulePage(widthPt: 240, heightPt: 180, scale: 8, geometry: false)
        for (label, page, forms) in [("rules", rules, [UInt32: DL3PreparedPage]()), ("tile-text", text, doc.forms)] {
            let scale = 8.0
            let (w, h) = DL3Renderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: scale)
            let rects = TileParityTests.rects(width: w, height: h)
            XCTAssertTrue(DL3Renderer.tileRoutes(page, scale: scale, rects: rects).contains { if case .clip = $0 { true } else { false } }, label)
            let whole = DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterizeToSurface(page, forms: forms, scale: scale).flatMap { DL3Renderer.image(of: $0) }))
            DL3Renderer.failNextClipMapsForTesting(Int.max)
            var calls = 0
            let surfaces = DL3Renderer.rasterizeTiles(page, forms: forms, scale: scale, rects: rects, pageRaster: { rs in
                calls += 1
                return DL3PageRaster(page, forms: forms, scale: scale)?.cut(rs)
            })
            DL3Renderer.failNextClipMapsForTesting(0)
            XCTAssertEqual(calls, 1, label)
            for (s, r) in zip(surfaces, rects) {
                let img = try XCTUnwrap(s.flatMap { DL3Renderer.image(of: $0) }, "\(label) \(r): no tile")
                XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(img), TileParityTests.window(whole, pageWidth: w, r)).pixels, 0, "\(label) \(r)")
            }
            // RGBA (`rasterizeTile`): the whole page in RGBA, cropped.
            let rgba = DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterize(page, forms: forms, scale: scale, layout: .rgba)))
            DL3Renderer.failNextClipMapsForTesting(Int.max)
            for r in rects.prefix(6) {
                let img = try XCTUnwrap(DL3Renderer.rasterizeTile(page, forms: forms, scale: scale, rect: r, layout: .rgba), "\(label) \(r)")
                XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(img), TileParityTests.window(rgba, pageWidth: w, r)).pixels, 0, "\(label) RGBA \(r)")
            }
            DL3Renderer.failNextClipMapsForTesting(0)
        }
    }
}
