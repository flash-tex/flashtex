import Foundation
import XCTest
@testable import FlashTeXMac

/// Project copies and hosts belong to one session of one app instance.
/// Regression for the owner's try-out: a bench started later deleted the
/// project copy a running app's host was compiling in, and that preview
/// stopped updating without a word.
@MainActor
final class EngineV3InstanceTests: XCTestCase {
    /// Environment set for a test and put back after it (never just unset).
    private var env = EnvironmentOverride()
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-instances-\(getpid())")
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

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
        // The first compile's DONE stamps the copy (`markComplete`, on the
        // walk queue): removing the copy while the stamp is written races it
        // (the removal fails), so the copy vanishes once it is stamped.
        let base = copy.deletingLastPathComponent()
        try await waitUntil("the stamp") { FileManager.default.fileExists(atPath: base.appendingPathComponent(EngineV3Mirror.stampName).path) }
        try FileManager.default.removeItem(at: base)
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

    /// An exited copy whose last compile ended (`markComplete`), `age`
    /// seconds ago by its stamp.
    func exitedCopy(key: String, pid: Int, source: URL, aux: String, age: TimeInterval) throws -> URL {
        let fm = FileManager.default
        let d = EngineV3.cacheDirectory.appendingPathComponent("projects/\(key)-\(pid)-0")
        let out = d.appendingPathComponent("out")
        try fm.createDirectory(at: out, withIntermediateDirectories: true)
        try fm.createDirectory(at: d.appendingPathComponent("src"), withIntermediateDirectories: true)
        try Data("999999 1 2".utf8).write(to: d.appendingPathComponent("owner"))
        try Data(aux.utf8).write(to: out.appendingPathComponent("paper.aux"))
        EngineV3Mirror.markComplete(base: d, source: source)
        let stamp = d.appendingPathComponent(EngineV3Mirror.stampName)
        var j = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: stamp)) as? [String: Any])
        j["time"] = Date().timeIntervalSince1970 - age
        try JSONSerialization.data(withJSONObject: j).write(to: stamp)
        return d
    }

    /// The project's newest exited copy is kept (an older one is removed)
    /// and the project's next copy takes it over whole: the same paths
    /// (the host's persisted S₀ is keyed by them) and its output (`.aux`,
    /// `.toc`, ...), so a reopened document compiles once from S₀. A second
    /// copy at the same time starts anew; a kept copy goes after
    /// `keptCopyDays` (by its stamp).
    func testTheProjectsLastCopyIsTakenOverByTheNextOne() throws {
        let fm = FileManager.default
        let source = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-carry-\(UUID().uuidString)")
        try fm.createDirectory(at: source, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: source) }
        let key = EngineV3Mirror.key(source)
        let older = try exitedCopy(key: key, pid: 999_991, source: source, aux: "older", age: 600)
        let newer = try exitedCopy(key: key, pid: 999_992, source: source, aux: "newer", age: 60)
        EngineV3Mirror.removeAbandoned(log: { _ in })
        XCTAssertFalse(fm.fileExists(atPath: older.path))
        XCTAssertTrue(fm.fileExists(atPath: newer.path))
        let first = EngineV3Mirror(source: source, session: 9_100 + Int.random(in: 0 ..< 100))
        let second = EngineV3Mirror(source: source, session: 9_200 + Int.random(in: 0 ..< 100))
        defer { for m in [first, second] { try? fm.removeItem(at: m.base) } }
        XCTAssertTrue(first.takenOver)
        XCTAssertEqual(first.base.standardizedFileURL.path, newer.standardizedFileURL.path, "the same paths")
        XCTAssertEqual(try String(contentsOf: first.output.appendingPathComponent("paper.aux"), encoding: .utf8), "newer")
        XCTAssertEqual(try String(contentsOf: first.base.appendingPathComponent("owner"), encoding: .utf8), EngineV3.instanceOwner)
        XCTAssertFalse(second.takenOver)
        XCTAssertNotEqual(second.base.standardizedFileURL.path, first.base.standardizedFileURL.path)
        XCTAssertFalse(fm.fileExists(atPath: second.output.appendingPathComponent("paper.aux").path))
        // this instance's copies stay; a project's last copy goes once it is too old
        EngineV3Mirror.removeAbandoned(log: { _ in })
        XCTAssertTrue(fm.fileExists(atPath: first.base.path))
        let stale = try exitedCopy(key: key, pid: 999_993, source: source, aux: "stale", age: Double(EngineV3Mirror.keptCopyDays + 1) * 86_400)
        EngineV3Mirror.removeAbandoned(log: { _ in })
        XCTAssertFalse(fm.fileExists(atPath: stale.path))
    }

    /// A copy whose output is not as its last compile left it (a run killed
    /// midway rewrote the `.aux`), or that was made for another project
    /// folder at the same path, is taken over without its output.
    func testATakenOverCopyKeepsItsOutputOnlyWhenItIsWhole() throws {
        let fm = FileManager.default
        for damage in ["rewritten", "another project"] {
            let source = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-whole-\(UUID().uuidString)")
            try fm.createDirectory(at: source, withIntermediateDirectories: true)
            defer { try? fm.removeItem(at: source) }
            let key = EngineV3Mirror.key(source)
            let d = try exitedCopy(key: key, pid: 999_994, source: source, aux: "whole", age: 60)
            if damage == "rewritten" {
                try Data("\\relax\n\\newlab".utf8).write(to: d.appendingPathComponent("out/paper.aux"))
            } else {
                try fm.removeItem(at: source)
                try fm.createDirectory(at: source, withIntermediateDirectories: true)
            }
            let m = EngineV3Mirror(source: source, session: 9_300 + Int.random(in: 0 ..< 100))
            defer { try? fm.removeItem(at: m.base) }
            XCTAssertTrue(m.takenOver, damage)
            XCTAssertFalse(fm.fileExists(atPath: m.output.appendingPathComponent("paper.aux").path), damage)
        }
    }

    /// Through the app's own path (open, start, compile): a document whose
    /// first launch needed more than one pass (its `.aux` appeared) compiles
    /// in one pass after a relaunch, from the copy its last instance left.
    func testARelaunchCompilesFromTheLastCopysAux() async throws {
        try EngineV3TestHost.require()
        let stored = UserDefaults.standard.object(forKey: EngineV3.enabledKey)
        defer { if let stored { UserDefaults.standard.set(stored, forKey: EngineV3.enabledKey) } else { UserDefaults.standard.removeObject(forKey: EngineV3.enabledKey) } }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-relaunch-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let file = dir.appendingPathComponent("paper.tex")
        try "\\documentclass{article}\n\\begin{document}\n\\section{One}\\label{one}\nSee section~\\ref{one} on page~\\pageref{one}.\n\\end{document}\n"
            .write(to: file, atomically: true, encoding: .utf8)
        let a = try await session(opening: file)
        try await waitUntil("the first launch's passes") { (a.engineV3.lastDone?["passes"]?.int ?? 0) >= 2 }
        let copy = try XCTUnwrap(a.engineV3.projectCopy).deletingLastPathComponent()
        try await waitUntil("the stamp") { FileManager.default.fileExists(atPath: copy.appendingPathComponent(EngineV3Mirror.stampName).path) }
        a.engineV3.stop()
        // the instance exits (another instance's copy now)
        try Data("999999 1 2".utf8).write(to: copy.appendingPathComponent("owner"))
        let b = try await session(opening: file)
        defer { b.engineV3.stop() }
        XCTAssertEqual(try XCTUnwrap(b.engineV3.projectCopy).deletingLastPathComponent().standardizedFileURL.path,
                       copy.standardizedFileURL.path, "the last copy, taken over")
        let done = try XCTUnwrap(b.engineV3.lastDone)
        XCTAssertLessThanOrEqual(done["passes"]?.int ?? 0, 1, "\(done)")
    }

    /// Two instances claiming one exited copy: one rename of its owner file
    /// succeeds; a claim that finds a live owner puts it back.
    func testOnlyOneClaimOfAnExitedCopySucceeds() throws {
        let fm = FileManager.default
        let d = EngineV3.cacheDirectory.appendingPathComponent("projects/claim-\(UUID().uuidString)")
        try fm.createDirectory(at: d, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: d) }
        try Data("999999 1 2".utf8).write(to: d.appendingPathComponent("owner"))
        XCTAssertTrue(EngineV3Mirror.claim(d, from: "999999 1 2"))
        XCTAssertFalse(EngineV3Mirror.claim(d, from: "999999 1 2"), "the owner is this instance now")
        XCTAssertEqual(try String(contentsOf: d.appendingPathComponent("owner"), encoding: .utf8), EngineV3.instanceOwner)
    }
}
