import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Cross-language gate for the `dl2-canon-1` header digest of a v2
/// diagnostic's `suggestion` (GH-277): the Swift consumer
/// (`DisplayListDelta.headerDigest`) must produce the
/// `expected_header_digest` recorded in
/// `protocol/fixtures/display-list-v2-delta-digest.json` for each case — the
/// SAME file (and the same digests) the Rust producer test
/// (`crates/render-pipeline/tests/display_list_delta_digest.rs`) asserts. A
/// silent disagreement here would make the Mac app reject every delta and
/// fall back to full lists.
final class DisplayListDeltaDigestTests: XCTestCase {
    /// The shared fixture, loaded in place (never copied): from
    /// `apps/mac/Tests/FlashTeXMacTests` up five levels is the repo root.
    static let digestFixture: URL = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .appendingPathComponent("protocol/fixtures/display-list-v2-delta-digest.json")

    struct Fixture: Decodable {
        struct Case: Decodable {
            var name: String
            var displayList: RenderingV2.Envelope
            var expectedHeaderDigest: String
            enum CodingKeys: String, CodingKey {
                case name, displayList = "display_list", expectedHeaderDigest = "expected_header_digest"
            }
        }
        var cases: [Case]
    }

    func testHeaderDigestMatchesFixture() throws {
        let fixture = try JSONDecoder().decode(Fixture.self, from: Data(contentsOf: Self.digestFixture))
        XCTAssertEqual(fixture.cases.count, 4, "the fixture must carry the four diagnostic/suggestion cases")
        for c in fixture.cases {
            XCTAssertNoThrow(try RenderingV2.validate(c.displayList.payload), "case \(c.name): fixture must be a valid display list")
            XCTAssertEqual(DisplayListDelta.hex(DisplayListDelta.headerDigest(c.displayList.payload)), c.expectedHeaderDigest, "case \(c.name)")
        }
    }
}
