import NearbyClient
import XCTest

/// The capture-companion proof, driven through the UI in the simulator:
/// the UI-test runner hosts a `FakeMac` (loopback TLS-PSK listener with the
/// Mac's parameters, no provider), the app pairs with it via
/// `-flashtexpad-test-mac`, the test draws strokes on the PencilKit canvas
/// (finger drags), sets an instruction, sends, and the runner asserts what
/// the Mac side received: a structurally valid PNG plus the instruction, for
/// the destination the fixture advertised. Also: discard before send, and the
/// bundled-sample-image path (the camera does not exist in the simulator).
final class CaptureFlowUITests: XCTestCase {
    let salt = Data((0..<16).map { UInt8($0 * 3 + 5) })
    let code = "731904"
    var mac: FakeMac!

    override func setUpWithError() throws {
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .landscapeLeft
        let derived = NearbyCrypto.derive(code: code, salt: salt)
        mac = try FakeMac(keys: [.init(identity: derived.pairId, psk: derived.psk, bootstrap: true)], macName: "Runner Mac",
                          destination: NearbyWire.Destination(destinationId: "dest-tikz", projectId: "demo", path: "main.tex", baseRevision: 3))
        mac.start()
        XCTAssertNotEqual(mac.port, 0)
    }

    override func tearDownWithError() throws { mac.stop() }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let a = XCTAttachment(screenshot: app.screenshot())
        a.name = name; a.lifetime = .keepAlways
        add(a)
        Thread.sleep(forTimeInterval: 2) // host-side `xcrun simctl io` window
    }

    private func el(_ app: XCUIApplication, _ id: String) -> XCUIElement {
        app.descendants(matching: .any).matching(identifier: id).firstMatch
    }

    private func text(_ app: XCUIApplication, startingWith p: String) -> XCUIElement {
        app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", p)).firstMatch
    }

    /// The capture canvas is full-screen with the sidebar hidden; its
    /// floating sidebar button brings the sidebar back.
    private func revealSidebar(_ app: XCUIApplication) {
        let button = app.descendants(matching: .any).matching(identifier: "capture.sidebar").firstMatch
        if button.waitForExistence(timeout: 10), button.isHittable { button.tap() }
    }

    private func launchPaired() -> XCUIApplication {
        let app = XCUIApplication()
        // `-flashtexpad-fresh`: wipe the Keychain pairing and the on-disk
        // captures of a previous run so the list starts empty.
        app.launchArguments = ["-flashtexpad-fresh", "-flashtexpad-canvas-defaults", "-flashtexpad-test-mac", "127.0.0.1:\(mac.port):\(NearbyCrypto.hex(salt)):\(NearbyCrypto.fingerprint(salt: salt)):\(code)"]
        app.launch()
        XCTAssertTrue(text(app, startingWith: "Connected to Runner Mac").waitForExistence(timeout: 15), app.debugDescription)
        XCTAssertEqual(mac.hellos.count, 1)
        return app
    }

    private func draw(_ app: XCUIApplication) {
        let canvas = el(app, "capture.canvas") // the SwiftUI wrapper is what XCUITest exposes (ScrollView)
        XCTAssertTrue(canvas.waitForExistence(timeout: 5))
        // A triangle: three finger drags (drawingPolicy = .anyInput).
        // Kept to the left half: the full-screen canvas has floating controls
        // at the top and the Captures panel at the trailing edge.
        let pts = [(0.12, 0.75), (0.45, 0.75), (0.28, 0.45), (0.12, 0.75)]
        for i in 0..<3 {
            let a = canvas.coordinate(withNormalizedOffset: CGVector(dx: pts[i].0, dy: pts[i].1))
            let b = canvas.coordinate(withNormalizedOffset: CGVector(dx: pts[i + 1].0, dy: pts[i + 1].1))
            a.press(forDuration: 0.05, thenDragTo: b)
        }
        // The first simulator launch can take several seconds to publish the
        // PencilKit stroke-count accessibility value; the drawing itself is
        // already complete, so wait for the eventual UI state instead of
        // making the acceptance test load-sensitive.
        XCTAssertTrue(text(app, startingWith: "3 strokes").waitForExistence(timeout: 15), app.debugDescription)
    }

    func testDrawSendReceipt() throws {
        let app = launchPaired()
        draw(app)
        attach(app, "10-canvas-drawn")

        // The default instruction is sent as-is (typing would raise the software
        // keyboard over the buttons; keyboard-free keeps the run deterministic).
        XCTAssertTrue(el(app, "capture.instructions").exists)

        // One tap: Prepare + Send are one step (lane mac-capture-fluid).
        XCTAssertTrue(el(app, "capture.chips").exists, "recent-instruction chips")
        attach(app, "11-capture-ready")
        el(app, "capture.send").tap()
        XCTAssertTrue(text(app, startingWith: "received — Mac inbox").waitForExistence(timeout: 15), app.debugDescription)
        attach(app, "12-capture-received")

        // What the Mac side got, over the real TLS-PSK session.
        XCTAssertEqual(mac.captures.count, 1)
        let cap = try XCTUnwrap(mac.captures.first)
        XCTAssertEqual(cap.destinationId, "dest-tikz")
        XCTAssertEqual(cap.baseRevision, 3)
        XCTAssertEqual(cap.instructions, "Convert to TikZ")
        XCTAssertEqual(cap.image.mimeType, "image/png")
        let png = try XCTUnwrap(Data(base64Encoded: cap.image.dataBase64))
        XCTAssertNil(NearbyWire.checkImage(png, mimeType: "image/png"))
        XCTAssertGreaterThan(png.count, 1000, "a drawn triangle is more than a blank PNG")
        XCTAssertTrue(NearbyWire.isValidID(cap.captureId))
        XCTAssertTrue(text(app, startingWith: "capture_received capture_id=\(cap.captureId) durable=false").exists)

        // Outcome (additive capture_status, polled every 2 s): inbox first, then
        // the runner's Mac reports a proposal — its LaTeX shows read-only — then
        // an insertion, after which the row is final.
        XCTAssertTrue(text(app, startingWith: "Mac: on the Mac (inbox").waitForExistence(timeout: 10), app.debugDescription)
        XCTAssertTrue(mac.statusRequests.contains(cap.captureId))
        mac.setStatus(cap.captureId, state: "proposal_ready", latex: "\\begin{tikzpicture}\\draw (0,0) -- (2,0) -- (1,1.5) -- cycle;\\end{tikzpicture}", note: "awaiting review")
        XCTAssertTrue(text(app, startingWith: "Mac: proposal ready").waitForExistence(timeout: 10), app.debugDescription)
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS %@", "tikzpicture")).firstMatch.waitForExistence(timeout: 5), app.debugDescription)
        attach(app, "16-proposal-ready-latex")
        mac.setStatus(cap.captureId, state: "inserted", latex: "\\begin{tikzpicture}\\end{tikzpicture}", newRevision: 4)
        XCTAssertTrue(text(app, startingWith: "Mac: inserted on the Mac (revision 4)").waitForExistence(timeout: 10), app.debugDescription)
        attach(app, "17-inserted")
        XCTAssertEqual(mac.captures.count, 1, "status polling never re-delivers")
    }

    /// Persistence across a relaunch: the received capture, its receipt and
    /// outcome come back from disk; the Keychain pairing is restored and the
    /// app reconnects with the stored key (no code needed).
    func testRelaunchRestoresCapturesAndPairing() throws {
        var app = launchPaired()
        el(app, "capture.sample").tap()
        XCTAssertTrue(el(app, "capture.pickedImage").waitForExistence(timeout: 5))
        el(app, "capture.send").tap()
        XCTAssertTrue(text(app, startingWith: "received — Mac inbox").waitForExistence(timeout: 15), app.debugDescription)
        let cap = try XCTUnwrap(mac.captures.first)
        mac.setStatus(cap.captureId, state: "inserted", latex: "\\alpha", newRevision: 7)
        XCTAssertTrue(text(app, startingWith: "Mac: inserted on the Mac (revision 7)").waitForExistence(timeout: 10), app.debugDescription)
        XCTAssertTrue(text(app, startingWith: "Inserted on Mac").waitForExistence(timeout: 5), "Inserted on Mac ✓")
        app.terminate()

        // Relaunch without -flashtexpad-fresh and without the pairing argument.
        app = XCUIApplication()
        app.launchArguments = ["-flashtexpad-no-autoreconnect"] // the runner Mac only holds the bootstrap key; auto-reconnect is covered by FluidCaptureTests
        app.launch()
        let row = el(app, "capture.row.\(cap.captureId)")
        if !row.waitForExistence(timeout: 5) { el(app, "capture.capturesToggle").tap() }
        XCTAssertTrue(row.waitForExistence(timeout: 15), app.debugDescription)
        XCTAssertTrue(text(app, startingWith: "received — Mac inbox").exists)
        XCTAssertTrue(text(app, startingWith: "Mac: inserted on the Mac (revision 7)").exists)
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS %@", "alpha")).firstMatch.exists, "the returned LaTeX came back from disk")
        XCTAssertTrue(text(app, startingWith: "Not connected — stored pairing: Runner Mac").exists, app.debugDescription)
        attach(app, "18-relaunch-restored")
        // The Mac link panel shows the Keychain-restored pairing (reconnecting
        // with the stored key against a restarted Mac is proven in
        // FinishTests.testPairingSurvivesInTheKeychainAndCapturesOnDisk).
        revealSidebar(app)
        app.staticTexts.matching(NSPredicate(format: "label == %@", "Mac link")).firstMatch.tap()
        XCTAssertTrue(text(app, startingWith: "Stored in the Keychain (this iPad only): Runner Mac").waitForExistence(timeout: 5), app.debugDescription)
        attach(app, "19-stored-pairing")
    }

    /// Clear before Send: an empty canvas is refused locally; nothing is drafted or sent.
    func testClearBeforeSendSendsNothing() throws {
        let app = launchPaired()
        draw(app)
        el(app, "capture.clear").tap()
        XCTAssertTrue(text(app, startingWith: "0 strokes").waitForExistence(timeout: 5))
        el(app, "capture.send").tap()
        XCTAssertTrue(text(app, startingWith: "draw something first").waitForExistence(timeout: 5), app.debugDescription)
        Thread.sleep(forTimeInterval: 1)
        XCTAssertEqual(mac.captures.count, 0, "nothing reaches the Mac")
        attach(app, "13-capture-cleared")
    }

    func testSampleImagePath() throws {
        let app = launchPaired()
        el(app, "capture.sample").tap()
        XCTAssertTrue(el(app, "capture.pickedImage").waitForExistence(timeout: 5))
        attach(app, "14-sample-image-ready")
        el(app, "capture.send").tap()
        XCTAssertTrue(text(app, startingWith: "received — Mac inbox").waitForExistence(timeout: 15), app.debugDescription)
        XCTAssertEqual(mac.captures.count, 1)
        let png = try XCTUnwrap(Data(base64Encoded: mac.captures[0].image.dataBase64))
        XCTAssertNil(NearbyWire.checkImage(png, mimeType: "image/png"))
        attach(app, "15-sample-image-received")
    }
}
