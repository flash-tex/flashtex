import Network
import XCTest
import FlashTeXProtocol
import NearbyClient
@testable import FlashTeXMac

/// Gap 7 acceptance (lane mac-capture-acceptance): companion capture
/// reconnect → review → explicit insert against the REAL helpers, using the
/// shared fixtures (`protocol/fixtures/capture-submission.json`,
/// `apps/mac/Samples/capture-proposal.json`). No provider is ever enabled
/// (`--enable-grok` is never passed), so nothing here talks to a network
/// beyond loopback, and no edit is ever applied without the explicit approval
/// call. Skipped cleanly when the helper environment variables are absent:
/// `FLASHTEX_BRIDGE` (+ optional `FLASHTEX_EDIT_LEDGER`) for the bridge tests,
/// `FLASHTEX_PREVIEW_CONTROLLER` + `FLASHTEX_COMPILER` for the helper route.
///
/// What is new here (the audit in `coordination/mac-capture-acceptance.md`
/// lists what older tests already cover): the reconnect/resend path through
/// the nearby listener with the real bridge journal as the ledger, an app
/// relaunch between receipt and insert, a stale-anchor capture decided by the
/// real bridge, and a reviewed insert riding the real preview-controller route.
@MainActor
final class CaptureAcceptanceTests: XCTestCase {
    // These cases assert the explicit-pin contract (destination_query answers null
    // until ⌘⌥P); the fluid caret pin (CaptureInboxTests) is switched off here.
    override func setUp() { CaptureInboxFeature.caretDestinationOverride = false }
    override func tearDown() { CaptureInboxFeature.caretDestinationOverride = nil }
    static let psk = Data(repeating: 0x7E, count: 32)
    static let pairId = "acceptancepair01"
    static var pair: PairedMac {
        PairedMac(fingerprint: "test-fp", macName: "Test Mac", pairId: pairId, pairPsk: psk.base64EncodedString(), companionName: "Acceptance iPad")
    }
    static var entry: NearbyListener.PSKEntry { .init(identity: pairId, key: psk, isBootstrap: false) }
    static let fixturePNG: Data = {
        let env = try! RuntimeV1.decodeCaptureSubmit(Data(contentsOf: NearbyListenerTests.fixtureURL))
        return Data(base64Encoded: env.payload.image.dataBase64)!
    }()
    static let demoText = "Hello FlashTeX.\n"

    private static func env(_ name: String) -> URL? {
        guard let p = ProcessInfo.processInfo.environment[name], FileManager.default.isExecutableFile(atPath: p) else { return nil }
        return URL(fileURLWithPath: p)
    }
    private func requireBridge() throws -> URL {
        guard let b = Self.env("FLASHTEX_BRIDGE") else { throw XCTSkip("set FLASHTEX_BRIDGE to the built flashtex-bridge binary") }
        return b
    }
    /// The real edit ledger when `FLASHTEX_EDIT_LEDGER` is set; otherwise the
    /// bridge attaches with insertion disabled, which changes nothing below
    /// (no insertion is ever attempted through the bridge: no provider).
    private var ledgerLaunch: ShellModel.LedgerLaunch? { Self.env("FLASHTEX_EDIT_LEDGER").map { .init(executable: $0) } }

