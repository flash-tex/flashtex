import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// Vim mode (VimMode.swift): the mode machine driven directly — key sequence
/// in, buffer and caret out — the way `EditorKeyHandlingTests` drives the
/// indentation logic, so nothing here needs a window or a first responder.
/// The last section hosts the real editor to prove the preference gate: with
/// it off every keystroke lands in the buffer exactly as it always did.
@MainActor
final class VimModeTests: XCTestCase {
    override func tearDown() {
        VimModeFeature.enabledOverride = nil
        super.tearDown()
    }

    // MARK: helpers

    private func machine(_ text: String, caret: Int = 0, indent: String = "    ") -> VimMachine {
        let m = VimMachine(text: text, caret: caret)
        m.indentUnit = indent
        return m
    }

    /// Runs a literal key sequence and returns the machine.
    @discardableResult
    private func run(_ text: String, _ keys: String, caret: Int = 0, indent: String = "    ") -> VimMachine {
        let m = machine(text, caret: caret, indent: indent)
        m.send(keys)
        return m
    }

    // MARK: modes

    func testStartsInNormalModeAndOnlyEscapeLeavesInsert() {
        let m = machine("hello")
        XCTAssertEqual(m.mode, .normal)
        m.send("i")
        XCTAssertEqual(m.mode, .insert)
        // Insert mode takes nothing but Escape: every other key is refused so
        // AppKit types it (input methods, dead keys, auto-close, completion).
        XCTAssertFalse(m.handle(.c("x")).handled)
        XCTAssertFalse(m.handle(.c("d")).handled)
        XCTAssertFalse(m.handle(.enter).handled)
        XCTAssertFalse(m.handle(.backspace).handled)
        XCTAssertFalse(m.handle(.ctrl("a")).handled)
        XCTAssertEqual(m.mode, .insert)
        XCTAssertEqual(m.text, "hello", "the machine never types: AppKit does")
        m.send(.escape)
        XCTAssertEqual(m.mode, .normal)
    }

    func testEscapeLeavesInsertModeSteppingLeftLikeVim() {
        let m = machine("hello", caret: 0)
        m.send("A")               // caret past the last character
        XCTAssertEqual(m.mode, .insert)
        XCTAssertEqual(m.caret, 5)
        m.send(.escape)
        XCTAssertEqual(m.caret, 4, "normal mode sits on a character, never past the line end")
    }

    func testControlBracketIsEscapeAndEscapeClearsAPendingCommand() {
        let m = machine("hello")
        m.send("i")
        m.send(.ctrl("["))
        XCTAssertEqual(m.mode, .normal)
        m.send("3d")
        XCTAssertEqual(m.pendingDisplay, "3d")
        m.send(.escape)
        XCTAssertEqual(m.pendingDisplay, "")
        m.send("d")               // the cancelled count must not come back
        m.send("d")
        XCTAssertEqual(m.text, "")
    }

    func testNormalModeSwallowsUnknownKeysInsteadOfTypingThem() {
        let m = machine("hello")
        let outcome = m.handle(.c("z"))
        XCTAssertTrue(outcome.handled)
        XCTAssertTrue(outcome.beep)
        XCTAssertTrue(outcome.edits.isEmpty)
        XCTAssertEqual(m.text, "hello")
    }

    // MARK: motions

    func testCharacterAndLineMotions() {
        let m = machine("abc def\nghi", caret: 0)
        m.send("l"); XCTAssertEqual(m.caret, 1)
        m.send("ll"); XCTAssertEqual(m.caret, 3)
        m.send("h"); XCTAssertEqual(m.caret, 2)
        m.send("$"); XCTAssertEqual(m.caret, 6, "$ lands on the last character, not past it")
        m.send("0"); XCTAssertEqual(m.caret, 0)
        m.send("j"); XCTAssertEqual(m.caret, 8)
        m.send("k"); XCTAssertEqual(m.caret, 0)
        m.send("h"); XCTAssertEqual(m.caret, 0, "h stops at the line start")
    }

    func testFirstNonBlankAndVerticalColumnMemory() {
        let m = machine("    indented\nx\n    also", caret: 0)
        m.send("^"); XCTAssertEqual(m.caret, 4)
        m.send("$"); XCTAssertEqual(m.caret, 11)
        m.send("j"); XCTAssertEqual(m.caret, 13, "the short line clamps to its only character")
        m.send("j"); XCTAssertEqual(m.caret, 22, "the remembered column comes back as far as the line allows")
    }

