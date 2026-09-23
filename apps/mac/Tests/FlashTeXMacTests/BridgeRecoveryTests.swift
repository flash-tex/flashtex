import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Fault/recovery tests for the bridge integration (issue #2 review of 48780a8,
/// plus the edit-ledger adoption addendum): receipt only after the helper's
/// durable commit (and the .tex export when file-backed), a corrupt/unreadable
/// durable store disables application, transient status failures keep
/// evidence, bridge writes never block the main thread, a detached session
/// cannot overwrite its successor, and (section 6) a crashed bridge or helper
/// is relaunched with bounded backoff and reconciled exactly once. Runs
/// against `Fixtures/fake_bridge.py` and `Fixtures/fake_edit_ledger.py`.
@MainActor
final class BridgeRecoveryTests: XCTestCase {
    typealias T = ShellModelBridgeTests

    private func attach(_ model: ShellModel, store: URL, ledgerStore: URL? = nil, ledger: ShellModel.LedgerLaunch? = T.fakeLedger) async -> Bool {
        await model.attachBridgeAndWait(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeBridge.path],
                                        storeDirectory: store, ledger: ledger, discoverLedger: false, ledgerStore: ledgerStore)
    }

    /// Attach, pin at byte 5, submit, convert: returns the proposal.
    private func stage(_ model: ShellModel, store: URL, ledgerStore: URL? = nil, captureId: String = "fixture-capture-1") async throws -> RuntimeV1.CaptureProposal {
        let ok = await attach(model, store: store, ledgerStore: ledgerStore)
        XCTAssertTrue(ok, model.captureNote ?? model.bridgeStatus)
        model.caretUTF16 = 5
        model.pinAnchorAtCaret()
        try await T.waitUntil { model.bridgeDestination != nil }
        let received = await model.submitCapture(image: try BridgeClientTests.fixtureCapture().image, captureId: captureId, instructions: "t")
        XCTAssertNotNil(received, model.captureNote ?? "")
        let maybeProposal = await model.convertCapture(captureId: captureId)
        return try XCTUnwrap(maybeProposal, model.captureNote ?? "")
    }

    private func chmod(_ url: URL, _ mode: Int) throws {
        try FileManager.default.setAttributes([.posixPermissions: mode], ofItemAtPath: url.path)
    }

    // MARK: 1. receipt only after the durable commit (helper) and the .tex export

    func testPersistenceFailureInsertsNothingAndSendsNoReceipt() async throws {
        let store = try BridgeClientTests.tempStore()
        let ledgerStore = store.appendingPathComponent("documents/doc")
        let model = ShellModel()
        model.autoCompile = false
        let proposal = try await stage(model, store: store, ledgerStore: ledgerStore)
        let bridge = try XCTUnwrap(model.bridge)
        let before = model.activeText
        // Inject: the helper's store directory becomes read-only, so its atomic commit fails.
        try chmod(ledgerStore, 0o500)
        defer { try? chmod(ledgerStore, 0o700) }
        let outcome = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(outcome, .refused("ledger"), model.captureNote ?? "")
        XCTAssertNil(model.pendingEdit, "nothing staged for the editor")
        XCTAssertEqual(model.activeText, before, "nothing inserted")
        XCTAssertEqual(bridge.transactionTrace, [])
        XCTAssertNil(bridge.pendingTransaction)
        XCTAssertTrue(model.captureNote?.contains("nothing inserted") == true, model.captureNote ?? "")
        try await Task.sleep(nanoseconds: 200_000_000)
        let st = try await bridge.status(captureId: "fixture-capture-1")
        XCTAssertNil(st.applied, "no capture_applied may reach the bridge without a durable commit")
        XCTAssertNotNil(st.prepared, "the bridge prepared the edit; it stays reusable")
        // The helper was reopened after the poisoned handle and is usable again once the store is writable.
        XCTAssertTrue(bridge.ledgerUsable, bridge.ledgerStatus)
        let durable = try await bridge.ledger!.status()
        XCTAssertEqual(durable.document?.text, before)
        XCTAssertEqual(durable.pendingReceipts, [])

        // Repair and approve again: same prepared edit, now committed, adopted, acknowledged.
        try chmod(ledgerStore, 0o700)
        model.enqueue(proposal)
        let retry = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(retry, .inserted(byteOffset: 5), model.captureNote ?? "")
        XCTAssertEqual(bridge.transactionTrace, ["ledger"])
        let pending = try XCTUnwrap(model.pendingEdit)
        model.editApplied(pending, newText: "Hello\\fakecapture{fixture-capture-1} FlashTeX.\n")
        try await T.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
        XCTAssertEqual(bridge.transactionTrace, ["ledger", "receipt", "confirmed"])
        model.detachBridge()
    }

    func testFileBackedDocumentIsExportedBeforeTheReceipt() async throws {
        let store = try BridgeClientTests.tempStore()
        let docURL = store.appendingPathComponent("doc.tex")
        try "Hello FlashTeX.\n".write(to: docURL, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.autoCompile = false
        model.openTex(at: docURL)
        let proposal = try await stage(model, store: store)
        let bridge = try XCTUnwrap(model.bridge)
        XCTAssertEqual(bridge.ledger?.storeDirectory.path, EditLedgerClient.storeDirectory(under: store, documentURL: docURL).path, "store keyed by the file")
        let outcome = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(outcome, .inserted(byteOffset: 5), model.captureNote ?? "")
        let after = "Hello\\fakecapture{fixture-capture-1} FlashTeX.\n"
        XCTAssertEqual(try String(contentsOf: docURL, encoding: .utf8), "Hello FlashTeX.\n", "export happens after adoption, not before")
        let pending = try XCTUnwrap(model.pendingEdit)
        model.editApplied(pending, newText: after)
        // Synchronously after adoption: durable commit, then the .tex export, then the receipt was queued.
        XCTAssertEqual(bridge.transactionTrace, ["ledger", "source", "receipt"])
        XCTAssertEqual(try String(contentsOf: docURL, encoding: .utf8), after, "the .tex holds the post-edit text before the receipt")
        XCTAssertFalse(model.isDirty, "the atomic export is the saved state")
        try await T.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
        XCTAssertEqual(bridge.transactionTrace, ["ledger", "source", "receipt", "confirmed"])
        let durable = try await bridge.ledger!.status()
        XCTAssertEqual(durable.document?.text, after)
        XCTAssertEqual(durable.document?.sourceSha256, SourceDigest.sha256Hex(try String(contentsOf: docURL, encoding: .utf8)))
        model.detachBridge()
    }

    /// A bridge session opened on a project member (attached while
    /// chapter.tex was active) must never export that member's text to the
    /// entry's `documentURL`: main.tex keeps its own text and baseline.
    func testMemberApplicationNeverExportsIntoTheEntryFile() async throws {
        let store = try BridgeClientTests.tempStore()
        let mainURL = store.appendingPathComponent("main.tex")
        try "Main.\n\\input{chapter}\n".write(to: mainURL, atomically: true, encoding: .utf8)
        try "Hello FlashTeX.\n".write(to: store.appendingPathComponent("chapter.tex"), atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.autoCompile = false
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: mainURL), .opened)
        let opened = await model.project.openDocument("chapter.tex")
        XCTAssertEqual(opened, .opened(path: "chapter.tex"))
        model.project.switchDocument(to: "chapter.tex")
        let proposal = try await stage(model, store: store)
        let bridge = try XCTUnwrap(model.bridge)
        let outcome = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(outcome, .inserted(byteOffset: 5), model.captureNote ?? "")
        let pending = try XCTUnwrap(model.pendingEdit)
        let after = "Hello\\fakecapture{fixture-capture-1} FlashTeX.\n"
        model.editApplied(pending, newText: after)
        XCTAssertEqual(model.activeText, after)
        XCTAssertEqual(try String(contentsOf: mainURL, encoding: .utf8), "Main.\n\\input{chapter}\n", "member text must never land in main.tex")
        XCTAssertEqual(model.savedText, "Main.\n\\input{chapter}\n", "the entry's saved baseline is untouched")
        try await T.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
        XCTAssertTrue(model.project.isDirty("chapter.tex"), "the member reaches disk through its own save/autosave")
        model.detachBridge()
    }

    func testExportFailureWithholdsReceiptWhileTheDurableCommitStands() async throws {
        let store = try BridgeClientTests.tempStore()
        let model = ShellModel()
        model.autoCompile = false
        let proposal = try await stage(model, store: store)
        let bridge = try XCTUnwrap(model.bridge)
        model.documentURL = store.appendingPathComponent("missing-dir/doc.tex") // atomic write cannot succeed here
        model.savedText = model.activeText
        let outcome = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(outcome, .inserted(byteOffset: 5), model.captureNote ?? "")
        let after = "Hello\\fakecapture{fixture-capture-1} FlashTeX.\n"
        model.editApplied(try XCTUnwrap(model.pendingEdit), newText: after)
        XCTAssertEqual(bridge.transactionTrace, ["ledger"], "durable commit stands; export failed; no receipt")
        XCTAssertTrue(bridge.pendingTransaction?.lastFailure?.contains("export") == true, bridge.pendingTransaction?.lastFailure ?? "")
        XCTAssertTrue(model.bridgeStatus.contains("receipt withheld"), model.bridgeStatus)
        try await Task.sleep(nanoseconds: 150_000_000)
        let st = try await bridge.status(captureId: "fixture-capture-1")
        XCTAssertNil(st.applied)
        let held = try await bridge.ledger!.status()
        XCTAssertEqual(held.pendingReceipts.count, 1, "helper retains the transaction and its snapshot")
        // Edits made meanwhile are held back from the bridge, not sent ahead of the receipt.
        model.updateActiveText(after + "typed\n")
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertFalse(bridge.log.contains { $0.contains("refused") }, bridge.log.joined(separator: "\n"))
        // Saving somewhere writable records the export and releases the receipt; the deferred edit follows.
        model.documentURL = store.appendingPathComponent("doc.tex")
        XCTAssertTrue(model.saveTex())
        try await T.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
        XCTAssertEqual(bridge.transactionTrace, ["ledger", "source", "receipt", "confirmed"])
        try await Task.sleep(nanoseconds: 150_000_000)
        XCTAssertFalse(bridge.log.contains { $0.contains("refused") }, "deferred document_edit applied after the receipt: \(bridge.log)")
        model.caretUTF16 = 0
        model.pinAnchorAtCaret()
        try await T.waitUntil { model.bridgeDestination != nil }
        XCTAssertEqual(model.bridgeDestination?.pinnedRevision, model.editorRevision)
        model.detachBridge()
    }

    // MARK: 2. corrupt / unreadable durable store fails closed

    func testCorruptOrUnreadableDurableStoreDisablesApplication() async throws {
        let store = try BridgeClientTests.tempStore()
        let ledgerStore = store.appendingPathComponent("documents/doc")
        try FileManager.default.createDirectory(at: ledgerStore, withIntermediateDirectories: true)
        let record = ledgerStore.appendingPathComponent("document.json")
        try Data("{not json".utf8).write(to: record)

        let model = ShellModel()
        model.autoCompile = false
        let proposal = try await stage(model, store: store, ledgerStore: ledgerStore)
        let bridge = try XCTUnwrap(model.bridge)
        XCTAssertFalse(bridge.ledgerUsable)
        XCTAssertTrue(bridge.ledgerError?.contains("unavailable") == true, bridge.ledgerError ?? "")
        XCTAssertTrue(model.bridgeStatus.contains("edit ledger unavailable"), model.bridgeStatus)
        XCTAssertTrue(bridge.reconciliationIncomplete)
        let outcome = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(outcome, .refused("ledger"))
        XCTAssertNil(model.pendingEdit, "nothing is applied while the durable store is unusable")
        XCTAssertTrue(model.captureNote?.contains("Edit ledger unusable") == true, model.captureNote ?? "")
        XCTAssertEqual(try Data(contentsOf: record), Data("{not json".utf8), "a broken store is never overwritten")
        model.detachBridge()

        // Unreadable (permissions) is the same failure; a missing record is a fresh store.
        try Data("{}".utf8).write(to: record)
        try chmod(record, 0o000)
        defer { try? chmod(record, 0o600) }
        let model2 = ShellModel()
        model2.autoCompile = false
        let ok2 = await attach(model2, store: store, ledgerStore: ledgerStore)
        XCTAssertTrue(ok2)
        XCTAssertFalse(model2.bridge!.ledgerUsable)
        model2.detachBridge()
        try chmod(record, 0o600)
        try FileManager.default.removeItem(at: record)
        let model3 = ShellModel()
        model3.autoCompile = false
        let ok3 = await attach(model3, store: store, ledgerStore: ledgerStore)
        XCTAssertTrue(ok3)
        XCTAssertTrue(model3.bridge!.ledgerUsable, model3.bridge!.ledgerStatus)
        XCTAssertEqual(model3.bridge!.durable?.text, model3.activeText)
        model3.detachBridge()
    }

    // MARK: 3. transient status failures keep evidence

    /// Pre-populates a helper store with applied-but-unconfirmed edits (each
    /// inserting at byte 5) and returns the resulting document text.
    private func seedPendingTransactions(store: URL, captureIds: [String], text: String, undoAfterwards: Bool = false) async throws -> String {
        let client = try EditLedgerClient(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeEditLedger.path],
                                          storeDirectory: store, queue: DispatchQueue(label: "seed"))
        var doc = try await client.initialize(.init(projectId: "demo", path: "main.tex", revision: 1, text: text))
        for id in captureIds {
            let edit = TransferV1.CaptureEdit(captureId: id, editId: "capture-\(id)", projectId: "demo", path: "main.tex", expectedRevision: doc.revision,
                                              startByte: 5, endByte: 5, removedText: "", replacement: "[\(id)]", documentBeforeSha256: doc.sourceSha256)
            doc = try await client.apply(edit).document
        }
        if undoAfterwards { doc = try await client.replaceDocument(expectedRevision: doc.revision, expectedSha256: doc.sourceSha256, text: text) }
        client.terminate()
        try await Task.sleep(nanoseconds: 100_000_000) // release the store lock
        return doc.text
    }

    func testTransientStatusFailuresRetainTransactionsAndEvidence() async throws {
        let store = try BridgeClientTests.tempStore()
        let ledgerStore = store.appendingPathComponent("documents/doc")
        let model = ShellModel()
        model.autoCompile = false
        model.bridgeStatusTimeout = 1
        let text = model.activeText
        let durableText = try await seedPendingTransactions(store: ledgerStore, captureIds: ["err-storage_error-1", "garbage-1", "err-capture_missing-1"], text: text)
        let ok = await attach(model, store: store, ledgerStore: ledgerStore)
        XCTAssertTrue(ok, model.captureNote ?? model.bridgeStatus)
        let bridge = try XCTUnwrap(model.bridge)
        // The durable document (with the three insertions) is authoritative: adopted as one undoable edit.
        let adoption = try XCTUnwrap(model.pendingEdit)
        XCTAssertEqual(adoption.text, durableText)
        XCTAssertEqual(adoption.nsRange, NSRange(location: 0, length: (text as NSString).length))
        XCTAssertGreaterThanOrEqual(model.editorRevision, 4)
        // Bridge-side error that is not a missing/conflicting record, and a garbage reply (no answer
        // within the deadline): transactions untouched, evidence kept, retry offered.
        XCTAssertTrue(bridge.reconciliationIncomplete)
        XCTAssertTrue(model.captureNote?.contains("Reconciliation incomplete") == true, model.captureNote ?? "")
        let st = try await bridge.ledger!.status()
        XCTAssertEqual(st.pendingReceipts.count, 3, "no transaction was confirmed or dropped")
        for tx in st.pendingReceipts { XCTAssertNotNil(tx.documentBefore, "snapshot retained for \(tx.edit.captureId)") }
        XCTAssertEqual(bridge.needsReconciliation.keys.sorted(), ["capture-err-capture_missing-1"])
        XCTAssertTrue(bridge.needsReconciliation["capture-err-capture_missing-1"]?.contains("evidence kept") == true)
        XCTAssertEqual(bridge.transactions.values.filter { !$0.confirmed }.count, 3)
        // Adopting the durable text keeps store and editor aligned (no divergence).
        model.editApplied(adoption, newText: durableText)
        try await T.waitUntil { bridge.durable?.revision == model.editorRevision }
        XCTAssertNil(bridge.ledgerError, bridge.ledgerError ?? "")
        // Explicit resolution is the only path that drops evidence (helper confirm).
        try await bridge.resolveReconciliation(editId: "capture-err-capture_missing-1", note: "operator verified the insertion is in the document")
        let resolved = try await bridge.ledger!.status()
        XCTAssertEqual(resolved.pendingReceipts.map(\.edit.captureId).sorted(), ["err-storage_error-1", "garbage-1"])
        XCTAssertTrue(bridge.needsReconciliation.isEmpty)
        model.detachBridge()
    }

    func testMissingReceiptIsReplayedFromTheRetainedSnapshot() async throws {
        let store = try BridgeClientTests.tempStore()
        let text = "Hello FlashTeX.\n"
        // Bring the fake bridge to "prepared" for a capture (its journal is in-memory, so the
        // reconciliation below runs against this live session rather than a process restart).
        let seed = ShellModel()
        seed.autoCompile = false
        let seeded = await attach(seed, store: store, ledger: nil)
        XCTAssertTrue(seeded)
        seed.caretUTF16 = 5
        seed.pinAnchorAtCaret()
        try await T.waitUntil { seed.bridgeDestination != nil }
        _ = await seed.submitCapture(image: try BridgeClientTests.fixtureCapture().image, captureId: "fixture-capture-1", instructions: "t")
        _ = await seed.convertCapture(captureId: "fixture-capture-1")
        let bridge = seed.bridge!
        let prepared = try await bridge.prepare(captureId: "fixture-capture-1", expectedRevision: seed.editorRevision)
        XCTAssertEqual(prepared.replacement, "\\fakecapture{fixture-capture-1}")

        // A helper store holding a *different* applied edit for the same capture: held, never replayed blindly.
        let ledgerStore = store.appendingPathComponent("documents/doc")
        _ = try await seedPendingTransactions(store: ledgerStore, captureIds: ["fixture-capture-1"], text: text, undoAfterwards: true)
        let open1 = await bridge.openLedger(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeEditLedger.path],
                                            store: ledgerStore, path: "main.tex", currentText: text, currentRevision: seed.editorRevision)
        guard case .aligned = open1 else { return XCTFail("\(open1)") }
        let (held, _) = await bridge.reconcile(path: "main.tex", currentSource: { (text, seed.editorRevision) }, statusTimeout: 2)
        guard case .needsReconciliation(let id, let why)? = held.first else { return XCTFail("\(held)") }
        XCTAssertEqual(id, "capture-fixture-capture-1")
        XCTAssertTrue(why.contains("no matching prepared edit"), why)
        let kept = try await bridge.ledger!.status()
        XCTAssertEqual(kept.pendingReceipts.count, 1, "evidence kept")

        // The genuine case: a helper transaction whose edit matches the bridge's prepared edit exactly
        // (applied durably, receipt lost). Reconciliation reopens the pre-edit snapshot and replays it.
        let ledgerStore2 = store.appendingPathComponent("documents/doc2")
        let client = try EditLedgerClient(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeEditLedger.path],
                                          storeDirectory: ledgerStore2, queue: DispatchQueue(label: "seed2"))
        _ = try await client.initialize(.init(projectId: "demo", path: "main.tex", revision: prepared.expectedRevision, text: text))
        let applied = try await client.apply(prepared)
        let afterText = applied.document.text
        client.terminate()
        try await Task.sleep(nanoseconds: 100_000_000)
        let open2 = await bridge.openLedger(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeEditLedger.path],
                                            store: ledgerStore2, path: "main.tex", currentText: afterText, currentRevision: applied.receipt.newRevision)
        guard case .aligned = open2 else { return XCTFail("\(open2)") }
        let (replay, minimum) = await bridge.reconcile(path: "main.tex", currentSource: { (afterText, applied.receipt.newRevision) }, statusTimeout: 2)
        XCTAssertEqual(replay, [.replayedReceipt(editId: prepared.editId, newRevision: applied.receipt.newRevision)])
        XCTAssertEqual(minimum, applied.receipt.newRevision + 1)
        let st = try await bridge.status(captureId: "fixture-capture-1")
        XCTAssertEqual(st.applied?.editId, prepared.editId, "the bridge now holds the replayed receipt")
        try await T.waitUntil { bridge.transactions[prepared.editId]?.confirmed == true }
        let dropped = try await bridge.ledger!.status()
        XCTAssertEqual(dropped.pendingReceipts, [], "helper dropped the snapshot after the exact acknowledgement")
        // Replaying again is a no-op.
        let (again, _) = await bridge.reconcile(path: "main.tex", currentSource: { (afterText, applied.receipt.newRevision + 1) }, statusTimeout: 2)
        XCTAssertEqual(again, [])
        seed.detachBridge()
    }

    // MARK: 4. writes never block the main thread

    func testStalledBridgeDoesNotBlockMainThreadAndBoundsQueuedBytes() async throws {
        let store = try BridgeClientTests.tempStore()
        let model = ShellModel()
        model.autoCompile = false
        let ok = await attach(model, store: store)
        XCTAssertTrue(ok)
        let bridge = try XCTUnwrap(model.bridge)
        model.caretUTF16 = 5
        model.pinAnchorAtCaret()
        try await T.waitUntil { model.bridgeDestination != nil }
        let image = try BridgeClientTests.fixtureCapture().image
        // Main-thread heartbeat: must keep ticking while the bridge stops reading stdin.
        var ticks = 0
        var maxGap: TimeInterval = 0
        var lastTick = Date()
        let heartbeat = DispatchSource.makeTimerSource(queue: .main)
        heartbeat.schedule(deadline: .now(), repeating: .milliseconds(25))
        heartbeat.setEventHandler { ticks += 1; let now = Date(); maxGap = max(maxGap, now.timeIntervalSince(lastTick)); lastTick = now }
        heartbeat.resume()
        defer { heartbeat.cancel() }

        // Requests are issued directly on the main thread so the measurement is of send() itself.
        let destination = try XCTUnwrap(model.bridgeDestination)
        func submit(_ id: String, _ img: RuntimeV1.CaptureImage, _ instructions: String) -> RuntimeV1.CaptureSubmit {
            .init(captureId: id, destinationId: destination.destinationId, baseRevision: destination.pinnedRevision, image: img, instructions: instructions)
        }
        var stalledReply: Result<TransferV1.CaptureReceived, BridgeClient.Failure>?
        var largeReply: Result<TransferV1.CaptureReceived, BridgeClient.Failure>?
        // 1. A "slow conversion": the fake bridge sleeps 2 s before replying and reads nothing meanwhile.
        bridge.client.send(.captureSubmit, submit("stall-1", image, "%stall:2"), as: TransferV1.CaptureReceived.self) { stalledReply = $0 }
        try await Task.sleep(nanoseconds: 100_000_000)
        // 2. A multi-MB capture behind it: fills the pipe; the write blocks the I/O queue only.
        let big = RuntimeV1.CaptureImage(mimeType: "image/png", dataBase64: Data(repeating: 0x41, count: 3 * 1024 * 1024).base64EncodedString())
        let start = Date()
        bridge.client.send(.captureSubmit, submit("large-1", big, "t"), as: TransferV1.CaptureReceived.self) { largeReply = $0 }
        let sendDuration = Date().timeIntervalSince(start)
        XCTAssertLessThan(sendDuration, 0.2, "send() returned to the main thread immediately (took \(sendDuration) s)")
        XCTAssertGreaterThan(bridge.client.pendingWriteBytes, 3 * 1024 * 1024, "the large request is queued behind the stalled reader")
        try await Task.sleep(nanoseconds: 800_000_000)
        XCTAssertGreaterThan(bridge.client.pendingWriteBytes, 0, "still queued: the bridge has not resumed reading")
        // The tick COUNT is a proxy for "the main thread kept running": it
        // asks a 25 ms timer to have fired 20 times inside ~0.9 s of sleeps,
        // which needs near-perfect timer delivery. CI saw 15 and 16 -- while
        // maxGap, the assertion that actually detects a blocked main thread,
        // passed. So the proxy is reported on a shared runner and the
        // invariant stays gating everywhere (TimingBudget.swift).
        TimingBudget.assertAtLeast(ticks, 20, "main-thread heartbeat ticks during the stall")
        XCTAssertLessThan(maxGap, 0.5, "a blocked main thread shows as a multi-second heartbeat gap (max gap \(maxGap) s)")
        // Wall-clock over two `Task.sleep`s that nominally total 0.9 s: CI
        // took 1.693 s simply by being slower, which says nothing about the
        // main actor. maxGap above is what proves it was never held.
        TimingBudget.assertWithin(Date().timeIntervalSince(start), 1.5, unit: "s",
                                  "elapsed while the bridge stalled")
        XCTAssertNil(bridge.pendingTransaction)
        // 3. Backpressure: beyond the bound, requests are refused immediately with a clear failure.
        let filler = String(repeating: "x", count: 11 * 1024 * 1024)
        var refused: BridgeClient.Failure?
        let refusal = expectation(description: "backpressure")
        for i in 0..<3 {
            bridge.client.send(.documentOpen, TransferV1.DocumentOpen(projectId: "demo", path: "big\(i).tex", revision: 1, text: filler), as: TransferV1.Empty.self) { result in
                if case .failure(let f) = result, case .backpressure = f, refused == nil { refused = f; refusal.fulfill() }
            }
        }
        await fulfillment(of: [refusal], timeout: 2)
        XCTAssertTrue(refused?.text.contains("not reading") == true, refused?.text ?? "")
        // Same proxy as above, measured again after backpressure. The
        // refusal itself -- the invariant -- is asserted on the line above
        // and gates everywhere.
        TimingBudget.assertAtLeast(ticks, 20, "main-thread heartbeat ticks through backpressure")
        // The stalled and the large capture both complete once the bridge resumes reading.
        try await T.waitUntil(timeout: 30) { stalledReply != nil && largeReply != nil }
        XCTAssertEqual(try stalledReply?.get().durable, true)
        XCTAssertEqual(try largeReply?.get().durable, true)
        XCTAssertGreaterThan(Date().timeIntervalSince(start), 1.0, "the large request really waited for the stall")
        try await T.waitUntil(timeout: 30) { bridge.client.pendingWriteBytes == 0 }
        model.detachBridge()
    }

    // MARK: 5. detached sessions cannot overwrite their successor

    func testDetachAndReattachIgnoresOldSessionEvents() async throws {
        let model = ShellModel()
        model.autoCompile = false
        model.bridgeStatusTimeout = 3
        let storeA = try BridgeClientTests.tempStore(), storeB = try BridgeClientTests.tempStore()
        let text = model.activeText
        // A's attach is still awaiting a slow capture_status (seeded pending transaction) when it is detached.
        let ledgerA = storeA.appendingPathComponent("documents/doc")
        _ = try await seedPendingTransactions(store: ledgerA, captureIds: ["stall-1-a"], text: text, undoAfterwards: true)
        let attachA = Task { await self.attach(model, store: storeA, ledgerStore: ledgerA) }
        try await Task.sleep(nanoseconds: 300_000_000)
        let sessionA = try XCTUnwrap(model.bridge)
        model.detachBridge()
        XCTAssertFalse(model.bridgeAttached)
        let okB = await attach(model, store: storeB)
        XCTAssertTrue(okB, model.captureNote ?? model.bridgeStatus)
        let sessionB = try XCTUnwrap(model.bridge)
        XCTAssertFalse(sessionA === sessionB)
        let resultA = await attachA.value
        XCTAssertFalse(resultA, "the detached attach must report failure, not open on top of B")
        // A's queued exit/termination events and its finished reconcile land now; B's state must survive them.
        try await Task.sleep(nanoseconds: 1_300_000_000)
        XCTAssertFalse(sessionA.running)
        XCTAssertTrue(sessionB.running)
        // Deterministically fire a late callback from the old session (its onChange runs synchronously).
        sessionA.terminate()
        XCTAssertEqual(sessionA.status, "bridge detached")
        XCTAssertNotEqual(model.bridgeStatus, sessionA.status, "a detached session's callback must not reach the model")
        XCTAssertTrue(model.bridgeStatus.contains("open at revision"), model.bridgeStatus)
        XCTAssertFalse(model.bridgeStatus.contains("exited") || model.bridgeStatus.contains("detached"), model.bridgeStatus)
        XCTAssertEqual(model.bridge?.storeDirectory, storeB)
        // B is fully usable.
        model.caretUTF16 = 5
        model.pinAnchorAtCaret()
        try await T.waitUntil { model.bridgeDestination != nil }
        XCTAssertEqual(model.bridgeDestination?.pinnedRevision, model.editorRevision)
        // Immediate detach/reattach without an await in between.
        model.detachBridge()
        let storeC = try BridgeClientTests.tempStore()
        let okC = await attach(model, store: storeC)
        XCTAssertTrue(okC)
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertTrue(model.bridgeAttached)
        XCTAssertTrue(model.bridgeStatus.contains("open at revision"), model.bridgeStatus)
        model.detachBridge()
    }

    // MARK: edits while reconciliation is awaiting the bridge

    func testEditsDuringReconciliationAreCarriedByTheInitialOpen() async throws {
        let store = try BridgeClientTests.tempStore()
        let ledgerStore = store.appendingPathComponent("documents/doc")
        let model = ShellModel()
        model.autoCompile = false
        model.bridgeStatusTimeout = 3
        let text = model.activeText
        _ = try await seedPendingTransactions(store: ledgerStore, captureIds: ["stall-1-e"], text: text, undoAfterwards: true)
        let attachTask = Task { await self.attach(model, store: store, ledgerStore: ledgerStore) }
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertNotNil(model.bridge)
        model.updateActiveText("edited while reconciling\n")
        model.updateActiveText("edited twice while reconciling\n")
        let revisionDuring = model.editorRevision
        let attached = await attachTask.value
        XCTAssertTrue(attached, model.captureNote ?? model.bridgeStatus)
        let bridge = try XCTUnwrap(model.bridge)
        // No document_edit went out before document_open (that would be document_missing → refused).
        XCTAssertFalse(bridge.log.contains { $0.contains("refused") }, bridge.log.joined(separator: "\n"))
        XCTAssertEqual(bridge.shadow["main.tex"]?.text, "edited twice while reconciling\n")
        XCTAssertEqual(bridge.shadow["main.tex"]?.revision, revisionDuring)
        // The durable document followed the edits (ordinary replaces); the pending transaction was
        // held (fake bridge has no record → needsReconciliation) with its evidence intact.
        try await T.waitUntil { bridge.durable?.revision == revisionDuring }
        XCTAssertEqual(bridge.durable?.text, "edited twice while reconciling\n")
        XCTAssertNil(bridge.ledgerError, bridge.ledgerError ?? "")
        XCTAssertEqual(bridge.needsReconciliation.keys.sorted(), ["capture-stall-1-e"])
        let st = try await bridge.ledger!.status()
        XCTAssertEqual(st.pendingReceipts.count, 1)
        // The bridge holds the edited text: a pin at the current revision and range succeeds.
        model.caretUTF16 = 6
        model.pinAnchorAtCaret()
        try await T.waitUntil { model.bridgeDestination != nil }
        XCTAssertEqual(model.bridgeDestination?.pinnedRevision, revisionDuring)
        XCTAssertEqual(model.bridgeDestination?.binding.sourceSha256, SourceDigest.sha256Hex("edited twice while reconciling\n"))
        model.detachBridge()
    }

    // MARK: 6. automatic relaunch of the bridge and edit-ledger helpers

    private struct LedgerCrashRequest: Encodable { var id: String; var operation = "crash" }

    /// Waits for `child`'s n-th automatic relaunch and its reconciliation to finish.
    private func waitForRelaunch(_ bridge: BridgeSession, _ child: BridgeSession.Child, count: Int, timeout: TimeInterval = 10) async throws {
        try await T.waitUntil(timeout: timeout) { bridge.relaunchCount[child] == count && bridge.relaunching == nil }
    }

    private func journaledCaptureIDs(store: URL) throws -> [String] {
        let data = try Data(contentsOf: store.appendingPathComponent("fake_journal.json"))
        return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any]).keys.sorted()
    }

    func testBridgeCrashIsRelaunchedAndStateReconciled() async throws {
        let store = try BridgeClientTests.tempStore()
        let model = ShellModel()
        model.autoCompile = false
        var notes: [(BridgeSession.Child, String)] = []
        let proposal = try await stage(model, store: store)
        let bridge = try XCTUnwrap(model.bridge)
        bridge.onRelaunched = { notes.append(($0, $1)) }
        // One full insertion, then a second capture pinned and received but not yet converted.
        let awaited1 = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(awaited1, .inserted(byteOffset: 5))
        model.editApplied(try XCTUnwrap(model.pendingEdit), newText: "Hello\\fakecapture{fixture-capture-1} FlashTeX.\n")
        try await T.waitUntil { model.bridgeCaptures.first?.state == .confirmed }
        model.caretUTF16 = 0
        model.pinAnchorAtCaret()
        try await T.waitUntil { model.bridgeDestination != nil }
        let pinned = try XCTUnwrap(model.bridgeDestination)
        let r2 = await model.submitCapture(image: try BridgeClientTests.fixtureCapture().image, captureId: "fixture-capture-2", instructions: "t")
        XCTAssertNotNil(r2, model.captureNote ?? "")
        let revisionBefore = model.editorRevision

        // Abnormal exit (status 3) mid-request: the request fails, the session reports the exit
        // with the relaunch delay, and relaunches the same executable/arguments/store.
        do { _ = try await bridge.status(captureId: "crash-status-1"); XCTFail("the fake bridge must have exited") }
        catch let f as BridgeClient.Failure { XCTAssertTrue(f.isTransient, f.text) }
        try await T.waitUntil { !bridge.running }
        XCTAssertTrue(bridge.status.contains("bridge exited (3); relaunching in 0.2 s"), bridge.status)
        XCTAssertTrue(model.bridgeStatus.contains("relaunching"), model.bridgeStatus)
        try await waitForRelaunch(bridge, .bridge, count: 1)
        XCTAssertTrue(bridge.running)
        XCTAssertTrue(model.bridgeAttached)
        XCTAssertTrue(bridge.client.isRunning)
        XCTAssertEqual(bridge.client.storeDirectory, store, "same store")
        XCTAssertTrue(bridge.status.contains("relaunched 1×") && bridge.status.contains("open at revision \(revisionBefore)"), bridge.status)
        XCTAssertEqual(model.editorRevision, revisionBefore, "no phantom revision")
        // The pinned destination was restored identically (document still at the pinned revision).
        XCTAssertEqual(model.bridgeDestination, pinned)
        XCTAssertEqual(notes.count, 1)
        XCTAssertEqual(notes.first?.0, .bridge)
        XCTAssertTrue(notes.first?.1.contains("pinned destination \(pinned.destinationId) restored") == true, notes.first?.1 ?? "")
        XCTAssertFalse(bridge.reconciliationIncomplete)
        XCTAssertNil(bridge.ledgerError)
        // The confirmed transaction stays confirmed (nothing re-inserted); the ledger holds no pending receipt.
        let awaited2 = try await bridge.ledger!.status().pendingReceipts
        XCTAssertEqual(awaited2, [])
        XCTAssertEqual(model.activeText, "Hello\\fakecapture{fixture-capture-1} FlashTeX.\n")
        // The earlier capture is still in the (durable) journal; the pending one converts and inserts
        // against the restored destination, exactly once.
        let st1 = try await bridge.status(captureId: "fixture-capture-1")
        XCTAssertEqual(st1.applied?.editId, "capture-fixture-capture-1")
        let maybe_p2 = await model.convertCapture(captureId: "fixture-capture-2")
        let p2 = try XCTUnwrap(maybe_p2, model.captureNote ?? "")
        let awaited4 = await model.approveBridgeProposal(p2, latex: p2.latex)
        XCTAssertEqual(awaited4, .inserted(byteOffset: 0), model.captureNote ?? "")
        let after2 = "\\fakecapture{fixture-capture-2}Hello\\fakecapture{fixture-capture-1} FlashTeX.\n"
        model.editApplied(try XCTUnwrap(model.pendingEdit), newText: after2)
        try await T.waitUntil { model.bridgeCaptures.first { $0.captureId == "fixture-capture-2" }?.state == .confirmed }
        XCTAssertEqual(model.activeText.components(separatedBy: "\\fakecapture{fixture-capture-2}").count, 2)
        // Typing after the relaunch still streams to both children.
        model.updateActiveText(after2 + "more\n")
        try await T.waitUntil { bridge.durable?.revision == model.editorRevision }
        model.caretUTF16 = 0
        model.pinAnchorAtCaret()
        try await T.waitUntil { model.bridgeDestination != nil }
        XCTAssertEqual(model.bridgeDestination?.pinnedRevision, model.editorRevision)
        XCTAssertFalse(bridge.log.contains { $0.contains("refused") }, bridge.log.joined(separator: "\n"))
        model.detachBridge()
    }

    func testDestinationLostWhenSourceMovedPastThePinIsReported() async throws {
        let store = try BridgeClientTests.tempStore()
        let model = ShellModel()
        model.autoCompile = false
        var notes: [String] = []
        let ok = await attach(model, store: store)
        XCTAssertTrue(ok)
        let bridge = try XCTUnwrap(model.bridge)
        bridge.onRelaunched = { notes.append($1) }
        model.caretUTF16 = 5
        model.pinAnchorAtCaret()
        try await T.waitUntil { model.bridgeDestination != nil }
        model.updateActiveText("Hello FlashTeX. typed\n") // the pin's revision is now in the past
        try await T.waitUntil { bridge.durable?.revision == model.editorRevision }
        _ = try? await bridge.status(captureId: "crash-status-1")
        try await waitForRelaunch(bridge, .bridge, count: 1)
        XCTAssertNil(model.bridgeDestination, "a destination that cannot be restored identically is dropped")
        XCTAssertTrue(notes.first?.contains("was lost with the bridge") == true, notes.first ?? "")
        XCTAssertTrue(bridge.status.contains("open at revision \(model.editorRevision)"), bridge.status)
        model.detachBridge()
    }

    func testReceiptInFlightAtBridgeCrashIsSettledFromTheLedgerExactlyOnce() async throws {
        for (captureId, expected) in [("crash-applied-once-1", "confirmed by the bridge"), ("crash-applied-before-once-1", "replayed receipt")] {
            let store = try BridgeClientTests.tempStore()
            let model = ShellModel()
            model.autoCompile = false
            var notes: [String] = []
            let proposal = try await stage(model, store: store, captureId: captureId)
            let bridge = try XCTUnwrap(model.bridge)
            bridge.onRelaunched = { notes.append($1) }
            let awaited5 = await model.approveBridgeProposal(proposal, latex: proposal.latex)
            XCTAssertEqual(awaited5, .inserted(byteOffset: 5), model.captureNote ?? "")
            let after = "Hello\\fakecapture{\(captureId)} FlashTeX.\n"
            // Adoption sends capture_applied; the fake bridge exits while it is in flight.
            model.editApplied(try XCTUnwrap(model.pendingEdit), newText: after)
            XCTAssertEqual(bridge.transactionTrace, ["ledger", "receipt"])
            try await T.waitUntil { !bridge.running }
            try await waitForRelaunch(bridge, .bridge, count: 1)
            // Reported uncertain at the relaunch, then settled through the ledger-guarded reconciliation.
            XCTAssertTrue(bridge.log.contains { $0.contains("receipt for capture-\(captureId) was") && $0.contains("when the bridge exited") }, bridge.log.joined(separator: "\n"))
            XCTAssertTrue(notes.first?.contains(expected) == true, "\(captureId): \(notes.first ?? "")")
            let capture = try XCTUnwrap(bridge.capture(captureId))
            XCTAssertEqual(capture.state, .confirmed, "\(captureId): \(capture.note)")
            XCTAssertNil(bridge.pendingTransaction)
            XCTAssertEqual(bridge.transactions["capture-\(captureId)"]?.confirmed, true)
            let awaited6 = try await bridge.ledger!.status().pendingReceipts
            XCTAssertEqual(awaited6, [], captureId)
            XCTAssertEqual(model.activeText, after, "inserted exactly once")
            XCTAssertEqual(bridge.durable?.text, after)
            let st = try await bridge.status(captureId: captureId)
            // With the parent's onRevisionFloor hook the reconciled ledger may raise the
            // editor revision past the revision the bridge recorded at application; the
            // applied revision is never ahead of the editor.
            XCTAssertNotNil(st.applied?.newRevision, captureId)
            XCTAssertLessThanOrEqual(st.applied?.newRevision ?? Int.max, model.editorRevision, captureId)
            // The bridge snapshot is current again: a pin at the live revision succeeds.
            model.caretUTF16 = 0
            model.pinAnchorAtCaret()
            try await T.waitUntil { model.bridgeDestination != nil }
            XCTAssertEqual(model.bridgeDestination?.pinnedRevision, model.editorRevision)
            model.detachBridge()
        }
    }

    func testDurableEditAwaitingAdoptionSurvivesABridgeRelaunch() async throws {
        let store = try BridgeClientTests.tempStore()
        let model = ShellModel()
        model.autoCompile = false
        var notes: [String] = []
        let proposal = try await stage(model, store: store)
        let bridge = try XCTUnwrap(model.bridge)
        bridge.onRelaunched = { notes.append($1) }
        let awaited7 = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(awaited7, .inserted(byteOffset: 5))
        let pending = try XCTUnwrap(model.pendingEdit)
        XCTAssertNotNil(bridge.expectedApplication)
        // The bridge dies before the editor adopts the durable edit.
        _ = try? await bridge.status(captureId: "crash-status-1")
        try await waitForRelaunch(bridge, .bridge, count: 1)
        XCTAssertNotNil(bridge.expectedApplication, "the adoption is still expected")
        XCTAssertNotNil(bridge.pendingTransaction)
        XCTAssertTrue(notes.first?.contains("awaits adoption") == true, notes.first ?? "")
        // Adoption now: recognized (no document_edit), receipt accepted by the relaunched bridge.
        let after = "Hello\\fakecapture{fixture-capture-1} FlashTeX.\n"
        model.editApplied(pending, newText: after)
        try await T.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
        XCTAssertEqual(bridge.transactionTrace, ["ledger", "receipt", "confirmed"])
        XCTAssertEqual(model.activeText, after)
        XCTAssertFalse(bridge.log.contains { $0.contains("refused") }, bridge.log.joined(separator: "\n"))
        model.detachBridge()
    }

    func testLedgerCrashDuringApplyInsertsExactlyOnce() async throws {
        // Crash after the durable commit (reply lost): the relaunched helper reports the
        // transaction; the edit is adopted once. Crash before the commit: nothing inserted,
        // the approval is retried and inserts once.
        for captureId in ["crash-after-commit-1", "crash-before-commit-1"] {
            let store = try BridgeClientTests.tempStore()
            let model = ShellModel()
            model.autoCompile = false
            let proposal = try await stage(model, store: store, captureId: captureId)
            let bridge = try XCTUnwrap(model.bridge)
            let after = "Hello\\fakecapture{\(captureId)} FlashTeX.\n"
            var outcome = await model.approveBridgeProposal(proposal, latex: proposal.latex)
            if captureId.hasPrefix("crash-before-commit") {
                XCTAssertEqual(outcome, .refused("ledger"), model.captureNote ?? "")
                XCTAssertNil(model.pendingEdit)
                XCTAssertEqual(bridge.capture(captureId)?.state, .proposed, bridge.capture(captureId)?.note ?? "")
                XCTAssertTrue(bridge.ledgerUsable, bridge.ledgerStatus)
                let awaited8 = try await bridge.ledger!.status().pendingReceipts
                XCTAssertEqual(awaited8, [], "nothing committed")
                model.enqueue(proposal)
                outcome = await model.approveBridgeProposal(proposal, latex: proposal.latex)
            }
            XCTAssertEqual(outcome, .inserted(byteOffset: 5), "\(captureId): \(model.captureNote ?? "")")
            XCTAssertTrue(bridge.ledgerUsable, bridge.ledgerStatus)
            XCTAssertEqual(bridge.transactionTrace, ["ledger"])
            XCTAssertEqual(bridge.durable?.text, after)
            model.editApplied(try XCTUnwrap(model.pendingEdit), newText: after)
            try await T.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
            try await T.waitUntil { bridge.transactions["capture-\(captureId)"]?.confirmed == true }
            XCTAssertEqual(model.activeText, after, "\(captureId): inserted exactly once")
            let awaited9 = try await bridge.ledger!.status().pendingReceipts
            XCTAssertEqual(awaited9, [])
            // A repeat approval is a duplicate on every path.
            model.enqueue(proposal)
            let awaited10 = await model.approveBridgeProposal(proposal, latex: proposal.latex)
            XCTAssertEqual(awaited10, .duplicate)
            XCTAssertEqual(model.activeText, after)
            // The dead helper's queued exit never reaches the session (identity-guarded): no error, no second relaunch.
            try await Task.sleep(nanoseconds: 400_000_000)
            XCTAssertNil(bridge.ledgerError, bridge.ledgerError ?? "")
            XCTAssertTrue(bridge.ledgerUsable)
            model.detachBridge()
        }
    }

    func testLedgerCrashIsRelaunchedAndRealignedWithTheEditor() async throws {
        let store = try BridgeClientTests.tempStore()
        let model = ShellModel()
        model.autoCompile = false
        var notes: [(BridgeSession.Child, String)] = []
        let ok = await attach(model, store: store)
        XCTAssertTrue(ok)
        let bridge = try XCTUnwrap(model.bridge)
        bridge.onRelaunched = { notes.append(($0, $1)) }
        model.updateActiveText("Hello FlashTeX. typed\n")
        try await T.waitUntil { bridge.durable?.revision == model.editorRevision }

        // Idle crash: exit 3 with no request outstanding.
        let dead = try XCTUnwrap(bridge.ledger)
        var reply: Result<EditLedgerV1.Status, LineProcessFailure>?
        dead.send({ LedgerCrashRequest(id: $0) }, as: EditLedgerV1.Status.self) { reply = $0 }
        try await T.waitUntil { reply != nil }
        try await T.waitUntil { !bridge.ledgerUsable }
        XCTAssertTrue(bridge.ledgerStatus.contains("relaunching in 0.2 s"), bridge.ledgerStatus)
        XCTAssertTrue(model.bridgeStatus.contains("edit ledger exited (3)"), model.bridgeStatus)
        // Typing while the helper is down is not lost: the relaunch realigns the store with the
        // live buffer (no pending receipts → the buffer replaces the stored text).
        model.updateActiveText("Hello FlashTeX. typed more\n")
        try await waitForRelaunch(bridge, .ledger, count: 1)
        XCTAssertTrue(bridge.ledgerUsable, bridge.ledgerStatus)
        XCTAssertFalse(bridge.ledger === dead)
        XCTAssertTrue(bridge.ledgerStatus.hasPrefix("relaunched: "), bridge.ledgerStatus)
        XCTAssertEqual(bridge.durable?.text, "Hello FlashTeX. typed more\n")
        XCTAssertEqual(bridge.durable?.revision, model.editorRevision)
        XCTAssertEqual(notes.map(\.0), [.ledger])
        XCTAssertTrue(bridge.running, "the bridge was untouched")
        XCTAssertNil(bridge.ledgerError)
        let relaunchedStatus = try await bridge.ledger!.status()
        XCTAssertEqual(relaunchedStatus.document?.text, "Hello FlashTeX. typed more\n")

        // Crash during replace_document (typing): the transient failure does not poison the
        // session; after the relaunch the store follows the editor again.
        model.updateActiveText("Hello %ledger-crash-once\n")
        try await waitForRelaunch(bridge, .ledger, count: 2)
        XCTAssertNil(bridge.ledgerError, bridge.ledgerError ?? "")
        XCTAssertEqual(bridge.durable?.text, "Hello %ledger-crash-once\n")
        XCTAssertEqual(bridge.durable?.revision, model.editorRevision)
        model.updateActiveText("Hello again\n")
        try await T.waitUntil { bridge.durable?.revision == model.editorRevision && bridge.durable?.text == "Hello again\n" }
        let awaited11 = try await bridge.ledger!.status().document?.revision
        XCTAssertEqual(awaited11, model.editorRevision)

        // A full insertion works on the relaunched helper.
        model.caretUTF16 = 5
        model.pinAnchorAtCaret()
        try await T.waitUntil { model.bridgeDestination != nil }
        _ = await model.submitCapture(image: try BridgeClientTests.fixtureCapture().image, captureId: "fixture-capture-1", instructions: "t")
        let maybe_proposal = await model.convertCapture(captureId: "fixture-capture-1")
        let proposal = try XCTUnwrap(maybe_proposal, model.captureNote ?? "")
        let awaited13 = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(awaited13, .inserted(byteOffset: 5), model.captureNote ?? "")
        model.editApplied(try XCTUnwrap(model.pendingEdit), newText: "Hello\\fakecapture{fixture-capture-1} again\n")
        try await T.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
        model.detachBridge()
    }

    func testLedgerCrashWithPendingReceiptKeepsTheEvidence() async throws {
        let store = try BridgeClientTests.tempStore()
        let model = ShellModel()
        model.autoCompile = false
        let proposal = try await stage(model, store: store)
        let bridge = try XCTUnwrap(model.bridge)
        model.documentURL = store.appendingPathComponent("missing-dir/doc.tex") // export fails → receipt withheld
        model.savedText = model.activeText
        let awaited14 = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(awaited14, .inserted(byteOffset: 5))
        let after = "Hello\\fakecapture{fixture-capture-1} FlashTeX.\n"
        model.editApplied(try XCTUnwrap(model.pendingEdit), newText: after)
        XCTAssertEqual(bridge.transactionTrace, ["ledger"])
        // Undo in the editor (durable store keeps the tombstone + pending receipt), then the helper dies.
        model.updateActiveText("Hello FlashTeX.\n")
        try await T.waitUntil { bridge.durable?.revision == model.editorRevision }
        var reply: Result<EditLedgerV1.Status, LineProcessFailure>?
        bridge.ledger!.send({ LedgerCrashRequest(id: $0) }, as: EditLedgerV1.Status.self) { reply = $0 }
        try await T.waitUntil { reply != nil }
        try await waitForRelaunch(bridge, .ledger, count: 1)
        // Store text == buffer (the undo was persisted): aligned, no adoption; the receipt is still owed.
        XCTAssertTrue(bridge.ledgerUsable, bridge.ledgerStatus)
        XCTAssertEqual(bridge.durable?.text, model.activeText)
        XCTAssertNil(model.pendingEdit)
        let kept = try await bridge.ledger!.status()
        XCTAssertEqual(kept.pendingReceipts.count, 1, "evidence kept across the relaunch")
        XCTAssertNotNil(bridge.pendingTransaction, "the live transaction still owes its receipt")
        model.detachBridge()
    }

    func testRelaunchLimitPerMinuteLeavesTheExitVisible() async throws {
        let store = try BridgeClientTests.tempStore()
        let model = ShellModel()
        model.autoCompile = false
        let ok = await attach(model, store: store)
        XCTAssertTrue(ok)
        let bridge = try XCTUnwrap(model.bridge)
        for n in 1...BridgeSession.maxRelaunches {
            _ = try? await bridge.status(captureId: "crash-status-\(n)")
            try await T.waitUntil { !bridge.running }
            let delay = BridgeSession.relaunchDelays[n - 1]
            XCTAssertTrue(bridge.status.contains(String(format: "relaunching in %.1f s", delay)), bridge.status)
            try await waitForRelaunch(bridge, .bridge, count: n)
            XCTAssertTrue(bridge.running)
        }
        _ = try? await bridge.status(captureId: "crash-status-4")
        try await T.waitUntil { !bridge.running }
        try await Task.sleep(nanoseconds: 600_000_000)
        XCTAssertFalse(bridge.running)
        XCTAssertEqual(bridge.relaunchCount[.bridge], BridgeSession.maxRelaunches)
        XCTAssertTrue(bridge.status.contains("not relaunched") && bridge.status.contains("Attach Capture Bridge"), bridge.status)
        XCTAssertTrue(model.bridgeStatus.contains("not relaunched"), model.bridgeStatus)
        XCTAssertFalse(model.bridgeAttached)
        XCTAssertTrue(bridge.ledgerUsable, "the ledger is independent of the bridge's fate")
        model.detachBridge()
    }

    func testExplicitDetachCancelsAPendingRelaunchAndCleanExitNeverRelaunches() async throws {
        let store = try BridgeClientTests.tempStore()
        let model = ShellModel()
        model.autoCompile = false
        let ok = await attach(model, store: store)
        XCTAssertTrue(ok)
        let session = try XCTUnwrap(model.bridge)
        _ = try? await session.status(captureId: "crash-status-1")
        try await T.waitUntil { !session.running }
        XCTAssertTrue(session.status.contains("relaunching"), session.status)
        model.detachBridge()
        XCTAssertTrue(session.detached)
        try await Task.sleep(nanoseconds: 700_000_000)
        XCTAssertFalse(session.running)
        XCTAssertNil(session.relaunchCount[.bridge], "detach cancelled the pending relaunch")
        XCTAssertFalse(session.client.isRunning)
        XCTAssertEqual(session.status, "bridge detached")
        XCTAssertFalse(model.bridgeAttached)

        // A clean exit (status 0) is not a crash: no relaunch.
        let model2 = ShellModel()
        model2.autoCompile = false
        let ok2 = await attach(model2, store: try BridgeClientTests.tempStore())
        XCTAssertTrue(ok2)
        let clean = try XCTUnwrap(model2.bridge)
        model2.caretUTF16 = 0
        model2.pinAnchorAtCaret()
        try await T.waitUntil { model2.bridgeDestination != nil }
        _ = await model2.submitCapture(image: try BridgeClientTests.fixtureCapture().image, captureId: "fixture-capture-1", instructions: "%exit")
        try await T.waitUntil { !clean.running }
        try await Task.sleep(nanoseconds: 700_000_000)
        XCTAssertEqual(clean.status, "bridge exited (0)")
        XCTAssertNil(clean.relaunchCount[.bridge])
        XCTAssertFalse(clean.running)
        model2.detachBridge()
    }

    func testCrashDuringCaptureSubmitLeavesTheCaptureUncertainAndRetryable() async throws {
        // Journaled before the crash (the retry returns the same record) and not journaled
        // (the retry creates it): both end with exactly one journal entry.
        for directive in ["%crash-once-journaled", "%crash-once"] {
            let store = try BridgeClientTests.tempStore()
            let model = ShellModel()
            model.autoCompile = false
            let ok = await attach(model, store: store)
            XCTAssertTrue(ok)
            let bridge = try XCTUnwrap(model.bridge)
            model.caretUTF16 = 5
            model.pinAnchorAtCaret()
            try await T.waitUntil { model.bridgeDestination != nil }
            let pinned = try XCTUnwrap(model.bridgeDestination)
            let image = try BridgeClientTests.fixtureCapture().image
            let r1 = await model.submitCapture(image: image, captureId: "fixture-capture-1", instructions: directive)
            XCTAssertNil(r1)
            let uncertain = try XCTUnwrap(bridge.capture("fixture-capture-1"))
            XCTAssertEqual(uncertain.state, .uncertain, uncertain.note)
            XCTAssertTrue(uncertain.note.contains("resubmit with the same capture ID"), uncertain.note)
            XCTAssertEqual(model.bridgeCaptures.last?.state, .uncertain, "visible to the shell")
            XCTAssertTrue(model.captureNote?.contains("Capture submit failed") == true, model.captureNote ?? "")
            try await waitForRelaunch(bridge, .bridge, count: 1)
            XCTAssertEqual(model.bridgeDestination, pinned, "destination restored: the resubmission binds to the same target")
            XCTAssertEqual(bridge.capture("fixture-capture-1")?.state, .uncertain, "the relaunch does not guess")
            // Identical resubmission: idempotent on the bridge; exactly one journal record.
            let r2 = await model.submitCapture(image: image, captureId: "fixture-capture-1", instructions: directive)
            XCTAssertEqual(r2?.durable, true, "\(directive): \(model.captureNote ?? "")")
            XCTAssertEqual(bridge.capture("fixture-capture-1")?.state, .received)
            XCTAssertEqual(try journaledCaptureIDs(store: store), ["fixture-capture-1"])
            // …and it inserts once against the restored destination.
            let maybe_proposal = await model.convertCapture(captureId: "fixture-capture-1")
            let proposal = try XCTUnwrap(maybe_proposal, model.captureNote ?? "")
            let awaited16 = await model.approveBridgeProposal(proposal, latex: proposal.latex)
            XCTAssertEqual(awaited16, .inserted(byteOffset: 5), model.captureNote ?? "")
            let after = "Hello\\fakecapture{fixture-capture-1} FlashTeX.\n"
            model.editApplied(try XCTUnwrap(model.pendingEdit), newText: after)
            try await T.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
            XCTAssertEqual(model.activeText, after)
            XCTAssertEqual(try journaledCaptureIDs(store: store), ["fixture-capture-1"])
            model.detachBridge()
        }
    }
}
