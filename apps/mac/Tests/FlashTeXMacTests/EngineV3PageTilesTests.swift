import AppKit
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
@testable import FlashTeXMac

/// The pane's tile manager (`EngineV3PageTiles`) on the checked-in display
/// lists, no host needed. It covers:
/// - the page kinds: clip-exact (table rules) and drawn whole (paths, one
///   kept raster per source);
/// - exactness of every installed tile;
/// - queued jobs skipped when their tiles leave the keep set, and requested
///   again when they come back before the skip is reported (review of #1287);
/// - jobs cancelled by teardown and `removeAll`;
/// - PDF fallbacks, the edit throttle, purged rasters, the kept-raster budget
///   and retries after a raster could not be drawn.
@MainActor
final class EngineV3PageTilesTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("FlashTeXDisplayListV3Tests/Fixtures")

    func load(_ name: String) throws -> DL3Document {
        try DL3Document(frames: Array(try Data(contentsOf: Self.fixtures.appendingPathComponent("\(name).dl3"))))
    }

    func source(_ doc: DL3Document, _ page: DL3PreparedPage, scale: Double) -> EngineV3TileSource {
        EngineV3TileSource(prepared: page, forms: doc.forms, pdf: nil, key: page.page.hash, pixelsPerPoint: scale,
                           displayScale: 2, screenPixelsPerPoint: scale)
    }

    /// Runs the main run loop until `cond` holds (tile jobs report back to main).
    func settle(_ what: String, timeout: TimeInterval = 30, _ cond: () -> Bool) {
        let end = Date().addingTimeInterval(timeout)
        while !cond() {
            if Date() > end { XCTFail("timeout waiting for \(what)"); return }
            RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        }
    }

    /// Blocks the tile queue until the returned closure is called.
    func holdTileQueue() -> () -> Void {
        let gate = DispatchSemaphore(value: 0)
        EngineV3TileGrid.queue.async { gate.wait() }
        return { gate.signal(); EngineV3TileGrid.queue.sync {} }
    }

    /// Every installed tile equals the same window of the whole-page raster.
    func assertExact(_ tiles: EngineV3PageTiles, _ doc: DL3Document, _ page: DL3PreparedPage, scale: Double, file: StaticString = #filePath, line: UInt = #line) throws {
        let whole = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: scale))
        let pixels = DL3Parity.rgba(whole)
        XCTAssertGreaterThan(tiles.layers.count, 0, file: file, line: line)
        for index in tiles.layers.keys {
            let r = EngineV3TileGrid.rect(index, pageWidth: whole.width, pageHeight: whole.height)
            let img = try XCTUnwrap(tiles.tileImage(index).flatMap { DL3Renderer.image(of: $0) })
            var window = [UInt8](); window.reserveCapacity(r.width * r.height * 4)
            for row in r.y ..< r.y + r.height { let o = (row * whole.width + r.x) * 4; window += pixels[o ..< o + r.width * 4] }
            XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(img), window).pixels, 0, "tile \(index) at \(scale) px/pt", file: file, line: line)
        }
    }

    func makeTiles(for page: DL3PreparedPage, scale: Double) -> EngineV3PageTiles {
        let tiles = EngineV3PageTiles()
        tiles.container.frame = CGRect(x: 0, y: 0, width: page.widthPt * scale / 2, height: page.heightPt * scale / 2)
        return tiles
    }

    /// A page with paths is cut from ONE kept full-scale raster per source,
    /// across the visible and prefetch jobs and later scroll steps; a new
    /// scale draws one more; leaving the keep set frees it.
    func testAPathPageKeepsOneRasterPerSource() throws {
        let doc = try load("tile-paths")
        let page = try XCTUnwrap(doc.orderedPages.first { !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) })
        let tiles = makeTiles(for: page, scale: 16)
        let src = source(doc, page, scale: 16)
        XCTAssertTrue(src.drawnWhole)
        let view = CGRect(x: 200, y: 150, width: 710, height: 846)
        tiles.show(src, visible: view, compileID: nil)
        settle("the first tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        let scrolled = view.offsetBy(dx: 0, dy: 400)
        tiles.update(visible: scrolled)
        settle("the scrolled tiles") { tiles.pending == 0 && tiles.missingVisible(scrolled) == 0 }
        XCTAssertGreaterThanOrEqual(tiles.jobsQueued, 3, "visible, prefetch and scroll jobs")
        XCTAssertEqual(tiles.raster.rastersDrawn, 1, "one raster for every job of the source")
        XCTAssertTrue(tiles.raster.holding)
        try assertExact(tiles, doc, page, scale: 16)

        // A new scale: a new raster (the old one is freed first).
        let src20 = source(doc, page, scale: 20)
        tiles.show(src20, visible: scrolled, compileID: nil)
        settle("the 20 px/pt tiles") { tiles.pending == 0 && tiles.missingVisible(scrolled) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawn, 2)
        try assertExact(tiles, doc, page, scale: 20)

        // Out of view: the keep set is empty and the raster is freed.
        tiles.update(visible: .zero)
        XCTAssertFalse(tiles.raster.holding)
    }

    /// The pane's tiles in dark appearance, on a path page (kept raster):
    /// exact windows of the dark whole page; toggling the appearance makes a
    /// new source (its tiles replace the old ones).
    func testDarkTilesOnAPathPage() throws {
        let doc = try load("tile-paths")
        let page = try XCTUnwrap(doc.orderedPages.first { !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) })
        let tiles = makeTiles(for: page, scale: 16)
        let view = CGRect(x: 100, y: 100, width: 710, height: 846)
        tiles.show(source(doc, page, scale: 16), visible: view, compileID: nil)
        settle("light tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        tiles.show(EngineV3TileSource(prepared: page, forms: doc.forms, pdf: nil, key: page.page.hash + [1], pixelsPerPoint: 16,
                                      displayScale: 2, screenPixelsPerPoint: 16, appearance: .dark), visible: view, compileID: nil)
        settle("dark tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        let whole = try XCTUnwrap(DL3Renderer.rasterizeToSurface(page, forms: doc.forms, scale: 16, appearance: .dark).flatMap { DL3Renderer.image(of: $0) })
        let pixels = DL3Parity.rgba(whole)
        for index in tiles.layers.keys {
            let r = EngineV3TileGrid.rect(index, pageWidth: whole.width, pageHeight: whole.height)
            let img = try XCTUnwrap(tiles.tileImage(index).flatMap { DL3Renderer.image(of: $0) })
            var window = [UInt8](); window.reserveCapacity(r.width * r.height * 4)
            for row in r.y ..< r.y + r.height { let o = (row * whole.width + r.x) * 4; window += pixels[o ..< o + r.width * 4] }
            XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(img), window).pixels, 0, "dark tile \(index)")
        }
        XCTAssertEqual(tiles.raster.rastersDrawn, 2, "one raster per source: light, then dark")
    }

    /// A page with table (stroked) rules: clipped rasters, nothing kept, exact.
    func testAClipExactPageKeepsNoRaster() throws {
        let doc = try load("tile-text")
        let page = try XCTUnwrap(doc.orderedPages.first)
        XCTAssertTrue(DL3Renderer.clipExact(page)); XCTAssertFalse(DL3Renderer.tilesByTranslation(page))
        let tiles = makeTiles(for: page, scale: 16)
        let src = source(doc, page, scale: 16)
        XCTAssertFalse(src.drawnWhole)
        let view = CGRect(x: 1000, y: 1200, width: 710, height: 846)
        tiles.show(src, visible: view, compileID: nil)
        settle("the tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawn, 0)
        try assertExact(tiles, doc, page, scale: 16)
    }

    /// Tiles whose job ran while they were out of the keep set are skipped
    /// undrawn; when the viewport comes back before that is reported on the
    /// main thread, they are requested again (no hole).
    func testSkippedTilesAreRequestedAgainWhenWantedAgain() throws {
        let doc = try load("tile-text")
        let page = try XCTUnwrap(doc.orderedPages.first)
        let tiles = makeTiles(for: page, scale: 16)
        let a = CGRect(x: 0, y: 0, width: 710, height: 846), b = CGRect(x: 0, y: 3000, width: 710, height: 846)
        let skippedBefore = EngineV3TileGrid.skippedTiles
        let release = holdTileQueue()
        tiles.show(source(doc, page, scale: 16), visible: a, compileID: nil) // A's jobs queue behind the gate
        tiles.update(visible: b)                                              // A leaves the keep set
        release()                                                             // A's jobs run now: skipped
        tiles.update(visible: a)                                              // back, before the skip is reported
        settle("A's tiles again") { tiles.pending == 0 && tiles.missingVisible(a) == 0 }
        XCTAssertGreaterThan(EngineV3TileGrid.skippedTiles, skippedBefore, "A's first jobs were skipped undrawn")
        XCTAssertEqual(tiles.missingVisible(a), 0)
        try assertExact(tiles, doc, page, scale: 16)
    }

    /// A torn-down page's queued jobs draw nothing, and neither do those of a
    /// page whose tiles were all removed.
    func testTeardownAndRemoveAllCancelQueuedJobs() throws {
        let doc = try load("tile-paths")
        let page = try XCTUnwrap(doc.orderedPages.first { !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) })
        let view = CGRect(x: 0, y: 0, width: 710, height: 846)

        var release = holdTileQueue()
        var skippedBefore = EngineV3TileGrid.skippedTiles
        var tiles: EngineV3PageTiles? = makeTiles(for: page, scale: 16)
        tiles!.show(source(doc, page, scale: 16), visible: view, compileID: nil)
        let holder = tiles!.raster
        let queued = tiles!.pending
        XCTAssertGreaterThan(queued, 0)
        tiles = nil // teardown (deinit bumps the generation)
        release()
        settle("the skips reported") { EngineV3TileGrid.skippedTiles >= skippedBefore + queued }
        XCTAssertEqual(holder.rastersDrawn, 0, "no raster drawn for a torn-down page")

        release = holdTileQueue()
        skippedBefore = EngineV3TileGrid.skippedTiles
        let kept = makeTiles(for: page, scale: 16)
        kept.show(source(doc, page, scale: 16), visible: view, compileID: nil)
        let queued2 = kept.pending
        kept.removeAll()
        release()
        settle("the skips reported") { EngineV3TileGrid.skippedTiles >= skippedBefore + queued2 }
        XCTAssertEqual(kept.raster.rastersDrawn, 0)
        XCTAssertEqual(kept.count, 0)
    }

    /// A PDF-fallback page: its grid is the PDF media box's, its tiles come
    /// from the kept light raster (dark through the per-tile pass), exact;
    /// a dark toggle does not draw the raster again.
    func testAPDFFallbackSourceKeepsItsLightRasterAcrossAppearances() throws {
        let doc = try load("tile-paths")
        let page = try XCTUnwrap(doc.orderedPages.first)
        let pdf = try XCTUnwrap(CGPDFDocument(Self.fixtures.appendingPathComponent("tile-paths.pdf") as CFURL)?.page(at: Int(page.page.index) + 1))
        func pdfSource(_ a: DL3Appearance) -> EngineV3TileSource {
            var s = EngineV3TileSource(prepared: page, forms: doc.forms, pdf: pdf, key: [0xFF] + page.page.hash + [a == .dark ? 1 : 0],
                                       pixelsPerPoint: 12, displayScale: 2, screenPixelsPerPoint: 12, appearance: a)
            s.pdfIdentity = [0xFF] + page.page.hash
            return s
        }
        let light = pdfSource(.light)
        XCTAssertTrue(light.drawnWhole)
        let grid = DL3PageRaster.gridSize(pdfPage: pdf, scale: 12)
        XCTAssertEqual([light.pixelSize.width, light.pixelSize.height], [grid.width, grid.height])
        let tiles = makeTiles(for: page, scale: 12)
        let view = CGRect(x: 0, y: 0, width: 710, height: 846)
        for (a, src) in [(DL3Appearance.light, light), (.dark, pdfSource(.dark))] {
            tiles.show(src, visible: view, compileID: nil)
            settle("\(a) PDF tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 && tiles.source?.appearance == a }
            let whole = try XCTUnwrap(DL3Renderer.rasterizeToSurface(pdfPage: pdf, scale: 12, appearance: a).flatMap { DL3Renderer.image(of: $0) })
            let pixels = DL3Parity.rgba(whole)
            XCTAssertGreaterThan(tiles.layers.count, 0)
            for index in tiles.layers.keys {
                let r = EngineV3TileGrid.rect(index, pageWidth: whole.width, pageHeight: whole.height)
                let img = try XCTUnwrap(tiles.tileImage(index).flatMap { DL3Renderer.image(of: $0) })
                var window = [UInt8](); window.reserveCapacity(r.width * r.height * 4)
                for row in r.y ..< r.y + r.height { let o = (row * whole.width + r.x) * 4; window += pixels[o ..< o + r.width * 4] }
                XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(img), window).pixels, 0, "\(a) PDF tile \(index)")
            }
        }
        XCTAssertEqual(tiles.raster.rastersDrawn, 1, "the light raster serves both appearances")
    }

    /// The throttle for a page drawn whole (leading edge, trailing redraw):
    /// - an occasional edit (no redraw in the last `throttleWindow`) is drawn
    ///   at once: no added latency;
    /// - edits inside the window keep the stale tiles up and get ONE trailing
    ///   redraw with the newest content, after the pause or at the window's
    ///   end, whichever comes first;
    /// - sustained fast edits draw about twice a second, not once per edit.
    func testEditsOnAPageDrawnWholeAreThrottled() throws {
        let doc = try load("tile-paths")
        let page0 = try XCTUnwrap(doc.orderedPages.first { !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) })
        let pages = doc.orderedPages.filter {
            !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) && $0.widthPt == page0.widthPt && $0.heightPt == page0.heightPt
        }
        XCTAssertGreaterThanOrEqual(pages.count, 3)
        let tiles = makeTiles(for: pages[0], scale: 12)
        let view = CGRect(x: 0, y: 0, width: 710, height: 846)
        tiles.show(source(doc, pages[0], scale: 12), visible: view, compileID: nil)
        settle("the first tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawn, 1)
        RunLoop.main.run(until: Date().addingTimeInterval(EngineV3PageTiles.throttleWindow + 0.1))

        // An occasional edit: drawn at once (leading edge).
        tiles.show(source(doc, pages[1], scale: 12), visible: view, compileID: nil)
        XCTAssertNil(tiles.deferred, "no redraw in the window: drawn at once")
        XCTAssertEqual(tiles.source?.key, pages[1].page.hash)
        settle("the edit's tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawn, 2)
        try assertExact(tiles, doc, pages[1], scale: 12)

        // A burst inside the window: the stale tiles stay up; one trailing
        // redraw with the newest content.
        let burst = [pages[2], pages[0], pages[2]]
        for p in burst {
            tiles.show(source(doc, p, scale: 12), visible: view, compileID: nil)
            XCTAssertNotNil(tiles.deferred, "inside the window: held back")
            XCTAssertEqual(tiles.source?.key, pages[1].page.hash, "the stale tiles stay up")
            RunLoop.main.run(until: Date().addingTimeInterval(0.03))
        }
        settle("the trailing redraw") { tiles.deferred == nil && tiles.source?.key == burst.last!.page.hash && tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawn, 3, "one trailing redraw for the burst")
        try assertExact(tiles, doc, burst.last!, scale: 12)

        // Sustained edits every 100 ms for 2 s: about two redraws a second.
        let before = tiles.raster.rastersDrawn
        let start = Date()
        var k = 0
        while Date().timeIntervalSince(start) < 2 {
            tiles.show(source(doc, pages[k % pages.count], scale: 12), visible: view, compileID: nil)
            k += 1
            RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        }
        settle("the last redraw") { tiles.deferred == nil && tiles.pending == 0 }
        let redraws = tiles.raster.rastersDrawn - before
        XCTAssertLessThanOrEqual(redraws, 7, "\(redraws) redraws for \(k) edits in 2 s")
        XCTAssertGreaterThanOrEqual(redraws, 3, "fast typing still redraws (the window expires)")
    }

    /// A raster the kernel purged (volatile while idle) is drawn again, exact.
    func testAPurgedRasterIsDrawnAgain() throws {
        let doc = try load("tile-paths")
        let page = try XCTUnwrap(doc.orderedPages.first { !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) })
        let tiles = makeTiles(for: page, scale: 12)
        let view = CGRect(x: 0, y: 0, width: 710, height: 846)
        tiles.show(source(doc, page, scale: 12), visible: view, compileID: nil)
        settle("the first tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        tiles.raster.purgeForTesting()
        let below = view.offsetBy(dx: 0, dy: 1200)
        tiles.update(visible: below)
        settle("tiles after the purge") { tiles.pending == 0 && tiles.missingVisible(below) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawn, 2, "drawn again after the purge")
        try assertExact(tiles, doc, page, scale: 12)
    }

    /// At most `budget` rasters are kept over all pages (least recently cut out).
    func testKeptRastersStayWithinTheBudget() throws {
        let doc = try load("tile-paths")
        let pages = Array(doc.orderedPages.filter { !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) }.prefix(4))
        XCTAssertGreaterThan(pages.count, EngineV3RasterHolder.budget)
        let view = CGRect(x: 0, y: 0, width: 710, height: 846)
        var all: [EngineV3PageTiles] = []
        for p in pages {
            let t = makeTiles(for: p, scale: 12)
            t.show(source(doc, p, scale: 12), visible: view, compileID: nil)
            settle("tiles") { t.pending == 0 && t.missingVisible(view) == 0 }
            all.append(t)
            XCTAssertLessThanOrEqual(EngineV3RasterHolder.keptCount, EngineV3RasterHolder.budget)
        }
        XCTAssertEqual(all.filter { $0.raster.holding }.count, EngineV3RasterHolder.budget)
        XCTAssertTrue(all.last!.raster.holding, "the most recently cut is kept")
        XCTAssertFalse(all.first!.raster.holding, "the least recently cut went first")
    }

    /// A raster that cannot be drawn (memory not available) leaves no hole:
    /// its tiles are asked for again and land.
    func testTilesThatCouldNotBeDrawnAreRetried() throws {
        let doc = try load("tile-paths")
        let page = try XCTUnwrap(doc.orderedPages.first { !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) })
        let failedBefore = EngineV3TileGrid.failedTiles
        EngineV3RasterHolder.failNextDrawsForTesting(2)
        defer { EngineV3RasterHolder.failNextDrawsForTesting(0) }
        let tiles = makeTiles(for: page, scale: 12)
        let view = CGRect(x: 0, y: 0, width: 710, height: 846)
        tiles.show(source(doc, page, scale: 12), visible: view, compileID: nil)
        settle("tiles after the retries", timeout: 30) { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        XCTAssertGreaterThan(EngineV3TileGrid.failedTiles, failedBefore, "the failure was seen and retried")
        try assertExact(tiles, doc, page, scale: 12)
    }
}
