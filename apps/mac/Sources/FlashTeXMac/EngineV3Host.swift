import Foundation
import FlashTeXDisplayListV3

// The engine-v3 preview (DESIGN.md §3, §6.1–6.2, P3): the pdfLaTeX-compatible
// engine runs as its own process, `flashtex-host` (crates/flashtex-engine,
// GPL-2.0-or-later), and the app (MIT) talks to it only through the
// display-list-v3 socket protocol (FlashTeXDisplayListV3). Nothing here links
// engine code: the licence boundary is this process boundary.
//
// Behind a flag, default OFF (docs: apps/mac/docs/engine-v3-preview.md):
//   defaults write <bundle id or FlashTeXMac> FlashTeX.EngineV3.enabled -bool YES
//   or FLASHTEX_ENGINE_V3=1 in the environment (0 forces it off), or
//   View > Engine v3 Preview (Experimental).
// With the flag off nothing below runs and the old preview path is unchanged.

enum EngineV3 {
    static let enabledKey = "FlashTeX.EngineV3.enabled"
    static let hostPathKey = "FlashTeX.EngineV3.hostPath"

    /// The flag's stored value (environment first, then user defaults).
    /// Under XCTest the stored default is not read: a test that wants v3
    /// turns it on itself, and one that does not must not inherit whatever
    /// an earlier run left in the test runner's defaults (with v3 on the old
    /// engine compiles nothing, `ShellModel.suspendOldEngineForV3`).
    static var enabledAtLaunch: Bool {
        switch ProcessInfo.processInfo.environment["FLASHTEX_ENGINE_V3"] {
        case "1": return true
        case "0": return false
        default: return underTest ? false : UserDefaults.standard.bool(forKey: enabledKey)
        }
    }

    /// XCTest is loaded: `swift test` sets neither of the variables
    /// `ShellModel.runningUnderXCTest` looks for, so ask for its class too.
    static let underTest = ShellModel.runningUnderXCTest || NSClassFromString("XCTestCase") != nil

    /// Finds `flashtex-host`: `FLASHTEX_HOST`, the `FlashTeX.EngineV3.hostPath`
    /// default, the app bundle's helper (`Contents/Helpers/flashtex-host`, or
    /// beside the app executable), then a repository build
    /// (`target/release/flashtex-host`, then `target/debug`).
    /// `FLASHTEX_HOST=none` finds none.
    static func locateHost() -> URL? {
        let fm = FileManager.default
        var candidates: [String] = []
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_HOST"] {
            if env == "none" { return nil } // no host at all (tests of the app side alone)
            candidates.append(env)
        }
        if let d = UserDefaults.standard.string(forKey: hostPathKey) { candidates.append(d) }
        if let exe = Bundle.main.executableURL {
            candidates.append(exe.deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Helpers/flashtex-host").path)
            candidates.append(exe.deletingLastPathComponent().appendingPathComponent("flashtex-host").path)
        }
        for root in repoRoots() {
            candidates.append(root.appendingPathComponent("target/release/flashtex-host").path)
            candidates.append(root.appendingPathComponent("target/debug/flashtex-host").path)
        }
        return candidates.first { fm.isExecutableFile(atPath: $0) }.map { URL(fileURLWithPath: $0) }
    }

    /// The engine's string pool: beside the host (bundled), else the
    /// repository's `crates/flashtex-engine/pdftex.pool` (development).
    static func locatePool(host: URL) -> URL? {
        let fm = FileManager.default
        let beside = host.deletingLastPathComponent().appendingPathComponent("pdftex.pool")
        if fm.fileExists(atPath: beside.path) { return beside }
        // The app bundle (make-app.sh): Contents/Resources/engine/pdftex.pool.
        if let bundled = Bundle.main.resourceURL?.appendingPathComponent("engine/pdftex.pool"), fm.fileExists(atPath: bundled.path) { return bundled }
        for root in [host.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()] + repoRoots() {
            let p = root.appendingPathComponent("crates/flashtex-engine/pdftex.pool")
            if fm.fileExists(atPath: p.path) { return p }
        }
        return nil
    }

