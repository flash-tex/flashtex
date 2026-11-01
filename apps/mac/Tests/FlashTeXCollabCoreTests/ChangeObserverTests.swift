import Foundation
import XCTest
@testable import FlashTeXCollabCore

/// `TextDocument.changeObserver`: the ranged changes an editor replays on
/// its text storage for remote operations and undo (proposal §2.4, "remote
/// edit path"). Replaying them one by one on a mirror of the old text must
/// give exactly the new text, for any interleaving.
final class ChangeObserverTests: XCTestCase {
    /// Replays `changes` on an NSMutableString, as `NSTextStorage` would.
    private func replay(_ changes: [TextChange], on mirror: NSMutableString) {
        for c in changes {
            XCTAssertLessThanOrEqual(c.location + c.length, mirror.length)
            mirror.replaceCharacters(in: NSRange(location: c.location, length: c.length), with: c.text)
        }
    }

    func testRemoteOperationsAndUndoReplayOnAMirror() throws {
        for seed in 0..<UInt64(envInt("FLASHTEX_COLLAB_OBSERVER_CASES", 400)) {
            var rng = SplitMix64(seed: seed)
            let docs = [TextDocument(replica: 7), TextDocument(replica: 9), TextDocument(replica: 4)]
            let undo = docs.map { TextUndoManager(document: $0) }
            let mirrors = docs.map { _ in NSMutableString() }
            var inflight: [[TextOp]] = [[], [], []]
            func send(_ from: Int, _ ops: [TextOp]) {
                for to in 0..<3 where to != from { inflight[to] += ops }
            }
            func observed(_ d: Int, _ body: () throws -> Void) rethrows {
                var changes: [TextChange] = []
                docs[d].changeObserver = { changes.append($0) }
                defer { docs[d].changeObserver = nil }
                try body()
                replay(changes, on: mirrors[d])
            }
            for _ in 0..<(5 + rng.below(30)) {
                let p = rng.below(3)
                let d = docs[p]
                switch rng.below(10) {
                case 0...3:
                    // A local edit: the editor already has it, so it is
                    // mirrored by hand and never observed.
                    let len = d.utf16Count
                    // Whole scalars, as `replace` widens a range that ends
                    // inside a surrogate pair.
                    let a = d.utf16Offset(ofScalar: d.scalarOffset(ofUTF16: rng.below(len + 1)))
                    let b = d.utf16Offset(ofScalar: d.scalarOffset(ofUTF16RoundingUp: min(len, a + rng.below(4))))
                    let text = rng.chance(70) ? randomText(&rng, maxPieces: 4) : ""
                    let ops = try d.replace(utf16Range: a..<b, with: text)
                    mirrors[p].replaceCharacters(in: NSRange(location: a, length: b - a), with: text)
                    undo[p].record(ops)
                    send(p, ops)
                case 4, 5:
                    var ops: [TextOp] = []
                    try observed(p) { ops = rng.chance(70) ? undo[p].undo() : undo[p].redo() }
                    send(p, ops)
                default:
                    guard !inflight[p].isEmpty else { continue }
                    let n = 1 + rng.below(inflight[p].count)
                    let batch = Array(inflight[p].prefix(n))
                    inflight[p].removeFirst(n)
                    try observed(p) { for op in batch { try d.apply(op) } }
                }
                XCTAssertEqual(mirrors[p] as String, d.text, "seed \(seed)")
            }
            for p in 0..<3 {
                try observed(p) { for op in inflight[p] { try docs[p].apply(op) } }
                inflight[p] = []
                XCTAssertEqual(mirrors[p] as String, docs[p].text, "seed \(seed): final")
            }
            XCTAssertEqual(docs[0].text, docs[1].text, "seed \(seed)")
            XCTAssertEqual(docs[1].text, docs[2].text, "seed \(seed)")
        }
    }

    /// A delete whose targets were split by a concurrent insertion reports
    /// one change per visible piece, in order.
    func testDeleteAroundAConcurrentInsertionReportsEachPiece() throws {
        let a = TextDocument(replica: 1), b = TextDocument(replica: 2)
        for op in try a.replace(utf16Range: 0..<0, with: "abcdef") { try b.apply(op) }
        let ins = try b.replace(utf16Range: 3..<3, with: "XY") // abcXYdef
        let del = try a.replace(utf16Range: 1..<5, with: "") // "af" for a: b, c, d and e go
        for op in ins { try a.apply(op) }
        var changes: [TextChange] = []
        b.changeObserver = { changes.append($0) }
        for op in del { try b.apply(op) }
        XCTAssertEqual(changes, [TextChange(location: 1, length: 2, text: ""), TextChange(location: 3, length: 2, text: "")])
        XCTAssertEqual(b.text, "aXYf")
        XCTAssertEqual(a.text, b.text)
    }

    func testSurrogatePairsAreReportedInUTF16() throws {
        let a = TextDocument(replica: 1), b = TextDocument(replica: 2)
        var changes: [TextChange] = []
        b.changeObserver = { changes.append($0) }
        for op in try a.replace(utf16Range: 0..<0, with: "a😀b") { try b.apply(op) }
        for op in try a.replace(utf16Range: 1..<3, with: "é") { try b.apply(op) }
        XCTAssertEqual(changes, [TextChange(location: 0, length: 0, text: "a😀b"),
                                 TextChange(location: 1, length: 2, text: ""),
                                 TextChange(location: 1, length: 0, text: "é")])
    }

    func testUndoCountsTrackTheStacks() throws {
        let d = TextDocument(replica: 3)
        let u = TextUndoManager(document: d)
        u.record(try d.replace(utf16Range: 0..<0, with: "hi"))
        XCTAssertEqual(u.undoCount, 1)
        _ = u.undo()
        XCTAssertEqual(u.undoCount, 0)
        XCTAssertEqual(u.redoCount, 1)
        u.beginGroup()
        XCTAssertTrue(u.isGrouping)
        u.endGroup()
        XCTAssertFalse(u.isGrouping)
    }
}
