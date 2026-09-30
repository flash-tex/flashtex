import FlashTeXEditorCore
import FlashTeXPadKit
import UIKit
import XCTest
@testable import FlashTeXPad

/// Linked `\begin{…}` / `\end{…}` name editing on the iPad (IPAD-ENV-SYNC):
/// the Mac's `LinkedEnvironmentRenameTests` scenario list, driven through a
/// `UITextView` hosted in a key window and first responder, over the input
/// paths the keyboards use: `insertText` / `deleteBackward` (the software and
/// hardware keyboards), `paste(_:)`, marked text (IME), the completion list's
/// `accept(_:replacing:)`, the accessory bar's keystroke path and the text
/// view's own undo manager.
@MainActor
final class LinkedEnvironmentRenameTests: XCTestCase {
    private var editor: EditorController!
    private var window: UIWindow?
    private var reported: [String] = []

    private static let doc = "\\begin{itemize}\n\\item a\n\\end{itemize}\n"

    override func setUp() async throws {
        editor = EditorController()
        reported = []
        editor.onChange = { [unowned self] text, _, _ in reported.append(text) }
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        let window = UIWindow(windowScene: scene)
        let host = UIViewController()
        window.rootViewController = host
        editor.textView.frame = CGRect(x: 0, y: 0, width: 600, height: 400)
        host.view.addSubview(editor.textView)
        window.makeKeyAndVisible()
        self.window = window
    }

    override func tearDown() async throws {
        editor.textView.resignFirstResponder()
        window?.isHidden = true
        window = nil
        editor = nil
    }

    private var tv: EditorTextView { editor.textView }
    private var undo: UndoManager { tv.undoManager! }

    private func load(_ text: String) async throws {
        editor.load(text: text, caret: 0, revision: 1)
        XCTAssertTrue(tv.becomeFirstResponder(), "the hosted text view takes the keyboard")
        try await turn()
        undo.removeAllActions()
    }

    /// Lets the run loop close the event's undo group, as between two real keys.
    private func turn() async throws { try await Task.sleep(nanoseconds: 20_000_000) }

    private func name(_ name: String, _ which: String, in text: String? = nil) -> NSRange {
        let t = (text ?? editor.text) as NSString
        let r = t.range(of: "\\\(which){\(name)}")
        return NSRange(location: r.location + which.utf16.count + 2, length: name.utf16.count)
    }

    private func end(of r: NSRange) -> NSRange { NSRange(location: NSMaxRange(r), length: 0) }

    /// Keyboard typing: one `insertText` per character (UIKeyInput, which
    /// both the software and a hardware keyboard drive).
    private func type(_ s: String) async throws {
        for ch in s {
            tv.insertText(String(ch))
            try await turn()
        }
    }

    private func backspace(times: Int = 1) async throws {
        for _ in 0..<times {
            tv.deleteBackward()
            try await turn()
        }
    }

    private func assertSynced(_ expected: String, _ label: String, file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertEqual(editor.text, expected, label, file: file, line: line)
        XCTAssertEqual(reported.last, editor.text, "\(label): the model saw the synced buffer", file: file, line: line)
        XCTAssertNil(editor.linkedSession, "\(label): no session outlives its edit", file: file, line: line)
    }

    private func environment(_ label: String) -> LocalCompletion.Suggestion {
        let snippet = LaTeXSnippets.environment(label, indent: "", unit: "    ", rules: .conventional)
        return LocalCompletion.Suggestion(kind: .environment, text: label, detail: "", replaceStart: 0, replaceEnd: 0, snippet: snippet)
    }

    // MARK: typing

    func testTypingOverTheBeginNameUpdatesEnd() async throws {
        try await load(Self.doc)
        editor.select(name("itemize", "begin"))
        try await type("enu")
        assertSynced("\\begin{enu}\n\\item a\n\\end{enu}\n", "typed over the begin name")
        XCTAssertEqual(editor.selectedRange, end(of: name("enu", "begin")))
    }

    func testTypingOverTheEndNameUpdatesBeginAndKeepsTheCaret() async throws {
        try await load(Self.doc)
        editor.select(name("itemize", "end"))
        try await type("enu")
        assertSynced("\\begin{enu}\n\\item a\n\\end{enu}\n", "typed over the end name")
        XCTAssertEqual(editor.selectedRange, end(of: name("enu", "end")),
                       "the caret stays after the typed letters when the begin name before it changes length")
    }

    func testAppendingToTheBeginName() async throws {
        try await load(Self.doc)
        editor.select(end(of: name("itemize", "begin")))
        try await type("*x")
        assertSynced("\\begin{itemize*x}\n\\item a\n\\end{itemize*x}\n", "appended to the begin name")
    }

