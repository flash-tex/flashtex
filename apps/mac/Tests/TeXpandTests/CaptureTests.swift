import XCTest
@testable import FlashTeXEditorCore

/// PLAN M3 acceptance, headless: the capture FSM driven by simulated event
/// streams, covering every §9.1 transition, plus the scope provider.
final class CaptureTests: XCTestCase {
    typealias T = TeXpand

    /// A minimal editor: text, a caret, and the controller fed exactly as
    /// the Mac text view feeds it (an edit, then the caret move after it).
    final class Editor {
        let text = NSMutableString()
        var caret = 0
        let scopes = T.ScopeProvider()
        let controller: T.CaptureController
        var last = T.CaptureController.Output()
        var undoStack: [(range: NSRange, old: String)] = []

        init(_ initial: String = "", settings: T.Settings = Editor.on, documentClass: String? = nil) {
            let scopes = self.scopes
            controller = T.CaptureController(engine: T.Engine(settings: settings), scope: { scopes.scope(at: $0, in: $1) })
            controller.documentClass = { documentClass }
            text.setString(initial)
            caret = text.length
        }

        static var on: T.Settings { var s = T.Settings(); s.enabled = true; return s }

        var state: T.CaptureController.State { controller.state }

        func edit(_ range: NSRange, _ replacement: String, kind: T.CaptureController.EditKind, caret newCaret: Int? = nil) {
            let old = text.substring(with: range)
            text.replaceCharacters(in: range, with: replacement)
            scopes.noteEdit(range: range, replacementLength: replacement.utf16.count)
            if kind != .undo { undoStack.append((NSRange(location: range.location, length: replacement.utf16.count), old)) }
            last = controller.edited(range, replacement: replacement, kind: kind, text: text)
            caret = newCaret ?? range.location + replacement.utf16.count
            let moved = controller.cursorMoved(to: NSRange(location: caret, length: 0), text: text)
            if moved != T.CaptureController.Output() || controller.state == .idle { last = moved }
        }

        func type(_ s: String) {
            for ch in s { edit(NSRange(location: caret, length: 0), String(ch), kind: .typed) }
        }

        func backspace() { edit(NSRange(location: caret - 1, length: 1), "", kind: .typed) }

        func move(to c: Int) {
            caret = c
            last = controller.cursorMoved(to: NSRange(location: c, length: 0), text: text)
        }

        /// Tab as the host runs it: a commit is applied as one undo step and
        /// the caret goes to the snippet's caret; an unconsumed Tab inserts `\t`.
        @discardableResult
        func tab() -> T.CaptureController.Output {
            let out = controller.tab(selection: NSRange(location: caret, length: 0), text: text)
            last = out
            if let c = out.commit {
                edit(c.range, c.snippet.text, kind: .programmatic, caret: c.range.location + c.snippet.caretUTF16)
            } else if !out.consumed {
                edit(NSRange(location: caret, length: 0), "\t", kind: .typed)
            }
            return out
        }

        func esc() -> T.CaptureController.Output {
            last = controller.escape()
            return last
        }

        func undo() {
            guard let u = undoStack.popLast() else { return }
            edit(u.range, u.old, kind: .undo, caret: u.range.location + u.old.utf16.count)
        }

        var string: String { text as String }
    }

    // MARK: §9.1 transitions

    func testIdleToArmedToCapturingAndCommit() {
        let e = Editor()
        e.type(";")
        XCTAssertEqual(e.state, .armed(leader: 0), "Idle → Armed: the leader is typed")
        e.type("e")
        XCTAssertEqual(e.state, .capturing(leader: 0, end: 2), "Armed → Capturing: a letter")
        e.type("num3")
        XCTAssertEqual(e.last.region, NSRange(location: 0, length: 6), "the region is highlighted")
        XCTAssertEqual(e.last.preview, "\\begin{enumerate}\n  \\item \n  \\item \n  \\item \n\\end{enumerate}", "Ok: a preview")
        let out = e.tab()
        XCTAssertTrue(out.consumed)
        XCTAssertEqual(e.string, "\\begin{enumerate}\n  \\item \n  \\item \n  \\item \n\\end{enumerate}", "Tab with Ok: commit")
        XCTAssertEqual(e.caret, 26, "the caret at the first \\item's field")
        XCTAssertEqual(e.state, .idle)
        XCTAssertEqual(out.commit?.snippet.stops.count, 3, "two more items, then the end")
    }

