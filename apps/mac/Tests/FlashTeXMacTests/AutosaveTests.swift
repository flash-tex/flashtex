import XCTest
@testable import FlashTeXMac

/// Autosave (owner: "autosave should be on by default"). Real time is
/// suppressed under XCTest (`ShellModel.autosaveSuppressedUnderTest`, same
/// reasoning as `PreviewHUD.lingerSuppressed`): these tests drive the write
/// through `flushPendingAutosave()` instead of waiting out `autosaveInterval`.
///
/// `EditorPreferences.shared` is a process-wide singleton over
/// `UserDefaults.standard`; `autosave` is restored in `tearDown` so this
/// suite never leaks a changed default into another test or a real launch.
@MainActor
final class AutosaveTests: XCTestCase {
    private var originalAutosave = true

    override func setUp() {
        super.setUp()
        originalAutosave = EditorPreferences.shared.autosave
    }

    override func tearDown() {
        EditorPreferences.shared.autosave = originalAutosave
        super.tearDown()
    }

    private func tempDir(_ tag: String) throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-autosave-\(tag)-\(UUID().uuidString)")
            .resolvingSymlinksInPath()
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: dir) }
        return dir
    }

    private func disk(_ url: URL) throws -> String { try String(contentsOf: url, encoding: .utf8) }

    func testDefaultOnWritesToDiskAfterFlush() throws {
        EditorPreferences.shared.autosave = true
        let dir = try tempDir("on")
        let url = dir.appendingPathComponent("paper.tex")
        try "one\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: url), .opened)

        model.updateActiveText("two (unsaved)\n")
        XCTAssertEqual(try disk(url), "one\n", "nothing written until the debounce fires")
        model.flushPendingAutosave()
        XCTAssertEqual(try disk(url), "two (unsaved)\n")
        XCTAssertFalse(model.isDirty)
    }

    func testExplicitlyOffNeverWritesInTheBackground() throws {
        EditorPreferences.shared.autosave = false
        let dir = try tempDir("off")
        let url = dir.appendingPathComponent("paper.tex")
        try "one\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: url), .opened)

        model.updateActiveText("two (unsaved)\n")
        model.flushPendingAutosave()
        XCTAssertEqual(try disk(url), "one\n", "an explicit off must stay off")
        XCTAssertTrue(model.isDirty, "the edit is still only in the buffer; Command-S still works")
    }

    /// A buffer with no file yet must never autosave: `saveTex()` falls back
    /// to `saveTexAs()` for a `nil` `documentURL`, which would pop a Save
    /// panel while the user is mid-keystroke.
    func testNeverSavesABufferWithNoFileYet() {
        EditorPreferences.shared.autosave = true
        let model = ShellModel()
        model.updateActiveText("some text with no file behind it\n")
        model.flushPendingAutosave() // must be a no-op, not a Save panel
        XCTAssertNil(model.documentURL)
        XCTAssertTrue(model.isDirty)
    }

    func testFlushWithNothingPendingIsANoOp() throws {
        let dir = try tempDir("idle")
        let url = dir.appendingPathComponent("paper.tex")
        try "one\n".write(to: url, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: url), .opened)
        model.flushPendingAutosave() // no edit since open: nothing dirty, nothing to write
        XCTAssertEqual(try disk(url), "one\n")
    }

    /// Opens `main.tex` (entry, `\input{chapter}`) and `chapter.tex` as a
    /// project member, the way the sidebar does, with no helper attached.
    private func openProject(_ tag: String) async throws -> (ShellModel, main: URL, chapter: URL) {
        let dir = try tempDir(tag)
        let main = dir.appendingPathComponent("main.tex"), chapter = dir.appendingPathComponent("chapter.tex")
        try "\\input{chapter}\n".write(to: main, atomically: true, encoding: .utf8)
        try "Chapter.\n".write(to: chapter, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: main), .opened)
        let opened = await model.project.openDocument("chapter.tex")
        XCTAssertEqual(opened, .opened(path: "chapter.tex"))
        return (model, main, chapter)
    }

    /// Owner: "the autosave isn't working". A non-entry project member was
    /// never autosaved (`scheduleAutosave` was scoped to the entry document).
    func testNonEntryMemberEditAutosaves() async throws {
        EditorPreferences.shared.autosave = true
        let (model, _, chapter) = try await openProject("member")
        model.project.switchDocument(to: "chapter.tex")
        XCTAssertEqual(model.activePath, "chapter.tex")
        model.updateActiveText("Chapter, edited.\n")
        XCTAssertEqual(try disk(chapter), "Chapter.\n")
        model.flushPendingAutosave()
        XCTAssertEqual(try disk(chapter), "Chapter, edited.\n")
        XCTAssertFalse(model.project.isDirty("chapter.tex"))
    }

    /// Typing in the entry and switching tabs before the debounce fires must
    /// not drop the pending save — nor may a later edit in another document
    /// cancel it.
    func testSwitchingDocumentsKeepsAPendingAutosave() async throws {
        EditorPreferences.shared.autosave = true
        let (model, main, chapter) = try await openProject("switch")
        model.updateActiveText("\\input{chapter}\n% entry edit\n")
        model.project.switchDocument(to: "chapter.tex")
        model.flushPendingAutosave()
        XCTAssertEqual(try disk(main), "\\input{chapter}\n% entry edit\n", "switching away must not drop the entry's pending autosave")
        XCTAssertFalse(model.project.isDirty("main.tex"))

        model.project.switchDocument(to: "main.tex")
        model.updateActiveText("\\input{chapter}\n% second entry edit\n")
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, edited.\n") // re-arms the debounce from another document
        model.flushPendingAutosave()
        XCTAssertEqual(try disk(main), "\\input{chapter}\n% second entry edit\n")
        XCTAssertEqual(try disk(chapter), "Chapter, edited.\n")
    }

    /// Autosave goes through the normal conflict-checked save: a file changed
    /// on disk is never clobbered, and no panel is raised.
    func testAutosaveNeverClobbersAFileChangedOnDisk() async throws {
        EditorPreferences.shared.autosave = true
        let (model, main, chapter) = try await openProject("conflict")
        try "\\input{chapter}\n% external\n".write(to: main, atomically: true, encoding: .utf8)
        try "Chapter, external.\n".write(to: chapter, atomically: true, encoding: .utf8)
        model.updateActiveText("\\input{chapter}\n% ours\n")
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, ours.\n")
        model.flushPendingAutosave()
        XCTAssertEqual(try disk(main), "\\input{chapter}\n% external\n")
        XCTAssertEqual(try disk(chapter), "Chapter, external.\n")
        XCTAssertTrue(model.project.isDirty("main.tex"))
        XCTAssertTrue(model.project.isDirty("chapter.tex"))
    }

    /// An edit made while a helper-routed save of the same document was in
    /// flight was skipped by autosave and never rescheduled: the disk kept
    /// the older text. `enqueueSave` re-arms autosave when the buffer moved.
    func testEditDuringAnInFlightSaveReArmsAutosave() async throws {
        EditorPreferences.shared.autosave = true
        let dir = try tempDir("inflight")
        let url = dir.appendingPathComponent("paper.tex")
        try "one\n".write(to: url, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: url), .opened)
        let entry = model.project.entryPath
        await model.enqueueSave(entry) {
            // The edit lands mid-save; its own autosave was skipped as in flight.
            if let i = model.documents.firstIndex(where: { $0.path == entry }) { model.documents[i].text = "two, typed during the save\n" }
        }.value
        model.flushPendingAutosave()
        XCTAssertEqual(try disk(url), "two, typed during the save\n")
        XCTAssertFalse(model.isDirty)
    }

    /// Command-S and autosave of the same path run one after the other, never
    /// concurrently (two exports with the same disk expectation: the second is
    /// refused as a spurious conflict).
    func testSavesOfTheSamePathAreSerialized() async throws {
        let model = ShellModel()
        var order: [String] = []
        let first = model.enqueueSave("main.tex") {
            order.append("first start")
            try? await Task.sleep(nanoseconds: 50_000_000)
            order.append("first end")
        }
        let second = model.enqueueSave("main.tex") { order.append("second start") }
        await first.value
        await second.value
        XCTAssertEqual(order, ["first start", "first end", "second start"])
    }

    /// Saving document B must not erase the conflict recorded for document A
    /// (autosave skips a member by that record).
    func testSavingAnotherMemberKeepsARecordedConflict() async throws {
        EditorPreferences.shared.autosave = true
        let (model, main, chapter) = try await openProject("conflict-kept")
        try "Other.\n".write(to: main.deletingLastPathComponent().appendingPathComponent("other.tex"), atomically: true, encoding: .utf8)
        let opened = await model.project.openDocument("other.tex")
        XCTAssertEqual(opened, .opened(path: "other.tex"))
        try "Chapter, external.\n".write(to: chapter, atomically: true, encoding: .utf8)
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, ours.\n")
        guard case .conflict = model.project.saveDocumentNow("chapter.tex") else { return XCTFail("expected a conflict") }
        model.project.switchDocument(to: "other.tex")
        model.updateActiveText("Other, ours.\n")
        guard case .saved = model.project.saveDocumentNow("other.tex") else { return XCTFail("expected a save") }
        XCTAssertEqual(model.project.saveConflict(for: "chapter.tex")?.url.standardizedFileURL, chapter.standardizedFileURL)
        model.flushPendingAutosave()
        XCTAssertEqual(try disk(chapter), "Chapter, external.\n")
    }

    /// Overwrite writes `activeText`: with another tab active (a queued ⌘S
    /// whose conflict answered after a switch) it must not force that
    /// document's text into the entry file.
    func testOverwriteRefusesWhileAnotherDocumentIsActive() async throws {
        let (model, main, _) = try await openProject("overwrite-switched")
        try "\\input{chapter}\n% external\n".write(to: main, atomically: true, encoding: .utf8)
        model.updateActiveText("\\input{chapter}\n% ours\n")
        XCTAssertFalse(model.saveEntryTex())
        XCTAssertNotNil(model.files.conflict)
        model.project.switchDocument(to: "chapter.tex")
        XCTAssertFalse(model.overwriteOnDisk())
        XCTAssertEqual(try disk(main), "\\input{chapter}\n% external\n", "the chapter's text must never land in main.tex")
        XCTAssertNotNil(model.files.conflict)
    }

    /// The direct save replaces the file through a temp file and a rename;
    /// the replacement keeps the original's mode and extended attributes (a
    /// 0600 file must not turn world-readable on the first autosave).
    func testDirectSaveKeepsPermissionsAndExtendedAttributes() throws {
        EditorPreferences.shared.autosave = true
        let dir = try tempDir("mode")
        let url = dir.appendingPathComponent("paper.tex")
        try "one\n".write(to: url, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
        let tag = Array("kept".utf8)
        XCTAssertEqual(setxattr(url.path, "org.flashtex.test", tag, tag.count, 0, 0), 0)
        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: url), .opened)
        model.updateActiveText("two\n")
        model.flushPendingAutosave()
        XCTAssertEqual(try disk(url), "two\n")
        let mode = try FileManager.default.attributesOfItem(atPath: url.path)[.posixPermissions] as? Int
        XCTAssertEqual(mode, 0o600)
        var buffer = [UInt8](repeating: 0, count: 16)
        let n = getxattr(url.path, "org.flashtex.test", &buffer, buffer.count, 0, 0)
        XCTAssertEqual(n, tag.count)
        XCTAssertEqual(Array(buffer.prefix(max(0, n))), tag)
        let leftovers = try FileManager.default.contentsOfDirectory(atPath: dir.path).filter { $0.contains("flashtex-save-") }
        XCTAssertEqual(leftovers, [])
    }

    /// `files.conflict` is the entry's record: a member save neither clears
    /// it (saving chapter.tex after main.tex conflicted) nor raises it (a
    /// conflict on chapter.tex must not block the entry's autosave).
    func testMemberSavesNeverTouchTheEntryConflict() async throws {
        EditorPreferences.shared.autosave = true
        let (model, main, chapter) = try await openProject("entry-record")
        try "\\input{chapter}\n% external\n".write(to: main, atomically: true, encoding: .utf8)
        model.updateActiveText("\\input{chapter}\n% ours\n")
        XCTAssertFalse(model.saveEntryTex())
        let entryConflict = model.files.conflict
        XCTAssertNotNil(entryConflict)
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, ours.\n")
        guard case .saved = model.project.saveDocumentNow("chapter.tex") else { return XCTFail("expected a save") }
        XCTAssertEqual(model.files.conflict, entryConflict, "saving a member must not clear the entry's conflict")

        let (other, otherMain, otherChapter) = try await openProject("member-record")
        try "Chapter, external.\n".write(to: otherChapter, atomically: true, encoding: .utf8)
        other.project.switchDocument(to: "chapter.tex")
        other.updateActiveText("Chapter, ours.\n")
        guard case .conflict = other.project.saveDocumentNow("chapter.tex") else { return XCTFail("expected a conflict") }
        XCTAssertNil(other.files.conflict, "a member conflict is recorded on the member, not the entry")
        other.project.switchDocument(to: "main.tex")
        other.updateActiveText("\\input{chapter}\n% entry edit\n")
        other.flushPendingAutosave()
        XCTAssertEqual(try disk(otherMain), "\\input{chapter}\n% entry edit\n")
    }

    /// "Save first" before opening another file, with a member tab active:
    /// `saveTex()` wrote `activeText` (the member's text) into main.tex.
    /// The entry file must get the entry buffer, and the member its own.
    func testSaveFirstWithAMemberActiveWritesEachDocumentsOwnText() async throws {
        EditorPreferences.shared.autosave = false // only the save-first path writes
        let (model, main, chapter) = try await openProject("save-first")
        model.updateActiveText("\\input{chapter}\n% entry edit\n")
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, edited.\n")
        let next = main.deletingLastPathComponent().appendingPathComponent("next.tex")
        try "Next.\n".write(to: next, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.openTex(at: next, dirty: .saveFirst), .opened)
        XCTAssertEqual(try disk(main), "\\input{chapter}\n% entry edit\n", "main.tex must hold the entry's text, never the active member's")
        XCTAssertEqual(try disk(chapter), "Chapter, edited.\n")
    }

    /// Discard before opening another file, with a member tab active: the
    /// recoverable buffer (and its snapshot under main.tex's URL) is the
    /// entry's text, never the member's — a later restore would otherwise put
    /// chapter text into main.tex.
    func testDiscardWithAMemberActiveKeepsTheEntrysText() async throws {
        EditorPreferences.shared.autosave = false
        let (model, main, _) = try await openProject("discard")
        model.updateActiveText("\\input{chapter}\n% entry edit\n")
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, edited.\n")
        let next = main.deletingLastPathComponent().appendingPathComponent("next.tex")
        try "Next.\n".write(to: next, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.openTex(at: next, dirty: .discard), .opened)
        XCTAssertEqual(model.recoverableBuffer?.url, main)
        XCTAssertEqual(model.recoverableBuffer?.text, "\\input{chapter}\n% entry edit\n")
        XCTAssertEqual(try disk(main), "\\input{chapter}\n")
    }

    /// A brand-new file saved directly gets the ordinary umask-derived mode,
    /// not the private 0600 the temp file uses when replacing an existing file.
    func testDirectSaveOfANewFileUsesTheUmaskMode() throws {
        let dir = try tempDir("newmode")
        let url = dir.appendingPathComponent("fresh.tex")
        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        guard case .saved = model.files.save(url, text: "fresh\n", expected: .newFile, force: false) else { return XCTFail("expected a save") }
        let mask = umask(0); umask(mask)
        let mode = try FileManager.default.attributesOfItem(atPath: url.path)[.posixPermissions] as? Int
        XCTAssertEqual(mode, 0o666 & ~Int(mask))
        XCTAssertEqual(try disk(url), "fresh\n")
    }

    /// Save As writes the entry buffer; with project members open it must not
    /// land on a member's file (replacing that document on disk) nor move the
    /// project root out from under the open members.
    func testSaveAsRefusesAnOpenMembersFileAndAnotherFolder() async throws {
        let (model, main, chapter) = try await openProject("save-as")
        model.updateActiveText("\\input{chapter}\n% entry edit\n")
        model.project.switchDocument(to: "chapter.tex")
        XCTAssertFalse(model.saveTexAs(to: chapter))
        XCTAssertEqual(try disk(chapter), "Chapter.\n", "the entry's text must never replace an open member on disk")
        let elsewhere = try tempDir("save-as-elsewhere").appendingPathComponent("main.tex")
        XCTAssertFalse(model.saveTexAs(to: elsewhere))
        XCTAssertFalse(FileManager.default.fileExists(atPath: elsewhere.path))
        XCTAssertEqual(model.documentURL, main)
        let copy = main.deletingLastPathComponent().appendingPathComponent("paper.tex")
        XCTAssertTrue(model.saveTexAs(to: copy), model.captureNote ?? "")
        XCTAssertEqual(try disk(copy), "\\input{chapter}\n% entry edit\n")
        XCTAssertEqual(model.documentURL, copy)
        XCTAssertEqual(model.project.entryPath, "main.tex", "the entry keeps its tab path")
        XCTAssertFalse(model.project.isDirty(model.project.entryPath))
    }

    /// A slow open must not switch once the user navigated meanwhile — even
    /// A→B→A, which lands back on the path the open started from.
    func testStaleOpenAndSwitchNeverSwitchesAfterAnInterveningNavigation() async throws {
        let (model, main, _) = try await openProject("nav-token")
        try "Other.\n".write(to: main.deletingLastPathComponent().appendingPathComponent("other.tex"), atomically: true, encoding: .utf8)
        XCTAssertEqual(model.activePath, "main.tex")
        var gate: CheckedContinuation<Void, Never>?
        let slow = Task { @MainActor in
            await model.openAndSwitch("other.tex", role: .opened, open: { path, role in
                await withCheckedContinuation { gate = $0 }
                return await model.project.openDocument(path, role: role)
            }, onRefusal: { _ in })
        }
        while gate == nil { await Task.yield() }
        model.project.switchDocument(to: "chapter.tex")
        model.project.switchDocument(to: "main.tex")
        gate?.resume()
        let opened = await slow.value
        XCTAssertTrue(opened)
        XCTAssertTrue(model.project.isOpen("other.tex"))
        XCTAssertEqual(model.activePath, "main.tex", "the stale open must not switch")
    }

    /// A direct save must never copy the replaced file's times or lock flags
    /// (`COPYFILE_SECURITY` implies `COPYFILE_STAT`): the saved file's mtime
    /// is now, so make/latexmk/git see a same-size edit; its mode is kept.
    func testDirectSaveGivesAFreshModificationTimeAndKeepsTheMode() throws {
        EditorPreferences.shared.autosave = true
        let dir = try tempDir("mtime")
        let url = dir.appendingPathComponent("paper.tex")
        try "one\n".write(to: url, atomically: true, encoding: .utf8)
        let old = Date(timeIntervalSince1970: 1_000_000_000)
        try FileManager.default.setAttributes([.posixPermissions: 0o640, .modificationDate: old], ofItemAtPath: url.path)
        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: url), .opened)
        model.updateActiveText("two\n") // same size as "one\n"
        model.flushPendingAutosave()
        XCTAssertEqual(try disk(url), "two\n")
        let attrs = try FileManager.default.attributesOfItem(atPath: url.path)
        let mtime = try XCTUnwrap(attrs[.modificationDate] as? Date)
        XCTAssertGreaterThan(mtime.timeIntervalSinceNow, -300, "mtime must be the save time, not \(old)")
        XCTAssertEqual(attrs[.posixPermissions] as? Int, 0o640)
    }

    /// A locked (uchg) file is refused up front: a failed save, the file
    /// untouched, and no `.name.flashtex-save-*` temp file left behind.
    func testDirectSaveOfALockedFileFailsAndLeavesNoTempFile() throws {
        EditorPreferences.shared.autosave = true
        let dir = try tempDir("locked")
        let url = dir.appendingPathComponent("paper.tex")
        try "one\n".write(to: url, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: url), .opened)
        try FileManager.default.setAttributes([.immutable: true], ofItemAtPath: url.path)
        addTeardownBlock {
            let names = (try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []
            for name in names { try? FileManager.default.setAttributes([.immutable: false], ofItemAtPath: dir.appendingPathComponent(name).path) }
        }
        model.updateActiveText("two\n")
        XCTAssertFalse(model.saveTex())
        XCTAssertTrue(model.isDirty)
        XCTAssertEqual(try disk(url), "one\n")
        let leftovers = try FileManager.default.contentsOfDirectory(atPath: dir.path).filter { $0.contains("flashtex-save-") }
        XCTAssertEqual(leftovers, [])
    }

    /// The Save As refusal compares file identity, not path spelling: a
    /// different-case name on a case-insensitive volume, or a hard link, is
    /// still the open member's file.
    func testSaveAsRefusesAnOpenMemberReachedByCaseOrHardLink() async throws {
        let (model, main, chapter) = try await openProject("save-as-identity")
        let dir = main.deletingLastPathComponent()
        let link = dir.appendingPathComponent("alias.tex")
        try FileManager.default.linkItem(at: chapter, to: link)
        XCTAssertFalse(model.saveTexAs(to: link), "a hard link to chapter.tex is chapter.tex")
        let caseInsensitive = try dir.resourceValues(forKeys: [.volumeSupportsCaseSensitiveNamesKey]).volumeSupportsCaseSensitiveNames == false
        if caseInsensitive {
            XCTAssertFalse(model.saveTexAs(to: dir.appendingPathComponent("CHAPTER.tex")))
        }
        XCTAssertEqual(try disk(chapter), "Chapter.\n")
        XCTAssertEqual(model.documentURL, main)
    }

    /// A helper export that answers only after File > Open replaced the
    /// project must not record the old document's save on the new one.
    func testLateControllerSaveAfterAProjectReplaceTouchesNothing() async throws {
        let fake = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures/fake_preview_controller.py")
        let dir = try tempDir("late-export")
        let main = dir.appendingPathComponent("main.tex"), other = dir.appendingPathComponent("other.tex")
        try "Hello\n".write(to: main, atomically: true, encoding: .utf8)
        try "Other\n".write(to: other, atomically: true, encoding: .utf8)
        let mark = dir.appendingPathComponent("export.mark"), release = dir.appendingPathComponent("export.release")
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", dir.appendingPathComponent("ledger").path, 1)
        setenv("FAKE_PC_EXPORT_MARK", mark.path, 1)
        setenv("FAKE_PC_EXPORT_RELEASE", release.path, 1)
        defer { for k in ["FLASHTEX_CONTROLLER_LEDGER_ROOT", "FAKE_PC_EXPORT_MARK", "FAKE_PC_EXPORT_RELEASE"] { unsetenv(k) } }
        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: main), .opened)
        model.attachController(at: fake)
        defer { FileManager.default.createFile(atPath: release.path, contents: nil); model.detachController() }
        func waitUntil(_ what: String, _ cond: () -> Bool) async throws {
            let start = Date()
            while !cond() {
                if Date().timeIntervalSince(start) > 10 { XCTFail("timed out waiting for \(what)"); throw XCTSkip(what) }
                try await Task.sleep(nanoseconds: 20_000_000)
            }
        }
        try await waitUntil("controller ready") { model.controllerState.ready && model.controllerState.durable["main.tex"] != nil }
        model.updateActiveText("Hello, edited\n")
        let save = Task { @MainActor in await model.controllerSave() }
        try await waitUntil("export sent") { FileManager.default.fileExists(atPath: mark.path) }
        XCTAssertEqual(model.openTex(at: other, dirty: .discard), .opened)
        FileManager.default.createFile(atPath: release.path, contents: nil)
        let outcome = await save.value
        guard case .failed = outcome else { return XCTFail("a save resuming in another project must fail, got \(outcome)") }
        XCTAssertEqual(model.documentURL, other)
        XCTAssertEqual(model.savedText, "Other\n", "the new document's saved baseline is untouched")
        XCTAssertFalse(model.isDirty)
        XCTAssertNil(model.files.conflict)
    }

    // MARK: #786 — a project replacement considers every dirty document

    /// Keeps the durable snapshot store inside the test's temp directory. The
    /// model has already created its store (opening files reads it), so the
    /// environment override would come too late: point the store itself (#806).
    private func privateSnapshots(_ model: ShellModel, _ dir: URL) {
        let store = dir.appendingPathComponent("snapshots")
        model.dirtySnapshots.directory = store
        XCTAssertEqual(model.dirtySnapshots.directory.path, store.path, "the private store takes effect")
    }

    /// Edits chapter.tex, then switches to the clean main.tex before
    /// autosave fires (the #786 repro): only an inactive member is dirty.
    private func projectWithInactiveDirtyMember(_ tag: String) async throws -> (ShellModel, main: URL, chapter: URL, next: URL) {
        EditorPreferences.shared.autosave = false
        let (model, main, chapter) = try await openProject(tag)
        privateSnapshots(model, main.deletingLastPathComponent())
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, edited.\n")
        model.project.switchDocument(to: "main.tex")
        XCTAssertFalse(model.isDirty, "the active document is clean")
        XCTAssertTrue(model.project.isDirty("chapter.tex"))
        let next = main.deletingLastPathComponent().appendingPathComponent("next.tex")
        try "Next.\n".write(to: next, atomically: true, encoding: .utf8)
        return (model, main, chapter, next)
    }

    func testOpenWithOnlyAnInactiveMemberDirtyIsRefused() async throws {
        let (model, _, chapter, next) = try await projectWithInactiveDirtyMember("786-open")
        XCTAssertEqual(model.openTex(at: next), .blockedByUnsavedEdits, "chapter.tex's edits would be dropped silently")
        XCTAssertEqual(model.project.entryPath, "main.tex")
        XCTAssertTrue(model.project.isDirty("chapter.tex"))
        XCTAssertEqual(try disk(chapter), "Chapter.\n")
    }

    func testDiscardSnapshotsAnInactiveDirtyMember() async throws {
        let (model, _, chapter, next) = try await projectWithInactiveDirtyMember("786-discard")
        XCTAssertEqual(model.openTex(at: next, dirty: .discard), .opened)
        XCTAssertEqual(model.dirtySnapshots.read(for: chapter)?.text, "Chapter, edited.\n", "the member's own text is kept under its own file")
        XCTAssertNil(model.recoverableBuffer, "a clean entry does not take the recoverable slot")
    }

    func testReloadWithOnlyAnInactiveMemberDirtyIsRefused() async throws {
        let (model, _, _, _) = try await projectWithInactiveDirtyMember("786-reload")
        XCTAssertEqual(model.reloadFromDisk(), .blockedByUnsavedEdits, "a direct reload replaces the whole project")
        XCTAssertTrue(model.project.isOpen("chapter.tex"))
    }

    func testFixtureLoadSavesEveryDirtyDocumentFirst() async throws {
        let (model, main, chapter, _) = try await projectWithInactiveDirtyMember("786-fixture-save")
        model.updateActiveText("\\input{chapter}\n% entry edit\n")
        XCTAssertEqual(model.loadFixturesReplacingProject(request: nil, result: CaretSyncTests.resultURL, dirty: .saveFirst), .opened)
        XCTAssertEqual(try disk(main), "\\input{chapter}\n% entry edit\n")
        XCTAssertEqual(try disk(chapter), "Chapter, edited.\n", "fixture load saved only the entry")
        XCTAssertEqual(model.previewSource, .fixture)
    }

    func testFixtureLoadIsAbortedWhenAMemberSaveConflicts() async throws {
        let (model, _, chapter, _) = try await projectWithInactiveDirtyMember("786-fixture-conflict")
        try "Chapter, external.\n".write(to: chapter, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.loadFixturesReplacingProject(request: nil, result: CaretSyncTests.resultURL, dirty: .saveFirst), .saveFailed)
        XCTAssertTrue(model.project.isOpen("chapter.tex"), "nothing replaced")
        XCTAssertEqual(model.documents.first(where: { $0.path == "chapter.tex" })?.text, "Chapter, edited.\n")
        XCTAssertEqual(try disk(chapter), "Chapter, external.\n")
        XCTAssertNotEqual(model.previewSource, .fixture)
    }

    // MARK: #806 — discard follow-ups

    func testDiscardSnapshotsLandInTheTestsPrivateStore() async throws {
        let (model, main, chapter, next) = try await projectWithInactiveDirtyMember("806-private-store")
        XCTAssertEqual(model.openTex(at: next, dirty: .discard), .opened)
        let store = main.deletingLastPathComponent().appendingPathComponent("snapshots")
        XCTAssertTrue(FileManager.default.fileExists(atPath: model.dirtySnapshots.fileURL(for: chapter).path))
        XCTAssertEqual(model.dirtySnapshots.fileURL(for: chapter).deletingLastPathComponent().path, store.path)
    }

    func testEntrySavedThenAMemberSaveFailsReplacesNothing() async throws {
        let (model, main, chapter, next) = try await projectWithInactiveDirtyMember("806-entry-then-member")
        model.updateActiveText("\\input{chapter}\n% entry edit\n")
        try "Chapter, external.\n".write(to: chapter, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.openTex(at: next, dirty: .saveFirst), .saveFailed)
        XCTAssertEqual(try disk(main), "\\input{chapter}\n% entry edit\n", "the entry is saved before the member")
        XCTAssertFalse(model.project.isDirty("main.tex"))
        XCTAssertEqual(model.project.entryPath, "main.tex", "nothing replaced")
        XCTAssertTrue(model.project.isDirty("chapter.tex"))
        XCTAssertEqual(try disk(chapter), "Chapter, external.\n")
        XCTAssertNotNil(model.project.saveConflict(for: "chapter.tex"))
        XCTAssertTrue(model.captureNote?.contains("chapter.tex") == true, model.captureNote ?? "")
    }

    func testDiscardViaFixtureLoadKeepsEachDocumentsTextAndNamesItsRecovery() async throws {
        let (model, _, chapter, _) = try await projectWithInactiveDirtyMember("806-fixture-discard")
        model.updateActiveText("\\input{chapter}\n% entry edit\n")
        XCTAssertEqual(model.loadFixturesReplacingProject(request: nil, result: CaretSyncTests.resultURL, dirty: .discard), .opened)
        XCTAssertEqual(model.previewSource, .fixture)
        XCTAssertEqual(model.recoverableBuffer?.text, "\\input{chapter}\n% entry edit\n")
        XCTAssertEqual(model.dirtySnapshots.read(for: chapter)?.text, "Chapter, edited.\n")
    }

    /// A rejected fixture replaces nothing, so it must not record a discard
    /// either (overwriting the recoverable slot or a member's snapshot).
    func testRejectedFixtureRecordsNoDiscard() async throws {
        let (model, main, chapter, _) = try await projectWithInactiveDirtyMember("806-fixture-rejected")
        model.updateActiveText("\\input{chapter}\n% entry edit\n")
        let bad = main.deletingLastPathComponent().appendingPathComponent("bad-result.json")
        try "{}".write(to: bad, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.loadFixturesReplacingProject(request: nil, result: bad, dirty: .discard), .readFailed)
        XCTAssertNil(model.recoverableBuffer, "a rejected fixture recorded a discard")
        XCTAssertNil(model.dirtySnapshots.read(for: main))
        XCTAssertNil(model.dirtySnapshots.read(for: chapter))
        XCTAssertTrue(model.project.isDirty("chapter.tex"))
    }

    /// Reload after an entry-save conflict replaces the whole project: the
    /// prompt names the dirty members it also discards, and each file's
    /// recovery command.
    func testReloadAfterAnEntryConflictNamesAndKeepsDirtyMembers() async throws {
        let (model, main, chapter, _) = try await projectWithInactiveDirtyMember("806-reload")
        model.updateActiveText("\\input{chapter}\n% entry edit\n")
        try "\\input{chapter}\n% external\n".write(to: main, atomically: true, encoding: .utf8)
        XCTAssertFalse(model.saveTex())
        XCTAssertNotNil(model.files.conflict)
        let review = try XCTUnwrap(model.prepareReload())
        XCTAssertTrue(review.bufferDirty)
        XCTAssertTrue(review.summary.contains("Unsaved edits to chapter.tex are discarded too (recoverable via File > Restore Unsaved Snapshot…)"), review.summary)
        XCTAssertTrue(review.summary.contains("Edit > Restore Discarded Buffer"), review.summary)
        let outcome = await model.confirmReload(review, dirty: .discard)
        XCTAssertEqual(outcome, .opened)
        XCTAssertEqual(try disk(main), "\\input{chapter}\n% external\n")
        XCTAssertEqual(model.recoverableBuffer?.text, "\\input{chapter}\n% entry edit\n")
        XCTAssertEqual(model.dirtySnapshots.read(for: chapter)?.text, "Chapter, edited.\n")
        let note = model.captureNote ?? ""
        XCTAssertTrue(note.contains("main.tex via Edit > Restore Discarded Buffer"), note)
        XCTAssertTrue(note.contains("chapter.tex via File > Restore Unsaved Snapshot…"), note)
    }

    func testOpenDiscardNamesTheMembersSnapshotCommand() async throws {
        let (model, _, _, next) = try await projectWithInactiveDirtyMember("806-open-hint")
        XCTAssertEqual(model.openTex(at: next, dirty: .discard), .opened)
        let note = model.captureNote ?? ""
        XCTAssertTrue(note.contains("chapter.tex via File > Restore Unsaved Snapshot…"), note)
        XCTAssertFalse(note.contains("Restore Discarded Buffer"), "the entry was clean: nothing went to the recoverable slot; \(note)")
    }

    func testMemberConflictIsCarriedAcrossARename() async throws {
        EditorPreferences.shared.autosave = false
        let (model, main, chapter) = try await openProject("806-rename")
        privateSnapshots(model, main.deletingLastPathComponent())
        try "Chapter, external.\n".write(to: chapter, atomically: true, encoding: .utf8)
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, ours.\n")
        guard case .conflict = model.project.saveDocumentNow("chapter.tex") else { return XCTFail("expected a conflict") }
        model.updateActiveText("Chapter.\n") // back to its baseline: clean, so it may be renamed
        XCTAssertFalse(model.project.isDirty("chapter.tex"))
        model.project.switchDocument(to: "main.tex")
        guard case .renamed = await model.project.renameDocument("chapter.tex", to: "renamed") else {
            return XCTFail("rename refused: \(model.project.status)")
        }
        XCTAssertNil(model.project.saveConflict(for: "chapter.tex"))
        XCTAssertNotNil(model.project.saveConflict(for: "renamed.tex"), "the conflict follows its member")
    }

    /// The snapshot is a discarded member's only copy: a store that cannot be
    /// written refuses every replacement and keeps the text in the editor.
    func testFailedSnapshotWriteAbortsTheReplacement() async throws {
        let (model, main, chapter, next) = try await projectWithInactiveDirtyMember("806-snapshot-fails")
        let blocker = main.deletingLastPathComponent().appendingPathComponent("blocker")
        try "not a directory".write(to: blocker, atomically: true, encoding: .utf8)
        model.dirtySnapshots.directory = blocker.appendingPathComponent("snapshots")

        XCTAssertEqual(model.openTex(at: next, dirty: .discard), .saveFailed)
        XCTAssertEqual(model.project.entryPath, "main.tex", "open replaced the project")
        XCTAssertTrue(model.captureNote?.contains("nothing replaced") == true, model.captureNote ?? "")
        XCTAssertTrue(model.captureNote?.contains("chapter.tex") == true, model.captureNote ?? "")

        XCTAssertEqual(model.loadFixturesReplacingProject(request: nil, result: CaretSyncTests.resultURL, dirty: .discard), .saveFailed)
        XCTAssertNotEqual(model.previewSource, .fixture, "fixture load replaced the project")

        XCTAssertEqual(model.reloadFromDisk(dirty: .discard), .saveFailed)

        XCTAssertNil(model.recoverableBuffer)
        XCTAssertTrue(model.project.isOpen("chapter.tex"))
        XCTAssertTrue(model.project.isDirty("chapter.tex"))
        XCTAssertEqual(model.documents.first(where: { $0.path == "chapter.tex" })?.text, "Chapter, edited.\n")
        XCTAssertEqual(try disk(chapter), "Chapter.\n")
    }

    // MARK: #789 — member save conflicts are recorded per path

    func testConflictsOnTwoMembersAreBothKeptAndNeitherIsRetried() async throws {
        EditorPreferences.shared.autosave = true
        let (model, main, chapter) = try await openProject("789-two")
        privateSnapshots(model, main.deletingLastPathComponent())
        let other = main.deletingLastPathComponent().appendingPathComponent("other.tex")
        try "Other.\n".write(to: other, atomically: true, encoding: .utf8)
        let opened = await model.project.openDocument("other.tex")
        XCTAssertEqual(opened, .opened(path: "other.tex"))
        try "Chapter, external.\n".write(to: chapter, atomically: true, encoding: .utf8)
        try "Other, external.\n".write(to: other, atomically: true, encoding: .utf8)
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, ours.\n")
        guard case .conflict = model.project.saveDocumentNow("chapter.tex") else { return XCTFail("expected a conflict") }
        model.project.switchDocument(to: "other.tex")
        model.updateActiveText("Other, ours.\n")
        guard case .conflict = model.project.saveDocumentNow("other.tex") else { return XCTFail("expected a conflict") }
        XCTAssertEqual(model.project.saveConflict(for: "chapter.tex")?.url.standardizedFileURL, chapter.standardizedFileURL)
        XCTAssertEqual(model.project.saveConflict(for: "other.tex")?.url.standardizedFileURL, other.standardizedFileURL)
        // A refused save re-keeps the buffer as a durable snapshot: with the
        // snapshots removed, a retry by autosave would write them again.
        model.dirtySnapshots.remove(for: chapter)
        model.dirtySnapshots.remove(for: other)
        model.updateActiveText("Other, ours again.\n") // arms autosave
        model.flushPendingAutosave()
        XCTAssertNil(model.dirtySnapshots.read(for: chapter), "autosave retried conflicted chapter.tex")
        XCTAssertNil(model.dirtySnapshots.read(for: other), "autosave retried conflicted other.tex")
        XCTAssertNotNil(model.project.saveConflict(for: "chapter.tex"))
    }
}
