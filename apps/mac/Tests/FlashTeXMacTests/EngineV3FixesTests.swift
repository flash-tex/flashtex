import Foundation
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Mechanical fixes and missing-file actions under the engine-v3 preview
/// (DESIGN §10 app parity, gaps B10 and B11): TeX's "Undefined control
/// sequence" gets the old engine's "did you mean \alpha?" replacement (same
/// vocabulary, distance and tie rules), offered by "Fix…" and Tab exactly as
/// for an old-engine result; TeX's "File `x.tex' not found" and "File
/// `x.sty' not found" offer Create x.tex and Create x.sty. The host tests
/// need a built `flashtex-host` and TeX Live (skipped otherwise).
@MainActor
final class EngineV3FixesTests: XCTestCase {
    private var env = EnvironmentOverride()
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-fixes-\(getpid())")
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

    // MARK: the rule

    func testDidYouMeanFollowsTheOldCompilersRule() {
        let v = Completion.defaultSupported
        XCTAssertTrue(v.contains("alpha") && v.contains("textbf") && v.contains("section"), "the inventory names commands without the backslash")
        XCTAssertEqual(EngineV3Fixes.closestCommands("alpah", vocabulary: v), ["alpha"], "a transposition is one edit")
        XCTAssertEqual(EngineV3Fixes.closestCommands("textbff", vocabulary: v), ["textbf"])
        XCTAssertEqual(EngineV3Fixes.closestCommands("alpha", vocabulary: v), [], "a known name has no fix")
        XCTAssertEqual(EngineV3Fixes.closestCommands("qqqqqqqq", vocabulary: v), [])
        // Ties are no fix (the old compiler's rule: a wrong rewrite on Tab is worse than none).
        XCTAssertEqual(EngineV3Fixes.closestCommands("xa", vocabulary: ["xb", "xc"]), ["xb", "xc"])
        XCTAssertEqual(EngineV3Fixes.closestCommands("abcd", vocabulary: ["abcde", "abc"]).sorted(), ["abc", "abcde"])
        // A name of up to 3 characters allows 1 edit, longer ones 2.
        XCTAssertEqual(EngineV3Fixes.closestCommands("xyz", vocabulary: ["xzz9"]), [])
        XCTAssertEqual(EngineV3Fixes.distance("alpah", "alpha"), 1)
        XCTAssertEqual(EngineV3Fixes.distance("kitten", "sitting"), 3)
    }

    func testAttachGivesTheOldEnginesHelpAndReplacement() {
        let text = "Hello \\alpah and \\2 and \\qqqqqqqq and \\section."
        func row(_ cs: String) -> RuntimeV1.Diagnostic {
            let start = text.utf8.distance(from: text.startIndex, to: text.range(of: cs)!.lowerBound)
            return RuntimeV1.Diagnostic(severity: .error, message: "Undefined control sequence.",
                                        source: .init(path: "main.tex", startByte: start, endByte: start + cs.utf8.count),
                                        recovery: nil, code: EngineV3Fixes.undefinedCode,
                                        help: .init(message: "The control sequence at the end of the top line..."))
        }
        let rows = EngineV3Fixes.attach([row("\\alpah"), row("\\2"), row("\\qqqqqqqq")], texts: ["main.tex": text])
        // The old compiler's exact wording and edit (crates/compiler/tests/diagnostic_codes.rs, recovery.rs).
        XCTAssertEqual(rows[0].help?.message, "did you mean \\alpha?")
        XCTAssertEqual(rows[0].help?.replacement?.text, "\\alpha")
        XCTAssertEqual(rows[0].suggestion, "\\alpha")
        XCTAssertEqual(rows[0].notes, ["The control sequence at the end of the top line..."], "TeX's help is kept")
        XCTAssertEqual(rows[1].help?.message, "did you mean `2` (without the backslash)?")
        XCTAssertEqual(rows[1].help?.replacement?.text, "2")
        XCTAssertNil(rows[2].help?.replacement, "nothing close: TeX's row as it was")
        XCTAssertEqual(rows[2].help?.message, "The control sequence at the end of the top line...")
    }

