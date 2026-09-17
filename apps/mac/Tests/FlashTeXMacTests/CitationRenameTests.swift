import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Pure `CitationRename` helpers: request shapes, plan parsing with exact
/// decimal offsets, key rule, refusal explanations, previews and labels.
final class CitationRenamePureTests: XCTestCase {
    static func reply(old: String = "knuth84", new: String = "knuth1984", edits: [[String: Any]]? = nil,
                      versions: [String: Int] = ["chapter.tex": 1, "main.tex": 1, "refs.bib": 1], generation: Int = 3) -> [String: Any] {
        let defaultEdits: [[String: Any]] = [
            ["file": "chapter.tex", "revision": "1", "start_byte": "20", "end_byte": "27", "expected_text": old, "replacement": new],
            ["file": "main.tex", "revision": "1", "start_byte": "60", "end_byte": "67", "expected_text": old, "replacement": new],
            ["file": "refs.bib", "revision": "1", "start_byte": "9", "end_byte": "16", "expected_text": old, "replacement": new],
        ]
        return [
            "source_versions": versions,
            "membership_generation": generation,
            "plan": [
                "schema": CitationRename.planSchema, "kind": CitationRename.planKind,
                "proposal_only": true, "requires_user_approval": true, "application_order": "reverse_byte_offset_per_document",
                "snapshot": ["project_id": "p", "generation": "\(generation)",
                             "documents": versions.keys.sorted().map { ["file": $0, "revision": "\(versions[$0]!)"] as [String: Any] }] as [String: Any],
                "rename": ["old_name": old, "new_name": new],
                "replacement": new,
                "edits": edits ?? defaultEdits,
            ] as [String: Any],
        ]
    }

    func testRequestShapesTypedAndAtVariants() {
        let typed = CitationRename.request(sourceVersions: ["main.tex": 2], membershipGeneration: 4, newName: "b", oldName: "a", span: nil)
        XCTAssertEqual(typed.operation, "plan_citation_rename")
        XCTAssertEqual(typed.payload["old_name"] as? String, "a")
        XCTAssertEqual(typed.payload["new_name"] as? String, "b")
        XCTAssertEqual(typed.payload["membership_generation"] as? Int, 4)
        XCTAssertEqual(typed.payload["max_bytes"] as? Int, ProjectSearch.planMaxBytes)
        XCTAssertNil(typed.payload["path"])
        let span = ShellModel.IndexLocation(["path": "main.tex", "revision": 2, "start_byte": 7, "end_byte": 9])!
        let at = CitationRename.request(sourceVersions: ["main.tex": 2], membershipGeneration: 4, newName: "b", oldName: nil, span: span)
        XCTAssertEqual(at.operation, "plan_citation_rename_at")
        XCTAssertEqual(at.payload["path"] as? String, "main.tex")
        XCTAssertEqual(at.payload["start_byte"] as? Int, 7)
        XCTAssertEqual(at.payload["end_byte"] as? Int, 9)
        XCTAssertNil(at.payload["old_name"])
    }

    func testParsePlanKeepsExactOffsetsAcrossThreeDocuments() throws {
        let plan = try CitationRename.parsePlan(reply: Self.reply(), oldName: "knuth84", newName: "knuth1984").get()
        XCTAssertEqual(plan.paths, ["chapter.tex", "main.tex", "refs.bib"])
        XCTAssertEqual(plan.summary, "3 occurrences in 3 files")
        XCTAssertEqual(plan.label, "Rename citation “knuth84” to “knuth1984”")
        XCTAssertEqual(plan.membershipGeneration, 3)
        XCTAssertEqual(plan.edits.map(\.start), [20, 60, 9])
        XCTAssertEqual(plan.edits(in: "refs.bib").map(\.end), [16])
        // The `_at` variant: the old name comes from the plan itself.
        let at = try CitationRename.parsePlan(reply: Self.reply(), oldName: nil, newName: "knuth1984").get()
        XCTAssertEqual(at.oldName, "knuth84")
        XCTAssertEqual(at, plan)
    }

