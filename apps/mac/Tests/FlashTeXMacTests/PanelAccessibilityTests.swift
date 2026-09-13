import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
import FlashTeXAccessibility
@testable import FlashTeXMac

/// Keyboard-only traversal of the secondary panels: the Settings scene
/// (`EditorPreferencesView`, ⌘,), the Durable History window
/// (`EditHistoryPanel`) and the Find in Project window (`ProjectSearchPanel`,
/// ⌘⇧F). Each panel is hosted off-screen through `NSHostingView` in a window
/// that is never made key (the tests must not steal focus).
///
/// What is measured in this in-process, never-key host:
/// - every AppKit-backed control SwiftUI creates for the panel (pop-up,
///   slider, steppers, switches, segmented control, text fields, lists)
///   accepts first responder, is enabled/visible as the table says, and
///   takes keyboard focus through `makeFirstResponder` in reading order
///   (top to bottom, left to right) — the move SwiftUI's Tab makes for such
///   controls — with the previous control resigning;
/// - SwiftUI wired each of them into its focus bridge (`nextKeyView` is set);
/// - the search panel's `@FocusState` puts the literal field's editor in as
///   first responder when the window opens;
/// - opening a panel while the editor text view is first responder in its
///   own window, focusing the panel's first control, then closing the panel
///   leaves the editor text view as first responder (`window.firstResponder`
///   before/after).
///
/// What cannot be measured here, and is pinned at the source level by
/// `CommandTableTests.testPanelFocusOrderMatchesThePanelSources` instead:
/// `nextValidKeyView`/`previousValidKeyView` return nil for every control of
/// a non-key hosting window (SwiftUI's `_NSCoreHostingView` bridge answers
/// only for a key window), SwiftUI-native buttons (`Button("Restore
/// Defaults")`, Search, Go to Match, …) are not AppKit views at all, and
/// SwiftUI materialises its accessibility tree (labels, hints, identifiers)
/// only for an assistive client — the AX API on this process answers
/// kAXErrorAPIDisabled (-25208) because Accessibility permission is not
/// granted. The traversal order and the spoken names are therefore the
/// `PanelFocusOrder` table, checked against the panel sources.
@MainActor
final class PanelAccessibilityTests: XCTestCase {

    // MARK: walk helpers

    enum KeyViewWalk {
        /// Depth-first descendants of `root` (excluding it).
        static func descendants(_ root: NSView) -> [NSView] {
            root.subviews.flatMap { [$0] + descendants($0) }
        }

        /// AppKit-backed controls keyboard focus can land on: first-responder
        /// capable, visible, enabled (scroll/clip views defer to their document view).
        static func focusables(in root: NSView) -> [NSView] {
            descendants(root).filter { v in
                guard v.acceptsFirstResponder, !v.isHiddenOrHasHiddenAncestor else { return false }
                if let c = v as? NSControl, !c.isEnabled { return false }
                if v is NSScrollView || v is NSClipView || v is NSScroller { return false } // scrollers are not Tab stops
                if let tv = v as? NSTextView, tv.isFieldEditor { return false } // the focused field's editor, not a control
                return true
            }
        }

        /// Reading order in AppKit window coordinates: top row first (larger y), then left to right.
        static func readingOrder(_ views: [NSView]) -> [NSView] {
            views.sorted { a, b in
                let fa = a.convert(a.bounds, to: nil), fb = b.convert(b.bounds, to: nil)
                if abs(fa.midY - fb.midY) > 6 { return fa.midY > fb.midY }
                return fa.minX < fb.minX
            }
        }

        static func describe(_ v: NSView) -> String {
            let f = v.convert(v.bounds, to: nil)
            return "\(type(of: v)) y=\(Int(f.minY)) x=\(Int(f.minX)) nextKeyView=\(v.nextKeyView.map { String(describing: type(of: $0)) } ?? "nil")"
        }

        /// The AppKit control that owns the window's first responder (a text
        /// field's field editor is an `NSTextView` whose delegate is the field).
        static func owner(of responder: NSResponder?) -> NSView? {
            if let tv = responder as? NSTextView, tv.isFieldEditor, let field = tv.delegate as? NSView { return field }
            return responder as? NSView
        }
    }

