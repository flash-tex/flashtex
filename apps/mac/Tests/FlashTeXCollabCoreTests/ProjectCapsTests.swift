import Foundation
import XCTest
@testable import FlashTeXCollabCore

/// collab-v1 §2.4/§2.6 project caps, the cases of the oracle's
/// `tests/v1_project_caps.rs`: a peer (a hostile hub, for a guest) cannot
/// grow a replica's memory without bound by creating files or by filling
/// many documents, each under the per-document limit.
final class ProjectCapsTests: XCTestCase {
    func fid(_ n: UInt32) -> FileID {
        var b = [UInt8](repeating: 0, count: 16)
        b[0] = UInt8(n >> 24); b[1] = UInt8((n >> 16) & 0xFF); b[2] = UInt8((n >> 8) & 0xFF); b[3] = UInt8(n & 0xFF)
        return FileID(bytes: b)
    }

    func uncapped(_ replica: UInt64) -> CollabProject {
        CollabProject(replica: replica, caps: .init(maxFiles: .max, maxTextBytes: .max))
    }

    func testTheContractCapsAreThese() {
        XCTAssertEqual(CollabLimits.maxFiles, 4096)
        XCTAssertEqual(CollabLimits.maxProjectTextBytes, 64 * 1024 * 1024)
        XCTAssertEqual(CollabLimits.localFileHeadroom, 64)
        XCTAssertEqual(CollabLimits.localTextHeadroom, 1024 * 1024)
        XCTAssertEqual(CollabProject.Caps(), .init(maxFiles: CollabLimits.maxFiles, maxTextBytes: CollabLimits.maxProjectTextBytes,
                                                   localFileHeadroom: 64, localTextHeadroom: 1024 * 1024))
    }

    func testACreatePastTheFileCapIsRefusedRemotelyAndLocally() throws {
        let hostile = uncapped(2)
        var sections: [Section] = []
        for n in 0...UInt32(CollabLimits.maxFiles) {
            sections.append(try hostile.fileOp { try $0.create(fid(n), kind: .text, path: "f\(n).tex") })
        }
        let guest = CollabProject(replica: 1)
        let errors = guest.receive(sections)
        XCTAssertEqual(errors.map(\.kindName), ["project_full"])
        XCTAssertEqual(errors.first?.op.counter, UInt64(CollabLimits.maxFiles))
        XCTAssertFalse(guest.files.contains(fid(UInt32(CollabLimits.maxFiles))))
        XCTAssertNotNil(guest.text(fid(UInt32(CollabLimits.maxFiles) - 1)))
        let before = guest.digest
        XCTAssertThrowsError(try guest.fileOp { try $0.create(fid(9_999_999), kind: .blob, path: "x.png") }) {
            XCTAssertEqual($0 as? CollabError, .projectFull)
        }
        XCTAssertEqual(guest.digest, before)
        XCTAssertTrue(guest.receive(Array(sections[..<10])).isEmpty)
    }

    func testTextPastTheProjectByteCapIsRefusedRemotelyAndLocally() throws {
        let hostile = uncapped(2)
        var sections: [Section] = []
        for n in 0..<UInt32(3) {
            sections.append(try hostile.fileOp { try $0.create(fid(n), kind: .text, path: "f\(n).tex") })
        }
        // 40 + 40 bytes fit; the third 40 would make 120 > 100.
        for n in 0..<UInt32(3) {
            sections.append(try XCTUnwrap(try hostile.insert(String(repeating: "é", count: 20), at: 0, in: fid(n))))
        }
        let guest = CollabProject(replica: 1, caps: .init(maxFiles: 16, maxTextBytes: 100, localFileHeadroom: 0, localTextHeadroom: 0))
        XCTAssertEqual(guest.receive(sections).map(\.kindName), ["project_full"])
        XCTAssertEqual(guest.text(fid(2))?.text, "")
        // Twenty bytes are left: a local 21-byte insert is refused, 20 fit.
        XCTAssertThrowsError(try guest.insert(String(repeating: "x", count: 21), at: 0, in: fid(2))) {
            XCTAssertEqual($0 as? CollabError, .projectFull)
        }
        XCTAssertNotNil(try guest.insert(String(repeating: "x", count: 20), at: 0, in: fid(2)))
        XCTAssertThrowsError(try guest.insert("y", at: 0, in: fid(0)))
        // Deleting frees nothing (tombstones are stored, §2.4).
        XCTAssertNotNil(guest.delete(at: 0, length: 5, in: fid(0)))
        XCTAssertThrowsError(try guest.insert("y", at: 0, in: fid(0)))
        // A document's own undo re-insertion respects the cap too.
        XCTAssertTrue(guest.text(fid(0))!.reinsertDeleted(CollabID(replica: 2, counter: 0), count: 5).isEmpty)
        XCTAssertNil(guest.text(fid(0))!.validate())
    }

