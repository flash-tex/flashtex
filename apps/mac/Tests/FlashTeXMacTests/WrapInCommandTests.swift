import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac
@testable import FlashTeXAccessibility

/// Wrap the selection in a command (⌘⇧B / ⌘I / ⌘U / ⌘⌥W,
/// EditorNavigation.swift + ShellModel+EditorNavigation.swift): the pure
/// wrap, the model commands (text and math mode), and the hosted editor's
/// one-undo-step edit with the placeholder `}` typed over.
@MainActor
final class WrapInCommandTests: XCTestCase {
    typealias EN = EditorNavigation

    // MARK: pure

    func testWrapAroundASelectionPutsTheCaretAfterTheBrace() {
        let s = "see foo here" as NSString
        let w = EN.wrap(selection: NSRange(location: 4, length: 3), in: s, command: "textbf")
        XCTAssertEqual(w.range, NSRange(location: 4, length: 3))
        XCTAssertEqual(w.replacement, "\\textbf{foo}")
        XCTAssertEqual(w.selection, NSRange(location: 4 + "\\textbf{foo}".utf16.count, length: 0))
        XCTAssertNil(EditorKeyHandling.programmaticCloser(in: w.replacement, insertedAt: 4, caretUTF16: w.selection.location),
                     "nothing to track: the caret is past the whole construct")
    }

    func testEmptySelectionLeavesTheCaretBetweenTheBracesWithTheCloserTracked() {
        let s = "ab" as NSString
        let w = EN.wrap(selection: NSRange(location: 1, length: 0), in: s, command: "textbf")
        XCTAssertEqual(w.replacement, "\\textbf{}")
        XCTAssertEqual(w.selection, NSRange(location: 1 + "\\textbf{".utf16.count, length: 0))
        XCTAssertEqual(EditorKeyHandling.programmaticCloser(in: w.replacement, insertedAt: 1, caretUTF16: w.selection.location), 9)
        // Out-of-range selections clamp instead of trapping.
        let clamped = EN.wrap(selection: NSRange(location: 10, length: 5), in: s, command: "emph")
        XCTAssertEqual(clamped.range, NSRange(location: 2, length: 0))
        XCTAssertEqual(clamped.replacement, "\\emph{}")
    }

    func testMathModeTriStatePicksTheMathCommandOnlyWhenKnown() {
        XCTAssertEqual(EN.wrapCommand(text: "textbf", math: "mathbf", mathMode: true), "mathbf")
        XCTAssertEqual(EN.wrapCommand(text: "textbf", math: "mathbf", mathMode: false), "textbf")
        XCTAssertEqual(EN.wrapCommand(text: "textbf", math: "mathbf", mathMode: nil), "textbf", "no syntax model: text is the safe default")
    }

    // MARK: model commands

    func testBoldEmphasisUnderlineInTextMode() throws {
        let model = ShellModel()
        model.updateActiveText("a b c")
        model.caretUTF16 = 2; model.caretLengthUTF16 = 1
        XCTAssertEqual(model.mathModeAtCaret, false)
        model.wrapSelectionBold()
        var edit = try XCTUnwrap(model.pendingEdit)
        XCTAssertEqual(edit.nsRange, NSRange(location: 2, length: 1))
        XCTAssertEqual(edit.text, "\\textbf{b}")
        XCTAssertEqual(edit.revision, model.editorRevision)
        XCTAssertNil(edit.trackedCloser)
        XCTAssertEqual(model.selection?.nsRange, NSRange(location: 2 + 10, length: 0))
        XCTAssertEqual(model.navigationNote, "Wrapped in \\textbf{…} (undo with ⌘Z).")
        model.wrapSelectionEmphasis()
        edit = try XCTUnwrap(model.pendingEdit)
        XCTAssertEqual(edit.text, "\\emph{b}")
        model.wrapSelectionUnderline()
        edit = try XCTUnwrap(model.pendingEdit)
        XCTAssertEqual(edit.text, "\\underline{b}")
    }

    func testBoldAndEmphasisSwitchToMathCommandsInMathMode() throws {
        let model = ShellModel()
        model.updateActiveText("a $x$ b")
        model.caretUTF16 = 3; model.caretLengthUTF16 = 1
        XCTAssertEqual(model.mathModeAtCaret, true)
        model.wrapSelectionBold()
        XCTAssertEqual(try XCTUnwrap(model.pendingEdit).text, "\\mathbf{x}")
        model.wrapSelectionEmphasis()
        XCTAssertEqual(try XCTUnwrap(model.pendingEdit).text, "\\mathit{x}")
        model.wrapSelectionUnderline()
        XCTAssertEqual(try XCTUnwrap(model.pendingEdit).text, "\\underline{x}")
        // An empty buffer has no mode to ask: text.
        model.updateActiveText("")
        model.caretUTF16 = 0; model.caretLengthUTF16 = 0
        XCTAssertNil(model.mathModeAtCaret)
        model.wrapSelectionBold()
        XCTAssertEqual(try XCTUnwrap(model.pendingEdit).text, "\\textbf{}")
    }

