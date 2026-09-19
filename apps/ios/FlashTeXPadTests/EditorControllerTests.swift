import FlashTeXEditorCore
import FlashTeXPadKit
import UIKit
import XCTest
@testable import FlashTeXPad

/// The editor's keystroke behaviour (lane-ipad-editor): auto-close with
/// type-over and pair deletion, the Return key's environment rules, the
/// bracket-match highlight, hardware key commands, the accessory bar and
/// snippet stops — each driven through the same delegate path a real
/// keystroke takes (`EditorController.type`), against the shared
/// FlashTeXEditorCore rules the Mac editor uses.
@MainActor
final class EditorControllerTests: XCTestCase {
    var editor: EditorController!
    var changes: [(text: String, caret: Int)] = []

    override func setUp() async throws {
        editor = EditorController()
        changes = []
        editor.onChange = { [unowned self] text, caret, _ in changes.append((text, caret)) }
    }

    private func load(_ text: String, caret: Int? = nil) {
        editor.load(text: text, caret: caret ?? (text as NSString).length, revision: 1)
    }

    private var caret: Int { editor.selectedRange.location }

    // MARK: auto-close

    func testBraceAutoClosesAndCaretSitsInside() {
        load("")
        editor.type("{")
        XCTAssertEqual(editor.text, "{}")
        XCTAssertEqual(caret, 1)
        XCTAssertEqual(editor.pendingClosers, [1])
        XCTAssertEqual(changes.last?.text, "{}", "the model sees the pair once, after the closer")
    }

    func testEveryConventionalPairCloses() {
        for (opener, expected) in [("{", "{}"), ("[", "[]"), ("(", "()"), ("$", "$$")] {
            load("")
            editor.type(opener)
            XCTAssertEqual(editor.text, expected, opener)
            XCTAssertEqual(caret, 1, opener)
        }
    }

    func testTypingTheCloserOvertypesInsteadOfDoubling() {
        load("")
        editor.type("{")
        editor.type("x")
        editor.type("}")
        XCTAssertEqual(editor.text, "{x}")
        XCTAssertEqual(caret, 3)
        XCTAssertEqual(editor.pendingClosers, [], "typed over: nothing pending")
        editor.type("}")
        XCTAssertEqual(editor.text, "{x}}", "a hand-typed closer is never stepped over")
    }

    func testBackspaceBetweenAPairRemovesBoth() {
        load("a")
        editor.type("{")
        XCTAssertEqual(editor.text, "a{}")
        editor.backspace()
        XCTAssertEqual(editor.text, "a")
        XCTAssertEqual(caret, 1)
        XCTAssertEqual(editor.pendingClosers, [])
    }

    func testBackspaceOnAHandTypedPairRemovesOneUnit() {
        load("{}", caret: 1)
        editor.backspace()
        XCTAssertEqual(editor.text, "}")
    }

    func testNoCloseBeforeAWordOrInAComment() {
        load("word", caret: 0)
        editor.type("{")
        XCTAssertEqual(editor.text, "{word", "an opener before a word is left alone")
        load("% note ")
        editor.type("{")
        XCTAssertEqual(editor.text, "% note {", "comments are not code")
        load("\\")
        editor.type("{")
        XCTAssertEqual(editor.text, "\\{", "an escaped brace is a literal")
    }

    func testDollarClosingOpenMathIsNotPaired() {
        load("$x")
        editor.type("$")
        XCTAssertEqual(editor.text, "$x$", "the second $ closes; no third")
    }

    func testBackslashParenAndBracketGetMathClosers() {
        load("\\")
        editor.type("(")
        XCTAssertEqual(editor.text, "\\(\\)")
        XCTAssertEqual(caret, 2)
        XCTAssertEqual(editor.pendingClosers, [2, 3])
        editor.type("x")
        // Only the terminal unit steps over the two-unit closer: `\` first, then `)`.
        editor.type("\\")
        XCTAssertEqual(editor.text, "\\(x\\\\)")
        editor.type(")")
        XCTAssertEqual(editor.text, "\\(x\\)")
        XCTAssertEqual(caret, 5)
        XCTAssertEqual(editor.pendingClosers, [])

        load("\\")
        editor.type("[")
        XCTAssertEqual(editor.text, "\\[\\]")
    }

    func testLeftParenInMathGetsRight() {
        load("$\\left")
        editor.type("(")
        XCTAssertEqual(editor.text, "$\\left(\\right)")
        XCTAssertEqual(caret, 7)
        load("\\left")
        editor.type("(")
        XCTAssertEqual(editor.text, "\\left()", "outside math \\left is a plain error: only the ordinary pair")
    }

