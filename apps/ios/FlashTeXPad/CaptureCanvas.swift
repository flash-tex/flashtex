import PencilKit
import SwiftUI
import UIKit

/// The full-screen PencilKit canvas behind the capture screen.
///
/// - Fills the scene edge to edge; the drawing lives in content coordinates
///   anchored at the top-left, so rotation, Split View and Stage Manager
///   resizes never move a stroke (`CanvasLayout.contentSize`).
/// - Grows downward as the ink nears the bottom, and sideways when ink drawn
///   in a wider window lies past the right edge, so every stroke Send will
///   transmit can be scrolled to; scroll with two fingers (one finger when
///   finger drawing is off).
/// - Keeps its own undo history, so undo gestures and ⌘Z never undo typing in
///   the instruction field.
/// - Opts out of the system three-finger editing gestures, which would
///   otherwise undo on the same three-finger double-tap we use for redo.
final class CaptureCanvasView: PKCanvasView {
    let canvasUndoManager = UndoManager()
    override var undoManager: UndoManager? { canvasUndoManager }
    override var editingInteractionConfiguration: UIEditingInteractionConfiguration { .none }

    override func layoutSubviews() {
        super.layoutSubviews()
        updateContentSize()
    }

    func updateContentSize() {
        let size = CanvasLayout.contentSize(viewport: bounds.size, drawingBounds: drawing.bounds)
        if size != .zero, contentSize != size { contentSize = size }
    }
}

/// Owns the canvas view, its tool picker, the gesture recognizers and the
/// Apple Pencil interaction; publishes what the floating controls show.
@MainActor
final class CanvasController: NSObject, ObservableObject {
    struct Toast: Equatable {
        let id = UUID()
        let text: String
    }

    @Published private(set) var strokeCount = 0
    @Published private(set) var canUndo = false
    @Published private(set) var canRedo = false
    /// A stroke is in progress (the floating controls fade while this is true).
    @Published private(set) var isDrawing = false
    @Published var toast: Toast?
    @Published var toolsVisible = true { didSet { applyToolPickerVisibility() } }
    var settings = CanvasSettings() { didSet { if settings != oldValue { applySettings() } } }

    private(set) var canvas: CaptureCanvasView?
    private var picker: PKToolPicker?
    private var twoFingerTap: UITapGestureRecognizer?
    private var threeFingerTap: UITapGestureRecognizer?
    private var pencil: UIPencilInteraction?
    private var restoreGuard = ToolRestoreGuard<ToolRef>()
    private var previousTool: PKTool?
    /// The tool in use before the latest picker change (the canvas, being
    /// the first observer, already holds the new tool when we hear of it).
    private var lastTool: PKTool?
    private var toastWork: DispatchWorkItem?

    var drawing: PKDrawing { canvas?.drawing ?? PKDrawing() }
    var undoManager: UndoManager? { canvas?.undoManager }

