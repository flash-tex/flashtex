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
///
/// Draw counts are `rastersDrawnForSources`: an idle kept raster is volatile,
/// and the kernel may purge it between two jobs whenever free memory runs
/// low (a loaded runner: a ~500 MB raster at 16 px/pt), after which it is
/// drawn again. Those redraws are counted apart (`rastersRedrawnAfterPurge`).
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
        XCTAssertEqual(tiles.raster.rastersDrawnForSources, 1, "one raster for every job of the source")
        XCTAssertTrue(tiles.raster.holding)
        try assertExact(tiles, doc, page, scale: 16)

        // A new scale: a new raster (the old one is freed first).
        let src20 = source(doc, page, scale: 20)
        tiles.show(src20, visible: scrolled, compileID: nil)
        settle("the 20 px/pt tiles") { tiles.pending == 0 && tiles.missingVisible(scrolled) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawnForSources, 2)
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
        XCTAssertEqual(tiles.raster.rastersDrawnForSources, 2, "one raster per source: light, then dark")
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

    /// A translatable page (filled rules) with a dense band of rules across
    /// it, 3.1 px apart at 16 px/pt: a clip inside the band would grow past
    /// `clipGrowthMax` (to the band's width and height: ~100 MB of scratch
    /// per tile), so the band's tiles are cut from the source's ONE kept raster
    /// across the visible, prefetch and scroll jobs; the tiles above it are
    /// still drawn by translation. Every tile exact.
    /// A letter page of filled rules (translatable) with a band of rules
    /// 3.1 px apart at `scale` across its middle fifth (vertical ones across
    /// the band's rows), and one rule above it; `index` varies the content.
    func denseBandPage(scale: Double, index: UInt32 = 0) -> DL3PreparedPage {
        let (wPt, hPt) = (612.0, 792.0)
        let K = DL3.spPerBp
        var p = DL3Page(kind: .page, index: index)
        p.box = [0, 0, wPt, hPt]
        p.width = Int32((wPt * K).rounded()); p.height = Int32((hPt * K).rounded())
        func rule(_ l: Double, _ b: Double, _ r: Double, _ t: Double) { // bp, y up
            let x = Int32((l * K).rounded()), y = Int32(((hPt - t) * K).rounded())
            p.items.append(.rule(kind: .fill, x: x, y: y, w: Int32((r * K).rounded()) - x, h: Int32(((hPt - b) * K).rounded()) - y))
        }
        let (lo, hi) = (hPt * 0.4, hPt * 0.6), pitch = 3.1 / scale, width = 1.3 / scale
        for x in stride(from: 0.05, to: wPt - 0.2, by: pitch) { rule(x, lo, x + width, hi) }
        for y in stride(from: lo, to: hi, by: pitch) { rule(0, y, wPt, y + width) }
        rule(100.37 + Double(index), 650.2, 300.6, 700.81)
        return DL3PreparedPage(page: p, fonts: [:], images: [:])
    }

    func testADenseRuleBandIsCutFromTheKeptRaster() throws {
        let (wPt, hPt, scale) = (612.0, 792.0, 16.0)
        let page = denseBandPage(scale: scale)
        XCTAssertTrue(DL3Renderer.tilesByTranslation(page))
        let doc = try DL3Document(frames: [])
        let tiles = makeTiles(for: page, scale: scale)
        let src = source(doc, page, scale: scale)
        XCTAssertFalse(src.drawnWhole)
        // Image rows 4000–5692 px: above the band (rows 5069–7603) and into it.
        let view = CGRect(x: 600, y: 2000, width: 710, height: 846)
        tiles.show(src, visible: view, compileID: nil)
        settle("the first tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        let scrolled = view.offsetBy(dx: 0, dy: 500)
        tiles.update(visible: scrolled)
        settle("the scrolled tiles") { tiles.pending == 0 && tiles.missingVisible(scrolled) == 0 }
        XCTAssertGreaterThanOrEqual(tiles.jobsQueued, 3, "visible, prefetch and scroll jobs")
        XCTAssertEqual(tiles.raster.rastersDrawnForSources, 1, "one raster for every job of the source")
        let (w, h) = DL3Renderer.pixelSize(widthPt: wPt, heightPt: hPt, scale: scale)
        let routes = DL3Renderer.tileRoutes(page, scale: scale, rects: tiles.layers.keys.map { EngineV3TileGrid.rect($0, pageWidth: w, pageHeight: h) })
        XCTAssertTrue(routes.contains(.pageRaster)); XCTAssertTrue(routes.contains(.translate))
        try assertExact(tiles, doc, page, scale: scale)
        // Out of view: the keep set is empty and the fallback raster is freed.
        tiles.update(visible: .zero)
        XCTAssertFalse(tiles.raster.holding)
    }

    /// Translatable pages' fallback rasters share the kept-raster budget
    /// (2 slots, least recently cut first out) with pages drawn whole: one
    /// more dense-band page than the budget keeps the most recent ones; each
    /// is freed when its page leaves the keep set.
    func testDenseBandFallbackRastersShareTheKeptRasterBudget() throws {
        let scale = 8.0
        let doc = try DL3Document(frames: [])
        let view = CGRect(x: 0, y: 1100, width: 710, height: 846) // across the band's top (image rows 2534–3802 px)
        var all: [EngineV3PageTiles] = []
        for n in 0 ... EngineV3RasterHolder.budget {
            let page = denseBandPage(scale: scale, index: UInt32(n))
            let t = makeTiles(for: page, scale: scale)
            t.show(source(doc, page, scale: scale), visible: view, compileID: nil)
            settle("page \(n)'s tiles") { t.pending == 0 && t.missingVisible(view) == 0 }
            XCTAssertEqual(t.raster.rastersDrawnForSources, 1, "page \(n): its band from one raster")
            XCTAssertLessThanOrEqual(EngineV3RasterHolder.keptCount, EngineV3RasterHolder.budget)
            try assertExact(t, doc, page, scale: scale)
            all.append(t)
        }
        XCTAssertFalse(all.first!.raster.holding, "the least recently cut went first")
        XCTAssertTrue(all.last!.raster.holding, "the most recently cut is kept")
        XCTAssertEqual(all.filter { $0.raster.holding }.count, EngineV3RasterHolder.budget)
        for t in all { t.update(visible: .zero) }
        XCTAssertEqual(all.filter { $0.raster.holding }.count, 0, "each freed when its page left the keep set")
        XCTAssertEqual(EngineV3RasterHolder.keptCount, 0)
    }

    /// With the page raster over the fallback limit (lowered here; a tall
    /// page at ~19 px/pt needs several GiB) the band's tiles keep their
    /// uncapped clips: no raster is drawn or kept, and every tile is exact.
    func testDenseBandOverTheFallbackLimitDrawsNoRaster() throws {
        let scale = 8.0
        DL3Renderer.setFallbackRasterMaxBytesForTesting(16 << 20)
        defer { DL3Renderer.setFallbackRasterMaxBytesForTesting(nil) }
        let page = denseBandPage(scale: scale)
        XCTAssertFalse(DL3Renderer.pageRasterFits(page, scale: scale))
        let doc = try DL3Document(frames: [])
        let tiles = makeTiles(for: page, scale: scale)
        let view = CGRect(x: 0, y: 1100, width: 710, height: 846)
        tiles.show(source(doc, page, scale: scale), visible: view, compileID: nil)
        settle("the tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawn, 0)
        XCTAssertFalse(tiles.raster.holding)
        try assertExact(tiles, doc, page, scale: scale)
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
        XCTAssertEqual(tiles.raster.rastersDrawnForSources, 1, "the light raster serves both appearances")
    }

    /// A manual clock for the redraw throttle: `advance` moves time and fires
    /// the trailing redraws that fall due, in order, each settling its tiles
    /// (as the real run loop draws them before the next edit).
    final class ThrottleClock {
        var nowNs: UInt64 = 1_000_000_000_000
        var timers: [(at: UInt64, seq: Int, work: @MainActor () -> Void)] = []
        var seq = 0
    }

    func drive(_ tiles: EngineV3PageTiles, _ clock: ThrottleClock) {
        tiles.throttleClock = { clock.nowNs }
        tiles.throttleAfter = { delay, work in
            clock.seq += 1
            clock.timers.append((clock.nowNs + UInt64((delay * 1e9).rounded()), clock.seq, work))
        }
    }

    func advance(_ tiles: EngineV3PageTiles, _ clock: ThrottleClock, by seconds: TimeInterval, view: CGRect) {
        let end = clock.nowNs + UInt64((seconds * 1e9).rounded())
        while let next = clock.timers.filter({ $0.at <= end }).min(by: { ($0.at, $0.seq) < ($1.at, $1.seq) }) {
            clock.timers.removeAll { $0.seq == next.seq }
            clock.nowNs = next.at
            next.work()
            settle("the tiles of a trailing redraw") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        }
        clock.nowNs = end
    }

    /// The throttle for a page drawn whole (leading edge, trailing redraw):
    /// - an occasional edit (no redraw in the last `throttleWindow`) is drawn
    ///   at once: no added latency;
    /// - edits inside the window keep the stale tiles up and get ONE trailing
    ///   redraw with the newest content, after the pause or at the window's
    ///   end, whichever comes first;
    /// - sustained fast edits draw about twice a second, not once per edit.
    /// Time is the manual `ThrottleClock`: a loaded runner cannot push a
    /// burst out of the window or add a trailing redraw.
    func testEditsOnAPageDrawnWholeAreThrottled() throws {
        let doc = try load("tile-paths")
        let page0 = try XCTUnwrap(doc.orderedPages.first { !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) })
        let pages = doc.orderedPages.filter {
            !DL3Renderer.clipExact($0) && !DL3Renderer.tilesByTranslation($0) && $0.widthPt == page0.widthPt && $0.heightPt == page0.heightPt
        }
        XCTAssertGreaterThanOrEqual(pages.count, 3)
        XCTAssertGreaterThan(EngineV3PageTiles.throttleWindow, 0)
        let tiles = makeTiles(for: pages[0], scale: 12)
        let clock = ThrottleClock()
        drive(tiles, clock)
        let view = CGRect(x: 0, y: 0, width: 710, height: 846)
        tiles.show(source(doc, pages[0], scale: 12), visible: view, compileID: nil)
        settle("the first tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawnForSources, 1)
        advance(tiles, clock, by: EngineV3PageTiles.throttleWindow + 0.1, view: view)

        // An occasional edit: drawn at once (leading edge).
        tiles.show(source(doc, pages[1], scale: 12), visible: view, compileID: nil)
        XCTAssertNil(tiles.deferred, "no redraw in the window: drawn at once")
        XCTAssertEqual(tiles.source?.key, pages[1].page.hash)
        settle("the edit's tiles") { tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawnForSources, 2)
        try assertExact(tiles, doc, pages[1], scale: 12)

        // A burst inside the window: the stale tiles stay up; one trailing
        // redraw with the newest content.
        let burst = [pages[2], pages[0], pages[2]]
        for p in burst {
            tiles.show(source(doc, p, scale: 12), visible: view, compileID: nil)
            XCTAssertNotNil(tiles.deferred, "inside the window: held back")
            XCTAssertEqual(tiles.source?.key, pages[1].page.hash, "the stale tiles stay up")
            advance(tiles, clock, by: 0.03, view: view)
        }
        advance(tiles, clock, by: EngineV3PageTiles.throttleWindow, view: view)
        settle("the trailing redraw") { tiles.deferred == nil && tiles.source?.key == burst.last!.page.hash && tiles.pending == 0 && tiles.missingVisible(view) == 0 }
        XCTAssertEqual(tiles.raster.rastersDrawnForSources, 3, "one trailing redraw for the burst")
        try assertExact(tiles, doc, burst.last!, scale: 12)

        // Sustained edits every 100 ms for 2 s: about two redraws a second.
        let before = tiles.raster.rastersDrawnForSources
        var k = 0
        while k < 20 {
            tiles.show(source(doc, pages[k % pages.count], scale: 12), visible: view, compileID: nil)
            k += 1
            advance(tiles, clock, by: 0.1, view: view)
        }
        advance(tiles, clock, by: EngineV3PageTiles.throttleWindow, view: view)
        settle("the last redraw") { tiles.deferred == nil && tiles.pending == 0 }
        let redraws = tiles.raster.rastersDrawnForSources - before
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
        XCTAssertEqual(tiles.raster.rastersDrawnForSources, 1, "one source")
        XCTAssertGreaterThanOrEqual(tiles.raster.rastersRedrawnAfterPurge, 1, "drawn again after the purge")
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
