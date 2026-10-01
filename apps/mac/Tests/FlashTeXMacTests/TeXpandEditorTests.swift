import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac
@testable import FlashTeXEditorCore

/// PLAN M3 acceptance in the real text view: `;enum3` Tab expands, Esc
/// leaves the literal, one undo restores it; Tab precedence; placeholders
/// selected; the completion list stays shut while capturing. Hosted
/// windows are non-activating and parked off-screen: the app never comes
/// forward and focus is never taken.
@MainActor
final class TeXpandEditorTests: XCTestCase {
    private var window: NSWindow!
    private var tv: CompletingTextView!

    override func setUp() async throws {
        var on = TeXpand.Settings()
        on.enabled = true
        TeXpandPreferences.override = on
        HostedWindowSupport.prepare()
        window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless()
        window.makeFirstResponder(tv)
        tv.allowsUndo = true
        tv.projectDocumentClass = { "article" }
    }

    override func tearDown() async throws {
        window.orderOut(nil)
        TeXpandPreferences.override = nil
    }

    private func key(_ chars: String, code: UInt16, flags: NSEvent.ModifierFlags = []) {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                                 windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: chars,
                                 charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        tv.keyDown(with: e)
    }

    private func type(_ s: String) { for ch in s { key(String(ch), code: 0) } }
    private func tab() { key("\t", code: 48) }
    private func esc() { key("\u{1B}", code: 53) }

    /// Ends the run-loop turn, as separate key events do: the undo manager
    /// groups by event, so keystrokes sent back to back in one turn would
    /// otherwise share an undo group with what follows.
    private func endEvent() {
        RunLoop.main.run(until: Date().addingTimeInterval(0.02))
    }

    private func start(_ text: String) {
        tv.string = text
        tv.setSelectedRange(NSRange(location: (text as NSString).length, length: 0))
        tv.undoManager?.removeAllActions()
    }

