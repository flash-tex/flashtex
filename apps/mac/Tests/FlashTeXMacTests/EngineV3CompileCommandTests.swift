import Foundation
import XCTest
@testable import FlashTeXMac

/// ⌘B and the auto-compile setting drive the engine-v3 preview (lane
/// P5-APP-PARITY, gaps A1, A3, A4, A14): ⌘B compiles with the host, never the
/// old engine; with auto-compile off, edits wait for ⌘B; an outside change to
/// an unopened `\input` file recompiles; ⌘B restarts a host that stopped.
/// The end-to-end cases need a built `flashtex-host` (skipped otherwise) and
/// use a private v3 cache.
@MainActor
final class EngineV3CompileCommandTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-compile-command-\(getpid())")
    private var storedFlag: Any?

    override func setUp() {
        setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1)
        storedFlag = UserDefaults.standard.object(forKey: EngineV3.enabledKey)
    }

    override class func tearDown() { try? FileManager.default.removeItem(at: cache) }

    override func tearDown() {
        unsetenv("FLASHTEX_HOST")
        unsetenv("FLASHTEX_V3_CACHE")
        if let storedFlag { UserDefaults.standard.set(storedFlag, forKey: EngineV3.enabledKey) } else { UserDefaults.standard.removeObject(forKey: EngineV3.enabledKey) }
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 90, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    // MARK: without a host

    func testCompileCommandNeverReachesTheOldWorkerUnderV3() {
        setenv("FLASHTEX_HOST", "none", 1)
        let model = ShellModel()
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        defer { model.engineV3.stop(); model.detachWorker() }
        model.engineV3Enabled = true
        XCTAssertTrue(model.canCompile)
        model.compileCommand()
        XCTAssertTrue(model.inFlightRequests.isEmpty, "⌘B under v3 must not reach the old worker")
        guard case .failed(let why) = model.engineV3.phase else { return XCTFail("no host: \(model.engineV3.phase)") }
        XCTAssertTrue(why.contains("flashtex-host not found"), why)
    }

    func testCanCompileWithoutAWorkerOnlyUnderV3() {
        setenv("FLASHTEX_HOST", "none", 1)
        let model = ShellModel()
        defer { model.engineV3.stop() }
        model.engineV3Enabled = false
        XCTAssertEqual(model.canCompile, model.workerAttached)
        model.engineV3Enabled = true
        XCTAssertTrue(model.canCompile, "⌘B and the auto-compile toggle are enabled with only the v3 host")
    }

    // MARK: with a host

    private func project(_ files: [String: String]) throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-cmd-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        for (name, text) in files { try text.write(to: dir.appendingPathComponent(name), atomically: true, encoding: .utf8) }
        return dir
    }

    private func startedModel(opening file: URL) async throws -> ShellModel {
        guard EngineV3.locateHost() != nil else { throw XCTSkip("no flashtex-host built (cargo build --release -p flashtex-engine --bin flashtex-host)") }
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: file, dirty: .discard), .opened)
        model.engineV3Enabled = true
        let s = model.engineV3
        try await waitUntil("the host") { s.phase == .ready || { if case .failed = s.phase { true } else { false } }() }
        guard s.phase == .ready else { throw XCTSkip("host did not start: \(s.phase)") }
        try await waitUntil("the first compile") { s.statusNote.hasPrefix("ok") && !s.compiling }
        return model
    }

    func testAutoCompileOffWaitsForCompileCommand() async throws {
        let dir = try project(["main.tex": "\\documentclass{article}\n\\begin{document}\nOne page.\n\\end{document}\n"])
        defer { try? FileManager.default.removeItem(at: dir) }
        let model = try await startedModel(opening: dir.appendingPathComponent("main.tex"))
        defer { model.engineV3.stop() }
        let s = model.engineV3
        XCTAssertEqual(s.pageCount, 1)

        model.autoCompile = false
        model.updateActiveText("\\documentclass{article}\n\\begin{document}\nOne page.\n\\newpage\nTwo.\n\\end{document}\n")
        XCTAssertTrue(s.editsWaiting)
        XCTAssertFalse(s.compiling, "with auto-compile off an edit sends nothing")
        try await Task.sleep(nanoseconds: 500_000_000)
        XCTAssertEqual(s.pageCount, 1)

        model.compileCommand() // ⌘B: walks the project off the main thread, then sends
        try await waitUntil("⌘B's compile") { s.pageCount == 2 && !s.compiling }
        XCTAssertFalse(s.editsWaiting)

        // Turning auto-compile back on sends what was typed meanwhile.
        model.updateActiveText("\\documentclass{article}\n\\begin{document}\nOne page.\n\\newpage\nTwo.\n\\newpage\nThree.\n\\end{document}\n")
        XCTAssertTrue(s.editsWaiting)
        model.autoCompile = true
        try await waitUntil("the waiting edits") { s.pageCount == 3 && !s.compiling }
        XCTAssertFalse(s.editsWaiting)
    }

    func testOutsideChangeToAnUnopenedInputRecompiles() async throws {
        let dir = try project([
            "main.tex": "\\documentclass{article}\n\\begin{document}\n\\input{chap}\n\\end{document}\n",
            "chap.tex": "A chapter on one page.\n",
        ])
        defer { try? FileManager.default.removeItem(at: dir) }
        let model = try await startedModel(opening: dir.appendingPathComponent("main.tex"))
        defer { model.engineV3.stop() }
        let s = model.engineV3
        XCTAssertEqual(s.pageCount, 1)
        // As `git checkout` or another editor would: the file changes on disk.
        try "A chapter.\n\\newpage\nNow on two pages.\n".write(to: dir.appendingPathComponent("chap.tex"), atomically: true, encoding: .utf8)
        try await waitUntil("the recompile after the outside change") { s.pageCount == 2 && !s.compiling }
    }
}
