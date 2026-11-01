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
        XCTAssertEqual(CollabProject.Caps(), .init(maxFiles: CollabLimits.maxFiles, maxTextBytes: CollabLimits.maxProjectTextBytes))
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
        let guest = CollabProject(replica: 1, caps: .init(maxFiles: 16, maxTextBytes: 100))
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
        let project = CollabProject(replica: 1, caps: .init(maxFiles: 2, maxTextBytes: 10))
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
