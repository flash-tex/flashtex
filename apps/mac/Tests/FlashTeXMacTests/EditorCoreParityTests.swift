import AppKit
import XCTest
import FlashTeXEditorCore
@testable import FlashTeXMac

/// The Mac editor's auto-close, completion vocabulary, snippets and
/// indentation are the shared `FlashTeXEditorCore` functions the iPad calls
/// (#955 §4): these tests pin the shared decisions themselves, and that the
/// Mac's names for them return the shared values. The iPad's hosted tests
/// (`EditorControllerTests`, `EditorCompletionTests`) cannot run on a Mac
/// without a simulator, so this is where the shared code runs on every
/// `swift test`.
final class EditorCoreParityTests: XCTestCase {
    // MARK: #932 in the shared core

    /// `\[` auto-closed to `\[|\]`, then `\alpha` typed by hand: the `\` of
    /// `\alpha` completes nothing (it is not a closer's terminal unit), so it
    /// is inserted; only `]` after a hand-typed `\` steps over the `\]`.
    func testOvertypeNeverConsumesTheBackslashOfAPendingTwoUnitCloser() {
        // `\[` + auto-inserted `\]`, pending at 2 and 3, caret at 2.
        XCTAssertNil(AutoClose.overtypePrefix(typing: "\\", at: 2, in: "\\[\\]" as NSString, pending: [2, 3]),
                     "`\\` starts a command; it never overtypes the closer's first unit")
        // After `\alpha`: `\[\alpha|\]`, pending 8, 9.
        let body = "\\[\\alpha\\]" as NSString
        XCTAssertNil(AutoClose.overtypePrefix(typing: "\\", at: 8, in: body, pending: [8, 9]))
        XCTAssertNil(AutoClose.overtypePrefix(typing: "]", at: 8, in: body, pending: [8, 9]),
                     "`]` without a hand-typed `\\` before it is a plain bracket")
        // `\` typed by hand: `\[\alpha\|\]`, pending 9, 10; `]` completes it, dropping one hand-typed unit.
        XCTAssertEqual(AutoClose.overtypePrefix(typing: "]", at: 9, in: "\\[\\alpha\\\\]" as NSString, pending: [9, 10]), 1)
        // Same for `\(` … `\)`.
        XCTAssertNil(AutoClose.overtypePrefix(typing: "\\", at: 2, in: "\\(\\)" as NSString, pending: [2, 3]))
        XCTAssertEqual(AutoClose.overtypePrefix(typing: ")", at: 3, in: "\\(\\\\)" as NSString, pending: [3, 4]), 1)
        // A single-unit closer is still stepped over directly.
        XCTAssertEqual(AutoClose.overtypePrefix(typing: "}", at: 1, in: "{}" as NSString, pending: [1]), 0)
    }

    /// The whole #932 keystroke sequence over the shared functions alone (the
    /// way the iPad's `EditorController` drives them): `\[`, `\alpha`, `\]`.
    func testTypingDisplayMathThroughTheSharedCoreGivesOneCloser() {
        var text = ""
        var caret = 0
        var pending: [Int] = []
        func type(_ keys: String) {
            for key in keys {
                let unit = String(key)
                let ns = text as NSString
                if let prefix = AutoClose.overtypePrefix(typing: unit, at: caret, in: ns, pending: pending) {
                    let start = caret - prefix
                    text = ns.replacingCharacters(in: NSRange(location: start, length: prefix), with: "")
                    pending = AutoClose.shifted(pending, edit: NSRange(location: start, length: prefix), replacementLength: 0)
                    pending.removeAll { $0 >= start && $0 <= start + prefix }
                    caret = start + prefix + 1
                    continue
                }
                text = ns.replacingCharacters(in: NSRange(location: caret, length: 0), with: unit)
                pending = AutoClose.shifted(pending, edit: NSRange(location: caret, length: 0), replacementLength: 1)
                caret += 1
                if let closer = AutoClose.closer(afterTyping: key, in: text, caretUTF16: caret, mathMode: false) {
                    text = (text as NSString).replacingCharacters(in: NSRange(location: caret, length: 0), with: closer)
                    pending += (0..<(closer as NSString).length).map { caret + $0 }
                }
            }
        }
        type("\\[")
        XCTAssertEqual(text, "\\[\\]")
        type("\\alpha")
        XCTAssertEqual(text, "\\[\\alpha\\]")
        type("\\]")
        XCTAssertEqual(text, "\\[\\alpha\\]")
        XCTAssertEqual(caret, 10)
        XCTAssertEqual(pending, [])
    }

    /// The mode lookup is only paid for a `\left` opener.
    func testCloserAsksForTheModeOnlyAfterLeft() {
        var asked = 0
        func math() -> Bool { asked += 1; return true }
        XCTAssertEqual(AutoClose.closer(afterTyping: "{", in: "x{", caretUTF16: 2, mathMode: math()), "}")
        XCTAssertEqual(AutoClose.closer(afterTyping: "[", in: "\\[", caretUTF16: 2, mathMode: math()), "\\]")
        XCTAssertEqual(asked, 0)
        XCTAssertEqual(AutoClose.closer(afterTyping: "(", in: "$\\left(", caretUTF16: 7, mathMode: math()), "\\right)")
        XCTAssertEqual(asked, 1)
    }

    // MARK: vocabulary and snippets

