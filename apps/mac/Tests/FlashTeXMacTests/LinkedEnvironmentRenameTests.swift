import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// Owner report (ENV-RENAME-SYNC): editing the name in `\begin{…}` sometimes
/// fails to update `\end{…}`. Every scenario drives a hosted
/// `SourceEditorView` / `CompletingTextView` with real key events, so the
/// completion popup, auto-close and the delegate path all take part.
@MainActor
final class LinkedEnvironmentRenameTests: XCTestCase {
    private var window: NSWindow?

    override func tearDown() async throws {
        window?.orderOut(nil)
        window = nil
    }

    struct Host: View {
        var model: ShellModel
        var body: some View {
            SourceEditorView(text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                             selection: model.selection, pendingEdit: model.pendingEdit,
                             onCaretChange: { model.caretUTF16 = $0 },
                             onSelectionChange: { model.caretLengthUTF16 = $0.length },
                             onEditApplied: { model.editApplied($0, newText: $1) })
        }
    }

    private func host(_ text: String) async throws -> (ShellModel, CompletingTextView, SourceEditorView.Coordinator) {
        let model = ShellModel()
        model.updateActiveText(text)
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model))
        window.orderFrontRegardless()
        self.window = window
        var found: NSTextView?
        try await waitUntil("editor text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        let tv = try XCTUnwrap(found as? CompletingTextView)
        let co = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        XCTAssertTrue(window.makeFirstResponder(tv))
        try await turn()
        tv.undoManager?.removeAllActions()
        return (model, tv, co)
    }

    private func turn(_ ms: UInt64 = 40) async throws { try await Task.sleep(nanoseconds: ms * 1_000_000) }

    private func waitUntil(_ what: String, timeout: TimeInterval = 5, file: StaticString = #filePath, line: UInt = #line,
                           _ cond: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !cond() {
            if Date() > deadline { XCTFail("timed out: \(what)", file: file, line: line); return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    private func event(_ chars: String, keyCode: UInt16 = 0, modifiers: NSEvent.ModifierFlags = [], window: NSWindow?) -> NSEvent {
        NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: modifiers,
                         timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window?.windowNumber ?? 0,
                         context: nil, characters: chars, charactersIgnoringModifiers: chars, isARepeat: false, keyCode: keyCode)!
    }

    /// Types `chars` one key event at a time. With `popup`, the automatic
    /// completion open fires after each key and the list is awaited, so every
    /// keystroke after the first lands while the list is showing.
    private func type(_ tv: CompletingTextView, _ chars: String, popup: Bool = false) async throws {
        for ch in chars {
            tv.keyDown(with: event(String(ch), window: tv.window))
            if popup {
                tv.flushAutomaticCompletion()
                try await waitUntil("the completion list is showing after \(ch)") { tv.isCompletionActive }
            }
            try await turn(10)
        }
    }

    private func backspace(_ tv: CompletingTextView, times: Int = 1) async throws {
        for _ in 0..<times {
            tv.keyDown(with: event("\u{7F}", keyCode: 51, window: tv.window))
            try await turn(10)
        }
    }

    private func pressReturn(_ tv: CompletingTextView) { tv.keyDown(with: event("\r", keyCode: 36, window: tv.window)) }
    private func pressEscape(_ tv: CompletingTextView) { tv.keyDown(with: event("\u{1B}", keyCode: 53, window: tv.window)) }

    private func nameRange(_ text: String, _ token: String, occurrence: Int) -> NSRange {
        let ns = text as NSString
        var from = 0
        var r = NSRange(location: NSNotFound, length: 0)
        for _ in 0...occurrence {
            r = ns.range(of: token, range: NSRange(location: from, length: ns.length - from))
            from = NSMaxRange(r)
        }
        return r
    }

    private func range(ofName name: String, which: String, in text: String) -> NSRange {
        let r = (text as NSString).range(of: "\\\(which){\(name)}")
        return NSRange(location: r.location + which.utf16.count + 2, length: name.utf16.count)
    }

    private func assertSynced(_ tv: NSTextView, _ model: ShellModel, _ expected: String, _ label: String,
                              file: StaticString = #filePath, line: UInt = #line) async throws {
        try await turn()
        XCTAssertEqual(tv.string, expected, label, file: file, line: line)
        XCTAssertEqual(model.activeText, tv.string, "\(label): the model sees the buffer", file: file, line: line)
    }

    private static let doc = "\\begin{itemize}\n\\item a\n\\end{itemize}\n"

    // MARK: typing while the completion list is open

    func testTypingIntoBeginNameWhileTheListIsOpenUpdatesEnd() async throws {
        let (model, tv, _) = try await host(Self.doc)
        tv.setSelectedRange(range(ofName: "itemize", which: "begin", in: Self.doc))
        try await type(tv, "enu", popup: true)
        try await assertSynced(tv, model, "\\begin{enu}\n\\item a\n\\end{enu}\n", "typed over the begin name with the list open")
        pressEscape(tv)
    }

    func testTypingIntoEndNameWhileTheListIsOpenUpdatesBegin() async throws {
        let (model, tv, _) = try await host(Self.doc)
        tv.setSelectedRange(range(ofName: "itemize", which: "end", in: Self.doc))
        try await type(tv, "enu", popup: true)
        try await assertSynced(tv, model, "\\begin{enu}\n\\item a\n\\end{enu}\n", "typed over the end name with the list open")
        let caret = tv.selectedRange()
        XCTAssertEqual(caret, NSRange(location: NSMaxRange(range(ofName: "enu", which: "end", in: tv.string)), length: 0),
                       "the caret stays after the typed letters when the begin name before it changes length")
        pressEscape(tv)
    }

    func testAppendingToBeginNameWithTheListOpenKeepsPairing() async throws {
        let (model, tv, _) = try await host(Self.doc)
        tv.setSelectedRange(NSRange(location: NSMaxRange(range(ofName: "itemize", which: "begin", in: Self.doc)), length: 0))
        try await type(tv, "*", popup: false)
        try await type(tv, "x", popup: false)
        try await assertSynced(tv, model, "\\begin{itemize*x}\n\\item a\n\\end{itemize*x}\n", "appended to the begin name")
    }

    // MARK: accepting a completion

    func testAcceptingACompletionInBeginNameRenamesBothWithoutASkeleton() async throws {
        let (model, tv, _) = try await host(Self.doc)
        tv.setSelectedRange(range(ofName: "itemize", which: "begin", in: Self.doc))
        try await type(tv, "enumer", popup: true)
        try await waitUntil("enumerate is offered") { tv.isCompletionActive }
        pressReturn(tv)
        try await assertSynced(tv, model, "\\begin{enumerate}\n\\item a\n\\end{enumerate}\n", "accepted enumerate in the begin name")
        XCTAssertFalse(tv.isCompletionActive)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: NSMaxRange(range(ofName: "enumerate", which: "begin", in: tv.string)), length: 0))
        tv.undoManager?.undo()
        try await assertSynced(tv, model, "\\begin{enumer}\n\\item a\n\\end{enumer}\n", "one ⌘Z undoes the accepted completion in both names")
    }

    func testAcceptingACompletionInEndNameRenamesBoth() async throws {
        let (model, tv, _) = try await host(Self.doc)
        tv.setSelectedRange(range(ofName: "itemize", which: "end", in: Self.doc))
        try await type(tv, "enumer", popup: true)
        pressReturn(tv)
        try await assertSynced(tv, model, "\\begin{enumerate}\n\\item a\n\\end{enumerate}\n", "accepted enumerate in the end name")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: NSMaxRange(range(ofName: "enumerate", which: "end", in: tv.string)), length: 0))
        tv.undoManager?.undo()
        try await assertSynced(tv, model, "\\begin{enumer}\n\\item a\n\\end{enumer}\n", "one ⌘Z undoes the accepted completion in both names")
    }

    /// A caret in the middle of the name: the accepted name replaces the
    /// whole name span, not only the part before the caret.
    func testAcceptingACompletionMidNameReplacesTheWholeName() async throws {
        let text = "\\begin{figure}\n\\centering\n\\end{figure}\n"
        let (model, tv, _) = try await host(text)
        let name = range(ofName: "figure", which: "begin", in: text)
        tv.setSelectedRange(NSRange(location: name.location + 3, length: 3)) // select "ure"
        try await backspace(tv)
        try await type(tv, "*", popup: false) // fig* : not a name the list completes
        try await backspace(tv)
        XCTAssertEqual(tv.string, "\\begin{fig}\n\\centering\n\\end{fig}\n")
        tv.setSelectedRange(NSRange(location: name.location + 2, length: 0)) // fi|g
        tv.requestCompletion()
        try await waitUntil("the list opens mid-name") { tv.isCompletionActive }
        if let i = tv.session?.items.firstIndex(where: { $0.label == "figure*" }) { tv.selectCompletion(at: i) }
        let chosen = try XCTUnwrap(tv.session?.selected?.label)
        pressReturn(tv)
        try await assertSynced(tv, model, "\\begin{\(chosen)}\n\\centering\n\\end{\(chosen)}\n", "accepted \(chosen) mid-name")
    }

    // MARK: deletion, emptied names, replacement

    func testBackspacingTheWholeNameThenTypingANewOneKeepsThePair() async throws {
        let (model, tv, _) = try await host(Self.doc)
        tv.setSelectedRange(NSRange(location: NSMaxRange(range(ofName: "itemize", which: "begin", in: Self.doc)), length: 0))
        try await backspace(tv, times: 7)
        try await assertSynced(tv, model, "\\begin{}\n\\item a\n\\end{}\n", "emptied the begin name")
        try await type(tv, "enumerate")
        pressEscape(tv)
        try await assertSynced(tv, model, "\\begin{enumerate}\n\\item a\n\\end{enumerate}\n", "typed a new name into the emptied begin name")
    }

    func testBackspacingTheWholeEndNameThenTypingANewOneKeepsThePair() async throws {
        let (model, tv, _) = try await host(Self.doc)
        tv.setSelectedRange(NSRange(location: NSMaxRange(range(ofName: "itemize", which: "end", in: Self.doc)), length: 0))
        try await backspace(tv, times: 7)
        try await type(tv, "enumerate")
        pressEscape(tv)
        try await assertSynced(tv, model, "\\begin{enumerate}\n\\item a\n\\end{enumerate}\n", "retyped the end name")
    }

    func testSelectingTheNameAndDeletingEmptiesBoth() async throws {
        let (model, tv, _) = try await host(Self.doc)
        tv.setSelectedRange(range(ofName: "itemize", which: "begin", in: Self.doc))
        try await backspace(tv)
        try await assertSynced(tv, model, "\\begin{}\n\\item a\n\\end{}\n", "deleted the selected begin name")
        tv.undoManager?.undo()
        try await assertSynced(tv, model, Self.doc, "⌘Z restores both names")
    }

    /// A multi-character replacement (paste over the selected name) with
    /// the coordinator's `lastKnownText` out of step with the view (a
    /// programmatic change it did not see): the partner must still follow.
    func testPasteOverTheNameWithAStaleLastKnownTextStillSyncs() async throws {
        let (model, tv, co) = try await host(Self.doc)
        co.lastKnownText = "stale"
        let name = range(ofName: "itemize", which: "begin", in: Self.doc)
        tv.setSelectedRange(name)
        tv.insertText("description", replacementRange: name)
        try await assertSynced(tv, model, "\\begin{description}\n\\item a\n\\end{description}\n", "pasted over the begin name")
        tv.undoManager?.undo()
        try await assertSynced(tv, model, Self.doc, "one ⌘Z undoes the paste in both names")
        tv.undoManager?.redo()
        try await assertSynced(tv, model, "\\begin{description}\n\\item a\n\\end{description}\n", "⌘⇧Z redoes both")
    }

    // MARK: structure

    func testNestedSameNameLinksTheMatchingEnd() async throws {
        let text = "\\begin{itemize}\n\\begin{itemize}\n\\item a\n\\end{itemize}\n\\end{itemize}\n"
        let (model, tv, _) = try await host(text)
        let inner = nameRange(text, "itemize", occurrence: 1)
        tv.setSelectedRange(inner)
        try await type(tv, "enu", popup: true)
        pressEscape(tv)
        try await assertSynced(tv, model, "\\begin{itemize}\n\\begin{enu}\n\\item a\n\\end{enu}\n\\end{itemize}\n", "renamed the inner begin")
        let outerEnd = range(ofName: "itemize", which: "end", in: tv.string)
        tv.setSelectedRange(NSRange(location: NSMaxRange(outerEnd), length: 0))
        try await type(tv, "x")
        pressEscape(tv)
        try await assertSynced(tv, model, "\\begin{itemizex}\n\\begin{enu}\n\\item a\n\\end{enu}\n\\end{itemizex}\n", "appended to the outer end")
    }

    func testBeginWithOptionalArgumentSyncs() async throws {
        let text = "\\begin{enumerate}[label=(\\alph*)]\n\\item a\n\\end{enumerate}\n"
        let (model, tv, _) = try await host(text)
        tv.setSelectedRange(range(ofName: "enumerate", which: "begin", in: text))
        try await type(tv, "itemi", popup: true)
        pressReturn(tv)
        try await assertSynced(tv, model, "\\begin{itemize}[label=(\\alph*)]\n\\item a\n\\end{itemize}\n", "renamed with an optional argument")
    }

    // MARK: undo / redo of typed keystrokes

    func testEachTypedKeystrokeUndoesInBothNames() async throws {
        let (model, tv, _) = try await host(Self.doc)
        tv.setSelectedRange(NSRange(location: NSMaxRange(range(ofName: "itemize", which: "begin", in: Self.doc)), length: 0))
        for ch in "ab" {
            tv.breakUndoCoalescing()
            try await type(tv, String(ch), popup: true)
        }
        pressEscape(tv)
        try await assertSynced(tv, model, "\\begin{itemizeab}\n\\item a\n\\end{itemizeab}\n", "typed ab")
        tv.undoManager?.undo()
        try await assertSynced(tv, model, "\\begin{itemizea}\n\\item a\n\\end{itemizea}\n", "⌘Z one keystroke")
        tv.undoManager?.undo()
        try await assertSynced(tv, model, Self.doc, "⌘Z the other")
        tv.undoManager?.redo()
        tv.undoManager?.redo()
        try await assertSynced(tv, model, "\\begin{itemizeab}\n\\item a\n\\end{itemizeab}\n", "⌘⇧Z both")
    }

    // MARK: input methods

    /// A dead key / IME composition inside the name: the partner follows
    /// once the composition commits.
    func testComposedCharacterInTheNameSyncsOnCommit() async throws {
        let (model, tv, _) = try await host(Self.doc)
        let at = NSMaxRange(range(ofName: "itemize", which: "begin", in: Self.doc))
        tv.setSelectedRange(NSRange(location: at, length: 0))
        tv.setMarkedText("´", selectedRange: NSRange(location: 1, length: 0), replacementRange: NSRange(location: at, length: 0))
        try await turn()
        XCTAssertTrue(tv.hasMarkedText())
        tv.insertText("é", replacementRange: tv.markedRange())
        try await assertSynced(tv, model, "\\begin{itemizeé}\n\\item a\n\\end{itemizeé}\n", "committed é in the begin name")
    }

    // MARK: things that must not link

    func testTypingANewEnvironmentDoesNotTouchOtherEnds() async throws {
        let text = "\\begin{itemize}\n\n\\end{itemize}\n"
        let (model, tv, _) = try await host(text)
        tv.setSelectedRange(NSRange(location: 16, length: 0))
        try await type(tv, "\\begin{")
        try await type(tv, "enu")
        pressEscape(tv)
        try await turn()
        XCTAssertTrue(tv.string.hasPrefix("\\begin{itemize}\n\\begin{enu"), tv.string)
        XCTAssertTrue(tv.string.hasSuffix("\\end{itemize}\n"), "the outer end is untouched: \(tv.string)")
        XCTAssertEqual(model.activeText, tv.string)
    }

    /// A nested `\begin{itemize` typed out in full pairs (by name) with the
    /// outer `\end{itemize}`; its `}` is the one the editor just auto-closed,
    /// so accepting still opens a new environment with its skeleton.
    func testAcceptingANewNestedEnvironmentStillInsertsItsSkeleton() async throws {
        let text = "\\begin{itemize}\n\n\\end{itemize}\n"
        let (model, tv, _) = try await host(text)
        tv.setSelectedRange(NSRange(location: 16, length: 0))
        try await type(tv, "\\begin{")
        XCTAssertEqual(tv.string, "\\begin{itemize}\n\\begin{}\n\\end{itemize}\n", "the brace auto-closes")
        try await type(tv, "itemize", popup: true)
        pressReturn(tv)
        try await turn()
        let ns = tv.string as NSString
        XCTAssertEqual(ns.components(separatedBy: "\\end{itemize}").count - 1, 2, "a second \\end{itemize}: \(tv.string)")
        XCTAssertTrue(tv.string.hasPrefix("\\begin{itemize}\n\\begin{itemize}\n"), tv.string)
        XCTAssertFalse(tv.string.contains("}}"), tv.string)
        XCTAssertEqual(model.activeText, tv.string)
    }
}
