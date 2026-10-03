import AppKit
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
import HostedWindows
@testable import FlashTeXMac

/// A page the reader thread drew with the old font-smoothing setting (the
/// toggle flipped mid-raster) is redrawn, and still goes through the
/// ordinary arrival path: the page is undimmed and a resized page re-lays out.
@MainActor
final class EngineV3SmoothingRaceTests: XCTestCase {
    private func fixturePages() throws -> [DL3PreparedPage] {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("FlashTeXDisplayListV3Tests/Fixtures/beamer-overlays.dl3")
        return try DL3Document(frames: Array(try Data(contentsOf: url))).orderedPages
    }

    /// A session (smoothing off) showing page 0 in a window-hosted scroll view.
    private func preview() throws -> (EngineV3Session, EngineV3PagesView, NSWindow, DL3PreparedPage) {
        let session = EngineV3Session(smoothFonts: false)
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 800, height: 1000))
        let pages = EngineV3PagesView(session: session)
        scroll.documentView = pages
        session.view = pages
        let window = HostedWindowSupport.window(contentRect: scroll.frame, styleMask: [.titled])
        window.isReleasedWhenClosed = false
        window.contentView = scroll
        let page = try XCTUnwrap(fixturePages().first)
        session.handle(.page(page, compileID: 1, timing: .init(), image: nil))
        return (session, pages, window, page)
    }

    private func pageView(_ pages: EngineV3PagesView) -> EngineV3PageView? {
        pages.subviews.compactMap { $0 as? EngineV3PageView }.first
    }

    /// A raster drawn with smoothing on, for a session whose setting is off.
    private func staleSmoothingRaster(_ page: DL3PreparedPage, ppp: Double) throws -> EngineV3Raster {
        let image = try XCTUnwrap(DL3Renderer.rasterize(page, forms: [:], scale: 1, smoothFonts: true))
        return EngineV3Raster(image: image, ticket: EngineV3LayerTarget.ticket(), installNs: 0, committedNs: nil,
                              pixelsPerPoint: ppp, hash: page.page.hash, smoothFonts: true)
    }

    func testMismatchedSmoothingRasterStillUndimsThePage() throws {
        let (session, pages, window, page) = try preview()
        defer { window.close() }
        let view = try XCTUnwrap(pageView(pages))
        // The next compile starts: the page on screen is stale until it is sent again.
        session.handle(.started(.object(["keep": .bool(false)])))
        // VoiceOver: the page label says so (the page's value is its text).
        XCTAssertTrue(view.axStale, "dimmed as stale")
        XCTAssertEqual(view.accessibilityLabel()?.hasSuffix(", stale"), true)

        // The same page arrives, drawn with the wrong smoothing.
        let raster = try staleSmoothingRaster(page, ppp: pages.currentPixelsPerPoint)
        session.handle(.page(page, compileID: 2, timing: .init(), image: raster))
        XCTAssertFalse(view.axStale, "the page is current again: undimmed, not left stale")
        XCTAssertEqual(view.accessibilityLabel()?.hasSuffix(", stale"), false)
        XCTAssertNotEqual(view.hashKey, raster.hash, "the refused raster's key is not recorded")
        XCTAssertEqual(view.hashKey, EngineV3PagesView.contentKey(page.page.hash, .light, smoothFonts: false),
                       "redrawn with the current setting")
    }

    func testMismatchedSmoothingRasterOfAResizedPageReLaysOut() throws {
        let (session, pages, window, page) = try preview()
        defer { window.close() }
        let before = try XCTUnwrap(pageView(pages)).frame
        var wider = page
        wider.page.box[2] += wider.page.box[2] - wider.page.box[0] // twice as wide
        wider.page.hash[0] &+= 1
        let raster = try staleSmoothingRaster(wider, ppp: pages.currentPixelsPerPoint)
        session.handle(.page(wider, compileID: 2, timing: .init(), image: raster))
        let after = try XCTUnwrap(pageView(pages)).frame
        XCTAssertNotEqual(after.height / after.width, before.height / before.width, accuracy: 0.01,
                          "the page's new size is laid out")
        XCTAssertEqual(pageView(pages)?.hashKey, EngineV3PagesView.contentKey(wider.page.hash, .light, smoothFonts: false))
    }
}