    func testEditsBeforeAPendingCloserShiftIt() {
        load("")
        editor.type("{")
        editor.select(NSRange(location: 0, length: 0))
        editor.type("ab")
        XCTAssertEqual(editor.text, "ab{}")
        XCTAssertEqual(editor.pendingClosers, [3])
        editor.select(NSRange(location: 3, length: 0))
        editor.type("}")
        XCTAssertEqual(editor.text, "ab{}", "the shifted closer still overtypes")
    }

    // MARK: Return

    func testReturnKeepsIndentation() {
        load("    foo")
        editor.pressReturn()
        XCTAssertEqual(editor.text, "    foo\n    ")
        XCTAssertEqual(caret, 12)
    }

    func testReturnAfterBeginIndentsAndCloses() {
        load("\\begin{itemize}")
        editor.pressReturn()
        XCTAssertEqual(editor.text, "\\begin{itemize}\n    \\item \n\\end{itemize}")
        XCTAssertEqual(caret, ("\\begin{itemize}\n    \\item " as NSString).length)
    }

    func testReturnContinuesAnItemAndABareItemJustBreaks() {
        load("\\begin{itemize}\n    \\item one\n\\end{itemize}", caret: ("\\begin{itemize}\n    \\item one" as NSString).length)
        editor.pressReturn()
        XCTAssertEqual(editor.text, "\\begin{itemize}\n    \\item one\n    \\item \n\\end{itemize}")
        // Return on the bare `\item ` line: no new item, just a line break.
        editor.pressReturn()
        XCTAssertEqual(editor.text, "\\begin{itemize}\n    \\item one\n    \\item \n    \n\\end{itemize}")
    }

    func testReturnAfterBeginDocumentIsAPlainNewline() {
        // The Mac never auto-closes or indents `document` (nobody indents a
        // whole document body); the shared rule is the same here.
        load("\\begin{document}")
        editor.pressReturn()
        XCTAssertEqual(editor.text, "\\begin{document}\n")
        XCTAssertEqual(caret, 17)
    }

    func testReturnInDescriptionPlacesCaretInBrackets() {
        load("\\begin{description}")
        editor.pressReturn()
        XCTAssertEqual(editor.text, "\\begin{description}\n    \\item[] \n\\end{description}")
        XCTAssertEqual(caret, ("\\begin{description}\n    \\item[" as NSString).length)
    }

    func testReturnWhenEnvironmentAlreadyClosedDoesNotCloseAgain() {
        load("\\begin{center}\n\\end{center}", caret: 14)
        editor.pressReturn()
        XCTAssertEqual(editor.text, "\\begin{center}\n    \n\\end{center}")
    }

    // MARK: bracket / environment match

    func testMatchHighlightFollowsTheCaret() {
        load("a{b[c]}d", caret: 0)
        XCTAssertEqual(editor.matchRanges, [])
        editor.select(NSRange(location: 2, length: 0)) // after `{`
        XCTAssertEqual(editor.matchRanges, [NSRange(location: 1, length: 1), NSRange(location: 6, length: 1)])
        XCTAssertNotNil(editor.storage.attribute(.backgroundColor, at: 1, effectiveRange: nil))
        XCTAssertNotNil(editor.storage.attribute(.backgroundColor, at: 6, effectiveRange: nil))
        editor.select(NSRange(location: 5, length: 0)) // before `]`
        XCTAssertEqual(editor.matchRanges, [NSRange(location: 3, length: 1), NSRange(location: 5, length: 1)])
        XCTAssertNil(editor.storage.attribute(.backgroundColor, at: 1, effectiveRange: nil), "the old pair is cleared")
        editor.select(NSRange(location: 0, length: 0))
        XCTAssertEqual(editor.matchRanges, [])
        XCTAssertNil(editor.storage.attribute(.backgroundColor, at: 3, effectiveRange: nil))
    }

    func testMatchHighlightPairsDollars() {
        load("$x^2$ and", caret: 1)
        XCTAssertEqual(editor.matchRanges, [NSRange(location: 0, length: 1), NSRange(location: 4, length: 1)])
    }

    func testTypedTextDoesNotInheritTheMatchBackground() {
        load("{}", caret: 1)
        XCTAssertEqual(editor.matchRanges.count, 2)
        editor.type("x")
        XCTAssertEqual(editor.text, "{x}")
        XCTAssertNil(editor.storage.attribute(.backgroundColor, at: 1, effectiveRange: nil))
    }

    // MARK: key commands