    func testWrapInCommandAcceptsABackslashRefusesNonNamesAndTracksThePlaceholderCloser() throws {
        let model = ShellModel()
        model.updateActiveText("a b c")
        model.caretUTF16 = 2; model.caretLengthUTF16 = 0
        model.editorNavigation.wrapCommandShown = true
        model.wrapSelection(inCommand: "\\textsc")
        XCTAssertFalse(model.editorNavigation.wrapCommandShown)
        let edit = try XCTUnwrap(model.pendingEdit)
        XCTAssertEqual(edit.nsRange, NSRange(location: 2, length: 0))
        XCTAssertEqual(edit.text, "\\textsc{}")
        XCTAssertEqual(edit.trackedCloser, 2 + "\\textsc{".utf16.count)
        XCTAssertEqual(model.selection?.nsRange, NSRange(location: 2 + "\\textsc{".utf16.count, length: 0))
        // A bad name is refused without an edit.
        let before = model.pendingEdit
        model.wrapSelection(inCommand: "no way")
        XCTAssertEqual(model.pendingEdit, before)
        XCTAssertEqual(model.navigationNote, "“no way” is not a command name.")
        model.wrapSelection(inCommand: "")
        XCTAssertEqual(model.pendingEdit, before)
    }

    // MARK: command table

    func testCommandsAreInTheTableAndThePalette() {
        for c in [AccessibilityCommand.boldSelection, .emphasizeSelection, .underlineSelection, .wrapInCommand] {
            XCTAssertTrue(CommandPaletteModel.isRunnable(c), "\(c)")
            XCTAssertNotNil(c.entry.menuItem)
            XCTAssertEqual(c.entry.menu, "Edit")
        }
        XCTAssertEqual(AccessibilityCommand.boldSelection.entry.shortcuts, ["⌘⇧B"], "⌘B is Compile")
        XCTAssertEqual(AccessibilityCommand.emphasizeSelection.entry.shortcuts, ["⌘I"])
        XCTAssertEqual(AccessibilityCommand.underlineSelection.entry.shortcuts, ["⌘U"])
        XCTAssertEqual(AccessibilityCommand.wrapInCommand.entry.shortcuts, ["⌘⌥W"])
        XCTAssertEqual(CommandPaletteModel.rows(matching: "bold").first?.id, .boldSelection)
    }

    // MARK: hosted editor

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
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
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

    private func settle(_ model: ShellModel) async throws {
        let deadline = Date().addingTimeInterval(3)
        while Date() < deadline, model.pendingEdit != nil { try await Task.sleep(nanoseconds: 20_000_000) }
        try await Task.sleep(nanoseconds: 100_000_000)
    }

    func testWrapThroughTheEditorIsOneUndoStepWithTheCaretAfterTheBrace() async throws {
        let model = ShellModel()
        model.updateActiveText("one two\n")
        let (window, tv, _) = try await host(model)
        defer { window.orderOut(nil) }
        tv.setSelectedRange(NSRange(location: 4, length: 3)) // "two"
        try await Task.sleep(nanoseconds: 30_000_000)
        model.wrapSelectionBold()
        try await settle(model)
        XCTAssertEqual(tv.string, "one \\textbf{two}\n")
        XCTAssertEqual(model.activeText, tv.string)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 4 + "\\textbf{two}".utf16.count, length: 0))
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "one two\n", "one undo step")
    }

    func testEmptySelectionThroughTheEditorTypesOverThePlaceholderBrace() async throws {
        let model = ShellModel()
        model.updateActiveText("ab")
        let (window, tv, co) = try await host(model)
        defer { window.orderOut(nil) }
        tv.setSelectedRange(NSRange(location: 1, length: 0))
        try await Task.sleep(nanoseconds: 30_000_000)
        model.wrapSelectionEmphasis()
        try await settle(model)
        XCTAssertEqual(tv.string, "a\\emph{}b")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 7, length: 0), "between the braces")
        XCTAssertEqual(co.pendingClosers, [7])
        tv.insertText("x", replacementRange: tv.selectedRange())
        tv.insertText("}", replacementRange: tv.selectedRange())
        XCTAssertEqual(tv.string, "a\\emph{x}b", "the `}` was typed over, not doubled")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 9, length: 0))
        XCTAssertEqual(co.pendingClosers, [])
    }
}
