import AppKit
import FlashTeXCollabCore
import FlashTeXCollabSession
import HostedWindows
import Network
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// Live Share's editor integration (proposal §2.4, §7.3): two real
/// `SourceEditorView`s in hosted windows, each bound to one side of a
/// session over loopback TLS 1.3 (a hub and a guest in this process),
/// driven by real key events and input-method calls.
@MainActor
final class LiveShareEditorTests: XCTestCase {
    private var windows: [NSWindow] = []
    private var hub: CollabHub?
    private var guest: CollabGuest?

    override func tearDown() async throws {
        guest?.leave()
        hub?.stop()
        for w in windows { w.orderOut(nil) }
        windows = []
    }

    struct Host: View {
        var model: ShellModel
        var link: LiveShareFileLink
        var body: some View {
            SourceEditorView(text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                             selection: model.selection, pendingEdit: model.pendingEdit,
                             onCaretChange: { model.caretUTF16 = $0 },
                             onSelectionChange: { model.caretLengthUTF16 = $0.length },
                             onEditApplied: { model.editApplied($0, newText: $1) },
                             autoClosePairs: ["{"],
                             liveShare: link)
        }
    }

    struct Side {
        var model: ShellModel
        var tv: CompletingTextView
        var co: SourceEditorView.Coordinator
        var link: LiveShareFileLink
        var text: String { tv.string }
    }

