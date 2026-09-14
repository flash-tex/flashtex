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
        guard !edits.isEmpty else { return nil }
        var cur = ns
        for e in edits.sorted(by: { $0.range.location > $1.range.location }) {
            cur = cur.replacingCharacters(in: e.range, with: e.replacement) as NSString
        }
        return cur as String
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
        XCTAssertEqual(edit.text, "enumerate")
        XCTAssertEqual(edit.groupedEdits.count, 2)
        var ns = model.activeText as NSString
        for e in edit.groupedEdits.sorted(by: { $0.range.location > $1.range.location }) {
            ns = ns.replacingCharacters(in: e.range, with: e.replacement) as NSString
        }
        XCTAssertEqual(ns as String, "\\begin{enumerate}\n\\item a\n\\end{enumerate}")
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

    // MARK: review blockers (fail on the unregistered-partner / covering-edit tree)

    /// Begin and end names of the innermost pair around `\\item`.
    private func pairNames(_ text: String) -> (begin: String, end: String)? {
        let ns = text as NSString
        let caret = ns.range(of: "\\item")
        guard caret.location != NSNotFound, let t = CE.target(in: ns, caret: caret.location) else { return nil }
        return (ns.substring(with: t.beginName), ns.substring(with: t.endName))
    }

    private func assertPaired(_ text: String, _ expected: String, _ message: String,
                              file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertEqual(text, expected, message, file: file, line: line)
        let got = pairNames(text)
        let want = pairNames(expected)
        XCTAssertEqual(got?.begin, want?.begin, "begin name after: \(message)", file: file, line: line)
        XCTAssertEqual(got?.end, want?.end, "end name after: \(message)", file: file, line: line)
        XCTAssertEqual(got?.begin, got?.end, "names must stay paired: \(message)", file: file, line: line)
    }

    private func nameRange(_ text: String, of token: String, which: String) -> NSRange {
        let ns = text as NSString
        if which == "begin" { return ns.range(of: token) }
        let first = ns.range(of: token)
        return ns.range(of: token, range: NSRange(location: NSMaxRange(first), length: ns.length - NSMaxRange(first)))
    }

    /// ⌘Z / ⌘⇧Z of a linked name edit must restore the whole document and both
    /// names. Editing `\\end{…}` is the blocker: the partner `\\begin{…}` is
    /// earlier, so an unregistered partner insert shifts the already-registered
    /// keystroke and undo removes the wrong character.
    func testUndoOfEndNameInsertRestoresBothNames() async throws {
        try await linkedNameUndoCase(which: "end", insert: "x", asSeparateGroups: false)
    }

    func testUndoOfEndNamePasteRestoresBothNames() async throws {
        try await linkedNameUndoCase(which: "end", insert: "xyz", asSeparateGroups: false)
    }

    func testUndoOfThreeEndNameKeystrokesRestoresEachPair() async throws {
        try await linkedNameUndoCase(which: "end", insert: "abc", asSeparateGroups: true)
    }

    func testUndoOfBeginNameInsertRestoresBothNames() async throws {
        try await linkedNameUndoCase(which: "begin", insert: "x", asSeparateGroups: false)
    }

    func testUndoOfBeginNamePasteRestoresBothNames() async throws {
        try await linkedNameUndoCase(which: "begin", insert: "xyz", asSeparateGroups: false)
    }

    func testUndoOfThreeBeginNameKeystrokesRestoresEachPair() async throws {
        try await linkedNameUndoCase(which: "begin", insert: "abc", asSeparateGroups: true)
    }

    private func linkedNameUndoCase(which: String, insert: String, asSeparateGroups: Bool) async throws {
        let model = ShellModel()
        let original = "\\begin{itemize}\n\\item a\n\\end{itemize}\n"
        model.updateActiveText(original)
        let (window, tv, _) = try await host(model)
        defer { window.orderOut(nil) }
        let name = nameRange(original, of: "itemize", which: which)
        tv.setSelectedRange(NSRange(location: name.location, length: 0))
        try await Task.sleep(nanoseconds: 30_000_000)
        tv.undoManager?.removeAllActions()
        if asSeparateGroups {
            for ch in insert {
                tv.breakUndoCoalescing()
                let loc = tv.selectedRange().location
                tv.insertText(String(ch), replacementRange: NSRange(location: loc, length: 0))
                try await Task.sleep(nanoseconds: 40_000_000)
            }
        } else {
            tv.insertText(insert, replacementRange: NSRange(location: name.location, length: 0))
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        var expected = original as NSString
        expected = expected.replacingCharacters(in: nameRange(expected as String, of: "itemize", which: "end"), with: insert + "itemize") as NSString
        expected = expected.replacingCharacters(in: nameRange(expected as String, of: "itemize", which: "begin"), with: insert + "itemize") as NSString
        XCTAssertEqual(model.activeText, tv.string)
        if asSeparateGroups {
            let prefixes = (1...insert.count).map { String(insert.prefix($0)) }
            assertPaired(tv.string, expected as String, "after typing \(insert) into the \(which) name")
            for p in prefixes.reversed() {
                tv.undoManager?.undo()
                try await Task.sleep(nanoseconds: 30_000_000)
                var step = original as NSString
                if !p.dropLast().isEmpty {
                    let pre = String(p.dropLast())
                    step = step.replacingCharacters(in: nameRange(step as String, of: "itemize", which: "end"), with: pre + "itemize") as NSString
                    step = step.replacingCharacters(in: nameRange(step as String, of: "itemize", which: "begin"), with: pre + "itemize") as NSString
                }
                let label = p.dropLast().isEmpty ? "original" : "after undo back to \(p.dropLast())"
                assertPaired(tv.string, step as String, "⌘Z \(which) → \(label)")
                XCTAssertEqual(model.activeText, tv.string)
            }
            for p in prefixes {
                tv.undoManager?.redo()
                try await Task.sleep(nanoseconds: 30_000_000)
                var step = original as NSString
                step = step.replacingCharacters(in: nameRange(step as String, of: "itemize", which: "end"), with: p + "itemize") as NSString
                step = step.replacingCharacters(in: nameRange(step as String, of: "itemize", which: "begin"), with: p + "itemize") as NSString
                assertPaired(tv.string, step as String, "⌘⇧Z \(which) → \(p)")
                XCTAssertEqual(model.activeText, tv.string)
            }
        } else {
            assertPaired(tv.string, expected as String, "after inserting \(insert) into the \(which) name")
            tv.undoManager?.undo()
            try await Task.sleep(nanoseconds: 30_000_000)
            assertPaired(tv.string, original, "⌘Z of \(which) insert \(insert)")
            XCTAssertEqual(model.activeText, tv.string)
            tv.undoManager?.redo()
            try await Task.sleep(nanoseconds: 30_000_000)
            assertPaired(tv.string, expected as String, "⌘⇧Z of \(which) insert \(insert)")
            XCTAssertEqual(model.activeText, tv.string)
        }
    }

    /// Deletion inside a name is a replacement with "" and must mirror.
    func testBackspaceInsideBeginNameMirrorsAndUndoes() async throws {
        try await linkedNameDeleteCase(which: "begin", forward: false)
    }

    func testForwardDeleteInsideBeginNameMirrorsAndUndoes() async throws {
        try await linkedNameDeleteCase(which: "begin", forward: true)
    }

    func testBackspaceInsideEndNameMirrorsAndUndoes() async throws {
        try await linkedNameDeleteCase(which: "end", forward: false)
    }

    func testForwardDeleteInsideEndNameMirrorsAndUndoes() async throws {
        try await linkedNameDeleteCase(which: "end", forward: true)
    }

    private func linkedNameDeleteCase(which: String, forward: Bool) async throws {
        let model = ShellModel()
        let original = "\\begin{itemize}\n\\item a\n\\end{itemize}\n"
        model.updateActiveText(original)
        let (window, tv, _) = try await host(model)
        defer { window.orderOut(nil) }
        let name = nameRange(original, of: "itemize", which: which)
        // Backspace the last letter (`e`); forward-delete the first (`i`).
        let caret = forward ? name.location : NSMaxRange(name)
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        try await Task.sleep(nanoseconds: 30_000_000)
        tv.undoManager?.removeAllActions()
        tv.doCommand(by: forward ? #selector(NSResponder.deleteForward(_:)) : #selector(NSResponder.deleteBackward(_:)))
        try await Task.sleep(nanoseconds: 50_000_000)
        let afterNames = forward ? "temize" : "itemiz"
        let after = "\\begin{\(afterNames)}\n\\item a\n\\end{\(afterNames)}\n"
        assertPaired(tv.string, after, "\(forward ? "forward-delete" : "backspace") inside the \(which) name")
        XCTAssertEqual(model.activeText, tv.string)
        tv.undoManager?.undo()
        try await Task.sleep(nanoseconds: 30_000_000)
        assertPaired(tv.string, original, "⌘Z of \(which) \(forward ? "forward-delete" : "backspace")")
        XCTAssertEqual(model.activeText, tv.string)
        tv.undoManager?.redo()
        try await Task.sleep(nanoseconds: 30_000_000)
        assertPaired(tv.string, after, "⌘⇧Z of \(which) \(forward ? "forward-delete" : "backspace")")
    }

    /// Change Environment… must replace the two name spans only. A covering
    /// replacement from begin-name through end-name rewrites the body (marks,
    /// folds, undo size).
    func testApplyChangeEnvironmentDoesNotRewriteTheBody() {
        let original = "\\begin{itemize}\n\\item UNIQUE_BODY\n\\end{itemize}"
        let model = ShellModel()
        model.updateActiveText(original)
        model.caretUTF16 = (original as NSString).range(of: "UNIQUE_BODY").location
        model.editorNavigation.changeName = "enumerate"
        model.applyChangeEnvironment()
        let edit = try! XCTUnwrap(model.pendingEdit)
        let body = (original as NSString).range(of: "UNIQUE_BODY")
        XCTAssertEqual(NSIntersectionRange(edit.nsRange, body).length, 0,
                       "pendingEdit.nsRange must not cover the environment body")
        XCTAssertEqual(edit.text, "enumerate",
                       "the replacement is a name span, not a whole-environment rewrite")
        let names = CE.replacements(in: original as NSString, caret: model.caretUTF16, newName: "enumerate")
        XCTAssertEqual(names.count, 2)
        for e in names {
            XCTAssertEqual(NSIntersectionRange(e.range, body).length, 0)
            XCTAssertEqual((original as NSString).substring(with: e.range), "itemize")
        }
    }

    func testChangeEnvironmentThroughTheEditorLeavesTheBodyBytesAlone() async throws {
        let model = ShellModel()
        let original = "\\begin{itemize}\n\\item UNIQUE_BODY\n\\end{itemize}\n"
        model.updateActiveText(original)
        let (window, tv, _) = try await host(model)
        defer { window.orderOut(nil) }
        model.caretUTF16 = (original as NSString).range(of: "UNIQUE_BODY").location
        model.editorNavigation.changeName = "enumerate"
        model.applyChangeEnvironment()
        let deadline = Date().addingTimeInterval(3)
        while Date() < deadline, model.pendingEdit != nil { try await Task.sleep(nanoseconds: 20_000_000) }
        let after = tv.string
        XCTAssertEqual(after, "\\begin{enumerate}\n\\item UNIQUE_BODY\n\\end{enumerate}\n")
        XCTAssertEqual(model.activeText, after)
        let beforeBody = (original as NSString).substring(with: (original as NSString).range(of: "\\item UNIQUE_BODY\n"))
        let afterBody = (after as NSString).substring(with: (after as NSString).range(of: "\\item UNIQUE_BODY\n"))
        XCTAssertEqual(Array(beforeBody.utf8), Array(afterBody.utf8), "body bytes must be identical")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, original)
        tv.undoManager?.redo()
        XCTAssertEqual(tv.string, after)
    }
}
