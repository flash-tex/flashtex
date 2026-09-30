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

    /// Windows hosting the views under test (never shown or activated).
    private var windows: [NSWindow] = []
    override func tearDown() {
        for w in windows { w.contentView = nil }
        windows.removeAll()
        super.tearDown()
    }

    /// Puts `root` in an offscreen, never-ordered window: a page view asks
    /// for tiles only once it is in a window, and converts them to its
    /// colour space (sRGB here, so tiles equal the sRGB rasters byte for byte).
    @MainActor
    func host(_ root: NSView, colorSpace: NSColorSpace = .sRGB) {
        let window = NSWindow(contentRect: root.frame, styleMask: .borderless, backing: .buffered, defer: true)
        window.isReleasedWhenClosed = false
        window.colorSpace = colorSpace
        window.contentView = root
        windows.append(window)
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
                                          space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: GlyphRunRenderer.bitmapInfo))
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

    /// The tiling sweep: every tiled scale from 3.25 to 8 px/pt in 0.25 steps.
    static let sweepScales = stride(from: 3.25, through: 8.0, by: 0.25).map { $0 }
    /// Fixtures of the sweep. `display-list-v2-dense.json` is the evidence's
    /// `dense.tex` (docs/evidence/preview-smoothing-2026-09-29/), 3,675 glyphs
    /// on page 1, whose tile (4, 1) at 6 px/pt once differed by 168 px
    /// (review of #1228); its runs' clusters are collapsed to one each, which
    /// leaves every glyph, position, font and paint unchanged.
    static let sweepFixtures = ["display-list-v2-dense.json", "display-list-v2-text.json",
                                "display-list-v2-math.json", "display-list-v2-math-rules.json"]

    /// Tiles of `page` at `scale` that differ from the same window of the
    /// whole-page raster, with their differing pixel counts (none: pixel-exact).
    static func differingTiles(_ page: V2PreparedPage, scale: Double, dark: Bool = false) throws -> [(V2TileGrid.Index, Int)] {
        let whole = try XCTUnwrap(GlyphRunRenderer.rasterize(page, scale: scale, dark: dark))
        let tiled = try assembled(page, scale: scale, dark: dark)
        // Both are bitmap-context images of one pixel format: compare their bytes row by row.
        XCTAssertEqual(whole.bitmapInfo, tiled.bitmapInfo)
        XCTAssertEqual(whole.bitsPerPixel, 32)
        let a = try XCTUnwrap(whole.dataProvider?.data) as Data, b = try XCTUnwrap(tiled.dataProvider?.data) as Data
        let (w, h) = (whole.width, whole.height)
        XCTAssertEqual(tiled.width, w)
        XCTAssertEqual(tiled.height, h)
        var out: [V2TileGrid.Index: Int] = [:]
        a.withUnsafeBytes { (pa: UnsafeRawBufferPointer) in
            b.withUnsafeBytes { (pb: UnsafeRawBufferPointer) in
                for y in 0..<h {
                    let ra = pa.baseAddress! + y * whole.bytesPerRow, rb = pb.baseAddress! + y * tiled.bytesPerRow
                    guard memcmp(ra, rb, w * 4) != 0 else { continue }
                    let ua = ra.assumingMemoryBound(to: UInt32.self), ub = rb.assumingMemoryBound(to: UInt32.self)
                    for x in 0..<w where ua[x] != ub[x] {
                        out[V2TileGrid.Index(column: x / V2TileGrid.tilePixels, row: y / V2TileGrid.tilePixels), default: 0] += 1
                    }
                }
            }
        }
        return out.sorted { ($0.key.row, $0.key.column) < ($1.key.row, $1.key.column) }.map { ($0.key, $0.value) }
    }

    /// The zero-tolerance identity at every tiled scale: each tile of every
    /// page, pasted at its place, is byte-identical to the whole-page raster.
    /// Glyph and rule pages are translated tiles whose glyph origins and rule
    /// edges round as on the page (`GlyphRunRenderer.tileOrigin`, `tileRect`);
    /// the TikZ page (paths, clips, dashes) is cut from a whole-page raster.
    func testTilesArePixelExactWindowsOfTheWholePageAcrossTheTiledScales() throws {
        var failures: [String] = []
        var pages = 0, tiles = 0
        let paths = try V2Frame.prepare(RenderingV2.decode(V2PathTests.data(V2PathTests.list())), store: PreviewV2ParityTests.store)
        XCTAssertFalse(paths.prepared[0].tilesByTranslation)
        let frames = try Self.sweepFixtures.map { ($0, try frame($0)) } + [("paths", paths)]
        for (name, frame) in frames {
            for page in frame.prepared {
                pages += 1
                for scale in Self.sweepScales {
                    let (w, h) = GlyphRunRenderer.pixelSize(widthPt: page.widthPt, heightPt: page.heightPt, scale: scale)
                    tiles += V2TileGrid.indices(covering: CGRect(x: 0, y: 0, width: w, height: h), pageWidth: w, pageHeight: h).count
                    for (index, pixels) in try Self.differingTiles(page, scale: scale) {
                        failures.append("\(name) page \(page.number) at \(scale) px/pt: tile (\(index.column), \(index.row)) differs in \(pixels) px")
                    }
                }
            }
        }
        XCTAssertEqual(pages, 6)
        XCTAssertEqual(failures, [], "\(failures.count) of \(tiles) tiles differ")
    }

    /// The review's glyph: device x 2127.99899999999980 px at 6 px/pt, whose
    /// `+ 0.001` rounds to 2128 on the page but stays below 80 in its tile.
    /// `tileCoordinate` moves it 1e-7 px onto the page's side; an origin far
    /// from a phase boundary passes unchanged.
    func testTileCoordinateDecidesAsThePageDoes() {
        let x = 354.6665, s = 6.0
        let d = s * x
        XCTAssertEqual((d + 0.001).rounded(.down), 2128)
        XCTAssertEqual((d - 2048 + 0.001).rounded(.down), 79, "the hazard: the tile rounds the other way")
        let t = GlyphRunRenderer.tileCoordinate(x, scale: s)
        let dt = s * t
        XCTAssertEqual((dt - 2048 + 0.001).rounded(.down), 80)
        XCTAssertEqual((dt + 0.001).rounded(.down), 2128)
        XCTAssertEqual(abs(dt - d), 1e-7, accuracy: 1e-9)
        XCTAssertEqual(GlyphRunRenderer.tileCoordinate(354.6666, scale: s), 354.6666)
        // Just below a boundary the page stays down, and so does the tile.
        let below = (2127.999 - 1e-8) / s
        let db = s * GlyphRunRenderer.tileCoordinate(below, scale: s)
        XCTAssertEqual((db - 2048 + 0.001).rounded(.down), 79)
    }

    /// The dark preview and the review's exact case (dense page 1, 6 px/pt).
    func testTilesArePixelExactInTheDarkPreviewAndAtTheReviewedCase() throws {
        let dense = try frame("display-list-v2-dense.json").prepared[0]
        XCTAssertEqual(dense.glyphCount, 3675)
        XCTAssertEqual(try Self.differingTiles(dense, scale: 6).map { "\($0.0) \($0.1)" }, [])
        for (name, scale) in [("display-list-v2-text.json", 6.25), ("display-list-v2-dense.json", 4.5)] {
            let page = try frame(name).prepared[0]
            XCTAssertEqual(try Self.differingTiles(page, scale: scale, dark: true).map { "\($0.0) \($0.1)" }, [], "\(name) at \(scale) px/pt, dark")
        }
    }

    /// And therefore V2Parity's export identity carries over: the tiles
    /// equal the exported PDF's raster pixel for pixel, at the pinned scales
    /// and at tiled ones.
    func testTiledPageEqualsTheExportedPDFRaster() throws {
        for name in ["display-list-v2-text.json", "display-list-v2-dense.json"] {
            let frame = try frame(name)
            for scale in PreviewV2ParityTests.pinnedScales + [3.5, 6] {
                var exports: [Int: CGImage] = [:]
                let report = V2Parity.compare(frame: frame, scale: scale) { exports[$0.page] = $0.export }
                XCTAssertTrue(report.identical, "\(name) at \(scale) px/pt: \(report.pages.map(\.differingPixels))")
                for page in frame.prepared {
                    let tiled = try Self.assembled(page, scale: scale)
                    let export = try XCTUnwrap(exports[page.number])
                    XCTAssertEqual(V2Parity.differingPixels(V2Parity.rgba(tiled), V2Parity.rgba(export)), 0, "\(name) page \(page.number) at \(scale) px/pt")
                }
            }
        }
    }

    /// A tiled page view inside a scroll view: tiles only around the visible
    /// rect, rasterized off the main thread (`show` and scrolling draw
    /// nothing), positioned 1 px = 1/displayScale pt from the page's
    /// top-left, re-tiled on scroll with far tiles dropped, and re-rasterized
    /// in place for a new page token (one paint recorded).
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
        host(scroll)
        let white = CGColor(gray: 1, alpha: 1)
        XCTAssertTrue(view.show(nil, tiles: source, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: white))
        // Nothing is drawn in `show`: two jobs are queued (the visible tiles, then the margin).
        XCTAssertEqual(view.tileCount, 0)
        XCTAssertEqual(view.tileRasterizations, 0)
        XCTAssertEqual(view.tileJobs, 2)
        XCTAssertEqual(view.installs, 0)
        // 500×400 pt at the top-left = 1000×800 px: columns 0...1, rows 0...1, plus the
        // 256 px prefetch margin (column 2, row 2).
        Self.settle { view.tileCount == 9 }
        XCTAssertEqual(view.tileIndices, Set((0...2).flatMap { r in (0...2).map { V2TileGrid.Index(column: $0, row: r) } }))
        XCTAssertEqual(view.installs, 1)
        let first = V2TileGrid.Index(column: 1, row: 2)
        XCTAssertEqual(view.tileFrame(first), CGRect(x: 256, y: viewSize.height - 3 * 256, width: 256, height: 256))
        let expected = try XCTUnwrap(GlyphRunRenderer.rasterizeTile(page, scale: 8, rect: V2TileGrid.rect(first, pageWidth: pw, pageHeight: ph)))
        XCTAssertEqual(V2Parity.rgba(try XCTUnwrap(view.tileImage(first))), V2Parity.rgba(expected))
        XCTAssertLessThan(view.retainedBytes, 10 << 20, "9 tiles of 1 MB, not a 124 MB page")

        // Scroll down 2000 pt: the top rows go at once (compositing only), the rows
        // around the new viewport arrive from the tile queue.
        let rasterized = view.tileRasterizations
        scroll.contentView.scroll(to: NSPoint(x: 0, y: 2000))
        XCTAssertEqual(view.tileRasterizations, rasterized, "scrolling draws nothing on the main thread")
        XCTAssertFalse(Set(view.tileIndices.map(\.row)).contains(0))
        Self.settle { Set(view.tileIndices.map(\.row)).contains(4800 / 512) }
        let rows = Set(view.tileIndices.map(\.row))
        XCTAssertFalse(rows.contains(0))
        XCTAssertTrue(rows.contains(4000 / 512) && rows.contains(4800 / 512))
        XCTAssertLessThanOrEqual(view.tileCount, 5 * 4)

        Self.settle { view.pendingTiles == 0 }
        // Same page, same scale: nothing re-rasterized.
        let before = view.tileRasterizations, jobs = view.tileJobs
        XCTAssertFalse(view.show(nil, tiles: source, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: white))
        XCTAssertEqual(view.tileRasterizations, before)
        XCTAssertEqual(view.tileJobs, jobs)
        // A new page token (here also the dark appearance) re-rasterizes the held tiles off-main;
        // the old ones stay up until theirs land, and one paint is counted.
        let next = V2TileSource(page: page, pageToken: "b", pixelsPerPoint: 8, displayScale: 2, dark: true)
        let held = view.tileIndices
        XCTAssertTrue(view.show(nil, tiles: next, pageToken: "b", pageNumber: 1, frameRevision: 2, expectedDraws: 1, background: white))
        XCTAssertEqual(view.tileIndices, held)
        XCTAssertEqual(view.staleCount, held.count)
        XCTAssertEqual(view.installs, 1)
        Self.settle { view.staleCount == 0 }
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
        host(scroll)
        let white = CGColor(gray: 1, alpha: 1)
        let a = V2TileSource(page: page, pageToken: "a", pixelsPerPoint: 8, displayScale: 2, dark: false)
        view.show(nil, tiles: a, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: white)
        Self.settle { view.tileCount == 9 } // the prefetch margin too
        XCTAssertEqual(view.installs, 1)
        let index = V2TileGrid.Index(column: 0, row: 0)
        let old = try XCTUnwrap(view.tileImage(index))
        let held = view.tileIndices
        // Different content under the same token scheme: the math page at the same scale.
        let other = try self.frame("display-list-v2-math.json").prepared[0]
        let b = V2TileSource(page: other, pageToken: "b", pixelsPerPoint: 8, displayScale: 2, dark: false)
        XCTAssertTrue(view.show(nil, tiles: b, pageToken: "b", pageNumber: 1, frameRevision: 2, expectedDraws: 1, background: white))
        XCTAssertTrue(view.tileImage(index) === old, "the previous tile stays up while the new one rasterizes")
        XCTAssertEqual(view.installs, 1)
        XCTAssertEqual(view.staleCount, 9)
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
        host(view)
        view.show(nil, tiles: source, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: CGColor(gray: 1, alpha: 1))
        let all = V2TileGrid.indices(covering: CGRect(x: 0, y: 0, width: pw, height: ph), pageWidth: pw, pageHeight: ph).count
        Self.settle { view.tileCount == all }
        XCTAssertEqual(view.tileCount, all)
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

    /// A committed zoom draws nothing on the main thread: the previous
    /// scale's tiles stay up, stretched to the new scale (linear), until the
    /// new scale's visible tiles land; then they go.
    @MainActor
    func testZoomShowsThePreviousScaleUntilTheNewTilesLand() throws {
        let page = try frame().prepared[0]
        let viewSize = NSSize(width: page.widthPt * 4, height: page.heightPt * 4)
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 500, height: 400))
        let doc = FlippedView(frame: NSRect(origin: .zero, size: viewSize))
        let view = PageBitmapView(frame: NSRect(origin: .zero, size: viewSize))
        doc.addSubview(view)
        scroll.documentView = doc
        host(scroll)
        let white = CGColor(gray: 1, alpha: 1)
        let at8 = V2TileSource(page: page, pageToken: "a", pixelsPerPoint: 8, displayScale: 2, dark: false)
        view.show(nil, tiles: at8, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: white)
        Self.settle { view.tileCount == 9 }
        let before = view.tileRasterizations

        // Zoom to 6 px/pt (3 view points per page point).
        let at6 = V2TileSource(page: page, pageToken: "a", pixelsPerPoint: 6, displayScale: 2, dark: false)
        XCTAssertTrue(view.show(nil, tiles: at6, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: white))
        XCTAssertEqual(view.tileRasterizations, before, "the zoom draws nothing on the main thread")
        XCTAssertEqual(view.tileCount, 0)
        XCTAssertEqual(view.outgoingCount, 9)
        // Stretched: tile (1, 1) of 8 px/pt (page points 64..128) now sits at 3 view points per point.
        view.setFrameSize(NSSize(width: page.widthPt * 3, height: page.heightPt * 3))
        XCTAssertEqual(view.outgoingCount, 9, "kept until the new scale's visible tiles land")
        XCTAssertTrue(view.outgoingFrames.contains(CGRect(x: 192, y: CGFloat(at6.viewHeight) - 384, width: 192, height: 192)))
        Self.settle { view.outgoingCount == 0 }
        XCTAssertGreaterThan(view.tileCount, 0)
        let (pw, ph) = at6.pixelSize
        for index in view.tileIndices {
            let expected = try XCTUnwrap(GlyphRunRenderer.rasterizeTile(page, scale: 6, rect: V2TileGrid.rect(index, pageWidth: pw, pageHeight: ph)))
            XCTAssertEqual(V2Parity.rgba(try XCTUnwrap(view.tileImage(index))), V2Parity.rgba(expected), "tile \(index)")
            let r = V2TileGrid.rect(index, pageWidth: pw, pageHeight: ph)
            XCTAssertEqual(view.tileFrame(index), CGRect(x: CGFloat(r.x) / 2, y: CGFloat(at6.viewHeight) - CGFloat(r.y + r.height) / 2,
                                                         width: CGFloat(r.width) / 2, height: CGFloat(r.height) / 2))
        }
    }

    /// Tiles queued for a source the view has since left are never
    /// installed: after three quick changes only the last one's tiles show.
    @MainActor
    func testTilesOfAnEarlierSourceAreDropped() throws {
        let page = try frame().prepared[0]
        let other = try frame("display-list-v2-math.json").prepared[0]
        let view = PageBitmapView(frame: NSRect(x: 0, y: 0, width: 400, height: 300))
        host(view)
        let white = CGColor(gray: 1, alpha: 1)
        view.show(nil, tiles: V2TileSource(page: page, pageToken: "a", pixelsPerPoint: 8, displayScale: 2, dark: false),
                  pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: white)
        view.show(nil, tiles: V2TileSource(page: page, pageToken: "a", pixelsPerPoint: 5, displayScale: 2, dark: false),
                  pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: white)
        let last = V2TileSource(page: other, pageToken: "b", pixelsPerPoint: 5, displayScale: 2, dark: false)
        view.show(nil, tiles: last, pageToken: "b", pageNumber: 1, frameRevision: 2, expectedDraws: 1, background: white)
        XCTAssertEqual(view.tileCount, 0)
        Self.settle { view.pendingTiles == 0 && view.tileCount > 0 }
        let (pw, ph) = last.pixelSize
        XCTAssertFalse(view.tileIndices.isEmpty)
        XCTAssertEqual(view.staleCount, 0)
        for index in view.tileIndices {
            let expected = try XCTUnwrap(GlyphRunRenderer.rasterizeTile(other, scale: 5, rect: V2TileGrid.rect(index, pageWidth: pw, pageHeight: ph)))
            XCTAssertEqual(V2Parity.rgba(try XCTUnwrap(view.tileImage(index))), V2Parity.rgba(expected), "tile \(index)")
        }
    }

    /// Nothing happens for a page view that is not in a window yet (SwiftUI
    /// updates a lazily created page before inserting it): no tile job, and
    /// no backdrop, which CoreAnimation would otherwise convert to the
    /// window's colour space on the main thread in the inserting commit. In
    /// a window, tiles and backdrop arrive already in its colour space.
    @MainActor
    func testTilesAndBackdropWaitForTheWindowAndArriveInItsColourSpace() throws {
        let page = try frame().prepared[0]
        let view = PageBitmapView(frame: NSRect(x: 0, y: 0, width: 400, height: 300))
        let backdrop = try XCTUnwrap(GlyphRunRenderer.rasterize(page, scale: V2TileGrid.backdropPixelsPerPoint))
        let source = V2TileSource(page: page, pageToken: "a", pixelsPerPoint: 8, displayScale: 2, dark: false)
        view.show(backdrop, tiles: source, pageToken: "a", pageNumber: 1, frameRevision: 1, expectedDraws: 1, background: CGColor(gray: 1, alpha: 1))
        XCTAssertEqual(view.tileJobs, 0)
        XCTAssertNil(view.layer?.contents)
        host(view, colorSpace: .displayP3)
        XCTAssertGreaterThan(view.tileJobs, 0)
        let p3 = try XCTUnwrap(NSColorSpace.displayP3.cgColorSpace)
        Self.settle { view.pendingTiles == 0 && view.layer?.contents != nil }
        let shown = try XCTUnwrap(view.layer?.contents as! CGImage?)
        XCTAssertEqual(shown.colorSpace, p3)
        XCTAssertEqual(shown.width, backdrop.width)
        let tile = try XCTUnwrap(view.tileIndices.first.flatMap(view.tileImage))
        XCTAssertEqual(tile.colorSpace, p3)
        XCTAssertEqual(tile.bitmapInfo.rawValue, GlyphRunRenderer.bitmapInfo)
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
