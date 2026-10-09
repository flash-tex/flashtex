import Foundation
import XCTest
#if canImport(Darwin)
import Darwin
#endif
@testable import FlashTeXCollabCore

/// The proposal's P0 exit gate (§7.1), measured: on a 1 MiB file with at
/// least 100k tombstones, local insert and remote apply ≤ 1 ms p95, memory
/// ≤ 150 B per character; plus throughput and the merge of a 10k-op
/// divergence. Opt-in (it takes a while and wants a release build):
///
///     FLASHTEX_COLLAB_BENCH=1 swift test -c release --filter PerformanceGateTests
///
/// The document is typed the way an editor produces operations: one
/// operation per keystroke at a caret, the caret jumping elsewhere every
/// ~25 keys, backspaces, and occasional range deletes, so runs fragment
/// the way real typing fragments them.
final class PerformanceGateTests: XCTestCase {
    static let alphabet: [String] = Array("abcdefghijklmnopqrstuvwxyz \\{}$^_\n").map(String.init)
        + Array(repeating: "e", count: 8).map { $0 } + ["é", "中", "😀"]

    static func footprint() -> Int {
        #if canImport(Darwin)
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<natural_t>.size)
        let kr = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count)
            }
        }
        return kr == KERN_SUCCESS ? Int(info.phys_footprint) : 0
        #else
        return 0
        #endif
    }

    static func ns() -> UInt64 { DispatchTime.now().uptimeNanoseconds }

    static func pct(_ xs: [UInt64], _ p: Double) -> Double {
        let s = xs.sorted()
        return Double(s[min(s.count - 1, Int(Double(s.count) * p))]) / 1_000
    }

    static func summary(_ name: String, _ xs: [UInt64]) -> String {
        String(format: "%@: n=%d p50=%.1fµs p95=%.1fµs p99=%.1fµs max=%.1fµs", name, xs.count,
               pct(xs, 0.5), pct(xs, 0.95), pct(xs, 0.99), Double(xs.max()!) / 1_000)
    }

    /// Typist: keystrokes at a caret. Returns ops produced.
    final class Typist {
        var rng: SplitMix64
        var caret = 0
        let doc: TextDocument
        var tombstones = 0

        init(_ doc: TextDocument, seed: UInt64) {
            self.doc = doc
            rng = SplitMix64(seed: seed)
        }

        func key() throws -> [TextOp] {
            let n = doc.count
            caret = min(caret, n)
            let r = rng.below(1000)
            if r < 40 {
                caret = rng.below(n + 1)
                return []
            } else if r < 80, caret > 0 {
                caret -= 1
                tombstones += 1
                return doc.delete(at: caret, length: 1)
            } else if r < 81, n > caret + 200 {
                let len = 5 + rng.below(195)
                tombstones += len
                return doc.delete(at: caret, length: len)
            }
            let s = PerformanceGateTests.alphabet[rng.below(PerformanceGateTests.alphabet.count)]
            let op = try doc.insert(s, at: caret)!
            caret += 1
            return [op]
        }
    }

    func testP0PerformanceGate() throws {
        guard ProcessInfo.processInfo.environment["FLASHTEX_COLLAB_BENCH"] != nil else {
            throw XCTSkip("set FLASHTEX_COLLAB_BENCH=1 (and use -c release) to run the P0 gate")
        }
        var lines: [String] = []
        func say(_ s: String) { print("collab-p0: " + s); lines.append(s) }

        // 1. Type a 1 MiB document with ≥ 100k tombstones.
        let target = 1 << 20
        let m0 = Self.footprint()
        let a = TextDocument(replica: 0xA11CE)
        let typist = Typist(a, seed: 1)
        var keys = 0, ops = 0
        let t0 = Self.ns()
        while a.count < target || typist.tombstones < 100_000 {
            ops += try typist.key().count
            keys += 1
        }
        let buildNs = Self.ns() - t0
        let m1 = Self.footprint()
        say(String(format: "built: %d visible scalars (%d UTF-8 bytes), %d tombstones, %d runs, %d log entries, %d keystrokes -> %d ops in %.2f s = %.0f ops/s",
                   a.count, a.utf8Count, a.rawCount - a.count, a.runCount, a.logCount, keys, ops,
                   Double(buildNs) / 1e9, Double(ops) / (Double(buildNs) / 1e9)))
        say(String(format: "memory (phys_footprint delta, includes the op log and indexes): %.1f MiB = %.1f B/char visible, %.1f B/scalar stored",
                   Double(m1 - m0) / 1_048_576, Double(m1 - m0) / Double(a.count), Double(m1 - m0) / Double(a.rawCount)))
        XCTAssertNil(a.validate())

        // 2. Local edits at random positions on the full document.
        var rng = SplitMix64(seed: 2)
        var localIns: [UInt64] = [], localDel: [UInt64] = []
        for _ in 0..<10_000 {
            let p = rng.below(a.count + 1)
            let t = Self.ns()
            _ = try a.insert("x", at: p)
            localIns.append(Self.ns() - t)
            let q = rng.below(a.count)
            let t2 = Self.ns()
            _ = a.delete(at: q, length: 1)
            localDel.append(Self.ns() - t2)
        }
        say(Self.summary("local insert (random position)", localIns))
        say(Self.summary("local delete (random position)", localDel))
        let utf16 = (0..<10_000).map { _ -> UInt64 in
            let u = rng.below(a.utf16Count + 1)
            let t = Self.ns()
            _ = a.scalarOffset(ofUTF16: u)
            return Self.ns() - t
        }
        say(Self.summary("UTF-16 offset -> scalar", utf16))

        // 3. A second replica joins: full sync through the codec.
        let m2 = Self.footprint()
        let tj = Self.ns()
        let joinFrame = CollabWire.encode(.syncReply([.text(FileID(bytes: Array(repeating: 1, count: 16)), a.diff(since: StateVector()))]))
        guard case let .syncReply(sections)? = try CollabWire.decode(joinFrame)?.0, case let .text(_, joinOps) = sections[0] else {
            return XCTFail("join frame")
        }
        let b = TextDocument(replica: 0xB0B)
        for op in joinOps { try b.apply(op) }
        let joinNs = Self.ns() - tj
        let m3 = Self.footprint()
        say(String(format: "join: %d ops, %.1f MiB frame, encode+decode+apply %.0f ms; replica built by remote apply: %.1f B/char (footprint delta, includes the frame)",
                   joinOps.count, Double(joinFrame.count) / 1_048_576, Double(joinNs) / 1e6, Double(m3 - m2) / Double(b.count)))
        XCTAssertEqual(a.digest, b.digest)

        // 4. Remote apply: B types, A applies each keystroke.
        let bt = Typist(b, seed: 3)
        bt.caret = b.count / 2
        var remote: [UInt64] = []
        var remoteOps = 0
        let tr = Self.ns()
        var applyTotal: UInt64 = 0
        while remoteOps < 10_000 {
            for op in try bt.key() {
                let t = Self.ns()
                try a.apply(op)
                let d = Self.ns() - t
                remote.append(d)
                applyTotal += d
                remoteOps += 1
            }
        }
        _ = tr
        say(Self.summary("remote apply (keystrokes)", remote) + String(format: ", %.0f ops/s", Double(remoteOps) / (Double(applyTotal) / 1e9)))
        XCTAssertEqual(a.digest, b.digest)

        // 5. Concurrent typing at one spot (the scanning path).
        let spot = a.count / 3
        let ta = Typist(a, seed: 4), tb = Typist(b, seed: 5)
        ta.caret = spot
        tb.caret = spot
        var aOps: [TextOp] = [], bOps: [TextOp] = []
        for _ in 0..<2_000 {
            aOps += try ta.key()
            bOps += try tb.key()
        }
        var contended: [UInt64] = []
        for op in bOps {
            let t = Self.ns(); try a.apply(op); contended.append(Self.ns() - t)
        }
        for op in aOps { try b.apply(op) }
        say(Self.summary("remote apply, concurrent typing at the same spot", contended))
        XCTAssertEqual(a.digest, b.digest)

        // 6. Merge a 10k-op divergence each way (encode, decode, apply).
        let svA = a.stateVector, svB = b.stateVector
        let da = Typist(a, seed: 6), db = Typist(b, seed: 7)
        var na = 0, nb = 0
        while na < 10_000 { na += try da.key().count }
        while nb < 10_000 { nb += try db.key().count }
        let f = FileID(bytes: Array(repeating: 2, count: 16))
        let tm = Self.ns()
        let toB = CollabWire.encode(.syncReply([.text(f, a.diff(since: svB))]))
        let toA = CollabWire.encode(.syncReply([.text(f, b.diff(since: svA))]))
        guard case let .syncReply(sb)? = try CollabWire.decode(toB)?.0, case let .text(_, forB) = sb[0],
              case let .syncReply(sa)? = try CollabWire.decode(toA)?.0, case let .text(_, forA) = sa[0] else {
            return XCTFail("merge frames")
        }
        let tmid = Self.ns()
        for op in forB { try b.apply(op) }
        let tB = Self.ns()
        for op in forA { try a.apply(op) }
        let tA = Self.ns()
        say(String(format: "merge of a 10k+10k-op divergence: frames %.0f KiB + %.0f KiB (%d + %d coalesced ops), encode+decode %.1f ms, B applies A's 10k in %.1f ms, A applies B's 10k in %.1f ms, total %.1f ms",
                   Double(toB.count) / 1024, Double(toA.count) / 1024, forB.count, forA.count,
                   Double(tmid - tm) / 1e6, Double(tB - tmid) / 1e6, Double(tA - tB) / 1e6, Double(tA - tm) / 1e6))
        XCTAssertEqual(Array(a.text.utf8), Array(b.text.utf8))
        XCTAssertEqual(a.digest, b.digest)
        XCTAssertNil(a.validate())
        XCTAssertNil(b.validate())

        XCTAssertLessThanOrEqual(Self.pct(localIns, 0.95), 1_000, "local insert p95 over 1 ms")
        XCTAssertLessThanOrEqual(Self.pct(remote, 0.95), 1_000, "remote apply p95 over 1 ms")
        XCTAssertLessThanOrEqual(Double(m1 - m0) / Double(a.count), 150, "over 150 B/char")
    }
}
