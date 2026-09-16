import XCTest
import FlashTeXAccessibility
@testable import FlashTeXMac

/// Go to Line: the pure parser/resolver (EditorNavigation.swift) and the
/// command-palette `:N` route (CommandPalette.swift).
@MainActor
final class GoToLineTests: XCTestCase {
    typealias EN = EditorNavigation

    private func spec(_ input: String) -> EN.LineSpec {
        switch EN.parseLineTarget(input) {
        case .success(let s): return s
        case .failure(let h): XCTFail("parse \(input): \(h)"); return .absolute(line: -1, column: nil)
        }
    }

    private func target(_ text: String, _ input: String, caret: Int = 0) -> EN.LineTarget {
        switch EN.resolveLineTarget(text, input: input, caret: caret) {
        case .success(let t): return t
        case .failure(let h): XCTFail("resolve \(input): \(h.message)"); return EN.LineTarget(utf16: -1, line: 0, column: 0)
        }
    }

    private func hint(_ input: String) -> String {
        if case .failure(let h) = EN.parseLineTarget(input) { return h.message }
        XCTFail("expected a parse failure for \(input)")
        return ""
    }

    // MARK: parser

    func testParseAbsoluteLineColumnRelativeAndInvalid() {
        XCTAssertEqual(spec("42"), .absolute(line: 42, column: nil))
        XCTAssertEqual(spec(" 42 "), .absolute(line: 42, column: nil))
        XCTAssertEqual(spec(":42"), .absolute(line: 42, column: nil), "palette colon")
        XCTAssertEqual(spec("42:7"), .absolute(line: 42, column: 7))
        XCTAssertEqual(spec(":42:7"), .absolute(line: 42, column: 7))
        XCTAssertEqual(spec("+5"), .relative(delta: 5))
        XCTAssertEqual(spec("-5"), .relative(delta: -5))
        XCTAssertEqual(spec(":+5"), .relative(delta: 5))
        XCTAssertEqual(spec("0"), .absolute(line: 0, column: nil), "zero is parsed; resolve clamps")
        XCTAssertEqual(spec("007"), .absolute(line: 7, column: nil))
        XCTAssertEqual(hint(""), EN.emptyLineTargetHint)
        XCTAssertEqual(hint("   "), EN.emptyLineTargetHint)
        XCTAssertEqual(hint(":"), EN.emptyLineTargetHint)
        XCTAssertEqual(spec(":7"), .absolute(line: 7, column: nil), "a leading colon is the palette prefix, not a missing line")
        XCTAssertEqual(hint("foo"), EN.invalidLineTargetHint)
        XCTAssertEqual(hint("42:"), EN.invalidLineTargetHint)
        XCTAssertEqual(hint("42:7:8"), EN.invalidLineTargetHint)
        XCTAssertEqual(hint("1.5"), EN.invalidLineTargetHint)
        XCTAssertEqual(hint("++5"), EN.invalidLineTargetHint)
        XCTAssertEqual(hint("--5"), EN.invalidLineTargetHint)
        XCTAssertEqual(hint("+"), EN.invalidLineTargetHint)
        XCTAssertEqual(hint("42 : 7"), EN.invalidLineTargetHint)
        XCTAssertEqual(hint("42a"), EN.invalidLineTargetHint)
    }

    func testParseSaturatesHugeNumbersRatherThanFailing() {
        let huge = String(repeating: "9", count: 40)
        XCTAssertEqual(spec(huge), .absolute(line: Int.max, column: nil))
        XCTAssertEqual(spec("1:" + huge), .absolute(line: 1, column: Int.max))
        XCTAssertEqual(spec("+" + huge), .relative(delta: Int.max))
    }

    // MARK: resolve — clamping