    func testRefusedReplaceLeavesTextAndDigestUnchanged() throws {
        let project = CollabProject(replica: 1, caps: .init(maxFiles: 2, maxTextBytes: 10, localFileHeadroom: 0, localTextHeadroom: 0))
        _ = try project.fileOp { try $0.create(fid(0), kind: .text, path: "a.tex") }
        _ = try project.fileOp { try $0.create(fid(1), kind: .text, path: "b.tex") }
        _ = try project.insert("abc", at: 0, in: fid(0))
        _ = try project.insert("1234567", at: 0, in: fid(1))
        let document = try XCTUnwrap(project.text(fid(0)))
        let digest = project.digest

        XCTAssertThrowsError(try document.replace(utf16Range: 1..<2, with: "x")) {
            XCTAssertEqual($0 as? CollabError, .projectFull)
        }
        XCTAssertEqual(document.text, "abc")
        XCTAssertEqual(project.digest, digest)
    }

    /// §2.4a: this replica's own edits stop the headroom below each cap;
    /// a peer's may use the cap itself.
    func testLocalEditsStopAtTheHeadroomAndRemoteOnesAtTheCap() throws {
        let caps = CollabProject.Caps(maxFiles: 8, maxTextBytes: 100, localFileHeadroom: 2, localTextHeadroom: 10)
        let peer = uncapped(2)
        let me = CollabProject(replica: 1, caps: caps)
        var mine: [Section] = []
        for n in 0..<UInt32(6) { mine.append(try me.fileOp { try $0.create(fid(n), kind: .text, path: "f\(n).tex") }) }
        XCTAssertThrowsError(try me.fileOp { try $0.create(fid(6), kind: .text, path: "f6.tex") }) {
            XCTAssertEqual($0 as? CollabError, .projectFull, "a local create stops 2 below the cap")
        }
        var creates: [Section] = []
        for n in 100..<UInt32(103) { creates.append(try peer.fileOp { try $0.create(fid(n), kind: .text, path: "p\(n).tex") }) }
        XCTAssertEqual(me.receive(creates).map(\.kindName), ["project_full"], "remote creates take the cap: 6 + 2 = 8, the third is refused")
        XCTAssertEqual(me.files.count, 8)

        XCTAssertNotNil(try me.insert(String(repeating: "x", count: 90), at: 0, in: fid(0)))
        XCTAssertThrowsError(try me.insert("y", at: 0, in: fid(1))) {
            XCTAssertEqual($0 as? CollabError, .projectFull, "a local insert stops 10 bytes below the cap")
        }
        XCTAssertTrue(peer.receive(mine).isEmpty)
        let remote = try XCTUnwrap(try peer.insert(String(repeating: "z", count: 10), at: 0, in: fid(0)))
        XCTAssertTrue(me.receive([remote]).isEmpty, "a remote insert may fill the cap")
        XCTAssertEqual(me.storedTextBytes, 100)
    }

