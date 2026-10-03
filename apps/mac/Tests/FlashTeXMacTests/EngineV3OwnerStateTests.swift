import Foundation
import XCTest
@testable import FlashTeXMac

/// The test suite never writes the owner's state (lane P5-APP-PARITY review):
/// under XCTest the engine-v3 cache, the host's format cache and the flag's
/// defaults are the test process's own, whatever a test does to the
/// environment, and a stopped session saves no page snapshot later.
/// `OwnerStateGuard` (installed here and by every engine-v3 test class)
/// fails any test after which the real `~/Library/Caches/FlashTeX` or the
/// runner's `FlashTeX.EngineV3.*` defaults changed.
@MainActor
final class EngineV3OwnerStateTests: XCTestCase {
    private var env = EnvironmentOverride()

    override func setUp() { OwnerStateGuard.install() }
    override func tearDown() { env.restore() }

    func testWithoutACacheSettingATestUsesItsOwnTemporaryCache() {
        env.set("FLASHTEX_V3_CACHE", "")
        env.set("FLASHTEX_FORMAT_CACHE_DIR", "")
        XCTAssertTrue(EngineV3.underTest)
        XCTAssertEqual(EngineV3.cacheDirectory, EngineV3.testCacheDirectory)
        XCTAssertFalse(EngineV3.cacheDirectory.path.hasPrefix(OwnerStateGuard.realCache.path), EngineV3.cacheDirectory.path)
        XCTAssertTrue(EngineV3.cacheIsPrivate)
        for dir in [EngineV3Snapshot.root, EngineV3HostProcess.pidDirectory] {
            XCTAssertFalse(dir.path.hasPrefix(OwnerStateGuard.realCache.path), dir.path)
        }
        // The host's format cache moves with it.
        let hostEnv = EngineV3HostProcess.environment(host: URL(fileURLWithPath: "/nonexistent/flashtex-host"))
        let formats = hostEnv["FLASHTEX_FORMAT_CACHE_DIR"] ?? ""
        XCTAssertTrue(formats.hasPrefix(EngineV3.testCacheDirectory.path), formats)
    }

    func testTheFlagIsKeptInTheTestProcessesOwnDefaults() {
        env.set("FLASHTEX_HOST", "none")
        XCTAssertFalse(EngineV3.defaults === UserDefaults.standard)
        let before = UserDefaults.standard.object(forKey: EngineV3.enabledKey).map { String(describing: $0) }
        let model = ShellModel()
        defer { model.engineV3.stop() }
        model.engineV3Enabled = true
        model.engineV3Enabled = false
        model.engineV3Enabled = true
        XCTAssertEqual(UserDefaults.standard.object(forKey: EngineV3.enabledKey).map { String(describing: $0) }, before,
                       "the runner's standard domain is untouched")
        XCTAssertEqual(EngineV3.defaults.bool(forKey: EngineV3.enabledKey), true)
    }

    /// The review's reproduction: a session that compiled is stopped and the
    /// test's cache setting is taken away at once (as tearDowns used to);
    /// the page-snapshot save that was pending must not land anywhere.
    func testStopCancelsThePendingSnapshotSave() async throws {
        let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-owner-state-\(getpid())")
        env.set("FLASHTEX_V3_CACHE", cache.path)
        defer { try? FileManager.default.removeItem(at: cache) }
        guard EngineV3.locateHost() != nil else { throw XCTSkip("no flashtex-host built") }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-owner-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let file = dir.appendingPathComponent("main.tex")
        try "\\documentclass{article}\n\\begin{document}\nOne page.\n\\end{document}\n".write(to: file, atomically: true, encoding: .utf8)
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: file, dirty: .discard), .opened)
        model.engineV3Enabled = true
        let s = model.engineV3
        let start = Date()
        while !(s.statusNote.hasPrefix("ok") && !s.compiling) {
            if case .failed(let why) = s.phase { throw XCTSkip("host did not start: \(why)") }
            if Date().timeIntervalSince(start) > 90 { return XCTFail("no compile") }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        let key = try XCTUnwrap(EngineV3Snapshot.key(for: model))
        s.stop()
        env.restore() // the cache setting is gone, as an old tearDown left it
        try await Task.sleep(nanoseconds: 2_500_000_000) // past the 1.5 s save delay
        for root in [cache, EngineV3.testCacheDirectory, OwnerStateGuard.realCache.appendingPathComponent("engine-v3")] {
            let snap = root.appendingPathComponent("snapshots/\(key)")
            XCTAssertFalse(FileManager.default.fileExists(atPath: snap.path), "a snapshot was saved after stop(): \(snap.path)")
        }
    }

    /// The guard itself: each kind of write is reported (shown on synthetic
    /// states; writing the real cache to prove it would be the failure).
    func testTheGuardReportsEachKindOfWrite() {
        let before = OwnerStateGuard.State()
        var after = before
        XCTAssertTrue(after.written(since: before).isEmpty)
        after.ownProjectCopies = ["630c-\(getpid())-2"]
        after.ownHostFiles = ["4242"]
        after.snapshotDirectories = ["abc123"]
        after.formats = ["pdflatex-1": Date()]
        after.runnerEngineV3Keys = [EngineV3.enabledKey: "1"]
        XCTAssertEqual(after.written(since: before).count, 5, after.written(since: before).joined(separator: "; "))
    }
}
