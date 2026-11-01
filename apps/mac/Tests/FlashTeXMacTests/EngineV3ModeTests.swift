import XCTest
@testable import FlashTeXMac

/// Unicode mode in the new preview (EngineV3Mode.swift, modes PROPOSAL.md
/// §4.1, XETEX-S3): the `% !TEX program` line, the resolution order, the
/// host each mode starts and the format it asks for.
@MainActor
final class EngineV3ModeTests: XCTestCase {
    private var env = EnvironmentOverride()

    override func tearDown() { env.restore() }

    func testMagicComments() {
        let u = EngineV3Mode.Program.known(.unicode, format: "xelatex")
        XCTAssertEqual(EngineV3Mode.magicComment("% !TEX program = xelatex\n\\documentclass{article}"), u)
        XCTAssertEqual(EngineV3Mode.magicComment("%!TEX TS-program = XeLaTeX\r\n\\documentclass{article}"), u)
        XCTAssertEqual(EngineV3Mode.magicComment("\u{feff}% !TEX program = xelatex\n"), u, "after a byte-order mark")
        XCTAssertEqual(EngineV3Mode.magicComment("% ! TeX program = xelatex\n"), u, "VS Code's spelling")
        XCTAssertEqual(EngineV3Mode.magicComment("% !TeX program=pdflatex\n"), .known(.classic, format: "pdflatex"))
        XCTAssertEqual(EngineV3Mode.magicComment("\n% a comment\n% !TEX program = xetex\n"), .known(.unicode, format: "xetex"), "plain XeTeX")
        XCTAssertNil(EngineV3Mode.magicComment("\\documentclass{article}\n% !TEX program = xelatex\n"), "only the leading comments")
        XCTAssertEqual(EngineV3Mode.magicComment("% !TEX program = lualatex\n"), .unsupported("lualatex"))
        XCTAssertNil(EngineV3Mode.magicComment("% !TEX root = main.tex\n"))
    }

    func testResolutionOrder() {
        let doc = "% !TEX program = xelatex\n\\documentclass{article}"
        XCTAssertEqual(EngineV3Mode.resolve(environment: nil, manifest: nil, mainText: doc).mode, .unicode)
        XCTAssertEqual(EngineV3Mode.resolve(environment: nil, manifest: "classic", mainText: doc).mode, .classic, "the manifest outranks the line")
        XCTAssertEqual(EngineV3Mode.resolve(environment: "unicode", manifest: "classic", mainText: nil).mode, .unicode, "the environment outranks the manifest")
        XCTAssertEqual(EngineV3Mode.resolve(environment: "", manifest: nil, mainText: "\\documentclass{x}").source, "default")
        let native = EngineV3Mode.resolve(environment: nil, manifest: "flashtex", mainText: doc)
        XCTAssertEqual(native.mode, .classic)
        XCTAssertTrue(native.warning?.contains("not available") == true, "\(native)")
        // a manifest value this version does not know: Classic, said, and
        // the line does not overrule it
        let bad = EngineV3Mode.resolve(environment: nil, manifest: nil, manifestWarning: "project.mode: expected classic", mainText: doc)
        XCTAssertEqual(bad.mode, .classic)
        XCTAssertNotNil(bad.warning)
        let lua = EngineV3Mode.resolve(environment: nil, manifest: nil, mainText: "% !TEX program = lualatex\n")
        XCTAssertEqual(lua.mode, .classic)
        XCTAssertTrue(lua.warning?.contains("LuaTeX isn't supported") == true, "\(lua)")
        XCTAssertEqual(EngineV3Mode.resolve(environment: nil, manifest: nil, mainText: "% !TEX program = xetex\n").format, "xetex")
    }

    func testEachModeHasItsHostAndFormat() throws {
        XCTAssertEqual(EngineV3Mode.classic.format, "pdflatex")
        XCTAssertEqual(EngineV3Mode.unicode.format, "xelatex")
        let exe = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-host-unicode-\(getpid())")
        try Data("#!/bin/sh\n".utf8).write(to: exe)
        defer { try? FileManager.default.removeItem(at: exe) }
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: exe.path)
        env.set("FLASHTEX_HOST_UNICODE", exe.path)
        env.set("FLASHTEX_HOST", "none")
        XCTAssertEqual(EngineV3.locateHost(mode: .unicode)?.path, exe.path)
        XCTAssertNil(EngineV3.locateHost(mode: .classic), "each mode its own variable")
        XCTAssertEqual(EngineV3HostProcess.hostLine("flashtex-host-unicode: listening on /tmp/x.sock"), "listening on /tmp/x.sock")
        XCTAssertEqual(EngineV3HostProcess.hostLine("flashtex-host: {\"a\":1}"), "{\"a\":1}")
        XCTAssertNil(EngineV3HostProcess.hostLine("stderr: flashtex-host: x"))
    }

    // MARK: with the hosts (skipped without them)

    static let unicodeDoc = """
    % !TEX program = xelatex
    \\documentclass{article}
    \\usepackage{fontspec}
    \\begin{document}
    Unicode: naïve café — façade.
    \\end{document}

    """

    private func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    /// A `% !TEX program = xelatex` document starts flashtex-host-unicode
    /// and asks for the xelatex format; its pages arrive like Classic's.
    /// Taking the line out relaunches the Classic host (when one is built).
    func testAUnicodeDocumentRunsInTheUnicodeHost() async throws {
        // Required where CI builds the Unicode host (FLASHTEX_REQUIRE_HOST_UNICODE=1,
        // ci.yml's mac-v3-host once crates/flashtex-xetex has it); skipped
        // otherwise, FLASHTEX_REQUIRE_HOST (Classic's host) notwithstanding.
        let required = ProcessInfo.processInfo.environment["FLASHTEX_REQUIRE_HOST_UNICODE"] == "1"
        func unavailable(_ why: String) -> Error {
            guard required else { return XCTSkip(why) }
            XCTFail("FLASHTEX_REQUIRE_HOST_UNICODE=1: " + why)
            return EngineV3TestHost.HostMissing(description: why)
        }
        guard EngineV3.locateHost(mode: .unicode) != nil else {
            throw unavailable("no flashtex-host-unicode built (cd crates/flashtex-xetex && cargo build --release --bin flashtex-host-unicode)")
        }
        guard EngineV3TestHost.texLive != nil else { throw unavailable("no TeX Live") }
        let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-mode-\(getpid())")
        env.set("FLASHTEX_V3_CACHE", cache.path)
        env.set("FLASHTEX_MODE", "")
        defer { try? FileManager.default.removeItem(at: cache) }
        let model = ShellModel()
        model.replaceProject(entryText: Self.unicodeDoc, named: "main.tex")
        model.engineV3Enabled = true
        model.autoCompile = true
        let s = model.engineV3
        defer { s.stop() }
        s.start(model: model)
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the Unicode compile") { s.statusNote.hasPrefix("ok") && s.pageCount == 1 && s.pages[0] != nil && !s.compiling }
        XCTAssertEqual(s.hostMode, .unicode)
        guard EngineV3.locateHost(mode: .classic) != nil else { return }
        let n = s.doneCount
        model.updateActiveText("\\documentclass{article}\n\\begin{document}\nClassic.\n\\end{document}\n")
        try await waitUntil("the Classic relaunch") { s.hostMode == .classic && s.doneCount > n && s.statusNote.hasPrefix("ok") && !s.compiling }
        XCTAssertEqual(s.hostMode, .classic, "\(s.phase) \(s.statusNote) done \(s.doneCount) (was \(n)) compiling \(s.compiling)")
    }
}
