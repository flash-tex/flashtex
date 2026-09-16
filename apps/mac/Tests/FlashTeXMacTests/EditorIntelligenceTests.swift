import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
import HostedWindows
@testable import FlashTeXMac

/// Hover data, ⌘-click routing, Return-key auto-indent/auto-close and the
/// line-number gutter (lane mac-syntax-highlight).
@MainActor
final class EditorIntelligenceTests: XCTestCase {
    typealias EI = EditorIntelligence

    // MARK: tokens and quick info (pure)

    func testTokenUnderPosition() {
        let s = "\\section{A} \\ref{eq:1} \\cite{k1, k2} \\input{ch/x} \\begin{align}" as NSString
        XCTAssertEqual(EI.token(in: s, at: 3), .command(name: "section", range: NSRange(location: 0, length: 8)))
        XCTAssertNil(EI.token(in: s, at: 9)) // "A" is plain
        XCTAssertEqual(EI.token(in: s, at: 18), .reference(command: "ref", key: "eq:1", range: NSRange(location: 17, length: 4)))
        // Comma-separated citation keys: the key under the position, trimmed.
        XCTAssertEqual(EI.token(in: s, at: 30), .reference(command: "cite", key: "k1", range: NSRange(location: 29, length: 2)))
        XCTAssertEqual(EI.token(in: s, at: 32), .reference(command: "cite", key: "k2", range: NSRange(location: 33, length: 2))) // the space before k2 belongs to it
        XCTAssertEqual(EI.token(in: s, at: 47), .file(command: "input", path: "ch/x", range: NSRange(location: 44, length: 4)))
        XCTAssertEqual(EI.token(in: s, at: 61), .environment(name: "align", range: NSRange(location: 57, length: 5)))
        XCTAssertNil(EI.token(in: s, at: -1)); XCTAssertNil(EI.token(in: s, at: s.length))
    }

    func testQuickInfoForCommandsReferencesAndDiagnostics() {
        let s = "\\frac{1}{2} \\ref{eq:1} \\begin{itemize}" as NSString
        let frac = EI.quickInfo(in: s, at: 2)
        XCTAssertEqual(frac?.title, "\\frac"); XCTAssertEqual(frac?.detail, "Command")
        XCTAssertEqual(frac?.documentation, "\\frac{num}{den}: a fraction.")
        let ref = EI.quickInfo(in: s, at: 18)
        XCTAssertEqual(ref?.title, "eq:1"); XCTAssertEqual(ref?.detail, "Label reference")
        // The resolved target comes first (EditorHoverResolution.swift); there
        // is no \label{eq:1} in this snippet, which the hover says outright.
        XCTAssertEqual(ref?.documentation, "No \\label{eq:1} in this document or the open ones.\n⌘-click to go to \\label{eq:1}.")
        let env = EI.quickInfo(in: s, at: 31)
        XCTAssertEqual(env?.title, "itemize"); XCTAssertEqual(env?.detail, "Environment")
        XCTAssertEqual(env?.documentation, "Bulleted list of \\item entries.")
        XCTAssertNil(EI.quickInfo(in: s, at: 6)) // "1": plain, no diagnostic
        // Unknown command: still a title and category, no documentation.
        let unknown = EI.quickInfo(in: "\\foobar" as NSString, at: 1)
        XCTAssertEqual(unknown?.title, "\\foobar"); XCTAssertNil(unknown?.documentation)

        // A diagnostic on plain text shows on its own; on a command it is appended.
        let marks = [
            SourceEditorViewTests.mark(NSRange(location: 6, length: 1), .error, "Missing }", recovery: "inserted }"),
            SourceEditorViewTests.mark(NSRange(location: 0, length: 5), .warning, "Overfull"),
        ]
        let plain = EI.quickInfo(in: s, at: 6, marks: marks)
        XCTAssertEqual(plain?.title, "Error"); XCTAssertEqual(plain?.detail, "Diagnostic")
        XCTAssertEqual(plain?.diagnostics, [.init(severity: .error, message: "Missing }", lines: ["inserted }"])])
        XCTAssertEqual(plain?.range, NSRange(location: 6, length: 1))
        let onCommand = EI.quickInfo(in: s, at: 1, marks: marks)
        XCTAssertEqual(onCommand?.title, "\\frac")
        XCTAssertEqual(onCommand?.diagnostics, [.init(severity: .warning, message: "Overfull", lines: [])])
    }

