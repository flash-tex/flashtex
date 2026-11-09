import Foundation
import XCTest
@testable import FlashTeXCollabCore

/// #2059: a refused or conflicting operation changes nothing, and peers
/// that see conflicting operations in different orders still converge.
/// The file-map cases mirror the oracle's `tests/v1_conflicting_creates.rs`.
final class RefusalTests: XCTestCase {
    let a: UInt64 = 0xa, b: UInt64 = 0xb

    func id(_ replica: UInt64, _ counter: UInt64) -> CollabID { CollabID(replica: replica, counter: counter) }

    func fileOp(_ replica: UInt64, _ counter: UInt64, _ lamport: UInt64, _ file: FileID, _ kind: FileOpKind) -> Section {
        .fileMap([FileOp(id: id(replica, counter), lamport: lamport, file: file, kind: kind)])
    }

    /// Delivers `sections` forwards and reversed to fresh observers; both
    /// must refuse nothing, leave nothing waiting and reach one state.
    @discardableResult
    func assertConverges(_ sections: [Section], _ label: String) -> CollabProject {
        let fwd = CollabProject(replica: 0x0b5e), rev = CollabProject(replica: 0x0b5e)
        var refused: [String] = []
        for s in sections { refused += fwd.receive([s]).map(\.kindName) }
        for s in sections.reversed() { refused += rev.receive([s]).map(\.kindName) }
        XCTAssertEqual(refused, [], label)
        XCTAssertEqual(fwd.pendingCount, 0, label)
        XCTAssertEqual(rev.pendingCount, 0, label)
        XCTAssertEqual(fwd.files.files, rev.files.files, label)
        XCTAssertEqual(fwd.files.digest, rev.files.digest, label)
        XCTAssertEqual(fwd.digest, rev.digest, label)
        XCTAssertEqual(fwd.stateVectors, rev.stateVectors, label)
        return fwd
    }

    /// Finding 2: before the fix, the second create to arrive was refused
    /// as an id conflict, so its replica's later operations waited for good
    /// and the two orders kept different paths.
    func testConflictingCreatesConvergeAndDoNotBlockTheReplica() {
        let f = FileID(bytes: Array(repeating: 0x2f, count: 16)), g = FileID(bytes: Array(repeating: 0x30, count: 16))
        let p = assertConverges([
            fileOp(a, 0, 1, f, .create(kind: .text, path: "a.tex")),
            fileOp(b, 0, 1, f, .create(kind: .text, path: "b.tex")),
            fileOp(b, 1, 2, g, .create(kind: .text, path: "g.tex")),
            fileOp(b, 2, 3, g, .setPath("h.tex")),
        ], "same kind")
        XCTAssertEqual(p.files.files.map(\.path), ["b.tex", "h.tex"])
        XCTAssertEqual(p.files.stateVector[a], 1)
        XCTAssertEqual(p.files.stateVector[b], 3)
        XCTAssertNotNil(p.text(f))
        XCTAssertNotNil(p.text(g))
    }

    /// Finding 3: before the fix, a replica that saw the text create first
    /// refused the blob create and its `set_blob`; one that saw the blob
    /// create first took both.
    func testAKindConflictKeepsTheEarliestCreateAndStillTakesSetBlob() {
        let f = FileID(bytes: Array(repeating: 0x3f, count: 16))
        let blob = BlobRef(sha256: Array(repeating: 7, count: 32), bytes: 3, mediaType: "image/png")
        for (la, lb, kind) in [(UInt64(1), UInt64(2), FileKind.text), (2, 1, .blob)] {
            let p = assertConverges([
                fileOp(a, 0, la, f, .create(kind: .text, path: "x.tex")),
                fileOp(b, 0, lb, f, .create(kind: .blob, path: "x.png")),
                fileOp(b, 1, lb + 1, f, .setBlob(blob)),
                fileOp(b, 2, lb + 2, f, .setDeleted(true)),
                fileOp(a, 1, 9, f, .setDeleted(false)),
            ], "\(kind) first")
            XCTAssertEqual(p.files.kind(f), kind)
            XCTAssertEqual(p.files.files.count, 1)
            XCTAssertEqual(p.files.files.first?.blob != nil, kind == .blob, "a text file shows no blob")
            XCTAssertNotNil(p.text(f))
        }
    }

    /// REVIEW-C on #2167: creates with one stamp are one replica's (same
    /// lamport, different counters); the earlier counter wins the kind and
    /// the path in every delivery order (`v1_conflicting_creates.rs`).
    func testCreatesWithOneStampResolveByCounterInEveryOrder() {
        let f = FileID(bytes: Array(repeating: 0x4f, count: 16))
        let sections = [
            fileOp(b, 0, 5, f, .create(kind: .text, path: "x.tex")),
            fileOp(b, 1, 5, f, .create(kind: .blob, path: "y.png")),
            fileOp(a, 0, 9, f, .create(kind: .blob, path: "a.png")),
        ]
        let p = assertConverges(sections, "one stamp")
        XCTAssertEqual(p.files.kind(f), .text)
        XCTAssertEqual(p.files.files.map(\.path), ["a.png"])
        let q = assertConverges(Array(sections.prefix(2)), "one stamp, one replica")
        XCTAssertEqual(q.files.kind(f), .text)
        XCTAssertEqual(q.files.files.map(\.path), ["x.tex"])
    }