    func testAccessoryBarKeystrokePathSyncs() async throws {
        try await load(Self.doc)
        editor.select(end(of: name("itemize", "end")))
        editor.type("x")
        assertSynced("\\begin{itemizex}\n\\item a\n\\end{itemizex}\n", "the controller's keystroke path")
        try await turn()
        undo.undo()
        XCTAssertEqual(editor.text, Self.doc, "one undo reverts the keystroke in both names")
        undo.redo()
        XCTAssertEqual(editor.text, "\\begin{itemizex}\n\\item a\n\\end{itemizex}\n", "redo reapplies both")
        try await turn()
        editor.backspace()
        assertSynced(Self.doc, "its backspace")
    }

    // MARK: deletion, emptied names, replacement, paste

    func testBackspacingTheWholeNameThenTypingANewOne() async throws {
        try await load(Self.doc)
        editor.select(end(of: name("itemize", "begin")))
        try await backspace(times: 7)
        assertSynced("\\begin{}\n\\item a\n\\end{}\n", "emptied the begin name")
        try await type("enumerate")
        assertSynced("\\begin{enumerate}\n\\item a\n\\end{enumerate}\n", "typed a new name into the emptied begin name")
    }

    func testBackspacingTheWholeEndNameThenTypingANewOne() async throws {
        try await load(Self.doc)
        editor.select(end(of: name("itemize", "end")))
        try await backspace(times: 7)
        try await type("enumerate")
        assertSynced("\\begin{enumerate}\n\\item a\n\\end{enumerate}\n", "retyped the end name")
    }

    func testSelectingTheNameAndDeletingEmptiesBothAndOneUndoRestores() async throws {
        try await load(Self.doc)
        editor.select(name("itemize", "begin"))
        try await backspace()
        assertSynced("\\begin{}\n\\item a\n\\end{}\n", "deleted the selected begin name")
        undo.undo()
        XCTAssertEqual(editor.text, Self.doc, "one undo restores both names")
    }

    /// Paste and dictation land as one multi-unit replacement of the
    /// selection through `UITextInput.replace(_:withText:)`.
    func testPasteOverTheNameSyncsAndUndoesAsOneStep() async throws {
        try await load(Self.doc)
        editor.select(name("itemize", "begin"))
        tv.replace(try XCTUnwrap(tv.selectedTextRange), withText: "description")
        try await turn()
        assertSynced("\\begin{description}\n\\item a\n\\end{description}\n", "pasted over the begin name")
        undo.undo()
        XCTAssertEqual(editor.text, Self.doc, "one undo reverts the paste in both names")
        undo.redo()
        XCTAssertEqual(editor.text, "\\begin{description}\n\\item a\n\\end{description}\n", "redo reapplies both")
    }

    func testMultiUnitInsertTextOverTheEndName() async throws {
        try await load(Self.doc)
        editor.select(name("itemize", "end"))
        tv.insertText("enumerate")
        try await turn()
        assertSynced("\\begin{enumerate}\n\\item a\n\\end{enumerate}\n", "a multi-unit insertText over the end name")
        undo.undo()
        XCTAssertEqual(editor.text, Self.doc, "one undo reverts both")
    }

    func testDeleteAllIsAPlainEdit() async throws {
        try await load(Self.doc)
        tv.selectAll(nil)
        try await backspace()
        XCTAssertEqual(editor.text, "")
        XCTAssertNil(editor.linkedSession)
        undo.undo()
        XCTAssertEqual(editor.text, Self.doc)
    }

    // MARK: completion acceptance

    func testAcceptingACompletionInTheBeginNameRenamesBothWithoutASkeleton() async throws {
        try await load(Self.doc)
        editor.select(name("itemize", "begin"))
        try await type("enumer")
        XCTAssertEqual(editor.text, "\\begin{enumer}\n\\item a\n\\end{enumer}\n")
        editor.accept(environment("enumerate"), replacing: name("enumer", "begin"))
        assertSynced("\\begin{enumerate}\n\\item a\n\\end{enumerate}\n", "accepted enumerate in the begin name")
        XCTAssertEqual(editor.selectedRange, end(of: name("enumerate", "begin")))
        XCTAssertEqual(editor.snippetStops, [], "a rename has no skeleton placeholders")
        undo.undo()
        XCTAssertEqual(editor.text, "\\begin{enumer}\n\\item a\n\\end{enumer}\n", "one undo reverts the accepted completion in both names")
    }

    func testAcceptingACompletionInTheEndNameRenamesBoth() async throws {
        try await load(Self.doc)
        editor.select(name("itemize", "end"))
        try await type("enumer")
        editor.accept(environment("enumerate"), replacing: name("enumer", "end"))
        assertSynced("\\begin{enumerate}\n\\item a\n\\end{enumerate}\n", "accepted enumerate in the end name")
        XCTAssertEqual(editor.selectedRange, end(of: name("enumerate", "end")))
        undo.undo()
        XCTAssertEqual(editor.text, "\\begin{enumer}\n\\item a\n\\end{enumer}\n")
    }