    func testParsePlanRefusesAnythingUnexpected() {
        func refused(_ reply: [String: Any], old: String? = "knuth84", new: String = "knuth1984", _ needle: String, line: UInt = #line) {
            switch CitationRename.parsePlan(reply: reply, oldName: old, newName: new) {
            case .success: XCTFail("accepted: \(needle)", line: line)
            case .failure(let e): XCTAssertTrue(e.message.contains(needle), e.message, line: line)
            }
        }
        var r = Self.reply(); var p = r["plan"] as! [String: Any]
        p["schema"] = ProjectSearch.planSchema; r["plan"] = p
        refused(r, "schema is flashtex.literal-replacement-plan.v1")
        r = Self.reply(); p = r["plan"] as! [String: Any]; p["kind"] = "label_rename"; r["plan"] = p
        refused(r, "kind is label_rename")
        r = Self.reply(); p = r["plan"] as! [String: Any]; p["proposal_only"] = false; r["plan"] = p
        refused(r, "proposal_only")
        refused(Self.reply(), old: "other", "renames “knuth84”, not “other”")
        refused(Self.reply(), new: "x", "new name is not “x”")
        refused(Self.reply(old: "same", new: "same"), old: "same", new: "same", "old and new key are the same")
        refused(Self.reply(edits: [["file": "main.tex", "revision": "1", "start_byte": "60", "end_byte": "66", "expected_text": "knuth84", "replacement": "knuth1984"]]),
                "does not rename exactly")
        refused(Self.reply(edits: [["file": "main.tex", "revision": "2", "start_byte": "60", "end_byte": "67", "expected_text": "knuth84", "replacement": "knuth1984"]]),
                "names main.tex r2, not the snapshot's r1")
        refused(Self.reply(edits: [["file": "main.tex", "revision": "1", "start_byte": "60", "end_byte": "67", "expected_text": "knuth84", "replacement": "knuth1984"],
                                   ["file": "main.tex", "revision": "1", "start_byte": "65", "end_byte": "72", "expected_text": "knuth84", "replacement": "knuth1984"]]),
                "overlaps or precedes")
        refused(Self.reply(edits: [["file": "main.tex", "revision": "1", "start_byte": 60.0, "end_byte": "67", "expected_text": "knuth84", "replacement": "knuth1984"]]),
                "malformed")
        r = Self.reply(); r["source_versions"] = ["main.tex": 1]
        refused(r, "differ from the reply's")
    }

    func testKeyRuleMirrorsTheHelper() {
        XCTAssertNil(CitationRename.keyProblem("knuth:1984-a_b/c.d"))
        XCTAssertEqual(CitationRename.keyProblem(""), "the new key is empty")
        XCTAssertTrue(CitationRename.keyProblem("a b")!.contains("whitespace"))
        XCTAssertTrue(CitationRename.keyProblem("a,b")!.contains("“,”"))
        XCTAssertTrue(CitationRename.keyProblem("a{b")!.contains("“{”"))
        XCTAssertTrue(CitationRename.keyProblem("a\u{1}b")!.contains("cannot contain"))
        XCTAssertTrue(CitationRename.keyProblem(String(repeating: "k", count: 4097))!.contains("4097 bytes"))
    }

    func testExplainQuotesTheHelperAndAddsAHint() {
        let e = CitationRename.explain(helperError: "project index: MissingBibliographyDefinition")
        XCTAssertTrue(e.contains("declared bibliography source"), e)
        XCTAssertTrue(e.hasSuffix("(helper: project index: MissingBibliographyDefinition)"), e)
        XCTAssertTrue(CitationRename.explain(helperError: "project index: RenameCollision { name: \"x\" }").contains("already exists"))
        XCTAssertTrue(CitationRename.explain(helperError: "project index: InvalidCitationRenamePlan").contains("not a citation key"))
        XCTAssertEqual(CitationRename.explain(helperError: "something else"), "helper: something else")
    }