    func testDefinitionTargets() {
        let s = "\\ref{eq:1} \\citep{knuth} \\include{ch/two} \\end{align} \\alpha x" as NSString
        XCTAssertEqual(EI.definitionTarget(in: s, at: 6), .label(key: "eq:1"))
        XCTAssertEqual(EI.definitionTarget(in: s, at: 19), .citation(key: "knuth"))
        XCTAssertEqual(EI.definitionTarget(in: s, at: 36), .file(path: "ch/two", command: "include"))
        XCTAssertEqual(EI.definitionTarget(in: s, at: 47), .environment(name: "align"))
        XCTAssertEqual(EI.definitionTarget(in: s, at: 55), .command(name: "alpha")) // routed to goToDefinition (EditorNavigation.swift), which explains a missing \newcommand
        XCTAssertNil(EI.definitionTarget(in: s, at: 61)) // plain text
    }

    // MARK: Return key (pure)

    func testNewlineKeepsIndentAndIndentsAfterBegin() {
        XCTAssertEqual(EI.newline(in: "  foo" as NSString, caret: 5, indentUnit: "  ", closeEnvironments: true),
                       .init(text: "\n  ", caretOffset: 3, closedEnvironment: nil))
        XCTAssertEqual(EI.newline(in: "\tfoo bar" as NSString, caret: 4, indentUnit: "  ", closeEnvironments: true),
                       .init(text: "\n\t", caretOffset: 2, closedEnvironment: nil))
        XCTAssertEqual(EI.newline(in: "" as NSString, caret: 0, indentUnit: "    ", closeEnvironments: true),
                       .init(text: "\n", caretOffset: 1, closedEnvironment: nil))
        // After \begin{env}: one level deeper and the matching \end{env} below the caret.
        XCTAssertEqual(EI.newline(in: "  \\begin{itemize}" as NSString, caret: 17, indentUnit: "  ", closeEnvironments: true),
                       .init(text: "\n    \n  \\end{itemize}", caretOffset: 5, closedEnvironment: "itemize"))
        // Optional arguments after the name are fine; trailing spaces too.
        XCTAssertEqual(EI.newline(in: "\\begin{figure}[htbp]{x} " as NSString, caret: 24, indentUnit: "\t", closeEnvironments: true).closedEnvironment, "figure")
        // Auto-close off: indent only.
        XCTAssertEqual(EI.newline(in: "\\begin{itemize}" as NSString, caret: 15, indentUnit: "  ", closeEnvironments: false),
                       .init(text: "\n  ", caretOffset: 3, closedEnvironment: nil))
    }

    func testNewlineDoesNotCloseWhatIsAlreadyClosed() {
        // Balanced already: no second \end.
        let balanced = "\\begin{itemize}\n\\item a\n\\end{itemize}" as NSString
        XCTAssertEqual(EI.newline(in: balanced, caret: 15, indentUnit: "  ", closeEnvironments: true).closedEnvironment, nil)
        XCTAssertEqual(EI.newline(in: balanced, caret: 15, indentUnit: "  ", closeEnvironments: true).text, "\n  ")
        // Two begins, one end: the second gets closed.
        let open = "\\begin{itemize}\n\\begin{itemize}\n\\end{itemize}" as NSString
        XCTAssertEqual(EI.newline(in: open, caret: 15, indentUnit: "  ", closeEnvironments: true).closedEnvironment, "itemize")
        // Text after the caret on the line: no closing (it would land after \end).
        XCTAssertEqual(EI.newline(in: "\\begin{itemize} tail" as NSString, caret: 15, indentUnit: "  ", closeEnvironments: true).closedEnvironment, nil)
        // \begin on the line but not at its end; a comment; `document`; verbatim: never.
        XCTAssertNil(EI.openingEnvironment(inLinePrefix: "\\begin{itemize} \\item"))
        XCTAssertNil(EI.openingEnvironment(inLinePrefix: "% \\begin{itemize}"))
        XCTAssertNil(EI.openingEnvironment(inLinePrefix: "\\begin{document}"))
        XCTAssertNil(EI.openingEnvironment(inLinePrefix: "\\begin{verbatim}"))
        XCTAssertNil(EI.openingEnvironment(inLinePrefix: "\\begin{a}\\end{a}"))
        XCTAssertEqual(EI.openingEnvironment(inLinePrefix: "  \\begin{align*}"), "align*")
    }

