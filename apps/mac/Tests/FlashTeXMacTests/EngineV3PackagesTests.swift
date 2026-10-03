import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Package resolution under the new engine (lane V3-PACKAGE-RESOLUTION;
/// app-parity rows A8 and B11): TeX's ``File `x.sty' not found`` names the
/// package, Fetch is offered and fetches through the project-files helper
/// into the package cache, and the delivered files reach the engine through
/// the session's copy; flashtex.toml's `[packages] pin` versions and
/// `[packages] path` libraries are resolved (no network) before the first
/// compile and come before TeX Live's copies, so those projects no longer
/// fall back to the previous engine. No test here touches the network: the
/// package source is a `file://` archive in a temporary directory.
@MainActor
final class EngineV3PackagesTests: XCTestCase {
    private var env = EnvironmentOverride()
    private var dirs: [URL] = []
    private var files: [URL] = []

    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-packages-tests-\(getpid())").path)
        env.set("FLASHTEX_ENGINE_V3", "") // the per-document choice decides
        EngineChoiceStore.appSetting = nil
        EngineChoice.builtInDefaultForTests = nil
    }

    override func tearDown() {
        for f in files { EngineChoiceStore.set(nil, for: f) }
        EngineChoiceStore.appSetting = nil
        for d in dirs {
            // The fixture cache's files are read-only (the resolver's store).
            if let e = FileManager.default.enumerator(atPath: d.path) {
                for case let rel as String in e { try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: d.appendingPathComponent(rel).path) }
            }
            try? FileManager.default.removeItem(at: d)
        }
        env.restore()
    }

    // MARK: helpers

    private func dir(_ name: String) throws -> URL {
        let d = FileManager.default.temporaryDirectory.appendingPathComponent("v3-packages-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        dirs.append(d)
        return d.resolvingSymlinksInPath()
    }

    private func write(_ text: String, to url: URL) throws {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: url, atomically: true, encoding: .utf8)
    }

    private static func decode<T: Decodable>(_ object: Any, as: T.Type) throws -> T {
        try JSONDecoder().decode(T.self, from: JSONSerialization.data(withJSONObject: object))
    }

    /// A manifest payload as the helper's `manifest` operation answers it.
    private static func manifest(pin: [String: String] = [:], libraries: [String: String] = [:]) throws -> ProjectFilesV1.Manifest {
        try decode([
            "path": "/p/flashtex.toml", "exists": true, "manifest_dir": "/p",
            "manifest": [
                "project": ["entry": "main.tex", "texinputs": [], "output": NSNull()],
                "fonts": ["text": NSNull(), "math": NSNull(), "mono": NSNull(), "sans": NSNull()],
                "packages": ["source": "ctan", "fetch": "ask", "pin": pin, "path": libraries],
                "library": NSNull(),
            ],
            "warnings": [], "texinputs": [], "files": [], "diagnostics": [], "template": "",
        ] as [String: Any], as: ProjectFilesV1.Manifest.self)
    }

    private static func file(_ path: String, _ text: String) -> [String: Any] {
        ["path": path, "text": text, "sha256": "00", "bytes": text.utf8.count]
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    private func errors(_ m: ShellModel) -> [String] { m.displayedDiagnostics.filter { $0.severity == .error }.map(\.message) }

    // MARK: the names (pure)

    /// TeX's wording under the new engine names the package or class; a
    /// missing `.tex` is not a package.
    func testTeXsMissingFileErrorNamesThePackage() {
        func row(_ message: String) -> RuntimeV1.Diagnostic {
            RuntimeV1.Diagnostic(severity: .error, message: message, source: nil, recovery: nil, code: "latex/file-not-found")
        }
        let rows = [row("LaTeX Error: File `mynotes.sty' not found."), row("LaTeX Error: File `thesis.cls' not found."),
                    row("LaTeX Error: File `chapter1.tex' not found."), row("LaTeX Error: File `../evil.sty' not found."),
                    row("LaTeX Error: File `mynotes.sty' not found.")]
        XCTAssertEqual(ProjectPackagesState.unresolvedNames(in: rows), ["mynotes", "thesis"])
        XCTAssertEqual(ProjectPackagesState.texMissingFile(in: "! LaTeX Error: File `a.sty' not found."), "a.sty")
        XCTAssertNil(ProjectPackagesState.texMissingFile(in: "Undefined control sequence."))
    }

    // MARK: the copy (no host)

    /// Delivered files are written into the copy's own `packages/` folder
    /// (read-only), linked by name after the texinputs; a project file of the
    /// same name wins; files no longer delivered go, links and all; a path
    /// that is not `packages/<name>/<file>` is never written.
    func testDeliveredFilesAreWrittenIntoTheCopyAndLinkedByName() throws {
        let root = try dir("mirror")
        try write("% the project's own", to: root.appendingPathComponent("own.sty"))
        let mirror = EngineV3Mirror(source: root, session: 9_000 + Int.random(in: 0 ..< 1000))
        defer { try? FileManager.default.removeItem(at: mirror.base) }
        _ = mirror.sync(except: ["main.tex"], fingerprints: false, quarantine: false)
        let docs: [ProjectDocuments.ImplicitDocument] = [
            .init(path: "packages/fx/fx.sty", text: "\\ProvidesPackage{fx}\n"),
            .init(path: "packages/fx/fx.cfg", text: "% cfg\n"),
            .init(path: "packages/own/own.sty", text: "% delivered, loses to the project's"),
            .init(path: "packages/../evil.sty", text: "x"),
            .init(path: "elsewhere/x.sty", text: "x"),
        ]
        let links = mirror.materializePackages(docs)
        XCTAssertEqual(links.map(\.name), ["fx.sty", "fx.cfg", "own.sty"])
        mirror.linkTexInputs(links, except: ["main.tex"])
        let fm = FileManager.default
        let pkgDir = mirror.base.appendingPathComponent("packages")
        XCTAssertEqual(try String(contentsOf: mirror.root.appendingPathComponent("fx.sty"), encoding: .utf8), "\\ProvidesPackage{fx}\n")
        XCTAssertEqual(try fm.destinationOfSymbolicLink(atPath: mirror.root.appendingPathComponent("fx.sty").path),
                       pkgDir.appendingPathComponent("fx/fx.sty").path)
        let perms = try fm.attributesOfItem(atPath: pkgDir.appendingPathComponent("fx/fx.sty").path)[.posixPermissions] as? Int
        XCTAssertEqual(perms, 0o444, "the engine only reads it")
        XCTAssertEqual(try String(contentsOf: mirror.root.appendingPathComponent("own.sty"), encoding: .utf8), "% the project's own",
                       "a project file of the same name wins")
        XCTAssertFalse(fm.fileExists(atPath: mirror.base.appendingPathComponent("evil.sty").path))
        XCTAssertFalse(fm.fileExists(atPath: pkgDir.appendingPathComponent("elsewhere").path))

        // A new version replaces the text; fx.cfg is no longer delivered.
        let again = mirror.materializePackages([.init(path: "packages/fx/fx.sty", text: "\\ProvidesPackage{fx}[v2]\n")])
        mirror.linkTexInputs(again, except: ["main.tex"])
        XCTAssertEqual(try String(contentsOf: mirror.root.appendingPathComponent("fx.sty"), encoding: .utf8), "\\ProvidesPackage{fx}[v2]\n")
        XCTAssertFalse(fm.fileExists(atPath: mirror.root.appendingPathComponent("fx.cfg").path))
        XCTAssertFalse(fm.fileExists(atPath: pkgDir.appendingPathComponent("fx/fx.cfg").path))
        // Nothing delivered: nothing left.
        mirror.linkTexInputs(mirror.materializePackages([]), except: ["main.tex"])
        XCTAssertNil(try? fm.destinationOfSymbolicLink(atPath: mirror.root.appendingPathComponent("fx.sty").path))
        XCTAssertEqual((try? fm.contentsOfDirectory(atPath: pkgDir.path)) ?? [], [])
    }

    // MARK: A8: pins and libraries under the new engine (real host)

    /// A project that pins a package TeX Live also has, and uses a local
    /// library, opens with the new engine (no fallback). Its first compile
    /// is held until the pins and libraries are resolved offline, and
    /// typesets with the cached pinned copy and the library, not TeX
    /// Live's. When the pin goes, TeX Live's copy is used again.
    func testAPinnedPackageAndALibraryCompileUnderTheNewEngine() async throws {
        try EngineV3TestHost.require()
        let root = try dir("pinned")
        let main = root.appendingPathComponent("main.tex")
        try write("\\documentclass{article}\n\\usepackage{lipsum}\n\\usepackage{mylib}\n\\begin{document}\n\\pinnedmarker{} \\librarymarker\n\\end{document}\n", to: main)
        files.append(main)
        var current = try Self.manifest(pin: ["lipsum": "1.0-test"], libraries: ["mylib": "../mylib"])
        let m = ShellModel()
        m.detachWorker()
        m.files.policy = .disabled(reason: "test: the resolver hooks answer")
        m.manifest.reader = { _, _ in .success(current) }
        var asked: [[String]] = []
        let pinned = "\\ProvidesPackage{lipsum}[2099/01/01 the pinned test copy]\n\\newcommand{\\pinnedmarker}{Pinned copy.}\n"
        m.projectPackages.offlineResolver = { _, names, _ in
            asked.append(names)
            let packages = names.map { name -> [String: Any] in
                ["name": name, "status": "cached", "version": "1.0-test", "from": "cache", "files": [Self.file("packages/\(name)/\(name).sty", pinned)]]
            }
            let libraries: [[String: Any]] = [["name": "mylib", "version": "abc123", "files": [
                Self.file("packages/mylib/mylib.sty", "\\ProvidesPackage{mylib}\n\\RequirePackage{mylib-extra}\n"),
                Self.file("packages/mylib/mylib-extra.sty", "\\ProvidesPackage{mylib-extra}\n\\newcommand{\\librarymarker}{From the library.}\n"),
            ]]]
            return .success(try! Self.decode(["cache": "/nonexistent", "policy": ["source": "ctan", "fetch": "ask"], "diagnostics": [],
                                              "packages": packages, "libraries": libraries] as [String: Any], as: ProjectFilesV1.ResolvePackages.self))
        }
        EngineChoiceStore.appSetting = .new
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: main, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled, m.engineChoice.explanation)
        XCTAssertNil(m.engineChoice.blocker, "pins and libraries no longer fall back")
        let s = m.engineV3
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount == 1 }
        XCTAssertNil(s.firstError)
        XCTAssertEqual(errors(m), [])
        XCTAssertEqual(asked, [["lipsum"]], "one offline resolution of the pins, with the libraries")
        XCTAssertEqual(m.projectPackages.engineV3Preparations, 1)
        XCTAssertEqual(s.doneCount, 1, "the first compile already had the pinned copy and the library (held, not redone)")
        let copy = try XCTUnwrap(s.projectCopy)
        XCTAssertEqual(try String(contentsOf: copy.appendingPathComponent("lipsum.sty"), encoding: .utf8), pinned)
        XCTAssertTrue(FileManager.default.fileExists(atPath: copy.appendingPathComponent("mylib-extra.sty").path))
        XCTAssertTrue(m.projectPackages.rows.contains { $0.path == "packages/lipsum/lipsum.sty" && ($0.source ?? "").contains("pinned") },
                      "the sidebar's Packages group shows the pinned copy")

        // The pin goes: TeX Live's lipsum, which has no \pinnedmarker.
        current = try Self.manifest(libraries: ["mylib": "../mylib"])
        m.manifest.refresh()
        try await waitUntil("the compile without the pin") { s.firstError != nil && !s.compiling }
        XCTAssertEqual(asked.last, [], "the libraries again, no pins")
        XCTAssertTrue(errors(m).contains { $0.contains("Undefined control sequence") }, "\(errors(m))")
        XCTAssertNotEqual(try? String(contentsOf: copy.appendingPathComponent("lipsum.sty"), encoding: .utf8), pinned)
    }

    // MARK: B11: Fetch under the new engine (real host and helper, no network)

    /// A package neither TeX Live nor the project has: TeX's error offers
    /// Fetch (the Problems row and the consent sheet). The first question is
    /// the helper's REAL offline request (libraries and cache); the sheet
    /// offers the name only, and the source is not asked anything: the
    /// archive does not even exist until after the sheet is up, and a
    /// request to it would have made the package "unavailable". Look Up
    /// describes it, Fetch downloads it through the project-files helper
    /// into the package cache, and the new engine recompiles with it.
    func testFetchIsOfferedAndFetchesIntoTheCacheUnderTheNewEngine() async throws {
        try EngineV3TestHost.require()
        guard ProjectFilesClient.locate() != nil else {
            throw XCTSkip("no flashtex-project-files built (cargo build -p flashtex-project-files)")
        }
        let base = try dir("fetch")
        let archive = base.appendingPathComponent("archive/macros/latex/contrib/fxpkgtest")
        let cache = base.appendingPathComponent("cache")
        env.set("FLASHTEX_PACKAGE_CACHE", cache.path)
        let root = base.appendingPathComponent("project")
        try write("[packages]\nsource = \"file://\(base.appendingPathComponent("archive").path)\"\nfetch = \"ask\"\n", to: root.appendingPathComponent("flashtex.toml"))
        let main = root.appendingPathComponent("main.tex")
        try write("\\documentclass{article}\n\\usepackage{fxpkgtest}\n\\begin{document}\n\\fxmarker\n\\end{document}\n", to: main)
        files.append(main)
        let m = ShellModel()
        m.detachWorker()
        EngineChoiceStore.appSetting = .new
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: main, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled, m.engineChoice.explanation)
        let s = m.engineV3
        let state = m.projectPackages
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the sheet", timeout: 60) { state.shown }
        let row = try XCTUnwrap(m.engineV3Diagnostics.first { ProjectPackagesState.texMissingFile(in: $0.message) == "fxpkgtest.sty" },
                                "\(m.engineV3Diagnostics.map(\.message))")
        // DiagnosticsListView's condition for the row's "Create fxpkgtest.sty" and "Fetch fxpkgtest…".
        XCTAssertEqual(ProjectPackagesState.missingPackages(for: row, projectRoot: m.project.projectRoot), ["fxpkgtest"],
                       "the Problems row offers Create and Fetch for it")
        XCTAssertEqual(state.offers.map(\.name), ["fxpkgtest"])
        XCTAssertFalse(state.offers[0].lookedUp, "offered by name: the source was not asked")
        XCTAssertNil(state.offers[0].version)
        XCTAssertTrue(state.unavailable.isEmpty, "\(state.unavailable)")
        XCTAssertFalse(FileManager.default.fileExists(atPath: cache.path), "nothing is fetched before the user says yes")

        // Now the source exists; Look Up describes the package (still nothing fetched).
        try write("<a href=\"fxpkgtest.sty\">fxpkgtest.sty</a>", to: archive.appendingPathComponent("index.html"))
        try write("\\ProvidesPackage{fxpkgtest}\n\\newcommand{\\fxmarker}{Fetched from the archive.}\n", to: archive.appendingPathComponent("fxpkgtest.sty"))
        let described = await state.lookUp()
        XCTAssertTrue(described, state.note ?? "")
        XCTAssertEqual(state.offers.count, 1)
        XCTAssertTrue(state.offers[0].lookedUp)
        XCTAssertTrue(state.offers[0].sourceURL.hasPrefix("file://"), state.offers[0].sourceURL)
        XCTAssertEqual(state.offers[0].files, ["fxpkgtest.sty"])
        XCTAssertFalse(FileManager.default.fileExists(atPath: cache.path), "Look Up fetches nothing")

        let ok = await state.fetch()
        XCTAssertTrue(ok, state.note ?? "")
        XCTAssertTrue(FileManager.default.fileExists(atPath: cache.appendingPathComponent("fxpkgtest").path), "fetched into the package cache")
        try await waitUntil("the compile with the fetched package") { s.statusNote.hasPrefix("ok") && !s.compiling && s.firstError == nil }
        XCTAssertEqual(errors(m), [])
        XCTAssertEqual(s.pageCount, 1)
        let copy = try XCTUnwrap(s.projectCopy)
        XCTAssertTrue(try String(contentsOf: copy.appendingPathComponent("fxpkgtest.sty"), encoding: .utf8).contains("\\fxmarker"))
    }

    /// The REAL helper, end to end: flashtex.toml is read by it, the pins
    /// and the library are resolved by its `offline` request with
    /// `libraries`, and the first compile (one compile, not two) already
    /// reads the library. The pin is not cached: it is offered by name, the
    /// source (a `file://` path that does not exist) is never asked, and
    /// TeX Live's copy is used meanwhile.
    func testTheRealHelperResolvesTheLibraryOfflineBeforeTheFirstCompile() async throws {
        try EngineV3TestHost.require()
        guard ProjectFilesClient.locate() != nil else {
            throw XCTSkip("no flashtex-project-files built (cargo build -p flashtex-project-files)")
        }
        let base = try dir("real-helper")
        let cache = base.appendingPathComponent("cache")
        env.set("FLASHTEX_PACKAGE_CACHE", cache.path)
        let lib = base.appendingPathComponent("mylib")
        try write("[library]\nname = \"mylib\"\n", to: lib.appendingPathComponent("flashtex.toml"))
        try write("\\ProvidesPackage{mylib}\n\\newcommand{\\librarymarker}{From the library.}\n", to: lib.appendingPathComponent("mylib.sty"))
        let root = base.appendingPathComponent("project")
        try write("[packages]\nsource = \"file://\(base.appendingPathComponent("no-such-archive").path)\"\nfetch = \"ask\"\npin = { lipsum = \"9.9\" }\npath = { mylib = \"../mylib\" }\n",
                  to: root.appendingPathComponent("flashtex.toml"))
        let main = root.appendingPathComponent("main.tex")
        try write("\\documentclass{article}\n\\usepackage{lipsum}\n\\usepackage{mylib}\n\\begin{document}\n\\librarymarker\n\\end{document}\n", to: main)
        files.append(main)
        let m = ShellModel()
        m.detachWorker()
        EngineChoiceStore.appSetting = .new
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.openTex(at: main, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled, m.engineChoice.explanation)
        XCTAssertNotNil(m.manifest.currentSnapshot, m.manifest.status)
        let s = m.engineV3
        let state = m.projectPackages
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount == 1 }
        XCTAssertEqual(errors(m), [])
        XCTAssertEqual(s.doneCount, 1, "one compile: it waited for the manifest and the offline resolution")
        XCTAssertEqual(state.engineV3Preparations, 1)
        XCTAssertEqual(state.engineV3Failures, 0, state.status)
        XCTAssertEqual(state.engineV3Documents().map(\.path), ["packages/mylib/mylib.sty"])
        try await waitUntil("the pin's offer", timeout: 10) { state.shown }
        XCTAssertEqual(state.offers.map(\.name), ["lipsum"])
        XCTAssertFalse(state.offers[0].lookedUp, "the source was not asked")
        XCTAssertTrue(state.unavailable.isEmpty, "a request to the missing archive would have made it unavailable: \(state.unavailable)")
        XCTAssertFalse(FileManager.default.fileExists(atPath: cache.path))
        XCTAssertEqual(s.doneCount, 1)
    }

    // MARK: hermetic (hooks or a fake helper; no host)

    /// A model whose window is on the new engine (no host runs: FLASHTEX_HOST
    /// is "none"), with `main.tex` saved in a project folder and the
    /// manifest read through `manifest`.
    private func v3Model(_ manifest: @escaping () -> ProjectFilesV1.Manifest) throws -> (ShellModel, URL) {
        let bin = try dir("texbin")
        FileManager.default.createFile(atPath: bin.appendingPathComponent("kpsewhich").path, contents: Data())
        env.set("FLASHTEX_TEXLIVE_BIN", bin.path)
        env.set("FLASHTEX_HOST", "none")
        let root = try dir("hermetic")
        let main = root.appendingPathComponent("main.tex")
        try write("\\documentclass{article}\n\\begin{document}\nHi.\n\\end{document}\n", to: main)
        files.append(main)
        let m = ShellModel()
        m.detachWorker()
        m.files.policy = .disabled(reason: "test: hooks answer")
        m.manifest.reader = { _, _ in .success(manifest()) }
        EngineChoiceStore.appSetting = .new
        XCTAssertEqual(m.openTex(at: main, dirty: .discard), .opened)
        XCTAssertTrue(m.engineV3Enabled, m.engineChoice.explanation)
        return (m, root)
    }

    private static func reply(packages: [[String: Any]] = [], libraries: [[String: Any]]? = nil) -> ProjectFilesV1.ResolvePackages {
        var o: [String: Any] = ["cache": "/nonexistent", "policy": ["source": "ctan", "fetch": "ask"], "diagnostics": [], "packages": packages]
        if let libraries { o["libraries"] = libraries }
        return try! decode(o, as: ProjectFilesV1.ResolvePackages.self)
    }

    private static func cached(_ name: String, _ text: String, from: String = "cache") -> [String: Any] {
        ["name": name, "status": "cached", "version": "1", "from": from, "files": [file("packages/\(name)/\(name).sty", text)]]
    }

    private static func notCached(_ name: String) -> [String: Any] {
        ["name": name, "status": "not_available", "reason": "\(name) is not cached and the fetch policy is \"never\""]
    }

    private static func lib(_ text: String) -> [[String: Any]] {
        [["name": "mylib", "version": "v", "files": [file("packages/mylib/mylib.sty", text)]]]
    }

    private static func texMissing(_ file: String) -> RuntimeV1.Diagnostic {
        RuntimeV1.Diagnostic(severity: .error, message: "LaTeX Error: File `\(file)' not found.", source: nil, recovery: nil, code: "latex/file-not-found")
    }

    /// A failed offline resolution keeps what the last one delivered,
    /// forgets its key and asks again after the backoff (doubling), and the
    /// retry that succeeds replaces the deliveries.
    func testAFailedOfflineResolutionKeepsTheDeliveriesAndRetriesWithBackoff() async throws {
        var current = try Self.manifest(libraries: ["mylib": "../mylib"])
        let (m, _) = try v3Model { current }
        defer { m.engineV3.stop() }
        let state = m.projectPackages
        state.engineV3RetryBase = 0.2
        var answers: [ProjectFilesV1.ResolvePackages?] = [Self.reply(libraries: Self.lib("% v1")), nil, nil, Self.reply(packages: [Self.cached("fx", "% pinned fx")], libraries: Self.lib("% v2"))]
        var calls: [(Date, [String])] = []
        state.offlineResolver = { _, names, libraries in
            XCTAssertTrue(libraries)
            calls.append((Date(), names))
            let a = answers.isEmpty ? nil : answers.removeFirst()
            return a.map { .success($0) } ?? .failure(.init("the helper did not answer within 10 s"))
        }
        state.prepareForEngineV3()
        try await waitUntil("the first resolution", timeout: 5) { !state.engineV3Preparing && calls.count == 1 }
        XCTAssertEqual(state.engineV3Documents().map(\.text), ["% v1"])

        // The manifest pins fx: two failures, then success.
        current = try Self.manifest(pin: ["fx": "1"], libraries: ["mylib": "../mylib"])
        m.manifest.refresh()
        state.prepareForEngineV3() // what the session's next walk asks (no host here)
        try await waitUntil("the failure", timeout: 5) { state.engineV3Failures == 1 && !state.engineV3Preparing }
        XCTAssertEqual(state.engineV3Documents().map(\.text), ["% v1"], "a failure keeps what was delivered")
        XCTAssertTrue(state.status.contains("trying again"), state.status)
        XCTAssertFalse(state.prepareForEngineV3(), "within the backoff the same manifest is not asked again")
        XCTAssertEqual(calls.count, 2)
        try await waitUntil("the retries", timeout: 10) { calls.count == 4 && !state.engineV3Preparing }
        XCTAssertEqual(calls[1].1, ["fx"])
        let first = calls[2].0.timeIntervalSince(calls[1].0), second = calls[3].0.timeIntervalSince(calls[2].0)
        XCTAssertGreaterThanOrEqual(first, 0.2)
        XCTAssertGreaterThanOrEqual(second, 0.4, "the delay doubles")
        XCTAssertEqual(state.engineV3Failures, 0)
        XCTAssertEqual(Set(state.engineV3Documents().map(\.text)), ["% v2", "% pinned fx"], "the success replaces the deliveries")
    }

    /// A real timeout: the helper (a fake that reads the request and never
    /// answers) times out the offline request; what was delivered stays.
    func testATimedOutOfflineResolutionKeepsTheDeliveries() async throws {
        var current = try Self.manifest(libraries: ["mylib": "../mylib"])
        let (m, _) = try v3Model { current }
        defer { m.engineV3.stop() }
        let state = m.projectPackages
        state.engineV3RetryBase = 30 // no retry within the test
        state.offlineResolver = { _, _, _ in .success(Self.reply(libraries: Self.lib("% kept"))) }
        state.prepareForEngineV3()
        try await waitUntil("the first resolution", timeout: 5) { !state.engineV3Preparing && !state.engineV3Documents().isEmpty }

        state.offlineResolver = nil
        m.files.policy = .executable(DocumentFilesTests.python, arguments: [DocumentFilesTests.fakeHelper.path, "--mode", "hang"])
        m.files.packagesOfflineTimeout = 0.5
        current = try Self.manifest(pin: ["fx": "1"], libraries: ["mylib": "../mylib"])
        m.manifest.refresh()
        let started = Date()
        state.prepareForEngineV3()
        try await waitUntil("the timeout", timeout: 10) { state.engineV3Failures == 1 && !state.engineV3Preparing }
        XCTAssertGreaterThanOrEqual(Date().timeIntervalSince(started), 0.5)
        XCTAssertTrue(state.status.contains("trying again"), state.status)
        XCTAssertEqual(state.engineV3Documents().map(\.text), ["% kept"])
    }

    /// The helper answers one request at a time: an offline resolution
    /// asked for while a fetch is running waits for it (it is not sent to
    /// time out behind it) and holds no compile meanwhile.
    func testAnOfflineResolutionWaitsForARunningFetchAndHoldsNoCompile() async throws {
        let (m, _) = try v3Model { try! Self.manifest(pin: ["fx": "1"]) }
        defer { m.engineV3.stop() }
        let state = m.projectPackages
        var fetchEnded: Date?
        var offlineStarted: Date?
        state.resolver = { _, names, _ in
            try? await Task.sleep(nanoseconds: 400_000_000)
            fetchEnded = Date()
            return .success(Self.reply(packages: names.map { Self.cached($0, "% \($0)") }))
        }
        state.offlineResolver = { _, _, _ in
            offlineStarted = Date()
            return .success(Self.reply(packages: [Self.cached("fx", "% fx")]))
        }
        state.noteCompileResult(diagnostics: []) // binds the state to this project
        let fetch = Task { await state.resolve(["other"], consent: true) }
        try await waitUntil("the fetch is sent", timeout: 5) { state.helperRequests == 1 }
        XCTAssertFalse(state.prepareForEngineV3(), "queued behind a fetch: no compile is held")
        XCTAssertTrue(state.engineV3Preparing)
        XCTAssertFalse(state.engineV3HoldsCompile)
        _ = await fetch.value
        try await waitUntil("the offline request", timeout: 5) { !state.engineV3Preparing }
        let started = try XCTUnwrap(offlineStarted), ended = try XCTUnwrap(fetchEnded)
        XCTAssertGreaterThanOrEqual(started, ended, "sent only after the fetch was answered")
        XCTAssertEqual(state.engineV3Failures, 0)
        XCTAssertEqual(state.engineV3Documents().map(\.path), ["packages/fx/fx.sty"])
        // With the helper idle, a preparation holds the compile until it ends.
        state.offlineResolver = { _, _, _ in try? await Task.sleep(nanoseconds: 200_000_000); return .success(Self.reply(packages: [Self.cached("fx", "% fx 2")])) }
        m.manifest.reader = { _, _ in .success(try! Self.manifest(pin: ["fx": "2"])) }
        m.manifest.refresh()
        XCTAssertTrue(state.prepareForEngineV3())
        XCTAssertTrue(state.engineV3HoldsCompile)
        try await waitUntil("the held resolution", timeout: 5) { !state.engineV3Preparing }
        XCTAssertFalse(state.engineV3HoldsCompile)
    }

    /// Engine switch: an unpinned cached copy the previous engine asked for
    /// is not given to the new engine (TeX Live's wins) and is not dropped
    /// either; a package the new engine's TeX reports missing is. Switching
    /// back gives the previous engine everything again.
    func testAnEngineSwitchKeepsDeliveriesButTheNewEngineGetsOnlyItsOwn() async throws {
        let (m, root) = try v3Model { try! Self.manifest() }
        defer { m.engineV3.stop() }
        let state = m.projectPackages
        var online: [[String]] = []
        state.resolver = { _, names, _ in online.append(names); return .success(Self.reply(packages: names.map { Self.cached($0, "% \($0) from the cache") })) }
        state.offlineResolver = { _, names, _ in .success(Self.reply(packages: names.map { Self.cached($0, "% \($0) from the cache") })) }
        // The previous engine: its compiler asks for siunitx, the cache has it.
        m.engineV3Enabled = false
        XCTAssertFalse(m.engineV3Enabled)
        state.noteCompileResult(diagnostics: [RuntimeV1.Diagnostic(severity: .warning, message: "packages siunitx are recognised but not implemented", source: nil, recovery: nil, code: "unsupported_feature")])
        try await waitUntil("the old engine's delivery", timeout: 5) { !state.documents().isEmpty }
        XCTAssertEqual(state.documents().map(\.path), ["packages/siunitx/siunitx.sty"])

        // The new engine: siunitx stays delivered but is not given to it.
        m.engineV3Enabled = true
        XCTAssertEqual(state.documents().map(\.path), ["packages/siunitx/siunitx.sty"], "nothing is reset by the switch")
        XCTAssertEqual(state.engineV3Documents(), [], "an unpinned cached copy never shadows TeX Live")
        XCTAssertEqual(state.rows, [], "the sidebar shows what the new engine reads")
        let mirror = EngineV3Mirror(source: root, session: 9_000 + Int.random(in: 0 ..< 1000))
        defer { try? FileManager.default.removeItem(at: mirror.base) }
        XCTAssertEqual(mirror.materializePackages(state.engineV3Documents()), [])
        // Its TeX reports mynotes missing: the cache has it, so it is given.
        state.noteCompileResult(diagnostics: [Self.texMissing("mynotes.sty")])
        try await waitUntil("the new engine's delivery", timeout: 5) { !state.engineV3Documents().isEmpty }
        XCTAssertEqual(state.engineV3Documents().map(\.path), ["packages/mynotes/mynotes.sty"])
        XCTAssertEqual(online, [["siunitx"]], "the new engine asked offline only")

        // Back to the previous engine: both.
        m.engineV3Enabled = false
        XCTAssertEqual(state.documents().map(\.path), ["packages/mynotes/mynotes.sty", "packages/siunitx/siunitx.sty"])
    }

    /// Privacy (Commander ruling): under the new engine a missing package
    /// is asked about offline only; the sheet offers it by name; the source
    /// hears of it only on Look Up (described, nothing fetched) or Fetch.
    func testNothingIsSentToTheSourceBeforeConsent() async throws {
        let (m, _) = try v3Model { try! Self.manifest() }
        defer { m.engineV3.stop() }
        let state = m.projectPackages
        var online: [(names: [String], consent: Bool)] = []
        var offline: [[String]] = []
        state.offlineResolver = { _, names, _ in offline.append(names); return .success(Self.reply(packages: names.map(Self.notCached))) }
        state.resolver = { _, names, consent in
            online.append((names, consent))
            let p: [[String: Any]] = names.map { n in consent
                ? Self.cached(n, "% \(n)", from: "cache").merging(["status": "fetched", "source_url": "https://mirrors.ctan.org/macros/latex/contrib/\(n)/"]) { $1 }
                : ["name": n, "status": "needs_consent", "version": "2.0", "source_url": "https://mirrors.ctan.org/macros/latex/contrib/\(n)/", "would_fetch": ["\(n).sty"]] }
            return .success(Self.reply(packages: p))
        }
        state.noteCompileResult(diagnostics: [Self.texMissing("myprivateclass.cls"), Self.texMissing("mytypo.sty")])
        try await waitUntil("the sheet", timeout: 5) { state.shown }
        XCTAssertEqual(offline, [["myprivateclass", "mytypo"]])
        XCTAssertTrue(online.isEmpty, "nothing left the machine: \(online)")
        XCTAssertEqual(state.offers.map(\.name), ["myprivateclass", "mytypo"])
        XCTAssertTrue(state.offers.allSatisfy { !$0.lookedUp && $0.version == nil })
        XCTAssertEqual(state.offers[0].sourceLabel, "CTAN")
        // The same compile result again: not asked again.
        state.noteCompileResult(diagnostics: [Self.texMissing("mytypo.sty")])
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertTrue(online.isEmpty)

        // Look Up: described, nothing fetched.
        let looked = await state.lookUp()
        XCTAssertTrue(looked)
        XCTAssertEqual(online.map(\.consent), [false])
        XCTAssertEqual(state.offers.map(\.version), ["2.0", "2.0"])
        XCTAssertTrue(state.offers.allSatisfy(\.lookedUp))
        XCTAssertTrue(state.engineV3Documents().isEmpty)
        // Fetch: the consent.
        let fetched = await state.fetch()
        XCTAssertTrue(fetched, state.note ?? "")
        XCTAssertEqual(online.map(\.consent), [false, true])
        XCTAssertEqual(state.engineV3Documents().map(\.path), ["packages/myprivateclass/myprivateclass.sty", "packages/mytypo/mytypo.sty"])
    }
}