    /// The same text and the same old-engine diagnostic on the v2 path and as
    /// a v3 row give the same caret fix; Tab applies it, one edit.
    func testCaretFixAndFixPreviewMatchTheOldPath() {
        let text = "Hello \\alpah world.\n"
        let start = 6, end = 12
        let old = RuntimeV1.Diagnostic(severity: .error, message: "\\alpah is not a defined command",
                                       source: .init(path: "main.tex", startByte: start, endByte: end), recovery: nil,
                                       code: "unknown_command", suggestion: "\\alpha",
                                       help: .init(message: "did you mean \\alpha?", replacement: .init(startByte: start, endByte: end, text: "\\alpha", path: "main.tex")))
        let v2 = ShellModel()
        v2.engineV3Enabled = false
        v2.replaceProject(entryText: text, named: "main.tex")
        v2.result = RuntimeV1.CompileResult(projectId: "p", revision: v2.editorRevision, status: .recovered, pages: [], diagnostics: [old], pdfPath: nil)
        v2.setCompiledDocuments(["main.tex": text])
        env.set("FLASHTEX_HOST", "none")
        let v3 = ShellModel()
        v3.replaceProject(entryText: text, named: "main.tex")
        v3.engineV3Enabled = true
        defer { v3.engineV3.stop() }
        let texRow = RuntimeV1.Diagnostic(severity: .error, message: "Undefined control sequence.",
                                          source: .init(path: "main.tex", startByte: start, endByte: end), recovery: nil,
                                          code: EngineV3Fixes.undefinedCode)
        v3.setEngineV3CompiledDocuments(["main.tex": text])
        v3.engineV3Diagnostics = EngineV3Fixes.attach([texRow], texts: ["main.tex": text])
        for m in [v2, v3] { m.caretUTF16 = 9 }
        let f2 = try? XCTUnwrap(v2.caretFix), f3 = try? XCTUnwrap(v3.caretFix)
        XCTAssertNotNil(f3)
        XCTAssertEqual(f3?.title, f2?.title)
        XCTAssertEqual(f3?.replacement, f2?.replacement)
        // "Fix…" previews the same edit.
        v2.previewQuickFix(diagnosticIndex: 0); v3.previewQuickFix(diagnosticIndex: 0)
        XCTAssertNotNil(v3.quickFix)
        XCTAssertEqual(v3.quickFix?.summary, v2.quickFix?.summary)
        v3.quickFix = nil
        // Tab applies it as one pending edit.
        v3.acceptCaretFix()
        XCTAssertEqual(v3.pendingEdit?.text, "\\alpha")
        // Once the editor moves past the compiled text, no fix is offered.
        v3.updateActiveText("Hello \\alpah world!\n")
        XCTAssertNil(v3.caretFix)
    }

    func testTeXsMissingFileWordingOffersCreate() {
        XCTAssertEqual(MissingIncludeFix.requested(from: "LaTeX Error: File `chap.tex' not found."), "chap")
        XCTAssertNil(MissingIncludeFix.requested(from: "LaTeX Error: File `pkg.sty' not found."), "a package is not an include")
        XCTAssertEqual(ProjectPackagesState.unresolvedNames(in: [
            .init(severity: .error, message: "LaTeX Error: File `mystyle.sty' not found.", source: nil, recovery: nil),
            .init(severity: .error, message: "LaTeX Error: File `myclass.cls' not found.", source: nil, recovery: nil),
            .init(severity: .error, message: "LaTeX Error: File `chap.tex' not found.", source: nil, recovery: nil),
        ]), ["mystyle", "myclass"])
        // The old compiler's wording still matches.
        XCTAssertEqual(MissingIncludeFix.requested(from: "included file not found: looked for 'x' and 'x.tex'"), "x")
    }

    // MARK: with a host

    func compiled(_ doc: String) async throws -> (ShellModel, URL) {
        try EngineV3TestHost.require()
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-fixes-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        try doc.write(to: dir.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let m = ShellModel()
        XCTAssertEqual(m.openTex(at: dir.appendingPathComponent("main.tex"), dirty: .discard), .opened)
        m.engineV3Enabled = true
        let s = m.engineV3
        s.start(model: m)
        try await EngineV3TestHost.awaitReady(s)
        let start = Date()
        while s.statusNote.isEmpty || s.compiling || m.engineV3Diagnostics.isEmpty {
            if Date().timeIntervalSince(start) > 120 { XCTFail("timeout: \(s.statusNote)"); break }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        return (m, dir)
    }

    func testAMistypedCommandIsFixedWithTabUnderV3() async throws {
        let doc = "\\documentclass{article}\n\\begin{document}\n\\textbff{Bold} text.\n\\end{document}\n"
        let (m, dir) = try await compiled(doc)
        defer { m.engineV3.stop(); try? FileManager.default.removeItem(at: dir) }
        let row = try XCTUnwrap(m.engineV3Diagnostics.first { $0.code == EngineV3Fixes.undefinedCode })
        XCTAssertEqual(row.help?.message, "did you mean \\textbf?")
        m.caretUTF16 = (doc as NSString).range(of: "\\textbff").location + 3
        let fix = try XCTUnwrap(m.caretFix)
        XCTAssertEqual(fix.replacement, "\\textbf")
        m.acceptCaretFix()
        XCTAssertEqual(m.pendingEdit?.text, "\\textbf")
        XCTAssertEqual(m.pendingEdit?.nsRange, (doc as NSString).range(of: "\\textbff"))
    }

    func testAMissingInputOffersCreateUnderV3() async throws {
        let doc = "\\documentclass{article}\n\\begin{document}\nIntro. \\input{chapone}\n\\end{document}\n"
        let (m, dir) = try await compiled(doc)
        defer { m.engineV3.stop(); try? FileManager.default.removeItem(at: dir) }
        let row = try XCTUnwrap(m.engineV3Diagnostics.first { $0.code == "latex/file-not-found" })
        let fix = try XCTUnwrap(MissingIncludeFix.quickFix(for: row, projectRoot: m.project.projectRoot))
        XCTAssertEqual(fix.path, "chapone.tex")
        XCTAssertEqual(fix.from, "main.tex")
    }

    func testAMissingPackageOffersCreateUnderV3() async throws {
        let doc = "\\documentclass{article}\n\\usepackage{mynotes}\n\\begin{document}\nText.\n\\end{document}\n"
        let (m, dir) = try await compiled(doc)
        defer { m.engineV3.stop(); try? FileManager.default.removeItem(at: dir) }
        let row = try XCTUnwrap(m.engineV3Diagnostics.first { $0.code == "latex/file-not-found" })
        XCTAssertEqual(ProjectPackagesState.missingPackages(for: row, projectRoot: m.project.projectRoot), ["mynotes"])
    }
}
