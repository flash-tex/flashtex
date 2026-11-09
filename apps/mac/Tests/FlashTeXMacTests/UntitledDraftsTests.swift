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
}