    func testResolveClampsOutOfRangeLineAndColumn() {
        let t = "ab\ncd\n"
        // 2 content lines + empty last line after the trailing newline.
        XCTAssertEqual(target(t, "1"), EN.LineTarget(utf16: 0, line: 1, column: 1))
        XCTAssertEqual(target(t, "2"), EN.LineTarget(utf16: 3, line: 2, column: 1))
        XCTAssertEqual(target(t, "3"), EN.LineTarget(utf16: 6, line: 3, column: 1))
        XCTAssertEqual(target(t, "99"), EN.LineTarget(utf16: 6, line: 3, column: 1), "line clamps to last")
        XCTAssertEqual(target(t, "0"), EN.LineTarget(utf16: 0, line: 1, column: 1), "line 0 clamps to 1")
        XCTAssertEqual(target(t, "1:1"), EN.LineTarget(utf16: 0, line: 1, column: 1))
        XCTAssertEqual(target(t, "1:2"), EN.LineTarget(utf16: 1, line: 1, column: 2))
        XCTAssertEqual(target(t, "1:3"), EN.LineTarget(utf16: 2, line: 1, column: 3), "after last grapheme, before newline")
        XCTAssertEqual(target(t, "1:99"), EN.LineTarget(utf16: 2, line: 1, column: 3), "column clamps to last")
        XCTAssertEqual(target(t, "1:0"), EN.LineTarget(utf16: 0, line: 1, column: 1), "column 0 clamps to 1")
        XCTAssertEqual(target(t, "2:2"), EN.LineTarget(utf16: 4, line: 2, column: 2))
    }

    func testResolveRelativeIsFromTheCaretLine() {
        let t = "a\nb\nc\n"
        // caret on 'b' (utf16 2) is line 2; +1 → line 3 "c", -1 → line 1 "a".
        XCTAssertEqual(target(t, "+1", caret: 2), EN.LineTarget(utf16: 4, line: 3, column: 1))
        XCTAssertEqual(target(t, "-1", caret: 2), EN.LineTarget(utf16: 0, line: 1, column: 1))
        XCTAssertEqual(target(t, "+0", caret: 2), EN.LineTarget(utf16: 2, line: 2, column: 1))
        XCTAssertEqual(target(t, "-99", caret: 2), EN.LineTarget(utf16: 0, line: 1, column: 1))
        XCTAssertEqual(target(t, "+99", caret: 2), EN.LineTarget(utf16: 6, line: 4, column: 1), "clamps to empty last line")
        XCTAssertEqual(target(t, "+1", caret: 0), EN.LineTarget(utf16: 2, line: 2, column: 1))
        XCTAssertEqual(target("", "+5", caret: 0), EN.LineTarget(utf16: 0, line: 1, column: 1))
        XCTAssertEqual(target("only", "-1", caret: 2), EN.LineTarget(utf16: 0, line: 1, column: 1))
    }

    // MARK: resolve — CRLF, trailing newline, non-ASCII

    func testResolveUsesNSStringLineBreaksOnCRLFAndTrailingNewline() {
        let crlf = "aa\r\nbb\r\n"
        XCTAssertEqual(target(crlf, "1"), EN.LineTarget(utf16: 0, line: 1, column: 1))
        XCTAssertEqual(target(crlf, "2"), EN.LineTarget(utf16: 4, line: 2, column: 1), "CRLF is one terminator")
        XCTAssertEqual(target(crlf, "3"), EN.LineTarget(utf16: 8, line: 3, column: 1), "trailing CRLF is an empty last line")
        XCTAssertEqual(target(crlf, "2:3"), EN.LineTarget(utf16: 6, line: 2, column: 3))
        XCTAssertEqual(target(crlf, "1:99"), EN.LineTarget(utf16: 2, line: 1, column: 3))

        let cr = "aa\rbb"
        XCTAssertEqual(target(cr, "2"), EN.LineTarget(utf16: 3, line: 2, column: 1))
        XCTAssertEqual(target("aa\rbb\r", "3"), EN.LineTarget(utf16: 6, line: 3, column: 1))

        let noNL = "only"
        XCTAssertEqual(target(noNL, "1:5"), EN.LineTarget(utf16: 4, line: 1, column: 5))
        XCTAssertEqual(target(noNL, "2"), EN.LineTarget(utf16: 0, line: 1, column: 1), "one-line buffer clamps")

        let trailing = "x\n"
        XCTAssertEqual(target(trailing, "2"), EN.LineTarget(utf16: 2, line: 2, column: 1))
        XCTAssertEqual(target(trailing, "+1", caret: 0), EN.LineTarget(utf16: 2, line: 2, column: 1))
        XCTAssertEqual(target(trailing, "+1", caret: 2), EN.LineTarget(utf16: 2, line: 2, column: 1), "caret on the empty last line")
    }