    func makeCanvas() -> CaptureCanvasView {
        if let canvas { return canvas }
        let v = CaptureCanvasView()
        v.tool = PKInkingTool(.pen, color: .black, width: 4)
        // Follows light/dark mode on screen (PencilKit shows black ink as
        // white on the dark canvas); the PNG is always rendered light on
        // white (`renderPNG`).
        v.backgroundColor = .systemBackground
        v.isOpaque = true
        v.alwaysBounceVertical = true
        v.minimumZoomScale = 1
        v.maximumZoomScale = 1
        v.contentInsetAdjustmentBehavior = .never
        v.delegate = self
        v.isAccessibilityElement = true
        v.accessibilityIdentifier = "capture.canvasView"
        v.accessibilityLabel = "Drawing canvas"
        v.accessibilityHint = "Draw with Apple Pencil or a finger. Double-tap with two fingers to undo, three fingers to redo."

        let two = UITapGestureRecognizer(target: self, action: #selector(twoFingerDoubleTapped(_:)))
        two.numberOfTouchesRequired = 2
        two.numberOfTapsRequired = 2
        two.delegate = self
        v.addGestureRecognizer(two)
        let three = UITapGestureRecognizer(target: self, action: #selector(threeFingerDoubleTapped(_:)))
        three.numberOfTouchesRequired = 3
        three.numberOfTapsRequired = 2
        three.delegate = self
        v.addGestureRecognizer(three)
        twoFingerTap = two
        threeFingerTap = three

        let p = UIPencilInteraction()
        p.delegate = self
        v.addInteraction(p)
        pencil = p

        let tp = PKToolPicker()
        tp.addObserver(v)
        tp.addObserver(self)
        picker = tp
        canvas = v
        restoreGuard = ToolRestoreGuard(initial: currentToolRef())
        lastTool = v.tool
        applySettings()
        applyToolPickerVisibility()
        return v
    }

    // MARK: drawing

    /// Clear, as one undoable step (undo brings the drawing back).
    func clear() {
        guard let c = canvas, !c.drawing.strokes.isEmpty else { return }
        replaceDrawing(PKDrawing(), actionName: "Clear")
    }

    /// After a send: an empty canvas and an empty history.
    func reset() {
        canvas?.drawing = PKDrawing()
        undoManager?.removeAllActions()
        canvas?.setContentOffset(.zero, animated: false)
        refresh()
    }

    private func replaceDrawing(_ new: PKDrawing, actionName: String) {
        guard let c = canvas else { return }
        let old = c.drawing
        c.undoManager?.registerUndo(withTarget: self) { target in
            target.replaceDrawing(old, actionName: actionName)
        }
        c.undoManager?.setActionName(actionName)
        c.drawing = new
        refresh()
    }

    func undo() { perform(.undo) }
    func redo() { perform(.redo) }

    func handle(_ gesture: CanvasGesture) {
        let command = CanvasGestureRouter(settings: settings).command(for: gesture, toolPickerVisible: picker?.isVisible ?? false)
        perform(command)
    }

    private func perform(_ command: CanvasCommand) {
        guard let c = canvas else { return }
        switch command {
        case .undo, .redo:
            guard let um = c.undoManager, let outcome = CanvasUndoHandler(undoManager: um).perform(command) else { return }
            show(outcome.toast)
            refresh()
        case .toggleEraser:
            if c.tool is PKEraserTool {
                c.tool = previousTool ?? PKInkingTool(.pen, color: .black, width: 4)
            } else {
                previousTool = c.tool
                c.tool = PKEraserTool(.vector)
            }
            lastTool = c.tool
            show(c.tool is PKEraserTool ? "Eraser" : "Pen")
        case .switchPrevious:
            guard let prev = previousTool else { return }
            previousTool = c.tool
            c.tool = prev
            lastTool = prev
        case .showToolPicker:
            toolsVisible = true
        case .none:
            break
        }
    }

    func show(_ text: String) {
        let t = Toast(text: text)
        toast = t
        UIAccessibility.post(notification: .announcement, argument: text)
        toastWork?.cancel()
        let work = DispatchWorkItem { [weak self] in
            if self?.toast == t { self?.toast = nil }
        }
        toastWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2, execute: work)
    }

    fileprivate func refresh() {
        strokeCount = canvas?.drawing.strokes.count ?? 0
        canUndo = undoManager?.canUndo ?? false
        canRedo = undoManager?.canRedo ?? false
        canvas?.updateContentSize()
    }

    // MARK: PNG

    enum RenderProblem: Error, Equatable { case empty, encoding }

    /// The ink, cropped with a margin (`CanvasLayout.captureRect`), rendered in
    /// the light appearance on white whatever the screen's appearance.
    func renderPNG() -> Result<(png: Data, pixels: (Int, Int)), RenderProblem> {
        Self.renderPNG(drawing)
    }

    static func renderPNG(_ d: PKDrawing) -> Result<(png: Data, pixels: (Int, Int)), RenderProblem> {
        guard !d.strokes.isEmpty, let rect = CanvasLayout.captureRect(drawingBounds: d.bounds) else { return .failure(.empty) }
        let scale = CanvasLayout.captureScale(for: rect)
        var ink = UIImage()
        UITraitCollection(userInterfaceStyle: .light).performAsCurrent {
            ink = d.image(from: rect, scale: scale)
        }
        let format = UIGraphicsImageRendererFormat()
        format.scale = scale
        format.opaque = true
        let size = rect.size
        let img = UIGraphicsImageRenderer(size: size, format: format).image { ctx in
            UIColor.white.setFill()
            ctx.fill(CGRect(origin: .zero, size: size))
            ink.draw(in: CGRect(origin: .zero, size: size))
        }
        guard let png = img.pngData() else { return .failure(.encoding) }
        return .success((png, (Int((size.width * scale).rounded()), Int((size.height * scale).rounded()))))
    }

    // MARK: settings and the tool picker

    private func applySettings() {
        guard let c = canvas else { return }
        c.drawingPolicy = settings.fingerDrawing ? .anyInput : .pencilOnly
        twoFingerTap?.isEnabled = settings.twoFingerUndo
        threeFingerTap?.isEnabled = settings.threeFingerRedo
    }

    private func applyToolPickerVisibility() {
        guard let c = canvas, let picker else { return }
        picker.setVisible(toolsVisible, forFirstResponder: c)
        DispatchQueue.main.async {
            if self.toolsVisible { c.becomeFirstResponder() } else { c.resignFirstResponder() }
        }
    }

    enum ToolRef {
        case item(String)
        case tool(PKTool)
    }

    private func currentToolRef() -> ToolRef? {
        guard let picker else { return nil }
        if #available(iOS 18.0, *) { return .item(picker.selectedToolItemIdentifier) }
        return .tool(picker.selectedTool)
    }

    private func restore(_ ref: ToolRef) {
        guard let picker else { return }
        switch ref {
        case .item(let id):
            if #available(iOS 18.0, *) { picker.selectedToolItemIdentifier = id }
        case .tool(let t):
            if #available(iOS 18.0, *) {} else { picker.selectedTool = t }
        }
    }

    private func pickerToolChanged() {
        guard let ref = currentToolRef() else { return }
        if let canvas {
            previousTool = lastTool
            lastTool = canvas.tool
        }
        if let back = restoreGuard.toolChanged(to: ref, at: ProcessInfo.processInfo.systemUptime) {
            restore(back)
        }
    }
}

// MARK: - PencilKit delegates

extension CanvasController: PKCanvasViewDelegate, PKToolPickerObserver {
    nonisolated func canvasViewDrawingDidChange(_ canvasView: PKCanvasView) {
        MainActor.assumeIsolated {
            refresh()
            // PencilKit registers the stroke's undo after this callback.
            DispatchQueue.main.async { self.refresh() }
        }
    }

