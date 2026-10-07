import Foundation
import XCTest
@testable import FlashTeXMac

/// App-parity row A19 (several open documents) against a real host: with
/// `main.tex` (`\input{chap}`) and `chap.tex` both open, unsaved edits in
/// either reach the engine-v3 compile -- the host's project copy holds them,
/// the files on disk do not, and the preview places the new text.
/// Needs a built `flashtex-host` and a TeX Live (EngineV3TestHost).
@MainActor
final class EngineV3OpenDocumentsTests: XCTestCase {
    /// Environment set for a test and put back after it (never just unset).
    private var env = EnvironmentOverride()
    /// A private cache root: never the one a running app uses.
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-documents-\(getpid())")
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

    func waitUntil(_ what: String, timeout: TimeInterval = 90, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    struct NotSwitched: Error {}

    func switchTo(_ path: String, _ model: ShellModel, file: StaticString = #filePath, line: UInt = #line) throws {
        let outcome = model.project.switchDocument(to: path)
        guard case .switched = outcome, model.activePath == path else {
            XCTFail("switching to \(path): \(outcome)", file: file, line: line)
            throw NotSwitched()
        }
    }

    func testUnsavedEditsInSeveralOpenDocumentsReachTheCompile() async throws {
        try EngineV3TestHost.require()
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-documents-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let main = "\\documentclass{article}\n\\begin{document}\nAlpha beta gamma.\n\n\\input{chap}\n\\end{document}\n"
        let chap = "First chapter line.\nSecond chapter line.\n"
        let mainURL = dir.appendingPathComponent("main.tex"), chapURL = dir.appendingPathComponent("chap.tex")
        try main.write(to: mainURL, atomically: true, encoding: .utf8)
        try chap.write(to: chapURL, atomically: true, encoding: .utf8)

        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: mainURL, dirty: .discard), .opened)
        let opened = await model.project.openDocument("chap.tex")
        XCTAssertEqual(opened, .opened(path: "chap.tex"))
        model.engineV3Enabled = true
        model.engineV3.start(model: model)
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the first compile") { !s.compiling && s.statusNote.hasPrefix("ok") && s.pageCount == 1 }
        XCTAssertNotNil(s.place(path: "chap.tex", line: 2, col: nil), "chap.tex is typeset")
        XCTAssertNil(s.place(path: "chap.tex", line: 4, col: nil), "chap.tex has no line 4 yet")

        // An unsaved edit in the \input document (made the editor's document).
        try switchTo("chap.tex", model)
        let chapEdited = chap + "\nA new paragraph, typed and not saved.\n" // line 4
        model.updateActiveText(chapEdited)
        // Then one in the entry, so chap.tex's edit is no longer the active document's.
        try switchTo("main.tex", model)
        let mainEdited = main.replacingOccurrences(of: "Alpha beta gamma.", with: "Alpha beta gamma epsilon.")
        model.updateActiveText(mainEdited)
        XCTAssertTrue(model.project.isDirty("chap.tex"))
        XCTAssertTrue(model.project.isDirty("main.tex"))

        try await waitUntil("both edits compiled") {
            !s.compiling && s.statusNote.hasPrefix("ok") && s.place(path: "chap.tex", line: 4, col: nil) != nil
        }
        // The host's copy holds both edits; the user's files are untouched.
        let copy = try XCTUnwrap(s.projectCopy)
        XCTAssertEqual(try String(contentsOf: copy.appendingPathComponent("chap.tex"), encoding: .utf8), chapEdited)
        XCTAssertEqual(try String(contentsOf: copy.appendingPathComponent("main.tex"), encoding: .utf8), mainEdited)
        XCTAssertEqual(try String(contentsOf: chapURL, encoding: .utf8), chap, "chap.tex on disk is never written")
        XCTAssertEqual(try String(contentsOf: mainURL, encoding: .utf8), main, "main.tex on disk is never written")

        // The preview shows them: chap.tex's new paragraph (line 4) sits below its line 2
        // and reverse-searches back to it; main.tex's new word is placed.
        XCTAssertEqual(s.pageCount, 1)
        XCTAssertNil(s.firstError)
        let line2 = try XCTUnwrap(s.place(path: "chap.tex", line: 2, col: nil))
        let line4 = try XCTUnwrap(s.place(path: "chap.tex", line: 4, col: nil))
        XCTAssertGreaterThan(line4.rect.minY, line2.rect.maxY, "the new paragraph is below chap.tex's line 2")
        let back = try XCTUnwrap(s.source(page: line4.page, at: CGPoint(x: line4.rect.minX + 2, y: line4.rect.midY)))
        XCTAssertEqual(back.path, "chap.tex"); XCTAssertEqual(back.line, 4)
        let epsilon = mainEdited.utf8.distance(from: mainEdited.startIndex, to: mainEdited.range(of: "epsilon")!.lowerBound)
        let word = try XCTUnwrap(s.place(path: "main.tex", byte: epsilon, in: mainEdited))
        let hit = try XCTUnwrap(s.source(page: word.page, at: CGPoint(x: word.rect.midX, y: word.rect.midY)))
        XCTAssertEqual(hit.path, "main.tex"); XCTAssertEqual(hit.line, 3)
        XCTAssertEqual(hit.col, "Alpha beta gamma ".utf8.count)
    }
}
