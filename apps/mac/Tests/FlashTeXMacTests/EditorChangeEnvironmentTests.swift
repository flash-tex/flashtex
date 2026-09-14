import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXMac
@testable import FlashTeXAccessibility

/// Change Environment… (EditorChangeEnvironment.swift / ⌃⌘E): pure name-span
/// rewrites, model command + refusal, hosted one-step undo and linked editing.
@MainActor
final class EditorChangeEnvironmentTests: XCTestCase {
    typealias CE = EditorChangeEnvironment

    private func names(_ text: String, caret: Int, to newName: String) -> [String] {
        let ns = text as NSString
        return CE.replacements(in: ns, caret: caret, newName: newName).map {
            ns.substring(with: $0.range)
        }
    }

    private func applied(_ text: String, caret: Int, to newName: String) -> String? {
        let ns = text as NSString
        let edits = CE.replacements(in: ns, caret: caret, newName: newName)
        guard let g = CE.groupedReplacement(edits: edits, in: ns) else { return nil }
        return ns.replacingCharacters(in: g.range, with: g.text)
    }

    func testNestedSameNamePicksInnermost() {
        let s = "\\begin{itemize}\n\\begin{itemize}\n\\item x\n\\end{itemize}\n\\end{itemize}"
        let innerItem = (s as NSString).range(of: "\\item x")
        XCTAssertEqual(names(s, caret: innerItem.location, to: "enumerate"), ["itemize", "itemize"])
        XCTAssertEqual(applied(s, caret: innerItem.location, to: "enumerate"),
                       "\\begin{itemize}\n\\begin{enumerate}\n\\item x\n\\end{enumerate}\n\\end{itemize}")
        // Caret on the outer \\begin name keeps the outer pair.
        let outerName = (s as NSString).range(of: "itemize")
        XCTAssertEqual(applied(s, caret: outerName.location, to: "enumerate"),
                       "\\begin{enumerate}\n\\begin{itemize}\n\\item x\n\\end{itemize}\n\\end{enumerate}")
    }

    func testStarredEnvironmentRewritesBothNames() {
        let s = "\\begin{align*}\nx=1\n\\end{align*}"
        let caret = (s as NSString).range(of: "x=1").location
        XCTAssertEqual(names(s, caret: caret, to: "gather*"), ["align*", "align*"])
        XCTAssertEqual(applied(s, caret: caret, to: "gather*"), "\\begin{gather*}\nx=1\n\\end{gather*}")
        XCTAssertEqual(applied(s, caret: caret, to: "equation"), "\\begin{equation}\nx=1\n\\end{equation}")
    }

    func testOptionalAndFollowingArgumentsArePreserved() {
        let s = "\\begin{minipage}[t]{0.5\\textwidth}\nx\n\\end{minipage}"
        let caret = (s as NSString).range(of: "\nx\n").location + 1
        XCTAssertEqual(applied(s, caret: caret, to: "figure"),
                       "\\begin{figure}[t]{0.5\\textwidth}\nx\n\\end{figure}")
        let starred = "\\begin{table*}[htbp]\ny\n\\end{table*}"
        XCTAssertEqual(applied(starred, caret: (starred as NSString).range(of: "y").location, to: "figure*"),
                       "\\begin{figure*}[htbp]\ny\n\\end{figure*}")
    }

    func testEndOnTheSameLine() {
        let s = "wrap \\begin{center}hi\\end{center} after"
        let caret = (s as NSString).range(of: "hi").location
        XCTAssertEqual(applied(s, caret: caret, to: "quote"),
                       "wrap \\begin{quote}hi\\end{quote} after")
        XCTAssertEqual(CE.replacements(in: s as NSString, caret: caret, newName: "quote").count, 2)
    }

    func testCRLFAndNonASCIINames() {
        let crlf = "\\begin{a}\r\nbody\r\n\\end{a}"
        XCTAssertEqual(applied(crlf, caret: (crlf as NSString).range(of: "body").location, to: "b"),
                       "\\begin{b}\r\nbody\r\n\\end{b}")
        let cafe = "\\begin{café}\nhi\n\\end{café}"
        XCTAssertEqual(applied(cafe, caret: (cafe as NSString).range(of: "hi").location, to: "naïve"),
                       "\\begin{naïve}\nhi\n\\end{naïve}")
        XCTAssertEqual(CE.nameProblem("café"), nil)
        XCTAssertEqual(CE.nameProblem("align*"), nil)
    }

