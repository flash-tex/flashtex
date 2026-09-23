import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The fluid capture path (lane mac-capture-fluid), hermetic against
/// `Fixtures/fake_bridge.py` + `Fixtures/fake_edit_ledger.py`: a companion's
/// destination query pins the caret (no ⌘⌥P), the capture appears in the
/// Captures inbox at once, converts on its own, and Insert at caret rides the
/// existing `approveBridgeProposal` path; plus the pure state mapping and the
/// auto-advertise rule.
@MainActor
final class CaptureInboxTests: XCTestCase {
    private func attach(_ model: ShellModel, provider: ConversionProvider = .xai) async throws {
        let store = try BridgeClientTests.tempStore()
        let ok = await model.attachBridgeAndWait(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeBridge.path],
                                                 storeDirectory: store, provider: provider, ledger: ShellModelBridgeTests.fakeLedger, discoverLedger: false)
        XCTAssertTrue(ok, model.captureNote ?? model.bridgeStatus)
    }

    private func submit(id: String, to d: NearbyV1.Destination, instructions: String = "convert to TikZ") throws -> RuntimeV1.CaptureSubmit {
        RuntimeV1.CaptureSubmit(captureId: id, destinationId: d.destinationId, baseRevision: d.baseRevision,
                                image: try BridgeClientTests.fixtureCapture().image, instructions: instructions)
    }

    // MARK: pure

    func testStateDerivationFollowsTheShellsSourcesOfTruth() {
        var item = CaptureInbox.Item(id: "c", receivedAt: Date(), pairId: "p", instructions: "matrix", mimeType: "image/png", image: Data(),
                                     destinationId: "d", baseRevision: 1, autoPinned: true)
        func s(_ bridge: BridgeSession.CaptureState?, queued: Bool = false, applied: Bool = false, attached: Bool = true, conv: Bool = true) -> CaptureInbox.State {
            CaptureInbox.state(for: item, bridge: bridge, bridgeNote: "n", queued: queued, applied: applied, bridgeAttached: attached, conversionEnabled: conv)
        }
        XCTAssertEqual(s(nil, attached: false), .received(note: "in the Mac's inbox; attach the capture bridge to convert it"))
        XCTAssertEqual(s(nil).label, "received")
        XCTAssertEqual(s(.received).label, "journaled")
        XCTAssertTrue(s(.received, conv: false).detail!.contains("Preferences"))
        XCTAssertEqual(s(.converting), .converting)
        XCTAssertEqual(s(.proposed), .proposalReady)
        XCTAssertEqual(s(.received, queued: true), .proposalReady, "a queued proposal wins over the bridge's row")
        XCTAssertEqual(s(.confirmed), .inserted)
        XCTAssertEqual(s(.proposed, applied: true), .inserted, "the shell's applied set is authoritative")
        XCTAssertEqual(s(.failed), .failed("n"))
        XCTAssertEqual(s(.rejected), .rejected)
        item.latex = "\\x"
        XCTAssertEqual(s(nil), .proposalReady, "kept LaTeX shows the proposal after it left the queue")
        item.rejected = true
        XCTAssertEqual(s(nil), .rejected)
        item.failure = "provider_disabled"
        XCTAssertEqual(s(nil), .rejected, "rejection outranks a failure note")
        item.rejected = false
        XCTAssertEqual(s(nil), .failed("provider_disabled"))
        XCTAssertTrue(s(nil).isFinal)
        XCTAssertFalse(CaptureInbox.State.converting.isFinal)
    }

    func testAutoAdvertiseOnlyWithAPairedCompanionAndNotWhenDisabled() {
        XCTAssertFalse(CaptureInboxFeature.autoAdvertise(pairs: 0, environment: [:]))
        XCTAssertTrue(CaptureInboxFeature.autoAdvertise(pairs: 1, environment: [:]))
        XCTAssertFalse(CaptureInboxFeature.autoAdvertise(pairs: 2, environment: ["FLASHTEX_NEARBY_AUTO_ADVERTISE": "0"]))
        XCTAssertTrue(CaptureInboxFeature.flag("FLASHTEX_CAPTURE_AUTO_CONVERT", environment: [:]))
        XCTAssertFalse(CaptureInboxFeature.flag("FLASHTEX_CAPTURE_AUTO_CONVERT", environment: ["FLASHTEX_CAPTURE_AUTO_CONVERT": "0"]))
    }

    func testInboxIsBoundedAndDropsFinalRowsFirst() throws {
        let inbox = CaptureInbox()
        let image = try BridgeClientTests.fixtureCapture().image
        for i in 0..<(CaptureInbox.maxItems + 3) {
            inbox.received(.init(captureId: "c\(i)", destinationId: "d", baseRevision: 1, image: image, instructions: ""), pairId: nil, autoPinned: true)
        }
        XCTAssertEqual(inbox.items.count, CaptureInbox.maxItems)
        XCTAssertEqual(inbox.items.first?.id, "c\(CaptureInbox.maxItems + 2)", "newest first")
        inbox.received(.init(captureId: "c0", destinationId: "d", baseRevision: 1, image: image, instructions: "again"), pairId: nil, autoPinned: true)
        XCTAssertEqual(inbox.items.count, CaptureInbox.maxItems, "a retried id adds nothing")
        inbox.noteProposal("c10", latex: "\\a")
        XCTAssertEqual(inbox.item("c10")?.latex, "\\a")
        inbox.clearFinished(["c10", "c11"])
        XCTAssertNil(inbox.item("c10"))
        XCTAssertEqual(inbox.items.count, CaptureInbox.maxItems - 2)
    }

    func testHighlightedProposalKeepsTheTextAndColoursCommands() {
        let text = "\\begin{tikzpicture} % note\n\\draw (0,0) -- (1,1);\n\\end{tikzpicture}"
        let a = CaptureInboxRow.highlighted(text)
        XCTAssertEqual(String(a.characters), text)
        let coloured = a.runs.filter { $0.foregroundColor != nil }.count
        XCTAssertGreaterThan(coloured, 2)
    }

    // MARK: with the fake bridge

    func testDestinationQueryPinsTheCaretWithoutAnExplicitPin() async throws {
        let model = ShellModel()
        model.autoCompile = false
        try await attach(model)
        XCTAssertNil(model.nearbyDestination, "nothing pinned yet")
        XCTAssertTrue(model.captureDestinationIsAutomatic)
        model.caretUTF16 = 5
        let dq = await model.nearbyDestinationPinningCaretIfNeeded(); let d = try XCTUnwrap(dq)
        XCTAssertEqual(d.destinationId, "mac-caret-1")
        XCTAssertEqual(d.path, model.activePath)
        XCTAssertEqual(model.bridgeDestination?.startByte, 5, "pinned on the bridge before the companion is answered")
        XCTAssertEqual(model.anchor?.byteOffset, 5)
        XCTAssertTrue(model.captureDestinationIsAutomatic)
        // A second query re-uses the pin; an explicit pin overrides it.
        let again = await model.nearbyDestinationPinningCaretIfNeeded()
        XCTAssertEqual(again?.destinationId, "mac-caret-1")
        model.caretUTF16 = 2
        model.pinAnchorAtCaret()
        try await ShellModelBridgeTests.waitUntil { model.bridgeDestination?.startByte == 2 }
        let pinned = await model.nearbyDestinationPinningCaretIfNeeded()
        XCTAssertEqual(pinned?.destinationId, "mac-anchor-2")
        XCTAssertFalse(model.captureDestinationIsAutomatic)
        model.detachBridge()
    }

    func testNearbyCaptureAppearsConvertsAndInsertsAtTheCaretFromTheInbox() async throws {
        let model = ShellModel()
        model.autoCompile = false
        try await attach(model)
        model.caretUTF16 = 6 // after "Hello "
        let dq = await model.nearbyDestinationPinningCaretIfNeeded(); let d = try XCTUnwrap(dq)

        let result = await model.forwardNearbyCapture(try submit(id: "pad-cap-1", to: d), pairId: "padpair")
        guard case .success(let ack) = result else { return XCTFail("\(result)") }
        XCTAssertTrue(ack.durable)
        let row = try XCTUnwrap(model.captureInbox.item("pad-cap-1"), "visible the moment it arrived")
        XCTAssertEqual(row.instructions, "convert to TikZ")
        XCTAssertEqual(row.pairId, "padpair")
        XCTAssertTrue(row.autoPinned)
        XCTAssertFalse(row.image.isEmpty)

        // received → converting → proposal ready without any menu click.
        try await ShellModelBridgeTests.waitUntil { model.captureInboxState(model.captureInbox.item("pad-cap-1")!) == .proposalReady }
        XCTAssertEqual(model.captureInbox.item("pad-cap-1")?.latex, "\\fakecapture{pad-cap-1}")
        XCTAssertEqual(model.proposals.map(\.captureId), ["pad-cap-1"], "same review queue as the sheet")
        let status = await model.nearbyCaptureStatus(captureId: "pad-cap-1")
        guard case .success(let s) = status else { return XCTFail("\(status)") }
        XCTAssertEqual(s.state, "proposal_ready", "the iPad sees the same state")
        XCTAssertEqual(s.latex, "\\fakecapture{pad-cap-1}")

        // Insert at caret: one explicit click, through approveBridgeProposal.
        let item = try XCTUnwrap(model.captureInbox.item("pad-cap-1"))
        let edited = await model.insertCaptureFromInbox(item, latex: "\\edited")
        XCTAssertEqual(edited, .refused("edited LaTeX"), "the bridge contract refuses edited text; the panel says so")
        let outcome = await model.insertCaptureFromInbox(item, latex: "\\fakecapture{pad-cap-1}")
        XCTAssertEqual(outcome, .inserted(byteOffset: 6))
        let pending = try XCTUnwrap(model.pendingEdit)
        XCTAssertEqual(pending.nsRange, NSRange(location: 6, length: 0))
        let after = "Hello \\fakecapture{pad-cap-1}FlashTeX.\n"
        model.editApplied(pending, newText: after)
        XCTAssertEqual(model.activeText, after)
        try await ShellModelBridgeTests.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
        XCTAssertEqual(model.captureInboxState(model.captureInbox.item("pad-cap-1")!), .inserted)
        XCTAssertEqual(model.captureInbox.item("pad-cap-1")?.latex, "\\fakecapture{pad-cap-1}", "the inserted row keeps what went in")
        XCTAssertEqual(model.captureInboxFinalIDs, ["pad-cap-1"])
        let inserted = await model.nearbyCaptureStatus(captureId: "pad-cap-1")
        guard case .success(let s2) = inserted else { return XCTFail("\(inserted)") }
        XCTAssertEqual(s2.state, "inserted")

        // The next capture: the pin was consumed by the insertion, so the caret is pinned
        // again on demand — under the same automatic id (it is the companion's handle for
        // "the caret", re-pinned wherever the caret is), at the new place.
        XCTAssertNil(model.nearbyDestination)
        model.caretUTF16 = 0
        let dq2 = await model.nearbyDestinationPinningCaretIfNeeded(); let d2 = try XCTUnwrap(dq2)
        XCTAssertEqual(d2.destinationId, d.destinationId, "stable automatic id")
        XCTAssertEqual(model.bridgeDestination?.startByte, 0, "re-pinned at the caret")
        XCTAssertEqual(model.bridgeDestination?.mode, .caret)
        let r2 = await model.forwardNearbyCapture(try submit(id: "pad-cap-2", to: d2, instructions: "matrix"), pairId: "padpair")
        guard case .success = r2 else { return XCTFail("\(r2)") }
        try await ShellModelBridgeTests.waitUntil { model.captureInboxState(model.captureInbox.item("pad-cap-2")!) == .proposalReady }
        XCTAssertEqual(model.captureInbox.items.map(\.id), ["pad-cap-2", "pad-cap-1"], "newest first")
        model.rejectCaptureFromInbox(model.captureInbox.item("pad-cap-2")!)
        XCTAssertEqual(model.captureInboxState(model.captureInbox.item("pad-cap-2")!), .rejected)
        XCTAssertTrue(model.proposals.isEmpty)
        model.detachBridge()
    }

    func testWithoutAProviderTheRowWaitsAsJournaledAndConvertReportsTheRefusal() async throws {
        let model = ShellModel()
        model.autoCompile = false
        try await attach(model, provider: .none)
        let dq = await model.nearbyDestinationPinningCaretIfNeeded(); let d = try XCTUnwrap(dq)
        let r = await model.forwardNearbyCapture(try submit(id: "pad-cap-3", to: d, instructions: "%provider_disabled"), pairId: nil)
        guard case .success = r else { return XCTFail("\(r)") }
        try await Task.sleep(nanoseconds: 200_000_000)
        let item = try XCTUnwrap(model.captureInbox.item("pad-cap-3"))
        XCTAssertEqual(model.captureInboxState(item).label, "journaled", "no auto-convert without a provider")
        let proposal = await model.convertCaptureForInbox(captureId: "pad-cap-3")
        XCTAssertNil(proposal)
        guard case .failed(let why) = model.captureInboxState(model.captureInbox.item("pad-cap-3")!) else { return XCTFail() }
        XCTAssertTrue(why.contains("provider_disabled"), why)
        model.detachBridge()
    }

    func testWithoutABridgeTheCaptureSitsInTheInboxAsReceived() async throws {
        let model = ShellModel()
        model.autoCompile = false
        model.caretUTF16 = 3
        let dq = await model.nearbyDestinationPinningCaretIfNeeded(); let d = try XCTUnwrap(dq, "the local anchor serves the inbox-only path")
        XCTAssertEqual(d.destinationId, "mac-caret-1")
        let r = await model.forwardNearbyCapture(try submit(id: "pad-cap-4", to: d), pairId: nil)
        guard case .success(let ack) = r else { return XCTFail("\(r)") }
        XCTAssertFalse(ack.durable)
        let item = try XCTUnwrap(model.captureInbox.item("pad-cap-4"))
        XCTAssertEqual(model.captureInboxState(item), .received(note: "in the Mac's inbox; attach the capture bridge to convert it"))
        XCTAssertNil(model.captureInboxProposal(item))
        let refused = await model.insertCaptureFromInbox(item, latex: "x")
        XCTAssertEqual(refused, .refused("no proposal"))
    }

    func testCodeRequestFlagIsPublishedOnNearbyState() {
        let nearby = NearbyState(store: PairStore(url: FileManager.default.temporaryDirectory.appendingPathComponent("pairs-\(UUID()).json")), loopbackOnly: true)
        XCTAssertFalse(nearby.codeRequested)
        nearby.codeRequested = true
        XCTAssertTrue(nearby.codeRequested)
    }
}
