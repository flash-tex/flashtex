import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac

/// Whole-document evidence hooks (lane INFDESC-APP): `FLASHTEX_V3_CAPTURE_*`
/// parsing, the window-frame hook's choice of window, and the pane scrolling
/// to a page and reporting that page's bitmap as current. The pane test needs
/// a built `flashtex-host` (skipped otherwise); its window is a hosted window
/// off every display.
@MainActor
final class EngineV3PageCaptureTests: XCTestCase {
    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-capture-tests-\(getpid())")
    override func setUp() { setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1) }
    override func tearDown() { unsetenv("FLASHTEX_V3_CACHE") }

    func testCaptureConfigIsOffWithoutAnOutputDirectory() {
        XCTAssertNil(EngineV3PageCapture.Config.parse([:]))
        XCTAssertNil(EngineV3PageCapture.Config.parse(["FLASHTEX_V3_CAPTURE_PAGES": "1,2"]))
        XCTAssertNil(EngineV3PageCapture.Config.parse(["FLASHTEX_V3_CAPTURE_OUT": ""]))
    }

    func testCaptureConfigParsesPagesAndTimings() throws {
        let c = try XCTUnwrap(EngineV3PageCapture.Config.parse([
            "FLASHTEX_V3_CAPTURE_OUT": "/tmp/cap", "FLASHTEX_V3_CAPTURE_PAGES": " 2, 10,x,0,-3,575 ",
            "FLASHTEX_V3_CAPTURE_SETTLE": "20", "FLASHTEX_V3_CAPTURE_HOLD": "4", "FLASHTEX_V3_CAPTURE_EXIT": "1",
        ]))
        XCTAssertEqual(c.out.path, "/tmp/cap")
        XCTAssertEqual(c.pages, [2, 10, 575], "1-based; non-numbers and pages below 1 are skipped")
        XCTAssertEqual(c.settle, 20); XCTAssertEqual(c.hold, 4); XCTAssertTrue(c.exitWhenDone)
        let d = try XCTUnwrap(EngineV3PageCapture.Config.parse(["FLASHTEX_V3_CAPTURE_OUT": "/tmp/cap", "FLASHTEX_V3_CAPTURE_SETTLE": "-1"]))
        XCTAssertEqual(d.pages, []); XCTAssertEqual(d.settle, 5, "a negative settle keeps the default"); XCTAssertFalse(d.exitWhenDone)
    }

    /// Lane BEAMER-V3: tiles (`FLASHTEX_V3_CAPTURE_TILES`) and one edit typed
    /// once settled (`FLASHTEX_V3_CAPTURE_EDIT='needle|text'`, `\n` a newline).
    func testCaptureConfigParsesTilesAndAnEdit() throws {
        let c = try XCTUnwrap(EngineV3PageCapture.Config.parse([
            "FLASHTEX_V3_CAPTURE_OUT": "/tmp/cap", "FLASHTEX_V3_CAPTURE_TILES": "1",
            "FLASHTEX_V3_CAPTURE_EDIT": "of frame 20.|\\n    \\item<4-> Four|x",
        ]))
        XCTAssertTrue(c.tiles)
        XCTAssertEqual(c.edit, .init(needle: "of frame 20.", text: "\n    \\item<4-> Four|x"), "split at the first bar; \\n is a newline")
        let d = try XCTUnwrap(EngineV3PageCapture.Config.parse(["FLASHTEX_V3_CAPTURE_OUT": "/tmp/cap", "FLASHTEX_V3_CAPTURE_EDIT": "|x"]))
        XCTAssertNil(d.edit, "no needle")
        XCTAssertFalse(d.tiles)
        XCTAssertNil(EngineV3PageCapture.Config.parse(["FLASHTEX_V3_CAPTURE_OUT": "/tmp/cap", "FLASHTEX_V3_CAPTURE_EDIT": "no bar"])?.edit)
    }

    /// The pages an edit changed: a different hash, or a page that appeared or went.
    func testChangedPagesAfterAnEdit() {
        let before: [Int: [UInt8]] = [0: [1], 1: [2], 2: [3]]
        XCTAssertEqual(EngineV3PageCapture.changedPages(before: before, after: [0: [1], 1: [9], 2: [3], 3: [4]], countBefore: 3, countAfter: 4), [2, 4])
        XCTAssertEqual(EngineV3PageCapture.changedPages(before: before, after: [0: [1], 1: [2]], countBefore: 3, countAfter: 2), [3])
        XCTAssertEqual(EngineV3PageCapture.changedPages(before: before, after: before, countBefore: 3, countAfter: 3), [])
    }

    func testWindowFrameSpec() {
        XCTAssertEqual(AppDelegate.windowFrame("40, 40,1500,1000"), NSRect(x: 40, y: 40, width: 1500, height: 1000))
        XCTAssertNil(AppDelegate.windowFrame("40,40,1500"))
        XCTAssertNil(AppDelegate.windowFrame("40,40,0,1000"))
        XCTAssertNil(AppDelegate.windowFrame("a,b,c,d"))
    }

    /// The hook placed `NSApp.windows.first` when the main window did not
    /// exist yet, so an early helper window was resized instead and the main
    /// window kept its narrow restored frame (the preview collapsed).
    func testWindowFrameHookPlacesOnlyTheMainWindow() {
        let helper = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 500, height: 500), styleMask: [.titled])
        helper.isReleasedWhenClosed = false
        XCTAssertNil(AppDelegate.mainWindow(in: [helper]), "no main window yet: wait, place nothing")
        let main = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 900, height: 600), styleMask: [.titled, .resizable])
        main.isReleasedWhenClosed = false
        main.title = "FlashTeX"
        XCTAssertTrue(AppDelegate.mainWindow(in: [helper, main]) === main)
    }

    static let document: String = {
        var s = "\\documentclass{article}\n\\begin{document}\n"
        for i in 1 ... 8 { s += "\\section{Part \(i)}\nPage \(i).\n\\newpage\n" }
        return s + "\\end{document}\n"
    }()

    func testScrollToPageShowsThatPageCurrent() async throws {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: Self.document, named: "main.tex")
        model.engineV3Enabled = true
        let s = model.engineV3
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 640, height: 820), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        s.start(model: model)
        defer { s.stop(); window.contentView = nil }
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && s.pageCount == 8 }
        XCTAssertEqual(s.doneCount, 1)
        XCTAssertEqual(s.lastDone?["status"]?.string, "ok")
        XCTAssertEqual(s.incompletePages, [], "plain text: nothing drawn from the PDF")
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.relayout()
        XCTAssertFalse(pages.scrollToPage(8), "no ninth page")
        // A middle page (the last pages cannot reach the top: the scroll clamps).
        XCTAssertTrue(pages.scrollToPage(4))
        try await waitUntil("page 5 current") { pages.pageShowsCurrent(4) }
        XCTAssertEqual(s.visiblePage, 4, "page 5 is the first visible page")
        XCTAssertNotNil(pages.installedImage(4))
        XCTAssertFalse(pages.pageShowsCurrent(0), "page 1 is far above: not held")
        XCTAssertTrue(pages.scrollToPage(0))
        try await waitUntil("page 1 current") { pages.pageShowsCurrent(0) }
        XCTAssertEqual(s.visiblePage, 0)
    }

    func waitUntil(_ what: String, timeout: TimeInterval = 90, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }
}
