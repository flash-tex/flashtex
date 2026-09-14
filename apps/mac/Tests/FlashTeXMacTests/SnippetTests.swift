import AppKit
import HostedWindows
import XCTest
@testable import FlashTeXMac

/// Snippet tab stops, context-aware completion sources and the editor
/// niceties added by mac-intellisense-2 (Completion.swift,
/// EditorIntelligence.swift).
final class SnippetTests: XCTestCase {
    private func labels(_ s: [Completion.Suggestion]) -> [String] { s.map(\.label) }

    // MARK: pure models

    func testVocabularySnippetsCarryTabStops() throws {
        let frac = try XCTUnwrap(Completion.Vocabulary.byName["frac"]?.snippet)
        XCTAssertEqual(frac.text, "\\frac{}{}")
        XCTAssertEqual(frac.caretUTF16, 6)
        XCTAssertEqual(frac.stops, [8, 9], "second braces, then the end")
        let section = try XCTUnwrap(Completion.Vocabulary.byName["section"]?.snippet)
        XCTAssertEqual(section.stops, [10], "one argument: Tab leaves")
        let left = try XCTUnwrap(Completion.Vocabulary.byName["left"]?.snippet)
        XCTAssertEqual(left.text, "\\left( \\right)")
        XCTAssertEqual(left.caretUTF16, 6)
        XCTAssertEqual(left.stops, [14])
        XCTAssertNil(Completion.Vocabulary.byName["alpha"]?.snippet)

        let itemize = Completion.environmentSnippet("itemize", indent: "  ")
        XCTAssertEqual(itemize.text, "itemize}\n  \\item \n  \\end{itemize}")
        XCTAssertEqual(itemize.caretUTF16, ("itemize}\n  \\item " as NSString).length)
        XCTAssertEqual(itemize.stops, [(itemize.text as NSString).length])
        let figure = Completion.environmentSnippet("figure", indent: "")
        XCTAssertEqual(figure.text, "figure}\n\\centering\n\\includegraphics[width=0.8\\linewidth]{}\n\\caption{}\n\\label{fig:}\n\\end{figure}")
        XCTAssertEqual(figure.caretUTF16, ("figure}\n\\centering\n\\includegraphics[width=0.8\\linewidth]{" as NSString).length)
        XCTAssertEqual(figure.stops.count, 3, "caption, label, end")
        XCTAssertEqual(figure.stops[0], ("figure}\n\\centering\n\\includegraphics[width=0.8\\linewidth]{}\n\\caption{" as NSString).length)
        let align = Completion.environmentSnippet("align*", indent: "")
        XCTAssertEqual(align.text, "align*}\n\n\\end{align*}")
        XCTAssertEqual(align.caretUTF16, 8)
    }

    func testFuzzyMatchingRanksExactThenPrefixThenSubsequence() {
        XCTAssertEqual(Completion.matchRank("sec", prefix: "sec"), 0)
        XCTAssertEqual(Completion.matchRank("section", prefix: "sec"), 1)
        XCTAssertEqual(Completion.matchRank("subsection", prefix: "sbs"), 2)
        XCTAssertNil(Completion.matchRank("section", prefix: "x"))
        XCTAssertNil(Completion.matchRank("section", prefix: "n"), "one character never matches as a subsequence")
        XCTAssertNil(Completion.matchRank("section", prefix: "tc"), "order matters")
        // Commands: prefix matches keep their order; subsequence matches follow.
        let s = Completion.suggestions(in: "\\sbs", caretUTF16: 4, result: nil)
        XCTAssertEqual(labels(s).first, "\\subsection{...}")
        XCTAssertTrue(s.allSatisfy { Completion.matchRank($0.insertText.dropFirst().description, prefix: "sbs") == 2 })
        let se = Completion.suggestions(in: "x \\se", caretUTF16: 5, result: nil)
        // Computed from the live vocabulary (not a hand-copied snapshot) so this
        // tracks the compiler's inventory as it grows.
        XCTAssertEqual(labels(se), CompletionTestVocabulary.labels(forPrefix: "se"), "prefix matches only, in table order")
        // The rule itself, isolated from the compiler's vocabulary through a
        // synthetic `supported:` list injected via the seam on
        // `Completion.suggestions`: prefix matches keep table order.
        let syntheticSe = Completion.suggestions(in: "x \\se", caretUTF16: 5, result: nil, supported: ["set", "search", "sea", "xyz"])
        XCTAssertEqual(syntheticSe.map(\.insertText), ["\\set", "\\search", "\\sea"])
        // Labels: `\ref{main}` finds `eq:main` only when no key starts with `main`.
        let text = "\\label{eq:main}\\label{main}\\label{sec:domain} \\ref{main"
        XCTAssertEqual(labels(Completion.suggestions(in: text, caretUTF16: (text as NSString).length, result: nil)), ["main"])
        let fuzzy = "\\label{eq:main}\\label{sec:domain} \\ref{main"
        XCTAssertEqual(labels(Completion.suggestions(in: fuzzy, caretUTF16: (fuzzy as NSString).length, result: nil)), ["eq:main", "sec:domain"])
        XCTAssertEqual(Completion.fuzzyFilter(["ab", "xab", "b"], prefix: "ab") { $0 }, ["ab"])
        XCTAssertEqual(Completion.fuzzyFilter(["xab", "b", "a-b"], prefix: "ab") { $0 }, ["xab", "a-b"])
    }

