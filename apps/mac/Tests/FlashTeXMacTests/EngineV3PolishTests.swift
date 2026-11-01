import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac

/// Smaller app-parity rows under the engine-v3 preview (DESIGN §10): a
/// double-click fits the width (C4); hovering text names its source (C16);
/// the status bar's latency is v3's own (C22); a compile that wrote no page
/// says so in the Problems panel (B8); View ▸ Show TeX Log opens the last
/// `.log` (A21). Hosted pane with a built `flashtex-host` and TeX Live
/// (skipped otherwise).
@MainActor
final class EngineV3PolishTests: XCTestCase {
    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-polish-\(getpid())")
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

    static let doc = "\\documentclass{article}\n\\begin{document}\nAlpha beta gamma.\n\\end{document}\n"

    func pane(_ text: String = doc) async throws -> (ShellModel, EngineV3PagesView, NSWindow) {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: text, named: "main.tex")
        model.engineV3Enabled = true
        model.autoCompile = true
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 800), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        model.engineV3.start(model: model)
        try await EngineV3TestHost.awaitReady(model.engineV3)
        let s = model.engineV3
        try await waitUntil("the compile") { !s.statusNote.isEmpty && !s.compiling }
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: model.previewZoom)
        pages.relayout()
        return (model, pages, window)
    }

    func click(_ pages: EngineV3PagesView, at p: CGPoint, count: Int, in window: NSWindow) throws {
        let e = try XCTUnwrap(NSEvent.mouseEvent(with: .leftMouseDown, location: pages.convert(p, to: nil), modifierFlags: [], timestamp: 0,
                                                 windowNumber: window.windowNumber, context: nil, eventNumber: 0, clickCount: count, pressure: 1))
        pages.mouseDown(with: e)
    }

    func testDoubleClickFitsTheWidthAsOnV2() async throws {
        let (model, pages, window) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        model.previewZoom = 2
        let p = try XCTUnwrap(pages.viewPoint(page: 0, CGPoint(x: 300, y: 400)))
        try click(pages, at: p, count: 2, in: window)
        XCTAssertEqual(model.previewZoom, 1, "Fit Width, as ContentView's double-tap on the v2 pane")
    }

    func testHoveringTextNamesItsSource() async throws {
        let (model, pages, window) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        let text = Self.doc
        let at = try XCTUnwrap(s.place(path: "main.tex", byte: text.utf8.distance(from: text.startIndex, to: text.range(of: "gamma")!.lowerBound), in: text))
        let p = try XCTUnwrap(pages.viewPoint(page: 0, CGPoint(x: at.rect.midX, y: at.rect.midY)))
        pages.hover(at: p)
        XCTAssertEqual(pages.toolTip, "main.tex, line 3, column 12 (click to go there)", "gamma starts at byte 11 of the line: column 12")
        pages.hover(at: try XCTUnwrap(pages.viewPoint(page: 0, CGPoint(x: 5, y: 5))))
        XCTAssertNil(pages.toolTip, "a margin names nothing")
    }

    func testTheLatencyReadoutIsV3s() async throws {
        let (model, _, window) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        model.updateActiveText(Self.doc.replacingOccurrences(of: "gamma", with: "gamma delta"))
        try await waitUntil("a keystroke sample") { !s.latency.samples.isEmpty }
        model.chrome.refresh(from: model)
        XCTAssertEqual(model.chrome.lastLatencyMs, s.latency.samples.last?.ms)
        XCTAssertTrue(model.chrome.latencyHelp.hasPrefix("Last keystroke to its page on screen"), model.chrome.latencyHelp)
    }

    /// The latency record is bounded (a session types for hours): the
    /// oldest samples go, the vsync marks still land on the newest ones,
    /// and the status bar's median reads the last 200.
    func testTheLatencyRecordIsBounded() {
        let l = EngineV3Latency()
        let n = EngineV3Latency.keep + 300
        for i in 1...n {
            let t = UInt64(i) * 1_000_000_000
            l.sent(compile: i, keystrokeNs: t, editNs: t, path: "main.tex", at: t + 1_000)
            l.committed(compile: i, page: 0, at: t + 20_000_000)
            if i < n { l.vsync(targetNs: t + 30_000_000) }
        }
        XCTAssertLessThanOrEqual(l.samples.count, EngineV3Latency.keep + 256)
        XCTAssertGreaterThanOrEqual(l.samples.count, EngineV3Latency.keep)
        XCTAssertEqual(l.samples.last?.compile, n, "the newest sample is kept")
        XCTAssertNil(l.samples.last?.vsyncNs)
        l.vsync(targetNs: 99)
        XCTAssertEqual(l.samples.last?.vsyncNs, 99, "the vsync marks the newest sample after a trim")
        XCTAssertEqual(l.samples.dropLast().last?.vsyncNs, UInt64(n - 1) * 1_000_000_000 + 30_000_000, "and only it")
    }

    func testACompileWithNoPageSaysSoInProblems() async throws {
        let (model, _, window) = try await pane("\\documentclass{article}\n\\usepackage{nosuchpackage}\n\\begin{document}\nX.\n\\end{document}\n")
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        XCTAssertEqual(model.resultStatus, .failed, "no page: the Problems line says the previous preview is kept (\(s.statusNote))")
        model.updateActiveText(Self.doc)
        try await waitUntil("the fixed compile") { s.statusNote.hasPrefix("ok") && !s.compiling }
        XCTAssertNil(model.resultStatus)
    }

    /// Another project in the window starts with no "failed" of the last
    /// one's: the status is reset when the session takes the new project,
    /// before its first compile has answered.
    func testAnotherProjectDropsTheLastOnesFailedStatus() async throws {
        let bad = "\\documentclass{article}\n\\usepackage{nosuchpackage}\n\\begin{document}\nX.\n\\end{document}\n"
        let (model, _, window) = try await pane(bad)
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        XCTAssertEqual(model.engineV3ResultStatus, .failed)
        model.replaceProject(entryText: bad, named: "other.tex")
        s.compile(model: model, reason: "open")
        XCTAssertNil(model.engineV3ResultStatus, "reset with the project, before its compile answered")
        XCTAssertNil(model.resultStatus)
    }

    func testShowTeXLogFindsTheLastLog() async throws {
        let (model, _, window) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        let url = try XCTUnwrap(model.engineV3.texLogURL)
        XCTAssertEqual(url.lastPathComponent, "main.log")
        let log = try String(contentsOf: url, encoding: .utf8)
        XCTAssertTrue(log.contains("pdfTeX") || log.contains("LaTeX2e"), String(log.prefix(200)))
    }
}
