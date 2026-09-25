import Accessibility
import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
import FlashTeXAccessibility
import HostedWindows
@testable import FlashTeXMac

/// Inline diagnostics as VoiceOver hears them in the real editor
/// (ux-editor-diagnostics-voiceover): a caret move onto an underline says
/// "Line L, column C; Error: message", the text view's custom content
/// carries the message, recovery and explanation while the caret is there,
/// and the Diagnostics rotor of the hosted view reads the marks the model
/// drew. The editor is hosted through `NSHostingView` in a window that is
/// never made key (the tests must not steal focus).
@MainActor
final class EditorDiagnosticsVoiceOverTests: XCTestCase {
    static let text = "line one\n\\foo here\nline three\n\\mathbb{R}\nlast\n"

    /// An error on line 2 (`\foo`, bytes 9..<13), a gap on line 4
    /// (`\mathbb`, bytes 30..<37) and an unsourced warning.
    static func diagnostics() -> [RuntimeV1.Diagnostic] {
        [
            .init(severity: .error, message: "Undefined control sequence \\foo",
                  source: .init(path: "main.tex", startByte: 9, endByte: 13), recovery: "ignored"),
            .init(severity: .warning, message: "\\mathbb is not supported in math mode",
                  source: .init(path: "main.tex", startByte: 30, endByte: 37), recovery: nil),
            .init(severity: .warning, message: "Overfull line", source: nil, recovery: nil),
        ]
    }

    /// A model whose result produced marks for `text` (the compiled text is the current text).
    static func model() -> ShellModel {
        let m = ShellModel()
        m.replaceProject(entryText: text)
        m.result = RuntimeV1.CompileResult(projectId: "p", revision: m.editorRevision, status: .recovered, pages: [],
                                           diagnostics: diagnostics(), pdfPath: nil)
        m.resultID = "r1"
        m.setCompiledDocuments(["main.tex": text])
        m.retainMarksAfterResultBound()
        return m
    }

    // MARK: pure

    func testMarksUnderTheCaretAndTheirSpokenForm() {
        let marks = Self.model().editorMarks
        XCTAssertEqual(marks.count, 2, "the unsourced warning has no underline")
        let error = marks.first { $0.severity == .error }!
        let gap = marks.first { $0.severity == .warning }!
        XCTAssertEqual(EditorCaretDiagnostics.marks(at: 8, in: marks), [])
        XCTAssertEqual(EditorCaretDiagnostics.marks(at: 9, in: marks), [error], "the start of the range counts")
        XCTAssertEqual(EditorCaretDiagnostics.marks(at: 13, in: marks), [error], "a caret just after the underline counts")
        XCTAssertEqual(EditorCaretDiagnostics.marks(at: 14, in: marks), [])
        XCTAssertEqual(EditorCaretDiagnostics.spokenSeverity(error), "Error")
        XCTAssertEqual(EditorCaretDiagnostics.spokenSeverity(gap), "Not implemented")
        XCTAssertEqual(EditorCaretDiagnostics.announcementSuffix([]), "")
        XCTAssertEqual(EditorCaretDiagnostics.announcementSuffix([error]), "; Error: Undefined control sequence \\foo")
        XCTAssertEqual(EditorCaretDiagnostics.announcementSuffix([error, gap]),
                       "; Error: Undefined control sequence \\foo; Not implemented: \\mathbb is not supported in math mode")
        let content = EditorCaretDiagnostics.customContent([error, gap])
        XCTAssertEqual(content.map(\.label), ["Error", "Recovery", "Not implemented", "Recovery"])
        XCTAssertEqual(content.map(\.value), ["Undefined control sequence \\foo", "recovery: ignored",
                                              "\\mathbb is not supported in math mode", "no provisional rendering"])
        XCTAssertEqual(content.map(\.importance), [.high, .default, .high, .default],
                       "the message is spoken with the element; the recovery line is there on request")
        var explained = error
        explained.explanation = "TeX does not know \\foo; define it or fix the spelling."
        XCTAssertEqual(EditorCaretDiagnostics.customContent([explained]).map(\.label), ["Error", "Recovery", "Explanation"])
    }

    // MARK: hosted editor

    private final class Probe { var coordinator: SourceEditorView.Coordinator? }

