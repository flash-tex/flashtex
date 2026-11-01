import Foundation
import XCTest
@testable import FlashTeXCollabCore

final class WireTests: XCTestCase {
    func testEveryMessageKindRoundTrips() throws {
        let f = FileID(bytes: Array(0..<16))
        let pins = CollabControl.SessionPins(main: "main.tex", sourceDateEpoch: 1_759_536_000, randomSeed: 42)
        let pos = CollabControl.Position(RelativePosition(anchor: CollabID(replica: .max, counter: 9), assoc: .before))
        XCTAssertEqual(pos.relativePosition, RelativePosition(anchor: CollabID(replica: .max, counter: 9), assoc: .before))
        let messages: [CollabMessage] = [
            .join(.init(inviteProof: "cHJvb2Y", nonce: "bm9uY2U", guestPublicKey: "a2V5", displayName: "Bob", deviceKind: "ipad")),
            .joinAck(.init(participantID: "00000000000000ff", role: "edit", token: "t", colourIndex: 2, pins: pins,
                           environmentDigest: "abc")),
            .syncRequest([DocVector(doc: .fileMap, vector: StateVector([1: 5, 2: 0])),
                          DocVector(doc: .text(f), vector: StateVector([.max: 1 << 40]))]),
            .syncReply([.text(f, [.insert(id: CollabID(replica: 3, counter: 0), originLeft: nil, originRight: nil, content: "a😀é")])]),
            .update(seq: 300, sections: [
                .fileMap([FileOp(id: CollabID(replica: 1, counter: 0), lamport: 1, file: f, kind: .create(kind: .blob, path: "fig/a.png")),
                          FileOp(id: CollabID(replica: 1, counter: 1), lamport: 2, file: f,
                                 kind: .setBlob(BlobRef(sha256: Array(repeating: 7, count: 32), bytes: 9, mediaType: "image/png"))),
                          FileOp(id: CollabID(replica: 1, counter: 2), lamport: 3, file: f, kind: .setDeleted(true)),
                          FileOp(id: CollabID(replica: 1, counter: 3), lamport: 4, file: f, kind: .setPath("b.png"))]),
                .text(f, [.delete(id: CollabID(replica: 3, counter: 3), target: CollabID(replica: 3, counter: 0), length: 2)]),
            ]),
            .ack(.init(through: 17)),
            .awareness(.init(participantID: "0000000000000001", name: "Alice", colourIndex: 0, file: f.hex,
                             anchor: pos, head: CollabControl.Position(RelativePosition(anchor: nil, assoc: .after)),
                             previewPage: 3, following: nil, seq: 8)),
            .blobWant(.init(sha256: String(repeating: "07", count: 32), offset: 0, length: 262_144)),
            .blobChunk(sha256: Array(repeating: 7, count: 32), offset: 262_144, data: [1, 2, 3]),
            .previewSubscribe(.init(source: nil, haveFonts: ["cmr10"])),
            .previewFrame([5, 0, 0, 0, 0x45, 1, 2, 3, 4]),
            .compileReport(.init(stateVectorDigest: "a", readSetDigest: "b", pageHashes: ["c", "d"])),
            .leave(.init(reason: nil)),
            .error(.init(code: "unknown_kind", message: "kind 0x7f")),
            .unknown(kind: 0x7F, body: [9]),
        ]
        var stream: [UInt8] = []
        for m in messages {
            let bytes = CollabWire.encode(m)
            let (back, used) = try XCTUnwrap(try CollabWire.decode(bytes))
            XCTAssertEqual(back, m)
            XCTAssertEqual(used, bytes.count)
            stream += bytes
        }
        // Back to back, delivered a byte at a time.
        var got: [CollabMessage] = []
        var buf: [UInt8] = []
        for b in stream {
            buf.append(b)
            if let (m, used) = try CollabWire.decode(buf) { got.append(m); buf.removeFirst(used) }
        }
        XCTAssertEqual(got, messages)
    }

