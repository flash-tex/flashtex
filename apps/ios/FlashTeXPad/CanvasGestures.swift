import CoreGraphics
import Foundation
import UIKit

// The capture canvas's gesture and layout rules, kept free of views so the
// unit tests can drive them directly (`CanvasGestureTests`).
//
// Gestures (Notability / GoodNotes conventions):
// - two-finger double-tap anywhere on the canvas: undo
// - three-finger double-tap: redo
// - Apple Pencil double-tap: undo by default (the owner's request), or the
//   system's Apple Pencil setting when "Pencil double-tap" is set to
//   "System setting". An Apple Pencil setting of **Off** is always honoured,
//   in either mode: turning double-tap off system-wide means nothing happens.
//
// Each gesture has an on/off switch in the canvas settings popover.

/// What the Pencil's double-tap does on the canvas.
enum PencilDoubleTapMode: String, CaseIterable, Identifiable {
    /// Undo the last stroke (default). The system "Off" setting still wins.
    case undo
    /// Whatever Settings › Apple Pencil › Double-tap says (switch to eraser,
    /// switch to the last tool, show the palette, off).
    case system

    var id: String { rawValue }
    var label: String {
        switch self {
        case .undo: return "Undo"
        case .system: return "System setting"
        }
    }
}

/// The user's canvas preferences (`@AppStorage`, keys below).
struct CanvasSettings: Equatable {
    static let twoFingerUndoKey = "flashtexpad.canvas.twoFingerUndo"
    static let threeFingerRedoKey = "flashtexpad.canvas.threeFingerRedo"
    static let pencilDoubleTapKey = "flashtexpad.canvas.pencilDoubleTap"
    static let fingerDrawingKey = "flashtexpad.canvas.fingerDrawing"
    static let autoHideControlsKey = "flashtexpad.canvas.autoHideControls"
    static let allKeys = [twoFingerUndoKey, threeFingerRedoKey, pencilDoubleTapKey, fingerDrawingKey, autoHideControlsKey]

    var twoFingerUndo = true
    var threeFingerRedo = true
    var pencilDoubleTap = PencilDoubleTapMode.undo
    /// `PKCanvasView.drawingPolicy = .anyInput` (a finger draws, two fingers
    /// scroll) when on; `.pencilOnly` (a finger scrolls) when off.
    var fingerDrawing = true
    /// Fade the floating controls while a stroke is being drawn.
    var autoHideControls = true
}

/// A system-level description of the Pencil's preferred double-tap action,
/// decoupled from `UIPencilPreferredAction` so tests can name every case.
enum PencilSystemAction: Equatable {
    case ignore, switchEraser, switchPrevious, showPalette, other

    init(_ a: UIPencilPreferredAction) {
        switch a {
        case .ignore: self = .ignore
        case .switchEraser: self = .switchEraser
        case .switchPrevious: self = .switchPrevious
        case .showColorPalette, .showInkAttributes, .showContextualPalette: self = .showPalette
        default: self = .other // runSystemShortcut and anything newer: the system handles it
        }
    }
}

enum CanvasGesture: Equatable {
    case pencilDoubleTap(system: PencilSystemAction)
    case twoFingerDoubleTap
    case threeFingerDoubleTap
}

enum CanvasCommand: Equatable {
    case undo, redo, toggleEraser, switchPrevious, showToolPicker, none
}

/// Maps a gesture to a command under the current settings.
struct CanvasGestureRouter {
    var settings: CanvasSettings

    /// `toolPickerVisible`: while PencilKit's tool picker is up it applies the
    /// system Pencil action itself, so in `.system` mode we do nothing more
    /// (doing it again would switch twice and cancel out).
    func command(for gesture: CanvasGesture, toolPickerVisible: Bool) -> CanvasCommand {
        switch gesture {
        case .twoFingerDoubleTap:
            return settings.twoFingerUndo ? .undo : .none
        case .threeFingerDoubleTap:
            return settings.threeFingerRedo ? .redo : .none
        case .pencilDoubleTap(let system):
            if system == .ignore { return .none } // "Off" in Settings › Apple Pencil is always honoured
            switch settings.pencilDoubleTap {
            case .undo:
                return .undo
            case .system:
                if toolPickerVisible { return .none }
                switch system {
                case .switchEraser: return .toggleEraser
                case .switchPrevious: return .switchPrevious
                case .showPalette: return .showToolPicker
                case .ignore, .other: return .none
                }
            }
        }
    }
}

/// Performs undo / redo on the canvas's own undo manager and says what
/// happened, for the toast and the VoiceOver announcement.
struct CanvasUndoHandler {
    enum Outcome: Equatable {
        case undid, redid, nothingToUndo, nothingToRedo

