import AppKit
import SwiftUI
import XCTest
import HostedWindows
import FlashTeXProtocol
@testable import FlashTeXMac

/// One engine (EngineChoice.swift; the old-engine retirement plan's stage S6,
/// #1236; app-parity rows A8, A9, A17): the TeX distribution the engine
/// prepares its format from and what the window says when there is none
/// (owner ruling RQ20: a message with a Download button, no other engine),
/// the one-time notice for a document last typeset by the retired previous
/// engine (ruling 4, plan §5.3), the per-document records, the retired
/// defaults and environment (§5.5), fontspec / `[fonts]` documents routed to
/// Unicode mode in memory, and the engine finding flashtex.toml's `texinputs`.
@MainActor
final class EngineChoiceTests: XCTestCase {
    private var env = EnvironmentOverride()
    private var dirs: [URL] = []
    private var files: [URL] = []

    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("engine-choice-tests-\(getpid())").path)
        env.set("FLASHTEX_ENGINE_V3", "")
        env.set("FLASHTEX_MODE", "")
        // No bundle unless a test configures one (never a lock file of this Mac's).
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
        // No Unicode host unless a test provides one.
        env.set("FLASHTEX_HOST_UNICODE", "none")
        EngineV3Bundle.forgetConsent()
    }

    override func tearDown() {
        EngineChoiceStore.identityProvider = EngineChoiceStore.liveIdentity
        for f in files { EngineChoiceStore.set(nil, for: f) }
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

    private func texFile(_ name: String = "main.tex", in d: URL? = nil, body: String = "Hello.", preamble: String = "") throws -> URL {
        let url = try (d ?? dir("doc")).appendingPathComponent(name)
        try "\\documentclass{article}\n\(preamble)\\begin{document}\n\(body)\n\\end{document}\n".write(to: url, atomically: true, encoding: .utf8)
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

    /// A Unicode host the app finds (an executable file; never run here).
    private func fakeUnicodeHost() throws {
        let exe = try dir("unicode-host").appendingPathComponent("flashtex-host-unicode")
        FileManager.default.createFile(atPath: exe.path, contents: Data("#!/bin/sh\nexit 1\n".utf8), attributes: [.posixPermissions: 0o755])
        env.set("FLASHTEX_HOST_UNICODE", exe.path)
    }

    private static func manifest(fonts: [String: String] = [:], pin: [String: String] = [:], libraries: [String: String] = [:],
                                 mode: String? = nil, files: String = "[]") -> ProjectFilesV1.Manifest {
        func obj(_ d: [String: String]) -> String { "{" + d.sorted { $0.key < $1.key }.map { "\"\($0.key)\":\"\($0.value)\"" }.joined(separator: ",") + "}" }
        let table = ["text", "math", "mono", "sans"].map { "\"\($0)\":" + (fonts[$0].map { "\"\($0)\"" } ?? "null") }.joined(separator: ",")
        let modeField = mode.map { ",\"mode\":\"\($0)\"" } ?? ""
        let json = """
        {"path":"/p/flashtex.toml","exists":true,"manifest_dir":"/p",
         "manifest":{"project":{"entry":"main.tex","texinputs":[],"output":null\(modeField)},"fonts":{\(table)},
                     "packages":{"source":"ctan","fetch":"ask","pin":\(obj(pin)),"path":\(obj(libraries))},"library":null},
         "warnings":[],"texinputs":[],"files":\(files),"diagnostics":[],"template":""}
        """
        return try! JSONDecoder().decode(ProjectFilesV1.Manifest.self, from: Data(json.utf8))
    }

    private func model(manifest: @escaping () -> ProjectFilesV1.Manifest?) -> ShellModel {
        let m = ShellModel()
        m.manifest.reader = { _, _ in manifest().map { .success($0) } ?? .failure(.init("no manifest (test)")) }
        return m
    }

    // MARK: the manifest

    /// A8 / A9: the `[fonts]` roles that name a family (only Unicode mode
    /// applies them); pins and libraries are the engine's too and change
    /// nothing here.
    func testManifestRules() {
        XCTAssertEqual(EngineChoice.fontRoles(manifest: Self.manifest().manifest), [])
        XCTAssertEqual(EngineChoice.fontRoles(manifest: Self.manifest(fonts: ["text": "Georgia", "math": "Libertinus Math"]).manifest),
                       ["text", "math"])
        XCTAssertEqual(EngineChoice.fontRoles(manifest: Self.manifest(fonts: ["text": "  "]).manifest), [], "a blank role is the class default")
        XCTAssertEqual(EngineChoice.fontRoles(manifest: Self.manifest(pin: ["siunitx": "3.3.24"]).manifest), [])
        XCTAssertEqual(EngineChoice.fontRoles(manifest: Self.manifest(libraries: ["mylib": "../mylib"]).manifest), [])
    }

    // MARK: the TeX distribution

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

    /// A17 / RQ20: without TeX Live (and no bundle) the engine cannot prepare
    /// its format. The host says so, and the window shows a plain,
    /// non-blocking message; there is no other engine and no Download button
    /// when no TeX bundle is configured.
    func testNoTeXLiveSaysCompilingNeedsTeX() throws {
        env.set("FLASHTEX_TEXLIVE_BIN", "/nonexistent/texlive/bin")
        env.set("FLASHTEX_HOST", "none")
        let m = model { nil }
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertNil(m.texMissing)
        m.engineV3HostLacksTeXLive()
        XCTAssertEqual(m.texMissing, .noTeX)
        XCTAssertFalse(m.texMissingDismissed)
        XCTAssertFalse(m.texDownloadAvailable, "no bundle is configured")
        XCTAssertEqual(TeXMissing.headline, "Compiling needs the TeX bundle or a TeX Live installation.")
        XCTAssertTrue(TeXMissing.noTeX.detail.contains("Install MacTeX or TeX Live"), TeXMissing.noTeX.detail)
        m.texMissingDismissed = true
        m.engineV3HostLacksTeXLive()
        XCTAssertFalse(m.texMissingDismissed, "a new report shows the message again")
    }

    /// NOTEX-WIRING and RQ20: a configured bundle (the environment, or a
    /// `flashtex-bundle.lock`) is a distribution; nothing is downloaded
    /// before the user agrees (the host runs offline until then). "Not Now"
    /// says that compiling needs the TeX files, and the message's Download
    /// button allows the download (no other engine is used meanwhile).
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
        XCTAssertEqual(EngineV3Bundle.currentGate(), .ask(config), "launching the host asks first")
        // A cache already there (an interrupted download, or one made outside
        // the app) does not skip the question: the stored answer decides.
        let cachedIndex = cache.appendingPathComponent("\(digest)/index.gz")
        try FileManager.default.createDirectory(at: cachedIndex.deletingLastPathComponent(), withIntermediateDirectories: true)
        FileManager.default.createFile(atPath: cachedIndex.path, contents: Data())
        XCTAssertEqual(EngineV3Bundle.currentGate(), .ask(config), "a cached index is not consent")
        XCTAssertEqual(EngineV3HostProcess.environment(host: host)["FLASHTEX_BUNDLE_OFFLINE"], "1")

        // The window (RQ20): "Not Now" says that compiling needs the TeX
        // files, with a Download button; Download allows the download.
        let m = model { nil }
        m.engineV3Enabled = true // the session (FLASHTEX_HOST=none: no host starts)
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: try texFile(), dirty: .discard), .opened)
        XCTAssertNil(m.texMissing)
        m.engineV3.answerBundleConsent(false)
        XCTAssertEqual(m.texMissing, .declined)
        XCTAssertTrue(m.engineV3Enabled, "no other engine takes over")
        XCTAssertTrue(m.texDownloadAvailable)
        XCTAssertEqual(EngineV3Bundle.consent(for: config), false)
        XCTAssertTrue(TeXMissing.declined.detail.contains("were not downloaded"), TeXMissing.declined.detail)
        m.retryTeXDownload()
        XCTAssertNil(m.texMissing)
        XCTAssertEqual(EngineV3Bundle.consent(for: config), true, "Download is the user's consent")
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

    /// The host itself reports no TeX Live (and no format): the window says
    /// so (RQ20), whatever the TeX bundle's state.
    func testTheHostsNoTeXLiveReportIsSaid() throws {
        try fakeTeXLive()
        let m = model { nil }
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: try texFile(), dirty: .discard), .opened)
        m.engineV3HostLacksTeXLive()
        XCTAssertEqual(m.texMissing, .noTeX)
        EngineV3Bundle.setConsent(false, for: .init(url: "https://e.org/b.ttb", digest: String(repeating: "ab", count: 32), origin: "environment"))
        m.engineV3HostLacksTeXLive()
        XCTAssertEqual(m.texMissing, .declined, "after a declined download, the message says the TeX files were not downloaded")
    }

    // MARK: records and the previous engine

    /// Ruling 4 at S6 (plan §5.3): a document last typeset by the previous
    /// engine opens on the new engine, with a one-time, dismissable notice
    /// that says so and why (and VoiceOver hears it); its record becomes
    /// `new`, so it is not shown again. A document with no record gets one
    /// and no notice.
    func testAPreviousEngineRecordOpensOnTheNewEngineWithAOneTimeNotice() throws {
        try fakeTeXLive()
        let file = try texFile()
        EngineChoiceStore.set(.init(engine: "previous", source: .record), for: file)
        XCTAssertEqual(EngineChoiceStore.entry(for: file)?.isRetiredEngine, true)
        let m = model { nil }
        defer { m.engineV3.stop() }
        var lines: [String] = []
        m.engineLog = { lines.append($0) }
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        XCTAssertTrue(m.previousEngineNotice)
        XCTAssertEqual(m.engineAnnouncements, [PreviousEngineNoticeBanner.headline])
        XCTAssertEqual(EngineChoiceStore.entry(for: file), .init(engine: "new", source: .record), "the record now says new")
        XCTAssertEqual(lines, ["engine: new (classic) main.tex"], "the line the attribution tools read")
        XCTAssertTrue(PreviousEngineNoticeBanner.detail.contains("previous engine, which has been retired"), PreviousEngineNoticeBanner.detail)
        m.previousEngineNotice = false // Dismiss

        let again = model { nil }
        defer { again.engineV3.stop() }
        XCTAssertEqual(again.openTex(at: file, dirty: .discard), .opened)
        XCTAssertFalse(again.previousEngineNotice, "once only")
        XCTAssertTrue(again.engineAnnouncements.isEmpty)

        // A user's choice of the previous engine is a record of it too.
        let chosen = try texFile()
        EngineChoiceStore.set(.init(engine: "previous", source: .user), for: chosen)
        XCTAssertEqual(again.openTex(at: chosen, dirty: .discard), .opened)
        XCTAssertTrue(again.previousEngineNotice)
        XCTAssertEqual(EngineChoiceStore.entry(for: chosen)?.engine, "new")

        let fresh = try texFile()
        XCTAssertNil(EngineChoiceStore.entry(for: fresh))
        XCTAssertEqual(again.openTex(at: fresh, dirty: .discard), .opened)
        XCTAssertFalse(again.previousEngineNotice)
        XCTAssertEqual(EngineChoiceStore.entry(for: fresh), .init(engine: "new", source: .record))
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
        EngineChoiceStore.set(.init(engine: "new", source: .user), for: file)
        XCTAssertTrue(EngineChoiceStore.key(file).hasSuffix("/thesis::main.tex"))
        let after = parent.appendingPathComponent("thesis-final")
        try FileManager.default.moveItem(at: before, to: after)
        let moved = after.appendingPathComponent("main.tex")
        files.append(moved)
        XCTAssertEqual(EngineChoiceStore.entry(for: moved), .init(engine: "new", source: .user), "found by the folder's identity")
        let dict = try XCTUnwrap(EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey))
        XCTAssertNotNil(dict[EngineChoiceStore.key(moved)])
        XCTAssertNil(dict[EngineChoiceStore.key(file)], "re-keyed, not copied")

        // #1421 keyed by the full path.
        let old = parent.appendingPathComponent("old.tex")
        files.append(old)
        var all = EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey) ?? [:]
        all[old.resolvingSymlinksInPath().path] = ["engine": "previous", "source": "user"]
        EngineV3.defaults.set(all, forKey: EngineChoiceStore.documentsKey)
        XCTAssertEqual(EngineChoiceStore.entry(for: old), .init(engine: "previous", source: .user))
        // An engine this build does not know is no entry.
        all = EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey) ?? [:]
        let odd = parent.appendingPathComponent("odd.tex")
        files.append(odd)
        all[EngineChoiceStore.key(odd)] = ["engine": "other", "source": "record"]
        EngineV3.defaults.set(all, forKey: EngineChoiceStore.documentsKey)
        XCTAssertNil(EngineChoiceStore.entry(for: odd))

        let saved = EngineV3.defaults.dictionary(forKey: EngineChoiceStore.documentsKey)
        defer { EngineV3.defaults.set(saved, forKey: EngineChoiceStore.documentsKey) }
        EngineV3.defaults.removeObject(forKey: EngineChoiceStore.documentsKey)
        let urls = (0 ... EngineChoiceStore.maxEntries).map { URL(fileURLWithPath: "/nonexistent/engine-choice/p\($0)/main.tex") }
        for u in urls { EngineChoiceStore.set(.init(engine: "new", source: .record), for: u) }
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
        XCTAssertEqual(EngineChoiceStore.entry(for: moved), .init(engine: "new", source: .user))
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
        EngineChoiceStore.set(.init(engine: "previous", source: .record), for: fat)
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

    // MARK: retired defaults and environment (plan §5.5)

    /// The first launch of the S6 build removes the defaults it no longer
    /// reads, once per defaults domain.
    func testTheRetiredDefaultsAreRemovedOnce() throws {
        let suite = "tech.flashtex.tests.retired-defaults.\(getpid()).\(UUID().uuidString)"
        let d = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { d.removePersistentDomain(forName: suite) }
        for k in EngineChoiceStore.retiredDefaultsKeys { d.set(true, forKey: k) }
        d.set(["x::main.tex": ["engine": "previous", "source": "record"]], forKey: EngineChoiceStore.documentsKey)
        d.set("/opt/flashtex-host", forKey: EngineV3.hostPathKey)
        EngineChoiceStore.removeRetiredDefaults(in: d)
        for k in EngineChoiceStore.retiredDefaultsKeys { XCTAssertNil(d.object(forKey: k), k) }
        XCTAssertNotNil(d.dictionary(forKey: EngineChoiceStore.documentsKey), "the records are kept (their previous entries get the notice)")
        XCTAssertEqual(d.string(forKey: EngineV3.hostPathKey), "/opt/flashtex-host", "kept")
        XCTAssertTrue(d.bool(forKey: EngineChoiceStore.retiredDefaultsRemovedKey))
        d.set(false, forKey: EngineChoiceStore.retiredDefaultsKeys[0])
        EngineChoiceStore.removeRetiredDefaults(in: d)
        XCTAssertEqual(d.object(forKey: EngineChoiceStore.retiredDefaultsKeys[0]) as? Bool, false, "the second run is a no-op")
        XCTAssertEqual(Set(EngineChoiceStore.retiredDefaultsKeys), ["FlashTeX.EngineV3.enabled", "FlashTeX.EngineV3.legacyMigrated.v1",
                                                                    "FlashTeX.EngineV3.defaultEngine", "FlashTeX.Preview.v1.debugStatus"])
    }

    /// None of the retired defaults or environment variables is read any
    /// more: no source file of the app names one, except the list of
    /// defaults to remove.
    func testNoRetiredDefaultOrEnvironmentIsRead() throws {
        var root = URL(fileURLWithPath: #filePath)
        for _ in 0 ..< 3 { root = root.deletingLastPathComponent() } // apps/mac
        let sources = root.appendingPathComponent("Sources")
        let retired = ["FLASHTEX_PREVIEW_V2", "FLASHTEX_LAYOUT_CAPABILITIES", "FLASHTEX_PDF_EXACT", "FLASHTEX_COMPILER",
                       "FLASHTEX_RENDER", "FLASHTEX_PREVIEW_CONTROLLER"] + EngineChoiceStore.retiredDefaultsKeys
        let walker = try XCTUnwrap(FileManager.default.enumerator(at: sources, includingPropertiesForKeys: nil))
        var checked = 0
        for case let url as URL in walker where url.pathExtension == "swift" {
            let text = try String(contentsOf: url, encoding: .utf8)
            checked += 1
            for name in retired where text.contains("\"" + name + "\"") {
                XCTAssertTrue(url.lastPathComponent == "EngineChoice.swift" && EngineChoiceStore.retiredDefaultsKeys.contains(name),
                              "\(url.lastPathComponent) still names \(name)")
            }
        }
        XCTAssertGreaterThan(checked, 100)
    }

    // MARK: Unicode routing (A9, the retirement plan's S6)

    /// A document that uses fontspec (or whose flashtex.toml sets
    /// `[fonts]`) and whose mode nothing sets is typeset in Unicode mode: the
    /// routing is in memory, flashtex.toml is not written, and the mode item
    /// says why. A mode the project or the document sets is never overruled,
    /// and without Unicode mode's host the document stays in Classic mode.
    func testAFontspecDocumentIsRoutedToUnicodeModeInMemory() throws {
        try fakeTeXLive()
        try fakeUnicodeHost()
        var current: ProjectFilesV1.Manifest?
        let m = model { current }
        defer { m.engineV3.stop() }
        let d = try dir("fontspec")
        let file = try texFile(in: d, preamble: "\\usepackage{fontspec}\n")
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        XCTAssertTrue(m.unicodeModeAvailable)
        XCTAssertEqual(m.documentMode.mode, .unicode)
        XCTAssertEqual(m.documentMode.source, EngineV3Mode.detectedSource)
        XCTAssertEqual(m.documentMode.format, "xelatex")
        XCTAssertEqual(m.unicodeRouting, "the document uses the fontspec package")
        XCTAssertNil(m.unicodeModeSuggestion, "no warning: it is in Unicode mode")
        XCTAssertTrue(m.documentMode.sourceLine.hasPrefix("Detected:"), m.documentMode.sourceLine)
        XCTAssertFalse(FileManager.default.fileExists(atPath: d.appendingPathComponent(ProjectManifest.fileName).path),
                       "the project's flashtex.toml is not written")

        // An explicit mode wins: flashtex.toml's, or the document's own line.
        current = Self.manifest(mode: "classic")
        m.manifest.refresh()
        XCTAssertEqual(m.documentMode.mode, .classic)
        XCTAssertEqual(m.documentMode.source, "flashtex.toml")
        XCTAssertEqual(m.unicodeModeSuggestion?.kind, .package("fontspec"), "the mode item warns instead")
        current = nil
        m.manifest.refresh()
        let pinned = try texFile(in: try dir("pinned"), preamble: "\\usepackage{fontspec}\n")
        try ("% !TEX program = pdflatex\n" + (try String(contentsOf: pinned, encoding: .utf8))).write(to: pinned, atomically: true, encoding: .utf8)
        XCTAssertEqual(m.openTex(at: pinned, dirty: .discard), .opened)
        XCTAssertEqual(m.documentMode.mode, .classic)
        XCTAssertEqual(m.documentMode.source, "% !TEX program")

        // [fonts] in flashtex.toml routes a plain document too.
        current = Self.manifest(fonts: ["text": "Georgia"])
        let plain = try texFile(in: try dir("fonts"))
        XCTAssertEqual(m.openTex(at: plain, dirty: .discard), .opened)
        XCTAssertEqual(m.documentMode.mode, .unicode)
        XCTAssertEqual(m.unicodeRouting, "flashtex.toml sets [fonts] (text)")
        current = nil
        m.manifest.refresh()
        XCTAssertEqual(m.documentMode.mode, .classic, "no [fonts], nothing that needs Unicode")

        // Without Unicode mode's host: Classic, and TeX's own error says why.
        env.set("FLASHTEX_HOST_UNICODE", "none")
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        XCTAssertFalse(m.unicodeModeAvailable)
        XCTAssertEqual(m.documentMode.mode, .classic)
        XCTAssertNil(m.unicodeRouting)
    }

    /// A Classic run's own report (fontspec's fatal error) routes a document
    /// the preamble scan missed, until the document opens again.
    func testTheEnginesReportRoutesTheDocument() throws {
        try fakeTeXLive()
        try fakeUnicodeHost()
        let m = model { nil }
        defer { m.engineV3.stop() }
        let file = try texFile(preamble: "\\input{fonts-setup}\n") // a file the scan cannot read
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        XCTAssertEqual(m.documentMode.mode, .classic)
        m.engineV3NeedsUnicodeFonts(.init(kind: .engineReport("fontspec Error: requires XeTeX or LuaTeX"), file: nil))
        XCTAssertEqual(m.documentMode.mode, .unicode)
        XCTAssertEqual(m.documentMode.source, EngineV3Mode.detectedSource)
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        XCTAssertEqual(m.documentMode.mode, .classic, "the report holds until the document opens again")
    }

    // MARK: A8: texinputs in the engine

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

    /// flashtex.toml's `texinputs` (a directory under the root and one
    /// outside it): the engine finds their packages by name, as `TEXINPUTS`
    /// would, and a project file of the same name wins. When the manifest
    /// stops naming them, they are gone again.
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
        let m = model { current }
        m.engineV3Enabled = true
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: main, dirty: .discard), .opened)
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