    func testResolveColumnsCountGraphemeClustersOnNonASCII() {
        // NFC café: 4 graphemes, é is one UTF-16 unit. Combining café: 5 UTF-16, 4 graphemes.
        let nfc = "café\n"
        XCTAssertEqual((nfc as NSString).length, 5)
        XCTAssertEqual(target(nfc, "1:4"), EN.LineTarget(utf16: 3, line: 1, column: 4), "on é")
        XCTAssertEqual(target(nfc, "1:5"), EN.LineTarget(utf16: 4, line: 1, column: 5), "after é, before newline")
        XCTAssertEqual(SourceEditorView.lineColumn(text: nfc, utf16: target(nfc, "1:4").utf16)?.column, 4)

        let combining = "cafe\u{0301}\n" // e + combining acute
        XCTAssertEqual((combining as NSString).length, 6)
        XCTAssertEqual(Array(combining.dropLast()).count, 4, "four graphemes before the newline")
        XCTAssertEqual(target(combining, "1:4").utf16, 3, "column 4 is the start of é (e + mark)")
        XCTAssertEqual(target(combining, "1:5").utf16, 5, "after the combining cluster (2 UTF-16 units)")
        XCTAssertEqual(SourceEditorView.lineColumn(text: combining, utf16: target(combining, "1:4").utf16)?.column, 4)
        XCTAssertEqual(SourceEditorView.lineColumn(text: combining, utf16: target(combining, "1:5").utf16)?.column, 5)

        // 🎉 is one grapheme, a surrogate pair (2 UTF-16). Line 2: 🎉é
        let emoji = "x\n🎉é"
        XCTAssertEqual((emoji as NSString).length, 1 + 1 + 2 + 1)
        XCTAssertEqual(target(emoji, "2:1").utf16, 2)
        XCTAssertEqual(target(emoji, "2:2").utf16, 4, "after the emoji, on é")
        XCTAssertEqual(target(emoji, "2:3").utf16, 5, "after é")
        XCTAssertEqual(target(emoji, "2:99").utf16, 5)
        XCTAssertEqual(SourceEditorView.lineColumn(text: emoji, utf16: target(emoji, "2:2").utf16)?.column, 2)
        XCTAssertEqual(SourceEditorView.lineColumn(text: emoji, utf16: target(emoji, "2:2").utf16)?.line, 2)
    }

    func testResolveInvalidInputStaysAHint() {
        if case .failure(let h) = EN.resolveLineTarget("abc", input: "nope", caret: 0) {
            XCTAssertEqual(h.message, EN.invalidLineTargetHint)
        } else { XCTFail("expected failure") }
        if case .failure(let h) = EN.resolveLineTarget("abc", input: "", caret: 0) {
            XCTAssertEqual(h.message, EN.emptyLineTargetHint)
        } else { XCTFail("expected failure") }
    }

    // MARK: palette :N routing

