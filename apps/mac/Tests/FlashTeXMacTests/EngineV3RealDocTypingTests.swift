import AppKit
import XCTest
@testable import FlashTeXMac

/// The real-doc gate's limits for one phase (`EngineV3RealDocTypingTests.gate`). Without
/// `FLASHTEX_REALDOC_STALL_MS` (the nightly, the reference Mac) one bound, the one-frame budget,
/// and nothing may cross it. With it (the PR job on a GitHub-hosted 3-vCPU VM, #2146: 100 ms there
/// failed 6 of 7 runs, on PRs that never touch the app, by 107-470 ms) the env value is a SOFT
/// budget and three limits apply:
///   - `hard`, max(5 x budget, 500 ms): no key and no turn may cross it. The owner's freeze was
///     1.9 s on main (before #1890 and #1900), about 4 x over;
///   - `allowed`, an eighth of the phase's keys and at least 3 (5 of the 40): at most that many
///     keys, and at most that many turns, over the soft budget. One VM pause late-delivers every
///     key posted during it (run 41432961769: turns of 123, 124 and 470 ms back to back made 5
///     keys late), while a regression that costs every key or every compile a long turn shows up
///     about 40 times;
///   - `totalMs`, 3 x `hard`: the turns over the soft budget may add up to at most that, so a
///     freeze cut into turns under `hard` (1.9 s as 5 x 380 ms) still fails. The worst VM noise
///     measured adds up to 0.99 s.
struct RealDocStallLimits {
    var soft: Double
    var hard: Double
    var allowed: Int
    var totalMs: Double

    static func of(budgetMs: Double, hosted: Bool, wanted: Int) -> RealDocStallLimits {
        guard hosted else { return RealDocStallLimits(soft: budgetMs, hard: budgetMs, allowed: 0, totalMs: .infinity) }
        let hard = max(5 * budgetMs, 500)
        return RealDocStallLimits(soft: budgetMs, hard: hard, allowed: max(3, wanted / 8), totalMs: 3 * hard)
    }

    var text: String {
        allowed == 0 ? "budget \(soft) ms"
            : "soft budget \(soft) ms, \(allowed) allowed over it, hard limit \(hard) ms, \(totalMs) ms in all"
    }

