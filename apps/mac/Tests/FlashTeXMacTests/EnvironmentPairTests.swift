import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac
@testable import FlashTeXAccessibility

/// Environment pair matching, select and wrap (lane mac-editor-dx-3,
/// EditorNavigation.swift): pure cases, the hosted editor's pair highlight
/// and the model commands.
@MainActor
final class EnvironmentPairTests: XCTestCase {
    typealias EN = EditorNavigation

    func testPairsTolerateNestingUnbalancedAndComments() {
        let s = "\\begin{a}\n\\begin{a}\n% \\end{a} commented\nx\n\\end{a}\n\\end{b}\n\\begin{c}\nnever closed" as NSString
        let pairs = EN.environmentPairs(in: s)
        XCTAssertEqual(pairs.map(\.name), ["a", "a", "c"])
        // Same-name nesting: the inner \end closes the inner \begin.
        XCTAssertEqual(pairs[1].end, NSRange(location: 42, length: 7))
        XCTAssertNil(pairs[0].end, "the outer \\begin{a} is unclosed (the commented \\end does not count)")
        XCTAssertNil(pairs[2].end)
        // A stray \end{b} is ignored; the caret on it matches nothing.
        XCTAssertNil(EN.environmentPair(at: 53, in: s))
        // The caret anywhere on the inner \begin (or right after its brace) finds the pair.
        XCTAssertEqual(EN.environmentPair(at: 10, in: s)?.begin, NSRange(location: 10, length: 9))
        XCTAssertEqual(EN.environmentPair(at: 19, in: s)?.end, NSRange(location: 42, length: 7))
        XCTAssertEqual(EN.environmentPair(at: 46, in: s)?.begin, NSRange(location: 10, length: 9))
    }

    func testVerbatimBodiesAreSkipped() {
        let s = "\\begin{verbatim}\n\\begin{itemize}\n\\end{verbatim}\n\\begin{itemize}\n\\item x\n\\end{itemize}" as NSString
        let pairs = EN.environmentPairs(in: s)
        XCTAssertEqual(pairs.map(\.name), ["verbatim", "itemize"])
        XCTAssertEqual(pairs[0].end, NSRange(location: 33, length: 14))
        XCTAssertEqual(pairs[1].begin.location, 48)
        XCTAssertNotNil(pairs[1].end)
    }

    func testSelectEnvironmentWidensOutwards() {
        let s = "\\begin{document}\n\\begin{itemize}\n\\item a\n\\end{itemize}\nafter\n\\end{document}" as NSString
        let inner = EN.selectEnvironment(around: NSRange(location: 36, length: 0), in: s)
        XCTAssertEqual(inner?.name, "itemize")
        let innerWhole = inner!.whole(limit: s.length)
        XCTAssertEqual(innerWhole, NSRange(location: 17, length: 37))
        let outer = EN.selectEnvironment(around: innerWhole, in: s)
        XCTAssertEqual(outer?.name, "document")
        XCTAssertNil(EN.selectEnvironment(around: outer!.whole(limit: s.length), in: s), "the outermost has no parent")
        XCTAssertNil(EN.selectEnvironment(around: NSRange(location: 0, length: 0), in: "plain $x$ text" as NSString))
        // An unclosed environment extends to the end of the text.
        let open = "\\begin{proof}\nstuck" as NSString
        XCTAssertEqual(EN.selectEnvironment(around: NSRange(location: 16, length: 0), in: open)?.whole(limit: open.length), NSRange(location: 0, length: open.length))
    }