    /// The caret mid-name: the accepted name replaces the whole name span,
    /// not only the prefix before the caret (the list's replace range).
    func testAcceptingACompletionMidNameReplacesTheWholeName() async throws {
        try await load("\\begin{fig}\n\\centering\n\\end{fig}\n")
        let n = name("fig", "begin")
        editor.select(NSRange(location: n.location + 2, length: 0)) // fi|g
        editor.accept(environment("figure*"), replacing: NSRange(location: n.location, length: 2))
        assertSynced("\\begin{figure*}\n\\centering\n\\end{figure*}\n", "accepted figure* mid-name")
    }

    /// The suggestion the real completion pass offers (its snippet included).
    func testAcceptingALiveSuggestionInTheNameRenames() async throws {
        try await load(Self.doc)
        editor.select(name("itemize", "begin"))
        try await type("enumer")
        let caret = editor.selectedRange.location
        let byte = try XCTUnwrap(LaTeXEditing.utf8Offset(of: caret, in: editor.text))
        let s = try XCTUnwrap(LocalCompletion.suggestions(in: editor.text, caretByte: byte, context: .init(vocabulary: PadModel.bundledVocabulary))
            .first { $0.kind == .environment && $0.text == "enumerate" })
        XCTAssertNotNil(s.snippet, "the list offers a skeleton; a rename must not insert it")
        let r = try XCTUnwrap(editor.text.rangeOfUTF8(start: s.replaceStart, end: s.replaceEnd))
        editor.accept(s, replacing: NSRange(r, in: editor.text))
        assertSynced("\\begin{enumerate}\n\\item a\n\\end{enumerate}\n", "accepted the live suggestion")
    }

    /// A `\begin{` typed now (its `}` auto-closed and pending) still opens a
    /// new environment with its skeleton, even though a nested
    /// `\begin{itemize` pairs by name with the outer `\end{itemize}`.
    func testAcceptingANewNestedEnvironmentStillInsertsItsSkeleton() async throws {
        try await load("\\begin{itemize}\n\n\\end{itemize}\n")
        editor.select(NSRange(location: 16, length: 0))
        editor.type("\\begin") // the keyboard's delegate path, where auto-close lives
        editor.type("{")
        XCTAssertEqual(editor.text, "\\begin{itemize}\n\\begin{}\n\\end{itemize}\n", "the brace auto-closes")
        editor.type("itemize")
        editor.accept(environment("itemize"), replacing: NSRange(location: 23, length: 7))
        let ns = editor.text as NSString
        XCTAssertEqual(ns.components(separatedBy: "\\end{itemize}").count - 1, 2, "a second \\end{itemize}: \(editor.text)")
        XCTAssertTrue(editor.text.hasPrefix("\\begin{itemize}\n\\begin{itemize}\n"), editor.text)
        XCTAssertFalse(editor.text.contains("}}"), editor.text)
    }

    // MARK: structure

    func testNestedSameNameLinksTheMatchingEnd() async throws {
        let text = "\\begin{itemize}\n\\begin{itemize}\n\\item a\n\\end{itemize}\n\\end{itemize}\n"
        try await load(text)
        let inner = (text as NSString).range(of: "itemize", range: NSRange(location: 10, length: 40))
        editor.select(inner)
        try await type("enu")
        assertSynced("\\begin{itemize}\n\\begin{enu}\n\\item a\n\\end{enu}\n\\end{itemize}\n", "renamed the inner pair")
        editor.select(end(of: name("itemize", "end")))
        try await type("x")
        assertSynced("\\begin{itemizex}\n\\begin{enu}\n\\item a\n\\end{enu}\n\\end{itemizex}\n", "appended to the outer end")
    }

    func testBeginWithAnOptionalArgument() async throws {
        let text = "\\begin{enumerate}[label=(\\alph*)]\n\\item a\n\\end{enumerate}\n"
        try await load(text)
        editor.select(name("enumerate", "begin"))
        try await type("itemi")
        editor.accept(environment("itemize"), replacing: name("itemi", "begin"))
        assertSynced("\\begin{itemize}[label=(\\alph*)]\n\\item a\n\\end{itemize}\n", "renamed with an optional argument")
    }

    // MARK: undo / redo of typed keystrokes