    /// A hub sharing `text` as main.tex and one guest; an editor for each.
    private func pair(_ text: String) async throws -> (hub: Side, guest: Side, file: FileID) {
        let hubSession = CollabSession(role: .hub, replica: UInt64.random(in: 1...UInt64.max), name: "Alice", colourIndex: 0)
        let file = try hubSession.shareFile(path: "main.tex", text: text)
        hubSession.flush()
        let pins = CollabControl.SessionPins(main: "main.tex", sourceDateEpoch: 0, randomSeed: 0, shellEscape: "off",
                                             externalTools: false, readConfinement: true)
        let hub = CollabHub(session: hubSession, identity: try CollabIdentity(), projectName: "T", pins: pins, environmentDigest: "t")
        hub.approve = { _, reply in reply(.allowEdit) }
        self.hub = hub
        try hub.start(loopbackOnly: true, advertise: false)
        try await waitUntil("hub ready") { hub.port != nil }
        let g = CollabGuest(invite: hub.makeInvite(addresses: [], hostName: nil), displayName: "Bob")
        g.endpointsOverride = [.hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: hub.port!)!)]
        guest = g
        g.connect()
        try await waitUntil("joined and synced") { g.session?.text(of: file) == text }
        let guestSession = try XCTUnwrap(g.session)
        let a = try await editor(text, LiveShareFileLink(binding: try XCTUnwrap(hubSession.binding(for: file)), session: hubSession))
        let b = try await editor(text, LiveShareFileLink(binding: try XCTUnwrap(guestSession.binding(for: file)), session: guestSession))
        return (a, b, file)
    }

    private func editor(_ text: String, _ link: LiveShareFileLink) async throws -> Side {
        let model = ShellModel()
        model.updateActiveText(text)
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model, link: link))
        window.orderFrontRegardless()
        windows.append(window)
        var found: NSTextView?
        try await waitUntil("editor text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        let tv = try XCTUnwrap(found as? CompletingTextView)
        let co = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        try await waitUntil("bound") { co.liveShare.link === link }
        XCTAssertFalse(tv.allowsUndo, "the text view's own undo is off in a session")
        return Side(model: model, tv: tv, co: co, link: link)
    }

    private func turn(_ ms: UInt64 = 10) async throws { try await Task.sleep(nanoseconds: ms * 1_000_000) }

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, file: StaticString = #filePath, line: UInt = #line,
                           _ cond: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !cond() {
            if Date() > deadline { XCTFail("timed out: \(what)", file: file, line: line); return }
            try await Task.sleep(nanoseconds: 5_000_000)
        }
    }

    private func key(_ chars: String, _ tv: NSTextView, keyCode: UInt16 = 0) {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                 windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: chars,
                                 charactersIgnoringModifiers: chars, isARepeat: false, keyCode: keyCode)!
        tv.window?.makeFirstResponder(tv)
        tv.keyDown(with: e)
    }

    private func type(_ s: String, _ side: Side) async throws {
        for ch in s {
            key(String(ch), side.tv)
            try await turn(2)
        }
    }

    private func backspace(_ side: Side) { key("\u{7F}", side.tv, keyCode: 51) }

    private func converged(_ a: Side, _ b: Side, _ file: FileID) -> Bool {
        a.text == b.text && a.link.session.text(of: file) == a.text && b.link.session.text(of: file) == b.text
            && a.model.activeText == a.text && b.model.activeText == b.text
    }

    // MARK: Tests

    /// The P1 integration test: both editors type at once with real key
    /// events (auto-close included); both buffers, both CRDTs and both
    /// models end identical, and no remote change ever reset the buffer.
    func testTwoEditorsCoEditWithRealKeystrokesAndConverge() async throws {
        let (a, b, file) = try await pair("\\section{Intro}\n\nEnd.\n")
        let resetsA = a.co.textResets, resetsB = b.co.textResets
        a.tv.setSelectedRange(NSRange(location: 16, length: 0))
        b.tv.setSelectedRange(NSRange(location: (b.text as NSString).length, length: 0))
        for i in 0..<12 {
            key(i % 4 == 0 ? "{" : "a", a.tv)
            key(i % 3 == 0 ? "é" : "b", b.tv)
            if i % 5 == 4 { backspace(b) }
            try await turn(3)
        }
        try await type("x\\emph{y", a)
        try await waitUntil("convergence") { self.converged(a, b, file) }
        XCTAssertTrue(a.text.contains("x\\emph{y}"), "auto-close reached the CRDT: \(a.text)")
        XCTAssertEqual(a.co.textResets, resetsA, "a remote change never resets the buffer")
        XCTAssertEqual(b.co.textResets, resetsB)
        XCTAssertGreaterThan(a.co.liveShare.remoteApplies, 0)
        XCTAssertGreaterThan(b.co.liveShare.remoteApplies, 0)
    }

    /// Remote insertions before the caret move it by their length; the
    /// selection keeps exactly the text it had.
    func testCaretAndSelectionStayOnTheirTextUnderRemoteEdits() async throws {
        let (a, b, file) = try await pair("one two three\n")
        b.tv.setSelectedRange(NSRange(location: 4, length: 3)) // "two"
        a.tv.setSelectedRange(NSRange(location: 0, length: 0))
        try await type("zero ", a)
        a.tv.setSelectedRange(NSRange(location: (a.text as NSString).length, length: 0))
        try await type("four", a)
        try await waitUntil("delivery") { self.converged(a, b, file) && b.text.hasSuffix("four") }
        XCTAssertEqual((b.text as NSString).substring(with: b.tv.selectedRange()), "two")
        XCTAssertEqual(b.tv.selectedRange(), NSRange(location: 9, length: 3))
    }

    /// The IME rule (proposal §2.4): composition steps never reach peers,
    /// and remote operations wait while this editor has marked text, then
    /// land as soon as it commits.
    func testRemoteEditsWaitForAnInputMethodComposition() async throws {
        let (a, b, file) = try await pair("abc\n")
        b.tv.window?.makeFirstResponder(b.tv)
        b.tv.setSelectedRange(NSRange(location: 0, length: 0))
        b.tv.setMarkedText("に", selectedRange: NSRange(location: 1, length: 0), replacementRange: NSRange(location: 0, length: 0))
        XCTAssertTrue(b.tv.hasMarkedText())
        a.tv.setSelectedRange(NSRange(location: 3, length: 0))
        try await type("XY", a)
        try await waitUntil("held at the composing editor") { b.link.binding.heldCount > 0 }
        try await turn(50)
        XCTAssertEqual(b.text, "にabc\n", "the composing buffer is left alone")
        XCTAssertFalse(a.link.session.text(of: file)!.contains("に"), "a composition step never leaves this Mac")
        b.tv.insertText("日", replacementRange: b.tv.markedRange()) // commit
        XCTAssertFalse(b.tv.hasMarkedText())
        try await waitUntil("convergence") { self.converged(a, b, file) }
        XCTAssertEqual(a.text, "日abcXY\n")
        XCTAssertEqual(b.link.binding.heldCount, 0)
    }

    /// ⌘Z undoes only this participant's typing, even with the other's text
    /// inside it; redo puts it back; both sides converge each time.
    func testUndoRevertsOnlyOwnTyping() async throws {
        let (a, b, file) = try await pair("")
        try await type("hello world", a)
        try await waitUntil("delivery") { b.text == "hello world" }
        b.tv.setSelectedRange(NSRange(location: 5, length: 0))
        try await type(" there", b)
        try await waitUntil("delivery") { a.text == "hello there world" }
        let undo = a.link.undoManager
        XCTAssertNil(a.tv.undoManager, "the text view registers no undo of its own in a session")
        XCTAssertTrue(undo.canUndo)
        XCTAssertTrue(a.tv.tryToPerform(Selector(("undo:")), with: nil), "Edit ▸ Undo (⌘Z) is handled by the editor")
        XCTAssertEqual(a.text, " there", "only Alice's characters went")
        try await waitUntil("undo converges") { self.converged(a, b, file) }
        XCTAssertTrue(undo.canRedo)
        XCTAssertTrue(a.tv.tryToPerform(Selector(("redo:")), with: nil))
        XCTAssertEqual(a.text, "hello there world")
        try await waitUntil("redo converges") { self.converged(a, b, file) }
        // Bob's own undo removes only " there".
        b.link.undoManager.undo()
        try await waitUntil("bob's undo converges") { self.converged(a, b, file) && a.text == "hello world" }
    }

    /// A remote edit above the viewport keeps the text the user is looking
    /// at where it was on screen.
    func testRemoteEditAboveTheViewportKeepsItInPlace() async throws {
        let body = (1...200).map { "line \($0)\n" }.joined()
        let (a, b, file) = try await pair(body)
        b.tv.layoutManager?.ensureLayout(for: b.tv.textContainer!)
        let target = (b.text as NSString).range(of: "line 120\n")
        b.tv.scrollRangeToVisible(target)
        b.tv.setSelectedRange(NSRange(location: target.location, length: 0))
        try await turn(20)
        let before = b.tv.firstRect(forCharacterRange: NSRange(location: target.location, length: 1), actualRange: nil)
        a.tv.setSelectedRange(NSRange(location: 0, length: 0))
        try await type("new\nnew\nnew\n", a)
        try await waitUntil("delivery") { self.converged(a, b, file) }
        try await turn(20)
        let moved = (b.text as NSString).range(of: "line 120\n")
        XCTAssertEqual(b.tv.selectedRange().location, moved.location, "the caret stayed on its line")
        let after = b.tv.firstRect(forCharacterRange: NSRange(location: moved.location, length: 1), actualRange: nil)
        XCTAssertEqual(after.minY, before.minY, accuracy: 1, "the line did not jump on screen")
    }

    /// Presence (proposal §4): Bob's selection appears in Alice's editor as a
    /// coloured caret and tint, drawn by the overlay over the visible text,
    /// and follows the text as Alice types before it.
    func testTheOtherCaretIsDrawnAndFollowsTheText() async throws {
        let (a, b, file) = try await pair("alpha beta gamma\n")
        b.tv.setSelectedRange(NSRange(location: 6, length: 4)) // "beta"
        try await waitUntil("presence") { a.link.session.remoteCursors(in: file).first?.range == 6..<10 }
        let overlay = try XCTUnwrap(a.co.liveShare.overlay)
        XCTAssertTrue(overlay.superview === a.tv)
        overlay.display()
        XCTAssertEqual(overlay.drawnCursors.map(\.name), ["Bob"])
        XCTAssertEqual(overlay.drawnCursors.first?.range, 6..<10)
        a.tv.setSelectedRange(NSRange(location: 0, length: 0))
        try await type(">> ", a)
        overlay.display()
        XCTAssertEqual(overlay.drawnCursors.first?.range, 9..<13, "the caret stays on Bob's text")
        XCTAssertNil(overlay.hitTest(NSPoint(x: 10, y: 10)), "clicks go to the text")
    }

    /// A remote change that does not fit the buffer (the view diverged from
    /// the CRDT, which should never happen) resynchronises the buffer from
    /// the CRDT instead of being skipped.
    func testAnOutOfRangeRemoteChangeResynchronisesTheBuffer() async throws {
        let (a, b, file) = try await pair("0123456789\n")
        b.co.liveShare.suspended += 1 // behind the session's back
        b.tv.textStorage!.replaceCharacters(in: NSRange(location: 5, length: 6), with: "")
        b.co.liveShare.suspended -= 1
        a.tv.setSelectedRange(NSRange(location: 11, length: 0))
        try await type("END", a)
        try await waitUntil("resync") { b.co.liveShare.resyncs > 0 }
        try await waitUntil("convergence") { self.converged(a, b, file) }
        XCTAssertEqual(b.text, "0123456789\nEND")
    }
}