    func testUnbalancedInputRefuses() {
        let open = "\\begin{itemize}\n\\item x"
        XCTAssertTrue(CE.replacements(in: open as NSString, caret: 16, newName: "enumerate").isEmpty)
        XCTAssertEqual(CE.refusal(in: open as NSString, caret: 16), .unbalanced)
        XCTAssertNil(CE.target(in: open as NSString, caret: 16))
        XCTAssertTrue(CE.replacements(in: "plain text" as NSString, caret: 3, newName: "itemize").isEmpty)
        XCTAssertEqual(CE.refusal(in: "plain text" as NSString, caret: 3), .notInside)
    }

    func testVerbatimBodyRefuses() {
        let s = "\\begin{verbatim}\n\\begin{itemize}\n\\end{verbatim}\n"
        let inside = (s as NSString).range(of: "\\begin{itemize}").location
        XCTAssertTrue(CE.replacements(in: s as NSString, caret: inside, newName: "enumerate").isEmpty)
        XCTAssertEqual(CE.refusal(in: s as NSString, caret: inside), .verbatim)
        let verb = "before \\verb|\\begin{a}\\end{a}| after"
        let v = (verb as NSString).range(of: "\\begin{a}").location
        XCTAssertEqual(CE.refusal(in: verb as NSString, caret: v), .verbatim)
        // The verbatim environment's own names still change.
        let name = (s as NSString).range(of: "verbatim")
        XCTAssertEqual(applied(s, caret: name.location, to: "lstlisting"),
                       "\\begin{lstlisting}\n\\begin{itemize}\n\\end{lstlisting}\n")
    }

    func testInvalidAndUnchangedNames() {
        let s = "\\begin{itemize}\\end{itemize}"
        XCTAssertTrue(CE.replacements(in: s as NSString, caret: 8, newName: "itemize").isEmpty)
        XCTAssertNil(CE.refusal(in: s as NSString, caret: 8), "same name is a no-op, not a refusal")
        XCTAssertTrue(CE.replacements(in: s as NSString, caret: 8, newName: "no way").isEmpty)
        XCTAssertNotNil(CE.nameProblem("no way"))
        XCTAssertNotNil(CE.nameProblem(""))
        XCTAssertNotNil(CE.nameProblem("foo-bar"))
    }

    func testSuggestionsIncludeInventoryAndDocumentNames() {
        XCTAssertTrue(CE.suggestions(query: "", in: "").contains("itemize"))
        XCTAssertTrue(CE.suggestions(query: "", in: "").contains("enumerate"))
        let doc = "\\begin{myenv}\\end{myenv}"
        XCTAssertTrue(CE.suggestions(query: "", in: doc).contains("myenv"))
        XCTAssertTrue(CE.suggestions(query: "item", in: doc).contains("itemize"))
        XCTAssertFalse(CE.suggestions(query: "item", in: doc).contains("enumerate"))
    }

    func testLinkedNamesOnlyOnNameSpansOfBalancedPairs() throws {
        let s = "\\begin{itemize}\n\\item x\n\\end{itemize}" as NSString
        let beginName = s.range(of: "itemize")
        XCTAssertEqual(CE.linkedNames(at: beginName.location, in: s)?.name, "itemize")
        XCTAssertEqual(CE.linkedNames(at: beginName.location, in: s)?.partner, s.range(of: "itemize", range: NSRange(location: 16, length: s.length - 16)))
        XCTAssertNil(CE.linkedNames(at: s.range(of: "\\item").location, in: s), "body is not a name span")
        XCTAssertNil(CE.linkedNames(at: 8, in: "\\begin{itemize}" as NSString), "unbalanced never links")
        XCTAssertTrue(CE.isOnEnvironmentName(in: s, at: beginName.location))
        XCTAssertFalse(CE.isOnEnvironmentName(in: s, at: s.range(of: "\\item").location))
        let endLine = "\\end{document}" as NSString
        XCTAssertFalse(CE.isOnEnvironmentName(in: endLine, at: 0), "caret on the backslash is not in the name")
        XCTAssertTrue(CE.isOnEnvironmentName(in: endLine, at: endLine.range(of: "document").location))
        let insertAt = NSRange(location: beginName.location, length: 0)
        let partner = try XCTUnwrap(CE.linkedPartnerEdit(old: s, edit: (insertAt, "x")))
        XCTAssertEqual(partner.replacement, "xitemize")
        let after = s.replacingCharacters(in: insertAt, with: "x") as NSString
        XCTAssertEqual(after.replacingCharacters(in: partner.range, with: partner.replacement),
                       "\\begin{xitemize}\n\\item x\n\\end{xitemize}")
        XCTAssertNil(CE.linkedPartnerEdit(old: s, edit: (s.range(of: "\\item"), "y")), "body edits do not link")
        let atEnd = NSRange(location: NSMaxRange(beginName), length: 0)
        let suffix = try XCTUnwrap(CE.linkedPartnerEdit(old: s, edit: (atEnd, "x")))
        XCTAssertEqual(suffix.replacement, "itemizex")
    }

