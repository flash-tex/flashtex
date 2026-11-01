import AppKit
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
import HostedWindows
@testable import FlashTeXMac

/// A page view placed while its hosted layer is detached from the layer
/// tree, as AppKit leaves it for a pass after the view is added (the page
/// label's flake, testEachPageIsLabelled, 6 of 25 hosted panes on
/// 2026-10-12). Its zoom tiles and "page N" label must be framed for the
/// flip the layer is shown with once attached, not the detached layer's
/// (#1821, #1823). The detached states are made on purpose, not raced for:
/// a flipped holder with no layer of its own, made layer-backed afterwards.
/// What a detached layer reads depends on its history, both measured on a
/// hosted runner (2026-10-12):
/// - fresh view: AppKit has set its `geometryFlipped`, so it reads flipped;
/// - view removed and added again: the flag is reset, so it reads unflipped.
/// Attached under a flipped view, it is shown flipped in both. No host needed.
@MainActor
final class EngineV3PageFlipTests: XCTestCase {
    final class FlippedHolder: NSView { override var isFlipped: Bool { true } }

    func settle(_ what: String, timeout: TimeInterval = 30, _ cond: () -> Bool) {
        let end = Date().addingTimeInterval(timeout)
        while !cond() {
            if Date() > end { XCTFail("timeout waiting for \(what)"); return }
            RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        }
    }

    struct Page {
        let doc: DL3Document, src: EngineV3TileSource, size: CGSize
    }

    func page() throws -> Page {
        let doc = try DL3Document(frames: Array(try Data(contentsOf: EngineV3PageTilesTests.fixtures.appendingPathComponent("tile-paths.dl3"))))
        let p = try XCTUnwrap(doc.orderedPages.first)
        let scale = 4.0 // above the tile threshold: tiles, 2 px per view point
        XCTAssertTrue(EngineV3TileGrid.tiles(scale))
        let src = EngineV3TileSource(prepared: p, forms: doc.forms, pdf: nil, key: p.page.hash, pixelsPerPoint: scale,
                                     displayScale: 2, screenPixelsPerPoint: scale)
        return Page(doc: doc, src: src, size: CGSize(width: p.widthPt * scale / 2, height: p.heightPt * scale / 2))
    }

    func window(_ size: CGSize) -> (NSWindow, NSView) {
        let w = HostedWindowSupport.window(contentRect: CGRect(origin: .zero, size: size), styleMask: [.titled])
        w.isReleasedWhenClosed = false
        let content = NSView(frame: CGRect(origin: .zero, size: size))
        content.wantsLayer = true
        w.contentView = content
        return (w, content)
    }

    /// Places the label and the tiles while `v`'s layer is detached (in a
    /// layerless `holder`), attaches it in `content`, and checks both where
    /// the tree shows them. Returns whether the detached layer read flipped.
    @discardableResult
    func placeDetachedThenCheck(_ v: EngineV3PageView, in holder: FlippedHolder, attachTo content: NSView, _ p: Page, _ what: String) throws -> Bool {
        let l = try XCTUnwrap(v.layer)
        XCTAssertNil(holder.layer, "\(what): the holder has no layer")
        XCTAssertNil(l.superlayer, "\(what): the page's layer is detached")
        let detachedFlipped = l.contentsAreFlipped()
        v.setLabel(number: 1, dark: false, contentsScale: 2)
        v.tiles.show(p.src, visible: v.bounds, compileID: nil)
        settle("\(what): the tiles") { v.tiles.pending == 0 && v.tiles.count > 0 }
        XCTAssertNil(l.superlayer, "\(what): still detached when the tiles were installed")

        content.addSubview(holder)
        settle("\(what): the page's layer in the layer tree") { l.superlayer != nil }
        let flipped = l.contentsAreFlipped()
        XCTAssertTrue(flipped, "\(what): under a flipped layer-backed view the page's layer is shown flipped")
        XCTAssertEqual(v.layerShownFlipped, flipped, "\(what)")

        // Where the tree shows them, from the page's top, in view points
        // (`convertFromLayer` does not apply the tree's flip).
        let container = v.tiles.container
        XCTAssertEqual(container.contentsAreFlipped(), flipped, "\(what)")
        let (pw, ph) = p.src.pixelSize
        let ds = CGFloat(p.src.pixelsPerViewPoint)
        for (index, tile) in v.tiles.layers {
            let r = EngineV3TileGrid.rect(index, pageWidth: pw, pageHeight: ph)
            let f = tile.frame
            let shownTop = flipped ? f.minY : container.bounds.height - f.maxY
            XCTAssertEqual(shownTop, CGFloat(r.y) / ds, accuracy: 0.5, "\(what): tile \(index) at \(f) in \(container.bounds), detached read flipped \(detachedFlipped)")
            XCTAssertEqual(f.minX, CGFloat(r.x) / ds, accuracy: 0.5, "\(what): tile \(index)")
        }
        let label = v.label.frame
        let labelShownY = flipped ? label.minY : v.bounds.height - label.maxY
        XCTAssertEqual(labelShownY, v.labelFrameInView.minY, accuracy: 0.5, "\(what): the label at the bottom on screen: \(label) in \(v.bounds), detached read flipped \(detachedFlipped)")
        return detachedFlipped
    }

    /// A fresh page view: its detached layer reads flipped, from a geometry
    /// flag AppKit resets when it attaches (#1821 read that flag, and put
    /// the label at the top).
    func testAFreshPageViewPlacedWhileDetachedIsRightOnceAttached() throws {
        let p = try page()
        let holder = FlippedHolder(frame: CGRect(origin: .zero, size: p.size))
        let v = EngineV3PageView(frame: holder.bounds)
        holder.addSubview(v)
        let (w, content) = window(p.size)
        defer { v.tiles.removeAll(); w.contentView = nil }
        XCTAssertTrue(try placeDetachedThenCheck(v, in: holder, attachTo: content, p, "fresh"), "a fresh view's detached layer reads flipped")
    }

    /// A page view removed and added again: its detached layer reads
    /// unflipped (the flake's state; the old tile code framed for it).
    func testAPageViewAddedAgainPlacedWhileDetachedIsRightOnceAttached() throws {
        let p = try page()
        let (w, content) = window(p.size)
        defer { w.contentView = nil }
        let first = FlippedHolder(frame: CGRect(origin: .zero, size: p.size))
        first.wantsLayer = true
        content.addSubview(first)
        let v = EngineV3PageView(frame: first.bounds)
        defer { v.tiles.removeAll() }
        first.addSubview(v)
        let l = try XCTUnwrap(v.layer)
        settle("attached the first time") { l.superlayer != nil }
        v.removeFromSuperview()
        let holder = FlippedHolder(frame: CGRect(origin: .zero, size: p.size))
        holder.addSubview(v)
        XCTAssertFalse(try placeDetachedThenCheck(v, in: holder, attachTo: content, p, "added again"), "a view added again: its detached layer reads unflipped")
    }
}
