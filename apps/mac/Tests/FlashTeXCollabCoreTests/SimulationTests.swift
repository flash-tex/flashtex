import Foundation
import XCTest
@testable import FlashTeXCollabCore

/// Proposal §7.2, product side: seeded random interleavings with two to five
/// peers through a hostile network must converge byte-identically, and the
/// intention-preservation properties hold. `FLASHTEX_COLLAB_FUZZ_CASES` sets
/// the scenario count (the P0 gate run used 10^6); a failure names its seed.
final class SimulationTests: XCTestCase {
    static func scenario(_ seed: UInt64) -> Int {
        var rng = SplitMix64(seed: seed)
        let n = 2 + rng.below(4)
        let sim = Sim(seed: seed ^ 0x5eed, peers: n)
        let main = FileID.random(using: &sim.rng)
        sim.broadcast(0, [try! sim.peers[0].fileOp { try $0.create(main, kind: .text, path: "main.tex") }])
        sim.flushInOrder()
        let steps = 5 + rng.below(40)
        for _ in 0..<steps {
            let roll = rng.below(100)
            if roll < 47 {
                sim.randomTextEdit(rng.below(n))
            } else if roll < 55 {
                sim.randomUndo(rng.below(n))
            } else if roll < 60 {
                sim.randomFileOp(rng.below(n))
            } else if roll < 63 && sim.partition == nil && n > 2 {
                sim.partition = (0..<n).map { _ in rng.chance(50) }
            } else if roll < 66 {
                sim.partition = nil
            } else {
                for _ in 0..<rng.below(4) { _ = sim.deliverOne() }
            }
        }
        sim.heal()
        sim.assertConverged("seed \(String(seed, radix: 16))")
        return sim.frames.count
    }

    func testRandomInterleavingsConverge() {
        let n = envInt("FLASHTEX_COLLAB_FUZZ_CASES", 500)
        let first = UInt64(envInt("FLASHTEX_COLLAB_FUZZ_FIRST", 0))
        var updates = 0
        let t0 = Date()
        for seed in first..<first + UInt64(n) { updates += Self.scenario(seed) }
        print("collab fuzz: \(n) scenarios, \(updates) updates, \(String(format: "%.1f", Date().timeIntervalSince(t0))) s")
    }

    /// Causal, non-concurrent delivery: the text equals a plain string
    /// edited the same way.
    func testSequentialDeliveryMatchesAPlainString() throws {
        for seed in 0..<UInt64(min(envInt("FLASHTEX_COLLAB_FUZZ_CASES", 1_000), 3_000)) {
            var rng = SplitMix64(seed: seed)
            let n = 2 + rng.below(3)
            let peers = (0..<n).map { TextDocument(replica: UInt64($0) + 1) }
            var reference: [Unicode.Scalar] = []
            for _ in 0..<40 {
                let p = rng.below(n)
                let len = reference.count
                var ops: [TextOp]
                if len > 0 && rng.chance(35) {
                    let pos = rng.below(len)
                    let k = 1 + rng.below(min(len - pos, 5))
                    reference.removeSubrange(pos..<pos + k)
                    ops = peers[p].delete(at: pos, length: k)
                } else {
                    let pos = rng.below(len + 1)
                    let t = randomText(&rng, maxPieces: 5)
                    reference.insert(contentsOf: t.unicodeScalars, at: pos)
                    ops = [try peers[p].insert(t, at: pos)!]
                }
                for (q, peer) in peers.enumerated() where q != p {
                    for op in ops { try peer.apply(op) }
                }
                var want = String.UnicodeScalarView()
                want.append(contentsOf: reference)
                for peer in peers { XCTAssertEqual(Array(peer.text.utf8), Array(String(want).utf8), "seed \(seed)") }
            }
        }
    }

