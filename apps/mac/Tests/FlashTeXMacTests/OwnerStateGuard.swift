import Foundation
import XCTest
@testable import FlashTeXMac

/// Fails the test during which the suite wrote the owner's state:
///
/// - the real `~/Library/Caches/FlashTeX`:
///   - a project copy or host pid file of this test process
///     (`engine-v3/projects/*-<pid>-*`, `engine-v3/hosts/*` naming this pid);
///   - a new page-snapshot directory (`engine-v3/snapshots/*`);
///   - any change to the format cache (`formats/*`);
/// - the engine-v3 keys (`FlashTeX.EngineV3.*`) of the standard defaults
///   domain, which under XCTest is the test runner's.
///
/// The owner's running app may write its own cache meanwhile (its s0
/// snapshots and its own snapshots, under keys of its projects), so those
/// are not compared. A new snapshot directory in the middle of a test run
/// counts as the suite's, since the app writes into the ones it already
/// has. Every engine-v3 test class installs the guard in `setUp`; it checks
/// after each test from then on, and removes the test process's defaults
/// suite (`EngineV3.removeTestDefaults`) when the bundle finishes.
final class OwnerStateGuard: NSObject, XCTestObservation {
    static let shared = OwnerStateGuard()
    private var installed = false
    private var baseline: State?

    /// Starts checking (once per process).
    static func install() {
        guard !shared.installed else { return }
        shared.installed = true
        shared.baseline = State.now()
        XCTestObservationCenter.shared.addTestObserver(shared)
    }

    /// The real cache root, whatever the environment says.
    static var realCache: URL {
        (FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Library/Caches"))
            .appendingPathComponent("FlashTeX", isDirectory: true)
    }

    struct State: Equatable {
        var ownProjectCopies: Set<String> = []
        var ownHostFiles: Set<String> = []
        var snapshotDirectories: Set<String> = []
        var formats: [String: Date] = [:]
        var runnerEngineV3Keys: [String: String] = [:]

        static func now() -> State {
            let fm = FileManager.default
            let v3 = realCache.appendingPathComponent("engine-v3", isDirectory: true)
            func names(_ dir: URL) -> [String] { (try? fm.contentsOfDirectory(atPath: dir.path)) ?? [] }
            var s = State()
            let pid = "\(getpid())"
            s.ownProjectCopies = Set(names(v3.appendingPathComponent("projects")).filter { $0.contains("-\(pid)-") })
            s.ownHostFiles = Set(names(v3.appendingPathComponent("hosts")).filter { name in
                (try? String(contentsOf: v3.appendingPathComponent("hosts/\(name)"), encoding: .utf8))?
                    .trimmingCharacters(in: .whitespacesAndNewlines) == pid
            })
            s.snapshotDirectories = Set(names(v3.appendingPathComponent("snapshots")))
            let formats = realCache.appendingPathComponent("formats", isDirectory: true)
            for n in names(formats) {
                let attrs = try? fm.attributesOfItem(atPath: formats.appendingPathComponent(n).path)
                s.formats[n] = attrs?[.modificationDate] as? Date ?? .distantPast
            }
            for (k, v) in UserDefaults.standard.dictionaryRepresentation() where k.hasPrefix("FlashTeX.EngineV3") {
                s.runnerEngineV3Keys[k] = String(describing: v)
            }
            return s
        }

        /// What `self` (later) has that `before` did not.
        func written(since before: State) -> [String] {
            var out: [String] = []
            for p in ownProjectCopies.subtracting(before.ownProjectCopies).sorted() { out.append("project copy engine-v3/projects/\(p)") }
            for h in ownHostFiles.subtracting(before.ownHostFiles).sorted() { out.append("host pid file engine-v3/hosts/\(h)") }
            for d in snapshotDirectories.subtracting(before.snapshotDirectories).sorted() { out.append("page snapshot engine-v3/snapshots/\(d)") }
            if formats != before.formats { out.append("format cache formats/ (\(before.formats.count) → \(formats.count) entries, or a newer one)") }
            if runnerEngineV3Keys != before.runnerEngineV3Keys { out.append("defaults \(runnerEngineV3Keys) (was \(before.runnerEngineV3Keys))") }
            return out
        }
    }

    func testCaseDidFinish(_ testCase: XCTestCase) {
        guard let before = baseline else { return }
        let now = State.now()
        let written = now.written(since: before)
        if !written.isEmpty {
            testCase.record(XCTIssue(type: .assertionFailure,
                                     compactDescription: "wrote the owner's state (\(Self.realCache.path)): " + written.joined(separator: "; ")))
        }
        baseline = now
    }

    func testBundleDidFinish(_ testBundle: Bundle) {
        EngineV3.removeTestDefaults()
        try? FileManager.default.removeItem(at: EngineV3.testCacheDirectory)
    }
}

/// Sets an environment variable for a test and puts back what was there.
struct EnvironmentOverride {
    private var saved: [String: String?] = [:]

    mutating func set(_ name: String, _ value: String) {
        if saved[name] == nil { saved[name] = .some(ProcessInfo.processInfo.environment[name]) }
        setenv(name, value, 1)
    }

    mutating func restore() {
        for (name, value) in saved {
            if let value { setenv(name, value, 1) } else { unsetenv(name) }
        }
        saved = [:]
    }
}
