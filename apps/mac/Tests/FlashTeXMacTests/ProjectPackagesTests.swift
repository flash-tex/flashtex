import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Package resolution in the shell (ProjectPackages.swift,
/// docs/user/project-manifest.md `[packages]`): the compiler's not-found
/// diagnostics name the packages, the helper's `resolve_packages` answers,
/// one consent sheet per project offers what would be fetched, Fetch
/// delivers the files to the compile request and the sidebar, Not now is
/// remembered for the session, Never writes the policy, and `always` never
/// shows the sheet. Hermetic through the `resolver`/`rewriter` hooks; no
/// network, no helper.
@MainActor
final class ProjectPackagesTests: XCTestCase {
    private var tmp: URL!

    override func setUp() {
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("project-packages-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
    }

    override func tearDown() { try? FileManager.default.removeItem(at: tmp) }

    private func write(_ rel: String, _ text: String) throws -> URL {
        let url = tmp.appendingPathComponent(rel)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    private func manifestPayload(root: URL, fetch: String, exists: Bool = true) -> ProjectFilesV1.Manifest {
        let json = """
        {"path":\(exists ? "\"\(root.path)/flashtex.toml\"" : "null"),"exists":\(exists),"manifest_dir":\(exists ? "\"\(root.path)\"" : "null"),
         "manifest":{"project":{"entry":"main.tex","texinputs":[],"output":null},"fonts":{"text":null,"math":null,"mono":null,"sans":null},
                     "packages":{"source":"ctan","fetch":"\(fetch)","pin":{},"path":{}},"library":null},
         "warnings":[],"texinputs":[],"files":[],"diagnostics":[],"template":"[project]\\nentry = \\"main.tex\\"\\n"}
        """
        return try! JSONDecoder().decode(ProjectFilesV1.Manifest.self, from: Data(json.utf8))
    }

    /// A `resolve_packages` payload as the helper writes it.
    private func resolvePayload(_ packages: [String]) -> ProjectFilesV1.ResolvePackages {
        let json = """
        {"cache":"/tmp/cache","policy":{"source":"ctan","fetch":"ask"},"diagnostics":[],"packages":[\(packages.joined(separator: ","))]}
        """
        return try! JSONDecoder().decode(ProjectFilesV1.ResolvePackages.self, from: Data(json.utf8))
    }

    private func needsConsent(_ name: String, version: String = "2.2") -> String {
        "{\"name\":\"\(name)\",\"status\":\"needs_consent\",\"version\":\"\(version)\",\"source_url\":\"https://mirrors.ctan.org/macros/latex/contrib/\(name)/\",\"would_fetch\":[\"\(name).sty\"]}"
    }

    private func fetched(_ name: String, from: String? = nil) -> String {
        let status = from == nil ? "fetched" : "cached"
        let origin = from.map { ",\"from\":\"\($0)\"" } ?? ",\"source_url\":\"https://mirrors.ctan.org/macros/latex/contrib/\(name)/\""
        return "{\"name\":\"\(name)\",\"status\":\"\(status)\",\"version\":\"2.2\"\(origin),\"files\":[{\"path\":\"packages/\(name)/\(name).sty\",\"text\":\"\\\\ProvidesPackage{\(name)}\\n\",\"sha256\":\"ab\",\"bytes\":24}]}"
    }

    private func result(unresolved: [String], notes: [String] = []) -> RuntimeV1.CompileResult {
        var diagnostics: [String] = []
        if !unresolved.isEmpty {
            diagnostics.append("{\"severity\":\"warning\",\"message\":\"packages \(unresolved.joined(separator: ", ")) are recognised but not implemented\",\"code\":\"unsupported_feature\"}")
        }
        for n in notes { diagnostics.append("{\"severity\":\"error\",\"message\":\"\\\\foo is not supported by this compiler version\",\"code\":\"unknown_command\",\"notes\":[\"\(n)\"]}") }
        let json = "{\"project_id\":\"p\",\"revision\":1,\"status\":\"ok\",\"pages\":[],\"diagnostics\":[\(diagnostics.joined(separator: ","))]}"
        return try! JSONDecoder().decode(RuntimeV1.CompileResult.self, from: Data(json.utf8))
    }

    private func project(fetch: String, manifestExists: Bool = true) throws -> ShellModel {
        _ = try write("main.tex", "\\documentclass{article}\n\\usepackage{cancel}\n\\begin{document}\nHello.\n\\end{document}\n")
        if manifestExists { _ = try write("flashtex.toml", "[packages]\nfetch = \"\(fetch)\"\n") }
        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .disabled(reason: "test: hermetic (direct writes)")
        model.manifest.reader = { root, _ in .success(self.manifestPayload(root: root, fetch: fetch, exists: manifestExists)) }
        XCTAssertEqual(model.openTex(at: tmp.appendingPathComponent("main.tex")), .opened)
        return model
    }

    private func settle(_ until: @escaping () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(5)
        while !until(), Date() < deadline { try await Task.sleep(nanoseconds: 20_000_000) }
        XCTAssertTrue(until(), "did not settle in 5 s")
    }

    // MARK: the names (pure)

    func testUnresolvedNamesComeFromTheCompilersDiagnostics() {
        let r = result(unresolved: ["siunitx", "cancel", "../evil", "siunitx"], notes: ["no project file found: looked for mystyle.sty, mystyle.cls", "no project file found: looked for thesis.cls"])
        XCTAssertEqual(ProjectPackagesState.unresolvedNames(in: r.diagnostics), ["siunitx", "cancel", "mystyle", "thesis"])
        XCTAssertEqual(ProjectPackagesState.unresolvedNames(in: result(unresolved: []).diagnostics), [])
        XCTAssertTrue(ProjectPackagesState.isPackageName("l3kernel-2e"))
        XCTAssertFalse(ProjectPackagesState.isPackageName("a b") || ProjectPackagesState.isPackageName(".x") || ProjectPackagesState.isPackageName(""))
    }

    // MARK: ask → one sheet → Fetch

    func testAskShowsOneSheetAndFetchDeliversToTheCompileRequestAndTheSidebar() async throws {
        let model = try project(fetch: "ask")
        let state = model.projectPackages
        var calls: [(names: [String], consent: Bool)] = []
        state.resolver = { _, names, consent in
            calls.append((names, consent))
            return .success(self.resolvePayload(names.map { consent ? self.fetched($0) : self.needsConsent($0) }))
        }
        // The compile reports two missing packages: one resolve, no fetch, the sheet.
        model.result = result(unresolved: ["cancel", "siunitx"])
        try await settle { state.shown }
        XCTAssertEqual(calls.count, 1)
        XCTAssertEqual(calls[0].names, ["cancel", "siunitx"])
        XCTAssertFalse(calls[0].consent, "ask never fetches by itself")
        XCTAssertEqual(state.offers.map(\.name), ["cancel", "siunitx"])
        XCTAssertEqual(state.offers[0].summary, "cancel 2.2 — cancel.sty")
        XCTAssertEqual(state.offers[0].sourceLabel, "CTAN")
        XCTAssertTrue(state.documents().isEmpty)
        XCTAssertEqual(state.recompiles, 0)
        // The same result again (a recompile with nothing new) asks nothing more.
        model.result = result(unresolved: ["cancel", "siunitx"])
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(calls.count, 1, "offers already pending are not re-resolved")

        let revision = state.revision
        let ok = await state.fetch()
        XCTAssertTrue(ok, state.note ?? "")
        XCTAssertFalse(state.shown)
        XCTAssertEqual(calls.count, 2)
        XCTAssertTrue(calls[1].consent, "Fetch is the consent")
        XCTAssertEqual(calls[1].names, ["cancel", "siunitx"])
        XCTAssertEqual(state.documents().map(\.path), ["packages/cancel/cancel.sty", "packages/siunitx/siunitx.sty"])
        XCTAssertEqual(state.documents()[0].text, "\\ProvidesPackage{cancel}\n")
        XCTAssertGreaterThan(state.revision, revision, "a delivery bumps the revision the compile guard compares")
        XCTAssertEqual(state.recompiles, 1)
        // The compile request path carries them after the project's own files.
        XCTAssertEqual(model.project.implicitClosureDocuments().map(\.path), ["packages/cancel/cancel.sty", "packages/siunitx/siunitx.sty"])
        // The sidebar: one Packages caption, then the files, not openable.
        let rows = state.rows
        XCTAssertEqual(rows.map(\.path), ["packages/cancel/cancel.sty", "packages/siunitx/siunitx.sty"])
        XCTAssertEqual(rows[0].kind, "package")
        XCTAssertTrue(rows[0].source?.contains("CTAN") == true, rows[0].source ?? "")
        let tree = ProjectSection.rows(listing: model.project.listing, kinds: model.documentKinds, closure: model.project.discoverClosure(), packages: model.manifest.rows + rows, activePath: model.activePath)
        let ids: [String] = tree.map { $0.id }
        XCTAssertTrue(ids.contains("packages:group") && ids.contains("package:packages/cancel/cancel.sty"), "\(ids)")
        XCTAssertFalse(tree.first { $0.id == "packages:group" }!.selectable)
        XCTAssertEqual(tree.first { $0.id == "package:packages/cancel/cancel.sty" }!.indent, 1)
        // A later compile still naming them (the engine without S1) resolves nothing again.
        model.result = result(unresolved: ["cancel", "siunitx"])
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(calls.count, 2)
    }

    // MARK: Not now / Never / always / unavailable

    func testNotNowIsRememberedForTheSessionAndNeverWritesThePolicy() async throws {
        let model = try project(fetch: "ask")
        let state = model.projectPackages
        var resolves = 0
        state.resolver = { _, names, _ in resolves += 1; return .success(self.resolvePayload(names.map { self.needsConsent($0) })) }
        var written: [(fetch: String?, pin: [String: String]?)] = []
        state.rewriter = { root, _, fetch, pin in
            written.append((fetch, pin))
            let text = "[packages]\nfetch = \"\(fetch ?? "ask")\"\n"
            return .success(ProjectFilesV1.SetPackages(path: root.appendingPathComponent("flashtex.toml").path, exists: true, changed: true, text: text))
        }
        model.result = result(unresolved: ["cancel"])
        try await settle { state.shown }
        XCTAssertEqual(resolves, 1)
        state.notNow()
        XCTAssertFalse(state.shown)
        XCTAssertTrue(state.declined.contains("cancel"))
        model.result = result(unresolved: ["cancel"])
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(resolves, 1, "declined this session: not asked again")
        XCTAssertTrue(written.isEmpty)
        // The menu command asks again.
        state.present()
        try await settle { state.shown }
        XCTAssertEqual(resolves, 2)
        // Never: the policy is written through the rewrite and the rooted save, nothing fetched.
        let ok = await state.never()
        XCTAssertTrue(ok, state.note ?? "")
        XCTAssertFalse(state.shown)
        XCTAssertEqual(written.count, 1)
        XCTAssertEqual(written[0].fetch, "never")
        XCTAssertNil(written[0].pin)
        XCTAssertEqual(try String(contentsOf: tmp.appendingPathComponent("flashtex.toml"), encoding: .utf8), "[packages]\nfetch = \"never\"\n")
        XCTAssertTrue(state.documents().isEmpty)
    }

    func testAlwaysFetchesWithoutASheetAndRememberWritesAlways() async throws {
        let model = try project(fetch: "always")
        let state = model.projectPackages
        var consents: [Bool] = []
        state.resolver = { _, names, consent in consents.append(consent); return .success(self.resolvePayload(names.map { self.fetched($0) })) }
        model.result = result(unresolved: ["cancel"])
        try await settle { !state.documents().isEmpty }
        XCTAssertEqual(consents, [true], "the manifest is the remembered consent")
        XCTAssertFalse(state.shown)
        XCTAssertEqual(state.recompiles, 1)

        // Ask + Remember: Fetch writes `always`.
        let asked = try project(fetch: "ask")
        let s2 = asked.projectPackages
        s2.resolver = { _, names, consent in .success(self.resolvePayload(names.map { consent ? self.fetched($0) : self.needsConsent($0) })) }
        var written: [String?] = []
        s2.rewriter = { root, _, fetch, _ in
            written.append(fetch)
            return .success(ProjectFilesV1.SetPackages(path: root.appendingPathComponent("flashtex.toml").path, exists: true, changed: true, text: "[packages]\nfetch = \"\(fetch ?? "")\"\n"))
        }
        asked.result = result(unresolved: ["cancel"])
        try await settle { s2.shown }
        s2.remember = true
        let fetchedOK = await s2.fetch()
        XCTAssertTrue(fetchedOK, s2.note ?? "")
        XCTAssertEqual(written, ["always"])
        XCTAssertEqual(try String(contentsOf: tmp.appendingPathComponent("flashtex.toml"), encoding: .utf8), "[packages]\nfetch = \"always\"\n")
    }

    func testUnavailableAndCachedAnswersNeedNoSheet() async throws {
        let model = try project(fetch: "ask")
        let state = model.projectPackages
        var resolves = 0
        state.resolver = { _, _, _ in
            resolves += 1
            return .success(self.resolvePayload([
                "{\"name\":\"lipsum\",\"status\":\"not_available\",\"reason\":\"needs docstrip: lipsum ships only lipsum.dtx, lipsum.ins\"}",
                self.fetched("mylib", from: "library"),
            ]))
        }
        model.result = result(unresolved: ["lipsum", "mylib"])
        try await settle { !state.documents().isEmpty }
        XCTAssertFalse(state.shown, "nothing to consent to")
        XCTAssertEqual(state.unavailable["lipsum"]?.hasPrefix("needs docstrip"), true)
        XCTAssertEqual(state.documents().map(\.path), ["packages/mylib/mylib.sty"])
        XCTAssertTrue(state.rows[0].source?.contains("local library") == true, state.rows[0].source ?? "")
        XCTAssertTrue(state.rows[0].tooltip.contains("not a project file"))
        model.result = result(unresolved: ["lipsum", "mylib"])
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(resolves, 1, "unavailable and delivered names are not re-resolved")
        // A failure from the helper is a note, never a sheet.
        state.resolver = { _, _, _ in .failure(.init("no helper")) }
        model.result = result(unresolved: ["other"])
        try await settle { state.note != nil }
        XCTAssertEqual(state.note, "no helper")
        XCTAssertFalse(state.shown)
    }

    func testNoProjectRootIsANoteAndTheHelperReplyDecodes() throws {
        let bare = ShellModel()
        bare.detachWorker()
        bare.projectPackages.present()
        XCTAssertFalse(bare.projectPackages.shown)
        XCTAssertTrue(bare.navigationNote?.contains("Save the entry document first") == true, bare.navigationNote ?? "")
        let r = resolvePayload([needsConsent("cancel"), fetched("x"), fetched("y", from: "cache")])
        XCTAssertEqual(r.cache, "/tmp/cache")
        XCTAssertEqual(r.policy.fetch, "ask")
        XCTAssertEqual(r.packages.map(\.status), [.needsConsent, .fetched, .cached])
        XCTAssertEqual(r.packages[0].wouldFetch, ["cancel.sty"])
        XCTAssertEqual(r.packages[1].files?.first?.path, "packages/x/x.sty")
        XCTAssertEqual(r.packages[2].from, "cache")
    }
}
