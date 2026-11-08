import Foundation
import FlashTeXDisplayListV3

// The engine-v3 preview (DESIGN.md §3, §6.1–6.2, P3): the pdfLaTeX-compatible
// engine runs as its own process, `flashtex-host` (crates/flashtex-engine,
// GPL-2.0-or-later), and the app (MIT) talks to it only through the
// display-list-v3 socket protocol (FlashTeXDisplayListV3). Nothing here links
// engine code: the licence boundary is this process boundary.
//
// The one engine since the old one retired (#1236 S6; docs:
// apps/mac/docs/engine-v3-preview.md). `FLASHTEX_ENGINE_V3=0` keeps a
// window's engine session off (an editor only); `1` turns it on under XCTest.

enum EngineV3 {
    static let hostPathKey = "FlashTeX.EngineV3.hostPath"

    /// Whether a window's engine session runs (`ShellModel.engineV3Enabled`):
    /// on, unless `FLASHTEX_ENGINE_V3=0`. Under XCTest it is off unless
    /// `FLASHTEX_ENGINE_V3=1`, so a model made for a test starts no host
    /// process; a test that wants the engine turns it on itself. Outside
    /// XCTest the first call also removes the defaults this build no longer
    /// reads (EngineChoiceStore.removeRetiredDefaults; the retirement plan's §5.5).
    @MainActor static var enabledAtLaunch: Bool {
        switch ProcessInfo.processInfo.environment["FLASHTEX_ENGINE_V3"] {
        case "1": return true
        case "0": return false
        default:
            if !underTest { EngineChoiceStore.removeRetiredDefaults(in: defaults) }
            return !underTest
        }
    }

    /// XCTest is loaded: `swift test` sets neither of the variables
    /// `ShellModel.runningUnderXCTest` looks for, so ask for its class too.
    static let underTest = ShellModel.runningUnderXCTest || NSClassFromString("XCTestCase") != nil

    /// Where the flag and the host path are kept: the app's defaults, or,
    /// under XCTest, a suite of this test process only, so a test never
    /// writes the runner's or the app's domain (and a test that crashes
    /// leaves nothing behind for the next run). The suite's plist is removed
    /// when the test process ends (`removeTestDefaults`).
    static let defaults: UserDefaults = underTest ? (UserDefaults(suiteName: testDefaultsSuite) ?? .standard) : .standard
    static let testDefaultsSuite = "tech.flashtex.tests.engine-v3.\(getpid())"
    static func removeTestDefaults() {
        guard underTest else { return }
        defaults.removePersistentDomain(forName: testDefaultsSuite)
    }

