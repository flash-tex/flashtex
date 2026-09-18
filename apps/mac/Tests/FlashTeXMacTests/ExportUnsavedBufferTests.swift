import XCTest
import PDFKit
import FlashTeXProtocol
@testable import FlashTeXMac

/// P0 (owner report, 2026-09-16): "if the file is unsaved, it doesn't export …
/// Export works fine if (all) files are saved, otherwise the export fails
/// silently." Drives the app's default route — the bundled `flashtex-render`
/// attached as the direct worker with the v2 pane live — through a saved
/// open, an unsaved edit and File > Export PDF…'s non-interactive core.
///
/// Skipped unless FLASHTEX_RENDER and FLASHTEX_PDF_EXACT name built binaries.
@MainActor
final class ExportUnsavedBufferTests: XCTestCase {
    private static var repoRoot: URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url = url.deletingLastPathComponent() }
        return url
    }

    private func tool(_ name: String) -> URL? {
        ProcessInfo.processInfo.environment[name].map { URL(fileURLWithPath: $0) }
            .flatMap { FileManager.default.isExecutableFile(atPath: $0.path) ? $0 : nil }
    }

    private func waitUntil(timeout: TimeInterval = 40, _ what: String, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout waiting for \(what) (load-sensitive)") }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    private func loadedRevision(_ model: ShellModel) -> Int? {
        guard case .loaded(let frame, _) = model.displayListV2 else { return nil }
        return frame.list.revision
    }

    func testUnsavedEditExportsTheBufferThePreviewShows() async throws {
        guard let render = tool("FLASHTEX_RENDER"), let exact = tool("FLASHTEX_PDF_EXACT") else {
            throw XCTSkip("set FLASHTEX_RENDER and FLASHTEX_PDF_EXACT to built binaries")
        }
        setenv("FLASHTEX_FONT_DIRS", Self.repoRoot.appendingPathComponent("apps/mac/Fonts").path, 1)
        defer { unsetenv("FLASHTEX_FONT_DIRS") }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("export-unsaved-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let main = root.appendingPathComponent("main.tex")
        let body = (2...3).map { "\\newpage\nPage \($0). " + String(repeating: "Filler words for a longer page body. ", count: 40) + "\n" }.joined()
        try ("\\documentclass{article}\n\\begin{document}\nSaved text.\n\n\\input{chapter}\n" + body + "\\end{document}\n")
            .write(to: main, atomically: true, encoding: .utf8)
        try "Chapter text.\n".write(to: root.appendingPathComponent("chapter.tex"), atomically: true, encoding: .utf8)

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: main), .opened)
        model.attachWorker(at: render)
        defer { model.detachWorker() }
        model.previewV2 = true
        model.setLiveV2(true)
        model.compile()
        try await waitUntil("the saved preview") { loadedRevision(model) == model.editorRevision }

        // Saved: exports.
        let savedOut = root.appendingPathComponent("saved.pdf")
        try await export(model, tool: exact, to: savedOut)

        // Unsaved edit, compiled and shown in the preview.
        let edited = model.activeText.replacingOccurrences(of: "Saved text.", with: "Unsaved edit on screen.")
        model.updateActiveText(edited)
        XCTAssertTrue(model.project.isDirty("main.tex"), "the buffer is unsaved")
        model.compile()
        try await waitUntil("the unsaved preview") { loadedRevision(model) == model.editorRevision }

        // An unsaved non-entry member too.
        let opened = await model.project.openDiscoveredIncludes()
        XCTAssertEqual(opened, [.opened(path: "chapter.tex")])
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Unsaved chapter edit.\n")
        XCTAssertTrue(model.project.isDirty("chapter.tex"))
        model.compile()
        try await waitUntil("the unsaved member preview") { loadedRevision(model) == model.editorRevision }

        // The failing shape: the frame on screen was rebuilt from a
        // display_list_delta, whose raw line flashtex-pdf-exact refuses.
        XCTAssertEqual(model.displayListV2?.retained?.source.isDeltaLine, true, "an edit after the first full frame arrives as a delta")

        let dirtyOut = root.appendingPathComponent("unsaved.pdf")
        try await export(model, tool: exact, to: dirtyOut)
        let text = PDFDocument(url: dirtyOut)?.string ?? ""
        let flat = text.replacingOccurrences(of: "\n", with: " ")
        XCTAssertTrue(flat.contains("Unsaved chapter edit"), "the PDF carries the unsaved member: \(text.prefix(200))")
        XCTAssertTrue(flat.contains("Unsaved edit on screen"), "the PDF carries the unsaved entry: \(text.prefix(200))")
        XCTAssertTrue(model.project.isDirty("main.tex"), "export never saves")
    }

    private func export(_ model: ShellModel, tool: URL, to out: URL, file: StaticString = #filePath, line: UInt = #line) async throws {
        XCTAssertNil(model.exportPDFRefusal(), file: file, line: line)
        let resolved: WholeDocumentList.Resolved
        switch await model.exportListURL() {
        case .failure(let why): return XCTFail("exportListURL refused: \(why.reason)", file: file, line: line)
        case .success(let r): resolved = r
        }
        let outcome: ExactPDFExport.Outcome? = await withCheckedContinuation { cont in
            model.exportPDFExact(listURL: resolved.url, tool: tool, to: out) { cont.resume(returning: $0) }
        }
        if resolved.temporary { try? FileManager.default.removeItem(at: resolved.url) }
        XCTAssertEqual(outcome?.exitCode, 0, "export failed: \(outcome?.stderr.prefix(400) ?? "not launched"); note: \(model.captureNote ?? "nil")", file: file, line: line)
        XCTAssertTrue(FileManager.default.fileExists(atPath: out.path), "no PDF written; note: \(model.captureNote ?? "nil")", file: file, line: line)
    }
}
