import XCTest
import AppKit
@testable import FlashTeXMac

/// #1806: the About panel's credits carry the GPL source offer, and the
/// release notes repeat the same line.
final class AboutPanelTests: XCTestCase {
    func testCreditsLinkTheEngineSource() {
        let credits = AboutPanel.credits()
        XCTAssertTrue(credits.string.contains("https://github.com/flash-tex/flashtex"))
        XCTAssertTrue(credits.string.contains("GPL"))
        let range = (credits.string as NSString).range(of: "https://github.com/flash-tex/flashtex")
        let link = credits.attribute(.link, at: range.location, effectiveRange: nil) as? URL
        XCTAssertEqual(link, URL(string: "https://github.com/flash-tex/flashtex"))
    }

    func testReleaseNotesCarryTheSameLine() throws {
        let repo = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
        let workflow = try String(contentsOf: repo.appendingPathComponent(".github/workflows/release.yml"), encoding: .utf8)
        XCTAssertTrue(workflow.contains(AboutPanel.sourceLine), "release.yml notes repeat AboutPanel.sourceLine")
    }
}