    private func loadAverage() -> Double {
        var l = [Double](repeating: 0, count: 3)
        return getloadavg(&l, 3) >= 1 ? l[0] : -1
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 20, file: StaticString = #filePath, line: UInt = #line,
                           _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 25_000_000)
        }
        XCTFail("timed out after \(timeout)s waiting for \(what)", file: file, line: line)
    }

    private func attach(_ model: ShellModel, bridge: URL, store: URL) async throws {
        let ok = await model.attachBridgeAndWait(executable: bridge, storeDirectory: store, ledger: ledgerLaunch, discoverLedger: false)
        XCTAssertTrue(ok, model.captureNote ?? model.bridgeStatus)
        XCTAssertTrue(model.bridgeAttached)
    }

    private func journalFiles(in store: URL, captureId: String) -> [String] {
        ((try? FileManager.default.contentsOfDirectory(atPath: store.path)) ?? []).filter { $0.hasPrefix(captureId) && $0.hasSuffix(".json") }
    }

    private func reconnector(port: UInt16, log: NearbyReferenceClientTests.EventLog? = nil,
                             policy: ReconnectPolicy = ReconnectPolicy(maxAttempts: 6, initialDelay: 0.2, maxDelay: 1, jitter: 0, connectTimeout: 5, requestTimeout: 20)) -> NearbyReconnector {
        NearbyReconnector(pair: Self.pair, policy: policy,
                          endpoints: { .hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!) },
                          onEvent: log.map { l in { l.record($0) } })
    }

    /// A durable receipt for `id` with no proposal and nothing applied.
    private func assertReceipt(_ ack: NearbyWire.CaptureReceived, _ id: String, file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertEqual(ack.captureId, id, file: file, line: line)
        XCTAssertTrue(ack.durable, "the bridge journaled it", file: file, line: line)
        XCTAssertFalse(ack.hasProposal, file: file, line: line)
        XCTAssertFalse(ack.applied, file: file, line: line)
    }

    /// Nothing has been reviewed or inserted: the review queue is empty, no
    /// editor edit is staged, the buffer is untouched and no capture is applied.
    private func assertNothingInserted(_ model: ShellModel, text: String, file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertNil(model.pendingEdit, "no editor edit is staged without explicit approval", file: file, line: line)
        XCTAssertTrue(model.proposals.isEmpty, file: file, line: line)
        XCTAssertNil(model.reviewing, file: file, line: line)
        XCTAssertEqual(model.activeText, text, file: file, line: line)
        XCTAssertTrue(model.appliedCaptureIDs.isEmpty, file: file, line: line)
        if let d = model.bridge?.durable { XCTAssertEqual(d.text, text, "durable ledger document untouched", file: file, line: line) }
    }

    // MARK: (a)+(b) drop after delivery, reconnect, resend: one journal entry, no edit

    /// The companion's connection drops after the Mac forwarded the capture to
    /// the real bridge (journaled, acknowledgement never delivered). It
    /// reconnects with the same pairing to a listener restarted on the same
    /// port and re-sends the identical `capture_id`. The listener's per-session
    /// dedup cannot know the old session, so the sink sees two deliveries; the
    /// bridge journal is the ledger that makes it exactly once (one file, one
    /// row, the same durable receipt). Nothing is reviewed or inserted.
    func testDropAfterDeliveryThenReconnectIsJournaledOnceByTheRealBridge() async throws {
        let bridgeBin = try requireBridge()
        let store = try BridgeClientTests.tempStore()
        defer { try? FileManager.default.removeItem(at: store) }
        let load = loadAverage()
        let model = ShellModel()
        model.autoCompile = false
        XCTAssertEqual(model.activeText, Self.demoText)
        try await attach(model, bridge: bridgeBin, store: store)
        model.caretUTF16 = 5
        model.pinAnchorAtCaret()
        try await waitUntil("bridge pin") { model.bridgeDestination != nil }
        let advertised = try XCTUnwrap(model.nearbyDestination)
        XCTAssertEqual(advertised.destinationId, model.bridgeDestination?.destinationId)

        let restarted = XCTestExpectation(description: "listener restarted")
        var h1: ListenerHarness!
        var h2: ListenerHarness?
        let sink = NearbyReferenceClientTests.AckDroppingSink(model) {
            h1.listener.stop { DispatchQueue.main.async { restarted.fulfill() } }
        }
        h1 = ListenerHarness(psks: [Self.entry], sink: sink, destinations: model)
        try h1.start()
        let port = h1.port
        defer { h1.stop(); h2?.stop() }

        let log = NearbyReferenceClientTests.EventLog()
        let r = reconnector(port: port, log: log)
        let session = try await r.connect()
        XCTAssertEqual(session.destination?.destinationId, advertised.destinationId)
        XCTAssertEqual(session.destination?.baseRevision, advertised.baseRevision)
        let capture = try session.makeCapture(captureId: "acceptance-drop-1", image: Self.fixturePNG, mimeType: "image/png", instructions: "once, please")
        let started = Date()
        let submitTask = Task { try await r.submit(capture) }
        await fulfillment(of: [restarted], timeout: 20)
        // The bridge journaled the capture before the drop; the companion does not know.
        XCTAssertEqual(journalFiles(in: store, captureId: "acceptance-drop-1").count, 1, "journaled once before the drop")
        h2 = ListenerHarness(psks: [Self.entry], sink: model, destinations: model, port: port)
        try h2!.start()
        let ack = try await submitTask.value
        let elapsed = Date().timeIntervalSince(started)
        let attempts = await r.attemptsMade
        print("measured: real-bridge drop→reconnect→duplicate-ack in \(String(format: "%.3f", elapsed))s; attempts \(attempts); 1-min load \(load)")

        assertReceipt(ack, "acceptance-drop-1")
        XCTAssertEqual(sink.deliveries, 1, "the first listener saw exactly one delivery")
        XCTAssertGreaterThanOrEqual(attempts, 2, "at least one reconnect: \(log.list)")
        XCTAssertTrue(h2!.snapshot.contains(.capture(captureId: "acceptance-drop-1")), "the second session delivered it again (fresh dedup memory): \(h2!.snapshot)")
        XCTAssertEqual(model.bridgeCaptures.filter { $0.captureId == "acceptance-drop-1" }.count, 1, "one row for two deliveries")
        XCTAssertEqual(model.bridgeCaptures.first { $0.captureId == "acceptance-drop-1" }?.state, .received)
        XCTAssertEqual(journalFiles(in: store, captureId: "acceptance-drop-1").count, 1, "the second delivery returned the existing record")
        XCTAssertEqual(model.nearbyInbox.received.count, 0, "forwarded captures never land in the in-memory inbox")
        XCTAssertEqual(model.nearbyInbox.lastCaptureId, "acceptance-drop-1")
        let status = try await model.bridge!.status(captureId: "acceptance-drop-1")
        XCTAssertNil(status.proposal); XCTAssertNil(status.prepared); XCTAssertNil(status.applied); XCTAssertFalse(status.rejected)
        assertNothingInserted(model, text: Self.demoText)

        // (b) The review path stops honestly without a provider: convert is refused as plain
        // text, prepare is refused (proposal_missing), and still nothing is staged.
        let proposal = await model.convertCapture(captureId: "acceptance-drop-1")
        XCTAssertNil(proposal)
        XCTAssertTrue(model.captureNote?.contains("provider_disabled") == true, model.captureNote ?? "")
        XCTAssertEqual(model.bridgeCaptures.first { $0.captureId == "acceptance-drop-1" }?.state, .received, "still convertible once a provider is configured")
        do { _ = try await model.bridge!.prepare(captureId: "acceptance-drop-1", expectedRevision: model.editorRevision); XCTFail("no proposal, no prepared edit") }
        catch let f as BridgeClient.Failure { XCTAssertEqual(f.code, "proposal_missing", f.text) }
        // Note: a direct prepare probe (the product only prepares after a proposal) leaves the
        // row as `.proposed` — BridgeSession.prepare assumes a proposal existed; still convertible.
        XCTAssertEqual(model.latestConvertibleCapture?.captureId, "acceptance-drop-1")
        assertNothingInserted(model, text: Self.demoText)
        XCTAssertEqual(journalFiles(in: store, captureId: "acceptance-drop-1").count, 1)
        await r.shutdown()
        model.detachBridge()
        XCTAssertFalse(model.bridgeAttached)
    }

    // MARK: (e) app relaunch between receipt and insert

    /// The app quits (bridge detached: the helper exits, the store stays)
    /// after a companion capture was journaled and before any review. A new
    /// ShellModel attached to the same store: the journal still answers
    /// `capture_status`, a companion resend against the re-pinned identical
    /// destination gets the identical receipt, and there is never a second
    /// journal entry or row. Finding (documented, not patched): transfer-v1
    /// has no capture-listing request, so the relaunched shell's
    /// `bridgeCaptures` is empty until the companion resends; the resend before
    /// a re-pin is refused client-side (`destinationChanged`) and sends nothing.
    func testAppRelaunchBetweenReceiptAndInsertKeepsOneJournaledCaptureWithoutDuplicates() async throws {
        let bridgeBin = try requireBridge()
        let store = try BridgeClientTests.tempStore()
        defer { try? FileManager.default.removeItem(at: store) }
        let model1 = ShellModel()
        model1.autoCompile = false
        try await attach(model1, bridge: bridgeBin, store: store)
        model1.caretUTF16 = 5
        model1.pinAnchorAtCaret()
        try await waitUntil("bridge pin") { model1.bridgeDestination != nil }
        let advertised = try XCTUnwrap(model1.nearbyDestination)

        let h1 = ListenerHarness(psks: [Self.entry], sink: model1, destinations: model1)
        try h1.start()
        let r1 = reconnector(port: h1.port)
        let s1 = try await r1.connect()
        let capture = try s1.makeCapture(captureId: "acceptance-relaunch-1", image: Self.fixturePNG, mimeType: "image/png", instructions: "before relaunch")
        let ack1 = try await r1.submit(capture)
        assertReceipt(ack1, "acceptance-relaunch-1")
        XCTAssertEqual(journalFiles(in: store, captureId: "acceptance-relaunch-1").count, 1)
        assertNothingInserted(model1, text: Self.demoText)
        await r1.shutdown()
        h1.stop()

        // "Quit": the bridge (and ledger) processes end; the store directory survives.
        model1.detachBridge()
        XCTAssertFalse(model1.bridgeAttached)
        try await waitUntil("bridge process gone") { RealHelperProcess.pid(commandLineContaining: "flashtex-bridge --store \(store.path)") == nil }
        XCTAssertEqual(journalFiles(in: store, captureId: "acceptance-relaunch-1").count, 1, "the journal outlives the process")

        // "Relaunch": a fresh shell on the same store.
        let model2 = ShellModel()
        model2.autoCompile = false
        try await attach(model2, bridge: bridgeBin, store: store)
        XCTAssertTrue(model2.bridgeCaptures.isEmpty, "finding: no capture listing in transfer-v1; the relaunched shell shows nothing until a resend")
        XCTAssertNil(model2.nearbyDestination, "anchors are in-memory on the bridge; nothing is pinned after a relaunch")
        let status = try await model2.bridge!.status(captureId: "acceptance-relaunch-1")
        XCTAssertNil(status.proposal); XCTAssertNil(status.prepared); XCTAssertNil(status.applied); XCTAssertFalse(status.rejected)
        assertNothingInserted(model2, text: Self.demoText)

        let h2 = ListenerHarness(psks: [Self.entry], sink: model2, destinations: model2)
        try h2.start()
        defer { h2.stop() }
        let r2 = reconnector(port: h2.port, policy: .immediate)
        // Before a re-pin the Mac reports no destination: the companion refuses client-side, nothing is sent.
        do { _ = try await r2.submit(capture); XCTFail("must not send against a missing destination") }
        catch let e as NearbyError {
            guard case .destinationChanged(captureDestination: _, current: let current) = e else { return XCTFail("unexpected \(e)") }
            XCTAssertNil(current)
            XCTAssertFalse(e.isRetryable)
        }
        XCTAssertTrue(model2.bridgeCaptures.isEmpty, "nothing reached the Mac")
        XCTAssertTrue(h2.snapshot.allSatisfy { if case .capture = $0 { return false }; return true }, "\(h2.snapshot)")

        // Re-pin at the same caret on the same text: same id, revision and binding as before the relaunch.
        model2.caretUTF16 = 5
        model2.pinAnchorAtCaret()
        try await waitUntil("bridge re-pin") { model2.bridgeDestination != nil }
        XCTAssertEqual(model2.nearbyDestination, advertised, "identical destination after the relaunch (same text, revision and caret)")
        let ack2 = try await r2.submit(capture)
        XCTAssertEqual(ack2, ack1, "the journal answers the resend with the original receipt")
        XCTAssertEqual(journalFiles(in: store, captureId: "acceptance-relaunch-1").count, 1, "never a second journal entry")
        XCTAssertEqual(model2.bridgeCaptures.filter { $0.captureId == "acceptance-relaunch-1" }.count, 1)
        XCTAssertEqual(model2.bridgeCaptures.first?.state, .received)
        assertNothingInserted(model2, text: Self.demoText)
        // A different payload under the same id after the relaunch is still a conflict, not a second capture.
        var changed = capture; changed.instructions = "after relaunch"
        do { _ = try await r2.submit(changed); XCTFail("conflicting resend must be refused") }
        catch let e as NearbyError {
            guard case .remote("capture_id_conflict", _) = e else { return XCTFail("unexpected \(e)") }
        }
        XCTAssertEqual(journalFiles(in: store, captureId: "acceptance-relaunch-1").count, 1)
        XCTAssertEqual(model2.bridgeCaptures.first { $0.captureId == "acceptance-relaunch-1" }?.state, .received, "a conflicting retry leaves the journaled capture usable")
        await r2.shutdown()
        model2.detachBridge()
    }

    // MARK: (d) capture bound to an anchor whose text changed

    /// After `hello_ack` advertised the pinned destination, an edit that
    /// overlaps the pin makes the real bridge's anchor invalid. The Mac mirrors
    /// that rule (mac-nearby-errors): the pin stays listed as invalid, the user
    /// is told, and companions are told `destination: null`, so the reference
    /// client's own check stops the send (`destinationChanged`, terminal). A
    /// client that skips the check reaches the real bridge, which decides the
    /// same way: `destination_reselection_required`, terminal, classified as
    /// "new capture at a new destination", nothing journaled, nothing inserted.
    /// A fresh pin accepts a new capture; an edit elsewhere keeps that anchor
    /// valid (accepted, rebased on the bridge and in the Mac's mirror) and
    /// still nothing is inserted without review.
    func testCaptureAgainstAChangedAnchorIsRefusedByTheRealBridgeAndNeverInserted() async throws {
        let bridgeBin = try requireBridge()
        let store = try BridgeClientTests.tempStore()
        defer { try? FileManager.default.removeItem(at: store) }
        let model = ShellModel()
        model.autoCompile = false
        try await attach(model, bridge: bridgeBin, store: store)
        let bridge = try XCTUnwrap(model.bridge)
        model.caretUTF16 = 5 // "Hello| FlashTeX.\n" → byte 5
        model.pinAnchorAtCaret()
        try await waitUntil("bridge pin") { model.bridgeDestination != nil }
        let advertised = try XCTUnwrap(model.nearbyDestination)
        XCTAssertEqual(model.bridgeDestination?.startByte, 5)

        let h = ListenerHarness(psks: [Self.entry], sink: model, destinations: model)
        try h.start()
        defer { h.stop() }
        let r = reconnector(port: h.port, policy: .immediate)
        let session = try await r.connect()
        // The session names the advertised anchor. Asserted field by field
        // rather than against a rebuilt `Destination`: the announced one also
        // carries `caret_context` (what the companion shows before sending),
        // and the wire contract is additive, so a whole-value comparison
        // breaks on every new optional field rather than on a real change.
        XCTAssertEqual(session.destination?.destinationId, advertised.destinationId)
        XCTAssertEqual(session.destination?.projectId, advertised.projectId)
        XCTAssertEqual(session.destination?.path, advertised.path)
        XCTAssertEqual(session.destination?.baseRevision, advertised.baseRevision)
        let stale = try session.makeCapture(captureId: "acceptance-stale-1", image: Self.fixturePNG, mimeType: "image/png", instructions: "stale anchor")

        // The user deletes the space the pin sits on (bytes 5..<6): the edit overlaps the anchor.
        let edited = "HelloFlashTeX.\n"
        model.updateActiveText(edited)
        let editedRevision = model.editorRevision
        try await waitUntil("document_edit sent") { bridge.shadow["main.tex"]?.revision == editedRevision }
        if ledgerLaunch != nil { try await waitUntil("durable edit") { bridge.durable?.revision == editedRevision } }
        XCTAssertNil(model.nearbyDestination, "the Mac stops advertising a pin an edit overlapped")
        XCTAssertEqual(model.bridgeDestination?.destinationId, advertised.destinationId, "the pin is listed (invalid), not silently removed")
        XCTAssertEqual(model.bridgeDestination?.valid, false)
        XCTAssertTrue(model.captureNote?.contains("dropped by an edit") == true, model.captureNote ?? "nil")
        let query = try await session.connection.destinationQuery()
        XCTAssertNil(query, "destination_query reports nothing pinned")

        // The reference client checks the destination first: terminal, nothing sent to the bridge.
        do { _ = try await r.submit(stale); XCTFail("a capture bound to a dropped destination must not be sent") }
        catch let e as NearbyError {
            guard case .destinationChanged(let was, let now) = e else { return XCTFail("unexpected \(e)") }
            XCTAssertTrue(was.hasPrefix(advertised.destinationId), was)
            XCTAssertNil(now)
            XCTAssertFalse(e.isRetryable); XCTAssertTrue(e.needsNewDestination); XCTAssertTrue(e.needsNewCapture)
        }
        XCTAssertFalse(h.snapshot.contains { if case .captureRefused = $0 { return true }; return false }, "the listener saw no capture: \(h.snapshot)")
        XCTAssertNil(model.bridgeCaptures.first { $0.captureId == "acceptance-stale-1" })

        // A client that skips the check: the real bridge decides, same conclusion.
        do { _ = try await r.submit(stale, requireCurrentDestination: false); XCTFail("a capture bound to an overlapped anchor must be refused") }
        catch let e as NearbyError {
            guard case .remote(let code, _) = e else { return XCTFail("unexpected \(e)") }
            XCTAssertEqual(code, "destination_reselection_required")
            XCTAssertFalse(e.isRetryable, "terminal: retrying the same bytes would repeat it")
            XCTAssertTrue(e.needsNewCapture, "classified: build a new capture")
            XCTAssertTrue(e.needsNewDestination, "…at a destination re-read from the Mac")
            print("measured: stale-anchor refusal code=\(code) needsNewCapture=\(e.needsNewCapture) needsNewDestination=\(e.needsNewDestination)")
        }
        XCTAssertTrue(journalFiles(in: store, captureId: "acceptance-stale-1").isEmpty, "refused before the journal")
        XCTAssertEqual(model.bridgeCaptures.first { $0.captureId == "acceptance-stale-1" }?.state, .failed, model.bridgeStatus)
        XCTAssertTrue(model.bridgeCaptures.first { $0.captureId == "acceptance-stale-1" }?.note.contains("destination_reselection_required") == true)
        do { _ = try await bridge.status(captureId: "acceptance-stale-1"); XCTFail("never received") }
        catch let f as BridgeClient.Failure { XCTAssertEqual(f.code, "capture_missing", f.text) }
        XCTAssertNil(model.pendingEdit)
        XCTAssertTrue(model.proposals.isEmpty)
        XCTAssertEqual(model.activeText, edited)
        XCTAssertTrue(model.appliedCaptureIDs.isEmpty)
        XCTAssertTrue(h.snapshot.contains { if case .captureRefused(Self.pairId?, "acceptance-stale-1"?, "destination_reselection_required", _) = $0 { return true }; return false }, "\(h.snapshot)")

        // Reselection: a new pin (new id, current revision) is what the companion must capture against.
        model.caretUTF16 = 5
        model.pinAnchorAtCaret()
        try await waitUntil("re-pin") { model.bridgeDestination?.pinnedRevision == editedRevision }
        XCTAssertEqual(model.bridgeDestination?.valid, true)
        let fresh = try XCTUnwrap(model.nearbyDestination)
        let requeried = try await session.connection.destinationQuery()
        XCTAssertEqual(requeried?.destinationId, fresh.destinationId)
        XCTAssertNotEqual(fresh.destinationId, advertised.destinationId)
        XCTAssertEqual(fresh.baseRevision, editedRevision)
        let freshWire = NearbyWire.Destination(destinationId: fresh.destinationId, projectId: fresh.projectId, path: fresh.path, baseRevision: fresh.baseRevision)
        let again = try session.makeCapture(captureId: "acceptance-stale-2", image: Self.fixturePNG, mimeType: "image/png", instructions: "fresh anchor", destination: freshWire)
        let ack2 = try await r.submit(again)
        assertReceipt(ack2, "acceptance-stale-2")
        XCTAssertEqual(journalFiles(in: store, captureId: "acceptance-stale-2").count, 1)

        // An edit after the anchor keeps it valid: the bridge rebases the anchor and still
        // accepts captures at the pinned revision; nothing is inserted without review.
        let appended = edited + "more\n"
        model.updateActiveText(appended)
        try await waitUntil("second document_edit sent") { bridge.shadow["main.tex"]?.revision == model.editorRevision }
        XCTAssertEqual(model.nearbyDestination, fresh, "unchanged pin, unchanged advertisement")
        XCTAssertEqual(model.bridgeDestination?.valid, true)
        XCTAssertEqual(model.bridgeDestination?.currentRevision, model.editorRevision, "the mirror follows the bridge's current_revision")
        XCTAssertEqual(model.bridgeDestination?.startByte, 5, "an edit after the pin does not move it")
        let third = try session.makeCapture(captureId: "acceptance-stale-3", image: Self.fixturePNG, mimeType: "image/png", instructions: "edit elsewhere", destination: freshWire)
        let ack3 = try await r.submit(third)
        XCTAssertTrue(ack3.durable)
        XCTAssertFalse(ack3.applied)
        do { _ = try await bridge.prepare(captureId: "acceptance-stale-3", expectedRevision: model.editorRevision); XCTFail() }
        catch let f as BridgeClient.Failure { XCTAssertEqual(f.code, "proposal_missing", f.text) }
        XCTAssertNil(model.pendingEdit)
        XCTAssertEqual(model.activeText, appended)
        XCTAssertTrue(model.appliedCaptureIDs.isEmpty)
        if let d = bridge.durable { XCTAssertEqual(d.text, appended) }
        await r.shutdown()
        model.detachBridge()
    }

    // MARK: (c) explicit insert through the real preview-controller route

    /// A reviewed proposal (the shared sample) is inserted at the pinned anchor
    /// only by the explicit approval; the editor's adoption is one edit that
    /// rides the durable helper route: exactly one new durable revision whose
    /// text carries the insertion once, compiled from that text. A duplicate
    /// approval never inserts again, and the revert the editor's ⌘Z produces
    /// reaches the helper as the next ordinary edit while the tombstone stays.
    /// (That the adoption is a single NSUndoManager step is covered by
    /// `SourceEditorViewTests.testCaptureInsertionIsOneUndoStepDeliveredToTheModelOnce`.)
    func testExplicitInsertRidesTheRealPreviewControllerAsOneDurableEdit() async throws {
        guard let helper = Self.env("FLASHTEX_PREVIEW_CONTROLLER"), ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("capture-acceptance-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = root.appendingPathComponent("project/main.tex")
        let before = "\\begin{document}\nHello durable world.\n\\end{document}\n"
        try before.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let load = loadAverage()

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: helper)
        defer { model.detachController() }
        XCTAssertTrue(model.controllerAttached)
        try await waitUntil("initial durable document + preview") {
            model.controllerState.durable["main.tex"] != nil && model.result?.revision == model.editorRevision && model.inFlightRevision == nil
        }
        let durableBefore = try XCTUnwrap(model.controllerState.durable["main.tex"]).revision
        XCTAssertEqual(model.controllerState.textByDurable["main.tex"]?[durableBefore], before)

        // Pin at the start of the body line ("\begin{document}\n" is 17 UTF-16 units and bytes).
        model.caretUTF16 = 17
        model.pinAnchorAtCaret()
        let anchor = try XCTUnwrap(model.anchor)
        XCTAssertEqual(anchor.byteOffset, 17)
        let sample = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("Samples/capture-proposal.json")
        let proposal = try RuntimeV1.decodeCaptureProposal(Data(contentsOf: sample)).payload
        model.enqueue(proposal)
        XCTAssertEqual(model.reviewing?.captureId, "sample-capture-1")
        // Reviewing changes nothing: no staged edit, no durable revision, buffer untouched.
        try await Task.sleep(nanoseconds: 200_000_000)
        XCTAssertNil(model.pendingEdit)
        XCTAssertEqual(model.activeText, before)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, durableBefore, "no durable edit without approval")

        // Explicit Insert.
        let outcome = model.approveProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(outcome, .inserted(byteOffset: 17))
        let pending = try XCTUnwrap(model.pendingEdit)
        XCTAssertEqual(pending.nsRange, NSRange(location: 17, length: 0))
        // A capture goes through `captureInsertion`, not `insertionText`: it
        // normalises the proposal for the caret it lands on. Byte 17 is the
        // start of the line holding "Hello durable world.", so the caret is
        // inline text and the sample's `\begin{equation}` is rewritten to
        // `$…$` -- display there would split the sentence.
        XCTAssertEqual(pending.text, Insertion.captureInsertion(proposal.latex, into: before, atByte: 17).text)
        XCTAssertEqual(model.activeText, before, "the editor has not adopted it yet")
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, durableBefore, "nothing durable until the editor adopts the edit")
        let after = "\\begin{document}\n" + pending.text + "Hello durable world.\n\\end{document}\n"
        let sentAt = Date()
        model.editApplied(pending, newText: after) // what the editor does after registering the undo step
        XCTAssertNil(model.pendingEdit)
        XCTAssertEqual(model.activeText, after)
        try await waitUntil("insertion durable + previewed") {
            model.controllerState.durable["main.tex"]?.revision == durableBefore + 1 && model.inFlightRevision == nil && model.result?.revision == model.editorRevision
        }
        let durable = try XCTUnwrap(model.controllerState.durable["main.tex"])
        XCTAssertEqual(durable.revision, durableBefore + 1, "exactly one durable edit for the insertion")
        let durableText = try XCTUnwrap(model.controllerState.textByDurable["main.tex"]?[durable.revision])
        XCTAssertEqual(durableText, after)
        XCTAssertEqual(durableText.components(separatedBy: "\\int_0^1 x^2").count - 1, 1, "the LaTeX appears exactly once")
        XCTAssertEqual(durable.sha256, SourceDigest.sha256Hex(after))
        XCTAssertEqual(model.compiledDocuments["main.tex"], after, "the preview was compiled from the inserted text")
        XCTAssertFalse(model.previewIsStale)
        XCTAssertTrue(model.appliedCaptureIDs.contains("sample-capture-1"))
        XCTAssertEqual(model.anchor?.byteOffset, 17 + pending.text.utf8.count, "anchor advances past the insertion")
        print("measured: capture insert → durable+preview via flashtex-preview-controller in \(String(format: "%.0f", Date().timeIntervalSince(sentAt) * 1000)) ms; 1-min load \(load)")

        // Duplicate approval never inserts twice, on any route.
        model.enqueue(proposal)
        XCTAssertTrue(model.proposals.isEmpty)
        XCTAssertEqual(model.approveProposal(proposal, latex: proposal.latex), .duplicate)
        XCTAssertNil(model.pendingEdit)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, durableBefore + 1)

        // The editor's ⌘Z restores `before` as an ordinary edit: it reaches the helper in
        // order as the next durable revision; the tombstone still refuses a re-insert.
        model.updateActiveText(before)
        try await waitUntil("revert durable") {
            model.controllerState.durable["main.tex"]?.revision == durableBefore + 2 && model.inFlightRevision == nil && model.result?.revision == model.editorRevision
        }
        XCTAssertEqual(model.controllerState.textByDurable["main.tex"]?[durableBefore + 2], before)
        XCTAssertEqual(model.compiledDocuments["main.tex"], before)
        model.enqueue(proposal)
        XCTAssertTrue(model.proposals.isEmpty, "tombstone survives the undo")
        XCTAssertEqual(model.approveProposal(proposal, latex: proposal.latex), .duplicate)
        XCTAssertEqual(model.activeText, before)
    }
}
