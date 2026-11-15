import XCTest
@testable import FlashTeXPadKit

/// `LocalCompletion.reanchored`: a suggestion from a pass one keystroke
/// behind the text is applied to the word under the caret now, or not at all.
final class LocalCompletionReanchorTests: XCTestCase {
    private func first(_ text: String) throws -> LocalCompletion.Suggestion {
        try XCTUnwrap(LocalCompletion.suggestions(in: text, caretByte: text.utf8.count).first)
    }

    func testTheSnapshotItCameFromIsUnchanged() throws {
        let text = "\\label{fig:a} \\ref{fi"
        let s = try first(text)
        XCTAssertEqual(LocalCompletion.reanchored(s, in: text, caretByte: text.utf8.count), s)
    }

    func testOneMoreWordCharacterExtendsTheRange() throws {
        let s = try first("é \\label{fig:a} \\ref{fi")
        let now = "é \\label{fig:a} \\ref{fig"
        let r = try XCTUnwrap(LocalCompletion.reanchored(s, in: now, caretByte: now.utf8.count))
        XCTAssertEqual(r.replaceStart, s.replaceStart)
        XCTAssertEqual(r.replaceEnd, now.utf8.count)
    }

    func testTheWordEndingDropsIt() throws {
        let s = try first("\\alpha \\al")
        XCTAssertNil(LocalCompletion.reanchored(s, in: "\\alpha \\al ", caretByte: 11))
    }

    func testTypingPastTheSuggestionDropsIt() throws {
        let s = try first("\\alpha \\al")
        XCTAssertEqual(s.text, "\\alpha")
        XCTAssertNil(LocalCompletion.reanchored(s, in: "\\alpha \\alx", caretByte: 11))
    }

    func testAnEditBeforeTheWordDropsIt() throws {
        let s = try first("\\alpha \\al")
        // A character inserted earlier moved the word one byte right.
        XCTAssertNil(LocalCompletion.reanchored(s, in: "x\\alpha \\al", caretByte: 11))
    }

    func testOutOfRangeCaretIsNil() throws {
        let s = try first("\\alpha \\al")
        XCTAssertNil(LocalCompletion.reanchored(s, in: "\\al", caretByte: 99))
        XCTAssertNil(LocalCompletion.reanchored(s, in: "\\alpha \\al", caretByte: 2))
    }
}
