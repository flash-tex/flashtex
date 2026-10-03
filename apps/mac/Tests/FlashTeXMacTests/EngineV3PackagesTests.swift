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
        m.projectPackages.localResolver = { _, names in
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
    /// Fetch (the Problems row and the consent sheet), Fetch downloads it
    /// from the manifest's source (a `file://` archive here) through the
    /// project-files helper into the package cache, and the new engine
    /// recompiles with it.
    func testFetchIsOfferedAndFetchesIntoTheCacheUnderTheNewEngine() async throws {
        try EngineV3TestHost.require()
        guard ProjectFilesClient.locate() != nil else {
            throw XCTSkip("no flashtex-project-files built (cargo build -p flashtex-project-files)")
        }
        let base = try dir("fetch")
        let archive = base.appendingPathComponent("archive/macros/latex/contrib/fxpkgtest")
        try write("<a href=\"fxpkgtest.sty\">fxpkgtest.sty</a>", to: archive.appendingPathComponent("index.html"))
        try write("\\ProvidesPackage{fxpkgtest}\n\\newcommand{\\fxmarker}{Fetched from the archive.}\n", to: archive.appendingPathComponent("fxpkgtest.sty"))
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
        XCTAssertTrue(state.offers[0].sourceURL.hasPrefix("file://"), state.offers[0].sourceURL)
        XCTAssertFalse(FileManager.default.fileExists(atPath: cache.path), "nothing is fetched before the user says yes")

        let ok = await state.fetch()
        XCTAssertTrue(ok, state.note ?? "")
        XCTAssertTrue(FileManager.default.fileExists(atPath: cache.appendingPathComponent("fxpkgtest").path), "fetched into the package cache")
        try await waitUntil("the compile with the fetched package") { s.statusNote.hasPrefix("ok") && !s.compiling && s.firstError == nil }
        XCTAssertEqual(errors(m), [])
        XCTAssertEqual(s.pageCount, 1)
        let copy = try XCTUnwrap(s.projectCopy)
        XCTAssertTrue(try String(contentsOf: copy.appendingPathComponent("fxpkgtest.sty"), encoding: .utf8).contains("\\fxmarker"))
    }
}
