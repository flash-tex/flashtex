import PencilKit
import SwiftUI
import XCTest
@testable import FlashTeXPad
@testable import FlashTeXPadKit

/// The full-screen canvas and its undo gestures: the gesture → command rules,
/// the undo handler on a real `UndoManager`, the Pencil tool-restore guard,
/// canvas geometry, the controller wiring (two-finger double-tap undoes a
/// Clear, three-finger redoes it), the PNG rendering, and the canvas filling
/// the scene across a portrait ↔ landscape resize without moving strokes.
@MainActor
final class CanvasGestureTests: XCTestCase {

    // MARK: router

    func testDefaultsUndoOnTwoFingersRedoOnThreeAndPencilUndoes() {
        let r = CanvasGestureRouter(settings: CanvasSettings())
        XCTAssertEqual(r.command(for: .twoFingerDoubleTap, toolPickerVisible: true), .undo)
        XCTAssertEqual(r.command(for: .threeFingerDoubleTap, toolPickerVisible: true), .redo)
        // Default Pencil mode is Undo, over the system default "switch to eraser".
        XCTAssertEqual(r.command(for: .pencilDoubleTap(system: .switchEraser), toolPickerVisible: true), .undo)
        XCTAssertEqual(r.command(for: .pencilDoubleTap(system: .switchPrevious), toolPickerVisible: false), .undo)
        XCTAssertEqual(r.command(for: .pencilDoubleTap(system: .showPalette), toolPickerVisible: false), .undo)
    }

    func testSystemOffIsAlwaysHonoured() {
        for mode in PencilDoubleTapMode.allCases {
            var s = CanvasSettings(); s.pencilDoubleTap = mode
            let r = CanvasGestureRouter(settings: s)
            XCTAssertEqual(r.command(for: .pencilDoubleTap(system: .ignore), toolPickerVisible: false), .none, "\(mode)")
            XCTAssertEqual(r.command(for: .pencilDoubleTap(system: .ignore), toolPickerVisible: true), .none, "\(mode)")
        }
    }

    func testSystemModeFollowsThePreferredActionWhenThePickerIsHidden() {
        var s = CanvasSettings(); s.pencilDoubleTap = .system
        let r = CanvasGestureRouter(settings: s)
        XCTAssertEqual(r.command(for: .pencilDoubleTap(system: .switchEraser), toolPickerVisible: false), .toggleEraser)
        XCTAssertEqual(r.command(for: .pencilDoubleTap(system: .switchPrevious), toolPickerVisible: false), .switchPrevious)
        XCTAssertEqual(r.command(for: .pencilDoubleTap(system: .showPalette), toolPickerVisible: false), .showToolPicker)
        XCTAssertEqual(r.command(for: .pencilDoubleTap(system: .other), toolPickerVisible: false), .none)
        // The visible tool picker applies the system action itself.
        XCTAssertEqual(r.command(for: .pencilDoubleTap(system: .switchEraser), toolPickerVisible: true), .none)
    }

    func testFingerGesturesCanBeTurnedOff() {
        var s = CanvasSettings(); s.twoFingerUndo = false; s.threeFingerRedo = false
        let r = CanvasGestureRouter(settings: s)
        XCTAssertEqual(r.command(for: .twoFingerDoubleTap, toolPickerVisible: false), .none)
        XCTAssertEqual(r.command(for: .threeFingerDoubleTap, toolPickerVisible: false), .none)
    }

