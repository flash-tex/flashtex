import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// Owner report: "in vim mode, commands / the current modes show up at the
/// bottom of the app window rather than at the bottom of the editor window.
/// So when there's the 'problem' window below where you're editing, it feels
/// unnatural."
///
/// Vim's status line belongs to the window being edited, so `VimStatusLine`
/// is the last row of the **editor pane** (ContentView.swift), not an item of
/// the window's global `StatusBar`. These tests pin that with real geometry in
/// the whole `ContentView`, as the app builds it: turning Vim on takes one
/// caption row off the bottom of the source editor **inside** the editor half
/// of the split — which is above the Problems panel and beside the preview —
/// and leaves the editor pane's own frame alone, so nothing was added to the
/// window's status bar. With Vim off the row costs nothing at all.
@MainActor
final class VimStatusLinePlacementTests: XCTestCase {
    private var window: NSWindow?
    private var vimWasOn = false

    /// A caption row with `.padding(.vertical, 3)`: more than a hairline, far
    /// less than a panel.
    private static let statusLineHeights: ClosedRange<CGFloat> = 6...40

    override func setUp() async throws {
        vimWasOn = EditorPreferences.shared.vimKeybindings
        EditorPreferences.shared.vimKeybindings = false
    }

    override func tearDown() async throws {
        EditorPreferences.shared.vimKeybindings = vimWasOn
        window?.orderOut(nil)
        window = nil
    }

    // MARK: harness

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    /// SwiftUI applies a preference change and re-lays out over a few turns.
    private func settle() async throws { try await Task.sleep(nanoseconds: 400_000_000) }

    /// The `NSScrollView` the source editor lives in.
    private func editorScrollView() throws -> NSScrollView {
        let content = try XCTUnwrap(window?.contentView, "hosted content view")
        let tv = try XCTUnwrap(TypingBenchDriver.findTextView(in: [content]), "source editor text view")
        return try XCTUnwrap(tv.enclosingScrollView, "the source editor's scroll view")
    }

    /// The source editor's slot in the editor pane's `VStack`, in window
    /// coordinates (y grows upwards).
    private func editorFrame() throws -> NSRect {
        let scroll = try editorScrollView()
        return scroll.convert(scroll.bounds, to: nil)
    }

    /// The editor half of the `HSplitView` — everything `EditorPane` draws:
    /// the tab bar, the source editor, the Vim status line, the capture and
    /// bridge bars. The Problems panel and the window's status bar are
    /// outside it, below.
    private func editorPaneFrame() throws -> NSRect {
        var view: NSView? = try editorScrollView()
        while let v = view {
            if String(describing: type(of: v)).contains("SplitViewItemViewWrapper") {
                return v.convert(v.bounds, to: nil)
            }
            view = v.superview
        }
        XCTFail("no split-view item wrapper above the source editor")
        return .zero
    }

    private func measure() throws -> (editor: NSRect, pane: NSRect) {
        (try editorFrame(), try editorPaneFrame())
    }

