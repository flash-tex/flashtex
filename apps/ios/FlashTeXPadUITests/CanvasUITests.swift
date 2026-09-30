import XCTest

/// The full-screen capture canvas in the simulator: it fills the screen in
/// portrait and landscape, a rotation keeps every stroke, two-finger
/// double-tap undoes (with an "Undo" toast), three-finger double-tap redoes,
/// the gestures can be turned off in the canvas settings, and the sidebar is
/// one tap away. No Mac is needed. Screenshots are kept as attachments
/// (docs/evidence/ipad-canvas-2026-09-30/).
final class CanvasUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .portrait
    }

    override func tearDownWithError() throws {
        XCUIDevice.shared.orientation = .portrait
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

    /// A triangle of three finger strokes in the lower-left part of the canvas.
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

    /// Two or three fingers tapping twice around `center` (window points,
    /// portrait). `XCUIElement.tap(withNumberOfTaps:numberOfTouches:)` fails
    /// here with "Unable to compute coordinates for gesture", so the touches
    /// are synthesized with XCTest's own event record, the machinery behind
    /// every XCUICoordinate tap.
    private func multiFingerDoubleTap(fingers: Int, at center: CGPoint) throws {
        let recordClass = try XCTUnwrap(NSClassFromString("XCSynthesizedEventRecord") as? NSObject.Type)
        let pathClass = try XCTUnwrap(NSClassFromString("XCPointerEventPath") as? NSObject.Type)

        typealias InitRecord = @convention(c) (AnyObject, Selector, NSString, Int) -> AnyObject
        typealias InitPath = @convention(c) (AnyObject, Selector, CGPoint, Double) -> AnyObject
        typealias AtOffset = @convention(c) (AnyObject, Selector, Double) -> Void
        typealias AddPath = @convention(c) (AnyObject, Selector, AnyObject) -> Void
        typealias Synthesize = @convention(c) (AnyObject, Selector, UnsafeMutablePointer<NSError?>?) -> Bool

        func method<T>(_ obj: AnyObject, _ name: String, _: T.Type) -> (T, Selector) {
            let sel = NSSelectorFromString(name)
            return (unsafeBitCast(obj.method(for: sel), to: T.self), sel)
        }

        let recordAlloc = recordClass.perform(NSSelectorFromString("alloc")).takeUnretainedValue()
        let (initRecord, initRecordSel) = method(recordAlloc, "initWithName:interfaceOrientation:", InitRecord.self)
        let record = initRecord(recordAlloc, initRecordSel, "\(fingers)-finger double tap" as NSString, 1 /* portrait */)
        let (add, addSel) = method(record, "addPointerEventPath:", AddPath.self)

        for tap in 0..<2 {
            let start = Double(tap) * 0.15
            for f in 0..<fingers {
                let pt = CGPoint(x: center.x + CGFloat(f) * 50 - CGFloat(fingers - 1) * 25, y: center.y)
                let pathAlloc = pathClass.perform(NSSelectorFromString("alloc")).takeUnretainedValue()
                let (initPath, initPathSel) = method(pathAlloc, "initForTouchAtPoint:offset:", InitPath.self)
                let path = initPath(pathAlloc, initPathSel, pt, start)
                let (lift, liftSel) = method(path, "liftUpAtOffset:", AtOffset.self)
                lift(path, liftSel, start + 0.05)
                add(record, addSel, path)
            }
        }
        let (synthesize, synthesizeSel) = method(record, "synthesizeWithError:", Synthesize.self)
        var error: NSError?
        XCTAssertTrue(synthesize(record, synthesizeSel, &error), String(describing: error))
    }

    func testCanvasFillsTheScreenInBothOrientations() throws {
        let app = launch()
        XCTAssertEqual(el(app, "capture.canvas").frame, app.windows.firstMatch.frame, "portrait: the canvas is the whole window")
        drawTriangle(app)
        attach(app, "01-portrait-drawn")

        el(app, "capture.settings").tap()
        XCTAssertTrue(el(app, "canvas.settings.twoFingerUndo").waitForExistence(timeout: 5))
        attach(app, "02-portrait-settings")
        dismissPopover(app)

        XCUIDevice.shared.orientation = .landscapeLeft
        XCTAssertTrue(text(app, startingWith: "3 strokes").waitForExistence(timeout: 10), "rotation keeps the drawing")
        Thread.sleep(forTimeInterval: 1)
        XCTAssertEqual(el(app, "capture.canvas").frame, app.windows.firstMatch.frame, "landscape: the canvas is the whole window")
        attach(app, "03-landscape-drawn")

        el(app, "capture.collapse").tap()
        XCTAssertTrue(el(app, "capture.send").waitForNonExistence(timeout: 5), "collapsed: only the corner controls remain")
        attach(app, "04-landscape-collapsed")
        el(app, "capture.collapse").tap()
        XCTAssertTrue(el(app, "capture.send").waitForExistence(timeout: 5))

        XCUIDevice.shared.orientation = .portrait
        XCTAssertTrue(text(app, startingWith: "3 strokes").waitForExistence(timeout: 10), "and back")
    }

    func testSidebarButtonRevealsTheSidebar() throws {
        let app = launch()
        let button = el(app, "capture.sidebar")
        XCTAssertTrue(button.waitForExistence(timeout: 5))
        XCTAssertFalse(el(app, "open.sample").exists, "the canvas starts full-screen, sidebar hidden")
        button.tap()
        Thread.sleep(forTimeInterval: 1)
        attach(app, "05-sidebar")
        XCTAssertTrue(el(app, "open.sample").waitForExistence(timeout: 5), "the sidebar button reveals the sidebar")
    }

    func testTwoFingerDoubleTapUndoesAndThreeFingerDoubleTapRedoes() throws {
        let app = launch()
        drawTriangle(app)
        // The window centre: the canvas, clear of the triangle and the
        // floating controls.
        let f = app.windows.firstMatch.frame
        let centre = CGPoint(x: f.midX, y: f.midY)

        try multiFingerDoubleTap(fingers: 2, at: centre)
        // The toast is up for 1.2 s; a loaded simulator can take longer than
        // that to answer the first query, so it is checked when caught (its
        // text is asserted in CanvasGestureTests).
        let toast = el(app, "capture.toast")
        if toast.waitForExistence(timeout: 1) { XCTAssertEqual(toast.label, "Undo") }
        XCTAssertTrue(text(app, startingWith: "2 strokes").waitForExistence(timeout: 5), app.debugDescription)
        attach(app, "06-two-finger-undo")

        try multiFingerDoubleTap(fingers: 2, at: centre)
        XCTAssertTrue(text(app, startingWith: "1 stroke").waitForExistence(timeout: 5), app.debugDescription)

        try multiFingerDoubleTap(fingers: 3, at: centre)
        XCTAssertTrue(text(app, startingWith: "2 strokes").waitForExistence(timeout: 5), app.debugDescription)
        try multiFingerDoubleTap(fingers: 3, at: centre)
        XCTAssertTrue(text(app, startingWith: "3 strokes").waitForExistence(timeout: 5), app.debugDescription)
        attach(app, "07-three-finger-redo")

        // Clear is undoable too.
        el(app, "capture.clear").tap()
        XCTAssertTrue(text(app, startingWith: "0 strokes").waitForExistence(timeout: 5))
        try multiFingerDoubleTap(fingers: 2, at: centre)
        XCTAssertTrue(text(app, startingWith: "3 strokes").waitForExistence(timeout: 5), "undo brings a cleared drawing back")

        // Turned off in the settings: the gesture does nothing.
        el(app, "capture.settings").tap()
        let toggle = app.switches["canvas.settings.twoFingerUndo"]
        XCTAssertTrue(toggle.waitForExistence(timeout: 5))
        toggle.coordinate(withNormalizedOffset: CGVector(dx: 0.92, dy: 0.5)).tap()
        XCTAssertEqual(toggle.value as? String, "0", "switched off")
        dismissPopover(app)
        try multiFingerDoubleTap(fingers: 2, at: centre)
        Thread.sleep(forTimeInterval: 1)
        XCTAssertTrue(text(app, startingWith: "3 strokes").exists, "two-finger undo turned off")
    }
}
