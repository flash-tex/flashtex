import AppKit
import SwiftUI
import XCTest
import HostedWindows
import FlashTeXProtocol
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
        // No bundle unless a test configures one (never a lock file of this Mac's).
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
        EngineV3Bundle.forgetConsent()
        EngineChoiceStore.appSetting = nil
        EngineChoice.builtInDefaultForTests = nil
        EngineV3.defaults.removeObject(forKey: EngineV3.enabledKey)
        EngineV3.defaults.removeObject(forKey: EngineChoiceStore.legacyMigratedKey)
    }

    override func tearDown() {
        EngineChoiceStore.identityProvider = EngineChoiceStore.liveIdentity
        EngineV3.defaults.removeObject(forKey: EngineV3.enabledKey)
        EngineV3.defaults.removeObject(forKey: EngineChoiceStore.legacyMigratedKey)
        for f in files { EngineChoiceStore.set(nil, for: f) }
        EngineChoiceStore.appSetting = nil
        EngineChoice.builtInDefaultForTests = nil
        EngineV3Bundle.forgetConsent()
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
        // V3-PACKAGE-RESOLUTION: pins and libraries are the new engine's too (no fallback).
        XCTAssertNil(EngineChoice.blocker(manifest: Self.manifest(pin: ["siunitx": "3.3.24"]).manifest))
        XCTAssertNil(EngineChoice.blocker(manifest: Self.manifest(libraries: ["mylib": "../mylib"]).manifest))
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
        XCTAssertNil(EngineChoiceStore.entry(for: file), "no record while a rule blocks the new engine")

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

    /// NOTEX-WIRING: a configured bundle (the environment, or a
    /// `flashtex-bundle.lock`) is a distribution, so the new engine does not
    /// fall back for want of TeX Live; nothing is downloaded before the user
    /// agrees (the host runs offline until then), and "Not Now" falls back
    /// with its own reason until the new engine is chosen again.
    func testAConfiguredBundleIsADistributionBehindConsent() throws {
        let digest = String(repeating: "ab", count: 32)
        let noTL = ["FLASHTEX_TEXLIVE_BIN": "/nonexistent/texlive/bin", "FLASHTEX_BUNDLE_LOCK": "/nonexistent/flashtex-bundle.lock"]
        XCTAssertFalse(EngineChoice.texLiveAvailable(environment: noTL))
        XCTAssertTrue(EngineChoice.texLiveAvailable(environment: noTL.merging(["FLASHTEX_BUNDLE_DIGEST": digest]) { $1 }),
                      "a bundle in the environment")
        // A lock file with CRLF line ends (Swift's "\r\n" is one Character); a relative url is the lock's directory's.
        let d = try dir("bundle-lock")
        let lock = d.appendingPathComponent(EngineV3Bundle.lockFileName)
        try "# pinned\r\nurl = \"core.ttb\"\r\ndigest = \"\(digest.uppercased())\"\r\n".write(to: lock, atomically: true, encoding: .utf8)
        let withLock = noTL.merging(["FLASHTEX_BUNDLE_LOCK": lock.path]) { $1 }
        XCTAssertTrue(EngineChoice.texLiveAvailable(environment: withLock))
        XCTAssertFalse(EngineChoice.texLiveInstalled(environment: withLock))
        let config = try XCTUnwrap(EngineV3Bundle.configured(environment: withLock, host: nil))
        XCTAssertEqual(config, EngineV3Bundle.Config(url: d.appendingPathComponent("core.ttb").path, digest: digest, origin: lock.path))

        // Consent: asked once per bundle (digest and source), only with no TeX Live.
        typealias B = EngineV3Bundle
        XCTAssertEqual(B.gate(texLiveInstalled: false, config: config, consent: nil), .ask(config))
        XCTAssertEqual(B.gate(texLiveInstalled: false, config: config, consent: false), .declined)
        XCTAssertEqual(B.gate(texLiveInstalled: false, config: config, consent: true), .none)
        XCTAssertEqual(B.gate(texLiveInstalled: true, config: config, consent: nil), .none)
        let fromEnv = B.Config(url: "https://e.org/b.ttb", digest: digest, origin: "environment")
        XCTAssertEqual(B.gate(texLiveInstalled: false, config: fromEnv, consent: nil), .ask(fromEnv),
                       "an environment bundle is asked about too")
        B.setConsent(true, for: config)
        XCTAssertEqual(B.consent(for: config), true)
        var newer = config; newer.digest = String(repeating: "cd", count: 32)
        XCTAssertNil(B.consent(for: newer), "a new pinned bundle is asked about again")
        var elsewhere = config; elsewhere.url = "https://other.example/core.ttb"
        XCTAssertNil(B.consent(for: elsewhere), "the same digest from another source too")
        B.forgetConsent()
        XCTAssertEqual(B.progressText(what: "core", name: digest, done: 1_048_576, total: 4_194_304), "downloading TeX files: 1.0 of 4.0 MB (25%)")
        XCTAssertNil(B.progressText(what: "core", name: digest, done: 4, total: 4), "done")

        // The host's environment fails closed: offline without consent, an
        // inherited ALLOW_FETCH dropped, and after consent only this digest.
        env.set("FLASHTEX_TEXLIVE_BIN", "/nonexistent/texlive/bin")
        env.set("FLASHTEX_HOST", "none")
        env.set("FLASHTEX_BUNDLE_LOCK", lock.path)
        let cache = try dir("bundle-cache")
        env.set("FLASHTEX_BUNDLE_CACHE_DIR", cache.path)
        env.set("FLASHTEX_BUNDLE_ALLOW_FETCH", "1")
        let host = URL(fileURLWithPath: "/nonexistent/flashtex-host")
        var h = EngineV3HostProcess.environment(host: host)
        XCTAssertEqual(h["FLASHTEX_BUNDLE_OFFLINE"], "1")
        XCTAssertNil(h["FLASHTEX_BUNDLE_ALLOW_FETCH"], "an inherited permission is not the user's consent")
        XCTAssertEqual(h["FLASHTEX_BUNDLE_LOCK"], lock.path)
        // A lock the app cannot read (whatever the host makes of it): still offline.
        let unread = try dir("bundle-lock-unread").appendingPathComponent(EngineV3Bundle.lockFileName)
        try "url = \"core.ttb\" trailing\ndigest = \(digest)\n".write(to: unread, atomically: true, encoding: .utf8)
        env.set("FLASHTEX_BUNDLE_LOCK", unread.path)
        XCTAssertNil(EngineV3Bundle.configured(host: nil))
        h = EngineV3HostProcess.environment(host: host)
        XCTAssertEqual(h["FLASHTEX_BUNDLE_OFFLINE"], "1")
        XCTAssertNil(h["FLASHTEX_BUNDLE_ALLOW_FETCH"])
        env.set("FLASHTEX_BUNDLE_LOCK", lock.path)
        // The new engine forced on: no fallback rule applies, but the host is still offline without consent.
        env.set("FLASHTEX_ENGINE_V3", "1")
        XCTAssertEqual(EngineChoice.atLaunch.effective, .new)
        XCTAssertEqual(EngineV3Bundle.currentGate(), .ask(config), "launching the host asks first, forced or not")
        // A cache already there (an interrupted download, or one made outside
        // the app) does not skip the question: the stored answer decides.
        let cachedIndex = cache.appendingPathComponent("\(digest)/index.gz")
        try FileManager.default.createDirectory(at: cachedIndex.deletingLastPathComponent(), withIntermediateDirectories: true)
        FileManager.default.createFile(atPath: cachedIndex.path, contents: Data())
        XCTAssertEqual(EngineV3Bundle.currentGate(), .ask(config), "a cached index is not consent")
        XCTAssertEqual(EngineV3HostProcess.environment(host: host)["FLASHTEX_BUNDLE_OFFLINE"], "1")
        env.set("FLASHTEX_ENGINE_V3", "")

        // The window: the new engine stays on; "Not Now" falls back; choosing it again asks again.
        EngineChoiceStore.appSetting = .new
        let m = model { nil }
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled, "no fallback: the bundle is the distribution")
        XCTAssertNil(m.engineChoice.blocker)
        m.engineV3.answerBundleConsent(false)
        XCTAssertFalse(m.engineV3Enabled)
        XCTAssertEqual(m.engineChoice.blocker, .bundleDeclined)
        XCTAssertEqual(EngineV3Bundle.consent(for: config), false)
        m.chooseEngine(.new)
        XCTAssertTrue(m.engineV3Enabled, "choosing it again asks again")
        XCTAssertNil(EngineV3Bundle.consent(for: config))
        m.engineV3.answerBundleConsent(true)
        h = EngineV3HostProcess.environment(host: host)
        XCTAssertNil(h["FLASHTEX_BUNDLE_OFFLINE"], "downloads once agreed")
        XCTAssertEqual(h["FLASHTEX_BUNDLE_ALLOW_FETCH"], "\(digest)@\(config.url)", "only the bundle agreed to, from the URL the app read")
    }

    /// A bundle set in the app's environment (FLASHTEX_BUNDLE_URL/DIGEST) is
    /// asked about like a lock file's: without consent the host is offline;
    /// with it, the host fetches that environment bundle (no ALLOW_FETCH is
    /// needed for it, and an inherited one is still dropped).
    func testAnEnvironmentBundleIsOfflineUntilConsent() throws {
        let digest = String(repeating: "ef", count: 32)
        env.set("FLASHTEX_TEXLIVE_BIN", "/nonexistent/texlive/bin")
        env.set("FLASHTEX_HOST", "none")
        env.set("FLASHTEX_BUNDLE_DIGEST", digest)
        env.set("FLASHTEX_BUNDLE_URL", "https://bundles.example/core.ttb")
        env.set("FLASHTEX_BUNDLE_ALLOW_FETCH", "1")
        let config = try XCTUnwrap(EngineV3Bundle.configured(host: nil))
        XCTAssertTrue(config.fromEnvironment)
        let host = URL(fileURLWithPath: "/nonexistent/flashtex-host")
        var h = EngineV3HostProcess.environment(host: host)
        XCTAssertEqual(h["FLASHTEX_BUNDLE_OFFLINE"], "1", "no consent: offline")
        XCTAssertNil(h["FLASHTEX_BUNDLE_ALLOW_FETCH"], "an inherited permission is dropped")
        XCTAssertEqual(h["FLASHTEX_BUNDLE_LOCK"], "/nonexistent/flashtex-bundle.lock", "the app names no lock for an environment bundle (this is setUp's)")
        XCTAssertEqual(EngineV3Bundle.currentGate(), .ask(config))
        EngineV3Bundle.setConsent(true, for: config)
        h = EngineV3HostProcess.environment(host: host)
        XCTAssertNil(h["FLASHTEX_BUNDLE_OFFLINE"], "consent: the environment bundle may fetch")
        XCTAssertNil(h["FLASHTEX_BUNDLE_ALLOW_FETCH"], "not needed for it, and the inherited one is still dropped")
        XCTAssertEqual(h["FLASHTEX_BUNDLE_DIGEST"], digest)
        XCTAssertEqual(EngineV3Bundle.currentGate(), .none)
    }

    /// BUNDLE-PUBLISH: the lock make-app.sh ships (tools/bundle/tl2026/)
    /// names a GitHub Release asset of this repository and a digest, and it
    /// is behind the consent sheet like any other: with no TeX Live and no
    /// answer the app asks, and the host it starts is offline.
    func testTheShippedLockIsAReleaseAssetBehindConsent() throws {
        var root = URL(fileURLWithPath: #filePath)
        for _ in 0 ..< 5 { root = root.deletingLastPathComponent() }
        let lock = root.appendingPathComponent("tools/bundle/tl2026/flashtex-bundle.lock")
        let parsed = try XCTUnwrap(EngineV3Bundle.parseLock(try String(contentsOf: lock, encoding: .utf8),
                                                            directory: lock.deletingLastPathComponent().path))
        XCTAssertTrue(parsed.url.hasPrefix("https://github.com/flash-tex/flashtex/releases/download/texbundle-tl2026-"), parsed.url)
        XCTAssertTrue(parsed.url.hasSuffix(".ttb"), parsed.url)
        XCTAssertEqual(parsed.digest.count, 64)
        let env = ["FLASHTEX_TEXLIVE_BIN": "/nonexistent/texlive/bin", "FLASHTEX_BUNDLE_LOCK": lock.path]
        let config = try XCTUnwrap(EngineV3Bundle.configured(environment: env, host: nil))
        XCTAssertEqual(config.sourceLabel, "github.com")
        EngineV3Bundle.forgetConsent()
        XCTAssertEqual(EngineV3Bundle.gate(texLiveInstalled: false, config: config, consent: EngineV3Bundle.consent(for: config)),
                       .ask(config), "asked before the first download")
        var h = env
        EngineV3Bundle.hostEnvironment(&h, host: URL(fileURLWithPath: "/nonexistent/flashtex-host"))
        XCTAssertEqual(h["FLASHTEX_BUNDLE_OFFLINE"], "1", "no consent: the host fetches nothing")
        XCTAssertNil(h["FLASHTEX_BUNDLE_ALLOW_FETCH"])
    }

    /// The lock is found where make-app.sh puts it: an app whose host is
    /// Contents/Helpers/flashtex-host reads Contents/Resources/engine/
    /// flashtex-bundle.lock, with no environment pointing at it.
    func testTheShippedLockIsFoundInTheAppsResources() throws {
        var root = URL(fileURLWithPath: #filePath)
        for _ in 0 ..< 5 { root = root.deletingLastPathComponent() }
        let shipped = root.appendingPathComponent("tools/bundle/tl2026/flashtex-bundle.lock")
        let app = try dir("app-layout").appendingPathComponent("FlashTeX.app/Contents")
        let engine = app.appendingPathComponent("Resources/engine")
        try FileManager.default.createDirectory(at: engine, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: app.appendingPathComponent("Helpers"), withIntermediateDirectories: true)
        let lock = engine.appendingPathComponent(EngineV3Bundle.lockFileName)
        try FileManager.default.copyItem(at: shipped, to: lock)
        let host = app.appendingPathComponent("Helpers/flashtex-host")
        let home = try dir("app-layout-home")
        let env = ["HOME": home.path, "FLASHTEX_TEXLIVE_BIN": "/nonexistent/texlive/bin"]
        let candidates = EngineV3Bundle.lockCandidates(environment: env, host: host)
        XCTAssertTrue(candidates.contains(lock.standardizedFileURL.path), "\(candidates)")
        let config = try XCTUnwrap(EngineV3Bundle.configured(environment: env, host: host))
        XCTAssertEqual(URL(fileURLWithPath: config.origin).standardizedFileURL.path, lock.standardizedFileURL.path)
        let expected = try XCTUnwrap(EngineV3Bundle.parseLock(try String(contentsOf: shipped, encoding: .utf8),
                                                              directory: shipped.deletingLastPathComponent().path))
        XCTAssertEqual(config.url, expected.url)
        XCTAssertEqual(config.digest, expected.digest)
        EngineV3Bundle.forgetConsent()
        var h = env
        EngineV3Bundle.hostEnvironment(&h, host: host)
        XCTAssertEqual(h["FLASHTEX_BUNDLE_LOCK"].map { URL(fileURLWithPath: $0).standardizedFileURL.path },
                       lock.standardizedFileURL.path, "the host is told which lock the app read")
        XCTAssertEqual(h["FLASHTEX_BUNDLE_OFFLINE"], "1", "and fetches nothing before consent")
    }

    /// The lock parser gives the engine's answers: the shared vectors
    /// (docs/contracts/bundle-lock-vectors.json, also run by the engine's
    /// `bundle::tests::lock_file_vectors`).
    func testLockParserMatchesTheEnginesVectors() throws {
        // apps/mac/Tests/FlashTeXMacTests/<this file> -> the repository root.
        var root = URL(fileURLWithPath: #filePath)
        for _ in 0 ..< 5 { root = root.deletingLastPathComponent() }
        let data = try Data(contentsOf: root.appendingPathComponent("docs/contracts/bundle-lock-vectors.json"))
        let json = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
        let dir = try XCTUnwrap(json["dir"] as? String)
        let cases = try XCTUnwrap(json["cases"] as? [[String: Any]])
        XCTAssertGreaterThanOrEqual(cases.count, 10)
        for c in cases {
            let name = c["name"] as? String ?? "?"
            let got = EngineV3Bundle.parseLock(try XCTUnwrap(c["text"] as? String), directory: dir)
            XCTAssertEqual(got?.url, c["url"] as? String, name)
            XCTAssertEqual(got?.digest, c["digest"] as? String, name)
        }
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
        current = Self.manifest(fonts: ["text": "Georgia"])
        XCTAssertEqual(after.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertFalse(after.engineV3Enabled)
        XCTAssertEqual(after.engineChoice.blocker, .projectFonts(["text"]))
        // A pinned package or a local library no longer falls back.
        current = Self.manifest(pin: ["siunitx": "3.3.24"], libraries: ["mylib": "../mylib"])
        XCTAssertEqual(after.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertTrue(after.engineV3Enabled, after.engineChoice.explanation)
        XCTAssertNil(after.engineChoice.blocker)
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

    // MARK: engine labels (retirement plan #1236, S3r)

    func testTheLogLineNamesTheEngineWhyAndTheFallback() {
        XCTAssertEqual(EngineChoice(preferred: .new, source: .record).logLine(document: "main.tex"),
                       "engine: new (record) main.tex")
        XCTAssertEqual(EngineChoice(preferred: .new, source: .user, blocker: .noTeXLive).logLine(document: nil),
                       "engine: previous (user; fallback from new: no TeX Live is installed)")
        XCTAssertEqual(EngineChoice(preferred: .previous, source: .builtInDefault).logLine(document: ""),
                       "engine: previous (builtInDefault)")
    }

    /// Every open and every change of the window's engine writes an
    /// `engine:` line (FLASHTEX_LOG), and the capture request names the engine
    /// that typesets the document.
    func testOpenAndChangeLogTheEngineAndTheCaptureListFollowsIt() throws {
        try fakeTeXLive()
        EngineChoiceStore.appSetting = .new
        var current: ProjectFilesV1.Manifest? = Self.manifest(fonts: ["text": "Georgia"])
        let m = model { current }
        defer { m.engineV3.stop() }
        final class Lines { var all: [String] = [] }
        let lines = Lines()
        m.engineLog = { lines.all.append($0) }
        XCTAssertEqual(m.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertEqual(lines.all.last, "engine: previous (appSetting; fallback from new: this project sets fonts in flashtex.toml) main.tex")
        XCTAssertEqual(m.typesettingEngine, .previous)
        XCTAssertEqual(m.captureConvertRequest(captureId: "c").engine, "previous")
        current = Self.manifest()
        m.manifest.refresh()
        XCTAssertEqual(lines.all.last, "engine: new (appSetting) main.tex")
        XCTAssertEqual(m.typesettingEngine, .new)
        XCTAssertEqual(m.captureConvertRequest(captureId: "c").engine, "new", "the capture request names the engine that typesets it")
        XCTAssertEqual(m.captureConvertRequest(captureId: "c").supportedFeatures, CaptureFeatures.supportedFeatures())
        m.engineV3Enabled = false // a direct set: the window's override
        XCTAssertEqual(lines.all.last, "engine: previous (window) main.tex")
    }

    // MARK: follow-ups (#1421 review)

    /// A blocked open records nothing, so after the default's flip the
    /// document follows the new default once the block is gone; an
    /// unblocked open records the preferred engine.
    func testABlockedOpenDoesNotPinTheDocumentPastTheFlip() throws {
        try fakeTeXLive()
        EngineChoice.builtInDefaultForTests = .new
        var current: ProjectFilesV1.Manifest? = Self.manifest(fonts: ["text": "Georgia"])
        let m = model { current }
        defer { m.engineV3.stop() }
        let file = try texFile()
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        XCTAssertFalse(m.engineV3Enabled)
        XCTAssertNil(EngineChoiceStore.entry(for: file))
        current = nil // the table is gone
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled)
        XCTAssertEqual(m.engineChoice.source, .builtInDefault)
        XCTAssertEqual(EngineChoiceStore.entry(for: file), .init(engine: .new, source: .record))
    }

    /// The old View toggle wrote `FlashTeX.EngineV3.enabled` on every
    /// change: a stored false is no choice (removed once, so the default
    /// and its flip apply); a stored true becomes each document's own
    /// choice as it opens, for documents with no entry only, below the app setting.
    func testTheOldGlobalSwitchMigrates() throws {
        try fakeTeXLive()
        let d = EngineV3.defaults
        d.set(false, forKey: EngineV3.enabledKey)
        EngineChoiceStore.migrateLegacyFlag(in: d)
        XCTAssertNil(d.object(forKey: EngineV3.enabledKey), "a stored false is no choice")
        XCTAssertTrue(d.bool(forKey: EngineChoiceStore.legacyMigratedKey))
        XCTAssertNil(EngineChoiceStore.appSetting)
        EngineChoice.builtInDefaultForTests = .new
        let m = model { nil }
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled, "that user gets the flipped default")
        EngineChoice.builtInDefaultForTests = nil

        d.removeObject(forKey: EngineChoiceStore.legacyMigratedKey)
        d.set(true, forKey: EngineV3.enabledKey)
        EngineChoiceStore.migrateLegacyFlag(in: d)
        XCTAssertEqual(d.object(forKey: EngineV3.enabledKey) as? Bool, true, "a stored true is kept, as a fallback")
        let fresh = try texFile(), recorded = try texFile()
        EngineChoiceStore.set(.init(engine: .previous, source: .record), for: recorded)
        XCTAssertEqual(m.openTex(at: fresh, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled)
        XCTAssertEqual(m.engineChoice.source, .legacySwitch)
        XCTAssertEqual(EngineChoiceStore.entry(for: fresh), .init(engine: .new, source: .user), "now that document's own choice")
        XCTAssertEqual(m.openTex(at: recorded, dirty: .discard), .opened)
        XCTAssertFalse(m.engineV3Enabled, "a document with an entry keeps it")
        EngineChoiceStore.appSetting = .previous
        XCTAssertEqual(m.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertFalse(m.engineV3Enabled, "the app setting outranks the old switch")
    }

    /// A window on the new engine opening a document the previous engine
    /// typesets never tells the v3 session about it (no compile of it);
    /// opening another new-engine document tells it exactly once.
    func testTheEngineIsChosenBeforeTheNewEngineOpensTheProject() throws {
        try fakeTeXLive()
        let a = try texFile(), b = try texFile(), c = try texFile()
        EngineChoiceStore.set(.init(engine: .new, source: .user), for: a)
        EngineChoiceStore.set(.init(engine: .new, source: .user), for: c)
        let m = model { nil }
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: a, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled)
        let n = m.engineV3.projectChanges
        XCTAssertEqual(m.openTex(at: b, dirty: .discard), .opened)
        XCTAssertFalse(m.engineV3Enabled)
        RunLoop.main.run(until: Date().addingTimeInterval(0.05)) // replaceProject's deferred call, had it been scheduled
        XCTAssertEqual(m.engineV3.projectChanges, n, "no v3 open of a previous-engine document")
        XCTAssertEqual(m.openTex(at: a, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled)
        let k = m.engineV3.projectChanges
        XCTAssertEqual(m.openTex(at: c, dirty: .discard), .opened)
        RunLoop.main.run(until: Date().addingTimeInterval(0.05))
        XCTAssertEqual(m.engineV3.projectChanges, k + 1, "a v3 window opening a v3 document: once")
    }

    /// A choice made on an unsaved buffer is stored at its first save; Save
    /// As keeps the choice for the new file and re-checks the fallback
    /// rules for its project.
    func testSavingKeepsTheChoiceAndRechecksTheRules() throws {
        try fakeTeXLive()
        var current: ProjectFilesV1.Manifest?
        let m = model { current }
        m.files.policy = .disabled(reason: "test: hermetic (direct writes)")
        defer { m.engineV3.stop() }
        m.replaceProject(entryText: "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n")
        m.chooseEngine(.new)
        XCTAssertTrue(m.engineV3Enabled)
        let first = try dir("save").appendingPathComponent("paper.tex")
        files.append(first)
        XCTAssertTrue(m.saveTexAs(to: first))
        XCTAssertEqual(EngineChoiceStore.entry(for: first), .init(engine: .new, source: .user))
        XCTAssertEqual(m.engineChoiceDocument, first)

        current = Self.manifest(fonts: ["sans": "Inter"])
        let second = try dir("save-as").appendingPathComponent("copy.tex")
        files.append(second)
        XCTAssertTrue(m.saveTexAs(to: second))
        XCTAssertEqual(EngineChoiceStore.entry(for: second), .init(engine: .new, source: .user))
        XCTAssertFalse(m.engineV3Enabled, "its project sets [fonts]")
        XCTAssertEqual(m.engineChoice.blocker, .projectFonts(["sans"]))
    }

    /// Entries are keyed by project root plus entry, follow a moved or
    /// renamed project folder, take over #1421's full-path keys, and are
    /// capped (least recently used dropped).
    func testTheStoreFollowsMovesAndIsBounded() throws {
        let parent = try dir("store")
        let before = parent.appendingPathComponent("thesis")
        try FileManager.default.createDirectory(at: before, withIntermediateDirectories: true)
        let file = before.appendingPathComponent("main.tex")
        try "x".write(to: file, atomically: true, encoding: .utf8)
        EngineChoiceStore.set(.init(engine: .new, source: .user), for: file)
        XCTAssertTrue(EngineChoiceStore.key(file).hasSuffix("/thesis::main.tex"))
        let after = parent.appendingPathComponent("thesis-final")
        try FileManager.default.moveItem(at: before, to: after)
        let moved = after.appendingPathComponent("main.tex")
        files.append(moved)
        XCTAssertEqual(EngineChoiceStore.entry(for: moved), .init(engine: .new, source: .user), "found by the folder's identity")
        let dict = try XCTUnwrap(EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey))
        XCTAssertNotNil(dict[EngineChoiceStore.key(moved)])
        XCTAssertNil(dict[EngineChoiceStore.key(file)], "re-keyed, not copied")

        // #1421 keyed by the full path.
        let old = parent.appendingPathComponent("old.tex")
        files.append(old)
        var all = EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey) ?? [:]
        all[old.resolvingSymlinksInPath().path] = ["engine": "previous", "source": "user"]
        EngineV3.defaults.set(all, forKey: EngineChoiceStore.documentsKey)
        XCTAssertEqual(EngineChoiceStore.entry(for: old), .init(engine: .previous, source: .user))

        let saved = EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey)
        defer { EngineV3.defaults.set(saved, forKey: EngineChoiceStore.documentsKey) }
        EngineV3.defaults.removeObject(forKey: EngineChoiceStore.documentsKey)
        let urls = (0 ... EngineChoiceStore.maxEntries).map { URL(fileURLWithPath: "/nonexistent/engine-choice/p\($0)/main.tex") }
        for u in urls { EngineChoiceStore.set(.init(engine: .previous, source: .record), for: u) }
        XCTAssertEqual(EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey)?.count, EngineChoiceStore.maxEntries)
        XCTAssertNil(EngineChoiceStore.entry(for: urls[0]), "the least recently used went")
        XCTAssertNotNil(EngineChoiceStore.entry(for: urls.last!))
    }

    // MARK: #1427 review

    /// Fake volumes for the identity seam: path → identity (nil: nothing there).
    private func volumes(_ map: [String: EngineChoiceStore.FolderIdentity]) {
        EngineChoiceStore.identityProvider = { map[$0] }
    }

    private func storeRaw(_ key: String, _ d: [String: Any]) {
        var all = EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey) ?? [:]
        all[key] = d
        EngineV3.defaults.set(all, forKey: EngineChoiceStore.documentsKey)
    }

    private func rawKeys() -> Set<String> { Set((EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey) ?? [:]).keys) }

    /// A move is followed only on the same volume (by UUID), with
    /// persistent file IDs, the old root gone and its volume mounted.
    func testAMoveIsFollowedOnlyWhenTheVolumeProvesIt() {
        typealias ID = EngineChoiceStore.FolderIdentity
        let entry: [String: Any] = ["engine": "new", "source": "user", "entry": "main.tex", "dir_id": "VOL-A:42"]
        let oldKey = "/fake/A/old::main.tex"
        let moved = URL(fileURLWithPath: "/fake/A/new/main.tex")

        // Positive: volume A, persistent IDs, same inode, old root gone, A mounted.
        storeRaw(oldKey, entry)
        volumes(["/fake/A/new": ID(volumeUUID: "VOL-A", persistentIDs: true, inode: 42),
                 "/fake/A": ID(volumeUUID: "VOL-A", persistentIDs: true, inode: 2)])
        XCTAssertEqual(EngineChoiceStore.entry(for: moved), .init(engine: .new, source: .user))
        XCTAssertFalse(rawKeys().contains(oldKey), "re-keyed")
        EngineChoiceStore.set(nil, for: moved)

        // A mismatched volume UUID: the same inode number on volume B.
        storeRaw(oldKey, entry)
        volumes(["/fake/A/new": ID(volumeUUID: "VOL-B", persistentIDs: true, inode: 42),
                 "/fake/A": ID(volumeUUID: "VOL-A", persistentIDs: true, inode: 2)])
        XCTAssertNil(EngineChoiceStore.entry(for: moved))
        XCTAssertTrue(rawKeys().contains(oldKey), "the old entry is left alone")

        // A reused inode where the old root's volume is not mounted (its
        // path now falls on the boot volume): unprovable, so a new folder.
        volumes(["/fake/A/new": ID(volumeUUID: "VOL-A", persistentIDs: true, inode: 42),
                 "/": ID(volumeUUID: "BOOT", persistentIDs: true, inode: 2)])
        XCTAssertNil(EngineChoiceStore.entry(for: moved))
        XCTAssertTrue(rawKeys().contains(oldKey))

        // A volume without persistent IDs (FAT, exFAT, SMB, NFS): never followed, and nothing stored to follow.
        volumes(["/fake/A/new": ID(volumeUUID: "VOL-A", persistentIDs: false, inode: 42),
                 "/fake/A": ID(volumeUUID: "VOL-A", persistentIDs: false, inode: 2)])
        XCTAssertNil(EngineChoiceStore.entry(for: moved))
        XCTAssertTrue(rawKeys().contains(oldKey))
        let fat = URL(fileURLWithPath: "/fake/A/fat/main.tex")
        volumes(["/fake/A/fat": ID(volumeUUID: "VOL-A", persistentIDs: false, inode: 7)])
        EngineChoiceStore.set(.init(engine: .previous, source: .record), for: fat)
        let d = EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey)?[EngineChoiceStore.key(fat)] as? [String: Any]
        XCTAssertNotNil(d)
        XCTAssertNil(d?["dir_id"])

        // The old root still exists (a copy, not a move): not followed.
        volumes(["/fake/A/new": ID(volumeUUID: "VOL-A", persistentIDs: true, inode: 42),
                 "/fake/A/old": ID(volumeUUID: "VOL-A", persistentIDs: true, inode: 43),
                 "/fake/A": ID(volumeUUID: "VOL-A", persistentIDs: true, inode: 2)])
        XCTAssertNil(EngineChoiceStore.entry(for: moved))
        EngineChoiceStore.set(nil, for: fat)
        var all = EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey) ?? [:]
        all[oldKey] = nil
        EngineV3.defaults.set(all, forKey: EngineChoiceStore.documentsKey)
    }

    /// Pruning evicts automatic records before deliberate choices, then the
    /// least recently used, then by key: deterministic with equal dates.
    func testPruningKeepsChoicesAndIsDeterministic() {
        let same = Date(timeIntervalSince1970: 1_000_000)
        var dict: [String: Any] = [:]
        for i in 0 ..< EngineChoiceStore.maxEntries - 5 { dict[String(format: "/u/%04d::main.tex", i)] = ["source": "user", "engine": "new", "used_at": same] }
        for i in 0 ..< 10 { dict[String(format: "/r/%04d::main.tex", i)] = ["source": "record", "engine": "previous", "used_at": same] }
        let pruned = EngineChoiceStore.prune(dict)
        XCTAssertEqual(pruned.count, EngineChoiceStore.maxEntries)
        XCTAssertEqual(pruned.keys.filter { $0.hasPrefix("/u/") }.count, EngineChoiceStore.maxEntries - 5, "every user choice kept")
        XCTAssertEqual(Set(pruned.keys.filter { $0.hasPrefix("/r/") }), Set((5 ..< 10).map { String(format: "/r/%04d::main.tex", $0) }),
                       "equal dates: the first keys go")
        XCTAssertEqual(Set(EngineChoiceStore.prune(dict).keys), Set(pruned.keys), "the same every time")
        var older = dict
        older["/r/0009::main.tex"] = ["source": "record", "engine": "previous", "used_at": same.addingTimeInterval(-1)]
        XCTAssertNil(EngineChoiceStore.prune(older)["/r/0009::main.tex"], "the least recently used record goes first")
    }

    /// The migration runs once: run again without resetting its flag, it
    /// touches nothing (a later stored false is not removed, a true is kept).
    func testTheMigrationRunsOnce() {
        let d = EngineV3.defaults
        d.set(false, forKey: EngineV3.enabledKey)
        EngineChoiceStore.migrateLegacyFlag(in: d)
        XCTAssertNil(d.object(forKey: EngineV3.enabledKey))
        d.set(false, forKey: EngineV3.enabledKey)
        EngineChoiceStore.migrateLegacyFlag(in: d)
        XCTAssertEqual(d.object(forKey: EngineV3.enabledKey) as? Bool, false, "the second run is a no-op")
        d.set(true, forKey: EngineV3.enabledKey)
        EngineChoiceStore.migrateLegacyFlag(in: d)
        EngineChoiceStore.migrateLegacyFlag(in: d)
        XCTAssertEqual(d.object(forKey: EngineV3.enabledKey) as? Bool, true)
        XCTAssertTrue(d.bool(forKey: EngineChoiceStore.legacyMigratedKey))
    }

    /// Settings lists documents whose choice the old switch made, and the
    /// reset forgets every choice (records stay) and the switch.
    func testSettingsShowsAndResetsLegacyChoices() throws {
        try fakeTeXLive()
        EngineV3.defaults.set(true, forKey: EngineV3.enabledKey)
        let m = model { nil }
        defer { m.engineV3.stop() }
        let a = try texFile(), b = try texFile(), r = try texFile()
        XCTAssertEqual(m.openTex(at: a, dirty: .discard), .opened)
        XCTAssertEqual(m.openTex(at: b, dirty: .discard), .opened)
        m.chooseEngine(.previous) // b: the user's own choice now
        EngineChoiceStore.set(.init(engine: .previous, source: .record), for: r)
        let counts = EngineChoiceStore.choiceCounts
        XCTAssertGreaterThanOrEqual(counts.user, 2)
        XCTAssertGreaterThanOrEqual(counts.fromLegacySwitch, 1)
        let note = try XCTUnwrap(EngineChoiceSettingsSection.choicesNote(user: 2, fromLegacySwitch: 1, legacyOn: true))
        XCTAssertTrue(note.hasPrefix("2 documents have their own engine choice, 1 of them set to the new engine by the earlier Engine v3 Preview switch."), note)
        XCTAssertNil(EngineChoiceSettingsSection.choicesNote(user: 0, fromLegacySwitch: 0, legacyOn: false))
        XCTAssertEqual(EngineChoiceSettingsSection.choicesNote(user: 1, fromLegacySwitch: 0, legacyOn: false), "1 document has its own engine choice.")
        EngineChoiceStore.resetChoices()
        XCTAssertEqual(EngineChoiceStore.choiceCounts.user, 0)
        XCTAssertNil(EngineChoiceStore.entry(for: a))
        XCTAssertEqual(EngineChoiceStore.entry(for: r), .init(engine: .previous, source: .record), "records stay")
        XCTAssertFalse(EngineChoiceStore.legacyAllNew)
    }

    /// Save As: into a blocked project writes no record; onto a path with
    /// an entry, that entry decides; and the new path gets no v3 open
    /// before its engine is chosen.
    func testSaveAsReadsTheNewProjectFirstAndKeepsAnExistingEntry() throws {
        try fakeTeXLive()
        EngineChoice.builtInDefaultForTests = .new
        var current: ProjectFilesV1.Manifest?
        let m = model { current }
        m.files.policy = .disabled(reason: "test: hermetic (direct writes)")
        defer { m.engineV3.stop() }
        let start = try texFile()
        XCTAssertEqual(m.openTex(at: start, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled)
        XCTAssertEqual(m.engineChoice.source, .builtInDefault)

        current = Self.manifest(fonts: ["text": "Georgia"])
        let blocked = try dir("blocked").appendingPathComponent("main.tex")
        files.append(blocked)
        let n = m.engineV3.projectChanges
        XCTAssertTrue(m.saveTexAs(to: blocked))
        XCTAssertFalse(m.engineV3Enabled)
        XCTAssertNil(EngineChoiceStore.entry(for: blocked), "no record into a blocked project")
        RunLoop.main.run(until: Date().addingTimeInterval(0.05))
        XCTAssertEqual(m.engineV3.projectChanges, n, "no v3 open of the new path")

        current = nil
        let chosen = try dir("chosen").appendingPathComponent("main.tex")
        files.append(chosen)
        EngineChoiceStore.set(.init(engine: .previous, source: .user), for: chosen)
        XCTAssertEqual(m.openTex(at: start, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled)
        XCTAssertTrue(m.saveTexAs(to: chosen))
        XCTAssertFalse(m.engineV3Enabled, "the file's own choice decides")
        XCTAssertEqual(m.engineChoice.source, .user)
        XCTAssertEqual(EngineChoiceStore.entry(for: chosen), .init(engine: .previous, source: .user))
    }

    /// A texinputs file outside the root is a snapshot input: changing it
    /// outside the app invalidates the stored pages.
    func testOutsideTexinputsInvalidateStoredPages() throws {
        let root = try dir("snap-root"), shared = try dir("snap-shared")
        let sty = shared.appendingPathComponent("lab.sty")
        try "% one".write(to: sty, atomically: true, encoding: .utf8)
        let inputs = EngineV3Snapshot.withExternal(try XCTUnwrap(EngineV3Snapshot.inputs(root: root)), paths: [sty.path])
        XCTAssertNotNil(inputs[EngineV3Snapshot.externalPrefix + sty.path])
        let snap = EngineV3Snapshot(main: "main.tex", documents: [:], inputs: inputs, pages: [], pixelsPerPoint: 2, dark: false, savedAt: Date())
        XCTAssertTrue(EngineV3Snapshot.inputsMatch(snap, root: root))
        try "% two, longer".write(to: sty, atomically: true, encoding: .utf8)
        XCTAssertFalse(EngineV3Snapshot.inputsMatch(snap, root: root))
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
