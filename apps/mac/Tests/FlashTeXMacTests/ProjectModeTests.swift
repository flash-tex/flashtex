import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The mode item, the switch and the suggestion (ProjectMode.swift, modes
/// PROPOSAL.md §4.2-§4.5, M5): the switch writes `[project] mode` through
/// the helper's `set_mode` and the rooted save, and only when asked; a
/// document that needs Unicode fonts, or `[fonts]` in a Classic project,
/// gets a one-click suggestion; in Unicode mode neither sends the document
/// to the compatibility engine. The real-helper case needs the built helper
/// and skips otherwise.
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

    func testTheSuggestionNeedsClassicModeAndAUnicodeNeed() throws {
        try fakeUnicodeHost()
        let model = ShellModel()
        model.detachWorker()
        model.replaceProject(entryText: "\\documentclass{article}\n\\usepackage{fontspec}\n\\begin{document}\nx\n\\end{document}\n", named: "main.tex")
        XCTAssertEqual(model.documentMode.mode, .classic)
        model.engineChoice = EngineChoice(preferred: .new, source: .builtInDefault,
                                          blocker: .unicodeFonts(UnicodeFontsNeed(kind: .package("fontspec"), file: "main.tex")))
        let m = try XCTUnwrap(UnicodeModeSuggestionBanner.message(model))
        XCTAssertTrue(m.headline.contains("the fontspec package"), m.headline)
        XCTAssertTrue(EngineFallbackBanner.coveredBySuggestion(model.engineChoice.blocker!, model) == (model.modeSwitchRefusal == nil))
        XCTAssertEqual(model.unicodeModeSuggestion?.what, "the fontspec package")
        // a `% !TEX program = xelatex` line: Unicode mode, no suggestion
        model.replaceProject(entryText: "% !TEX program = xelatex\n\\documentclass{article}\n\\usepackage{fontspec}\n\\begin{document}\nx\n\\end{document}\n", named: "main.tex")
        XCTAssertEqual(model.documentMode.mode, .unicode)
        XCTAssertNil(UnicodeModeSuggestionBanner.message(model))
        XCTAssertNil(model.unicodeModeSuggestion)
    }

    func testUnicodeModeLiftsTheUnicodeFontsRule() throws {
        try fakeUnicodeHost()
        let model = ShellModel()
        model.detachWorker()
        model.replaceProject(entryText: "% !TEX program = xelatex\n\\documentclass{article}\n\\usepackage{fontspec}\n\\begin{document}\nx\n\\end{document}\n", named: "main.tex")
        // (no TeX Live on the machine would be the first rule; the rest needs one)
        guard EngineChoice.texLiveAvailable() else { throw XCTSkip("no TeX Live: the no-TeX-Live rule applies first") }
        XCTAssertNil(model.engineBlocker(), "Unicode mode typesets fontspec documents itself")
        env.set("FLASHTEX_HOST_UNICODE", "none")
        XCTAssertFalse(model.unicodeModeAvailable)
        XCTAssertNotNil(model.engineBlocker(), "without the Unicode host, the compatibility engine still does")
    }

    func testTheItemsHelpSaysTheModeItsSourceAndTheSuggestion() {
        let r = EngineV3Mode.Resolution(mode: .classic, format: "pdflatex", source: "default", warning: nil)
        let help = ModeStatusItem.help(r, suggestion: UnicodeFontsNeed(kind: .command("setmainfont"), file: nil))
        XCTAssertTrue(help.contains("Classic (pdfLaTeX-compatible)") && help.contains("Default") && help.contains("\\setmainfont"), help)
        XCTAssertEqual(EngineV3Mode.Resolution(mode: .unicode, format: "xelatex", source: "flashtex.toml", warning: nil).sourceLine, "Set in flashtex.toml")
    }

    func testRealHelperWritesTheModeOnlyWhenAsked() async throws {
        guard let helper = DocumentFilesTests.realHelper else {
            throw XCTSkip("build crates/project-files (cargo build --release) or set FLASHTEX_PROJECT_FILES")
        }
        try fakeUnicodeHost()
        _ = try write("main.tex", "\\documentclass{article}\n\\usepackage{fontspec}\n\\begin{document}\nHello.\n\\end{document}\n")
        let model = ShellModel()
        model.detachWorker()
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
    }

    func testFlashtexModeInTheEnvironmentRefusesTheSwitch() throws {
        env.set("FLASHTEX_MODE", "unicode")
        let model = ShellModel()
        model.detachWorker()
        model.replaceProject(entryText: "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n", named: "main.tex")
        XCTAssertNotNil(model.modeSwitchRefusal)
        XCTAssertNotNil(model.setProjectMode(.classic))
    }
}