    func testHostileFramesAreRefusedNotCrashed() {
        XCTAssertThrowsError(try CollabWire.decode([0, 0, 0, 0, 5])) { XCTAssertEqual($0 as? CollabWireError, .emptyFrame) }
        XCTAssertThrowsError(try CollabWire.decode([0, 0, 0, 2, 5])) { XCTAssertEqual($0 as? CollabWireError, .frameTooLarge) }
        XCTAssertNil(try CollabWire.decode([3, 0, 0]))
        XCTAssertThrowsError(try CollabWire.decode([5, 0, 0, 0, 5, 1, 1, 0, 200])) { XCTAssertEqual($0 as? CollabWireError, .truncated) }
        XCTAssertThrowsError(try CollabWire.decode([3, 0, 0, 0, 5, 0x80, 0x00])) { XCTAssertEqual($0 as? CollabWireError, .badVarint) }
        // invalid UTF-8 in an insert's content
        let f = FileID(bytes: Array(repeating: 0, count: 16))
        var bad = CollabWire.encode(.update(seq: 1, sections: [.text(f, [.insert(id: CollabID(replica: 1, counter: 0),
                                                                                    originLeft: nil, originRight: nil, content: "ab")])]))
        bad[bad.count - 1] = 0xFF
        XCTAssertThrowsError(try CollabWire.decode(bad)) { XCTAssertEqual($0 as? CollabWireError, .badUTF8) }
        var rng = SplitMix64(seed: 42)
        for _ in 0..<20_000 {
            var b = (0..<rng.below(40)).map { _ in UInt8(truncatingIfNeeded: rng.next()) }
            if b.count >= 5 {
                let len = UInt32(b.count - 4)
                b[0] = UInt8(len & 0xFF); b[1] = 0; b[2] = 0; b[3] = 0
                b[4] = [3, 4, 5, 9, 1, 7][rng.below(6)]
            }
            _ = try? CollabWire.decode(b)
        }
    }
}

final class TreeTests: XCTestCase {
    /// One replica against a plain scalar array: thousands of random edits
    /// (long runs, cross-run deletes), checking the text, the unit
    /// conversions and the tree's invariants as it grows and splits.
    func testRandomEditsMatchAPlainArrayAndKeepInvariants() throws {
        var rng = SplitMix64(seed: 7)
        let d = TextDocument(replica: 1)
        var ref: [Unicode.Scalar] = []
        for step in 0..<8_000 {
            let len = ref.count
            if len > 0 && rng.chance(30) {
                let pos = rng.below(len)
                let n = 1 + rng.below(min(len - pos, rng.chance(10) ? 600 : 8))
                ref.removeSubrange(pos..<pos + n)
                d.delete(at: pos, length: n)
            } else {
                let t = randomText(&rng, maxPieces: rng.chance(5) ? 700 : 8)
                let pos = rng.below(len + 1)
                ref.insert(contentsOf: t.unicodeScalars, at: pos)
                try d.insert(t, at: pos)
            }
            if step % 500 == 499 {
                var v = String.UnicodeScalarView()
                v.append(contentsOf: ref)
                let s = String(v)
                XCTAssertEqual(Array(d.text.utf8), Array(s.utf8))
                XCTAssertEqual(d.count, ref.count)
                XCTAssertEqual(d.utf16Count, s.utf16.count)
                XCTAssertEqual(d.utf8Count, s.utf8.count)
                XCTAssertNil(d.validate())
                for _ in 0..<50 {
                    let k = rng.below(ref.count + 1)
                    let prefix = String(String.UnicodeScalarView(ref[..<k]))
                    XCTAssertEqual(d.utf16Offset(ofScalar: k), prefix.utf16.count)
                    XCTAssertEqual(d.utf8Offset(ofScalar: k), prefix.utf8.count)
                    XCTAssertEqual(d.scalarOffset(ofUTF16: prefix.utf16.count), k)
                    XCTAssertEqual(d.scalarOffset(ofUTF8: prefix.utf8.count), k)
                }
            }
        }
        XCTAssertGreaterThan(d.runCount, 1_000)
    }

    func testReplaceByUTF16Range() throws {
        let d = TextDocument(replica: 1)
        try d.insert("a😀b", at: 0)
        // NSString ranges: "a" 0..<1, "😀" 1..<3, "b" 3..<4
        let ops = try d.replace(utf16Range: 1..<3, with: "é")
        XCTAssertEqual(d.text, "aéb")
        XCTAssertEqual(ops.count, 2)
    }

    /// A range ending inside a surrogate pair grows outward to the whole
    /// scalar instead of shrinking to nothing.
    func testReplaceRoundsASplitSurrogateOutward() throws {
        let d = TextDocument(replica: 1)
        try d.insert("a😀b", at: 0)
        XCTAssertEqual(d.scalarOffset(ofUTF16RoundingUp: 2), 2)
        XCTAssertEqual(d.scalarOffset(ofUTF16: 2), 1)
        _ = try d.replace(utf16Range: 1..<2, with: "")
        XCTAssertEqual(d.text, "ab")
        try d.insert("😀", at: 1)
        _ = try d.replace(utf16Range: 2..<3, with: "Z") // starts inside the pair: rounds down, so whole
        XCTAssertEqual(d.text, "aZb")
    }
}
