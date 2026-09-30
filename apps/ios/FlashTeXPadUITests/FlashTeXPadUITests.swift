import XCTest

/// Drives the app in the simulator: opens the bundled sample, then the review
/// fixture, cancels, re-opens, approves, and checks the receipt is shown.
/// Runs in landscape so the NavigationSplitView sidebar stays visible.
/// Screenshots are taken from the host with `xcrun simctl io <udid> screenshot`
/// during the 2 s pauses (docs/evidence/ios-acceptance-2026-09-12/);
/// XCTAttachments are kept in the xcresult too.
final class FlashTeXPadUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .landscapeLeft
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let a = XCTAttachment(screenshot: app.screenshot())
        a.name = name; a.lifetime = .keepAlways
        add(a)
        Thread.sleep(forTimeInterval: 2) // host-side simctl screenshot window
    }

    private func el(_ app: XCUIApplication, _ id: String) -> XCUIElement {
        app.descendants(matching: .any).matching(identifier: id).firstMatch
    }

    /// First static text whose label starts with `prefix` (SwiftUI Text labels
    /// are matched by content; identifiers may land on a container).
    private func text(_ app: XCUIApplication, startingWith prefix: String) -> XCUIElement {
        app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", prefix)).firstMatch
    }

    private func text(_ app: XCUIApplication, containing s: String) -> XCUIElement {
        app.staticTexts.matching(NSPredicate(format: "label CONTAINS %@", s)).firstMatch
    }

    func testOpenSampleThenCancelThenApprove() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(el(app, "open.sample").waitForExistence(timeout: 10))
        el(app, "open.sample").tap()
        app.staticTexts.matching(NSPredicate(format: "label == %@", "Editor")).firstMatch.tap()
        XCTAssertTrue(text(app, startingWith: "demo.tex · revision 1").waitForExistence(timeout: 5), app.debugDescription)
        XCTAssertTrue(app.textViews.firstMatch.waitForExistence(timeout: 5))
        XCTAssertTrue(String(describing: app.textViews.firstMatch.value).contains("FlashTeX demo"))
        attach(app, "01-sample-open")

        // Review fixture → cancel
        el(app, "open.fixture").tap()
        app.staticTexts.matching(NSPredicate(format: "label == %@", "Review")).firstMatch.tap()
        XCTAssertTrue(text(app, startingWith: "pending — nothing inserted").waitForExistence(timeout: 5), app.debugDescription)
        attach(app, "02-review-pending")
        el(app, "review.cancel").tap()
        XCTAssertTrue(text(app, startingWith: "cancelled (").waitForExistence(timeout: 5))
        XCTAssertFalse(text(app, startingWith: "insertions this session").exists)
        attach(app, "03-review-cancelled")

        // Re-open fixture → approve
        el(app, "open.fixture").tap()
        XCTAssertTrue(text(app, startingWith: "pending — nothing inserted").waitForExistence(timeout: 5))
        el(app, "review.approve").tap()
        XCTAssertTrue(text(app, startingWith: "applied once — revision 2").waitForExistence(timeout: 5), app.debugDescription)
        XCTAssertTrue(text(app, startingWith: "command assistant-144af470").exists, "receipt echoed")
        XCTAssertTrue(text(app, startingWith: "review 144af470").exists)
        XCTAssertFalse(el(app, "review.approve").isEnabled)
        attach(app, "04-review-applied")

        app.staticTexts.matching(NSPredicate(format: "label == %@", "Editor")).firstMatch.tap()
        XCTAssertTrue(app.textViews.firstMatch.waitForExistence(timeout: 5))
        XCTAssertTrue(String(describing: app.textViews.firstMatch.value).contains("Example text"))
        XCTAssertTrue(text(app, startingWith: "main.tex · revision 2").exists)
        attach(app, "05-editor-after-insert")

        app.staticTexts.matching(NSPredicate(format: "label == %@", "Diagnostics")).firstMatch.tap()
        XCTAssertTrue(text(app, startingWith: "Source: runtime-v1 fixture").waitForExistence(timeout: 5))
        XCTAssertTrue(text(app, containing: "unknowncommand is not supported").exists)
        attach(app, "06-diagnostics")

        app.staticTexts.matching(NSPredicate(format: "label == %@", "Mac link")).firstMatch.tap()
        XCTAssertTrue(text(app, startingWith: "not paired").waitForExistence(timeout: 5)
                      || text(app, startingWith: "stored pairing").waitForExistence(timeout: 1))
        attach(app, "07-mac-link")
    }

    func testAccessibilityLabelsForCaptureStatusAndPairingControls() throws {
        let app = XCUIApplication()
        app.launch()

        let capture = el(app, "capture.sample")
        XCTAssertTrue(capture.waitForExistence(timeout: 10))
        XCTAssertFalse(capture.label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)

        let status = el(app, "capture.connection")
        XCTAssertTrue(status.exists)
        XCTAssertFalse(status.label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)

        app.staticTexts.matching(NSPredicate(format: "label == %@", "Mac link")).firstMatch.tap()
        let pairing = el(app, "pair.browse")
        XCTAssertTrue(pairing.waitForExistence(timeout: 5))
        XCTAssertFalse(pairing.label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
    }
}
