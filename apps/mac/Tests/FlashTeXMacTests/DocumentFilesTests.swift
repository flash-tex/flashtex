import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The shell's file layer (`DocumentFiles.swift`) over the rooted
/// `flashtex-project-files` helper (`crates/project-files`), its direct
/// Foundation fallback, and the lost/late-reply paths through
/// `Fixtures/fake_project_files.py` (a Python double, not the Rust helper).
///
/// Real-helper tests need the binary: `FLASHTEX_PROJECT_FILES` or
/// `crates/project-files/target/{release,debug}/flashtex-project-files`
/// (`cargo build --release --manifest-path crates/project-files/Cargo.toml`);
/// they are skipped otherwise, never silently passed.
@MainActor
final class DocumentFilesTests: XCTestCase {
    static let python = URL(fileURLWithPath: "/usr/bin/python3")
    static let fakeHelper = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().appendingPathComponent("Fixtures/fake_project_files.py")
    static var repoRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url
    }
    static var realHelper: URL? {
        let fm = FileManager.default
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_PROJECT_FILES"], fm.isExecutableFile(atPath: env) {
            return URL(fileURLWithPath: env)
        }
        for profile in ["release", "debug"] {
            let url = repoRoot.appendingPathComponent("crates/project-files/target/\(profile)/flashtex-project-files")
            if fm.isExecutableFile(atPath: url.path) { return url }
        }
        return nil
    }

    private func requireRealHelper() throws -> URL {
        guard let url = Self.realHelper else {
            throw XCTSkip("build crates/project-files (cargo build --release) or set FLASHTEX_PROJECT_FILES")
        }
        return url
    }

    private func tempDir(_ tag: String) throws -> URL {
        // Resolved so the helper's root (its own directory walk) equals what the shell derives.
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-files-\(tag)-\(UUID().uuidString)")
            .resolvingSymlinksInPath()
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: dir) }
        return dir
    }

    private func fake(_ flags: [String]) -> DocumentFilesState.HelperPolicy {
        .executable(Self.python, arguments: [Self.fakeHelper.path] + flags)
    }

    private func disk(_ url: URL) throws -> String { try String(contentsOf: url, encoding: .utf8) }

    private func pump(_ seconds: TimeInterval) async throws {
        try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
    }

    // MARK: rooted helper

    func testHelperSaveRefusesExternalChangesUntilOverwriteOrReload() throws {
        let helper = try requireRealHelper()
        let dir = try tempDir("conflict")
        let url = dir.appendingPathComponent("paper.tex")
        try "Version 1\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.files.policy = .executable(helper, arguments: [])
        XCTAssertEqual(model.openTex(at: url), .opened)
        XCTAssertEqual(model.files.backend, .helper(helper))
        XCTAssertEqual(model.files.helperRoot?.path, dir.path)
        XCTAssertTrue(model.files.helperRunning)
        XCTAssertFalse(model.isDirty)

        // Ordinary save: compare-and-replace against the opened baseline.
        model.updateActiveText("Version 2 (editor)\n")
        XCTAssertTrue(model.saveTex())
        XCTAssertEqual(try disk(url), "Version 2 (editor)\n")
        XCTAssertFalse(model.isDirty)
        XCTAssertNil(model.files.conflict)
        XCTAssertTrue(model.files.status.contains("rooted helper"), model.files.status)

        // Someone else writes the file; our next save must not clobber it.
        try "Version 3 (external)\n".write(to: url, atomically: true, encoding: .utf8)
        model.updateActiveText("Version 2b (editor, unsaved)\n")
        XCTAssertFalse(model.saveTex())
        let conflict = try XCTUnwrap(model.files.conflict)
        XCTAssertEqual(conflict.kind, .modifiedExternally)
        XCTAssertEqual(conflict.url, url)
        XCTAssertTrue(conflict.viaHelper)
        XCTAssertEqual(conflict.ours, SourceDigest.sha256Hex("Version 2 (editor)\n"))
        XCTAssertEqual(conflict.theirs, SourceDigest.sha256Hex("Version 3 (external)\n"))
        XCTAssertEqual(try disk(url), "Version 3 (external)\n", "nothing written")
        XCTAssertEqual(model.activeText, "Version 2b (editor, unsaved)\n", "buffer kept")
        XCTAssertTrue(model.isDirty)
        XCTAssertTrue(model.captureNote?.contains("modified on disk") == true, model.captureNote ?? "")
        XCTAssertEqual(model.checkDiskStatus(), .modified)

        // Explicit decision 1: overwrite.
        XCTAssertTrue(model.overwriteOnDisk())
        XCTAssertEqual(try disk(url), "Version 2b (editor, unsaved)\n")
        XCTAssertFalse(model.isDirty)
        XCTAssertNil(model.files.conflict)
        XCTAssertEqual(model.checkDiskStatus(), .unchanged)

        // Explicit decision 2: reload a clean buffer after an external change
        // noticed by a status check (before any save was attempted).
        try "Version 4 (external)\n".write(to: url, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.checkDiskStatus(), .modified)
        XCTAssertEqual(model.files.conflict?.kind, .modifiedExternally)
        XCTAssertEqual(model.reloadFromDisk(), .opened)
        XCTAssertEqual(model.activeText, "Version 4 (external)\n")
        XCTAssertNil(model.files.conflict)
        XCTAssertFalse(model.isDirty)

        // A dirty buffer is never reloaded over without an explicit discard,
        // and the discarded text stays recoverable.
        model.updateActiveText("Version 4b (editor)\n")
        try "Version 5 (external)\n".write(to: url, atomically: true, encoding: .utf8)
        XCTAssertFalse(model.saveTex())
        XCTAssertEqual(model.reloadFromDisk(), .blockedByUnsavedEdits)
        XCTAssertEqual(model.activeText, "Version 4b (editor)\n")
        XCTAssertEqual(model.reloadFromDisk(dirty: .discard), .opened)
        XCTAssertEqual(model.activeText, "Version 5 (external)\n")
        XCTAssertEqual(model.recoverableBuffer, .init(url: url, text: "Version 4b (editor)\n"))

        // Deleted underneath us: nothing can be overwritten, so Save recreates
        // the file (the helper's `deleted_externally` refusal is answered with
        // an expected-new-file save, which still refuses if a file appeared).
        try FileManager.default.removeItem(at: url)
        XCTAssertEqual(model.checkDiskStatus(), .deleted)
        XCTAssertNil(model.files.conflict, "deletion is a notice, not a blocking conflict")
        XCTAssertTrue(model.captureNote?.contains("deleted on disk") == true, model.captureNote ?? "")
        model.updateActiveText("Version 6 (editor)\n")
        XCTAssertTrue(model.saveTex())
        XCTAssertTrue(model.captureNote?.contains("recreated") == true, model.captureNote ?? "")
        XCTAssertEqual(try disk(url), "Version 6 (editor)\n")
        XCTAssertFalse(model.isDirty)
        XCTAssertNil(model.files.conflict)

        // A buffer that was never on disk (its file vanished before restore)
        // must not clobber a file that appears at its path.
        model.updateActiveText("Version 7 (editor)\n")
        let other = dir.appendingPathComponent("other.tex")
        try "other\n".write(to: other, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.openTex(at: other, dirty: .discard), .opened)
        try FileManager.default.removeItem(at: url)
        XCTAssertTrue(model.restoreDiscardedBuffer())
        XCTAssertNil(model.savedText, "no file on disk: the next save expects a new file")
        try "Version 8 (external, appeared)\n".write(to: url, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.checkDiskStatus(), .created)
        XCTAssertEqual(model.files.conflict?.kind, .alreadyExists)
        XCTAssertFalse(model.saveTex())
        XCTAssertEqual(model.files.conflict?.kind, .alreadyExists)
        XCTAssertEqual(try disk(url), "Version 8 (external, appeared)\n", "nothing written")
        XCTAssertEqual(model.activeText, "Version 7 (editor)\n")
        XCTAssertTrue(model.overwriteOnDisk())
        XCTAssertEqual(try disk(url), "Version 7 (editor)\n")
        XCTAssertEqual(model.files.helperRestarts, 0, "one helper served every operation on this root")
    }

    func testHelperRefusesSymlinksAndNeverWritesOutsideTheRoot() throws {
        let helper = try requireRealHelper()
        let outside = try tempDir("outside")
        let dir = try tempDir("root")
        let target = outside.appendingPathComponent("secret.tex")
        try "outside original\n".write(to: target, atomically: true, encoding: .utf8)
        let link = dir.appendingPathComponent("link.tex")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: target)

        let model = ShellModel()
        model.files.policy = .executable(helper, arguments: [])
        // Opening through a symlink is refused (the helper reads with O_NOFOLLOW).
        XCTAssertEqual(model.openTex(at: link), .readFailed)
        XCTAssertTrue(model.captureNote?.contains("SymlinkComponent") == true, model.captureNote ?? "")

        // A file that becomes a symlink after it was opened: the save is refused,
        // the outside file is untouched, and the buffer stays dirty.
        let real = dir.appendingPathComponent("real.tex")
        try "real\n".write(to: real, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.openTex(at: real), .opened)
        try FileManager.default.removeItem(at: real)
        try FileManager.default.createSymbolicLink(at: real, withDestinationURL: target)
        model.updateActiveText("real edited\n")
        XCTAssertFalse(model.saveTex())
        XCTAssertTrue(model.captureNote?.contains("SymlinkComponent") == true, model.captureNote ?? "")
        XCTAssertEqual(try disk(target), "outside original\n")
        XCTAssertTrue(model.isDirty)
        XCTAssertEqual(model.activeText, "real edited\n")
        XCTAssertFalse(model.overwriteOnDisk(), "force never follows a symlink either")
        XCTAssertEqual(try disk(target), "outside original\n")
    }

    // MARK: direct fallback

    func testDirectFallbackIsReportedAndStillDetectsConflicts() throws {
        let dir = try tempDir("direct")
        let url = dir.appendingPathComponent("paper.tex")
        try "one\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: url), .opened)
        XCTAssertEqual(model.files.backend, .direct(reason: "test: no helper binary"))
        XCTAssertFalse(model.files.helperRunning)
        model.updateActiveText("two\n")
        XCTAssertTrue(model.saveTex())
        XCTAssertEqual(try disk(url), "two\n")
        XCTAssertTrue(model.files.status.contains("directly") && model.files.status.contains("best-effort"), model.files.status)

        try "three (external)\n".write(to: url, atomically: true, encoding: .utf8)
        model.updateActiveText("two-b\n")
        XCTAssertFalse(model.saveTex())
        let conflict = try XCTUnwrap(model.files.conflict)
        XCTAssertEqual(conflict.kind, .modifiedExternally)
        XCTAssertFalse(conflict.viaHelper)
        XCTAssertTrue(conflict.summary.contains("best-effort"), conflict.summary)
        XCTAssertEqual(try disk(url), "three (external)\n")
        XCTAssertTrue(model.isDirty)
        XCTAssertEqual(model.checkDiskStatus(), .modified)
        XCTAssertTrue(model.overwriteOnDisk())
        XCTAssertEqual(try disk(url), "two-b\n")
        XCTAssertNil(model.files.conflict)

        try FileManager.default.removeItem(at: url)
        model.updateActiveText("four\n")
        XCTAssertEqual(model.checkDiskStatus(), .deleted)
        XCTAssertNil(model.files.conflict)
        XCTAssertEqual(model.reloadFromDisk(dirty: .discard), .readFailed, "nothing on disk to reload")
        XCTAssertEqual(model.activeText, "four\n", "failed reload leaves the buffer alone")
        XCTAssertNil(model.recoverableBuffer)
        XCTAssertTrue(model.saveTex(), "recreated")
        XCTAssertEqual(try disk(url), "four\n")
        XCTAssertTrue(model.captureNote?.contains("recreated") == true, model.captureNote ?? "")
    }

    // MARK: fixtures never inherit a real document's identity (#72)

    /// Reload Fixture / Open Compile Result Fixture… must never let Save write
    /// fixture content over a real file: `loadFixtures` detaches `documentURL`
    /// (and `savedText`) the moment a fixture replaces the project, so a later
    /// `saveTex()` falls back to Save As instead of overwriting the old file.
    func testLoadFixturesDetachesRealDocumentIdentity() throws {
        let dir = try tempDir("fixture-detach")
        let url = dir.appendingPathComponent("main.tex")
        let original = "\\begin{document}\nReal content, not a fixture.\n\\end{document}\n"
        try original.write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: url), .opened)
        XCTAssertEqual(model.documentURL, url)
        XCTAssertFalse(model.isDirty)

        let samples = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("Samples")
        model.loadFixtures(request: samples.appendingPathComponent("multipage-request.json"),
                            result: samples.appendingPathComponent("multipage-result.json"))
        XCTAssertNil(model.loadError, model.loadError ?? "")

        XCTAssertNil(model.documentURL, "loading a fixture must detach the real document's URL")
        XCTAssertNil(model.savedText, "loading a fixture must clear the real document's saved baseline")
        XCTAssertNil(model.files.conflict)
        XCTAssertEqual(try disk(url), original, "loading a fixture never touches the real file on disk")
    }

    // MARK: the entry document keeps the opened file's real name

    /// Opening a file not named `main.tex` must not read as `main.tex`:
    /// the entry document (and so the compile request's `entry_path`, every
    /// tab/diagnostic path, and the helper's rooted project) carries the
    /// opened file's actual name. Restoring a discarded buffer keeps its
    /// name too.
    func testOpenedFileKeepsItsRealNameAsTheEntryDocument() throws {
        let dir = try tempDir("entry-name")
        let url = dir.appendingPathComponent("paper.tex")
        try "Hello\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: url), .opened)
        XCTAssertEqual(model.activePath, "paper.tex")
        XCTAssertEqual(model.documents.map(\.path), ["paper.tex"])
        XCTAssertEqual(model.project.entryPath, "paper.tex", "the compile request's entry_path uses the real name")

        // A discarded buffer restored later comes back under its own name.
        model.updateActiveText("Hello edited\n")
        let other = dir.appendingPathComponent("other.tex")
        try "Other\n".write(to: other, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.openTex(at: other, dirty: .discard), .opened)
        XCTAssertEqual(model.activePath, "other.tex")
        XCTAssertTrue(model.restoreDiscardedBuffer())
        XCTAssertEqual(model.activePath, "paper.tex")
        XCTAssertEqual(model.activeText, "Hello edited\n")
    }

    // MARK: lost and late replies (fake helper)

    func testHangingHelperKeepsTheDirtyBufferAndIsRestartedNextTime() throws {
        let dir = try tempDir("hang")
        let url = dir.appendingPathComponent("paper.tex")
        try "base\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.files.policy = fake([])
        model.files.helperTimeout = 0.5
        XCTAssertEqual(model.openTex(at: url), .opened)
        XCTAssertEqual(model.files.backend, .helper(Self.python))
        model.updateActiveText("edited\n")

        // The helper reads the save request and never answers.
        model.files.policy = fake(["--mode", "hang"])
        let started = Date()
        XCTAssertFalse(model.saveTex())
        XCTAssertLessThan(Date().timeIntervalSince(started), 5, "bounded wait")
        XCTAssertTrue(model.isDirty)
        XCTAssertEqual(model.activeText, "edited\n")
        XCTAssertEqual(try disk(url), "base\n")
        XCTAssertTrue(model.captureNote?.contains("no save receipt") == true, model.captureNote ?? "")
        XCTAssertTrue(model.files.status.contains("did not confirm"), model.files.status)
        XCTAssertNil(model.files.conflict, "a lost reply is not a conflict")
        XCTAssertEqual(model.files.helperRestarts, 1, "switching to the hanging fake restarted the helper")

        // Same fake, next request: the stuck process is replaced rather than queued behind.
        XCTAssertFalse(model.saveTex())
        XCTAssertEqual(model.files.helperRestarts, 2)
        XCTAssertTrue(model.isDirty)

        // A responsive helper then saves the still-dirty buffer.
        model.files.policy = fake([])
        XCTAssertTrue(model.saveTex())
        XCTAssertEqual(try disk(url), "edited\n")
        XCTAssertFalse(model.isDirty)
        XCTAssertEqual(model.files.helperRestarts, 3)
    }

    func testHelperExitingMidRequestKeepsTheDirtyBuffer() async throws {
        let dir = try tempDir("exit")
        let url = dir.appendingPathComponent("paper.tex")
        try "base\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.files.policy = fake([])
        model.files.helperTimeout = 5
        XCTAssertEqual(model.openTex(at: url), .opened)
        model.updateActiveText("edited\n")

        model.files.policy = fake(["--mode", "exit", "--exit", "7"])
        let started = Date()
        XCTAssertFalse(model.saveTex())
        XCTAssertLessThan(Date().timeIntervalSince(started), 4, "an exit fails the request at once, not at the deadline")
        XCTAssertTrue(model.isDirty)
        XCTAssertEqual(model.activeText, "edited\n")
        XCTAssertEqual(try disk(url), "base\n")
        XCTAssertTrue(model.captureNote?.contains("exited (7)") == true, model.captureNote ?? "")
        try await pump(0.3)
        XCTAssertEqual(model.files.helperExits, 1)
        XCTAssertFalse(model.files.helperRunning)

        // Reads fail the same honest way; the buffer is untouched.
        XCTAssertEqual(model.openTex(at: url, dirty: .discard), .readFailed)
        XCTAssertEqual(model.activeText, "edited\n")
        XCTAssertNil(model.recoverableBuffer, "a failed open does not consume the discard decision")

        model.files.policy = fake([])
        XCTAssertTrue(model.saveTex())
        XCTAssertEqual(try disk(url), "edited\n")
        XCTAssertFalse(model.isDirty)
    }

    func testLateSaveReceiptIsReconciledWithoutLosingLaterEdits() async throws {
        let dir = try tempDir("late")
        let url = dir.appendingPathComponent("paper.tex")
        try "base\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.files.policy = fake([])
        model.files.helperTimeout = 0.4
        XCTAssertEqual(model.openTex(at: url), .opened)

        // Case 1: the reply arrives after the wait; nothing changed meanwhile.
        model.files.policy = fake(["--mode", "late", "--delay", "1.2"])
        model.updateActiveText("edited once\n")
        XCTAssertFalse(model.saveTex(), "no receipt within the wait")
        XCTAssertTrue(model.isDirty)
        try await pump(1.6)
        XCTAssertEqual(try disk(url), "edited once\n", "the fake did write before answering late")
        XCTAssertFalse(model.isDirty, "late receipt for exactly the sent text marks it saved")
        XCTAssertEqual(model.savedText, "edited once\n")
        XCTAssertTrue(model.captureNote?.contains("Late confirmation") == true, model.captureNote ?? "")
        XCTAssertEqual(model.files.lateReplies.count, 1)

        // Case 2: the user kept typing before the late receipt: the receipt's
        // text becomes the baseline, the newer edits stay dirty.
        model.updateActiveText("edited twice\n")
        XCTAssertFalse(model.saveTex())
        model.updateActiveText("edited thrice (after the timeout)\n")
        try await pump(1.6)
        XCTAssertEqual(try disk(url), "edited twice\n")
        XCTAssertEqual(model.savedText, "edited twice\n")
        XCTAssertTrue(model.isDirty, "edits after the late-confirmed save are still unsaved")
        XCTAssertEqual(model.activeText, "edited thrice (after the timeout)\n")
        XCTAssertEqual(model.files.lateReplies.count, 2)

        // Case 3: a late *conflict* is surfaced, never applied.
        try "external\n".write(to: url, atomically: true, encoding: .utf8)
        XCTAssertFalse(model.saveTex())
        XCTAssertNil(model.files.conflict, "not yet known")
        try await pump(1.6)
        XCTAssertEqual(model.files.conflict?.kind, .modifiedExternally)
        XCTAssertEqual(try disk(url), "external\n")
        XCTAssertTrue(model.isDirty)
        XCTAssertEqual(model.activeText, "edited thrice (after the timeout)\n")
    }

    func testGarbageReplyIsAProtocolFailureNotASave() throws {
        let dir = try tempDir("garbage")
        let url = dir.appendingPathComponent("paper.tex")
        try "base\n".write(to: url, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.files.policy = fake([])
        model.files.helperTimeout = 0.5
        XCTAssertEqual(model.openTex(at: url), .opened)
        model.files.policy = fake(["--mode", "garbage"])
        model.updateActiveText("edited\n")
        XCTAssertFalse(model.saveTex())
        XCTAssertTrue(model.isDirty)
        XCTAssertNil(model.files.conflict)
        // The fake wrote before answering garbage: the shell cannot know, so it
        // stays dirty; a status check then reports the truth.
        XCTAssertEqual(try disk(url), "edited\n")
        model.files.policy = fake([])
        XCTAssertEqual(model.checkDiskStatus(), .modified)
        XCTAssertEqual(model.files.conflict?.kind, .modifiedExternally)
        XCTAssertEqual(model.files.conflict?.theirs, SourceDigest.sha256Hex("edited\n"))
        XCTAssertTrue(model.overwriteOnDisk())
        XCTAssertFalse(model.isDirty)
    }

    // MARK: reviewed reload (direct / project-files route)

    func testLineChangeSummaryCountsAddedAndRemovedLines() {
        XCTAssertEqual(ShellModel.lineChanges(from: "a\nb\nc\n", to: "a\nc\nd\n").added, 1)
        XCTAssertEqual(ShellModel.lineChanges(from: "a\nb\nc\n", to: "a\nc\nd\n").removed, 1)
        XCTAssertEqual(ShellModel.lineChanges(from: "x\n", to: "x\n").added, 0)
        XCTAssertEqual(ShellModel.lineChanges(from: "", to: "one\ntwo\n").added, 2)
        XCTAssertEqual(ShellModel.lineChanges(from: "one\none\n", to: "one\n").removed, 1)
    }

    func testReloadIsReviewedAndPinnedToTheReviewedSnapshot() async throws {
        let dir = try tempDir("review")
        let url = dir.appendingPathComponent("paper.tex")
        try "line 1\nline 2\nline 3\n".write(to: url, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.files.policy = fake([])
        XCTAssertEqual(model.openTex(at: url), .opened)
        model.updateActiveText("line 1\nline 2 (edited)\nline 3\n")

        // Review: what the disk snapshot would change, without touching the buffer.
        try "line 1\nline 3\nline 4\nline 5\n".write(to: url, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.checkDiskStatus(), .modified)
        let review = try XCTUnwrap(model.prepareReload())
        XCTAssertFalse(review.viaController)
        XCTAssertTrue(review.bufferDirty)
        XCTAssertEqual(review.diskText, "line 1\nline 3\nline 4\nline 5\n")
        XCTAssertEqual(review.diskSha256, SourceDigest.sha256Hex(review.diskText))
        XCTAssertEqual(review.bytesBefore, 30); XCTAssertEqual(review.bytesAfter, 28)
        XCTAssertEqual(review.linesAdded, 2, "line 4, line 5")
        XCTAssertEqual(review.linesRemoved, 1, "line 2 (edited); the trailing empty line stays")
        XCTAssertTrue(review.summary.contains("30 → 28 bytes") && review.summary.contains("+2 / −1 lines"), review.summary)
        XCTAssertTrue(review.summary.contains("unsaved edits are replaced"), review.summary)
        XCTAssertEqual(model.activeText, "line 1\nline 2 (edited)\nline 3\n", "review changes nothing")

        // Confirmation is gated on the dirty decision and pinned to the reviewed hash.
        do { let got = await model.confirmReload(review); XCTAssertEqual(got, .blockedByUnsavedEdits) }
        do { let got = await model.confirmReload(review, dirty: .saveFirst); XCTAssertEqual(got, .saveFailed) }
        try "changed after review\n".write(to: url, atomically: true, encoding: .utf8)
        do { let got = await model.confirmReload(review, dirty: .discard); XCTAssertEqual(got, .readFailed) }
        XCTAssertTrue(model.captureNote?.contains("changed again") == true, model.captureNote ?? "")
        XCTAssertEqual(model.activeText, "line 1\nline 2 (edited)\nline 3\n")
        XCTAssertNil(model.recoverableBuffer, "a refused reload does not consume the discard")
        XCTAssertEqual(model.files.lastDiskState, .modified)

        // A fresh review of the current snapshot imports it; the edits stay recoverable.
        do { let got = await model.reloadFromDiskReviewed(dirty: .discard); XCTAssertEqual(got, .opened) }
        XCTAssertEqual(model.activeText, "changed after review\n")
        XCTAssertFalse(model.isDirty)
        XCTAssertNil(model.files.conflict)
        XCTAssertEqual(model.recoverableBuffer, .init(url: url, text: "line 1\nline 2 (edited)\nline 3\n"))
        let identical = try XCTUnwrap(model.prepareReload())
        XCTAssertTrue(identical.identical)
        XCTAssertTrue(identical.summary.contains("identical"), identical.summary)

        // The synchronous direct reload is the same reviewed path.
        try "sync reload\n".write(to: url, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.reloadFromDisk(), .opened)
        XCTAssertEqual(model.activeText, "sync reload\n")
        try FileManager.default.removeItem(at: url)
        XCTAssertNil(model.prepareReload())
        XCTAssertTrue(model.captureNote?.contains("does not exist") == true, model.captureNote ?? "")
    }

    // MARK: multi-file open / detach / reopen (real helper, real files)

    func testMultiFileOpenDetachReopenAcrossProjectRoots() throws {
        let helper = try requireRealHelper()
        let dirA = try tempDir("A"), dirB = try tempDir("B").appendingPathComponent("chapters")
        try FileManager.default.createDirectory(at: dirB, withIntermediateDirectories: true)
        let a = dirA.appendingPathComponent("a.tex"), b = dirB.appendingPathComponent("b.tex")
        try "A1\n".write(to: a, atomically: true, encoding: .utf8)
        try "B1\n".write(to: b, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.files.policy = .executable(helper, arguments: [])
        XCTAssertEqual(model.openTex(at: a), .opened)
        XCTAssertEqual(model.files.helperRoot?.path, dirA.path)
        model.updateActiveText("A2\n")
        XCTAssertTrue(model.saveTex())

        // Another root: the helper is rebound (one process per project directory).
        XCTAssertEqual(model.openTex(at: b), .opened)
        XCTAssertEqual(model.files.helperRoot?.path, dirB.path)
        XCTAssertEqual(model.files.helperRestarts, 1)
        XCTAssertEqual(model.activeText, "B1\n")
        model.updateActiveText("B2\n")
        XCTAssertTrue(model.saveTex())
        XCTAssertEqual(try disk(b), "B2\n")

        // Detach: no process; the next operation respawns one for the right root.
        model.files.detachHelper()
        XCTAssertFalse(model.files.helperRunning)
        XCTAssertNil(model.files.helperRoot)
        try "A3 (external while b was open)\n".write(to: a, atomically: true, encoding: .utf8)
        model.updateActiveText("B3\n")
        XCTAssertEqual(model.openTex(at: a), .blockedByUnsavedEdits)
        XCTAssertEqual(model.openTex(at: a, dirty: .saveFirst), .opened)
        XCTAssertEqual(try disk(b), "B3\n", "save-first wrote b through a fresh helper on b's root")
        XCTAssertEqual(model.activeText, "A3 (external while b was open)\n")
        XCTAssertEqual(model.files.helperRoot?.path, dirA.path)
        XCTAssertTrue(model.files.helperRunning)
        XCTAssertFalse(model.isDirty)

        // Reopen b: its saved text comes back; a conflict on a does not leak to b.
        try "A4 (external)\n".write(to: a, atomically: true, encoding: .utf8)
        model.updateActiveText("A3b\n")
        XCTAssertFalse(model.saveTex())
        XCTAssertEqual(model.files.conflict?.url, a)
        XCTAssertEqual(model.openTex(at: b, dirty: .discard), .opened)
        XCTAssertNil(model.files.conflict, "opening another file clears the previous file's conflict")
        XCTAssertEqual(model.activeText, "B3\n")
        XCTAssertEqual(model.recoverableBuffer, .init(url: a, text: "A3b\n"))
        XCTAssertTrue(model.restoreDiscardedBuffer())
        XCTAssertEqual(model.documentURL, a)
        XCTAssertEqual(model.savedText, "A4 (external)\n", "baseline is what is on disk now")
        XCTAssertTrue(model.isDirty)
        // The restore re-baselined on the current disk content, so saving the
        // restored buffer is a deliberate, ordinary save.
        XCTAssertTrue(model.saveTex())
        XCTAssertEqual(try disk(a), "A3b\n")
        XCTAssertFalse(model.isDirty)
        XCTAssertEqual(try disk(b), "B3\n")
    }
}

/// The reviewed reload and disk status through the real
/// `flashtex-preview-controller` (`reload {…, user_approved:true}` and
/// `file_status`, STDIO.md) with the real compiler. Skipped unless
/// `FLASHTEX_PREVIEW_CONTROLLER` and `FLASHTEX_COMPILER` point at built binaries.
@MainActor
final class DocumentFilesControllerTests: XCTestCase {
    static var helper: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) }
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 15, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timed out waiting for \(what)"); throw XCTSkip("timeout: \(what)") }
            try await Task.sleep(nanoseconds: 30_000_000)
        }
    }

    func testControllerReloadIsReviewedPinnedAndImportedIntoTheDurableSource() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("pc-reload-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = root.appendingPathComponent("project/main.tex")
        let v1 = "\\begin{document}\nReload me.\n\\end{document}\n"
        try v1.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: helper)
        try await waitUntil("initial preview") { model.result?.revision == model.editorRevision && model.controllerState.durable["main.tex"] != nil }
        XCTAssertTrue(model.controllerRoutesFiles)

        // Disk status through the helper: unchanged, then an external write is a conflict.
        do { let got = await model.refreshDiskStatus(); XCTAssertEqual(got, .unchanged) }
        let v2 = "\\begin{document}\nReloaded from disk.\nSecond line.\n\\end{document}\n"
        try v2.write(to: tex, atomically: true, encoding: .utf8)
        do { let got = await model.refreshDiskStatus(); XCTAssertEqual(got, .modified) }
        let conflict = try XCTUnwrap(model.files.conflict)
        XCTAssertTrue(conflict.viaHelper)
        XCTAssertEqual(conflict.kind, .modifiedExternally)
        XCTAssertEqual(conflict.theirs, SourceDigest.sha256Hex(v2))

        // Typed edits are durable in the ledger but not on disk: still not an external change.
        model.updateActiveText("\\begin{document}\nReload me, edited.\n\\end{document}\n")
        try await waitUntil("edit durable") { model.inFlightRevision == nil && model.controllerState.textByDurable["main.tex"]?.values.contains { $0 == model.activeText } == true }
        do { let got = await model.refreshDiskStatus(); XCTAssertEqual(got, .modified, "disk still differs from the baseline, not from the typed text") }

        // Review: pinned to the durable identity and the disk hash; nothing changes yet.
        let review = try XCTUnwrap(model.prepareReload())
        XCTAssertTrue(review.viaController)
        XCTAssertEqual(review.durable?.revision, model.controllerState.durable["main.tex"]?.revision)
        XCTAssertEqual(review.diskText, v2)
        XCTAssertTrue(review.bufferDirty)
        XCTAssertTrue(review.summary.contains("preview controller") && review.summary.contains("lines"), review.summary)
        XCTAssertEqual(model.activeText, "\\begin{document}\nReload me, edited.\n\\end{document}\n")

        // The file changes again after the review: the helper refuses, nothing replaced.
        let v3 = "\\begin{document}\nChanged after review.\n\\end{document}\n"
        try v3.write(to: tex, atomically: true, encoding: .utf8)
        do { let got = await model.confirmReload(review, dirty: .discard); XCTAssertEqual(got, .readFailed) }
        XCTAssertTrue(model.captureNote?.contains("refused") == true, model.captureNote ?? "")
        XCTAssertEqual(model.activeText, "\\begin{document}\nReload me, edited.\n\\end{document}\n")
        XCTAssertTrue(model.isDirty)
        XCTAssertNil(model.recoverableBuffer)
        XCTAssertEqual(model.reloadFromDisk(dirty: .discard), .readFailed, "the synchronous direct reload never bypasses the controller")

        // Reviewed again and confirmed: the ledger holds v3, the buffer shows it, the
        // preview binds to the new revision, and the previous text stays recoverable.
        let before = model.activeText
        let durableBefore = try XCTUnwrap(model.controllerState.durable["main.tex"]).revision
        let fresh = try XCTUnwrap(model.prepareReload())
        XCTAssertEqual(fresh.diskText, v3)
        do { let got = await model.confirmReload(fresh); XCTAssertEqual(got, .blockedByUnsavedEdits) }
        do { let got = await model.confirmReload(fresh, dirty: .discard); XCTAssertEqual(got, .opened) }
        XCTAssertEqual(model.activeText, v3)
        XCTAssertEqual(model.savedText, v3)
        XCTAssertFalse(model.isDirty)
        XCTAssertNil(model.files.conflict)
        XCTAssertEqual(model.files.lastDiskState, .unchanged)
        XCTAssertEqual(model.recoverableBuffer, .init(url: tex, text: before))
        let durableAfter = try XCTUnwrap(model.controllerState.durable["main.tex"]).revision
        XCTAssertGreaterThan(durableAfter, durableBefore)
        XCTAssertEqual(model.controllerState.textByDurable["main.tex"]?[durableAfter], v3)
        XCTAssertEqual(try String(contentsOf: tex, encoding: .utf8), v3, "reload never writes disk")
        try await waitUntil("preview for the reloaded revision") { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        do { let got = await model.refreshDiskStatus(); XCTAssertEqual(got, .unchanged) }
        let status = await model.controllerFileStatus(path: "main.tex")
        XCTAssertEqual(status?.state, "matches_source")

        // Deleted on disk: a notice through the helper, and nothing to review.
        try FileManager.default.removeItem(at: tex)
        do { let got = await model.refreshDiskStatus(); XCTAssertEqual(got, .deleted) }
        XCTAssertNil(model.files.conflict)
        XCTAssertNil(model.prepareReload())
        model.detachController()
    }
}