    /// This replica still makes neither a second create nor a blob on a
    /// text file itself.
    func testLocalCreateOfAnExistingIDAndLocalBlobOnTextAreRefused() throws {
        let m = FileMap(replica: 1)
        let f = FileID(bytes: Array(repeating: 1, count: 16))
        _ = try m.create(f, kind: .text, path: "a.tex")
        let before = m.digest
        XCTAssertThrowsError(try m.create(f, kind: .blob, path: "b.png")) {
            XCTAssertEqual($0 as? CollabError, .idConflict(CollabID(replica: 1, counter: 1)))
        }
        XCTAssertThrowsError(try m.setBlob(f, BlobRef(sha256: Array(repeating: 0, count: 32), bytes: 1, mediaType: "x"))) {
            XCTAssertEqual($0 as? CollabError, .malformed("blob on a text file"))
        }
        XCTAssertEqual(m.digest, before)
        XCTAssertEqual(m.stateVector[1], 1)
    }

    /// A document holding "abcdef" as one run of replica 1.
    func oneRun(caps: CollabProject.Caps = .init()) throws -> (CollabProject, FileID, TextDocument) {
        let p = CollabProject(replica: 1, caps: caps)
        let f = FileID(bytes: Array(repeating: 9, count: 16))
        _ = try p.fileOp { try $0.create(f, kind: .text, path: "a.tex") }
        _ = try p.insert("abcdef", at: 0, in: f)
        let d = try XCTUnwrap(p.text(f))
        XCTAssertEqual(d.runCount, 1)
        return (p, f, d)
    }

    /// Finding 4: a refused remote insert split the runs at its origins
    /// before it was refused, so a peer could fragment a document with
    /// operations it knew would be refused (and the hub relayed them).
    func testARefusedRemoteInsertLeavesTheRunsAsTheyWere() throws {
        // Origins out of order: left 'e', right 'b'.
        var (p, f, d) = try oneRun()
        let digest = d.digest
        let outOfOrder = TextOp.insert(id: id(9, 0), originLeft: id(1, 4), originRight: id(1, 1), content: "x")
        XCTAssertEqual(p.receive([.text(f, [outOfOrder])]).map(\.kindName), ["malformed"])
        XCTAssertEqual(d.runCount, 1)
        XCTAssertEqual(d.digest, digest)
        XCTAssertEqual(d.text, "abcdef")

        // Past the project's text cap, between 'b' and 'c'.
        (p, f, d) = try oneRun(caps: .init(maxFiles: 4, maxTextBytes: 6, localFileHeadroom: 0, localTextHeadroom: 0))
        let full = TextOp.insert(id: id(9, 0), originLeft: id(1, 1), originRight: id(1, 2), content: "x")
        XCTAssertEqual(p.receive([.text(f, [full])]).map(\.kindName), ["project_full"])
        XCTAssertEqual(d.runCount, 1)
        XCTAssertEqual(d.text, "abcdef")
        XCTAssertEqual(d.stateVector[9], 0)

        // The same insert with room is taken, and splits as it must.
        (p, f, d) = try oneRun()
        XCTAssertEqual(p.receive([.text(f, [full])]), [])
        XCTAssertEqual(d.text, "abxcdef")
        XCTAssertEqual(d.runCount, 3)
    }

    /// Finding 5: undo's re-insertion split a run before deciding to skip
    /// a piece (one still visible, or one that no longer fits), so the runs
    /// changed although no operation was made.
    func testASkippedReinsertionLeavesTheRunsAsTheyWere() throws {
        // A piece that no longer fits: the whole run is a tombstone and the
        // project is full.
        var (_, _, d) = try oneRun(caps: .init(maxFiles: 4, maxTextBytes: 6, localFileHeadroom: 0, localTextHeadroom: 0))
        XCTAssertEqual(d.delete(at: 0, length: 6).count, 1)
        XCTAssertEqual(d.runCount, 1)
        XCTAssertTrue(d.reinsertDeleted(id(1, 1), count: 2).isEmpty)
        XCTAssertEqual(d.runCount, 1)
        // A piece still visible.
        (_, _, d) = try oneRun()
        XCTAssertTrue(d.reinsertDeleted(id(1, 1), count: 2).isEmpty)
        XCTAssertEqual(d.runCount, 1)
        // With room, the deleted piece is copied right after itself.
        (_, _, d) = try oneRun()
        _ = d.delete(at: 0, length: 6)
        XCTAssertEqual(d.reinsertDeleted(id(1, 1), count: 2).count, 1)
        XCTAssertEqual(d.text, "bc")
    }
}
