import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// The editor's text kept in step edit by edit (EditorTextShadow.swift,
/// APP-EDITOR-INSTANT): a keystroke hands the model a spliced String, never a
/// transcode of the whole buffer, and that String is always the storage's.
@MainActor
final class EditorTextShadowTests: XCTestCase {
    override func tearDown() async throws {
        SourceEditorView.Coordinator.verifyShadow = false
    }

    /// Random edits (multi-byte, surrogate pairs, combining marks, deletes
    /// across them, whole replacements) against an NSTextStorage: after each,
    /// the shadow is the storage's text byte for byte, or it gave up.
    func testTheShadowFollowsRandomEditsExactly() {
        var rng = SystemRandomNumberGenerator()
        let pieces = ["a", "b", " ", "\n", "\\frac", "{", "}", "é", "e\u{301}", "€", "𝔸", "😀", "👩‍👩‍👧", "中文", "\t"]
        func text(_ k: Int) -> String { (0 ..< k).map { _ in pieces[Int.random(in: 0 ..< pieces.count, using: &rng)] }.joined() }
        for round in 0 ..< 40 {
            // (An NSTextStorage's editedRange is reset once its edit is processed: the edit is computed here.)
            let storage = NSMutableString(string: text(Int.random(in: 0 ..< 200, using: &rng)))
            var shadow: EditorTextShadow? = EditorTextShadow(text: storage as String, length16: storage.length)
            var gaveUp = 0
            for _ in 0 ..< 200 {
                let ns = storage as NSString
                var at = Int.random(in: 0 ... ns.length, using: &rng)
                if at < ns.length { at = ns.rangeOfComposedCharacterSequence(at: at).location }
                var end = min(ns.length, at + Int.random(in: 0 ..< 6, using: &rng))
                if end < ns.length, end > at { end = NSMaxRange(ns.rangeOfComposedCharacterSequence(at: end)) }
                let range = NSRange(location: at, length: max(0, end - at))
                let insert = Int.random(in: 0 ..< 10, using: &rng) == 0 ? text(40) : text(Int.random(in: 0 ..< 3, using: &rng))
                storage.replaceCharacters(in: range, with: insert)
                let insertLength = (insert as NSString).length
                if var s = shadow {
                    shadow = s.apply(storage: storage, edited: NSRange(location: at, length: insertLength), delta: insertLength - range.length) ? s : nil
                }
                if shadow == nil { gaveUp += 1; shadow = EditorTextShadow(text: storage as String, length16: storage.length) }
                XCTAssertTrue(shadow!.text.sameBytes(as: storage as String), "round \(round)")
                XCTAssertEqual(shadow!.bytes, (storage as String).utf8.count)
                XCTAssertEqual(shadow!.length16, storage.length)
            }
            XCTAssertEqual(gaveUp, 0, "every one of these edits is one it can follow")
        }
    }

    /// An edit it was not told about (the storage changed behind it): the
    /// length check refuses the next one.
    func testAMissedEditIsRefused() {
        let storage = NSMutableString(string: "hello world")
        var s = EditorTextShadow(text: storage as String, length16: storage.length)
        storage.replaceCharacters(in: NSRange(location: 0, length: 5), with: "bye") // not applied to the shadow
        storage.replaceCharacters(in: NSRange(location: 0, length: 0), with: "x")
        XCTAssertFalse(s.apply(storage: storage, edited: NSRange(location: 0, length: 1), delta: 1))
    }

    /// Typing, deleting, a paste and undo in the hosted editor: the model's
    /// text is the view's after each, every keystroke's from the shadow, and
    /// it never drifted (checked against a transcode on every use).
    func testTypingInTheEditorHandsTheModelTheShadowText() async throws {
        SourceEditorView.Coordinator.verifyShadow = true
        let model = ShellModel()
        model.replaceProject(entryText: "\\documentclass{article}\n\\begin{document}\nCafé 😀 naïve text here.\n\\end{document}\n")
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 1000, height: 700), styleMask: [.titled])
        window.contentView = NSHostingView(rootView: ContentView().environment(model).environmentObject(NearbyState()))
        window.orderFrontRegardless()
        defer { window.orderOut(nil); window.contentView = nil }
        var found: NSTextView?
        for _ in 0 ..< 300 {
            found = TypingBenchDriver.findTextView(in: [window.contentView!])
            if found != nil { break }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        let tv = try XCTUnwrap(found)
        let co = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        let at = (tv.string as NSString).range(of: "naïve").location
        tv.setSelectedRange(NSRange(location: at, length: 0))
        let hits = co.shadowHits
        for ch in ["x", "é", "😀", "y"] {
            tv.insertText(ch, replacementRange: tv.selectedRange())
            XCTAssertTrue(model.activeText.sameBytes(as: tv.string))
        }
        tv.deleteBackward(nil); tv.deleteBackward(nil)
        XCTAssertTrue(model.activeText.sameBytes(as: tv.string))
        tv.insertText("pasted\nlines {with} 中文\n", replacementRange: NSRange(location: 0, length: 0))
        XCTAssertTrue(model.activeText.sameBytes(as: tv.string))
        tv.undoManager?.undo()
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertTrue(model.activeText.sameBytes(as: tv.string), "after undo")
        XCTAssertGreaterThanOrEqual(co.shadowHits - hits, 6, "keystrokes took the shadow's text")
        XCTAssertEqual(co.shadowDrifts, 0)
    }
}
