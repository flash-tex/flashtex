import Foundation
import PDFKit
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXMac

/// The app's flows in Unicode mode (lane UNICODE-PREVIEW-QA): a
/// `% !TEX program = xelatex` document in `flashtex-host-unicode`, then
/// Export PDF…, Print…, an error in the Problems rows with its file and
/// line, and reverse/forward search. Needs a built `flashtex-host-unicode`
/// and a TeX Live (skipped otherwise, failed with
/// FLASHTEX_REQUIRE_HOST_UNICODE=1 as ci.yml's mac-v3-host sets it).
@MainActor
final class EngineV3UnicodeFlowsTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-unicode-\(getpid())")
    private var env = EnvironmentOverride()
    private var dirs: [URL] = []

    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        env.set("FLASHTEX_MODE", "")
    }

    override func tearDown() {
        env.restore()
        for d in dirs { try? FileManager.default.removeItem(at: d) }
    }

    override class func tearDown() { try? FileManager.default.removeItem(at: cache) }

    private func waitUntil(_ what: String, timeout: TimeInterval = 120, debug: () -> String = { "" }, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what) \(debug())"); return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    private func unavailable(_ why: String) -> Error {
        guard ProcessInfo.processInfo.environment["FLASHTEX_REQUIRE_HOST_UNICODE"] == "1" else { return XCTSkip(why) }
        XCTFail("FLASHTEX_REQUIRE_HOST_UNICODE=1: " + why)
        return EngineV3TestHost.HostMissing(description: why)
    }

    static let source = """
    % !TEX program = xelatex
    \\documentclass{article}
    \\usepackage{fontspec}
    \\begin{document}
    \\section{Alpha}\\label{alpha}
    Unicode text: naïve café, Œuvre, see section~\\ref{alpha}.
    \\newpage
    Second page.
    \\end{document}

    """

    /// A started model on `text` (saved as paper.tex), in Unicode mode, after its first compile.
    private func startedModel(_ text: String = EngineV3UnicodeFlowsTests.source, pages: Int = 2, ok: Bool = true) async throws -> ShellModel {
        guard EngineV3.locateHost(mode: .unicode) != nil else { throw unavailable("no flashtex-host-unicode built") }
        guard EngineV3TestHost.texLive != nil else { throw unavailable("no TeX Live") }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-unicode-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        dirs.append(dir)
        let file = dir.appendingPathComponent("paper.tex")
        try text.write(to: file, atomically: true, encoding: .utf8)
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: file, dirty: .discard), .opened)
        model.engineV3Enabled = true
        let s = model.engineV3
        try await EngineV3TestHost.awaitReady(s)
        XCTAssertEqual(s.hostMode, .unicode)
        try await waitUntil("the first compile", debug: { "\(s.phase) \(s.statusNote) pages \(s.pageCount) done \(s.doneCount) error \(s.firstError ?? "-")" }) { !s.compiling && s.doneCount > 0 && s.pageCount == pages && (!ok || s.statusNote.hasPrefix("ok")) }
        return model
    }

    func testExportWritesFlashTeXsUnicodePDF() async throws {
        let model = try await startedModel()
        defer { model.engineV3.stop() }
        XCTAssertTrue(model.exportAvailable)
        let dest = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-unicode-\(UUID().uuidString).pdf")
        defer { try? FileManager.default.removeItem(at: dest) }
        let report = await withCheckedContinuation { c in
            model.exportPDFEngineV3(to: .recordingCurrentDisk(dest)) { c.resume(returning: $0) }
        }
        guard case .succeeded = report.state else { return XCTFail("export: \(report.state)") }
        let pdf = try XCTUnwrap(PDFDocument(url: dest))
        XCTAssertEqual(pdf.pageCount, 2)
        let text = pdf.string ?? ""
        for word in ["naïve", "café", "Œuvre", "see section 1"] {
            XCTAssertTrue(text.contains(word), "\(word) in the exported text: \(text)")
        }
        let data = try Data(contentsOf: dest)
        XCTAssertNotNil(data.range(of: Data("LMRoman10-Regular".utf8)), "the OpenType font (Latin Modern, TU) is embedded")
    }

    func testPrintUsesTheExportedPDF() async throws {
        let model = try await startedModel()
        defer { model.engineV3.stop() }
        switch await PrintController.makeDocumentPrint(from: model) {
        case .refused(let why): XCTFail(why)
        case .ready(let prepared): XCTAssertEqual(prepared.pageCount, 2)
        }
    }

    /// An error in Unicode mode is a Problems row on its token; one
    /// TeX recovers from is, under best effort, a warning that names
    /// XeLaTeX (it said "pdfLaTeX would report an error here").
    func testAnErrorIsAProblemOnItsFileAndLine() async throws {
        let text = "% !TEX program = xelatex\n\\documentclass{article}\n\\usepackage{fontspec}\n\\begin{document}\nHello \\undefinedmacro{} world.\n\\end{document}\n"
        let model = try await startedModel(text, pages: 1, ok: false)
        defer { model.engineV3.stop() }
        XCTAssertEqual(model.engineV3.engineName, "XeLaTeX")
        let rows = model.engineV3Diagnostics
        let row = try XCTUnwrap(rows.first { $0.message.contains("Undefined control sequence") }, rows.map(\.message).description)
        if row.severity == .warning {
            XCTAssertTrue(row.message.hasSuffix("(XeLaTeX would report an error here)"), row.message)
        }
        let source = try XCTUnwrap(row.source)
        XCTAssertEqual(source.path, "paper.tex")
        let line5 = Array(text.utf8).count - Array("Hello \\undefinedmacro{} world.\n\\end{document}\n".utf8).count
        // diag-v1 (#1791): the row is the undefined command itself, with its
        // code and the explanation, as in Classic mode.
        XCTAssertTrue(model.engineV3.hostOffersDiagV1, "the Unicode host offers diag-v1")
        XCTAssertEqual(text.utf8Slice(source.startByte, source.endByte), "\\undefinedmacro", "the token TeX stopped after")
        XCTAssertEqual(row.code, "tex/undefined-control-sequence")
        XCTAssertNotNil(row.notes?.first, "an explanation")
    }

    /// Reverse search (a click on a glyph), forward search (a line and
    /// column of the source) and caret follow, from the display list's
    /// SOURCES, SPAN items and glyph columns (#1789; a native word's glyphs
    /// carry the column of its first character).
    func testReverseAndForwardSearch() async throws {
        let model = try await startedModel()
        defer { model.engineV3.stop() }
        let s = model.engineV3
        let ix = try XCTUnwrap(s.sourceIndex(page: 0))
        XCTAssertFalse(ix.glyphs.isEmpty, "page 1 has glyphs")
        let hits = ix.glyphs.compactMap { s.source(page: 0, at: CGPoint(x: $0.cell.midX, y: $0.cell.midY)) }
        XCTAssertEqual(hits.first?.path, "paper.tex", "reverse search")
        XCTAssertEqual(hits.first?.line, 5, "the section number: \\section's line")
        // Line 6 is `Unicode text: naïve café, Œuvre, see section~\\ref{alpha}.`:
        // `naïve` starts at byte 14, `café` at 21 (ï is two bytes).
        let line6 = hits.filter { $0.line == 6 }
        XCTAssertTrue(line6.contains { $0.col == 14 }, "naïve: \(line6.map(\.col))")
        XCTAssertTrue(line6.contains { $0.col == 21 }, "café: \(line6.map(\.col))")
        // Forward search: the line's word at the column, on its page.
        let naive = try XCTUnwrap(s.place(path: "paper.tex", line: 6, col: 14), "forward search")
        XCTAssertEqual(naive.page, 0)
        let cafe = try XCTUnwrap(s.place(path: "paper.tex", line: 6, col: 21))
        XCTAssertGreaterThan(cafe.rect.minX, naive.rect.minX, "café is right of naïve")
        XCTAssertEqual(s.place(path: "paper.tex", line: 9, col: nil)?.page, 1, "Second page. is on page 2")
    }
}