    func testWordMotions() {
        let m = machine("alpha beta, gamma", caret: 0)
        m.send("w"); XCTAssertEqual(m.caret, 6)
        m.send("w"); XCTAssertEqual(m.caret, 10, "punctuation is its own word")
        m.send("w"); XCTAssertEqual(m.caret, 12)
        m.send("b"); XCTAssertEqual(m.caret, 10)
        m.send("b"); XCTAssertEqual(m.caret, 6)
        m.send("e"); XCTAssertEqual(m.caret, 9, "e lands on the word's last character")
    }

    func testDocumentAndParagraphMotions() {
        let text = "one\ntwo\n\nthree\nfour"
        let m = machine(text, caret: 0)
        m.send("G"); XCTAssertEqual(m.caret, 15, "G goes to the first non-blank of the last line")
        m.send("gg"); XCTAssertEqual(m.caret, 0)
        m.send("}"); XCTAssertEqual(m.caret, 8, "} stops on the blank line")
        m.send("}"); XCTAssertEqual(m.caret, (text as NSString).length - 1, "the buffer end clamps onto the last character")
        m.send("{"); XCTAssertEqual(m.caret, 8)
        m.send("{"); XCTAssertEqual(m.caret, 0)
        m.send("3G"); XCTAssertEqual(m.caret, 8, "a count on G is a line number")
    }

    func testFindTillAndRepeat() {
        let m = machine("a-b-c-d", caret: 0)
        m.send("f-"); XCTAssertEqual(m.caret, 1)
        m.send(";"); XCTAssertEqual(m.caret, 3)
        m.send(","); XCTAssertEqual(m.caret, 1)
        m.send("0")
        m.send("t-"); XCTAssertEqual(m.caret, 0, "t stops one short of the target")
        m.send(";"); XCTAssertEqual(m.caret, 2, "a repeated t steps on rather than standing still")
        m.send("$")
        m.send("F-"); XCTAssertEqual(m.caret, 5)
        m.send("T-"); XCTAssertEqual(m.caret, 4)
        let miss = machine("abc", caret: 0)
        XCTAssertTrue(miss.handle(.c("f")).handled)
        let outcome = miss.handle(.c("z"))
        XCTAssertTrue(outcome.beep, "a target that is not on the line beeps")
        XCTAssertEqual(miss.caret, 0)
    }

    func testCountedMotion() {
        let m = machine("a\nb\nc\nd\ne\nf", caret: 0)
        m.send("5j")
        XCTAssertEqual(m.caret, 10)
        m.send("3k")
        XCTAssertEqual(m.caret, 4)
        let wide = machine("abcdefghij", caret: 0)
        wide.send("7l")
        XCTAssertEqual(wide.caret, 7)
    }

    // MARK: operators composed with motions

    func testDeleteWithMotions() {
        XCTAssertEqual(run("hello world", "dw").text, "world")
        XCTAssertEqual(run("hello world", "d$", caret: 6).text, "hello ")
        XCTAssertEqual(run("hello world", "D", caret: 5).text, "hello")
        XCTAssertEqual(run("foo(bar)baz", "df)").text, "baz")
        XCTAssertEqual(run("foo(bar)baz", "dt)").text, ")baz")
        XCTAssertEqual(run("alpha beta", "de").text, " beta", "e is an inclusive motion")
    }

    func testDeleteWordStopsAtTheLineEndInsteadOfJoiningLines() {
        let m = run("one two\nthree", "dw", caret: 4)
        XCTAssertEqual(m.text, "one \nthree", "dw on the last word of a line never swallows the newline")
    }

    func testDeleteLineAndCounts() {
        XCTAssertEqual(run("one\ntwo\nthree", "dd").text, "two\nthree")
        XCTAssertEqual(run("one\ntwo\nthree\nfour", "3dd").text, "four")
        XCTAssertEqual(run("one\ntwo\nthree", "dj").text, "three", "dj is linewise over both lines")
        XCTAssertEqual(run("one\ntwo\nthree", "dk", caret: 4).text, "three", "dk takes the line above too")
        XCTAssertEqual(run("one\ntwo", "dd", caret: 4).text, "one\n", "the last line keeps the buffer valid")
    }

