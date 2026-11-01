import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

@MainActor
final class ShellModelTests: XCTestCase {
    func testLoadsFixturesAndNavigatesToUTF16Selection() throws {
        let model = ShellModel()
        XCTAssertNil(model.loadError, model.loadError ?? "")
        XCTAssertNotNil(model.fixtureURL)
        XCTAssertEqual(model.activeText, "Hello FlashTeX.\n")
        // The model shows nothing of the fixture's result; its item is the
        // source range navigation is asked for, and the fixture's text is
        // what that compile read (the engine's baseline at DONE).
        let fixtures = try XCTUnwrap(ShellModel.locateFixturesDirectory())
        let result = try RuntimeV1.decodeCompileResult(Data(contentsOf: fixtures.appendingPathComponent("compile-result.json"))).payload
        XCTAssertEqual(result.status, .ok)
        model.setCompiledDocuments(["main.tex": model.activeText])

        guard case .text(let item) = result.pages[0].items[0] else { return XCTFail() }
        model.navigate(to: item.source)
        let sel = try XCTUnwrap(model.selection)
        XCTAssertEqual(sel.path, "main.tex")
        XCTAssertEqual(sel.nsRange, NSRange(location: 0, length: 14))
        XCTAssertEqual((model.activeText as NSString).substring(with: sel.nsRange), "Hello FlashTeX")

        // Editing bumps the revision. An edit inside the item's span refuses navigation
        // (stale offsets are never applied); an edit after it is rebased.
        model.updateActiveText("Héllo FlashTeX.\n")
        let selBefore = model.selection
        model.navigate(to: item.source!, expectedText: item.text)
        XCTAssertEqual(model.selection, selBefore)
        XCTAssertTrue(model.navigationNote?.contains("recompile to navigate") == true, model.navigationNote ?? "")
        model.updateActiveText("Hello FlashTeX. Appended\n")
        model.navigate(to: item.source!, expectedText: nil)
        XCTAssertEqual(model.selection?.nsRange, NSRange(location: 0, length: 14))
        model.updateActiveText("Prefix! Hello FlashTeX.\n")
        // The contract fixture's item text ("Hello FlashTeX.") is one byte longer than
        // its range (0..<14), so expected-text verification would (correctly) refuse;
        // navigate without it here. Reported to the fixture owner.
        model.navigate(to: item.source!, expectedText: nil)
        XCTAssertEqual(model.selection?.nsRange, NSRange(location: 8, length: 14), model.navigationNote ?? "nil")
        XCTAssertTrue(model.navigationNote?.contains("rebased") == true, model.navigationNote ?? "nil")
        model.updateActiveText("Hello FlashTeX.\n") // back to the compiled text

        // Invalid ranges are reported, not applied.
        let before = model.selection
        model.navigate(to: .init(path: "main.tex", startByte: 0, endByte: 999))
        XCTAssertEqual(model.selection, before)
        XCTAssertTrue(model.navigationNote?.contains("not a valid range") == true)
        model.navigate(to: .init(path: "other.tex", startByte: 0, endByte: 1))
        XCTAssertTrue(model.navigationNote?.contains("No open document") == true)
        model.navigate(to: nil)
        XCTAssertTrue(model.navigationNote?.contains("no source mapping") == true)
    }
}

@MainActor
final class DocumentFileTests: XCTestCase {
    func testOpenAndSaveRoundTripWithDirtyTracking() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-doc-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let url = dir.appendingPathComponent("paper.tex")
        try "Café naïve\n".write(to: url, atomically: true, encoding: .utf8)

        let model = ShellModel()
        XCTAssertNotNil(model.fixtureURL, "fixture loaded first")
        model.openTex(at: url)
        XCTAssertEqual(model.activeText, "Café naïve\n")
        XCTAssertEqual(model.documentURL, url)
        XCTAssertNil(model.fixtureURL, "opening a file replaces the fixture project")
        XCTAssertFalse(model.isDirty)

        model.updateActiveText("Café naïve — edited\n")
        XCTAssertTrue(model.isDirty)
        XCTAssertTrue(model.saveTex())
        XCTAssertFalse(model.isDirty)
        XCTAssertEqual(try String(contentsOf: url, encoding: .utf8), "Café naïve — edited\n")

        model.openTex(at: dir.appendingPathComponent("missing.tex"))
        XCTAssertTrue(model.captureNote?.contains("Could not open") == true)
        XCTAssertEqual(model.activeText, "Café naïve — edited\n", "failed open leaves the buffer alone")
    }
}

/// Issue #19 (2): opening another file must not discard unsaved edits without
/// an explicit decision, and a discarded buffer stays recoverable.
@MainActor
final class DirtyOpenTests: XCTestCase {
    func testOpenIsBlockedWhileDirtyAndDiscardKeepsTheBufferRecoverable() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-dirty-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let a = dir.appendingPathComponent("a.tex"), b = dir.appendingPathComponent("b.tex")
        try "A original\n".write(to: a, atomically: true, encoding: .utf8)
        try "B text\n".write(to: b, atomically: true, encoding: .utf8)

        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: a), .opened)
        model.updateActiveText("A edited but unsaved\n")
        XCTAssertTrue(model.isDirty)

        // Default: refused, nothing replaced.
        XCTAssertEqual(model.openTex(at: b), .blockedByUnsavedEdits)
        XCTAssertEqual(model.activeText, "A edited but unsaved\n")
        XCTAssertEqual(model.documentURL, a)

        // Discard: replaced, but the prior source is recoverable.
        XCTAssertEqual(model.openTex(at: b, dirty: .discard), .opened)
        XCTAssertEqual(model.activeText, "B text\n")
        XCTAssertEqual(model.recoverableBuffer, .init(url: a, text: "A edited but unsaved\n"))
        XCTAssertEqual(try String(contentsOf: a, encoding: .utf8), "A original\n", "disk untouched by discard")
        XCTAssertTrue(model.restoreDiscardedBuffer())
        XCTAssertEqual(model.activeText, "A edited but unsaved\n")
        XCTAssertTrue(model.isDirty, "restored buffer is still unsaved")
        XCTAssertNil(model.recoverableBuffer)

        // Save first: the edit reaches disk, then the open proceeds.
        XCTAssertEqual(model.openTex(at: b, dirty: .saveFirst), .opened)
        XCTAssertEqual(try String(contentsOf: a, encoding: .utf8), "A edited but unsaved\n")
        XCTAssertFalse(model.isDirty)
    }
}