    private func host(_ model: ShellModel) async throws -> NSTextView {
        HostedWindowSupport.prepare()
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1400, height: 900), styleMask: [.titled],
                              backing: .buffered, defer: false)
        let hosting = NSHostingView(rootView: ContentView().environment(model).environmentObject(NearbyState()))
        window.contentView = hosting
        window.orderFrontRegardless()
        hosting.layoutSubtreeIfNeeded()
        self.window = window
        var found: NSTextView?
        try await waitUntil("editor text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        try await settle()
        return try XCTUnwrap(found)
    }

    private func hostedModel(problemsVisible: Bool) async throws -> NSTextView {
        let model = ShellModel()
        model.replaceProject(entryText: "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n")
        model.problemsVisible = problemsVisible
        return try await host(model)
    }

    private func turnVimOn() async throws {
        EditorPreferences.shared.vimKeybindings = true
        try await waitUntil("Vim mode is armed", timeout: 5) { VimMode.Status.shared.indicator != nil }
        try await settle()
    }

    private func turnVimOff() async throws {
        EditorPreferences.shared.vimKeybindings = false
        try await waitUntil("Vim mode is off", timeout: 5) { VimMode.Status.shared.indicator == nil }
        try await settle()
    }

    /// Turning Vim on must insert exactly one status-line row at the bottom of
    /// the source editor, inside the editor pane.
    private func assertStatusLineIsTheEditorsBottomRow(_ before: (editor: NSRect, pane: NSRect),
                                                       _ after: (editor: NSRect, pane: NSRect),
                                                       _ what: String) {
        XCTAssertEqual(after.pane, before.pane,
                       "\(what): the editor pane's own frame must not move — a row added to the window's status bar would shrink it instead")
        let lost = before.editor.height - after.editor.height
        XCTAssertTrue(Self.statusLineHeights.contains(lost),
                      "\(what): turning Vim on must take one caption row off the source editor (lost \(lost) pt; editor \(before.editor) -> \(after.editor))")
        XCTAssertEqual(after.editor.minY - before.editor.minY, lost, accuracy: 1,
                       "\(what): the row must come off the BOTTOM of the source editor, where vim puts the status line")
        XCTAssertEqual(after.editor.maxY, before.editor.maxY, accuracy: 1,
                       "\(what): the top of the source editor must not move")
    }

    // MARK: the report

    /// The bug itself. With the Problems panel open, the mode indicator must
    /// stay with the text it describes: a row inside the editor pane, which
    /// sits above the Problems panel — not an item of the window's status bar
    /// two panes below the caret.
    func testStatusLineIsTheEditorsBottomRowWithTheProblemsPanelOpen() async throws {
        _ = try await hostedModel(problemsVisible: true)
        let before = try measure()
        try await turnVimOn()
        XCTAssertEqual(VimMode.Status.shared.indicator, "-- NORMAL --")
        assertStatusLineIsTheEditorsBottomRow(before, try measure(), "Problems panel open")
    }

    /// The same with the preview alone beside the editor: the status line is
    /// the editor's, so it lives in the editor half of the split and cannot
    /// span the window the way the status bar does.
    func testStatusLineIsTheEditorsBottomRowBesideThePreview() async throws {
        _ = try await hostedModel(problemsVisible: false)
        let before = try measure()
        try await turnVimOn()
        let after = try measure()
        assertStatusLineIsTheEditorsBottomRow(before, after, "preview beside the editor")
        let windowWidth = try XCTUnwrap(window).frame.width
        XCTAssertLessThan(after.pane.width, windowWidth - 100,
                          "the editor pane is one half of the split, so the status line inside it does not span the window")
    }

    /// The `:` command line is part of the same status line: typing `:` fills
    /// the row that is already there rather than adding a second one.
    func testCommandLineSharesTheSameRow() async throws {
        let tv = try await hostedModel(problemsVisible: true)
        let before = try measure()
        try await turnVimOn()
        let armed = try measure()
        assertStatusLineIsTheEditorsBottomRow(before, armed, "`:` command line")
        XCTAssertTrue(try XCTUnwrap(tv.window).makeFirstResponder(tv))
        let colon = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [],
                                     timestamp: ProcessInfo.processInfo.systemUptime,
                                     windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: ":",
                                     charactersIgnoringModifiers: ":", isARepeat: false, keyCode: 0)!
        tv.keyDown(with: colon)
        try await waitUntil("the `:` line is showing", timeout: 3) { (VimMode.Status.shared.commandLine ?? "").hasPrefix(":") }
        try await settle()
        let typing = try measure()
        XCTAssertEqual(typing.editor, armed.editor,
                       "the `:` command line joins the status line already under the editor; it must not add a row")
        XCTAssertEqual(typing.pane, armed.pane, "the `:` command line must not resize the editor pane")
    }

    // MARK: off costs nothing

    /// With Vim off there is no indicator and no empty strip: the editor is
    /// exactly as tall as it is with the feature absent, before and after.
    func testVimOffTakesNoSpaceAtAll() async throws {
        _ = try await hostedModel(problemsVisible: true)
        XCTAssertNil(VimMode.Status.shared.indicator, "Vim is off, so there is no mode to show")
        let off = try measure()
        try await turnVimOn()
        let on = try measure()
        XCTAssertLessThan(on.editor.height, off.editor.height, "turning Vim on adds the status line")
        try await turnVimOff()
        let offAgain = try measure()
        XCTAssertNil(VimMode.Status.shared.indicator)
        XCTAssertEqual(offAgain.editor, off.editor,
                       "turning Vim off gives the row back: the editor must not keep an empty strip")
        XCTAssertEqual(offAgain.pane, off.pane)
    }
}