    func testCountsMultiplyAcrossOperatorAndMotion() {
        XCTAssertEqual(run("a b c d e f g", "d3w").text, "d e f g")
        XCTAssertEqual(run("a b c d e f g", "2d3w").text, "g", "2d3w is d6w")
        XCTAssertEqual(run("1\n2\n3\n4\n5\n6\n7", "2d2d").text, "5\n6\n7", "counts on both halves of dd multiply")
    }

    func testChangeEntersInsertModeAndBehavesLikeVim() {
        let word = run("hello world", "cw")
        XCTAssertEqual(word.text, " world", "cw changes to the end of the word, like ce")
        XCTAssertEqual(word.mode, .insert)
        XCTAssertEqual(word.caret, 0)

        let lastChar = run("hello world", "cw", caret: 4)
        XCTAssertEqual(lastChar.text, "hell world", "on a word's last character cw changes just it")

        let line = run("    foo\nbar", "cc")
        XCTAssertEqual(line.text, "    \nbar", "cc empties the line, keeps it and keeps its indent")
        XCTAssertEqual(line.mode, .insert)
        XCTAssertEqual(line.caret, 4)

        let toEnd = run("hello world", "C", caret: 6)
        XCTAssertEqual(toEnd.text, "hello ")
        XCTAssertEqual(toEnd.mode, .insert)
    }

    func testYankAndPaste() {
        let charwise = run("abc def", "yw")
        XCTAssertEqual(charwise.register.text, "abc ")
        XCTAssertFalse(charwise.register.linewise)
        charwise.send("p")
        XCTAssertEqual(charwise.text, "aabc bc def", "charwise p pastes after the character under the cursor")

        let linewise = run("one\ntwo", "yy")
        XCTAssertTrue(linewise.register.linewise)
        linewise.send("p")
        XCTAssertEqual(linewise.text, "one\none\ntwo", "linewise p opens a new line below")
        XCTAssertEqual(linewise.caret, 4)

        let above = run("one\ntwo", "yyP")
        XCTAssertEqual(above.text, "one\none\ntwo")

        let lastLine = run("one\ntwo", "yy", caret: 4)
        lastLine.send("p")
        XCTAssertEqual(lastLine.text, "one\ntwo\ntwo", "pasting below an unterminated last line adds its newline")

        let counted = run("ab", "yl")
        counted.send("3p")
        XCTAssertEqual(counted.text, "aaaab")
    }

    func testDeleteAndSubstituteSingleCharacters() {
        XCTAssertEqual(run("abc", "x").text, "bc")
        XCTAssertEqual(run("abcdef", "3x").text, "def")
        let atEnd = run("abc", "x", caret: 2)
        XCTAssertEqual(atEnd.text, "ab")
        XCTAssertEqual(atEnd.caret, 1, "the caret steps left off the line end")
        let sub = run("abc", "s")
        XCTAssertEqual(sub.text, "bc")
        XCTAssertEqual(sub.mode, .insert)
        let replaced = run("abc", "rz")
        XCTAssertEqual(replaced.text, "zbc")
        XCTAssertEqual(replaced.mode, .normal, "r replaces one character without entering insert mode")
        XCTAssertEqual(run("abcd", "2rz").text, "zzcd")
    }

    func testInsertEntryPoints() {
        XCTAssertEqual(run("  foo", "i").caret, 0)
        XCTAssertEqual(run("  foo", "I").caret, 2, "I goes to the first non-blank")
        XCTAssertEqual(run("  foo", "a").caret, 1)
        XCTAssertEqual(run("  foo", "A").caret, 5)
        for keys in ["i", "I", "a", "A"] { XCTAssertEqual(run("  foo", keys).mode, .insert, keys) }
    }

    func testOpenLineCopiesTheIndent() {
        let below = run("    foo\nbar", "o")
        XCTAssertEqual(below.text, "    foo\n    \nbar")
        XCTAssertEqual(below.caret, 12)
        XCTAssertEqual(below.mode, .insert)
        let above = run("    foo\nbar", "O", caret: 5)
        XCTAssertEqual(above.text, "    \n    foo\nbar")
        XCTAssertEqual(above.caret, 4)
    }

