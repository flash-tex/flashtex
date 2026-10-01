import Foundation
import XCTest
@testable import FlashTeXMac

/// Project copies and hosts belong to one session of one app instance.
/// Regression for the owner's try-out: a bench started later deleted the
/// project copy a running app's host was compiling in, and that preview
/// stopped updating without a word.
@MainActor
final class EngineV3InstanceTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-instances-\(getpid())")
    override func setUp() { setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1) }
    override func tearDown() { unsetenv("FLASHTEX_V3_CACHE") }

    func waitUntil(_ what: String, timeout: TimeInterval = 90, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    func project(_ name: String) throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let file = dir.appendingPathComponent("paper.tex")
        try "\\documentclass{article}\n\\begin{document}\nOne page of text.\n\\end{document}\n".write(to: file, atomically: true, encoding: .utf8)
        return file
    }

    func session(opening file: URL) async throws -> ShellModel {
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: file, dirty: .discard), .opened)
        model.engineV3Enabled = true
        model.engineV3.start(model: model)
        let s = model.engineV3
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the first compile") { s.statusNote.hasPrefix("ok") && s.pageCount == 1 }
        return model
    }

    func edit(_ model: ShellModel, _ from: String, _ to: String) async throws {
        let s = model.engineV3
        model.updateActiveText(model.activeText.replacingOccurrences(of: from, with: to))
        try await waitUntil("the edit compiled") {
            !s.compiling && s.statusNote.hasPrefix("ok") && s.staleCount == 0
                && ((try? String(contentsOf: s.projectCopy!.appendingPathComponent("paper.tex"), encoding: .utf8))?.contains(to) ?? false)
        }
    }

    /// Two windows (sessions) with the same project: two copies, two hosts;
    /// one stopping leaves the other compiling.
    func testTwoConcurrentSessionsKeepTheirOwnCopies() async throws {
        try EngineV3TestHost.require()
        let stored = UserDefaults.standard.object(forKey: EngineV3.enabledKey)
        defer { if let stored { UserDefaults.standard.set(stored, forKey: EngineV3.enabledKey) } else { UserDefaults.standard.removeObject(forKey: EngineV3.enabledKey) } }
        let file = try project("two")
        let a = try await session(opening: file)
        let b = try await session(opening: file)
        defer { a.engineV3.stop(); b.engineV3.stop() }
        let copyA = try XCTUnwrap(a.engineV3.projectCopy), copyB = try XCTUnwrap(b.engineV3.projectCopy)
        XCTAssertNotEqual(copyA, copyB, "each session compiles its own copy")
        XCTAssertNotEqual(a.engineV3.hostPID, b.engineV3.hostPID, "each session has its own host")
        try await edit(a, "One page", "A's page")
        try await edit(b, "One page", "B's page")
        XCTAssertFalse(try String(contentsOf: copyA.appendingPathComponent("paper.tex"), encoding: .utf8).contains("B's"))
        // A new session start (as a second app or a bench would) cleans up
        // only copies of exited instances: both copies stay.
        EngineV3Mirror.removeAbandoned(log: { _ in })
        XCTAssertTrue(FileManager.default.fileExists(atPath: copyA.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: copyB.path))
        a.engineV3.stop()
        try await edit(b, "B's page", "B's second page")
    }

    /// Something removes the copy under a running host: the next edit
    /// re-creates it, starts a fresh host and compiles again.
    func testAVanishedCopyIsRecreated() async throws {
        try EngineV3TestHost.require()
        let stored = UserDefaults.standard.object(forKey: EngineV3.enabledKey)
        defer { if let stored { UserDefaults.standard.set(stored, forKey: EngineV3.enabledKey) } else { UserDefaults.standard.removeObject(forKey: EngineV3.enabledKey) } }
        let model = try await session(opening: try project("vanish"))
        defer { model.engineV3.stop() }
        let s = model.engineV3
        let copy = try XCTUnwrap(s.projectCopy)
        let starts = s.hostStarts
        try FileManager.default.removeItem(at: copy.deletingLastPathComponent())
        model.updateActiveText(model.activeText.replacingOccurrences(of: "One page", with: "Still here"))
        try await waitUntil("a new host") { s.hostStarts == starts + 1 && s.phase == .ready }
        try await waitUntil("the recompile") {
            !s.compiling && s.statusNote.hasPrefix("ok")
                && ((try? String(contentsOf: copy.appendingPathComponent("paper.tex"), encoding: .utf8))?.contains("Still here") ?? false)
        }
        XCTAssertEqual(s.pageCount, 1)
    }

    /// Cleanup removes a copy only when its owner file names an instance
    /// that has exited; a live owner's copy and an owner-less copy (an
    /// older build's) stay.
    func testCleanupRemovesOnlyExitedOwnersCopies() throws {
        let projects = EngineV3.cacheDirectory.appendingPathComponent("projects")
        let fm = FileManager.default
        func make(_ name: String, owner: String?) throws -> URL {
            let d = projects.appendingPathComponent(name)
            try fm.createDirectory(at: d.appendingPathComponent("src"), withIntermediateDirectories: true)
            if let owner { try Data(owner.utf8).write(to: d.appendingPathComponent("owner")) }
            return d
        }
        let launchd = EngineV3.processStart(1).map { "1 \($0.0) \($0.1)" } ?? "1 0 0"
        let dead = try make("dead-owner", owner: "999999 1 2")
        let reused = try make("reused-pid", owner: "1 1 2") // pid 1 runs, but started at another time
        let alive = try make("live-owner", owner: launchd)
        let mine = try make("this-instance", owner: EngineV3.instanceOwner)
        let legacy = try make("no-owner", owner: nil)
        EngineV3Mirror.removeAbandoned(log: { _ in })
        XCTAssertFalse(fm.fileExists(atPath: dead.path))
        XCTAssertFalse(fm.fileExists(atPath: reused.path))
        XCTAssertTrue(fm.fileExists(atPath: alive.path))
        XCTAssertTrue(fm.fileExists(atPath: mine.path))
        XCTAssertTrue(fm.fileExists(atPath: legacy.path))
        for d in [alive, mine, legacy] { try? fm.removeItem(at: d) }
    }
}
