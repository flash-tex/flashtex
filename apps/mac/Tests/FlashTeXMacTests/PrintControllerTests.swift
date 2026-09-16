import XCTest
import Darwin
import CoreGraphics
import PDFKit
import FlashTeXProtocol
@testable import FlashTeXMac

/// File > Print… prints the bytes File > Export PDF… writes: the loaded v2
/// display list through `flashtex-pdf-exact`. The end-to-end cases need that
/// tool (`FLASHTEX_PDF_EXACT`); the refusal, enablement and Print Source cases
/// do not.
@MainActor
final class PrintControllerTests: XCTestCase {
    private static var repoRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url = url.deletingLastPathComponent() }
        return url
    }
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static var tool: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_PDF_EXACT"].map { URL(fileURLWithPath: $0) }
            .flatMap { FileManager.default.isExecutableFile(atPath: $0.path) ? $0 : nil }
    }

    override func setUp() {
        super.setUp()
        setenv("FLASHTEX_FONT_DIRS", Self.repoRoot.appendingPathComponent("apps/mac/Fonts").path, 1)
    }

    override func tearDown() {
        unsetenv("FLASHTEX_FONT_DIRS")
        super.tearDown()
    }

    /// A model showing `fixture` as its verified v2 frame.
    private func modelShowing(_ fixture: String) async throws -> ShellModel {
        let model = ShellModel()
        let list = Self.fixtures.appendingPathComponent(fixture)
        await withCheckedContinuation { cont in model.loadDisplayListV2(url: list) { cont.resume() } }
        guard case .loaded = model.displayListV2 else {
            throw XCTSkip("fixture \(fixture) did not load: \(String(describing: model.displayListV2))")
        }
        model.flushChrome()
        return model
    }

    // MARK: - the printed bytes are the exported bytes

    func testDocumentPrintIsTheExactExportOfTheSameDisplayList() async throws {
        guard let tool = Self.tool else { throw XCTSkip("set FLASHTEX_PDF_EXACT to a built flashtex-pdf-exact") }
        let model = try await modelShowing("display-list-v2-text.json")
        let frame = try XCTUnwrap(model.displayListV2?.frame)
        XCTAssertTrue(model.toolbarExportable)
        XCTAssertTrue(PrintController.documentEnabled(model))
        XCTAssertTrue(PrintController.exportWouldProceed(model))
        XCTAssertTrue(PrintController.documentHelp(model).contains("⌘P"))

        guard case .ready(let prepared) = await PrintController.makeDocumentPrint(from: model) else {
            return XCTFail("a complete display list plus the tool must build a print operation")
        }
        let printed = try XCTUnwrap(PDFDocument(data: try XCTUnwrap(prepared.pdfData)))
        XCTAssertEqual(printed.pageCount, frame.list.pages.count)
        XCTAssertEqual(prepared.pageCount, printed.pageCount)
        XCTAssertNil(prepared.text, "the document print never carries editor text")
        XCTAssertFalse(prepared.operation.showsPrintPanel, "tests must not present the system panel")
        XCTAssertEqual(prepared.jobTitle, PrintController.documentName(from: model))
        XCTAssertEqual(prepared.operation.jobTitle, prepared.jobTitle)
        XCTAssertEqual(prepared.operation.printInfo.paperSize.width, prepared.paperSize.width, accuracy: 0.01)

        // The same bytes Export PDF… writes: a direct run of the tool on the
        // same list produces the same page count, sizes and text.
        let out = FileManager.default.temporaryDirectory.appendingPathComponent("print-parity-\(UUID().uuidString).pdf")
        defer { try? FileManager.default.removeItem(at: out) }
        let outcome = try ExactPDFExport.run(tool: tool, list: Self.fixtures.appendingPathComponent("display-list-v2-text.json"),
                                             out: out, fontDirs: ExactPDFExport.fontDirectories())
        XCTAssertTrue(outcome.succeeded, outcome.stderr + outcome.stdout)
        let exported = try XCTUnwrap(PDFDocument(url: out))
        XCTAssertEqual(printed.pageCount, exported.pageCount)
        XCTAssertEqual(printed.page(at: 0)?.string, exported.page(at: 0)?.string)
        let exportedBounds = try XCTUnwrap(exported.page(at: 0)).bounds(for: .mediaBox)
        XCTAssertEqual(prepared.paperSize.width, exportedBounds.width, accuracy: 0.01)
        XCTAssertEqual(prepared.paperSize.height, exportedBounds.height, accuracy: 0.01)
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

    func testDisabledWithoutADisplayList() async {
        let model = ShellModel()
        model.flushChrome()
        XCTAssertFalse(model.toolbarExportable)
        XCTAssertFalse(PrintController.documentEnabled(model))
        XCTAssertEqual(PrintController.documentHelp(model),
                       "Nothing to print: no compiled PDF (compile the document first).")
        XCTAssertFalse(PrintController.exportWouldProceed(model))
        guard case .refused(let why) = await PrintController.makeDocumentPrint(from: model) else {
            return XCTFail("no display list must refuse, not build an operation")
        }
        XCTAssertTrue(why.contains("no rendering-v2 display list"), why)
    }

    /// A windowed reply is an incomplete view of the document (window proposal
    /// §4.1), so it is never exported as it stands. With the render pipeline
    /// available the whole document is re-rendered to a private file instead —
    /// the producer's `--v2` side output has no reply-line limit — and that
    /// file is what `flashtex-pdf-exact` is given.
    ///
    /// `exportPDF()` is deliberately not called in these tests: with the route
    /// available it opens a save panel.
    func testWindowedDisplayListIsExportedByReRenderingTheWholeDocument() async throws {
        let model = try await modelShowing("display-list-v2-window.json")
        XCTAssertNotNil(model.displayListV2?.frame?.list.window, "fixture must be a windowed list")
        guard model.wholeDocumentProducer != nil else {
            throw XCTSkip("set FLASHTEX_RENDER (or build crates/render-pipeline) for the whole-document export route")
        }
        XCTAssertTrue(model.toolbarExportable, "a windowed document is still exportable")
        XCTAssertTrue(PrintController.documentEnabled(model))
        XCTAssertNil(model.exportPDFRefusal(), "the whole-document route is available, so nothing to refuse")

        let resolved = await model.exportListURL()
        guard case .success(let list) = resolved else {
            return XCTFail("the whole-document render must produce a list: \(resolved)")
        }
        defer { try? FileManager.default.removeItem(at: list.url) }
        XCTAssertTrue(list.temporary, "the windowed frame itself must never be handed to the writer")
        let text = try String(contentsOf: list.url, encoding: .utf8)
        XCTAssertTrue(text.contains("\"display_list\""), "the side output is a rendering-v2 envelope")
        XCTAssertFalse(text.contains("\"window\""), "the whole-document list is not windowed")
    }

    /// Without a render pipeline the windowed case is a real limit, and says
    /// so in different words than "nothing to export".
    func testWindowedDisplayListWithoutARenderPipelineNamesTheLimit() async throws {
        let saved = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"]
        unsetenv("FLASHTEX_RENDER")
        defer { if let saved { setenv("FLASHTEX_RENDER", saved, 1) } }
        let model = try await modelShowing("display-list-v2-window.json")
        guard model.wholeDocumentProducer == nil else {
            throw XCTSkip("a flashtex-render is discoverable without FLASHTEX_RENDER here")
        }
        let why = try XCTUnwrap(model.exportPDFRefusal())
        XCTAssertTrue(why.contains("too large to send in one reply"), why)
        XCTAssertTrue(why.contains("flashtex build"), why)
        guard case .refused(let printWhy) = await PrintController.makeDocumentPrint(from: model) else {
            return XCTFail("a windowed list with no way to complete it must refuse")
        }
        XCTAssertEqual(printWhy, why, "Print and Export refuse in the same words")
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

    func testStaleEditorStillPrintsTheLastVerifiedFrame() async throws {
        let model = try await modelShowing("display-list-v2-text.json")
        XCTAssertFalse(model.previewIsStale)

        model.updateActiveText(model.activeText + "% edited after compile\n")
        XCTAssertTrue(model.previewIsStale, "editor is ahead of the last compile")
        model.flushChrome()
        XCTAssertTrue(model.toolbarExportable, "Export PDF… stays enabled on a stale preview")
        XCTAssertTrue(PrintController.documentEnabled(model), "Print… still uses the last verified frame")
        XCTAssertTrue(PrintController.documentHelp(model).contains("⌘P"))
    }

    func testHistoricalPreviewRefusesLikeExport() async throws {
        let model = try await modelShowing("display-list-v2-text.json")
        XCTAssertNil(model.historicalRefusal(of: "export"))
        model.historicalPreview = HistoricalDisplay(shownEditorRevision: 1, compilingEditorRevision: 2,
                                                    shownCompileRevision: 1, currentCompileRevision: 2,
                                                    requestID: "preview-1", resultID: "r1")
        XCTAssertFalse(PrintController.exportWouldProceed(model))
        XCTAssertNotNil(model.historicalRefusal(of: "export"))
        guard case .refused(let why) = await PrintController.makeDocumentPrint(from: model) else {
            return XCTFail("historical preview must not print the older snapshot as current")
        }
        XCTAssertTrue(why.contains("unavailable"), why)
        XCTAssertTrue(why.contains("historical revision 1"), why)
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