    func testIndentAndOutdent() {
        XCTAssertEqual(run("foo\nbar", ">>").text, "    foo\nbar")
        XCTAssertEqual(run("a\nb\nc", "3>>").text, "    a\n    b\n    c")
        XCTAssertEqual(run("    foo", "<<").text, "foo")
        XCTAssertEqual(run("foo", "<<").text, "foo", "nothing to remove is not an error")
        XCTAssertEqual(run("foo\nbar", ">>", indent: "\t").text, "\tfoo\nbar", "the editor's indent setting is used")
        let landing = run("foo", ">>")
        XCTAssertEqual(landing.caret, 4, "the caret follows the line's first non-blank")
    }

    // MARK: visual modes

    func testVisualSelectionExtendsWithMotions() {
        let m = machine("abcdef", caret: 0)
        m.send("v")
        XCTAssertEqual(m.mode, .visual)
        XCTAssertEqual(m.selection, NSRange(location: 0, length: 1), "v selects the character under the cursor")
        m.send("ll")
        XCTAssertEqual(m.selection, NSRange(location: 0, length: 3))
        m.send("o")
        XCTAssertEqual(m.caret, 0, "o swaps the ends of the selection")
        m.send(.escape)
        XCTAssertEqual(m.mode, .normal)
        XCTAssertEqual(m.selection.length, 0)
    }

    func testVisualOperators() {
        XCTAssertEqual(run("abcdef", "vlld").text, "def")
        XCTAssertEqual(run("abcdef", "vllx").text, "def")
        let changed = run("abcdef", "vllc")
        XCTAssertEqual(changed.text, "def")
        XCTAssertEqual(changed.mode, .insert)
        let yanked = run("abcdef", "vlly")
        XCTAssertEqual(yanked.register.text, "abc")
        XCTAssertEqual(yanked.mode, .normal)
        // `u`/`U` mean case conversion in vim; unimplemented, so they beep
        // instead of quietly undoing.
        let m = machine("abcdef")
        m.send("vl")
        let unsupported = m.handle(.c("u"))
        XCTAssertTrue(unsupported.beep)
        XCTAssertTrue(unsupported.requests.isEmpty)
    }

    func testVisualLineOperators() {
        let deleted = run("one\ntwo\nthree", "Vjd")
        XCTAssertEqual(deleted.text, "three")
        XCTAssertEqual(deleted.mode, .normal)
        let yanked = run("one\ntwo", "Vy")
        XCTAssertTrue(yanked.register.linewise)
        XCTAssertEqual(yanked.register.text, "one\n")
        XCTAssertEqual(run("a\nb\nc", "Vj>").text, "    a\n    b\nc")
        XCTAssertEqual(run("    a\n    b", "Vj<").text, "a\nb")
        let selection = machine("one\ntwo", caret: 1)
        selection.send("V")
        XCTAssertEqual(selection.selection, NSRange(location: 0, length: 4), "V selects the whole line")
    }

    func testVisualPasteReplacesTheSelection() {
        let m = run("abc def", "yw")          // register: "abc "
        m.send("$")
        m.send("vh")
        m.send("p")
        XCTAssertEqual(m.text, "abc dabc ")
    }

    // MARK: undo, search, ex commands, %

    func testUndoAndRedoAskTheAppsUndoManager() {
        let m = machine("abc")
        XCTAssertEqual(m.handle(.c("u")).requests, [.undo])
        XCTAssertEqual(m.handle(.ctrl("r")).requests, [.redo])
        XCTAssertTrue(m.handle(.c("u")).edits.isEmpty, "vim mode never keeps its own undo stack")
    }

    func testSearchUsesTheAppsFindBar() {
        let m = machine("abc")
        XCTAssertEqual(m.handle(.c("/")).requests, [.openFind(backward: false)])
        XCTAssertNil(m.commandLine, "the Find bar owns the query, so no vim command line stays open")
        XCTAssertEqual(m.handle(.c("n")).requests, [.findNext])
        XCTAssertEqual(m.handle(.c("N")).requests, [.findPrevious])
        XCTAssertEqual(m.handle(.c("?")).requests, [.openFind(backward: true)])
        XCTAssertEqual(m.handle(.c("n")).requests, [.findPrevious], "after ? the directions swap")
        XCTAssertEqual(m.handle(.c("N")).requests, [.findNext])
    }