    func testMacVocabularyIsTheSharedDecodersRows() {
        let shared = Completion.Vocabulary.shared
        XCTAssertFalse(shared.commands.isEmpty)
        XCTAssertEqual(Completion.Vocabulary.entries.map(\.name), shared.commands.map(\.name), "same rows, same order")
        for (entry, command) in zip(Completion.Vocabulary.entries, shared.commands) {
            XCTAssertEqual(entry.origin.rawValue, command.origin, entry.name)
            XCTAssertEqual(entry.mathDescription, command.mathDescription, entry.name)
            XCTAssertEqual(entry.requiresClass, command.requiresClass, entry.name)
            for mode in [nil, true, false] as [Bool?] {
                XCTAssertEqual(Completion.allows(entry, mathMode: mode), LaTeXVocabulary.allows(command, mathMode: mode), entry.name)
            }
            XCTAssertEqual(entry.snippet, LaTeXSnippets.argument(name: command.name, arguments: command.arguments), entry.name)
        }
        XCTAssertEqual(Completion.Vocabulary.environments, shared.environments.map(\.name))
    }

    func testMacSnippetsAreTheSharedOnes() {
        for name in ["itemize", "description", "figure", "table*", "align*", "proof", "mything"] {
            XCTAssertEqual(Completion.environmentSnippet(name, indent: "  ", unit: "    "),
                           LaTeXSnippets.environment(name, indent: "  ", unit: "    "), name)
        }
        XCTAssertEqual(Completion.argumentSnippet(name: "frac", arguments: "{num}{den}"),
                       LaTeXSnippet(text: "\\frac{}{}", caretUTF16: 6, stops: [8, 9]))
        // A document's own `\left` macro gets its skeleton, not the vocabulary override.
        XCTAssertNil(Completion.argumentSnippet(name: "left", arguments: ""))
        XCTAssertEqual(Completion.Vocabulary.byName["left"]?.snippet, LaTeXSnippets.overrides["left"])
    }

    // MARK: indentation

    func testIndentSkipsBlankLinesInABlockButNotAtACaret() {
        let text = "a\n\n  \nb" as NSString
        let block = LaTeXEditing.indentEdits(in: text, range: NSRange(location: 0, length: text.length), unit: "  ")!
        XCTAssertEqual(block.edits.map(\.range.location), [0, 6], "the empty and whitespace-only lines get no trailing indent")
        let caret = LaTeXEditing.indentEdits(in: text, range: NSRange(location: 2, length: 0), unit: "  ")!
        XCTAssertEqual(caret.edits.map(\.range.location), [2], "a caret on a blank line indents it")
        XCTAssertEqual(caret.selection, NSRange(location: 4, length: 0))
    }

    func testOutdentKeepsACaretInTheIndentationOnItsLine() {
        // Caret between the second and third of four leading spaces.
        let text = "    x\ny" as NSString
        let plan = LaTeXEditing.outdentEdits(in: text, range: NSRange(location: 2, length: 0), unit: "    ")!
        XCTAssertEqual(plan.edits, [LaTeXEditing.LineEdit(range: NSRange(location: 0, length: 4), replacement: "")])
        XCTAssertEqual(plan.selection, NSRange(location: 0, length: 0), "at the line start, not past `x`")
    }

    func testOutdentUnderATabUnitLeavesSpacesAlone() {
        let plan = LaTeXEditing.outdentEdits(in: "    x" as NSString, range: NSRange(location: 0, length: 0), unit: "\t")!
        XCTAssertEqual(plan.edits, [], "a tab unit does not guess how many spaces a tab is")
    }

    func testIndentHandlesCRLFLines() {
        let text = "a\r\nb\r\nc" as NSString
        let plan = LaTeXEditing.indentEdits(in: text, range: NSRange(location: 0, length: 4), unit: "\t")!
        XCTAssertEqual(plan.edits.map(\.range.location), [0, 3])
    }

    /// The iPad's `EditorControllerTests.testIndentAndOutdent` sequence over
    /// `LaTeXEditing.indent` alone (that hosted test needs a simulator).
    func testIPadIndentAndOutdentSequence() {
        var text = "a\nb"
        var selection = NSRange(location: 0, length: 3)
        func run(outdent: Bool) {
            guard let t = LaTeXEditing.indent(in: text as NSString, selection: selection, unit: "    ", outdent: outdent) else { return }
            text = (text as NSString).replacingCharacters(in: t.range, with: t.replacement)
            selection = t.selection
        }
        run(outdent: false)
        XCTAssertEqual(text, "    a\n    b")
        run(outdent: true)
        XCTAssertEqual(text, "a\nb")
        selection = NSRange(location: 1, length: 0)
        run(outdent: false)
        XCTAssertEqual(text, "    a\nb")
        XCTAssertEqual(selection, NSRange(location: 5, length: 0), "the caret stays after `a`")
        run(outdent: true)
        XCTAssertEqual(text, "a\nb")
        XCTAssertEqual(selection, NSRange(location: 1, length: 0))
    }

    /// The iPad's one-replacement form equals the Mac's per-line edits applied.
    func testIPadIndentIsTheMacEditsAsOneReplacement() {
        let text = "\\begin{itemize}\n\\item a\n\n\\item b\n\\end{itemize}" as NSString
        for outdent in [false, true] {
            let source = outdent ? "  x\n\ty\nz" as NSString : text
            let range = NSRange(location: 0, length: source.length)
            let plan = outdent ? LaTeXEditing.outdentEdits(in: source, range: range, unit: "  ")!
                : LaTeXEditing.indentEdits(in: source, range: range, unit: "  ")!
            let applied = NSMutableString(string: source as String)
            for edit in plan.edits.sorted(by: { $0.range.location > $1.range.location }) {
                applied.replaceCharacters(in: edit.range, with: edit.replacement)
            }
            let toggle = LaTeXEditing.indent(in: source, selection: range, unit: "  ", outdent: outdent)!
            XCTAssertEqual(source.replacingCharacters(in: toggle.range, with: toggle.replacement), applied as String)
            XCTAssertEqual(toggle.selection, plan.selection)
        }
    }
}