    // MARK: hosting

    private var windows: [NSWindow] = []

    override func tearDown() {
        for w in windows { w.orderOut(nil) }
        windows.removeAll()
        super.tearDown()
    }

    /// Hosts `view` in a titled, closable window that is ordered front but never key.
    private func host<V: View>(_ view: V, title: String, size: NSSize) async throws -> NSWindow {
        let hostView = NSHostingView(rootView: view)
        hostView.frame = NSRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: hostView.frame, styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = title
        window.isReleasedWhenClosed = false
        window.contentView = hostView
        window.orderFrontRegardless() // never makeKey
        windows.append(window)
        hostView.layoutSubtreeIfNeeded()
        // SwiftUI creates the AppKit-backed controls and runs onAppear on later turns.
        for _ in 0..<5 { try await Task.sleep(nanoseconds: 50_000_000) }
        return window
    }

    /// The real editor (`SourceEditorView`) bound to a model, as `ContentView` hosts it.
    private struct EditorHost: View {
        var model: ShellModel
        var body: some View {
            SourceEditorView(
                text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                selection: model.selection, pendingEdit: model.pendingEdit, marks: model.editorMarks, result: model.result,
                onCaretChange: { model.caretUTF16 = $0 },
                onSelectionChange: { model.caretLengthUTF16 = $0.length },
                onEditApplied: { edit, text in model.editApplied(edit, newText: text) })
        }
    }

    private func hostEditor(_ model: ShellModel) async throws -> (NSWindow, NSTextView) {
        let window = try await host(EditorHost(model: model), title: "FlashTeX", size: NSSize(width: 600, height: 400))
        var found: NSTextView?
        let deadline = Date().addingTimeInterval(10)
        while found == nil, Date() < deadline {
            found = TypingBenchDriver.findTextView(in: [window.contentView!])
            if found == nil { try await Task.sleep(nanoseconds: 10_000_000) }
        }
        let tv = try XCTUnwrap(found, "editor text view hosted")
        XCTAssertTrue(window.makeFirstResponder(tv))
        return (window, tv)
    }

    // MARK: shared assertions

    /// Focuses every AppKit-backed control of the panel in reading order and
    /// checks each one takes keyboard focus and the previous one gives it up.
    @discardableResult
    private func assertControlsTakeKeyboardFocus(in window: NSWindow, panel: String, atLeast: Int,
                                                 file: StaticString = #filePath, line: UInt = #line) -> [NSView] {
        let root = window.contentView!
        let ordered = KeyViewWalk.readingOrder(KeyViewWalk.focusables(in: root).filter { $0 !== root })
        print("[\(panel)] AppKit-backed controls in reading order (\(ordered.count)); full keyboard access \(NSApp.isFullKeyboardAccessEnabled ? "on" : "off"), key window: \(window.isKeyWindow)")
        for (i, v) in ordered.enumerated() { print("  \(i + 1). \(KeyViewWalk.describe(v))") }
        XCTAssertGreaterThanOrEqual(ordered.count, atLeast, "\(panel): AppKit-backed controls hosted", file: file, line: line)
        var previous: NSView?
        for v in ordered {
            XCTAssertTrue(window.makeFirstResponder(v), "\(panel): \(KeyViewWalk.describe(v)) refuses keyboard focus", file: file, line: line)
            let owner = KeyViewWalk.owner(of: window.firstResponder)
            XCTAssertTrue(owner === v, "\(panel): first responder is \(String(describing: window.firstResponder)) after focusing \(KeyViewWalk.describe(v))", file: file, line: line)
            if let previous { XCTAssertFalse(owner === previous, "\(panel): previous control kept focus", file: file, line: line) }
            XCTAssertNotNil(v.nextKeyView, "\(panel): SwiftUI wired \(KeyViewWalk.describe(v)) into its focus bridge", file: file, line: line)
            previous = v
        }
        // Documented limitation: the bridge answers Tab only for a key window.
        if let first = ordered.first, first.nextValidKeyView == nil {
            print("[\(panel)] nextValidKeyView is nil in the never-key host; Tab order is pinned by PanelFocusOrder against the source")
        }
        return ordered
    }

