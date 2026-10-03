import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac

/// Per-document engine choice and its fallback rules (EngineChoice.swift;
/// lane P5-ENGINE-CHOICE, app-parity rows A8, A9, A17): the resolution
/// order, the no-TeX-Live / `[fonts]` / pinned-package fallbacks and how the
/// window says so, per-document storage, the built-in default's flip, and
/// the new engine finding flashtex.toml's `texinputs`.
@MainActor
final class EngineChoiceTests: XCTestCase {
    private var env = EnvironmentOverride()
    private var dirs: [URL] = []
    private var files: [URL] = []

    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("engine-choice-tests-\(getpid())").path)
        env.set("FLASHTEX_ENGINE_V3", "") // neither 1 nor 0: the per-document choice decides
        EngineChoiceStore.appSetting = nil
        EngineChoice.builtInDefaultForTests = nil
    }

    override func tearDown() {
        for f in files { EngineChoiceStore.set(nil, for: f) }
        EngineChoiceStore.appSetting = nil
        EngineChoice.builtInDefaultForTests = nil
        for d in dirs { try? FileManager.default.removeItem(at: d) }
        env.restore()
    }

    // MARK: helpers

    private func dir(_ name: String) throws -> URL {
        let d = FileManager.default.temporaryDirectory.appendingPathComponent("engine-choice-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        dirs.append(d)
        return d
    }

    private func texFile(_ name: String = "main.tex", in d: URL? = nil, body: String = "Hello.") throws -> URL {
        let url = try (d ?? dir("doc")).appendingPathComponent(name)
        try "\\documentclass{article}\n\\begin{document}\n\(body)\n\\end{document}\n".write(to: url, atomically: true, encoding: .utf8)
        files.append(url)
        return url
    }

    /// A TeX Live the probe accepts (a `kpsewhich` file in FLASHTEX_TEXLIVE_BIN); no host runs.
    private func fakeTeXLive() throws {
        let bin = try dir("texbin")
        FileManager.default.createFile(atPath: bin.appendingPathComponent("kpsewhich").path, contents: Data())
        env.set("FLASHTEX_TEXLIVE_BIN", bin.path)
        env.set("FLASHTEX_HOST", "none")
    }

    private static func manifest(fonts: [String: String] = [:], pin: [String: String] = [:], libraries: [String: String] = [:],
                                 files: String = "[]") -> ProjectFilesV1.Manifest {
        func obj(_ d: [String: String]) -> String { "{" + d.sorted { $0.key < $1.key }.map { "\"\($0.key)\":\"\($0.value)\"" }.joined(separator: ",") + "}" }
        let table = ["text", "math", "mono", "sans"].map { "\"\($0)\":" + (fonts[$0].map { "\"\($0)\"" } ?? "null") }.joined(separator: ",")
        let json = """
        {"path":"/p/flashtex.toml","exists":true,"manifest_dir":"/p",
         "manifest":{"project":{"entry":"main.tex","texinputs":[],"output":null},"fonts":{\(table)},
                     "packages":{"source":"ctan","fetch":"ask","pin":\(obj(pin)),"path":\(obj(libraries))},"library":null},
         "warnings":[],"texinputs":[],"files":\(files),"diagnostics":[],"template":""}
        """
        return try! JSONDecoder().decode(ProjectFilesV1.Manifest.self, from: Data(json.utf8))
    }

    private func model(manifest: @escaping () -> ProjectFilesV1.Manifest?) -> ShellModel {
        let m = ShellModel()
        m.detachWorker()
        m.manifest.reader = { _, _ in manifest().map { .success($0) } ?? .failure(.init("no manifest (test)")) }
        return m
    }

    // MARK: resolution (pure)

    func testResolutionOrderEnvironmentWindowUserSettingRecordDefault() {
        let user = EngineChoiceStore.Entry(engine: .new, source: .user)
        let record = EngineChoiceStore.Entry(engine: .previous, source: .record)
        func r(_ e: String?, _ w: EngineChoice.Engine?, _ entry: EngineChoiceStore.Entry?, _ app: EngineChoice.Engine?, _ d: EngineChoice.Engine = .previous) -> EngineChoice {
            EngineChoice.resolve(environment: e, window: w, entry: entry, appSetting: app, builtInDefault: d, blocker: { nil })
        }
        XCTAssertEqual(r("1", .previous, record, .previous), EngineChoice(preferred: .new, source: .environment))
        XCTAssertEqual(r("0", .new, user, .new), EngineChoice(preferred: .previous, source: .environment))
        XCTAssertEqual(r("", .new, record, .previous), EngineChoice(preferred: .new, source: .window))
        XCTAssertEqual(r(nil, nil, user, .previous), EngineChoice(preferred: .new, source: .user))
        XCTAssertEqual(r(nil, nil, record, .new), EngineChoice(preferred: .new, source: .appSetting), "the app setting outranks a record")
        XCTAssertEqual(r(nil, nil, record, nil, .new), EngineChoice(preferred: .previous, source: .record), "a record outranks the built-in default")
        XCTAssertEqual(r(nil, nil, nil, nil, .new), EngineChoice(preferred: .new, source: .builtInDefault))
        XCTAssertEqual(r(nil, nil, nil, nil), EngineChoice(preferred: .previous, source: .builtInDefault))
        XCTAssertEqual(EngineChoice.defaultForNewDocuments, .previous, "the P5 flip is the owner's gate, not this lane's")
    }

    func testFallbackRulesApplyOnlyToAnUnforcedNewEngine() {
        var asked = 0
        func r(_ e: String?, _ w: EngineChoice.Engine?, _ app: EngineChoice.Engine?) -> EngineChoice {
            EngineChoice.resolve(environment: e, window: w, entry: nil, appSetting: app, builtInDefault: .previous,
                                 blocker: { asked += 1; return .noTeXLive })
        }
        let fell = r(nil, nil, .new)
        XCTAssertEqual(fell.preferred, .new)
        XCTAssertEqual(fell.effective, .previous)
        XCTAssertEqual(fell.blocker, .noTeXLive)
        XCTAssertEqual(asked, 1)
        XCTAssertEqual(r("1", nil, nil).effective, .new, "FLASHTEX_ENGINE_V3=1 forces")
        XCTAssertEqual(r(nil, .new, nil).effective, .new, "a window override forces")
        XCTAssertEqual(r(nil, nil, .previous).effective, .previous)
        XCTAssertEqual(asked, 1, "the rules are asked only for an unforced new engine")
    }

    func testManifestRules() {
        XCTAssertNil(EngineChoice.blocker(manifest: Self.manifest().manifest))
        XCTAssertEqual(EngineChoice.blocker(manifest: Self.manifest(fonts: ["text": "Georgia", "math": "Libertinus Math"]).manifest),
                       .projectFonts(["text", "math"]))
        XCTAssertNil(EngineChoice.blocker(manifest: Self.manifest(fonts: ["text": "  "]).manifest), "a blank role is the class default")
        XCTAssertEqual(EngineChoice.blocker(manifest: Self.manifest(pin: ["siunitx": "3.3.24"]).manifest), .pinnedPackages(["siunitx"]))
        XCTAssertEqual(EngineChoice.blocker(manifest: Self.manifest(libraries: ["mylib": "../mylib"]).manifest), .packageLibraries(["mylib"]))
        XCTAssertTrue(EngineChoice.Blocker.projectFonts(["text"]).detail.contains("[fonts] (text)"))
    }

    func testTeXLiveProbeFollowsTheEnginesSearchAndTheHostsReport() throws {
        let bin = try dir("texbin-probe")
        XCTAssertFalse(EngineChoice.texLiveAvailable(environment: ["FLASHTEX_TEXLIVE_BIN": bin.path]), "no kpsewhich there")
        FileManager.default.createFile(atPath: bin.appendingPathComponent("kpsewhich").path, contents: Data())
        XCTAssertTrue(EngineChoice.texLiveAvailable(environment: ["FLASHTEX_TEXLIVE_BIN": bin.path]))
        XCTAssertTrue(EngineChoice.texLiveAvailable(environment: ["FLASHTEX_TEXLIVE_BIN": "/nonexistent", "FLASHTEX_BUNDLE": "/b"]), "a bundle needs no TeX Live")
        XCTAssertEqual(EngineChoice.texLiveCandidates(environment: ["FLASHTEX_TEXLIVE_BIN": "/x"]), ["/x"], "as resolver.rs: that directory alone")
        let c = EngineChoice.texLiveCandidates(environment: ["PATH": "/a/bin:relative:/b", "HOME": "/nohome"])
        XCTAssertEqual(Array(c.prefix(2)), ["/a/bin", "/b"])
        XCTAssertTrue(c.contains("/Library/TeX/texbin"))
        XCTAssertEqual(Array(c.suffix(3)), ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"])
        XCTAssertTrue(EngineChoice.hostLacksTeXLive(DL3JSONView(texlive: nil, formatFailed: true)))
        XCTAssertFalse(EngineChoice.hostLacksTeXLive(DL3JSONView(texlive: nil, formatFailed: false)), "a bundle prepared the format")
        XCTAssertFalse(EngineChoice.hostLacksTeXLive(DL3JSONView(texlive: "/Library/TeX/texbin (MacTeX)", formatFailed: true)))
    }

    // MARK: the window

    /// A9: `[fonts]` in flashtex.toml keeps the project on the previous
    /// engine, said in the banner, the status item and to VoiceOver; the
    /// record is the previous engine. Removing the table switches to the new
    /// engine; adding it back falls back again.
    func testProjectFontsFallBackAndSayWhy() throws {
        try fakeTeXLive()
        EngineChoiceStore.appSetting = .new
        var current: ProjectFilesV1.Manifest? = Self.manifest(fonts: ["text": "Georgia"])
        let m = model { current }
        defer { m.engineV3.stop() }
        let file = try texFile()
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        XCTAssertFalse(m.engineV3Enabled, "the previous engine typesets it")
        XCTAssertEqual(m.engineChoice.preferred, .new)
        XCTAssertEqual(m.engineChoice.source, .appSetting)
        XCTAssertEqual(m.engineChoice.blocker, .projectFonts(["text"]))
        XCTAssertEqual(m.engineChoice.title, "Previous engine")
        XCTAssertTrue(m.engineChoice.explanation.contains("sets [fonts] (text)"), m.engineChoice.explanation)
        XCTAssertEqual(m.engineAnnouncements, ["Typeset with the previous engine: this project sets fonts in flashtex.toml."])
        XCTAssertEqual(EngineChoiceStore.entry(for: file), .init(engine: .previous, source: .record))

        current = Self.manifest()
        m.manifest.refresh()
        XCTAssertTrue(m.engineV3Enabled, "no rule applies any more")
        XCTAssertNil(m.engineChoice.blocker)

        current = Self.manifest(fonts: ["math": "Libertinus Math"])
        m.manifest.refresh()
        XCTAssertFalse(m.engineV3Enabled)
        XCTAssertEqual(m.engineChoice.blocker, .projectFonts(["math"]))
        XCTAssertEqual(m.engineAnnouncements.count, 2)
    }

    /// A17: without TeX Live the new engine cannot prepare its format, so
    /// a document that would use it is typeset by the previous engine, and
    /// choosing the new engine says why it cannot.
    func testNoTeXLiveFallsBackAndTheChoiceSaysWhy() throws {
        env.set("FLASHTEX_TEXLIVE_BIN", "/nonexistent/texlive/bin")
        env.set("FLASHTEX_HOST", "none")
        EngineChoiceStore.appSetting = .new
        let m = model { nil }
        defer { m.engineV3.stop() }
        let file = try texFile()
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        XCTAssertFalse(m.engineV3Enabled)
        XCTAssertEqual(m.engineChoice.blocker, .noTeXLive)
        m.chooseEngine(.new)
        XCTAssertFalse(m.engineV3Enabled)
        XCTAssertEqual(m.engineChoice.source, .user)
        XCTAssertEqual(m.navigationNote?.hasPrefix("The new engine cannot typeset this project: No TeX Live installation was found."), true, m.navigationNote ?? "")
        XCTAssertEqual(EngineChoiceStore.entry(for: file)?.engine, .new, "the choice is kept for when TeX Live is installed")
        env.set("FLASHTEX_ENGINE_V3", "1")
        XCTAssertEqual(EngineChoice.atLaunch.effective, .new, "the environment forces it anyway")
    }

    /// The host itself reports no TeX Live (and no format): the window falls
    /// back with the reason, unless the new engine was forced.
    func testTheHostsNoTeXLiveReportFallsBack() throws {
        try fakeTeXLive()
        EngineChoiceStore.appSetting = .new
        let m = model { nil }
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled)
        m.engineV3HostLacksTeXLive()
        XCTAssertFalse(m.engineV3Enabled)
        XCTAssertEqual(m.engineChoice.blocker, .noTeXLive)

        let forced = model { nil }
        defer { forced.engineV3.stop() }
        forced.engineV3Enabled = true
        forced.engineV3HostLacksTeXLive()
        XCTAssertTrue(forced.engineV3Enabled, "a forced engine is never switched")
    }

    /// The choice is per document and survives the window: another window
    /// opening the same file uses it, another file does not; "Use the
    /// Default" forgets it.
    func testTheChoiceIsPerDocumentAndOutlivesTheWindow() throws {
        try fakeTeXLive()
        let d = try dir("two")
        let a = try texFile("a.tex", in: d), b = try texFile("b.tex", in: d)
        let first = model { nil }
        defer { first.engineV3.stop() }
        XCTAssertEqual(first.openTex(at: a, dirty: .discard), .opened)
        XCTAssertFalse(first.engineV3Enabled, "the built-in default")
        XCTAssertEqual(first.engineChoice.source, .builtInDefault)
        first.chooseEngine(.new)
        XCTAssertTrue(first.engineV3Enabled)

        let second = model { nil }
        defer { second.engineV3.stop() }
        XCTAssertEqual(second.openTex(at: a, dirty: .discard), .opened)
        XCTAssertTrue(second.engineV3Enabled)
        XCTAssertEqual(second.engineChoice.source, .user)
        XCTAssertEqual(second.openTex(at: b, dirty: .discard), .opened)
        XCTAssertFalse(second.engineV3Enabled, "b has no choice")
        XCTAssertEqual(second.openTex(at: a, dirty: .discard), .opened)
        XCTAssertTrue(second.engineV3Enabled)
        second.clearEngineChoice()
        XCTAssertFalse(second.engineV3Enabled)
        XCTAssertEqual(second.engineChoice.source, .builtInDefault)
    }

    /// The P5 flip: with the built-in default the new engine, a document
    /// nobody has typeset opens with it, one already typeset with the
    /// previous engine keeps it (no silent switch), and the fallback rules
    /// still hold.
    func testFlippingTheDefaultKeepsTypesetDocumentsOnTheirEngine() throws {
        try fakeTeXLive()
        let seen = try texFile()
        let before = model { nil }
        defer { before.engineV3.stop() }
        XCTAssertEqual(before.openTex(at: seen, dirty: .discard), .opened)
        XCTAssertEqual(EngineChoiceStore.entry(for: seen), .init(engine: .previous, source: .record))

        EngineChoice.builtInDefaultForTests = .new
        var current: ProjectFilesV1.Manifest?
        let after = model { current }
        defer { after.engineV3.stop() }
        XCTAssertEqual(after.openTex(at: seen, dirty: .discard), .opened)
        XCTAssertFalse(after.engineV3Enabled)
        XCTAssertEqual(after.engineChoice.source, .record)
        XCTAssertEqual(after.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertTrue(after.engineV3Enabled, "a new document follows the new default")
        XCTAssertEqual(after.engineChoice.source, .builtInDefault)
        current = Self.manifest(pin: ["siunitx": "3.3.24"])
        XCTAssertEqual(after.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertFalse(after.engineV3Enabled)
        XCTAssertEqual(after.engineChoice.blocker, .pinnedPackages(["siunitx"]))
    }

    /// Setting `engineV3Enabled` directly (benches, the existing v3 tests)
    /// is the window's override: kept across opens, no rule applies, nothing recorded.
    func testADirectSetIsTheWindowsOverride() throws {
        try fakeTeXLive()
        let m = model { Self.manifest(fonts: ["text": "Georgia"]) }
        defer { m.engineV3.stop() }
        m.engineV3Enabled = true
        let file = try texFile()
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled)
        XCTAssertEqual(m.engineChoice.source, .window)
        XCTAssertNil(m.engineChoice.blocker)
        XCTAssertNil(EngineChoiceStore.entry(for: file))
    }

    /// The status item and the banner, in a hosted window: the banner is
    /// laid out over the preview while the fallback applies and goes when
    /// dismissed or when no rule applies; the status item stays. What they
    /// show and speak names the engine and the reason.
    func testStatusItemAndBannerShowTheFallback() throws {
        try fakeTeXLive()
        EngineChoiceStore.appSetting = .new
        var current: ProjectFilesV1.Manifest? = Self.manifest(fonts: ["text": "Georgia"])
        let m = model { current }
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: try texFile(), dirty: .discard), .opened)
        let c = m.engineChoice
        XCTAssertEqual(EngineChoiceStatusItem.spokenLabel(c), "Engine: Previous engine")
        XCTAssertEqual(EngineChoiceStatusItem.spokenValue(c), "fallback, this project sets fonts in flashtex.toml")
        XCTAssertEqual(EngineFallbackBanner.headline(try XCTUnwrap(c.blocker)), "Typeset with the previous engine: this project sets fonts in flashtex.toml.")

        let host = NSHostingView(rootView: VStack(spacing: 0) { EngineFallbackBanner(); EngineChoiceStatusItem() }.environment(m))
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 640, height: 300), styleMask: [.titled])
        window.isReleasedWhenClosed = false
        window.contentView = host
        defer { window.contentView = nil }
        func height() -> CGFloat { host.layoutSubtreeIfNeeded(); return host.fittingSize.height }
        let withBanner = height()
        m.engineFallbackDismissed = true
        let dismissed = height()
        XCTAssertGreaterThan(withBanner, dismissed + 20, "the banner takes room while shown")
        XCTAssertGreaterThan(dismissed, 0, "the status item stays")
        m.engineFallbackDismissed = false
        XCTAssertEqual(height(), withBanner)
        current = Self.manifest()
        m.manifest.refresh() // no rule applies: the new engine, no banner
        XCTAssertTrue(m.engineV3Enabled)
        XCTAssertEqual(height(), dismissed)
        XCTAssertEqual(EngineChoiceStatusItem.spokenLabel(m.engineChoice), "Engine: New engine")
        XCTAssertEqual(EngineChoiceStatusItem.spokenValue(m.engineChoice), "Settings > Compile")
    }

    // MARK: A8: texinputs in the new engine

    /// flashtex.toml's `texinputs` (a directory under the root and one
    /// outside it): the new engine finds their packages by name, as
    /// `TEXINPUTS` would, and a project file of the same name wins. When the
    /// manifest stops naming them, they are gone again.
    func testTheNewEngineFindsTexinputsPackages() async throws {
        try EngineV3TestHost.require()
        let root = try dir("texinputs")
        let shared = try dir("shared")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("styles"), withIntermediateDirectories: true)
        try "\\ProvidesPackage{mystyle}\n\\newcommand{\\mymacro}{Inside the root.}\n".write(to: root.appendingPathComponent("styles/mystyle.sty"), atomically: true, encoding: .utf8)
        try "\\ProvidesPackage{lab}\n\\newcommand{\\labnote}{Outside the root.}\n".write(to: shared.appendingPathComponent("lab.sty"), atomically: true, encoding: .utf8)
        let main = root.appendingPathComponent("main.tex")
        try "\\documentclass{article}\n\\usepackage{mystyle}\n\\usepackage{lab}\n\\begin{document}\n\\mymacro{} \\labnote{}\n\\end{document}\n"
            .write(to: main, atomically: true, encoding: .utf8)
        files.append(main)
        let filesJSON = """
        [{"path":"styles/mystyle.sty","kind":"package","texinput":0,"origin":null,"text":"","sha256":"a","bytes":1},
         {"path":"texinputs/1/lab.sty","kind":"package","texinput":1,"origin":"\(shared.appendingPathComponent("lab.sty").path)","text":"","sha256":"b","bytes":1}]
        """
        var current: ProjectFilesV1.Manifest? = Self.manifest(files: filesJSON)
        EngineChoiceStore.appSetting = .new
        let m = model { current }
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: main, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled, m.engineChoice.explanation)
        let s = m.engineV3
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount == 1 }
        XCTAssertNil(s.firstError)
        XCTAssertTrue(m.displayedDiagnostics.filter { $0.severity == .error }.isEmpty, "\(m.displayedDiagnostics.map(\.message))")
        let copy = try XCTUnwrap(s.projectCopy)
        XCTAssertEqual(try FileManager.default.destinationOfSymbolicLink(atPath: copy.appendingPathComponent("mystyle.sty").path), "styles/mystyle.sty")
        XCTAssertEqual(try FileManager.default.destinationOfSymbolicLink(atPath: copy.appendingPathComponent("lab.sty").path),
                       shared.appendingPathComponent("lab.sty").path)

        // The manifest no longer names them: the links go, and TeX says the package is missing.
        current = Self.manifest()
        m.manifest.refresh()
        try await waitUntil("the compile without texinputs") { s.firstError != nil && !s.compiling }
        XCTAssertTrue(s.firstError?.contains("mystyle.sty") == true, s.firstError ?? "")
        XCTAssertFalse(FileManager.default.fileExists(atPath: copy.appendingPathComponent("lab.sty").path))
    }

    /// The copy's texinputs links (no host): a project file of the same
    /// name wins, also one that appears later; an open document's name is
    /// left to the editor; links the manifest no longer names go.
    func testTexinputsLinksYieldToProjectFiles() throws {
        let root = try dir("links")
        let shared = try dir("links-shared")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("styles"), withIntermediateDirectories: true)
        for name in ["a.sty", "b.sty", "c.sty"] { try "%".write(to: root.appendingPathComponent("styles/" + name), atomically: true, encoding: .utf8) }
        try "% root".write(to: root.appendingPathComponent("a.sty"), atomically: true, encoding: .utf8)
        try "% shared".write(to: shared.appendingPathComponent("d.sty"), atomically: true, encoding: .utf8)
        let mirror = EngineV3Mirror(source: root, session: 9_000 + Int.random(in: 0 ..< 1000))
        defer { try? FileManager.default.removeItem(at: mirror.base) }
        _ = mirror.sync(except: ["main.tex"], fingerprints: false, quarantine: false)
        let links: [EngineV3Mirror.TexInputLink] = [
            .init(name: "a.sty", inCopy: "styles/a.sty", external: nil),
            .init(name: "b.sty", inCopy: "styles/b.sty", external: nil),
            .init(name: "c.sty", inCopy: "styles/c.sty", external: nil),
            .init(name: "d.sty", inCopy: nil, external: shared.appendingPathComponent("d.sty").path),
        ]
        mirror.linkTexInputs(links, except: ["main.tex", "c.sty"])
        let fm = FileManager.default
        func dest(_ n: String) -> String? {
            guard let d = try? fm.destinationOfSymbolicLink(atPath: mirror.root.appendingPathComponent(n).path) else { return nil }
            return d.hasPrefix("/") ? URL(fileURLWithPath: d).resolvingSymlinksInPath().path : d
        }
        let rootR = root.resolvingSymlinksInPath(), sharedR = shared.resolvingSymlinksInPath()
        XCTAssertEqual(dest("a.sty"), rootR.appendingPathComponent("a.sty").path, "the root's own a.sty wins")
        XCTAssertEqual(dest("b.sty"), "styles/b.sty")
        XCTAssertNil(dest("c.sty"), "an open document of that name is the editor's")
        XCTAssertEqual(dest("d.sty"), sharedR.appendingPathComponent("d.sty").path)
        XCTAssertEqual(try String(contentsOf: mirror.root.appendingPathComponent("b.sty"), encoding: .utf8), "%")

        // b.sty appears in the root: it takes the link's place.
        try "% root b".write(to: root.appendingPathComponent("b.sty"), atomically: true, encoding: .utf8)
        mirror.linkTexInputs(links, except: ["main.tex"])
        XCTAssertEqual(dest("b.sty"), rootR.appendingPathComponent("b.sty").path)
        // The manifest names only c.sty now: d.sty's link goes.
        mirror.linkTexInputs([links[2]], except: ["main.tex"])
        XCTAssertNil(dest("d.sty"))
        XCTAssertEqual(dest("c.sty"), "styles/c.sty")
        XCTAssertEqual(dest("a.sty"), rootR.appendingPathComponent("a.sty").path, "the walk's link to a project file is never removed")
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }
}
