import XCTest
import AppKit
import CoreGraphics
import QuartzCore
import FlashTeXProtocol
@testable import FlashTeXMac

/// High-zoom tiling (V2TileGrid, DESIGN §6.2): tiles are pixel-exact windows
/// of the whole-page raster, so V2Parity's zero tolerance holds for tiled
/// pages; a tiled page holds only the tiles around its visible rect.
final class PreviewV2TileTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")

    func frame(_ name: String = "display-list-v2-text.json") throws -> V2Frame {
        try V2Frame.prepare(RenderingV2.decode(try Data(contentsOf: Self.fixtures.appendingPathComponent(name))), store: PreviewV2ParityTests.store)
    }

    /// Runs the main run loop until `done` (off-main tile passes land there).
    @MainActor
    static func settle(timeout: TimeInterval = 10, _ done: () -> Bool) {
        let deadline = Date().addingTimeInterval(timeout)
        while !done(), Date() < deadline { RunLoop.main.run(until: Date().addingTimeInterval(0.01)) }
    }

    /// Pastes every tile of `page` at `scale` into one page-sized bitmap.
    static func assembled(_ page: V2PreparedPage, scale: Double, dark: Bool = false) throws -> CGImage {
        let (w, h) = GlyphRunRenderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: scale)
        let ctx = try XCTUnwrap(CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                          space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        let all = V2TileGrid.indices(covering: CGRect(x: 0, y: 0, width: w, height: h), pageWidth: w, pageHeight: h)
        let source = V2TileSource(page: page, pageToken: "t", pixelsPerPoint: scale, displayScale: 2, dark: dark)
        for (index, tile) in zip(all, V2TileGrid.rasterize(all, of: source)) {
            let r = V2TileGrid.rect(index, pageWidth: w, pageHeight: h)
            let image = try XCTUnwrap(tile)
            XCTAssertEqual(image.width, r.width)
            XCTAssertEqual(image.height, r.height)
            ctx.draw(image, in: CGRect(x: r.x, y: h - r.y - r.height, width: r.width, height: r.height))
        }
        return try XCTUnwrap(ctx.makeImage())
    }

    func testGridPartitionsThePageExactly() {
        let (w, h) = (4896, 6336) // US letter at 8 px/pt
        let all = V2TileGrid.indices(covering: CGRect(x: 0, y: 0, width: w, height: h), pageWidth: w, pageHeight: h)
        XCTAssertEqual(all.count, 10 * 13)
        XCTAssertEqual(all.reduce(0) { let r = V2TileGrid.rect($1, pageWidth: w, pageHeight: h); return $0 + r.width * r.height }, w * h)
        XCTAssertEqual(V2TileGrid.rect(V2TileGrid.Index(column: 9, row: 12), pageWidth: w, pageHeight: h),
                       V2TileGrid.PixelRect(x: 4608, y: 6144, width: 288, height: 192))
        // A rect straddling a tile edge by one pixel takes both tiles; one ending on the edge does not.
        XCTAssertEqual(V2TileGrid.indices(covering: CGRect(x: 511, y: 0, width: 2, height: 1), pageWidth: w, pageHeight: h).count, 2)
        XCTAssertEqual(V2TileGrid.indices(covering: CGRect(x: 0, y: 0, width: 512, height: 512), pageWidth: w, pageHeight: h).count, 1)
        XCTAssertEqual(V2TileGrid.indices(covering: CGRect(x: -900, y: -900, width: 100, height: 100), pageWidth: w, pageHeight: h), [])
        XCTAssertFalse(V2TileGrid.tiles(2))
        XCTAssertFalse(V2TileGrid.tiles(3))
        XCTAssertTrue(V2TileGrid.tiles(3.5))
        XCTAssertTrue(V2TileGrid.tiles(8))
    }

    /// The zero-tolerance identity: the tiles of a page, pasted together, are
    /// byte-identical to the whole-page raster (whole-pixel translation only),
    /// at the tiled scales, fractional ones and the dark preview included.
    func testTilesArePixelExactWindowsOfTheWholePage() throws {
        let cases: [(String, Double, Bool)] = [("display-list-v2-text.json", 8, false), ("display-list-v2-text.json", 3.5, false),
                                               ("display-list-v2-text.json", 6.25, true), ("display-list-v2-math.json", 4, false)]
        for (name, scale, dark) in cases {
            let page = try frame(name).prepared[0]
            do {
                let whole = try XCTUnwrap(GlyphRunRenderer.rasterize(page, scale: scale, dark: dark))
                let tiled = try Self.assembled(page, scale: scale, dark: dark)
                XCTAssertEqual(V2Parity.differingPixels(V2Parity.rgba(whole), V2Parity.rgba(tiled)), 0, "\(name) at \(scale) px/pt dark=\(dark)")
            }
        }
    }

    /// And therefore V2Parity's export identity carries over: at the pinned
    /// scale the tiles equal the exported PDF's raster, pixel for pixel.
    func testTiledPageEqualsTheExportedPDFRaster() throws {
        let frame = try frame()
        for scale in PreviewV2ParityTests.pinnedScales {
            var exports: [Int: CGImage] = [:]
            let report = V2Parity.compare(frame: frame, scale: scale) { exports[$0.page] = $0.export }
            XCTAssertTrue(report.identical)
            for page in frame.prepared {
                let tiled = try Self.assembled(page, scale: scale)
                let export = try XCTUnwrap(exports[page.number])
                XCTAssertEqual(V2Parity.differingPixels(V2Parity.rgba(tiled), V2Parity.rgba(export)), 0, "page \(page.number) at \(scale) px/pt")
            }
        }
    }

    /// A tiled page view inside a scroll view: tiles only around the visible
    /// rect, positioned 1 px = 1/displayScale pt from the page's top-left,
    /// re-tiled on scroll with far tiles dropped, and re-rasterized in place
    /// for a new page token (one paint recorded).
    @MainActor
    func testTiledPageViewHoldsOnlyTheVisibleTiles() throws {
        let frame = try frame()
        let page = frame.prepared[0]
        let source = V2TileSource(page: page, pageToken: "a", pixelsPerPoint: 8, displayScale: 2, dark: false)
        let (pw, ph) = source.pixelSize
        let viewSize = NSSize(width: page.widthPt * 4, height: page.heightPt * 4)
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 500, height: 400))
        let doc = FlippedView(frame: NSRect(origin: .zero, size: viewSize))
        let view = PageBitmapView(frame: NSRect(origin: .zero, size: viewSize))
        doc.addSubview(view)
        scroll.documentView = doc
        let white = CGColor(gray: 1, alpha: 1)
        XCTAssertTrue(view.show(nil, tiles: source, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: white))
        // 500×400 pt at the top-left = 1000×800 px: columns 0...1, rows 0...1 at once, synchronously…
        XCTAssertEqual(view.tileIndices, Set((0...1).flatMap { r in (0...1).map { V2TileGrid.Index(column: $0, row: r) } }))
        XCTAssertEqual(view.installs, 1)
        // …and the 256 px prefetch margin (column 2, row 2) from off-main.
        Self.settle { view.tileCount == 9 }
        XCTAssertEqual(view.tileIndices, Set((0...2).flatMap { r in (0...2).map { V2TileGrid.Index(column: $0, row: r) } }))
        XCTAssertEqual(view.prefetchedTiles, 5)
        let first = V2TileGrid.Index(column: 1, row: 2)
        XCTAssertEqual(view.tileFrame(first), CGRect(x: 256, y: viewSize.height - 3 * 256, width: 256, height: 256))
        let expected = try XCTUnwrap(GlyphRunRenderer.rasterizeTile(page, scale: 8, rect: V2TileGrid.rect(first, pageWidth: pw, pageHeight: ph)))
        XCTAssertEqual(V2Parity.rgba(try XCTUnwrap(view.tileImage(first))), V2Parity.rgba(expected))
        XCTAssertLessThan(view.retainedBytes, 10 << 20, "9 tiles of 1 MB, not a 124 MB page")

        // Scroll down 2000 pt: the top rows go, the rows around the new viewport arrive.
        scroll.contentView.scroll(to: NSPoint(x: 0, y: 2000))
        Self.settle { Set(view.tileIndices.map(\.row)).contains(4800 / 512) }
        let rows = Set(view.tileIndices.map(\.row))
        XCTAssertFalse(rows.contains(0))
        XCTAssertTrue(rows.contains(4000 / 512) && rows.contains(4800 / 512))
        XCTAssertLessThanOrEqual(view.tileCount, 5 * 4)

        // Same page, same scale: nothing re-rasterized.
        let before = view.tileRasterizations
        XCTAssertFalse(view.show(nil, tiles: source, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: white))
        XCTAssertEqual(view.tileRasterizations, before)
        // A new page token re-rasterizes the held visible tiles in place, and counts one paint.
        let next = V2TileSource(page: page, pageToken: "b", pixelsPerPoint: 8, displayScale: 2, dark: true)
        let held = view.tileIndices
        XCTAssertTrue(view.show(nil, tiles: next, pageToken: "b", pageNumber: 1, frameRevision: 2, expectedDraws: 1, background: white))
        XCTAssertEqual(view.installs, 2)
        XCTAssertTrue(view.tileIndices.isSubset(of: held))
        let dark = try XCTUnwrap(GlyphRunRenderer.rasterizeTile(page, scale: 8, dark: true, rect: V2TileGrid.rect(try XCTUnwrap(view.tileIndices.first), pageWidth: pw, pageHeight: ph)))
        XCTAssertEqual(V2Parity.rgba(try XCTUnwrap(view.tileImage(try XCTUnwrap(view.tileIndices.first)))), V2Parity.rgba(dark))

        // Scrolled out of view: every tile is dropped.
        doc.setFrameSize(NSSize(width: viewSize.width, height: viewSize.height + 5000))
        view.setFrameOrigin(NSPoint(x: 0, y: 5000))
        scroll.contentView.scroll(to: .zero)
        XCTAssertEqual(view.tileCount, 0)

        // Leaving tiled mode removes the tiles and shows a whole-page bitmap again.
        let bitmap = try XCTUnwrap(GlyphRunRenderer.rasterize(page, scale: 1))
        XCTAssertTrue(view.show(bitmap, pageToken: "b", pageNumber: 1, frameRevision: 3, expectedDraws: 1, background: white))
        XCTAssertNil(view.tileSource)
        XCTAssertTrue(view.layer?.contents as! CGImage === bitmap)
    }

    /// A keystroke at high zoom (same scale, new page token): the previous
    /// tiles stay on screen, the new ones arrive from off-main together.
    @MainActor
    func testNewPageContentAtTheSameScaleSwapsTilesInOffMain() throws {
        let frame = try frame()
        let page = frame.prepared[0]
        let viewSize = NSSize(width: page.widthPt * 4, height: page.heightPt * 4)
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 500, height: 400))
        let doc = FlippedView(frame: NSRect(origin: .zero, size: viewSize))
        let view = PageBitmapView(frame: NSRect(origin: .zero, size: viewSize))
        doc.addSubview(view)
        scroll.documentView = doc
        let white = CGColor(gray: 1, alpha: 1)
        let a = V2TileSource(page: page, pageToken: "a", pixelsPerPoint: 8, displayScale: 2, dark: false)
        view.show(nil, tiles: a, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: white)
        Self.settle { view.tileCount == 9 } // the prefetch margin too
        let index = V2TileGrid.Index(column: 0, row: 0)
        let old = try XCTUnwrap(view.tileImage(index))
        let held = view.tileIndices
        // Different content under the same token scheme: the math page at the same scale.
        let other = try self.frame("display-list-v2-math.json").prepared[0]
        let b = V2TileSource(page: other, pageToken: "b", pixelsPerPoint: 8, displayScale: 2, dark: false)
        XCTAssertTrue(view.show(nil, tiles: b, pageToken: "b", pageNumber: 1, frameRevision: 2, expectedDraws: 1, background: white))
        XCTAssertTrue(view.tileImage(index) === old, "the previous tile stays up while the new one rasterizes")
        XCTAssertEqual(view.installs, 1)
        let deadline = Date().addingTimeInterval(10)
        while view.installs < 2, Date() < deadline { RunLoop.main.run(until: Date().addingTimeInterval(0.01)) }
        XCTAssertEqual(view.installs, 2)
        XCTAssertEqual(view.tileIndices, held)
        let (pw, ph) = b.pixelSize
        let expected = try XCTUnwrap(GlyphRunRenderer.rasterizeTile(other, scale: 8, rect: V2TileGrid.rect(index, pageWidth: pw, pageHeight: ph)))
        XCTAssertEqual(V2Parity.rgba(try XCTUnwrap(view.tileImage(index))), V2Parity.rgba(expected))
    }

    /// The tile layers compose the whole page: rendering the view's layer tree
    /// reproduces the whole-page raster (orientation and placement).
    @MainActor
    func testTileLayersComposeTheWholePage() throws {
        let page = try frame().prepared[0]
        let source = V2TileSource(page: page, pageToken: "a", pixelsPerPoint: 4, displayScale: 2, dark: false)
        let (pw, ph) = source.pixelSize
        let view = PageBitmapView(frame: NSRect(x: 0, y: 0, width: page.widthPt * 2, height: page.heightPt * 2))
        view.show(nil, tiles: source, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: CGColor(gray: 1, alpha: 1))
        XCTAssertEqual(view.tileCount, V2TileGrid.indices(covering: CGRect(x: 0, y: 0, width: pw, height: ph), pageWidth: pw, pageHeight: ph).count)
        let ctx = try XCTUnwrap(CGContext(data: nil, width: pw, height: ph, bitsPerComponent: 8, bytesPerRow: pw * 4,
                                          space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        // Layer space is y-up view points; the bitmap is page pixels. The page's
        // pixel height exceeds the exact view height by a fraction of a pixel:
        // align the tops, as the tile frames do.
        ctx.translateBy(x: 0, y: CGFloat(ph) - CGFloat(source.viewHeight) * 2)
        ctx.scaleBy(x: 2, y: 2)
        try XCTUnwrap(view.layer).render(in: ctx)
        let rendered = try XCTUnwrap(ctx.makeImage())
        let whole = try XCTUnwrap(GlyphRunRenderer.rasterize(page, scale: 4))
        let differing = V2Parity.differingPixels(V2Parity.rgba(rendered), V2Parity.rgba(whole))
        // renderInContext resamples through CoreGraphics, so this checks placement, not bytes:
        // a flipped or shifted tile would differ on a large share of the ink.
        XCTAssertLessThan(Double(differing) / Double(pw * ph), 0.002, "\(differing) differing pixel(s)")
    }

    /// The pinch transform scales about the viewport's top centre: that point
    /// stays put, a point below it moves down (away from it) by the factor.
    @MainActor
    func testPinchTransformKeepsTheViewportTopCentreFixed() {
        let layer = CALayer()
        layer.bounds = CGRect(x: 0, y: 300, width: 800, height: 600)
        layer.anchorPoint = .zero
        let q = CGPoint(x: 400, y: 300)
        let t = V2PinchTransform.transform(2, about: q, in: layer)
        func apply(_ p: CGPoint) -> CGPoint {
            // Sublayer transforms act about the anchor point in bounds space.
            let a = CGPoint(x: layer.bounds.minX + layer.anchorPoint.x * layer.bounds.width, y: layer.bounds.minY + layer.anchorPoint.y * layer.bounds.height)
            let m = CATransform3DGetAffineTransform(t)
            let r = CGPoint(x: p.x - a.x, y: p.y - a.y).applying(m)
            return CGPoint(x: r.x + a.x, y: r.y + a.y)
        }
        XCTAssertEqual(apply(q).x, q.x, accuracy: 1e-9)
        XCTAssertEqual(apply(q).y, q.y, accuracy: 1e-9)
        XCTAssertEqual(apply(CGPoint(x: 500, y: 400)).x, 600, accuracy: 1e-9)
        XCTAssertEqual(apply(CGPoint(x: 500, y: 400)).y, 500, accuracy: 1e-9)
        XCTAssertTrue(CATransform3DIsIdentity(V2PinchTransform.transform(1, about: q, in: layer)))
    }

    /// Evidence capture (not a gate): `FLASHTEX_TILE_BENCH_LIST=<display list>`
    /// times the whole-page raster at 8 px/pt against the tiles of a viewport
    /// and one scroll step, and writes JSON to `FLASHTEX_TILE_BENCH_OUT`.
    func testTileBenchmark() throws {
        let env = ProcessInfo.processInfo.environment
        guard let list = env["FLASHTEX_TILE_BENCH_LIST"] else { throw XCTSkip("set FLASHTEX_TILE_BENCH_LIST to run the tile benchmark") }
        let frame = try V2Frame.prepare(RenderingV2.decode(try Data(contentsOf: URL(fileURLWithPath: list))), store: PreviewV2ParityTests.store)
        let page = frame.prepared.max { $0.glyphCount < $1.glyphCount }!
        let scale = 8.0
        func median(_ runs: Int = 9, _ body: () -> Void) -> Double {
            var times: [Double] = []
            for _ in 0..<runs { let t = MonotonicClock.nowNs(); body(); times.append(Double(MonotonicClock.nowNs() &- t) / 1e6) }
            return times.sorted()[runs / 2]
        }
        var wholeBytes = 0
        _ = GlyphRunRenderer.rasterize(page, scale: scale) // warm fonts and caches
        let wholeMs = median { let i = GlyphRunRenderer.rasterize(page, scale: scale)!; wholeBytes = i.bytesPerRow * i.height }
        let source = V2TileSource(page: page, pageToken: "bench", pixelsPerPoint: scale, displayScale: 2, dark: false)
        let (pw, ph) = source.pixelSize
        // The viewport: a 1100×900 pt preview pane at 2x, over the page's middle, plus the prefetch margin.
        let viewport = CGRect(x: 800, y: 2400, width: 2200, height: 1800)
        let m = V2TileGrid.prefetchPixels
        let visible = V2TileGrid.indices(covering: viewport.insetBy(dx: -m, dy: -m), pageWidth: pw, pageHeight: ph)
        var tileBytes = 0
        let tilesMs = median { tileBytes = V2TileGrid.rasterize(visible, of: source).reduce(0) { $0 + $1!.bytesPerRow * $1!.height } }
        let sequentialMs = median {
            for i in visible { _ = GlyphRunRenderer.rasterizeTile(page, scale: scale, rect: V2TileGrid.rect(i, pageWidth: pw, pageHeight: ph)) }
        }
        let row = visible.filter { $0.row == visible.last!.row }
        let rowMs = median { _ = V2TileGrid.rasterize(row, of: source) }
        let oneMs = median { _ = GlyphRunRenderer.rasterizeTile(page, scale: scale, rect: V2TileGrid.rect(visible[visible.count / 2], pageWidth: pw, pageHeight: ph)) }
        let backdrop = GlyphRunRenderer.rasterize(page, scale: V2TileGrid.backdropPixelsPerPoint)!
        let backdropMs = median { _ = GlyphRunRenderer.rasterize(page, scale: V2TileGrid.backdropPixelsPerPoint) }
        let result: [String: Any] = [
            "list": list, "page": page.number, "glyphs": page.glyphCount, "px_per_pt": scale, "page_px": [pw, ph],
            "before_whole_page_ms_median": wholeMs, "before_whole_page_bytes": wholeBytes,
            "after_viewport_px": [2200, 1800], "after_tiles": visible.count, "after_tiles_ms_median": tilesMs, "after_tiles_sequential_ms_median": sequentialMs,
            "workers": V2TileGrid.workers, "after_tiles_bytes": tileBytes,
            "after_scroll_row_tiles": row.count, "after_scroll_row_ms_median": rowMs, "one_tile_ms_median": oneMs,
            "backdrop_px_per_pt": V2TileGrid.backdropPixelsPerPoint, "backdrop_bytes": backdrop.bytesPerRow * backdrop.height, "backdrop_ms_median": backdropMs,
            "cpus": ProcessInfo.processInfo.activeProcessorCount,
        ]
        let data = try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys])
        print(String(data: data, encoding: .utf8)!)
        if let out = env["FLASHTEX_TILE_BENCH_OUT"] { try data.write(to: URL(fileURLWithPath: out)) }
    }
}

