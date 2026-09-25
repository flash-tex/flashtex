import XCTest
import FlashTeXProtocol
import FlashTeXAccessibility
@testable import FlashTeXMac

/// Accessibility audit for the diagnostics panel rows
/// (ux-diagnostics-panel-voiceover, slice 1): with VoiceOver on, each list
/// row must expose one accessibility label combining severity, message, and
/// line — never the raw message text alone. Pins the exact strings the row
/// modifier (`accessibleDiagnostic`, via `EditorDiagnostics.rowAccessibility`)
/// publishes, without hosting a window or taking keyboard focus.
@MainActor
final class DiagnosticsRowAccessibilityAuditTests: XCTestCase {
    static let text = "first\n\\bad here\nthird\n"

    static func error() -> RuntimeV1.Diagnostic {
        .init(severity: .error, message: "Undefined control sequence \\bad",
              source: .init(path: "main.tex", startByte: 6, endByte: 10), recovery: nil)
    }

    static func warning() -> RuntimeV1.Diagnostic {
        .init(severity: .warning, message: "Overfull hbox",
              source: .init(path: "main.tex", startByte: 16, endByte: 21), recovery: nil)
    }

    func testErrorRowLabelCombinesSeverityMessageAndLine() {
        let diags = [Self.error()]
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: ["main.tex"])
        XCTAssertEqual(groups.count, 1)
        let row = EditorDiagnostics.rowAccessibility(groups[0], occurrence: 0, in: diags, status: .ok,
                                                     texts: ["main.tex": Self.text])
        XCTAssertEqual(row.label, "Diagnostic 1 of 1: Error: Undefined control sequence \\bad, main.tex line 2")
    }

    func testWarningRowLabelCombinesSeverityMessageAndLine() {
        let diags = [Self.warning()]
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: ["main.tex"])
        XCTAssertEqual(groups.count, 1)
        let row = EditorDiagnostics.rowAccessibility(groups[0], occurrence: 0, in: diags, status: .ok,
                                                     texts: ["main.tex": Self.text])
        XCTAssertEqual(row.label, "Diagnostic 1 of 1: Warning: Overfull hbox, main.tex line 3")
    }

    func testGroupedRowLabelCombinesSeverityMessageAndLine() {
        let diags = [Self.error(), Self.error()]
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: ["main.tex"])
        XCTAssertEqual(groups.count, 1, "identical diagnostics fold into one row")
        let row = EditorDiagnostics.rowAccessibility(groups[0], occurrence: 0, in: diags, status: .ok,
                                                     texts: ["main.tex": Self.text])
        XCTAssertEqual(row.label, "Diagnostic 1 of 2: Error: Undefined control sequence \\bad, 2 places, 1 of 2, main.tex line 2")
    }
}
