import XCTest
import PDFKit
@testable import FlashTeXMac

/// Searchable text of the exact route as PDFKit (Preview, Spotlight's PDF
/// importer, the app's own search) reads it — exact strings, not "contains a
/// word": word boundaries are the producer's inter-glyph gaps replayed
/// exactly, ligatures come back as their cluster text, kerned pairs and TFM
/// widths narrower than hmtx do not split words. Policy and measurements:
/// `docs/proposals/pdf-searchable-text.md` (GH48). Skipped unless
/// `FLASHTEX_PDF_EXACT` names a built `flashtex-pdf-exact`.
final class SearchableTextTests: XCTestCase {
    static var tool: URL? { ProcessInfo.processInfo.environment["FLASHTEX_PDF_EXACT"].map { URL(fileURLWithPath: $0) } }
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let fonts = fixtures.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Fonts")

    private func export(_ fixture: String) throws -> (PDFDocument, ExactPDFExport.Outcome) {
        guard let tool = Self.tool, FileManager.default.isExecutableFile(atPath: tool.path) else {
            throw XCTSkip("set FLASHTEX_PDF_EXACT to a built flashtex-pdf-exact")
        }
        let out = FileManager.default.temporaryDirectory.appendingPathComponent("searchable-\(UUID().uuidString).pdf")
        addTeardownBlock { try? FileManager.default.removeItem(at: out) }
        let outcome = try ExactPDFExport.run(tool: tool, list: Self.fixtures.appendingPathComponent(fixture), out: out, fontDirs: [Self.fonts.path])
        XCTAssertTrue(outcome.succeeded, outcome.stderr)
        return (try XCTUnwrap(PDFDocument(url: out)), outcome)
    }

    /// GH48's `\text{a b}`: two one-glyph runs with glue between them and no
    /// space cluster (the published display list, rendering-core 647c50c5).
    /// PDFKit reads the 0.326 em gap as a word boundary without any space
    /// glyph, ActualText or `Tw` in the file.
    func testGlueBetweenSingleGlyphRunsIsAWordBoundary() throws {
        let (pdf, outcome) = try export("display-list-v2-searchable-a-b.json")
        XCTAssertEqual(pdf.page(at: 0)?.string, "a b")
        XCTAssertTrue(outcome.stderr.contains("1 word gap(s)"), outcome.stderr)
        XCTAssertTrue(outcome.stderr.contains("0 ambiguous gap(s)"), outcome.stderr)
    }

    /// Kerned `AV`, `ffi`/`fi` ligatures, a `:` and a `.` attached to their
    /// words, and math: exact PDFKit text. The two spaces inside `\forall`
    /// (typeset as source text by this producer build) and before `(` are
    /// the producer's 0.099 em italic corrections, which PDFKit breaks on and
    /// the report names as ambiguous; they are asserted as observed, not
    /// promised.
    func testKernsLigaturesAndWordGapsExtractExactly() throws {
        let (pdf, outcome) = try export("display-list-v2-searchable-mixed.json")
        let text = try XCTUnwrap(pdf.page(at: 0)?.string)
        XCTAssertTrue(text.hasPrefix("The AV office fixed a b: "), text)
        XCTAssertTrue(text.hasSuffix(" and ffi."), text)
        XCTAssertFalse(text.contains("A V"), text)
        XCTAssertFalse(text.contains("of fice"), text)
        XCTAssertFalse(text.contains("f fi"), text)
        XCTAssertEqual(text, "The AV office fixed a b: \\f orallx, f (x) and ffi.")
        XCTAssertTrue(outcome.stderr.contains("9 word gap(s)"), outcome.stderr)
        XCTAssertTrue(outcome.stderr.contains("2 ambiguous gap(s)"), outcome.stderr)
        XCTAssertTrue(outcome.stderr.contains("between \"f\" and \"(\""), outcome.stderr)
        // Searching finds words across the boundary the geometry encodes.
        XCTAssertEqual(pdf.findString("a b:", withOptions: []).count, 1)
        XCTAssertEqual(pdf.findString("office", withOptions: []).count, 1)
    }

    /// The existing text fixture (bold `W`, TFM widths narrower than hmtx):
    /// the mac-live report of the 12:00 bundle read `A V`; with the display
    /// list's advances as `/W` PDFKit reads the line as typed.
    func testBoldTfmWidthsDoNotSplitWords() throws {
        let (pdf, _) = try export("display-list-v2-text.json")
        let text = try XCTUnwrap(pdf.page(at: 0)?.string)
        XCTAssertEqual(text.replacingOccurrences(of: "\n", with: " "), "Office fixtures The AV office fixed the fi ligature: office, bold, and café.")
    }
}
