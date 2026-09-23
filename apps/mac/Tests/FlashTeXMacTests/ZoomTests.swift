import AppKit
import HostedWindows
import XCTest
@testable import FlashTeXMac

/// Preview zoom (PreviewZoom.swift): multiplier math and clamping, the View
/// menu commands on `ShellModel`, fit-width reset, persistence, and the editor
/// font-size commands over `EditorPreferences`.
@MainActor
final class ZoomTests: XCTestCase {
    func testMultiplierIsClampedTo25To400Percent() {
        XCTAssertEqual(PreviewZoom.clamped(0.1), 0.25)
        XCTAssertEqual(PreviewZoom.clamped(10), 4)
        XCTAssertEqual(PreviewZoom.clamped(1.5), 1.5)
        XCTAssertEqual(PreviewZoom.clamped(.nan), 1)
        XCTAssertEqual(PreviewZoom.clamped(0), 1)
        XCTAssertEqual(PreviewZoom.clamped(-2), 1)
    }

    func testScaleIsFitTimesZoomAndActualSizeInvertsFit() {
        XCTAssertEqual(PreviewZoom.scale(fit: 0.5, zoom: 2), 1, accuracy: 1e-9)
        XCTAssertEqual(PreviewZoom.scale(fit: 0.8, zoom: 1), 0.8, accuracy: 1e-9)
        XCTAssertEqual(PreviewZoom.scale(fit: 0.8, zoom: 100), 3.2, accuracy: 1e-9, "zoom is clamped before multiplying")
        XCTAssertEqual(PreviewZoom.actualSizeZoom(fit: 0.5), 2, accuracy: 1e-9)
        XCTAssertEqual(PreviewZoom.actualSizeZoom(fit: 0.1), 4, "actual size clamps at 400 %")
        XCTAssertEqual(PreviewZoom.actualSizeZoom(fit: 0), 1)
        XCTAssertEqual(PreviewZoom.percent(fit: 0.5, zoom: 2), 100)
        XCTAssertEqual(PreviewZoom.percent(fit: 0.62, zoom: 1), 62)
    }

    func testMenuCommandsChangeTheModelAndFitWidthRestoresOne() {
        let model = ShellModel()
        model.previewZoom = 1
        model.previewFitScale = 0.5
        model.previewZoomIn()
        XCTAssertEqual(model.previewZoom, 1.25, accuracy: 1e-9)
        model.previewZoomOut()
        XCTAssertEqual(model.previewZoom, 1, accuracy: 1e-9)
        model.previewActualSize()
        XCTAssertEqual(model.previewZoom, 2, accuracy: 1e-9, "1 pt per screen point at fit 0.5")
        for _ in 0..<20 { model.previewZoomIn() }
        XCTAssertEqual(model.previewZoom, 4, "zoom in stops at 400 %")
        for _ in 0..<40 { model.previewZoomOut() }
        XCTAssertEqual(model.previewZoom, 0.25, "zoom out stops at 25 %")
        model.previewFitWidth()
        XCTAssertEqual(model.previewZoom, 1)
        model.previewZoom = 99
        XCTAssertEqual(model.previewZoom, 4, "direct assignment is clamped")
    }

    func testZoomPersistsInUserDefaults() {
        let suite = "ZoomTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        XCTAssertEqual(PreviewZoom.load(defaults), 1, "absent value loads as fit width")
        PreviewZoom.store(2.5, in: defaults)
        XCTAssertEqual(PreviewZoom.load(defaults), 2.5)
        defaults.set(0.01, forKey: PreviewZoom.storageKey)
        XCTAssertEqual(PreviewZoom.load(defaults), 0.25, "stored values are clamped on load")
        defaults.set("junk", forKey: PreviewZoom.storageKey)
        XCTAssertEqual(PreviewZoom.load(defaults), 1, "invalid stored value loads as the default")

        // The model writes through to the standard suite on every change.
        let model = ShellModel()
        let before = UserDefaults.standard.object(forKey: PreviewZoom.storageKey)
        defer { if let before { UserDefaults.standard.set(before, forKey: PreviewZoom.storageKey) } else { UserDefaults.standard.removeObject(forKey: PreviewZoom.storageKey) } }
        model.previewZoom = 1.75
        XCTAssertEqual(PreviewZoom.load(.standard), 1.75)
    }