/// Evidence capture for the font-smoothing decision (not a gate):
/// `FLASHTEX_SMOOTHING_OUT=<dir>` writes the exported PDF of the text page
/// (`FLASHTEX_SMOOTHING_LIST`, default the text fixture) and, for every
/// scale in `FLASHTEX_SMOOTHING_SCALES`, our raster of page 1 with Core
/// Graphics font smoothing off and on, to compare with Preview.app's ink
/// (docs/evidence/preview-smoothing-2026-09-29/).
final class PreviewSmoothingEvidenceTests: XCTestCase {
    func testWriteSmoothingEvidence() throws {
        let env = ProcessInfo.processInfo.environment
        guard let dir = env["FLASHTEX_SMOOTHING_OUT"] else { throw XCTSkip("set FLASHTEX_SMOOTHING_OUT to write the smoothing evidence") }
        let list = env["FLASHTEX_SMOOTHING_LIST"].map { URL(fileURLWithPath: $0) } ?? PreviewV2TileTests.fixtures.appendingPathComponent("display-list-v2-text.json")
        let frame = try V2Frame.prepare(RenderingV2.decode(try Data(contentsOf: list)), store: PreviewV2ParityTests.store)
        let out = URL(fileURLWithPath: dir)
        try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
        try GlyphRunRenderer.pdfData(frame: frame).write(to: out.appendingPathComponent("export.pdf"))
        for scale in (env["FLASHTEX_SMOOTHING_SCALES"] ?? "2").split(separator: ",").compactMap({ Double($0) }) {
            for smooth in [false, true] {
                let image = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: scale, smoothFonts: smooth))
                V2ParityEvidence.write(image, to: out.appendingPathComponent("ours-smooth-\(smooth ? "on" : "off")-\(scale).png"))
            }
        }
    }
}

private final class FlippedView: NSView {
    override var isFlipped: Bool { true }
}
