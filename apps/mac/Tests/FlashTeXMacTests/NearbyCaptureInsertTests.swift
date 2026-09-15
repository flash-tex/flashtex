import XCTest
import FlashTeXProtocol
import NearbyClient
@testable import FlashTeXMac

/// nearby-v1 `capture_insert` (protocol/proposals/nearby-v1-companion-insert.md):
/// the companion approves the proposal it was shown and the Mac applies it
/// through the same path the Captures inspector's Insert button uses.
///
/// The property under test is not "a companion can insert". It is that a
/// companion can only insert **the proposal it read**: every refusal below is
/// what keeps this an approval rather than an automatic insertion.
@MainActor
final class NearbyCaptureInsertTests: XCTestCase {
    private func attach(_ model: ShellModel) async throws {
        let store = try BridgeClientTests.tempStore()
        let ok = await model.attachBridgeAndWait(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeBridge.path],
                                                 storeDirectory: store, provider: .xai, ledger: ShellModelBridgeTests.fakeLedger, discoverLedger: false)
        XCTAssertTrue(ok, model.captureNote ?? model.bridgeStatus)
    }

    private func submit(id: String, to d: NearbyV1.Destination) throws -> RuntimeV1.CaptureSubmit {
        RuntimeV1.CaptureSubmit(captureId: id, destinationId: d.destinationId, baseRevision: d.baseRevision,
                                image: try BridgeClientTests.fixtureCapture().image, instructions: "convert to TikZ")
    }

    // MARK: the digest both ends compute

    func testTheApprovalDigestIsTheSameOnTheMacAndInTheClientPackage() {
        for latex in ["", "\\alpha", "$x^2$", "\\begin{align}\na &= b\\\\\n\\end{align}", "π ≤ ∞ 🙂"] {
            let mac = NearbyV1.proposalDigest(latex)
            XCTAssertEqual(mac.count, 64, "64 hex digits")
            XCTAssertEqual(mac, mac.lowercased(), "lowercase, so a byte comparison is enough")
            XCTAssertEqual(mac, NearbyWire.proposalDigest(latex),
                           "the companion and the Mac must derive the approval token identically")
        }
        // A known vector, so a change to either side is caught here rather than on a desk.
        XCTAssertEqual(NearbyV1.proposalDigest(""),
                       "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    }

    func testRequestAndAckUseTheWireSpelling() throws {
        let request = NearbyV1.CaptureInsertRequest(captureId: "cap-1", approvedLatexSha256: String(repeating: "a", count: 64))
        let requestJSON = try XCTUnwrap(String(data: JSONEncoder().encode(request), encoding: .utf8))
        XCTAssertTrue(requestJSON.contains("\"capture_id\""))
        XCTAssertTrue(requestJSON.contains("\"approved_latex_sha256\""))
        let ack = NearbyV1.CaptureInsertAck(captureId: "cap-1", state: .inserted, newRevision: 7, note: "done")
        let ackJSON = try XCTUnwrap(String(data: JSONEncoder().encode(ack), encoding: .utf8))
        XCTAssertTrue(ackJSON.contains("\"new_revision\":7"))
        // The client package decodes exactly what the Mac encodes.
        let decoded = try JSONDecoder().decode(NearbyWire.CaptureInsertAck.self, from: JSONEncoder().encode(ack))
        XCTAssertEqual(decoded.state, "inserted")
        XCTAssertEqual(decoded.newRevision, 7)
        XCTAssertEqual(decoded.captureId, "cap-1")
    }

    // MARK: refusals — nothing reaches the document

    func testAnUnknownCaptureIsRefused() async {
        let model = ShellModel()
        model.autoCompile = false
        let r = await model.nearbyCaptureInsert(captureId: "never-sent", approvedDigest: NearbyV1.proposalDigest("\\alpha"))
        guard case .failure(let e) = r else { return XCTFail("\(r)") }
        XCTAssertEqual(e.code, "unknown_capture")
    }

    func testACaptureWithNoProposalYetIsRefused() async throws {
        let model = ShellModel()
        model.autoCompile = false
        try await attach(model)
        model.caretUTF16 = 6
        let dq = await model.nearbyDestinationPinningCaretIfNeeded(); let d = try XCTUnwrap(dq)
        // Put the row in the inbox without letting a proposal exist for it.
        model.captureInbox.received(try submit(id: "pad-cap-np", to: d), pairId: "padpair", autoPinned: true)
        let r = await model.nearbyCaptureInsert(captureId: "pad-cap-np", approvedDigest: NearbyV1.proposalDigest("\\alpha"))
        guard case .failure(let e) = r else { return XCTFail("\(r)") }
        XCTAssertEqual(e.code, "no_proposal")
        XCTAssertNil(model.pendingEdit, "no edit was prepared")
        model.detachBridge()
    }

    /// The invariant this message has to keep: approving text that is not the
    /// Mac's current proposal inserts nothing.
    func testApprovingTextThatIsNotTheMacsProposalIsRefusedAndInsertsNothing() async throws {
        let model = ShellModel()
        model.autoCompile = false
        try await attach(model)
        model.caretUTF16 = 6
        let dq = await model.nearbyDestinationPinningCaretIfNeeded(); let d = try XCTUnwrap(dq)
        let sent = await model.forwardNearbyCapture(try submit(id: "pad-cap-x", to: d), pairId: "padpair")
        guard case .success = sent else { return XCTFail("\(sent)") }
        try await ShellModelBridgeTests.waitUntil { model.captureInboxState(model.captureInbox.item("pad-cap-x")!) == .proposalReady }
        let real = try XCTUnwrap(model.captureInbox.item("pad-cap-x")?.latex)

        // Someone read an older (or invented) proposal and approved that.
        let stale = await model.nearbyCaptureInsert(captureId: "pad-cap-x", approvedDigest: NearbyV1.proposalDigest(real + " % edited"))
        guard case .failure(let e) = stale else { return XCTFail("\(stale)") }
        XCTAssertEqual(e.code, "proposal_changed")
        XCTAssertNil(model.pendingEdit, "nothing was prepared for a proposal nobody approved")
        XCTAssertEqual(model.captureInboxState(model.captureInbox.item("pad-cap-x")!), .proposalReady, "still awaiting a real approval")

        // Approving the text the Mac actually holds inserts exactly that.
        let good = await model.nearbyCaptureInsert(captureId: "pad-cap-x", approvedDigest: NearbyV1.proposalDigest(real))
        guard case .success(let ack) = good else { return XCTFail("\(good)") }
        XCTAssertEqual(ack.state, "inserted")
        let pending = try XCTUnwrap(model.pendingEdit, "the same one undoable edit the Insert button makes")
        XCTAssertEqual(pending.nsRange, NSRange(location: 6, length: 0))
        model.detachBridge()
    }

    /// A retried tap after a dropped reply reports the existing insertion; it
    /// never produces a second edit (transfer-v1 ledger rule).
    func testASecondTapIsIdempotent() async throws {
        let model = ShellModel()
        model.autoCompile = false
        try await attach(model)
        model.caretUTF16 = 6
        let dq = await model.nearbyDestinationPinningCaretIfNeeded(); let d = try XCTUnwrap(dq)
        let sent = await model.forwardNearbyCapture(try submit(id: "pad-cap-i", to: d), pairId: "padpair")
        guard case .success = sent else { return XCTFail("\(sent)") }
        try await ShellModelBridgeTests.waitUntil { model.captureInboxState(model.captureInbox.item("pad-cap-i")!) == .proposalReady }
        let real = try XCTUnwrap(model.captureInbox.item("pad-cap-i")?.latex)
        let digest = NearbyV1.proposalDigest(real)

        let first = await model.nearbyCaptureInsert(captureId: "pad-cap-i", approvedDigest: digest)
        guard case .success(let a1) = first else { return XCTFail("\(first)") }
        XCTAssertEqual(a1.state, "inserted")
        let pending = try XCTUnwrap(model.pendingEdit)
        model.editApplied(pending, newText: "Hello " + real + "FlashTeX.\n")
        try await ShellModelBridgeTests.waitUntil { model.bridgeCaptures.last?.state == .confirmed }
        let textAfterFirst = model.activeText

        let second = await model.nearbyCaptureInsert(captureId: "pad-cap-i", approvedDigest: digest)
        guard case .success(let a2) = second else { return XCTFail("\(second)") }
        XCTAssertEqual(a2.state, "inserted")
        XCTAssertEqual(model.activeText, textAfterFirst, "the document did not change a second time")
        model.detachBridge()
    }
}
