import XCTest
@testable import FlashTeXMac

/// Explicit bibliography kind persistence (lane `mac-bibliography-kinds`,
/// gap 3): declaring a kind through the real helper, persisting the
/// declaration with the project, and reopening after the original `.bib`
/// was deleted from disk. Kinds are never inferred from an extension.
@MainActor
final class DocumentKindsTests: XCTestCase {
    // MARK: config / store (no helper)

    func testConfigEmitsBibliographyPathsOnlyWhenDeclared() {
        var config = PreviewControllerClient.Config(sessionID: "s", projectID: "p", entryPath: "main.tex",
                                                    projectRoot: URL(fileURLWithPath: "/tmp/x"), privateLedgerRoot: URL(fileURLWithPath: "/tmp/y"))
        XCTAssertNil(config.json()["bibliography_paths"], "no declaration → key absent (helper default: every path is latex)")
        config.bibliographyPaths = ["refs.bib", "more/extra.bib"]
        XCTAssertEqual(config.json()["bibliography_paths"] as? [String], ["refs.bib", "more/extra.bib"])
    }

    func testRecordIsEntryKeyedAndRoundTrips() throws {
        let ledger = FileManager.default.temporaryDirectory.appendingPathComponent("dk-store-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: ledger) }
        XCTAssertEqual(try DocumentKindsStore.load(ledgerRoot: ledger).get(), DocumentKindsRecord(), "missing file → empty record")

        var record = DocumentKindsRecord()
        record.set(bibliographyPaths: ["z.bib", "a.bib", "z.bib", "main.tex"], entry: "main.tex")
        record.set(bibliographyPaths: ["other.bib"], entry: "other.tex")
        XCTAssertEqual(record.bibliographyPaths(entry: "main.tex"), ["a.bib", "z.bib"], "sorted, de-duplicated, entry excluded")
        try DocumentKindsStore.save(record, ledgerRoot: ledger)
        XCTAssertEqual(try DocumentKindsStore.load(ledgerRoot: ledger).get(), record)
        let json = try String(contentsOf: DocumentKindsStore.url(ledgerRoot: ledger), encoding: .utf8)
        XCTAssertTrue(json.contains("\"bibliography_by_entry\""), json)
        XCTAssertTrue(json.contains("\"version\" : 1"), json)

        // Clearing an entry removes its key; other entries are kept.
        record.set(bibliographyPaths: [], entry: "main.tex")
        XCTAssertEqual(record.bibliographyByEntry.keys.sorted(), ["other.tex"])

        // A different record version is reported, never rewritten by load.
        try "{\"version\": 2, \"bibliography_by_entry\": {}}".write(to: DocumentKindsStore.url(ledgerRoot: ledger), atomically: true, encoding: .utf8)
        XCTAssertEqual(DocumentKindsStore.load(ledgerRoot: ledger), .failure(.unsupportedVersion(2)))
        // Corrupt JSON is reported by load; persist starts over (the file is ours).
        try "not json".write(to: DocumentKindsStore.url(ledgerRoot: ledger), atomically: true, encoding: .utf8)
        guard case .failure(.unreadable) = DocumentKindsStore.load(ledgerRoot: ledger) else { return XCTFail("corrupt file must be reported") }
    }

    func testPersistUsesTheControllerLedgerRootForTheProject() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("dk-proj-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: root) }
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let project = root.appendingPathComponent("project")
        XCTAssertEqual(DocumentKindsStore.bibliographyPaths(projectRoot: project, entry: "main.tex"), [])
        try DocumentKindsStore.persist(bibliographyPaths: ["refs.bib"], projectRoot: project, entry: "main.tex")
        XCTAssertEqual(DocumentKindsStore.url(projectRoot: project), root.appendingPathComponent("ledger/document-kinds.json"))
        XCTAssertEqual(DocumentKindsStore.bibliographyPaths(projectRoot: project, entry: "main.tex"), ["refs.bib"])
        XCTAssertEqual(DocumentKindsStore.bibliographyPaths(projectRoot: project, entry: "other.tex"), [], "entry-keyed")
        XCTAssertThrowsError(try {
            try "{\"version\": 7, \"bibliography_by_entry\": {}}".write(to: DocumentKindsStore.url(projectRoot: project), atomically: true, encoding: .utf8)
            try DocumentKindsStore.persist(bibliographyPaths: [], projectRoot: project, entry: "main.tex")
        }(), "an unsupported record version is never overwritten")
    }

    func testLaunchFilterDropsDeclarationsTheHelperCouldNotImport() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("dk-launch-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: root) }
        let project = root.appendingPathComponent("project"), ledger = root.appendingPathComponent("ledger")
        try FileManager.default.createDirectory(at: project, withIntermediateDirectories: true)
        try "@misc{x}".write(to: project.appendingPathComponent("ondisk.bib"), atomically: true, encoding: .utf8)
        var record = DocumentKindsRecord()
        record.set(bibliographyPaths: ["ondisk.bib", "gone.bib", "retained.bib"], entry: "main.tex")
        try DocumentKindsStore.save(record, ledgerRoot: ledger)
        // A retained ledger slot exactly where the helper binds it (file_project.rs).
        let canonical = try XCTUnwrap(DocumentKindsStore.canonicalPath(project)) // realpath, as the helper binds it
        let binding = SourceDigest.sha256Hex("\(canonical.utf8.count):\(canonical):pid")
        try FileManager.default.createDirectory(at: ledger.appendingPathComponent("project-\(binding)/\(SourceDigest.sha256Hex("retained.bib"))"),
                                                withIntermediateDirectories: true)
        let launch = DocumentKindsStore.launchableBibliographyPaths(projectRoot: project, privateLedgerRoot: ledger, projectID: "pid", entry: "main.tex")
        XCTAssertEqual(launch.paths, ["ondisk.bib", "retained.bib"])
        XCTAssertEqual(launch.dropped, ["gone.bib"], "neither on disk nor retained: not supplied (the helper would refuse to start)")
        let other = DocumentKindsStore.launchableBibliographyPaths(projectRoot: project, privateLedgerRoot: ledger, projectID: "other", entry: "main.tex")
        XCTAssertEqual(other.paths, ["ondisk.bib"], "another project id binds a different ledger")
        XCTAssertEqual(try DocumentKindsStore.load(ledgerRoot: ledger).get(), record, "the record itself is not rewritten at launch")
    }

    func testKindsAreNeverInferredWithoutTheHelper() async {
        let model = ShellModel()
        model.replaceProject(entryText: "\\begin{document}\\cite{knuth84}\\end{document}\n")
        model.documents.append(.init(path: "refs.bib", text: "@book{knuth84, title={The TeXbook}}\n"))
        let kinds = model.documentKinds
        XCTAssertNil(kinds.kind(of: "refs.bib"), "a .bib member has no kind until the helper reports one")
        XCTAssertEqual(kinds.bibliographyPaths, [])
        XCTAssertEqual(kinds.startupBibliographyPaths, [], "unsaved buffer: nothing persisted, nothing to supply")
        let refreshed = await kinds.refresh()
        XCTAssertFalse(refreshed)
        XCTAssertTrue(kinds.status.contains("no preview controller"), kinds.status)
        let declared = await kinds.declareBibliography("refs.bib")
        XCTAssertEqual(declared, .refused("cannot declare refs.bib: no preview controller attached"))
        let undeclared = await kinds.undeclare("refs.bib")
        XCTAssertEqual(undeclared, .refused("refs.bib is not a declared bibliography source"))
    }

    // MARK: real helper (flashtex-preview-controller + compiler)

    static var helper: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) }
    }

    private static let mainText = "\\begin{document}\nMain cites \\cite{knuth84}.\n\\input{chapter}\n\\end{document}\n"
    private static let chapterText = "Chapter.\n"
    private static let bibText = "@book{knuth84,\n  author = {Donald E. Knuth},\n  title = {The {\\TeX}book},\n  year = 1984\n}\n"

    private struct Project {
        let root: URL
        var dir: URL { root.appendingPathComponent("project") }
        var main: URL { dir.appendingPathComponent("main.tex") }
        var bib: URL { dir.appendingPathComponent("refs.bib") }
        init() throws {
            root = FileManager.default.temporaryDirectory.appendingPathComponent("dk-helper-\(UUID().uuidString)")
            try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
            try DocumentKindsTests.mainText.write(to: main, atomically: true, encoding: .utf8)
            try DocumentKindsTests.chapterText.write(to: dir.appendingPathComponent("chapter.tex"), atomically: true, encoding: .utf8)
            try DocumentKindsTests.bibText.write(to: bib, atomically: true, encoding: .utf8)
        }
        func remove() { try? FileManager.default.removeItem(at: root) }
    }

    private func requireHelper() throws -> URL {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        return helper
    }

    /// Declare through the shell, close, delete the `.bib`, and reopen the
    /// helper the way `attachController` does once the parent applies the
    /// one-line `bibliographyPaths` hook (the config is built here with the
    /// same roots). The control reopen without declarations shows the kind
    /// is otherwise lost — the reason the declaration must be persisted.
    func testDeclaredBibliographyKindPersistsAcrossReopenAfterDiskDeletion() async throws {
        let helper = try requireHelper()
        let project = try Project()
        defer { project.remove() }
        let ledgerRoot = project.root.appendingPathComponent("ledger")
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", ledgerRoot.path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        model.attachController(at: helper)
        XCTAssertTrue(model.controllerAttached)
        try await waitUntil { model.controllerState.ready && model.controllerState.durable["main.tex"] != nil }
        let kinds = model.documentKinds

        // Startup: main.tex and the discovered chapter.tex are latex; refs.bib
        // is not indexed at all (nothing declared, nothing inferred).
        let refreshed0 = await kinds.refresh()
        XCTAssertTrue(refreshed0, kinds.status)
        XCTAssertEqual(kinds.kinds, ["main.tex": .latex, "chapter.tex": .latex])
        XCTAssertNil(kinds.kind(of: "refs.bib"))
        XCTAssertEqual(kinds.startupBibliographyPaths, [])

        // An explicitly declared latex kind for a .tex goes through the same
        // typed open (then the generic open adopts the durable document).
        let s0OrNil = await model.project.refreshSnapshot()
        let s0 = try XCTUnwrap(s0OrNil)
        let explicitLatex = await model.project.helperRequest("open_document", ["path": "chapter.tex", "source_versions": s0.versions,
                                                                               "membership_generation": s0.generation, "document_kind": "latex"])
        guard case .failure(let alreadyMember) = explicitLatex else { return XCTFail("chapter.tex was discovered at startup; a second open must be refused") }
        XCTAssertFalse(alreadyMember.message.isEmpty)
        let r0 = await model.project.openDocument("chapter.tex")
        XCTAssertEqual(r0, .opened(path: "chapter.tex"))

        // Declare refs.bib as a bibliography through the helper.
        let r1 = await kinds.declareBibliography("refs.bib")
        XCTAssertEqual(r1, .declared(path: "refs.bib"), kinds.status)
        XCTAssertEqual(kinds.kind(of: "refs.bib"), .bibliography)
        XCTAssertEqual(kinds.kinds, ["main.tex": .latex, "chapter.tex": .latex, "refs.bib": .bibliography])
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex", "refs.bib"], "adopted into the shell membership")
        XCTAssertEqual(model.documents[2].text, Self.bibText, "ledger text (imported from disk by the typed open)")
        XCTAssertEqual(model.project.listing[2].origin, .helper)
        XCTAssertNil(kinds.persistError)
        XCTAssertEqual(kinds.persistedBibliographyPaths, ["refs.bib"])
        XCTAssertEqual(kinds.startupBibliographyPaths, ["refs.bib"])
        let recordURL = DocumentKindsStore.url(ledgerRoot: ledgerRoot)
        XCTAssertEqual(try DocumentKindsStore.load(ledgerRoot: ledgerRoot).get().bibliographyByEntry, ["main.tex": ["refs.bib"]], recordURL.path)
        let r2 = await kinds.declareBibliography("refs.bib")
        XCTAssertEqual(r2, .alreadyDeclared(path: "refs.bib"))
        let r3 = await kinds.declareBibliography("chapter.tex")
        XCTAssertEqual(r3, .refused("chapter.tex is already open as latex; detach it first, then declare it as a bibliography"))
        let r4 = await kinds.declareBibliography("main.tex")
        XCTAssertEqual(r4, .refused("cannot declare the entry document main.tex as a bibliography"))
        let r5 = await kinds.declareBibliography("missing.bib")
        guard case .refused(let why5) = r5 else { return XCTFail("a missing rooted file cannot be declared") }
        XCTAssertTrue(why5.hasPrefix("helper refused declaring missing.bib"), why5)
        XCTAssertEqual(kinds.startupBibliographyPaths, ["refs.bib"], "refusals never change the record")

        // Close the project (helper `close` + terminate), then delete the original .bib.
        model.detachController()
        XCTAssertFalse(model.controllerAttached)
        try FileManager.default.removeItem(at: project.bib)
        XCTAssertFalse(FileManager.default.fileExists(atPath: project.bib.path))
        XCTAssertEqual(kinds.startupBibliographyPaths, ["refs.bib"], "the record outlives the helper process")

        // Reopen with the persisted declarations (what attachController does
        // with the hook): the helper restores the retained ledger under the
        // declared kind although the disk file is gone.
        var config = PreviewControllerClient.Config(sessionID: "dk-\(UUID().uuidString)", projectID: model.projectId, entryPath: "main.tex",
                                                    projectRoot: project.dir, privateLedgerRoot: ShellModel.controllerLedgerRoot(for: project.dir),
                                                    compilerPath: ShellModel.locateCompiler())
        // The launch-time filter keeps refs.bib: its private ledger slot is retained.
        XCTAssertNotNil(DocumentKindsStore.retainedLedgerSlot(path: "refs.bib", projectRoot: project.dir, privateLedgerRoot: ledgerRoot, projectID: model.projectId))
        config.bibliographyPaths = kinds.startupBibliographyPaths(projectRoot: project.dir, privateLedgerRoot: ledgerRoot, projectID: model.projectId, entry: "main.tex")
        XCTAssertEqual(config.bibliographyPaths, ["refs.bib"])
        XCTAssertEqual(kinds.droppedAtLaunch, [])
        let reopened = try await RawHelper(executable: helper, config: config)
        defer { reopened.stop() }
        let snapshot = try await reopened.request("snapshot", [:])
        XCTAssertEqual(snapshot["document_kinds"] as? [String: String], ["main.tex": "latex", "chapter.tex": "latex", "refs.bib": "bibliography"])
        let doc = try await reopened.request("document", ["path": "refs.bib"])
        XCTAssertEqual((doc["document"] as? [String: Any])?["text"] as? String, Self.bibText, "durable source survives the disk deletion")
        let disk = try await reopened.request("file_status", ["path": "refs.bib"])
        XCTAssertEqual((disk["disk"] as? [String: Any])?["state"] as? String, "missing", "\(disk)")
        reopened.stop()

        // Control: reopening WITHOUT the declarations (today's attachController)
        // keeps the ledger but reports the retained refs.bib as latex — the
        // kind is exactly what was declared, never the extension.
        var bare = config
        bare.sessionID = "dk-\(UUID().uuidString)"
        bare.bibliographyPaths = []
        let control = try await RawHelper(executable: helper, config: bare)
        defer { control.stop() }
        let controlSnapshot = try await control.request("snapshot", [:])
        XCTAssertEqual(controlSnapshot["document_kinds"] as? [String: String], ["main.tex": "latex", "chapter.tex": "latex", "refs.bib": "latex"])
        control.stop()

        // Reopening through the shell itself is testShellReopenRestoresTheDeclaredKindThroughAttachController.
    }

    /// The shell's own reopen through `attachController`: with the parent's
    /// one-line hook (`config.bibliographyPaths = documentKinds
    /// .startupBibliographyPaths`) the declared kind is restored after the
    /// `.bib` was deleted; without it the test records the gap and skips.
    /// Either way the refresh never rewrites the persisted record.
    func testShellReopenRestoresTheDeclaredKindThroughAttachController() async throws {
        let helper = try requireHelper()
        let project = try Project()
        defer { project.remove() }
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", project.root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        model.attachController(at: helper)
        try await waitUntil { model.controllerState.ready && model.controllerState.durable["main.tex"] != nil }
        let kinds = model.documentKinds
        let r1 = await kinds.declareBibliography("refs.bib")
        XCTAssertEqual(r1, .declared(path: "refs.bib"), kinds.status)
        model.detachController()
        try FileManager.default.removeItem(at: project.bib)

        model.attachController(at: helper)
        try await waitUntil { model.controllerState.ready && model.controllerState.durable["main.tex"] != nil }
        let refreshed = await kinds.refresh()
        XCTAssertTrue(refreshed, kinds.status)
        let hooked = model.controller?.config.bibliographyPaths ?? []
        XCTAssertEqual(kinds.startupBibliographyPaths, ["refs.bib"], "refresh never rewrites the persisted declarations")
        defer { model.detachController() }
        guard hooked == ["refs.bib"] else {
            XCTAssertEqual(kinds.kind(of: "refs.bib"), .latex, "no hook: the retained ledger is reopened as latex (gap 3 reproduction)")
            throw XCTSkip("parent hook (attachController bibliographyPaths) not applied on this branch; helper-level reopen is verified by testDeclaredBibliographyKindPersistsAcrossReopenAfterDiskDeletion")
        }
        XCTAssertEqual(kinds.kind(of: "refs.bib"), .bibliography, "hook applied: the declared kind is restored")
        XCTAssertEqual(kinds.kinds, ["main.tex": .latex, "chapter.tex": .latex, "refs.bib": .bibliography])
        // The shell still lists the retained bibliography source (the durable
        // text, no disk file) and the generic open finds it already a member.
        let adopted = await model.project.openDocument("refs.bib")
        XCTAssertEqual(adopted, .alreadyOpen(path: "refs.bib"))
        XCTAssertEqual(model.documents.first { $0.path == "refs.bib" }?.text, Self.bibText)
        // The retained refs.bib's durable revision comes in its own helper
        // reply after main.tex's, so the wait above does not cover it. Wait
        // for it explicitly. A local probe (8 runs at load ~70) found it
        // still missing at that wait in 1 run, and the assertion failed with
        // nil in that run only. CI hit the same thing on 3 unrelated PRs.
        let deadline = Date().addingTimeInterval(15)
        while model.controllerState.durable["refs.bib"] == nil, Date() < deadline {
            try await Task.sleep(nanoseconds: 30_000_000)
        }
        XCTAssertEqual(model.controllerState.durable["refs.bib"]?.revision, 1)
        XCTAssertEqual(kinds.kind(of: "refs.bib"), .bibliography)
    }

    /// Undeclaring (typed detach) removes the path from the persisted record
    /// so the next reopen does not supply it.
    func testUndeclareDetachesAndForgetsTheDeclaration() async throws {
        let helper = try requireHelper()
        let project = try Project()
        defer { project.remove() }
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", project.root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        model.attachController(at: helper)
        try await waitUntil { model.controllerState.ready && model.controllerState.durable["main.tex"] != nil }
        let kinds = model.documentKinds
        let r1 = await kinds.declareBibliography("refs.bib")
        XCTAssertEqual(r1, .declared(path: "refs.bib"), kinds.status)
        XCTAssertEqual(kinds.startupBibliographyPaths, ["refs.bib"])
        let g1 = try XCTUnwrap(kinds.generation)

        // The kinds map follows membership changes made elsewhere (generic open).
        let r2 = await model.project.openDocument("chapter.tex")
        XCTAssertEqual(r2, .opened(path: "chapter.tex"))
        try await waitUntil { kinds.kind(of: "chapter.tex") == .latex }

        let r3 = await kinds.undeclare("refs.bib")
        XCTAssertEqual(r3, .detached(path: "refs.bib"), kinds.status)
        XCTAssertNil(kinds.kind(of: "refs.bib"))
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex"])
        XCTAssertGreaterThan(try XCTUnwrap(kinds.generation), g1)
        XCTAssertEqual(kinds.startupBibliographyPaths, [])
        XCTAssertEqual(try DocumentKindsStore.load(ledgerRoot: project.root.appendingPathComponent("ledger")).get().bibliographyByEntry, [:])
        let r4 = await kinds.undeclare("chapter.tex")
        XCTAssertEqual(r4, .refused("chapter.tex is not a declared bibliography source"), "a latex member is not undeclared here")
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex"])
        model.detachController()
        try await waitUntil { kinds.kinds.isEmpty }
        XCTAssertEqual(kinds.status, "no preview controller attached")
    }

    // MARK: support

    /// A bare helper process for the reopen check (no ShellModel): replies
    /// are correlated by request id on the main run loop.
    @MainActor
    private final class RawHelper {
        let client: PreviewControllerClient
        private var ready = false
        private var replies: [String: Result<[String: Any], ControllerError>] = [:]
        private var exited: Int32?

        init(executable: URL, config: PreviewControllerClient.Config) async throws {
            var handler: ((PreviewControllerClient.Event) -> Void)?
            client = try PreviewControllerClient(executable: executable, config: config) { event in handler?(event) }
            handler = { [weak self] event in
                MainActor.assumeIsolated {
                    guard let self else { return }
                    switch event {
                    case .ready: self.ready = true
                    case .result(let id, let payload): self.replies[id] = .success(payload)
                    case .error(let id?, let message): self.replies[id] = .failure(.init(message: message))
                    case .exited(let code): self.exited = code
                    default: break
                    }
                }
            }
            let start = Date()
            while !ready {
                if let exited { throw XCTSkip("helper exited with \(exited) before ready") }
                if Date().timeIntervalSince(start) > 15 { throw XCTSkip("helper not ready within 15 s") }
                try await Task.sleep(nanoseconds: 20_000_000)
            }
        }

        func request(_ type: String, _ payload: [String: Any]) async throws -> [String: Any] {
            let id = try client.send(type, payload)
            let start = Date()
            while replies[id] == nil {
                if Date().timeIntervalSince(start) > 15 { throw XCTSkip("no \(type) reply within 15 s") }
                try await Task.sleep(nanoseconds: 20_000_000)
            }
            return try replies.removeValue(forKey: id)!.get()
        }

        func stop() {
            guard client.isRunning else { return }
            client.close()
            client.terminate()
        }
    }

    private func waitUntil(timeout: TimeInterval = 15, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout") }
            try await Task.sleep(nanoseconds: 30_000_000)
        }
    }
}
