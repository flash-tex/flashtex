import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Multi-file projects (lane `mac-multifile`): lexical include discovery,
/// rooted direct opens, helper-route opens with exact membership snapshots,
/// `ShellModel.documents` sync, explicit detach, and caret-preserving switches.
@MainActor
final class ProjectDocumentsTests: XCTestCase {
    // MARK: lexical discovery

    func testScanFindsInputAndIncludeWithByteSpans() {
        let text = "\\documentclass{article}\n\\begin{document}\n\\input{chapter}\n\\include{ch/two.tex}\n\\input bare\\relax\n\\end{document}\n"
        let refs = ProjectIncludes.scan(text)
        XCTAssertEqual(refs.map(\.argument), ["chapter", "ch/two.tex", "bare"])
        XCTAssertEqual(refs.map(\.kind), [.input, .include, .input])
        XCTAssertTrue(refs.allSatisfy(\.literal))
        // Spans are UTF-8 byte offsets of the whole command and of the argument.
        let bytes = Array(text.utf8)
        for r in refs {
            let command = String(decoding: bytes[r.startByte..<r.endByte], as: UTF8.self)
            let argument = String(decoding: bytes[r.argumentStartByte..<r.argumentEndByte], as: UTF8.self)
            XCTAssertTrue(command.hasPrefix("\\input") || command.hasPrefix("\\include"), command)
            XCTAssertEqual(argument, r.argument)
        }
        XCTAssertEqual(String(decoding: bytes[refs[0].startByte..<refs[0].endByte], as: UTF8.self), "\\input{chapter}")
        XCTAssertEqual(String(decoding: bytes[refs[2].startByte..<refs[2].endByte], as: UTF8.self), "\\input bare")
    }

    func testScanSpansAreBytesNotCharacters() {
        let text = "Ünïcödé — \\input{ch}\n"
        let refs = ProjectIncludes.scan(text)
        XCTAssertEqual(refs.count, 1)
        guard let ref = refs.first else { return XCTFail("expected one reference") }
        XCTAssertEqual(ref.startByte, Array("Ünïcödé — ".utf8).count)
        XCTAssertEqual(ref.argumentStartByte, ref.startByte + "\\input{".utf8.count)
    }

    func testScanSkipsCommentsVerbVerbatimAndCommandPrefixes() {
        let text = """
        % \\input{commented}
        \\verb|\\input{verb}| text \\inputfoo{x}
        \\begin{verbatim}
        \\input{verbatim}
        \\end{verbatim}
        \\input[opt]{real} \\include{\\jobname}
        \\include
        """
        let refs = ProjectIncludes.scan(text)
        XCTAssertEqual(refs.map(\.argument), ["real", "\\jobname"])
        XCTAssertEqual(refs.map(\.literal), [true, false])
    }

    func testScanIsBounded() {
        let text = String(repeating: "\\input{a}\n", count: 200)
        XCTAssertEqual(ProjectIncludes.scan(text).count, ProjectIncludes.maxReferences)
        XCTAssertEqual(ProjectIncludes.scan(text, limit: 3).count, 3)
        XCTAssertEqual(ProjectIncludes.scan("\\input{unbalanced").count, 0, "an unbalanced brace consumes to the end")
    }

    func testRootedPathNormalizationMirrorsProjectFiles() throws {
        XCTAssertEqual(try ProjectIncludes.normalize("./chapters//intro.tex"), "chapters/intro.tex")
        XCTAssertEqual(try ProjectIncludes.normalize("a/../b"), "b")
        XCTAssertThrowsError(try ProjectIncludes.normalize("../escape")) { XCTAssertEqual($0 as? ProjectIncludes.PathError, .escapesRoot) }
        XCTAssertThrowsError(try ProjectIncludes.normalize("/etc/passwd")) { XCTAssertEqual($0 as? ProjectIncludes.PathError, .absolute) }
        XCTAssertThrowsError(try ProjectIncludes.normalize("~/x")) { XCTAssertEqual($0 as? ProjectIncludes.PathError, .absolute) }
        XCTAssertThrowsError(try ProjectIncludes.normalize("a\\b")) { XCTAssertEqual($0 as? ProjectIncludes.PathError, .forbiddenCharacter("\\")) }
        XCTAssertThrowsError(try ProjectIncludes.normalize("c:x")) { XCTAssertEqual($0 as? ProjectIncludes.PathError, .forbiddenCharacter(":")) }
        XCTAssertThrowsError(try ProjectIncludes.normalize("./")) { XCTAssertEqual($0 as? ProjectIncludes.PathError, .empty) }
        XCTAssertEqual(try ProjectIncludes.candidates(for: "chapter"), ["chapter.tex", "chapter"])
        XCTAssertEqual(try ProjectIncludes.candidates(for: "ch/two.tex"), ["ch/two.tex"])
        XCTAssertEqual(try ProjectIncludes.candidates(for: "notes.md"), ["notes.md.tex", "notes.md"])
    }

    // MARK: direct mode (no helper)

    private struct TempProject {
        let root: URL
        let main: URL
        init(main mainText: String = "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n",
             chapter: String = "Chapter one, with caret memory.\n", extra: [String: String] = [:]) throws {
            root = FileManager.default.temporaryDirectory.appendingPathComponent("pd-test-\(UUID().uuidString)")
            try FileManager.default.createDirectory(at: root.appendingPathComponent("project/ch"), withIntermediateDirectories: true)
            main = root.appendingPathComponent("project/main.tex")
            try mainText.write(to: main, atomically: true, encoding: .utf8)
            try chapter.write(to: root.appendingPathComponent("project/chapter.tex"), atomically: true, encoding: .utf8)
            for (name, text) in extra { try text.write(to: root.appendingPathComponent("project/\(name)"), atomically: true, encoding: .utf8) }
        }
        func remove() { try? FileManager.default.removeItem(at: root) }
    }

