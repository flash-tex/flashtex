import Foundation
import XCTest
@testable import FlashTeXMac

/// A compile that never finishes (gap A15). TeX stops a superseded or
/// cancelled compile only at a page or segment checkpoint, so an endless
/// macro loop before the first page holds the engine thread for good: every
/// later edit queues behind it and the preview never recovers. The app
/// bounds it as the old path did (a stall bound): no message from the host
/// for `FLASHTEX_V3_STALL_S` seconds while a compile is out stops the host,
/// says so, and the next edit (or ⌘B) compiles on a fresh one. Needs a built
/// `flashtex-host` and TeX Live (skipped otherwise).
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
}
