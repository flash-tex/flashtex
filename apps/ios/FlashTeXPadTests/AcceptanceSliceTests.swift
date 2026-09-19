import FlashTeXPadKit
import FlashTeXProtocol
import NearbyClient
import XCTest
@testable import FlashTeXPad

/// The acceptance slice, run in the iPad simulator against a Mac-side fixture
/// hosted in this test process (FakeMac, loopback TLS-PSK — no live provider):
///   (a) open the bundled sample .tex;
///   (b) pair with the pairing code, send one capture over the real nearby-v1
///       transport and get its `capture_received` echoed; attach the recorded
///       reviewed proposal (assistant-context fixture);
///   (c) cancel the pending proposal → nothing inserted;
///   (d) approve → exactly one insertion, receipt echoed, second approve refused.
@MainActor
final class AcceptanceSliceTests: XCTestCase {
    var mac: FakeMac!
    var model: PadModel!
    let salt = Data((0..<16).map { UInt8($0 * 7 + 1) })
    let code = "482913"

    override func setUp() async throws {
        let derived = NearbyCrypto.derive(code: code, salt: salt)
        mac = try FakeMac(keys: [.init(identity: derived.pairId, psk: derived.psk, bootstrap: true)], macName: "Fixture Mac",
                          destination: NearbyWire.Destination(destinationId: "dest-1", projectId: "review-fixture", path: "main.tex", baseRevision: 1))
        mac.start()
        XCTAssertNotEqual(mac.port, 0, "FakeMac did not bind")
        model = PadModel(link: MacLink(store: nil))
    }

    override func tearDown() async throws {
        model.disconnect()
        mac.stop()
    }

    // (a)
    func testOpensBundledSample() throws {
        model.openBundledSample()
        let d = try XCTUnwrap(model.document)
        XCTAssertEqual(d.path, "demo.tex")
        XCTAssertTrue(d.text.contains("\\section{Résumé of a small journey}"))
        XCTAssertEqual(d.revision, 1)
        XCTAssertNil(model.openError)
        // UTF-8 offset discipline on non-ASCII text: caret inside "Résumé".
        let caretByte = try XCTUnwrap(d.byteOffset(ofUTF16: (d.text as NSString).range(of: "sumé").location))
        XCTAssertEqual(d.slice(startByte: caretByte, endByte: caretByte + 5), "sumé")
    }

    // (b) real transport: pairing-code bootstrap, hello_ack, capture_submit → capture_received
    func testPairAndCaptureReceiptOverNearbyV1() async throws {
        model.openBundledSample()
        await model.pair(host: "127.0.0.1", port: String(mac.port), saltHex: NearbyCrypto.hex(salt),
                         fingerprint: NearbyCrypto.fingerprint(salt: salt), macName: "Fixture Mac", code: code)
        XCTAssertNil(model.linkError)
        XCTAssertTrue(model.link.isConnected)
        XCTAssertEqual(model.pairedMac?.macName, "Fixture Mac")
        XCTAssertEqual(model.destination?.destinationId, "dest-1")
        XCTAssertEqual(mac.hellos.count, 1)

        let maybeReceipt = await model.sendTestCapture(instructions: "transcribe")
        let receipt = try XCTUnwrap(maybeReceipt)
        XCTAssertEqual(mac.captures.count, 1)
        XCTAssertEqual(mac.captures.first?.captureId, receipt.captureId)
        XCTAssertEqual(mac.captures.first?.destinationId, "dest-1")
        XCTAssertEqual(mac.captures.first?.baseRevision, 1)
        XCTAssertFalse(receipt.durable, "in-memory inbox answers durable:false")
        XCTAssertFalse(receipt.hasProposal, "nearby-v1 never returns a proposal to the companion")
        XCTAssertEqual(model.lastCaptureReceived, receipt)
        let sent = model.link.transcript.filter { $0.direction == .sent }.map(\.text)
        XCTAssertTrue(sent.contains { $0.contains("\"type\":\"hello\"") })
        XCTAssertTrue(sent.contains { $0.contains("\"type\":\"capture_submit\"") && $0.contains("<image bytes>") })
        XCTAssertTrue(model.link.transcript.contains { $0.direction == .received && $0.text.contains("\"type\":\"capture_received\"") })
        XCTAssertFalse(model.link.transcript.contains { $0.text.contains(mac.longTermPSK.base64EncodedString()) }, "pair_psk must be redacted")
        // Evidence: the redacted wire transcript as the iPad saw it (stdout of the test run).
        for l in model.link.transcript { print("NEARBY-TRANSCRIPT \(l.direction.rawValue) \(l.text)") }
    }

    func testWrongPairingCodeIsRefused() async throws {
        await model.pair(host: "127.0.0.1", port: String(mac.port), saltHex: NearbyCrypto.hex(salt),
                         fingerprint: NearbyCrypto.fingerprint(salt: salt), macName: "Fixture Mac", code: "000000")
        XCTAssertNotNil(model.linkError)
        XCTAssertFalse(model.link.isConnected)
        XCTAssertTrue(mac.captures.isEmpty)
    }