    func testPreviewsAndAccessibilityLabelsNameTheDeclaredKind() throws {
        let plan = try CitationRename.parsePlan(reply: Self.reply(edits: [
            ["file": "refs.bib", "revision": "1", "start_byte": "9", "end_byte": "16", "expected_text": "knuth84", "replacement": "knuth1984"],
        ]), oldName: "knuth84", newName: "knuth1984").get()
        let bib = "@article{knuth84,\n  title={The TeXbook}\n}\n"
        let previews = CitationRename.previews(for: plan, texts: ["refs.bib": bib])
        XCTAssertEqual(previews.count, 1)
        guard let firstPreview = previews.first else { return XCTFail("expected one preview") }
        XCTAssertEqual(firstPreview.line, 1)
        XCTAssertEqual(firstPreview.before?.text, "@article{knuth84,")
        XCTAssertEqual(firstPreview.after?.text, "@article{knuth1984,")
        XCTAssertEqual(CitationRename.accessibilityLabel(index: 0, count: 1, preview: firstPreview, kind: .bibliography),
                       "occurrence 1 of 1, refs.bib (bibliography), line 1, @article{knuth84, becomes @article{knuth1984,")
        let unread = CitationRename.previews(for: plan, texts: [:])
        XCTAssertNil(unread[0].before)
        XCTAssertTrue(CitationRename.accessibilityLabel(index: 0, count: 1, preview: unread[0], kind: nil).contains("bytes 9 to 16, “knuth84” becomes “knuth1984”"))
    }

    @MainActor
    func testWithoutHelperEverythingIsRefusedAndNothingIsSent() async {
        let model = ShellModel()
        let client = CitationRenameClient(model: model)
        XCTAssertFalse(client.helperAvailable)
        let located = await client.locateKeyAtCaret()
        XCTAssertFalse(located)
        XCTAssertEqual(client.status, CitationRename.noHelperMessage)
        client.oldName = "a"; client.newName = "b"
        await client.planRename()
        XCTAssertNil(client.plan)
        XCTAssertEqual(client.status, CitationRename.noHelperMessage)
        await client.applyRename()
        XCTAssertEqual(client.status, "No proposal to apply.")
        await client.retryUncertain(commandID: "citation-rename-none")
        XCTAssertEqual(client.status, "No retained command citation-rename-none.")
        XCTAssertTrue(client.retainedCommands.isEmpty)
    }
}

/// Against the real helper (plan_citation_rename endpoints) on a rooted
/// project whose `.bib` is declared through Document Kinds: proposal over
/// three documents, guarded per-file application with durable revisions
/// advancing and exact text, the caret (`_at`) variant, refusals for a
/// caret elsewhere and for a stale snapshot, and exact `apply_group` replay.
@MainActor
final class CitationRenameHelperTests: XCTestCase {
    static let main = "\\documentclass{article}\n\\begin{document}\n\\section{Intro}\\label{sec:a}\nSee \\cite{knuth84} and \\ref{sec:b}.\n\\input{chapter}\n\\end{document}\n"
    static let chapter = "\\section{Chapter}\\label{sec:b}\nRésumé — again \\cite[p.~3]{knuth84}.\n"
    static let bib = "@article{knuth84,\n  author={Knuth, Donald E.},\n  title={The \\TeX book},\n  year={1984}\n}\n@book{lamport94,\n  title={LaTeX},\n  year={1994}\n}\n"

    private func byte(_ needle: String, in text: String) -> Int {
        text.utf8.distance(from: text.startIndex, to: text.range(of: needle)!.lowerBound)
    }

    private func utf16(_ needle: String, in text: String) -> Int { (text as NSString).range(of: needle).location }

