import Foundation
import QuartzCore
import XCTest
@testable import FlashTeXMac

/// Key → presented bookkeeping: an install's commit time and its probe's
/// presented time meet in either order, and reach the keystroke's sample
/// whether the probe reports before or after the main thread records it.
final class EngineV3PresentTests: XCTestCase {
    func testPairReportsOnceInEitherOrder() {
        final class Box: @unchecked Sendable { var got: [(UInt64, UInt64)] = [] }
        let box = Box()
        let a = EngineV3PresentPair { box.got.append(($0, $1)) }
        a.committed(10); XCTAssertTrue(box.got.isEmpty); a.presented(30)
        let b = EngineV3PresentPair { box.got.append(($0, $1)) }
        b.presented(70); XCTAssertEqual(box.got.count, 1); b.committed(50)
        XCTAssertEqual(box.got.map(\.0), [10, 50])
        XCTAssertEqual(box.got.map(\.1), [30, 70])
    }

    @MainActor
    func testPresentedReachesTheSampleInEitherOrder() {
        let l = EngineV3Latency()
        // Presented after the sample exists.
        l.sent(compile: 1, keystrokeNs: 1_000_000, editNs: 1_100_000, path: "main.tex", at: 1_200_000)
        l.committed(compile: 1, page: 0, at: 11_000_000)
        l.presented(commitNs: 11_000_000, presentedNs: 31_000_000)
        // Presented before the main thread recorded the commit.
        l.sent(compile: 2, keystrokeNs: 100_000_000, editNs: 100_100_000, path: "main.tex", at: 100_200_000)
        l.presented(commitNs: 112_000_000, presentedNs: 130_000_000)
        l.committed(compile: 2, page: 0, at: 112_000_000)
        XCTAssertEqual(l.samples.map(\.toPresentedMs), [30, 30])
        let st = l.stageStats()
        XCTAssertEqual(st["key_to_presented"]?["n"], 2)
        XCTAssertEqual(st["commit_to_presented"]?["p50"] ?? 0, 19, accuracy: 1.01)
        // A frame that was not shown: recorded as 0, not a time.
        l.sent(compile: 3, keystrokeNs: 200_000_000, editNs: 200_100_000, path: "main.tex", at: 200_200_000)
        l.committed(compile: 3, page: 0, at: 210_000_000)
        l.presented(commitNs: 210_000_000, presentedNs: 0)
        XCTAssertEqual(l.samples.last?.presentedNs, 0)
        XCTAssertNil(l.samples.last?.toPresentedMs)
    }
}
