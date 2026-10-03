import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXPreviewV3
import HostedWindows
@testable import FlashTeXMac

/// Bench (`FLASHTEX_CARET_BENCH=1`, skipped otherwise): the caret mark's
/// cost per caret move (`EngineV3PagesView.setCaret`) on a 560 KB document
/// shown at its middle page, with the caret on lines of that page. Thread
/// CPU per call: the main thread's cost of one caret move, which the caret
/// mark adds to a keystroke. Prints p50/p95/max in microseconds.
@MainActor
final class EngineV3CaretBench: XCTestCase {
    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-caret-bench-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

    func testCaretMarkCostPerCaretMove() async throws {
        guard ProcessInfo.processInfo.environment["FLASHTEX_CARET_BENCH"] == "1" else { throw XCTSkip("bench: FLASHTEX_CARET_BENCH=1") }
        try EngineV3TestHost.require()
        let doc = LargeDocumentEditorTests.proseDocument(bytes: 560_000)
        let model = ShellModel()
        model.replaceProject(entryText: doc, named: "main.tex")
        model.engineV3Enabled = true
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 700, height: 900), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        s.start(model: model)
        try await EngineV3TestHost.awaitReady(s)
        let t0 = Date()
        while !(s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount > 50) {
            if Date().timeIntervalSince(t0) > 600 { XCTFail("timeout waiting for the compile (\(s.statusNote))"); return }
            try await Task.sleep(nanoseconds: 100_000_000)
        }
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 1)
        pages.relayout()
        let mid = s.pageCount / 2
        _ = pages.scrollToPage(mid)
        try await Task.sleep(nanoseconds: 500_000_000)
        // A line on the middle page: its first glyph's span.
        let ix = try XCTUnwrap(s.sourceIndex(page: mid))
        let g = try XCTUnwrap(ix.glyphs.first { $0.span != 0 && s.sourceMap.location(of: $0.span) != nil })
        let line = try XCTUnwrap(s.sourceMap.location(of: g.span)).line
        var starts = [0]
        for (i, u) in doc.utf16.enumerated() where u == 0x0A { starts.append(i + 1) }
        var samples: [Double] = []
        var marked = 0
        for i in 0..<240 {
            let l = line - 1 + (i / 40) // six lines, 40 caret places each
            let caret = starts[min(l, starts.count - 1)] + (i % 40)
            let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            pages.setCaret(path: model.activePath, utf16: caret, stamp: i) // a new key each time: nothing cached by the key
            let c1 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            if i >= 40 { samples.append(Double(c1 - c0) / 1_000) } // the first line warms up
            if pages.caretMark != nil { marked += 1 }
        }
        samples.sort()
        func q(_ p: Double) -> Double { samples[min(samples.count - 1, Int(Double(samples.count) * p))] }
        print(String(format: "caret-mark bench: %d pages, page %d held %@, %d/240 marked; setCaret thread CPU p50 %.0f µs, p95 %.0f µs, max %.0f µs (n=%d); load %@",
                     s.pageCount, mid, "\(pages.heldPageIndexes)", marked, q(0.5), q(0.95), samples.last ?? 0, samples.count,
                     IMEHarness.uptime()))
        XCTAssertGreaterThan(marked, 0, "the caret was marked on the page")
    }
}
