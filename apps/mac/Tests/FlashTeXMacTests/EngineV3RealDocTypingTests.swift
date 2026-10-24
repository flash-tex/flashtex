import AppKit
import XCTest
@testable import FlashTeXMac

/// REAL-DOC-GATE's app-level gate (2026-10-24). The owner typed on page 883 of Infinite Descent ×2
/// (1,152 pages, the second copy's real-numbers chapter): each key took 32 s to reach the
/// preview, and the editor froze. T7 gated generated documents only, and EditorInstantTests'
/// cold-compile case feeds synthetic host events, so neither saw it.
///
/// The whole app (ContentView, engine v3, a real `flashtex-host`) opens the real book, switches
/// to the chapter file of the edited page, and types keys 50 ms apart (`ClockTypist`, from another
/// thread, as an input source arrives):
///   - `cold`: while the document's first compile is still streaming its pages in;
///   - `late`: once it has settled, at the late page, where each key starts the long compile;
///     timing goes on until that compile has settled again (or 120 s), since a freeze may come
///     when its results arrive.
/// It fails when, in either phase, any main run-loop turn is longer than one frame
/// (`MainThreadStall.frameMs`, 16 ms), or any key is not in the editor's text storage within
/// one frame of being posted, or a key is never handled.
///
/// The document: `FLASHTEX_REALDOC_DIR` (an unpacked infdesc-x2 tree: `tools/incr-bench/mkbook.py
/// infdesc-x2`), entry `FLASHTEX_REALDOC_MAIN` (default `infdesc-x2.tex`), the edited chapter
/// `FLASHTEX_REALDOC_EDIT` (default `x2/book/real-numbers/inequalities.tex`, the owner's page
/// 883) and the line `FLASHTEX_REALDOC_LINE` (default 112, a prose line on page 887 of the oracle's
/// 1,152). Without the document the test skips; `FLASHTEX_REQUIRE_REALDOC=1` (ci.yml's
/// `real-doc gate` job) makes that a failure. `scripts/realdoc-gate.sh` runs it locally.
@MainActor
final class EngineV3RealDocTypingTests: XCTestCase {
    private var env = EnvironmentOverride()
    private var window: NSWindow?

