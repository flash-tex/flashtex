import XCTest
import PDFKit
import FlashTeXProtocol
@testable import FlashTeXMac

@MainActor
final class PrintControllerTests: XCTestCase {
    private static var repoRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url = url.deletingLastPathComponent() }
        return url
    }

    private func loadFixtureResult() throws -> RuntimeV1.CompileResult {
        let url = Self.repoRoot.appendingPathComponent("protocol/fixtures/compile-result.json")
        return try RuntimeV1.decodeCompileResult(Data(contentsOf: url)).payload
    }

    func testDocumentPrintMatchesExportPDFPageCountSizeAndJobTitle() throws {
        let result = try loadFixtureResult()
        let exportDoc = try XCTUnwrap(PDFDocument(data: PDFExport.render(result, dark: false)))
        let prepared = try XCTUnwrap(PrintController.prepareDocument(result: result, jobTitle: "main.tex"))
        let printed = try XCTUnwrap(PDFDocument(data: try XCTUnwrap(prepared.pdfData)))
        // CoreGraphics PDFs are not byte-identical across renders (IDs / dates);
        // Print must still be a PDFExport.render of the same result.
        XCTAssertEqual(printed.pageCount, exportDoc.pageCount)
        XCTAssertEqual(printed.pageCount, result.pages.count)
        XCTAssertEqual(printed.pageCount, 1)
        XCTAssertEqual(printed.page(at: 0)?.string, exportDoc.page(at: 0)?.string)
        XCTAssertTrue(printed.page(at: 0)?.string?.contains("Hello FlashTeX.") == true)
        XCTAssertEqual(prepared.pageCount, result.pages.count)
        XCTAssertEqual(prepared.paperSize.width, 612, accuracy: 0.01)
        XCTAssertEqual(prepared.paperSize.height, 792, accuracy: 0.01)
        XCTAssertEqual(prepared.jobTitle, "main.tex")
        XCTAssertEqual(prepared.operation.jobTitle, "main.tex")
        XCTAssertEqual(prepared.operation.printInfo.paperSize.width, 612, accuracy: 0.01)
        XCTAssertEqual(prepared.operation.printInfo.paperSize.height, 792, accuracy: 0.01)
        XCTAssertFalse(prepared.operation.showsPrintPanel, "tests must not present the system panel")
        XCTAssertNil(prepared.text)
    }

    func testTwoPageResultKeepsPageCountAndFirstPagePaperSize() throws {
        let json = """
        {"protocol_version":1,"id":"t","type":"compile_result","payload":{
          "project_id":"demo","revision":3,"status":"ok","pages":[
            {"number":1,"width_pt":595.276,"height_pt":841.89,"items":[
              {"kind":"text","text":"First page","x_pt":72,"baseline_y_pt":100,"font_size_pt":14,"source":null}]},
            {"number":2,"width_pt":400,"height_pt":300,"items":[
              {"kind":"text","text":"Second page","x_pt":10,"baseline_y_pt":50,"font_size_pt":10,"source":null}]}
          ],"diagnostics":[],"pdf_path":null}}
        """
        let result = try RuntimeV1.decodeCompileResult(Data(json.utf8)).payload
        let prepared = try XCTUnwrap(PrintController.prepareDocument(result: result, jobTitle: "demo"))
        let printed = try XCTUnwrap(PDFDocument(data: try XCTUnwrap(prepared.pdfData)))
        let exported = try XCTUnwrap(PDFDocument(data: PDFExport.render(result, dark: false)))
        XCTAssertEqual(printed.pageCount, exported.pageCount)
        XCTAssertEqual(printed.pageCount, 2)
        XCTAssertEqual(printed.page(at: 0)?.string, exported.page(at: 0)?.string)
        XCTAssertTrue(printed.page(at: 0)?.string?.contains("First page") == true)
        XCTAssertTrue(printed.page(at: 1)?.string?.contains("Second page") == true)
        XCTAssertEqual(prepared.pageCount, 2)
        XCTAssertEqual(prepared.paperSize.width, 595.276, accuracy: 0.01)
        XCTAssertEqual(prepared.paperSize.height, 841.89, accuracy: 0.01)
        XCTAssertEqual(prepared.jobTitle, "demo")
    }

    func testDisabledWithoutPDFAndWithoutDocument() {
        let model = ShellModel()
        model.result = nil
        XCTAssertFalse(model.toolbarHasResult)
        XCTAssertFalse(PrintController.documentEnabled(model))
        XCTAssertEqual(PrintController.documentHelp(model),
                       "Nothing to print: no compiled PDF (compile the document first).")
        guard case .refused(let why) = PrintController.makeDocumentPrint(from: model) else {
            return XCTFail("no PDF must refuse, not build an operation")
        }
        XCTAssertTrue(why.contains("no compile result"), why)
    }

    func testFailedResultIsNotPrintable() {
        let model = ShellModel()
        XCTAssertTrue(PrintController.documentEnabled(model), "fixture result is printable")
        model.result = RuntimeV1.CompileResult(projectId: "demo", revision: 1, status: .failed,
                                               pages: [], diagnostics: [], pdfPath: nil)
        XCTAssertEqual(model.resultStatus, .failed)
        XCTAssertEqual(model.toolbarPageCount, 0)
        XCTAssertFalse(PrintController.documentEnabled(model))
        XCTAssertEqual(PrintController.documentHelp(model), "Compile failed — nothing to print")
        guard case .refused(let why) = PrintController.makeDocumentPrint(from: model) else {
            return XCTFail("a failed result must not build a print operation")
        }
        XCTAssertEqual(why, "Compile failed — nothing to print")
        XCTAssertTrue(PrintController.exportWouldProceed(model),
                      "Export PDF… still uses the current result; Print is the stricter command")
    }

    func testEmptyPagesAreNotPrintable() {
        let model = ShellModel()
        model.result = RuntimeV1.CompileResult(projectId: "demo", revision: 1, status: .ok,
                                               pages: [], diagnostics: [], pdfPath: nil)
        XCTAssertEqual(model.resultStatus, .ok)
        XCTAssertEqual(model.toolbarPageCount, 0)
        XCTAssertFalse(PrintController.documentEnabled(model))
        XCTAssertEqual(PrintController.documentHelp(model), "Compile failed — nothing to print")
        guard case .refused(let why) = PrintController.makeDocumentPrint(from: model) else {
            return XCTFail("empty pages must not print a blank PDF")
        }
        XCTAssertEqual(why, "Compile failed — nothing to print")
    }

    func testFileMenuEnablementUsesDocumentEnabledAndSourceEnabled() throws {
        // The File menu must call these functions (not a parallel helper or a
        // hard-coded hasDocument: true). CommandTableTests already checks
        // .disabled() vs requires; this pins the callee.
        let url = Self.repoRoot.appendingPathComponent("apps/mac/Sources/FlashTeXMac/FlashTeXMacApp.swift")
        let text = try String(contentsOf: url, encoding: .utf8)
        XCTAssertTrue(text.contains("PrintController.documentEnabled(model)"))
        XCTAssertTrue(text.contains("PrintController.documentHelp(model)"))
        XCTAssertTrue(text.contains("PrintController.sourceEnabled(model)"))
        XCTAssertTrue(text.contains("PrintController.sourceHelp(model)"))
        XCTAssertFalse(text.contains("sourceHelp(hasDocument: true)"))
        XCTAssertTrue(text.contains("CommandGroup(replacing: .printItem)"), "Print… and Print Source stay in the File menu")
    }

    func testStaleCompilePrintsLastResultLikeExportPDF() throws {
        let model = ShellModel()
        XCTAssertNotNil(model.result)
        XCTAssertTrue(PrintController.exportWouldProceed(model))
        XCTAssertFalse(model.previewIsStale)
        let last = PDFExport.render(model.result!, dark: false)

        model.updateActiveText(model.activeText + "% edited after compile\n")
        XCTAssertTrue(model.previewIsStale, "editor is ahead of the last compile")
        XCTAssertTrue(model.toolbarHasResult, "Export PDF… stays enabled on a stale preview")
        XCTAssertTrue(PrintController.exportWouldProceed(model), "Export PDF… still uses the last result")
        XCTAssertTrue(PrintController.documentEnabled(model))
        XCTAssertTrue(PrintController.documentHelp(model).contains("⌘P"))

        guard case .ready(let prepared) = PrintController.makeDocumentPrint(from: model) else {
            return XCTFail("stale editor must print the last compiled PDF, not refuse")
        }
        let printed = try XCTUnwrap(PDFDocument(data: try XCTUnwrap(prepared.pdfData)))
        let lastDoc = try XCTUnwrap(PDFDocument(data: last))
        let currentExport = try XCTUnwrap(PDFDocument(data: PDFExport.render(model.result!, dark: false)))
        XCTAssertEqual(printed.pageCount, lastDoc.pageCount)
        XCTAssertEqual(printed.page(at: 0)?.string, lastDoc.page(at: 0)?.string)
        XCTAssertEqual(printed.page(at: 0)?.string, currentExport.page(at: 0)?.string)
        XCTAssertEqual(prepared.jobTitle, PrintController.documentName(from: model))
        XCTAssertEqual(prepared.jobTitle, "main.tex")
    }

    func testHistoricalPreviewRefusesLikeExport() {
        let model = ShellModel()
        XCTAssertTrue(PrintController.exportWouldProceed(model))
        model.historicalPreview = HistoricalDisplay(shownEditorRevision: 1, compilingEditorRevision: 2,
                                                    shownCompileRevision: 1, currentCompileRevision: 2,
                                                    requestID: "preview-1", resultID: "r1")
        XCTAssertFalse(PrintController.exportWouldProceed(model))
        XCTAssertNotNil(model.historicalRefusal(of: "export"))
        guard case .refused(let why) = PrintController.makeDocumentPrint(from: model) else {
            return XCTFail("historical preview must not print the older snapshot as current")
        }
        XCTAssertTrue(why.contains("unavailable"), why)
        XCTAssertTrue(why.contains("historical revision 1"), why)
    }

    func testPrintSourceTextEqualsTheBufferAndUsesACopy() {
        let buffer = "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n"
        let model = ShellModel()
        model.replaceProject(entryText: buffer)
        XCTAssertEqual(model.activeText, buffer)
        XCTAssertTrue(PrintController.sourceEnabled(model))
        XCTAssertTrue(PrintController.sourceHelp(model).contains("line numbers"))

        guard case .ready(let prepared) = PrintController.makeSourcePrint(from: model) else {
            return XCTFail("an open buffer must build a Print Source operation")
        }
        XCTAssertEqual(prepared.text, buffer)
        XCTAssertEqual(prepared.jobTitle, "main.tex")
        XCTAssertEqual(prepared.operation.jobTitle, "main.tex")
        XCTAssertFalse(prepared.operation.showsPrintPanel)
        XCTAssertNil(prepared.pdfData)
        XCTAssertGreaterThanOrEqual(prepared.pageCount, 1)

        model.documents = []
        XCTAssertFalse(model.toolbarHasDocument)
        XCTAssertFalse(PrintController.sourceEnabled(model))
        XCTAssertEqual(PrintController.sourceHelp(model),
                       "Nothing to print: no document is open.")
        guard case .refused(let why) = PrintController.makeSourcePrint(from: model) else {
            return XCTFail("no document must refuse Print Source")
        }
        XCTAssertTrue(why.contains("no document"), why)
    }
}
