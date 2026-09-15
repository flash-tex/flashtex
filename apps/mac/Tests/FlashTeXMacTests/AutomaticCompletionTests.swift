import AppKit
import HostedWindows
import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// Typing opens the completion list on its own — no ⌃Space.
///
/// Before this, `CompletingTextView.requestCompletion` had exactly four
/// callers (AppKit's `complete:`, ⌃Space, Esc, and the re-scan that narrows an
/// *already open* list), so a list that was never asked for never appeared.
/// `EditorPreferences.completionPopup` documented "typing, Esc, ⌃Space"; only
/// the last two were implemented.
///
/// The trigger is deliberately narrow and cheap:
///
/// * `Completion.opensAutomatically` decides from the token at the caret
///   alone — a control word or an argument key whose command gives it meaning
///   — so a prose word never pops the list up mid-sentence.
/// * `Completion.caretToken` reads that token from a bounded window, so the
///   per-keystroke cost is O(window), not O(document). The document-wide scan
///   happens once per typing pause, behind `automaticCompletionDelay`.
/// * Esc dismisses for the token it was pressed on and typing more of that
///   token stays quiet; any other token, or a caret move, re-arms it.
final class AutomaticCompletionPolicyTests: XCTestCase {
    /// The pure policy, with no AppKit in the way: which tokens typing opens
    /// the list for.
    func testOnlyCommandsAndArgumentKeysOpenTheListWhileTyping() {
        func opens(_ text: String) -> Bool {
            Completion.opensAutomatically(Completion.token(in: text, caretUTF16: (text as NSString).length))
        }
        // A control word, including the `\` on its own (which lists the vocabulary).
        XCTAssertTrue(opens("\\"), "the backslash alone lists the vocabulary")
        XCTAssertTrue(opens("\\s"), "one letter is enough after a backslash")
        XCTAssertTrue(opens("\\section"))
        XCTAssertTrue(opens("text \\emp"))
        // Argument keys whose command gives them meaning, at any length.
        for opener in ["\\begin{", "\\end{", "\\ref{", "\\eqref{", "\\autoref{", "\\pageref{", "\\cite{",
                       "\\citep{", "\\label{", "\\usepackage{", "\\input{", "\\include{", "\\includegraphics{"] {
            XCTAssertTrue(opens(opener), "\(opener) opens with an empty key")
            XCTAssertTrue(opens(opener + "a"), "\(opener)a")
        }
        XCTAssertTrue(opens("\\usepackage{amsmath,gra"), "the second key of a list")
        // Prose. `wordSuggestions` still offers these on ⌃Space; typing them
        // must not raise a list over every fourth letter of a sentence.
        XCTAssertFalse(opens("introduction"))
        XCTAssertFalse(opens("The theore"))
        XCTAssertFalse(opens("\\frac{numer"), "a plain brace argument is not a key context")
        // Nothing to complete at all.
        XCTAssertFalse(opens("x "), "after a space")
        XCTAssertFalse(opens("\\ref{eq:main}"), "after the closing brace")
        XCTAssertFalse(opens(""))
    }

