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
}