    func testExCommands() {
        func ex(_ command: String) -> VimOutcome {
            let m = machine("abc")
            m.send(command)
            return m.handle(.enter)
        }
        XCTAssertEqual(ex(":w").requests, [.save])
        XCTAssertEqual(ex(":q").requests, [.close])
        XCTAssertEqual(ex(":wq").requests, [.saveAndClose])
        XCTAssertEqual(ex(":x").requests, [.saveAndClose])
        XCTAssertEqual(ex(":q!").requests, [.discardAndClose])
        let unknown = ex(":nope")
        XCTAssertTrue(unknown.requests.isEmpty)
        XCTAssertTrue(unknown.beep)

        let typing = machine("abc")
        typing.send(":w")
        XCTAssertEqual(typing.pendingDisplay, ":w", "the indicator shows the line being typed")
        typing.send(.backspace)
        XCTAssertEqual(typing.pendingDisplay, ":")
        typing.send(.escape)
        XCTAssertNil(typing.commandLine)

        let jump = machine("one\ntwo\nthree")
        jump.send(":3")
        _ = jump.handle(.enter)
        XCTAssertEqual(jump.caret, 8, ":<line> jumps")
    }

    func testPercentUsesTheEditorsDelimiterMatcherThenTheAppsGoToMatching() {
        let m = machine("\\frac{a}{b}", caret: 5)
        m.send("%")
        XCTAssertEqual(m.caret, 7, "% jumps to the partner of the brace under the cursor")
        m.send("%")
        XCTAssertEqual(m.caret, 5, "and back again")

        let scan = machine("\\frac{ab}", caret: 0)
        scan.send("%")
        XCTAssertEqual(scan.caret, 8, "with no delimiter under the cursor % scans forward on the line")

        // No bracket or math pair in reach: the app's own Go to Matching
        // command handles \begin/\end and \label/\ref.
        let fallback = machine("plain text", caret: 0)
        XCTAssertEqual(fallback.handle(.c("%")).requests, [.goToMatching])
    }

    func testCommentedDelimitersAreNotMatched() {
        // BraceMatcher is LaTeX-aware, so a brace after % is not a delimiter.
        let m = machine("% {comment\ntext", caret: 2)
        XCTAssertEqual(m.handle(.c("%")).requests, [.goToMatching])
    }

    // MARK: the preference gate

    func testFeatureIsOffByDefaultAndTheOverrideIsTheTestSeam() {
        let suite = "flashtex.tests.VimMode.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let prefs = EditorPreferences(defaults: defaults)
        XCTAssertFalse(prefs.vimMode, "vim mode is off out of the box")
        XCTAssertFalse(EditorPreferences.defaultSnapshot.vimMode)
        prefs.vimMode = true
        XCTAssertTrue(defaults.bool(forKey: EditorPreferences.Key.vimMode.storageKey), "and it persists")
        prefs.resetToDefaults()
        XCTAssertFalse(prefs.vimMode)

        VimModeFeature.enabledOverride = false
        XCTAssertFalse(VimModeFeature.isEnabled)
        VimModeFeature.enabledOverride = true
        XCTAssertTrue(VimModeFeature.isEnabled)
        VimModeFeature.enabledOverride = nil
        XCTAssertFalse(VimModeFeature.flag("FLASHTEX_VIM_MODE", environment: [:]))
        XCTAssertTrue(VimModeFeature.flag("FLASHTEX_VIM_MODE", environment: ["FLASHTEX_VIM_MODE": "1"]))
        XCTAssertFalse(VimModeFeature.flag("FLASHTEX_VIM_MODE", environment: ["FLASHTEX_VIM_MODE": "0"]),
                       "unlike the capture switches this one is opt-in, not opt-out")
    }

    func testCommandAndOptionKeysAreNeverVimKeys() {
        XCTAssertNil(VimModeFeature.key(for: Self.event("s", code: 1, flags: [.command])), "⌘S stays the app's")
        XCTAssertNil(VimModeFeature.key(for: Self.event("d", code: 2, flags: [.command, .shift])))
        XCTAssertNil(VimModeFeature.key(for: Self.event("ø", code: 31, flags: [.option])), "⌥-composed characters type")
        XCTAssertEqual(VimModeFeature.key(for: Self.event("d", code: 2)), .c("d"))
        XCTAssertEqual(VimModeFeature.key(for: Self.event("\u{1b}", code: 53)), .escape)
        XCTAssertEqual(VimModeFeature.key(for: Self.event("r", code: 15, flags: [.control])), .ctrl("r"))
        XCTAssertEqual(VimModeFeature.key(for: Self.event("", code: 125)), .c("j"), "↓ is j")
        XCTAssertNil(VimModeFeature.key(for: Self.event("", code: 125, flags: [.shift])), "⇧↓ keeps AppKit's selection")
    }