    nonisolated func canvasViewDidBeginUsingTool(_ canvasView: PKCanvasView) {
        MainActor.assumeIsolated {
            isDrawing = true
            // Drawing after typing in the instruction field: bring the tool
            // picker back with the canvas as first responder.
            if toolsVisible, !canvasView.isFirstResponder { canvasView.becomeFirstResponder() }
        }
    }

    nonisolated func canvasViewDidEndUsingTool(_ canvasView: PKCanvasView) {
        MainActor.assumeIsolated { isDrawing = false }
    }

    nonisolated func toolPickerSelectedToolDidChange(_ toolPicker: PKToolPicker) {
        if #available(iOS 18.0, *) { return }
        MainActor.assumeIsolated { pickerToolChanged() }
    }

    nonisolated func toolPickerSelectedToolItemDidChange(_ toolPicker: PKToolPicker) {
        MainActor.assumeIsolated { pickerToolChanged() }
    }

    nonisolated func toolPickerVisibilityDidChange(_ toolPicker: PKToolPicker) {
        MainActor.assumeIsolated {
            // The user closed the picker (or it hid with the keyboard): keep
            // the Tools button's state truthful without re-showing it.
            if !toolPicker.isVisible, toolsVisible, canvas?.isFirstResponder == true { toolsVisible = false }
        }
    }
}

// MARK: - gestures

extension CanvasController: UIGestureRecognizerDelegate, UIPencilInteractionDelegate {
    @objc func twoFingerDoubleTapped(_ g: UITapGestureRecognizer) {
        if g.state == .ended { handle(.twoFingerDoubleTap) }
    }

    @objc func threeFingerDoubleTapped(_ g: UITapGestureRecognizer) {
        if g.state == .ended { handle(.threeFingerDoubleTap) }
    }

    /// Taps never block PencilKit's drawing or the scroll view's pan.
    func gestureRecognizer(_ g: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool {
        true
    }

    /// Finger taps only: a Pencil touch is always ink.
    func gestureRecognizer(_ g: UIGestureRecognizer, shouldReceive touch: UITouch) -> Bool {
        touch.type != .pencil
    }

    @available(iOS 17.5, *)
    nonisolated func pencilInteraction(_ interaction: UIPencilInteraction, didReceiveTap tap: UIPencilInteraction.Tap) {
        MainActor.assumeIsolated { pencilDoubleTapped() }
    }

    nonisolated func pencilInteractionDidTap(_ interaction: UIPencilInteraction) {
        if #available(iOS 17.5, *) { return }
        MainActor.assumeIsolated { pencilDoubleTapped() }
    }

    private func pencilDoubleTapped() {
        let system = PencilSystemAction(UIPencilInteraction.preferredTapAction)
        let gesture = CanvasGesture.pencilDoubleTap(system: system)
        let command = CanvasGestureRouter(settings: settings).command(for: gesture, toolPickerVisible: picker?.isVisible ?? false)
        if command == .undo, picker?.isVisible == true,
           let back = restoreGuard.doubleTapped(at: ProcessInfo.processInfo.systemUptime) {
            restore(back)
        }
        perform(command)
    }
}

/// SwiftUI wrapper; the controller owns the view so it survives re-renders
/// and size changes.
struct PencilCanvas: UIViewRepresentable {
    @ObservedObject var controller: CanvasController

    func makeUIView(context: Context) -> CaptureCanvasView {
        let v = controller.makeCanvas()
        DispatchQueue.main.async { if controller.toolsVisible { v.becomeFirstResponder() } }
        return v
    }

    func updateUIView(_ v: CaptureCanvasView, context: Context) {}
}
