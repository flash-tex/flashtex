import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXPreviewV3
import HostedWindows
@testable import FlashTeXMac

/// The caret on the page and the page labels in the engine-v3 preview
/// (DESIGN §10 app parity, gaps C12 and C17): as on the v2 pane, the
/// caret's line has a faint band, its glyph is tinted and a caret bar
/// stands at its column, following the caret; each page says "page N" at
/// its bottom right. The pane tests need a built `flashtex-host` and TeX
/// Live (skipped otherwise).
@MainActor
final class EngineV3CaretMarkTests: XCTestCase {
    struct Host: View {
        let model: ShellModel
        var body: some View {
            EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom, caretPath: model.activePath,
                               caretUTF16: model.caretUTF16, contentStamp: model.engineV3.contentStamp)
        }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-caret-tests-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

    func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    func testRowsGroupGlyphsByBaseline() {
        func g(_ x: CGFloat, _ y: CGFloat, col: UInt16) -> DL3GlyphRef {
            DL3GlyphRef(span: 1, col: col, origin: CGPoint(x: x, y: y), cell: CGRect(x: x, y: y - 7.5, width: 5, height: 10), ink: .zero)
        }
        let rows = EngineV3CaretMark.rows([g(0, 100, col: 0), g(5, 100.2, col: 1), g(0, 112, col: 2)])
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(rows[0].minX, 0); XCTAssertEqual(rows[0].maxX, 10)
        XCTAssertEqual(rows[0].height, 10.2, accuracy: 1e-9, "one row, 0.2 pt of baseline jitter")
    }

    static let doc = "\\documentclass{article}\n\\begin{document}\nAlpha beta gamma delta.\n\nSecond paragraph here.\n\\newpage\nPage two.\n\\end{document}\n"

    func pane() async throws -> (ShellModel, EngineV3PagesView, NSWindow) {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: Self.doc, named: "main.tex")
        model.engineV3Enabled = true
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 800), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        model.engineV3.start(model: model)
        try await EngineV3TestHost.awaitReady(model.engineV3)
        let s = model.engineV3
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount == 2 && s.pages[0] != nil }
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 1)
        pages.relayout()
        return (model, pages, window)
    }

    /// The caret in "gamma" marks gamma's line and puts the bar before the
    /// glyph at its column; moving the caret moves the mark; the mark's
    /// layer is drawn over the page at that place.
    func testTheCaretIsMarkedOnThePageAndFollows() async throws {
        let (model, pages, window) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        let gamma = (Self.doc as NSString).range(of: "gamma").location
        model.caretUTF16 = gamma
        pages.setCaret(path: model.activePath, utf16: gamma, stamp: s.contentStamp)
        let mark = try XCTUnwrap(pages.caretMark, "a mark for the caret")
        XCTAssertEqual(mark.page, 0)
        XCTAssertEqual(mark.band.count, 1, "one row: the line is one row on the page")
        // The bar stands at the start of gamma's "g": the glyph forward search puts at that column.
        let place = try XCTUnwrap(s.place(path: "main.tex", byte: Self.doc.utf8.distance(from: Self.doc.startIndex, to: Self.doc.range(of: "gamma")!.lowerBound), in: Self.doc))
        let bar = try XCTUnwrap(mark.bar)
        XCTAssertEqual(bar.minX, place.rect.minX, accuracy: 0.01)
        XCTAssertTrue(mark.band[0].contains(CGPoint(x: bar.minX, y: bar.midY)))
        XCTAssertFalse(pages.caretMarkLayer.isHidden)
        XCTAssertNotNil(pages.caretMarkLayer.bar.path)
        let barInView = pages.convertFromLayer(pages.caretMarkLayer.convert(try XCTUnwrap(pages.caretMarkLayer.bar.path?.boundingBox), to: pages.layer))
        let expected = try XCTUnwrap(pages.viewPoint(page: 0, CGPoint(x: bar.minX, y: bar.midY)))
        XCTAssertTrue(barInView.insetBy(dx: -1, dy: -1).contains(expected), "\(barInView) at \(expected)")
        // The second paragraph: another line, lower on the page.
        let second = (Self.doc as NSString).range(of: "paragraph").location
        pages.setCaret(path: model.activePath, utf16: second, stamp: s.contentStamp)
        let mark2 = try XCTUnwrap(pages.caretMark)
        XCTAssertGreaterThan(mark2.band[0].minY, mark.band[0].maxY)
        // A caret in the preamble maps to nothing: no mark.
        pages.setCaret(path: model.activePath, utf16: 3, stamp: s.contentStamp)
        XCTAssertNil(pages.caretMark)
        XCTAssertTrue(pages.caretMarkLayer.isHidden)
    }

    /// Each page says "page N" at its bottom right, in the label colour of
    /// its appearance (as the v2 pane's).
    func testEachPageIsLabelled() async throws {
        let (model, pages, window) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        let v = try XCTUnwrap(pages.heldPageView(1))
        XCTAssertEqual(v.label.string as? String, "page 2")
        // Bottom right of the page in the view's own (flipped) coordinates.
        let frame = v.labelFrameInView
        XCTAssertGreaterThan(frame.minY, v.bounds.height / 2, "at the bottom: \(frame) in \(v.bounds)")
        XCTAssertEqual(frame.maxX, v.bounds.width - 4, accuracy: 0.5)
        XCTAssertEqual(v.label.alignmentMode, .right)
        // Where the layer tree really puts it, in the view's coordinates.
        let shown = v.convertFromLayer(v.label.frame)
        XCTAssertEqual(shown.minY, frame.minY, accuracy: 0.5, "the label layer is at the bottom on screen: \(shown) in \(v.bounds)")
        pages.setAppearance(.dark)
        XCTAssertEqual(v.label.foregroundColor, NSColor(white: 0.7, alpha: 1).cgColor, "the dark page's label colour")
    }
}