    private func waitUntil(timeout: TimeInterval = 15, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout") }
            try await Task.sleep(nanoseconds: 30_000_000)
        }
    }

    /// Rooted project (main.tex → chapter.tex by `\input`, refs.bib on disk),
    /// helper attached, refs.bib declared as a bibliography through the
    /// integrated Document Kinds (never inferred from its extension). Skips
    /// when the helper build predates the citation plan endpoints.
    private func attached() async throws -> (ShellModel, CitationRenameClient, URL) {
        guard let helper = PreviewControllerTests.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("citation-rename-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        try Self.main.write(to: root.appendingPathComponent("project/main.tex"), atomically: true, encoding: .utf8)
        try Self.chapter.write(to: root.appendingPathComponent("project/chapter.tex"), atomically: true, encoding: .utf8)
        try Self.bib.write(to: root.appendingPathComponent("project/refs.bib"), atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: root.appendingPathComponent("project/main.tex")), .opened)
        model.attachController(at: helper)
        try await waitUntil { model.controllerState.ready && model.controllerState.durable["main.tex"] != nil && model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        let declared = await model.documentKinds.declareBibliography("refs.bib")
        guard declared == .declared(path: "refs.bib") else {
            cleanup(model, root)
            throw XCTSkip("could not declare refs.bib through the helper: \(declared) (\(model.documentKinds.status))")
        }
        XCTAssertEqual(model.documentKinds.kinds, ["main.tex": .latex, "chapter.tex": .latex, "refs.bib": .bibliography])
        let client = CitationRenameClient(model: model)
        // Probe the endpoint once so an old helper skips instead of failing.
        client.oldName = "knuth84"; client.newName = "knuth84probe"
        await client.planRename()
        if client.status.contains("has no plan_citation_rename") {
            cleanup(model, root)
            throw XCTSkip("helper binary predates plan_citation_rename: \(client.status)")
        }
        XCTAssertNotNil(client.plan, client.status)
        return (model, client, root)
    }

    private func cleanup(_ model: ShellModel, _ root: URL) {
        unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT")
        model.detachController()
        try? FileManager.default.removeItem(at: root)
    }

    func testTypedRenameCoversAllThreeDocumentsAndApplyAdvancesDurableRevisionsExactly() async throws {
        let (model, client, root) = try await attached()
        defer { cleanup(model, root) }
        client.newName = "knuth1984"
        await client.planRename()
        guard let plan = client.plan else { return XCTFail(client.status) }
        XCTAssertEqual(plan.oldName, "knuth84")
        XCTAssertEqual(plan.paths, ["chapter.tex", "main.tex", "refs.bib"], "two .tex references and the declared record")
        XCTAssertEqual(plan.summary, "3 occurrences in 3 files")
        XCTAssertEqual(plan.sourceVersions, ["chapter.tex": 1, "main.tex": 1, "refs.bib": 1])
        XCTAssertEqual(plan.edits.map(\.start), [byte("knuth84", in: Self.chapter), byte("knuth84", in: Self.main), byte("knuth84", in: Self.bib)])
        XCTAssertEqual(client.previews.map(\.line), [2, 4, 1])
        guard client.previews.count == 3 else { return XCTFail("expected three previews, got \(client.previews.count)") }
        XCTAssertEqual(client.previews[0].after?.text, "Résumé — again \\cite[p.~3]{knuth1984}.")
        XCTAssertEqual(client.previews[2].after?.text, "@article{knuth1984,")
        XCTAssertTrue(client.status.hasPrefix("Proposal: 3 occurrences in 3 files, renaming “knuth84” to “knuth1984” at durable chapter.tex r1 (latex), main.tex r1 (latex), refs.bib r1 (bibliography). Nothing is changed"), client.status)
        // Proposal only: nothing moved.
        XCTAssertEqual(model.controllerState.durable["refs.bib"]?.revision, 1)
        XCTAssertTrue(model.activeText.contains("knuth84}"))
        let editorRevisionBefore = model.editorRevision

        await client.applyRename()
        XCTAssertEqual(client.outcomes.map(\.path), ["chapter.tex", "main.tex", "refs.bib"])
        for o in client.outcomes {
            guard case .applied(let rev, let id, let note) = o.state else { return XCTFail(o.description) }
            XCTAssertEqual(rev, 2)
            XCTAssertTrue(id.hasPrefix("citation-rename-"), id)
            XCTAssertEqual(note, "", o.description)
        }
        XCTAssertTrue(client.status.hasPrefix("Rename citation “knuth84” to “knuth1984”: 3 of 3 files applied."), client.status)
        XCTAssertNil(client.plan)
        XCTAssertTrue(client.retainedCommands.isEmpty)
        XCTAssertEqual(client.applyCount, 1)
        // Durable revisions advanced and the text is exact in every file, including the adopted .bib buffer.
        for path in ["chapter.tex", "main.tex", "refs.bib"] { XCTAssertEqual(model.controllerState.durable[path]?.revision, 2, path) }
        XCTAssertEqual(model.activeText, Self.main.replacingOccurrences(of: "knuth84", with: "knuth1984"))
        XCTAssertGreaterThan(model.editorRevision, editorRevisionBefore)
        XCTAssertEqual(model.controllerState.textByDurable["chapter.tex"]?[2], Self.chapter.replacingOccurrences(of: "knuth84", with: "knuth1984"))
        XCTAssertEqual(model.controllerState.textByDurable["refs.bib"]?[2], Self.bib.replacingOccurrences(of: "knuth84", with: "knuth1984"))
        XCTAssertEqual(model.documents.first { $0.path == "refs.bib" }?.text, Self.bib.replacingOccurrences(of: "knuth84", with: "knuth1984"))
        try await waitUntil { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2, "no extra durable revision from a resubmitted buffer")
        // The helper agrees: its documents carry the new key, its history the group label, and the kind survived.
        for path in ["chapter.tex", "main.tex", "refs.bib"] {
            guard case .success(let d)? = await model.controllerRequest("document", ["path": path]) else { return XCTFail(path) }
            let doc = d["document"] as? [String: Any]
            XCTAssertEqual(doc?["revision"] as? Int, 2, path)
            XCTAssertTrue((doc?["text"] as? String)?.contains("knuth1984") == true, path)
            XCTAssertFalse((doc?["text"] as? String)?.contains("knuth84}") == true, path)
        }
        guard case .success(let status)? = await model.controllerRequest("history_status", ["path": "refs.bib"]) else { return XCTFail() }
        let labels = ((status["history"] as? [String: Any])?["undo_labels"] as? [String]) ?? []
        XCTAssertEqual(labels.last, "Rename citation “knuth84” to “knuth1984” (1 in refs.bib)", "\(labels)")
        let refreshed = await model.documentKinds.refresh()
        XCTAssertTrue(refreshed, model.documentKinds.status)
        XCTAssertEqual(model.documentKinds.kind(of: "refs.bib"), .bibliography)
        // Renaming back to a key that no longer exists is refused by the helper with an explanation.
        client.oldName = "knuth84"; client.newName = "x"
        await client.planRename()
        XCTAssertNil(client.plan)
        XCTAssertTrue(client.status.hasPrefix("Rename refused: "), client.status)
        XCTAssertTrue(client.status.contains("(helper: "), client.status)
        // A collision with an existing key is refused too.
        client.oldName = "knuth1984"; client.newName = "lamport94"
        await client.planRename()
        XCTAssertNil(client.plan)
        XCTAssertTrue(client.status.contains("already exists"), client.status)
        XCTAssertTrue(client.status.contains("RenameCollision"), client.status)
    }

    func testCaretVariantUsesTheExactKeySpanAndRefusesACaretElsewhere() async throws {
        let (model, client, root) = try await attached()
        defer { cleanup(model, root) }
        // Caret inside the key of main.tex's \cite{knuth84}: the helper names the span.
        model.caretUTF16 = utf16("knuth84", in: Self.main) + 3
        let located = await client.locateKeyAtCaret()
        XCTAssertTrue(located, client.status)
        let keyStart = byte("knuth84", in: Self.main)
        XCTAssertEqual(client.keySpan, .init(name: "knuth84", location: ShellModel.IndexLocation(["path": "main.tex", "revision": 1, "start_byte": keyStart, "end_byte": keyStart + 7])!))
        XCTAssertEqual(client.oldName, "knuth84")
        XCTAssertTrue(client.status.hasPrefix("Citation key “knuth84” at main.tex bytes \(keyStart)..<\(keyStart + 7) (durable r1)."), client.status)
        client.newName = "knuth1984"
        await client.planRename()
        guard let plan = client.plan else { return XCTFail(client.status) }
        XCTAssertEqual(plan.paths, ["chapter.tex", "main.tex", "refs.bib"], "the _at plan equals the typed plan")
        XCTAssertEqual(plan.edits.count, 3)
        XCTAssertEqual(plan.oldName, "knuth84")
        XCTAssertNotNil(client.keySpan, "the span is kept until applied or discarded")

        // A caret on `\cite` itself (the command name) still resolves to its key argument.
        model.caretUTF16 = utf16("\\cite{knuth84}", in: Self.main) + 2
        let located1 = await client.locateKeyAtCaret()
        XCTAssertTrue(located1, client.status)
        XCTAssertEqual(client.keySpan?.location.start, keyStart)

        // Caret on a label (an indexed symbol that is not a citation): refused, nothing planned.
        model.caretUTF16 = utf16("sec:a", in: Self.main) + 1
        let located4 = await client.locateKeyAtCaret()
        XCTAssertFalse(located4, client.status)
        XCTAssertNil(client.keySpan)
        XCTAssertTrue(client.status.contains("“sec:a”"), client.status)
        XCTAssertTrue(client.status.contains("no citation key"), client.status)
        // Caret on plain prose: no symbol at all.
        model.caretUTF16 = utf16("See", in: Self.main) + 1
        let located5 = await client.locateKeyAtCaret()
        XCTAssertFalse(located5, client.status)
        XCTAssertNil(client.keySpan)
        XCTAssertTrue(client.status.contains("on no citation key the project index knows"), client.status)
        // Caret on \begin: matched in the buffer, never sent.
        model.caretUTF16 = utf16("\\begin{document}", in: Self.main) + 3
        let located6 = await client.locateKeyAtCaret()
        XCTAssertFalse(located6, client.status)
        XCTAssertTrue(client.status.contains("\\begin/\\end"), client.status)
        // With no span, the typed key is used; an empty one is refused before any request.
        client.oldName = ""; client.newName = "k"
        await client.planRename()
        XCTAssertNil(client.plan)
        XCTAssertTrue(client.status.hasPrefix("Type the citation key to rename"), client.status)
        client.oldName = "knuth84"; client.newName = "bad key"
        await client.planRename()
        XCTAssertEqual(client.status, "Cannot plan: the key contains “whitespace”, which a citation key cannot contain.")
        client.newName = "knuth84"
        await client.planRename()
        XCTAssertEqual(client.status, "The new key equals the old key; nothing to change.")

        // A local, not yet durable edit: the caret lookup is refused (the helper indexed other bytes).
        model.autoCompile = false
        model.updateActiveText(Self.main + "% local\n")
        model.caretUTF16 = utf16("knuth84", in: Self.main) + 3
        let located7 = await client.locateKeyAtCaret()
        XCTAssertFalse(located7, client.status)
        XCTAssertTrue(client.status.contains("has edits the helper has not indexed yet"), client.status)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 1)
    }

    func testStaleSnapshotsAreRefusedAsAWholeAndTheCaretSpanIsDroppedAfterAnEdit() async throws {
        let (model, client, root) = try await attached()
        defer { cleanup(model, root) }
        model.caretUTF16 = utf16("knuth84", in: Self.main) + 1
        let located2 = await client.locateKeyAtCaret()
        XCTAssertTrue(located2, client.status)
        client.newName = "knuth1984"
        await client.planRename()
        guard let plan = client.plan else { return XCTFail(client.status) }
        XCTAssertEqual(plan.sourceVersions["main.tex"], 1)
        // The project moves on before Apply (a durable edit of main.tex): refused as a whole, nothing applied anywhere.
        model.updateActiveText(Self.main + "% more\n")
        try await waitUntil { model.controllerState.durable["main.tex"]?.revision == 2 && model.inFlightRevision == nil }
        await client.applyRename()
        XCTAssertTrue(client.outcomes.isEmpty)
        XCTAssertEqual(client.status, "Project changed since this proposal (main.tex r1→r2); nothing applied — plan again.")
        XCTAssertNil(client.plan)
        XCTAssertEqual(client.applyCount, 0)
        XCTAssertEqual(model.controllerState.durable["chapter.tex"]?.revision, 1)
        XCTAssertEqual(model.controllerState.durable["refs.bib"]?.revision, 1)
        XCTAssertTrue(model.activeText.contains("knuth84}"))
        // The held caret span named r1: planning again refuses it and asks for the caret again.
        XCTAssertNotNil(client.keySpan)
        await client.planRename()
        XCTAssertNil(client.plan)
        XCTAssertTrue(client.status.hasPrefix("main.tex changed since the key was located (durable r1→r2); place the caret on the key again."), client.status)
        XCTAssertNil(client.keySpan)
        // Locating again on the new durable text works, and the plan is fresh.
        try await waitUntil { model.controllerState.textByDurable["main.tex"]?[2] != nil }
        let located3 = await client.locateKeyAtCaret()
        XCTAssertTrue(located3, client.status)
        XCTAssertEqual(client.keySpan?.location.revision, 2)
        await client.planRename()
        XCTAssertEqual(client.plan?.sourceVersions, ["chapter.tex": 1, "main.tex": 2, "refs.bib": 1])

        // A durable edit of the .bib after planning is a stale snapshot too (it is one of the plan's documents).
        guard case .success(let d)? = await model.controllerRequest("document", ["path": "refs.bib"]) else { return XCTFail() }
        let doc = d["document"] as! [String: Any]
        guard case .success? = await model.controllerRequest("edit", ["path": "refs.bib", "expected_revision": 1, "expected_sha256": doc["source_sha256"]!,
                                                                        "text": Self.bib + "% touched\n"]) else { return XCTFail("bib edit") }
        await client.applyRename()
        XCTAssertTrue(client.outcomes.isEmpty)
        XCTAssertEqual(client.status, "Project changed since this proposal (refs.bib r1→r2); nothing applied — plan again.")
    }

    func testApplyGroupReplaysTheIdenticalCommandAndRefusesAChangedPayload() async throws {
        let (model, client, root) = try await attached()
        defer { cleanup(model, root) }
        client.newName = "knuth1984"
        await client.planRename()
        guard let plan = client.plan else { return XCTFail(client.status) }
        // Send refs.bib's group by hand with a fixed id, then again: the ledger
        // replays it without a second revision — what a retry after an
        // uncertain reply relies on (same id, identical payload).
        let edits = plan.edits(in: "refs.bib")
        XCTAssertEqual(edits.count, 1)
        let durable = model.controllerState.durable["refs.bib"]!
        let payload = ProjectSearch.applyGroupPayload(path: "refs.bib", commandID: "citation-rename-fixed", expectedRevision: 1,
                                                      expectedSHA256: durable.sha256, label: plan.label, edits: edits)
        guard case .success(let first)? = await model.controllerRequest("apply_group", payload) else { return XCTFail() }
        let doc1 = (first["history"] as! [String: Any])["document"] as! [String: Any]
        XCTAssertEqual(doc1["revision"] as? Int, 2)
        XCTAssertEqual(doc1["text"] as? String, Self.bib.replacingOccurrences(of: "knuth84", with: "knuth1984"))
        XCTAssertEqual((first["history"] as! [String: Any])["replayed_command"] as? Bool, false)
        guard case .success(let second)? = await model.controllerRequest("apply_group", payload) else { return XCTFail() }
        let doc2 = (second["history"] as! [String: Any])["document"] as! [String: Any]
        XCTAssertEqual(doc2["revision"] as? Int, 2, "replayed, no second revision")
        XCTAssertEqual((second["history"] as! [String: Any])["replayed_command"] as? Bool, true)
        var changed = payload
        var command = changed["command"] as! [String: Any]
        command["label"] = "other"
        changed["command"] = command
        guard case .failure(let e)? = await model.controllerRequest("apply_group", changed) else { return XCTFail("changed payload accepted") }
        XCTAssertFalse(e.message.isEmpty)
        // The client's own Apply now sees refs.bib at r2: the whole plan is
        // stale (never a partial silent apply), and a fresh plan covers only
        // the two remaining references at the new snapshot.
        await client.applyRename()
        XCTAssertEqual(client.status, "Project changed since this proposal (refs.bib r1→r2); nothing applied — plan again.")
        await client.planRename()
        XCTAssertNil(client.plan, "knuth84 has no declared record any more: \(client.status)")
        XCTAssertTrue(client.status.contains("MissingBibliographyDefinition"), client.status)
    }
}
