import FlashTeXDisplayListV3
import XCTest

/// `DL3ViewportHint` (docs/protocol/display-list-v3.md §6.3.1, `viewport-v1`).
@MainActor
final class ViewportHintTests: XCTestCase {
    func testTheProtocolsNames() {
        XCTAssertEqual(DL3.viewportCapability, "viewport-v1")
        XCTAssertEqual(DL3.Kind.cViewport, 0x09)
        XCTAssertEqual(DL3.Kind.name(DL3.Kind.cViewport), "client-viewport")
    }

    func testAJumpIsSentAtOnceAndScrollingWhenItSettles() async throws {
        var sent: [Int] = []
        let hint = DL3ViewportHint { sent.append($0) }
        hint.note(0)
        XCTAssertEqual(sent, [0], "the first page at once")
        hint.note(400)
        XCTAssertEqual(sent, [0, 400], "a jump at once")
        hint.note(401)
        hint.note(402)
        XCTAssertEqual(sent, [0, 400], "scrolling waits for it to settle")
        try await Task.sleep(for: .milliseconds(400))
        XCTAssertEqual(sent, [0, 400, 402], "the page scrolling settled on, once")
        hint.note(402)
        try await Task.sleep(for: .milliseconds(300))
        XCTAssertEqual(sent, [0, 400, 402], "the same page is not sent again")
    }
}