    func testWrapInlineBlockAndEmptySelection() {
        // Inline: a selection inside a line.
        let s = "see x + y here" as NSString
        let inline = EN.wrap(selection: NSRange(location: 4, length: 5), in: s, environment: "math", indentUnit: "  ")
        XCTAssertEqual(inline.replacement, "\\begin{math}x + y\\end{math}")
        XCTAssertEqual(inline.range, NSRange(location: 4, length: 5))
        XCTAssertEqual(inline.selection, NSRange(location: 4 + 12, length: 0))
        // Block: whole lines, indented one unit, blank lines kept blank, the caret at the first body line.
        let t = "  \\item a\n\n  \\item b\ntail" as NSString
        let block = EN.wrap(selection: NSRange(location: 2, length: 18), in: t, environment: "itemize", indentUnit: "  ")
        XCTAssertEqual(block.range, NSRange(location: 0, length: 20))
        XCTAssertEqual(block.replacement, "  \\begin{itemize}\n    \\item a\n\n    \\item b\n  \\end{itemize}")
        XCTAssertEqual(block.selection, NSRange(location: "  \\begin{itemize}\n    ".utf16.count, length: 0))
        // A selection that ends after the newline of the last line excludes the next line.
        let block2 = EN.wrap(selection: NSRange(location: 0, length: 21), in: t, environment: "itemize", indentUnit: "\t")
        XCTAssertEqual(block2.range, NSRange(location: 0, length: 20))
        XCTAssertTrue(block2.replacement.hasSuffix("\t  \\item b\n  \\end{itemize}"))
        // Empty selection on a blank line: an empty indented body line with the caret on it.
        let u = "a\n\nb" as NSString
        let empty = EN.wrap(selection: NSRange(location: 2, length: 0), in: u, environment: "center", indentUnit: "  ")
        XCTAssertEqual(empty.range, NSRange(location: 2, length: 0))
        XCTAssertEqual(empty.replacement, "\\begin{center}\n  \n\\end{center}")
        XCTAssertEqual(empty.selection, NSRange(location: 2 + "\\begin{center}\n  ".utf16.count, length: 0))
        // Verbatim-like environments never indent the body.
        let v = EN.wrap(selection: NSRange(location: 0, length: 1), in: "x" as NSString, environment: "verbatim", indentUnit: "  ")
        XCTAssertEqual(v.replacement, "\\begin{verbatim}\nx\n\\end{verbatim}")
    }

    // MARK: model commands

    func testSelectEnvironmentCommandSelectsAndWidens() {
        let model = ShellModel()
        model.updateActiveText("\\begin{document}\n\\begin{itemize}\n\\item a\n\\end{itemize}\n\\end{document}")
        model.caretUTF16 = 36
        model.selectEnvironment()
        XCTAssertEqual(model.selection?.nsRange, NSRange(location: 17, length: 37))
        XCTAssertEqual(model.caretLengthUTF16, 37)
        XCTAssertEqual(model.navigationNote, "Selected \\begin{itemize}…\\end{itemize} (lines 2–4).")
        model.selectEnvironment()
        XCTAssertEqual(model.selection?.nsRange, NSRange(location: 0, length: 69))
        model.selectEnvironment()
        XCTAssertEqual(model.navigationNote, "The selection is the outermost environment.")
        model.caretUTF16 = 0; model.caretLengthUTF16 = 0
        model.updateActiveText("plain")
        model.selectEnvironment()
        XCTAssertEqual(model.navigationNote, "Caret is not inside an environment.")
    }

    func testWrapCommandPostsOnePendingEditAndCaret() {
        let model = ShellModel()
        model.updateActiveText("a b c")
        model.caretUTF16 = 2; model.caretLengthUTF16 = 1
        model.editorNavigation.wrapShown = true
        model.wrapSelection(inEnvironment: "center")
        XCTAssertFalse(model.editorNavigation.wrapShown)
        let edit = try! XCTUnwrap(model.pendingEdit)
        XCTAssertEqual(edit.nsRange, NSRange(location: 2, length: 1))
        XCTAssertEqual(edit.text, "\\begin{center}b\\end{center}")
        XCTAssertEqual(edit.revision, model.editorRevision)
        XCTAssertEqual(model.selection?.nsRange, NSRange(location: 2 + 14, length: 0))
        // A bad name is refused without an edit.
        let before = model.pendingEdit
        model.wrapSelection(inEnvironment: "no way")
        XCTAssertEqual(model.pendingEdit, before)
        XCTAssertEqual(model.navigationNote, "“no way” is not an environment name.")
    }

    // MARK: hosted editor

    final class Probe { var coordinator: SourceEditorView.Coordinator? }

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

