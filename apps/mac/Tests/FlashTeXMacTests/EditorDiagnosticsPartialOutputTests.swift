import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXAccessibility
@testable import FlashTeXMac

/// The retention and grouping rules of partial output, on hand-made results
/// (EditorDiagnostics.swift). The live-compiler half of this suite drove the
/// old engine's `flashtex-compiler` and retired with its route (#1236 S6).
final class EditorDiagnosticsPartialOutputTests: XCTestCase {
    /// The retention rule without a compiler: a failed result with pages
    /// does not keep (it has its own output), retention never chains.
    func testRetentionRuleIsFailedWithNoPagesOnly() {
        func result(_ status: RuntimeV1.Status, pages: Int, revision: Int) -> RuntimeV1.CompileResult {
            .init(projectId: "p", revision: revision, status: status,
                  pages: (0..<pages).map { .init(number: $0 + 1, widthPt: 100, heightPt: 100, items: []) }, diagnostics: [], pdfPath: nil)
        }
        XCTAssertTrue(EditorDiagnostics.keepsPreviousMarks(result(.failed, pages: 0, revision: 2)))
        XCTAssertFalse(EditorDiagnostics.keepsPreviousMarks(result(.failed, pages: 1, revision: 2)))
        XCTAssertFalse(EditorDiagnostics.keepsPreviousMarks(result(.recovered, pages: 0, revision: 2)))
        XCTAssertFalse(EditorDiagnostics.keepsPreviousMarks(result(.ok, pages: 0, revision: 2)))
        // A first result that is itself a failure retains nothing; the report is the plain one.
        XCTAssertNil(EditorDiagnostics.retained(after: result(.failed, pages: 0, revision: 1), resultID: "a", compiledDocuments: [:], previous: nil))
        let plain = EditorDiagnostics.report(for: result(.failed, pages: 0, revision: 1), resultID: "a", retained: nil, path: "main.tex",
                                             compiledText: nil, currentText: "x")
        XCTAssertEqual(plain, .empty)
        XCTAssertEqual(EditorDiagnostics.Report.empty.staleNote, nil)
    }

    /// Grouping without a compiler: severity separates groups with one
    /// message, unsourced occurrences sort last, mixed recoveries drop the
    /// shared note, documents follow the project order.
    func testGroupingRulesAcrossSeverityDocumentsAndUnsourced() {
        let diags: [RuntimeV1.Diagnostic] = [
            .init(severity: .warning, message: "m", source: .init(path: "chapter.tex", startByte: 5, endByte: 6), recovery: "a"),
            .init(severity: .error, message: "m", source: .init(path: "main.tex", startByte: 9, endByte: 10), recovery: nil),
            .init(severity: .warning, message: "m", source: nil, recovery: "b"),
            .init(severity: .warning, message: "m", source: .init(path: "main.tex", startByte: 2, endByte: 3), recovery: "a"),
            .init(severity: .warning, message: "m", source: .init(path: "main.tex", startByte: 0, endByte: 1), recovery: "a"),
            .init(severity: .error, message: "z", source: .init(path: "main.tex", startByte: 1, endByte: 2), recovery: nil),
        ]
        let result = RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .recovered, pages: [], diagnostics: diags, pdfPath: nil)
        let groups = EditorDiagnostics.groups(of: result, documentOrder: ["main.tex", "chapter.tex"])
        XCTAssertEqual(groups.map(\.id), ["error:z", "error:m", "warning:m"])
        let warnings = groups[2]
        XCTAssertEqual(warnings.occurrences, [4, 3, 0, 2], "main.tex by start, then chapter.tex, unsourced last")
        XCTAssertNil(warnings.recovery, "mixed recovery notes: none shared")
        XCTAssertEqual(warnings.title, "4× m")
        XCTAssertEqual(groups[1].title, "m")
        XCTAssertEqual(EditorDiagnostics.occurrenceLabel(3, of: warnings, in: result), "4 of 4: no source")
        XCTAssertEqual(EditorDiagnostics.occurrenceLabel(2, of: warnings, in: result, texts: ["chapter.tex": "ab\ncd\nef"]), "3 of 4: chapter.tex line 2")
        XCTAssertEqual(EditorDiagnostics.lineNumber(ofByte: 99, in: "ab"), nil)
        XCTAssertEqual(EditorDiagnostics.lineNumber(ofByte: 2, in: "ab"), 1)
        XCTAssertEqual(EditorDiagnostics.lineNumber(ofByte: 3, in: "ab\n"), 2)
        XCTAssertEqual(EditorDiagnostics.groups(of: .init(projectId: "p", revision: 1, status: .ok, pages: [], diagnostics: [], pdfPath: nil)), [])
    }
}
