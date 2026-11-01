import AppKit
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// CJK input-method composition against the REAL `flashtex-preview-controller`
/// and `flashtex-compiler` (skipped unless `FLASHTEX_PREVIEW_CONTROLLER` and
/// `FLASHTEX_COMPILER` are set): marked text is never a durable edit, a
/// background preview landing mid-composition disturbs nothing, a helper
/// killed by pid mid-composition leaves the composition intact and the commit
/// becomes durable exactly once after an explicit reattach, and one undo step
/// removes the whole committed composition while `history_status` shows one
/// entry for it. Byte offsets are checked on multi-codepoint text (a
/// surrogate-pair CJK ideograph, Hangul syllables).
///
/// See `IMEHarness` for how the composition is driven (NSTextInputClient calls
/// on the real editor, never key events, never focus).
@MainActor
final class IMECompositionTests: XCTestCase {
    static let original = "\\begin{document}\nHello world.\n\\end{document}\n"
    /// UTF-16 offset right after "Hello " (all ASCII up to there).
    static let insertAt = ("\\begin{document}\nHello " as NSString).length

    private func caretAtInsertionPoint(_ h: IMEHarness) async throws {
        h.textView.setSelectedRange(NSRange(location: Self.insertAt, length: 0))
        try await h.turn()
        XCTAssertEqual(h.model.caretUTF16, Self.insertAt)
        XCTAssertEqual(h.model.caretByte, Self.insertAt, "ASCII prefix: bytes equal UTF-16 units")
    }

    private func expected(inserting s: String) -> String {
        let ns = Self.original as NSString
        return ns.replacingCharacters(in: NSRange(location: Self.insertAt, length: 0), with: s)
    }

    // MARK: (a) + (d) marked text is never durable; the commit is one durable edit, one undo step

    func testMarkedTextIsNeverDurableAndTheCommitIsOneDurableEditAndOneUndoStep() async throws {
        print("IME test uptime: \(IMEHarness.uptime())")
        let h = try await IMEHarness.attached("durable", text: Self.original)
        defer { h.close() }
        let model = h.model
        try await caretAtInsertionPoint(h)
        let rev = model.editorRevision
        let r1 = try await h.helperDocument()
        XCTAssertEqual(r1.revision, 1)
        XCTAssertEqual(r1.text, Self.original)
        XCTAssertEqual(r1.sha256, SourceDigest.sha256Hex(Self.original))
        let disk0 = await h.fileState()
        XCTAssertEqual(disk0, "matches_source")
        h.textView.undoManager?.removeAllActions()

        // Japanese: kana steps, then the kanji candidate, all marked. The helper's
        // durable document, revision, hash and the disk state never move.
        for (i, step) in ["に", "にほ", "にほん", "にほんご", "日本語"].enumerated() {
            h.compose(step)
            XCTAssertTrue(h.hasMarkedText, "step \(i)")
            XCTAssertEqual(h.string, expected(inserting: step), "the view shows the marked text (step \(i))")
            XCTAssertEqual(h.markedRange, NSRange(location: Self.insertAt, length: (step as NSString).length))
            XCTAssertEqual(h.selectedRange, NSRange(location: Self.insertAt + (step as NSString).length, length: 0))
            XCTAssertEqual(model.activeText, Self.original, "step \(i) never reached the model")
            XCTAssertEqual(model.editorRevision, rev, "no editor revision for step \(i)")
            XCTAssertNil(model.controllerState.inFlight, "no edit in flight for step \(i)")
            XCTAssertEqual(model.caretUTF16, Self.insertAt, "the caret the model hears is the composition start")
            let doc = try await h.helperDocument()
            XCTAssertEqual(doc, r1, "the helper's durable document is unchanged by step \(i)")
            XCTAssertTrue(h.hasMarkedText, "the helper round trip did not end the composition (step \(i))")
        }
        try await h.turn()
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 1)
        let diskMid = await h.fileState()
        XCTAssertEqual(diskMid, "matches_source", "disk still matches the pre-composition source")
        let (undoMid, _) = try await h.historyLabels()
        XCTAssertEqual(undoMid, [], "nothing in the ledger's history before the commit")