    // MARK: completion popup presentation

    func testCompletionRowsCarryKindIconAndDocumentation() {
        let section = Completion.Suggestion(label: "\\section", insertText: "\\section", kind: .command, detail: "supported by this compiler")
        let row = CompletionPopup.attributed(section)
        var attachments = 0
        row.enumerateAttribute(.attachment, in: NSRange(location: 0, length: row.length)) { value, _, _ in
            if let a = value as? NSTextAttachment, a.image != nil { attachments += 1 }
        }
        XCTAssertEqual(attachments, 1, "one kind icon")
        XCTAssertTrue(row.string.contains("\\section  cmd · supported by this compiler — \\section{title}: a numbered section heading."), row.string)
        XCTAssertEqual(CompletionPopup.documentation(for: section), EI.CommandDocs.documentation(for: "section"))
        let env = Completion.Suggestion(label: "\\begin{itemize}", insertText: "itemize}", kind: .environment, detail: "environment")
        XCTAssertEqual(CompletionPopup.documentation(for: env), "Bulleted list of \\item entries.")
        let ref = Completion.Suggestion(label: "eq:1", insertText: "eq:1", kind: .reference, detail: "label")
        XCTAssertNil(CompletionPopup.documentation(for: ref))
        XCTAssertFalse(CompletionPopup.attributed(ref).string.contains(" — "))
        // The spoken label is unchanged by the presentation (accessibility tests read it back).
        XCTAssertTrue(CompletionPopup.spokenLabel(section).hasPrefix("\\section"), CompletionPopup.spokenLabel(section))
        XCTAssertFalse(CompletionPopup.spokenLabel(section).contains("numbered section"), "documentation is visual only")
        for kind in [Completion.Kind.command, .environment, .reference, .citation, .word] { XCTAssertNotNil(kind.icon, "\(kind)") }
    }

    // MARK: hosted editor: gutter, ⌘-click, Return, current line, hover

    final class Probe {
        var coordinator: SourceEditorView.Coordinator?
        var definitions: [EI.DefinitionTarget] = []
        var carets: [Int] = []
    }

    struct Host: View {
        var model: ShellModel
        var probe: Probe
        var marks: [EditorDiagnostics.Mark]
        var body: some View {
            SourceEditorView(
                text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                selection: model.selection, pendingEdit: model.pendingEdit, marks: marks, result: model.result,
                onCaretChange: { probe.carets.append($0); model.caretUTF16 = $0 },
                onDefinitionRequest: { probe.definitions.append($0) }
            )
        }
    }