    /// The landing offset a command reports is in the buffer the edits
    /// *produce*. Clamping it against the pre-edit length silently dragged the
    /// caret back on every command that grows the buffer (`o`, `O`, `p`, `P`).
    func testTheAppliedSelectionIsClampedAgainstThePostEditBuffer() {
        let open = [EditorKeyHandling.LineEdit(range: NSRange(location: 7, length: 0), replacement: "\n")]
        XCTAssertEqual(VimModeFeature.length(after: open, from: 7), 8)
        XCTAssertEqual(VimModeFeature.clamp(NSRange(location: 8, length: 0), to: 8), NSRange(location: 8, length: 0))
        XCTAssertEqual(VimModeFeature.clamp(NSRange(location: 8, length: 0), to: 7), NSRange(location: 7, length: 0),
                       "the pre-edit length is what used to strand the caret")

        let cut = [EditorKeyHandling.LineEdit(range: NSRange(location: 0, length: 4), replacement: "")]
        XCTAssertEqual(VimModeFeature.length(after: cut, from: 7), 3)
        XCTAssertEqual(VimModeFeature.clamp(NSRange(location: 9, length: 2), to: 3), NSRange(location: 3, length: 0))
        XCTAssertEqual(VimModeFeature.clamp(NSRange(location: -4, length: -1), to: 3), NSRange(location: 0, length: 0),
                       "never a negative or out-of-range NSRange")
        XCTAssertEqual(VimModeFeature.length(after: [], from: 12), 12)
    }

    // MARK: hosted — the feature gate on the real editor

    func testWithThePreferenceOffEveryKeystrokeTypesAsBefore() async throws {
        VimModeFeature.enabledOverride = false
        let (window, tv) = try await hostEditor("alpha\nbeta")
        defer { window.close() }
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        Self.type(tv, "dd")
        XCTAssertEqual(tv.string, "ddalpha\nbeta", "with vim mode off d d is just two characters")
        Self.type(tv, "ixo")
        XCTAssertEqual(tv.string, "ddixoalpha\nbeta")
    }

    func testWithThePreferenceOnTheSameKeysRunVimCommands() async throws {
        VimModeFeature.enabledOverride = true
        let (window, tv) = try await hostEditor("alpha\nbeta")
        defer { window.close() }
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        Self.type(tv, "dd")
        XCTAssertEqual(tv.string, "beta", "dd deletes the line")
        // `i` then typing goes through AppKit untouched, so the text lands.
        Self.type(tv, "ix")
        XCTAssertEqual(tv.string, "xbeta")
        let coordinator = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        XCTAssertEqual(coordinator.vim.mode, .insert)
        Self.type(tv, "\u{1b}", code: 53)
        XCTAssertEqual(coordinator.vim.mode, .normal)
    }

    func testVimEditsGoThroughTheEditorsOwnUndoManager() async throws {
        VimModeFeature.enabledOverride = true
        let (window, tv) = try await hostEditor("alpha\nbeta")
        defer { window.close() }
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        Self.type(tv, "dd")
        XCTAssertEqual(tv.string, "beta")
        XCTAssertEqual(tv.undoManager?.canUndo, true, "one vim command is one step on the editor's stack")
        Self.type(tv, "u")                       // vim's undo
        XCTAssertEqual(tv.string, "alpha\nbeta")
        Self.type(tv, "r", code: 15, flags: [.control]) // ⌃R
        XCTAssertEqual(tv.string, "beta")
        tv.undoManager?.undo()                   // and ⌘Z is the same stack
        XCTAssertEqual(tv.string, "alpha\nbeta")
    }

