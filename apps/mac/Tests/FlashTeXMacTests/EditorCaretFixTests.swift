import AppKit
import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// The fix at the caret: what the editor offers, and therefore what Tab is
/// allowed to accept. These are the pure decisions — `fixOffered` is the one
/// value that both draws the hint and arms the key, so a test that pins it
/// pins both halves at once.
final class EditorCaretFixTests: XCTestCase {
    /// `Text \alpah here.` — `\alpah` is bytes 5..<11.
    private let text = "Text \\alpah here."

    private func source(_ start: Int, _ end: Int, path: String = "main.tex") -> RuntimeV1.SourceRange {
        .init(path: path, startByte: start, endByte: end)
    }

    /// What the shipping wire actually carries: a flat `suggestion` over the
    /// diagnostic's own `source`, and no `help` at all.
    private func wireDiagnostic(path: String = "main.tex") -> RuntimeV1.Diagnostic {
        .init(severity: .error, message: "\\alpah is not supported by this compiler version",
              source: source(5, 11, path: path), recovery: "skipped the command",
              code: "unknown_command", suggestion: "\\alpha")
    }

    /// The richer form: `help.replacement`, which only reaches the app when the
    /// compiler itself is the attached producer.
    private func helpDiagnostic() -> RuntimeV1.Diagnostic {
        .init(severity: .error, message: "\\alpah is not supported by this compiler version",
              source: source(5, 11), recovery: nil, code: "unknown_command",
              help: .init(message: "did you mean \\alpha?",
                          replacement: .init(startByte: 5, endByte: 11, text: "\\alpha")))
    }

    // MARK: the gate

    /// The regression this feature turned on. `flashtex-render` is the default
    /// producer and sends no `help`, so gating the affordance on
    /// `help.replacement` alone left it permanently dark.
    func testAFlatSuggestionOverItsSourceIsApplicable() {
        XCTAssertTrue(EditorDiagnostics.canApplyHelpReplacement(
            wireDiagnostic(), path: "main.tex", currentText: text,
            compiledRevision: 7, editorRevision: 7))
    }

    func testAHelpReplacementIsStillApplicable() {
        XCTAssertTrue(EditorDiagnostics.canApplyHelpReplacement(
            helpDiagnostic(), path: "main.tex", currentText: text,
            compiledRevision: 7, editorRevision: 7))
    }

    /// Advice with no edit, an out-of-bounds range, another document, and a
    /// buffer that has moved on since the compile all stay unapplicable.
    func testNothingApplicableIsOffered() {
        let adviceOnly = RuntimeV1.Diagnostic(
            severity: .error, message: "m", source: source(5, 11), recovery: nil,
            help: .init(message: "check the spelling"))
        XCTAssertFalse(EditorDiagnostics.canApplyHelpReplacement(
            adviceOnly, path: "main.tex", currentText: text, compiledRevision: 7, editorRevision: 7))

        let past = RuntimeV1.Diagnostic(
            severity: .error, message: "m", source: source(500, 511), recovery: nil,
            code: "unknown_command", suggestion: "\\alpha")
        XCTAssertFalse(EditorDiagnostics.canApplyHelpReplacement(
            past, path: "main.tex", currentText: text, compiledRevision: 7, editorRevision: 7),
            "a range past the end of the buffer")

        XCTAssertFalse(EditorDiagnostics.canApplyHelpReplacement(
            wireDiagnostic(path: "other.tex"), path: "main.tex", currentText: text,
            compiledRevision: 7, editorRevision: 7), "another document")

        XCTAssertFalse(EditorDiagnostics.canApplyHelpReplacement(
            wireDiagnostic(), path: "main.tex", currentText: text,
            compiledRevision: 6, editorRevision: 7), "the buffer moved on since the compile")
    }

    // MARK: what the caret is on

    /// Inclusive at both ends: an author who has just typed the word leaves the
    /// caret directly after it, and the fix has to still be there.
    func testTheFixIsOfferedAcrossTheTokenAndAtBothEnds() {
        for caret in 5...11 {
            let fix = EditorDiagnostics.fixOffered(
                at: caret, in: [wireDiagnostic()], path: "main.tex", currentText: text,
                compiledRevision: 7, editorRevision: 7)
            XCTAssertEqual(fix?.replacement, "\\alpha", "caret at byte \(caret)")
            XCTAssertEqual(fix?.diagnosticIndex, 0)
            XCTAssertEqual(fix?.startByte, 5)
            XCTAssertEqual(fix?.endByte, 11)
        }
    }

    /// Off the token there is no fix — and so, in the editor, no hint and no
    /// claim on Tab.
    func testNoFixAwayFromTheDiagnostic() {
        for caret in [0, 1, 4, 12, 16] {
            XCTAssertNil(EditorDiagnostics.fixOffered(
                at: caret, in: [wireDiagnostic()], path: "main.tex", currentText: text,
                compiledRevision: 7, editorRevision: 7), "caret at byte \(caret)")
        }
    }

    /// A diagnostic carrying no applicable edit never offers one, even with the
    /// caret sitting right on it.
    func testADiagnosticWithoutAnEditOffersNothingAtTheCaret() {
        let adviceOnly = RuntimeV1.Diagnostic(
            severity: .error, message: "m", source: source(5, 11), recovery: nil,
            help: .init(message: "check the spelling"))
        XCTAssertNil(EditorDiagnostics.fixOffered(
            at: 8, in: [adviceOnly], path: "main.tex", currentText: text,
            compiledRevision: 7, editorRevision: 7))
    }