    private struct Host: View {
        var model: ShellModel
        var probe: Probe
        var body: some View {
            SourceEditorView(
                text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                selection: model.selection, pendingEdit: model.pendingEdit, marks: model.editorMarks, result: model.result,
                onCaretChange: { model.caretUTF16 = $0 },
                onSelectionChange: { model.caretLengthUTF16 = $0.length },
                onEditApplied: { edit, text in model.editApplied(edit, newText: text) },
                onEditRefused: { edit, reason in model.editRefused(edit, reason: reason) }
            )
        }
    }

    private func host(_ model: ShellModel, probe: Probe) async throws -> (NSWindow, CompletingTextView) {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model, probe: probe))
        window.orderFrontRegardless() // never makeKey
        var found: NSTextView?
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline, found == nil {
            found = TypingBenchDriver.findTextView(in: [window.contentView!])
            if found == nil { try await Task.sleep(nanoseconds: 10_000_000) }
        }
        let tv = try XCTUnwrap(found as? CompletingTextView)
        probe.coordinator = tv.delegate as? SourceEditorView.Coordinator
        XCTAssertNotNil(probe.coordinator)
        XCTAssertTrue(window.makeFirstResponder(tv))
        return (window, tv)
    }

    /// Lets the current run-loop turn end (coalesced announcements).
    private func turn() async throws { try await Task.sleep(nanoseconds: 30_000_000) }

    func testCaretOnAnUnderlineIsAnnouncedAndExposedAsCustomContent() async throws {
        let model = Self.model()
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        // The painter received the model's marks (SwiftUI's first update).
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline, co.marks.marks.count != 2 { try await Task.sleep(nanoseconds: 10_000_000) }
        XCTAssertEqual(co.marks.marks.count, 2)
        XCTAssertEqual(tv.diagnosticMarks().count, 2, "the view reads the painter's marks")
        XCTAssertEqual(tv.accessibilityHelp(), "LaTeX source editor. Moving the selection announces the line and column, and any diagnostic under the insertion point.")
        var spoken: [String] = []
        co.announce = { spoken.append($0) }

        // Off every mark: the plain line/column, no custom content.
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        try await turn()
        XCTAssertEqual(spoken.last, "Line 1, column 1")
        XCTAssertEqual(tv.accessibilityCustomContent.count, 0)

        // Onto the error: the message follows the position, and is readable on request.
        tv.setSelectedRange(NSRange(location: 10, length: 0))
        try await turn()
        XCTAssertEqual(spoken.last, "Line 2, column 2; Error: Undefined control sequence \\foo")
        XCTAssertEqual(tv.accessibilityCustomContent.map(\.label), ["Error", "Recovery"])
        XCTAssertEqual(tv.accessibilityCustomContent.map(\.value), ["Undefined control sequence \\foo", "recovery: ignored"])

        // Onto the gap: spoken as "Not implemented", never as a bare warning.
        tv.setSelectedRange(NSRange(location: 33, length: 0))
        try await turn()
        XCTAssertEqual(spoken.last, "Line 4, column 4; Not implemented: \\mathbb is not supported in math mode")
        XCTAssertEqual(tv.accessibilityCustomContent.map(\.label), ["Not implemented", "Recovery"])

        // A selection announces its extent as before (the marks are in the rotor and the content).
        tv.setSelectedRange(NSRange(location: 9, length: 4))
        try await turn()
        XCTAssertEqual(spoken.last, "Selected 4 characters, line 2 column 1 to 5")
        XCTAssertEqual(tv.accessibilityCustomContent.map(\.label), ["Error", "Recovery"])

        // The Diagnostics rotor of the hosted view reads the same marks.
        let rotors = tv.accessibilityCustomRotors()
        let rotor = try XCTUnwrap(rotors.first { $0.label == "Diagnostics" })
        let params = NSAccessibilityCustomRotor.SearchParameters()
        params.searchDirection = .next
        params.filterString = ""
        let first = try XCTUnwrap(tv.rotorSearch.rotor(rotor, resultFor: params))
        XCTAssertEqual(first.targetRange, NSRange(location: 9, length: 4))
        XCTAssertEqual(first.customLabel, "Error at line 2: Undefined control sequence \\foo — recovery: ignored")
        params.currentItem = first
        let second = try XCTUnwrap(tv.rotorSearch.rotor(rotor, resultFor: params))
        XCTAssertEqual(second.targetRange, NSRange(location: 30, length: 7))
        XCTAssertEqual(second.customLabel, "Not implemented at line 4: \\mathbb is not supported in math mode")
        params.currentItem = second
        XCTAssertNil(tv.rotorSearch.rotor(rotor, resultFor: params))
    }
}
