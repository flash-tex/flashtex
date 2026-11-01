import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Preview → source into a document that is NOT open in the window
/// (ShellModel+UnopenedNavigation.swift): `main.tex` + `\input{chapter}` with
/// only `main.tex` opened. A hit naming `chapter.tex` has no open buffer, so
/// it is a typed refusal and no caret is placed; a hit in an open document
/// navigates exactly.
@MainActor
final class UnopenedNavigationTests: XCTestCase {
    static let main = "\\documentclass{article}\n\\begin{document}\nCaf\u{E9} office \\input{chapter}\nend done.\n\\end{document}\n"

    private func byte(_ text: String, _ needle: String) -> Int {
        let hay = Array(text.utf8), n = Array(needle.utf8)
        for i in 0...(hay.count - n.count) where Array(hay[i..<(i + n.count)]) == n { return i }
        XCTFail("\(needle) not found"); return 0
    }

    private func selectedBytes(_ model: ShellModel) throws -> (path: String, start: Int, end: Int, text: String) {
        let sel = try XCTUnwrap(model.selection, model.navigationNote ?? "nil")
        let bytes = try XCTUnwrap(model.activeText.utf8ByteRange(of: sel.nsRange))
        return (sel.path, bytes.start, bytes.end, (model.activeText as NSString).substring(with: sel.nsRange))
    }

    func testVerdictsWithoutAHelperAreTypedAndNeverPlaceACaret() async {
        let model = ShellModel()
        model.replaceProject(entryText: Self.main)
        let awaited1 = await model.openForNavigation(path: "main.tex", compiledRevision: 1)
        XCTAssertEqual(awaited1, .alreadyOpen(path: "main.tex"))
        let awaited2 = await model.openForNavigation(path: "chapter.tex", compiledRevision: 1)
        XCTAssertEqual(awaited2, .notAttached(path: "chapter.tex"))
        let hit = RuntimeV1.SourceRange(path: "chapter.tex", startByte: 0, endByte: 1)
        let verdict = await model.navigateOpeningIfNeeded(to: hit)
        XCTAssertEqual(verdict, .notAttached(path: "chapter.tex"))
        XCTAssertNil(model.selection)
        XCTAssertEqual(model.activePath, "main.tex")
        XCTAssertEqual(model.documents.map(\.path), ["main.tex"], "nothing was opened from disk")
        XCTAssertEqual(model.navigationNote, "Preview → source: " + UnopenedNavigation.Verdict.notAttached(path: "chapter.tex").note)
        // An open document goes straight to navigateExactly (nil verdict).
        let own = RuntimeV1.SourceRange(path: "main.tex", startByte: byte(Self.main, "office"), endByte: byte(Self.main, "office") + 6)
        let ownVerdict = await model.navigateOpeningIfNeeded(to: own)
        XCTAssertNil(ownVerdict)
        XCTAssertEqual(try selectedBytes(model).text, "office")
        // Every verdict note names the document.
        for v: UnopenedNavigation.Verdict in [.alreadyOpen(path: "c"), .notAttached(path: "c")] {
            XCTAssertTrue(v.note.contains("c"), v.note)
        }
    }
}
