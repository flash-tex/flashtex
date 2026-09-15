import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The owner's report: "when the popup shows up in the editor, the insert
/// button doesnt work (it also doesnt work when you pull up the sidebar with
/// all the past responses)".
///
/// Both contexts are one mechanism. `enqueue` raises the window-modal review
/// sheet automatically (`if reviewing == nil { reviewing = proposals.first }`),
/// and a window-modal sheet swallows clicks aimed at the rest of the window —
/// including the Captures inspector's "Insert at caret". That button is
/// enabled only while the capture is in `proposals`, which is precisely the
/// condition that raised the sheet, so the two deadlock: the sidebar's Insert
/// can never be clicked for a proposal it is able to act on.
///
/// The second half is that every refusal is reported through `captureNote`,
/// which renders in the status bar and the inspector header — both behind the
/// sheet. A refused insertion therefore looked identical to a dead button.
@MainActor
final class ProposalReviewDismissalTests: XCTestCase {
    private func proposal(_ id: String) -> RuntimeV1.CaptureProposal {
        RuntimeV1.CaptureProposal(captureId: id, latex: "\\alpha", ambiguities: [], requiredDependencies: [])
    }

    /// Closing the sheet must not decide anything: the proposal stays queued,
    /// so the Captures inspector can still insert it.
    func testClosingTheReviewSheetLeavesTheProposalInsertableFromTheInspector() {
        let model = ShellModel()
        model.enqueue(proposal("cap-1"))
        XCTAssertEqual(model.reviewing?.captureId, "cap-1", "the sheet is raised automatically")
        XCTAssertEqual(model.proposals.map(\.captureId), ["cap-1"])

        model.dismissReviewWithoutDeciding()

        XCTAssertNil(model.reviewing, "the sheet is down, so the window is clickable again")
        XCTAssertEqual(model.proposals.map(\.captureId), ["cap-1"],
                       "closing is not rejecting — the inspector's Insert stays enabled")
        let item = CaptureInbox.Item(id: "cap-1", receivedAt: Date(), pairId: "p", instructions: "i",
                                     mimeType: "image/png", image: Data(), destinationId: "d", baseRevision: 1,
                                     autoPinned: true)
        XCTAssertNotNil(model.captureInboxProposal(item),
                        "Insert at caret is enabled only while the proposal is queued")
        XCTAssertEqual(model.captureInboxState(item), .proposalReady)
    }

    /// Closing must not silently swallow the capture: the user is told where
    /// it went, in the place that is now visible.
    func testClosingExplainsWhereTheCaptureWent() {
        let model = ShellModel()
        model.enqueue(proposal("cap-2"))
        model.dismissReviewWithoutDeciding()
        let note = model.captureNote ?? ""
        XCTAssertTrue(note.contains("cap-2"), note)
        XCTAssertTrue(note.contains("Captures inspector"), note)
    }

    /// Closing with nothing under review is a no-op, not a crash or a stray note.
    func testClosingWithNothingUnderReviewDoesNothing() {
        let model = ShellModel()
        model.captureNote = nil
        model.dismissReviewWithoutDeciding()
        XCTAssertNil(model.reviewing)
        XCTAssertNil(model.captureNote)
    }

    /// A bridge insertion the editor refuses builds its `pendingEdit` without a
    /// `captureRefund`, so it took the branch that only wrote `navigationNote`
    /// — the status bar, behind the sheet. Nothing reached `captureNote`, which
    /// is what the inspector and the sheet show, so Insert looked dead.
    func testRefusedInsertWithoutARefundIsReportedWhereTheUserCanSeeIt() {
        let model = ShellModel()
        let edit = ShellModel.PendingEdit(path: "main.tex", nsRange: NSRange(location: 0, length: 0),
                               text: "\\alpha", token: 1)
        model.captureNote = nil

        model.editRefused(edit, reason: "the buffer moved on")

        XCTAssertNotNil(model.navigationNote)
        let note = try? XCTUnwrap(model.captureNote)
        XCTAssertNotNil(note, "a refused insertion must say so where the capture UI shows notes")
        XCTAssertTrue(model.captureNote?.contains("the buffer moved on") == true, model.captureNote ?? "nil")
        XCTAssertTrue(model.captureNote?.contains("Nothing was inserted") == true, model.captureNote ?? "nil")
    }
}
