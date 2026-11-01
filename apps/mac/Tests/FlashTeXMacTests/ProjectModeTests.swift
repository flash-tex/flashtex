import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The mode item, the switch and the routing (ProjectMode.swift, modes
/// PROPOSAL.md §4.2-§4.5, M5; UnicodeFonts.swift): the switch writes
/// `[project] mode` through the helper's `set_mode` and the rooted save, and
/// only when asked; a document that needs Unicode fonts and whose mode
/// nothing sets is typeset in Unicode mode (in memory); one set to Classic
/// stays Classic and the mode item marks the need. The real-helper case
/// needs the built helper and skips otherwise.
@MainActor
final class ProjectModeTests: XCTestCase {
    private var tmp: URL!
    private var env = EnvironmentOverride()

    override func setUp() {
        OwnerStateGuard.install()
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("project-mode-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        env.set("FLASHTEX_MODE", "")
    }

    override func tearDown() {
        env.restore()
        try? FileManager.default.removeItem(at: tmp)
    }

    private func write(_ rel: String, _ text: String) throws -> URL {
        let url = tmp.appendingPathComponent(rel)
        try text.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    /// A stand-in for flashtex-host-unicode, so that Unicode mode counts as available.
    private func fakeUnicodeHost() throws {
        let exe = tmp.appendingPathComponent("flashtex-host-unicode")
        try Data("#!/bin/sh\n".utf8).write(to: exe)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: exe.path)
        env.set("FLASHTEX_HOST_UNICODE", exe.path)
    }

    /// A document that needs Unicode fonts and whose mode nothing sets is
    /// typeset in Unicode mode, decided in memory (source "detected"); a
    /// mode the document sets is never overruled: Classic set so keeps the
    /// document in Classic mode and only marks the need (the status item).
    func testANeedRoutesAnUnsetModeAndMarksAClassicOne() throws {
        try fakeUnicodeHost()
        let model = ShellModel()
        let body = "\\documentclass{article}\n\\usepackage{fontspec}\n\\begin{document}\nx\n\\end{document}\n"
        model.replaceProject(entryText: body, named: "main.tex")
        XCTAssertEqual(model.documentMode.mode, .unicode)
        XCTAssertEqual(model.documentMode.source, EngineV3Mode.detectedSource)
        XCTAssertEqual(model.unicodeRouting, "the document uses the fontspec package")
        XCTAssertNil(model.unicodeModeSuggestion, "routed: nothing to mark")
        // `% !TEX program = pdflatex` sets Classic: kept, the need marked
        model.replaceProject(entryText: "% !TEX program = pdflatex\n" + body, named: "main.tex")
        _ = model.unicodeFontsNeed() // the scan at an open
        XCTAssertEqual(model.documentMode.mode, .classic)
        XCTAssertEqual(model.documentMode.source, "% !TEX program")
        XCTAssertEqual(model.unicodeModeSuggestion?.what, "the fontspec package")
        // `% !TEX program = xelatex`: Unicode mode, set by the document; no mark
        model.replaceProject(entryText: "% !TEX program = xelatex\n" + body, named: "main.tex")
        XCTAssertEqual(model.documentMode.mode, .unicode)
        XCTAssertEqual(model.documentMode.source, "% !TEX program")
        XCTAssertNil(model.unicodeModeSuggestion)
    }

    /// Without Unicode mode's engine there is nothing to route to: the
    /// document stays in Classic mode (and shows TeX's own error).
    func testWithoutTheUnicodeHostANeedStaysInClassicMode() throws {
        env.set("FLASHTEX_HOST_UNICODE", "none")
        let model = ShellModel()
        model.replaceProject(entryText: "\\documentclass{article}\n\\usepackage{fontspec}\n\\begin{document}\nx\n\\end{document}\n", named: "main.tex")
        XCTAssertFalse(model.unicodeModeAvailable)
        XCTAssertNil(model.unicodeRouting)
        XCTAssertEqual(model.documentMode.mode, .classic)
        XCTAssertEqual(model.documentMode.source, "default")
    }

    func testTheItemsHelpSaysTheModeItsSourceAndTheSuggestion() {
        let r = EngineV3Mode.Resolution(mode: .classic, format: "pdflatex", source: "default", warning: nil)
        let help = ModeStatusItem.help(r, suggestion: UnicodeFontsNeed(kind: .command("setmainfont"), file: nil))
        XCTAssertTrue(help.contains("Classic (pdfLaTeX-compatible)") && help.contains("Default") && help.contains("\\setmainfont"), help)
        XCTAssertEqual(EngineV3Mode.Resolution(mode: .unicode, format: "xelatex", source: "flashtex.toml", warning: nil).sourceLine, "Set in flashtex.toml")
        XCTAssertTrue(EngineV3Mode.Resolution(mode: .unicode, format: "xelatex", source: EngineV3Mode.detectedSource, warning: nil)
            .sourceLine.contains("not saved in flashtex.toml"))
    }

    func testRealHelperWritesTheModeOnlyWhenAsked() async throws {
        guard let helper = DocumentFilesTests.realHelper else {
            throw XCTSkip("build crates/project-files (cargo build --release) or set FLASHTEX_PROJECT_FILES")
        }
        try fakeUnicodeHost()
        _ = try write("main.tex", "\\documentclass{article}\n\\usepackage{fontspec}\n\\begin{document}\nHello.\n\\end{document}\n")
        let model = ShellModel()
        model.files.policy = .executable(helper, arguments: [])
        XCTAssertEqual(model.openTex(at: tmp.appendingPathComponent("main.tex")), .opened)
        XCTAssertFalse(FileManager.default.fileExists(atPath: tmp.appendingPathComponent("flashtex.toml").path), "opening writes nothing")
        XCTAssertNil(model.setProjectMode(.unicode))
        let written = try String(contentsOf: tmp.appendingPathComponent("flashtex.toml"), encoding: .utf8)
        XCTAssertTrue(written.contains("\nmode = \"unicode\"\n") && written.contains("entry = \"main.tex\""), written)
        // read back through the helper
        for _ in 0 ..< 100 where model.documentMode.mode != .unicode { try await Task.sleep(nanoseconds: 20_000_000) }
        XCTAssertEqual(model.documentMode.mode, .unicode)
        XCTAssertEqual(model.documentMode.source, "flashtex.toml")
        // and back, the rest of the file kept
        XCTAssertNil(model.setProjectMode(.classic))
        let again = try String(contentsOf: tmp.appendingPathComponent("flashtex.toml"), encoding: .utf8)
        XCTAssertEqual(again, written.replacingOccurrences(of: "mode = \"unicode\"", with: "mode = \"classic\""))
        for _ in 0 ..< 100 where model.documentMode.source != "flashtex.toml" || model.documentMode.mode != .classic {
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        // Classic set in flashtex.toml is the user's choice: a fontspec
        // document stays in Classic mode (the status item marks the need)
        XCTAssertEqual(model.documentMode.mode, .classic)
        XCTAssertEqual(model.documentMode.source, "flashtex.toml")
        // flashtex.toml open with unsaved edits: refused, the file untouched
        let opened = await model.project.openDocument("flashtex.toml")
        XCTAssertEqual(opened, .opened(path: "flashtex.toml"))
        _ = model.project.switchDocument(to: "flashtex.toml")
        model.updateActiveText(again + "# my unsaved note\n")
        let refusal = model.setProjectMode(.unicode)
        XCTAssertTrue(refusal?.contains("unsaved edits") == true, refusal ?? "nil")
        XCTAssertEqual(try String(contentsOf: tmp.appendingPathComponent("flashtex.toml"), encoding: .utf8), again)
    }

    /// The owner's report: a document whose main file says `% !TEX program =
    /// xelatex` (Unicode from the magic comment); choosing Classic in the
    /// mode menu must write `mode = "classic"` to the user's own folder's
    /// flashtex.toml, which then wins over the comment.
    func testChoosingClassicOverridesTheMagicCommentInTheUsersFolder() async throws {
        guard let helper = DocumentFilesTests.realHelper else {
            throw XCTSkip("build crates/project-files (cargo build --release) or set FLASHTEX_PROJECT_FILES")
        }
        try fakeUnicodeHost()
        let root = tmp.resolvingSymlinksInPath()
        _ = try write("main.tex", "% !TEX program = xelatex\n\n\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n")
        let model = ShellModel()
        model.files.policy = .executable(helper, arguments: [])
        XCTAssertEqual(model.openTex(at: root.appendingPathComponent("main.tex")), .opened)
        for _ in 0 ..< 100 where model.manifest.refreshes == 0 { try await Task.sleep(nanoseconds: 20_000_000) }
        XCTAssertEqual(model.documentMode.mode, .unicode)
        XCTAssertEqual(model.documentMode.source, "% !TEX program")
        XCTAssertNil(model.modeSwitchRefusal)
        XCTAssertFalse(ModeMenuItems.isDisabled(.classic, mode: model.documentMode, refusal: nil, unicodeAvailable: true))
        XCTAssertNil(model.setProjectMode(.classic))
        let toml = root.appendingPathComponent("flashtex.toml")
        let written = try String(contentsOf: toml, encoding: .utf8)
        XCTAssertTrue(written.contains("mode = \"classic\""), written)
        for _ in 0 ..< 100 where model.documentMode.mode != .classic { try await Task.sleep(nanoseconds: 20_000_000) }
        XCTAssertEqual(model.documentMode.mode, .classic)
        XCTAssertEqual(model.documentMode.source, "flashtex.toml")
        XCTAssertNil(model.modeSwitchFailure)
        XCTAssertTrue(EngineV3Mode.Resolution(mode: .unicode, format: "xelatex", source: "% !TEX program", warning: nil)
            .menuExplanation(main: "main.tex").contains("% !TEX program in main.tex"))
    }

    /// A helper from before `set_mode` (a bundle that copied a stale build,
    /// the owner's case) refuses with "unknown operation": the refusal says
    /// so, and is shown (an alert), not only noted.
    func testARefusedSwitchIsShownAndAStaleHelperIsNamed() throws {
        let why = ShellModel.setModeFailure("unknown operation \"set_mode\"")
        XCTAssertTrue(why.contains("out of date"), why)
        env.set("FLASHTEX_MODE", "unicode")
        let model = ShellModel()
        model.replaceProject(entryText: "x\n", named: "main.tex")
        XCTAssertNotNil(model.setProjectMode(.classic))
        XCTAssertNotNil(model.modeSwitchFailure)
    }

    func testFlashtexModeInTheEnvironmentRefusesTheSwitch() throws {
        env.set("FLASHTEX_MODE", "unicode")
        let model = ShellModel()
        model.replaceProject(entryText: "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n", named: "main.tex")
        XCTAssertNotNil(model.modeSwitchRefusal)
        XCTAssertNotNil(model.setProjectMode(.classic))
    }
}