    func testEnum3TabExpandsAndOneUndoRestoresTheLiteral() throws {
        start("")
        type(";enum3")
        XCTAssertTrue(tv.texpand.isCapturing)
        XCTAssertEqual(tv.texpand.region, NSRange(location: 0, length: 6))
        XCTAssertNotNil(tv.texpand.preview, "a live preview while capturing")
        endEvent()
        tab()
        endEvent()
        let unit = EditorPreferences.shared.indentString
        XCTAssertEqual(tv.string, "\\begin{enumerate}\n\(unit)\\item \n\(unit)\\item \n\(unit)\\item \n\\end{enumerate}")
        XCTAssertEqual(tv.selectedRange().location, ("\\begin{enumerate}\n\(unit)\\item " as NSString).length, "the caret in the first item")
        XCTAssertTrue(tv.isSnippetActive, "Tab visits the other items")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Expand Abbreviation")
        XCTAssertNil(tv.texpand.region)
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, ";enum3", "one undo restores the literal")
        XCTAssertFalse(tv.texpand.isCapturing, "and capture does not re-arm")
        tv.setSelectedRange(NSRange(location: 6, length: 0))
        tab()
        XCTAssertEqual(tv.string, ";enum3\t", "the undone literal is marked: Tab is a Tab again (a bare view inserts \\t)")
    }

    func testEscLeavesTheLiteralText() {
        start("See ")
        type(";sec{Intro}")
        XCTAssertTrue(tv.texpand.isCapturing)
        esc()
        XCTAssertEqual(tv.string, "See ;sec{Intro}")
        XCTAssertFalse(tv.texpand.isCapturing)
        XCTAssertNil(tv.session, "Esc ended the capture; it did not open the completion list")
        tab()
        XCTAssertNotEqual(tv.string, "See \\section{Intro}", "the Esc'd literal is not expanded by Tab")
    }

    func testPlaceholdersAreSelectedAndTabWalksThem() {
        start("")
        type(";fig")
        tab()
        let sel = tv.selectedRange()
        XCTAssertEqual((tv.string as NSString).substring(with: sel), "width=0.8\\linewidth", "the first placeholder is selected")
        XCTAssertTrue(tv.isSnippetActive, "a selected placeholder keeps the snippet")
        type("scale=1")
        XCTAssertTrue(tv.string.contains("\\includegraphics[scale=1]{}"), "typing replaced the placeholder")
        tab()
        XCTAssertEqual(tv.selectedRange().length, 0, "an empty field is a caret")
        type("a.pdf")
        key("\t", code: 48, flags: .shift)
        XCTAssertEqual((tv.string as NSString).substring(with: tv.selectedRange()), "scale=1", "⇧Tab reselects what was typed there")
    }

    func testTabPrecedence() {
        // Incomplete capture: Tab is swallowed (no Tab character) and explained.
        start("")
        type(";sec{Intro")
        tab()
        XCTAssertEqual(tv.string, ";sec{Intro")
        XCTAssertEqual(tv.texpand.diagnostic, "expected `}`")
        // Capture inside an active snippet: the capture wins over the stop.
        start("")
        type(";items2")
        tab()
        XCTAssertTrue(tv.isSnippetActive)
        type(";sec")
        tab()
        XCTAssertTrue(tv.string.contains("\\item \\section{}"), tv.string)
        // No capture: snippet stops, then indentation, as before.
        start("x")
        tab()
        XCTAssertNotEqual(tv.string, "x", "plain Tab still reaches the editor")
    }

    func testCompletionListStaysShutWhileCapturing() async throws {
        tv.automaticCompletionDelay = 0
        start("")
        type(";sec")
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertNil(tv.session)
        XCTAssertFalse(tv.hasPendingAutomaticCompletion)
    }

    private final class TextBox { var text = "" }

    private struct Host: View {
        let box: TextBox
        var body: some View {
            SourceEditorView(text: Binding(get: { box.text }, set: { box.text = $0 }), autoClosePairs: ["{", "["],
                             projectDocumentClass: { "article" })
        }
    }

    /// Through the hosted `SourceEditorView`: its coordinator auto-closes
    /// `{` (inserting `{}`) and types over the `}`, and the capture follows.
    func testThroughTheRealEditorWithAutoClose() throws {
        let box = TextBox()
        let host = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        host.contentView = NSHostingView(rootView: Host(box: box))
        host.orderFrontRegardless()
        defer { host.orderOut(nil) }
        var found: NSTextView?
        let deadline = Date().addingTimeInterval(10)
        while found == nil, Date() < deadline {
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.005))
            found = TypingBenchDriver.findTextView(in: [host.contentView!])
        }
        tv = try XCTUnwrap(found as? CompletingTextView)
        XCTAssertTrue(host.makeFirstResponder(tv))
        tv.allowsUndo = true
        type(";sec{")
        XCTAssertEqual(tv.string, ";sec{}", "the editor auto-closed the brace")
        type("Intro}")
        XCTAssertEqual(tv.string, ";sec{Intro}", "and the } was typed over")
        XCTAssertEqual(tv.texpand.preview, "\\section{Intro}")
        endEvent()
        tab()
        XCTAssertEqual(tv.string, "\\section{Intro}")
        endEvent()
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, ";sec{Intro}", "one undo restores the literal")
        RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.05))
        XCTAssertEqual(box.text, ";sec{Intro}", "the binding follows")
    }

    func testOffByDefaultDoesNothing() {
        TeXpandPreferences.override = TeXpand.Settings()
        start("")
        type(";enum3")
        XCTAssertFalse(tv.texpand.isCapturing)
        tab()
        XCTAssertTrue(tv.string.hasPrefix(";enum3"), "with the master switch off, Tab is the editor's: \(tv.string.debugDescription)")
    }

    func testTheSettingsSwitchTakesEffectLive() {
        TeXpandPreferences.override = TeXpand.Settings()
        start("")
        type(";sec")
        XCTAssertFalse(tv.texpand.isCapturing)
        var on = TeXpand.Settings()
        on.enabled = true
        on.leader = ","
        TeXpandPreferences.override = on
        start("")
        type(",sec{A}")
        tab()
        XCTAssertEqual(tv.string, "\\section{A}")
    }
}