    /// Opens the panel while the editor is first responder, moves keyboard
    /// focus into the panel, closes it, and checks the editor kept it.
    private func assertFocusRestoration(panel window: NSWindow, editor: (NSWindow, NSTextView), name: String,
                                        file: StaticString = #filePath, line: UInt = #line) {
        let (editorWindow, tv) = editor
        XCTAssertTrue(editorWindow.firstResponder === tv, "\(name): editor is first responder before the panel opens", file: file, line: line)
        let focusables = KeyViewWalk.focusables(in: window.contentView!).filter { $0 !== window.contentView }
        if let first = KeyViewWalk.readingOrder(focusables).first {
            XCTAssertTrue(window.makeFirstResponder(first), "\(name): the first control takes keyboard focus", file: file, line: line)
            XCTAssertTrue(KeyViewWalk.owner(of: window.firstResponder) === first, file: file, line: line)
        }
        // Keyboard focus lives in the panel's window; the editor's window is untouched.
        XCTAssertTrue(editorWindow.firstResponder === tv, "\(name): opening the panel does not steal the editor's responder", file: file, line: line)
        window.close() // what ⌘W / Esc do
        RunLoop.main.run(until: Date().addingTimeInterval(0.05))
        XCTAssertFalse(window.isVisible, file: file, line: line)
        XCTAssertTrue(editorWindow.firstResponder === tv, "\(name): closing the panel leaves the editor first responder", file: file, line: line)
        XCTAssertTrue(editorWindow.isVisible, file: file, line: line)
    }

    // MARK: Settings (⌘,)

    /// The Capture conversion section (ConversionPreferencesView.swift, shown
    /// in the app's Settings after the editor sections): its AppKit-backed
    /// controls — provider picker, secure key field, model picker — take
    /// keyboard focus in reading order; the window is taller, so it is hosted
    /// at its own size. The key field is never given a value here.
    func testConversionPreferencesSectionControlsTakeKeyboardFocus() async throws {
        let defaults = UserDefaults(suiteName: "PanelAccessibilityTests.conversion.\(UUID().uuidString)")!
        let prefs = EditorPreferences(defaults: defaults)
        let window = try await host(EditorPreferencesView(preferences: prefs, showConversion: true), title: "Editor Preferences", size: NSSize(width: 480, height: 900))
        let controls = assertControlsTakeKeyboardFocus(in: window, panel: "Settings+Conversion", atLeast: 10)
        let kinds = controls.map { String(describing: type(of: $0)) }
        XCTAssertEqual(kinds.filter { $0.contains("Switch") }.count, 7, kinds.description)
        XCTAssertGreaterThanOrEqual(kinds.filter { $0.contains("PopupButton") }.count, 2, "provider + model pickers: \(kinds)")
        window.close()
    }