    /// Diagnostics for other documents are skipped even when their byte range
    /// would cover the caret in this one.
    func testAnotherDocumentsDiagnosticIsNotOfferedHere() {
        XCTAssertNil(EditorDiagnostics.fixOffered(
            at: 8, in: [wireDiagnostic(path: "other.tex")], path: "main.tex", currentText: text,
            compiledRevision: 7, editorRevision: 7))
    }

    /// Document order decides when two diagnostics both cover the caret, so the
    /// hint and Tab always agree on which one.
    func testTheFirstCoveringDiagnosticWins() {
        let second = RuntimeV1.Diagnostic(
            severity: .error, message: "second", source: source(0, 17), recovery: nil,
            code: "unknown_command", suggestion: "\\beta")
        let fix = EditorDiagnostics.fixOffered(
            at: 8, in: [wireDiagnostic(), second], path: "main.tex", currentText: text,
            compiledRevision: 7, editorRevision: 7)
        XCTAssertEqual(fix?.replacement, "\\alpha")
        XCTAssertEqual(fix?.diagnosticIndex, 0)
    }

    /// The hint names the fix, so `help.message` is preferred when the producer
    /// sent one and the replacement text stands in when it did not.
    func testTheTitleExplainsWhatTabWillDo() {
        XCTAssertEqual(EditorDiagnostics.fixOffered(
            at: 8, in: [helpDiagnostic()], path: "main.tex", currentText: text,
            compiledRevision: 7, editorRevision: 7)?.title, "did you mean \\alpha?")
        XCTAssertEqual(EditorDiagnostics.fixOffered(
            at: 8, in: [wireDiagnostic()], path: "main.tex", currentText: text,
            compiledRevision: 7, editorRevision: 7)?.title, "Replace with \\alpha")
    }

    // MARK: applying it

    // MARK: what Tab does (the precedence rule)

    private func caretFix() -> EditorDiagnostics.CaretFix {
        .init(diagnosticIndex: 0, title: "did you mean \\alpha?", replacement: "\\alpha",
              startByte: 5, endByte: 11)
    }

    /// Builds a coordinator over `text` with (or without) a fix on offer, and
    /// reports whether Tab accepted it and what the buffer became.
    @MainActor
    private func tab(reverse: Bool, selection: NSRange, fix: EditorDiagnostics.CaretFix?,
                     text body: String = "aa\nbb\ncc") -> (handled: Bool, accepted: Bool, text: String) {
        var accepted = false
        let view = SourceEditorView(text: .constant(body), caretFix: fix,
                                    onAcceptCaretFix: { accepted = true })
        let co = view.makeCoordinator()
        let tv = NSTextView(frame: NSRect(x: 0, y: 0, width: 400, height: 200))
        tv.string = body
        tv.setSelectedRange(selection)
        let handled = co.handleTab(reverse: reverse, in: tv)
        return (handled, accepted, tv.string)
    }

    /// Tab on an offered fix accepts it and never also indents.
    @MainActor
    func testTabAcceptsTheFixAtACaret() {
        let r = tab(reverse: false, selection: NSRange(location: 1, length: 0), fix: caretFix())
        XCTAssertTrue(r.handled)
        XCTAssertTrue(r.accepted)
        XCTAssertEqual(r.text, "aa\nbb\ncc", "accepting the fix must not also insert an indent")
    }

    /// The case that would be a bug rather than a feature: with nothing on
    /// offer Tab must behave exactly as it always has.
    @MainActor
    func testTabIndentsWhenNoFixIsOffered() {
        let r = tab(reverse: false, selection: NSRange(location: 1, length: 0), fix: nil)
        XCTAssertTrue(r.handled)
        XCTAssertFalse(r.accepted)
        XCTAssertEqual(r.text, "a" + EditorPreferences.shared.indentString + "a\nbb\ncc",
                       "an indent unit at the caret, as before")
    }

    /// A multi-line selection is unambiguously "indent this block", so the fix
    /// does not take the key even when one is offered.
    @MainActor
    func testAMultiLineSelectionStillIndentsWithAFixOffered() {
        let r = tab(reverse: false, selection: NSRange(location: 1, length: 5), fix: caretFix())
        XCTAssertTrue(r.handled)
        XCTAssertFalse(r.accepted)
        XCTAssertTrue(r.text.hasPrefix(EditorPreferences.shared.indentString), "the block was indented")
    }

    /// A one-line selection is a replacement target, not a fix target: Tab
    /// replaces it with an indent unit, as before.
    @MainActor
    func testASelectionOnOneLineStillIndentsWithAFixOffered() {
        let r = tab(reverse: false, selection: NSRange(location: 0, length: 2), fix: caretFix())
        XCTAssertTrue(r.handled)
        XCTAssertFalse(r.accepted)
    }

    /// ⇧Tab always means outdent; it never accepts a fix.
    @MainActor
    func testShiftTabNeverAcceptsAFix() {
        let r = tab(reverse: true, selection: NSRange(location: 1, length: 0), fix: caretFix())
        XCTAssertTrue(r.handled)
        XCTAssertFalse(r.accepted)
    }

    // MARK: applying it

    /// Accepting the caret fix produces the same single grouped edit the
    /// Problems panel's "Fix…" produces — one undo step, one replacement.
    func testTheEditProducedIsTheTokenReplacement() throws {
        let preview = try XCTUnwrap(try? EditorDiagnostics.prepareHelpReplacement(
            wireDiagnostic(), path: "main.tex", in: text, compiledText: text).get())
        let grouped = preview.apply()
        XCTAssertEqual(grouped.path, "main.tex")
        XCTAssertTrue(grouped.matches(text))
        let applied = (text as NSString).replacingCharacters(in: grouped.nsRange, with: grouped.text)
        XCTAssertEqual(applied, "Text \\alpha here.", "the document compiles further than before")
    }
}
