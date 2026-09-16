import NearbyClient
import XCTest
@testable import FlashTeXPad
@testable import FlashTeXPadKit

/// The Insert button on the Captures screen (nearby-v1 `capture_insert`,
/// protocol/proposals/nearby-v1-companion-insert.md): the iPad approves the
/// proposal it is showing, so the capture lands in the document without a walk
/// to the Mac and a second click there.
///
/// These tests are mostly about what the button *cannot* do. It only exists
/// once there is a proposal to read, it approves that exact text by digest,
/// and a Mac holding anything else refuses.
@MainActor
final class CompanionInsertTests: XCTestCase {
    var mac: FakeMac!
    var model: PadModel!
    let salt = Data((0..<16).map { UInt8($0 + 17) })
    let code = "314159"
    var store: PairFile!

    override func setUp() async throws {
        let d = NearbyCrypto.derive(code: code, salt: salt)
        mac = try FakeMac(keys: [.init(identity: d.pairId, psk: d.psk, bootstrap: true)], macName: "Insert Mac",
                          destination: NearbyWire.Destination(destinationId: "mac-caret-1", projectId: "demo", path: "main.tex", baseRevision: 2))
        mac.start()
        store = try PairFile(url: FileManager.default.temporaryDirectory.appendingPathComponent("insert-pairs-\(UUID()).json"))
        model = PadModel(link: MacLink(store: store))
        model.pollInterval = 0.05
        await model.pair(host: "127.0.0.1", port: String(mac.port), saltHex: NearbyCrypto.hex(salt),
                         fingerprint: NearbyCrypto.fingerprint(salt: salt), macName: "Insert Mac", code: code)
        XCTAssertNil(model.linkError)
    }

    override func tearDown() async throws { model.disconnect(); mac.stop() }

    /// Sends a capture and drives it to `proposal_ready` with `latex`.
    private func captureAwaitingReview(_ latex: String) async throws -> String {
        let r = await model.sendNow(png: CaptureQueueTests.trianglePNG(), source: .pencil, instructions: "matrix")
        guard case .success(let id) = r else { throw XCTSkip("send failed: \(r)") }
        mac.setStatus(id, state: "proposal_ready", latex: latex)
        let deadline = Date().addingTimeInterval(3)
        while model.captures.first(where: { $0.id == id })?.outcome?.state != "proposal_ready", Date() < deadline {
            try await Task.sleep(nanoseconds: 30_000_000)
        }
        return id
    }

    func testTheButtonOnlyExistsOnceThereIsAProposalToRead() async throws {
        let r = await model.sendNow(png: CaptureQueueTests.trianglePNG(), source: .pencil, instructions: "matrix")
        guard case .success(let id) = r else { return XCTFail("\(r)") }
        // Received, converting, no proposal: nothing to approve, so no Insert.
        XCTAssertNil(model.captures.first { $0.id == id }?.reviewableLatex)
        mac.setStatus(id, state: "converting")
        await model.refreshOutcome(id)
        XCTAssertNil(model.captures.first { $0.id == id }?.reviewableLatex)
        // Tapping anyway (a stale view) does nothing and sends nothing.
        let ack = await model.insertCapture(id)
        XCTAssertNil(ack)
        XCTAssertTrue(mac.insertRequests.isEmpty, "no approval was sent for a proposal nobody has seen")
    }

    func testInsertApprovesTheDisplayedProposalAndTheRowBecomesInserted() async throws {
        let latex = "\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}"
        let id = try await captureAwaitingReview(latex)
        let shown = try XCTUnwrap(model.captures.first { $0.id == id }?.reviewableLatex)
        XCTAssertEqual(shown, latex, "what the button approves is what the screen shows")

        let sent = await model.insertCapture(id); let ack = try XCTUnwrap(sent)
        XCTAssertEqual(ack.state, "inserted")
        XCTAssertEqual(ack.newRevision, 9)

        // Exactly one approval went out, naming the displayed text by digest —
        // the text itself is never sent.
        XCTAssertEqual(mac.insertRequests.count, 1)
        XCTAssertEqual(mac.insertRequests.first?.captureId, id)
        XCTAssertEqual(mac.insertRequests.first?.approvedLatexSha256, NearbyWire.proposalDigest(latex))

        let row = try XCTUnwrap(model.captures.first { $0.id == id })
        XCTAssertEqual(row.outcome?.state, "inserted")
        XCTAssertTrue(row.outcomeIsFinal, "polling stops")
        XCTAssertEqual(row.outcome?.latex, latex, "an inserted row still shows what went in")
        XCTAssertNil(row.insertProblem)
        XCTAssertFalse(row.inserting)
        XCTAssertNil(row.reviewableLatex, "the button is gone once it is inserted")
    }

    /// The safety property: if the Mac's proposal is no longer the text this
    /// iPad displayed, the tap is refused and nothing is inserted.
    func testApprovingAProposalTheMacNoLongerHoldsIsRefused() async throws {
        let id = try await captureAwaitingReview("\\alpha")
        // The Mac re-converted (stale context) behind the iPad's back.
        mac.setStatus(id, state: "proposal_ready", latex: "\\beta")

        let ack = await model.insertCapture(id)
        XCTAssertNil(ack, "refused")
        let row = try XCTUnwrap(model.captures.first { $0.id == id })
        XCTAssertNotEqual(row.outcome?.state, "inserted")
        let problem = try XCTUnwrap(row.insertProblem)
        XCTAssertTrue(problem.contains("read it again"), problem)
        XCTAssertEqual(mac.insertRequests.first?.approvedLatexSha256, NearbyWire.proposalDigest("\\alpha"),
                       "the iPad approved what it had shown, and that is what was refused")
    }

    func testAMacThatPredatesTheMessageSaysSoInsteadOfLookingBroken() async throws {
        mac.answersInsert = false
        let id = try await captureAwaitingReview("\\alpha")
        let ack = await model.insertCapture(id)
        XCTAssertNil(ack)
        let problem = try XCTUnwrap(model.captures.first { $0.id == id }?.insertProblem)
        XCTAssertTrue(problem.contains("predates capture_insert"), problem)
        XCTAssertTrue(problem.contains("on the Mac"), "it points at the path that still works")
    }

    func testWithoutALinkNothingIsSentAndTheRowSaysWhy() async throws {
        let id = try await captureAwaitingReview("\\alpha")
        model.disconnect()
        let ack = await model.insertCapture(id)
        XCTAssertNil(ack)
        XCTAssertEqual(model.captures.first { $0.id == id }?.insertProblem, "not connected to the Mac")
        XCTAssertTrue(mac.insertRequests.isEmpty)
    }
}