    func testOpenLineAndPasteAtTheBufferEndLandOnTheNewText() async throws {
        VimModeFeature.enabledOverride = true
        let (window, tv) = try await hostEditor("one\ntwo")
        defer { window.close() }
        tv.setSelectedRange(NSRange(location: 4, length: 0))
        Self.type(tv, "o")
        XCTAssertEqual(tv.string, "one\ntwo\n")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 8, length: 0),
                       "the caret is on the opened line, not clamped back onto the old buffer end")
        Self.type(tv, "x")                        // insert mode: AppKit types it
        XCTAssertEqual(tv.string, "one\ntwo\nx")
        Self.type(tv, "\u{1b}", code: 53)
        Self.type(tv, "yyp")                      // paste below the last line
        XCTAssertEqual(tv.string, "one\ntwo\nx\nx")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 11, length: 0))
    }

    func testTheIndicatorReportsTheMode() async throws {
        VimModeFeature.enabledOverride = true
        let statuses = StatusProbe()
        let (window, tv) = try await hostEditor("alpha", statuses: statuses)
        defer { window.close() }
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        Self.type(tv, "i")
        XCTAssertEqual(statuses.last?.mode, .insert)
        XCTAssertEqual(statuses.last?.enabled, true)
        Self.type(tv, "\u{1b}", code: 53)
        XCTAssertEqual(statuses.last?.mode, .normal)
        Self.type(tv, "v")
        XCTAssertEqual(statuses.last?.mode, .visual)
        XCTAssertEqual(VimMode.visual.indicator, "VISUAL")
        Self.type(tv, "\u{1b}", code: 53)
        Self.type(tv, "3")
        XCTAssertEqual(statuses.last?.pending, "3", "a half-typed command is visible")
    }

    // MARK: hosting

    private final class StatusProbe { var all: [VimModeStatus] = []; var last: VimModeStatus? { all.last } }

    private struct Host: View {
        var model: ShellModel
        var statuses: StatusProbe
        var body: some View {
            SourceEditorView(
                text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                onCaretChange: { model.caretUTF16 = $0 },
                onVimStatus: { statuses.all.append($0) }
            )
        }
    }

    private func hostEditor(_ text: String, statuses: StatusProbe = StatusProbe()) async throws -> (NSWindow, NSTextView) {
        let model = ShellModel()
        model.updateActiveText(text)
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                              backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model, statuses: statuses))
        window.orderFrontRegardless() // never makeKey: the tests must not steal focus
        var found: NSTextView?
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            found = TypingBenchDriver.findTextView(in: [window.contentView!])
            if found != nil, found?.string == text { break }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        let tv = try XCTUnwrap(found, "editor text view")
        XCTAssertEqual(tv.string, text)
        XCTAssertTrue(window.makeFirstResponder(tv))
        return (window, tv)
    }

    /// A real key event through `CompletingTextView.keyDown`, the path the vim
    /// hook sits on (the same helper `CompletionTests` uses).
    private static func event(_ chars: String, code: UInt16, flags: NSEvent.ModifierFlags = [],
                              window: NSWindow? = nil) -> NSEvent {
        NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags,
                         timestamp: ProcessInfo.processInfo.systemUptime,
                         windowNumber: window?.windowNumber ?? 0, context: nil,
                         characters: chars, charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
    }

    /// US-layout virtual key codes, so `interpretKeyEvents` resolves the same
    /// characters a real keyboard would.
    private static let codes: [Character: UInt16] = [
        "a": 0, "s": 1, "d": 2, "f": 3, "h": 4, "g": 5, "z": 6, "x": 7, "c": 8, "v": 9,
        "b": 11, "q": 12, "w": 13, "e": 14, "r": 15, "y": 16, "t": 17, "1": 18, "2": 19,
        "3": 20, "4": 21, "6": 22, "5": 23, "9": 25, "7": 26, "8": 28, "0": 29,
        "o": 31, "u": 32, "i": 34, "p": 35, "l": 37, "j": 38, "k": 40, "n": 45, "m": 46,
    ]

    private static func type(_ tv: NSTextView, _ chars: String, code: UInt16? = nil, flags: NSEvent.ModifierFlags = []) {
        if let code {
            tv.keyDown(with: event(chars, code: code, flags: flags, window: tv.window))
            return
        }
        for ch in chars {
            tv.keyDown(with: event(String(ch), code: codes[ch] ?? 0, flags: flags, window: tv.window))
        }
    }
}
