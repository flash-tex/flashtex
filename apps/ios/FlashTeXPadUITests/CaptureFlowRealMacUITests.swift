import UIKit
import XCTest

/// Opt-in end-to-end against a **real** FlashTeX Mac process (not FakeMac).
/// Skipped unless `FLASHTEX_PAD_E2E_MAC=1` plus either discrete host/port/
/// bootstrap vars or `FLASHTEX_PAD_E2E_INFO`, so `xcodebuild test` on CI is
/// unchanged. The XCUITest runner lives **in the simulator**: host env is
/// invisible unless xcodebuild prefixes it `TEST_RUNNER_` (Xcode strips that
/// prefix when injecting the runner process). A Mac `/tmp` JSON path is also
/// invisible to the simulator, so prefer:
///
///   TEST_RUNNER_FLASHTEX_PAD_E2E_MAC=1
///   TEST_RUNNER_FLASHTEX_PAD_E2E_HOST=127.0.0.1
///   TEST_RUNNER_FLASHTEX_PAD_E2E_PORT=<port>
///   TEST_RUNNER_FLASHTEX_PAD_E2E_BOOTSTRAP='flashtex-nearby://pair?…'
///
/// The test types host/port (host already defaults to 127.0.0.1), types the
/// bootstrap URL (Paste trips a ~60s Allow Paste idle in the simulator),
/// sends the bundled sample image and a finger-drawn triangle, and
/// asserts both receipts. No provider call is involved on either side.
final class CaptureFlowRealMacUITests: XCTestCase {
    private struct MacInfo: Decodable {
        var host: String
        var port: UInt16
        var bootstrap: String
        var name: String?
    }

    override func setUpWithError() throws {
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .landscapeLeft
    }

    private func el(_ app: XCUIApplication, _ id: String) -> XCUIElement {
        app.descendants(matching: .any).matching(identifier: id).firstMatch
    }

    /// SwiftUI `Form` cells below the fold are not in the accessibility tree
    /// until they have been scrolled on-screen (NavigationSplitView + iPad
    /// landscape). Swipe the already-visible QR field so the sidebar is not
    /// the swipe target.
    @discardableResult
    private func reveal(_ app: XCUIApplication, _ id: String, timeout: TimeInterval = 10) -> XCUIElement {
        let target = el(app, id)
        if target.waitForExistence(timeout: 0.4) { return target }
        let anchor = el(app, "pair.qr.text")
        let deadline = Date().addingTimeInterval(timeout)
        var n = 0
        while Date() < deadline, !target.exists {
            let up = n % 6 < 4
            if anchor.exists { up ? anchor.swipeUp() : anchor.swipeDown() }
            else { up ? app.swipeUp() : app.swipeDown() }
            n += 1
        }
        return target
    }

    private func text(_ app: XCUIApplication, startingWith p: String) -> XCUIElement {
        app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", p)).firstMatch
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let a = XCTAttachment(screenshot: app.screenshot())
        a.name = name; a.lifetime = .keepAlways
        add(a)
    }

    private func loadInfo() throws -> MacInfo {
        let env = ProcessInfo.processInfo.environment
        guard env["FLASHTEX_PAD_E2E_MAC"] == "1" else {
            throw XCTSkip("set FLASHTEX_PAD_E2E_MAC=1 (xcodebuild TEST_RUNNER_FLASHTEX_PAD_E2E_MAC=1) to run against a real Mac")
        }
        if let host = env["FLASHTEX_PAD_E2E_HOST"], let portS = env["FLASHTEX_PAD_E2E_PORT"],
           let port = UInt16(portS), let bootstrap = env["FLASHTEX_PAD_E2E_BOOTSTRAP"],
           !host.isEmpty, !bootstrap.isEmpty {
            return MacInfo(host: host, port: port, bootstrap: bootstrap, name: env["FLASHTEX_PAD_E2E_NAME"])
        }
        guard let path = env["FLASHTEX_PAD_E2E_INFO"], !path.isEmpty else {
            throw XCTSkip("FLASHTEX_PAD_E2E_HOST/PORT/BOOTSTRAP or FLASHTEX_PAD_E2E_INFO is missing")
        }
        let data = try Data(contentsOf: URL(fileURLWithPath: path))
        return try JSONDecoder().decode(MacInfo.self, from: data)
    }

