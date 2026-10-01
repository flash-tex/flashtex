import Foundation
import PDFKit
import XCTest
@testable import FlashTeXMac

/// Export PDF… and Print… under the engine-v3 preview (lane P5-APP-PARITY,
/// gaps D1-D3): the PDF comes from the host's `export: true` run (the
/// compressed PDF pdflatex would write, with the resident run's `.aux`), is
/// published atomically through the export session, and never comes from
/// the old engine. Edits typed while it runs are held and sent after. The
/// end-to-end cases need a built `flashtex-host` (skipped otherwise) and use
/// a private v3 cache.
@MainActor
final class EngineV3ExportTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-export-\(getpid())")
    private var storedFlag: Any?
    private var dirs: [URL] = []

    override func setUp() {
        setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1)
        storedFlag = UserDefaults.standard.object(forKey: EngineV3.enabledKey)
    }

    override func tearDown() {
        unsetenv("FLASHTEX_HOST")
        unsetenv("FLASHTEX_V3_CACHE")
        if let storedFlag { UserDefaults.standard.set(storedFlag, forKey: EngineV3.enabledKey) } else { UserDefaults.standard.removeObject(forKey: EngineV3.enabledKey) }
        for d in dirs { try? FileManager.default.removeItem(at: d) }
    }

    override class func tearDown() { try? FileManager.default.removeItem(at: cache) }

    private func waitUntil(_ what: String, timeout: TimeInterval = 90, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    private func tempDir(_ name: String) throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-export-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        dirs.append(dir)
        return dir
    }

    static let source = """
    \\documentclass{article}
    \\begin{document}
    \\section{Alpha}\\label{alpha}
    See section~\\ref{alpha} on page~\\pageref{alpha}.
    \\newpage
    Second page.
    \\end{document}

    """

    private func startedModel(_ text: String = EngineV3ExportTests.source) async throws -> ShellModel {
        guard EngineV3.locateHost() != nil else { throw XCTSkip("no flashtex-host built (cargo build --release -p flashtex-engine --bin flashtex-host)") }
        let dir = try tempDir("project")
        let file = dir.appendingPathComponent("paper.tex")
        try text.write(to: file, atomically: true, encoding: .utf8)
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: file, dirty: .discard), .opened)
        model.engineV3Enabled = true
        let s = model.engineV3
        try await waitUntil("the host") { s.phase == .ready || { if case .failed = s.phase { true } else { false } }() }
        guard s.phase == .ready else { throw XCTSkip("host did not start: \(s.phase)") }
        // The resident run converges its .aux (references resolved) before DONE.
        try await waitUntil("the first compile") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount == 2 }
        return model
    }

    private func export(_ model: ShellModel, to dest: URL) async -> ExportSession.Report {
        await withCheckedContinuation { c in
            model.exportPDFEngineV3(to: .recordingCurrentDisk(dest)) { c.resume(returning: $0) }
        }
    }

    // MARK: without a host

    func testNoHostRefusesExportAndPrintWithTheV3Reason() {
        setenv("FLASHTEX_HOST", "none", 1)
        let model = ShellModel()
        defer { model.engineV3.stop() }
        model.engineV3Enabled = true
        XCTAssertFalse(model.exportAvailable)
        XCTAssertFalse(PrintController.documentEnabled(model))
        let why = model.exportPDFRefusal() ?? ""
        XCTAssertTrue(why.contains("engine-v3 preview stopped"), why)
        XCTAssertTrue(PrintController.documentHelp(model).hasPrefix("Nothing to print"))
    }

    // MARK: with a host

    func testExportWritesTheCompressedPDFWithResolvedReferences() async throws {
        let model = try await startedModel()
        defer { model.engineV3.stop() }
        XCTAssertTrue(model.exportAvailable)
        let dest = try tempDir("out").appendingPathComponent("paper.pdf")
        let report = await export(model, to: dest)
        guard case .succeeded(let bytes, _) = report.state else { return XCTFail("export: \(report.state)") }
        let data = try Data(contentsOf: dest)
        XCTAssertEqual(data.count, bytes)
        XCTAssertTrue(data.starts(with: Data("%PDF-".utf8)))
        XCTAssertNotNil(data.range(of: Data("/FlateDecode".utf8)), "the export is the compressed PDF, not the preview's stored one")
        let pdf = try XCTUnwrap(PDFDocument(url: dest))
        XCTAssertEqual(pdf.pageCount, 2)
        let text = pdf.string ?? ""
        XCTAssertTrue(text.contains("See section 1 on page 1"), "references come from the resident run's .aux: \(text)")
        XCTAssertFalse(text.contains("??"))
        XCTAssertFalse(model.engineV3.exporting)
        // No stray temp file beside the destination.
        let siblings = try FileManager.default.contentsOfDirectory(atPath: dest.deletingLastPathComponent().path)
        XCTAssertEqual(siblings, ["paper.pdf"])
        // FLASHTEX_V3_EXPORT_KEEP=<dir>: keep the PDF and its source for a P-T2
        // comparison with pdflatex outside the test (tools/parity/tiers.py).
        if let keep = ProcessInfo.processInfo.environment["FLASHTEX_V3_EXPORT_KEEP"], !keep.isEmpty {
            let k = URL(fileURLWithPath: keep, isDirectory: true)
            try? FileManager.default.createDirectory(at: k, withIntermediateDirectories: true)
            try? FileManager.default.removeItem(at: k.appendingPathComponent("paper.pdf"))
            try FileManager.default.copyItem(at: dest, to: k.appendingPathComponent("paper.pdf"))
            try Self.source.write(to: k.appendingPathComponent("paper.tex"), atomically: true, encoding: .utf8)
        }
    }

    func testEditsDuringTheExportAreHeldThenSentAndThePreviewStaysIntact() async throws {
        let model = try await startedModel()
        defer { model.engineV3.stop() }
        let s = model.engineV3
        let dest = try tempDir("out").appendingPathComponent("held.pdf")
        let done = expectation(description: "export")
        var report: ExportSession.Report?
        model.exportPDFEngineV3(to: .recordingCurrentDisk(dest)) { report = $0; done.fulfill() }
        XCTAssertTrue(s.exporting)
        // Typed while the export is out: the third page must arrive after it.
        try await waitUntil("the export to start running") { !s.compiling }
        XCTAssertTrue(s.exporting, "the export run is still going when the edit is typed")
        model.updateActiveText(Self.source.replacingOccurrences(of: "Second page.", with: "Second page.\n\\newpage\nThird page."))
        await fulfillment(of: [done], timeout: 90)
        guard case .succeeded = report?.state else { return XCTFail("export: \(String(describing: report?.state))") }
        XCTAssertEqual(PDFDocument(url: dest)?.pageCount, 2, "the export is the text when it was asked for")
        try await waitUntil("the held edit") { s.pageCount == 3 && !s.compiling }
        // The export's frames never touched the preview's resources: every page
        // still draws from the display list.
        XCTAssertTrue(s.pdfFallback.isEmpty, "pages fell back to the PDF: \(s.pdfFallback.keys.sorted())")
        XCTAssertEqual(Set(s.pages.keys), [0, 1, 2])
    }

    func testPrintUsesTheExportedPDF() async throws {
        let model = try await startedModel()
        defer { model.engineV3.stop() }
        switch await PrintController.makeDocumentPrint(from: model) {
        case .refused(let why): XCTFail(why)
        case .ready(let prepared): XCTAssertEqual(prepared.pageCount, 2)
        }
    }

    func testCancelBeforeTheRunWritesNothing() async throws {
        let model = try await startedModel()
        defer { model.engineV3.stop() }
        let dest = try tempDir("out").appendingPathComponent("cancelled.pdf")
        let done = expectation(description: "export")
        var report: ExportSession.Report?
        model.exportPDFEngineV3(to: .recordingCurrentDisk(dest)) { report = $0; done.fulfill() }
        model.cancelExactExport() // the capture bar's Cancel
        await fulfillment(of: [done], timeout: 30)
        XCTAssertEqual(report?.state, .cancelled)
        XCTAssertFalse(FileManager.default.fileExists(atPath: dest.path))
        XCTAssertFalse(model.engineV3.exporting)
        // The preview carries on.
        model.updateActiveText(Self.source.replacingOccurrences(of: "Second page.", with: "Second page.\n\\newpage\nThird page."))
        try await waitUntil("an edit after the cancel") { model.engineV3.pageCount == 3 && !model.engineV3.compiling }
    }
}