        var toast: String {
            switch self {
            case .undid: return "Undo"
            case .redid: return "Redo"
            case .nothingToUndo: return "Nothing to undo"
            case .nothingToRedo: return "Nothing to redo"
            }
        }
    }

    let undoManager: UndoManager

    @discardableResult
    func perform(_ command: CanvasCommand) -> Outcome? {
        switch command {
        case .undo:
            guard undoManager.canUndo else { return .nothingToUndo }
            undoManager.undo()
            return .undid
        case .redo:
            guard undoManager.canRedo else { return .nothingToRedo }
            undoManager.redo()
            return .redid
        default:
            return nil
        }
    }
}

/// PencilKit's tool picker applies the system Pencil double-tap (normally
/// "switch to eraser") on its own while it is visible. When our setting says
/// double-tap means Undo, that switch is unwanted: this remembers the tool in
/// use at the tap and says which tool to put back if the picker changed it
/// within `window` seconds on either side of the tap (the order of the two
/// callbacks is not specified).
struct ToolRestoreGuard<Tool> {
    var window: TimeInterval = 0.35
    private var current: Tool?
    private var previous: Tool?
    private var changedAt: TimeInterval = -.infinity
    private var tapAt: TimeInterval = -.infinity
    private var toolAtTap: Tool?
    private var restoring = false

    init(initial: Tool? = nil, window: TimeInterval = 0.35) {
        current = initial
        self.window = window
    }

    /// The picker reported a tool change. Returns the tool to put back when
    /// the change was the picker reacting to a double-tap we turned into Undo.
    mutating func toolChanged(to tool: Tool, at t: TimeInterval) -> Tool? {
        if restoring { restoring = false; current = tool; changedAt = t; return nil }
        previous = current
        current = tool
        changedAt = t
        if t - tapAt <= window, let keep = toolAtTap {
            toolAtTap = nil
            restoring = true
            return keep
        }
        return nil
    }

    /// A double-tap we handled as Undo. Returns the tool to put back when the
    /// picker already switched just before this callback.
    mutating func doubleTapped(at t: TimeInterval) -> Tool? {
        tapAt = t
        if t - changedAt <= window, let keep = previous {
            toolAtTap = nil
            restoring = true
            return keep
        }
        toolAtTap = current
        return nil
    }

    /// Our own restore finished without the picker echoing a change.
    mutating func restored() { restoring = false }
}

/// Canvas geometry: the canvas fills the scene and grows downward as the
/// drawing approaches its bottom, so there is always room to keep writing.
/// Stroke coordinates are the canvas's content coordinates, anchored at the
/// top-left, so rotating or resizing the window never moves a stroke.
enum CanvasLayout {
    /// How much blank space to keep below the lowest stroke, as a fraction of
    /// the visible height.
    static let growthSlack: CGFloat = 0.5
    /// Blank margin kept around the ink when the capture is cropped.
    static let captureMargin: CGFloat = 32
    /// Smallest capture, so a single symbol still reaches the Mac legibly.
    static let minimumCapture = CGSize(width: 320, height: 200)
    /// Longest side of the PNG in pixels.
    static let maxCapturePixels: CGFloat = 4096

    static func contentSize(viewport: CGSize, drawingBounds: CGRect) -> CGSize {
        guard viewport.width > 0, viewport.height > 0 else { return viewport }
        var height = viewport.height
        if !drawingBounds.isNull, !drawingBounds.isEmpty {
            height = max(height, drawingBounds.maxY + viewport.height * growthSlack)
        }
        return CGSize(width: viewport.width, height: height.rounded(.up))
    }

    /// The region of the drawing sent to the Mac: the ink plus a margin, at
    /// least `minimumCapture`, centred on the ink. `nil` for an empty drawing.
    static func captureRect(drawingBounds b: CGRect) -> CGRect? {
        guard !b.isNull, !b.isInfinite, b.width > 0 || b.height > 0 else { return nil }
        var r = b.insetBy(dx: -captureMargin, dy: -captureMargin)
        if r.width < minimumCapture.width { r = r.insetBy(dx: -(minimumCapture.width - r.width) / 2, dy: 0) }
        if r.height < minimumCapture.height { r = r.insetBy(dx: 0, dy: -(minimumCapture.height - r.height) / 2) }
        return r.integral
    }

    /// Render scale: 2× unless that would exceed `maxCapturePixels`.
    static func captureScale(for rect: CGRect) -> CGFloat {
        let longest = max(rect.width, rect.height)
        guard longest > 0 else { return 2 }
        return min(2, maxCapturePixels / longest)
    }
}