    func testSystemActionMapping() {
        XCTAssertEqual(PencilSystemAction(.ignore), .ignore)
        XCTAssertEqual(PencilSystemAction(.switchEraser), .switchEraser)
        XCTAssertEqual(PencilSystemAction(.switchPrevious), .switchPrevious)
        XCTAssertEqual(PencilSystemAction(.showColorPalette), .showPalette)
        XCTAssertEqual(PencilSystemAction(.showInkAttributes), .showPalette)
        if #available(iOS 17.5, *) {
            XCTAssertEqual(PencilSystemAction(.showContextualPalette), .showPalette)
            XCTAssertEqual(PencilSystemAction(.runSystemShortcut), .other)
        }
    }

    // MARK: undo handler

    final class Box { var value = 0 }

    private func set(_ box: Box, _ v: Int, _ um: UndoManager) {
        let old = box.value
        um.registerUndo(withTarget: box) { [unowned self] b in self.set(b, old, um) }
        box.value = v
    }

    func testUndoHandlerUndoesAndRedoes() {
        let um = UndoManager()
        um.groupsByEvent = false
        let box = Box()
        um.beginUndoGrouping(); set(box, 1, um); um.endUndoGrouping()
        um.beginUndoGrouping(); set(box, 2, um); um.endUndoGrouping()
        let h = CanvasUndoHandler(undoManager: um)
        XCTAssertEqual(h.perform(.undo), .undid)
        XCTAssertEqual(box.value, 1)
        XCTAssertEqual(h.perform(.undo), .undid)
        XCTAssertEqual(box.value, 0)
        XCTAssertEqual(h.perform(.undo), .nothingToUndo)
        XCTAssertEqual(h.perform(.redo), .redid)
        XCTAssertEqual(box.value, 1)
        XCTAssertEqual(h.perform(.redo), .redid)
        XCTAssertEqual(h.perform(.redo), .nothingToRedo)
        XCTAssertEqual(box.value, 2)
        XCTAssertNil(h.perform(.toggleEraser))
        XCTAssertEqual(CanvasUndoHandler.Outcome.undid.toast, "Undo")
        XCTAssertEqual(CanvasUndoHandler.Outcome.redid.toast, "Redo")
    }

    // MARK: tool-restore guard

    func testGuardRestoresWhenThePickerSwitchesAfterTheTap() {
        var g = ToolRestoreGuard<String>(initial: "pen")
        XCTAssertNil(g.doubleTapped(at: 10))
        XCTAssertEqual(g.toolChanged(to: "eraser", at: 10.02), "pen")
        XCTAssertNil(g.toolChanged(to: "pen", at: 10.03), "our own restore is not re-restored")
        XCTAssertNil(g.toolChanged(to: "marker", at: 20), "a later user choice stands")
    }

    func testGuardRestoresWhenThePickerSwitchedJustBeforeTheTap() {
        var g = ToolRestoreGuard<String>(initial: "pen")
        XCTAssertNil(g.toolChanged(to: "eraser", at: 10))
        XCTAssertEqual(g.doubleTapped(at: 10.01), "pen")
        XCTAssertNil(g.toolChanged(to: "pen", at: 10.02))
    }

    func testGuardLeavesDeliberateToolChangesAlone() {
        var g = ToolRestoreGuard<String>(initial: "pen")
        XCTAssertNil(g.toolChanged(to: "pencil", at: 1))
        XCTAssertNil(g.doubleTapped(at: 5), "no picker change near the tap")
        XCTAssertNil(g.toolChanged(to: "marker", at: 9))
    }

    // MARK: layout

    func testContentGrowsBelowTheInkAndCoversItSideways() {
        let viewport = CGSize(width: 1180, height: 820)
        XCTAssertEqual(CanvasLayout.contentSize(viewport: viewport, drawingBounds: .null), viewport)
        XCTAssertEqual(CanvasLayout.contentSize(viewport: viewport, drawingBounds: CGRect(x: 10, y: 10, width: 50, height: 50)), viewport)
        let deep = CanvasLayout.contentSize(viewport: viewport, drawingBounds: CGRect(x: 10, y: 700, width: 50, height: 60))
        XCTAssertEqual(deep.width, 1180)
        XCTAssertEqual(deep.height, 760 + 410)
        let portrait = CanvasLayout.contentSize(viewport: CGSize(width: 820, height: 1180), drawingBounds: CGRect(x: 10, y: 700, width: 50, height: 60))
        XCTAssertEqual(portrait, CGSize(width: 820, height: 760 + 590), "half a screen of room below the ink")
        let shallow = CanvasLayout.contentSize(viewport: CGSize(width: 820, height: 1180), drawingBounds: CGRect(x: 10, y: 100, width: 50, height: 60))
        XCTAssertEqual(shallow, CGSize(width: 820, height: 1180))

        // Ink drawn across a 1180 pt landscape canvas, then the window
        // narrows (rotation, Split View, Stage Manager): the content widens
        // to cover it, so every stroke Send transmits can be scrolled to.
        let wide = CGRect(x: 700, y: 100, width: 420, height: 200)
        for viewport in [CGSize(width: 820, height: 1180), CGSize(width: 375, height: 820), CGSize(width: 1180, height: 820)] {
            let size = CanvasLayout.contentSize(viewport: viewport, drawingBounds: wide)
            XCTAssertTrue(CGRect(origin: .zero, size: size).contains(wide), "\(viewport): \(size) covers the ink")
            XCTAssertGreaterThanOrEqual(size.width, viewport.width)
            XCTAssertGreaterThanOrEqual(size.height, viewport.height)
        }
        XCTAssertEqual(CanvasLayout.contentSize(viewport: CGSize(width: 820, height: 1180), drawingBounds: wide).width, 1120 + 32)
        XCTAssertEqual(CanvasLayout.contentSize(viewport: CGSize(width: 1180, height: 820), drawingBounds: wide).width, 1180,
                       "no sideways scrolling while the ink fits")
    }

    func testCaptureRectCropsToInkWithMarginAndMinimum() {
        XCTAssertNil(CanvasLayout.captureRect(drawingBounds: .null))
        let big = CanvasLayout.captureRect(drawingBounds: CGRect(x: 100, y: 100, width: 600, height: 400))
        XCTAssertEqual(big, CGRect(x: 68, y: 68, width: 664, height: 464))
        let dot = CanvasLayout.captureRect(drawingBounds: CGRect(x: 500, y: 500, width: 4, height: 4))!
        XCTAssertEqual(dot.width, 320, accuracy: 1)
        XCTAssertEqual(dot.height, 200, accuracy: 1)
        XCTAssertEqual(dot.midX, 502, accuracy: 1)
        XCTAssertEqual(dot.midY, 502, accuracy: 1)
        XCTAssertEqual(CanvasLayout.captureScale(for: CGRect(x: 0, y: 0, width: 800, height: 600)), 2)
        XCTAssertEqual(CanvasLayout.captureScale(for: CGRect(x: 0, y: 0, width: 4096, height: 600)), 1)
    }

    // MARK: controller wiring

    static func line(from a: CGPoint, to b: CGPoint) -> PKStroke {
        let ink = PKInk(.pen, color: .black)
        let pts = (0...10).map { i -> PKStrokePoint in
            let t = CGFloat(i) / 10
            return PKStrokePoint(location: CGPoint(x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t), timeOffset: Double(i) * 0.01,
                                 size: CGSize(width: 4, height: 4), opacity: 1, force: 1, azimuth: 0, altitude: .pi / 2)
        }
        return PKStroke(ink: ink, path: PKStrokePath(controlPoints: pts, creationDate: Date()))
    }

    static let triangle = PKDrawing(strokes: [
        line(from: CGPoint(x: 200, y: 400), to: CGPoint(x: 600, y: 400)),
        line(from: CGPoint(x: 600, y: 400), to: CGPoint(x: 400, y: 150)),
        line(from: CGPoint(x: 400, y: 150), to: CGPoint(x: 200, y: 400)),
    ])

    func testCanvasHasItsOwnUndoHistoryAndNoSystemEditingGestures() {
        let c = CanvasController()
        let v = c.makeCanvas()
        XCTAssertTrue(v.undoManager === v.canvasUndoManager)
        XCTAssertEqual(v.editingInteractionConfiguration, .none)
        XCTAssertEqual(v.drawingPolicy, .anyInput)
        let taps = (v.gestureRecognizers ?? []).compactMap { $0 as? UITapGestureRecognizer }
        XCTAssertTrue(taps.contains { $0.numberOfTouchesRequired == 2 && $0.numberOfTapsRequired == 2 })
        XCTAssertTrue(taps.contains { $0.numberOfTouchesRequired == 3 && $0.numberOfTapsRequired == 2 })
        XCTAssertTrue(v.interactions.contains { $0 is UIPencilInteraction })

        var s = CanvasSettings(); s.fingerDrawing = false; s.twoFingerUndo = false
        c.settings = s
        XCTAssertEqual(v.drawingPolicy, .pencilOnly)
        XCTAssertFalse(taps.first { $0.numberOfTouchesRequired == 2 }!.isEnabled)
        XCTAssertTrue(taps.first { $0.numberOfTouchesRequired == 3 }!.isEnabled)
    }

    func testTwoFingerDoubleTapUndoesClearAndThreeFingerRedoesIt() {
        let c = CanvasController()
        let v = c.makeCanvas()
        v.canvasUndoManager.groupsByEvent = false
        v.drawing = Self.triangle
        XCTAssertEqual(v.drawing.strokes.count, 3)

        v.canvasUndoManager.beginUndoGrouping(); c.clear(); v.canvasUndoManager.endUndoGrouping()
        XCTAssertEqual(v.drawing.strokes.count, 0)
        XCTAssertEqual(c.strokeCount, 0)
        XCTAssertTrue(c.canUndo)

        c.handle(.twoFingerDoubleTap)
        XCTAssertEqual(v.drawing.strokes.count, 3, "two-finger double-tap undid the Clear")
        XCTAssertEqual(c.strokeCount, 3)
        XCTAssertEqual(c.toast?.text, "Undo")

        c.handle(.threeFingerDoubleTap)
        XCTAssertEqual(v.drawing.strokes.count, 0, "three-finger double-tap redid it")
        XCTAssertEqual(c.toast?.text, "Redo")

        c.handle(.threeFingerDoubleTap)
        XCTAssertEqual(c.toast?.text, "Nothing to redo")

        var s = CanvasSettings(); s.twoFingerUndo = false
        c.settings = s
        c.toast = nil
        c.handle(.twoFingerDoubleTap)
        XCTAssertEqual(v.drawing.strokes.count, 0, "turned off: nothing happens")
        XCTAssertNil(c.toast)

        c.handle(.pencilDoubleTap(system: .switchEraser))
        XCTAssertEqual(v.drawing.strokes.count, 3, "Pencil double-tap undoes by default")
    }

    func testResetEmptiesTheCanvasAndItsHistory() {
        let c = CanvasController()
        let v = c.makeCanvas()
        v.drawing = Self.triangle
        c.clear()
        c.reset()
        XCTAssertEqual(v.drawing.strokes.count, 0)
        XCTAssertFalse(c.canUndo)
    }

    // MARK: PNG

    func testRenderIsCroppedOpaqueAndLightEvenInDarkMode() throws {
        var result: Result<(png: Data, pixels: (Int, Int)), CanvasController.RenderProblem>!
        UITraitCollection(userInterfaceStyle: .dark).performAsCurrent {
            result = CanvasController.renderPNG(Self.triangle)
        }
        let r = try result.get()
        let rect = try XCTUnwrap(CanvasLayout.captureRect(drawingBounds: Self.triangle.bounds))
        XCTAssertEqual(r.pixels.0, Int(rect.width * 2))
        XCTAssertEqual(r.pixels.1, Int(rect.height * 2))
        XCTAssertNil(CaptureQueue.validate(png: r.png, instructions: "Convert to TikZ"))
        let img = try XCTUnwrap(UIImage(data: r.png)?.cgImage)
        XCTAssertEqual(img.width, r.pixels.0)
        // The corner is white paper, not transparent and not dark.
        let px = try XCTUnwrap(Self.pixel(img, x: 1, y: 1))
        XCTAssertEqual(px.a, 255)
        XCTAssertGreaterThan(Int(px.r) + Int(px.g) + Int(px.b), 3 * 250)
        // The ink is dark.
        let inkY = Int((400 - rect.minY) * 2), inkX = Int((400 - rect.minX) * 2)
        let ink = try XCTUnwrap(Self.pixel(img, x: inkX, y: inkY))
        XCTAssertLessThan(Int(ink.r) + Int(ink.g) + Int(ink.b), 3 * 100)
        XCTAssertEqual(CanvasController.renderPNG(PKDrawing()).map { _ in 0 }, .failure(.empty))
    }

    static func pixel(_ img: CGImage, x: Int, y: Int) -> (r: UInt8, g: UInt8, b: UInt8, a: UInt8)? {
        var buf = [UInt8](repeating: 0, count: 4)
        guard let ctx = CGContext(data: &buf, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4,
                                  space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        ctx.draw(img, in: CGRect(x: -x, y: -(img.height - 1 - y), width: img.width, height: img.height))
        return (buf[0], buf[1], buf[2], buf[3])
    }

    // MARK: full-screen layout

    private func findCanvas(in view: UIView) -> CaptureCanvasView? {
        if let c = view as? CaptureCanvasView { return c }
        for s in view.subviews { if let c = findCanvas(in: s) { return c } }
        return nil
    }

    private func settle(_ window: UIWindow) {
        for _ in 0..<5 {
            window.layoutIfNeeded()
            RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        }
    }

    func testCanvasFillsTheSceneAndKeepsStrokesAcrossResize() throws {
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        let model = PadModel(link: MacLink(store: nil))
        let host = UIHostingController(rootView: NavigationStack { CaptureView() }.environmentObject(model))
        let window = UIWindow(windowScene: scene)
        window.frame = CGRect(x: 0, y: 0, width: 1180, height: 820)
        window.rootViewController = host
        window.isHidden = false
        defer { window.isHidden = true }
        settle(window)

        let canvas = try XCTUnwrap(findCanvas(in: window), "the PencilKit canvas is in the hierarchy")
        XCTAssertEqual(canvas.convert(canvas.bounds, to: window), window.bounds, "landscape: edge to edge, safe areas included")

        canvas.drawing = Self.triangle
        let before = canvas.drawing.bounds

        window.frame = CGRect(x: 0, y: 0, width: 820, height: 1180)
        settle(window)
        XCTAssertEqual(canvas.convert(canvas.bounds, to: window), window.bounds, "portrait: edge to edge")
        XCTAssertEqual(canvas.drawing.bounds, before, "strokes keep their coordinates across the resize")
        assertEveryStrokeReachable(canvas, "portrait")

        // A narrow Split View / Stage Manager window: the triangle reaches
        // x = 600, past the 375 pt window.
        window.frame = CGRect(x: 0, y: 0, width: 375, height: 820)
        settle(window)
        XCTAssertEqual(canvas.convert(canvas.bounds, to: window), window.bounds, "narrow window: edge to edge")
        XCTAssertEqual(canvas.drawing.bounds, before)
        assertEveryStrokeReachable(canvas, "narrow window")
    }

    /// Every stroke lies inside the scrollable content, and scrolling to it
    /// brings it on screen.
    private func assertEveryStrokeReachable(_ canvas: CaptureCanvasView, _ what: String,
                                            file: StaticString = #filePath, line: UInt = #line) {
        let content = CGRect(origin: .zero, size: canvas.contentSize)
        for (i, stroke) in canvas.drawing.strokes.enumerated() {
            let r = stroke.renderBounds
            XCTAssertTrue(content.contains(r), "\(what): stroke \(i) \(r) outside content \(content)", file: file, line: line)
            let maxOffset = CGPoint(x: max(0, canvas.contentSize.width - canvas.bounds.width),
                                    y: max(0, canvas.contentSize.height - canvas.bounds.height))
            let offset = CGPoint(x: min(max(0, r.midX - canvas.bounds.width / 2), maxOffset.x),
                                 y: min(max(0, r.midY - canvas.bounds.height / 2), maxOffset.y))
            canvas.setContentOffset(offset, animated: false)
            let visible = CGRect(origin: canvas.contentOffset, size: canvas.bounds.size)
            XCTAssertTrue(visible.intersects(r), "\(what): stroke \(i) \(r) not on screen at offset \(offset)", file: file, line: line)
        }
        canvas.setContentOffset(.zero, animated: false)
    }

    /// Drawn on the right of a landscape window, then the window narrows:
    /// the ink stays reachable (it used to fall off the edge while still
    /// being sent).
    func testInkOnTheRightStaysReachableWhenTheWindowNarrows() throws {
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        let model = PadModel(link: MacLink(store: nil))
        let host = UIHostingController(rootView: NavigationStack { CaptureView() }.environmentObject(model))
        let window = UIWindow(windowScene: scene)
        window.frame = CGRect(x: 0, y: 0, width: 1180, height: 820)
        window.rootViewController = host
        window.isHidden = false
        defer { window.isHidden = true }
        settle(window)
        let canvas = try XCTUnwrap(findCanvas(in: window))

        canvas.drawing = PKDrawing(strokes: [
            Self.line(from: CGPoint(x: 900, y: 500), to: CGPoint(x: 1120, y: 500)),
            Self.line(from: CGPoint(x: 1120, y: 500), to: CGPoint(x: 1010, y: 300)),
            Self.line(from: CGPoint(x: 1010, y: 300), to: CGPoint(x: 900, y: 500)),
        ])
        settle(window)
        XCTAssertEqual(canvas.contentSize.width, 1180, "fits: no sideways scrolling")

        for (w, h) in [(820.0, 1180.0), (375.0, 820.0), (507.0, 820.0)] {
            window.frame = CGRect(x: 0, y: 0, width: w, height: h)
            settle(window)
            XCTAssertEqual(canvas.bounds.width, w)
            XCTAssertGreaterThan(canvas.contentSize.width, w, "\(w) pt: scrollable sideways")
            assertEveryStrokeReachable(canvas, "\(w) pt window")
        }

        // Wide again: back to one screen.
        window.frame = CGRect(x: 0, y: 0, width: 1180, height: 820)
        settle(window)
        XCTAssertEqual(canvas.contentSize.width, 1180)
    }

    // MARK: compact width navigation

    /// Push/pop transitions finish on their own schedule on a loaded
    /// simulator: poll until `condition` holds (or 5 s pass).
    private func eventually(_ window: UIWindow, _ condition: () -> Bool) -> Bool {
        let deadline = Date().addingTimeInterval(5)
        while Date() < deadline {
            if condition() { return true }
            window.layoutIfNeeded()
            RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        }
        return condition()
    }

    private func viewControllers(_ vc: UIViewController) -> [UIViewController] {
        [vc] + vc.children.flatMap(viewControllers) + (vc.presentedViewController.map(viewControllers) ?? [])
    }

    /// Slide Over, 1/3 Split View, a narrow Stage Manager window: the split
    /// view collapses into a stack. The floating sidebar button must still
    /// get out of Capture, and the bar with its back button stays visible.
    func testCompactWidthCanLeaveCapture() throws {
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        let model = PadModel(link: MacLink(store: nil))
        let nav = PadNavigation()
        let host = UIHostingController(rootView: ContentView(navigation: nav).environmentObject(model))
        host.traitOverrides.horizontalSizeClass = .compact
        let window = UIWindow(windowScene: scene)
        window.frame = CGRect(x: 0, y: 0, width: 375, height: 820)
        window.rootViewController = host
        window.isHidden = false
        defer { window.isHidden = true }
        settle(window)

        let split = try XCTUnwrap(viewControllers(host).compactMap { $0 as? UISplitViewController }.first, "a split view")
        XCTAssertTrue(split.isCollapsed, "compact width collapses the split view")
        func stackDepth() -> Int {
            viewControllers(split).compactMap { $0 as? UINavigationController }
                .filter { $0.view.window != nil }.map(\.viewControllers.count).max() ?? 0
        }
        func barVisible() -> Bool {
            viewControllers(split).compactMap { $0 as? UINavigationController }
                .filter { $0.view.window != nil }.contains { !$0.isNavigationBarHidden && !$0.navigationBar.isHidden }
        }

        XCTAssertNotNil(findCanvas(in: window), "starts on Capture")
        XCTAssertTrue(eventually(window) { stackDepth() == 2 }, "Capture is pushed over the sidebar")
        XCTAssertTrue(barVisible(), "compact: the navigation bar (and its back button) stays visible on Capture")

        // What the floating sidebar button does.
        nav.showSidebar(compact: true)
        settle(window)
        XCTAssertTrue(eventually(window) { stackDepth() == 1 }, "the sidebar button pops back to the sidebar")
        XCTAssertTrue(eventually(window) { self.findCanvas(in: window) == nil }, "Capture is off screen")

        // Choosing Mac link from the sidebar shows it.
        nav.panel = .mac
        settle(window)
        XCTAssertTrue(eventually(window) { stackDepth() == 2 }, "Mac link is pushed")
        XCTAssertTrue(eventually(window) { self.findCanvas(in: window) == nil })

        // And back to Capture.
        nav.showSidebar(compact: true)
        settle(window)
        XCTAssertTrue(eventually(window) { stackDepth() == 1 })
        nav.panel = .capture
        settle(window)
        XCTAssertTrue(eventually(window) { stackDepth() == 2 })
        XCTAssertTrue(eventually(window) { self.findCanvas(in: window) != nil }, "back on Capture")
    }

    /// Regular width: the sidebar starts hidden on Capture, the button shows
    /// it, and other panels use the default split.
    func testRegularWidthSidebarState() {
        let nav = PadNavigation()
        XCTAssertEqual(nav.columns, .detailOnly)
        nav.showSidebar(compact: false)
        XCTAssertEqual(nav.panel, .capture, "regular width keeps the selection")
        XCTAssertEqual(nav.columns, .all)
        XCTAssertEqual(nav.compactColumn, .sidebar)
        nav.panel = .editor
        XCTAssertEqual(nav.columns, .automatic)
        XCTAssertEqual(nav.compactColumn, .detail)
        nav.panel = .capture
        XCTAssertEqual(nav.columns, .detailOnly)
    }
}
