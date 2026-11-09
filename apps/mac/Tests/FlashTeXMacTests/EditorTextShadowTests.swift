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

    // MARK: no whole-buffer copy per keystroke (APP-KEYPATH)

    /// Typing, backspaces and a paste in a 3 MB non-ASCII book while the
    /// last two texts are still held elsewhere (the model and the compiled
    /// baseline): after the first few keys no keystroke allocates or copies
    /// a buffer, each text is a retired one brought up to date in place,
    /// every text is the storage's byte for byte, and texts held elsewhere
    /// never change.
    func testKeystrokesReuseARetiredBufferInsteadOfCopyingTheText() {
        var s = ""
        let line = "Naïve café patrons — résumé in hand — watched the tide turn; 😀 $x^2$ \\emph{ö}.\n"
        while s.utf8.count < 3_000_000 { s += line }
        let storage = NSMutableString(string: s)
        var shadow = EditorTextShadow(text: s, length16: storage.length)
        var held: [(text: String, bytes: [UInt8])] = []
        EditorTextShadow.Spares.resetCounters()
        var caret = storage.length / 2
        var keys = 0, copiedAfterWarmup = 0
        let warmup = EditorTextShadow.Spares.capacity + 1
        func edit(_ range: NSRange, _ insert: String) {
            storage.replaceCharacters(in: range, with: insert)
            let n = (insert as NSString).length
            let before = EditorTextShadow.Spares.copiedBytes
            XCTAssertTrue(shadow.apply(storage: storage, edited: NSRange(location: range.location, length: n), delta: n - range.length))
            keys += 1
            if keys > warmup { copiedAfterWarmup += EditorTextShadow.Spares.copiedBytes - before }
            caret = range.location + n
            XCTAssertEqual(shadow.bytes, (storage as String).utf8.count)
            if keys % 10 == 0 { XCTAssertTrue(shadow.text.sameBytes(as: storage as String), "key \(keys)") }
            held.append((shadow.text, keys % 10 == 0 ? Array(shadow.text.utf8) : []))
            if held.count > 2 { held.removeFirst() }
        }
        for ch in "typing a sentence — naïvely 😀 " { edit(NSRange(location: caret, length: 0), String(ch)) }
        for _ in 0 ..< 5 { edit((storage as NSString).rangeOfComposedCharacterSequence(at: caret - 1), "") }
        edit(NSRange(location: 1000, length: 0), "a paste\nof two lines é\n")
        for ch in "more" { edit(NSRange(location: caret, length: 0), String(ch)) }
        XCTAssertTrue(shadow.text.sameBytes(as: storage as String))
        for h in held where !h.bytes.isEmpty { XCTAssertEqual(Array(h.text.utf8), h.bytes, "a text held elsewhere changed") }
        XCTAssertEqual(copiedAfterWarmup, 0, "no keystroke after the first \(warmup) copied the 3 MB text")
        XCTAssertGreaterThanOrEqual(EditorTextShadow.Spares.reusedInPlace, keys - warmup)
        print("EditorTextShadow: \(keys) keys at \(shadow.bytes) bytes: copied \(EditorTextShadow.Spares.copiedBytes) B, moved in place \(EditorTextShadow.Spares.movedBytes) B, reused \(EditorTextShadow.Spares.reusedInPlace)")
    }

    /// The same in the hosted editor (real NSTextView, real ShellModel): a
    /// 1 MB book, 30 keystrokes; the model's text is the view's after each,
    /// and the shadow copied the whole text for at most a few of them (the
    /// first keys, and any key whose oldest kept text a background scan still
    /// reads).
    func testTypingInTheHostedEditorCopiesTheTextForFewKeys() async throws {
        SourceEditorView.Coordinator.verifyShadow = true
        var s = "\\documentclass{article}\n\\begin{document}\n"
        while s.utf8.count < 1_000_000 { s += "Naïve café patrons — résumé in hand — watched the tide turn.\n\n" }
        s += "\\end{document}\n"
        let model = ShellModel()
        model.replaceProject(entryText: s)
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
        tv.setSelectedRange(NSRange(location: (tv.string as NSString).length / 2, length: 0))
        EditorTextShadow.Spares.resetCounters()
        var copiedKeys = 0
        for ch in "the quick brown fox jumps over" {
            let before = EditorTextShadow.Spares.copiedBytes
            tv.insertText(String(ch), replacementRange: tv.selectedRange())
            if EditorTextShadow.Spares.copiedBytes > before { copiedKeys += 1 }
            XCTAssertTrue(model.activeText.sameBytes(as: tv.string))
            try await Task.sleep(nanoseconds: 5_000_000) // the run loop turns between keys, as typing does
        }
        XCTAssertEqual(co.shadowDrifts, 0)
        XCTAssertLessThanOrEqual(copiedKeys, EditorTextShadow.Spares.capacity + 8, "most keystrokes reused a buffer in place")
        print("EditorTextShadow hosted: 30 keys at \(model.activeText.utf8.count) bytes: \(copiedKeys) copied the text, reused \(EditorTextShadow.Spares.reusedInPlace)")
    }
}