    /// `caretToken` reads the token from a window, so it must recognise the
    /// same token as a scan of the whole text — same kind, same spelling, same
    /// argument context — and report a start that indexes the whole text.
    ///
    /// The token's own `start`/`end` are deliberately window-relative (see
    /// `CaretToken.token`), so position is compared through `startUTF16`, the
    /// one offset that means anything outside the window.
    func testCaretTokenFromABoundedWindowRecognisesTheSameTokenAsTheWholeTextScan() throws {
        /// The token without its byte offsets: what it is, not where it is.
        func shape(_ token: Completion.Token?) -> String? {
            switch token {
            case .command(let name, _, _): return "command(\(name))"
            case .word(let text, _, _, let context): return "word(\(text), \(context))"
            case nil: return nil
            }
        }
        let filler = String(repeating: "Lorem ipsum dolor sit amet. ", count: 400) // ≫ the window
        for suffix in ["\\sec", "\\", "\\begin{item", "\\ref{eq:ma", "\\cite{knuth-84", "\\usepackage{ams",
                       "\\includegraphics{fig/a.png", "plainword", "x "] {
            let text = filler + suffix
            let ns = text as NSString
            let whole = Completion.token(in: text, caretUTF16: ns.length)
            let windowed = Completion.caretToken(in: ns, caretUTF16: ns.length)
            XCTAssertEqual(shape(windowed?.token), shape(whole), "token for \(suffix)")
            XCTAssertEqual(Completion.opensAutomatically(windowed?.token), Completion.opensAutomatically(whole),
                           "the policy agrees for \(suffix)")
            if let whole, let windowed {
                let expected = try XCTUnwrap(text.nsRange(utf8Bytes: .init(path: "", startByte: whole.start, endByte: whole.end)))
                XCTAssertEqual(windowed.startUTF16, expected.location, "start for \(suffix) indexes the whole text")
                XCTAssertLessThanOrEqual(windowed.token.end, Completion.caretTokenWindow,
                                         "the token's own offsets stay inside the window, as documented")
            }
        }
        // The window is measured in UTF-16 units and must never split a
        // surrogate pair: an emoji-heavy prefix still resolves the token, and
        // its start still counts UTF-16 units of the whole text.
        let emoji = String(repeating: "🧮", count: 900) + "\\alp"
        let emojiNS = emoji as NSString
        let windowedEmoji = Completion.caretToken(in: emojiNS, caretUTF16: emojiNS.length)
        XCTAssertEqual(shape(windowedEmoji?.token), shape(Completion.token(in: emoji, caretUTF16: emojiNS.length)))
        XCTAssertEqual(windowedEmoji?.startUTF16, 1800, "900 surrogate pairs before the `\\`")
        // A caret past the end, and an empty text, are answers not crashes.
        XCTAssertNil(Completion.caretToken(in: "" as NSString, caretUTF16: 0))
        XCTAssertNotNil(Completion.caretToken(in: "\\se" as NSString, caretUTF16: 99))
    }

    /// A key event only arms the list when it types a character: Return, Tab,
    /// Esc, Delete, the arrows and every ⌘/⌃ shortcut must not.
    func testOnlyTypedCharactersArmTheList() throws {
        func event(_ chars: String, _ code: UInt16, _ flags: NSEvent.ModifierFlags = []) throws -> NSEvent {
            try XCTUnwrap(NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags,
                                           timestamp: 0, windowNumber: 0, context: nil, characters: chars,
                                           charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code))
        }
        XCTAssertTrue(CompletingTextView.typesACharacter(try event("a", 0)))
        XCTAssertTrue(CompletingTextView.typesACharacter(try event("\\", 42)))
        XCTAssertTrue(CompletingTextView.typesACharacter(try event("{", 33)))
        XCTAssertTrue(CompletingTextView.typesACharacter(try event("É", 14, .shift)), "⇧ and ⌥ still type")
        XCTAssertFalse(CompletingTextView.typesACharacter(try event("\r", 36)), "Return")
        XCTAssertFalse(CompletingTextView.typesACharacter(try event("\t", 48)), "Tab")
        XCTAssertFalse(CompletingTextView.typesACharacter(try event("\u{1B}", 53)), "Esc")
        XCTAssertFalse(CompletingTextView.typesACharacter(try event("\u{7F}", 51)), "Delete")
        XCTAssertFalse(CompletingTextView.typesACharacter(try event("\u{F702}", 123)), "←")
        XCTAssertFalse(CompletingTextView.typesACharacter(try event("\u{F704}", 122)), "F1")
        XCTAssertFalse(CompletingTextView.typesACharacter(try event("a", 0, .command)), "⌘A")
        XCTAssertFalse(CompletingTextView.typesACharacter(try event(" ", 49, .control)), "⌃Space")
    }
}

@MainActor
final class AutomaticCompletionTests: XCTestCase {
    private var window: NSWindow!
    private var tv: CompletingTextView!