    private func host(_ model: ShellModel) async throws -> (NSWindow, NSTextView, SourceEditorView.Coordinator) {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model))
        window.orderFrontRegardless()
        var found: NSTextView?
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline, found == nil {
            found = TypingBenchDriver.findTextView(in: [window.contentView!])
            if found == nil { try await Task.sleep(nanoseconds: 10_000_000) }
        }
        let tv = try XCTUnwrap(found)
        let co = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        XCTAssertTrue(window.makeFirstResponder(tv))
        try await Task.sleep(nanoseconds: 50_000_000)
        return (window, tv, co)
    }

    func testCaretOnBeginHighlightsBothEndsLikeABracket() async throws {
        let model = ShellModel()
        let text = "\\begin{itemize}\n\\item {x}\n\\end{itemize}\n"
        model.updateActiveText(text)
        let (window, tv, co) = try await host(model)
        defer { window.orderOut(nil) }
        tv.setSelectedRange(NSRange(location: 3, length: 0)) // inside \begin
        try await Task.sleep(nanoseconds: 30_000_000)
        XCTAssertEqual(co.braceHighlight, .init(open: NSRange(location: 0, length: 15), close: NSRange(location: 26, length: 13)))
        XCTAssertNotNil(tv.layoutManager?.temporaryAttribute(.backgroundColor, atCharacterIndex: 27, effectiveRange: nil))
        // On the \end: the same pair; the VoiceOver suffix names the partner.
        tv.setSelectedRange(NSRange(location: 28, length: 0))
        try await Task.sleep(nanoseconds: 30_000_000)
        XCTAssertEqual(co.braceHighlight?.open, NSRange(location: 0, length: 15))
        XCTAssertEqual(co.matchSuffix(in: tv), ", matches line 1 column 1")
        // A brace pair on the line still wins; plain text clears the highlight.
        tv.setSelectedRange(NSRange(location: 23, length: 0)) // after `{x`… before `}`
        try await Task.sleep(nanoseconds: 30_000_000)
        XCTAssertEqual(co.braceHighlight?.open, NSRange(location: 22, length: 1))
        tv.setSelectedRange(NSRange(location: 18, length: 0))
        try await Task.sleep(nanoseconds: 30_000_000)
        XCTAssertNil(co.braceHighlight)
        XCTAssertNil(tv.layoutManager?.temporaryAttribute(.backgroundColor, atCharacterIndex: 27, effectiveRange: nil))
    }

    func testWrapThroughTheEditorIsOneUndoStepWithTheCaretAtTheBody() async throws {
        let model = ShellModel()
        model.updateActiveText("one\ntwo\n")
        let (window, tv, _) = try await host(model)
        defer { window.orderOut(nil) }
        tv.setSelectedRange(NSRange(location: 4, length: 3)) // "two"
        try await Task.sleep(nanoseconds: 30_000_000)
        model.wrapSelection(inEnvironment: "center")
        let deadline = Date().addingTimeInterval(3)
        while Date() < deadline, model.pendingEdit != nil { try await Task.sleep(nanoseconds: 20_000_000) }
        XCTAssertEqual(tv.string, "one\n\\begin{center}\n\(EditorPreferences.shared.indentString)two\n\\end{center}\n")
        XCTAssertEqual(model.activeText, tv.string)
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 4 + "\\begin{center}\n".utf16.count + EditorPreferences.shared.indentString.utf16.count, length: 0))
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "one\ntwo\n")
    }

    // MARK: command table

    func testCommandsAreInTheTableAndThePalette() {
        for c in [AccessibilityCommand.selectEnvironment, .wrapInEnvironment, .changeEnvironment, .goToDefinition, .goToSymbol, .renameSymbol] {
            XCTAssertTrue(CommandPaletteModel.isRunnable(c), "\(c)")
            XCTAssertNotNil(c.entry.menuItem)
        }
        XCTAssertEqual(AccessibilityCommand.selectEnvironment.entry.menu, "Navigate")
        XCTAssertEqual(AccessibilityCommand.wrapInEnvironment.entry.menu, "Navigate")
        XCTAssertEqual(AccessibilityCommand.changeEnvironment.entry.menu, "Editor")
        XCTAssertEqual(AccessibilityCommand.selectEnvironment.entry.shortcuts, ["⌘⇧A"])
        XCTAssertEqual(AccessibilityCommand.wrapInEnvironment.entry.shortcuts, ["⌘⇧W"])
        XCTAssertEqual(AccessibilityCommand.changeEnvironment.entry.shortcuts, ["⌃⌘E"])
        XCTAssertEqual(AccessibilityCommand.renameSymbol.entry.shortcuts, ["⌥⇧R"])
        XCTAssertEqual(CommandPaletteModel.rows(matching: "wrap").first?.id, .wrapInEnvironment)
        XCTAssertEqual(CommandPaletteModel.rows(matching: "change environment").first?.id, .changeEnvironment)
        XCTAssertEqual(CommandPaletteModel.rows(matching: "rename symbol").first?.id, .renameSymbol)
    }
}