    func testArmedReturnsToIdleOnAnythingElse() {
        let e = Editor("x")
        e.type("; ")
        XCTAssertEqual(e.state, .idle, "Armed → Idle: a non-letter; the leader stays literal")
        XCTAssertEqual(e.string, "x; ")
        let f = Editor()
        f.type(";")
        f.move(to: 0)
        XCTAssertEqual(f.state, .idle, "Armed → Idle: the caret moves")
        let g = Editor()
        g.type(";")
        g.tab()
        XCTAssertEqual(g.string, ";\t", "Armed → Idle: Tab passes through")
        let h = Editor()
        h.type(";;")
        XCTAssertEqual(h.state, .armed(leader: 1), "a second leader arms at its own position")
        let i = Editor()
        i.type(";3")
        XCTAssertEqual(i.state, .idle)
    }

    func testNoArmingInVerbatimOrComments() {
        let e = Editor("% note ")
        e.type(";")
        XCTAssertEqual(e.state, .idle, "comment")
        let v = Editor("\\begin{verbatim}\n")
        v.type(";e")
        XCTAssertEqual(v.state, .idle, "verbatim")
        let inline = Editor("\\verb|a")
        inline.type(";")
        XCTAssertEqual(inline.state, .idle, "\\verb")
        let after = Editor("\\verb|a| ")
        after.type(";")
        XCTAssertEqual(after.state, .armed(leader: 9), "after \\verb closes")
    }

    func testPrefixGuardCancelsSilently() {
        let e = Editor()
        e.type(";enumx")
        XCTAssertEqual(e.state, .idle, "no registered name starts with `enumx`")
        XCTAssertNil(e.last.region)
        let item = Editor()
        item.type(";ite")
        XCTAssertEqual(item.state, .capturing(leader: 0, end: 4), "`items` exists in text")
        item.type("m")
        XCTAssertEqual(item.state, .capturing(leader: 0, end: 5), "`item` is a prefix of `items`")
        item.type("z")
        XCTAssertEqual(item.state, .idle)
        let list = Editor("\\begin{itemize}\n")
        list.type(";itemq")
        XCTAssertEqual(list.state, .idle)
    }

    func testInvalidCancelsAndIncompleteKeepsCapturing() {
        let e = Editor()
        e.type(";sec{Intro")
        XCTAssertEqual(e.state, .capturing(leader: 0, end: 10), "Incomplete: keep capturing")
        XCTAssertNil(e.last.preview, "no preview while incomplete")
        let out = e.tab()
        XCTAssertTrue(out.consumed, "Tab with Incomplete: no Tab character")
        XCTAssertEqual(out.diagnostic, "expected `}`")
        XCTAssertEqual(e.string, ";sec{Intro")
        e.type(" Part}")
        XCTAssertEqual(e.last.preview, "\\section{Intro Part}", "whitespace inside braces is fine")
        e.type(")")
        XCTAssertEqual(e.state, .idle, "Invalid: cancel silently")
        XCTAssertEqual(e.string, ";sec{Intro Part})")
    }

    func testWhitespaceNewlineAndCaretLeavingCancel() {
        let e = Editor()
        e.type(";enum ")
        XCTAssertEqual(e.state, .idle, "whitespace outside brackets")
        let n = Editor()
        n.type(";enum")
        n.edit(NSRange(location: n.caret, length: 0), "\n", kind: .typed)
        XCTAssertEqual(n.state, .idle, "newline")
        let m = Editor("abc ")
        m.type(";enum")
        m.move(to: 1)
        XCTAssertEqual(m.state, .idle, "the caret leaves the region")
        let inside = Editor()
        inside.type(";enum3")
        inside.move(to: 3)
        XCTAssertEqual(inside.state, .capturing(leader: 0, end: 6), "moving inside the region keeps capturing")
        inside.tab()
        XCTAssertTrue(inside.string.hasPrefix("\\begin{enumerate}"), "Tab commits the whole region")
        let f = Editor()
        f.type(";enum")
        _ = f.controller.focusLost()
        XCTAssertEqual(f.state, .idle, "focus loss")
    }

    func testBackspaceAndEditsBeforeTheRegion() {
        let e = Editor("xy ")
        e.type(";enum3")
        e.backspace()
        XCTAssertEqual(e.state, .capturing(leader: 3, end: 8))
        e.type("2")
        e.edit(NSRange(location: 0, length: 0), "AB", kind: .programmatic, caret: 10)
        XCTAssertEqual(e.state, .capturing(leader: 5, end: 11), "an edit before the leader shifts the region")
        e.backspace(); e.backspace(); e.backspace(); e.backspace(); e.backspace()
        XCTAssertEqual(e.state, .idle, "deleting the leader ends capture")
    }

    func testAutoClosedBracesStayInTheRegion() {
        // A host that auto-closes `{` inserts `{}` and leaves the caret between.
        let e = Editor()
        e.type(";sec")
        e.edit(NSRange(location: e.caret, length: 0), "{}", kind: .typed, caret: e.caret + 1)
        XCTAssertEqual(e.state, .capturing(leader: 0, end: 6))
        e.type("Intro")
        XCTAssertEqual(e.last.preview, "\\section{Intro}")
        e.move(to: e.caret + 1) // type-over steps across the `}`
        XCTAssertEqual(e.state, .capturing(leader: 0, end: 11))
        e.tab()
        XCTAssertEqual(e.string, "\\section{Intro}")
    }

