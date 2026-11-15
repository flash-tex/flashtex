import XCTest
@testable import FlashTeXMac

/// Text typed into the untitled document survives a crash
/// (UntitledDrafts.swift): the autosave pass keeps it as a draft, a file or
/// a deliberate discard drops it, and the next untitled document takes a
/// dead instance's draft as its unsaved text.
@MainActor
final class UntitledDraftsTests: XCTestCase {
    private var dir: URL!
    private var autosave = true

    override func setUp() {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("untitled-drafts-\(UUID().uuidString)")
        autosave = EditorPreferences.shared.autosave
    }

    override func tearDown() {
        EditorPreferences.shared.autosave = autosave
        try? FileManager.default.removeItem(at: dir)
    }

    private func untitled() -> ShellModel {
        let model = ShellModel(startup: .untitledDocument)
        model.untitledDrafts.directory = dir
        return model
    }

    func testTypingIntoTheUntitledDocumentKeepsADraftWhateverTheAutosaveSetting() {
        for setting in [true, false] {
            EditorPreferences.shared.autosave = setting
            let model = untitled()
            XCTAssertNil(model.untitledDrafts.read(model.untitledDraftID), "an untouched document keeps nothing")
            model.updateActiveText(UntitledDocument.text + "My unsaved paragraph.\n")
            model.flushPendingAutosave()
            let draft = model.untitledDrafts.read(model.untitledDraftID)
            XCTAssertEqual(draft?.text, UntitledDocument.text + "My unsaved paragraph.\n", "autosave \(setting)")
            XCTAssertEqual(draft?.owner, EngineV3.instanceOwner)
            XCTAssertNil(model.documentURL, "nothing was saved to a file")
            model.updateActiveText("")
            model.flushPendingAutosave()
            XCTAssertNil(model.untitledDrafts.read(model.untitledDraftID), "an empty buffer keeps nothing")
        }
    }

    func testTheDraftGoesAwayOnceTheBufferHasAFile() throws {
        let model = untitled()
        model.files.policy = .disabled(reason: "test: no helper binary")
        model.updateActiveText("draft text\n")
        model.flushPendingAutosave()
        XCTAssertNotNil(model.untitledDrafts.read(model.untitledDraftID))
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let file = dir.appendingPathComponent("paper.tex")
        try "Paper\n".write(to: file, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.openTex(at: file, dirty: .discard), .opened)
        XCTAssertNil(model.untitledDrafts.read(model.untitledDraftID))
    }

    func testTheNextUntitledDocumentRecoversADeadInstancesNewestDraft() throws {
        let store = UntitledDraftStore(directory: dir)
        let dead = "1 0 0" // launchd's pid with another start time: not running as that instance
        XCTAssertTrue(store.write(UntitledDraft(id: "old", owner: dead, text: "older text\n", savedAt: Date(timeIntervalSinceNow: -600))))
        XCTAssertTrue(store.write(UntitledDraft(id: "new", owner: dead, text: "newest text\n", savedAt: Date(timeIntervalSinceNow: -60))))
        XCTAssertTrue(store.write(UntitledDraft(id: "live", owner: EngineV3.instanceOwner, text: "another window's\n", savedAt: Date())))
        XCTAssertEqual(store.leftovers().map(\.id), ["new", "old"], "a running instance's draft is not a leftover")

        let model = ShellModel(startup: .untitledDocument)
        model.untitledDrafts.directory = dir
        XCTAssertTrue(model.recoverUntitledDraft())
        XCTAssertEqual(model.activeText, "newest text\n")
        XCTAssertNil(model.documentURL)
        XCTAssertTrue(model.isDirty, "recovered text is unsaved: quitting asks")
        XCTAssertTrue(model.captureNote?.contains("Recovered") == true, model.captureNote ?? "")
        XCTAssertNil(store.read("new"), "taken over")
        XCTAssertEqual(model.untitledDrafts.read(model.untitledDraftID)?.text, "newest text\n", "kept under this instance")
        XCTAssertNotNil(store.read("live"))
        // Only an untouched untitled document recovers.
        XCTAssertFalse(model.recoverUntitledDraft())
        XCTAssertEqual(model.activeText, "newest text\n")
        let second = ShellModel(startup: .untitledDocument)
        second.untitledDrafts.directory = dir
        XCTAssertTrue(second.recoverUntitledDraft())
        XCTAssertEqual(second.activeText, "older text\n", "the next window takes the next one")
    }

