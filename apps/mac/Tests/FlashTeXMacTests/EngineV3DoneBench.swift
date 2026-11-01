import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac

/// The main thread's time in a DONE on a long document whose every page is
/// drawn from the compile's PDF (a pgf shading on each: INCOMPLETE), so the
/// DONE loads the PDF fallback for all of them. Evidence only: it runs when
/// `FLASHTEX_DONE_BENCH_PAGES` is set (the page count, e.g. 592: Infinite
/// Descent's), with a built `flashtex-host` and TeX Live (tikz, lipsum).
@MainActor
final class EngineV3DoneBench: XCTestCase {
    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    private var env = EnvironmentOverride()
    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-done-bench-\(getpid())").path)
    }
    override func tearDown() { env.restore() }

    static func document(pages: Int, word: String) -> String {
        """
        \\documentclass{article}
        \\usepackage{tikz}
        \\usepackage{lipsum}
        \\begin{document}
        \\count255=0
        \\loop
        \\section{\(word) \\the\\count255}
        \\lipsum[1-3]
        \\begin{center}\\tikz\\shade[ball color=blue!60] (0,0) circle (0.6cm);\\end{center}
        \\clearpage
        \\advance\\count255 by 1
        \\ifnum\\count255<\(pages) \\repeat
        \\end{document}

        """
    }

    func testDoneMainThreadTimeOnALongDocument() async throws {
        guard let n = ProcessInfo.processInfo.environment["FLASHTEX_DONE_BENCH_PAGES"].flatMap(Int.init) else {
            throw XCTSkip("evidence only: set FLASHTEX_DONE_BENCH_PAGES")
        }
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: Self.document(pages: n, word: "Section"), named: "main.tex")
        model.engineV3Enabled = true
        model.autoCompile = true
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 560, height: 700), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        s.start(model: model)
        try await EngineV3TestHost.awaitReady(s)
        func waitDone(after done: Int) async throws {
            let start = Date()
            while !(s.doneCount > done && !s.compiling && s.pageCount == n) {
                if Date().timeIntervalSince(start) > 900 { XCTFail("timeout (\(s.statusNote), \(s.pageCount) pages)"); return }
                try await Task.sleep(nanoseconds: 100_000_000)
            }
        }
        try await waitDone(after: 0)
        var rows: [String] = []
        for k in 0 ..< 4 {
            let done = s.doneCount
            if k > 0 { model.updateActiveText(Self.document(pages: n, word: "Part\(k)")) } // every page changes: all fall back again
            if k > 0 { try await waitDone(after: done) }
            XCTAssertEqual(s.pdfFallbackCount, n, "every page is drawn from the PDF")
            rows.append(String(format: "DONE %.1f ms on main, fallback load %.1f ms", s.lastDoneMainMs, s.lastFallbackLoadMs))
        }
        print("EngineV3DoneBench: \(n) pages, all PDF fallback:\n  " + rows.joined(separator: "\n  "))
    }
}