    /// §2.4a: replicas that each edit within their headroom while the
    /// others' edits are still on their way all take each other's, and
    /// end with the same project.
    func testConcurrentEditsAtTheCapConverge() throws {
        let caps = CollabProject.Caps(maxFiles: 8, maxTextBytes: 100, localFileHeadroom: 2, localTextHeadroom: 10)
        let a = CollabProject(replica: 1, caps: caps), b = CollabProject(replica: 2, caps: caps), c = CollabProject(replica: 3, caps: caps)
        var base: [Section] = []
        for n in 0..<UInt32(5) { base.append(try a.fileOp { try $0.create(fid(n), kind: .text, path: "f\(n).tex") }) }
        base.append(try XCTUnwrap(try a.insert(String(repeating: "x", count: 85), at: 0, in: fid(0))))
        XCTAssertTrue(b.receive(base).isEmpty)
        XCTAssertTrue(c.receive(base).isEmpty)
        // Each at 85 bytes and 5 files: 5 more bytes and 1 more file are
        // within each one's local room (90, 6); nobody has seen the others'.
        var sent: [[Section]] = []
        for (i, p) in [a, b, c].enumerated() {
            var mine: [Section] = []
            mine.append(try p.fileOp { try $0.create(fid(UInt32(10 + i)), kind: .text, path: "new\(i).tex") })
            mine.append(try XCTUnwrap(try p.insert(String(repeating: ["a", "b", "c"][i], count: 5), at: 0, in: fid(UInt32(1 + i)))))
            sent.append(mine)
        }
        for (i, p) in [a, b, c].enumerated() {
            for (j, s) in sent.enumerated() where j != i {
                XCTAssertTrue(p.receive(s).isEmpty, "replica \(i + 1) takes replica \(j + 1)'s edits")
            }
        }
        XCTAssertEqual(a.digest, b.digest)
        XCTAssertEqual(b.digest, c.digest)
        XCTAssertEqual(a.storedTextBytes, 100)
        XCTAssertEqual(a.files.count, 8)
    }

    /// More concurrent edits than the headroom holds: a replica refuses
    /// one as `project_full` and says so (the session then ends, §2.4a).
    func testConcurrentEditsPastTheCapAreReportedAsProjectFull() throws {
        let caps = CollabProject.Caps(maxFiles: 8, maxTextBytes: 100, localFileHeadroom: 2, localTextHeadroom: 10)
        let peers = (1...3).map { CollabProject(replica: UInt64($0), caps: caps) }
        var base: [Section] = [try peers[0].fileOp { try $0.create(fid(0), kind: .text, path: "f0.tex") }]
        base.append(try XCTUnwrap(try peers[0].insert(String(repeating: "x", count: 80), at: 0, in: fid(0))))
        for p in peers.dropFirst() { XCTAssertTrue(p.receive(base).isEmpty) }
        let edits = try peers.map { try XCTUnwrap(try $0.insert(String(repeating: "y", count: 10), at: 0, in: fid(0))) }
        let errors = peers[0].receive([edits[1], edits[2]])
        XCTAssertEqual(errors.map(\.kindName), ["project_full"], "80 + 10 + 10 fits; the third 10 does not")
        XCTAssertTrue(errors.allSatisfy(\.isProjectFull))
    }

    func testAnUncappedProjectKeepsThePerDocumentLimit() throws {
        let hostile = uncapped(2)
        let create = try hostile.fileOp { try $0.create(fid(0), kind: .text, path: "a.tex") }
        XCTAssertThrowsError(try hostile.insert(String(repeating: "x", count: CollabLimits.maxDocumentBytes + 1), at: 0, in: fid(0))) {
            XCTAssertEqual($0 as? CollabError, .documentFull)
        }
        let guest = CollabProject(replica: 1, caps: .init(maxFiles: 16, maxTextBytes: 10))
        XCTAssertTrue(guest.receive([create]).isEmpty)
        let over = try XCTUnwrap(try hostile.insert(String(repeating: "x", count: 11), at: 0, in: fid(0)))
        XCTAssertEqual(guest.receive([over]).map(\.kindName), ["project_full"])
    }
}
