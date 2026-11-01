import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The exact SHA guard on the non-entry member save through the preview
/// controller's `export` (`ProjectDocuments.exportVerdict`, lane
/// mac-document-files-2, follow-up 1). The helper already locks, checks the
/// durable identity and the disk expectation and renames atomically
/// (STDIO.md); the shell adds the receipt check (path, hash, bytes must be
/// exactly the text sent) and the documented "error after rename" case, which
/// is settled by inspecting disk, never assumed either way.
@MainActor
final class MemberExportGuardTests: XCTestCase {
    private let text = "Chapter, exported.\n"
    private var sent: String { SourceDigest.sha256Hex(text) }

    private func verdict(_ reply: Result<[String: Any], ControllerError>, disk: String?) async -> ProjectDocuments.ExportVerdict {
        await ProjectDocuments.exportVerdict(reply, path: "chapter.tex", text: text) { disk }
    }

    func testReceiptIsASaveOnlyWhenItNamesThePathAndHashesToTheTextSent() async {
        let ok = await verdict(.success(["path": "chapter.tex", "sha256": sent, "bytes": text.utf8.count]), disk: nil)
        XCTAssertEqual(ok, .saved(sha256: sent, afterError: nil))
        let noBytes = await verdict(.success(["path": "chapter.tex", "sha256": sent]), disk: nil)
        XCTAssertEqual(noBytes, .saved(sha256: sent, afterError: nil), "bytes are optional in the receipt")
        let other = SourceDigest.sha256Hex("some other durable text\n")
        guard case .failed(let why) = await verdict(.success(["path": "chapter.tex", "sha256": other, "bytes": 24]), disk: sent) else {
            return XCTFail("a receipt for another text is not a save, even if disk happens to match")
        }
        XCTAssertTrue(why.contains("not the text sent"), why)
        guard case .failed(let wrongPath) = await verdict(.success(["path": "main.tex", "sha256": sent]), disk: nil) else { return XCTFail("wrong path") }
        XCTAssertTrue(wrongPath.contains("names main.tex"), wrongPath)
        guard case .failed(let noHash) = await verdict(.success(["path": "chapter.tex"]), disk: nil) else { return XCTFail("no hash") }
        XCTAssertTrue(noHash.contains("no sha256"), noHash)
        guard case .failed(let bytes) = await verdict(.success(["path": "chapter.tex", "sha256": sent, "bytes": text.utf8.count + 1]), disk: nil) else { return XCTFail("bytes") }
        XCTAssertTrue(bytes.contains("bytes"), bytes)
    }

    func testRefusalsAreConflictsWithTheDiskHashAndOtherErrorsAreCheckedAgainstDisk() async {
        let theirs = SourceDigest.sha256Hex("external\n")
        let modified = await verdict(.failure(.init(message: "save of chapter.tex refused: ModifiedExternally")), disk: theirs)
        XCTAssertEqual(modified, .conflict(.modifiedExternally, theirs: theirs))
        let appeared = await verdict(.failure(.init(message: "save of chapter.tex refused: AlreadyExists")), disk: theirs)
        XCTAssertEqual(appeared, .conflict(.alreadyExists, theirs: theirs))
        let gone = await verdict(.failure(.init(message: "save of chapter.tex refused: DeletedExternally")), disk: nil)
        XCTAssertEqual(gone, .conflict(.deletedExternally, theirs: nil))
        // "An error can follow rename": the file holds exactly the text → saved, error recorded.
        let landed = await verdict(.failure(.init(message: "directory_sync: fsync of project directory failed")), disk: sent)
        XCTAssertEqual(landed, .saved(sha256: sent, afterError: "directory_sync: fsync of project directory failed"))
        // The same error with the file unchanged (or missing) → failed, baseline untouched.
        let notLanded = await verdict(.failure(.init(message: "directory_sync: fsync of project directory failed")), disk: theirs)
        XCTAssertEqual(notLanded, .failed("directory_sync: fsync of project directory failed"))
        let missing = await verdict(.failure(.init(message: "io: permission denied")), disk: nil)
        XCTAssertEqual(missing, .failed("io: permission denied"))
        // A stale durable identity is a plain failure (the buffer is re-flushed next time), not a disk conflict.
        let stale = await verdict(.failure(.init(message: "document_conflict: expected revision 3, store at 4")), disk: theirs)
        XCTAssertEqual(stale, .failed("document_conflict: expected revision 3, store at 4"))
    }

    /// Through the real helper: the receipt the shell accepts is exactly what
    /// landed on disk (hash and bytes), and the member's baseline follows it.
    func testRealExportReceiptMatchesTheFileOnDisk() async throws {
        guard let helper = ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map({ URL(fileURLWithPath: $0) }),
              FileManager.default.isExecutableFile(atPath: helper.path), ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("export-guard-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let main = root.appendingPathComponent("project/main.tex")
        let chapter = root.appendingPathComponent("project/chapter.tex")
        try "\\begin{document}\n\\input{chapter}\n\\end{document}\n".write(to: main, atomically: true, encoding: .utf8)
        try "Chapter.\n".write(to: chapter, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: main), .opened)
        model.attachController(at: helper)
        defer { model.detachController() }
        let start = Date()
        while !(model.result?.revision == model.editorRevision && model.controllerState.durable["main.tex"] != nil) {
            if Date().timeIntervalSince(start) > 15 { throw XCTSkip("timeout: initial preview") }
            try await Task.sleep(nanoseconds: 30_000_000)
        }
        let p = model.project
        let opened = await p.openDiscoveredIncludes()
        XCTAssertEqual(opened, [.opened(path: "chapter.tex")])
        p.switchDocument(to: "chapter.tex")
        let edited = "Chapter, exported through the guard. ünïcödé\n"
        model.updateActiveText(edited)
        let saved = await p.saveDocument("chapter.tex")
        XCTAssertEqual(saved, .saved(path: "chapter.tex", sha256: SourceDigest.sha256Hex(edited)))
        let onDisk = try Data(contentsOf: chapter)
        XCTAssertEqual(SourceDigest.sha256Hex(onDisk), SourceDigest.sha256Hex(edited), "the accepted receipt is the file's hash")
        XCTAssertEqual(onDisk.count, edited.utf8.count)
        XCTAssertFalse(p.isDirty("chapter.tex"))
        XCTAssertTrue(p.status.contains("through the preview controller"), p.status)
        XCTAssertFalse(p.status.contains("after the rename"), "a clean receipt is not reported as an error-after-rename: \(p.status)")
        // Saving the unchanged member again: same expectation, same receipt, still clean.
        let again = await p.saveDocument("chapter.tex")
        XCTAssertEqual(again, .saved(path: "chapter.tex", sha256: SourceDigest.sha256Hex(edited)))
        XCTAssertEqual(try String(contentsOf: chapter, encoding: .utf8), edited)
    }
}
