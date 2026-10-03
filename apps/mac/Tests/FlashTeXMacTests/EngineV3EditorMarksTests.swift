import Foundation
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Editor diagnostics under the engine-v3 preview (DESIGN §10 app parity,
/// gaps B4, B5 and B12): the host's rows draw the same underlines, gutter
/// marks and Error Lens lines as an old-engine result with the same
/// diagnostics, rebase across typing the same way, ⌘⇧]/[ steps through
/// them, and a finished compile announces its counts to VoiceOver. The
/// first tests run with no host (`FLASHTEX_HOST=none`); the last compiles
/// with a built `flashtex-host` and TeX Live (skipped otherwise).
@MainActor
final class EngineV3EditorMarksTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-marks-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore(); try? FileManager.default.removeItem(at: Self.cache) }

    static let text = "line one\n\\foo here\nline three\n\\bar\nlast\n"
    static func diagnostics() -> [RuntimeV1.Diagnostic] {
        [
            .init(severity: .error, message: "Undefined control sequence.", source: .init(path: "main.tex", startByte: 9, endByte: 13), recovery: nil),
            .init(severity: .warning, message: "Overfull \\hbox (3.0pt too wide) in paragraph", source: .init(path: "main.tex", startByte: 29, endByte: 33), recovery: nil),
            .init(severity: .warning, message: "Unsourced warning", source: nil, recovery: nil),
        ]
    }

    /// The old path: a result with these diagnostics, compiled from `text`.
    func v2Model() -> ShellModel {
        let m = ShellModel()
        m.engineV3Enabled = false
        m.replaceProject(entryText: Self.text, named: "main.tex")
        m.result = RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .recovered, pages: [], diagnostics: Self.diagnostics(), pdfPath: nil)
        m.resultID = "r1"
        m.setCompiledDocuments(["main.tex": Self.text])
        return m
    }

    /// The v3 path: the same rows as the host's, with `text` as the DONE baseline.
    func v3Model() -> ShellModel {
        env.set("FLASHTEX_HOST", "none")
        let m = ShellModel()
        m.replaceProject(entryText: Self.text, named: "main.tex")
        m.engineV3Enabled = true
        m.setEngineV3CompiledDocuments(["main.tex": Self.text])
        m.engineV3Diagnostics = Self.diagnostics()
        return m
    }

    struct Seen: Equatable { var range: NSRange; var severity: RuntimeV1.Severity; var message: String }
    func seen(_ m: ShellModel) -> [Seen] { m.editorMarks.map { Seen(range: $0.nsRange, severity: $0.severity, message: $0.message) } }

    func testV3RowsDrawTheSameEditorMarksAsTheOldPath() {
        let v2 = v2Model(), v3 = v3Model()
        defer { v3.engineV3.stop() }
        XCTAssertEqual(seen(v3).count, 2, "the two sourced rows are marks")
        XCTAssertEqual(seen(v3), seen(v2))
        // Typing before the marks moves them the same way on both paths.
        let typed = "new\n" + Self.text
        v2.updateActiveText(typed)
        v3.updateActiveText(typed)
        XCTAssertEqual(seen(v3), seen(v2))
        XCTAssertEqual(v3.editorMarks.first?.nsRange.location, 13)
        // Typing inside a mark withholds it on both paths (stale), with the same note.
        let inside = typed.replacingOccurrences(of: "\\foo", with: "\\fo")
        v2.updateActiveText(inside)
        v3.updateActiveText(inside)
        XCTAssertEqual(seen(v3), seen(v2))
        XCTAssertEqual(v3.editorMarkReport.staleNote, v2.editorMarkReport.staleNote)
        XCTAssertNotNil(v3.editorMarkReport.staleNote)
    }

    func testNextAndPreviousDiagnosticStepThroughV3MarksAsOnTheOldPath() {
        let v2 = v2Model(), v3 = v3Model()
        defer { v3.engineV3.stop() }
        for m in [v2, v3] { m.caretUTF16 = 0 }
        for forward in [true, true, false] {
            v2.goToDiagnostic(forward: forward)
            v3.goToDiagnostic(forward: forward)
            XCTAssertEqual(v3.selection?.nsRange, v2.selection?.nsRange, "forward: \(forward)")
            XCTAssertEqual(v3.navigationNote, v2.navigationNote)
        }
        XCTAssertNotNil(v3.selection)
        // No rows: a note, no selection change.
        v3.engineV3Diagnostics = []
        let before = v3.selection
        v3.goToDiagnostic(forward: true)
        XCTAssertEqual(v3.selection, before)
        XCTAssertEqual(v3.navigationNote, "The last compile has no diagnostics.")
    }

    func testV3CompileAnnouncesChangedCountsLikeTheOldPath() {
        let m = v3Model()
        defer { m.engineV3.stop() }
        m.noteCompileCompletedForVoiceOver()
        XCTAssertEqual(m.diagnosticAnnouncements, ["1 error, 2 warnings"])
        // Same counts again: silent.
        m.noteCompileCompletedForVoiceOver()
        XCTAssertEqual(m.diagnosticAnnouncements, ["1 error, 2 warnings"])
    }

    /// A view that drew the marks from the memo (a hit reads only the memo's
    /// key) is invalidated when a DONE brings new rows or a new baseline, so
    /// underlines appear and clear without waiting for the next keystroke.
    func testAMemoHitStillObservesNewV3RowsAndBaseline() {
        let m = v3Model()
        defer { m.engineV3.stop() }
        _ = m.editorMarkReport // fill the memo: the reads below are hits
        var fired = false
        withObservationTracking { _ = m.editorMarkReport } onChange: { fired = true }
        m.engineV3Diagnostics = Array(Self.diagnostics().prefix(1))
        XCTAssertTrue(fired, "new rows invalidate a view that read the marks")
        XCTAssertEqual(m.editorMarks.count, 1)

        _ = m.editorMarkReport
        fired = false
        withObservationTracking { _ = m.editorMarkReport } onChange: { fired = true }
        m.setEngineV3CompiledDocuments(["main.tex": "x\n" + Self.text])
        XCTAssertTrue(fired, "a new baseline invalidates it too")
    }

    /// TeX's line numbers refer to the text the compile read. Typing a line
    /// above the error while the compile runs must not move the underline to
    /// the line above it: the row is mapped on the compiled text and rebased
    /// to the editor's, as an old-engine result is.
    func testRowsMapOnTheTextTheCompileReadThenRebase() async throws {
        try EngineV3TestHost.require()
        let doc = "\\documentclass{article}\n\\begin{document}\nHello \\undefinedthing{} world.\n\\end{document}\n"
        let m = ShellModel()
        m.replaceProject(entryText: doc, named: "main.tex")
        m.engineV3Enabled = true
        m.autoCompile = true
        let s = m.engineV3
        s.start(model: m)
        defer { s.stop() }
        try await EngineV3TestHost.awaitReady(s)
        let start = Date()
        func wait(_ what: String, _ cond: () -> Bool) async throws {
            while !cond() {
                if Date().timeIntervalSince(start) > 120 { XCTFail("timeout: \(what) (\(s.statusNote))"); return }
                try await Task.sleep(nanoseconds: 50_000_000)
            }
        }
        try await wait("the first compile's mark") { !s.compiling && !m.editorMarks.isEmpty }
        // An edit is sent (compile N); before its DONE is handled, a line is
        // typed above the error and held (auto-compile off: not sent).
        let sent = doc.replacingOccurrences(of: "world", with: "there")
        m.updateActiveText(sent)
        XCTAssertTrue(s.compiling, "compile N is out")
        m.autoCompile = false
        let typed = sent.replacingOccurrences(of: "\\begin{document}\n", with: "\\begin{document}\nA new line.\n")
        m.updateActiveText(typed)
        try await wait("compile N's DONE") { !s.compiling }
        let mark = try XCTUnwrap(m.editorMarks.first { $0.severity == .error })
        let at = (typed as NSString).range(of: "\\undefinedthing")
        XCTAssertTrue(NSIntersectionRange(mark.nsRange, at).length > 0,
                      "the mark is on \\undefinedthing in the current text: \(mark.nsRange) vs \(at)")
    }

    /// End to end with a host: an undefined control sequence is underlined
    /// where TeX reports it and the compile's counts are spoken.
    func testHostErrorIsUnderlinedAndAnnounced() async throws {
        try EngineV3TestHost.require()
        let doc = "\\documentclass{article}\n\\begin{document}\nHello \\undefinedthing{} world.\n\\end{document}\n"
        let m = ShellModel()
        m.replaceProject(entryText: doc, named: "main.tex")
        m.engineV3Enabled = true
        let s = m.engineV3
        s.start(model: m)
        defer { s.stop() }
        try await EngineV3TestHost.awaitReady(s)
        let start = Date()
        while m.editorMarks.isEmpty || m.diagnosticAnnouncements.isEmpty {
            if Date().timeIntervalSince(start) > 120 { return XCTFail("timeout: \(s.statusNote), \(m.engineV3Diagnostics)") }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        let mark = try XCTUnwrap(m.editorMarks.first { $0.severity == .error })
        let at = (doc as NSString).range(of: "\\undefinedthing")
        XCTAssertTrue(NSIntersectionRange(mark.nsRange, at).length > 0, "\(mark.nsRange) vs \(at)")
        XCTAssertEqual(m.diagnosticAnnouncements.last.map { $0.hasPrefix("1 error") }, true, "\(m.diagnosticAnnouncements)")
    }
}
