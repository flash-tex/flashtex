import Foundation
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// App-parity row A9, project and system fonts (`[fonts]`) under the new
/// engine (modes PROPOSAL.md §4.5, owner ruling Q5 and decision 3A): a
/// project in Unicode mode is typeset by the new engine, with the manifest's
/// `[fonts]` applied as `xelatex` applies the equivalent fontspec setup; a
/// `[fonts]` project whose mode nothing sets is routed to Unicode mode (in
/// memory, UnicodeFonts.swift); a change of `[fonts]` compiles again; and the
/// families the Fonts sheet and `\setmainfont{` offer come from the new
/// engine's own catalog (`flashtex-host-unicode --list-fonts`).
///
/// The first two tests need no host. The last two need a built
/// `flashtex-host-unicode`, a TeX Live and its `xelatex` (the oracle: the
/// expected font is whatever TeX Live's `xelatex` loads, never this app's
/// output); they skip without them, and fail with
/// FLASHTEX_REQUIRE_HOST_UNICODE=1, as ci.yml's mac-v3-host sets it.
@MainActor
final class EngineV3ProjectFontsTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-fonts-\(getpid())")
    private var env = EnvironmentOverride()
    private var dirs: [URL] = []
    private var files: [URL] = []

    /// A system font every macOS has (/System/Library/Fonts/Supplemental),
    /// not the class default.
    static let family = "Georgia"

    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        env.set("FLASHTEX_ENGINE_V3", "")
        env.set("FLASHTEX_MODE", "")
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
        EngineV3Bundle.forgetConsent()
    }

    override func tearDown() {
        for f in files { EngineChoiceStore.set(nil, for: f) }
        EngineV3Bundle.forgetConsent()
        for d in dirs { try? FileManager.default.removeItem(at: d) }
        env.restore()
    }

    override class func tearDown() { try? FileManager.default.removeItem(at: cache) }

    // MARK: helpers

    private func dir(_ name: String) throws -> URL {
        let d = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-fonts-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        dirs.append(d)
        return d.resolvingSymlinksInPath()
    }

    private func executable(_ url: URL, _ script: String) throws {
        try Data(script.utf8).write(to: url)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path)
    }

    /// The helper's `manifest` payload for `root`, with `[project] mode` and `[fonts]`.
    private static func manifest(root: URL, mode: String?, fonts: [String: String]) -> ProjectFilesV1.Manifest {
        let table = ["text", "math", "mono", "sans"].map { "\"\($0)\":" + (fonts[$0].map { "\"\($0)\"" } ?? "null") }.joined(separator: ",")
        let modeField = mode.map { ",\"mode\":\"\($0)\"" } ?? ""
        let json = """
        {"path":"\(root.path)/flashtex.toml","exists":true,"manifest_dir":"\(root.path)",
         "manifest":{"project":{"entry":"main.tex","texinputs":[],"output":null\(modeField)},"fonts":{\(table)},
                     "packages":{"source":"ctan","fetch":"ask","pin":{},"path":{}},"library":null},
         "warnings":[],"texinputs":[],"files":[],"diagnostics":[],"template":""}
        """
        return try! JSONDecoder().decode(ProjectFilesV1.Manifest.self, from: Data(json.utf8))
    }

    /// The flashtex.toml the helper would have read that payload from (the
    /// Unicode host reads this file itself, from the project's copy).
    private static func toml(mode: String?, fonts: [String: String]) -> String {
        var s = "[project]\nentry = \"main.tex\"\n" + (mode.map { "mode = \"\($0)\"\n" } ?? "")
        if !fonts.isEmpty { s += "[fonts]\n" + fonts.sorted { $0.key < $1.key }.map { "\($0.key) = \"\($0.value)\"\n" }.joined() }
        return s
    }

    /// A model whose manifest reads are `manifest()` (the helper's answer).
    private func model(_ manifest: @escaping () -> ProjectFilesV1.Manifest?) -> ShellModel {
        let m = ShellModel()
        m.manifest.reader = { _, _ in manifest().map { .success($0) } ?? .failure(.init("no manifest (test)")) }
        return m
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 180, debug: () -> String = { "" }, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what) \(debug())"); throw EngineV3TestHost.HostNotReady(description: what) }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    // MARK: no host

    /// A `[fonts]` project in Unicode mode is the new engine's, with no
    /// announcement; one whose mode nothing sets is routed to Unicode mode
    /// in memory (only Unicode mode applies `[fonts]`), never written to
    /// flashtex.toml; Classic chosen in flashtex.toml is the user's and stays
    /// (Classic, like pdfLaTeX, does not apply `[fonts]`, PROPOSAL §4.5). No
    /// other engine takes over in any of them.
    func testAFontsProjectInUnicodeModeIsTypesetByTheNewEngine() throws {
        let bin = try dir("texbin")
        FileManager.default.createFile(atPath: bin.appendingPathComponent("kpsewhich").path, contents: Data())
        env.set("FLASHTEX_TEXLIVE_BIN", bin.path)
        env.set("FLASHTEX_HOST", "none")
        let host = bin.appendingPathComponent("flashtex-host-unicode")
        try executable(host, "#!/bin/sh\nexit 2\n")
        env.set("FLASHTEX_HOST_UNICODE", host.path)
        let root = try dir("unicode")
        let main = root.appendingPathComponent("main.tex")
        try "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n".write(to: main, atomically: true, encoding: .utf8)
        files.append(main)
        var current = Self.manifest(root: root, mode: "unicode", fonts: ["text": Self.family, "math": "Latin Modern Math"])
        let m = model { current }
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: main, dirty: .discard), .opened)
        m.engineV3Enabled = true // the session (the Unicode host stub exits at once)
        XCTAssertEqual(m.documentMode.mode, .unicode)
        XCTAssertEqual(m.documentMode.source, "flashtex.toml")
        XCTAssertTrue(m.engineV3Enabled, "the new engine typesets it")
        XCTAssertTrue(m.engineAnnouncements.isEmpty, "\(m.engineAnnouncements)")
        XCTAssertNil(m.unicodeModeSuggestion, "nothing to mark in Unicode mode")

        // Classic, chosen in flashtex.toml: kept; the new engine still typesets it.
        current = Self.manifest(root: root, mode: "classic", fonts: ["text": Self.family])
        m.manifest.refresh()
        XCTAssertEqual(m.documentMode.mode, .classic)
        XCTAssertEqual(m.documentMode.source, "flashtex.toml")
        XCTAssertTrue(m.engineV3Enabled, "no other engine takes over")

        // No mode set: `[fonts]` routes the project to Unicode mode, in memory.
        current = Self.manifest(root: root, mode: nil, fonts: ["text": Self.family])
        m.manifest.refresh()
        XCTAssertEqual(m.documentMode.mode, .unicode)
        XCTAssertEqual(m.documentMode.source, EngineV3Mode.detectedSource)
        XCTAssertEqual(m.unicodeRouting, "flashtex.toml sets [fonts] (text)")
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("flashtex.toml").path), "the routing is not written")
        XCTAssertTrue(m.engineV3Enabled)

        // Back to Unicode, set in flashtex.toml.
        current = Self.manifest(root: root, mode: "unicode", fonts: ["text": Self.family])
        m.manifest.refresh()
        XCTAssertEqual(m.documentMode.mode, .unicode)
        XCTAssertEqual(m.documentMode.source, "flashtex.toml")
        XCTAssertTrue(m.engineV3Enabled)
    }

    /// The Fonts sheet and `\setmainfont{` completion list the families the
    /// new engine's catalog finds by name (`flashtex-host-unicode
    /// --list-fonts` and `--list-math-fonts`), not the previous engine's
    /// `flashtex-render`.
    func testTheFamiliesComeFromTheNewEngine() async throws {
        let bin = try dir("lister")
        let host = bin.appendingPathComponent("flashtex-host-unicode")
        try executable(host, """
        #!/bin/sh
        case "$1" in
          --list-fonts) printf 'Georgia\\nLatin Modern Math\\nTeX Gyre Pagella\\n' ;;
          --list-math-fonts) printf 'Latin Modern Math\\n' ;;
          *) exit 2 ;;
        esac

        """)
        env.set("FLASHTEX_HOST_UNICODE", host.path)
        env.set("FLASHTEX_RENDER", "/nonexistent/flashtex-render")
        XCTAssertEqual(InstalledFonts.lister()?.path, host.path)
        XCTAssertEqual(InstalledFonts.families(tool: host), ["Georgia", "Latin Modern Math", "TeX Gyre Pagella"])
        XCTAssertEqual(InstalledFonts.mathFamilies(tool: host), ["Latin Modern Math"])

        // The sheet, with no test lister: the same families, Math's own list.
        let root = try dir("sheet")
        let main = root.appendingPathComponent("main.tex")
        try "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n".write(to: main, atomically: true, encoding: .utf8)
        files.append(main)
        let m = model { Self.manifest(root: root, mode: nil, fonts: [:]) }
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: main, dirty: .discard), .opened)
        m.projectFonts.present()
        XCTAssertTrue(m.projectFonts.shown)
        try await waitUntil("the listing", timeout: 20) { !m.projectFonts.listing && !m.projectFonts.families.isEmpty }
        XCTAssertEqual(m.projectFonts.families(for: .text), ["Georgia", "Latin Modern Math", "TeX Gyre Pagella"])
        XCTAssertEqual(m.projectFonts.families(for: .math), ["Latin Modern Math"])

        env.set("FLASHTEX_HOST_UNICODE", "none")
        XCTAssertNil(InstalledFonts.lister(), "no Unicode host and no flashtex-render: nothing to ask")
    }

    // MARK: with the Unicode host and TeX Live (the oracle)

    private func unavailable(_ why: String) -> Error {
        guard ProcessInfo.processInfo.environment["FLASHTEX_REQUIRE_HOST_UNICODE"] == "1" else { return XCTSkip(why) }
        XCTFail("FLASHTEX_REQUIRE_HOST_UNICODE=1: " + why)
        return EngineV3TestHost.HostMissing(description: why)
    }

    /// TeX Live's `xelatex`, where the engine's resolver looks for TeX Live.
    private static var xelatex: URL? {
        let env = ProcessInfo.processInfo.environment
        var c = EngineChoice.texLiveCandidates(environment: env)
        if let bin = env["FLASHTEX_TEXLIVE_BIN"] { c.insert(bin, at: 0) }
        return c.map { URL(fileURLWithPath: $0).appendingPathComponent("xelatex") }
            .first { FileManager.default.isExecutableFile(atPath: $0.path) }
    }

    /// What the document reports about its body font: the font's name as
    /// fontspec loaded it, its glyph count and x-height (both from the face
    /// actually loaded).
    static let probe = "\\typeout{A9 probe: \\fontname\\font; glyphs \\the\\XeTeXcountglyphs\\font; ex \\the\\fontdimen5\\font; A9 end}"

    static let document = "\\documentclass{article}\n\\begin{document}\nHello, world.\n\(probe)\n\\end{document}\n"

    /// The probe's line in a log (TeX folds long lines at 79 columns).
    private static func probeLine(_ log: String) -> String? {
        let unfolded = log.replacingOccurrences(of: "\n", with: "")
        guard let r = unfolded.range(of: "A9 probe: ") else { return nil }
        let rest = unfolded[r.upperBound...]
        guard let end = rest.range(of: "; A9 end") else { return nil }
        return String(rest[..<end.lowerBound])
    }

    /// The oracle: TeX Live's `xelatex` on the document with `[fonts]` as
    /// PROPOSAL §4.5 defines it (fontspec's `\setmainfont` once the class
    /// is loaded, ahead of the document's own preamble), the probe's line.
    private func oracle(text family: String?) throws -> String {
        guard let xelatex = Self.xelatex else { throw unavailable("no xelatex in TeX Live") }
        let d = try dir("oracle")
        try Self.document.write(to: d.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let first = family.map { "\\AddToHook{class/after}{\\RequirePackage{fontspec}\\setmainfont{\($0)}}\\input{main.tex}" }
            ?? "\\input{main.tex}"
        let p = Process()
        p.executableURL = xelatex
        p.currentDirectoryURL = d
        p.arguments = ["-interaction=nonstopmode", "-halt-on-error", "-no-pdf", "-jobname=oracle", first]
        p.standardOutput = FileHandle.nullDevice
        p.standardError = FileHandle.nullDevice
        p.standardInput = FileHandle.nullDevice
        try p.run()
        p.waitUntilExit()
        let log = (try? String(contentsOf: d.appendingPathComponent("oracle.log"), encoding: .utf8)) ?? ""
        guard p.terminationStatus == 0, let line = Self.probeLine(log) else {
            XCTFail("the oracle xelatex failed (\(p.terminationStatus)): \(log.suffix(1500))")
            throw EngineV3TestHost.HostNotReady(description: "oracle")
        }
        return line
    }

    /// A started model on a saved Unicode-mode project whose manifest is
    /// `fonts` (read by the stub and written to flashtex.toml), after its
    /// first compile.
    private func started(fonts: [String: String]) async throws -> (ShellModel, URL, (ProjectFilesV1.Manifest) -> Void) {
        guard EngineV3.locateHost(mode: .unicode) != nil else {
            throw unavailable("no flashtex-host-unicode built (cd crates/flashtex-xetex && cargo build --release --bin flashtex-host-unicode)")
        }
        guard EngineV3TestHost.texLive != nil, EngineChoice.texLiveAvailable() else { throw unavailable("no TeX Live") }
        let root = try dir("project")
        let main = root.appendingPathComponent("main.tex")
        try Self.document.write(to: main, atomically: true, encoding: .utf8)
        try Self.toml(mode: "unicode", fonts: fonts).write(to: root.appendingPathComponent("flashtex.toml"), atomically: true, encoding: .utf8)
        files.append(main)
        final class Box { var m: ProjectFilesV1.Manifest; init(_ m: ProjectFilesV1.Manifest) { self.m = m } }
        let box = Box(Self.manifest(root: root, mode: "unicode", fonts: fonts))
        let m = model { box.m }
        XCTAssertEqual(m.openTex(at: main, dirty: .discard), .opened)
        XCTAssertEqual(m.documentMode.mode, .unicode)
        m.engineV3Enabled = true // the session starts on the opened project
        let s = m.engineV3
        try await EngineV3TestHost.awaitReady(s)
        XCTAssertEqual(s.hostMode, .unicode)
        try await waitUntil("the first compile", debug: { "\(s.phase) \(s.statusNote) done \(s.doneCount) error \(s.firstError ?? "-")" }) {
            !s.compiling && s.doneCount > 0 && s.statusNote.hasPrefix("ok") && s.pageCount == 1
        }
        return (m, root, { box.m = $0 })
    }

    private func log(_ m: ShellModel) throws -> String {
        let out = try XCTUnwrap(m.engineV3.outputDirectory)
        return try String(contentsOf: out.appendingPathComponent("main.log"), encoding: .utf8)
    }

    /// A9: `[fonts] text = "Georgia"` in a Unicode-mode project is applied
    /// by the new engine as `xelatex` applies `\setmainfont{Georgia}`: the
    /// body font the document reports is the oracle's, and the log says the
    /// role was applied.
    func testUnicodeModeAppliesTheProjectsFontsAsXeLaTeXDoes() async throws {
        let (m, _, _) = try await started(fonts: ["text": Self.family])
        defer { m.engineV3.stop() }
        let expected = try oracle(text: Self.family)
        XCTAssertTrue(expected.contains(Self.family), "the oracle loaded \(Self.family): \(expected)")
        let ours = try log(m)
        XCTAssertEqual(Self.probeLine(ours), expected)
        XCTAssertTrue(ours.replacingOccurrences(of: "\n", with: "").contains("FlashTeX [fonts]: \\setmainfont{\(Self.family)}"), String(ours.suffix(2000)))
        XCTAssertNotEqual(expected, try oracle(text: nil), "the class default is another font")
    }

    /// A change of `[fonts]` (the Fonts sheet writes flashtex.toml and reads
    /// it again) compiles again with the new table, without an edit, as the
    /// previous engine's `configure_fonts` did; clearing it compiles back to
    /// the class default.
    func testAChangeOfFontsCompilesAgainWithThem() async throws {
        let (m, root, setManifest) = try await started(fonts: [:])
        defer { m.engineV3.stop() }
        let s = m.engineV3
        XCTAssertEqual(Self.probeLine(try log(m)), try oracle(text: nil))

        let n = s.doneCount
        try Self.toml(mode: "unicode", fonts: ["text": Self.family]).write(to: root.appendingPathComponent("flashtex.toml"), atomically: true, encoding: .utf8)
        setManifest(Self.manifest(root: root, mode: "unicode", fonts: ["text": Self.family]))
        m.manifest.refresh()
        try await waitUntil("the compile with [fonts]", debug: { "\(s.statusNote) done \(s.doneCount) (was \(n))" }) {
            s.doneCount > n && !s.compiling && s.statusNote.hasPrefix("ok")
        }
        XCTAssertEqual(Self.probeLine(try log(m)), try oracle(text: Self.family))

        let k = s.doneCount
        try Self.toml(mode: "unicode", fonts: [:]).write(to: root.appendingPathComponent("flashtex.toml"), atomically: true, encoding: .utf8)
        setManifest(Self.manifest(root: root, mode: "unicode", fonts: [:]))
        m.manifest.refresh()
        try await waitUntil("the compile without [fonts]", debug: { "\(s.statusNote) done \(s.doneCount) (was \(k))" }) {
            s.doneCount > k && !s.compiling && s.statusNote.hasPrefix("ok")
        }
        XCTAssertEqual(Self.probeLine(try log(m)), try oracle(text: nil))
    }
}