    func testEditorFontSizeCommandsDriveThePreference() {
        let prefs = EditorPreferences.shared
        let original = prefs.fontSize
        defer { prefs.fontSize = original }
        let model = ShellModel()
        model.resetEditorFontSize()
        XCTAssertEqual(prefs.fontSize, EditorPreferences.defaultSnapshot.fontSize)
        model.increaseEditorFontSize()
        XCTAssertEqual(prefs.fontSize, EditorPreferences.defaultSnapshot.fontSize + 1)
        model.decreaseEditorFontSize(); model.decreaseEditorFontSize()
        XCTAssertEqual(prefs.fontSize, EditorPreferences.defaultSnapshot.fontSize - 1)
        for _ in 0..<60 { model.increaseEditorFontSize() }
        XCTAssertEqual(prefs.fontSize, EditorPreferences.fontSizeRange.upperBound, "clamped by EditorPreferences")
        for _ in 0..<60 { model.decreaseEditorFontSize() }
        XCTAssertEqual(prefs.fontSize, EditorPreferences.fontSizeRange.lowerBound)
        model.resetEditorFontSize()
        XCTAssertEqual(prefs.fontSize, 13)
    }

    // MARK: EditorFontMagnifier targeting (preview pinch-to-zoom regression)

    /// A synthetic event whose `.window`/`.locationInWindow` `targets(_:_:)`
    /// reads; the type is otherwise irrelevant (it never reads `.type` or
    /// `.magnification`), and AppKit has no public constructor for a real
    /// `.magnify` event to synthesize in a test.
    private func event(at point: NSPoint, in window: NSWindow?) -> NSEvent {
        NSEvent.keyEvent(with: .keyDown, location: point, modifierFlags: [], timestamp: 0,
                         windowNumber: window?.windowNumber ?? 0, context: nil, characters: "",
                         charactersIgnoringModifiers: "", isARepeat: false, keyCode: 0)!
    }

    /// The editor and preview live under separate `NSHostingController`s
    /// since the appearance overhaul (#653/#666); `targets` must key off
    /// AppKit's own hit-test of where the pointer actually is now, not a
    /// hand-rolled `bounds.contains` snapshot that risks going stale and
    /// swallowing a pinch meant for a sibling view.
    func testEditorFontMagnifierTargetsOnlyTheEditorsOwnScrollView() {
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 400, height: 200),
                                                 styleMask: [.titled], backing: .buffered, defer: false)
        defer { window.orderOut(nil) }
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 400, height: 200))
        // Stand-ins for the editor's scroll view and a sibling pane (the
        // preview) side by side, exactly the split-view shape in
        // WorkspaceSplit.swift — not nested one inside the other.
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 200, height: 200))
        let preview = NSView(frame: NSRect(x: 200, y: 0, width: 200, height: 200))
        container.addSubview(scroll)
        container.addSubview(preview)
        window.contentView = container

        let overEditor = event(at: NSPoint(x: 50, y: 100), in: window)
        let overPreview = event(at: NSPoint(x: 250, y: 100), in: window)
        let noWindow = event(at: NSPoint(x: 50, y: 100), in: nil)
        let otherWindow = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 10, height: 10),
                                                      styleMask: [.titled], backing: .buffered, defer: false)
        defer { otherWindow.orderOut(nil) }
        let fromAnotherWindow = event(at: NSPoint(x: 5, y: 5), in: otherWindow)

        XCTAssertTrue(EditorFontMagnifier.targets(scroll, overEditor), "a pinch over the editor's own region targets it")
        XCTAssertFalse(EditorFontMagnifier.targets(scroll, overPreview), "a pinch over the sibling preview pane must not be swallowed")
        XCTAssertFalse(EditorFontMagnifier.targets(scroll, noWindow), "an event with no resolvable window never matches")
        XCTAssertFalse(EditorFontMagnifier.targets(scroll, fromAnotherWindow), "an event from a different window never matches")
    }
}