    // MARK: model

    func testPresentPrefillsAndApplyPostsOneGroupedEdit() {
        let model = ShellModel()
        model.updateActiveText("\\begin{itemize}\n\\item a\n\\end{itemize}")
        model.caretUTF16 = (model.activeText as NSString).range(of: "\\item").location
        model.presentChangeEnvironment()
        XCTAssertTrue(model.editorNavigation.changeShown)
        XCTAssertEqual(model.editorNavigation.changeName, "itemize")
        model.editorNavigation.changeName = "enumerate"
        model.applyChangeEnvironment()
        XCTAssertFalse(model.editorNavigation.changeShown)
        let edit = try! XCTUnwrap(model.pendingEdit)
        XCTAssertEqual((model.activeText as NSString).replacingCharacters(in: edit.nsRange, with: edit.text),
                       "\\begin{enumerate}\n\\item a\n\\end{enumerate}")
        XCTAssertEqual(model.navigationNote, "Changed environment to \\begin{enumerate}…\\end{enumerate} (undo with ⌘Z).")
    }

    func testPresentRefusesUnbalancedAndVerbatimWithAnnouncement() {
        let model = ShellModel()
        model.updateActiveText("\\begin{itemize}\n\\item a")
        model.caretUTF16 = 16
        model.presentChangeEnvironment()
        XCTAssertFalse(model.editorNavigation.changeShown)
        XCTAssertEqual(model.navigationNote, CE.Refusal.unbalanced.message)
        XCTAssertEqual(model.editorNavigation.changeAnnouncements.last, CE.Refusal.unbalanced.message)
        model.updateActiveText("\\begin{verbatim}\n\\begin{a}\n\\end{verbatim}")
        model.caretUTF16 = (model.activeText as NSString).range(of: "\\begin{a}").location
        model.presentChangeEnvironment()
        XCTAssertEqual(model.navigationNote, CE.Refusal.verbatim.message)
        XCTAssertEqual(model.editorNavigation.changeAnnouncements.last, CE.Refusal.verbatim.message)
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
        HostedWindowSupport.prepare()
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

    func testChangeThroughTheEditorIsOneUndoStep() async throws {
        let model = ShellModel()
        let original = "\\begin{itemize}\n\\item a\n\\end{itemize}\n"
        model.updateActiveText(original)
        let (window, tv, _) = try await host(model)
        defer { window.orderOut(nil) }
        model.caretUTF16 = (original as NSString).range(of: "\\item").location
        model.editorNavigation.changeName = "enumerate"
        model.applyChangeEnvironment()
        let deadline = Date().addingTimeInterval(3)
        while Date() < deadline, model.pendingEdit != nil { try await Task.sleep(nanoseconds: 20_000_000) }
        XCTAssertEqual(tv.string, "\\begin{enumerate}\n\\item a\n\\end{enumerate}\n")
        XCTAssertEqual(model.activeText, tv.string)
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, original)
    }

    func testLinkedEditingUpdatesPartnerAndUndoesTogether() async throws {
        let model = ShellModel()
        let original = "\\begin{itemize}\n\\item a\n\\end{itemize}\n"
        model.updateActiveText(original)
        let (window, tv, _) = try await host(model)
        defer { window.orderOut(nil) }
        let beginName = (original as NSString).range(of: "itemize")
        tv.setSelectedRange(NSRange(location: beginName.location, length: 0))
        try await Task.sleep(nanoseconds: 30_000_000)
        let insertAt = NSRange(location: beginName.location, length: 0)
        tv.insertText("x", replacementRange: insertAt)
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(tv.string, "\\begin{xitemize}\n\\item a\n\\end{xitemize}\n")
        XCTAssertEqual(model.activeText, tv.string)
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, original, "linked partner rewrite shares the typing undo group")
    }

    func testCommandsAreInTheTableAndThePalette() {
        XCTAssertTrue(CommandPaletteModel.isRunnable(.changeEnvironment))
        XCTAssertEqual(AccessibilityCommand.changeEnvironment.entry.menu, "Editor")
        XCTAssertEqual(AccessibilityCommand.changeEnvironment.entry.menuItem, "Change Environment…")
        XCTAssertEqual(AccessibilityCommand.changeEnvironment.entry.shortcuts, ["⌃⌘E"])
        XCTAssertEqual(CommandPaletteModel.rows(matching: "change environment").first?.id, .changeEnvironment)
        XCTAssertEqual(CommandPaletteModel.rows(matching: "⌃⌘E").first?.id, .changeEnvironment)
    }
}
