import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXMac

/// View > Show Preview Debug Status under v3 (app-parity row C25): the v2
/// pane's debug strip showed its frame's id, revision and pages; the v3
/// pane shows `EngineV3Session.debugLine`, the session's own counters. No
/// host is needed: the events are fed to the session as the reader would
/// deliver them (`FLASHTEX_HOST=none`, a private cache, no bundle).
@MainActor
final class EngineV3DebugStatusTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-debug-status-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        env.set("FLASHTEX_HOST", "none")
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
    }
    override func tearDown() {
        env.restore()
        try? FileManager.default.removeItem(at: Self.cache)
    }

    static let doc = "\\documentclass{article}\n\\begin{document}\nText.\n\\end{document}\n"

    func testTheDebugLineCarriesTheSessionsCounters() {
        let m = ShellModel()
        m.replaceProject(entryText: Self.doc, named: "main.tex")
        m.engineV3Enabled = true
        let s = m.engineV3
        s.start(model: m)
        defer { s.stop() }
        XCTAssertNil(s.hostPID, "FLASHTEX_HOST=none starts no host")
        XCTAssertEqual(s.debugLine, "no host · 0 host starts · no DONE yet · layout revision \(s.layoutRevision) · 0 pages, 0 stale · 0 DONEs received")

        // A compile: STARTED, then PAGES (two pages, the second stale), then its DONE.
        s.handle(.started(.object(["id": .int(3), "keep": .bool(true)])))
        s.handle(.pages(.object(["count": .int(2), "stale": .array([.array([.int(1), .int(1)])])])))
        XCTAssertEqual(s.pageCount, 2)
        XCTAssertEqual(s.staleCount, 1)
        XCTAssertTrue(s.debugLine.contains("no DONE yet · layout revision \(s.layoutRevision) · 2 pages, 1 stale · 0 DONEs received"), s.debugLine)
        s.handle(.done(.object(["id": .int(3), "status": .string("ok"), "pages": .int(2), "mode": .string("full"), "elapsed_ms": .double(12)]), compileID: 3))
        XCTAssertEqual(s.staleCount, 0)
        XCTAssertTrue(s.debugLine.contains("last DONE #3 · layout revision \(s.layoutRevision) · 2 pages, 0 stale · 1 DONE received"), s.debugLine)

        // A cancelled compile's DONE is received but is not the last one done.
        s.handle(.started(.object(["id": .int(4), "keep": .bool(true)])))
        s.handle(.done(.object(["id": .int(4), "status": .string("cancelled")]), compileID: 4))
        XCTAssertTrue(s.debugLine.contains("last DONE #3 · "), s.debugLine)
        XCTAssertTrue(s.debugLine.contains("2 DONEs received"), s.debugLine)
        XCTAssertFalse(s.debugLine.contains("median"), "no keystroke reached the screen yet")
    }

    /// The keystroke-to-screen median is the status bar's (the last 200
    /// samples, ShellChrome): three keystrokes at 10, 30 and 20 ms.
    func testTheDebugLineShowsTheKeystrokeMedian() {
        let s = EngineV3Session()
        for (compile, ms) in [(1, 10), (2, 30), (3, 20)] {
            let key = UInt64(compile) * 1_000_000_000
            s.latency.sent(compile: compile, keystrokeNs: key, editNs: key, path: "main.tex", at: key)
            s.latency.committed(compile: compile, page: 0, at: key + UInt64(ms) * 1_000_000)
        }
        XCTAssertEqual(s.latency.samples.map(\.ms), [10, 30, 20])
        XCTAssertTrue(s.debugLine.contains("keystroke to screen median 20 ms over 3"), s.debugLine)
    }
}