    func testKeyCommandsAreDiscoverableWithTitles() {
        let commands = editor.textView.keyCommands ?? []
        func find(_ input: String, _ mods: UIKeyModifierFlags) -> UIKeyCommand? {
            commands.first { $0.input == input && $0.modifierFlags == mods }
        }
        XCTAssertEqual(find(" ", .control)?.title, "Show Completions")
        XCTAssertEqual(find(UIKeyCommand.inputEscape, [])?.title, "Dismiss Completions")
        XCTAssertEqual(find("\t", [])?.title, "Accept Completion / Next Placeholder")
        XCTAssertEqual(find("/", .command)?.title, "Toggle Comment")
        XCTAssertEqual(find("]", .command)?.title, "Indent")
        XCTAssertEqual(find("[", .command)?.title, "Outdent")
        XCTAssertEqual(find("b", [.command, .shift])?.title, "Bold (\\textbf)")
        XCTAssertEqual(find("i", .command)?.title, "Italic (\\textit)")
        XCTAssertEqual(find("e", .command)?.title, "Toggle Diagnostics")
        XCTAssertTrue(find("\t", [])!.wantsPriorityOverSystemBehavior, "Tab must beat the system's tab insertion")
        for c in commands { XCTAssertFalse(c.title.isEmpty, "\(c.input ?? "?") needs a title for the ⌘ HUD") }
    }

    func testKeyCommandRoutesToTheController() {
        load("a")
        let command = (editor.textView.keyCommands ?? []).first { $0.input == "/" && $0.modifierFlags == .command }!
        editor.textView.editorCommand(command)
        XCTAssertEqual(editor.text, "% a")
    }

    func testToggleCommentOnSelectionAndBack() {
        load("one\ntwo\n\nthree")
        editor.select(NSRange(location: 0, length: 14))
        editor.perform(.toggleComment)
        XCTAssertEqual(editor.text, "% one\n% two\n\n% three", "blank lines are left alone")
        editor.perform(.toggleComment)
        XCTAssertEqual(editor.text, "one\ntwo\n\nthree")
        // A selection ending at a line start excludes that line (the Mac rule).
        editor.select(NSRange(location: 0, length: 9))
        editor.perform(.toggleComment)
        XCTAssertEqual(editor.text, "% one\n% two\n\nthree")
    }

    func testIndentAndOutdent() {
        load("a\nb", caret: 0)
        editor.select(NSRange(location: 0, length: 3))
        editor.perform(.indent)
        XCTAssertEqual(editor.text, "    a\n    b")
        editor.perform(.outdent)
        XCTAssertEqual(editor.text, "a\nb")
        editor.select(NSRange(location: 1, length: 0))
        editor.perform(.indent)
        XCTAssertEqual(editor.text, "    a\nb")
        XCTAssertEqual(caret, 5, "the caret stays after `a`")
        editor.perform(.outdent)
        XCTAssertEqual(editor.text, "a\nb")
        XCTAssertEqual(caret, 1)
    }

    func testBoldAndItalicWrapOrOpen() {
        load("word")
        editor.select(NSRange(location: 0, length: 4))
        editor.perform(.bold)
        XCTAssertEqual(editor.text, "\\textbf{word}")
        XCTAssertEqual(editor.selectedRange, NSRange(location: 8, length: 4), "the wrapped text stays selected")
        load("")
        editor.perform(.italic)
        XCTAssertEqual(editor.text, "\\textit{}")
        XCTAssertEqual(caret, 8)
        editor.type("x")
        editor.type("}")
        XCTAssertEqual(editor.text, "\\textit{x}", "the placeholder closer overtypes")
    }

    func testModelCommandsGoToTheHandler() {
        var received: [EditorCommand] = []
        editor.onCommand = { received.append($0); return false }
        editor.perform(.showCompletions)
        editor.perform(.toggleDiagnostics)
        editor.perform(.dismiss)
        XCTAssertEqual(received, [.showCompletions, .toggleDiagnostics, .dismiss])
    }

    func testTabInsertsIndentWhenNothingElseApplies() {
        load("x")
        editor.onCommand = { _ in false }
        editor.pressTab()
        XCTAssertEqual(editor.text, "x    ")
    }

    // MARK: snippets and completion acceptance

    func testSnippetStopsVisitedByTab() {
        load("")
        let frac = LocalCompletion.Suggestion(kind: .command, text: "\\frac", detail: "", replaceStart: 0, replaceEnd: 0,
                                              snippet: LaTeXSnippets.argument(name: "frac", arguments: "{numerator}{denominator}"))
        editor.accept(frac, replacing: NSRange(location: 0, length: 0))
        XCTAssertEqual(editor.text, "\\frac{}{}")
        XCTAssertEqual(caret, 6)
        XCTAssertEqual(editor.snippetStops, [8, 9])
        editor.type("a")
        editor.onCommand = { _ in false }
        editor.pressTab()
        XCTAssertEqual(caret, 9, "second group (shifted by the typed `a`)")
        editor.type("b")
        editor.pressTab()
        XCTAssertEqual(caret, 11, "after the snippet")
        XCTAssertEqual(editor.text, "\\frac{a}{b}")
        XCTAssertEqual(editor.snippetStops, [])
    }

