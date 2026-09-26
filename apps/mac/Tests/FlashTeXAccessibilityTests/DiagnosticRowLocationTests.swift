import XCTest
import FlashTeXProtocol
@testable import FlashTeXAccessibility

/// A single-place diagnostics row speaks where it is after the message
/// ("…, main.tex line 41") and a FlashTeX gap speaks "Not implemented"
/// instead of the severity word — what the visible row conveys only by the
/// dimmed trailing text and the grey puzzle glyph (ux-diagnostics-panel-voiceover).
final class DiagnosticRowLocationTests: XCTestCase {
    let error = RuntimeV1.Diagnostic(severity: .error, message: "Undefined control sequence \\foo",
                                     source: .init(path: "main.tex", startByte: 10, endByte: 14), recovery: "ignored")
    let gap = RuntimeV1.Diagnostic(severity: .warning, message: "\\mathbb is not supported in math mode",
                                   source: .init(path: "main.tex", startByte: 30, endByte: 37), recovery: nil)

    func testLocationIsSpokenAfterTheMessage() {
        let row = DiagnosticRowAccessibility(error, index: 1, total: 3, status: .recovered, location: "main.tex line 2")
        XCTAssertEqual(row.label, "Diagnostic 2 of 3: Error: Undefined control sequence \\foo, main.tex line 2")
        XCTAssertEqual(row.value, "recovery: ignored; main.tex bytes 10 to 14")
        XCTAssertNil(row.hint)
        XCTAssertEqual(row.actions, ["Go to source"])
        // Without a location the label is exactly what it was.
        XCTAssertEqual(DiagnosticRowAccessibility(error, index: 1, total: 3, status: .recovered).label,
                       "Diagnostic 2 of 3: Error: Undefined control sequence \\foo")
    }

    func testGroupSpeaksItsOwnPlaceInsteadOfTheLocation() {
        let row = DiagnosticRowAccessibility(error, index: 0, total: 3, status: .ok,
                                             group: .init(count: 2, occurrence: 1, location: "main.tex line 5"),
                                             location: "main.tex line 2")
        XCTAssertEqual(row.label, "Diagnostic 1 of 3: Error: Undefined control sequence \\foo, 2 places, 2 of 2, main.tex line 5")
        // A group of one falls back to the plain location.
        let single = DiagnosticRowAccessibility(error, index: 0, total: 3, status: .ok,
                                                group: .init(count: 1, occurrence: 0, location: "main.tex line 2"),
                                                location: "main.tex line 2")
        XCTAssertEqual(single.label, "Diagnostic 1 of 3: Error: Undefined control sequence \\foo, main.tex line 2")
    }

    func testGapSpeaksNotImplementedInsteadOfTheSeverity() {
        XCTAssertEqual(DiagnosticRowAccessibility.gapWord, "Not implemented")
        let row = DiagnosticRowAccessibility(gap, index: 2, total: 3, status: .recovered, location: "main.tex line 4", gap: true)
        XCTAssertEqual(row.label, "Diagnostic 3 of 3: Not implemented: \\mathbb is not supported in math mode, main.tex line 4")
        XCTAssertEqual(row.value, "no provisional rendering; main.tex bytes 30 to 37")
        // The flag is the shell's decision; the same diagnostic without it speaks its severity.
        XCTAssertEqual(DiagnosticRowAccessibility(gap, index: 2, total: 3, status: .recovered, location: "main.tex line 4").label,
                       "Diagnostic 3 of 3: Warning: \\mathbb is not supported in math mode, main.tex line 4")
    }

    func testUnsourcedRowKeepsItsHintAndNoLocation() {
        let unsourced = RuntimeV1.Diagnostic(severity: .warning, message: "Overfull line", source: nil, recovery: nil)
        let row = DiagnosticRowAccessibility(unsourced, index: 0, total: 1, status: .ok)
        XCTAssertEqual(row.label, "Diagnostic 1 of 1: Warning: Overfull line")
        XCTAssertEqual(row.hint, DiagnosticRowAccessibility.noSourceHint)
        XCTAssertEqual(row.actions, [])
    }
}