    func testArgumentKeysAndNewContexts() {
        // Keys with `:` `-` `/` `.` are one token after a key-taking command.
        let ref = "\\label{eq:main-1}\\label{eq:other} \\ref{eq:ma"
        let r = Completion.suggestions(in: ref, caretUTF16: (ref as NSString).length, result: nil)
        XCTAssertEqual(labels(r), ["eq:main-1"])
        XCTAssertEqual(Completion.completionRange(in: ref, caretUTF16: (ref as NSString).length), NSRange(location: 39, length: 5))
        // A list argument completes the key after the comma.
        let cite = "\\bibitem{knuth84}\\bibitem{lamport94} \\cite{knuth84,lam"
        XCTAssertEqual(labels(Completion.suggestions(in: cite, caretUTF16: (cite as NSString).length, result: nil)), ["lamport94"])
        // `\usepackage{`: known package names, fuzzy.
        let pkg = Completion.suggestions(in: "\\usepackage{ams", caretUTF16: 15, result: nil)
        XCTAssertEqual(Array(labels(pkg).prefix(4)), ["amsmath", "amssymb", "amsthm", "amsfonts"])
        XCTAssertEqual(pkg.first?.insertText, "amsmath")
        let pkg2 = Completion.suggestions(in: "\\usepackage{amsmath,hyp", caretUTF16: 23, result: nil)
        XCTAssertEqual(labels(pkg2).first, "hyperref")
        XCTAssertEqual(Completion.completionRange(in: "\\usepackage{amsmath,hyp", caretUTF16: 23), NSRange(location: 20, length: 3))
        // `\input{`/`\include{`/`\includegraphics{`: project files, matched on path or basename.
        let files = ["chapters/one.tex", "chapters/two.tex", "figures/plot.pdf", "main.tex"]
        let inp = Completion.suggestions(in: "\\input{ch", caretUTF16: 9, metadata: nil, projectFiles: files)
        XCTAssertEqual(labels(inp), ["chapters/one.tex", "chapters/two.tex"])
        XCTAssertEqual(inp.first?.detail, "project document")
        let gfx = Completion.suggestions(in: "\\includegraphics{plot", caretUTF16: 21, metadata: nil, projectFiles: files)
        XCTAssertEqual(labels(gfx), ["figures/plot.pdf"])
        XCTAssertEqual(labels(Completion.suggestions(in: "\\include{", caretUTF16: 9, metadata: nil, projectFiles: files)), files)
        XCTAssertTrue(Completion.suggestions(in: "\\input{zz", caretUTF16: 9, metadata: nil, projectFiles: files).isEmpty)
        XCTAssertTrue(Completion.suggestions(in: "\\input{ch", caretUTF16: 9, result: nil).isEmpty, "no project: nothing")
        // `\label{` follows the innermost open environment.
        let fig = "\\section{River Walk}\n\\begin{figure}\n\\label{"
        let f = Completion.suggestions(in: fig, caretUTF16: (fig as NSString).length, result: nil)
        XCTAssertEqual(labels(f), ["fig:river-walk", "fig:"])
        XCTAssertEqual(f.first?.detail, "unique figure key under \\section{River Walk}")
        XCTAssertEqual(f.last?.insertText, "fig:", "the bare prefix leaves the caret for the key")
        let eq = "\\begin{equation}\n\\label{"
        XCTAssertEqual(labels(Completion.suggestions(in: eq, caretUTF16: (eq as NSString).length, result: nil)), ["eq:"], "no heading: the prefix alone")
        let tab = "\\section{Data}\\begin{table}\\label{ta"
        XCTAssertEqual(labels(Completion.suggestions(in: tab, caretUTF16: (tab as NSString).length, result: nil)), ["tab:data", "tab:"])
        let closed = "\\section{Data}\\begin{figure}\\end{figure}\\label{"
        XCTAssertEqual(labels(Completion.suggestions(in: closed, caretUTF16: (closed as NSString).length, result: nil)), ["sec:data"], "a closed environment does not count")
        XCTAssertTrue(Completion.suggestions(in: "\\label{", caretUTF16: 7, result: nil).isEmpty, "no heading, no environment: nothing")
    }