    private func host(text: String, marks: [EditorDiagnostics.Mark] = []) async throws -> (NSWindow, NSTextView, ShellModel, Probe) {
        let model = ShellModel()
        model.updateActiveText(text)
        let probe = Probe()
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model, probe: probe, marks: marks))
        window.orderFrontRegardless()
        var found: NSTextView?
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline, found == nil {
            found = TypingBenchDriver.findTextView(in: [window.contentView!])
            if found == nil { try await Task.sleep(nanoseconds: 10_000_000) }
        }
        let tv = try XCTUnwrap(found)
        probe.coordinator = tv.delegate as? SourceEditorView.Coordinator
        XCTAssertTrue(window.makeFirstResponder(tv))
        try await Task.sleep(nanoseconds: 50_000_000)
        return (window, tv, model, probe)
    }

    func testGutterIsInstalledWithLineNumbersAndMarkers() async throws {
        let text = "\\documentclass{article}\n\\begin{document}\nHello $x$\n\\end{document}\n"
        let marks = [SourceEditorViewTests.mark(NSRange(location: 47, length: 1), .error, "bad"),
                     SourceEditorViewTests.mark(NSRange(location: 2, length: 3), .warning, "meh")]
        let (window, tv, _, probe) = try await host(text: text, marks: marks)
        defer { window.orderOut(nil) }
        let scroll = try XCTUnwrap(tv.enclosingScrollView)
        XCTAssertTrue(scroll.rulersVisible)
        let gutter = try XCTUnwrap(scroll.verticalRulerView as? LineNumberGutter)
        XCTAssertTrue(gutter === probe.coordinator?.gutter)
        XCTAssertEqual(gutter.severities, [2: .error, 0: .warning])
        XCTAssertGreaterThan(gutter.ruleThickness, 30)
        // Drawing the visible lines into an offscreen context must not touch the storage.
        let rep = try XCTUnwrap(NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(max(1, gutter.bounds.width)), pixelsHigh: Int(max(1, gutter.bounds.height)),
                                                 bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                                                 colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
        let ctx = try XCTUnwrap(NSGraphicsContext(bitmapImageRep: rep))
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = ctx
        gutter.drawHashMarksAndLabels(in: gutter.bounds)
        NSGraphicsContext.restoreGraphicsState()
        XCTAssertEqual(tv.string, text)
        // Colours are on: the \documentclass run carries the command colour as a temporary attribute.
        let attrs = tv.layoutManager?.temporaryAttributes(atCharacterIndex: 1, effectiveRange: nil)
        XCTAssertNotNil(attrs?[.foregroundColor])
        XCTAssertGreaterThan(probe.coordinator?.syntax.paints ?? 0, 0)
    }

    func testCommandClickRoutesDefinitionAndMovesCaret() async throws {
        let text = "See \\ref{eq:main} and \\cite{knuth84}.\n\\input{ch/one}\n"
        let (window, tv, model, probe) = try await host(text: text)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        XCTAssertTrue(co.commandClick(at: 10))
        XCTAssertEqual(probe.definitions, [.label(key: "eq:main")])
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 10, length: 0))
        XCTAssertEqual(model.caretUTF16, 10)
        XCTAssertTrue(co.commandClick(at: 29))
        XCTAssertTrue(co.commandClick(at: 46))
        XCTAssertEqual(probe.definitions, [.label(key: "eq:main"), .citation(key: "knuth84"), .file(path: "ch/one", command: "input")])
        XCTAssertFalse(co.commandClick(at: 1)) // plain text: not consumed, nothing routed
        XCTAssertEqual(probe.definitions.count, 3)
        XCTAssertEqual(co.definitionRequests.count, 3)
        // The hook on the text view is the coordinator's handler.
        let completing = try XCTUnwrap(tv as? CompletingTextView)
        XCTAssertEqual(completing.commandClickHandler?(10), true)
        XCTAssertEqual(probe.definitions.count, 4)
    }

    func testReturnAutoIndentsAndClosesEnvironment() async throws {
        let text = "  \\begin{itemize}"
        let (window, tv, model, probe) = try await host(text: text)
        defer { window.orderOut(nil) }
        let unit = EditorPreferences.shared.indentString // the user's real preference (tests never write it)
        tv.setSelectedRange(NSRange(location: text.utf16.count, length: 0))
        tv.doCommand(by: #selector(NSResponder.insertNewline(_:))) // the key path: delegate doCommandBy(insertNewline:)
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(tv.string, "  \\begin{itemize}\n  \(unit)\n  \\end{itemize}")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 20 + unit.utf16.count, length: 0))
        XCTAssertEqual(model.activeText, tv.string) // the binding saw the edit once
        XCTAssertEqual(probe.coordinator?.currentLine, 1)
        // Undo removes the whole insertion.
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, text)
        // Plain Return keeps the indentation of the line.
        tv.setSelectedRange(NSRange(location: 2, length: 0))
        tv.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        XCTAssertEqual(tv.string, "  \n  \\begin{itemize}")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 5, length: 0))
    }

    func testCurrentLineFollowsCaretAndHoverPresentsQuickInfo() async throws {
        let text = "one\n\\section{two}\nthree"
        let (window, tv, _, probe) = try await host(text: text)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        XCTAssertEqual(co.currentLine, 0)
        tv.setSelectedRange(NSRange(location: 6, length: 0))
        XCTAssertEqual(co.currentLine, 1)
        XCTAssertEqual(co.gutter?.currentLine, 1)
        tv.setSelectedRange(NSRange(location: text.utf16.count, length: 0))
        XCTAssertEqual(co.currentLine, 2)
        // Hover: the coordinator answers with the token's info; presenting anchors a popover.
        let info = try XCTUnwrap(co.quickInfo(at: 6))
        XCTAssertEqual(info.title, "\\section")
        co.hover.present(info, in: tv)
        XCTAssertEqual(co.hover.shownRange, NSRange(location: 4, length: 8))
        XCTAssertEqual(co.hover.presented.count, 1)
        co.hover.dismiss()
        XCTAssertNil(co.hover.shownRange)
        XCTAssertNil(co.quickInfo(at: 0)) // plain text without a diagnostic
    }
}