    func testPaletteListsGoToLineAndColonQueryJumpsDirectly() {
        XCTAssertTrue(CommandPaletteModel.isRunnable(.goToLine))
        XCTAssertEqual(AccessibilityCommand.goToLine.entry.menu, "Navigate")
        XCTAssertEqual(AccessibilityCommand.goToLine.entry.menuItem, "Go to Line…")
        XCTAssertEqual(AccessibilityCommand.goToLine.entry.shortcuts, ["⌘L"])
        XCTAssertEqual(CommandPaletteModel.rows(matching: "go to line").first?.id, .goToLine)
        XCTAssertEqual(CommandPaletteModel.rows(matching: "⌘L").first?.id, .goToLine)

        XCTAssertEqual(CommandPaletteModel.lineJumpInput(from: ":42"), ":42")
        XCTAssertEqual(CommandPaletteModel.lineJumpInput(from: " :42:7 "), ":42:7")
        XCTAssertEqual(CommandPaletteModel.lineJumpInput(from: ":+5"), ":+5")
        XCTAssertEqual(CommandPaletteModel.lineJumpInput(from: ":-3"), ":-3")
        XCTAssertNil(CommandPaletteModel.lineJumpInput(from: "42"), "without a colon, the palette still filters commands")
        XCTAssertNil(CommandPaletteModel.lineJumpInput(from: ":"))
        XCTAssertNil(CommandPaletteModel.lineJumpInput(from: ":nope"))
        XCTAssertNil(CommandPaletteModel.lineJumpInput(from: "go to line"))

        let m = ShellModel()
        m.replaceProject(entryText: "a\nb\nc\n")
        m.caretUTF16 = 0
        XCTAssertTrue(CommandPaletteModel.performLineJump(":2", model: m))
        XCTAssertEqual(m.caretUTF16, 2)
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 2, length: 0))
        XCTAssertEqual(m.navigationNote, "Line 2, column 1.")
        XCTAssertTrue(CommandPaletteModel.performLineJump(":3:1", model: m))
        XCTAssertEqual(m.caretUTF16, 4)
        XCTAssertFalse(CommandPaletteModel.performLineJump("2", model: m), "not a colon query")
        XCTAssertEqual(m.caretUTF16, 4, "a non-colon query must not move")
        m.caretUTF16 = 2
        XCTAssertTrue(CommandPaletteModel.performLineJump(":+1", model: m))
        XCTAssertEqual(m.caretUTF16, 4, ":+1 from line 2 is line 3")
    }

    // MARK: model apply / cancel

    func testApplyGoToLineSelectsAndCancelRestores() {
        let m = ShellModel()
        m.replaceProject(entryText: "aa\nbb\ncc")
        m.caretUTF16 = 0
        m.caretLengthUTF16 = 2
        m.selection = .init(path: "main.tex", nsRange: NSRange(location: 0, length: 2), token: 1)
        m.presentGoToLine()
        XCTAssertTrue(m.editorNavigation.goToLineShown)
        XCTAssertEqual(m.editorNavigation.goToLineHint, EN.emptyLineTargetHint)
        m.editorNavigation.goToLineInput = "nope"
        m.refreshGoToLineHint()
        XCTAssertEqual(m.editorNavigation.goToLineHint, EN.invalidLineTargetHint)
        XCTAssertFalse(m.applyGoToLine(), "invalid text stays in the sheet")
        XCTAssertTrue(m.editorNavigation.goToLineShown)
        XCTAssertEqual(m.caretUTF16, 0, "caret unmoved until a valid Return")
        m.editorNavigation.goToLineInput = "2:2"
        XCTAssertTrue(m.applyGoToLine())
        XCTAssertFalse(m.editorNavigation.goToLineShown)
        XCTAssertEqual(m.caretUTF16, 4)
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 4, length: 0))
        XCTAssertEqual(m.navigationNote, "Line 2, column 2.")

        m.caretUTF16 = 4
        m.caretLengthUTF16 = 0
        m.selection = .init(path: "main.tex", nsRange: NSRange(location: 4, length: 0), token: 3)
        m.presentGoToLine()
        m.editorNavigation.goToLineInput = "1"
        m.cancelGoToLine()
        XCTAssertFalse(m.editorNavigation.goToLineShown)
        XCTAssertEqual(m.caretUTF16, 4, "Esc restores the caret from when the sheet opened")
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 4, length: 0))
        XCTAssertNil(m.editorNavigation.goToLineRestore)
    }
}
