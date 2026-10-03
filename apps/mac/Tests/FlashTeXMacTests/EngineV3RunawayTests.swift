import Foundation
import XCTest
@testable import FlashTeXMac

/// A compile that never finishes (gap A15). TeX stops a superseded or
/// cancelled compile only at a page or segment checkpoint, so an endless
/// macro loop before the first page holds the engine thread for good: every
/// later edit queues behind it and the preview never recovers. The app
/// bounds it as the old path did (`EngineV3StallBound`): while the host
/// typesets a compile and runs no external tool, no message for 30 s (5 min
/// after ⌘B; `FLASHTEX_V3_STALL_S` and ten times it in tests) stops the host,
/// says so, and the next edit (or ⌘B) compiles on a fresh one. A long silent
/// tool phase (bibtex, makeindex and their compiles) is never stopped, and
/// Stop Compile ends any compile. The host tests need a built `flashtex-host`
/// and TeX Live (skipped otherwise).
@MainActor
final class EngineV3RunawayTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-runaway-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        env.set("FLASHTEX_V3_STALL_S", "4")
    }
    override func tearDown() { env.restore() }

    func waitUntil(_ what: String, timeout: TimeInterval, _ cond: @escaping () -> Bool) async throws -> Bool {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { return false }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        return true
    }

    static let good = "\\documentclass{article}\n\\begin{document}\nGood text.\n\\end{document}\n"
    static let loop = "\\documentclass{article}\n\\begin{document}\n\\def\\x{\\x}\\x\nGood text.\n\\end{document}\n"
    static let fixed = "\\documentclass{article}\n\\begin{document}\nFixed text.\n\\end{document}\n"

    func testAnEndlessLoopIsStoppedAndTheNextEditCompiles() async throws {
        try EngineV3TestHost.require()
        let m = ShellModel()
        m.replaceProject(entryText: Self.good, named: "main.tex")
        m.engineV3Enabled = true
        m.autoCompile = true
        let s = m.engineV3
        s.start(model: m)
        defer { s.stop() }
        try await EngineV3TestHost.awaitReady(s)
        let ok = try await waitUntil("the first compile", timeout: 120) { s.statusNote.hasPrefix("ok") && !s.compiling }
        XCTAssertTrue(ok, s.statusNote)

        // An endless loop: the stall bound stops it and says so.
        m.updateActiveText(Self.loop)
        XCTAssertTrue(s.compiling)
        let stopped = try await waitUntil("the stall bound", timeout: 60) { s.statusNote.hasPrefix("stopped") }
        XCTAssertTrue(stopped, "the endless loop was never stopped: \(s.statusNote)")
        XCTAssertFalse(s.compiling)
        XCTAssertEqual(s.firstError?.hasPrefix("TeX did not finish"), true, s.firstError ?? "nil")
        // It is not compiled again by itself (it would loop again).
        try await Task.sleep(nanoseconds: 6_000_000_000)
        XCTAssertTrue(s.statusNote.hasPrefix("stopped"), "not recompiled: \(s.statusNote)")

        // The fix compiles on a fresh host.
        m.updateActiveText(Self.fixed)
        let fixed = try await waitUntil("the fixed text", timeout: 120) { s.statusNote.hasPrefix("ok") && !s.compiling }
        XCTAssertTrue(fixed, "the fixed text did not compile: \(s.statusNote)")
    }

    // MARK: the rule

    func testTheBoundIs30sForTypingAnd5MinutesForCommandB() {
        env.set("FLASHTEX_V3_STALL_S", "") // no override: the shipped bounds
        XCTAssertEqual(EngineV3StallBound.seconds(explicit: false), 30)
        XCTAssertEqual(EngineV3StallBound.seconds(explicit: true), 300)
        XCTAssertEqual(EngineV3StallBound.describe(300), "5 min")
        XCTAssertEqual(EngineV3StallBound.describe(30), "30 s")
        func stop(_ silent: Double, explicit: Bool = false, tools: Bool = false, typesetting: Bool = true, exporting: Bool = false) -> Bool {
            EngineV3StallBound.shouldStop(typesetting: typesetting, toolsRunning: tools, exporting: exporting, explicit: explicit, silentSeconds: silent)
        }
        XCTAssertFalse(stop(29))
        XCTAssertTrue(stop(31))
        XCTAssertFalse(stop(299, explicit: true), "⌘B: 5 min")
        XCTAssertTrue(stop(301, explicit: true))
        // Long silent phases that are not runaways are never stopped.
        XCTAssertFalse(stop(446, tools: true), "an external-tools round (INFDESC: makeindex and its compile, 446 s)")
        XCTAssertFalse(stop(3600, typesetting: false), "nothing typesetting (a compile queued, the host idle)")
        XCTAssertFalse(stop(3600, exporting: true), "an export has its own bound")
    }

    /// A stub of the host's frames around a real session: while the host
    /// reports a tool running, an hour of silence during a compile stops
    /// nothing; once the cycle settles, the same silence stops it.
    func testALongSilentToolPhaseIsNotStopped() async throws {
        try EngineV3TestHost.require()
        let m = ShellModel()
        m.replaceProject(entryText: Self.good, named: "main.tex")
        m.engineV3Enabled = true
        m.autoCompile = true
        let s = m.engineV3
        s.start(model: m)
        defer { s.stop() }
        try await EngineV3TestHost.awaitReady(s)
        let ok = try await waitUntil("the first compile", timeout: 120) { s.statusNote.hasPrefix("ok") && !s.compiling }
        XCTAssertTrue(ok, s.statusNote)
        // All on the main thread, no suspension: the real DONE cannot land in between.
        m.updateActiveText(Self.fixed)
        XCTAssertTrue(s.compiling)
        s.handle(.started(.object(["id": .int(999), "mode": .string("resident")])))
        s.handle(.tool(.object(["id": .int(999), "event": .string("run"), "tool": .string("makeindex")])))
        XCTAssertTrue(s.toolsRunning)
        let hour: UInt64 = 3_600_000_000_000
        s.checkStall(nowNs: MonotonicClock.nowNs() + hour)
        XCTAssertFalse(s.statusNote.hasPrefix("stopped"), "a tool phase is not stopped: \(s.statusNote)")
        XCTAssertTrue(s.compiling)
        XCTAssertTrue(s.compileRunningLong, "the pane offers Stop Compile")
        s.handle(.tool(.object(["id": .int(999), "event": .string("settled")])))
        XCTAssertFalse(s.toolsRunning)
        s.checkStall(nowNs: MonotonicClock.nowNs() + hour)
        XCTAssertTrue(s.statusNote.hasPrefix("stopped"), "typesetting again, silent for an hour: stopped (\(s.statusNote))")
        // A fresh host compiles the next edit.
        m.updateActiveText(Self.good)
        let again = try await waitUntil("the next edit", timeout: 120) { s.statusNote.hasPrefix("ok") && !s.compiling }
        XCTAssertTrue(again, s.statusNote)
    }

    /// Stop Compile ends a running compile at once; the next edit compiles.
    func testStopCompileEndsARunningCompile() async throws {
        try EngineV3TestHost.require()
        let m = ShellModel()
        m.replaceProject(entryText: Self.good, named: "main.tex")
        m.engineV3Enabled = true
        m.autoCompile = true
        let s = m.engineV3
        s.start(model: m)
        defer { s.stop() }
        try await EngineV3TestHost.awaitReady(s)
        let ok = try await waitUntil("the first compile", timeout: 120) { s.statusNote.hasPrefix("ok") && !s.compiling }
        XCTAssertTrue(ok, s.statusNote)
        m.updateActiveText(Self.loop)
        XCTAssertTrue(s.compiling)
        s.stopCompile()
        XCTAssertFalse(s.compiling)
        XCTAssertEqual(s.statusNote, "stopped · ⌘B to compile again")
        m.updateActiveText(Self.fixed)
        let fixed = try await waitUntil("the fixed text", timeout: 120) { s.statusNote.hasPrefix("ok") && !s.compiling }
        XCTAssertTrue(fixed, s.statusNote)
    }

    // MARK: review of #1417

    /// A two-pass document whose second pass re-typesets every page and
    /// sends almost none (unchanged pages are not sent again): the pass is
    /// silent apart from the host's `progress-v1` heartbeat. Measured here:
    /// the longest stretch with no frame but PROGRESS while the compile
    /// typesets is longer than the bound (0.5 s) plus the check interval,
    /// so without the heartbeat the bound would have stopped it; with it,
    /// the compile finishes. A client that does not accept `progress-v1`
    /// (an older app) gets no PROGRESS frame.
    func testASilentLaterPassIsNotStoppedWithTheHeartbeat() async throws {
        try EngineV3TestHost.require()
        env.set("FLASHTEX_V3_STALL_S", "0.5")
        let para = String(repeating: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ", count: 12)
        // Each section also does some silent work (a counting loop), so a
        // pass takes seconds while the time between pages stays short; and
        // each section reads a reference the first pass does not know yet,
        // in an invisible, fixed-width box. The `.aux` then changes (a
        // second pass runs), every page reads the changed reference (so the
        // pass cannot stop early), and no page changes (so none is sent).
        let work = "{\\count255=0 \\loop\\advance\\count255 1 \\ifnum\\count255<150000 \\repeat}"
        let ref = "\\makebox[3em][l]{\\phantom{\\pageref{s120}}}"
        let body = (1 ... 120).map { "\\section{S\($0)}\\label{s\($0)} \(ref)\(work)\(para)\n" }.joined()
        // A preamble of its own: the run is not resumed from another
        // test's post-preamble snapshot.
        let doc = "\\documentclass{article}\n\\newcommand\\runid{\(UUID().uuidString)}\n\\begin{document}\n\(body)\\end{document}\n"
        let m = ShellModel()
        m.replaceProject(entryText: doc, named: "main.tex")
        m.engineV3Enabled = true
        let s = m.engineV3
        var lastFrame = Date(), maxGap = 0.0, passes = Set<Int>()
        var trace: [String] = []
        let t0 = Date()
        s.afterEvent = { out in
            let t = String(format: "%.2f", Date().timeIntervalSince(t0))
            switch out {
            case .progress(let j):
                if let p = j["pass"]?.int { passes.insert(Int(p)) }
                trace.append("\(t) P\(j["pass"]?.int ?? 0):\(j["page"]?.int ?? 0)")
            default:
                if s.compiling { maxGap = max(maxGap, Date().timeIntervalSince(lastFrame)) }
                lastFrame = Date()
                switch out {
                case .page(let p, _, _, _): trace.append("\(t) page\(p.page.index)")
                case .started: trace.append("\(t) STARTED")
                case .done(let j, _): trace.append("\(t) DONE \(j["mode"]?.string ?? "") passes=\(j["passes"]?.int ?? -1)")
                default: break
                }
            }
        }
        s.start(model: m)
        defer { s.stop() }
        try await EngineV3TestHost.awaitReady(s)
        lastFrame = Date()
        let done = try await waitUntil("the two-pass compile", timeout: 240) {
            s.statusNote.hasPrefix("stopped") || (s.statusNote.hasPrefix("ok") && !s.compiling)
        }
        XCTAssertTrue(done, s.statusNote)
        XCTAssertFalse(s.statusNote.hasPrefix("stopped"), "with the heartbeat the compile finishes: \(s.statusNote)")
        XCTAssertGreaterThan(s.progressFrames, 0, "the host sent PROGRESS")
        XCTAssertTrue(passes.contains(2), "a second pass: \(passes.sorted())")
        XCTAssertGreaterThan(maxGap, 0.5 + 1.0, "a silent stretch longer than the bound and the check interval (\(maxGap) s): without the heartbeat it would be stopped; passes \(passes.sorted()); trace \(trace.joined(separator: " "))")

        // An older app (no `progress-v1` in its HELLO) gets none.
        env.set("FLASHTEX_V3_NO_PROGRESS", "1")
        env.set("FLASHTEX_V3_STALL_S", "30")
        let old = ShellModel()
        old.replaceProject(entryText: Self.good, named: "main.tex")
        old.engineV3Enabled = true
        let o = old.engineV3
        o.start(model: old)
        defer { o.stop() }
        try await EngineV3TestHost.awaitReady(o)
        let ok = try await waitUntil("the older client's compile", timeout: 120) { o.statusNote.hasPrefix("ok") && !o.compiling }
        XCTAssertTrue(ok, o.statusNote)
        XCTAssertEqual(o.progressFrames, 0, "no PROGRESS to a client that did not accept it")
    }

    /// The tool cycle is keyed to its compile id: a newer client compile's
    /// STARTED ends a cycle that never settled; a tools follow-up does not;
    /// stopping the compile clears it.
    func testToolsRunningEndsWithANewerCompile() {
        env.set("FLASHTEX_HOST", "none")
        let m = ShellModel()
        m.replaceProject(entryText: Self.good, named: "main.tex")
        m.engineV3Enabled = true
        let s = m.engineV3
        defer { s.stop() }
        s.handle(.tool(.object(["id": .int(5), "event": .string("run"), "tool": .string("bibtex")])))
        XCTAssertTrue(s.toolsRunning)
        s.handle(.started(.object(["id": .int(5), "cause": .string("tools")])))
        XCTAssertTrue(s.toolsRunning, "the cycle's own follow-up compile")
        s.handle(.started(.object(["id": .int(6)])))
        XCTAssertFalse(s.toolsRunning, "a newer client compile ends a cycle that never settled")
        s.handle(.tool(.object(["id": .int(7), "event": .string("run"), "tool": .string("makeindex")])))
        s.handle(.tool(.object(["id": .int(6), "event": .string("settled")])))
        XCTAssertTrue(s.toolsRunning, "an older cycle's settled does not end a newer one")
        s.handle(.tool(.object(["id": .int(7), "event": .string("settled")])))
        XCTAssertFalse(s.toolsRunning)
        // A late run of a cycle a newer compile superseded (its settled may never come) is ignored.
        s.handle(.started(.object(["id": .int(9)])))
        s.handle(.tool(.object(["id": .int(8), "event": .string("run"), "tool": .string("bibtex")])))
        XCTAssertFalse(s.toolsRunning, "a run for compile 8 after compile 9 started does not hold the bound off")
        s.handle(.tool(.object(["id": .int(9), "event": .string("run"), "tool": .string("bibtex")])))
        XCTAssertTrue(s.toolsRunning, "the newest compile's own run does")
    }

    /// A superseded cycle's late run changes nothing the current cycle
    /// shows: its tool rows stay in the Problems panel and no "Running …"
    /// note appears (#1438 review: all three side effects under the guard).
    func testASupersededRunLeavesTheCurrentCyclesRowsAndNote() {
        env.set("FLASHTEX_HOST", "none")
        let m = ShellModel()
        m.replaceProject(entryText: Self.good, named: "main.tex")
        m.engineV3Enabled = true
        let s = m.engineV3
        s.start(model: m)
        defer { s.stop() }
        s.handle(.started(.object(["id": .int(9)])))
        s.handle(.tool(.object(["id": .int(9), "event": .string("run"), "tool": .string("bibtex")])))
        XCTAssertEqual(s.toolNote, "Running bibtex…")
        s.handle(.diagnostic(.object(["id": .int(9), "source": .string("bibtex"), "severity": .string("warning"),
                                      "message": .string("I didn't find a database entry for \"knuth\"")])))
        s.handle(.tool(.object(["id": .int(9), "event": .string("done"), "tool": .string("bibtex"), "status": .string("warnings")])))
        let rows = m.engineV3Diagnostics
        XCTAssertEqual(rows.count, 1, "the current cycle's bibtex row")
        XCTAssertNil(s.toolNote)
        s.handle(.tool(.object(["id": .int(8), "event": .string("run"), "tool": .string("makeindex")])))
        XCTAssertNil(s.toolNote, "a superseded cycle's run does not show as running")
        s.handle(.tool(.object(["id": .int(9), "event": .string("done"), "tool": .string("bibtex"), "status": .string("ok")])))
        XCTAssertEqual(m.engineV3Diagnostics, rows, "nor does it replace the current cycle's rows")
    }

    /// An export waiting for a compile that is stopped fails at once instead of hanging.
    func testStoppingACompileFailsAPendingExport() async throws {
        try EngineV3TestHost.require()
        let m = ShellModel()
        m.replaceProject(entryText: Self.good, named: "main.tex")
        m.engineV3Enabled = true
        m.autoCompile = true
        let s = m.engineV3
        s.start(model: m)
        defer { s.stop() }
        try await EngineV3TestHost.awaitReady(s)
        let ok = try await waitUntil("the first compile", timeout: 120) { s.statusNote.hasPrefix("ok") && !s.compiling }
        XCTAssertTrue(ok, s.statusNote)
        m.updateActiveText(Self.loop)
        var result: Result<Data, EngineV3Session.ExportFailure>?
        s.export(model: m) { result = $0 }
        try await Task.sleep(nanoseconds: 500_000_000)
        XCTAssertNil(result, "the export waits for the looping compile")
        s.stopCompile()
        let finished = try await waitUntil("the export's end", timeout: 10) { result != nil }
        XCTAssertTrue(finished, "the export did not end when the compile was stopped")
        if case .success = result { XCTFail("a stopped compile cannot export") }
    }

    /// Stop Compile measures from the oldest unanswered compile: continuous
    /// typing (each compile superseded and answered) never shows it.
    func testContinuousTypingDoesNotShowStopCompile() async throws {
        try EngineV3TestHost.require()
        let m = ShellModel()
        m.replaceProject(entryText: Self.good, named: "main.tex")
        m.engineV3Enabled = true
        m.autoCompile = true
        let s = m.engineV3
        s.start(model: m)
        defer { s.stop() }
        try await EngineV3TestHost.awaitReady(s)
        let ok = try await waitUntil("the first compile", timeout: 120) { s.statusNote.hasPrefix("ok") && !s.compiling }
        XCTAssertTrue(ok, s.statusNote)
        var text = Self.good
        var shown = false
        for i in 0 ..< 30 { // 3 s of typing, a key every 100 ms
            text = Self.good.replacingOccurrences(of: "Good text.", with: "Good text \(i).")
            m.updateActiveText(text)
            try await Task.sleep(nanoseconds: 100_000_000)
            s.checkStall()
            shown = shown || s.compileRunningLong
        }
        XCTAssertFalse(shown, "Stop Compile appeared while typing")
    }
}

