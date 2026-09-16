import AppKit
import HostedWindows
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
        window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
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
        tv?.vimEnabledOverride = false
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

    // MARK: dispatch policy — normal and visual mode own the keyboard

    /// Keys that legitimately edit the buffer as a single normal-mode
    /// keystroke. `p`/`P` are here because an earlier key in the sweep (`Y`)
    /// fills the unnamed register, after which they paste. Every key *not*
    /// in this set must leave the buffer untouched: before the
    /// consume-unmapped-keys policy, `_`, `-`, `q`, `[`, `#` and every other
    /// unmapped printable key fell through to the editor and typed itself
    /// into the document.
    private static let normalModeEditingKeys: Set<Character> = ["x", "X", "s", "S", "D", "C", "o", "O", "J", "~", "p", "P"]

    func testNormalModeConsumesEveryUnmappedPrintableKey() {
        let buffer = "alpha beta(gamma) {delta}\n  second line\nthird"
        for v in UInt8(32)...UInt8(126) {
            let ch = Character(UnicodeScalar(v))
            if Self.normalModeEditingKeys.contains(ch) { continue }
            load(buffer, caret: 8)
            key(String(ch))
            XCTAssertEqual(text, buffer, "normal-mode '\(ch)' must not edit the buffer")
            type("<Esc>") // clear any pending operator / count / command line the key armed
        }
    }

    /// Same sweep over a visual selection, where a fallthrough is worse:
    /// the typed character *replaces the whole selection* (`u`, `r`, `U` and
    /// every other unmapped key did exactly that).
    ///
    /// This allowlist grows as visual commands are implemented — it is the
    /// register of keys that *may* edit, not a licence for them to do
    /// anything: what each one actually does is pinned by its own row in the
    /// table-driven tests below (`vjD`, `vX`, `vC`, `veU`, `ver-`, …). The
    /// sweep's job is only to catch a key editing the buffer when nothing
    /// implements it.
    private static let visualModeEditingKeys: Set<Character> = [
        "d", "x", "c", "s", "J", "~", ">", "<", "p", "P",
        "u", "U", "D", "X", "C", "S", "R", // case + linewise commands (this PR)
    ]

    func testVisualModeConsumesEveryUnmappedKeyInsteadOfReplacingTheSelection() {
        let buffer = "alpha beta gamma\nsecond line here\n"
        for v in UInt8(32)...UInt8(126) {
            let ch = Character(UnicodeScalar(v))
            if Self.visualModeEditingKeys.contains(ch) { continue }
            load(buffer, caret: 0)
            type("ve") // "alpha" selected
            key(String(ch))
            XCTAssertEqual(text, buffer, "visual-mode '\(ch)' must not edit the buffer")
            type("<Esc><Esc>")
        }
    }

    /// Unmapped ⌃-chords must never run the editor's Cocoa bindings from
    /// normal or visual mode (⌃K killed the line, ⌃O opened one, ⌃T
    /// transposed, ⌃H deleted, ⌃W deleted a word, ⌃Y yanked the kill buffer).
    func testControlChordsNeverReachTheEditorsCocoaBindings() {
        let buffer = "one two three\nfour five six\nseven eight\n"
        for v in UInt8(ascii: "a")...UInt8(ascii: "z") {
            let ch = String(UnicodeScalar(v))
            load(buffer, caret: 4)
            key(ch, flags: .control)
            XCTAssertEqual(text, buffer, "normal-mode ⌃\(ch) must not edit the buffer")
            type("<Esc>")
            load(buffer, caret: 0)
            type("ve")
            key(ch, flags: .control)
            XCTAssertEqual(text, buffer, "visual-mode ⌃\(ch) must not edit the buffer")
            type("<Esc><Esc>")
        }
    }

    /// Enter inserted a newline, Backspace deleted a character and Tab
    /// inserted an indent — all from normal mode. Until they gain their Vim
    /// motions they are consumed no-ops.
    func testEnterBackspaceAndTabAreNoOpsInNormalAndVisualMode() {
        let buffer = "first line\nsecond line\n"
        for (chars, code) in [("\r", UInt16(36)), ("\u{7F}", UInt16(51)), ("\t", UInt16(48))] {
            load(buffer, caret: 3)
            key(chars, code: code)
            XCTAssertEqual(text, buffer, "normal-mode key code \(code) must not edit the buffer")
            load(buffer, caret: 0)
            type("ve")
            key(chars, code: code)
            XCTAssertEqual(text, buffer, "visual-mode key code \(code) must not edit the buffer")
            type("<Esc>")
        }
    }

    /// The Tab consume is normal/visual-mode only: insert mode still hands
    /// Tab to the editor (indentation, snippet placeholders, completion).
    func testTabStillReachesTheEditorInInsertMode() {
        load("ab")
        type("i")
        key("\t", code: 48)
        XCTAssertNotEqual(text, "ab", "insert-mode Tab must keep taking the editor's path")
    }

    // MARK: shared status-line lifecycle

    /// The status line is a singleton (`VimMode.Status.shared`) but the truth
    /// behind it is per-view. Turning the preference off used to reach
    /// `deactivate()` only through an observation held weakly by a live
    /// editor's coordinator, so switching Vim off from the menu or the
    /// command palette with no editor alive left the singleton at
    /// `-- NORMAL --` — and the next editor opened showed a phantom status
    /// row with Vim off.
    func testPreferenceOffClearsTheStatusLineWithNoLiveEditor() {
        let was = EditorPreferences.shared.vimKeybindings
        defer { EditorPreferences.shared.vimKeybindings = was }
        tv.vimEnabledOverride = nil // this view follows the preference, like the app's
        EditorPreferences.shared.vimKeybindings = true
        XCTAssertEqual(VimMode.Status.shared.indicator, "-- NORMAL --")

        // Every editor goes away, then the preference is switched off with
        // nothing left observing it.
        window.contentView?.subviews.forEach { $0.removeFromSuperview() }
        tv = nil
        EditorPreferences.shared.vimKeybindings = false
        XCTAssertNil(VimMode.Status.shared.indicator, "no editor may leave a phantom status row behind")
        XCTAssertNil(VimMode.Status.shared.commandLine)
    }

    /// The clear must be driven by the preference *transition*, not by a
    /// render gate on the preference: `vimEnabledOverride` pins Vim per-view
    /// independently of it, and that must keep showing its status line.
    func testPerViewOverrideKeepsItsStatusLineWhenThePreferenceGoesOff() {
        let was = EditorPreferences.shared.vimKeybindings
        defer { EditorPreferences.shared.vimKeybindings = was }
        EditorPreferences.shared.vimKeybindings = true
        tv.vimEnabledOverride = true // pinned on, regardless of the preference
        load("abc")
        XCTAssertEqual(VimMode.Status.shared.indicator, "-- NORMAL --")
        EditorPreferences.shared.vimKeybindings = false
        XCTAssertEqual(VimMode.Status.shared.indicator, "-- NORMAL --", "a pinned view still owns the status line")
        type("i")
        XCTAssertEqual(VimMode.Status.shared.indicator, "-- INSERT --", "and keeps driving it")
        type("<Esc>")
    }

    /// Turning the preference off is synchronous — the old path scheduled a
    /// `Task { @MainActor }` that lost the race against coordinator teardown
    /// on a loaded machine, which is what made this flaky in CI rather than
    /// always broken.
    func testPreferenceOffClearsTheStatusLineWithoutWaitingForATask() {
        let was = EditorPreferences.shared.vimKeybindings
        defer { EditorPreferences.shared.vimKeybindings = was }
        tv.vimEnabledOverride = nil
        EditorPreferences.shared.vimKeybindings = true
        XCTAssertNotNil(VimMode.Status.shared.indicator)
        EditorPreferences.shared.vimKeybindings = false
        XCTAssertNil(VimMode.Status.shared.indicator, "cleared on the setter, not on a later run-loop turn")
    }

    // MARK: table-driven rows — buffer + caret + keys → buffer + caret

    private struct VimRow {
        var keys: String
        var before: String
        var caret: Int
        var after: String
        var caretAfter: Int
    }

    /// One row per action so coverage is visible and a regression names
    /// itself (the failing row's keys are in the assertion message).
    private func run(_ rows: [VimRow]) {
        for r in rows {
            load(r.before, caret: r.caret)
            type(r.keys)
            XCTAssertEqual(text, r.after, "\(r.keys): buffer")
            XCTAssertEqual(caret, r.caretAfter, "\(r.keys): caret")
            type("<Esc>")
        }
    }

    // MARK: linewise first-non-blank motions (- + _ Enter |) and g_ / ge / gE / #

    func testLinewiseFirstNonBlankMotions() {
        let b = "  one\n  two\n  three" // line starts 0 / 6 / 12; first non-blanks 2 / 8 / 14
        run([
            VimRow(keys: "-", before: b, caret: 8, after: b, caretAfter: 2),
            VimRow(keys: "+", before: b, caret: 8, after: b, caretAfter: 14),
            VimRow(keys: "<CR>", before: b, caret: 2, after: b, caretAfter: 8),
            VimRow(keys: "2+", before: b, caret: 2, after: b, caretAfter: 14),
            VimRow(keys: "_", before: b, caret: 4, after: b, caretAfter: 2),
            VimRow(keys: "2_", before: b, caret: 2, after: b, caretAfter: 8),
            VimRow(keys: "-", before: b, caret: 2, after: b, caretAfter: 2), // first line: nowhere to go
            VimRow(keys: "+", before: b, caret: 14, after: b, caretAfter: 14), // last line: nowhere to go
        ])
    }

    func testBackspaceColumnAndLastNonBlankMotions() {
        run([
            VimRow(keys: "<BS>", before: "  one\n  two", caret: 3, after: "  one\n  two", caretAfter: 2),
            VimRow(keys: "3<BS>", before: "  one\n  two", caret: 4, after: "  one\n  two", caretAfter: 1),
            VimRow(keys: "|", before: "  one\n  two", caret: 8, after: "  one\n  two", caretAfter: 6),
            VimRow(keys: "4|", before: "  one\n  two", caret: 6, after: "  one\n  two", caretAfter: 9),
            VimRow(keys: "99|", before: "  one\n  two", caret: 6, after: "  one\n  two", caretAfter: 10), // clamps to the line's last character
            VimRow(keys: "g_", before: "one  \ntwo", caret: 0, after: "one  \ntwo", caretAfter: 2), // trailing blanks skipped
        ])
    }

    func testWordEndBackMotions() {
        let b = "one two three"
        run([
            VimRow(keys: "ge", before: b, caret: 8, after: b, caretAfter: 6),
            VimRow(keys: "ge", before: b, caret: 6, after: b, caretAfter: 2),
            VimRow(keys: "ge", before: b, caret: 10, after: b, caretAfter: 6), // from inside a word
            VimRow(keys: "ge", before: "foo( bar", caret: 5, after: "foo( bar", caretAfter: 3), // punctuation run has its own end
            VimRow(keys: "gE", before: "foo( bar", caret: 5, after: "foo( bar", caretAfter: 3), // big words: ( ends "foo("
        ])
    }

    func testNewMotionsAsOperatorTargets() {
        run([
            VimRow(keys: "dge", before: "one two three", caret: 8, after: "one twhree", caretAfter: 6), // inclusive, backward
            VimRow(keys: "d<CR>", before: "  one\n  two\n  three", caret: 2, after: "  three", caretAfter: 2), // linewise: two lines
            VimRow(keys: "d-", before: "  one\n  two\n  three", caret: 14, after: "  one", caretAfter: 2), // linewise: this line and the one above
            VimRow(keys: "d_", before: "aa\nbb\ncc", caret: 4, after: "aa\ncc", caretAfter: 3), // one whole line, like dd
            VimRow(keys: "2d_", before: "aa\nbb\ncc", caret: 3, after: "aa", caretAfter: 0),
            VimRow(keys: "d|", before: "abcdef", caret: 3, after: "def", caretAfter: 0), // exclusive, back to column 1
            VimRow(keys: "dg_", before: "one  ", caret: 0, after: "  ", caretAfter: 0), // inclusive to the last non-blank
        ])
    }

    func testHashSearchesTheWordUnderTheCaretBackwards() {
        let b = "foo bar foo baz foo"
        run([
            VimRow(keys: "#", before: b, caret: 8, after: b, caretAfter: 0),
            VimRow(keys: "*", before: b, caret: 8, after: b, caretAfter: 16),
            VimRow(keys: "#", before: b, caret: 0, after: b, caretAfter: 16), // wraps backwards
        ])
        // n continues in the # direction (backwards).
        load(b, caret: 16)
        type("#")
        XCTAssertEqual(caret, 8)
        type("n")
        XCTAssertEqual(caret, 0)
    }

    func testScreenLineMotionsTakeCounts() {
        load("l1\nl2\nl3\nl4\nl5\nl6", caret: 7) // 6 short lines, all visible
        type("H")
        XCTAssertEqual(caret, 0)
        type("2H")
        XCTAssertEqual(caret, 3)
        type("L")
        XCTAssertEqual(caret, 15)
        type("2L")
        XCTAssertEqual(caret, 12)
    }

    // MARK: correctness fixes pulled forward from the audit

    func testNamedRegisterSelectionDoesNotLeakIntoTheNextCommand() {
        load("one\ntwo\n")
        type("\"ayy") // register a: "one\n"
        type("j")
        type("yy") // unnamed yank — must not overwrite register a
        type("\"ap") // paste register a below "two"
        XCTAssertEqual(text, "one\ntwo\none\n")
        // …and the register selection is spent: a plain p pastes the unnamed register ("two\n").
        type("p")
        XCTAssertEqual(text, "one\ntwo\none\ntwo\n")
    }

    func testGvRestoresTheLastVisualSelection() {
        load("alpha beta gamma")
        type("ve")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 0, length: 5))
        type("<Esc>w")
        XCTAssertEqual(mode, .normal)
        type("gv")
        XCTAssertEqual(mode, .visual)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 0, length: 5))
        // Operators act on the restored selection.
        type("d")
        XCTAssertEqual(text, " beta gamma")
    }

    func testGvWithoutAPriorSelectionAnchorsAtTheCaret() {
        load("alpha beta", caret: 6)
        type("gv")
        XCTAssertEqual(mode, .visual)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 6, length: 1))
        type("e") // extending moves the head, proving the anchor is the caret, not stale state
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 6, length: 4))
    }

    func testDotRepeatsALinewiseEnterDelete() {
        load("l1\nl2\nl3\nl4\nl5\nl6")
        type("d<CR>")
        XCTAssertEqual(text, "l3\nl4\nl5\nl6")
        type(".")
        XCTAssertEqual(text, "l5\nl6")
    }

    // MARK: registers — black hole, numbered chain, append

    /// `"_` discards the text entirely: unlike every other register, it
    /// never touches the unnamed register either, so a prior yank still
    /// pastes back afterwards.
    func testBlackHoleRegisterNeverTouchesTheUnnamedRegister() {
        load("one\ntwo\nthree")
        type("yy") // unnamed + "0" = "one\n"
        type("j\"_dd") // discard "two" into the void
        XCTAssertEqual(text, "one\nthree", "\"_dd still deletes")
        type("p")
        XCTAssertEqual(text, "one\nthree\none", "unnamed register still holds the yank, not the black-holed delete")
    }

    /// `"_` also refuses to *paste* anything (there is nothing in it).
    func testBlackHoleRegisterPastesNothing() {
        load("abc")
        type("\"_p")
        XCTAssertEqual(text, "abc", "black hole has nothing to paste")
    }

    /// A yank without an explicit register also fills `"0`, which a
    /// following delete does not clobber (deletes never touch `"0`).
    func testYankRegisterZeroSurvivesAnInterveningDelete() {
        load("one\ntwo")
        type("yy") // "0 = "one\n"
        type("jdd") // deletes "two"; "0 must still be "one\n"
        type("\"0p")
        XCTAssertEqual(text, "one\none", "\"0p pastes the last yank, unaffected by the delete")
    }

    /// Whole-line (or multi-line) deletes shift into the numbered registers
    /// `"1`…`"9`, oldest first, most recent always in `"1`.
    func testNumberedRegistersShiftOnLinewiseDeletes() {
        load("a\nb\nc\nd")
        type("dd") // "1 = a
        type("dd") // "1 = b, "2 = a
        type("dd") // "1 = c, "2 = b, "3 = a
        XCTAssertEqual(text, "d")
        type("\"3p")
        XCTAssertEqual(text, "d\na", "\"3 holds the oldest of the three deletes")
        type("u\"1p")
        XCTAssertEqual(text, "d\nc", "\"1 still holds the most recent delete after undoing the \"3 paste")
    }

    /// A delete that stays within one line (too small for a numbered slot)
    /// goes to `"-`, and never disturbs `"1`.
    func testSmallDeleteGoesToTheDashRegister() {
        load("abc")
        type("x") // "-" = "a"; "1"/"0" untouched
        XCTAssertEqual(text, "bc")
        type("\"-p")
        XCTAssertEqual(text, "bac")
    }

    /// `"A` appends to `"a` (with the combined text landing in both slots,
    /// and readable through either name); `"a` alone overwrites as before.
    func testUppercaseRegisterAppendsInsteadOfOverwriting() {
        load("alpha beta")
        type("\"ayiw") // "a" = "alpha"
        type("w\"Ayiw") // "A" appends -> "a" = "alphabeta" (word-wise, no separator)
        load("-")
        type("\"ap")
        XCTAssertEqual(text, "-alphabeta", "\"A appended onto \"a instead of replacing it")
        type("\"Ap")
        XCTAssertEqual(text, "-alphabetaalphabeta", "\"A also reads the same slot as \"a")
    }

    /// Appending a linewise yank onto a linewise register joins with a
    /// newline rather than concatenating mid-line.
    func testUppercaseRegisterAppendsLinewiseWithANewlineJoin() {
        load("one\ntwo\nthree")
        type("\"ayy") // "a" = "one\n"
        type("j\"Ayy") // append "two\n" -> "a" = "one\ntwo\n"
        load("x")
        type("\"ap")
        XCTAssertEqual(text, "x\none\ntwo", "the appended register pastes as two whole lines")
    }

    // MARK: case operators gu / gU / g~

    func testCaseOperatorsWithMotionsAndDoubledForms() {
        run([
            VimRow(keys: "guw", before: "HELLO World", caret: 0, after: "hello World", caretAfter: 0),
            VimRow(keys: "gUw", before: "hello world", caret: 0, after: "HELLO world", caretAfter: 0),
            VimRow(keys: "g~w", before: "Hello", caret: 0, after: "hELLO", caretAfter: 0),
            VimRow(keys: "gu$", before: "ABC DEF", caret: 4, after: "ABC def", caretAfter: 4),
            VimRow(keys: "guu", before: "ABC Def\nGHI", caret: 2, after: "abc def\nGHI", caretAfter: 0),
            VimRow(keys: "gugu", before: "ABC Def\nGHI", caret: 2, after: "abc def\nGHI", caretAfter: 0),
            VimRow(keys: "2gUU", before: "ab\ncd\nef", caret: 0, after: "AB\nCD\nef", caretAfter: 0),
            VimRow(keys: "g~~", before: "aBc", caret: 1, after: "AbC", caretAfter: 0),
        ])
    }

    func testCaseOperatorsInVisualModeAndDotRepeat() {
        run([
            VimRow(keys: "vegu", before: "ABC DEF", caret: 0, after: "abc DEF", caretAfter: 0),
            VimRow(keys: "veU", before: "abc def", caret: 0, after: "ABC def", caretAfter: 0),
            VimRow(keys: "vju", before: "AB\nCD", caret: 0, after: "ab\ncD", caretAfter: 0), // charwise through 'C'
            VimRow(keys: "VjU", before: "ab\ncd", caret: 0, after: "AB\nCD", caretAfter: 0), // linewise: both lines
        ])
        load("AAA BBB")
        type("guw")
        XCTAssertEqual(text, "aaa BBB")
        type("w.")
        XCTAssertEqual(text, "aaa bbb", "`.` repeats guw at the new position")
    }

    // MARK: visual-mode commands that previously fell through

    func testVisualReplaceEachSelectedCharacter() {
        run([
            VimRow(keys: "ver-", before: "abc def", caret: 0, after: "--- def", caretAfter: 0),
            VimRow(keys: "Vjrx", before: "ab\ncd", caret: 0, after: "xx\nxx", caretAfter: 0), // newline survives
        ])
    }

    func testVisualLinewiseDeleteChangeAndYank() {
        run([
            VimRow(keys: "vjD", before: "one\ntwo\nthree", caret: 0, after: "three", caretAfter: 0),
            VimRow(keys: "vX", before: "one\ntwo", caret: 5, after: "one", caretAfter: 0),
            VimRow(keys: "vYp", before: "one\ntwo", caret: 0, after: "one\none\ntwo", caretAfter: 4), // Y is linewise
        ])
        load("  one\n  two", caret: 8)
        type("vC")
        XCTAssertEqual(mode, .insert)
        XCTAssertEqual(text, "  one\n  ", "linewise change keeps the indent")
        type("x")
        key("\u{1B}", code: 53)
        XCTAssertEqual(text, "  one\n  x")
    }

    func testVisualOSwapsTheSelectionCorners() {
        load("abcdef")
        type("vll")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 0, length: 3))
        type("O")
        XCTAssertEqual(mode, .visual)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 0, length: 3), "O keeps the region, moving the head")
        type("l") // the head is now the LEFT end, so l shrinks from the left
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 1, length: 2))
        type("<Esc>")
    }

    // MARK: ip / ap paragraph text objects

    func testParagraphTextObjects() {
        let b = "aaa\nbbb\n\nccc\nddd\n\neee"
        run([
            VimRow(keys: "dip", before: b, caret: 5, after: "\nccc\nddd\n\neee", caretAfter: 0),
            VimRow(keys: "dap", before: b, caret: 5, after: "ccc\nddd\n\neee", caretAfter: 0), // trailing blank line included
            VimRow(keys: "dip", before: "aaa\n\n\nbbb", caret: 4, after: "aaa\nbbb", caretAfter: 4), // a blank run is its own paragraph
            VimRow(keys: "dap", before: "aaa\nbbb\n\n\n", caret: 0, after: "", caretAfter: 0), // paragraph and every trailing blank line
            VimRow(keys: "yipP", before: b, caret: 9, after: "aaa\nbbb\n\nccc\nddd\nccc\nddd\n\neee", caretAfter: 9),
        ])
        // Register is linewise: p pastes on the next line.
        load(b, caret: 9)
        type("yipGp")
        XCTAssertTrue(text.hasSuffix("eee\nccc\nddd"), "yip yanks whole lines, so p pastes below; got \(text)")
    }

    // MARK: ex line jumps

    func testExLineNumberJumps() {
        let b = "  l1\nl2\nl3\n  l4\nl5"
        load(b, caret: 0)
        type(":3<CR>")
        XCTAssertEqual(caret, 8)
        type(":1<CR>")
        XCTAssertEqual(caret, 2)
        type(":$<CR>")
        XCTAssertEqual(caret, 16)
        type(":99<CR>")
        XCTAssertEqual(caret, 16, "past the end clamps to the last line")
    }

    // MARK: zt / zb scrolling

    func testZtAndZbScrollTheCaretLineToTheEdges() throws {
        load((0..<200).map { "line \($0)" }.joined(separator: "\n"))
        let lm = try XCTUnwrap(tv.layoutManager)
        lm.ensureLayout(for: try XCTUnwrap(tv.textContainer)) // bounds/height must be final before scrolling
        type("100G")
        let glyph = lm.glyphIndexForCharacter(at: tv.selectedRange().location)
        let rect = lm.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
        let inset = tv.textContainerInset.height
        type("zt")
        let top = try XCTUnwrap(tv.enclosingScrollView).documentVisibleRect
        XCTAssertEqual(top.minY, rect.minY + inset, accuracy: rect.height * 1.5, "zt puts the caret line at the top edge")
        type("zb")
        let bottom = try XCTUnwrap(tv.enclosingScrollView).documentVisibleRect
        XCTAssertEqual(bottom.maxY, rect.maxY + inset, accuracy: rect.height * 1.5, "zb puts the caret line at the bottom edge")
    }

    // MARK: insert-mode chords ⌃W ⌃U ⌃R ⌃T ⌃O

    func testControlWDeletesTheWordBeforeTheCaret() {
        load("")
        type("i")
        type("hello world")
        type("<C-w>")
        XCTAssertEqual(text, "hello ")
        XCTAssertEqual(caret, 6)
        XCTAssertEqual(mode, .insert)
        type("<C-w>") // a trailing space alone still counts as "before the word"
        XCTAssertEqual(text, "")
    }

    func testControlUDeletesToInsertSessionStartOrFirstNonBlank() {
        load("")
        type("i")
        type("indent text")
        type("<C-u>")
        XCTAssertEqual(text, "", "deletes everything typed this session")
        XCTAssertEqual(caret, 0)
        type("<Esc>") // back to normal mode: the next `i` must start a fresh session, not type a literal "i"

        load("    abcdef", caret: 10)
        type("i") // nothing typed yet this session
        type("<C-u>")
        XCTAssertEqual(text, "    ", "with nothing typed, ⌃U falls back to the line's first non-blank")
        XCTAssertEqual(caret, 4)
    }

    func testControlTIndentsTheCurrentLineFromInsertMode() {
        let unit = EditorPreferences.shared.indentString
        load("abc", caret: 1)
        type("i")
        type("<C-t>")
        XCTAssertEqual(text, unit + "abc")
        XCTAssertEqual(caret, 1 + (unit as NSString).length)
        XCTAssertEqual(mode, .insert)
    }

    func testControlRInsertsARegistersContentsAndKeepsTyping() {
        load("word")
        type("\"ayiw")
        load("X")
        type("i")
        type("<C-r>a")
        XCTAssertEqual(text, "wordX")
        XCTAssertEqual(caret, 4)
        XCTAssertEqual(mode, .insert)
        type("!")
        XCTAssertEqual(text, "word!X")
    }

    func testControlOEntersOneShotNormalModeThenReturnsToInsert() {
        load("hello", caret: 5)
        type("i")
        type("<C-o>0")
        XCTAssertEqual(mode, .insert, "a complete one-key motion returns to insert")
        XCTAssertEqual(caret, 0)
        type("X")
        XCTAssertEqual(text, "Xhello")
    }

    func testControlOStaysInNormalModeMidOperatorThenReturnsAfterTheMotion() {
        load("keep drop here", caret: 0)
        type("A") // caret at end of line, insert mode
        type("<C-o>")
        XCTAssertEqual(mode, .normal)
        type("b") // a complete one-key motion returns to insert immediately
        XCTAssertEqual(mode, .insert)
        XCTAssertEqual(caret, 10)
        type("<C-o>")
        XCTAssertEqual(mode, .normal)
        type("d")
        XCTAssertEqual(mode, .normal, "mid-operator: must not return to insert before its motion")
        type("w")
        XCTAssertEqual(text, "keep drop ", "dw deleted \"here\" as the one-shot command")
        XCTAssertEqual(mode, .insert, "the operator+motion pair completed, so ⌃O returns to insert")
        // Known simplification: real Vim remembers that the caret was past
        // the last character (end of line) before the one-shot command and
        // restores that on return; this implementation applies the ordinary
        // normal-mode clamp (caret sits ON the last character, not after
        // it), so the caret lands one column short of true end-of-line here.
        type("!")
        XCTAssertEqual(text, "keep drop! ")
    }
}
