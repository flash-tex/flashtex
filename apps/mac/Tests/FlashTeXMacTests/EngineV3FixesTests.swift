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
        let rows = EngineV3Fixes.attach([row("\\alpah"), row("\\2"), row("\\qqqqqqqq")], named: ["\\alpah", "\\2", "\\qqqqqqqq"],
                                        texts: ["main.tex": text])
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

    /// TeX's top line names the undefined control sequence; inside a
    /// macro the range is the macro call, so no fix is offered (it would
    /// rewrite a correct user macro), and none when the range is not the
    /// named control sequence.
    func testNoFixWhenTheRangeIsNotTheUndefinedName() {
        XCTAssertEqual(EngineV3Fixes.undefinedName(trace: [(kind: "file", before: "Hello \\textbff")]), "\\textbff")
        XCTAssertEqual(EngineV3Fixes.undefinedName(trace: [(kind: "file", before: "\\2 ")]), "\\2")
        XCTAssertNil(EngineV3Fixes.undefinedName(trace: [(kind: "macro", before: "\\textbff"), (kind: "file", before: "\\mycite")]))
        XCTAssertNil(EngineV3Fixes.undefinedName(trace: []))
        let text = "x \\mycite y"
        let row = RuntimeV1.Diagnostic(severity: .error, message: "Undefined control sequence.",
                                       source: .init(path: "main.tex", startByte: 2, endByte: 9), recovery: nil,
                                       code: EngineV3Fixes.undefinedCode)
        XCTAssertNil(EngineV3Fixes.fix(row, named: "\\textbff", texts: ["main.tex": text]).help, "the range is \\mycite, TeX names \\textbff")
        XCTAssertNil(EngineV3Fixes.fix(row, named: nil, texts: ["main.tex": text]).help, "a macro frame: no name, no fix")
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
        v3.engineV3Diagnostics = EngineV3Fixes.attach([texRow], named: ["\\alpah"], texts: ["main.tex": text])
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
        // A class is created as a class, on both paths.
        XCTAssertEqual(ProjectPackagesState.unresolvedFiles(in: [
            .init(severity: .error, message: "LaTeX Error: File `mystyle.sty' not found.", source: nil, recovery: nil),
            .init(severity: .error, message: "LaTeX Error: File `myclass.cls' not found.", source: nil, recovery: nil),
            .init(severity: .error, message: "x", source: nil, recovery: nil, notes: ["no project file found: looked for oldclass.cls"]),
        ]), ["mystyle.sty", "myclass.cls", "oldclass.cls"])
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

    /// A typo inside a user macro's definition is reported at the macro's
    /// call: `\mycite` must never be "fixed" to `\nocite`, nor `\mysec`
    /// (containing `\secton`) or `\myemph` (containing `\emp`) rewritten.
    func testATypoInsideAMacroOffersNoFixForTheCall() async throws {
        let doc = """
        \\documentclass{article}
        \\newcommand\\mycite{\\textbff x}
        \\newcommand\\mysec{\\secton{A}}
        \\newcommand\\myemph{\\emp{b}}
        \\begin{document}
        \\mycite\\ \\mysec\\ \\myemph
        \\end{document}

        """
        let (m, dir) = try await compiled(doc)
        defer { m.engineV3.stop(); try? FileManager.default.removeItem(at: dir) }
        let rows = m.engineV3Diagnostics.filter { $0.code == EngineV3Fixes.undefinedCode }
        XCTAssertEqual(rows.count, 3, "\(m.engineV3Diagnostics.map(\.message))")
        for r in rows {
            // The fix (if any) is in the definition (EngineV3DiagPresent), never on the call.
            if let f = r.help?.replacement, let s = r.source { XCTAssertTrue(f.endByte < s.startByte || f.startByte > s.endByte, "no fix on a macro call: \(r)") }
            XCTAssertNil(r.suggestion)
        }
        for cs in ["\\mycite", "\\mysec", "\\myemph"] {
            // The call in the body: the last occurrence (the definitions come first).
            let at = (doc as NSString).range(of: cs, options: .backwards)
            XCTAssertNotEqual(at.location, NSNotFound)
            m.caretUTF16 = at.location + 2
            XCTAssertNil(m.caretFix, "no Tab fix on \(cs)")
        }
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

    /// `\documentclass{myclass}` with no myclass.cls: "Create myclass.cls"
    /// (from the class template), not myclass.sty.
    func testAMissingClassOffersCreateClassUnderV3() async throws {
        let doc = "\\documentclass{myclass}\n\\begin{document}\nText.\n\\end{document}\n"
        let (m, dir) = try await compiled(doc)
        defer { m.engineV3.stop(); try? FileManager.default.removeItem(at: dir) }
        let row = try XCTUnwrap(m.engineV3Diagnostics.first { $0.code == "latex/file-not-found" })
        XCTAssertEqual(ProjectPackagesState.missingFiles(for: row, projectRoot: m.project.projectRoot), ["myclass.cls"])
        XCTAssertEqual(ProjectPackagesState.missingPackages(for: row, projectRoot: m.project.projectRoot), [], "not a package")
        let created = await m.createPackageFile(named: "myclass", class: true)
        guard case .created(let path) = created else { return XCTFail("\(created)") }
        XCTAssertEqual(path, "myclass.cls")
        let text = try String(contentsOf: dir.appendingPathComponent("myclass.cls"), encoding: .utf8)
        XCTAssertTrue(text.contains("\\ProvidesClass{myclass}"), text)
    }

    /// A compile's DONE starts a package resolution; when the window (its
    /// ShellModel) is gone before that task runs, the task does nothing.
    /// It used to read the unowned model and trap ("Attempted to read an
    /// unowned reference but object ... was already deallocated"), which
    /// crashed the test process after the tests above.
    func testAResolutionStartedByACompileOutlivingItsWindowDoesNothing() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-fixes-gone-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let main = dir.appendingPathComponent("main.tex")
        try "\\documentclass{article}\n\\usepackage{nosuchpkg}\n\\begin{document}\nX.\n\\end{document}\n".write(to: main, atomically: true, encoding: .utf8)
        weak var gone: ShellModel?
        do {
            let m = ShellModel()
            XCTAssertEqual(m.openTex(at: main), .opened)
            m.engineV3Enabled = true
            m.projectPackages.noteCompileResult(diagnostics: [
                .init(severity: .error, message: "LaTeX Error: File `nosuchpkg.sty' not found.", source: nil, recovery: nil),
            ])
            m.engineV3Enabled = false
            gone = m
        }
        try await Task.sleep(nanoseconds: 300_000_000) // the resolution task runs
        if gone != nil { throw XCTSkip("the model outlived its scope here; nothing to check") }
    }

    func testAMissingPackageOffersCreateUnderV3() async throws {
        let doc = "\\documentclass{article}\n\\usepackage{mynotes}\n\\begin{document}\nText.\n\\end{document}\n"
        let (m, dir) = try await compiled(doc)
        defer { m.engineV3.stop(); try? FileManager.default.removeItem(at: dir) }
        let row = try XCTUnwrap(m.engineV3Diagnostics.first { $0.code == "latex/file-not-found" })
        XCTAssertEqual(ProjectPackagesState.missingPackages(for: row, projectRoot: m.project.projectRoot), ["mynotes"])
    }
}
