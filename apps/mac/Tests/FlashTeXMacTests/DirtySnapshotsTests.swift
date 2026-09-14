import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Durable unsaved-text snapshots (`DirtySnapshots.swift`, lane
/// mac-document-files-2): the dirty buffer survives an external change, a
/// reload, an open, a detach, a quit without saving and a relaunch, and is
/// offered (never applied) when the file is opened again. Real files in temp
/// directories; the rooted `flashtex-project-files` helper where the entry
/// document's saves go through it (skipped, never faked, when it is not
/// built); the real preview controller + compiler for the Fable-found case
/// (a file not named `main.tex` through the helper).
@MainActor
final class DirtySnapshotsTests: XCTestCase {
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
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-snap-\(tag)-\(UUID().uuidString)")
            .resolvingSymlinksInPath()
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: dir) }
        return dir
    }

    /// Points every model created afterwards at a private store (the app
    /// default is Application Support); the env var is what the app honours.
    private func privateStore(_ dir: URL) -> URL {
        let store = dir.appendingPathComponent("snapshots")
        setenv("FLASHTEX_DIRTY_SNAPSHOTS", store.path, 1)
        addTeardownBlock { unsetenv("FLASHTEX_DIRTY_SNAPSHOTS") }
        return store
    }

    private func disk(_ url: URL) throws -> String { try String(contentsOf: url, encoding: .utf8) }

    // MARK: store

    func testStoreKeepsOneSnapshotPerFileAndListsByProject() throws {
        let dir = try tempDir("store")
        let store = DirtySnapshotStore(directory: dir.appendingPathComponent("s"))
        let a = dir.appendingPathComponent("proj/a.tex"), b = dir.appendingPathComponent("proj/ch/b.tex"), c = dir.appendingPathComponent("other/c.tex")
        XCTAssertNil(store.read(for: a))
        XCTAssertEqual(store.all(), [], "an absent directory is an empty store, not an error")
        let s1 = DirtySnapshot(file: a.path, text: "A dirty\n", diskSha256: "00", savedAt: Date(timeIntervalSince1970: 1), reason: "test")
        XCTAssertNotNil(store.write(s1))
        XCTAssertEqual(store.read(for: a), s1)
        XCTAssertEqual(store.read(for: URL(fileURLWithPath: dir.path + "/proj/./a.tex")), s1, "keyed by the standardized path")
        XCTAssertEqual(DirtySnapshotStore.key(for: a).count, 32)
        // A second write for the same file replaces, never accumulates.
        let s1b = DirtySnapshot(file: a.path, text: "A dirtier\n", diskSha256: nil, savedAt: Date(timeIntervalSince1970: 2), reason: "again")
        store.write(s1b)
        XCTAssertEqual(store.all().count, 1)
        XCTAssertEqual(store.read(for: a)?.text, "A dirtier\n")
        store.write(.init(file: b.path, text: "B\n", diskSha256: nil, savedAt: Date(timeIntervalSince1970: 3), reason: "t"))
        store.write(.init(file: c.path, text: "C\n", diskSha256: nil, savedAt: Date(timeIntervalSince1970: 4), reason: "t"))
        XCTAssertEqual(store.all().map(\.file), [c.path, b.path, a.path], "newest first")
        XCTAssertEqual(store.snapshots(under: dir.appendingPathComponent("proj")).map(\.file), [b.path, a.path])
        store.remove(for: a)
        XCTAssertNil(store.read(for: a))
        XCTAssertEqual(store.all().count, 2)
        // Garbage in the directory is ignored; a snapshot whose recorded path
        // is not the asked-for file is not returned for it.
        try "not json".write(to: store.directory.appendingPathComponent("junk.json"), atomically: true, encoding: .utf8)
        try Data(try JSONEncoder().encode(["file": "/elsewhere.tex"])).write(to: store.fileURL(for: a))
        XCTAssertNil(store.read(for: a))
        XCTAssertEqual(store.all().count, 2)
        XCTAssertTrue(store.summaryIsReadable(for: b))
    }

    // MARK: entry document: external change, reload, relaunch, restore (real helper, paper.tex)

    func testDirtyTextSurvivesExternalChangeReloadAndRelaunchAndIsOfferedNotApplied() throws {
        let helper = try requireRealHelper()
        let dir = try tempDir("entry")
        let store = privateStore(dir)
        let url = dir.appendingPathComponent("paper.tex") // the entry is named after the file, not main.tex
        try "v1\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .executable(helper, arguments: [])
        XCTAssertEqual(model.openTex(at: url), .opened)
        XCTAssertEqual(model.activePath, "paper.tex")
        XCTAssertEqual(model.files.offeredSnapshots, [], "nothing kept yet")
        XCTAssertEqual(model.dirtySnapshots.directory.path, store.path)
        model.updateActiveText("v1 edited\n")
        XCTAssertTrue(model.isDirty)
        XCTAssertNil(model.dirtySnapshots.read(for: url), "typing alone does not write the store")

        // External change while dirty: conflict surfaced, buffer untouched, and
        // BOTH texts recoverable — disk through the reviewed reload, the buffer
        // durably in the store.
        try "v2 (external)\n".write(to: url, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.checkDiskStatus(), .modified)
        XCTAssertEqual(model.files.conflict?.kind, .modifiedExternally)
        XCTAssertEqual(model.activeText, "v1 edited\n")
        let kept = try XCTUnwrap(model.dirtySnapshots.read(for: url))
        XCTAssertEqual(kept.text, "v1 edited\n")
        XCTAssertEqual(kept.diskSha256, SourceDigest.sha256Hex("v2 (external)\n"))
        XCTAssertTrue(kept.reason.contains("changed on disk"), kept.reason)
        XCTAssertEqual(model.prepareReload()?.diskText, "v2 (external)\n")
        XCTAssertEqual(try disk(url), "v2 (external)\n", "nothing written")
        // A refused save keeps it too (and never writes).
        XCTAssertFalse(model.saveTex())
        XCTAssertEqual(model.dirtySnapshots.read(for: url)?.reason, "save refused: file changed on disk")
        XCTAssertEqual(try disk(url), "v2 (external)\n")

        // Reload with the dirty buffer: explicit discard only; the text stays in
        // session memory AND in the store.
        XCTAssertEqual(model.reloadFromDisk(), .blockedByUnsavedEdits)
        XCTAssertEqual(model.reloadFromDisk(dirty: .discard), .opened)
        XCTAssertEqual(model.activeText, "v2 (external)\n")
        XCTAssertFalse(model.isDirty)
        XCTAssertEqual(model.recoverableBuffer, .init(url: url, text: "v1 edited\n"))
        let discarded = try XCTUnwrap(model.dirtySnapshots.read(for: url))
        XCTAssertEqual(discarded.text, "v1 edited\n")
        XCTAssertEqual(discarded.reason, "discarded by a reload from disk")
        XCTAssertEqual(model.files.offeredSnapshots, [], "a reload offers nothing: the discard is the user's own decision this session")
        XCTAssertEqual(model.files.helperRestarts, 0, "the store never rebinds the rooted helper")
        model.files.detachHelper()

        // Relaunch (a fresh model, same store): opening the file offers the
        // snapshot; the disk text is what opened.
        let model2 = ShellModel()
        model2.detachWorker()
        model2.files.policy = .executable(helper, arguments: [])
        XCTAssertEqual(model2.openTex(at: url), .opened)
        XCTAssertEqual(model2.activeText, "v2 (external)\n")
        XCTAssertFalse(model2.isDirty)
        XCTAssertEqual(model2.files.offeredSnapshots.map(\.text), ["v1 edited\n"])
        XCTAssertTrue(model2.captureNote?.contains("unsaved text from before is available") == true, model2.captureNote ?? "")
        let offered = try XCTUnwrap(model2.files.offeredSnapshots.first)
        XCTAssertTrue(offered.summary.contains("paper.tex") && offered.summary.contains("10 bytes"), offered.summary)

        // Restore is refused over unsaved edits, then applied as a dirty buffer
        // against the current disk text (never written).
        model2.updateActiveText("v2 typed\n")
        XCTAssertFalse(model2.restoreDirtySnapshot(offered))
        XCTAssertTrue(model2.captureNote?.contains("unsaved edits") == true, model2.captureNote ?? "")
        XCTAssertEqual(model2.activeText, "v2 typed\n")
        XCTAssertNotNil(model2.dirtySnapshots.read(for: url), "a refused restore consumes nothing")
        model2.updateActiveText("v2 (external)\n")
        XCTAssertTrue(model2.restoreDirtySnapshot(offered))
        XCTAssertEqual(model2.activeText, "v1 edited\n")
        XCTAssertTrue(model2.isDirty)
        XCTAssertEqual(model2.savedText, "v2 (external)\n", "baseline is the disk text: the next save is an ordinary compare-and-replace")
        XCTAssertEqual(try disk(url), "v2 (external)\n", "restore writes nothing")
        XCTAssertNil(model2.dirtySnapshots.read(for: url), "consumed")
        XCTAssertEqual(model2.files.offeredSnapshots, [])
        XCTAssertTrue(model2.saveTex())
        XCTAssertEqual(try disk(url), "v1 edited\n")
        model2.files.detachHelper()

        // Nothing left to offer; a moot snapshot (text equal to disk) is dropped on open.
        model2.dirtySnapshots.write(.init(file: url.path, text: "v1 edited\n", diskSha256: nil, savedAt: Date(), reason: "moot"))
        let model3 = ShellModel()
        model3.detachWorker()
        model3.files.policy = .executable(helper, arguments: [])
        XCTAssertEqual(model3.openTex(at: url), .opened)
        XCTAssertEqual(model3.files.offeredSnapshots, [])
        XCTAssertNil(model3.dirtySnapshots.read(for: url), "moot snapshot removed")
        XCTAssertFalse(model3.captureNote?.contains("unsaved text") == true, model3.captureNote ?? "")

        // Explicit discard of an offered snapshot.
        model3.dirtySnapshots.write(.init(file: url.path, text: "leftover\n", diskSha256: nil, savedAt: Date(), reason: "old"))
        let model4 = ShellModel()
        model4.detachWorker()
        model4.files.policy = .executable(helper, arguments: [])
        XCTAssertEqual(model4.openTex(at: url), .opened)
        let leftover = try XCTUnwrap(model4.files.offeredSnapshots.first)
        model4.discardDirtySnapshot(leftover)
        XCTAssertEqual(model4.files.offeredSnapshots, [])
        XCTAssertNil(model4.dirtySnapshots.read(for: url))
        // Open-discard of a dirty buffer when switching files keeps it per file.
        let other = dir.appendingPathComponent("other.tex")
        try "other\n".write(to: other, atomically: true, encoding: .utf8)
        model4.updateActiveText("v1 edited again\n")
        XCTAssertEqual(model4.openTex(at: other, dirty: .discard), .opened)
        XCTAssertEqual(model4.dirtySnapshots.read(for: url)?.reason, "discarded when other.tex was opened")
        XCTAssertEqual(model4.dirtySnapshots.read(for: url)?.text, "v1 edited again\n")
        XCTAssertEqual(model4.files.offeredSnapshots, [], "other.tex has no snapshot")
        model4.files.detachHelper()
    }

    // MARK: members: detach, quit without saving, reopen (direct mode)

    func testEveryDirtyMemberIsKeptOnQuitOrDetachAndOfferedWhenTheProjectReopens() async throws {
        let dir = try tempDir("members")
        _ = privateStore(dir)
        let project = dir.appendingPathComponent("project")
        try FileManager.default.createDirectory(at: project.appendingPathComponent("ch"), withIntermediateDirectories: true)
        let main = project.appendingPathComponent("main.tex")
        let chapter = project.appendingPathComponent("chapter.tex")
        let two = project.appendingPathComponent("ch/two.tex")
        try "\\begin{document}\n\\input{chapter}\n\\input{ch/two}\n\\end{document}\n".write(to: main, atomically: true, encoding: .utf8)
        try "Chapter.\n".write(to: chapter, atomically: true, encoding: .utf8)
        try "Two.\n".write(to: two, atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .disabled(reason: "test: direct")
        XCTAssertEqual(model.openTex(at: main), .opened)
        let p = model.project
        let opened = await p.openDiscoveredIncludes()
        XCTAssertEqual(opened, [.opened(path: "chapter.tex"), .opened(path: "ch/two.tex")])
        XCTAssertEqual(model.files.offeredSnapshots, [])
        p.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, edited.\n")
        p.switchDocument(to: "ch/two.tex")
        model.updateActiveText("Two, edited.\n")
        p.switchDocument(to: "main.tex")
        model.updateActiveText("\\begin{document}\nedited\n\\input{chapter}\n\\input{ch/two}\n\\end{document}\n")
        XCTAssertEqual(p.listing.map(\.isDirty), [true, true, true])

        // Quit without saving (parent hook): every dirty member is kept, none written.
        let kept = model.preserveDirtyBuffers(reason: "quit without saving")
        XCTAssertEqual(Set(kept.map(\.file)), [main.path, chapter.path, two.path])
        XCTAssertEqual(model.dirtySnapshots.snapshots(under: project).count, 3)
        XCTAssertEqual(try disk(chapter), "Chapter.\n")
        XCTAssertEqual(try disk(main).contains("edited"), false)
        // A clean member is not kept (and a stale entry for it is dropped).
        p.switchDocument(to: "ch/two.tex")
        model.updateActiveText("Two.\n")
        XCTAssertFalse(p.isDirty("ch/two.tex"))
        XCTAssertEqual(model.preserveDirtyBuffers(reason: "again").map(\.file).sorted(), [main.path, chapter.path].sorted())
        XCTAssertEqual(model.dirtySnapshots.read(for: two)?.text, "Two, edited.\n", "a clean member is not kept; its earlier snapshot is left for an explicit decision")
        XCTAssertNil(model.preserveDirtyText("Two.\n", at: two, reason: "same as disk"))
        XCTAssertNil(model.dirtySnapshots.read(for: two), "a text equal to disk is not dirty and clears the stale entry")

        // Detach with unsaved edits: session memory plus the store.
        model.updateActiveText("Two, edited twice.\n")
        p.switchDocument(to: "main.tex")
        guard case .refused = await p.detachDocument("ch/two.tex") else { return XCTFail("dirty detach must be refused without discarding") }
        let detached = await p.detachDocument("ch/two.tex", discardingEdits: true)
        XCTAssertEqual(detached, .detached(path: "ch/two.tex"))
        XCTAssertEqual(p.detachedBuffers["ch/two.tex"], "Two, edited twice.\n")
        XCTAssertEqual(model.dirtySnapshots.read(for: two)?.text, "Two, edited twice.\n")
        XCTAssertEqual(model.dirtySnapshots.read(for: two)?.reason, "detached from the project with unsaved edits")
        // A member saved with exactly the kept text consumes its snapshot; a
        // refused member save keeps the buffer durably.
        p.switchDocument(to: "chapter.tex")
        let saved = await p.saveDocument("chapter.tex")
        XCTAssertEqual(saved, .saved(path: "chapter.tex", sha256: SourceDigest.sha256Hex("Chapter, edited.\n")))
        XCTAssertNil(model.dirtySnapshots.read(for: chapter))
        try "Chapter, external.\n".write(to: chapter, atomically: true, encoding: .utf8)
        model.updateActiveText("Chapter, edited more.\n")
        guard case .conflict = await p.saveDocument("chapter.tex") else { return XCTFail("expected a conflict") }
        XCTAssertEqual(model.dirtySnapshots.read(for: chapter)?.text, "Chapter, edited more.\n")
        XCTAssertEqual(try disk(chapter), "Chapter, external.\n")

        // Reopen the project in a new session: the entry's snapshot is offered
        // on open, each member's when it is opened; restores go to the right
        // document and leave everything else untouched.
        let model2 = ShellModel()
        model2.detachWorker()
        model2.files.policy = .disabled(reason: "test: direct")
        XCTAssertEqual(model2.openTex(at: main), .opened)
        XCTAssertEqual(model2.files.offeredSnapshots.map(\.file), [main.path])
        let p2 = model2.project
        let reopened = await p2.openDiscoveredIncludes()
        XCTAssertEqual(reopened, [.opened(path: "chapter.tex"), .opened(path: "ch/two.tex")])
        XCTAssertEqual(Set(model2.files.offeredSnapshots.map(\.file)), [main.path, chapter.path, two.path])
        XCTAssertEqual(model2.documents.map(\.text), [try disk(main), "Chapter, external.\n", "Two.\n"], "offered, not applied")
        let twoSnap = try XCTUnwrap(model2.files.offeredSnapshots.first { $0.file == two.path })
        XCTAssertTrue(model2.restoreDirtySnapshot(twoSnap))
        XCTAssertEqual(model2.activePath, "ch/two.tex")
        XCTAssertEqual(model2.activeText, "Two, edited twice.\n")
        XCTAssertTrue(p2.isDirty("ch/two.tex"))
        XCTAssertFalse(p2.isDirty("chapter.tex"))
        XCTAssertFalse(p2.isDirty("main.tex"))
        XCTAssertEqual(try disk(two), "Two.\n")
        XCTAssertEqual(model2.files.offeredSnapshots.count, 2)
        let mainSnap = try XCTUnwrap(model2.files.offeredSnapshots.first { $0.file == main.path })
        XCTAssertTrue(model2.restoreDirtySnapshot(mainSnap))
        XCTAssertEqual(model2.activePath, "main.tex")
        XCTAssertTrue(model2.activeText.contains("edited"))
        XCTAssertTrue(model2.isDirty)
        XCTAssertEqual(model2.savedText, try disk(main))
        XCTAssertEqual(model2.documents[2].text, "Two, edited twice.\n", "the other restore is intact")
        // A snapshot for a file that is not open is refused with a pointer, not applied.
        let r = await p2.detachDocument("chapter.tex")
        XCTAssertEqual(r, .detached(path: "chapter.tex"))
        let chapterSnap = try XCTUnwrap(model2.files.offeredSnapshots.first { $0.file == chapter.path })
        XCTAssertFalse(model2.restoreDirtySnapshot(chapterSnap))
        XCTAssertTrue(model2.captureNote?.contains("Open chapter.tex") == true, model2.captureNote ?? "")
        XCTAssertNotNil(model2.dirtySnapshots.read(for: chapter))
        // Quit-without-saving now keeps both restored buffers again (one file, one entry).
        XCTAssertEqual(Set(model2.preserveDirtyBuffers(reason: "quit").map(\.file)), [main.path, two.path])
        XCTAssertEqual(model2.dirtySnapshots.snapshots(under: project).count, 3)
    }

    // MARK: no project root

    func testAnUnsavedBufferWithoutAFileCannotBeKeptAndSaysSo() {
        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .disabled(reason: "test")
        model.updateActiveText("typed into the seeded buffer\n")
        model.updateActiveText("typed into the seeded buffer, more\n")
        XCTAssertTrue(model.isDirty)
        XCTAssertEqual(model.preserveDirtyBuffers(reason: "quit"), [], "no URL, nothing to key the snapshot by")
        XCTAssertNil(model.documentURL)
    }
}

private extension DirtySnapshotStore {
    func summaryIsReadable(for url: URL) -> Bool {
        guard let s = read(for: url) else { return false }
        return s.summary.contains(url.lastPathComponent) && s.summary.contains("bytes")
    }
}
