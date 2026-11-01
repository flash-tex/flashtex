import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// GH39 — the search/citation `apply_group` round trip versus typing.
///
/// Against the REAL `flashtex-preview-controller` + compiler
/// (`FLASHTEX_PREVIEW_CONTROLLER`, `FLASHTEX_COMPILER`; skipped otherwise),
/// launched behind `Fixtures/holding_preview_controller_proxy.py`: a
/// byte-transparent stdio proxy that forwards every request unchanged but
/// queues every helper→app line from the first `apply_group` request until a
/// release file exists. The helper applies the group and moves its ledger
/// immediately; only the app's view of the reply is deterministically late —
/// the exact GH39 window, independent of machine load.
///
/// Reported (coordination/mac-core-review.md "Reported only" #7): keystrokes
/// typed in that window were overwritten by the returned document. The three
/// tests here are the acceptance criteria of issue #39: typing survives and
/// becomes durable on top; the unchanged-buffer control still adopts the
/// returned result exactly; an uncertain reply is retried with the identical
/// command id and payload.
@MainActor
final class SearchReconcileTests: XCTestCase {
    static let proxy = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().appendingPathComponent("Fixtures/holding_preview_controller_proxy.py")
    static let main = ProjectSearchHelperTests.main
    static let chapter = ProjectSearchHelperTests.chapter
    static let replaced = ProjectSearchHelperTests.main.replacingOccurrences(of: "café", with: "tea")

    struct Fixture {
        var model: ShellModel
        var client: ProjectSearchClient
        var root: URL
        var release: URL
        var mark: URL
        var helper: URL
    }