    /// Pair by typed loopback host/port + typed bootstrap URL (Paste is the
    /// product path but the simulator's Allow Paste prompt idles ~60s and can
    /// miss the Mac's 120s code), then send
    /// sample-capture.png and a synthetic drawing. Both must show a Mac inbox
    /// receipt (provider none: durable=false, no proposal).
    func testPairByPastedBootstrapThenSendSampleAndDrawing() throws {
        let info = try loadInfo()
        let app = XCUIApplication()
        app.launchArguments = ["-flashtexpad-fresh"]
        app.launch()

        app.staticTexts.matching(NSPredicate(format: "label == %@", "Mac link")).firstMatch.tap()
        XCTAssertTrue(el(app, "pair.qr.text").waitForExistence(timeout: 10), "Mac link QR field missing")

        // Host/port first: the QR Paste button trips a 60s "Allow Paste" idle
        // wait that can outlive the Mac's 120s pairing code. Loopback host is
        // already 127.0.0.1; do not re-type it (that duplicates the value).
        if info.host != "127.0.0.1" {
            let hostField = reveal(app, "pair.host")
            XCTAssertTrue(hostField.waitForExistence(timeout: 2), "pair.host missing")
            hostField.tap()
            hostField.typeText(info.host)
        }
        let portField = reveal(app, "pair.port")
        XCTAssertTrue(portField.waitForExistence(timeout: 2), "pair.port still missing after scrolling the Form")
        portField.tap()
        portField.typeText(String(info.port))

        let qr = reveal(app, "pair.qr.text")
        XCTAssertTrue(qr.waitForExistence(timeout: 2), "pair.qr.text after port")
        qr.tap()
        qr.typeText(info.bootstrap)

        let go = reveal(app, "pair.qr.go")
        XCTAssertTrue(go.waitForExistence(timeout: 5) && go.isEnabled, "bootstrap URL should enable Pair from payload")
        attach(app, "e2e-01-mac-link-filled")
        go.tap()

        // Link status sits above the fold we scrolled; Capture shows "Connected to".
        app.staticTexts.matching(NSPredicate(format: "label == %@", "Capture")).firstMatch.tap()
        XCTAssertTrue(el(app, "capture.sample").waitForExistence(timeout: 10), "Capture panel")
        XCTAssertTrue(text(app, startingWith: "Connected to").waitForExistence(timeout: 25),
                      "pairing did not connect")
        attach(app, "e2e-02-paired")

        el(app, "capture.sample").tap()
        XCTAssertTrue(el(app, "capture.pickedImage").waitForExistence(timeout: 5))
        attach(app, "e2e-03-sample-ready")
        el(app, "capture.send").tap()
        XCTAssertTrue(text(app, startingWith: "received — Mac inbox").waitForExistence(timeout: 20), app.debugDescription)
        attach(app, "e2e-04-sample-received")

        // Back to the canvas for a synthetic (finger) drawing. The first
        // capture shrinks the canvas; three drags still produce strokes.
        if el(app, "capture.pickedImage").exists {
            app.buttons.matching(NSPredicate(format: "label == %@", "Back to canvas")).firstMatch.tap()
        }
        let canvas = el(app, "capture.canvas")
        XCTAssertTrue(canvas.waitForExistence(timeout: 5))
        let pts = [(0.2, 0.8), (0.8, 0.8), (0.5, 0.2), (0.2, 0.8)]
        for i in 0..<3 {
            let a = canvas.coordinate(withNormalizedOffset: CGVector(dx: pts[i].0, dy: pts[i].1))
            let b = canvas.coordinate(withNormalizedOffset: CGVector(dx: pts[i + 1].0, dy: pts[i + 1].1))
            a.press(forDuration: 0.05, thenDragTo: b)
        }
        XCTAssertTrue(text(app, startingWith: "3 strokes").waitForExistence(timeout: 15), app.debugDescription)
        attach(app, "e2e-05-drawing-ready")
        el(app, "capture.send").tap()
        let received = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", "received — Mac inbox"))
        XCTAssertTrue(received.element(boundBy: 1).waitForExistence(timeout: 20), app.debugDescription)
        attach(app, "e2e-06-drawing-received")
        XCTAssertGreaterThanOrEqual(received.count, 2, "sample + drawing both received: \(app.debugDescription)")
    }
}
