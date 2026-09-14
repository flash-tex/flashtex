import AppKit
import XCTest
@testable import FlashTeXMac

/// Vim keybinding emulation (VimMode.swift) driven through the real editor
/// view's `keyDown` in an offscreen window (never key, no focus stolen), with
/// the preference pinned per view so the user's defaults are untouched.
@MainActor
final class VimModeTests: XCTestCase {
    private var window: NSWindow!
    private var tv: CompletingTextView!

    override func setUp() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        tv.allowsUndo = true
        window.orderFrontRegardless() // never makeKey
        window.makeFirstResponder(tv)
        tv.vimEnabledOverride = true
    }

    override func tearDown() async throws {
        tv.vimEnabledOverride = false
        window.orderOut(nil)
    }

    // MARK: driving

    private func load(_ text: String, caret: Int = 0) {
        tv.string = text
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        tv.undoManager?.removeAllActions()
    }

    private func key(_ chars: String, code: UInt16 = 0, flags: NSEvent.ModifierFlags = []) {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                                 windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: chars,
                                 charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        tv.keyDown(with: e)
        // Each key is its own event, as in the app: the undo manager's
        // per-event group closes between keys.
        RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.001))
    }

    /// Types `keys` one character at a time; `<Esc>`, `<CR>`, `<BS>` and `<C-x>` are escapes.
    private func type(_ keys: String) {
        var rest = Substring(keys)
        while let ch = rest.first {
            if ch == "<", let close = rest.firstIndex(of: ">") {
                let name = rest[rest.index(after: rest.startIndex)..<close]
                rest = rest[rest.index(after: close)...]
                switch name {
                case "Esc": key("\u{1B}", code: 53)
                case "CR": key("\r", code: 36)
                case "BS": key("\u{7F}", code: 51)
                default:
                    if name.hasPrefix("C-") { key(String(name.dropFirst(2)), flags: .control) } else { XCTFail("unknown key <\(name)>") }
                }
                continue
            }
            key(String(ch))
            rest = rest.dropFirst()
        }
    }

    private var text: String { tv.string }
    private var caret: Int { tv.selectedRange().location }
    private var mode: VimMode.Mode { tv.vim.mode }

    // MARK: modes

    func testStartsInNormalModeAndInsertReturnsWithEsc() {
        load("abc")
        XCTAssertEqual(mode, .normal)
        XCTAssertEqual(VimMode.Status.shared.indicator, "-- NORMAL --")
        type("i")
        XCTAssertEqual(mode, .insert)
        XCTAssertEqual(VimMode.Status.shared.indicator, "-- INSERT --")
        XCTAssertFalse(tv.vim.wantsBlockCaret)
        key("X") // insert mode: the editor's own typing path
        XCTAssertEqual(text, "Xabc")
        type("<Esc>")
        XCTAssertEqual(mode, .normal)
        XCTAssertEqual(caret, 0, "Esc steps back onto the last typed character")
        XCTAssertTrue(tv.vim.wantsBlockCaret)
    }

    func testPreferenceOffMeansNoInterception() {
        tv.vimEnabledOverride = false
        load("abc")
        XCTAssertNil(VimMode.Status.shared.indicator)
        key("x")
        key("j")
        XCTAssertEqual(text, "xjabc", "normal keys are typed, not interpreted")
    }

    func testControlBracketLeavesInsertAndCommandShortcutsPassThrough() {
        load("abc")
        type("i")
        key("[", flags: .control)
        XCTAssertEqual(mode, .normal)
        // ⌘-shortcuts are never Vim's: nothing is typed and the mode is unchanged.
        key("a", flags: .command)
        XCTAssertEqual(mode, .normal)
        XCTAssertEqual(text, "abc")
    }

    // MARK: motions and counts

    func testBasicMotionsWithCounts() {
        load("hello world\nsecond line\nthird")
        type("3l"); XCTAssertEqual(caret, 3)
        type("h"); XCTAssertEqual(caret, 2)
        type("j"); XCTAssertEqual(caret, 14, "same column on the next line")
        type("k"); XCTAssertEqual(caret, 2)
        type("$"); XCTAssertEqual(caret, 10, "$ lands on the last character")
        type("j"); XCTAssertEqual(caret, 22, "the column is remembered")
        type("0"); XCTAssertEqual(caret, 12)
        type("w"); XCTAssertEqual(caret, 19)
        type("b"); XCTAssertEqual(caret, 12)
        type("e"); XCTAssertEqual(caret, 17)
        type("gg"); XCTAssertEqual(caret, 0)
        type("G"); XCTAssertEqual(caret, 24)
        type("2G"); XCTAssertEqual(caret, 12)
        type("2j"); XCTAssertEqual(caret, 24, "past the last line clamps")
    }

    func testWordMotionsDistinguishPunctuationAndBigWords() {
        load("\\section{Intro} x")
        type("w"); XCTAssertEqual(caret, 1, "the backslash is punctuation, `section` the next word")
        type("w"); XCTAssertEqual(caret, 8)
        type("0W"); XCTAssertEqual(caret, 16, "W skips the whole \\section{Intro}")
        type("B"); XCTAssertEqual(caret, 0)
        type("E"); XCTAssertEqual(caret, 14)
    }

    func testFirstNonBlankParagraphsAndSentences() {
        load("  indented\n\npara two. Next one!\n\nthree")
        type("^"); XCTAssertEqual(caret, 2)
        type("}"); XCTAssertEqual(caret, 11, "} stops on the blank line")
        type("}"); XCTAssertEqual(caret, 32)
        type("{"); XCTAssertEqual(caret, 11)
        type("j)"); XCTAssertEqual(caret, 22, ") goes to the next sentence")
        type("("); XCTAssertEqual(caret, 12)
    }

    func testFindCharAndRepeat() {
        load("a-b-c-d")
        type("f-"); XCTAssertEqual(caret, 1)
        type(";"); XCTAssertEqual(caret, 3)
        type(","); XCTAssertEqual(caret, 1)
        type("t-"); XCTAssertEqual(caret, 2, "t stops before the character")
        type(";"); XCTAssertEqual(caret, 4, "; after t does not stick")
        type("$F-"); XCTAssertEqual(caret, 5)
        type("2Fa"); XCTAssertEqual(caret, 5, "a miss leaves the caret alone")
    }

    func testPercentMatchesBracketsAndEnvironments() {
        load("f(a[b]c) end")
        type("%"); XCTAssertEqual(caret, 7, "% from before a bracket jumps to the first one's match")
        type("%"); XCTAssertEqual(caret, 1)
        load("\\begin{itemize}\n\\item x\n\\end{itemize}\n", caret: 3)
        type("%"); XCTAssertEqual(caret, 24, "on \\begin: the matching \\end (EditorNavigation pairs)")
        type("%"); XCTAssertEqual(caret, 0)
    }

    func testMarksAndScreenLines() {
        load("one\ntwo\nthree\nfour")
        type("jlma"); XCTAssertEqual(caret, 5)
        type("G'a"); XCTAssertEqual(caret, 4, "' goes to the mark's line start")
        type("G`a"); XCTAssertEqual(caret, 5, "` goes to the exact position")
        type("'z"); XCTAssertEqual(caret, 5, "an unset mark is refused")
        type("H"); XCTAssertEqual(caret, 0)
        type("L"); XCTAssertEqual(caret, 14, "every line is on screen: L is the last")
        type("<C-u>"); XCTAssertEqual(caret, 0)
    }

    // MARK: operators, text objects, registers

    func testDeleteChangeYankWithMotionsAndLines() {
        load("one two three\nfour")
        type("dw"); XCTAssertEqual(text, "two three\nfour")
        type("de"); XCTAssertEqual(text, " three\nfour", "e is inclusive")
        type("u"); XCTAssertEqual(text, "two three\nfour", "u is the editor's undo")
        type("<C-r>"); XCTAssertEqual(text, " three\nfour")
        type("dd"); XCTAssertEqual(text, "four")
        XCTAssertEqual(tv.vim.unnamedRegister, VimMode.Register(text: " three\n", linewise: true))
        type("p"); XCTAssertEqual(text, "four\n three", "a linewise paste goes below")
        type("P"); XCTAssertEqual(text, "four\n three\n three")
        load("alpha beta")
        type("yw"); XCTAssertEqual(tv.vim.unnamedRegister?.text, "alpha ")
        type("$p"); XCTAssertEqual(text, "alpha betaalpha ")
        load("say hello world")
        type("wcw"); XCTAssertEqual(text, "say  world"); XCTAssertEqual(mode, .insert)
        key("X"); type("<Esc>")
        XCTAssertEqual(text, "say X world")
        type("0D"); XCTAssertEqual(text, "")
    }

    func testCountsMultiplyAcrossOperatorAndMotion() {
        load("a b c d e f")
        type("2d2w"); XCTAssertEqual(text, "e f")
        load("l1\nl2\nl3\nl4")
        type("3dd"); XCTAssertEqual(text, "l4")
        load("l1\nl2\nl3\nl4", caret: 3)
        type("dj"); XCTAssertEqual(text, "l1\nl4", "a linewise motion deletes whole lines")
        type("kyj"); XCTAssertEqual(tv.vim.unnamedRegister, VimMode.Register(text: "l1\nl4", linewise: true))
        load("l1\nl2\nl3")
        type("2yy"); XCTAssertEqual(tv.vim.unnamedRegister?.text, "l1\nl2\n")
        type("3x"); XCTAssertEqual(text, "\nl2\nl3", "x never crosses the line end")
    }

    func testSimpleCommands() {
        load("abc def")
        type("x"); XCTAssertEqual(text, "bc def")
        type("lX"); XCTAssertEqual(text, "c def")
        type("~"); XCTAssertEqual(text, "C def"); XCTAssertEqual(caret, 1)
        type("rZ"); XCTAssertEqual(text, "CZdef")
        type("s"); XCTAssertEqual(text, "Cdef"); XCTAssertEqual(mode, .insert)
        type("<Esc>")
        load("  first\nsecond\nthird")
        type("J"); XCTAssertEqual(text, "  first second\nthird"); XCTAssertEqual(caret, 7)
        type("S"); XCTAssertEqual(text, "  \nthird", "S keeps the indent"); XCTAssertEqual(mode, .insert)
        type("<Esc>")
        load("keep\nchange me")
        type("jC"); XCTAssertEqual(text, "keep\n"); XCTAssertEqual(mode, .insert)
        type("<Esc>")
        load("a\nb")
        type("Yjp"); XCTAssertEqual(text, "a\nb\na")
        load("x")
        type("o"); XCTAssertEqual(text, "x\n"); XCTAssertEqual(mode, .insert); type("<Esc>")
        type("O"); XCTAssertEqual(text, "x\n\n"); XCTAssertEqual(caret, 2); type("<Esc>")
        load("    deep", caret: 6)
        type("I"); XCTAssertEqual(caret, 4); type("<Esc>")
        type("A"); XCTAssertEqual(caret, 8); type("<Esc>")
    }

    func testTextObjects() {
        load("call(a, (b)) end", caret: 6)
        type("di("); XCTAssertEqual(text, "call() end")
        load("call(a, (b)) end", caret: 6)
        type("da("); XCTAssertEqual(text, "call end")
        load("x [one] y", caret: 4)
        type("ci["); XCTAssertEqual(text, "x [] y"); type("<Esc>")
        load("\\emph{word here}", caret: 8)
        type("yi{"); XCTAssertEqual(tv.vim.unnamedRegister?.text, "word here")
        type("da{"); XCTAssertEqual(text, "\\emph")
        load("say \"quoted text\" now", caret: 7)
        type("di\""); XCTAssertEqual(text, "say \"\" now")
        load("say \"quoted text\" now", caret: 7)
        type("da\""); XCTAssertEqual(text, "say  now")
        load("let $x^2 + y$ be", caret: 6)
        type("di$"); XCTAssertEqual(text, "let $$ be")
        load("let $x^2 + y$ be", caret: 6)
        type("da$"); XCTAssertEqual(text, "let  be")
        load("one two three", caret: 5)
        type("diw"); XCTAssertEqual(text, "one  three")
        load("one two three", caret: 5)
        type("daw"); XCTAssertEqual(text, "one three", "aw takes the trailing space")
        load("one two", caret: 5)
        type("daw"); XCTAssertEqual(text, "one", "no trailing space: the leading one goes")
    }

    func testEnvironmentTextObjects() {
        let doc = "\\begin{itemize}\n  \\item a\n  \\item b\n\\end{itemize}\nafter"
        load(doc, caret: 20)
        type("die"); XCTAssertEqual(text, "\\begin{itemize}\n\\end{itemize}\nafter", "ie is the body lines")
        load(doc, caret: 20)
        type("dae"); XCTAssertEqual(text, "\nafter")
        load(doc, caret: 20)
        type("yie"); XCTAssertEqual(tv.vim.unnamedRegister?.text, "  \\item a\n  \\item b\n")
        load("\\begin{center}x\\end{center}", caret: 14)
        type("cie"); XCTAssertEqual(text, "\\begin{center}\\end{center}"); type("<Esc>")
    }

    func testIndentAndOutdent() {
        load("a\nb\nc")
        let unit = EditorPreferences.shared.indentString
        type(">j"); XCTAssertEqual(text, "\(unit)a\n\(unit)b\nc")
        type("<<"); XCTAssertEqual(text, "a\n\(unit)b\nc")
        type("u"); XCTAssertEqual(text, "\(unit)a\n\(unit)b\nc", ">/< are one undo step each")
    }

    func testRegistersIncludeTheSystemClipboard() {
        load("clip me")
        type("\"+yw")
        XCTAssertEqual(NSPasteboard.general.string(forType: .string), "clip ")
        type("\"ayw")
        load("z")
        type("\"ap"); XCTAssertEqual(text, "zclip ")
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString("PB", forType: .string)
        type("\"*P"); XCTAssertEqual(text, "zclipPB ")
    }

    // MARK: visual mode

    func testVisualAndVisualLine() {
        load("one two three")
        type("vee"); XCTAssertEqual(mode, .visual)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 0, length: 7))
        type("d"); XCTAssertEqual(text, " three"); XCTAssertEqual(mode, .normal)
        load("l1\nl2\nl3")
        type("Vj"); XCTAssertEqual(mode, .visualLine)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 0, length: 6))
        type("y"); XCTAssertEqual(tv.vim.unnamedRegister, VimMode.Register(text: "l1\nl2\n", linewise: true))
        type("Vd"); XCTAssertEqual(text, "l2\nl3")
        load("abc")
        type("vl<Esc>"); XCTAssertEqual(mode, .normal); XCTAssertEqual(caret, 1)
        load("word here", caret: 6)
        type("viw~"); XCTAssertEqual(text, "word HERE")
        load("x", caret: 0)
        tv.setSelectedRange(NSRange(location: 0, length: 1)) // a mouse selection
        XCTAssertEqual(mode, .visual)
        type("<Esc>"); XCTAssertEqual(mode, .normal)
    }

    // MARK: dot repeat and undo granularity

    func testDotRepeatsChangesIncludingInsertedText() {
        load("a b c d")
        type("dw."); XCTAssertEqual(text, "c d")
        load("x y")
        type("iQ<Esc>"); XCTAssertEqual(text, "Qx y")
        type("w."); XCTAssertEqual(text, "Qx Qy")
        load("one two three")
        type("cwONE<Esc>w."); XCTAssertEqual(text, "ONE ONE three")
    }

    func testOneUndoStepPerInsertSession() {
        load("")
        type("i")
        key("a"); key("b"); key("c")
        type("<Esc>")
        XCTAssertEqual(text, "abc")
        type("A")
        key("d"); key("e")
        type("<Esc>")
        XCTAssertEqual(text, "abcde")
        type("u"); XCTAssertEqual(text, "abc", "the second session is one step")
        type("u"); XCTAssertEqual(text, "")
    }

    // MARK: search and command line

    func testSearchForwardBackwardAndStar() {
        load("foo bar foo baz foo")
        type("/foo<CR>"); XCTAssertEqual(caret, 8)
        type("n"); XCTAssertEqual(caret, 16)
        type("n"); XCTAssertEqual(caret, 0, "wraps")
        type("N"); XCTAssertEqual(caret, 16)
        type("?bar<CR>"); XCTAssertEqual(caret, 4)
        type("0w*"); XCTAssertEqual(caret, 4, "* searches the word under the caret")
        XCTAssertEqual(NSPasteboard(name: .find).string(forType: .string), "bar", "shared with the find bar")
        type("/zzz<CR>"); XCTAssertEqual(caret, 4)
        XCTAssertEqual(VimMode.Status.shared.commandLine, "E486: Pattern not found: zzz")
    }

    func testIncrementalSearchPreviewsAndEscRestores() {
        load("abc def abc")
        type("/de")
        XCTAssertEqual(VimMode.Status.shared.commandLine, "/de")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 4, length: 2), "the match is shown as it is typed")
        type("<BS><BS>")
        XCTAssertEqual(caret, 0)
        type("<BS>")
        XCTAssertFalse(tv.vim.isCommandLineActive, "backspace on an empty line cancels")
        type("/abc<Esc>")
        XCTAssertEqual(caret, 0); XCTAssertEqual(mode, .normal)
    }

    func testExCommandsRouteToTheHandlerAndSubstituteEditsTheBuffer() {
        var received: [VimMode.ExCommand] = []
        tv.vim.exCommandHandler = { received.append($0); return nil }
        load("aaa bbb\naaa")
        type(":w<CR>:q<CR>:q!<CR>:wq<CR>:x<CR>:e other.tex<CR>:set nu<CR>:set nonu<CR>")
        XCTAssertEqual(received, [.write, .quit(force: false), .quit(force: true), .writeQuit, .writeQuit, .edit("other.tex"), .setNumber(true), .setNumber(false)])
        XCTAssertEqual(mode, .normal)
        type(":%s/aaa/X/g<CR>"); XCTAssertEqual(text, "X bbb\nX")
        type("u"); XCTAssertEqual(text, "aaa bbb\naaa", "one undo step")
        type("gg:s/a/Y/<CR>"); XCTAssertEqual(text, "Yaa bbb\naaa", "no range: the caret's line, first match only")
        type(":s/a/Y/g<CR>"); XCTAssertEqual(text, "YYY bbb\naaa")
        type(":noh<CR>"); XCTAssertEqual(mode, .normal)
        type(":frobnicate<CR>")
        XCTAssertEqual(VimMode.Status.shared.commandLine, "E492: Not an editor command: frobnicate")
        tv.vim.exCommandHandler = nil
        type(":w<CR>")
        XCTAssertEqual(VimMode.Status.shared.commandLine, "E319: Command not available here")
    }

    // MARK: IME

    func testMarkedTextIsNeverIntercepted() {
        load("")
        type("i")
        tv.setMarkedText("か", selectedRange: NSRange(location: 1, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
        XCTAssertTrue(tv.hasMarkedText())
        type("<Esc>")
        XCTAssertEqual(mode, .insert, "Esc during a composition belongs to the input method")
        tv.insertText("か", replacementRange: NSRange(location: NSNotFound, length: 0))
        XCTAssertFalse(tv.hasMarkedText())
        type("<Esc>")
        XCTAssertEqual(mode, .normal)
        XCTAssertEqual(text, "か")
    }

    func testEscInInsertClosesTheCompletionListWithoutInserting() {
        load("\\sec")
        type("A")
        tv.requestCompletion()
        type("<Esc>")
        XCTAssertEqual(mode, .normal)
        XCTAssertFalse(tv.isCompletionActive)
        XCTAssertEqual(text, "\\sec")
    }

    // MARK: visual-row motions (gj / gk / g0 / g^ / g$)

    /// Forces a narrow, monospaced text container so the sample paragraph
    /// really wraps, then lays it out. Line wrapping is on by default in the
    /// app (`EditorPreferences.lineWrapping`), which is exactly why these
    /// motions matter: without them `j` skips a whole wrapped paragraph.
    private func wrap(at width: CGFloat) {
        tv.font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
        tv.textContainer?.widthTracksTextView = false
        tv.textContainer?.containerSize = NSSize(width: width, height: .greatestFiniteMagnitude)
        if let container = tv.textContainer { tv.layoutManager?.ensureLayout(for: container) }
    }

    /// Every laid-out visual row, as character ranges — the layout's own
    /// answer, which the motions must agree with.
    private func rows() -> [NSRange] {
        guard let lm = tv.layoutManager, let container = tv.textContainer else { return [] }
        lm.ensureLayout(for: container)
        var out: [NSRange] = []
        var glyph = 0
        while glyph < lm.numberOfGlyphs {
            var fragment = NSRange()
            _ = lm.lineFragmentRect(forGlyphAt: glyph, effectiveRange: &fragment)
            out.append(lm.characterRange(forGlyphRange: fragment, actualGlyphRange: nil))
            glyph = NSMaxRange(fragment)
        }
        return out
    }

    private let paragraph = String(repeating: "word ", count: 40).trimmingCharacters(in: .whitespaces)

    func testGjAndGkStepOneVisualRowInsideAWrappedLine() throws {
        load(paragraph + "\nsecond line\n")
        wrap(at: 160)
        let rows = rows()
        try XCTSkipIf(rows.count < 4, "the paragraph did not wrap into enough rows to test")
        XCTAssertEqual(caret, 0)
        type("gj")
        XCTAssertEqual(caret, rows[1].location, "gj lands at the same column of the next visual row")
        type("gj")
        XCTAssertEqual(caret, rows[2].location)
        type("gk")
        XCTAssertEqual(caret, rows[1].location)
        type("gk")
        XCTAssertEqual(caret, 0)
        type("gk")
        XCTAssertEqual(caret, 0, "gk at the first row stays put")
    }

    func testPlainJSkipsTheWholeWrappedParagraphWhileGjDoesNot() throws {
        load(paragraph + "\nsecond line\n")
        wrap(at: 160)
        let rows = rows()
        try XCTSkipIf(rows.count < 3, "the paragraph did not wrap")
        type("j")
        let afterJ = caret
        XCTAssertGreaterThan(afterJ, NSMaxRange(rows[0]), "j leaves the paragraph entirely — the gap these motions close")
        type("gg")
        type("gj")
        XCTAssertLessThan(caret, afterJ, "gj is still inside the first paragraph")
        XCTAssertTrue(NSLocationInRange(caret, rows[1]))
    }

    func testGjKeepsItsColumnAndClampsOnAShortRow() throws {
        load(paragraph + "\nab\ncdefgh\n")
        wrap(at: 160)
        let rows = rows()
        try XCTSkipIf(rows.count < 3, "the paragraph did not wrap")
        type("5l") // column 5 of the first row
        XCTAssertEqual(caret, 5)
        type("gj")
        XCTAssertEqual(caret, rows[1].location + 5, "the column is kept across rows")
        type("gj")
        XCTAssertEqual(caret, rows[2].location + 5)
        // Down onto the two-character line: clamped to its last character, and
        // the remembered column is restored on the row after it.
        let ab = try XCTUnwrap(rows.first { tv.string[Range($0, in: tv.string)!].hasPrefix("ab") })
        let abIndex = try XCTUnwrap(rows.firstIndex(of: ab))
        tv.setSelectedRange(NSRange(location: rows[abIndex - 1].location + 5, length: 0))
        type("gj")
        XCTAssertEqual(caret, ab.location + 1, "clamped to the last character of the short row")
        type("gj")
        XCTAssertEqual(caret, rows[abIndex + 1].location + 5, "the column comes back on a row that is long enough")
    }

    func testJAndGjKeepSeparateColumns() throws {
        load("abcdefghij\nklmnopqrst\nuvwxyz\n")
        wrap(at: 4000) // nothing wraps: every visual row is a logical line
        type("8l")
        XCTAssertEqual(caret, 8)
        type("j")
        XCTAssertEqual(caret, 19, "j keeps column 8 on line 2")
        type("gj")
        XCTAssertEqual(caret, 27, "gj keeps the same column when a line does not wrap")
        // With no wrapping gj is j, which is the point: the user never has to
        // think about which one to press.
        type("gk")
        XCTAssertEqual(caret, 19)
    }

    func testG0AndG6AndGDollarGoToTheEndsOfTheVisualRow() throws {
        load("    " + paragraph)
        wrap(at: 160)
        let rows = rows()
        try XCTSkipIf(rows.count < 3, "the paragraph did not wrap")
        tv.setSelectedRange(NSRange(location: rows[1].location + 3, length: 0))
        type("g$")
        XCTAssertEqual(caret, NSMaxRange(rows[1]) - 1, "g$ is the last character of this row, not of the line")
        type("g0")
        XCTAssertEqual(caret, rows[1].location)
        // `g^` on the first row skips the indent; `g0` does not.
        type("gg")
        type("g0")
        XCTAssertEqual(caret, 0)
        type("g^")
        XCTAssertEqual(caret, 4, "g^ is the first non-blank of the row")
    }

    func testDgjDeletesThroughTheNextVisualRow() throws {
        load(paragraph)
        wrap(at: 160)
        let rows = rows()
        try XCTSkipIf(rows.count < 3, "the paragraph did not wrap")
        let removed = rows[1].location
        type("dgj")
        XCTAssertEqual(text, (paragraph as NSString).substring(from: removed),
                       "dgj is exclusive: it removes up to the same column of the next row")
        XCTAssertEqual(caret, 0)
    }

    func testGjReachesTheEmptyLineAfterATrailingNewline() {
        load("abc\n")
        wrap(at: 4000)
        type("gj")
        XCTAssertEqual(caret, 4, "the empty final line has no glyphs of its own but j reaches it, so gj must too")
        type("gk")
        XCTAssertEqual(caret, 0)
    }

    func testCountedGjMovesThatManyRowsAndStopsAtTheEnd() throws {
        load(paragraph)
        wrap(at: 160)
        let rows = rows()
        try XCTSkipIf(rows.count < 4, "the paragraph did not wrap into enough rows")
        type("3gj")
        XCTAssertEqual(caret, rows[3].location)
        type("99gj")
        XCTAssertEqual(caret, rows.last!.location, "a count past the end stops on the last row instead of refusing")
    }

    func testVisualModeSelectsByVisualRows() throws {
        load(paragraph)
        wrap(at: 160)
        let rows = rows()
        try XCTSkipIf(rows.count < 3, "the paragraph did not wrap")
        type("vgj")
        XCTAssertEqual(mode, .visual)
        XCTAssertEqual(tv.selectedRange().location, 0)
        XCTAssertEqual(NSMaxRange(tv.selectedRange()), rows[1].location + 1)
    }
}
