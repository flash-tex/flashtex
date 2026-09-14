import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// GH#280: a document opened into a fresh editor must be coloured and must
/// open the completion list on typing without first switching documents.
/// Hosted like `SourceEditorViewTests` (never key: tests must not steal focus).
@MainActor
final class EditorFirstFocusTests: XCTestCase {
    private struct Host: View {
        var model: ShellModel
        var body: some View {
            SourceEditorView(
                text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                selection: model.selection,
                pendingEdit: model.pendingEdit,
                marks: model.editorMarks,
                result: model.result,
                editorRevision: model.editorRevision,
                projectIndexMetadata: model.completionMetadata,
                projectFiles: model.documents.map(\.path),
                onCaretChange: { model.caretUTF16 = $0 },
                onEditApplied: { model.editApplied($0, newText: $1) },
                onEditRefused: { model.editRefused($0, reason: $1) },
                syntaxHighlighting: true
            )
        }
    }

    private var window: NSWindow?

    override func tearDown() async throws {
        window?.orderOut(nil)
        window = nil
    }

    private func host(_ model: ShellModel) async throws -> CompletingTextView {
        HostedWindowSupport.prepare()
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                              backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model))
        window.orderFrontRegardless()
        self.window = window
        var found: NSTextView?
        try await waitUntil("editor text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        return try XCTUnwrap(found as? CompletingTextView)
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    private func turn() async throws { try await Task.sleep(nanoseconds: 50_000_000) }

    /// Number of distinct temporary foreground-colour runs over the text.
    private func colourRuns(_ tv: NSTextView) -> Int {
        guard let lm = tv.layoutManager else { return -1 }
        let length = (tv.string as NSString).length
        var runs = 0, index = 0
        while index < length {
            var effective = NSRange()
            if lm.temporaryAttribute(SyntaxPainter.key, atCharacterIndex: index, effectiveRange: &effective) != nil { runs += 1 }
            index = max(NSMaxRange(effective), index + 1)
        }
        return runs
    }

    private func type(_ tv: CompletingTextView, _ chars: String) {
        for ch in chars {
            let s = String(ch)
            let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [],
                                     timestamp: ProcessInfo.processInfo.systemUptime,
                                     windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: s,
                                     charactersIgnoringModifiers: s, isARepeat: false, keyCode: 0)!
            tv.keyDown(with: e)
        }
        tv.flushAutomaticCompletion()
    }

    private static let document = "\\documentclass{article}\n\\begin{document}\n\\section{Intro} Some $x^2$ text.\n\n\\end{document}\n"

    private func assertColouredAndCompletes(_ tv: CompletingTextView, _ label: String) async throws {
        try await turn()
        XCTAssertGreaterThan(colourRuns(tv), 0, "\(label): the opened document is coloured")
        XCTAssertTrue(window!.makeFirstResponder(tv))
        let end = (tv.string as NSString).range(of: "\n\n\\end").location + 1
        tv.setSelectedRange(NSRange(location: end, length: 0))
        let before = colourRuns(tv)
        type(tv, "\\se")
        XCTAssertEqual(tv.automaticOpenCount, 1, "\(label): typing \\se requests the list")
        try await waitUntil("\(label): the completion list is showing", timeout: 3) { tv.isCompletionActive }
        XCTAssertGreaterThan(colourRuns(tv), before, "\(label): the typed command is coloured")
        tv.close(.escape)
    }

    /// The whole window, as `FlashTeXMacApp` builds it.
    private func hostContentView(_ model: ShellModel) async throws -> CompletingTextView {
        HostedWindowSupport.prepare()
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1200, height: 640), styleMask: [.titled],
                              backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: ContentView().environment(model).environmentObject(NearbyState()))
        window.orderFrontRegardless()
        self.window = window
        var found: NSTextView?
        try await waitUntil("editor text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        return try XCTUnwrap(found as? CompletingTextView)
    }

    func testContentViewOpenedDocumentIsColouredAndCompletes() async throws {
        let model = ShellModel()
        let tv = try await hostContentView(model)
        try await turn()
        model.replaceProject(entryText: Self.document)
        try await waitUntil("opened text reaches the view") { tv.string == Self.document }
        try await assertColouredAndCompletes(tv, "ContentView, opened after mount")
    }

    /// GH#280 repro context: the startup document `ShellModel` loads itself
    /// (no File > Open), in the whole window that was never key at launch.
    func testStartupDocumentInContentViewIsColouredAndCompletes() async throws {
        let model = ShellModel()
        let tv = try await hostContentView(model)
        try await turn()
        let length = (tv.string as NSString).length
        XCTAssertGreaterThan(length, 0, "the startup document is loaded")
        // The fixture-backed startup text ("Hello FlashTeX.") holds no LaTeX
        // token, so there is nothing to colour until the user types one.
        XCTAssertTrue(window!.makeFirstResponder(tv))
        let caret = min(length, 200)
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        let before = colourRuns(tv)
        type(tv, " \\se")
        XCTAssertEqual(tv.automaticOpenCount, 1, "startup document: typing \\se requests the list")
        try await waitUntil("startup document: the completion list is showing", timeout: 3) { tv.isCompletionActive }
        try await turn()
        XCTAssertGreaterThan(colourRuns(tv), before, "startup document: the typed command is coloured")
        tv.close(.escape)
    }

    /// The painted-range arithmetic behind GH#280: edits that touch a painted
    /// range join it; edits clear of it only move it.
    func testPaintedRangeAbsorbsEditsAtItsEdges() {
        let r = NSRange(location: 10, length: 6)
        func shifted(_ at: Int, _ length: Int, _ replacement: Int) -> NSRange {
            SyntaxPainter.shifted(r, edit: NSRange(location: at, length: length), replacementLength: replacement)
        }
        XCTAssertEqual(shifted(16, 0, 4), NSRange(location: 10, length: 10), "typing at the end joins the range")
        XCTAssertEqual(shifted(10, 0, 4), NSRange(location: 10, length: 10), "typing at the start joins the range")
        XCTAssertEqual(shifted(12, 0, 3), NSRange(location: 10, length: 9), "typing inside grows the range")
        XCTAssertEqual(shifted(20, 0, 3), r, "an edit after the range leaves it")
        XCTAssertEqual(shifted(2, 0, 3), NSRange(location: 13, length: 6), "an edit before the range moves it")
        XCTAssertEqual(shifted(16, 2, 0), r, "deleting just after the end leaves it")
        XCTAssertEqual(shifted(8, 2, 0), NSRange(location: 8, length: 6), "deleting just before the start moves it")
        XCTAssertEqual(shifted(14, 2, 0), NSRange(location: 10, length: 4), "deleting inside shrinks it")
    }

    /// Local-only (it takes keyboard focus): the owner's launch, a window that
    /// was only ordered back, then a real click and real key events through
    /// the window's event dispatch, as a person produces them.
    func testFocusProbeClickThenTypeInABackgroundWindow() async throws {
        try XCTSkipUnless(ProcessInfo.processInfo.environment["FLASHTEX_FOCUS_PROBE"] == "1", "set FLASHTEX_FOCUS_PROBE=1 (takes focus)")
        let model = ShellModel()
        let tv = try await hostContentView(model)
        let window = try XCTUnwrap(self.window)
        window.orderBack(nil)
        try await turn()
        func send(_ type: NSEvent.EventType, at point: NSPoint) {
            window.sendEvent(NSEvent.mouseEvent(with: type, location: point, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                                windowNumber: window.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!)
        }
        let length = (tv.string as NSString).length
        let glyphs = tv.layoutManager!.glyphRange(forCharacterRange: NSRange(location: max(0, length - 1), length: 1), actualCharacterRange: nil)
        var rect = tv.layoutManager!.boundingRect(forGlyphRange: glyphs, in: tv.textContainer!)
        rect.origin.x += tv.textContainerOrigin.x + rect.width + 2; rect.origin.y += tv.textContainerOrigin.y + rect.height / 2
        let point = tv.convert(rect.origin, to: nil)
        send(.leftMouseDown, at: point); send(.leftMouseUp, at: point)
        try await turn()
        XCTAssertTrue(window.firstResponder === tv, "the click gives the editor first responder")
        for ch in " \\se" {
            let s = String(ch)
            window.sendEvent(NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                              windowNumber: window.windowNumber, context: nil, characters: s,
                                              charactersIgnoringModifiers: s, isARepeat: false, keyCode: 0)!)
        }
        try await Task.sleep(nanoseconds: 400_000_000)
        let popupVisible = window.childWindows?.contains { $0 is CompletionPopup && $0.isVisible } ?? false
        XCTAssertTrue(tv.string.hasSuffix(" \\se"), "the keys reached the editor")
        XCTAssertTrue(tv.isCompletionActive, "typing \\se opens a session")
        XCTAssertTrue(popupVisible, "the completion panel is on screen")
        XCTAssertGreaterThan(colourRuns(tv), 0, "the typed command is coloured")
        tv.close(.escape)
    }

    func testDocumentPresentBeforeTheEditorMountsIsColouredAndCompletes() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: Self.document)
        let tv = try await host(model)
        try await assertColouredAndCompletes(tv, "present before mount")
    }

    func testDocumentOpenedAfterTheEditorMountsIsColouredAndCompletes() async throws {
        let model = ShellModel()
        let tv = try await host(model)
        try await turn()
        model.replaceProject(entryText: Self.document)
        try await waitUntil("opened text reaches the view") { tv.string == Self.document }
        try await assertColouredAndCompletes(tv, "opened after mount")
    }
}