    override func setUp() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                                            backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        tv.allowsUndo = true
        tv.automaticCompletionDelay = 0 // fired explicitly by `type`; no wall-clock wait
        window.orderFrontRegardless() // never makeKey: the test must not steal focus
        window.makeFirstResponder(tv)
    }

    override func tearDown() async throws {
        tv.close(.escape)
        window.orderOut(nil)
        window = nil
        tv = nil
    }

    private func key(_ chars: String, code: UInt16, flags: NSEvent.ModifierFlags = []) {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags,
                                 timestamp: ProcessInfo.processInfo.systemUptime,
                                 windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: chars,
                                 charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        tv.keyDown(with: e)
    }

    /// Types `chars` one key at a time and then runs whatever automatic open
    /// they armed, so the test never waits on a timer.
    private func type(_ chars: String) {
        for ch in chars { key(String(ch), code: 0) }
        tv.flushAutomaticCompletion()
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 5_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    /// Set the buffer without arming anything (a document switch, not typing).
    private func load(_ text: String, caret: Int? = nil) {
        tv.string = text
        tv.setSelectedRange(NSRange(location: caret ?? (text as NSString).length, length: 0))
        XCTAssertFalse(tv.hasPendingAutomaticCompletion, "a programmatic replacement never opens the list")
    }

    /// The headline: typing a command opens the list with no ⌃Space, and the
    /// items are the ones the pure function computes for that caret.
    func testTypingACommandOpensTheListWithoutAnExplicitInvoke() async throws {
        load("\\begin{document}\n", caret: 17)
        XCTAssertNil(tv.session)
        type("\\se")
        XCTAssertEqual(tv.string, "\\begin{document}\n\\se")
        try await waitUntil("list opened by typing") { self.tv.session != nil }
        XCTAssertEqual(tv.automaticOpenCount, 1, "one scan for the burst, not one per keystroke")
        let expected = Completion.suggestions(in: tv.string, caretUTF16: 20, result: nil).map(\.label)
        XCTAssertEqual(tv.session?.items.map(\.label), expected)
        XCTAssertTrue(tv.completionPopup.isVisible)
        // Typing on narrows the open list the same way ⌃Space's does.
        key("c", code: 8)
        let narrowed = Completion.suggestions(in: "\\begin{document}\n\\sec", caretUTF16: 21, result: nil).map(\.label)
        try await waitUntil("narrowed") { self.tv.session?.items.map(\.label) == narrowed }
        // Return inserts whatever is selected (the vocabulary's order is the
        // compiler's, so the test reads the choice rather than assuming it).
        let chosen = try XCTUnwrap(tv.session?.selected)
        key("\r", code: 36)
        XCTAssertEqual(tv.string, "\\begin{document}\n" + (chosen.snippet?.text ?? chosen.insertText))
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.lastCloseReason, .accepted)
    }

    /// The backslash on its own is enough: nothing else has to be typed.
    func testTypingOnlyABackslashOpensTheVocabulary() async throws {
        load("x ")
        type("\\")
        try await waitUntil("vocabulary list") { self.tv.session != nil }
        XCTAssertEqual(tv.session?.items.count, Completion.maxSuggestions)
    }

    /// Every structured context a LaTeX author actually types opens by itself.
    func testEveryArgumentContextOpensWhileTyping() async throws {
        let document = """
        \\documentclass{article}
        \\begin{document}
        \\section{Method}\\label{sec:method}
        \\begin{figure}\\caption{A}\\label{fig:one}\\end{figure}
        \\begin{thebibliography}{9}\\bibitem{knuth-84}TeX\\end{thebibliography}

        """
        // `\ref{` offers this document's real labels; `\cite{` its real keys.
        for (typed, expected) in [("\\ref{", "sec:method"), ("\\ref{fi", "fig:one"), ("\\cite{", "knuth-84")] {
            load(document)
            type(typed)
            try await waitUntil("list for \(typed)") { self.tv.session != nil }
            XCTAssertTrue(tv.session!.items.contains { $0.label == expected },
                          "\(typed) offers \(expected), got \(tv.session!.items.map(\.label))")
            tv.close(.escape)
        }
        // `\begin{`/`\end{` offer environment names, `\usepackage{` packages.
        for (typed, expected) in [("\\begin{item", "itemize"), ("\\usepackage{amsm", "amsmath")] {
            load(document)
            type(typed)
            try await waitUntil("list for \(typed)") { self.tv.session != nil }
            XCTAssertTrue(tv.session!.items.contains { $0.label == expected },
                          "\(typed) offers \(expected), got \(tv.session!.items.map(\.label))")
            tv.close(.escape)
        }
        // `\includegraphics{` offers the project's real files.
        load(document)
        tv.projectFiles = ["figures/plot.pdf", "chapters/one.tex"]
        type("\\includegraphics{plo")
        try await waitUntil("list for includegraphics") { self.tv.session != nil }
        XCTAssertEqual(tv.session?.items.first?.label, "figures/plot.pdf")
    }

    /// Prose is left alone: the word list is worth asking for, not worth
    /// raising over a sentence.
    func testTypingProseDoesNotOpenTheList() async throws {
        load("The theorem of Noether. ")
        type("theore")
        XCTAssertFalse(tv.hasPendingAutomaticCompletion)
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.automaticOpenCount, 0)
        // ⌃Space still offers the document's words, as it always did.
        key(" ", code: 49, flags: .control)
        try await waitUntil("word list on ⌃Space") { self.tv.session != nil }
        XCTAssertEqual(tv.session?.items.first?.label, "theorem")
    }

    /// Esc dismisses for this token and typing more of it stays quiet; a
    /// different token, or a caret move, re-arms the automatic open.
    func testEscapeDismissesWithoutReopeningOnTheNextKeystroke() async throws {
        load("\\begin{document}\n", caret: 17)
        type("\\se")
        try await waitUntil("opened") { self.tv.session != nil }
        key("\u{1B}", code: 53)
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.lastCloseReason, .escape)
        // More of the same token: still nothing, and nothing even scheduled.
        type("c")
        XCTAssertFalse(tv.hasPendingAutomaticCompletion, "Esc holds for the token it dismissed")
        try await Task.sleep(nanoseconds: 60_000_000)
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.automaticOpenCount, 1, "no second automatic open")
        // ⌃Space is never suppressed — the user asked.
        key(" ", code: 49, flags: .control)
        try await waitUntil("⌃Space overrides the dismissal") { self.tv.session != nil }
        key("\u{1B}", code: 53)
        // A new token re-arms it.
        type(" \\al")
        try await waitUntil("a different token opens again") { self.tv.session != nil }
        XCTAssertEqual(tv.automaticOpenCount, 2)
    }

    /// The list must not fight the editor: typing through a non-match closes
    /// it, and a session with nothing selected never swallows Return or Tab.
    func testTypingThroughANonMatchClosesAndKeysAreNeverStolen() async throws {
        load("\\begin{document}\n", caret: 17)
        type("\\se")
        try await waitUntil("opened") { self.tv.session != nil }
        // `\sezzz` matches nothing, not even as a subsequence: the list closes.
        type("zzz")
        try await waitUntil("closed by a non-match") { self.tv.session == nil }
        XCTAssertEqual(tv.lastCloseReason, .noCandidates)
        XCTAssertEqual(tv.string, "\\begin{document}\n\\sezzz")
        // Return is the editor's again.
        key("\r", code: 36)
        XCTAssertEqual(tv.string, "\\begin{document}\n\\sezzz\n")
        // A session whose selection is gone hands the key back instead of
        // eating it (Return inserts a newline, Tab a tab).
        for (chars, code, inserted) in [("\r", UInt16(36), "\n"), ("\t", UInt16(48), "\t")] {
            let before = tv.string
            type("\\se")
            try await waitUntil("opened for \(code)") { self.tv.session != nil }
            tv.dropSessionSelectionForTesting()
            key(chars, code: code)
            XCTAssertNil(tv.session, "the empty session closed instead of consuming the key")
            XCTAssertEqual(tv.string, before + "\\se" + inserted)
            load(before)
        }
    }

    /// Deleting back through a word must not pop the list open, and neither
    /// must a caret move or a programmatic replacement.
    func testDeletionCaretMovesAndProgrammaticEditsNeverOpenTheList() async throws {
        load("\\begin{document}\n\\section", caret: 25)
        for _ in 0..<3 { key("\u{7F}", code: 51) } // Delete back to `\sect`
        XCTAssertEqual(tv.string, "\\begin{document}\n\\sect")
        XCTAssertFalse(tv.hasPendingAutomaticCompletion, "deleting never opens the list")
        key("\u{F702}", code: 123) // ←
        key("\u{F703}", code: 124) // →
        XCTAssertFalse(tv.hasPendingAutomaticCompletion, "moving the caret never opens the list")
        tv.string = "x \\al" // a document switch
        tv.setSelectedRange(NSRange(location: 5, length: 0))
        XCTAssertFalse(tv.hasPendingAutomaticCompletion, "a document switch never opens the list")
        try await Task.sleep(nanoseconds: 60_000_000)
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.automaticOpenCount, 0)
        // Typing one more character does open it — the machinery is armed, it
        // just refuses every path above.
        type("p")
        try await waitUntil("typing still opens it") { self.tv.session != nil }
        XCTAssertEqual(tv.automaticOpenCount, 1)
    }

    /// The preference that documented "typing, Esc, ⌃Space" now governs all
    /// three; with it off, typing raises nothing.
    func testThePreferenceTurnsTypingOffToo() async throws {
        let prefs = EditorPreferences.shared
        let restore = prefs.completionPopup
        defer { prefs.completionPopup = restore }
        prefs.completionPopup = false
        load("x ")
        type("\\se")
        XCTAssertFalse(tv.hasPendingAutomaticCompletion)
        try await Task.sleep(nanoseconds: 60_000_000)
        XCTAssertNil(tv.session)
        prefs.completionPopup = true
        type("c")
        try await waitUntil("typing opens it once the preference is back") { self.tv.session != nil }
    }

    /// A burst of typing costs one document scan, not one per keystroke: the
    /// wait is the whole point of the debounce, and the per-keystroke gate
    /// reads a bounded window instead of the document. Measured on a document
    /// far larger than the window.
    func testABurstOfTypingCostsOneScanAndTheGateDoesNotGrowWithTheDocument() async throws {
        let big = String(repeating: "\\section{Chapter}\\label{sec:c}\nLorem ipsum dolor sit amet.\n", count: 2_000)
        load(big + "\n")
        let scheduledBefore = tv.scheduler.statistics.scheduled
        tv.automaticCompletionDelay = 0.05
        for ch in "\\subsec" { key(String(ch), code: 0) } // 7 keystrokes, no flush
        XCTAssertEqual(tv.scheduler.statistics.scheduled, scheduledBefore,
                       "the keystrokes only armed a timer; nothing was scanned yet")
        XCTAssertTrue(tv.hasPendingAutomaticCompletion)
        tv.flushAutomaticCompletion()
        try await waitUntil("opened after the burst") { self.tv.session != nil }
        XCTAssertEqual(tv.scheduler.statistics.scheduled, scheduledBefore + 1, "one scan for the whole burst")

        // The per-keystroke gate reads a window; the scan the pure function
        // does reads the document. On a document this size the difference is
        // the reason typing can afford to arm the list at all.
        let text = tv.string
        let ns = text as NSString
        let caret = ns.length
        var windowedMs = 0.0, wholeMs = 0.0
        for _ in 0..<50 {
            var t = MonotonicClock.nowNs()
            _ = Completion.caretToken(in: ns, caretUTF16: caret)
            windowedMs += Double(MonotonicClock.nowNs() - t) / 1e6
            t = MonotonicClock.nowNs()
            _ = Completion.token(in: text, caretUTF16: caret)
            wholeMs += Double(MonotonicClock.nowNs() - t) / 1e6
        }
        print("automatic-completion gate on a \(text.utf8.count) B document (50 reads): "
              + String(format: "windowed %.4f ms/read vs whole-text %.4f ms/read", windowedMs / 50, wholeMs / 50)
              + "; off-main scan for the open " + String(format: "%.2f", tv.lastOutcome?.computeMs ?? .nan) + " ms")
        XCTAssertLessThan(windowedMs, wholeMs,
                          "the gate must not grow with the document (\(ns.length) UTF-16 units)")
    }
}
