import Foundation
import XCTest
@testable import FlashTeXMac

/// `killStaleHosts` and pid reuse (review lane REVIEW-APP): a pid file names
/// a host and the app that started it, but pids are reused. A file whose
/// host died unseen (the app crashed) must never get a later, unrelated
/// `flashtex-host` with that pid signalled, and an app pid taken by some
/// later process must not keep a dead app's hosts alive.
@MainActor
final class EngineV3StaleHostTests: XCTestCase {
    private var env = EnvironmentOverride()
    private var dir: URL!
    private var children: [Process] = []

    override func setUpWithError() throws {
        OwnerStateGuard.install()
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("stale-host-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        env.set("FLASHTEX_V3_CACHE", dir.appendingPathComponent("cache").path)
        try FileManager.default.createDirectory(at: EngineV3HostProcess.pidDirectory, withIntermediateDirectories: true)
    }

    override func tearDown() {
        for p in children where p.isRunning { p.terminate() }
        env.restore()
        try? FileManager.default.removeItem(at: dir)
    }

    /// A process whose executable path ends in `/flashtex-host`: a re-signed
    /// copy of sleep (a plain copy of a platform binary is not allowed to run).
    private func fakeHost() throws -> Process {
        let exe = dir.appendingPathComponent("bin/flashtex-host")
        if !FileManager.default.fileExists(atPath: exe.path) {
            try FileManager.default.createDirectory(at: exe.deletingLastPathComponent(), withIntermediateDirectories: true)
            try FileManager.default.copyItem(at: URL(fileURLWithPath: "/bin/sleep"), to: exe)
            let sign = Process()
            sign.executableURL = URL(fileURLWithPath: "/usr/bin/codesign")
            sign.arguments = ["-s", "-", "-f", exe.path]
            sign.standardError = FileHandle.nullDevice
            try sign.run()
            sign.waitUntilExit()
            try XCTSkipUnless(sign.terminationStatus == 0, "codesign failed")
        }
        return try spawn(exe)
    }

    private func spawn(_ exe: URL) throws -> Process {
        let p = Process()
        p.executableURL = exe
        p.arguments = ["60"]
        try p.run()
        children.append(p)
        return p
    }

    /// A pid that names no process.
    private func deadPid() throws -> Int32 {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/true")
        try p.run()
        p.waitUntilExit()
        XCTAssertNil(EngineV3.processStart(p.processIdentifier))
        return p.processIdentifier
    }

    private func pidFile(host: Int32, owner: Int32, written: Date) throws {
        let file = EngineV3HostProcess.pidDirectory.appendingPathComponent("\(host)")
        try Data("\(owner)".utf8).write(to: file)
        try FileManager.default.setAttributes([.modificationDate: written], ofItemAtPath: file.path)
    }

    private func gone(_ p: Process, within seconds: Double = 20) async -> Bool {
        let end = Date().addingTimeInterval(seconds)
        while Date() < end {
            if !p.isRunning { return true }
            try? await Task.sleep(nanoseconds: 50_000_000)
        }
        return !p.isRunning
    }

    /// A dead app left a pid file whose host died with it; the pid now names
    /// another `flashtex-host`, started after the file was written. It is
    /// not signalled, and the stale file goes.
    func testAHostPidReusedByALaterFlashtexHostIsNotKilled() async throws {
        let owner = try deadPid()
        let other = try fakeHost()
        try pidFile(host: other.processIdentifier, owner: owner, written: Date().addingTimeInterval(-3600))
        var said: [String] = []
        EngineV3HostProcess.killStaleHosts { said.append($0) }
        try? await Task.sleep(nanoseconds: 500_000_000)
        XCTAssertTrue(other.isRunning, "an unrelated flashtex-host was signalled: \(said)")
        XCTAssertTrue(said.isEmpty, "\(said)")
        XCTAssertFalse(FileManager.default.fileExists(atPath: EngineV3HostProcess.pidDirectory.appendingPathComponent("\(other.processIdentifier)").path))
    }

    /// The app that started a host is gone and its pid now names a later,
    /// unrelated process: the host is stale and is killed.
    func testAnOwnerPidReusedByALaterProcessDoesNotKeepItsHostAlive() async throws {
        let host = try fakeHost()
        let written = Date()
        try? await Task.sleep(nanoseconds: 100_000_000)
        let impostor = try spawn(URL(fileURLWithPath: "/bin/sleep")) // "the owner's pid", started after the file
        try pidFile(host: host.processIdentifier, owner: impostor.processIdentifier, written: written)
        var said: [String] = []
        EngineV3HostProcess.killStaleHosts { said.append($0) }
        let killed = await gone(host)
        XCTAssertTrue(killed, "a dead app's host was kept alive by a reused owner pid")
        XCTAssertTrue(impostor.isRunning)
        XCTAssertEqual(said.count, 1, "\(said)")
    }

    /// The ordinary cases stay: a dead app's host is killed; a live app's
    /// (this one's, written after this process started) is not.
    func testAStaleHostIsKilledAndALiveAppsIsKept() async throws {
        let stale = try fakeHost(), live = try fakeHost()
        try? await Task.sleep(nanoseconds: 100_000_000)
        try pidFile(host: stale.processIdentifier, owner: try deadPid(), written: Date())
        try pidFile(host: live.processIdentifier, owner: getpid(), written: Date())
        EngineV3HostProcess.killStaleHosts { _ in }
        let killed = await gone(stale)
        XCTAssertTrue(killed)
        XCTAssertTrue(live.isRunning)
        XCTAssertTrue(FileManager.default.fileExists(atPath: EngineV3HostProcess.pidDirectory.appendingPathComponent("\(live.processIdentifier)").path))
    }

    /// A host that exits at once (before `run` returned to write its pid
    /// file, at times) leaves no pid file behind.
    func testAHostThatExitsAtOnceLeavesNoPidFile() async throws {
        let exe = URL(fileURLWithPath: "/usr/bin/true")
        for _ in 0 ..< 20 {
            let done = expectation(description: "exit")
            let h = try EngineV3HostProcess(executable: exe) { if case .exited = $0 { done.fulfill() } }
            await fulfillment(of: [done], timeout: 30)
            let names = (try? FileManager.default.contentsOfDirectory(atPath: EngineV3HostProcess.pidDirectory.path)) ?? []
            XCTAssertTrue(names.isEmpty, "stray pid files \(names) after \(h.socketPath)")
            for n in names { try? FileManager.default.removeItem(at: EngineV3HostProcess.pidDirectory.appendingPathComponent(n)) }
        }
    }
}
