import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// EditHistoryPanel.swift: pure decoding/row logic, and the durable undo/redo
/// route end-to-end against the real `flashtex-preview-controller` helper with
/// the real compiler (skipped unless `FLASHTEX_PREVIEW_CONTROLLER` and
/// `FLASHTEX_COMPILER` point at built binaries).
@MainActor
final class EditHistoryTests: XCTestCase {
    static var helper: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) }
    }

    // MARK: pure

    func testStatusAndResultDecodeTheHelperWire() throws {
        let status = try XCTUnwrap(EditHistory.Status.decode([
            "history": ["history_bytes": 84, "permanent_command_ids": 1, "redo_labels": ["Source edit"], "undo_labels": ["Capture c-1", "Source edit"]],
        ]))
        XCTAssertEqual(status.undoLabels, ["Capture c-1", "Source edit"])
        XCTAssertEqual(status.redoLabels, ["Source edit"])
        XCTAssertEqual(status.entries, 3)
        XCTAssertEqual(status.historyBytes, 84)
        XCTAssertEqual(status.permanentCommandIDs, 1)
        XCTAssertFalse(status.isFull)
        XCTAssertFalse(status.nearCapacity)
        XCTAssertNil(EditHistory.Status.decode(["history": ["undo_labels": []]]), "both stacks are required")
        XCTAssertEqual(status.limits, .init(), "an older helper reports no limits: the built-in constants apply")
        XCTAssertFalse(status.limits.reported)
        XCTAssertNil(status.identity)

        // main ≥ 64829a0d: same-turn document identity (no text) and the actual ledger limits.
        let newer = try XCTUnwrap(EditHistory.Status.decode([
            "history": ["history_bytes": 10, "permanent_command_ids": 0, "redo_labels": [], "undo_labels": ["Source edit"]],
            "document": ["project_id": "demo", "path": "main.tex", "revision": 7, "source_sha256": "ff"],
            "limits": ["history_bytes": 1024, "history_entries": 4, "permanent_command_ids": 8],
        ]))
        XCTAssertEqual(newer.identity, .init(path: "main.tex", revision: 7, sha256: "ff"))
        XCTAssertEqual(newer.limits, .init(entries: 4, bytes: 1024, commandIDs: 8, reported: true))
        XCTAssertEqual(newer.usage, 0.25, accuracy: 1e-9, "usage is measured against the reported limits")
        XCTAssertEqual(newer.retentionSummary, "1 of 4 steps · 10 B of 1 KiB · 0 of 8 command ids")
        XCTAssertFalse(newer.isFull)

        let result = try XCTUnwrap(EditHistory.Result.decode([
            "history": ["can_redo": true, "can_undo": false, "command_revision": 3, "replayed_command": true,
                        "document": ["path": "main.tex", "project_id": "demo", "revision": 3, "source_sha256": "abc", "text": "x"]],
            "preview_error": "compiler unavailable; source remains saved", "save_and_submit_ms": 8.5,
        ]))
        XCTAssertEqual(result.document, .init(path: "main.tex", revision: 3, sha256: "abc", text: "x"))
        XCTAssertEqual(result.commandRevision, 3)
        XCTAssertTrue(result.replayedCommand)
        XCTAssertTrue(result.canRedo)
        XCTAssertFalse(result.canUndo)
        XCTAssertEqual(result.previewError, "compiler unavailable; source remains saved")
        XCTAssertEqual(result.saveAndSubmitMs, 8.5)
        XCTAssertNil(EditHistory.Result.decode(["history": ["command_revision": 3]]), "the document is required")
    }

    func testFailureClassificationFollowsLedgerCodes() {
        XCTAssertEqual(EditHistory.Failure.classify("document_conflict: history command source revision/hash is stale"), .documentConflict)
        XCTAssertEqual(EditHistory.Failure.classify("command_id_conflict: command ID already binds another operation"), .commandIDConflict)
        XCTAssertEqual(EditHistory.Failure.classify("history_empty: no retained history in that direction"), .historyEmpty)
        XCTAssertEqual(EditHistory.Failure.classify("history_full: history limit reached; explicit payload retention required"), .historyFull)
        XCTAssertEqual(EditHistory.Failure.classify("history_ids_full: permanent command-ID limit reached; IDs cannot be evicted"), .historyIDsFull)
        XCTAssertEqual(EditHistory.Failure.classify("invalid_id: expected 1–128 ASCII letters, digits, hyphens or underscores"), .invalidID)
        XCTAssertEqual(EditHistory.Failure.classify("unknown document"), .unknownDocument)
        XCTAssertEqual(EditHistory.Failure.classify("helper exited (9)"), .uncertain("helper exited (9)"))
        XCTAssertEqual(EditHistory.Failure.classify("project closed"), .other("project closed"))
        XCTAssertTrue(EditHistory.Failure.historyFull.isCapacity)
        XCTAssertTrue(EditHistory.Failure.historyIDsFull.isCapacity)
        XCTAssertFalse(EditHistory.Failure.documentConflict.isCapacity)
        XCTAssertTrue(EditHistory.Failure.historyFull.description.contains("256"))
    }

    func testCommandIDsAreLedgerIdentifiersAndPayloadMatchesTheContract() throws {
        let id = EditHistory.Command.newID()
        XCTAssertLessThanOrEqual(id.utf8.count, 128)
        XCTAssertTrue(id.utf8.allSatisfy { ($0 >= 0x30 && $0 <= 0x39) || ($0 >= 0x41 && $0 <= 0x5A) || ($0 >= 0x61 && $0 <= 0x7A) || $0 == 0x2D || $0 == 0x5F })
        XCTAssertNotEqual(id, EditHistory.Command.newID(), "every new action gets its own id")
        let c = EditHistory.Command(direction: .undo, commandID: "cmd-1", path: "main.tex", expectedRevision: 2, expectedSHA256: "deadbeef")
        let p = c.payload
        XCTAssertEqual(p["path"] as? String, "main.tex")
        let cmd = try XCTUnwrap(p["command"] as? [String: Any])
        XCTAssertEqual(cmd["command_id"] as? String, "cmd-1")
        XCTAssertEqual(cmd["expected_revision"] as? Int, 2)
        XCTAssertEqual(cmd["expected_sha256"] as? String, "deadbeef")
        XCTAssertEqual(cmd.count, 3)
    }

    func testRowsCollapseTypingRunsAndLabelCapturesReloadsAndGroups() {
        let labels = ["Source edit", "Source edit", "Capture cap-7", "Source edit", "Source edit", "Source edit", "Rename label", "Source edit"]
        var annotations: [String?] = Array(repeating: nil, count: labels.count)
        annotations[3] = "Reload from disk"
        let rows = EditHistory.rows(labels: labels, annotations: annotations, stack: .undo)
        XCTAssertEqual(rows.map(\.title), ["Typing", "Rename label", "Typing run", "Reload from disk", "Capture insertion", "Typing run"])
        XCTAssertEqual(rows.map(\.steps), [1, 1, 2, 1, 1, 2])
        XCTAssertEqual(rows.map(\.distance), [0, 1, 2, 4, 5, 6], "distance is the newest step's depth from the next undo")
        XCTAssertEqual(rows.map(\.kind), [.typing, .group, .typing, .reload, .capture, .typing])
        guard rows.count == 6 else { return XCTFail("expected six rows, got \(rows.count)") }
        XCTAssertEqual(rows[4].detail, "Capture cap-7")
        XCTAssertEqual(rows[3].detail, "recorded by the ledger as “Source edit”")
        XCTAssertEqual(Set(rows.map(\.id)).count, rows.count)
        XCTAssertEqual(rows[0].accessibilityLabel(direction: .undo), "Typing, 1 step, next undo. one durable source edit")
        XCTAssertEqual(rows[2].accessibilityLabel(direction: .undo), "Typing run, 2 steps, 2 steps below the next undo. 2 durable source edits, one step each")
        XCTAssertEqual(EditHistory.rows(labels: [], annotations: [], stack: .redo), [])
        XCTAssertEqual(EditHistory.rows(labels: ["Source edit"], annotations: [], stack: .redo).first?.accessibilityLabel(direction: .redo),
                       "Typing, 1 step, next redo. one durable source edit")
    }

    func testRetentionUsageAndWarnings() {
        var s = EditHistory.Status(undoLabels: Array(repeating: "Source edit", count: 200), redoLabels: Array(repeating: "Source edit", count: 10), permanentCommandIDs: 3, historyBytes: 1000)
        XCTAssertEqual(s.entries, 210)
        XCTAssertEqual(s.usage, 210.0 / 256.0, accuracy: 1e-9)
        XCTAssertTrue(s.nearCapacity)
        XCTAssertFalse(s.isFull)
        s.historyBytes = 32 * 1024 * 1024
        XCTAssertTrue(s.isFull)
        XCTAssertEqual(EditHistory.Status.bytes(32 * 1024 * 1024), "32.0 MiB")
        XCTAssertEqual(EditHistory.Status.bytes(2048), "2 KiB")
        XCTAssertEqual(EditHistory.Status.bytes(84), "84 B")
        XCTAssertTrue(s.retentionSummary.hasPrefix("210 of 256 steps · 32.0 MiB of 32.0 MiB · 3 of 4096 command ids"))
    }

    func testClientIsUnavailableWithoutAHelperAndRefusesLocally() {
        let model = ShellModel()
        let client = EditHistoryClient()
        client.bind(model)
        XCTAssertEqual(client.phase, .unavailable("no preview controller attached"))
        XCTAssertFalse(client.canUndo)
        XCTAssertFalse(client.bufferIsDurable)
        client.undo()
        XCTAssertNil(client.pending, "no request without a helper")
        client.resend()
        XCTAssertEqual(client.note, "nothing to retry")
        XCTAssertNil(client.capacityWarning)
        client.refresh()
        XCTAssertEqual(client.phase, .unavailable("no preview controller attached"))
    }

    // MARK: real helper

    private struct Fixture {
        var root: URL
        var tex: URL
        var model: ShellModel
        var client: EditHistoryClient
    }

    /// Opens a temporary project, attaches the helper and waits for the first
    /// preview bound to the buffer (durable r1).
    private func makeFixture(_ name: String, text: String = "\\begin{document}\nHello durable history.\n\\end{document}\n") async throws -> Fixture {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("history-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let tex = root.appendingPathComponent("project/main.tex")
        try text.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: helper)
        XCTAssertTrue(model.controllerAttached)
        try await waitUntil { model.result?.revision == model.editorRevision && model.controllerState.durable["main.tex"]?.revision == 1 && model.controllerState.inFlight == nil }
        let client = EditHistoryClient()
        client.bind(model)
        return Fixture(root: root, tex: tex, model: model, client: client)
    }

    private func tearDown(_ f: Fixture) {
        f.model.detachController()
        unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT")
        try? FileManager.default.removeItem(at: f.root)
    }

    func testEditUndoRedoRoundTripAdoptsExactDurableText() async throws {
        let f = try await makeFixture("roundtrip")
        defer { tearDown(f) }
        let model = f.model, client = f.client
        let original = model.activeText
        try await waitUntil { client.status != nil }
        XCTAssertEqual(client.status?.undoLabels, [], "a fresh ledger has no history")
        XCTAssertEqual(client.phase, .idle)
        XCTAssertFalse(client.canUndo)

        // One durable edit, then the stacks name it.
        let edited = "\\begin{document}\nHello durable history, edited.\n\\end{document}\n"
        model.updateActiveText(edited)
        try await waitUntil { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
        client.refresh()
        try await waitUntil { client.status?.undoLabels == ["Source edit"] }
        XCTAssertEqual(client.undoRows.map(\.title), ["Typing"])
        XCTAssertTrue(client.bufferIsDurable)
        XCTAssertTrue(client.canUndo)
        if let identity = client.status?.identity {
            // Helper from main ≥ 64829a0d: the stacks came with the identity they describe.
            XCTAssertEqual(identity.revision, 2)
            XCTAssertEqual(identity.sha256, model.controllerState.durable["main.tex"]?.sha256)
            XCTAssertEqual(identity.path, "main.tex")
            XCTAssertTrue(client.status!.limits.reported)
            XCTAssertEqual(client.status!.limits, .init(entries: 256, bytes: 32 * 1024 * 1024, commandIDs: 4096, reported: true), "the helper reports the ledger constants")
        } else {
            print("history round-trip test: helper predates 64829a0d (no history_status document/limits)")
        }
        XCTAssertFalse(client.canRedo)
        let editorBefore = model.editorRevision

        // Undo: guarded by r2's hash; the buffer takes exactly the original bytes, the revision ADVANCES to r3.
        client.undo()
        XCTAssertEqual(client.pending?.command.expectedRevision, 2)
        XCTAssertEqual(client.phase, .sending(.undo))
        try await waitUntil { client.pending == nil && client.lastResult != nil }
        let undone = try XCTUnwrap(client.lastResult)
        XCTAssertEqual(undone.document.text, original)
        XCTAssertEqual(undone.document.revision, 3)
        XCTAssertEqual(undone.commandRevision, 3)
        XCTAssertFalse(undone.replayedCommand)
        XCTAssertFalse(undone.canUndo)
        XCTAssertTrue(undone.canRedo)
        XCTAssertNil(undone.previewError, "the compiler is attached: the helper compiles the undone source")
        XCTAssertTrue(model.activeText.sameBytes(as: original), "the buffer adopted the durable text exactly")
        XCTAssertGreaterThan(model.editorRevision, editorBefore, "adoption is an editor revision like typing")
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 3)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.sha256, SourceDigest.sha256Hex(original))
        XCTAssertEqual(model.controllerState.editorRevisionByDurable["main.tex"]?[3], model.editorRevision, "the next preview binds to the adopted revision")
        // The helper's follow-up preview for r3 arrives and is not stale.
        try await waitUntil { model.result?.revision == model.editorRevision && model.controllerState.inFlight == nil }
        XCTAssertEqual(model.compiledDocuments["main.tex"], original)
        try await waitUntil { client.status?.redoLabels == ["Source edit"] && client.status?.undoLabels == [] }
        XCTAssertEqual(client.status?.permanentCommandIDs, 1)
        XCTAssertTrue(client.canRedo)
        XCTAssertFalse(client.canUndo)

        // Redo: guarded by r3; the buffer takes the edited bytes again at r4.
        client.redo()
        XCTAssertEqual(client.pending?.command.expectedRevision, 3)
        try await waitUntil { client.pending == nil && client.lastResult?.document.revision == 4 }
        XCTAssertTrue(model.activeText.sameBytes(as: edited))
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 4)
        XCTAssertEqual(client.lastResult?.canUndo, true)
        XCTAssertEqual(client.lastResult?.canRedo, false)
        try await waitUntil { model.result?.revision == model.editorRevision && model.controllerState.inFlight == nil }
        XCTAssertEqual(model.compiledDocuments["main.tex"], edited)
        try await waitUntil { client.status?.undoLabels == ["Source edit"] && client.status?.redoLabels == [] }
        XCTAssertEqual(client.status?.permanentCommandIDs, 2, "undo and redo each bound a fresh permanent id")
        XCTAssertNil(client.lastFailure)
        XCTAssertEqual(client.phase, .idle)

        // Typing after a redo is an ordinary edit on top (r5) and the stacks grow.
        model.updateActiveText(edited + "% more\n")
        try await waitUntil { model.controllerState.durable["main.tex"]?.revision == 5 && model.controllerState.inFlight == nil }
        client.refresh()
        try await waitUntil { client.status?.undoLabels.count == 2 }
        XCTAssertEqual(client.undoRows.map(\.title), ["Typing run"])
        XCTAssertEqual(client.undoRows.first?.steps, 2)

        // A session annotation names the next "Source edit" the panel witnesses
        // (the shell announces an explicit reload this way) and follows that step
        // across the stacks; the ledger label underneath stays visible.
        client.noteNextLabel("Reload from disk")
        model.updateActiveText(edited + "% more\n% reloaded\n")
        try await waitUntil { model.controllerState.durable["main.tex"]?.revision == 6 && model.controllerState.inFlight == nil }
        client.refresh()
        try await waitUntil { client.status?.undoLabels.count == 3 && !client.refreshing }
        XCTAssertEqual(client.undoRows.map(\.title), ["Reload from disk", "Typing run"])
        XCTAssertEqual(client.undoRows.first?.kind, .reload)
        XCTAssertEqual(client.undoRows.first?.detail, "recorded by the ledger as “Source edit”")
        XCTAssertEqual(client.undoRows.first?.accessibilityLabel(direction: .undo), "Reload from disk, 1 step, next undo. recorded by the ledger as “Source edit”")
        client.undo()
        try await waitUntil { client.pending == nil && client.lastResult?.document.revision == 7 }
        try await waitUntil { client.status?.redoLabels.count == 1 && !client.refreshing }
        XCTAssertEqual(client.redoRows.map(\.title), ["Reload from disk"], "the annotation moved with the step to the redo stack")
        XCTAssertEqual(client.undoRows.map(\.title), ["Typing run"])
        client.redo()
        try await waitUntil { client.pending == nil && client.lastResult?.document.revision == 8 }
        try await waitUntil { client.status?.undoLabels.count == 3 && !client.refreshing }
        XCTAssertEqual(client.undoRows.map(\.title), ["Reload from disk", "Typing run"], "and back")
        XCTAssertEqual(client.redoRows, [])

        // With a same-turn identity (helper ≥ 64829a0d) a refresh while nothing
        // durable changed sends no request; a forced one always does.
        if client.status?.identity != nil {
            XCTAssertTrue(client.statusIsCurrent)
            let sent = client.statusRequests
            client.refresh()
            XCTAssertEqual(client.statusRequests, sent, "identity equals the durable snapshot: no round trip")
            client.refresh(force: true)
            XCTAssertEqual(client.statusRequests, sent + 1)
            try await waitUntil { !client.refreshing }
        }
    }

    func testStaleRevisionIsRefusedAndTheDocumentIsReread() async throws {
        let f = try await makeFixture("stale")
        defer { tearDown(f) }
        let model = f.model, client = f.client
        let original = model.activeText
        model.updateActiveText(original + "% edit\n")
        try await waitUntil { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
        client.refresh()
        try await waitUntil { client.status?.undoLabels.count == 1 }
        let buffer = model.activeText

        // A command naming r1 (the revision before the edit) is refused as a
        // document conflict: nothing moves, the panel re-reads and stays usable.
        client.issue(EditHistory.Command(direction: .undo, commandID: EditHistory.Command.newID(), path: "main.tex",
                                         expectedRevision: 1, expectedSHA256: SourceDigest.sha256Hex(original)))
        try await waitUntil { client.pending == nil && client.lastFailure != nil }
        XCTAssertEqual(client.lastFailure, .documentConflict)
        XCTAssertEqual(client.phase, .idle)
        XCTAssertNil(client.lastResult)
        XCTAssertTrue(model.activeText.sameBytes(as: buffer), "a refused command never touches the buffer")
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2, "the re-read confirms r2")
        try await waitUntil { client.status?.undoLabels.count == 1 && !client.refreshing }
        XCTAssertEqual(client.status?.permanentCommandIDs, 0, "a refused command binds no permanent id")
        XCTAssertTrue(client.note?.contains("moved since you looked") == true)

        // A matching hash but wrong revision is also stale.
        client.issue(EditHistory.Command(direction: .undo, commandID: EditHistory.Command.newID(), path: "main.tex",
                                         expectedRevision: 9, expectedSHA256: SourceDigest.sha256Hex(buffer)))
        try await waitUntil { client.pending == nil }
        XCTAssertEqual(client.lastFailure, .documentConflict)
        // Stacks read at r2, then another edit lands (r3) before any refresh: the
        // rows on screen describe r2, so the move is refused locally (no request,
        // no permanent id) and the stacks are re-read.
        if client.status?.identity != nil {
            try await waitUntil { client.status?.identity?.revision == 2 && !client.refreshing }
            model.updateActiveText(buffer + "% another\n")
            try await waitUntil { model.controllerState.durable["main.tex"]?.revision == 3 && model.controllerState.inFlight == nil }
            XCTAssertEqual(client.status?.identity?.revision, 2, "no refresh happened yet")
            client.undo()
            XCTAssertNil(client.pending, "refused before sending")
            XCTAssertTrue(client.note?.contains("describe r2 but the durable document is r3") == true, client.note ?? "-")
            try await waitUntil { client.status?.identity?.revision == 3 && !client.refreshing }
            XCTAssertEqual(client.status?.undoLabels.count, 2)
            XCTAssertEqual(client.status?.permanentCommandIDs, 0)
            client.undo()
            try await waitUntil { client.pending == nil && client.lastResult?.document.revision == 4 }
            XCTAssertTrue(model.activeText.sameBytes(as: buffer))
            try await waitUntil { client.canUndo }
        } else {
            print("history stale test: helper predates 64829a0d (no identity guard)")
        }
        // Then the real undo still works from the re-read snapshot.
        try await waitUntil { client.canUndo }
        let before = try XCTUnwrap(model.controllerState.durable["main.tex"]?.revision)
        client.undo()
        try await waitUntil { client.pending == nil && client.lastResult?.document.revision == before + 1 }
        XCTAssertTrue(model.activeText.sameBytes(as: original))
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, before + 1)
    }

    func testIdenticalRetryAfterALostReplyIsIdempotent() async throws {
        let f = try await makeFixture("retry")
        defer { tearDown(f) }
        let model = f.model, client = f.client
        let original = model.activeText
        model.updateActiveText(original + "% edit\n")
        try await waitUntil { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
        client.refresh()
        try await waitUntil { client.status?.undoLabels.count == 1 }

        // Lose the reply: the waiter is dropped so the helper's answer is never
        // seen; the panel times out into `uncertain` with the command intact.
        client.replyTimeout = 0.3
        client.undo()
        let first = try XCTUnwrap(client.pending)
        XCTAssertEqual(first.attempts, 1)
        model.controllerState.awaiting.removeValue(forKey: first.requestIDs[0])
        try await waitUntil { client.isUncertain }
        XCTAssertEqual(client.phase, .uncertain(.undo))
        XCTAssertEqual(client.pending?.command, first.command, "the command id and expectation are unchanged")
        XCTAssertEqual(client.lastFailure, .uncertain("no reply within 0.3 s"))
        XCTAssertFalse(client.canUndo, "no new command while one is uncertain")

        // Retry with the SAME id: the ledger replays its receipt instead of moving again.
        client.resend()
        XCTAssertEqual(client.pending?.attempts, 2)
        XCTAssertEqual(client.phase, .sending(.undo))
        try await waitUntil { client.pending == nil && client.lastResult != nil }
        let result = try XCTUnwrap(client.lastResult)
        XCTAssertTrue(result.replayedCommand, "the first delivery was applied; the retry was answered from the permanent id")
        XCTAssertEqual(result.commandRevision, 3)
        XCTAssertEqual(result.document.revision, 3, "one move, not two")
        XCTAssertTrue(model.activeText.sameBytes(as: original))
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 3)
        try await waitUntil { client.status?.redoLabels.count == 1 && client.status?.undoLabels.count == 0 && !client.refreshing }
        XCTAssertEqual(client.status?.permanentCommandIDs, 1, "both deliveries share one permanent id")
        XCTAssertTrue(client.note?.contains("already applied earlier") == true)

        // A different id with the same (now stale) expectation is a conflict, never a second undo.
        client.issue(EditHistory.Command(direction: .undo, commandID: EditHistory.Command.newID(), path: "main.tex",
                                         expectedRevision: first.command.expectedRevision, expectedSHA256: first.command.expectedSHA256))
        try await waitUntil { client.pending == nil }
        XCTAssertEqual(client.lastFailure, .documentConflict)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 3)
        // Reusing an id for a DIFFERENT operation is a command-id conflict.
        client.issue(EditHistory.Command(direction: .redo, commandID: first.command.commandID, path: "main.tex",
                                         expectedRevision: 3, expectedSHA256: SourceDigest.sha256Hex(original)))
        try await waitUntil { client.pending == nil }
        XCTAssertEqual(client.lastFailure, .commandIDConflict)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 3)
    }

    func testHelperDetachMarksThePendingCommandUncertainAndRetryConvergesAfterReattach() async throws {
        let f = try await makeFixture("restart")
        defer { tearDown(f) }
        let model = f.model, client = f.client
        let original = model.activeText
        let edited = original + "% edit\n"
        model.updateActiveText(edited)
        try await waitUntil { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
        client.refresh()
        try await waitUntil { client.status?.undoLabels.count == 1 }

        // Undo, then the helper goes away before any reply is seen (what the
        // panel does when `controllerAttached` flips: see the file comment).
        client.undo()
        let command = try XCTUnwrap(client.pending?.command)
        model.detachController()
        client.controllerAttachmentChanged(false)
        XCTAssertEqual(client.phase, .uncertain(.undo))
        XCTAssertEqual(client.pending?.command, command, "the command survives the detach unchanged")
        XCTAssertFalse(client.canUndo)

        // Relaunch: the shell re-reads the ledger and, per its attach policy,
        // resubmits a differing buffer as a new edit.
        model.attachController(at: Self.helper!)
        try await waitUntil { model.controllerState.ready && model.controllerState.durable["main.tex"] != nil && model.controllerState.inFlight == nil && model.result?.revision == model.editorRevision }
        client.controllerAttachmentChanged(true)
        XCTAssertEqual(client.phase, .uncertain(.undo), "reattaching keeps the uncertain command for the user to retry")
        try await waitUntil { client.status != nil && !client.refreshing }

        // Retry with the same id converges either way and never applies twice.
        client.resend()
        try await waitUntil { client.pending == nil && client.lastResult != nil }
        let result = try XCTUnwrap(client.lastResult)
        let durable = try XCTUnwrap(model.controllerState.durable["main.tex"])
        XCTAssertEqual(result.document.revision, durable.revision)
        XCTAssertEqual(model.controllerState.textByDurable["main.tex"]?[durable.revision], model.activeText, "the buffer equals the durable text")
        if result.replayedCommand {
            // Applied before the detach (r3 = original); the relaunch resubmitted
            // the buffer on top (r4 = edited), so the current document is adopted.
            XCTAssertEqual(result.commandRevision, 3)
            XCTAssertEqual(durable.revision, 4)
            XCTAssertTrue(model.activeText.sameBytes(as: edited))
            print("history restart test: undo had been applied before the detach; replayed receipt adopted r4")
        } else {
            // Not applied before the detach: applied now, from r2 to r3.
            XCTAssertEqual(result.commandRevision, 3)
            XCTAssertEqual(durable.revision, 3)
            XCTAssertTrue(model.activeText.sameBytes(as: original))
            print("history restart test: undo was applied by the retry")
        }
        try await waitUntil { client.status != nil && !client.refreshing && client.status?.permanentCommandIDs == 1 }
        XCTAssertEqual(client.status?.permanentCommandIDs, 1, "the retried id is permanent across the relaunch")
        XCTAssertEqual(client.phase, .idle)
    }

    func testRetryAfterRelaunchReplaysAnUndoAppliedBeforeTheLostReply() async throws {
        let f = try await makeFixture("replay")
        defer { tearDown(f) }
        let model = f.model, client = f.client
        let original = model.activeText
        let edited = original + "% edit\n"
        model.updateActiveText(edited)
        try await waitUntil { model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil }
        client.refresh()
        try await waitUntil { client.status?.undoLabels.count == 1 }

        // The undo IS applied (r3) but its reply is lost, then the helper goes away.
        client.replyTimeout = 60
        client.undo()
        let command = try XCTUnwrap(client.pending?.command)
        model.controllerState.awaiting.removeValue(forKey: client.pending!.requestIDs[0])
        try await waitUntil { model.workerLog.contains { $0.contains("controller result") && $0.contains("history") } }
        model.detachController()
        client.controllerAttachmentChanged(false)
        XCTAssertEqual(client.phase, .uncertain(.undo))
        XCTAssertTrue(model.activeText.sameBytes(as: edited), "the buffer never saw the undo")

        // Relaunch: the ledger holds r3 (original) while the buffer holds the
        // edit; the shell's attach policy resubmits the buffer as r4.
        model.attachController(at: Self.helper!)
        try await waitUntil { model.controllerState.durable["main.tex"]?.revision == 4 && model.controllerState.inFlight == nil && model.result?.revision == model.editorRevision }
        client.controllerAttachmentChanged(true)
        try await waitUntil { client.status != nil && !client.refreshing }
        XCTAssertEqual(client.status?.undoLabels.count, 1, "the undo emptied the stack; the resubmitted buffer is the one step (original → edited) now recorded")
        XCTAssertEqual(client.status?.redoLabels.count, 0, "the resubmission cleared the redo stack")
        XCTAssertEqual(client.status?.permanentCommandIDs, 1)

        // Retry with the same id: replayed, not applied again; the panel adopts
        // the CURRENT document (r4, the buffer) and reports the replay.
        client.resend()
        XCTAssertEqual(client.pending?.command, command)
        try await waitUntil { client.pending == nil && client.lastResult != nil }
        let result = try XCTUnwrap(client.lastResult)
        XCTAssertTrue(result.replayedCommand)
        XCTAssertEqual(result.commandRevision, 3)
        XCTAssertEqual(result.document.revision, 4)
        XCTAssertTrue(model.activeText.sameBytes(as: edited))
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 4)
        XCTAssertTrue(client.note?.contains("already applied earlier; adopted current r4") == true)
        try await waitUntil { client.status?.permanentCommandIDs == 1 && !client.refreshing }
        XCTAssertEqual(client.status?.undoLabels.count, 1)
        // The user can still undo the resubmission explicitly to get back to the undone text.
        try await waitUntil { client.canUndo }
        client.undo()
        try await waitUntil { client.pending == nil && client.lastResult?.document.revision == 5 }
        XCTAssertTrue(model.activeText.sameBytes(as: original))
    }

    func testCapacityErrorsAreSurfaced() async throws {
        let f = try await makeFixture("capacity", text: "\\begin{document}\nA.\n\\end{document}\n")
        defer { tearDown(f) }
        let model = f.model, client = f.client
        let controller = try XCTUnwrap(model.controller)
        // Fill the ledger's retention (256 entries) with raw edits, each a
        // durable change, without going through the shell's coalescing.
        var revision = 1, sha = SourceDigest.sha256Hex(model.activeText)
        for i in 1...EditHistory.maxEntries {
            let text = "\\begin{document}\nA \(i).\n\\end{document}\n"
            let id = try controller.edit(path: "main.tex", expectedRevision: revision, expectedSHA256: sha, text: text)
            let reply: Result<[String: Any], ControllerError> = await withCheckedContinuation { cont in
                model.controllerState.awaiting[id] = { cont.resume(returning: $0) }
            }
            guard case .success(let payload) = reply, let doc = payload["document"] as? [String: Any] else { return XCTFail("edit \(i): \(reply)") }
            revision = doc["revision"] as! Int
            sha = doc["source_sha256"] as! String
        }
        XCTAssertEqual(revision, 257)
        client.refresh(force: true) // the shell was bypassed: its durable snapshot still says r1
        try await waitUntil { client.status?.entries == EditHistory.maxEntries }
        let full = try XCTUnwrap(client.status)
        XCTAssertTrue(full.isFull)
        XCTAssertEqual(full.usage, 1)
        XCTAssertNotNil(client.capacityWarning)
        XCTAssertTrue(client.capacityWarning!.contains("refuses new edits"))
        XCTAssertEqual(client.undoRows.first?.steps, 256, "one typing run of 256 steps")

        // The shell's own next edit is refused by the ledger (history_full):
        // the refusal is visible on the shell status and classified by the panel.
        _ = try controller.document(path: "main.tex") // re-sync the shell to r257; its buffer differs, so it resubmits and is refused
        try await waitUntil { model.controllerStatus.hasPrefix("edit refused: history_full") }
        XCTAssertEqual(client.shellCapacityRefusal, .historyFull)
        XCTAssertTrue(client.capacityWarning!.contains("The last edit was refused"))
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 257, "the refused edit did not move the ledger")

        // Undo still works when full (entries only move between stacks): the
        // buffer adopts A 255, and retention stays at the limit.
        model.updateActiveText("\\begin{document}\nA 256.\n\\end{document}\n") // make the buffer durable-equal so a move is allowed
        try await waitUntil { client.bufferIsDurable }
        client.refresh()
        try await waitUntil { client.canUndo }
        client.undo()
        try await waitUntil { client.pending == nil && client.lastResult != nil }
        XCTAssertEqual(client.lastFailure, nil)
        XCTAssertEqual(model.activeText, "\\begin{document}\nA 255.\n\\end{document}\n")
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 258)
        try await waitUntil { client.status?.redoLabels.count == 1 && !client.refreshing }
        XCTAssertEqual(client.status?.entries, EditHistory.maxEntries)
        XCTAssertTrue(client.status!.isFull)
        XCTAssertNotNil(client.capacityWarning)
    }

    private struct Timeout: Error {}

    /// Unlike the older helper tests this FAILS on a timeout instead of skipping,
    /// so a hang in the history route is never reported as an absent binary.
    private func waitUntil(timeout: TimeInterval = 20, file: StaticString = #filePath, line: UInt = #line, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout {
                XCTFail("condition not met within \(Int(timeout)) s", file: file, line: line)
                throw Timeout()
            }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }
}
