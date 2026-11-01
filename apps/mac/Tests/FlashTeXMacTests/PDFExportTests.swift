import XCTest
import PDFKit
import FlashTeXProtocol
@testable import FlashTeXMac

@MainActor
final class PDFExportTests: XCTestCase {
    private static var repoRoot: URL {
        // Tests/FlashTeXMacTests/PDFExportTests.swift -> repo root is five levels up.
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url = url.deletingLastPathComponent() }
        return url
    }

    private func loadFixtureResult() throws -> RuntimeV1.CompileResult {
        let url = Self.repoRoot.appendingPathComponent("protocol/fixtures/compile-result.json")
        return try RuntimeV1.decodeCompileResult(Data(contentsOf: url)).payload
    }

    func testFixtureExportsToLetterPDFContainingItemText() throws {
        let result = try loadFixtureResult()
        let data = PDFExport.render(result)

        XCTAssertTrue(data.starts(with: Array("%PDF".utf8)), "output is not a PDF")
        let doc = try XCTUnwrap(PDFDocument(data: data))
        XCTAssertEqual(doc.pageCount, result.pages.count)
        let page = try XCTUnwrap(doc.page(at: 0))
        let bounds = page.bounds(for: .mediaBox)
        XCTAssertEqual(bounds.width, 612, accuracy: 0.01)
        XCTAssertEqual(bounds.height, 792, accuracy: 0.01)
        XCTAssertTrue(page.string?.contains("Hello FlashTeX.") == true, "page text: \(page.string ?? "nil")")

        // Dark export is the same document with different colors only.
        let dark = try XCTUnwrap(PDFDocument(data: PDFExport.render(result, dark: true)))
        XCTAssertEqual(dark.pageCount, doc.pageCount)
        XCTAssertTrue(dark.page(at: 0)?.string?.contains("Hello FlashTeX.") == true)
    }

    func testTwoPageResultKeepsPerPageSizesAndSkipsUnknownItems() throws {
        let json = """
        {"protocol_version":1,"id":"t","type":"compile_result","payload":{
          "project_id":"demo","revision":3,"status":"ok","pages":[
            {"number":1,"width_pt":595.276,"height_pt":841.89,"items":[
              {"kind":"text","text":"First page","x_pt":72,"baseline_y_pt":100,"font_size_pt":14,"source":null},
              {"kind":"image","path":"x.png"}]},
            {"number":2,"width_pt":400,"height_pt":300,"items":[
              {"kind":"text","text":"Second page","x_pt":10,"baseline_y_pt":50,"font_size_pt":10,"source":null}]}
          ],"diagnostics":[],"pdf_path":null}}
        """
        let result = try RuntimeV1.decodeCompileResult(Data(json.utf8)).payload
        XCTAssertEqual(result.pages[0].items[1], .unknown(kind: "image"))

        let doc = try XCTUnwrap(PDFDocument(data: PDFExport.render(result)))
        XCTAssertEqual(doc.pageCount, 2)
        let p0 = try XCTUnwrap(doc.page(at: 0)).bounds(for: .mediaBox)
        let p1 = try XCTUnwrap(doc.page(at: 1)).bounds(for: .mediaBox)
        XCTAssertEqual(p0.width, 595.276, accuracy: 0.01)
        XCTAssertEqual(p0.height, 841.89, accuracy: 0.01)
        XCTAssertEqual(p1.width, 400, accuracy: 0.01)
        XCTAssertEqual(p1.height, 300, accuracy: 0.01)
        XCTAssertTrue(doc.page(at: 0)?.string?.contains("First page") == true)
        XCTAssertTrue(doc.page(at: 1)?.string?.contains("Second page") == true)
        XCTAssertFalse(doc.page(at: 0)?.string?.contains("Second page") == true)
    }

    func testEmptyResultStillProducesValidPDF() throws {
        // A PDF must contain at least one page; CoreGraphics emits a single
        // blank letter-size page when no page was begun.
        let result = RuntimeV1.CompileResult(projectId: "demo", revision: 1, status: .failed,
                                             pages: [], diagnostics: [], pdfPath: nil)
        let data = PDFExport.render(result)
        XCTAssertTrue(data.starts(with: Array("%PDF".utf8)))
        let doc = try XCTUnwrap(PDFDocument(data: data))
        XCTAssertEqual(doc.pageCount, 1)
        XCTAssertEqual(doc.page(at: 0)?.string?.trimmingCharacters(in: .whitespacesAndNewlines) ?? "", "")
    }
}
