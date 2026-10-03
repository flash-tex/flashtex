import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXPreviewV3
import HostedWindows
@testable import FlashTeXMac

/// Bench (`FLASHTEX_CARET_BENCH=1`, skipped otherwise): the caret mark's
/// cost per caret move (`EngineV3PagesView.setCaret`) on a 560 KB document
/// shown at its middle page, with the caret on lines of that page, in four
/// cells: the document as compiled, and after typing near its start (so
/// the editor's text and the compiled one differ before every caret place),
/// each for a non-ASCII document (the prose has é, ï, —) and an ASCII one.
/// The typing goes through an editor (`NSTextView`, TextKit 1) in the same
/// window, as in the app, and the compile does not run (auto-compile off).
/// Thread CPU per call; prints p50/p95/max in microseconds, how many places
/// the edit window mapped, and the cost of comparing the whole texts (what
/// a caret move cost before the window) for reference.
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
        let prose = LargeDocumentEditorTests.proseDocument(bytes: 560_000)
        let ascii = prose.replacingOccurrences(of: "ï", with: "i").replacingOccurrences(of: "é", with: "e")
            .replacingOccurrences(of: "—", with: "-")
        XCTAssertTrue(ascii.utf8.allSatisfy { $0 < 0x80 })
        for (name, doc) in [("non-ASCII", prose), ("ASCII", ascii)] {
            try await cell(name: name, doc: doc)
        }
    }

    private func cell(name: String, doc: String) async throws {
        let model = ShellModel()
        model.replaceProject(entryText: doc, named: "main.tex")
        model.engineV3Enabled = true
        model.autoCompile = false
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 1000, height: 900), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 1000, height: 900))
        let hosting = NSHostingView(rootView: Host(model: model))
        hosting.frame = NSRect(x: 300, y: 0, width: 700, height: 900)
        let tv = NSTextView(usingTextLayoutManager: false) // TextKit 1, as the editor
        tv.frame = NSRect(x: 0, y: 0, width: 300, height: 900)
        tv.setAccessibilityLabel("LaTeX source")
        tv.string = doc
        container.addSubview(tv)
        container.addSubview(hosting)
        window.contentView = container
        let s = model.engineV3
        defer { s.stop(); window.contentView = nil }
        s.start(model: model)
        try await EngineV3TestHost.awaitReady(s)
        if !s.compiling, !s.statusNote.hasPrefix("ok") { s.compile(model: model, reason: "explicit") }
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

        func run(_ label: String, shift: Int) -> (marked: Int, summary: String) {
            let text = model.activeText as NSString
            var starts = [0]
            for i in 0..<text.length where text.character(at: i) == 0x0A { starts.append(i + 1) }
            var samples: [Double] = []
            var marked = 0
            let mapped0 = s.caretMapsByWindow
            for i in 0..<240 {
                let l = line - 1 + (i / 40) + shift // six lines, 40 caret places each
                let caret = starts[min(l, starts.count - 1)] + (i % 40)
                let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
                pages.setCaret(path: model.activePath, utf16: caret, stamp: 1000 + i) // a new key each time
                let c1 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
                if i >= 40 { samples.append(Double(c1 - c0) / 1_000) } // the first line warms up
                if pages.caretMark != nil { marked += 1 }
            }
            samples.sort()
            func q(_ p: Double) -> Double { samples[min(samples.count - 1, Int(Double(samples.count) * p))] }
            return (marked, String(format: "%@ %@: setCaret p50 %.0f µs, p95 %.0f µs, max %.0f µs (n=%d), %d/240 marked, %d by the window",
                                   name, label, q(0.5), q(0.95), samples.last ?? 0, samples.count, marked, s.caretMapsByWindow - mapped0))
        }
        let same = run("as compiled", shift: 0)
        // Typing near the start: a line above everything and words on the
        // first prose line, through the editor, as keystrokes make them.
        let first = (tv.string as NSString).range(of: "The quick").location
        for ch in "Typed. " { tv.insertText(String(ch), replacementRange: NSRange(location: first, length: 0)) }
        tv.insertText("A new line.\n", replacementRange: NSRange(location: first, length: 0))
        model.updateActiveText(tv.string)
        let differ = run("after typing", shift: 1)
        // Reference: the whole-text comparison a caret move did before the window.
        let compiled = try XCTUnwrap(model.compiledDocuments["main.tex"])
        let current = model.activeText
        var full: [Double] = []
        for _ in 0..<20 {
            let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            _ = EngineV3CaretPlace.map(caret: 300_000, current: current as NSString, compiled: compiled as NSString)
            full.append(Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - c0) / 1_000)
        }
        full.sort()
        print("caret-mark bench (\(s.pageCount) pages, page \(mid), held \(pages.heldPageIndexes)):\n  \(same.summary)\n  \(differ.summary)\n  "
              + String(format: "%@ whole-text comparison (the old path) p50 %.0f µs; load %@", name, full[10], IMEHarness.uptime()))
        XCTAssertGreaterThan(same.marked, 0)
        XCTAssertGreaterThan(differ.marked, 0, "the caret was marked on the page after typing")
    }
}