    static func repoRoots() -> [URL] {
        var out: [URL] = []
        var starts = [URL(fileURLWithPath: FileManager.default.currentDirectoryPath), Bundle.main.bundleURL, URL(fileURLWithPath: #filePath)]
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_REPO"] { starts.insert(URL(fileURLWithPath: env), at: 0) }
        for start in starts {
            var url = start
            for _ in 0 ..< 8 {
                if FileManager.default.fileExists(atPath: url.appendingPathComponent("crates/flashtex-engine").path) { out.append(url); break }
                let parent = url.deletingLastPathComponent()
                if parent == url { break }
                url = parent
            }
        }
        return out
    }

    /// `~/Library/Caches/FlashTeX/engine-v3`.
    /// `FLASHTEX_V3_CACHE`, else `~/Library/Caches/FlashTeX/engine-v3`.
    /// Benches and tests use their own root; project copies inside are
    /// per app instance anyway (EngineV3Mirror).
    static var cacheDirectory: URL {
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_V3_CACHE"], !env.isEmpty {
            return URL(fileURLWithPath: (env as NSString).expandingTildeInPath, isDirectory: true)
        }
        let base = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask).first ?? URL(fileURLWithPath: NSTemporaryDirectory())
        return base.appendingPathComponent("FlashTeX/engine-v3", isDirectory: true)
    }

    /// A process's start time (seconds, microseconds), nil when it is not running.
    static func processStart(_ pid: Int32) -> (Int, Int)? {
        var info = kinfo_proc()
        var size = MemoryLayout<kinfo_proc>.stride
        var mib: [Int32] = [CTL_KERN, KERN_PROC, KERN_PROC_PID, pid]
        guard sysctl(&mib, 4, &info, &size, nil, 0) == 0, size > 0, info.kp_proc.p_pid == pid else { return nil }
        let t = info.kp_proc.p_un.__p_starttime
        return (Int(t.tv_sec), Int(t.tv_usec))
    }

    /// This app instance: "pid start-sec start-usec" (a pid alone can be reused).
    static let instanceOwner: String = {
        let pid = getpid()
        let s = processStart(pid) ?? (0, 0)
        return "\(pid) \(s.0) \(s.1)"
    }()

    /// Whether the instance an owner string names is still running.
    static func ownerAlive(_ owner: String) -> Bool {
        let f = owner.split(separator: " ").compactMap { Int($0) }
        guard f.count == 3, let s = processStart(Int32(f[0])) else { return false }
        return s.0 == f[1] && s.1 == f[2]
    }
}

/// One `flashtex-host` process (spec §6.1): started with a per-process
/// socket, it prepares the format from the user's TeX Live (the first use
/// builds it, a few seconds), warms up, prints what it chose, then listens.
final class EngineV3HostProcess: @unchecked Sendable {
    enum Event: Sendable {
        /// A line the host printed (stdout or stderr), for the log.
        case line(String)
        /// The start-up summary: `texmf` (which TeX Live, format status), `warm_ms`.
        case prepared(DL3JSON)
        case listening(socket: String)
        case exited(pid: Int32, status: Int32)
    }

    let executable: URL
    let socketPath: String
    private let process = Process()
    private var buffer = Data()
    private let lock = NSLock()

