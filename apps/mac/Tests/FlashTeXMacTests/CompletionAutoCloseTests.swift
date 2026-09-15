import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// GH#2: accepting a completion whose replacement supplies its own closing
/// delimiter, while the editor's own auto-inserted closer sits immediately
/// after the replaced token.
///
/// The owner's report: `\begin` completes to `\begin{|}` (the `}` is
/// auto-inserted and tracked in `pendingClosers`), `proof` is typed inside it,
/// and accepting the environment completion inserts `proof}\n\n\end{proof}` —
/// which carries the `}` for that same `{`. The tracked `}` was left behind,
/// stranded past the whole construct (`\end{proof}}`).
///
/// These tests drive the real path: the hosted `SourceEditorView` (so
/// `Coordinator` auto-closes and tracks closers) plus the real
/// `CompletingTextView` completion session, with the scan on a manual executor
/// so nothing depends on timing. `CompletionTests` covers ranking and the
/// bare text view; this file covers only the interaction between the two.
@MainActor
final class CompletionAutoCloseTests: XCTestCase {

    // MARK: harness

    private final class TextBox { var text = "" }

    private struct Host: View {
        let box: TextBox
        let pairs: Set<Character>
        var body: some View {
            SourceEditorView(text: Binding(get: { box.text }, set: { box.text = $0 }), autoClosePairs: pairs)
        }
    }

    /// Runs the completion scan when the test says so (delivery is a run-loop
    /// block, like `CompletionTests`' own executor).
    private final class ManualExecutor {
        var jobs: [@Sendable () -> Void] = []
        var run: CompletionScheduler.Executor { { [self] job in jobs.append(job) } }
        func runAll() { let j = jobs; jobs = []; j.forEach { $0() } }
    }

    private var windows: [NSWindow] = []

    override func tearDown() {
        windows.forEach { $0.orderOut(nil) }
        windows = []
        super.tearDown()
    }