    /// Concurrent typing at one spot never interleaves, forwards or
    /// backwards, one operation per key.
    func testConcurrentTypingAtOneSpotDoesNotInterleave() throws {
        for seed in 0..<UInt64(min(envInt("FLASHTEX_COLLAB_FUZZ_CASES", 1_000), 3_000)) {
            var rng = SplitMix64(seed: seed)
            let n = 2 + rng.below(3)
            let base = TextDocument(replica: 999)
            let baseOp = try base.insert("[]", at: 0)!
            var peers: [TextDocument] = []
            for _ in 0..<n {
                let d = TextDocument(replica: rng.next() | 1)
                try d.apply(baseOp)
                peers.append(d)
            }
            let backwards = rng.chance(50)
            var words: [String] = []
            var ops: [[TextOp]] = []
            for (k, peer) in peers.enumerated() {
                let letter = String(UnicodeScalar(UInt8(ascii: "a") + UInt8(k)))
                let len = 1 + rng.below(6)
                var mine: [TextOp] = []
                for i in 0..<len { mine.append(try peer.insert(letter, at: backwards ? 1 : 1 + i)!) }
                words.append(String(repeating: letter, count: len))
                ops.append(mine)
            }
            for (q, peer) in peers.enumerated() {
                var cursors = Array(repeating: 0, count: n)
                while true {
                    let ready = (0..<n).filter { $0 != q && cursors[$0] < ops[$0].count }
                    if ready.isEmpty { break }
                    let k = ready[rng.below(ready.count)]
                    try peer.apply(ops[k][cursors[k]])
                    cursors[k] += 1
                }
            }
            let text = peers[0].text
            for peer in peers { XCTAssertEqual(peer.text, text, "seed \(seed)") }
            for w in words { XCTAssertTrue(text.contains(w), "seed \(seed): \(w) interleaved in \(text)") }
        }
    }

    /// A caret anchored to a scalar stays beside it through edits elsewhere.
    func testRelativePositionsFollowTheirAnchor() throws {
        for seed in 0..<UInt64(1_000) {
            var rng = SplitMix64(seed: seed)
            let a = TextDocument(replica: 1), b = TextDocument(replica: 2)
            try b.apply(try a.insert(randomText(&rng, maxPieces: 12), at: 0)!)
            let pos = rng.below(a.count + 1)
            let assoc: Assoc = rng.chance(50) ? .before : .after
            let rp = a.relativePosition(at: pos, assoc: assoc)
            let scalars = Array(a.text.unicodeScalars)
            let anchorScalar: Unicode.Scalar? = assoc == .before
                ? (pos < scalars.count ? scalars[pos] : nil) : (pos > 0 ? scalars[pos - 1] : nil)
            for _ in 0..<10 {
                let l = b.count
                let ops = l > 0 && rng.chance(30)
                    ? b.delete(at: rng.below(l), length: 1)
                    : [try b.insert(randomText(&rng, maxPieces: 3), at: rng.below(l + 1))!]
                for op in ops { try a.apply(op) }
            }
            let at = try XCTUnwrap(a.resolve(rp))
            XCTAssertLessThanOrEqual(at, a.count)
            let now = Array(a.text.unicodeScalars)
            if let anchor = rp.anchor {
                guard a.visibleIDs.contains(anchor) else { continue }
                if assoc == .before { XCTAssertEqual(now[at], anchorScalar, "seed \(seed)") }
                else { XCTAssertEqual(now[at - 1], anchorScalar, "seed \(seed)") }
            } else {
                XCTAssertEqual(at, assoc == .before ? a.count : 0)
            }
        }
    }

    func testConcurrentDeleteAndInsertKeepBothIntentions() throws {
        let a = TextDocument(replica: 1), b = TextDocument(replica: 2), c = TextDocument(replica: 3)
        let op0 = try a.insert("hello world", at: 0)!
        try b.apply(op0); try c.apply(op0)
        let da = a.delete(at: 0, length: 6)
        let ib = try b.insert("XY", at: 3)!
        let dc = c.delete(at: 2, length: 4)
        for op in da + [ib] + dc { for d in [a, b, c] { try d.apply(op) } }
        XCTAssertEqual(a.text, "XYworld")
        XCTAssertEqual(a.text, b.text)
        XCTAssertEqual(a.digest, c.digest)
    }

    func testDuplicatesPartialOverlapsAndReusedIDs() throws {
        let a = TextDocument(replica: 7), b = TextDocument(replica: 8)
        let op = try a.insert("abcdef", at: 0)!
        let tail = op.withoutPrefix(2)
        XCTAssertThrowsError(try b.apply(tail)) { XCTAssertEqual($0 as? CollabError, .missingDependency(CollabID(replica: 7, counter: 0))) }
        XCTAssertEqual(try b.apply(op), .applied)
        XCTAssertEqual(try b.apply(op), .duplicate)
        XCTAssertEqual(try b.apply(tail), .duplicate)
        guard case let .insert(id, l, r, _) = op else { return XCTFail() }
        XCTAssertThrowsError(try b.apply(.insert(id: id, originLeft: l, originRight: r, content: "abXdef"))) {
            XCTAssertEqual($0 as? CollabError, .idConflict(CollabID(replica: 7, counter: 2)))
        }
        XCTAssertEqual(b.text, "abcdef")
        XCTAssertEqual(a.digest, b.digest)
        // A delete naming a deletion unit is malformed, not pending forever.
        let d = a.delete(at: 0, length: 1)[0]
        try b.apply(d)
        XCTAssertThrowsError(try b.apply(.delete(id: CollabID(replica: 9, counter: 0), target: d.id, length: 1))) {
            XCTAssertEqual($0 as? CollabError, .malformed("reference to a deletion unit"))
        }
    }