    func testDirectModeOpensIncludesSwitchesAndDetaches() async throws {
        let project = try TempProject(extra: ["ch/two.tex": "Two.\n"])
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        XCTAssertFalse(model.controllerAttached)
        let p = model.project
        XCTAssertEqual(p.entryPath, "main.tex")
        XCTAssertEqual(p.projectRoot?.path, project.root.appendingPathComponent("project").standardizedFileURL.path)

        // Discovery resolves against the rooted project directory.
        var found = p.discoverIncludes()
        XCTAssertEqual(found.map(\.resolvedPath), ["chapter.tex"])
        XCTAssertEqual(found.map(\.state), [.available])
        guard let firstFound = found.first else { return XCTFail("expected one discovered include") }
        XCTAssertEqual(firstFound.candidates, ["chapter.tex", "chapter"])

        // Open: appended after the entry, baseline recorded, listing in sync.
        let outcomes = await p.openDiscoveredIncludes()
        XCTAssertEqual(outcomes, [.opened(path: "chapter.tex")])
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex"])
        guard model.documents.count == 2 else { return XCTFail("expected two documents, got \(model.documents.count)") }
        XCTAssertEqual(model.documents[1].text, "Chapter one, with caret memory.\n")
        XCTAssertEqual(p.listing.map(\.role), [.entry, .included(from: "main.tex")])
        XCTAssertEqual(p.listing.map(\.origin), [.disk, .disk])
        XCTAssertEqual(p.listing.map(\.durableRevision), [nil, nil])
        XCTAssertEqual(p.listing.map(\.isDirty), [false, false])
        found = p.discoverIncludes()
        XCTAssertEqual(found.map(\.state), [.open])
        let again = await p.openInclude("chapter")
        XCTAssertEqual(again, .alreadyOpen(path: "chapter.tex"), "the .tex candidate is the open member")
        let exact = await p.openDocument("chapter")
        XCTAssertEqual(exact, .refused("cannot open chapter: no such file under \(project.root.appendingPathComponent("project").standardizedFileURL.path)"), "openDocument is exact-path")
        let viaInclude = await p.openInclude("ch/two")
        XCTAssertEqual(viaInclude, .opened(path: "ch/two.tex"))
        let detachedAgain = await p.detachDocument("ch/two.tex")
        XCTAssertEqual(detachedAgain, .detached(path: "ch/two.tex"))
        let r1 = await p.openDocument("./ch/../ch/two.tex")
        XCTAssertEqual(r1, .opened(path: "ch/two.tex"))
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex", "ch/two.tex"], "open order is stable, entry first")

        // Switch: the outgoing caret is remembered, the incoming one restored.
        XCTAssertEqual(model.activePath, "main.tex")
        model.caretUTF16 = 7; model.caretLengthUTF16 = 4 // "Main." region
        let switched = p.switchDocument(to: "chapter.tex")
        XCTAssertEqual(switched, .switched(to: "chapter.tex", restoredCaret: NSRange(location: 0, length: 0)), "first visit starts at 0")
        XCTAssertEqual(model.activePath, "chapter.tex")
        XCTAssertEqual(model.activeText, "Chapter one, with caret memory.\n")
        XCTAssertEqual(model.selection?.path, "chapter.tex")
        XCTAssertEqual(p.carets["main.tex"], NSRange(location: 7, length: 4))
        model.caretUTF16 = 8; model.caretLengthUTF16 = 3 // "one"
        XCTAssertEqual(p.switchDocument(to: "main.tex"), .switched(to: "main.tex", restoredCaret: NSRange(location: 7, length: 4)))
        XCTAssertEqual(model.caretUTF16, 7)
        XCTAssertEqual(model.caretLengthUTF16, 4)
        XCTAssertEqual(model.selection, .init(path: "main.tex", nsRange: NSRange(location: 7, length: 4), token: model.selection!.token))
        XCTAssertEqual(p.switchDocument(to: "main.tex"), .unchanged)
        XCTAssertEqual(p.switchDocument(to: "nope.tex"), .refused("nope.tex is not open"))

        // Unsaved text survives a round trip and is never lost by a switch.
        p.switchDocument(to: "chapter.tex")
        XCTAssertEqual(model.caretUTF16, 8, "the chapter caret was remembered while main was active")
        model.updateActiveText("Chapter one, edited.\n")
        XCTAssertTrue(p.isDirty("chapter.tex"))
        XCTAssertEqual(p.listing.map(\.isDirty), [false, true, false], "the entry stays clean whichever document is active")
        p.switchDocument(to: "main.tex")
        XCTAssertEqual(model.documents[1].text, "Chapter one, edited.\n")
        XCTAssertTrue(p.anyDirty)
        // A remembered range that no longer fits is clamped.
        model.caretUTF16 = 100; model.caretLengthUTF16 = 5
        XCTAssertEqual(p.switchDocument(to: "chapter.tex"), .switched(to: "chapter.tex", restoredCaret: NSRange(location: 8, length: 3)))
        p.switchDocument(to: "ch/two.tex")
        XCTAssertEqual(p.carets["chapter.tex"], NSRange(location: 8, length: 3))
        model.caretUTF16 = 99; model.caretLengthUTF16 = 0
        _ = p.switchDocument(to: "main.tex")
        XCTAssertEqual(p.carets["ch/two.tex"], NSRange(location: 99, length: 0))
        XCTAssertEqual(p.switchDocument(to: "ch/two.tex"), .switched(to: "ch/two.tex", restoredCaret: NSRange(location: 5, length: 0)), "clamped to the 5-unit text")

        // A pending capture insertion blocks the switch (it carries no document check).
        model.pendingEdit = .init(path: "ch/two.tex", nsRange: NSRange(location: 0, length: 0), text: "x", token: 1)
        guard case .refused(let why) = p.switchDocument(to: "main.tex") else { return XCTFail("switch must be refused while an edit is pending") }
        XCTAssertTrue(why.contains("pending"), why)
        model.pendingEdit = nil

        // Detach: refused for the entry and for unsaved edits; explicit discard keeps the text.
        let r2 = await p.detachDocument("main.tex")
        XCTAssertEqual(r2, .refused("cannot detach the entry document main.tex"))
        guard case .refused(let dirtyWhy) = await p.detachDocument("chapter.tex") else { return XCTFail("dirty detach must be refused") }
        XCTAssertTrue(dirtyWhy.contains("unsaved"), dirtyWhy)
        XCTAssertEqual(model.documents.count, 3)
        let r3 = await p.detachDocument("chapter.tex", discardingEdits: true)
        XCTAssertEqual(r3, .detached(path: "chapter.tex"))
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "ch/two.tex"])
        XCTAssertEqual(p.detachedBuffers["chapter.tex"], "Chapter one, edited.\n")
        // Detaching the active document switches back to the entry first.
        XCTAssertEqual(model.activePath, "ch/two.tex")
        let r4 = await p.detachDocument("ch/two.tex")
        XCTAssertEqual(r4, .detached(path: "ch/two.tex"))
        XCTAssertEqual(model.activePath, "main.tex")
        XCTAssertEqual(model.documents.map(\.path), ["main.tex"])
        let r5 = await p.detachDocument("ch/two.tex")
        XCTAssertEqual(r5, .refused("ch/two.tex is not open"))
        // Re-opening the discarded document reads the (unchanged) disk text again.
        let r6 = await p.openDocument("chapter.tex")
        XCTAssertEqual(r6, .opened(path: "chapter.tex"))
        guard model.documents.count > 1 else { return XCTFail("expected more than one document after reopening, got \(model.documents.count)") }
        XCTAssertEqual(model.documents[1].text, "Chapter one, with caret memory.\n")
        XCTAssertNil(p.detachedBuffers["chapter.tex"])
    }

    func testDirectModeRefusesEscapesSymlinksAndMissingFiles() async throws {
        let project = try TempProject(main: "\\input{../outside}\n\\input{link}\n\\input{missing}\n\\input{/abs}\n\\input{\\macro}\n")
        defer { project.remove() }
        try "outside\n".write(to: project.root.appendingPathComponent("outside.tex"), atomically: true, encoding: .utf8)
        try FileManager.default.createSymbolicLink(at: project.root.appendingPathComponent("project/link.tex"),
                                                   withDestinationURL: project.root.appendingPathComponent("outside.tex"))
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let found = model.project.discoverIncludes()
        XCTAssertEqual(found.count, 5)
        for d in found {
            guard case .unresolvable = d.state else { return XCTFail("\(d.reference.argument) must be unresolvable: \(d.state)") }
        }
        guard found.count == 5 else { return XCTFail("expected five discovered includes, got \(found.count)") }
        XCTAssertEqual(found[0].state, .unresolvable("path escapes the project root via '..'"))
        XCTAssertEqual(found[1].state, .unresolvable("link.tex is a symbolic link"))
        XCTAssertEqual(found[2].state, .unresolvable("no such file under the project root"))
        XCTAssertEqual(found[3].state, .unresolvable("path must be project-relative, not absolute"))
        XCTAssertEqual(found[4].state, .unresolvable("argument needs macro expansion"))
        let r7 = await model.project.openDiscoveredIncludes()
        XCTAssertEqual(r7, [])
        XCTAssertEqual(model.documents.map(\.path), ["main.tex"])
        // Direct opens by path apply the same rules.
        let r8 = await model.project.openDocument("../outside.tex")
        XCTAssertEqual(r8, .refused("../outside.tex: path escapes the project root via '..'"))
        let r9 = await model.project.openDocument("link.tex")
        XCTAssertEqual(r9, .refused("cannot open link.tex: link.tex is a symbolic link"))
        XCTAssertEqual(model.documents.count, 1)
    }

    func testDirectModeNeedsAProjectRoot() async {
        let model = ShellModel()
        model.detachWorker()
        model.replaceProject(entryText: "\\input{chapter}\n")
        XCTAssertNil(model.documentURL)
        let found = model.project.discoverIncludes()
        XCTAssertEqual(found.map(\.state), [.unresolvable("no project root (the entry document is not saved)")])
        guard case .refused(let why) = await model.project.openDocument("chapter.tex") else { return XCTFail() }
        XCTAssertTrue(why.contains("no project root"), why)
    }

    // MARK: silent-failure fixes (GH: tab bar / Project menu save, go-to-definition
    // / palette / sidebar open) — `ShellModel.saveDocumentInteractive` and
    // `.openAndSwitch` map a discarded `SaveOutcome`/`OpenOutcome` to a footer note.

    /// The entry document is unsaved (no project root): `saveDocumentInteractive`
    /// (shared by the tab bar's and Project menu's "Save <path>" items) must not
    /// drop the refusal — it names the reason in `captureNote`.
    func testSaveDocumentInteractiveNamesTheReasonWhenTheDocumentIsNotOpen() async {
        let model = ShellModel()
        model.detachWorker()
        model.replaceProject(entryText: "\\input{chapter}\n")
        XCTAssertNil(model.captureNote)
        await model.saveDocumentInteractive("chapter.tex")
        XCTAssertEqual(model.captureNote, "Save of chapter.tex failed: chapter.tex is not open")
    }

    /// The success path the tab bar's and Project menu's "Save <path>" items
    /// now go through directly (previously they discarded the outcome entirely).
    func testSaveDocumentInteractiveSavesAnOpenNonEntryDocument() async throws {
        let project = try TempProject(main: "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n", chapter: "Chapter.\n")
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let opened = await model.project.openDiscoveredIncludes()
        XCTAssertEqual(opened, [.opened(path: "chapter.tex")])
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, saved via the tab bar's context menu.\n")
        await model.saveDocumentInteractive("chapter.tex")
        XCTAssertEqual(model.captureNote, "Saved chapter.tex")
        let chapterURL = project.root.appendingPathComponent("project/chapter.tex")
        XCTAssertEqual(try String(contentsOf: chapterURL, encoding: .utf8), "Chapter, saved via the tab bar's context menu.\n")
    }

    /// `openAndSwitch` (shared by go-to-definition file targets, the command
    /// palette's Files rows, the sidebar's closed-row click, and the Project
    /// menu's "Open <name>" item) reports a refusal through the caller's note
    /// instead of leaving the open silently failed.
    func testOpenAndSwitchReportsARefusalInsteadOfDroppingIt() async {
        let model = ShellModel()
        model.detachWorker()
        model.replaceProject(entryText: "\\input{chapter}\n")
        var note: String?
        let ok = await model.openAndSwitch("chapter.tex", role: .opened) { note = $0 }
        XCTAssertFalse(ok)
        XCTAssertEqual(note.map { $0.contains("no project root") }, true, note ?? "nil")
    }

    /// On success `openAndSwitch` switches the active document, the same as
    /// the other go-to-definition branches (`reveal`).
    func testOpenAndSwitchSwitchesToTheOpenedDocumentOnSuccess() async throws {
        let project = try TempProject(extra: ["ch/two.tex": "Two.\n"])
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        var note: String?
        let ok = await model.openAndSwitch("chapter.tex", role: .included(from: "main.tex")) { note = $0 }
        XCTAssertTrue(ok)
        XCTAssertNil(note)
        XCTAssertEqual(model.activePath, "chapter.tex")
    }

    /// `openAndSwitch` switches to the member path the open normalized to
    /// (`./chapter.tex` is the member `chapter.tex`), not the raw argument.
    func testOpenAndSwitchUsesTheCanonicalPath() async throws {
        let project = try TempProject(extra: ["ch/two.tex": "Two.\n"])
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        var note: String?
        let ok = await model.openAndSwitch("./chapter.tex", role: .opened) { note = $0 }
        XCTAssertTrue(ok)
        XCTAssertNil(model.navigationNote)
        XCTAssertNil(note)
        XCTAssertEqual(model.activePath, "chapter.tex")
    }

    func testNavigationSwitchRecordsTheOutgoingCaret() throws {
        // Navigation.swift switches `activePath` directly for a span in another
        // open document; the caret of the document being left is still recorded.
        let model = ShellModel()
        model.detachWorker()
        model.replaceProject(entryText: "Main text.\n")
        model.documents.append(.init(path: "chapter.tex", text: "\\label{x}\n"))
        let p = model.project
        model.caretUTF16 = 5; model.caretLengthUTF16 = 0
        model.setCompiledDocuments(["main.tex": "Main text.\n", "chapter.tex": "\\label{x}\n"])
        model.navigateExactly(to: .init(path: "chapter.tex", startByte: 7, endByte: 8), expectedText: nil)
        XCTAssertEqual(model.activePath, "chapter.tex", model.navigationNote ?? "")
        XCTAssertEqual(p.carets["main.tex"], NSRange(location: 5, length: 0))
        XCTAssertEqual(p.switchDocument(to: "main.tex"), .switched(to: "main.tex", restoredCaret: NSRange(location: 5, length: 0)))
    }

    // MARK: helper route (real flashtex-preview-controller + compiler)

    static var helper: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) }
    }

    func testHelperRouteOpensWithExactSnapshotAndDetaches() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let project = try TempProject(main: "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n",
                                      chapter: "Chapter via helper.\n",
                                      extra: ["appendix.tex": "Appendix, not referenced.\n"])
        defer { project.remove() }
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", project.root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        model.attachController(at: helper)
        XCTAssertTrue(model.controllerAttached)
        try await waitUntil { model.result?.revision == model.editorRevision && model.controllerState.durable["main.tex"] != nil }
        let p = model.project

        // The helper discovered chapter.tex at startup: the snapshot lists it,
        // and opening it reads the durable document (no membership change).
        let snapshotOrNil = await p.refreshSnapshot()
        let snapshot = try XCTUnwrap(snapshotOrNil)
        XCTAssertEqual(Set(snapshot.versions.keys), ["main.tex", "chapter.tex"])
        let g0 = snapshot.generation
        XCTAssertEqual(p.discoverIncludes().map(\.state), [.available])
        let r10 = await p.openDiscoveredIncludes()
        XCTAssertEqual(r10, [.opened(path: "chapter.tex")])
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex"])
        guard model.documents.count == 2 else { return XCTFail("expected two documents, got \(model.documents.count)") }
        XCTAssertEqual(model.documents[1].text, "Chapter via helper.\n")
        guard p.listing.count > 1 else { return XCTFail("expected more than one listing entry, got \(p.listing.count)") }
        XCTAssertEqual(p.listing[1].origin, .helper)
        XCTAssertEqual(p.listing[1].durableRevision, 1)
        XCTAssertEqual(model.controllerState.durable["chapter.tex"]?.revision, 1)
        XCTAssertEqual(p.membershipGeneration, g0)

        // An unreferenced rooted file goes through open_document with the exact
        // snapshot; the membership generation advances.
        let r11 = await p.openDocument("appendix.tex")
        XCTAssertEqual(r11, .opened(path: "appendix.tex"))
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex", "appendix.tex"])
        guard p.listing.count > 2 else { return XCTFail("expected more than two listing entries, got \(p.listing.count)") }
        XCTAssertEqual(p.listing[2].origin, .helper)
        XCTAssertEqual(p.listing[2].durableRevision, 1)
        let g1 = try XCTUnwrap(p.membershipGeneration)
        XCTAssertGreaterThan(g1, g0)
        XCTAssertEqual(Set(p.sourceVersions.keys), ["main.tex", "chapter.tex", "appendix.tex"])
        let statusOrNil = await p.projectStatus()
        let status = try XCTUnwrap(statusOrNil)
        XCTAssertEqual(status["total_documents"] as? Int, 3)
        XCTAssertEqual(status["scope"] as? String, "active_sources_only")

        // A stale snapshot is refused by the helper (nothing opened twice).
        let stale: [String: Any] = ["path": "appendix.tex", "source_versions": snapshot.versions, "membership_generation": g0]
        guard case .failure(let refusal) = await p.helperRequest("open_document", stale) else { return XCTFail("stale open must be refused") }
        XCTAssertTrue(refusal.message.contains("stale"), refusal.message)

        // Editing an opened document goes through the durable route and its
        // preview binds to the editor revision (not stale).
        XCTAssertEqual(p.switchDocument(to: "chapter.tex"), .switched(to: "chapter.tex", restoredCaret: NSRange(location: 0, length: 0)))
        model.updateActiveText("Chapter via helper, edited.\n")
        let rev = model.editorRevision
        try await waitUntil { model.result?.revision == rev && model.inFlightRevision == nil }
        XCTAssertFalse(model.previewIsStale)
        XCTAssertEqual(model.controllerState.durable["chapter.tex"]?.revision, 2)
        guard p.listing.count > 1 else { return XCTFail("expected more than one listing entry, got \(p.listing.count)") }
        XCTAssertEqual(p.listing[1].durableRevision, 2)
        XCTAssertEqual(model.compiledDocuments["chapter.tex"], "Chapter via helper, edited.\n")
        XCTAssertTrue(p.isDirty("chapter.tex"), "durable is not saved: the disk baseline still differs")

        // Detach the unreferenced document through the helper: generation advances,
        // the shell membership and durable state drop it, the entry is refused.
        p.switchDocument(to: "appendix.tex")
        let r12 = await p.detachDocument("appendix.tex")
        XCTAssertEqual(r12, .detached(path: "appendix.tex"))
        XCTAssertEqual(model.activePath, "main.tex")
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex"])
        XCTAssertNil(model.controllerState.durable["appendix.tex"])
        let g2 = try XCTUnwrap(p.membershipGeneration)
        XCTAssertGreaterThan(g2, g1)
        XCTAssertEqual(Set(p.sourceVersions.keys), ["main.tex", "chapter.tex"])
        let r13 = await p.detachDocument("main.tex")
        XCTAssertEqual(r13, .refused("cannot detach the entry document main.tex"))
        // The preview after the membership change still binds to the current buffers.
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertEqual(model.activeText, "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n")
        model.detachController()
    }

    func testHelperSyncAttachesDocumentsOpenedBeforeTheController() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let project = try TempProject(main: "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n",
                                      chapter: "Chapter on disk.\n",
                                      extra: ["notes.tex": "Notes on disk.\n"])
        defer { project.remove() }
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", project.root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let model = ShellModel()
        model.autoCompile = true
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let p = model.project
        // Direct mode first: a discovered include and an unreferenced file, one edited.
        let r1 = await p.openDiscoveredIncludes()
        XCTAssertEqual(r1, [.opened(path: "chapter.tex")])
        let r2 = await p.openDocument("notes.tex")
        XCTAssertEqual(r2, .opened(path: "notes.tex"))
        XCTAssertEqual(p.listing.map(\.origin), [.disk, .disk, .disk])
        p.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter edited before the helper.\n")
        p.switchDocument(to: "main.tex") // attachController names the entry after the active path (parent diff)

        // Attaching the helper: chapter.tex is read from the helper (it discovered
        // it), notes.tex is imported with open_document, and the edited buffer is
        // submitted so the helper's durable text is what the editor shows.
        model.attachController(at: helper)
        try await waitUntil { model.controllerState.durable["chapter.tex"] != nil && model.controllerState.durable["notes.tex"] != nil }
        try await waitUntil { model.controllerState.textByDurable["chapter.tex"]?[model.controllerState.durable["chapter.tex"]!.revision]?.sameBytes(as: "Chapter edited before the helper.\n") == true }
        XCTAssertGreaterThanOrEqual(p.helperSyncs, 1)
        XCTAssertEqual(model.controllerState.durable["chapter.tex"]?.revision, 2, "disk import r1, buffer submitted as r2")
        XCTAssertEqual(model.controllerState.durable["notes.tex"]?.revision, 1)
        XCTAssertEqual(p.listing.map(\.origin), [.disk, .helper, .helper])
        XCTAssertEqual(p.listing.map(\.durableRevision), [1, 2, 1])
        XCTAssertEqual(Set(p.sourceVersions.keys), ["main.tex", "chapter.tex", "notes.tex"])
        XCTAssertTrue(p.isDirty("chapter.tex"), "durable on the helper, still unsaved on disk")
        try await waitUntil { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        XCTAssertEqual(model.compiledDocuments["chapter.tex"], "Chapter edited before the helper.\n", "the preview was compiled from the edited chapter")
        // Nothing left to sync: a later status change sends no request.
        let syncs = p.helperSyncs
        await p.syncWithHelper()
        XCTAssertEqual(p.helperSyncs, syncs)
        model.detachController()
    }

    func testDirectModeSavesNonEntryDocumentsAndRefusesChangedDisk() async throws {
        let project = try TempProject()
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let p = model.project
        let opened = await p.openDiscoveredIncludes()
        XCTAssertEqual(opened, [.opened(path: "chapter.tex")])
        let chapterURL = project.root.appendingPathComponent("project/chapter.tex")
        // The entry document is not this API's business.
        let entry = await p.saveDocument("main.tex")
        guard case .failed(let why) = entry, why.contains("entry") else { return XCTFail("\(entry)") }
        // Clean save: the file holds exactly the buffer; the baseline follows.
        p.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter one, saved.\n")
        XCTAssertTrue(p.isDirty("chapter.tex"))
        let saved = await p.saveDocument("chapter.tex")
        XCTAssertEqual(saved, .saved(path: "chapter.tex", sha256: SourceDigest.sha256Hex("Chapter one, saved.\n")))
        XCTAssertEqual(try String(contentsOf: chapterURL, encoding: .utf8), "Chapter one, saved.\n")
        XCTAssertFalse(p.isDirty("chapter.tex"))
        XCTAssertEqual(try String(contentsOf: project.main, encoding: .utf8), "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n", "the entry file is untouched")
        XCTAssertFalse(p.isDirty("main.tex"))
        // An external change is a conflict: nothing overwritten, buffer kept dirty.
        try "external chapter\n".write(to: chapterURL, atomically: true, encoding: .utf8)
        model.updateActiveText("Chapter one, saved twice.\n")
        let refused = await p.saveDocument("chapter.tex")
        guard case .conflict(let c) = refused else { return XCTFail("expected a conflict, got \(refused)") }
        XCTAssertEqual(c.kind, .modifiedExternally)
        XCTAssertEqual(c.url, chapterURL)
        XCTAssertEqual(try String(contentsOf: chapterURL, encoding: .utf8), "external chapter\n")
        XCTAssertTrue(p.isDirty("chapter.tex"))
        XCTAssertEqual(p.saveConflict(for: "chapter.tex"), c)
        XCTAssertEqual(model.activeText, "Chapter one, saved twice.\n")
    }

    func testHelperRouteSavesThroughExportAndFlushesOnSwitch() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let project = try TempProject(chapter: "Chapter via helper.\n")
        defer { project.remove() }
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", project.root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        model.attachController(at: helper)
        try await waitUntil { model.result?.revision == model.editorRevision && model.controllerState.durable["main.tex"] != nil }
        let p = model.project
        let opened = await p.openDiscoveredIncludes()
        XCTAssertEqual(opened, [.opened(path: "chapter.tex")])
        let chapterURL = project.root.appendingPathComponent("project/chapter.tex")

        // Keystrokes in chapter.tex, then an immediate switch back to main: the
        // controller's one-slot queue now follows main.tex, but the chapter
        // buffer is flushed to the ledger explicitly.
        p.switchDocument(to: "chapter.tex")
        for i in 1...4 { model.updateActiveText("Chapter via helper \(String(repeating: "y", count: i)).\n") }
        let chapterText = model.activeText
        p.switchDocument(to: "main.tex")
        XCTAssertEqual(model.activePath, "main.tex")
        try await waitUntil { model.controllerState.textByDurable["chapter.tex"]?[model.controllerState.durable["chapter.tex"]?.revision ?? -1]?.sameBytes(as: chapterText) == true }
        XCTAssertEqual(model.documents[1].text, chapterText, "the buffer never lost a keystroke")
        try await waitUntil { model.compiledDocuments["chapter.tex"]?.sameBytes(as: chapterText) == true }
        XCTAssertEqual(model.result?.revision, model.editorRevision, "the preview compiled from the flushed chapter is current")
        XCTAssertNil(model.inFlightRevision)

        // Save the (non-active) chapter through export: disk gets the durable text.
        let saved = await p.saveDocument("chapter.tex")
        XCTAssertEqual(saved, .saved(path: "chapter.tex", sha256: SourceDigest.sha256Hex(chapterText)))
        XCTAssertEqual(try String(contentsOf: chapterURL, encoding: .utf8), chapterText)
        XCTAssertFalse(p.isDirty("chapter.tex"))
        XCTAssertEqual(try String(contentsOf: project.main, encoding: .utf8), "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n")
        // External change: export is refused with the disk conflict; nothing overwritten.
        try "external chapter\n".write(to: chapterURL, atomically: true, encoding: .utf8)
        p.switchDocument(to: "chapter.tex")
        model.updateActiveText(chapterText + "more\n")
        let refused = await p.saveDocument("chapter.tex")
        guard case .conflict(let c) = refused else { return XCTFail("expected a conflict, got \(refused)") }
        XCTAssertTrue(c.viaHelper)
        XCTAssertEqual(c.kind, .modifiedExternally)
        XCTAssertEqual(c.theirs, SourceDigest.sha256Hex("external chapter\n"))
        XCTAssertEqual(try String(contentsOf: chapterURL, encoding: .utf8), "external chapter\n")
        XCTAssertTrue(p.isDirty("chapter.tex"))
        model.detachController()
    }

    // MARK: transitive discovery

    func testClosureIsDepthFirstRefusesCyclesAndListsDiamondsOnce() async throws {
        let project = try TempProject(main: "\\input{chapter}\n\\input{appendix}\n",
                                      chapter: "\\input{ch/section}\n\\input{missing}\n",
                                      extra: ["ch/section.tex": "\\input{main}\n\\input{appendix}\n", "appendix.tex": "Appendix.\n"])
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let p = model.project
        let closure = p.discoverClosure()
        XCTAssertEqual(closure.paths, ["chapter.tex", "ch/section.tex", "appendix.tex"], "depth-first, source order, first-reached")
        XCTAssertEqual(closure.nodes.map(\.from), ["main.tex", "chapter.tex", "ch/section.tex", "ch/section.tex", "chapter.tex", "main.tex"])
        XCTAssertEqual(closure.nodes.map(\.depth), [0, 1, 2, 2, 1, 0])
        XCTAssertEqual(closure.nodes.map(\.resolvedPath), ["chapter.tex", "ch/section.tex", "main.tex", "appendix.tex", nil, "appendix.tex"])
        guard closure.nodes.count == 6 else { return XCTFail("expected six closure nodes, got \(closure.nodes.count)") }
        XCTAssertEqual(closure.nodes[2].state, .unresolvable("\\input{main} closes an include cycle: main.tex → chapter.tex → ch/section.tex → main.tex"))
        XCTAssertEqual(closure.nodes[4].state, .unresolvable("no such file under the project root"))
        XCTAssertTrue(closure.nodes[5].duplicate, "appendix reached again from main.tex is a diamond, not a cycle")
        XCTAssertEqual(closure.nodes[5].state, .available)
        XCTAssertFalse(closure.truncated)
        XCTAssertEqual(closure.unresolvable.count, 2)
        XCTAssertEqual(p.discoverIncludes().map(\.resolvedPath), ["chapter.tex", "appendix.tex"], "single-level discovery is unchanged")

        // Open All: the closure in stable order; unresolvable ones reported.
        let outcomes = await p.openDiscoveredIncludes()
        XCTAssertEqual(outcomes, [.opened(path: "chapter.tex"), .opened(path: "ch/section.tex"), .opened(path: "appendix.tex")])
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex", "ch/section.tex", "appendix.tex"])
        XCTAssertEqual(p.listing.map(\.role), [.entry, .included(from: "main.tex"), .included(from: "chapter.tex"), .included(from: "ch/section.tex")])
        let report = try XCTUnwrap(p.lastOpenReport)
        XCTAssertEqual(report.opened, ["chapter.tex", "ch/section.tex", "appendix.tex"])
        XCTAssertEqual(report.unresolvable, [
            "\\input{main} in ch/section.tex: \\input{main} closes an include cycle: main.tex → chapter.tex → ch/section.tex → main.tex",
            "\\input{missing} in chapter.tex: no such file under the project root",
        ])
        XCTAssertTrue(p.status.hasPrefix("open all includes: opened 3"), p.status)
        // A second Open All opens nothing new and keeps reporting.
        let again = await p.openDiscoveredIncludes()
        XCTAssertEqual(again, [])
        XCTAssertEqual(p.lastOpenReport?.opened, [])
        XCTAssertEqual(p.lastOpenReport?.unresolvable.count, 2)
        XCTAssertEqual(p.discoverClosure().nodes.map(\.state).filter { $0 == .open }.count, 4, "three members plus the diamond")
    }

    func testClosureDepthIsBounded() throws {
        var extra: [String: String] = [:]
        for i in 1...10 { extra["d\(i).tex"] = i < 10 ? "\\input{d\(i + 1)}\n" : "leaf\n" }
        let project = try TempProject(main: "\\input{d1}\n", extra: extra)
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let closure = model.project.discoverClosure()
        XCTAssertEqual(closure.paths, (1...8).map { "d\($0).tex" })
        XCTAssertEqual(closure.nodes.count, 9)
        XCTAssertEqual(closure.nodes.last?.from, "d8.tex")
        XCTAssertEqual(closure.nodes.last?.state, .unresolvable("nested deeper than 8 levels; not discovered"))
        XCTAssertEqual(model.project.discoverClosure(maxDepth: 2).paths, ["d1.tex", "d2.tex"])
    }

    func testClosureRefusesSelfInclude() throws {
        let project = try TempProject(main: "\\input{main}\n\\input{chapter}\n", chapter: "\\input{chapter.tex}\n")
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let closure = model.project.discoverClosure()
        XCTAssertEqual(closure.paths, ["chapter.tex"])
        XCTAssertEqual(closure.nodes[0].state, .unresolvable("\\input{main} closes an include cycle: main.tex → main.tex"))
        XCTAssertEqual(closure.nodes[2].state, .unresolvable("\\input{chapter.tex} closes an include cycle: main.tex → chapter.tex → chapter.tex"))
    }

    // MARK: ⌘S routing (parent-applied saveTexInteractive)

    func testSaveCommandWritesOnlyTheActiveNonEntryDocumentDirectly() async throws {
        let project = try TempProject(main: "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n", chapter: "Chapter.\n")
        defer { project.remove() }
        let mainBytes = try Data(contentsOf: project.main)
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let opened = await model.project.openDiscoveredIncludes()
        XCTAssertEqual(opened, [.opened(path: "chapter.tex")])
        model.project.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, saved with the Save command.\n")
        XCTAssertTrue(model.isDirty, "parent isDirty follows the active document")
        model.saveTexInteractive() // ⌘S
        let chapterURL = project.root.appendingPathComponent("project/chapter.tex")
        try await waitUntil { (try? String(contentsOf: chapterURL, encoding: .utf8)) == "Chapter, saved with the Save command.\n" }
        XCTAssertEqual(try Data(contentsOf: project.main), mainBytes, "main.tex bytes unchanged")
        try await waitUntil { model.captureNote == "Saved chapter.tex" }
        XCTAssertFalse(model.isDirty)
        XCTAssertFalse(model.project.isDirty("chapter.tex"))
        XCTAssertEqual(model.savedText, "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n", "the entry baseline is untouched")
    }

    func testSaveCommandWritesOnlyTheActiveNonEntryDocumentThroughTheHelper() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let project = try TempProject(main: "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n",
                                      chapter: "\\input{ch/section}\nChapter.\n", extra: ["ch/section.tex": "Section.\n"])
        defer { project.remove() }
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", project.root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let mainBytes = try Data(contentsOf: project.main)
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        model.attachController(at: helper)
        try await waitUntil { model.result?.revision == model.editorRevision && model.controllerState.durable["main.tex"] != nil }
        let p = model.project
        // The helper discovered the whole closure at startup; Open All reads each ledger document.
        let opened = await p.openDiscoveredIncludes()
        XCTAssertEqual(opened, [.opened(path: "chapter.tex"), .opened(path: "ch/section.tex")])
        XCTAssertEqual(p.listing.map(\.origin), [.disk, .helper, .helper])
        XCTAssertEqual(p.lastOpenReport?.unresolvable, [])
        p.switchDocument(to: "chapter.tex")
        model.updateActiveText("\\input{ch/section}\nChapter, saved through the helper.\n")
        model.saveTexInteractive() // ⌘S → project.saveDocument → export
        let chapterURL = project.root.appendingPathComponent("project/chapter.tex")
        try await waitUntil { (try? String(contentsOf: chapterURL, encoding: .utf8)) == "\\input{ch/section}\nChapter, saved through the helper.\n" }
        XCTAssertEqual(try Data(contentsOf: project.main), mainBytes, "main.tex bytes unchanged")
        try await waitUntil { model.captureNote == "Saved chapter.tex" }
        XCTAssertFalse(model.project.isDirty("chapter.tex"))
        XCTAssertEqual(try String(contentsOf: project.root.appendingPathComponent("project/ch/section.tex"), encoding: .utf8), "Section.\n")
        let disk = await model.controllerFileStatus(path: "chapter.tex")
        XCTAssertEqual(disk?.state, "matches_source")
        model.detachController()
    }

    private func waitUntil(timeout: TimeInterval = 15, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout") }
            try await Task.sleep(nanoseconds: 30_000_000)
        }
    }

    // MARK: implicit include-closure compile (lane mac-includes-auto)

    /// Every resolved, unopened include in the closure goes out with the
    /// compile request (`ProjectDocuments.implicitClosureDocuments`), even a
    /// nested one two levels deep — without opening any of them.
    func testImplicitClosureSendsUnopenedIncludesWithoutOpeningThem() throws {
        let project = try TempProject(main: "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n",
                                      chapter: "Chapter.\n\\input{ch/two}\n",
                                      extra: ["ch/two.tex": "Section two.\n"])
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let implicit = model.project.implicitClosureDocuments()
        XCTAssertEqual(Set(implicit.map(\.path)), ["chapter.tex", "ch/two.tex"], "the whole transitive closure, not just the entry's direct includes")
        XCTAssertEqual(implicit.first(where: { $0.path == "chapter.tex" })?.text, "Chapter.\n\\input{ch/two}\n")
        XCTAssertEqual(implicit.first(where: { $0.path == "ch/two.tex" })?.text, "Section two.\n")
        // Discovery-only: nothing was opened, the project membership is unchanged.
        XCTAssertEqual(model.documents.map(\.path), ["main.tex"])
        XCTAssertEqual(model.project.listing.count, 1)
    }

    /// An on-disk edit to a still-unopened include is what the next compile
    /// request carries (read fresh, never cached); opening the file makes the
    /// buffer authoritative and removes it from the implicit set entirely —
    /// an unsaved buffer edit is therefore never leaked into a "disk" read —
    /// and detaching it (discarding the buffer) reverts it to implicit.
    func testImplicitClosureTextUpdatesOnDiskChangeAndOpenedBufferOverridesDisk() async throws {
        let project = try TempProject(main: "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n",
                                      chapter: "Chapter, first version.\n")
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let p = model.project
        XCTAssertEqual(p.implicitClosureDocuments().map(\.text), ["Chapter, first version.\n"])

        let chapterURL = project.root.appendingPathComponent("project/chapter.tex")
        try "Chapter, edited on disk.\n".write(to: chapterURL, atomically: true, encoding: .utf8)
        XCTAssertEqual(p.implicitClosureDocuments().map(\.text), ["Chapter, edited on disk.\n"])

        let opened = await p.openDocument("chapter.tex")
        XCTAssertEqual(opened, .opened(path: "chapter.tex"))
        XCTAssertTrue(p.implicitClosureDocuments().isEmpty, "an open member is never also reported as implicit")
        p.switchDocument(to: "chapter.tex")
        model.updateActiveText("Chapter, edited in the buffer (not saved).\n")
        XCTAssertEqual(model.documents.last?.text, "Chapter, edited in the buffer (not saved).\n")
        XCTAssertEqual(try String(contentsOf: chapterURL, encoding: .utf8), "Chapter, edited on disk.\n", "the buffer edit was never written")
        XCTAssertTrue(p.implicitClosureDocuments().isEmpty)

        let detached = await p.detachDocument("chapter.tex", discardingEdits: true)
        XCTAssertEqual(detached, .detached(path: "chapter.tex"))
        XCTAssertEqual(p.implicitClosureDocuments().map(\.text), ["Chapter, edited on disk.\n"], "reverted to implicit; reads disk again, not the discarded buffer")
    }

    /// A missing include is left out of the implicit set — never fabricated —
    /// and discovery's existing "referenced but missing" diagnostic path is
    /// unchanged.
    func testImplicitClosureLeavesOutAMissingIncludeWithoutFabricatingContent() throws {
        let project = try TempProject(main: "\\begin{document}\nMain.\n\\input{chapter}\n\\input{missing}\n\\end{document}\n")
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let p = model.project
        let found = p.discoverIncludes()
        XCTAssertEqual(found.map(\.reference.argument), ["chapter", "missing"])
        guard found.count > 1 else { return XCTFail("expected more than one discovered include, got \(found.count)") }
        XCTAssertEqual(found[1].state, .unresolvable("no such file under the project root"), "unchanged diagnostic path")
        XCTAssertEqual(p.implicitClosureDocuments().map(\.path), ["chapter.tex"])
    }

    /// A symlinked include is refused (never read, never sent) exactly like
    /// direct-mode discovery already refuses it.
    func testImplicitClosureRefusesASymlinkedInclude() throws {
        let project = try TempProject(main: "\\begin{document}\nMain.\n\\input{chapter}\n\\input{link}\n\\end{document}\n")
        defer { project.remove() }
        try "outside\n".write(to: project.root.appendingPathComponent("outside.tex"), atomically: true, encoding: .utf8)
        try FileManager.default.createSymbolicLink(at: project.root.appendingPathComponent("project/link.tex"),
                                                   withDestinationURL: project.root.appendingPathComponent("outside.tex"))
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        let p = model.project
        let found = p.discoverIncludes()
        XCTAssertEqual(found[1].state, .unresolvable("link.tex is a symbolic link"))
        XCTAssertEqual(p.implicitClosureDocuments().map(\.path), ["chapter.tex"], "the symlinked include is refused, never read")
    }

    // MARK: live producer (FLASHTEX_RENDER)

    /// The include closure reaches the engine even when only the entry is
    /// open: compiling `fixtures/real-world/input-bibliography/main.tex`
    /// (its `sections/*.tex` are real `\input` files) with only `main.tex`
    /// open must produce the same page count as compiling it with every file
    /// opened through `openDiscoveredIncludes()`.
    func testLiveProducerClosurePageCountMatchesAllFilesOpen() async throws {
        guard let render = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"], FileManager.default.isExecutableFile(atPath: render) else {
            throw XCTSkip("FLASHTEX_RENDER not set to a built flashtex-render")
        }
        guard let repoRoot = ShellModel.locateRepoRoot() else { throw XCTSkip("repo root not found") }
        let fixture = repoRoot.appendingPathComponent("fixtures/real-world/input-bibliography/main.tex")
        guard FileManager.default.fileExists(atPath: fixture.path) else { throw XCTSkip("fixture not found at \(fixture.path)") }

        let onlyEntry = ShellModel()
        onlyEntry.autoCompile = false
        XCTAssertEqual(onlyEntry.openTex(at: fixture), .opened)
        XCTAssertEqual(onlyEntry.documents.map(\.path), ["main.tex"], "only the entry is open")
        XCTAssertEqual(Set(onlyEntry.project.implicitClosureDocuments().map(\.path)), ["sections/intro.tex", "sections/method.tex"])
        onlyEntry.attachWorker(at: URL(fileURLWithPath: render))
        onlyEntry.compile()
        try await waitUntil(timeout: 30) { onlyEntry.inFlightRevision == nil }
        onlyEntry.detachWorker()
        let onlyEntryResult = try XCTUnwrap(onlyEntry.result)
        XCTAssertNotEqual(onlyEntryResult.status, .failed, "\(onlyEntryResult.diagnostics)")

        let allOpen = ShellModel()
        allOpen.autoCompile = false
        XCTAssertEqual(allOpen.openTex(at: fixture), .opened)
        let opened = await allOpen.project.openDiscoveredIncludes()
        XCTAssertEqual(opened.count, 2)
        XCTAssertEqual(Set(allOpen.documents.map(\.path)), ["main.tex", "sections/intro.tex", "sections/method.tex"])
        allOpen.attachWorker(at: URL(fileURLWithPath: render))
        allOpen.compile()
        try await waitUntil(timeout: 30) { allOpen.inFlightRevision == nil }
        allOpen.detachWorker()
        let allOpenResult = try XCTUnwrap(allOpen.result)
        XCTAssertNotEqual(allOpenResult.status, .failed, "\(allOpenResult.diagnostics)")

        XCTAssertEqual(onlyEntryResult.pages.count, allOpenResult.pages.count,
                       "only-main-open must compile the same page count as every file open (implicit include closure)")
        XCTAssertGreaterThan(onlyEntryResult.pages.count, 0)
    }
}
