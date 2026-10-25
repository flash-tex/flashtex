import Foundation
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
@testable import FlashTeXMac

/// How long the main run loop keeps an event waiting (EDITOR-FREEZE): a
/// thread posts a block to main every `intervalUs` and records how long it
/// waited to run. A main thread that is busy for 2 s shows as a 2,000 ms
/// sample, whatever kept it busy. Shared by the typing tests
/// (`EditorInstantTests`) and the real-document gate (REAL-DOC-GATE): start
/// it before the work, `stop()` after, and assert on `stats`.
final class RunLoopLatencyProbe: @unchecked Sendable {
    private let lock = NSLock()
    private var stopFlag = false
    private var samples: [Double] = []
    let intervalUs: UInt32

    init(intervalUs: UInt32 = 2000) { self.intervalUs = intervalUs }

    /// Starts the posting thread (once).
    func start() {
        let t = Thread { [self] in
            while !stopped {
                let t0 = MonotonicClock.nowNs()
                CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.commonModes.rawValue) { [self] in
                    let ms = Double(MonotonicClock.nowNs() &- t0) / 1e6
                    lock.lock(); if samples.count < 1_000_000 { samples.append(ms) }; lock.unlock()
                }
                CFRunLoopWakeUp(CFRunLoopGetMain())
                usleep(intervalUs)
            }
        }
        t.qualityOfService = .userInteractive
        t.start()
    }

    private var stopped: Bool { lock.lock(); defer { lock.unlock() }; return stopFlag }

    /// Stops posting; the waits measured so far (ms, in order).
    @discardableResult
    func stop() -> [Double] {
        lock.lock(); defer { lock.unlock() }
        stopFlag = true
        return samples
    }

    /// The waits measured so far (ms).
    var waits: [Double] { lock.lock(); defer { lock.unlock() }; return samples }

    /// p50, p95, p99, max and n of `v` (ms); empty for no samples.
    static func stats(_ v: [Double]) -> [String: Double] {
        guard !v.isEmpty else { return [:] }
        let s = v.sorted()
        func p(_ q: Double) -> Double { s[min(s.count - 1, max(0, Int((q / 100 * Double(s.count)).rounded(.up)) - 1))] }
        return ["p50": p(50), "p95": p(95), "p99": p(99), "max": s.last!, "n": Double(s.count)]
    }
}

/// Host frames for a document with many distinct problems, as a long book
/// has (Infinite Descent ×2: thousands of overfull and underfull boxes, each
/// its own group in the Problems panel, since the amounts differ).
enum ManyProblemsFixture {
    /// `n` warnings with distinct messages (so `n` groups), spread over `lines` lines of `path`.
    static func warnings(_ n: Int, path: String, lines: Int) -> [DL3JSON] {
        (0 ..< n).map { i in
            let kind = i % 3 == 0 ? "Underfull \\hbox (badness \(1000 + i))" : "Overfull \\hbox (\(i / 10).\(i % 10)pt too wide)"
            return .object(["severity": .string("warning"), "message": .string("\(kind) in paragraph at lines \(i)--\(i + 1)"),
                            "file": .string(path), "line": .int(Int64((i * 37) % max(lines, 1) + 1))])
        }
    }

    /// A compile that reports `warnings` and ends: STARTED, the page, PAGES, the warnings, DONE.
    static func compile(id: Int, page: DL3PreparedPage, pages n: Int, warnings: [DL3JSON]) -> [EngineV3Reader.Output] {
        var outs: [EngineV3Reader.Output] = [.started(.object(["id": .int(Int64(id)), "keep": .bool(true), "mode": .string("incremental")]))]
        outs.append(.page(page, compileID: id, timing: .init(), image: nil))
        outs.append(.pages(.object(["count": .int(Int64(n)), "current": .array([.array([.int(0), .int(Int64(n - 1))])])])))
        outs += warnings.map { .diagnostic($0) }
        outs.append(.done(.object(["id": .int(Int64(id)), "status": .string("ok"), "mode": .string("incremental"), "pages": .int(Int64(n)),
                                   "elapsed_ms": .double(20)]), compileID: id))
        return outs
    }
}
