import Foundation
import XCTest
@testable import FlashTeXCollabCore

/// Local-only undo in CRDT ids (proposal §2.4, invariant 5 of §7.2).
final class UndoTests: XCTestCase {
    func testUndoAndRedoOfOneUserReturnToEachState() throws {
        for seed in 0..<UInt64(300) {
            var rng = SplitMix64(seed: seed)
            let d = TextDocument(replica: 5)
            let undo = TextUndoManager(document: d)
            var states = [d.text]
            for _ in 0..<(1 + rng.below(25)) {
                let len = d.count
                if len > 0 && rng.chance(40) {
                    let pos = rng.below(len)
                    undo.record(d.delete(at: pos, length: 1 + rng.below(min(len - pos, 6))))
                } else if rng.chance(15), len > 0 {
                    // a grouped step: replace a range
                    undo.beginGroup()
                    let pos = rng.below(len)
                    undo.record(d.delete(at: pos, length: 1))
                    undo.record([try d.insert(randomText(&rng, maxPieces: 4), at: pos)!])
                    undo.endGroup()
                } else {
                    undo.record([try d.insert(randomText(&rng, maxPieces: 6), at: rng.below(len + 1))!])
                }
                states.append(d.text)
            }
            for want in states.dropLast().reversed() {
                _ = undo.undo()
                XCTAssertEqual(d.text, want, "seed \(seed): undo")
            }
            XCTAssertFalse(undo.canUndo)
            for want in states.dropFirst() {
                _ = redo(undo)
                XCTAssertEqual(d.text, want, "seed \(seed): redo")
            }
            XCTAssertNil(d.validate())
        }
    }

    private func redo(_ u: TextUndoManager) -> [TextOp] { u.redo() }

    /// Undo never removes another replica's characters, even ones typed
    /// inside the undone insertion, and every peer converges on the undo.
    func testUndoTouchesOnlyOwnCharactersAndConverges() throws {
        for seed in 0..<UInt64(envInt("FLASHTEX_COLLAB_UNDO_CASES", 1_000)) {
            var rng = SplitMix64(seed: seed)
            let docs = [TextDocument(replica: 11), TextDocument(replica: 22), TextDocument(replica: 33)]
            let undo = docs.map { TextUndoManager(document: $0) }
            var inflight: [[TextOp]] = [[], [], []]
            func send(_ from: Int, _ ops: [TextOp]) {
                for to in 0..<3 where to != from { inflight[to] += ops }
            }
            func deliverSome() throws {
                for to in 0..<3 where !inflight[to].isEmpty && rng.chance(50) {
                    for op in inflight[to] { try docs[to].apply(op) }
                    inflight[to] = []
                }
            }
            for _ in 0..<30 {
                let p = rng.below(3)
                let d = docs[p]
                if rng.chance(20) {
                    let before = othersVisible(d, mine: d.replica)
                    let ops = rng.chance(75) ? undo[p].undo() : undo[p].redo()
                    let after = othersVisible(d, mine: d.replica)
                    // Undo may restore others' characters (a deletion undone)
                    // but never remove any.
                    XCTAssertTrue(before.isSubset(of: after), "seed \(seed): undo removed another replica's text")
                    send(p, ops)
                } else {
                    let len = d.count
                    let ops: [TextOp]
                    if len > 0 && rng.chance(35) {
                        let pos = rng.below(len)
                        ops = d.delete(at: pos, length: 1 + rng.below(min(len - pos, 5)))
                    } else {
                        ops = [try d.insert(randomText(&rng, maxPieces: 5), at: rng.below(len + 1))!]
                    }
                    undo[p].record(ops)
                    send(p, ops)
                }
                try deliverSome()
            }
            for to in 0..<3 { for op in inflight[to] { try docs[to].apply(op) } }
            XCTAssertEqual(docs[0].text, docs[1].text, "seed \(seed)")
            XCTAssertEqual(docs[0].digest, docs[2].digest, "seed \(seed)")
        }
    }

    /// The copy map shrinks with the step limit.
    func testCopiesArePrunedWithTheStepLimit() throws {
        let d = TextDocument(replica: 4)
        let u = TextUndoManager(document: d)
        u.limit = 8
        for k in 0..<400 {
            u.record([try d.insert("ab", at: 0)!])
            if k % 3 == 0 { _ = u.undo(); _ = u.redo() }
        }
        XCTAssertLessThanOrEqual(u.copyCount, 64)
    }

    private func othersVisible(_ d: TextDocument, mine: UInt64) -> Set<CollabID> {
        Set(d.visibleIDs.filter { $0.replica != mine })
    }

    /// Undoing your own insertion that someone typed inside removes only
    /// your characters.
    func testUndoKeepsTextOthersTypedInside() throws {
        let a = TextDocument(replica: 1), b = TextDocument(replica: 2)
        let undo = TextUndoManager(document: a)
        let ins = try a.insert("hello", at: 0)!
        undo.record([ins])
        try b.apply(ins)
        try a.apply(try b.insert("XY", at: 2)!)
        XCTAssertEqual(a.text, "heXYllo")
        for op in undo.undo() { try b.apply(op) }
        XCTAssertEqual(a.text, "XY")
        XCTAssertEqual(b.text, "XY")
        for op in undo.redo() { try b.apply(op) }
        XCTAssertEqual(a.text, b.text)
        XCTAssertEqual(a.text, "heXYllo")
    }
}