    /// Load-aware wait: a timeout is a skip (never a silent pass), and the
    /// skip names the shell state so a logic failure is not mistaken for load.
    private func waitUntil(timeout: TimeInterval = 20, _ model: ShellModel? = nil, wire: URL? = nil, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout {
                var state = ""
                if let model {
                    state = " — durable r\(model.controllerState.durable["main.tex"]?.revision ?? -1), inFlight \(model.controllerState.inFlight.map { "\($0.id) editor \($0.editorRevision) durable \($0.durableRevision.map(String.init) ?? "nil")" } ?? "nil"), editor \(model.editorRevision), result \(model.result?.revision.description ?? "nil"), status “\(model.controllerStatus)”, log: \(model.workerLog.suffix(40).joined(separator: " | "))"
                    if let wire, let text = try? String(contentsOf: wire, encoding: .utf8) { state += "\nwire:\n" + text.split(separator: "\n").suffix(30).joined(separator: "\n") }
                }
                throw XCTSkip("timeout after \(timeout) s (load \(PasteRecoveryTests.loadAverage1()))\(state)")
            }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    /// The two-file search project behind the holding proxy, searched and
    /// planned for the ACTIVE document only (one `apply_group`, for main.tex).
    private func attached(_ name: String) async throws -> Fixture {
        guard let helper = PreviewControllerTests.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("search-reconcile-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        try Self.main.write(to: root.appendingPathComponent("project/main.tex"), atomically: true, encoding: .utf8)
        try Self.chapter.write(to: root.appendingPathComponent("project/chapter.tex"), atomically: true, encoding: .utf8)
        let release = root.appendingPathComponent("release"), mark = root.appendingPathComponent("held")
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        setenv("FLASHTEX_HOLD_TYPE", "apply_group", 1)
        setenv("FLASHTEX_HOLD_RELEASE", release.path, 1)
        setenv("FLASHTEX_HOLD_MARK", mark.path, 1)
        setenv("FLASHTEX_HOLD_WIRE", root.appendingPathComponent("wire.log").path, 1)
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: root.appendingPathComponent("project/main.tex")), .opened)
        model.attachController(at: Self.proxy)
        XCTAssertTrue(model.controllerAttached)
        try await waitUntil(model) { model.controllerState.durable["main.tex"] != nil && model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        let client = ProjectSearchClient(model: model)
        client.query = "café"
        client.replacement = "tea"
        client.scope = .activeDocument
        await client.search()
        XCTAssertEqual(client.results?.matches.count, 2, client.status)
        await client.planReplacement()
        if client.planStatus.contains("has no plan_literal_replacement") {
            cleanup(.init(model: model, client: client, root: root, release: release, mark: mark, helper: helper))
            throw XCTSkip("helper binary predates plan_literal_replacement: \(client.planStatus)")
        }
        XCTAssertEqual(client.plan?.summary, "2 replacements in 1 file", client.planStatus)
        XCTAssertEqual(client.plan?.sourceVersions, ["chapter.tex": 1, "main.tex": 1])
        return .init(model: model, client: client, root: root, release: release, mark: mark, helper: helper)
    }

    private func cleanup(_ f: Fixture) {
        for v in ["FLASHTEX_CONTROLLER_LEDGER_ROOT", "FLASHTEX_HOLD_TYPE", "FLASHTEX_HOLD_RELEASE", "FLASHTEX_HOLD_MARK", "FLASHTEX_HOLD_WIRE"] { unsetenv(v) }
        f.model.detachController()
        if ProcessInfo.processInfo.environment["FLASHTEX_KEEP_FIXTURE"] == nil { try? FileManager.default.removeItem(at: f.root) } else { print("search-reconcile: fixture kept at \(f.root.path)") }
    }

    private func release(_ f: Fixture) throws { try Data().write(to: f.release) }

    /// Starts Apply and returns once the proxy has forwarded the `apply_group`
    /// (the helper is applying it; the reply is now held).
    private func startApplyAndWaitForHold(_ f: Fixture) async throws -> Task<Void, Never> {
        let apply = Task { @MainActor in await f.client.applyPlan() }
        try await waitUntil(timeout: 20) { FileManager.default.fileExists(atPath: f.mark.path) }
        XCTAssertTrue(f.client.isApplying)
        return apply
    }

    // MARK: acceptance 1 — typing during the delayed reply survives and becomes durable on top

    func testTypingDuringADelayedApplyGroupIsKeptAndResubmittedOnTop() async throws {
        let f = try await attached("typing")
        defer { cleanup(f) }
        let model = f.model, client = f.client
        let snapshotRevision = model.editorRevision
        let apply = try await startApplyAndWaitForHold(f)

        // The GH39 window: the helper has applied the group (its ledger is at
        // r2) and the app is awaiting a reply that will not come until released.
        // The user types.
        let typed = Self.main + "% typed while apply_group was in flight\n"
        model.updateActiveText(typed)
        let typedRevision = model.editorRevision
        XCTAssertGreaterThan(typedRevision, snapshotRevision)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 1, "no reply has been seen yet")
        try await Task.sleep(nanoseconds: 150_000_000) // the typed edit is sent against r1 meanwhile; its conflict refusal is held behind the reply too
        XCTAssertTrue(model.activeText.sameBytes(as: typed))

        try release(f)
        await apply.value

        // The helper DID apply the group: the outcome says so and names the note.
        XCTAssertEqual(client.applyOutcomes.count, 1)
        guard let firstOutcome = client.applyOutcomes.first else { return XCTFail("expected at least one apply outcome") }
        guard case .applied(let rev, let id, let note) = firstOutcome.state else { return XCTFail(firstOutcome.description) }
        XCTAssertEqual(rev, 2)
        XCTAssertTrue(id.hasPrefix("search-replace-"))
        XCTAssertTrue(note.contains("the editor moved during the apply"), note)
        XCTAssertTrue(client.retainedCommands.isEmpty, "a delivered reply retains nothing")

        // (1) The newer typing survived: the buffer was NOT overwritten by the returned document.
        XCTAssertTrue(model.activeText.sameBytes(as: typed), "typing during the round trip was overwritten:\n\(model.activeText)")
        XCTAssertFalse(model.activeText.sameBytes(as: Self.replaced))
        // (2) The helper's durable identity for the returned document was recorded exactly.
        let r2 = try XCTUnwrap(model.controllerState.textByDurable["main.tex"]?[2])
        XCTAssertTrue(r2.sameBytes(as: Self.replaced), "durable r2 is the helper's post-apply text")
        // (3) The normal edit path resubmits the kept buffer on top: it becomes durable r3.
        try await waitUntil(timeout: 30, model) { model.controllerState.durable["main.tex"]?.revision == 3 && model.inFlightRevision == nil }
        let d3 = try XCTUnwrap(model.controllerState.durable["main.tex"])
        XCTAssertEqual(d3.revision, 3)
        XCTAssertEqual(d3.sha256, SourceDigest.sha256Hex(typed))
        XCTAssertTrue(model.controllerState.textByDurable["main.tex"]?[3]?.sameBytes(as: typed) == true)
        XCTAssertTrue(model.activeText.sameBytes(as: typed), "still the typed text after the resubmission")
        try await waitUntil(timeout: 30, model) { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        XCTAssertFalse(model.previewIsStale)
        XCTAssertTrue(model.compiledDocuments["main.tex"]?.sameBytes(as: typed) == true, "the preview shown is the typed buffer's")
        // A subsequent edit becomes durable on top (r4) — the pipeline is not wedged.
        let again = typed + "% and again\n"
        model.updateActiveText(again)
        try await waitUntil(timeout: 30, model) { model.controllerState.durable["main.tex"]?.revision == 4 && model.inFlightRevision == nil }
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.sha256, SourceDigest.sha256Hex(again))
        XCTAssertTrue(model.activeText.sameBytes(as: again))
        // The ledger's history is truthful: the group, then the two buffer edits.
        guard case .success(let status)? = await model.controllerRequest("history_status", ["path": "main.tex"]) else { return XCTFail() }
        let labels = ((status["history"] as? [String: Any])?["undo_labels"] as? [String]) ?? []
        XCTAssertTrue(labels.contains("Replace “café” with “tea” (2 in main.tex)"), "\(labels)")
        print("search-reconcile: typing survived (editor \(snapshotRevision)→\(typedRevision)); durable r2 = helper text, r3 = typed buffer, r4 = next edit; load \(PasteRecoveryTests.loadAverage1())")
    }

