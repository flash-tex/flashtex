import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// End-to-end checks against the actual Rust edit-ledger helper
/// (`crates/edit-ledger` at afb1583, `flashtex-edit-ledger --store <dir>`).
/// Skipped unless `FLASHTEX_EDIT_LEDGER` points at a built binary. Covers:
/// apply → files durable (hash check on disk), duplicate edit_id refused,
/// missing-receipt recovery from the retained before-source, undo keeps dedup,
/// service metadata (session id / sequence) accepted, and the full shell flow
/// through the fake bridge with the real helper.
@MainActor
final class RealEditLedgerTests: XCTestCase {
    static var binary: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_EDIT_LEDGER"].map { URL(fileURLWithPath: $0) }
    }

    private func requireBinary() throws -> URL {
        guard let binary = Self.binary, FileManager.default.isExecutableFile(atPath: binary.path) else {
            throw XCTSkip("set FLASHTEX_EDIT_LEDGER to the built flashtex-edit-ledger binary")
        }
        return binary
    }

    private struct OnDisk: Decodable {
        struct Doc: Decodable { var revision: Int; var text: String; var source_sha256: String }
        struct Tx: Decodable { var confirmed: Bool; var document_before: Doc?; var document_after_sha256: String }
        var schema_version: Int
        var document: Doc
        var transactions: [String: Tx]
    }
    private func onDisk(_ store: URL) throws -> OnDisk {
        try JSONDecoder().decode(OnDisk.self, from: Data(contentsOf: store.appendingPathComponent("document.json")))
    }

    func testDurableApplyDuplicateRefusalRecoveryAndUndoDedup() async throws {
        let binary = try requireBinary()
        let store = try BridgeClientTests.tempStore().appendingPathComponent("doc")
        let client = try EditLedgerClient(executable: binary, storeDirectory: store, queue: DispatchQueue(label: "real-ledger"))
        defer { client.terminate() }
        let text = "Hello naïve FlashTeX.\n"
        let empty = try await client.status()
        XCTAssertNil(empty.document)
        XCTAssertNotNil(client.ledgerSessionId, "afb1583 replies carry a session id")
        let doc = try await client.initialize(.init(projectId: "demo", path: "main.tex", revision: 3, text: text))
        XCTAssertEqual(doc.revision, 3)

        // apply → durable on disk (hash check) before the reply is used.
        let edit = TransferV1.CaptureEdit(captureId: "c1", editId: "capture-c1", projectId: "demo", path: "main.tex", expectedRevision: 3,
                                          startByte: 13, endByte: 13, removedText: "", replacement: "\\alpha", documentBeforeSha256: doc.sourceSha256)
        let applied = try await client.apply(edit)
        XCTAssertEqual(applied.receipt, .init(captureId: "c1", editId: "capture-c1", newRevision: 4))
        let after = "Hello naïve \\alphaFlashTeX.\n"
        XCTAssertEqual(applied.document.text, after)
        var disk = try onDisk(store)
        XCTAssertEqual(disk.document.text, after)
        XCTAssertEqual(disk.document.source_sha256, SourceDigest.sha256Hex(after))
        XCTAssertEqual(disk.document.revision, 4)
        XCTAssertEqual(disk.transactions["capture-c1"]?.confirmed, false)
        XCTAssertEqual(disk.transactions["capture-c1"]?.document_before?.text, text, "before-source retained for recovery")
        XCTAssertEqual(client.lastObserved.revision, 4)
        XCTAssertEqual(client.lastObserved.sha256, SourceDigest.sha256Hex(after))

        // Identical retry → original receipt, no second insertion; changed fields → refused.
        let again = try await client.apply(edit)
        XCTAssertEqual(again.receipt, applied.receipt)
        XCTAssertEqual(again.document.text, after)
        var changed = edit; changed.replacement = "\\beta"
        do { _ = try await client.apply(changed); XCTFail() } catch let f as LineProcessFailure { XCTAssertEqual(f.code, "edit_id_conflict", f.text) }
        var sameCapture = edit; sameCapture.editId = "capture-c1-again"; sameCapture.expectedRevision = 4; sameCapture.documentBeforeSha256 = applied.document.sourceSha256
        do { _ = try await client.apply(sameCapture); XCTFail() } catch let f as LineProcessFailure { XCTAssertEqual(f.code, "capture_id_conflict", f.text) }

        // Undo (ordinary replace back to the original text) keeps the tombstone: dedup survives.
        let undone = try await client.replaceDocument(expectedRevision: 4, expectedSha256: applied.document.sourceSha256, text: text)
        XCTAssertEqual(undone.revision, 5)
        let afterUndo = try await client.apply(edit)
        XCTAssertEqual(afterUndo.receipt, applied.receipt, "same receipt after undo")
        XCTAssertEqual(afterUndo.document.text, text, "never re-inserted")
        disk = try onDisk(store)
        XCTAssertEqual(disk.document.text, text)
        XCTAssertNotNil(disk.transactions["capture-c1"])

        // Missing-receipt recovery: export → (bridge says prepared) → import → replay_receipt with the before-source.
        let export = try await client.recoveryExport()
        XCTAssertEqual(export.pendingReceipts.count, 1)
        XCTAssertEqual(export.pendingReceipts.first?.documentBefore?.text, text)
        let pendingReceipt = try XCTUnwrap(export.pendingReceipts.first)
        let pendingDocumentBefore = try XCTUnwrap(pendingReceipt.documentBefore)
        let plan = try await client.recoveryImport(snapshotToken: export.snapshotToken, observations: [.prepared(edit: edit)])
        XCTAssertEqual(plan.actions, [.replayReceipt(documentBefore: pendingDocumentBefore, receipt: applied.receipt)])
        XCTAssertEqual(plan.recovery.pendingReceipts.count, 1, "a replay plan never confirms by itself")
        // A stale token (state advanced meanwhile) is refused.
        _ = try await client.replaceDocument(expectedRevision: undone.revision, expectedSha256: undone.sourceSha256, text: text + "x")
        do { _ = try await client.recoveryImport(snapshotToken: export.snapshotToken, observations: [.prepared(edit: edit)]); XCTFail() }
        catch let f as LineProcessFailure { XCTAssertEqual(f.code, "stale_recovery_snapshot", f.text) }
        // Unavailable keeps evidence; applied (exact receipt) confirms durably and drops the snapshot.
        let fresh = try await client.recoveryExport()
        let kept = try await client.recoveryImport(snapshotToken: fresh.snapshotToken, observations: [.unavailable(captureId: "c1", reason: "bridge offline")])
        XCTAssertEqual(kept.actions, [.retryStatus(captureId: "c1", reason: "bridge offline")])
        XCTAssertEqual(kept.recovery.pendingReceipts.first?.documentBefore?.text, text)
        let confirmed = try await client.recoveryImport(snapshotToken: kept.recovery.snapshotToken, observations: [.applied(receipt: applied.receipt)])
        XCTAssertEqual(confirmed.actions, [.confirmed(receipt: applied.receipt)])
        XCTAssertEqual(confirmed.recovery.pendingReceipts, [])
        disk = try onDisk(store)
        XCTAssertEqual(disk.transactions["capture-c1"]?.confirmed, true)
        XCTAssertNil(disk.transactions["capture-c1"]?.document_before)
        // A wrong receipt is a conflict; the plain confirm op agrees.
        do { _ = try await client.confirm(.init(captureId: "c1", editId: "capture-c1", newRevision: 9)); XCTFail() }
        catch let f as LineProcessFailure { XCTAssertEqual(f.code, "receipt_conflict", f.text) }
        // Reopening the same store from a second process is refused while this one holds the lock.
        let second = try EditLedgerClient(executable: binary, storeDirectory: store, queue: DispatchQueue(label: "real-ledger-2"))
        defer { second.terminate() }
        do { _ = try await second.status(timeout: 10); XCTFail("second writer must be refused") }
        catch let f as LineProcessFailure { XCTAssertTrue(f.code == "store_in_use" || f.isTransient, f.text) }
    }

    /// The real helper killed mid-session (SIGKILL): its flock is released with the
    /// process, the relaunch reopens the same store, realigns it with the buffer
    /// (typing while it was down is not lost), and insertion works afterwards.
    func testKilledHelperIsRelaunchedAndTheStoreReopens() async throws {
        let binary = try requireBinary()
        let store = try BridgeClientTests.tempStore()
        let ledgerStore = store.appendingPathComponent("documents/doc")
        let model = ShellModel()
        model.autoCompile = false
        let attached = await model.attachBridgeAndWait(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeBridge.path],
                                                       storeDirectory: store, ledger: .init(executable: binary), discoverLedger: false, ledgerStore: ledgerStore)
        XCTAssertTrue(attached, model.captureNote ?? model.bridgeStatus)
        let bridge = try XCTUnwrap(model.bridge)
        XCTAssertTrue(bridge.ledgerUsable, bridge.ledgerStatus)
        let firstSession = bridge.ledger?.ledgerSessionId
        model.updateActiveText("Hello naïve FlashTeX.\n")
        try await ShellModelBridgeTests.waitUntil { bridge.durable?.revision == model.editorRevision }
        let pid = try XCTUnwrap(RealHelperProcess.pid(commandLineContaining: "flashtex-edit-ledger --store \(ledgerStore.path)"), "helper pid")
        XCTAssertEqual(kill(pid, SIGKILL), 0)
        try await ShellModelBridgeTests.waitUntil { !bridge.ledgerUsable }
        XCTAssertTrue(bridge.ledgerStatus.contains("edit ledger exited (9); relaunching in 0.2 s"), bridge.ledgerStatus)
        model.updateActiveText("Hello naïve FlashTeX. typed while down\n")
        try await ShellModelBridgeTests.waitUntil { bridge.relaunchCount[.ledger] == 1 && bridge.relaunching == nil }
        XCTAssertTrue(bridge.ledgerUsable, bridge.ledgerStatus)
        XCTAssertTrue(bridge.ledgerStatus.hasPrefix("relaunched: "), bridge.ledgerStatus)
        XCTAssertNotEqual(bridge.ledger?.ledgerSessionId, firstSession, "a new helper session")
        XCTAssertEqual(bridge.durable?.text, "Hello naïve FlashTeX. typed while down\n")
        XCTAssertEqual(bridge.durable?.revision, model.editorRevision)
        XCTAssertEqual(try onDisk(ledgerStore).document.text, "Hello naïve FlashTeX. typed while down\n")
        XCTAssertEqual(try onDisk(ledgerStore).document.revision, model.editorRevision)
        XCTAssertTrue(bridge.running, "the bridge was untouched")
        // Insertion on the relaunched helper.
        model.caretUTF16 = 12
        model.pinAnchorAtCaret()
        try await ShellModelBridgeTests.waitUntil { model.bridgeDestination != nil }
        let received = await model.submitCapture(image: try BridgeClientTests.fixtureCapture().image, captureId: "fixture-capture-1", instructions: "t")
        XCTAssertNotNil(received, model.captureNote ?? "")
        let maybeProposal = await model.convertCapture(captureId: "fixture-capture-1")
        let proposal = try XCTUnwrap(maybeProposal, model.captureNote ?? "")
        let outcome = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(outcome, .inserted(byteOffset: 13), model.captureNote ?? "")
        let after = "Hello naïve \\fakecapture{fixture-capture-1}FlashTeX. typed while down\n"
        model.editApplied(try XCTUnwrap(model.pendingEdit), newText: after)
        try await ShellModelBridgeTests.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
        try await ShellModelBridgeTests.waitUntil { bridge.transactions["capture-fixture-capture-1"]?.confirmed == true }
        XCTAssertEqual(try onDisk(ledgerStore).document.text, after)
        XCTAssertEqual(try onDisk(ledgerStore).transactions["capture-fixture-capture-1"]?.confirmed, true)
        model.detachBridge()
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertNil(RealHelperProcess.pid(commandLineContaining: "flashtex-edit-ledger --store \(ledgerStore.path)"), "detach leaves no helper process")
    }

    func testShellFlowWithRealHelperAndFakeBridge() async throws {
        let binary = try requireBinary()
        let store = try BridgeClientTests.tempStore()
        let docURL = store.appendingPathComponent("paper.tex")
        try "Hello FlashTeX.\n".write(to: docURL, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.autoCompile = false
        model.openTex(at: docURL)
        let attached = await model.attachBridgeAndWait(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeBridge.path],
                                                       storeDirectory: store, ledger: .init(executable: binary), discoverLedger: false)
        XCTAssertTrue(attached, model.captureNote ?? model.bridgeStatus)
        let bridge = try XCTUnwrap(model.bridge)
        XCTAssertTrue(bridge.ledgerUsable, bridge.ledgerStatus)
        XCTAssertNotNil(bridge.ledger?.ledgerSessionId)
        model.updateActiveText("Hello naïve FlashTeX.\n")
        try await ShellModelBridgeTests.waitUntil { bridge.durable?.revision == model.editorRevision }
        model.caretUTF16 = 12
        model.pinAnchorAtCaret()
        try await ShellModelBridgeTests.waitUntil { model.bridgeDestination != nil }
        let received = await model.submitCapture(image: try BridgeClientTests.fixtureCapture().image, captureId: "fixture-capture-1", instructions: "t")
        XCTAssertNotNil(received, model.captureNote ?? "")
        let maybeProposal = await model.convertCapture(captureId: "fixture-capture-1")
        let proposal = try XCTUnwrap(maybeProposal, model.captureNote ?? "")
        let outcome = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(outcome, .inserted(byteOffset: 13), model.captureNote ?? "")
        let after = "Hello naïve \\fakecapture{fixture-capture-1}FlashTeX.\n"
        XCTAssertEqual(bridge.durable?.text, after, "durable before the editor adopts it")
        let ledgerStore = EditLedgerClient.storeDirectory(under: store, documentURL: docURL)
        XCTAssertEqual(try onDisk(ledgerStore).document.text, after)
        model.editApplied(try XCTUnwrap(model.pendingEdit), newText: after)
        XCTAssertEqual(bridge.transactionTrace, ["ledger", "source", "receipt"])
        XCTAssertEqual(try String(contentsOf: docURL, encoding: .utf8), after)
        try await ShellModelBridgeTests.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
        try await ShellModelBridgeTests.waitUntil { bridge.transactions["capture-fixture-capture-1"]?.confirmed == true }
        XCTAssertEqual(try onDisk(ledgerStore).transactions["capture-fixture-capture-1"]?.confirmed, true)
        // Duplicate approval is refused by the bridge (already_applied) and would be by the helper.
        let dup = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(dup, .duplicate)
        model.detachBridge()
    }
}
