import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Accessibility audit for the inline error popover under a squiggled
/// command (ux-editor-diagnostics-voiceover, slice 1): the popover was
/// discoverable only by sighted mouse hover — no accessibility label, no
/// hint, no keyboard path. `EditorDiagnostics.popoverAccessibility(for:)`
/// vends the label/hint a popover view binds to
/// `accessibilityLabel`/`accessibilityHint`, and `popover(at:in:)` is the
/// caret query a keyboard "show diagnostics here" command opens with.
///
/// These tests pin the pure contract in `EditorDiagnostics.swift`. They do
/// not bind AppKit views: presenting the popover from the keyboard and
/// setting the strings on `QuickInfoView` (`EditorIntelligence.swift`) is
/// the UI half, outside this slice.
final class EditorDiagnosticsPopoverAccessibilityTests: XCTestCase {
    static let text = "line one\n\\foo here\nline three\n"

    /// An error (`\foo`, bytes 9..<13) and a gap (`\mathbb`-style wording,
    /// bytes 22..<26) plus an unsourced warning that has no popover.
    static func marks() -> [EditorDiagnostics.Mark] {
        let result = RuntimeV1.CompileResult(
            projectId: "p", revision: 1, status: .recovered, pages: [],
            diagnostics: [
                .init(severity: .error, message: "Undefined control sequence \\foo",
                      source: .init(path: "main.tex", startByte: 9, endByte: 13), recovery: "ignored"),
                .init(severity: .warning, message: "\\bar is not implemented",
                      source: .init(path: "main.tex", startByte: 22, endByte: 26), recovery: nil),
                .init(severity: .warning, message: "Overfull line", source: nil, recovery: nil),
            ],
            pdfPath: nil)
        return EditorDiagnostics.marks(for: result, resultID: "r1", path: "main.tex",
                                       compiledText: text, currentText: text)
    }

    // MARK: label

    /// The label speaks severity, message, and the recovery/explanation
    /// lines — the same content `toolTip` shows a sighted reader on hover.
    func testPopoverLabelSpeaksWhatHoverShows() {
        let marks = Self.marks()
        XCTAssertEqual(marks.count, 2, "the unsourced warning has no popover")
        let error = marks.first { $0.severity == .error }!
        XCTAssertEqual(error.popoverAccessibilityLabel,
                       "Error: Undefined control sequence \\foo — recovery: ignored")
        XCTAssertTrue(error.popoverAccessibilityLabel.contains(error.message))
        XCTAssertTrue(error.toolTip.contains(error.message),
                      "the keyboard label and the hover tooltip describe the same diagnostic")

        var explained = error
        explained.explanation = "explain: Unknown command — check the spelling"
        XCTAssertEqual(explained.popoverAccessibilityLabel,
                       "Error: Undefined control sequence \\foo — recovery: ignored — explain: Unknown command — check the spelling")
    }

    /// A FlashTeX gap is spoken as "Not implemented" by the panel's rule —
    /// never as a bare warning — matching the caret announcement and rotor.
    func testPopoverLabelNamesGapsLikeThePanel() {
        let error = Self.marks().first { $0.severity == .error }!
        let gap = Self.marks().first { $0.isGap }!
        XCTAssertEqual(EditorDiagnostics.popoverSeverityWord(for: error), "Error")
        XCTAssertEqual(EditorDiagnostics.popoverSeverityWord(for: gap), "Not implemented")
        XCTAssertEqual(gap.popoverAccessibilityLabel,
                       "Not implemented: \\bar is not implemented — no provisional rendering")
        XCTAssertEqual(EditorDiagnostics.popoverAccessibility(for: gap, ordinal: 2, total: 2).label,
                       "Not implemented 2 of 2: \\bar is not implemented — no provisional rendering")
    }

    // MARK: hint

    /// Every popover hints identically, and the hint names a keyboard path:
    /// the caret reaches the underline, and ⌘⇧]/⌘⇧[ step through every
    /// diagnostic (`Navigation.goToDiagnostic`).
    func testPopoverHintNamesTheKeyboardPath() {
        XCTAssertEqual(EditorDiagnostics.popoverHint,
                       "Move the insertion point onto the underlined text to hear this message. "
                           + "Step through every diagnostic with ⌘⇧] and ⌘⇧[; the Problems list shows the same message.")
        for mark in Self.marks() {
            XCTAssertEqual(mark.popoverAccessibilityHint, EditorDiagnostics.popoverHint,
                           "one shared hint, so every popover hints identically")
            XCTAssertFalse(mark.popoverAccessibilityHint.isEmpty)
        }
    }

    // MARK: keyboard reachability

    /// The caret opens the popover: at either end of the range inclusive
    /// (a caret just after the underline counts), with its "n of m"
    /// position; off every mark there is no popover to open.
    func testPopoverAtCaretIsKeyboardReachable() {
        let marks = Self.marks()
        let error = marks.first { $0.severity == .error }!
        let gap = marks.first { $0.isGap }!

        XCTAssertNil(EditorDiagnostics.popover(at: 8, in: marks), "just before the underline: no popover")
        for caret in [error.nsRange.location, NSMaxRange(error.nsRange)] {
            let found = try? XCTUnwrap(EditorDiagnostics.popover(at: caret, in: marks))
            XCTAssertEqual(found?.mark, error)
            XCTAssertEqual(found?.accessibility.label, error.popoverAccessibilityLabel.replacingOccurrences(
                of: "Error:", with: "Error 1 of 2:"))
        }
        let second = try? XCTUnwrap(EditorDiagnostics.popover(at: gap.nsRange.location, in: marks))
        XCTAssertEqual(second?.mark, gap)
        XCTAssertEqual(second?.accessibility.label,
                       "Not implemented 2 of 2: \\bar is not implemented — no provisional rendering")
        XCTAssertEqual(second?.accessibility.hint, EditorDiagnostics.popoverHint)
        XCTAssertNil(EditorDiagnostics.popover(at: 0, in: marks))
        XCTAssertNil(EditorDiagnostics.popover(at: 0, in: []), "no marks: no popover")
    }

    /// Marks sharing a start offset both stay reachable: the caret query
    /// returns the first in document order (⌘⇧]/⌘⇧[ stepping reaches each
    /// one in turn via `currentID`).
    func testPopoverAtCaretPicksDocumentOrderForOverlaps() {
        let marks = Self.marks()
        let error = marks.first { $0.severity == .error }!
        let inner = EditorDiagnostics.Mark(
            identity: error.identity, nsRange: NSRange(location: error.nsRange.location, length: 1),
            severity: .error, message: "inner", recovery: nil, resultStatus: .recovered)
        let found = EditorDiagnostics.popover(at: error.nsRange.location, in: [error, inner])
        XCTAssertEqual(found?.mark, error, "same start offset: the first in document order wins")
        XCTAssertEqual(found?.accessibility.label,
                       "Error 1 of 2: Undefined control sequence \\foo — recovery: ignored")
    }
}