    func testUndoAndRedoRevertBothNamesTogether() async throws {
        try await load(Self.doc)
        editor.select(end(of: name("itemize", "begin")))
        try await type("ab")
        assertSynced("\\begin{itemizeab}\n\\item a\n\\end{itemizeab}\n", "typed ab")
        var states: [String] = []
        while undo.canUndo {
            undo.undo()
            let t = editor.text as NSString
            let b = LaTeXScan.uses(in: t).filter { $0.name == "begin" }.map(\.arg)
            let e = LaTeXScan.uses(in: t).filter { $0.name == "end" }.map(\.arg)
            XCTAssertEqual(b, e, "every undo step leaves the names matched: \(editor.text)")
            states.append(editor.text)
        }
        XCTAssertEqual(states.last, Self.doc, "undo walks back to the original")
        while undo.canRedo { undo.redo() }
        XCTAssertEqual(editor.text, "\\begin{itemizeab}\n\\item a\n\\end{itemizeab}\n", "redo reapplies both names")
    }

    // MARK: input methods

    func testComposedCharacterInTheNameSyncsOnCommit() async throws {
        try await load(Self.doc)
        editor.select(end(of: name("itemize", "begin")))
        tv.setMarkedText("´", selectedRange: NSRange(location: 1, length: 0))
        try await turn()
        XCTAssertNotNil(tv.markedTextRange)
        XCTAssertTrue(editor.text.hasSuffix("\\end{itemize}\n"), "the partner waits while marked text shows: \(editor.text)")
        tv.setMarkedText("é", selectedRange: NSRange(location: 1, length: 0))
        try await turn()
        tv.unmarkText()
        try await turn()
        if editor.text.contains("\\end{itemize}") {
            // Committing unchanged marked text is no text change; the next
            // keystroke in the name brings the partner along.
            try await type("x")
            assertSynced("\\begin{itemizeéx}\n\\item a\n\\end{itemizeéx}\n", "committed é, then typed")
        } else {
            assertSynced("\\begin{itemizeé}\n\\item a\n\\end{itemizeé}\n", "committed é in the begin name")
        }
    }

    func testComposedCharacterReplacedByInsertTextSyncs() async throws {
        try await load(Self.doc)
        editor.select(end(of: name("itemize", "end")))
        tv.setMarkedText("´", selectedRange: NSRange(location: 1, length: 0))
        try await turn()
        tv.insertText("é")
        try await turn()
        XCTAssertNil(tv.markedTextRange)
        assertSynced("\\begin{itemizeé}\n\\item a\n\\end{itemizeé}\n", "committed é in the end name")
    }

    // MARK: things that must not link

    func testTypingANewEnvironmentDoesNotTouchOtherEnds() async throws {
        try await load("\\begin{itemize}\n\n\\end{itemize}\n")
        editor.select(NSRange(location: 16, length: 0))
        try await type("\\begin{enu")
        XCTAssertTrue(editor.text.hasPrefix("\\begin{itemize}\n\\begin{enu"), editor.text)
        XCTAssertTrue(editor.text.hasSuffix("\\end{itemize}\n"), "the outer end is untouched: \(editor.text)")
    }

    func testVerbatimBodyAndUnbalancedNamesDoNotLink() async throws {
        try await load("\\begin{verbatim}\n\\begin{x}\n\\end{verbatim}\n\\begin{itemize}\n")
        editor.select(end(of: name("x", "begin")))
        try await type("y")
        XCTAssertEqual(editor.text, "\\begin{verbatim}\n\\begin{xy}\n\\end{verbatim}\n\\begin{itemize}\n")
        editor.select(end(of: name("itemize", "begin")))
        try await type("z")
        XCTAssertEqual(editor.text, "\\begin{verbatim}\n\\begin{xy}\n\\end{verbatim}\n\\begin{itemizez}\n", "an unclosed begin has no partner")
        editor.select(end(of: name("verbatim", "begin")))
        try await type("2")
        XCTAssertEqual(editor.text, "\\begin{verbatim2}\n\\begin{xy}\n\\end{verbatim2}\n\\begin{itemizez}\n",
                       "a verbatim environment's own names still link")
    }

    /// Typing on a line with no environment name costs no document scan: the
    /// O(line) gate refuses before `LinkedEnvironmentEditing.session`.
    func testTypingOutsideANameOpensNoSession() async throws {
        var text = ""
        for i in 0..<3000 { text += "\\begin{itemize}\n\\item \(i)\n\\end{itemize}\n" }
        try await load(text)
        editor.onChange = nil // the model's own copy of the text is not the gate's cost
        editor.select(NSRange(location: (text as NSString).length, length: 0))
        let start = DispatchTime.now().uptimeNanoseconds
        for _ in 0..<50 { editor.type("a") }
        let ms = Double(DispatchTime.now().uptimeNanoseconds - start) / 1_000_000 / 50
        print("ipad.linked.gate: \(ms) ms per keystroke outside a name on \((text as NSString).length) units")
        XCTAssertNil(editor.linkedSession)
        XCTAssertTrue(editor.text.hasSuffix("aaaa"))
    }
}
