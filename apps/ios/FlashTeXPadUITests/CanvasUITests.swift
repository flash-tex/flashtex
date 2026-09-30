import XCTest

/// The full-screen capture canvas in the simulator: it fills the screen in
/// landscape and portrait, a rotation keeps every stroke, two-finger
/// double-tap undoes (with an "Undo" toast), three-finger double-tap redoes,
/// the gestures can be turned off in the canvas settings, and the sidebar is
/// one tap away. No Mac is needed. Screenshots are kept as attachments
/// (docs/evidence/ipad-canvas-2026-09-30/).
final class CanvasUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .landscapeLeft
    }

    override func tearDownWithError() throws {
        XCUIDevice.shared.orientation = .landscapeLeft
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let a = XCTAttachment(screenshot: app.screenshot())
        a.name = name; a.lifetime = .keepAlways
        add(a)
    }

    private func el(_ app: XCUIApplication, _ id: String) -> XCUIElement {
        app.descendants(matching: .any).matching(identifier: id).firstMatch
    }

    private func text(_ app: XCUIApplication, startingWith p: String) -> XCUIElement {
        app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", p)).firstMatch
    }

    /// Tap the canvas's left edge, away from the popover and the ink.
    private func dismissPopover(_ app: XCUIApplication) {
        el(app, "capture.canvas").coordinate(withNormalizedOffset: CGVector(dx: 0.03, dy: 0.5)).tap()
        XCTAssertTrue(el(app, "canvas.settings.twoFingerUndo").waitForNonExistence(timeout: 5), "popover dismissed")
    }

    private func launch() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments = ["-flashtexpad-fresh", "-flashtexpad-no-autoreconnect", "-flashtexpad-canvas-defaults"]
        app.launch()
        XCTAssertTrue(el(app, "capture.canvas").waitForExistence(timeout: 15), app.debugDescription)
        return app
    }

    /// A triangle of three finger strokes in the free left part of the canvas.
    private func drawTriangle(_ app: XCUIApplication, expect: String = "3 strokes") {
        let canvas = el(app, "capture.canvas")
        let pts = [(0.12, 0.75), (0.45, 0.75), (0.28, 0.45), (0.12, 0.75)]
        for i in 0..<3 {
            let a = canvas.coordinate(withNormalizedOffset: CGVector(dx: pts[i].0, dy: pts[i].1))
            let b = canvas.coordinate(withNormalizedOffset: CGVector(dx: pts[i + 1].0, dy: pts[i + 1].1))
            a.press(forDuration: 0.05, thenDragTo: b)
        }
        XCTAssertTrue(text(app, startingWith: expect).waitForExistence(timeout: 15), app.debugDescription)
    }

    func testCanvasFillsTheScreenInBothOrientations() throws {
        let app = launch()
        let canvas = el(app, "capture.canvas")
        XCTAssertEqual(canvas.frame, app.windows.firstMatch.frame, "landscape: the canvas is the whole window")
        drawTriangle(app)
        attach(app, "01-landscape-drawn")

        XCUIDevice.shared.orientation = .portrait
        XCTAssertTrue(text(app, startingWith: "3 strokes").waitForExistence(timeout: 10), "rotation keeps the drawing")
        Thread.sleep(forTimeInterval: 1)
        XCTAssertEqual(el(app, "capture.canvas").frame, app.windows.firstMatch.frame, "portrait: the canvas is the whole window")
        attach(app, "02-portrait-drawn")

        el(app, "capture.settings").tap()
        XCTAssertTrue(el(app, "canvas.settings.twoFingerUndo").waitForExistence(timeout: 5))
        attach(app, "03-portrait-settings")
        dismissPopover(app)

        XCUIDevice.shared.orientation = .landscapeLeft
        XCTAssertTrue(text(app, startingWith: "3 strokes").waitForExistence(timeout: 10))

        el(app, "capture.collapse").tap()
        XCTAssertTrue(el(app, "capture.send").waitForNonExistence(timeout: 5), "collapsed: only the corner controls remain")
        attach(app, "04-landscape-collapsed")
        el(app, "capture.collapse").tap()
        XCTAssertTrue(el(app, "capture.send").waitForExistence(timeout: 5))

        el(app, "capture.sidebar").tap()
        XCTAssertTrue(el(app, "open.sample").waitForExistence(timeout: 5), "the sidebar button reveals the sidebar")
        attach(app, "05-landscape-sidebar")
    }

    func testTwoFingerDoubleTapUndoesAndThreeFingerDoubleTapRedoes() throws {
        let app = launch()
        drawTriangle(app)
        let canvas = el(app, "capture.canvas")
        // XCUIElement.tap(withNumberOfTaps:numberOfTouches:) taps the
        // canvas centre, clear of the triangle and the floating controls.
        canvas.tap(withNumberOfTaps: 2, numberOfTouches: 2)
        XCTAssertTrue(el(app, "capture.toast").waitForExistence(timeout: 3), "Undo toast")
        XCTAssertEqual(el(app, "capture.toast").label, "Undo")
        XCTAssertTrue(text(app, startingWith: "2 strokes").waitForExistence(timeout: 5), app.debugDescription)
        attach(app, "06-two-finger-undo")

        canvas.tap(withNumberOfTaps: 2, numberOfTouches: 2)
        XCTAssertTrue(text(app, startingWith: "1 stroke").waitForExistence(timeout: 5), app.debugDescription)

        canvas.tap(withNumberOfTaps: 2, numberOfTouches: 3)
        XCTAssertTrue(text(app, startingWith: "2 strokes").waitForExistence(timeout: 5), app.debugDescription)
        canvas.tap(withNumberOfTaps: 2, numberOfTouches: 3)
        XCTAssertTrue(text(app, startingWith: "3 strokes").waitForExistence(timeout: 5), app.debugDescription)
        attach(app, "07-three-finger-redo")

        // Clear is undoable too.
        el(app, "capture.clear").tap()
        XCTAssertTrue(text(app, startingWith: "0 strokes").waitForExistence(timeout: 5))
        canvas.tap(withNumberOfTaps: 2, numberOfTouches: 2)
        XCTAssertTrue(text(app, startingWith: "3 strokes").waitForExistence(timeout: 5), "undo brings a cleared drawing back")

        // Turned off in the settings: the gesture does nothing.
        el(app, "capture.settings").tap()
        let toggle = app.switches["canvas.settings.twoFingerUndo"]
        XCTAssertTrue(toggle.waitForExistence(timeout: 5))
        toggle.coordinate(withNormalizedOffset: CGVector(dx: 0.92, dy: 0.5)).tap()
        XCTAssertEqual(toggle.value as? String, "0", "switched off")
        dismissPopover(app)
        let before = el(app, "capture.strokes").label
        canvas.tap(withNumberOfTaps: 2, numberOfTouches: 2)
        Thread.sleep(forTimeInterval: 1)
        XCTAssertEqual(el(app, "capture.strokes").label, before, "two-finger undo turned off")
    }
}
