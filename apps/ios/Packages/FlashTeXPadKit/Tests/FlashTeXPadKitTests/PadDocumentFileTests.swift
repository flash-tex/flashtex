import Foundation
import XCTest
@testable import FlashTeXPadKit

/// Saving the iPad buffer to the file it came from, guarded by the hash of
/// the version it was made from, and the unsaved-text draft store.
final class PadDocumentFileTests: XCTestCase {
    private var dir: URL!
    private var file: URL!

    override func setUpWithError() throws {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("pad-docfile-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        file = dir.appendingPathComponent("main.tex")
        try Data("\\documentclass{article}\n".utf8).write(to: file)
    }

    override func tearDownWithError() throws { try? FileManager.default.removeItem(at: dir) }

    func testReadHashesTheBytes() throws {
        let s = try PadDocumentFile.read(file)
        XCTAssertEqual(s.text, "\\documentclass{article}\n")
        XCTAssertEqual(s.sha256, PadDocumentFile.sha256(Data("\\documentclass{article}\n".utf8)))
        try Data([0xFF, 0xFE, 0x00]).write(to: file)
        XCTAssertThrowsError(try PadDocumentFile.read(file)) { XCTAssertEqual($0 as? PadDocumentFile.ReadError, .notUTF8("main.tex")) }
    }

    func testSaveOverTheVersionItWasMadeFrom() throws {
        let base = try PadDocumentFile.read(file)
        let r = PadDocumentFile.save("edited é\n", to: file, expectedSha256: base.sha256)
        XCTAssertEqual(r, .saved(sha256: PadDocumentFile.sha256(Data("edited é\n".utf8))))
        XCTAssertEqual(try String(contentsOf: file, encoding: .utf8), "edited é\n")
    }

    func testAnExternalChangeIsAConflictAndIsNotOverwritten() throws {
        let base = try PadDocumentFile.read(file)
        try Data("changed on the Mac\n".utf8).write(to: file)
        let r = PadDocumentFile.save("my edit\n", to: file, expectedSha256: base.sha256)
        guard case .conflict(let disk?) = r else { return XCTFail("\(r)") }
        XCTAssertEqual(disk.text, "changed on the Mac\n")
        XCTAssertEqual(try String(contentsOf: file, encoding: .utf8), "changed on the Mac\n", "nothing written")
        // Keeping mine on purpose: save against what the disk holds now.
        XCTAssertEqual(PadDocumentFile.save("my edit\n", to: file, expectedSha256: disk.sha256), .saved(sha256: PadDocumentFile.sha256(Data("my edit\n".utf8))))
    }

    func testADeletedFileIsAConflictUntilRecreatedOnPurpose() throws {
        let base = try PadDocumentFile.read(file)
        try FileManager.default.removeItem(at: file)
        XCTAssertEqual(PadDocumentFile.save("x", to: file, expectedSha256: base.sha256), .conflict(disk: nil))
        XCTAssertFalse(FileManager.default.fileExists(atPath: file.path))
        XCTAssertEqual(PadDocumentFile.save("x", to: file, expectedSha256: nil), .saved(sha256: PadDocumentFile.sha256(Data("x".utf8))))
    }

    func testDraftsAreKeptPerSourceAndOfferedUntilResolved() async throws {
        let store = PadDraftStore(directory: dir.appendingPathComponent("drafts"))
        XCTAssertTrue(store.takeOffered().isEmpty)
        let aSource = PadUnsavedDraft.Source.file(path: file.path, bookmark: nil)
        let a = PadUnsavedDraft(source: aSource, title: "main.tex", text: "a1", diskSha256: "abc", savedAt: Date(timeIntervalSince1970: 1))
        var a2 = a; a2.text = "a2"
        let b = PadUnsavedDraft(source: .file(path: dir.appendingPathComponent("b.tex").path, bookmark: nil), title: "b.tex", text: "b",
                                diskSha256: nil, savedAt: Date(timeIntervalSince1970: 2))
        store.save(a); store.save(a2); store.save(b)
        store.clear(b.source) // b was saved: only its draft goes
        await store.flush()
        // Next launch: this run's drafts become offered ones.
        let offered = store.takeOffered()
        XCTAssertEqual(offered.map(\.draft), [a2], "one per source, the last write; another source's save removed only its own")
        // Editing and saving `a` again in this run never touches the offered copy.
        store.save(a); store.clear(aSource)
        await store.flush()
        XCTAssertEqual(store.takeOffered().map(\.draft), [a2])
        store.removeOffered(offered[0].id)
        await store.flush()
        XCTAssertTrue(store.takeOffered().isEmpty)
        let kept = store.offer(b)
        await store.flush()
        XCTAssertEqual(store.takeOffered(), [kept])
    }
}