    /// The misses of one phase: each key's way into the text storage (ms, in the order typed) and
    /// main's run-loop turns (ms, and when from the phase's start). Empty: the phase passed.
    func misses(_ name: String, keysMs: [Double], turns: [(ms: Double, atMs: Double)]) -> [String] {
        var out: [String] = []
        let late = keysMs.enumerated().filter { $0.element > soft }
        let lateText = late.prefix(8).map { "key \($0.offset) \(String(format: "%.1f", $0.element)) ms" }.joined(separator: ", ")
        if late.contains(where: { $0.element > hard }) {
            out.append("\(name): a key reached the text storage after the hard limit (\(text)): " + lateText)
        } else if late.count > allowed {
            out.append("\(name): \(late.count) of \(keysMs.count) keys reached the text storage later than the budget (\(text)): " + lateText)
        }
        let stalls = turns.filter { $0.ms > soft }.sorted { $0.ms > $1.ms }
        let stallText = stalls.prefix(8).map { String(format: "%.1f ms at +%.0f ms", $0.ms, $0.atMs) }.joined(separator: ", ")
        let total = stalls.reduce(0.0) { $0 + $1.ms }
        if stalls.contains(where: { $0.ms > hard }) {
            out.append("\(name): a main run-loop turn over the hard limit (\(text)), longest " + stallText)
        } else if stalls.count > allowed {
            out.append("\(name): \(stalls.count) main run-loop turns over the budget (\(text)), longest " + stallText)
        } else if total > totalMs {
            out.append("\(name): main run-loop turns over the budget add up to \(String(format: "%.0f", total)) ms (\(text)): " + stallText)
        }
        return out
    }
}

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
/// (`MainThreadStall.frameMs`, 16 ms), or any key is not in the editor's text storage within that
/// of being posted, or a key is never handled. `FLASHTEX_REALDOC_STALL_MS` (100 in the PR CI's
/// hosted job) makes the frame a soft budget with a hard limit, a count and a total (`RealDocStallLimits`).
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

    /// The budget: one frame (`MainThreadStall.frameMs`), or `FLASHTEX_REALDOC_STALL_MS` (the PR
    /// CI's hosted job uses 100 ms as a soft budget, see `RealDocStallLimits`: a shared 3-vCPU VM has hiccups
    /// of its own; the one-frame bound stays the nightly's and the reference's).
    private var budgetMs: Double {
        ProcessInfo.processInfo.environment["FLASHTEX_REALDOC_STALL_MS"].flatMap(Double.init) ?? MainThreadStall.frameMs
    }

    private func gate(_ p: Phase) {
        let hosted = ProcessInfo.processInfo.environment["FLASHTEX_REALDOC_STALL_MS"].flatMap(Double.init) != nil
        let limits = RealDocStallLimits.of(budgetMs: budgetMs, hosted: hosted, wanted: p.wanted)
        XCTAssertEqual(p.keys.count, p.wanted, "\(p.name): \(p.wanted - p.keys.count) of \(p.wanted) keys never handled")
        let missing = p.keys.filter { !$0.inStorage }.count
        XCTAssertEqual(missing, 0, "\(p.name): \(missing) keys not in the editor's text storage")
        let turns = p.report.over(limits.soft).map { (ms: $0.ms, atMs: Double($0.startNs &- p.report.startNs) / 1e6) }
        for miss in limits.misses(p.name, keysMs: p.keys.map(\.ms), turns: turns) { XCTFail(miss) }
    }

    /// The limits themselves, on the numbers #2146 measured (no TeX or book needed: this runs in
    /// the mac-app job): every hosted noise run passes, and the freeze fails however it comes.
    func testTheHostedLimitsPassTheMeasuredNoiseAndFailTheFreeze() {
        let hosted = RealDocStallLimits.of(budgetMs: 100, hosted: true, wanted: 40)
        XCTAssertEqual([hosted.hard, Double(hosted.allowed), hosted.totalMs], [500, 5, 1500])
        func keys(_ late: [Double]) -> [Double] { late + Array(repeating: 5, count: 40 - late.count) }
        func turns(_ ms: [Double]) -> [(ms: Double, atMs: Double)] { ms.enumerated().map { (ms: $0.element, atMs: Double($0.offset) * 100) } }
        // Every hosted phase that missed 100 ms in #2146's runs and main's 41510925330: the keys
        // and the turns over 100 ms, from the runs' RealDocTyping lines and failures.
        let noise: [(String, [Double], [Double])] = [
            ("41510925330 (main) cold", [107.1], [120.1]),
            ("41466466009 late", [], [118.8]),
            ("41432999342 late", [], [159.3, 121.7, 102.9]),
            ("41432961769 cold", [], [116.0]),
            ("41432961769 late", [253.8, 439.4, 354.3, 275.1, 108.2], [470.5, 138.1, 134.0, 124.0, 123.4]),
            ("41427377621 cold", [], [161.1, 149.7, 142.8]),
            ("41427377621 late", [], [104.4]),
            ("41389406318 cold", [], [102.7]),
            ("41389406318 late", [147.5, 149.5], [197.5, 149.2, 130.5, 116.6]),
            ("41385226411/2 late", [128.8], [238.1]),
            ("41385226411/3 cold", [], [119.1]),
            ("41385226411/3 late", [], [111.1, 103.2]),
            ("41385296674/2 cold", [], [139.3]),
            ("41385296674/2 late", [142.9], [232.1, 113.6, 107.2]),
            ("41385296674/3 cold", [], [143.9]),
            ("41385296674/3 late", [106.7], [107.1]),
        ]
        for (name, k, t) in noise {
            XCTAssertEqual(hosted.misses(name, keysMs: keys(k), turns: turns(t)), [], name)
        }
        let freezes: [(String, [Double], [Double])] = [
            ("the owner's freeze: one 1.9 s turn", [1900, 1850, 1800], [1900]),
            ("1.9 s cut into 5 turns of 380 ms", [380, 340, 300], [380, 380, 380, 380, 380]),
            // REVIEW-C's Q1: 5 x 300 ms (1.5 s, the total's bound) plus four turns of 100 ms (not
            // over the soft budget). The turns pass; the keys typed 50 ms apart during each 300 ms
            // pause do not (four of them over 100 ms per pause).
            ("1.9 s as 5 x 300 ms and 4 x 100 ms", Array(repeating: [300.0, 250, 200, 150], count: 5).flatMap { $0 },
             [300, 300, 300, 300, 300, 100, 100, 100, 100]),
            ("every compile costs main a 150 ms turn", [], Array(repeating: 150, count: 40)),
            ("every key 120 ms late", Array(repeating: 120, count: 40), []),
        ]
        for (name, k, t) in freezes {
            XCTAssertFalse(hosted.misses(name, keysMs: keys(k), turns: turns(t)).isEmpty, name)
        }
        // Without the env (nightly, reference): one frame, nothing over it.
        let frame = RealDocStallLimits.of(budgetMs: MainThreadStall.frameMs, hosted: false, wanted: 40)
        XCTAssertFalse(frame.misses("one late turn", keysMs: keys([]), turns: turns([17])).isEmpty)
        XCTAssertEqual(frame.misses("all within a frame", keysMs: keys([]), turns: turns([16])), [])
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
