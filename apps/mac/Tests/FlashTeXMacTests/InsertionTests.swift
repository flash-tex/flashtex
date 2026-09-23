import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

final class InsertionTests: XCTestCase {
    func testAnchorUsesUTF8BytesAndContext() throws {
        let text = "aé😀 rest of line\nnext"
        let a = try XCTUnwrap(Insertion.makeAnchor(id: "x", path: "m", text: text, caretUTF16: 4, revision: 1)) // after 😀
        XCTAssertEqual(a.byteOffset, 7)
        XCTAssertEqual(a.contextAfter, " rest of line\nnext")
    }

    func testResolveExactRebasedAndReselection() {
        let a = InsertionAnchor(id: "x", path: "m", byteOffset: 6, revision: 1, contextAfter: "world")
        XCTAssertEqual(Insertion.resolve(a, in: "hello world", revision: 1), .exact(byteOffset: 6))
        XCTAssertEqual(Insertion.resolve(a, in: "hey", revision: 1), .needsReselection("anchor beyond end of buffer"))
        XCTAssertEqual(Insertion.resolve(a, in: "well hello world", revision: 2), .rebased(byteOffset: 11))
        XCTAssertEqual(Insertion.resolve(a, in: "hello there", revision: 2), .needsReselection("destination text was deleted or changed"))
        XCTAssertEqual(Insertion.resolve(a, in: "world xworld", revision: 2), .needsReselection("destination is ambiguous after edits"))
        XCTAssertEqual(Insertion.resolve(a, in: "hello world world", revision: 2), .rebased(byteOffset: 6), "keeps original offset when it is still one of the matches")
    }

    func testInsertionTextAddsNewlinesOnlyWhenNeeded() {
        XCTAssertEqual(Insertion.insertionText("  x  ", into: "a\nb", atByte: 2), "x\n")
        XCTAssertEqual(Insertion.insertionText("x", into: "ab", atByte: 1), "\nx\n")
        XCTAssertEqual(Insertion.insertionText("x", into: "ab\n", atByte: 3), "x")
        XCTAssertEqual(Insertion.insertionText("x", into: "", atByte: 0), "x")
    }

    func testSampleProposalDecodes() throws {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("Samples/capture-proposal.json")
        let env = try RuntimeV1.decodeCaptureProposal(Data(contentsOf: url))
        XCTAssertEqual(env.payload.captureId, "sample-capture-1")
        XCTAssertEqual(env.payload.requiredDependencies, ["amsmath"])
    }

    func testCaptureSubmitFixtureDecodesAndMimeIsAccepted() throws {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        let env = try RuntimeV1.decodeCaptureSubmit(Data(contentsOf: url.appendingPathComponent("protocol/fixtures/capture-submission.json")))
        XCTAssertEqual(env.payload.destinationId, "fixture-anchor-1")
        XCTAssertTrue(RuntimeV1.acceptedCaptureMimeTypes.contains(env.payload.image.mimeType))
    }
}

@MainActor
final class ShellModelCaptureTests: XCTestCase {
    func testReviewFlowInsertsOnceAndRequiresAnchor() {
        let model = ShellModel()
        let p = RuntimeV1.CaptureProposal(captureId: "c1", latex: "x^2", ambiguities: [], requiredDependencies: [])
        model.enqueue(p)
        XCTAssertEqual(model.reviewing, p)
        XCTAssertEqual(model.approveProposal(p, latex: "x^2"), .noAnchor)

        model.caretUTF16 = 5 // "Hello| FlashTeX.\n"
        model.pinAnchorAtCaret()
        XCTAssertEqual(model.anchor?.byteOffset, 5)
        XCTAssertEqual(model.approveProposal(p, latex: " y^2 "), .inserted(byteOffset: 5))
        let edit = model.pendingEdit!
        XCTAssertEqual(edit.nsRange, NSRange(location: 5, length: 0))
        // `y^2` is bare mathematics recognised at a mid-sentence text caret, so
        // it is wrapped as inline math. It used to be inserted raw, between
        // newlines, which typeset the `^` as a literal character and broke the
        // sentence across lines (issue #2, owner report).
        XCTAssertEqual(edit.text, "$y^2$")
        XCTAssertTrue(model.proposals.isEmpty)
        XCTAssertNil(model.reviewing)
        // Editor applies the edit and reports the new buffer.
        model.editApplied(edit, newText: "Hello$y^2$ FlashTeX.\n")
        XCTAssertNil(model.pendingEdit)
        XCTAssertEqual(model.activeText, "Hello$y^2$ FlashTeX.\n")
        XCTAssertEqual(model.anchor?.byteOffset, 10, "anchor advances past inserted text")

        // Duplicate capture ID never inserts twice.
        model.enqueue(p)
        XCTAssertTrue(model.proposals.isEmpty)
        XCTAssertTrue(model.captureNote?.contains("duplicate") == true)
        XCTAssertEqual(model.approveProposal(p, latex: "again"), .duplicate)

        // Second proposal appends after the first at the advanced anchor (rebased via context).
        let p2 = RuntimeV1.CaptureProposal(captureId: "c2", latex: "z", ambiguities: [], requiredDependencies: [])
        model.enqueue(p2)
        XCTAssertEqual(model.approveProposal(p2, latex: "z"), .inserted(byteOffset: 10))
        let edit2 = model.pendingEdit!
        // `z` carries no math marker, so it is prose and is inserted untouched:
        // undelimited text is never guessed into mathematics.
        XCTAssertEqual(edit2.text, "z")
        model.editApplied(edit2, newText: "Hello$y^2$z FlashTeX.\n")
        XCTAssertEqual(model.anchor?.byteOffset, 11)
        XCTAssertEqual(model.anchor?.revision, model.editorRevision)

        // Deleting the destination context forces reselection.
        let p3 = RuntimeV1.CaptureProposal(captureId: "c3", latex: "w", ambiguities: [], requiredDependencies: [])
        model.updateActiveText("totally different")
        model.enqueue(p3)
        if case .needsReselection = model.approveProposal(p3, latex: "w") {} else { XCTFail("expected reselection") }
        XCTAssertNil(model.anchor)
    }
}