    func testEscLeavesLiteralAndSuppresses() {
        let e = Editor()
        e.type(";enum3")
        XCTAssertTrue(e.esc().consumed)
        XCTAssertEqual(e.state, .idle)
        XCTAssertEqual(e.string, ";enum3")
        XCTAssertEqual(e.controller.suppressed, [NSRange(location: 0, length: 6)])
        e.tab()
        XCTAssertEqual(e.string, ";enum3\t", "the suppression mark keeps Tab from expanding it")
        let idle = Editor()
        XCTAssertFalse(idle.esc().consumed, "Esc with nothing captured is the host's")
    }

    func testUndoToLiteral() {
        let e = Editor("a\n")
        e.type(";sec{A}")
        e.tab()
        XCTAssertEqual(e.string, "a\n\\section{A}")
        e.undo()
        XCTAssertEqual(e.string, "a\n;sec{A}", "one undo restores the literal")
        XCTAssertEqual(e.controller.suppressed, [NSRange(location: 2, length: 7)], "and marks it")
        XCTAssertEqual(e.state, .idle, "capture does not re-arm")
        e.tab()
        XCTAssertEqual(e.string, "a\n;sec{A}\t", "Tab does not re-expand it")
    }

    func testSuppressionClearsWhenTheCaretLeavesTheLine() {
        let e = Editor("top\n")
        e.type(";sec")
        _ = e.esc()
        e.move(to: 1)
        XCTAssertEqual(e.controller.suppressed, [], "the caret left the line")
        e.move(to: 8)
        e.tab()
        XCTAssertEqual(e.string, "top\n\\section{}", "Tab after the literal expands it again")
    }

    func testRetroTabAndEmptyLeader() {
        let e = Editor("text ;sec{Intro}")
        e.tab()
        XCTAssertEqual(e.string, "text \\section{Intro}", "idle Tab expands leader+abbreviation before the caret")
        let miss = Editor("text ;zzz")
        miss.tab()
        XCTAssertEqual(miss.string, "text ;zzz\t", "nothing to expand: Tab is the host's")
        var bare = Editor.on
        bare.leader = ""
        let b = Editor("\\begin{itemize}\n  item", settings: bare)
        b.tab()
        XCTAssertEqual(b.string, "\\begin{itemize}\n  \\item ", "empty leader: bare Tab abbreviations")
        b.type(";")
        XCTAssertEqual(b.state, .idle, "an empty leader never arms")
    }

    func testMasterSwitchAndTierOff() {
        let off = Editor(settings: T.Settings())
        off.type(";enum3")
        XCTAssertEqual(off.state, .idle)
        off.tab()
        XCTAssertEqual(off.string, ";enum3\t")
        var noA = Editor.on
        noA.abbreviations = false
        let e = Editor(settings: noA)
        e.type(";enum3")
        e.tab()
        XCTAssertEqual(e.string, ";enum3\t")
    }

    func testCustomLeaderAndScopeAwareExpansion() {
        var s = Editor.on
        s.leader = ","
        let e = Editor("\\begin{itemize}\n  ", settings: s)
        e.type(",item{a}")
        e.tab()
        XCTAssertEqual(e.string, "\\begin{itemize}\n  \\item a")
        let beamer = Editor("\\begin{frame}\n  ", documentClass: "beamer")
        beamer.type(";items>item*2<+->")
        XCTAssertNotNil(beamer.last.preview, "overlays inside a frame")
        beamer.tab()
        XCTAssertEqual(beamer.string, "\\begin{frame}\n  \\begin{itemize}\n    \\item<+-> \n    \\item<+-> \n  \\end{itemize}",
                       "the line's indentation is the base indent")
    }

    func testExpansionErrorsExplainOnTab() {
        let e = Editor()
        e.type(";sec3")
        let out = e.tab()
        XCTAssertTrue(out.consumed)
        XCTAssertEqual(out.diagnostic, "`sec` takes no size")
        XCTAssertEqual(e.string, ";sec3")
    }

    func testPlaceholdersAreSelected() {
        let e = Editor()
        e.type(";fig")
        let out = e.tab()
        let s = try! XCTUnwrap(out.commit?.snippet)
        XCTAssertEqual(s.caretLength, ("width=0.8\\linewidth" as NSString).length, "the first placeholder is selected")
        XCTAssertEqual(s.stopLengths, [0, 0, 0])
    }

    // MARK: scope provider