    init(executable: URL, onEvent: @escaping @Sendable (Event) -> Void) throws {
        self.executable = executable
        let dir = NSTemporaryDirectory()
        socketPath = (dir as NSString).appendingPathComponent("ftx-\(getpid())-\(UInt32.random(in: 0 ... .max)).sock")
        let s0 = EngineV3.cacheDirectory.appendingPathComponent("s0", isDirectory: true)
        try? FileManager.default.createDirectory(at: s0, withIntermediateDirectories: true)
        process.executableURL = executable
        // --once: serve one connection, then exit. The app's connection closes
        // when the app quits or dies (the kernel closes the socket), so the
        // host never outlives it once connected.
        process.arguments = ["--socket", socketPath, "--s0-cache", s0.path, "--once"]
        // Checkpoint interval inside a page (engine default 0.02 s): the
        // restart re-typesets up to that much before an edit. A/B knob.
        if let t = ProcessInfo.processInfo.environment["FLASHTEX_V3_TIMED"], Double(t) != nil { process.arguments! += ["--timed", t] }
        var env = ProcessInfo.processInfo.environment
        if env["FLASHTEX_POOL"] == nil, let pool = EngineV3.locatePool(host: executable) { env["FLASHTEX_POOL"] = pool.path }
        process.environment = env
        let out = Pipe(), err = Pipe()
        process.standardOutput = out
        process.standardError = err
        process.standardInput = FileHandle.nullDevice
        let socketPath = self.socketPath
        let handle: @Sendable (Data) -> Void = { [weak self] data in
            guard let self else { return }
            self.lock.lock()
            self.buffer.append(data)
            var lines: [String] = []
            while let nl = self.buffer.firstIndex(of: 0x0A) {
                lines.append(String(decoding: self.buffer[self.buffer.startIndex ..< nl], as: UTF8.self))
                self.buffer.removeSubrange(self.buffer.startIndex ... nl)
            }
            self.lock.unlock()
            for line in lines {
                onEvent(.line(line))
                if line.hasPrefix("flashtex-host: listening on") {
                    onEvent(.listening(socket: socketPath))
                } else if line.hasPrefix("flashtex-host: {"), let j = try? DL3JSON.parse(Array(line.dropFirst("flashtex-host: ".count).utf8)),
                          j["texmf"] != nil || j["formats"] != nil || j["warm_ms"] != nil {
                    onEvent(.prepared(j))
                }
            }
        }
        out.fileHandleForReading.readabilityHandler = { h in let d = h.availableData; if !d.isEmpty { handle(d) } }
        err.fileHandleForReading.readabilityHandler = { h in
            let d = h.availableData
            if !d.isEmpty { onEvent(.line("stderr: " + String(decoding: d, as: UTF8.self).trimmingCharacters(in: .newlines))) }
        }
        process.terminationHandler = { p in
            out.fileHandleForReading.readabilityHandler = nil
            err.fileHandleForReading.readabilityHandler = nil
            Self.forget(pid: p.processIdentifier)
            onEvent(.exited(pid: p.processIdentifier, status: p.terminationStatus))
        }
        try process.run()
        Self.remember(pid: process.processIdentifier)
    }

    // MARK: stale hosts (the app died before its host connected)

    static var pidDirectory: URL { EngineV3.cacheDirectory.appendingPathComponent("hosts", isDirectory: true) }

    static func remember(pid: Int32) {
        try? FileManager.default.createDirectory(at: pidDirectory, withIntermediateDirectories: true)
        try? Data("\(getpid())".utf8).write(to: pidDirectory.appendingPathComponent("\(pid)"))
    }

    static func forget(pid: Int32) { try? FileManager.default.removeItem(at: pidDirectory.appendingPathComponent("\(pid)")) }

    /// Kills hosts started by an app process that no longer runs (their pid
    /// files name the app's pid); only processes whose executable is a
    /// `flashtex-host`.
    static func killStaleHosts(log: (String) -> Void) {
        guard let names = try? FileManager.default.contentsOfDirectory(atPath: pidDirectory.path) else { return }
        for name in names {
            guard let pid = Int32(name) else { continue }
            let file = pidDirectory.appendingPathComponent(name)
            let owner = (try? String(contentsOf: file, encoding: .utf8)).flatMap { Int32($0) } ?? 0
            if owner != 0, owner != getpid(), kill(owner, 0) == 0 { continue } // its app is alive
            if owner == getpid() { continue }
            var buf = [CChar](repeating: 0, count: 4096)
            if proc_pidpath(pid, &buf, UInt32(buf.count)) > 0, String(cString: buf).hasSuffix("/flashtex-host") {
                kill(pid, SIGTERM)
                log("killed stale host \(pid) (its app \(owner) is gone)")
            }
            try? FileManager.default.removeItem(at: file)
        }
    }

    var pid: Int32 { process.processIdentifier }
    var isRunning: Bool { process.isRunning }

    func terminate() {
        if process.isRunning { process.terminate() }
        unlink(socketPath) // the host removes it itself when it exits normally
    }

    deinit { terminate() }
}