    /// A window the user closed stays closed: its untitled text is
    /// discarded, not recovered by the next launch's untitled window
    /// (Commander ruling 2026-11-09). Before, closing kept the draft on disk,
    /// so once this instance was gone it read as a crash's leftover. A late
    /// autosave pass (a timer that outlives the window) keeps nothing either.
    func testClosingAnUntitledWindowDiscardsItsDraft() throws {
        let windows = ProjectWindows()
        windows.makeModel = { [dir] _ in
            let m = ShellModel(startup: .untitledDocument, launchHooks: false)
            m.untitledDrafts.directory = dir!
            return m
        }
        let id = windows.newWindow()
        let model = try XCTUnwrap(windows.entries.first { $0.id == id }?.model)
        model.updateActiveText(UntitledDocument.text + "Text I then closed.\n")
        model.flushPendingAutosave()
        XCTAssertNotNil(model.untitledDrafts.read(model.untitledDraftID), "kept while the window is open")

        windows.closed(id)
        XCTAssertNil(model.untitledDrafts.read(model.untitledDraftID), "discarded with the window")
        model.updateActiveText(UntitledDocument.text + "A late edit.\n")
        model.flushPendingAutosave()
        XCTAssertNil(model.untitledDrafts.read(model.untitledDraftID), "a late pass keeps nothing")
        // The next launch (this instance dead) has nothing to recover.
        XCTAssertTrue(UntitledDraftStore(directory: dir).leftovers(isAlive: { _ in false }).isEmpty)
        let next = ShellModel(startup: .untitledDocument)
        next.untitledDrafts.directory = dir
        XCTAssertFalse(next.recoverUntitledDraft(isAlive: { _ in false }))
    }

    /// Quitting with two windows (S-REVIEW MAJOR on #2442): Don't Save in A,
    /// then A's autosave timer fires while B's question is still up (GCD's
    /// main queue is served in the modal panel's run-loop mode). Before, the
    /// pass wrote A's draft again after Don't Save had dropped it, and the
    /// next launch recovered the discarded text as a crash's leftover.
    func testDontSaveAtQuitKeepsNoDraftWhenATimerFiresWhileALaterWindowAsks() {
        let a = untitled(), b = untitled()
        a.updateActiveText(UntitledDocument.text + "Text A, discarded at quit.\n")
        a.flushPendingAutosave()
        XCTAssertNotNil(a.untitledDrafts.read(a.untitledDraftID), "kept while A is open")
        a.updateActiveText(UntitledDocument.text + "Text A, discarded at quit, edited again.\n") // a pass is pending
        b.updateActiveText(UntitledDocument.text + "Text B.\n")

        let quit = AppDelegate.confirmQuit([a, b]) { model in
            if model === a { return .discard }
            a.flushPendingAutosave() // A's timer fires while B asks
            return .discard
        }
        XCTAssertTrue(quit)
        a.flushPendingAutosave() // and once more after the loop, before the app is gone
        XCTAssertNil(a.untitledDrafts.read(a.untitledDraftID), "Don't Save in A keeps no draft of A")
        XCTAssertTrue(UntitledDraftStore(directory: dir).leftovers(isAlive: { _ in false }).isEmpty,
                      "the next launch recovers nothing that was discarded")
    }

    /// The same timer of a file-backed window with autosave on: Don't Save
    /// must not write the user's file while a later window asks.
    func testDontSaveAtQuitNeverWritesTheFileWhileALaterWindowAsks() throws {
        EditorPreferences.shared.autosave = true
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let url = dir.appendingPathComponent("paper.tex")
        try "one\n".write(to: url, atomically: true, encoding: .utf8)
        let a = untitled(), b = untitled()
        a.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(a.openTex(at: url), .opened)
        a.updateActiveText("two (discarded at quit)\n") // a pass is pending
        b.updateActiveText(UntitledDocument.text + "Text B.\n")

        let quit = AppDelegate.confirmQuit([a, b]) { model in
            if model === a { return .discard }
            a.flushPendingAutosave() // A's timer fires while B asks
            return .discard
        }
        XCTAssertTrue(quit)
        XCTAssertEqual(try String(contentsOf: url, encoding: .utf8), "one\n", "Don't Save wrote nothing to the file")
    }

    /// Don't Save in A, then Cancel in B (S-REVIEW MINOR on #2442): the app
    /// stays open with A's unsaved text on screen, so A keeps its crash
    /// draft and its autosave. Before, A's draft was already gone.
    func testCancelInALaterWindowKeepsTheEarlierWindowsDraft() {
        let a = untitled(), b = untitled()
        a.updateActiveText(UntitledDocument.text + "Text A, still open.\n")
        a.flushPendingAutosave()
        b.updateActiveText(UntitledDocument.text + "Text B.\n")

        let quit = AppDelegate.confirmQuit([a, b]) { $0 === a ? .discard : .cancel }
        XCTAssertFalse(quit)
        XCTAssertEqual(a.untitledDrafts.read(a.untitledDraftID)?.text, UntitledDocument.text + "Text A, still open.\n",
                       "A is still open: its draft stays")
        a.updateActiveText(UntitledDocument.text + "Text A, still open, edited.\n")
        a.flushPendingAutosave()
        XCTAssertEqual(a.untitledDrafts.read(a.untitledDraftID)?.text, UntitledDocument.text + "Text A, still open, edited.\n",
                       "A's autosave goes on")
    }
}