    /// The real editor in a window, its coordinator, and a manual scan executor.
    private func editor(autoClosePairs: Set<Character> = ["{"])
        throws -> (tv: CompletingTextView, co: SourceEditorView.Coordinator, exec: ManualExecutor, box: TextBox) {
        let box = TextBox()
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(box: box, pairs: autoClosePairs))
        window.orderFrontRegardless() // never makeKey
        windows.append(window)
        var found: NSTextView?
        spin("editor text view") { found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil }
        let tv = try XCTUnwrap(found as? CompletingTextView)
        let co = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        XCTAssertTrue(window.makeFirstResponder(tv))
        tv.allowsUndo = true
        let exec = ManualExecutor()
        tv.scheduler = CompletionScheduler(executor: exec.run)
        return (tv, co, exec, box)
    }

    private func spin(_ what: String, timeout: TimeInterval = 10, _ cond: () -> Bool) {
        let deadline = Date().addingTimeInterval(timeout)
        while !cond(), Date() < deadline {
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.005))
        }
        XCTAssertTrue(cond(), "timed out waiting for \(what)")
    }

    /// Lets queued main-queue work (the binding push, undo grouping) run.
    private func turn() { RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.05)) }

    /// Types `s` one character at a time at the caret, through the path that
    /// auto-closes (`insertText`), never as a programmatic replacement.
    private func type(_ s: String, into tv: NSTextView) {
        for ch in s { tv.insertText(String(ch), replacementRange: tv.selectedRange()) }
    }

    /// Opens the list and accepts the first item `match` picks, as the user
    /// would with ⌃Space + Return.
    private func accept(_ match: (Completion.Suggestion) -> Bool, in tv: CompletingTextView, _ exec: ManualExecutor,
                        file: StaticString = #filePath, line: UInt = #line) {
        tv.flushAutomaticCompletion() // no-op unless typing armed the automatic open
        tv.requestCompletion()
        exec.runAll()
        spin("completion list") { tv.session != nil }
        guard let items = tv.session?.items, let index = items.firstIndex(where: match) else {
            XCTFail("no matching suggestion in \(tv.session?.items.map(\.label) ?? [])", file: file, line: line)
            return
        }
        tv.selectCompletion(at: index)
        tv.acceptSelectedCompletion()
        XCTAssertEqual(tv.lastCloseReason, .accepted, file: file, line: line)
    }

    private func environment(_ name: String) -> (Completion.Suggestion) -> Bool {
        { $0.kind == .environment && $0.label == name }
    }

    private func command(_ snippetText: String) -> (Completion.Suggestion) -> Bool {
        { $0.snippet?.text == snippetText }
    }

    /// The invariant `pendingClosers` exists to hold: every tracked offset is
    /// inside the buffer and holds a closing delimiter.
    private func assertClosersMatchTheText(_ co: SourceEditorView.Coordinator, _ tv: NSTextView,
                                           file: StaticString = #filePath, line: UInt = #line) {
        let ns = tv.string as NSString
        for offset in co.pendingClosers {
            guard offset >= 0, offset < ns.length else {
                XCTFail("pending closer \(offset) is outside a buffer of \(ns.length)", file: file, line: line); continue
            }
            let ch = ns.substring(with: NSRange(location: offset, length: 1))
            XCTAssertTrue(ch.first.map(SourceEditorView.BraceMatcher.isCloser) ?? false,
                          "pending closer \(offset) points at \(ch.debugDescription), not a closer", file: file, line: line)
        }
    }

    // MARK: the reported bug

    /// The owner's exact sequence: `\begin` → accept → `proof` → accept.
    func testAcceptingAnEnvironmentConsumesTheAutoClosedBraceItReplaces() throws {
        let (tv, co, exec, box) = try editor()
        let undo = try XCTUnwrap(tv.undoManager)

        type("\\begin", into: tv)
        accept(command("\\begin{}"), in: tv, exec)
        XCTAssertEqual(tv.string, "\\begin{}")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 7, length: 0), "caret between the braces")
        XCTAssertEqual(co.pendingClosers, [7], "the snippet's own `}` is tracked, like a hand-typed one")
        turn()

        type("proof", into: tv)
        XCTAssertEqual(tv.string, "\\begin{proof}")
        XCTAssertEqual(co.pendingClosers, [12], "shifted by the typing before it")
        undo.removeAllActions()

        accept(environment("proof"), in: tv, exec)
        // The whole point: one `}`, not two.
        XCTAssertEqual(tv.string, "\\begin{proof}\n\n\\end{proof}")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 14, length: 0), "caret on the empty middle line")
        XCTAssertEqual(co.pendingClosers, [], "the tracked closer was consumed by the replacement, not left dangling")
        assertClosersMatchTheText(co, tv)
        turn()
        XCTAssertEqual(box.text, tv.string, "the model got the same buffer")

        // One ⌘Z takes the accepted completion back, closer and all.
        XCTAssertEqual(undo.undoActionName, "Insert Environment")
        undo.undo()
        XCTAssertEqual(tv.string, "\\begin{proof}", "one undo restores exactly the pre-accept buffer")
        undo.redo()
        XCTAssertEqual(tv.string, "\\begin{proof}\n\n\\end{proof}")
    }

    /// `\end{` completes without a snippet (`insertCompletion`), and carries a
    /// `}` just the same.
    func testAcceptingAClosingEnvironmentConsumesItToo() throws {
        let (tv, co, exec, _) = try editor()
        type("\\begin", into: tv)
        accept(command("\\begin{}"), in: tv, exec)
        type("itemize", into: tv)
        accept(environment("itemize"), in: tv, exec)
        XCTAssertEqual(tv.string, "\\begin{itemize}\n\\item \n\\end{itemize}")
        XCTAssertEqual(co.pendingClosers, [])
        turn()

        // Now the same shape with the plain-text insertion path: `\end` opens
        // its own braces, and the environment name closes them itself.
        tv.setSelectedRange(NSRange(location: (tv.string as NSString).length, length: 0))
        type("\n\\end", into: tv)
        accept(command("\\end{}"), in: tv, exec)
        let closer = try XCTUnwrap(co.pendingClosers.last)
        type("item", into: tv)
        XCTAssertEqual(co.pendingClosers, [closer + 4])
        accept({ $0.kind == .environment && $0.label == "itemize" && $0.snippet == nil }, in: tv, exec)
        XCTAssertEqual(tv.string, "\\begin{itemize}\n\\item \n\\end{itemize}\n\\end{itemize}")
        XCTAssertEqual(co.pendingClosers, [])
        assertClosersMatchTheText(co, tv)
    }

    // MARK: neighbours that must not change

    /// A balanced snippet (`\frac{}{}`) closes nothing that was opened before
    /// it, so the tracked closer around it survives — including nested.
    func testABalancedSnippetLeavesTheSurroundingCloserAlone() throws {
        let (tv, co, exec, _) = try editor()
        type("\\fr", into: tv)
        accept(command("\\frac{}{}"), in: tv, exec)
        XCTAssertEqual(tv.string, "\\frac{}{}")
        XCTAssertEqual(co.pendingClosers, [6])
        turn()

        type("\\fr", into: tv)
        XCTAssertEqual(tv.string, "\\frac{\\fr}{}")
        accept(command("\\frac{}{}"), in: tv, exec)
        XCTAssertEqual(tv.string, "\\frac{\\frac{}{}}{}", "nothing eaten: the inner snippet balances itself")
        XCTAssertEqual(co.pendingClosers.sorted(), [12, 15])
        assertClosersMatchTheText(co, tv)
    }

    /// `\section{}` still tracks its own `}` for overtype: typing `}` steps
    /// over it instead of doubling it (GH74 behaviour, unchanged).
    func testATypedCloserStillOvertypesASnippetCloser() throws {
        let (tv, co, exec, _) = try editor()
        type("x \\se", into: tv)
        accept(command("\\section{}"), in: tv, exec)
        XCTAssertEqual(tv.string, "x \\section{}")
        XCTAssertEqual(co.pendingClosers, [11])
        turn()
        tv.insertText("}", replacementRange: tv.selectedRange())
        XCTAssertEqual(tv.string, "x \\section{}", "typed over, not duplicated")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 12, length: 0))
        XCTAssertEqual(co.pendingClosers, [])
        // A second `}` is a real character.
        tv.insertText("}", replacementRange: tv.selectedRange())
        XCTAssertEqual(tv.string, "x \\section{}}")
    }

    /// A brace the editor did not insert is never eaten, even where eating one
    /// would look tidier: `pendingClosers` is the whole authority.
    func testAHandTypedCloserIsNeverConsumed() throws {
        let (tv, co, exec, _) = try editor(autoClosePairs: [])
        type("\\begin{proof}", into: tv)
        XCTAssertEqual(tv.string, "\\begin{proof}", "auto-close off: the `}` is the user's")
        XCTAssertEqual(co.pendingClosers, [], "nothing tracked")
        tv.setSelectedRange(NSRange(location: 12, length: 0))
        accept(environment("proof"), in: tv, exec)
        XCTAssertEqual(tv.string, "\\begin{proof}\n\n\\end{proof}}", "the user's own brace is left exactly where they put it")
        assertClosersMatchTheText(co, tv)
    }

    /// With auto-close off the completion snippets still open and track their
    /// own braces, so the bug (and the fix) are independent of the preference.
    func testTheFlowsBehaveTheSameWithAutoCloseOff() throws {
        let (tv, co, exec, _) = try editor(autoClosePairs: [])
        tv.insertText("{", replacementRange: NSRange(location: 0, length: 0))
        XCTAssertEqual(tv.string, "{", "auto-close is off for hand-typed openers")
        XCTAssertEqual(co.pendingClosers, [])
        tv.setSelectedRange(NSRange(location: 0, length: 1))
        tv.insertText("", replacementRange: tv.selectedRange())
        turn()

        type("\\begin", into: tv)
        accept(command("\\begin{}"), in: tv, exec)
        XCTAssertEqual(tv.string, "\\begin{}", "the snippet brings its own pair either way")
        XCTAssertEqual(co.pendingClosers, [7])
        type("proof", into: tv)
        accept(environment("proof"), in: tv, exec)
        XCTAssertEqual(tv.string, "\\begin{proof}\n\n\\end{proof}")
        XCTAssertEqual(co.pendingClosers, [])
        assertClosersMatchTheText(co, tv)
    }

    /// The multi-line templates with placeholders (`itemize`, `figure`) go the
    /// same way, and their Tab stops still line up with the text.
    func testMultiLineTemplatesWithPlaceholdersKeepTheirBraces() throws {
        for name in ["itemize", "figure"] {
            let (tv, co, exec, _) = try editor()
            type("\\begin", into: tv)
            accept(command("\\begin{}"), in: tv, exec)
            type(String(name.prefix(3)), into: tv)
            accept(environment(name), in: tv, exec)
            let expected = "\\begin{" + Completion.environmentSnippet(name, indent: "").text
            XCTAssertEqual(tv.string, expected, "\(name): exactly the template, no stray closer")
            XCTAssertFalse(tv.string.hasSuffix("}}"), "\(name): no doubled brace at the end")
            // `figure` parks the caret inside `\includegraphics{}`, so that
            // closer is tracked for overtype; `itemize` parks it after `\item `,
            // where nothing is. Either way: exactly the closer at the caret.
            let caret = tv.selectedRange().location
            let ns = tv.string as NSString
            let atCaret = caret < ns.length ? ns.substring(with: NSRange(location: caret, length: 1)).first : nil
            let tracked = (atCaret.map(SourceEditorView.BraceMatcher.isCloser) ?? false) ? [caret] : []
            XCTAssertEqual(co.pendingClosers, tracked, "\(name): only a closer the caret sits before is tracked")
            assertClosersMatchTheText(co, tv)
            // Tab reaches every placeholder and ends inside the buffer.
            let stops = Completion.environmentSnippet(name, indent: "").stops.map { $0 + 7 }
            XCTAssertTrue(stops.allSatisfy { $0 <= (tv.string as NSString).length }, "\(name): stops fit the buffer")
            turn()
        }
    }
}