    func testReturnContinuesListItemsAndCommentToggleIsExact() {
        typealias EI = EditorIntelligence
        XCTAssertEqual(EI.itemContinuation(inLinePrefix: "  \\item first"), "\\item ")
        XCTAssertEqual(EI.itemContinuation(inLinePrefix: "\\item[term] text"), "\\item[] ")
        XCTAssertNil(EI.itemContinuation(inLinePrefix: "\\item"), "bare item: leave the list")
        XCTAssertNil(EI.itemContinuation(inLinePrefix: "\\item   "))
        XCTAssertNil(EI.itemContinuation(inLinePrefix: "\\itemize x"))
        XCTAssertNil(EI.itemContinuation(inLinePrefix: "text \\item x"), "only a line that starts with \\item")
        let text = "\\begin{itemize}\n  \\item one\n\\end{itemize}" as NSString
        let ins = EI.newline(in: text, caret: 27, indentUnit: "  ", closeEnvironments: true)
        XCTAssertEqual(ins.text, "\n  \\item ")
        XCTAssertEqual(ins.caretOffset, 9)
        XCTAssertNil(ins.closedEnvironment)
        // Return in the middle of an item just breaks the line (the rest moves down).
        XCTAssertEqual(EI.newline(in: text, caret: 24, indentUnit: "  ", closeEnvironments: true).text, "\n  ")
        // A bare \item: plain newline with the indent.
        XCTAssertEqual(EI.newline(in: "  \\item" as NSString, caret: 7, indentUnit: "  ", closeEnvironments: true).text, "\n  ")

        // ⌘/ on a caret line, on a selection, and back.
        let one = EI.toggleComment(in: "a\n  b\nc" as NSString, selection: NSRange(location: 3, length: 0))
        XCTAssertEqual(one?.range, NSRange(location: 2, length: 3))
        XCTAssertEqual(one?.replacement, "  % b")
        XCTAssertEqual(one?.selection, NSRange(location: 5, length: 0), "the caret stays on its line, shifted")
        let back = EI.toggleComment(in: "a\n  % b\nc" as NSString, selection: NSRange(location: 6, length: 0))
        XCTAssertEqual(back?.replacement, "  b")
        XCTAssertEqual(back?.selection, NSRange(location: 4, length: 0))
        let many = EI.toggleComment(in: "a\n\nb\n% c\n" as NSString, selection: NSRange(location: 0, length: 9))
        XCTAssertEqual(many?.range, NSRange(location: 0, length: 8), "a selection ending after a newline excludes the next line")
        XCTAssertEqual(many?.replacement, "% a\n\n% b\n% % c", "mixed: comment everything, blank lines untouched")
        XCTAssertEqual(many?.selection, NSRange(location: 0, length: 14))
        let all = EI.toggleComment(in: "% a\n%b\n\n" as NSString, selection: NSRange(location: 0, length: 7))
        XCTAssertEqual(all?.replacement, "a\nb", "all commented: uncomment, `%` with or without a space")
        XCTAssertNil(EI.toggleComment(in: "\n  \n" as NSString, selection: NSRange(location: 0, length: 4)), "only blank lines: nothing")
        XCTAssertNotNil(EI.toggleComment(in: "x" as NSString, selection: NSRange(location: 99, length: 5)), "out-of-range selection is clamped")
    }