    func testEnvironmentSkeletonEatsTheAutoClosedBrace() {
        load("")
        editor.type("\\begin")
        editor.type("{")
        editor.type("ite")
        XCTAssertEqual(editor.text, "\\begin{ite}")
        let s = LocalCompletion.suggestions(in: editor.text, caretByte: 10, context: .init(vocabulary: PadModel.bundledVocabulary))
            .first { $0.text == "itemize" }!
        editor.accept(s, replacing: NSRange(location: 7, length: 3))
        XCTAssertEqual(editor.text, "\\begin{itemize}\n    \\item \n\\end{itemize}")
        XCTAssertEqual(caret, ("\\begin{itemize}\n    \\item " as NSString).length)
    }

    func testEscapeLeavesTheSnippet() {
        load("")
        editor.accept(LocalCompletion.Suggestion(kind: .command, text: "\\frac", detail: "", replaceStart: 0, replaceEnd: 0,
                                                 snippet: LaTeXSnippets.argument(name: "frac", arguments: "{n}{d}")),
                      replacing: NSRange(location: 0, length: 0))
        XCTAssertFalse(editor.snippetStops.isEmpty)
        editor.onCommand = { _ in false }
        editor.pressEscape()
        XCTAssertEqual(editor.snippetStops, [])
    }

    // MARK: accessory bar

    func testAccessoryBarInsertsThroughTheKeystrokePath() {
        editor.hardwareKeyboardOverride = false
        load("")
        editor.accessoryBar.tap(.backslash)
        editor.accessoryBar.tap(.braces)
        XCTAssertEqual(editor.text, "\\{", "after a backslash the brace is literal, so no pair")
        load("")
        editor.accessoryBar.tap(.braces)
        XCTAssertEqual(editor.text, "{}")
        XCTAssertEqual(caret, 1)
        load("")
        editor.accessoryBar.tap(.dollar)
        editor.accessoryBar.tap(.frac)
        XCTAssertEqual(editor.text, "$\\frac{}{}$")
        XCTAssertEqual(caret, 7)
        load("")
        editor.accessoryBar.tap(.sqrt)
        XCTAssertEqual(editor.text, "\\sqrt{}")
        load("")
        editor.accessoryBar.tap(.caret)
        editor.accessoryBar.tap(.underscore)
        editor.accessoryBar.tap(.brackets)
        XCTAssertEqual(editor.text, "^_[]")
        load("")
        editor.accessoryBar.tap(.item)
        XCTAssertEqual(editor.text, "\\item ")
        var asked: [EditorCommand] = []
        editor.onCommand = { asked.append($0); return false }
        load("")
        editor.accessoryBar.tap(.begin)
        XCTAssertEqual(editor.text, "\\begin{}")
        XCTAssertEqual(caret, 7)
        XCTAssertEqual(asked, [.showCompletions])
        editor.accessoryBar.tap(.tab)
        XCTAssertEqual(editor.text, "\\begin{    }")
    }

    func testAccessoryBarHidesWithAHardwareKeyboard() {
        editor.hardwareKeyboardOverride = false
        XCTAssertTrue(editor.accessoryBarShown)
        XCTAssertEqual(editor.accessoryBar.buttons.count, EditorAccessoryBar.Item.allCases.count)
        editor.hardwareKeyboardOverride = true
        XCTAssertFalse(editor.accessoryBarShown)
        editor.hardwareKeyboardOverride = false
        XCTAssertTrue(editor.accessoryBarShown)
    }

    // MARK: reporting

    func testEveryChangeReachesTheModelOnceWithTheCaret() {
        load("")
        editor.type("a")
        editor.type("{")
        editor.pressReturn()
        XCTAssertEqual(changes.map(\.text), ["a", "a{}", "a{\n}"])
        XCTAssertEqual(changes.map(\.caret), [1, 2, 3])
    }

    func testUndoRestoresTheTextAndForgetsPendingClosers() throws {
        load("")
        editor.type("{")
        XCTAssertEqual(editor.text, "{}")
        let undo = try XCTUnwrap(editor.textView.undoManager)
        XCTAssertTrue(undo.canUndo)
        undo.undo()
        XCTAssertEqual(editor.pendingClosers, [], "after an edit the delegate did not see, nothing is trusted")
    }
}