    /// Finds the mode's host (EngineV3Mode.swift), `flashtex-host` for
    /// Classic: `FLASHTEX_HOST`, the `FlashTeX.EngineV3.hostPath` default,
    /// the app bundle's helper (`Contents/Helpers/flashtex-host`, or beside
    /// the app executable), then a repository build
    /// (`target/release/flashtex-host`, then `target/debug`).
    /// `FLASHTEX_HOST=none` finds none. For Unicode, `flashtex-host-unicode`
    /// the same way, from `FLASHTEX_HOST_UNICODE` and the
    /// `FlashTeX.EngineV3.hostPath.unicode` default, and among the
    /// repository builds also the xetex crate's own target
    /// (`crates/flashtex-xetex/target/release`, it is its own workspace).
    static func locateHost(mode: EngineV3Mode = .classic) -> URL? {
        let fm = FileManager.default
        let name = mode.hostName
        var candidates: [String] = []
        if let env = ProcessInfo.processInfo.environment[mode == .unicode ? "FLASHTEX_HOST_UNICODE" : "FLASHTEX_HOST"] {
            if env == "none" { return nil } // no host at all (tests of the app side alone)
            candidates.append(env)
        }
        if let d = defaults.string(forKey: mode == .unicode ? hostPathKey + ".unicode" : hostPathKey) { candidates.append(d) }
        if let exe = Bundle.main.executableURL {
            candidates.append(exe.deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Helpers/\(name)").path)
            candidates.append(exe.deletingLastPathComponent().appendingPathComponent(name).path)
        }
        for root in repoRoots() {
            candidates.append(root.appendingPathComponent("target/release/\(name)").path)
            if mode == .unicode { candidates.append(root.appendingPathComponent("crates/flashtex-xetex/target/release/\(name)").path) }
            candidates.append(root.appendingPathComponent("target/debug/\(name)").path)
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

    /// `FLASHTEX_V3_CACHE`, else, under XCTest, a directory of this test
    /// process in the temporary directory (never the app's cache, whatever a
    /// test's tearDown did to the environment), else
    /// `~/Library/Caches/FlashTeX/engine-v3`. Benches and tests use their own
    /// root; project copies inside are per app instance anyway (EngineV3Mirror).
    static var cacheDirectory: URL {
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_V3_CACHE"], !env.isEmpty {
            return URL(fileURLWithPath: (env as NSString).expandingTildeInPath, isDirectory: true)
        }
        if underTest { return testCacheDirectory }
        let base = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask).first ?? URL(fileURLWithPath: NSTemporaryDirectory())
        return base.appendingPathComponent("FlashTeX/engine-v3", isDirectory: true)
    }

    /// The cache of a test process that names none.
    static let testCacheDirectory = URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
        .appendingPathComponent("flashtex-engine-v3-tests-\(getpid())", isDirectory: true)

    /// Whether the cache is not the app's own (`FLASHTEX_V3_CACHE`, a test):
    /// then the host's format cache moves with it.
    static var cacheIsPrivate: Bool {
        !(ProcessInfo.processInfo.environment["FLASHTEX_V3_CACHE"] ?? "").isEmpty || underTest
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
        /// A bundle fetch's progress (`bundle_progress`, protocol §6.2):
        /// `what` is `index`, `core` or `file`; a step ends with done = total.
        case bundleProgress(what: String, name: String, done: Int64, total: Int64)
    }

    let executable: URL
    let socketPath: String
    /// The mode this host typesets (its program) and the format its
    /// compiles ask for.
    let mode: EngineV3Mode
    let format: String
    private let process = Process()
    private var buffer = Data()
    private let lock = NSLock()

    /// `confineRoots`: the host compiles text from a Live Share session and
    /// may read only inside these folders besides the job's own
    /// (`confinedEnvironment`); nil for an ordinary host.
    init(executable: URL, mode: EngineV3Mode = .classic, format: String? = nil, confineRoots: [String]? = nil, onEvent: @escaping @Sendable (Event) -> Void) throws {
        self.executable = executable
        self.mode = mode
        self.format = format ?? mode.format
        self.confineRoots = confineRoots
        let dir = NSTemporaryDirectory()
        socketPath = (dir as NSString).appendingPathComponent("ftx-\(getpid())-\(UInt32.random(in: 0 ... .max)).sock")
        let s0 = EngineV3.cacheDirectory.appendingPathComponent("s0", isDirectory: true)
        try? FileManager.default.createDirectory(at: s0, withIntermediateDirectories: true)
        process.executableURL = executable
        // --once: serve one connection, then exit. The app's connection closes
        // when the app quits or dies (the kernel closes the socket), so the
        // host never outlives it once connected.
        process.arguments = ["--socket", socketPath, "--once"]
        // The resident pdfTeX host's S0 cache, performance mode and checkpoint
        // interval. The Unicode host compiles cold and takes none of them.
        if mode == .classic {
            process.arguments! += ["--s0-cache", s0.path]
            process.arguments! += ["--profile", PerformanceMode.current.hostProfile]
            if let t = ProcessInfo.processInfo.environment["FLASHTEX_V3_TIMED"], Double(t) != nil { process.arguments! += ["--timed", t] }
        }
        refusalsPath = socketPath + ".refused"
        unlink(refusalsPath)
        let refusals = refusalsPath
        process.environment = confineRoots.map {
            Self.confinedEnvironment(Self.environment(host: executable), roots: $0, refusals: refusals,
                                     searchInRoots: EngineV3Session.areUntrustedRoots($0))
        }
            ?? Self.environment(host: executable)
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
                // Either host program names itself (`flashtex-host-unicode: …`).
                let said = Self.hostLine(line)
                if said?.hasPrefix("listening on") == true {
                    onEvent(.listening(socket: socketPath))
                } else if let said, said.hasPrefix("{"), let j = try? DL3JSON.parse(Array(said.utf8)) {
                    if let p = j["bundle_progress"] {
                        onEvent(.bundleProgress(what: p["what"]?.string ?? "", name: p["name"]?.string ?? "",
                                                done: p["done"]?.int ?? 0, total: p["total"]?.int ?? 0))
                    } else if j["texmf"] != nil || j["formats"] != nil || j["warm_ms"] != nil {
                        onEvent(.prepared(j))
                    }
                }
            }
        }
        out.fileHandleForReading.readabilityHandler = { h in let d = h.availableData; if !d.isEmpty { handle(d) } }
        err.fileHandleForReading.readabilityHandler = { h in
            let d = h.availableData
            if !d.isEmpty { onEvent(.line("stderr: " + String(decoding: d, as: UTF8.self).trimmingCharacters(in: .newlines))) }
        }
        // The pid file is written after `run` and removed at the exit, which
        // may come first (a host that fails at once): under one lock, so a
        // host that already exited leaves no file naming its pid, which a
        // later process may reuse (`killStaleHosts`).
        let pidFile = PidFileState()
        process.terminationHandler = { p in
            out.fileHandleForReading.readabilityHandler = nil
            err.fileHandleForReading.readabilityHandler = nil
            pidFile.lock.lock()
            pidFile.exited = true
            Self.forget(pid: p.processIdentifier)
            pidFile.lock.unlock()
            onEvent(.exited(pid: p.processIdentifier, status: p.terminationStatus))
        }
        try process.run()
        pidFile.lock.lock()
        if !pidFile.exited { Self.remember(pid: process.processIdentifier) }
        pidFile.lock.unlock()
    }

    private final class PidFileState: @unchecked Sendable {
        let lock = NSLock()
        var exited = false
    }

    /// What a host printed after its name (`flashtex-host: ` or
    /// `flashtex-host-unicode: `), nil for any other line.
    static func hostLine(_ line: String) -> Substring? {
        for p in ["flashtex-host: ", "flashtex-host-unicode: "] where line.hasPrefix(p) { return line.dropFirst(p.count) }
        return nil
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
    ///
    /// Pids are reused, so a pid alone proves nothing: both the app and the
    /// host a file names started before the file was written (`remember`
    /// runs after `run`, in the running app). A process with that pid that
    /// started later is another one: an app pid taken by any later process
    /// does not keep its hosts alive (nor does this app's own pid, if a dead
    /// app had it before), and a host pid taken by a later `flashtex-host`
    /// (another app's, a bench's, a test's) is never signalled.
    static func killStaleHosts(log: (String) -> Void) {
        guard let names = try? FileManager.default.contentsOfDirectory(atPath: pidDirectory.path) else { return }
        for name in names {
            guard let pid = Int32(name) else { continue }
            let file = pidDirectory.appendingPathComponent(name)
            let owner = (try? String(contentsOf: file, encoding: .utf8)).flatMap { Int32($0) } ?? 0
            let written = (try? FileManager.default.attributesOfItem(atPath: file.path))?[.modificationDate] as? Date
            if owner != 0, startedBy(owner, written) { continue } // its app is alive
            var buf = [CChar](repeating: 0, count: 4096)
            if startedBy(pid, written), proc_pidpath(pid, &buf, UInt32(buf.count)) > 0,
               String(cString: buf).hasSuffix("/flashtex-host") || String(cString: buf).hasSuffix("/flashtex-host-unicode") {
                kill(pid, SIGTERM)
                log("killed stale host \(pid) (its app \(owner) is gone)")
            }
            try? FileManager.default.removeItem(at: file)
        }
    }

    /// Whether `pid` runs and started no later than `date` (nil: no date
    /// known, running is enough).
    static func startedBy(_ pid: Int32, _ date: Date?) -> Bool {
        guard let s = EngineV3.processStart(pid) else { return false }
        guard let date else { return true }
        return Double(s.0) + Double(s.1) / 1_000_000 <= date.timeIntervalSince1970
    }

    /// The host's environment: this process's, with the string pool and,
    /// for a private cache (`FLASHTEX_V3_CACHE`, a test), the format cache
    /// moved with it, so a bench or test host never reads or writes
    /// ~/Library/Caches/FlashTeX/formats.
    static func environment(host executable: URL) -> [String: String] {
        var env = ProcessInfo.processInfo.environment
        if env["FLASHTEX_POOL"] == nil, let pool = EngineV3.locatePool(host: executable) { env["FLASHTEX_POOL"] = pool.path }
        if (env["FLASHTEX_FORMAT_CACHE_DIR"] ?? "").isEmpty, EngineV3.cacheIsPrivate {
            env["FLASHTEX_FORMAT_CACHE_DIR"] = EngineV3.cacheDirectory.appendingPathComponent("formats", isDirectory: true).path
        }
        // The bundle (no TeX Live): the lock the app found, offline until the user agreed.
        EngineV3Bundle.hostEnvironment(&env, host: executable)
        // biber with no TeX Live: only the pinned one the user agreed to download.
        EngineV3Biber.hostEnvironment(&env, host: executable)
        return env
    }

    /// Launched with `confinedEnvironment` and these roots (nil: not).
    let confineRoots: [String]?
    var confined: Bool { confineRoots != nil }
    /// Where a confined host's engines list the reads they refused
    /// (`FLASHTEX_CONFINE_REFUSED`, one line each: `name` or
    /// `name<TAB>where it led`); beside the socket, removed with it.
    private(set) var refusalsPath = ""

    /// The reads this host refused so far, each once, in order (empty
    /// when it is not confined or refused nothing).
    func refusals() -> [String] { confined ? Self.readRefusals(at: refusalsPath) : [] }

    /// `refusals()` of the file at `path`: at most 64 KiB and 50 names are read.
    static func readRefusals(at path: String) -> [String] {
        guard let h = FileHandle(forReadingAtPath: path) else { return [] }
        defer { try? h.close() }
        let data = (try? h.read(upToCount: 64 * 1024)) ?? Data()
        var seen = Set<String>(), out: [String] = []
        for line in String(decoding: data, as: UTF8.self).split(separator: "\n") where !line.isEmpty {
            let l = String(line)
            if seen.insert(l).inserted { out.append(l) }
            if out.count == 50 { break }
        }
        return out
    }

    /// The environment of a host that compiles other people's text (a Live
    /// Share session, proposal §6.2): reads confined to names relative to
    /// the job or found along the search paths (`FLASHTEX_CONFINE_READS`, the
    /// engine's own check: kpathsea's paranoid `openin_any` does not confine
    /// reads), writes confined by kpathsea's paranoid `openout_any` (no
    /// absolute name, no `..`, no dotfile; relative names land in the
    /// output directory, which is the project copy's, never the project),
    /// with no `TEXMFOUTPUT` that would let an absolute name through. Every
    /// file the engine opens must also resolve, through links, into `roots`
    /// (the real project folder), the job's folder, the output folder or a
    /// TeX tree (`FLASHTEX_CONFINE_ROOTS`). No mktex script runs (a `\font`
    /// name must never start METAFONT). `refusals`: the file the engines
    /// list refused reads in (`FLASHTEX_CONFINE_REFUSED`). The same
    /// environment confines an untrusted project's host
    /// (`EngineV3Session.wantsUntrustedConfinement`), with `searchInRoots`
    /// (its roots name the TeX trees, `EngineV3Session.untrustedRoots`): a
    /// file found along the search paths must lie in a root too
    /// (`FLASHTEX_CONFINE_SEARCH=roots`), so `TEXINPUTS` and `TEXMFHOME`
    /// directories outside them are not read.
    static func confinedEnvironment(_ base: [String: String], roots: [String], refusals: String? = nil,
                                    searchInRoots: Bool = false) -> [String: String] {
        var env = base
        env["FLASHTEX_CONFINE_READS"] = "1"
        env.removeValue(forKey: "FLASHTEX_CONFINE_SEARCH")
        if searchInRoots { env["FLASHTEX_CONFINE_SEARCH"] = "roots" }
        env.removeValue(forKey: "FLASHTEX_CONFINE_REFUSED")
        if let refusals { env["FLASHTEX_CONFINE_REFUSED"] = refusals }
        env["FLASHTEX_CONFINE_ROOTS"] = confineRootList(roots)
        for k in ["MKTEXTFM", "MKTEXPK", "MKTEXMF", "MKTEXTEX", "MKOCP", "MKOFM"] { env[k] = "0" }
        env["openin_any"] = "p"
        env["openout_any"] = "p"
        env.removeValue(forKey: "TEXMFOUTPUT")
        for k in env.keys where k.hasPrefix("openin_any_") || k.hasPrefix("openout_any_") || k.hasPrefix("TEXMFOUTPUT_") {
            env.removeValue(forKey: k)
        }
        return env
    }

    /// `FLASHTEX_CONFINE_ROOTS`: the roots joined by `:`, each with `%` and
    /// `:` written `%25` and `%3A` (the engine's `confine_root_list`), so a
    /// folder named `~/Downloads/:paper` stays one root and never makes
    /// `~/Downloads` one (#2026 review).
    static func confineRootList(_ roots: [String]) -> String {
        roots.map { $0.replacingOccurrences(of: "%", with: "%25").replacingOccurrences(of: ":", with: "%3A") }
            .joined(separator: ":")
    }

    var pid: Int32 { process.processIdentifier }
    var isRunning: Bool { process.isRunning }

    /// Ends the host: the caller has said BYE (the host leaves on its own,
    /// its compile cancelled), and SIGTERM follows after `grace` should it
    /// still run. The engine of a Unicode host ends with the host however
    /// it ends (its lifeline, crates/flashtex-xetex/src/host/proc.rs).
    func terminate(grace: TimeInterval = 1) {
        let p = process
        if p.isRunning {
            if grace <= 0 {
                p.terminate()
            } else {
                DispatchQueue.global().asyncAfter(deadline: .now() + grace) { if p.isRunning { p.terminate() } }
            }
        }
        unlink(socketPath) // the host removes it itself when it exits normally
        if !refusalsPath.isEmpty { unlink(refusalsPath) }
    }

    deinit { terminate() }
}