    func testPreferencesPanelControlsTakeKeyboardFocusAndTheEditorKeepsIt() async throws {
        let defaults = UserDefaults(suiteName: "PanelAccessibilityTests.prefs.\(UUID().uuidString)")!
        let prefs = EditorPreferences(defaults: defaults)
        let model = ShellModel()
        let editor = try await hostEditor(model)
        let window = try await host(EditorPreferencesView(preferences: prefs), title: "Editor Preferences", size: NSSize(width: 480, height: 640))
        // Pop-up, slider, size stepper, wrap switch, tab-width stepper, segmented
        // control, then the Typing switches: auto-close, completion list,
        // spelling, vim keybindings and the two error-lens rows (ErrorLens.swift).
        // The count said 4 before this branch: the error-lens rows landed
        // without updating it, and the vim switch is the seventh.
        let controls = assertControlsTakeKeyboardFocus(in: window, panel: "Settings", atLeast: 8)
        let kinds = controls.map { String(describing: type(of: $0)) }
        XCTAssertTrue(kinds.contains { $0.contains("PopupButton") || $0.contains("PopUpButton") }, kinds.description)
        XCTAssertTrue(kinds.contains { $0.contains("Slider") }, kinds.description)
        XCTAssertEqual(kinds.filter { $0.contains("Stepper") }.count, 2, kinds.description)
        XCTAssertEqual(kinds.filter { $0.contains("Switch") }.count, 7, kinds.description)
        XCTAssertTrue(kinds.contains { $0.contains("SegmentedControl") }, kinds.description)
        // Reading order agrees with the table: pop-up first, the typing switches last.
        XCTAssertTrue(kinds.first?.contains("Popup") == true || kinds.first?.contains("PopUp") == true, kinds.description)
        XCTAssertTrue(kinds.last?.contains("Switch") == true, kinds.description)
        // The table's shortcut is the system Settings item.
        XCTAssertEqual(PanelFocusOrder.panels[0].command.entry.shortcuts, ["⌘,"])
        assertFocusRestoration(panel: window, editor: editor, name: "Settings")
    }

    // MARK: Durable History

    func testHistoryPanelListTakesKeyboardFocusAndTheEditorKeepsIt() async throws {
        let model = ShellModel()
        let editor = try await hostEditor(model)
        let window = try await host(EditHistoryPanel().environment(model), title: "Durable History", size: NSSize(width: 420, height: 400))
        // Without a preview controller every button is disabled (the table says
        // "controller attached"); the stacks list is the keyboard's landing spot.
        let controls = assertControlsTakeKeyboardFocus(in: window, panel: "Durable History", atLeast: 1)
        XCTAssertTrue(controls.contains { $0 is NSTableView }, "Durable History: the undo/redo list takes focus")
        XCTAssertFalse(model.controllerAttached)
        XCTAssertFalse(controls.contains { $0 is NSButton }, "no enabled button without a controller")
        XCTAssertEqual(PanelFocusOrder.panels[1].controls.filter { $0.when == "controller attached" }.count, 3)
        assertFocusRestoration(panel: window, editor: editor, name: "Durable History")
    }

    // MARK: Find in Project (⌘⇧F)

    func testSearchPanelFocusesTheLiteralFieldControlsTakeKeyboardFocusAndTheEditorKeepsIt() async throws {
        let model = ShellModel()
        let editor = try await hostEditor(model)
        let window = try await host(ProjectSearchPanel().environment(model), title: "Find in Project", size: NSSize(width: 760, height: 600))
        // `@FocusState` on appear: the literal field's editor is first responder.
        let deadline = Date().addingTimeInterval(3)
        while KeyViewWalk.owner(of: window.firstResponder) as? NSTextField == nil, Date() < deadline {
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        let initial = KeyViewWalk.owner(of: window.firstResponder) as? NSTextField
        XCTAssertNotNil(initial, "Find in Project: the literal field takes focus on open (first responder: \(String(describing: window.firstResponder)))")
        XCTAssertEqual(initial?.placeholderString, "Find in project (case-sensitive literal, no regex)")
        // Literal field, scope pop-up, match-limit stepper, replacement field (no results list without a search).
        let controls = assertControlsTakeKeyboardFocus(in: window, panel: "Find in Project", atLeast: 4)
        let fields = controls.compactMap { $0 as? NSTextField }
        XCTAssertEqual(fields.map(\.placeholderString), ["Find in project (case-sensitive literal, no regex)", "Replace with (exact bytes)"],
                       "literal field before the replacement field")
        XCTAssertTrue(controls.first === initial, "the literal field is the first control in reading order")
        XCTAssertTrue(controls.contains { String(describing: type(of: $0)).contains("Popup") || String(describing: type(of: $0)).contains("PopUp") })
        XCTAssertTrue(controls.contains { String(describing: type(of: $0)).contains("Stepper") })
        XCTAssertEqual(PanelFocusOrder.panels[2].command.entry.shortcuts, ["⌘⇧F"])
        assertFocusRestoration(panel: window, editor: editor, name: "Find in Project")
    }
}