    func testFileMapRenamesCollisionsAndDeletes() throws {
        let f1 = FileID(bytes: Array(repeating: 1, count: 16)), f2 = FileID(bytes: Array(repeating: 2, count: 16))
        let a = CollabProject(replica: 1), b = CollabProject(replica: 2)
        b.receive([try a.fileOp { try $0.create(f1, kind: .text, path: "main.tex") }])
        b.receive([try a.insert("\\section{A}", at: 0, in: f1)!])
        let r = try a.fileOp { try $0.rename(f1, to: "intro.tex") }
        let e2 = try b.insert("% b\n", at: 0, in: f1)!
        let c2 = try b.fileOp { try $0.create(f2, kind: .text, path: "intro.tex") }
        a.receive([e2, c2])
        b.receive([r])
        XCTAssertEqual(a.files.files, b.files.files)
        XCTAssertEqual(a.text(f1)!.text, "% b\n\\section{A}")
        XCTAssertEqual(a.files.files.map(\.path).sorted(),
                       ["intro (conflict 0000000000000002-2).tex", "intro.tex"])
        let d = try a.fileOp { try $0.setDeleted(f1, true) }
        let e3 = try b.insert("x", at: 0, in: f1)!
        a.receive([e3]); b.receive([d])
        XCTAssertEqual(a.files.files, b.files.files)
        XCTAssertEqual(a.files.files.count, 1)
        XCTAssertEqual(a.digest, b.digest)
    }

    /// Contract §2.6 over bytes: a combining mark after `/` or `\\` must not
    /// hide the separator (the review's `../\u{301}etc/passwd`). The same
    /// paths are in the shared `hostile-paths` fixture.
    func testPathRulesAreBytewise() {
        let bad = ["../\u{301}etc/passwd", "a\\\u{301}b", "./\u{301}x", "a/..", "a/../b", "..", "", "/abs.tex",
                   "a//b.tex", "a/", "a\0b", String(repeating: "x", count: 1025), "\u{301}/../x"]
        for p in bad { XCTAssertFalse(FileMap.isValidPath(p), "accepted \(p.unicodeScalars.map { $0.value })") }
        let good = ["a/\u{301}", "e\u{301}.tex", "\u{e9}.tex", "ch/\u{4e2d}.tex", ".x", "...", "a.b/c..d",
                    String(repeating: "x", count: 1024)]
        for p in good { XCTAssertTrue(FileMap.isValidPath(p), "refused \(p)") }
        // NFC and NFD are different files, never merged (byte-exact).
        let m = CollabProject(replica: 1)
        _ = try? m.fileOp { try $0.create(FileID(bytes: Array(repeating: 1, count: 16)), kind: .text, path: "\u{e9}.tex") }
        _ = try? m.fileOp { try $0.create(FileID(bytes: Array(repeating: 2, count: 16)), kind: .text, path: "e\u{301}.tex") }
        XCTAssertEqual(m.files.files.filter(\.conflict).count, 0)
    }

    /// Parked operations are bounded by bytes as well as by count, and an
    /// exact redelivery is not parked twice.
    func testPendingBufferIsBoundedByBytes() {
        let p = CollabProject(replica: 1)
        let f = FileID(bytes: Array(repeating: 3, count: 16))
        _ = p.receive([.fileMap([FileOp(id: CollabID(replica: 9, counter: 0), lamport: 1, file: f,
                                        kind: .create(kind: .text, path: "a.tex"))])])
        let ghost = CollabID(replica: 0x6057, counter: 0)
        let big = String(repeating: "x", count: 1 << 20)
        var refused = 0
        for k in 0..<20 {
            let op = TextOp.insert(id: CollabID(replica: 100 + UInt64(k), counter: 0), originLeft: ghost,
                                   originRight: nil, content: big)
            let errors = p.receive([.text(f, [op]), .text(f, [op])])
            refused += errors.filter { $0.kindName == "pending_full" }.count
        }
        XCTAssertEqual(p.pendingCount, 15) // 15 x (1 MiB + 64) fit in 16 MiB
        XCTAssertLessThanOrEqual(p.pendingBytes, CollabLimits.maxPendingBytes)
        XCTAssertEqual(refused, 10) // 5 ops, each delivered twice
    }
}
