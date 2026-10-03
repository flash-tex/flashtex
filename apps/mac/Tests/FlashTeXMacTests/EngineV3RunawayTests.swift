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
}