    func testScopeProvider() {
        let doc = """
        \\documentclass{article}
        \\usepackage{amsmath} % $ not math
        \\begin{document}
        Text $x + \\(y\\) $ and \\[ z \\] then
        \\begin{itemize}
          \\item a \\verb|$| b
          \\begin{align*}
            a &= b
          \\end{align*}
        \\end{itemize}
        \\begin{verbatim}
        \\begin{itemize} $
        \\end{verbatim}
        \\end{document}
        """ as NSString
        let p = T.ScopeProvider()
        func flags(after marker: String, _ offset: Int = 0) -> Set<String> {
            let r = doc.range(of: marker)
            XCTAssertNotEqual(r.location, NSNotFound, marker)
            return p.scope(at: NSMaxRange(r) + offset, in: doc).flags
        }
        XCTAssertEqual(flags(after: "\\usepackage"), ["preamble"])
        XCTAssertEqual(flags(after: "% $"), ["comment"], "a comment is its own mode")
        XCTAssertEqual(flags(after: "Text"), ["text"])
        XCTAssertEqual(flags(after: "Text $x"), ["math", "math:inline"])
        XCTAssertEqual(flags(after: "\\(y"), ["math", "math:inline"])
        XCTAssertEqual(flags(after: "\\) $ and"), ["text"])
        XCTAssertEqual(flags(after: "\\[ z"), ["math", "math:display"])
        XCTAssertEqual(flags(after: "\\item a"), ["text", "list", "env:itemize"])
        XCTAssertEqual(flags(after: "\\verb|"), ["verbatim", "list", "env:itemize"])
        XCTAssertEqual(flags(after: "$| b"), ["text", "list", "env:itemize"])
        XCTAssertEqual(flags(after: "a &="), ["math", "math:display", "list", "structure", "env:itemize", "env:align"])
        XCTAssertEqual(flags(after: "\\end{itemize}\n"), ["text"])
        XCTAssertEqual(flags(after: "\\begin{itemize} $"), ["verbatim", "env:verbatim"])
        XCTAssertEqual(flags(after: "\\end{verbatim}"), ["text"])
        // Offsets and frames for the structure editor (M10b).
        let at = doc.range(of: "a &=").location
        let stack = p.scope(at: at, in: doc)
        let align = stack.innermost { $0 == "align*" }
        XCTAssertEqual(align?.start, doc.range(of: "\\begin{align*}").location)
        XCTAssertEqual(align?.bodyStart, NSMaxRange(doc.range(of: "\\begin{align*}")))
        // No \documentclass: a fragment is body text.
        XCTAssertEqual(T.ScopeProvider().scope(at: 3, in: "abc"), T.ScopeStack([.init(.document)]))
    }

    func testScopeProviderIsIncrementalAndFast() {
        var lines: [String] = ["\\documentclass{article}", "\\begin{document}"]
        for k in 0..<5000 { lines.append(k % 50 == 0 ? "\\begin{itemize} \\item $x_\(k)$ \\end{itemize} % c" : "Line \(k) with $a+b$ and \\textbf{bold} text.") }
        lines.append("\\end{document}")
        let text = NSMutableString(string: lines.joined(separator: "\n"))
        let p = T.ScopeProvider()
        let end = text.length - 20
        XCTAssertEqual(p.scope(at: end, in: text).flags, ["text"]) // warm-up: builds checkpoints
        let t0 = Date()
        for k in 0..<100 { _ = p.scope(at: end - k * 37, in: text) }
        let perQuery = Date().timeIntervalSince(t0) / 100
        XCTAssertLessThan(perQuery, 0.005, "warm queries scan at most a checkpoint's worth (§8 budget: 1 ms in release)")
        // An edit invalidates the checkpoints after it, and only those.
        let mid = text.length / 2
        text.replaceCharacters(in: NSRange(location: mid, length: 0), with: "\\begin{itemize}\n")
        p.noteEdit(range: NSRange(location: mid, length: 0), replacementLength: 16)
        XCTAssertEqual(p.scope(at: text.length - 20, in: text).flags, ["text", "list", "env:itemize"])
        XCTAssertEqual(p.scope(at: 30, in: text).flags, ["preamble"], "inside `\\begin{document}`")
        XCTAssertEqual(p.scope(at: 41, in: text).flags, ["text"], "the first body line's start")
    }

    func testLaTeXScanMovedIntoTheCore() {
        let text = "\\begin{a} \\begin{b} x \\end{b} \\end{a}" as NSString
        XCTAssertEqual(LaTeXScan.enclosingEnvironment(at: 20, in: text)?.name, "b")
        XCTAssertEqual(LaTeXScan.enclosingEnvironment(at: 31, in: text)?.name, "a")
        XCTAssertEqual(LaTeXScan.environmentPairs(in: text).count, 2)
    }
}