    // (c) cancel: nothing inserted
    func testCancelPendingProposalInsertsNothing() throws {
        model.openReviewFixture()
        let before = try XCTUnwrap(model.document)
        XCTAssertEqual(before.sha256Hex, model.review?.approved.payload.group.expectedSha256)
        XCTAssertTrue(model.review?.isPending == true)
        XCTAssertEqual(model.diagnostics.count, 1)
        XCTAssertEqual(model.diagnostics.first?.excerpt, "\\unknowncommand")

        model.cancelReview()
        XCTAssertEqual(model.document, before)
        XCTAssertEqual(model.insertionCount, 0)
        XCTAssertNil(model.lastReceipt)
        if case .cancelled = model.review!.state {} else { XCTFail("expected cancelled, got \(model.review!.state)") }
        // A cancelled review cannot be approved afterwards.
        model.approveReview()
        XCTAssertEqual(model.document, before)
        XCTAssertEqual(model.insertionCount, 0)
    }

    // (d) approve: exactly one insertion + receipt
    func testApproveInsertsExactlyOnceAndEchoesReceipt() async throws {
        // Pair first so the flow is the full one: capture over the wire, then review locally.
        await model.pair(host: "127.0.0.1", port: String(mac.port), saltHex: NearbyCrypto.hex(salt),
                         fingerprint: NearbyCrypto.fingerprint(salt: salt), macName: "Fixture Mac", code: code)
        XCTAssertNil(model.linkError)
        model.openReviewFixture()
        let maybeCapture = await model.sendTestCapture(instructions: "explain the unknown command")
        let capture = try XCTUnwrap(maybeCapture)
        XCTAssertEqual(mac.captures.count, 1)
        XCTAssertEqual(capture.captureId, mac.captures[0].captureId)

        let before = try XCTUnwrap(model.document)
        let review = try XCTUnwrap(model.review)
        model.approveReview()
        XCTAssertNil(model.reviewError)
        let after = try XCTUnwrap(model.document)
        let receipt = try XCTUnwrap(model.lastReceipt)
        XCTAssertEqual(model.insertionCount, 1)
        XCTAssertEqual(after.revision, before.revision + 1)
        XCTAssertEqual(after.text, before.text.replacingOccurrences(of: "\\unknowncommand", with: "Example text"))
        XCTAssertEqual(after.text.components(separatedBy: "Example text").count - 1, 1, "inserted exactly once")
        XCTAssertEqual(receipt.commandId, review.approved.payload.group.commandId)
        XCTAssertEqual(receipt.reviewId, review.review.reviewId)
        XCTAssertEqual(receipt.newRevision, after.revision)
        XCTAssertEqual(receipt.sha256After, after.sha256Hex)
        XCTAssertEqual(receipt.insertedRanges.count, 1)
        XCTAssertEqual(after.slice(startByte: receipt.insertedRanges[0].start, endByte: receipt.insertedRanges[0].end), "Example text")

        // Second approve is refused and inserts nothing.
        model.approveReview()
        XCTAssertEqual(model.document, after)
        XCTAssertEqual(model.insertionCount, 1)
        XCTAssertEqual(model.reviewError, ReviewSession.ApplyError.notPending.description)
    }

    func testApproveRefusedWhenBufferDrifted() throws {
        model.openReviewFixture()
        var d = try XCTUnwrap(model.document)
        d.text += "% drift\n"
        model.textChanged(d.text)
        let drifted = try XCTUnwrap(model.document)
        model.approveReview()
        XCTAssertEqual(model.document, drifted, "refused approval must not write")
        XCTAssertEqual(model.insertionCount, 0)
        XCTAssertNotNil(model.reviewError)
        if case .refused = model.review!.state {} else { XCTFail("expected refused") }
    }

    func testLocalCompletionsFromSource() async {
        model.openBundledSample()
        let text = model.document!.text
        let caret = (text as NSString).range(of: "\\subsection{At the café}").location + 4 // after "\sub"
        model.caretMoved(caret)
        await model.settleCompletions() // computed off the main actor after the debounce
        XCTAssertTrue(model.completions.contains { $0.text == "\\subsection" })
        let endCaret = (text as NSString).length
        model.caretMoved(endCaret)
        await model.settleCompletions()
        // Document has an unclosed \begin{document}? demo.tex closes it; check the helper directly.
        XCTAssertEqual(LocalCompletion.openEnvironments(in: "\\begin{document}\\begin{itemize}"), ["document", "itemize"])
        let s = LocalCompletion.suggestions(in: "\\begin{itemize}\n\\e", caretByte: 18)
        XCTAssertEqual(s.first?.text, "\\end{itemize}")
    }
}