    // MARK: acceptance 2 — unchanged buffer: the returned result is applied exactly as before

    func testUnchangedBufferAdoptsTheDelayedApplyGroupResultExactly() async throws {
        let f = try await attached("control")
        defer { cleanup(f) }
        let model = f.model, client = f.client
        let editorBefore = model.editorRevision
        let apply = try await startApplyAndWaitForHold(f)
        try await Task.sleep(nanoseconds: 150_000_000) // the same window, no typing
        XCTAssertEqual(model.editorRevision, editorBefore)
        XCTAssertTrue(model.activeText.sameBytes(as: Self.main))
        try release(f)
        await apply.value

        guard let firstOutcome = client.applyOutcomes.first else { return XCTFail("expected at least one apply outcome") }
        guard case .applied(let rev, _, let note) = firstOutcome.state else { return XCTFail(firstOutcome.description) }
        XCTAssertEqual(rev, 2)
        XCTAssertEqual(note, "", "no moved-editor note for an unchanged buffer")
        XCTAssertTrue(model.activeText.sameBytes(as: Self.replaced), "the buffer adopted the returned document exactly")
        XCTAssertGreaterThan(model.editorRevision, editorBefore, "adoption is an editor revision like typing")
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.sha256, SourceDigest.sha256Hex(Self.replaced))
        XCTAssertEqual(model.controllerState.editorRevisionByDurable["main.tex"]?[2], model.editorRevision, "the follow-up preview binds to the adopted revision")
        XCTAssertTrue(client.planStatus.hasPrefix("Replace “café” with “tea”: 1 of 1 file applied."), client.planStatus)
        try await waitUntil(timeout: 30, model) { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2, "nothing was resubmitted: the buffer was already durable")
        XCTAssertTrue(model.compiledDocuments["main.tex"]?.sameBytes(as: Self.replaced) == true)
        XCTAssertEqual(client.results?.matches.count, 0, "the re-run search finds no café in main.tex: \(client.status)")
    }

    // MARK: acceptance 3 — an uncertain reply is retried with the identical command, then reconciled

    func testUncertainApplyGroupIsRetriedWithTheIdenticalCommandAndReconciled() async throws {
        let f = try await attached("retry")
        defer { cleanup(f) }
        let model = f.model, client = f.client
        let apply = try await startApplyAndWaitForHold(f)
        // The helper goes away while the reply is held: the outcome is
        // uncertain and the command (id + payload) is retained unchanged.
        try await Task.sleep(nanoseconds: 100_000_000)
        model.detachController()
        await apply.value
        XCTAssertEqual(client.applyOutcomes.count, 1)
        guard let firstOutcome = client.applyOutcomes.first else { return XCTFail("expected at least one apply outcome") }
        guard case .uncertain(let why, let commandID) = firstOutcome.state else { return XCTFail(firstOutcome.description) }
        XCTAssertTrue(why.hasPrefix("helper exited"), why)
        let retained = try XCTUnwrap(client.retainedCommands[commandID])
        XCTAssertEqual((retained["command"] as? [String: Any])?["command_id"] as? String, commandID)
        XCTAssertTrue(model.activeText.sameBytes(as: Self.main), "no reply was seen: the buffer is untouched")

        // Relaunch the real helper on the same ledger (the group WAS applied: r2).
        // The shell's attach policy resubmits the differing buffer on top (r3).
        for v in ["FLASHTEX_HOLD_TYPE", "FLASHTEX_HOLD_RELEASE", "FLASHTEX_HOLD_MARK"] { unsetenv(v) }
        model.attachController(at: Self.proxy) // transparent now (no hold), traced
        try await waitUntil(timeout: 30, model, wire: f.root.appendingPathComponent("wire.log")) { model.controllerState.ready && model.controllerState.durable["main.tex"]?.revision == 3 && model.inFlightRevision == nil && model.result?.revision == model.editorRevision }
        XCTAssertTrue(model.controllerState.textByDurable["main.tex"]?[3]?.sameBytes(as: Self.main) == true)
        XCTAssertEqual(client.retainedCommands[commandID].map { NSDictionary(dictionary: $0) }, NSDictionary(dictionary: retained), "the retained payload is byte-for-byte the original")

        // Retry: identical id and payload → the ledger replays (never applies
        // twice), the current document (r3) is reconciled, the note says so.
        let editorBefore = model.editorRevision
        await client.retryUncertain(commandID: commandID)
        guard let firstOutcome = client.applyOutcomes.first else { return XCTFail("expected at least one apply outcome") }
        guard case .applied(let rev, let id, let note) = firstOutcome.state else { return XCTFail(firstOutcome.description) }
        XCTAssertEqual(id, commandID, "the retry keeps the command id")
        XCTAssertEqual(rev, 3, "replayed: the current document, no fourth revision")
        XCTAssertTrue(note.contains("(replayed: the ledger had already applied this command)"), note)
        XCTAssertFalse(note.contains("the editor moved"), note)
        XCTAssertTrue(client.retainedCommands.isEmpty)
        XCTAssertEqual(model.editorRevision, editorBefore, "the buffer already equalled r3: nothing adopted, nothing resubmitted")
        XCTAssertTrue(model.activeText.sameBytes(as: Self.main))
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 3)
        XCTAssertTrue(client.planStatus.contains("applied as durable r3"), client.planStatus)
        // The replay's follow-up preview releases the pipeline; the next edit becomes durable on top.
        try await waitUntil(timeout: 30, model, wire: f.root.appendingPathComponent("wire.log")) { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 3, "a replay never applies twice")
        let next = Self.main + "% after the retry\n"
        model.updateActiveText(next)
        try await waitUntil(timeout: 30, model, wire: f.root.appendingPathComponent("wire.log")) { model.controllerState.durable["main.tex"]?.revision == 4 && model.inFlightRevision == nil }
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.sha256, SourceDigest.sha256Hex(next))
        // (A changed payload under the same id is refused by the helper:
        // ProjectSearchHelperTests.testRetryingAnUncertainCommandReplaysExactly.)
    }
}
