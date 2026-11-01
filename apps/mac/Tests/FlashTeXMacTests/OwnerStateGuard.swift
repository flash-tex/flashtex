import Foundation
import XCTest
@testable import FlashTeXMac

/// Fails the test during which the suite wrote the owner's state:
///
/// - the real `~/Library/Caches/FlashTeX`:
///   - a project copy or host pid file of this test process
///     (`engine-v3/projects/*-<pid>-*`, `engine-v3/hosts/*` naming this pid);
///   - a new page-snapshot directory (`engine-v3/snapshots/*`);
/// - the engine-v3 keys (`FlashTeX.EngineV3.*`) of the standard defaults
///   domain, which under XCTest is the test runner's.
///
/// The owner's running app may write its own cache meanwhile (its s0
/// snapshots and its own snapshots, under keys of its projects), so those
/// are not compared. A new snapshot directory in the middle of a test run
/// counts as the suite's, since the app writes into the ones it already
/// has.
///
/// The format cache (`formats/*`) is not compared: other processes on the
/// machine write it while the suite runs (seen: the self-hosted Actions
/// runner's `flashtex-v3` end-to-end tests, the owner's app, engine test
/// runs), and a change there cannot be told apart from the suite's. That
/// the suite's hosts use their own format cache is checked where it is
/// decided (`EngineV3HostProcess.environment`,
/// `EngineV3OwnerStateTests.testWithoutACacheSettingATestUsesItsOwnTemporaryCache`).
///
/// Every engine-v3 test class installs the guard in `setUp`; from then on
/// it checks each test in a teardown block it adds when the test starts,
/// while the test is still running, so the failure is that test's. (It
/// recorded the failure in `testCaseDidFinish` before: XCTest does not take
/// an issue for a test that has finished, and the whole run aborted with
/// "terminate_handler unexpectedly threw an exception".) A write seen only
/// after a test finished (a process exiting late) is printed and counted
/// against the next test. The guard removes the test process's defaults
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

    /// Takes the current state as the baseline (a test that wrote on
    /// purpose and was failed for it, once it has put things back).
    static func rebase() { shared.baseline = State.now() }

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
            if runnerEngineV3Keys != before.runnerEngineV3Keys { out.append("defaults \(runnerEngineV3Keys) (was \(before.runnerEngineV3Keys))") }
            return out
        }
    }

    func testCaseWillStart(_ testCase: XCTestCase) {
        // (Teardown blocks run after `tearDown`, while the test still runs.)
        testCase.addTeardownBlock { [weak self] in self?.check(testCase) }
    }

    /// Records what was written since the last check as the test's failure.
    private func check(_ testCase: XCTestCase) {
        guard let before = baseline else { return }
        let now = State.now()
        let written = now.written(since: before)
        if !written.isEmpty {
            testCase.record(XCTIssue(type: .assertionFailure,
                                     compactDescription: "wrote the owner's state (\(Self.realCache.path)): " + written.joined(separator: "; ")))
        }
        baseline = now
    }

    /// After a test finished an issue can no longer be recorded: a write
    /// seen here is printed, and the next test's check still reports it.
    func testCaseDidFinish(_ testCase: XCTestCase) {
        guard let before = baseline else { return }
        let written = State.now().written(since: before)
        if !written.isEmpty {
            print("OwnerStateGuard: after \(testCase.name) finished, the owner's state was written: " + written.joined(separator: "; "))
        }
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
