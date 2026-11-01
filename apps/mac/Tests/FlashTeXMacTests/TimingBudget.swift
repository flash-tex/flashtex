//  What to do with a wall-clock budget on a machine whose speed we do not
//  control.
//
//  Four tests failed on the first CI run that could actually report failures
//  (35022802823), all of them by small margins against absolute constants:
//  54.6 ms against a 50 ms budget, 15 timer ticks against a required 20,
//  1.69 s against a 1.5 s bound. None of them indicates a regression. GitHub's
//  shared runners are slower and noisier than the M1 Max these numbers were
//  calibrated on -- this project has already measured the same binary varying
//  ~7% run to run on its own Linux host, and a shared runner is worse.
//
//  Deleting the budgets would throw away real information: they are how a
//  10x regression gets noticed. Keeping them gating on CI is worse than
//  deleting them, because a perf test that fails a third of the time teaches
//  people that a red CI is normal -- which is exactly how 35 failures went
//  unnoticed here until `pipefail` landed.
//
//  So: measure everywhere, report everywhere, enforce where the number means
//  something. On a shared runner an overrun prints a `perf(non-gating)` line
//  -- greppable in swift-test.log, and visible to anyone who wonders whether
//  something got slower -- instead of failing the build. Locally and on the
//  owner's Mac the budget is enforced exactly as before, unless the 1-minute
//  load average exceeds the core count: a Mac saturated by other lanes'
//  builds is as uncontrolled as a shared runner, and says so in the line.
//
//  This applies ONLY to "how fast is this machine" assertions. Invariants that
//  hold at any speed stay gating everywhere: a main thread that stalls for
//  half a second is broken on any hardware, backpressure must refuse, and
//  results must be correct.

import Foundation
import XCTest

enum TimingBudget {
    /// GitHub Actions sets `CI=1` for the mac job (.github/workflows/ci.yml).
    static var isSharedRunner: Bool {
        ProcessInfo.processInfo.environment["CI"] == "1"
    }

    /// The 1-minute load average, or nil when the kernel does not say.
    static var loadAverage1: Double? {
        var l = [0.0, 0.0, 0.0]
        return getloadavg(&l, 3) >= 1 ? l[0] : nil
    }

    /// More runnable work than cores: the owner's Mac with other lanes
    /// building (load 20-60 is routine) times the scheduler, not the code.
    static func isLoaded(load: Double?, cores: Int) -> Bool {
        guard let load else { return false }
        return load > Double(max(cores, 1))
    }

    /// Whether a budget overrun here says anything about the code: not on a
    /// shared runner, not on a saturated machine.
    static var speedIsUncontrolled: Bool {
        isSharedRunner || isLoaded(load: loadAverage1, cores: ProcessInfo.processInfo.activeProcessorCount)
    }

    /// What a measurement does, kept separate from measuring so the policy
    /// itself can be tested rather than inferred from a CI run.
    enum Outcome: Equatable {
        /// Inside the budget: reported, nothing to decide.
        case met
        /// Outside it, on a machine whose speed we do not control: reported,
        /// not gating.
        case reported
        /// Outside it, on a machine whose speed we do: a failure.
        case failed
    }

    static func outcome(met: Bool, sharedRunner: Bool) -> Outcome {
        if met { return .met }
        return sharedRunner ? .reported : .failed
    }

    private static var uncontrolledReason: String {
        if isSharedRunner { return "shared CI runner" }
        return String(format: "1-minute load %.1f on %d cores", loadAverage1 ?? -1, ProcessInfo.processInfo.activeProcessorCount)
    }

    private static func report(_ line: String) {
        print(line)
        // Also on stderr: the CI step pipes stdout through `tail -n 400`, and
        // a measurement is worth nothing if it is the part that gets cut.
        FileHandle.standardError.write(Data((line + "\n").utf8))
    }

    /// `measured` must not exceed `budget`. Always reported; gating only where
    /// the machine's speed is known.
    static func assertWithin(_ measured: Double, _ budget: Double, unit: String = "ms", _ what: String,
                             file: StaticString = #filePath, line: UInt = #line) {
        let over = measured > budget
        let overshoot = budget > 0 ? (measured / budget - 1) * 100 : 0
        let summary = String(format: "%@: %.3f %@ against a %.3f %@ budget", what, measured, unit, budget, unit)
        switch outcome(met: !over, sharedRunner: speedIsUncontrolled) {
        case .met: report("perf: " + summary)
        case .reported: report(String(format: "perf(non-gating: %@): %@ — over by %.1f%%", uncontrolledReason, summary, overshoot))
        case .failed: XCTFail(summary + " — over budget", file: file, line: line)
        }
    }

    /// `measured` must reach `floor` (tick counts, sample counts): the same
    /// policy from the other side.
    static func assertAtLeast(_ measured: Int, _ floor: Int, _ what: String,
                              file: StaticString = #filePath, line: UInt = #line) {
        let summary = "\(what): \(measured) against a required \(floor)"
        switch outcome(met: measured >= floor, sharedRunner: speedIsUncontrolled) {
        case .met: report("perf: " + summary)
        case .reported: report("perf(non-gating: \(uncontrolledReason)): " + summary + " — short")
        case .failed: XCTFail(summary + " — short", file: file, line: line)
        }
    }
}

/// The policy itself, so "non-gating on CI" is a decision with a test rather
/// than a claim in a comment.
final class TimingBudgetTests: XCTestCase {
    func testABudgetThatIsMetIsNeverAFailure() {
        XCTAssertEqual(TimingBudget.outcome(met: true, sharedRunner: true), .met)
        XCTAssertEqual(TimingBudget.outcome(met: true, sharedRunner: false), .met)
    }

    /// The point of the whole file: a shared runner reports an overrun, a
    /// machine whose speed we control fails on it. Neither one is silent.
    func testAnOverrunIsReportedOnSharedHardwareAndFailsElsewhere() {
        XCTAssertEqual(TimingBudget.outcome(met: false, sharedRunner: true), .reported,
                       "a shared runner's clock is not evidence of a regression")
        XCTAssertEqual(TimingBudget.outcome(met: false, sharedRunner: false), .failed,
                       "on known hardware the budget is still a gate")
    }

    /// A saturated machine is as uncontrolled as a shared runner; an idle one
    /// (or one that will not report its load) still gates.
    func testALoadedMachineIsTreatedLikeASharedRunner() {
        XCTAssertTrue(TimingBudget.isLoaded(load: 24, cores: 10))
        XCTAssertFalse(TimingBudget.isLoaded(load: 6, cores: 10))
        XCTAssertFalse(TimingBudget.isLoaded(load: nil, cores: 10))
    }

    func testSharedRunnerFollowsTheCIVariableTheWorkflowSets() {
        XCTAssertEqual(TimingBudget.isSharedRunner, ProcessInfo.processInfo.environment["CI"] == "1")
    }
}
