import AppKit
import XCTest
@testable import FlashTeXDisplayListV3
import FlashTeXPreviewV3
import HostedWindows
@testable import FlashTeXMac

/// `/Rotate` in the pane (spec §4.7 ROTATE): the rotation is outside the
/// content hash, yet a page turned by a new `/Rotate` alone is redrawn; and
/// a click or a highlight on a turned page lands where the turned image
/// shows the thing, not where it would be unturned.
@MainActor
final class EngineV3PageRotateTests: XCTestCase {
    /// A 100 × 60 bp page (y down) with `links`, turned `q` quarter turns.
    private func page(quarterTurns q: Int, links: [DL3Link] = []) -> DL3PreparedPage {
        var p = DL3Page(kind: .page, index: 0)
        p.box = [0, 0, 100, 60]
        p.hash = Array(repeating: 7, count: 32)
        p.flags = UInt32(q) << 4
        p.links = links
        return DL3PreparedPage(page: p, fonts: [:], images: [:])
    }

    private func link(_ data: String, left: Double, top: Double, right: Double, bottom: Double) -> DL3Link {
        DL3Link(rect: [left, top, right, bottom].map { Int32(($0 * EngineV3Links.spPerBP).rounded()) }, span: 0, kind: 4,
                file: [], data: Array(data.utf8))
    }

    /// A session showing `first` in a window-hosted scroll view.
    private func preview(_ first: DL3PreparedPage) throws -> (EngineV3Session, EngineV3PagesView, NSWindow) {
        let session = EngineV3Session(smoothFonts: false)
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 800, height: 1000))
        let pages = EngineV3PagesView(session: session)
        scroll.documentView = pages
        session.view = pages
        let window = HostedWindowSupport.window(contentRect: scroll.frame, styleMask: [.titled])
        window.isReleasedWhenClosed = false
        window.contentView = scroll
        session.handle(.page(first, compileID: 1, timing: .init(), image: nil))
        return (session, pages, window)
    }

    private func pageView(_ pages: EngineV3PagesView) -> EngineV3PageView? {
        pages.subviews.compactMap { $0 as? EngineV3PageView }.first
    }

    // MARK: finding 1: a rotation-only change

    /// /Rotate 0 → 180, same content and same size: the bitmap on screen was
    /// drawn unturned, so the page is drawn again under a new key.
    func testARotationOnlyChangeRedrawsThePage() throws {
        let (session, pages, window) = try preview(page(quarterTurns: 0))
        defer { window.close() }
        let view = try XCTUnwrap(pageView(pages))
        let keyBefore = try XCTUnwrap(view.hashKey)
        let generationBefore = view.generation
        session.handle(.page(page(quarterTurns: 2), compileID: 2, timing: .init(), image: nil))
        XCTAssertNotEqual(view.hashKey, keyBefore, "the turned page is not keyed as the unturned one")
        XCTAssertGreaterThan(view.generation, generationBefore, "the turned page is drawn again")
    }

    /// The same when the page is tiled: the tiles' key follows the rotation.
    func testARotationOnlyChangeReplacesTheTiles() throws {
        let (session, pages, window) = try preview(page(quarterTurns: 0))
        defer { window.close() }
        pages.update(revision: session.layoutRevision, zoom: 8)
        pages.relayout()
        try XCTSkipUnless(pages.tiled, "zoom 8 should tile (threshold \(EngineV3TileGrid.threshold))")
        let view = try XCTUnwrap(pageView(pages))
        let keyBefore = try XCTUnwrap(view.tiles.source?.key)
        session.handle(.page(page(quarterTurns: 2), compileID: 2, timing: .init(), image: nil))
        XCTAssertNotEqual(view.tiles.source?.key, keyBefore, "the tiles of the unturned page are not kept")
    }

    // MARK: finding 2: clicks and highlights on a turned page

    /// /Rotate 90: link A (10–30 × 5–15 bp, unturned) is shown at
    /// 45–55 × 10–30 bp (clockwise: x' = 60 − y, y' = x). A click at its
    /// shown centre (50, 20) opens A; B sits where that click would land if
    /// the point were not turned back.
    func testAClickOnAQuarterTurnedPageHitsTheLinkShownThere() throws {
        let a = link("https://a.example", left: 10, top: 5, right: 30, bottom: 15)
        let b = link("https://b.example", left: 45, top: 15, right: 55, bottom: 25)
        let (session, pages, window) = try preview(page(quarterTurns: 1, links: [a, b]))
        defer { window.close(); withExtendedLifetime(session) {} } // (the view holds its session weakly)
        let frame = try XCTUnwrap(pageView(pages)).frame
        XCTAssertEqual(frame.width / frame.height, 60.0 / 100.0, accuracy: 0.01, "shown turned")
        let scale = frame.width / 60
        let click = CGPoint(x: frame.minX + 50 * scale, y: frame.minY + 20 * scale)
        XCTAssertEqual(pages.link(at: click)?.dataString, "https://a.example")

        // Forward (highlights, the caret mark): A's centre is drawn at the click.
        let shownA = try XCTUnwrap(pages.viewPoint(page: 0, CGPoint(x: 20, y: 10)))
        XCTAssertEqual(shownA.x, click.x, accuracy: 0.01)
        XCTAssertEqual(shownA.y, click.y, accuracy: 0.01)
    }

    /// Every quarter turn: a point of the page drawn into the view and
    /// clicked there finds the link around it.
    func testClicksRoundTripOnEveryTurn() throws {
        let a = link("https://a.example", left: 10, top: 5, right: 30, bottom: 15)
        for q in 0 ..< 4 {
            let (session, pages, window) = try preview(page(quarterTurns: q, links: [a]))
            defer { window.close(); withExtendedLifetime(session) {} }
            let p = try XCTUnwrap(pages.viewPoint(page: 0, CGPoint(x: 12, y: 7)))
            XCTAssertEqual(pages.link(at: p)?.dataString, "https://a.example", "q \(q)")
            // On a turned page the point is not drawn at its unturned place.
            let frame = try XCTUnwrap(pageView(pages)).frame
            let scale = frame.width / (q % 2 == 0 ? 100 : 60)
            let unturned = CGPoint(x: frame.minX + 12 * scale, y: frame.minY + 7 * scale)
            if q != 0 { XCTAssertNotEqual(p, unturned, "q \(q)") }
        }
    }
}
