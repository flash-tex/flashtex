import XCTest
import CoreGraphics
import PDFKit
import FlashTeXProtocol
@testable import FlashTeXMac

/// File > Print… prints the bytes File > Export PDF… writes: the engine
/// host's export run (ExactPDFExport.swift, PrintController.swift). The
/// end-to-end case, which needs a built `flashtex-host`, is
/// EngineV3ExportTests.testPrintUsesTheExportedPDF; the cases here (building
/// an operation from bytes, the refusals, enablement and Print Source) need
/// no tool.
@MainActor
final class PrintControllerTests: XCTestCase {
    private static var repoRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url = url.deletingLastPathComponent() }
        return url
    }

    // MARK: - building an operation from finished bytes

    /// Two pages of different sizes: page count and the *first* page's size
    /// drive the print info, whatever produced the bytes.
    func testPrepareDocumentFromBytesKeepsPageCountAndFirstPagePaperSize() throws {
        let data = Self.twoPagePDF()
        let prepared = try XCTUnwrap(PrintController.prepareDocument(pdfData: data, jobTitle: "demo"))
        XCTAssertEqual(prepared.pageCount, 2)
        XCTAssertEqual(prepared.paperSize.width, 612, accuracy: 0.01)
        XCTAssertEqual(prepared.paperSize.height, 792, accuracy: 0.01)
        XCTAssertEqual(prepared.jobTitle, "demo")
        XCTAssertEqual(prepared.operation.jobTitle, "demo")
        XCTAssertEqual(prepared.operation.printInfo.paperSize.width, 612, accuracy: 0.01)
        XCTAssertEqual(try XCTUnwrap(PDFDocument(data: try XCTUnwrap(prepared.pdfData))).pageCount, 2)
        XCTAssertNil(prepared.text)
    }

    func testPrepareDocumentRefusesBytesThatAreNotAPDF() {
        XCTAssertNil(PrintController.prepareDocument(pdfData: Data("not a pdf".utf8), jobTitle: "demo"))
        XCTAssertNil(PrintController.prepareDocument(pdfData: Data(), jobTitle: "demo"))
    }

    // MARK: - refusals (no tool needed)

    /// With the engine's session off (as under XCTest) there is nothing to
    /// print: Print… and Export PDF… refuse in the same words.
    func testDisabledWithoutADisplayList() async {
        let model = ShellModel()
        model.flushChrome()
        XCTAssertFalse(model.exportAvailable)
        XCTAssertFalse(PrintController.documentEnabled(model))
        let why = model.exportPDFRefusal() ?? ""
        XCTAssertFalse(why.isEmpty, "Export PDF… refuses too")
        XCTAssertEqual(PrintController.documentHelp(model), "Nothing to print: " + why)
        XCTAssertFalse(PrintController.exportWouldProceed(model))
        guard case .refused(let printWhy) = await PrintController.makeDocumentPrint(from: model) else {
            return XCTFail("no engine must refuse, not build an operation")
        }
        XCTAssertEqual(printWhy, "Nothing to print: " + why)
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

    /// Exactly one export command is wired anywhere in the shell.
    func testOnlyOneExportRouteIsWired() throws {
        let app = try String(contentsOf: Self.repoRoot.appendingPathComponent("apps/mac/Sources/FlashTeXMac/FlashTeXMacApp.swift"), encoding: .utf8)
        XCTAssertEqual(app.components(separatedBy: "Button(\"Export PDF…\")").count - 1, 1)
        for gone in ["exportPDFViaRust", "exportPDFV2", "Export PDF via Rust Writer", "Export PDF (exact, v2)"] {
            XCTAssertFalse(app.contains(gone), "\(gone) is still wired in the File menu")
        }
        let titleBar = try String(contentsOf: Self.repoRoot.appendingPathComponent("apps/mac/Sources/FlashTeXMac/TitleBar.swift"), encoding: .utf8)
        for gone in ["exportPDFViaRust", "exportPDFV2", "exportPDFExact()"] {
            XCTAssertFalse(titleBar.contains(gone), "\(gone) is still wired in the title bar")
        }
    }

    // MARK: - Print Source

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
        model.flushChrome()
        XCTAssertFalse(model.toolbarHasDocument)
        XCTAssertFalse(PrintController.sourceEnabled(model))
        XCTAssertEqual(PrintController.sourceHelp(model),
                       "Nothing to print: no document is open.")
        guard case .refused(let why) = PrintController.makeSourcePrint(from: model) else {
            return XCTFail("no document must refuse Print Source")
        }
        XCTAssertTrue(why.contains("no document"), why)
    }

    // MARK: - helpers

    /// Letter then a small page, drawn with CoreGraphics — bytes only, so this
    /// test does not depend on any export route. Integral sizes: PDFKit reports
    /// a rounded media box for fractional ones.
    private static func twoPagePDF() -> Data {
        let data = NSMutableData()
        guard let consumer = CGDataConsumer(data: data), let ctx = CGContext(consumer: consumer, mediaBox: nil, nil) else { return Data() }
        for size in [CGSize(width: 612, height: 792), CGSize(width: 400, height: 300)] {
            var box = CGRect(origin: .zero, size: size)
            ctx.beginPDFPage([kCGPDFContextMediaBox as String: NSData(bytes: &box, length: MemoryLayout<CGRect>.size)] as CFDictionary)
            ctx.setFillColor(CGColor(gray: 0, alpha: 1))
            ctx.fill(CGRect(x: 10, y: 10, width: 20, height: 20))
            ctx.endPDFPage()
        }
        ctx.closePDF()
        return data as Data
    }
}