        // Commit: one editor revision, one durable edit (r2) with exactly the
        // committed bytes, one ledger entry; the caret is byte-exact.
        h.commit("日本語")
        XCTAssertFalse(h.hasMarkedText)
        let committed = expected(inserting: "日本語")
        XCTAssertEqual(h.string, committed)
        XCTAssertEqual(model.activeText, committed)
        XCTAssertEqual(model.editorRevision, rev + 1, "the whole composition is one revision")
        XCTAssertEqual(model.caretUTF16, Self.insertAt + 3)
        XCTAssertEqual(model.caretByte, Self.insertAt + "日本語".utf8.count, "3 ideographs = 9 UTF-8 bytes")
        try await h.waitUntil("durable r2 and its preview") {
            model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil
                && model.result?.revision == model.editorRevision
        }
        let r2 = try await h.helperDocument()
        XCTAssertEqual(r2.revision, 2)
        XCTAssertEqual(r2.text, committed)
        XCTAssertTrue(r2.text.sameBytes(as: committed), "durable bytes are the committed bytes")
        XCTAssertEqual(r2.sha256, SourceDigest.sha256Hex(committed), "the helper hashed the same UTF-8 bytes")
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.sha256, r2.sha256)
        XCTAssertEqual(model.compiledDocuments["main.tex"], committed, "the preview was compiled from the committed text")
        let diskAfter = await h.fileState()
        XCTAssertEqual(diskAfter, "differs_from_source", "the commit is durable in the ledger, not yet on disk")
        let (undoAfter, redoAfter) = try await h.historyLabels()
        XCTAssertEqual(undoAfter, ["Source edit"], "one ledger entry for the whole composition, not one per step")
        XCTAssertEqual(redoAfter, [])

        // Local undo (⌘Z): one step removes the whole composition — never
        // "にほんご" or a kana at a time — and the buffer is resubmitted once.
        let undo = try XCTUnwrap(h.textView.undoManager)
        XCTAssertTrue(undo.canUndo)
        undo.undo()
        XCTAssertEqual(h.string, Self.original, "one undo step removes exactly the committed composition")
        XCTAssertEqual(model.activeText, Self.original)
        XCTAssertFalse(h.hasMarkedText)
        XCTAssertEqual(model.editorRevision, rev + 2)
        try await h.waitUntil("durable r3 (the undone buffer)") {
            model.controllerState.durable["main.tex"]?.revision == 3 && model.controllerState.inFlight == nil
        }
        let r3 = try await h.helperDocument()
        XCTAssertEqual(r3.text, Self.original)
        let (undoTwice, _) = try await h.historyLabels()
        XCTAssertEqual(undoTwice, ["Source edit", "Source edit"], "the local undo is one more source edit in the ledger")
        let diskUndone = await h.fileState()
        XCTAssertEqual(diskUndone, "matches_source")
        XCTAssertFalse(undo.canUndo, "nothing else to undo: the composition steps were never undo steps")

        // Surrogate pair (𠮷 U+20BB7: 2 UTF-16 units, 4 UTF-8 bytes) through a
        // reading → candidate conversion; the durable hash and the caret bytes
        // follow the UTF-8 length, not the UTF-16 length.
        h.textView.setSelectedRange(NSRange(location: Self.insertAt, length: 0))
        try await h.turn()
        let rev2 = model.editorRevision
        for step in ["よし", "𠮷", "𠮷野家"] {
            h.compose(step)
            XCTAssertEqual(h.markedRange, NSRange(location: Self.insertAt, length: (step as NSString).length))
            XCTAssertEqual(model.activeText, Self.original)
            XCTAssertEqual(model.editorRevision, rev2)
        }
        XCTAssertEqual(h.markedRange.length, 4, "𠮷野家 is 4 UTF-16 units")
        h.commit("𠮷野家")
        let committed2 = expected(inserting: "𠮷野家")
        XCTAssertEqual(model.activeText, committed2)
        XCTAssertEqual(model.editorRevision, rev2 + 1)
        XCTAssertEqual(model.caretUTF16, Self.insertAt + 4)
        XCTAssertEqual(model.caretByte, Self.insertAt + 10, "𠮷 (4) + 野 (3) + 家 (3) UTF-8 bytes")
        XCTAssertEqual("𠮷野家".utf8.count, 10)
        try await h.waitUntil("durable r4") {
            model.controllerState.durable["main.tex"]?.revision == 4 && model.controllerState.inFlight == nil
        }
        let r4 = try await h.helperDocument()
        XCTAssertEqual(r4.revision, 4)
        XCTAssertTrue(r4.text.sameBytes(as: committed2))
        XCTAssertEqual(r4.sha256, SourceDigest.sha256Hex(committed2))
        let (undo4, _) = try await h.historyLabels()
        XCTAssertEqual(undo4.count, 3, "one ledger entry per commit")
        print("IME durable test: r1..r4 verified; \(IMEHarness.uptime())")
    }

    // MARK: (b) a background preview lands mid-composition

    func testPreviewArrivingMidCompositionLeavesTheMarkedRangeCaretAndCompositionIntact() async throws {
        print("IME test uptime: \(IMEHarness.uptime())")
        let h = try await IMEHarness.attached("preview", text: Self.original)
        defer { h.close() }
        let model = h.model
        try await caretAtInsertionPoint(h)

        // Plain typing (no input method): the edit goes out and stays in
        // flight until its preview arrives (hold-until-preview release).
        h.commit("x ")
        let typed = expected(inserting: "x ")
        XCTAssertEqual(model.activeText, typed)
        let typedRev = model.editorRevision
        XCTAssertNotNil(model.controllerState.inFlight, "the edit is in flight")
        XCTAssertEqual(model.inFlightRevision, typedRev)
        XCTAssertNotEqual(model.result?.revision, typedRev, "its preview has not arrived yet")

        // The composition starts while that compile is in the background.
        // Korean: jamo assemble into a syllable, then a second syllable.
        let start = Self.insertAt + 2
        h.textView.setSelectedRange(NSRange(location: start, length: 0))
        for step in ["ㅎ", "하", "한"] { h.compose(step) }
        XCTAssertTrue(h.hasMarkedText)
        let markedBefore = h.markedRange, selectedBefore = h.selectedRange, stringBefore = h.string
        XCTAssertEqual(markedBefore, NSRange(location: start, length: 1))
        XCTAssertEqual(selectedBefore, NSRange(location: start + 1, length: 0))
        XCTAssertEqual(stringBefore, expected(inserting: "x 한"))
        XCTAssertEqual(model.activeText, typed)

        // The preview for the typed text arrives (a SwiftUI update of the
        // hosted editor with a new `result`) while the view has marked text.
        try await h.waitUntil("preview for the typed revision") {
            model.result?.revision == typedRev && model.controllerState.inFlight == nil
        }
        try await h.turn()
        XCTAssertEqual(model.compiledDocuments["main.tex"], typed)
        XCTAssertTrue(h.hasMarkedText, "the preview did not end the composition")
        XCTAssertEqual(h.markedRange, markedBefore, "marked range untouched")
        XCTAssertEqual(h.selectedRange, selectedBefore, "caret untouched")
        XCTAssertEqual(h.string, stringBefore, "the view was not re-synced from the model over the composition")
        XCTAssertEqual(model.activeText, typed, "the marked text still did not reach the model")
        XCTAssertEqual(model.editorRevision, typedRev)
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 2)
        let doc = try await h.helperDocument()
        XCTAssertEqual(doc.revision, 2)
        XCTAssertEqual(doc.text, typed)
        try await h.holds("composition across the settled preview", for: 0.3) { h.hasMarkedText && h.markedRange == markedBefore }

        // The composition continues after the preview and commits once.
        for step in ["한ㄱ", "한구", "한국", "한국ㅇ", "한국어"] {
            h.compose(step)
            XCTAssertEqual(model.activeText, typed)
        }
        XCTAssertEqual(h.markedRange, NSRange(location: start, length: 3))
        h.commit("한국어")
        XCTAssertFalse(h.hasMarkedText)
        let committed = expected(inserting: "x 한국어")
        XCTAssertEqual(model.activeText, committed)
        XCTAssertEqual(model.editorRevision, typedRev + 1, "one revision for the whole composition")
        XCTAssertEqual(model.caretUTF16, start + 3)
        XCTAssertEqual(model.caretByte, start + 9, "3 precomposed Hangul syllables = 9 UTF-8 bytes")
        try await h.waitUntil("durable r3 and its preview") {
            model.controllerState.durable["main.tex"]?.revision == 3 && model.controllerState.inFlight == nil
                && model.result?.revision == model.editorRevision
        }
        let r3 = try await h.helperDocument()
        XCTAssertTrue(r3.text.sameBytes(as: committed))
        XCTAssertEqual(r3.sha256, SourceDigest.sha256Hex(committed))
        XCTAssertEqual(model.compiledDocuments["main.tex"], committed)
        let (undo, _) = try await h.historyLabels()
        XCTAssertEqual(undo, ["Source edit", "Source edit"], "typing, then the composition: two ledger entries")
        print("IME preview test: preview r2 landed under marked 한; commit r3; \(IMEHarness.uptime())")
    }

    // MARK: (c) the helper dies mid-composition

    func testHelperKilledMidCompositionKeepsTheCompositionAndTheCommitIsDurableOnceAfterReattach() async throws {
        print("IME test uptime: \(IMEHarness.uptime())")
        let h = try await IMEHarness.attached("restart", text: Self.original)
        defer { h.close() }
        let model = h.model
        try await caretAtInsertionPoint(h)
        let rev = model.editorRevision
        let firstPID = try XCTUnwrap(model.controller?.processIdentifier)

        // Chinese (pinyin): the reading is marked first, then the candidate.
        for step in ["n", "ni", "你", "你h", "你好"] { h.compose(step) }
        XCTAssertTrue(h.hasMarkedText)
        let markedBefore = h.markedRange, selectedBefore = h.selectedRange, stringBefore = h.string
        XCTAssertEqual(markedBefore, NSRange(location: Self.insertAt, length: 2))
        XCTAssertEqual(stringBefore, expected(inserting: "你好"))
        XCTAssertEqual(model.activeText, Self.original)

        // SIGKILL by the pid this harness launched; the shell observes the exit
        // (status "helper exited", controller nil). The buffer, the marked
        // range and the caret are untouched.
        let killed = try await h.killHelper()
        XCTAssertEqual(killed, firstPID)
        XCTAssertFalse(model.controllerAttached)
        XCTAssertFalse(model.workerAttached)
        XCTAssertTrue(model.controllerStatus.hasPrefix("helper exited"), model.controllerStatus)
        try await h.turn()
        XCTAssertTrue(h.hasMarkedText, "the exit did not end the composition")
        XCTAssertEqual(h.markedRange, markedBefore)
        XCTAssertEqual(h.selectedRange, selectedBefore)
        XCTAssertEqual(h.string, stringBefore)
        XCTAssertEqual(model.activeText, Self.original)
        XCTAssertEqual(model.editorRevision, rev, "no revision from the exit")

        // Composing on without a helper is still local only.
        h.compose("你好世")
        XCTAssertEqual(model.activeText, Self.original)
        XCTAssertEqual(model.editorRevision, rev)

        // Reattach (File > Attach; the shell has no automatic controller
        // relaunch — see the handoff): the ledger restores r1, the buffer
        // equals it so nothing is resubmitted, and the composition survives.
        h.attach()
        let secondPID = try XCTUnwrap(model.controller?.processIdentifier)
        XCTAssertNotEqual(secondPID, firstPID)
        try await h.waitUntil("helper ready with durable r1") {
            model.controllerState.ready && model.controllerState.durable["main.tex"]?.revision == 1 && model.controllerState.inFlight == nil
        }
        try await h.turn()
        XCTAssertEqual(model.controllerState.durable["main.tex"]?.revision, 1, "nothing was resubmitted: the buffer is still r1")
        XCTAssertTrue(h.hasMarkedText, "reattaching did not end the composition")
        XCTAssertEqual(h.markedRange, NSRange(location: Self.insertAt, length: 3))
        XCTAssertEqual(h.string, expected(inserting: "你好世"))
        XCTAssertEqual(model.activeText, Self.original)
        XCTAssertEqual(model.editorRevision, rev)
        let doc1 = try await h.helperDocument()
        XCTAssertEqual(doc1.revision, 1)
        XCTAssertEqual(doc1.text, Self.original, "the relaunched helper holds the pre-composition text")

        // Commit after the restart: durable exactly once (r2), one ledger entry.
        h.compose("你好世界")
        h.commit("你好世界")
        XCTAssertFalse(h.hasMarkedText)
        let committed = expected(inserting: "你好世界")
        XCTAssertEqual(model.activeText, committed)
        XCTAssertEqual(model.editorRevision, rev + 1)
        XCTAssertEqual(model.caretUTF16, Self.insertAt + 4)
        XCTAssertEqual(model.caretByte, Self.insertAt + 12, "4 ideographs = 12 UTF-8 bytes")
        try await h.waitUntil("durable r2 and its preview") {
            model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil
                && model.result?.revision == model.editorRevision
        }
        try await h.holds("durable stays r2 with nothing in flight", for: 0.5) {
            model.controllerState.durable["main.tex"]?.revision == 2 && model.controllerState.inFlight == nil && !model.controllerState.queued
        }
        let r2 = try await h.helperDocument()
        XCTAssertEqual(r2.revision, 2, "the commit became durable exactly once")
        XCTAssertTrue(r2.text.sameBytes(as: committed))
        XCTAssertEqual(r2.sha256, SourceDigest.sha256Hex(committed))
        XCTAssertEqual(model.compiledDocuments["main.tex"], committed)
        let (undo, redo) = try await h.historyLabels()
        XCTAssertEqual(undo, ["Source edit"], "one ledger entry for the commit after the restart")
        XCTAssertEqual(redo, [])
        let disk = await h.fileState()
        XCTAssertEqual(disk, "differs_from_source")
        XCTAssertEqual(h.launchedPIDs, [firstPID, secondPID])
        print("IME restart test: killed pid \(firstPID), relaunched pid \(secondPID), commit durable once as r2; \(IMEHarness.uptime())")
    }

    // MARK: (a') IME cancel against the helper

    func testCancelledCompositionLeavesTheHelperAndLedgerUntouched() async throws {
        let h = try await IMEHarness.attached("cancel", text: Self.original)
        defer { h.close() }
        let model = h.model
        try await caretAtInsertionPoint(h)
        let rev = model.editorRevision
        for step in ["か", "かん", "漢"] { h.compose(step) }
        XCTAssertTrue(h.hasMarkedText)
        h.cancel()
        XCTAssertFalse(h.hasMarkedText)
        XCTAssertEqual(h.string, Self.original)
        XCTAssertEqual(model.activeText, Self.original)
        XCTAssertEqual(model.editorRevision, rev)
        try await h.holds("nothing submitted after a cancel", for: 0.3) {
            model.controllerState.inFlight == nil && model.controllerState.durable["main.tex"]?.revision == 1
        }
        let doc = try await h.helperDocument()
        XCTAssertEqual(doc.revision, 1)
        XCTAssertEqual(doc.text, Self.original)
        let (ledgerUndo, _) = try await h.historyLabels()
        XCTAssertEqual(ledgerUndo, [])
        // Measured (AppKit, macOS 26): a cancelled composition leaves ONE undo
        // action behind (the marked-text insertion went through the undo-
        // registering path). Undoing it must be a no-op for the buffer, the
        // model and the helper — never a revision or a durable edit.
        let undo = try XCTUnwrap(h.textView.undoManager)
        print("IME cancel test: canUndo after cancel = \(undo.canUndo) (undoActionName '\(undo.undoActionName)')")
        if undo.canUndo {
            undo.undo()
            try await h.turn()
            XCTAssertEqual(h.string, Self.original, "undoing the leftover action changes nothing")
            XCTAssertEqual(model.activeText, Self.original)
            XCTAssertEqual(model.editorRevision, rev, "no revision from the leftover undo action")
            XCTAssertFalse(h.hasMarkedText)
            try await h.holds("nothing submitted after the leftover undo", for: 0.3) {
                model.controllerState.inFlight == nil && model.controllerState.durable["main.tex"]?.revision == 1
            }
            let (undoLabels, _) = try await h.historyLabels()
            XCTAssertEqual(undoLabels, [])
        }
    }
}
