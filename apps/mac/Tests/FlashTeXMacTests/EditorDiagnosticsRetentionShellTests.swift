import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// The shell hooks of ShellModel+DiagnosticRetention.swift (applied branch):
/// after a real partial-output result (the generated fixture of
/// EditorDiagnosticsPartialOutputTests, ~115 diagnostics), a real failed
/// result with no output keeps the underlines flagged, keyboard navigation
/// still walks them and says so, an edit rebases them, and the next result
/// with output replaces them.
final class EditorDiagnosticsRetentionShellTests: XCTestCase {
    @MainActor
    func testFailedResultKeepsMarksInTheShellAndNavigationAnnouncesIt() throws {
        guard let path = ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"],
              FileManager.default.isExecutableFile(atPath: path) else {
            throw XCTSkip("set FLASHTEX_COMPILER to run the shell retention test against the live compiler")
        }
        let compiler = URL(fileURLWithPath: path)
        let text = EditorDiagnosticsPartialOutputTests.fixtureText
        let good = try EditorDiagnosticsPartialOutputTests.compile([.init(path: "main.tex", text: text)], revision: 1, id: "g-1", with: compiler)
        let failed = try EditorDiagnosticsPartialOutputTests.compile([], entry: "", revision: 2, id: "f-2", with: compiler)
        XCTAssertEqual(good.payload.status, .recovered)
        XCTAssertEqual(failed.payload.status, .failed)

        let model = ShellModel()
        model.replaceProject(entryText: text)
        model.result = good.payload; model.resultID = good.id
        model.setCompiledDocuments(["main.tex": text])
        model.retainMarksAfterResultBound()
        let n = good.payload.diagnostics.count
        XCTAssertEqual(n, EditorDiagnosticsPartialOutputTests.PartialOutputFixture.diagnostics)
        XCTAssertEqual(model.editorMarks.count, n)
        XCTAssertNil(model.editorMarkReport.carried)
        XCTAssertEqual(model.retainedMarks?.result.revision, 1)

        // The failure binds as the shell binds every result; the marks stay.
        model.result = failed.payload; model.resultID = failed.id
        model.setCompiledDocuments([:])
        model.retainMarksAfterResultBound()
        XCTAssertEqual(model.retainedMarks?.result.revision, 1, "a failure never replaces the retained result")
        let report = model.editorMarkReport
        XCTAssertEqual(report.marks.count, n, "kept, not cleared")
        guard report.marks.count == n, n > 0 else { return XCTFail("expected \(n) (>0) marks, got \(report.marks.count)") }
        XCTAssertEqual(Set(report.marks.map(\.id)).count, n, "not duplicated")
        XCTAssertEqual(report.carried, .init(revision: 1, failedRevision: 2))
        XCTAssertEqual(report.staleNote, "\(n) underlines kept from revision 1: revision 2 failed with no output")
        XCTAssertEqual(model.displayedDiagnostics.count, 1, "the list shows the failed result's own diagnostic")

        model.caretUTF16 = 0
        model.currentDiagnosticID = nil
        model.goToDiagnostic(forward: true)
        XCTAssertEqual((model.activeText as NSString).substring(with: model.selection!.nsRange), EditorDiagnosticsPartialOutputTests.PartialOutputFixture.firstMark)
        XCTAssertTrue(model.navigationNote?.hasPrefix("Warning 1 of \(n), line 4: ") == true, model.navigationNote ?? "nil")
        XCTAssertTrue(model.navigationNote?.hasSuffix("; \(n) underlines kept from revision 1: revision 2 failed with no output") == true, model.navigationNote ?? "nil")

        // An edit while the failure stands: rebased from the retained text.
        model.updateActiveText("% edited\n" + text)
        let edited = model.editorMarkReport
        XCTAssertEqual(edited.marks.count, n)
        guard edited.marks.count == n else { return XCTFail("expected \(n) marks, got \(edited.marks.count)") }
        XCTAssertEqual(edited.marks[0].nsRange.location, report.marks[0].nsRange.location + "% edited\n".utf16.count)
        XCTAssertEqual(edited.carried, report.carried)

        // A second failure keeps the same marks once; a new good result replaces them.
        let failed3 = try EditorDiagnosticsPartialOutputTests.compile([], entry: "", revision: 3, id: "f-3", with: compiler)
        model.result = failed3.payload; model.resultID = failed3.id
        model.retainMarksAfterResultBound()
        XCTAssertEqual(model.editorMarks.count, n)
        XCTAssertEqual(model.editorMarkReport.carried, .init(revision: 1, failedRevision: 3))
        let again = try EditorDiagnosticsPartialOutputTests.compile([.init(path: "main.tex", text: model.activeText)], revision: 4, id: "g-4", with: compiler)
        model.result = again.payload; model.resultID = again.id
        model.setCompiledDocuments(["main.tex": model.activeText])
        model.retainMarksAfterResultBound()
        XCTAssertEqual(model.retainedMarks?.result.revision, 4)
        XCTAssertNil(model.editorMarkReport.carried)
        XCTAssertEqual(model.editorMarks.count, n)
        XCTAssertTrue(model.editorMarks.allSatisfy { $0.carried == nil && $0.identity.resultID == "g-4" })

        // File > Open forgets the retained result.
        model.replaceProject(entryText: "plain\n")
        XCTAssertNil(model.retainedMarks)
        XCTAssertEqual(model.editorMarks, [])
    }
}