    // MARK: through the real text view

    @MainActor
    private func key(_ tv: NSTextView, _ chars: String, code: UInt16, flags: NSEvent.ModifierFlags = []) {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                                 windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: chars,
                                 charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        tv.keyDown(with: e)
    }

    @MainActor
    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 5_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    @MainActor
    func testTabMovesBetweenPlaceholdersAndEscLeaves() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless()
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        tv.allowsUndo = true

        func accept(after typing: String, from seed: String) async throws {
            tv.string = seed
            tv.setSelectedRange(NSRange(location: (seed as NSString).length, length: 0))
            for ch in typing { key(tv, String(ch), code: 0) }
            key(tv, " ", code: 49, flags: .control)
            try await waitUntil("popup for \(typing)") { tv.session != nil }
            key(tv, "\r", code: 36)
            XCTAssertNil(tv.session)
        }

        // \frac{|}{}: type the numerator, Tab to the denominator, Tab out.
        try await accept(after: "\\fra", from: "$")
        XCTAssertEqual(tv.string, "$\\frac{}{}")
        XCTAssertEqual(tv.selectedRange().location, 7)
        XCTAssertTrue(tv.isSnippetActive)
        XCTAssertEqual(tv.snippetStops, [9, 10])
        XCTAssertTrue(tv.isSignatureHelpVisible, "the argument pattern shows for the inserted snippet")
        for ch in "ab" { key(tv, String(ch), code: 0) }
        XCTAssertEqual(tv.snippetStops, [11, 12], "typing shifts the stops")
        key(tv, "\t", code: 48)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 11, length: 0), "Tab: into the denominator")
        XCTAssertTrue(tv.isSnippetActive)
        key(tv, "c", code: 0)
        key(tv, "\t", code: 48, flags: .shift)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 7, length: 0), "⇧Tab: back to the first placeholder")
        key(tv, "\t", code: 48)
        XCTAssertEqual(tv.selectedRange().location, 11, "the placeholder stays at the start of what was typed there")
        key(tv, "\t", code: 48)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 13, length: 0), "Tab past the last placeholder: after the snippet")
        XCTAssertFalse(tv.isSnippetActive)
        XCTAssertEqual(tv.string, "$\\frac{ab}{c}")
        key(tv, "\t", code: 48)
        XCTAssertEqual(tv.string, "$\\frac{ab}{c}\t", "no snippet: Tab is a Tab")

        // Esc leaves the snippet without opening the completion list.
        try await accept(after: "\\fra", from: "$")
        XCTAssertTrue(tv.isSnippetActive)
        key(tv, "\u{1B}", code: 53)
        XCTAssertFalse(tv.isSnippetActive)
        XCTAssertNil(tv.session)
        XCTAssertFalse(tv.isSignatureHelpVisible)

        // The caret leaving the snippet ends it.
        try await accept(after: "\\fra", from: "$")
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        XCTAssertFalse(tv.isSnippetActive)

        // Environment template: itemize with its first \item, Tab to the end.
        try await accept(after: "\\begin{item", from: "")
        XCTAssertEqual(tv.string, "\\begin{itemize}\n\\item \n\\end{itemize}")
        XCTAssertEqual(tv.selectedRange().location, 22)
        key(tv, "\t", code: 48)
        XCTAssertEqual(tv.selectedRange().location, (tv.string as NSString).length)
        XCTAssertFalse(tv.isSnippetActive)

        // ⌘/ toggles the caret line as one undo step.
        tv.string = "a\nb"
        tv.setSelectedRange(NSRange(location: 3, length: 0))
        tv.undoManager?.removeAllActions()
        key(tv, "/", code: 44, flags: .command)
        XCTAssertEqual(tv.string, "a\n% b")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 5, length: 0))
        XCTAssertEqual(tv.undoManager?.undoActionName, "Toggle Comment")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "a\nb")
        tv.undoManager?.redo()
        XCTAssertEqual(tv.string, "a\n% b")
        key(tv, "/", code: 44, flags: .command)
        XCTAssertEqual(tv.string, "a\nb", "toggles back")
    }
}