    override func setUp() async throws {
        OwnerStateGuard.install()
        if ProcessInfo.processInfo.environment["FLASHTEX_V3_CACHE"] == nil {
            env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("realdoc-\(getpid())").path)
        }
    }

    override func tearDown() async throws {
        window?.orderOut(nil)
        window?.contentView = nil
        window = nil
        env.restore()
    }

    private func waitUntil(_ what: String, timeout: TimeInterval, _ cond: @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("timed out after \(Int(timeout)) s waiting for \(what)")
        throw CancellationError()
    }

    struct Phase {
        var name: String
        var keys: [ClockTypist.Key]
        var report: MainThreadStallMonitor.Report
        var wanted: Int
        var summary: [String: Any] {
            ["keys": keys.count, "wanted": wanted, "key_to_storage_ms": MainThreadStall.stats(keys.map(\.ms)),
             "not_in_storage": keys.filter { !$0.inStorage }.count, "main": report.summary]
        }
    }

    /// Types `count` keys 50 ms apart while the monitor runs; after typing, `settle` runs (on
    /// main, still monitored) before the monitor stops.
    private func typePhase(_ name: String, _ count: Int, into tv: NSTextView, session: EngineV3Session,
                           settle: (() async throws -> Void)? = nil) async throws -> Phase {
        let monitor = MainThreadStallMonitor()
        monitor.start()
        let keys = await ClockTypist().type(count, into: tv, timeout: Double(count) * 0.05 + 120) { [weak session] ns in
            session?.nextKeystrokeNs = ns
        }
        try await Task.sleep(nanoseconds: 200_000_000)
        try? await settle?()
        let phase = Phase(name: name, keys: keys, report: monitor.stop(), wanted: count)
        if let d = try? JSONSerialization.data(withJSONObject: phase.summary, options: [.sortedKeys]) {
            print("RealDocTyping \(name): " + String(decoding: d, as: UTF8.self))
        }
        return phase
    }

    private func gate(_ p: Phase) {
        let frame = MainThreadStall.frameMs
        XCTAssertEqual(p.keys.count, p.wanted, "\(p.name): \(p.wanted - p.keys.count) of \(p.wanted) keys never handled")
        let late = p.keys.enumerated().filter { $0.element.ms > frame }
        XCTAssertTrue(late.isEmpty, "\(p.name): \(late.count) of \(p.keys.count) keys reached the text storage later than one frame "
                      + "(\(frame) ms): " + late.prefix(8).map { "key \($0.offset) \(String(format: "%.1f", $0.element.ms)) ms" }.joined(separator: ", "))
        let missing = p.keys.filter { !$0.inStorage }.count
        XCTAssertEqual(missing, 0, "\(p.name): \(missing) keys not in the editor's text storage")
        let stalls = p.report.over(frame)
        XCTAssertTrue(stalls.isEmpty, "\(p.name): \(stalls.count) main run-loop turns over one frame (\(frame) ms), longest "
                      + stalls.prefix(8).map { String(format: "%.1f ms at +%.0f ms", $0.ms, Double($0.startNs &- p.report.startNs) / 1e6) }
                          .joined(separator: ", "))
    }

    func testTypingOnALatePageOfARealBookNeverStallsTheEditor() async throws {
        let e = ProcessInfo.processInfo.environment
        guard let dir = e["FLASHTEX_REALDOC_DIR"], !dir.isEmpty else {
            let why = "set FLASHTEX_REALDOC_DIR to an unpacked infdesc-x2 (tools/incr-bench/mkbook.py infdesc-x2; scripts/realdoc-gate.sh)"
            if e["FLASHTEX_REQUIRE_REALDOC"] == "1" { XCTFail("FLASHTEX_REQUIRE_REALDOC=1: " + why); return }
            throw XCTSkip(why)
        }
        try EngineV3TestHost.require()
        let main = e["FLASHTEX_REALDOC_MAIN"] ?? "infdesc-x2.tex"
        let edit = e["FLASHTEX_REALDOC_EDIT"] ?? "x2/book/real-numbers/inequalities.tex"
        let line = Int(e["FLASHTEX_REALDOC_LINE"] ?? "") ?? 112
        let keys = Int(e["FLASHTEX_REALDOC_KEYS"] ?? "") ?? 40

        let model = ShellModel()
        _ = model.openTex(at: URL(fileURLWithPath: dir).appendingPathComponent(main))
        model.engineV3Enabled = true
        model.autoCompile = true
        let (window, tv) = try await MainThreadStall.hostEditor(model)
        self.window = window
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the first page", timeout: 600) { s.pageCount > 0 }
        let switched = await model.openAndSwitch(edit, role: .opened) { XCTFail("opening \(edit): \($0)") }
        XCTAssertTrue(switched)
        try await waitUntil("the editor showing \(edit)", timeout: 30) { model.activePath.hasSuffix(edit) }
        try await Task.sleep(nanoseconds: 300_000_000)

        // The caret: inside a word just after the middle of the given line.
        func placeCaret() throws {
            let ns = tv.string as NSString
            var start = 0
            for _ in 1 ..< line {
                let r = ns.range(of: "\n", range: NSRange(location: start, length: ns.length - start))
                guard r.location != NSNotFound else { XCTFail("\(edit) has fewer than \(line) lines"); throw CancellationError() }
                start = r.location + 1
            }
            let end = ns.range(of: "\n", range: NSRange(location: start, length: ns.length - start)).location
            let text = ns.substring(with: NSRange(location: start, length: (end == NSNotFound ? ns.length : end) - start))
            let mid = (text as NSString).range(of: " ", range: NSRange(location: text.utf16.count / 2, length: text.utf16.count / 2))
            XCTAssertNotEqual(mid.location, NSNotFound, "line \(line) of \(edit) is not prose")
            tv.setSelectedRange(NSRange(location: start + mid.location + 3, length: 0))
            tv.window?.makeFirstResponder(tv)
        }
        try placeCaret()

        // cold: typing while the first compile is still running.
        let coldRunning = s.compiling
        let cold = try await typePhase("cold", keys, into: tv, session: s)
        try await waitUntil("the first compile settled", timeout: 1800) { !s.compiling && s.pageCount > 0 }
        try await Task.sleep(nanoseconds: 500_000_000)

        // late: typing at the late page, each key a compile there; timed until it settles.
        try placeCaret()
        let late = try await typePhase("late", keys, into: tv, session: s) {
            let deadline = Date().addingTimeInterval(120)
            while s.compiling, Date() < deadline { try await Task.sleep(nanoseconds: 50_000_000) }
        }
        print("RealDocTyping env: pages \(s.pageCount), status \(s.statusNote), cold phase began during the first compile: \(coldRunning)")
        gate(cold)
        gate(late)
    }
}
