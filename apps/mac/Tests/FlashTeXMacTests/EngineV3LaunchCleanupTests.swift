import Foundation
import XCTest
@testable import FlashTeXMac

/// Cleanup and whole-file work never block the main thread at a host launch
/// or a project's first compile (review lane REVIEW-APP): removing exited
/// instances' project copies (`EngineV3Mirror.removeAbandoned`, every launch
/// and restart) ran on main and took as long as their files took to delete;
/// a taken-over copy's output check and emptying ran in `EngineV3Mirror.init`
/// on main.
@MainActor
final class EngineV3LaunchCleanupTests: XCTestCase {
    private var env = EnvironmentOverride()
    private var dir: URL!

    override func setUpWithError() throws {
        OwnerStateGuard.install()
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-launch-cleanup-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        env.set("FLASHTEX_V3_CACHE", dir.appendingPathComponent("cache").path)
        env.set("FLASHTEX_HOST", "none")
        // No bundle (never a lock file of this Mac's): nothing asks before the host starts.
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
    }

    override func tearDownWithError() throws {
        env.restore()
        try? FileManager.default.removeItem(at: dir)
    }

    static let key = "0123456789abcdef"

    /// An exited instance's copy `projects/<key>-<n>-1` holding `files` files.
    private func abandonedCopy(_ n: Int, files: Int) throws -> URL {
        let fm = FileManager.default
        let base = EngineV3.cacheDirectory.appendingPathComponent("projects/\(Self.key)-\(n)-1", isDirectory: true)
        let src = base.appendingPathComponent("src", isDirectory: true)
        try fm.createDirectory(at: src, withIntermediateDirectories: true)
        try fm.createDirectory(at: base.appendingPathComponent("out", isDirectory: true), withIntermediateDirectories: true)
        let bytes = Data(repeating: 0x25, count: 64)
        for k in 0 ..< files { try bytes.write(to: src.appendingPathComponent("f\(k).tex")) }
        try Data("999999 1 \(n)".utf8).write(to: base.appendingPathComponent("owner")) // an exited instance
        return base
    }

    private func remaining() -> [String] {
        let projects = EngineV3.cacheDirectory.appendingPathComponent("projects")
        return ((try? FileManager.default.contentsOfDirectory(atPath: projects.path)) ?? []).filter { $0.hasPrefix(Self.key) }
    }

    /// Enabling the engine launches a host; the launch's cleanup of 40
    /// exited copies (8,000 files) happens off main, and still happens: all
    /// but the project's newest copy (kept for its next take-over) go.
    func testALaunchRemovesAbandonedCopiesWithoutBlockingMain() async throws {
        try XCTSkipUnless(FileManager.default.isExecutableFile(atPath: "/usr/bin/false"), "no /usr/bin/false")
        env.set("FLASHTEX_HOST", "/usr/bin/false") // a host that exits at once: the launch path only
        func launch() -> (ShellModel, Double) {
            let model = ShellModel()
            model.autoCompile = false
            model.replaceProject(entryText: "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n", named: "main.tex")
            let t0 = Date()
            model.engineV3Enabled = true // launchHost, on main
            return (model, Date().timeIntervalSince(t0) * 1000)
        }
        // A first launch with nothing to clean pays the one-time costs.
        let (warm, warmMs) = launch()
        warm.engineV3Enabled = false
        EngineV3Session.cleanupQueue.sync {} // its cleanup is over
        for n in 0 ..< 40 { _ = try abandonedCopy(n, files: 200) }
        XCTAssertEqual(remaining().count, 40)
        let (model, ms) = launch()
        defer { model.engineV3Enabled = false }
        XCTAssertGreaterThanOrEqual(model.engineV3.hostStarts, 1, "the host was launched")
        // (before: 1,422 ms here on a loaded M5 Pro, the files removed on main; after: 4-102 ms)
        XCTAssertLessThan(ms, 400, String(format: "the launch held main %.0f ms (warm launch without copies: %.0f ms)", ms, warmMs))
        let end = Date().addingTimeInterval(180)
        while remaining().count > 1, Date() < end { try await Task.sleep(nanoseconds: 100_000_000) }
        XCTAssertEqual(remaining().count, 1, "the exited copies were removed (the newest one is kept): \(remaining().count) left")
        print(String(format: "EngineV3LaunchCleanupTests: launch held main %.1f ms (warm, no copies: %.1f ms)", ms, warmMs))
    }

    /// A session's copy is made on main (`compile`): with `settleLater` the
    /// take-over only claims there; the output check and the emptying of a
    /// copy whose output is not whole wait for `settleTakeOver` (the walk
    /// queue), and a claimed copy is never removed by a cleanup meanwhile.
    func testATakeOverOnlyClaimsOnMainAndSettlesLater() throws {
        let fm = FileManager.default
        let source = dir.appendingPathComponent("project", isDirectory: true)
        try fm.createDirectory(at: source, withIntermediateDirectories: true)
        let key = EngineV3Mirror.key(source)
        let base = EngineV3.cacheDirectory.appendingPathComponent("projects/\(key)-999998-0", isDirectory: true)
        let out = base.appendingPathComponent("out", isDirectory: true)
        try fm.createDirectory(at: out, withIntermediateDirectories: true)
        try fm.createDirectory(at: base.appendingPathComponent("src"), withIntermediateDirectories: true)
        for k in 0 ..< 2_000 { try Data("x".utf8).write(to: out.appendingPathComponent("o\(k).aux")) }
        try Data("999999 1 2".utf8).write(to: base.appendingPathComponent("owner")) // exited; no stamp: not whole
        let m = EngineV3Mirror(source: source, session: 9_400 + Int.random(in: 0 ..< 100), settleLater: true)
        defer { try? fm.removeItem(at: m.base) }
        XCTAssertTrue(m.takenOver)
        XCTAssertEqual(m.base.standardizedFileURL.path, base.standardizedFileURL.path)
        XCTAssertEqual(try String(contentsOf: base.appendingPathComponent("owner"), encoding: .utf8), EngineV3.instanceOwner)
        XCTAssertEqual(try fm.contentsOfDirectory(atPath: out.path).count, 2_000, "not settled on main")
        EngineV3Mirror.removeAbandoned(log: { _ in })
        XCTAssertTrue(fm.fileExists(atPath: base.path), "a claimed copy is this instance's")
        m.settleTakeOver()
        XCTAssertEqual(try fm.contentsOfDirectory(atPath: out.path), [], "output that is not whole is emptied")
        XCTAssertTrue(m.exists)
    }
}
