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

    /// Both hosts are resident and take the S0 cache (each mode its own
    /// folder) and the performance mode; only the pdfTeX host takes the
    /// checkpoint interval. (The Unicode host was started without either,
    /// so a reopened Unicode document ran its preamble again.)
    func testBothHostsTakeTheS0CacheAndTheProfile() {
        for mode in [EngineV3Mode.classic, .unicode] {
            let s0 = EngineV3HostProcess.s0Directory(mode: mode)
            let a = EngineV3HostProcess.arguments(socketPath: "/tmp/x.sock", mode: mode, s0Directory: s0, profile: "low-memory", timed: "0.05")
            XCTAssertEqual(Array(a.prefix(3)), ["--socket", "/tmp/x.sock", "--once"], "\(mode)")
            XCTAssertEqual(a.firstIndex(of: "--s0-cache").map { a[$0 + 1] }, s0.path, "\(mode): \(a)")
            XCTAssertEqual(a.firstIndex(of: "--profile").map { a[$0 + 1] }, "low-memory", "\(mode): \(a)")
            XCTAssertEqual(a.contains("--timed"), mode == .classic, "\(mode): \(a)")
        }
        XCTAssertNotEqual(EngineV3HostProcess.s0Directory(mode: .classic), EngineV3HostProcess.s0Directory(mode: .unicode))
        XCTAssertFalse(EngineV3HostProcess.arguments(socketPath: "/s", mode: .classic, s0Directory: URL(fileURLWithPath: "/c"), profile: "balanced", timed: "x").contains("--timed"), "a malformed interval is dropped")
    }

    /// Each S0 folder (Classic's and Unicode's) is kept under its budget by
    /// one rule: least recently written S0 files first out; files that are
    /// not a host's S0 or S1 are neither counted nor removed.
    func testS0FoldersStayUnderTheirBudget() throws {
        let fm = FileManager.default
        let dir = fm.temporaryDirectory.appendingPathComponent("ftx-s0trim-\(UUID().uuidString)", isDirectory: true)
        try fm.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: dir) }
        let names = ["a.s0", "b.s1", "c.xs0", "d.xs0", "e.s0"] // oldest first
        let kept = ["old.tmp", "notes.txt", "f.xs0.123.456.0.tmp"]
        let base = Date(timeIntervalSince1970: 1_700_000_000)
        for (i, n) in (kept + names).enumerated() {
            let u = dir.appendingPathComponent(n)
            try Data(count: 100).write(to: u)
            try fm.setAttributes([.modificationDate: base.addingTimeInterval(Double(i) * 60)], ofItemAtPath: u.path)
        }
        try fm.createDirectory(at: dir.appendingPathComponent("z.s0", isDirectory: true), withIntermediateDirectories: true)
        EngineV3HostProcess.trimS0(dir, budget: 250)
        let left = Set(try fm.contentsOfDirectory(atPath: dir.path))
        XCTAssertEqual(left, Set(kept + ["d.xs0", "e.s0", "z.s0"]), "the two newest S0 files fit 250 bytes")
        EngineV3HostProcess.trimS0(dir, budget: 1000)
        XCTAssertEqual(Set(try fm.contentsOfDirectory(atPath: dir.path)), left, "under the budget nothing goes")
        XCTAssertEqual(EngineV3HostProcess.s0Extensions, ["s0", "s1", "xs0"])
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
