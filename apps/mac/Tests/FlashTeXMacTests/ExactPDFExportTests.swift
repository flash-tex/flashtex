import XCTest
import PDFKit
@testable import FlashTeXMac

/// The exact route end to end: a verified v2 display list → `flashtex-pdf-exact
/// from-v2` → a PDF whose text and page count match the list. Skipped unless
/// `FLASHTEX_PDF_EXACT` names a built tool (crates/pdf, mac-pdf lane ≥ 654f626).
@MainActor
final class ExactPDFExportTests: XCTestCase {
    static var tool: URL? { ProcessInfo.processInfo.environment["FLASHTEX_PDF_EXACT"].map { URL(fileURLWithPath: $0) } }
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")

    func testLoadedDisplayListExportsThroughTheExactRoute() async throws {
        guard let tool = Self.tool, FileManager.default.isExecutableFile(atPath: tool.path) else {
            throw XCTSkip("set FLASHTEX_PDF_EXACT to a built flashtex-pdf-exact")
        }
        let model = ShellModel()
        let list = Self.fixtures.appendingPathComponent("display-list-v2-text.json")
        await withCheckedContinuation { cont in model.loadDisplayListV2(url: list) { cont.resume() } }
        guard case .loaded(let frame, _)? = model.displayListV2 else { return XCTFail("list did not load: \(String(describing: model.displayListV2))") }
        let out = FileManager.default.temporaryDirectory.appendingPathComponent("exact-\(UUID().uuidString).pdf")
        defer { try? FileManager.default.removeItem(at: out) }
        setenv("FLASHTEX_FONT_DIRS", Self.fixtures.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Fonts").path, 1)
        defer { unsetenv("FLASHTEX_FONT_DIRS") }
        let outcome: ExactPDFExport.Outcome? = await withCheckedContinuation { cont in
            model.exportPDFExact(listURL: list, tool: tool, to: out) { cont.resume(returning: $0) }
        }
        let o = try XCTUnwrap(outcome)
        XCTAssertTrue(o.succeeded, o.stderr + o.stdout)
        XCTAssertTrue(model.captureNote?.hasPrefix("Exported exact PDF") == true, model.captureNote ?? "")
        let pdf = try XCTUnwrap(PDFDocument(url: out))
        XCTAssertEqual(pdf.pageCount, frame.list.pages.count)
        // The exported text is the list's cluster text (ToUnicode), not glyph ids.
        let text = pdf.string ?? ""
        let expected = frame.list.pages.flatMap(\.items).compactMap { item -> String? in
            if case .glyphRun(let run) = item { return run.text } else { return nil }
        }.joined(separator: " ")
        for word in expected.split(separator: " ").prefix(6) where word.count > 2 {
            XCTAssertTrue(text.contains(word), "exported text lacks \(word)")
        }
        // Refusals name the item and never produce a file: an unreadable list.
        let bad = FileManager.default.temporaryDirectory.appendingPathComponent("bad-\(UUID().uuidString).json")
        try Data("{}".utf8).write(to: bad)
        defer { try? FileManager.default.removeItem(at: bad) }
        let out2 = FileManager.default.temporaryDirectory.appendingPathComponent("exact-\(UUID().uuidString).pdf")
        let refused: ExactPDFExport.Outcome? = await withCheckedContinuation { cont in
            model.exportPDFExact(listURL: bad, tool: tool, to: out2) { cont.resume(returning: $0) }
        }
        XCTAssertEqual(refused?.succeeded, false)
        XCTAssertFalse(FileManager.default.fileExists(atPath: out2.path))
        XCTAssertTrue(model.captureNote?.hasPrefix("Exact export refused") == true, model.captureNote ?? "")
    }
}
