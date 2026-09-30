import AppKit
import SwiftUI
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
import HostedWindows
@testable import FlashTeXMac

/// The engine-v3 pane zoomed in (lane P3-V3-ZOOM-TILES): above the tile
/// threshold the visible pages show 512 px tiles that are pixel-exact
/// windows of the whole-page raster, only around the visible rect, drawn
/// off the main thread; a pinch commits about its fixed point; zooming out
/// drops the tiles again. The pane lives in a hosted window off every display.
/// Needs a built `flashtex-host` (skipped otherwise).
@MainActor
final class EngineV3ZoomTilesTests: XCTestCase {
    /// The pane as PreviewV3Pane hosts it: the zoom comes from the model.
    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-tiles-tests-\(getpid())")
    override func setUp() { setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1) }
    override func tearDown() { unsetenv("FLASHTEX_V3_CACHE") }

    func waitUntil(_ what: String, timeout: TimeInterval = 90, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    static let document = """
    \\documentclass{article}
    \\usepackage{amsmath}
    \\begin{document}
    \\section{Tiles}
    \(String(repeating: "The quick brown fox jumps over the lazy dog, and zooming in keeps every glyph where the page put it. ", count: 12))
    \\[ \\left( \\frac{\\dfrac{a}{b}}{\\dfrac{c}{d}} \\right) = \\sum_{k=1}^{n} \\int_0^1 x^k \\, dx \\]
    \(String(repeating: "Another paragraph of ordinary text, so that the page has glyphs across its whole width. ", count: 10))
    \\newpage
    Second page.
    \\end{document}

    """

    func testZoomedPaneShowsExactTilesAroundTheViewport() async throws {
        guard EngineV3.locateHost() != nil else { throw XCTSkip("no flashtex-host built (cargo build --release -p flashtex-engine --bin flashtex-host)") }
        let model = ShellModel()
        model.replaceProject(entryText: Self.document, named: "main.tex")
        let stored = UserDefaults.standard.object(forKey: EngineV3.enabledKey)
        model.engineV3Enabled = true
        let s = model.engineV3
        // A hosted window, parked off every display: nothing is activated or focused.
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 640, height: 820), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        model.engineV3.start(model: model)
        defer {
            model.engineV3.stop()
            window.contentView = nil
            if let stored { UserDefaults.standard.set(stored, forKey: EngineV3.enabledKey) } else { UserDefaults.standard.removeObject(forKey: EngineV3.enabledKey) }
        }
        try await waitUntil("the host") { s.phase == .ready || { if case .failed = s.phase { true } else { false } }() }
        guard s.phase == .ready else { throw XCTSkip("host did not start: \(s.phase)") }
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && s.pageCount == 2 && s.pages[0] != nil }
        window.layoutIfNeeded()
        model.previewZoom = 1
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 1)
        let scroll = try XCTUnwrap(pages.enclosingScrollView as? EngineV3ScrollContainer)
        pages.relayout()
        XCTAssertFalse(pages.tiled, "fit to width is below the tile threshold")

        // Zoom in: tiles over the visible part of page 1.
        model.previewZoom = 4
        pages.update(revision: s.layoutRevision, zoom: model.previewZoom)
        XCTAssertTrue(pages.tiled, "\(pages.currentPixelsPerPoint) px/pt")
        let ppp = pages.currentPixelsPerPoint
        let v = try XCTUnwrap(pages.heldPageViews[0])
        try await waitUntil("the visible tiles") { pages.missingVisibleTiles == 0 && v.tiles.count > 0 && v.tiles.pending == 0 }
        let prepared = try XCTUnwrap(s.pages[0])
        let tileScale = try XCTUnwrap(v.tiles.source?.pixelsPerPoint) // the screen's, or capped for a page drawn whole
        let whole = try XCTUnwrap(DL3Renderer.rasterize(prepared, forms: s.forms, scale: tileScale))
        let bytes = DL3Parity.rgba(whole)
        let size = DL3Renderer.pixelSize(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: tileScale)
        XCTAssertEqual(whole.width, size.width)
        var compared = 0
        for (index, _) in v.tiles.layers {
            let r = EngineV3TileGrid.rect(index, pageWidth: size.width, pageHeight: size.height)
            let tile = try XCTUnwrap(v.tiles.tileImage(index).flatMap { DL3Renderer.image(of: $0) })
            var window = [UInt8](); window.reserveCapacity(r.width * r.height * 4)
            for row in r.y ..< r.y + r.height { let o = (row * whole.width + r.x) * 4; window += bytes[o ..< o + r.width * 4] }
            XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(tile), window).pixels, 0, "tile \(index) at \(ppp) px/pt")
            compared += 1
        }
        XCTAssertGreaterThan(compared, 0)
        // Only the viewport (plus margins) is held, never the whole page.
        let all = (size.width + 511) / 512 * ((size.height + 511) / 512)
        XCTAssertLessThan(v.tiles.count, all, "\(v.tiles.count) of \(all) tiles held")
        XCTAssertLessThan(pages.retainedBytes, size.width * size.height * 4, "less than one whole page at \(ppp) px/pt")

        // A pinch keeps the page point under its anchor (away from the
        // document's edges, where the scroll position clamps).
        let clip = scroll.contentView
        clip.scroll(to: CGPoint(x: clip.bounds.minX, y: 1200))
        scroll.reflectScrolledClipView(clip)
        let anchor = CGPoint(x: clip.bounds.midX, y: clip.bounds.minY + 200)
        let before = try XCTUnwrap(pages.pagePoint(at: anchor))
        let offset = CGPoint(x: anchor.x - clip.bounds.minX, y: anchor.y - clip.bounds.minY)
        scroll.beginPinch(at: anchor, zoom: pages.zoom)
        scroll.updatePinch(by: -0.2)
        XCTAssertTrue(scroll.isPinching)
        XCTAssertNotEqual(clip.layer?.sublayerTransform.m11, 1, "the pinch is a transform")
        scroll.endPinch()
        XCTAssertEqual(clip.layer?.sublayerTransform.m11, 1)
        XCTAssertEqual(pages.zoom, 4 * 0.8, accuracy: 1e-9)
        XCTAssertEqual(model.previewZoom, 4 * 0.8, accuracy: 1e-9)
        let after = try XCTUnwrap(pages.pagePoint(at: CGPoint(x: clip.bounds.minX + offset.x, y: clip.bounds.minY + offset.y)))
        XCTAssertEqual(after.page, before.page)
        XCTAssertEqual(after.point.x, before.point.x, accuracy: 1)
        XCTAssertEqual(after.point.y, before.point.y, accuracy: 1)

        // An edit while zoomed in: the visible tiles show the new content.
        let oldHash = prepared.page.hash
        model.updateActiveText(model.activeText.replacingOccurrences(of: "\\section{Tiles}", with: "\\section{Tiles, edited}"))
        try await waitUntil("the edit compiled") { !s.compiling && s.statusNote.hasPrefix("ok") && s.pages[0]?.page.hash != oldHash }
        try await waitUntil("the new tiles") { pages.missingVisibleTiles == 0 && v.tiles.pending == 0 && v.tiles.source?.key.starts(with: s.pages[0]!.page.hash) == true }

        // Fit to width again: no tiles are held.
        model.previewZoom = 1
        pages.update(revision: s.layoutRevision, zoom: model.previewZoom)
        XCTAssertFalse(pages.tiled)
        XCTAssertEqual(pages.heldPageViews.values.reduce(0) { $0 + $1.tiles.count }, 0)
    }
}
