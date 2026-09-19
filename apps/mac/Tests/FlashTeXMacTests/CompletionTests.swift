import CryptoKit
import SwiftUI
import XCTest
import FlashTeXAccessibility
import HostedWindows
@testable import FlashTeXProtocol
@testable import FlashTeXMac
@testable import FlashTeXEditorCore

final class CompletionTests: XCTestCase {
    private func caret(after needle: String, in text: String) -> Int {
        let r = (text as NSString).range(of: needle)
        XCTAssertNotEqual(r.location, NSNotFound, "needle \(needle) missing")
        return NSMaxRange(r)
    }

    private func labels(_ s: [Completion.Suggestion]) -> [String] { s.map(\.label) }

    // MARK: triggers and prefix filtering

    func testBackslashTriggersCommandsAndPrefixFilters() {
        // The fixtures of this test are an article project (`projectClass`),
        // as `CompletionTestVocabulary` assumes: beamer's `\setbeamer…` and
        // `\subtitle` are text entries that would otherwise take the head
        // of the `\se`/`\sub` lists by table order.
        let text = "Hello \\se"
        let s = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, result: nil, projectClass: "article")
        // The label shows the argument shape; the inserted text is the command alone.
        // Text-mode entries precede math ones (table order); nothing is spelled `se`.
        // Computed from the live vocabulary (not a hand-copied snapshot) so this
        // tracks the compiler's inventory as it grows.
        XCTAssertEqual(labels(s), CompletionTestVocabulary.labels(forPrefix: "se"))
        XCTAssertTrue(s.allSatisfy { $0.kind == .command && $0.insertText.hasPrefix("\\se") })
        XCTAssertEqual(s.first?.insertText, "\\section")
        XCTAssertEqual(s.first?.detail, "numbered section heading; starred form unnumbered")
        XCTAssertEqual(s.map(\.detail).suffix(2), ["math · upright operator name", "math · symbol ∖"])

        // The command spelled exactly as typed ranks first; the rest keep table order.
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\sec", caretUTF16: 6, result: nil, projectClass: "article")), CompletionTestVocabulary.labels(forPrefix: "sec"))
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\it", caretUTF16: 5, result: nil, projectClass: "article")), CompletionTestVocabulary.labels(forPrefix: "it"))
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\sub", caretUTF16: 6, result: nil, projectClass: "article")), CompletionTestVocabulary.labels(forPrefix: "sub"))

        // The ranking rule itself, isolated from the compiler's (growing)
        // vocabulary through the `supported:` injection seam: the name typed
        // exactly ranks first, the rest keep the order `supported` gave them,
        // and a candidate that does not start with the prefix is dropped.
        let synthetic = ["second", "sec", "sea", "search", "setminus", "xyz"]
        let ruleCheck = Completion.suggestions(in: "x \\se", caretUTF16: 5, result: nil, supported: synthetic)
        XCTAssertEqual(ruleCheck.map(\.insertText), ["\\second", "\\sec", "\\sea", "\\search", "\\setminus"])
        // `second` leads `sec` in table order, but typing `\sec` exactly must
        // still rank it first.
        let ruleCheckExact = Completion.suggestions(in: "x \\sec", caretUTF16: 6, result: nil, supported: synthetic)
        XCTAssertEqual(ruleCheckExact.map(\.insertText), ["\\sec", "\\second"],
                       "the exact typed spelling ranks first even though it is later in table order")

        // A lone backslash lists every supported command (capped at 12).
        let all = Completion.suggestions(in: "x \\", caretUTF16: 3, result: nil)
        XCTAssertEqual(all.count, Completion.maxSuggestions)
        XCTAssertEqual(all.first?.label, Completion.Vocabulary.entries.first?.label)
        XCTAssertTrue(all.allSatisfy { $0.kind == .command })

        // `\\` itself is a supported command.
        let dbl = Completion.suggestions(in: "a\\\\", caretUTF16: 3, result: nil)
        XCTAssertEqual(labels(dbl), ["\\\\"])
        XCTAssertEqual(dbl.first?.detail, "line break; an optional [length] is consumed")

        // Math commands say so and show the glyph the compiler renders.
        // `\allowdisplaybreaks` and `\allowbreak` (the \penalty0 break
        // permission, #568) match `al` too and are not symbols; they sort
        // ahead of the two by inventory order, which is what this asserts.
        // An article project, like `\fr` below: beamer's `\alert` is a text
        // entry that would otherwise lead the list by table order.
        let math = Completion.suggestions(in: "$\\al", caretUTF16: 4, result: nil, projectClass: "article")
        // `\alph{counter}` joined the inventory with #940 (the counter
        // representations) and sorts with the text entries, before the symbols.
        XCTAssertEqual(labels(math), ["\\allowdisplaybreaks[0-4]", "\\allowbreak", "\\alph{counter}", "\\alpha", "\\aleph"],
                       "inventory (math_symbol) order")
        XCTAssertEqual(math.map(\.detail).suffix(2), ["math · symbol α", "math · symbol ℵ"])
        XCTAssertEqual(math.map(\.detail).prefix(2), ["amsmath page-break permission inside displays; no material",
                                            "\\penalty0 · in math: zero-penalty breakpoint in a formula (\\penalty0); layout-neutral, formulas never break"])
        XCTAssertEqual(Completion.Vocabulary.symbols.count, Completion.Vocabulary.inventory.commands.filter { $0.origin == .mathSymbol && $0.renders }.count)
        XCTAssertGreaterThan(Completion.Vocabulary.entries.count, Completion.Vocabulary.symbols.count)
        // Math-only commands are marked once, by the `math ·` prefix of `Entry.detail`.
        // `\fr` means `\frac`: this fragment belongs to an article project,
        // so beamer's `\frametitle` (a text entry, ahead of `\frac` in table
        // order) is gated out by the root document's class — the way an
        // included file that declares no `\documentclass` learns its class
        // (`testClassScopedCommandsAreOfferedOnlyUnderTheirOwnClass`).
        let frac = Completion.suggestions(in: "\\fr", caretUTF16: 3, result: nil, projectClass: "article")
        XCTAssertEqual(labels(frac), ["\\frac{num}{den}"])
        XCTAssertEqual(frac.first?.insertText, "\\frac")
        XCTAssertEqual(frac.first?.detail, "math · fraction; \\cfrac lays out as \\frac")
        XCTAssertTrue(Completion.Vocabulary.entries.allSatisfy { !$0.description.contains("math mode only") },
                      "the mode is stated by the detail prefix, never repeated in the description")
        XCTAssertTrue(Completion.Vocabulary.entries.allSatisfy { ($0.mode == .math) == $0.detail.hasPrefix("math · ") })
        // `\quad`/`\qquad` work in text and math; they are one text entry each
        // (no `math ·` prefix) that also states the math behaviour.
        XCTAssertEqual(Completion.suggestions(in: "\\qq", caretUTF16: 3, result: nil).first?.detail, "2em of horizontal space · in math: 1em/2em math space")

        // No suggestions for a non-matching prefix or a caret with nothing before it.
        XCTAssertTrue(Completion.suggestions(in: "\\zzz", caretUTF16: 4, result: nil).isEmpty)
        XCTAssertTrue(Completion.suggestions(in: "abc ", caretUTF16: 4, result: nil).isEmpty)
    }

    func testWordsNeedTwoLettersAndAreFrequencyRanked() {
        let text = "theorem theory theorem thesis theorem theory th"
        let caret = (text as NSString).length
        XCTAssertTrue(Completion.suggestions(in: text, caretUTF16: caret - 1, result: nil).isEmpty, "one letter never triggers")
        let s = Completion.suggestions(in: text, caretUTF16: caret, result: nil)
        XCTAssertEqual(labels(s), ["theorem", "theory", "thesis"])
        XCTAssertEqual(s.first?.detail, "3× in this document")
        XCTAssertTrue(s.allSatisfy { $0.kind == .word })

        // Prefix filtering is ASCII case-insensitive; short words (<= 3 chars) never appear;
        // the word being typed is not counted as its own completion.
        let t2 = "The the them Then thesis thesis Th"
        let s2 = Completion.suggestions(in: t2, caretUTF16: (t2 as NSString).length, result: nil)
        XCTAssertEqual(labels(s2), ["thesis", "Then", "them"]) // "The"/"the" are too short
        let t3 = "thesis thesis"
        XCTAssertEqual(labels(Completion.suggestions(in: t3, caretUTF16: 13, result: nil)), ["thesis"]) // token itself not counted
        XCTAssertEqual(Completion.suggestions(in: t3, caretUTF16: 13, result: nil).first?.detail, "1× in this document")
    }

    func testUnclosedEnvironmentSuggestsEndFirst() {
        let text = "\\begin{document}\n\\begin{itemize}\n\\item a\n\\e"
        let s = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, result: nil)
        // Innermost closer first, then the vocabulary's `e` commands in table
        // order (text entries, then math entries). The line/page control
        // parameters `\emergencystretch` and `\enlargethispage` (#568) are
        // kernel commands, so they rank here in every document. letter.cls's
        // enclosure line `\encl{text}` sits between them and `\enspace`: it
        // is class-scoped (`Entry.requiresClass`), but this text declares no
        // `\documentclass` and has no project root to read one from, so
        // nothing gates it and it keeps its table place (the converse checks
        // below are where the gate shows).
        XCTAssertEqual(Array(labels(s).prefix(9)), ["\\end{itemize}", "\\end{document}", "\\emph{...}", "\\end{env}", "\\eqref{key}", "\\em", "\\emergencystretch=<dimen>", "\\enlargethispage*{dimension}", "\\encl{text}"])
        // `\enspace`/`\enskip` are dual-mode entries (like `\quad`/`\qquad`): text
        // entries whose detail states their math behaviour without a `math ·`
        // prefix, so the math-only run starts only after them: two closers
        // plus ten text entries (csquotes' `\enquote` last) fill the 12-entry
        // cap, so no math entry (`\eqqcolon` would be next) survives it.
        XCTAssertEqual(s.count, 12, "\(labels(s))")
        XCTAssertEqual(labels(s).last, "\\enquote{text}")
        XCTAssertTrue(s.allSatisfy { !$0.detail.hasPrefix("math · ") }, "\(labels(s))")
        // The same text in an article — declared by the text itself, or by
        // the root document of the project it is included in — hides
        // `\encl`; under letter.cls it keeps its old place. The command is
        // gated on the class, not dropped from the vocabulary.
        let article = "\\documentclass{article}\n" + text
        let inArticle = Completion.suggestions(in: article, caretUTF16: (article as NSString).length, result: nil)
        XCTAssertFalse(labels(inArticle).contains("\\encl{text}"), "\(labels(inArticle))")
        let included = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, result: nil, projectClass: "article")
        XCTAssertFalse(labels(included).contains("\\encl{text}"), "\(labels(included))")
        XCTAssertEqual(labels(included), labels(inArticle), "the root's class and the text's own gate the same way")
        let letter = "\\documentclass{letter}\n" + text
        let inLetter = Completion.suggestions(in: letter, caretUTF16: (letter as NSString).length, result: nil)
        XCTAssertEqual(Array(labels(inLetter).prefix(9)).suffix(2), ["\\enlargethispage*{dimension}", "\\encl{text}"])
        guard let first = s.first else { return XCTFail("expected at least one suggestion") }
        XCTAssertEqual(first.kind, .environment)
        XCTAssertEqual(first.detail, "closes \\begin{itemize} at byte 17")
        XCTAssertEqual(first.insertText, "\\end{itemize}")

        // Once itemize is closed only document remains open; the exact `\end`
        // spelling still ranks behind the closer it would have to name.
        let closed = text + "nd{itemize}\n\\en"
        let s2 = Completion.suggestions(in: closed, caretUTF16: (closed as NSString).length, result: nil)
        // longtable's `\endfirsthead`/`\endhead`/`\endfoot`/`\endlastfoot`
        // (inventoried with #940) match `\en` too and follow the closers.
        XCTAssertEqual(labels(s2).prefix(7), ["\\end{document}", "\\end{env}", "\\enlargethispage*{dimension}", "\\encl{text}", "\\enspace", "\\enskip", "\\enquote{text}"])
        let typed = closed + "d"
        XCTAssertEqual(labels(Completion.suggestions(in: typed, caretUTF16: (typed as NSString).length, result: nil)),
                       ["\\end{document}", "\\end{env}", "\\endfirsthead", "\\endhead", "\\endfoot", "\\endlastfoot"])

        // Inside `\end{` the open environments come first, then known/seen names.
        // Environments are class-gated like commands (`environmentOffered`):
        // beamer's `invisibleenv` is not offered in an article project, and
        // the same buffer in a beamer project — or with no class and no
        // project, which gates nothing — offers it after the open `itemize`.
        let inBrace = "\\begin{document}\\begin{itemize}\\end{"
        let env = Completion.suggestions(in: inBrace + "i", caretUTF16: (inBrace as NSString).length + 1, result: nil, projectClass: "article")
        XCTAssertEqual(labels(env), ["itemize"])
        XCTAssertEqual(env.first?.insertText, "itemize}")
        XCTAssertEqual(env.first?.kind, .environment)
        XCTAssertEqual(Completion.Vocabulary.environmentClasses["invisibleenv"], "beamer")
        for projectClass in ["beamer", nil] {
            let deck = Completion.suggestions(in: inBrace + "i", caretUTF16: (inBrace as NSString).length + 1, result: nil, projectClass: projectClass)
            XCTAssertEqual(labels(deck), ["itemize", "invisibleenv"], "project class: \(projectClass ?? "none")")
        }
        let declaredDeck = "\\documentclass{beamer}\n" + inBrace + "i"
        XCTAssertEqual(labels(Completion.suggestions(in: declaredDeck, caretUTF16: (declaredDeck as NSString).length, result: nil, projectClass: "article")),
                       ["itemize", "invisibleenv"], "the text's own class wins over the root's")
        // `\begin{` gates the same way; the name typed out in full is never hidden.
        let beginBlock = "\\begin{document}\\begin{bl"
        XCTAssertFalse(labels(Completion.suggestions(in: beginBlock, caretUTF16: (beginBlock as NSString).length, result: nil, projectClass: "article")).contains("block"))
        XCTAssertEqual(labels(Completion.suggestions(in: beginBlock, caretUTF16: (beginBlock as NSString).length, result: nil, projectClass: "beamer")).first, "block")
        let beginBlockFull = "\\begin{document}\\begin{block"
        XCTAssertEqual(labels(Completion.suggestions(in: beginBlockFull, caretUTF16: (beginBlockFull as NSString).length, result: nil, projectClass: "article")).first, "block")
        let beginCtx = "\\begin{itemize}\\end{itemize}\\begin{d"
        let b = Completion.suggestions(in: beginCtx, caretUTF16: (beginCtx as NSString).length, result: nil)
        // Every known environment starting with `d`, in table order (computed
        // from the live vocabulary, not a hand-copied snapshot).
        XCTAssertEqual(labels(b), Completion.knownEnvironments.filter { $0.hasPrefix("d") })
        XCTAssertEqual(b.first?.detail, "supported by this compiler")
    }

    func testUnsupportedDocumentCommandsAreMarked() {
        let text = "\\documentclass{article}\n\\usepackage{amsmath}\n\\newwidget\n\\ne"
        let diag = RuntimeV1.Diagnostic(severity: .error,
                                        message: "\\newwidget is not supported by this compiler version; unrestricted TeX math mode is not implemented",
                                        source: nil, recovery: nil)
        let result = RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .recovered, pages: [], diagnostics: [diag], pdfPath: nil)
        // A handful of real `ne`-prefixed vocabulary entries, named explicitly
        // so the cap (maxSuggestions) has room left for the unsupported typed
        // command regardless of how many `\ne*` commands the inventory grows to.
        let neSupported = ["newcommand", "newpage", "neq"]
        let s = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, result: result, supported: neSupported)
        XCTAssertTrue(labels(s).contains("\\newcommand{\\name}[n]{body}"))
        XCTAssertTrue(labels(s).contains("\\newpage"), "a real compiler command must not be marked unsupported")
        XCTAssertEqual(s.first { $0.label == "\\neq" }?.detail, "math · symbol ≠")
        XCTAssertEqual(s.first { $0.label == "\\newwidget" }?.detail, "not supported by the compiler — " + diag.message)

        // Without a diagnostic naming it the mark is still there, without a message.
        let s2 = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, result: nil, supported: neSupported)
        XCTAssertEqual(s2.first { $0.label == "\\newwidget" }?.detail, "not supported by the compiler")

        // The command being typed is not offered as its own completion (only the
        // vocabulary entry it is a prefix of).
        let s3 = Completion.suggestions(in: "\\usepack", caretUTF16: 8, result: nil)
        XCTAssertEqual(labels(s3), ["\\usepackage[options]{a,b,c}"])
        XCTAssertTrue(Completion.suggestions(in: "\\zzq", caretUTF16: 4, result: nil).isEmpty)

        // A custom supported list replaces the default; commands typed in the
        // document that it does not name are still listed after it, marked.
        let s4 = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, result: nil, supported: ["newpage"])
        XCTAssertEqual(labels(s4), ["\\newpage", "\\newwidget"])
        guard s4.count == 2 else { return XCTFail("expected two suggestions, got \(s4.count)") }
        XCTAssertEqual(s4[0].detail, "forces a page break")
        XCTAssertEqual(s4[1].detail, "not supported by the compiler")
    }

    func testReferencesSuggestLabels() {
        let text = "\\label{eq:main}\\label{fig:naïve}\nsee \\ref{fi"
        let s = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, result: nil)
        XCTAssertEqual(labels(s), ["fig:naïve"])
        XCTAssertEqual(s.first?.kind, .reference)
        XCTAssertEqual(s.first?.insertText, "fig:naïve}")
        let e = "\\label{eq:main} \\eqref{eq"
        XCTAssertEqual(labels(Completion.suggestions(in: e, caretUTF16: (e as NSString).length, result: nil)), ["eq:main"])
    }

    // MARK: non-ASCII and invalid carets

    func testNonASCIIWordsAndCaretsAreSafe() {
        let text = "Résumé naïve naïveté 😀 naïve na"
        let ns = text as NSString
        XCTAssertNotEqual(text.utf8.count, ns.length)
        let s = Completion.suggestions(in: text, caretUTF16: ns.length, result: nil)
        XCTAssertEqual(labels(s), ["naïve", "naïveté"])
        XCTAssertEqual(s.first?.detail, "2× in this document")

        // Typing a non-ASCII prefix.
        let t2 = "Résumé Réunion Ré"
        XCTAssertEqual(labels(Completion.suggestions(in: t2, caretUTF16: (t2 as NSString).length, result: nil)), ["Résumé", "Réunion"])

        // A caret inside the emoji's surrogate pair, past the end, or negative yields nothing.
        let emoji = ns.range(of: "😀")
        XCTAssertEqual(emoji.length, 2)
        XCTAssertTrue(Completion.suggestions(in: text, caretUTF16: emoji.location + 1, result: nil).isEmpty)
        XCTAssertTrue(Completion.suggestions(in: text, caretUTF16: ns.length + 5, result: nil).isEmpty)
        XCTAssertTrue(Completion.suggestions(in: text, caretUTF16: -1, result: nil).isEmpty)
        XCTAssertTrue(Completion.suggestions(in: "", caretUTF16: 0, result: nil).isEmpty)
        // Non-letter scalars adjacent to a word never form a token.
        XCTAssertTrue(Completion.suggestions(in: "ab—", caretUTF16: 3, result: nil).isEmpty)
        XCTAssertNil(Completion.token(in: "ab—", caretUTF16: 3))

        // Completion range: includes the backslash; empty at a caret with no token;
        // clamped when out of range; UTF-16 exact after a multi-byte prefix.
        let cmd = "naïve \\sec"
        XCTAssertEqual(Completion.completionRange(in: cmd, caretUTF16: (cmd as NSString).length), NSRange(location: 6, length: 4))
        XCTAssertEqual(Completion.completionRange(in: "abc ", caretUTF16: 4), NSRange(location: 4, length: 0))
        XCTAssertEqual(Completion.completionRange(in: "abc", caretUTF16: 99), NSRange(location: 0, length: 3))
        XCTAssertEqual(Completion.completionRange(in: text, caretUTF16: emoji.location + 1), NSRange(location: emoji.location + 1, length: 0))
    }

    // MARK: NSTextView integration

    @MainActor
    func testCompletingTextViewUsesSubclassAndBackslashRange() throws {
        let scroll = CompletingTextView.scrollable()
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        tv.projectDocumentClass = { "article" } // the fixture's project class, as `CompletionTestVocabulary` assumes
        tv.string = "\\begin{document}\nnaïve \\se"
        let end = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: end, length: 0))
        XCTAssertEqual(tv.rangeForUserCompletion, NSRange(location: end - 3, length: 3))
        var index = -1
        let items = tv.completions(forPartialWordRange: tv.rangeForUserCompletion, indexOfSelectedItem: &index)
        // AppKit's list carries the insert texts (no argument shapes), in the pure function's order.
        XCTAssertEqual(items, CompletionTestVocabulary.insertTexts(forPrefix: "se"))
        XCTAssertEqual(items, Completion.suggestions(in: tv.string, caretUTF16: end, result: nil, projectClass: "article").map(\.insertText))
        XCTAssertEqual(index, 0)
        // `\e` offers the unclosed environment first.
        tv.string = "\\begin{document}\n\\e"
        tv.setSelectedRange(NSRange(location: 19, length: 0))
        XCTAssertEqual(tv.completions(forPartialWordRange: tv.rangeForUserCompletion, indexOfSelectedItem: &index)?.first, "\\end{document}")
        // No token: nil (AppKit shows nothing rather than an empty popup).
        tv.string = "abc "
        tv.setSelectedRange(NSRange(location: 4, length: 0))
        XCTAssertNil(tv.completions(forPartialWordRange: tv.rangeForUserCompletion, indexOfSelectedItem: &index))
        // Stale or absurd ranges (text shrank after the range was computed) are refused, not trapped.
        for bad in [NSRange(location: 2, length: 10), NSRange(location: NSNotFound, length: 0),
                    NSRange(location: 99, length: 0), NSRange(location: -1, length: 1)] {
            XCTAssertNil(tv.completions(forPartialWordRange: bad, indexOfSelectedItem: &index), "\(bad)")
            tv.insertCompletion("\\section", forPartialWordRange: bad, movement: NSReturnTextMovement, isFinal: true)
            XCTAssertEqual(tv.string, "abc ", "stale range \(bad) must not splice text")
        }
        // A valid final insertion replaces exactly the partial token.
        tv.string = "x \\se"
        tv.setSelectedRange(NSRange(location: 5, length: 0))
        tv.insertCompletion("\\section", forPartialWordRange: tv.rangeForUserCompletion, movement: NSReturnTextMovement, isFinal: true)
        XCTAssertEqual(tv.string, "x \\section")
    }

    // MARK: fault tolerance

    func testNeverThrowsOnMalformedInput() {
        let junk = ["\\", "\\\\\\", "\\begin{", "\\begin{}\\end{", "{{{}}}}", "\\ref{", "\\end{x}\\end{x}", "a\u{FFFD}b", String(repeating: "\\", count: 50)]
        for text in junk {
            for caret in -1...((text as NSString).length + 1) {
                _ = Completion.suggestions(in: text, caretUTF16: caret, result: nil)
                _ = Completion.completionRange(in: text, caretUTF16: caret)
            }
        }
        let empty = RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .failed, pages: [], diagnostics: [
            .init(severity: .error, message: "\\", source: nil, recovery: nil),
            .init(severity: .error, message: "no command here", source: nil, recovery: nil),
        ], pdfPath: nil)
        XCTAssertFalse(Completion.suggestions(in: "\\x \\", caretUTF16: 4, result: empty).isEmpty)
    }

    // MARK: snippets (argument shapes, environments, label keys)

    func testVocabularySnippetsFollowArgumentShapes() {
        let v = Completion.Vocabulary.byName
        // `stops`: the later braces then the end, visited with Tab (SnippetTests).
        XCTAssertEqual(v["section"]?.snippet, .init(text: "\\section{}", caretUTF16: 9, stops: [10]))
        XCTAssertEqual(v["textbf"]?.snippet, .init(text: "\\textbf{}", caretUTF16: 8, stops: [9]))
        XCTAssertEqual(v["emph"]?.snippet, .init(text: "\\emph{}", caretUTF16: 6, stops: [7]))
        XCTAssertEqual(v["frac"]?.snippet, .init(text: "\\frac{}{}", caretUTF16: 6, stops: [8, 9]))
        XCTAssertEqual(v["newcommand"]?.snippet, .init(text: "\\newcommand{}{}", caretUTF16: 12, stops: [14, 15]), "optional [n] dropped")
        XCTAssertEqual(v["documentclass"]?.snippet, .init(text: "\\documentclass{}", caretUTF16: 15, stops: [16]), "leading [options] dropped")
        XCTAssertEqual(v["begin"]?.snippet, .init(text: "\\begin{}", caretUTF16: 7, stops: [8]))
        XCTAssertEqual(v["label"]?.snippet, .init(text: "\\label{}", caretUTF16: 7, stops: [8]))
        for plain in ["item", "par", "\\", "alpha", "int"] { XCTAssertNil(v[plain]?.snippet, plain) }
        // Suggestions carry the shape; a project override of a builtin does not
        // (the macro's arguments are unknown).
        let s = Completion.suggestions(in: "x \\sect", caretUTF16: 7, metadata: nil)
        XCTAssertEqual(s.first?.snippet, .init(text: "\\section{}", caretUTF16: 9, stops: [10]))
        XCTAssertEqual(s.first?.insertText, "\\section")
        XCTAssertNil(Completion.suggestions(in: "x \\sec", caretUTF16: 6, metadata: nil).first?.snippet, "\\sec (exact) takes no argument")
        let versions = ["main.tex": 1]
        let m = try! Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("section", 1, 0, false)]),
                                                                 category: .command, editorRevision: 1, expectedSourceVersions: versions)
        XCTAssertNil(Completion.suggestions(in: "x \\sect", caretUTF16: 7, metadata: m).first?.snippet)
    }

    func testEnvironmentSnippetKeepsIndentationAndClosingStaysExact() {
        let text = "\\begin{document}\n  \\begin{it"
        let s = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: nil)
        XCTAssertEqual(s.map(\.label), ["itemize"])
        XCTAssertEqual(s.first?.insertText, "itemize}")
        // Templates and tab stops: SnippetTests. A list starts with its first `\item`.
        XCTAssertEqual(s.first?.snippet, .init(text: "itemize}\n  \\item \n  \\end{itemize}", caretUTF16: 17, stops: [33]))
        let tabbed = "\t\\begin{eq"
        XCTAssertEqual(Completion.suggestions(in: tabbed, caretUTF16: (tabbed as NSString).length, metadata: nil).first?.snippet,
                       .init(text: "equation}\n\t\n\t\\end{equation}", caretUTF16: 11, stops: [27]))
        let flat = "\\begin{cen"
        XCTAssertEqual(Completion.suggestions(in: flat, caretUTF16: 10, metadata: nil).first?.snippet,
                       .init(text: "center}\n\n\\end{center}", caretUTF16: 8, stops: [21]))
        XCTAssertEqual(Completion.lineIndent(in: "a\n  \tb", beforeByte: 6), "  \t")
        XCTAssertEqual(Completion.lineIndent(in: "abc", beforeByte: 3), "")
        // `\end{` completes the innermost open environment exactly, no skeleton.
        let closing = "  \\begin{document}\n  \\begin{itemize}\n\\end{it"
        let c = Completion.suggestions(in: closing, caretUTF16: (closing as NSString).length, metadata: nil)
        XCTAssertEqual(c.map(\.label), ["itemize"])
        XCTAssertEqual(c.first?.insertText, "itemize}")
        XCTAssertNil(c.first?.snippet)
        let closer = "\\begin{itemize}\n\\e"
        let e = Completion.suggestions(in: closer, caretUTF16: (closer as NSString).length, metadata: nil)
        XCTAssertEqual(e.first?.insertText, "\\end{itemize}")
        XCTAssertNil(e.first?.snippet)
    }

    func testLabelKeyIsDerivedFromTheEnclosingHeadingAndMadeUnique() throws {
        let text = "\\section{The \\emph{Best} Idea!}\nText \\label{"
        let s = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: nil)
        XCTAssertEqual(s.map(\.label), ["sec:the-best-idea"])
        let first = try XCTUnwrap(s.first)
        XCTAssertEqual(first.insertText, "sec:the-best-idea}")
        XCTAssertEqual(first.kind, .reference)
        XCTAssertEqual(first.detail, "unique key for \\section{The \\emph{Best} Idea!}")
        XCTAssertNil(first.snippet)

        // Taken in the document and in the project index: next free suffix.
        let versions = ["main.tex": 1]
        let m = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("sec:the-best-idea-2", 1, 0, false)]),
                                                                category: .reference, editorRevision: 5, expectedSourceVersions: versions)
        let taken = "\\label{sec:the-best-idea}" + text
        let u = Completion.suggestions(in: taken, caretUTF16: (taken as NSString).length, metadata: m)
        XCTAssertEqual(u.map(\.label), ["sec:the-best-idea-3"])
        let uFirst = try XCTUnwrap(u.first)
        XCTAssertEqual(uFirst.detail, "unique key for \\section{The \\emph{Best} Idea!} (sec:the-best-idea is taken) · checked against 1 project label · revision 5")

        // The nearest heading before the caret wins; headings after it do not count.
        let sub = "\\section{A}\n\\subsection{Résumé 2024}\n\\label{se"
        XCTAssertEqual(Completion.suggestions(in: sub, caretUTF16: (sub as NSString).length, metadata: nil).map(\.label), ["sec:résumé-2024"])
        let after = "\\label{\n\\section{Later}"
        XCTAssertTrue(Completion.suggestions(in: after, caretUTF16: 7, metadata: nil).isEmpty)
        XCTAssertTrue(Completion.suggestions(in: "\\label{", caretUTF16: 7, metadata: nil).isEmpty)
        // A typed prefix the key does not start with yields nothing.
        let fig = "\\section{Intro}\\label{fig"
        XCTAssertTrue(Completion.suggestions(in: fig, caretUTF16: (fig as NSString).length, metadata: nil).isEmpty)
        // Heading with an unbalanced or multi-line brace group is ignored.
        XCTAssertNil(Completion.enclosingHeading(in: "\\section{Open\n}", beforeByte: 15))
        XCTAssertEqual(Completion.enclosingHeading(in: "\\subsection{A {b} c}", beforeByte: 20), .init(command: "subsection", title: "A {b} c"))
        XCTAssertEqual(Completion.kebabCase("  Hello,   World -- 42 "), "hello-world-42")
        XCTAssertEqual(Completion.kebabCase("\\textbf{Bold}\\ Ünïcode"), "bold-ünïcode")
        XCTAssertEqual(Completion.kebabCase("!!!"), "")
    }

    // MARK: vocabulary generated from the compiler's inventory

    private static let repoRoot = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()

    /// The bundled `Resources/supported-latex.json` is a byte-identical copy
    /// of `crates/compiler/supported/supported-latex.json` (kept by
    /// `apps/mac/scripts/sync-supported-latex.sh`; the compiler crate gates
    /// its own file against `flashtex-compiler --supported json`). Compared
    /// by sha256 whenever the repository checkout is available.
    func testBundledInventoryMatchesTheCompiler() throws {
        let bundled = try Completion.Vocabulary.loadInventoryData()
        XCTAssertFalse(bundled.isEmpty)
        let compiler = Self.repoRoot.appendingPathComponent("crates/compiler/supported/supported-latex.json")
        guard FileManager.default.fileExists(atPath: compiler.path) else {
            throw XCTSkip("no repository checkout at \(compiler.path); the sha256 comparison needs the compiler's file")
        }
        let expected = try Data(contentsOf: compiler)
        let hash = { (d: Data) in SHA256.hash(data: d).map { String(format: "%02x", $0) }.joined() }
        XCTAssertEqual(hash(bundled), hash(expected),
                       "apps/mac/Sources/FlashTeXMac/Resources/supported-latex.json drifted from crates/compiler/supported/supported-latex.json; run apps/mac/scripts/sync-supported-latex.sh")
        // The bundled copy is what the vocabulary decoded (never a fallback).
        XCTAssertEqual(Completion.Vocabulary.inventory.schema, Completion.Vocabulary.Inventory.schema)
        XCTAssertEqual(Completion.Vocabulary.inventory.generator, "flashtex-compiler --supported json")
        XCTAssertFalse(Completion.Vocabulary.inventory.commands.isEmpty)
        // A schema the editor does not know is refused, never partially used.
        let other = String(data: bundled, encoding: .utf8)!.replacingOccurrences(of: "flashtex-supported-latex/1", with: "flashtex-supported-latex/2")
        XCTAssertThrowsError(try Completion.Vocabulary.decodeInventory(other.data(using: .utf8)!))
    }

    // MARK: class-scoped commands

    /// `\documentclass[options]{class}` read from the document's own text —
    /// the only place the Mac side can learn the class from.
    func testDocumentClassIsReadFromThePreamble() {
        XCTAssertEqual(Completion.documentClass(in: "\\documentclass{beamer}\n\\begin{document}\n"), "beamer")
        XCTAssertEqual(Completion.documentClass(in: "\\documentclass[11pt,a4paper]{article}\n"), "article")
        XCTAssertEqual(Completion.documentClass(in: "\\documentclass [11pt] {letter}\n"), "letter")
        XCTAssertEqual(Completion.documentClass(in: "% a preamble\n\\RequirePackage{fix}\n\\documentclass{book}\n"), "book")
        // Nothing to read: a fragment, a commented-out declaration, a
        // declaration that only appears after the preamble (prose about
        // LaTeX), and an unterminated one.
        XCTAssertNil(Completion.documentClass(in: "\\section{Intro}\nBody.\n"))
        XCTAssertNil(Completion.documentClass(in: "% \\documentclass{beamer}\n\\begin{document}\n"))
        XCTAssertNil(Completion.documentClass(in: "\\begin{document}\n\\documentclass{beamer} is the first line.\n"))
        XCTAssertNil(Completion.documentClass(in: "\\documentclass{beamer\n"))
        XCTAssertNil(Completion.documentClass(in: ""))
    }

    /// The regression this gate exists for: beamer's `\frametitle` and
    /// `\alert` are text-mode entries, so in table order they lead the popup
    /// — `\fra` meant `\frametitle` rather than `\frac`, the most-used
    /// command in LaTeX, in every document. A class-scoped command is hidden
    /// in a document of another class. The class is the text's own
    /// `\documentclass`, else the project root document's (`projectClass`:
    /// an included chapter or slide file declares none), else unknown —
    /// which gates nothing, because two-thirds of real `.tex` files declare
    /// no class and the ones that use `\frametitle` are beamer's own slide
    /// files.
    func testClassScopedCommandsAreOfferedOnlyUnderTheirOwnClass() throws {
        let deck = "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}\n"
        let article = "\\documentclass{article}\n\\begin{document}\n"
        func offered(_ prefix: String, in preamble: String, projectClass: String? = nil) -> [String] {
            let text = preamble + prefix
            return Completion.suggestions(in: text, caretUTF16: (text as NSString).length, result: nil, projectClass: projectClass)
                .map(\.insertText)
        }
        // The inventory really does carry the scope this reads.
        XCTAssertEqual(Completion.Vocabulary.byName["frametitle"]?.requiresClass, "beamer")
        XCTAssertEqual(Completion.Vocabulary.byName["opening"]?.requiresClass, "letter")
        XCTAssertNil(Completion.Vocabulary.byName["frac"]?.requiresClass, "\\frac is universal")

        // A beamer deck offers the beamer family, ahead of `\frac` by table order.
        XCTAssertEqual(offered("\\fra", in: deck).first, "\\frametitle")
        XCTAssertTrue(offered("\\al", in: deck).contains("\\alert"))
        // An article does not.
        XCTAssertEqual(offered("\\fra", in: article), ["\\frac"])
        XCTAssertFalse(offered("\\al", in: article).contains("\\alert"))
        XCTAssertFalse(offered("\\op", in: article).contains("\\opening"), "letter.cls scopes the same way")

        // An included file declares no class of its own, so the project's
        // root document decides: a slide file of a beamer project gets
        // `\frametitle` on first use, the same file in an article project
        // does not.
        let slide = "\\begin{frame}\n"
        XCTAssertEqual(offered("\\fra", in: slide, projectClass: "beamer").first, "\\frametitle")
        XCTAssertTrue(offered("\\al", in: slide, projectClass: "beamer").contains("\\alert"))
        XCTAssertEqual(offered("\\fra", in: slide, projectClass: "article"), ["\\frac"])
        XCTAssertFalse(offered("\\al", in: slide, projectClass: "article").contains("\\alert"))
        XCTAssertFalse(offered("\\op", in: slide, projectClass: "article").contains("\\opening"))
        XCTAssertTrue(offered("\\op", in: slide, projectClass: "letter").contains("\\opening"))
        // A file's own declaration wins over the root's.
        XCTAssertEqual(offered("\\fra", in: article, projectClass: "beamer"), ["\\frac"])
        XCTAssertEqual(offered("\\fra", in: deck, projectClass: "article").first, "\\frametitle")

        // No class in the text and no project to read one from: nothing to
        // gate on, so nothing is hidden (the compiler's `offered_in_class`
        // reads `None` the same way).
        XCTAssertEqual(offered("\\fra", in: "").first, "\\frametitle")
        XCTAssertTrue(offered("\\al", in: "").contains("\\alert"))
        XCTAssertTrue(offered("\\op", in: "").contains("\\opening"))

        // Two escape hatches keep the command reachable in a document of
        // another class: the name typed out in full, and a file that already
        // uses it (completed from its own text, below the universal entry).
        XCTAssertEqual(offered("\\frametitle", in: article).first, "\\frametitle")
        let fragment = article + "\\frametitle{Earlier}\n\\fra"
        XCTAssertEqual(Completion.suggestions(in: fragment, caretUTF16: (fragment as NSString).length, result: nil)
                         .map(\.insertText), ["\\frac", "\\frametitle"],
                       "a document that already uses it completes it from its own text, below the universal entry")
    }

    /// The project layer resolves the class an included file inherits:
    /// `ProjectDocuments.entryDocumentClass` reads the entry document's
    /// preamble whichever tab is active, follows the entry's edits, and is
    /// nil when the entry declares no class — which completion then treats
    /// as unknown.
    @MainActor
    func testIncludedFileInheritsTheEntryDocumentClass() {
        let model = ShellModel()
        let entry = { (cls: String) in "\\documentclass{\(cls)}\n\\begin{document}\n\\input{slides}\n\\end{document}\n" }
        model.documents = [.init(path: "main.tex", text: entry("beamer")), .init(path: "slides.tex", text: "\\begin{frame}\n\\fra")]
        model.activePath = "slides.tex"
        XCTAssertEqual(model.project.entryPath, "main.tex")
        XCTAssertEqual(model.project.entryDocumentClass, "beamer")
        func offered() -> [String] {
            let text = model.activeText
            return Completion.suggestions(in: text, caretUTF16: (text as NSString).length, result: nil,
                                          projectClass: model.project.entryDocumentClass).map(\.insertText)
        }
        XCTAssertEqual(offered().first, "\\frametitle", "a beamer deck's slide file completes \\frametitle on first use")
        XCTAssertEqual(model.project.entryDocumentClass, "beamer", "cached: the documents did not change")

        // The entry's preamble changes while another tab is active: the
        // cache is keyed on the documents revision, so the answer follows.
        model.documents[0].text = entry("article")
        XCTAssertEqual(model.project.entryDocumentClass, "article")
        XCTAssertEqual(offered(), ["\\frac"], "the same slide file in an article project gets no \\frametitle")

        // An entry that declares no class leaves the class unknown.
        model.documents[0].text = "\\input{slides}\n"
        XCTAssertNil(model.project.entryDocumentClass)
        XCTAssertEqual(offered().first, "\\frametitle", "unknown gates nothing")
    }

    /// Every rendered inventory command is offered exactly once with the
    /// right mode; the hand-written overrides name inventory commands; the
    /// derived tables and the offer order follow the file.
    func testVocabularyIsGeneratedFromTheInventory() throws {
        typealias V = Completion.Vocabulary
        let inventory = V.inventory
        let entries = V.entries
        let rendered = inventory.commands.filter(\.renders)
        XCTAssertGreaterThan(inventory.commands.count, rendered.count, "the inventory names commands that parse without rendering (\\check, \\breve)")

        // (1) Exactly once, with the right mode: text if the compiler accepts
        //     it in text mode (or both), math otherwise. Control symbols other
        //     than `\\` (`\,` `\;` …) are inventory commands the popup does not offer.
        XCTAssertEqual(Set(entries.map(\.name)).count, entries.count, "no duplicate names")
        var modes: [String: Set<V.Mode>] = [:]
        for c in rendered { modes[c.name, default: []].insert(c.mode) }
        for (name, accepted) in modes {
            let entry = V.byName[name]
            if name != "\\", rendered.contains(where: { $0.name == name && $0.origin == .controlSymbol }) {
                XCTAssertNil(entry, "control symbol \\\(name) is not completed")
                continue
            }
            let e = try XCTUnwrap(entry, "rendered command \\\(name) is missing from the vocabulary")
            XCTAssertEqual(e.mode, accepted.contains(.text) ? .text : .math, name)
            XCTAssertEqual(e.mathDescription != nil, accepted == [.text, .math], "\\\(name) carries the math description only when it works in both modes")
            if accepted == [.text, .math] {
                XCTAssertEqual(e.description, rendered.first { $0.name == name && $0.mode == .text }?.description)
                XCTAssertEqual(e.mathDescription, rendered.first { $0.name == name && $0.mode == .math }?.description)
                XCTAssertTrue(e.detail.contains(" · in math: "), e.detail)
            }
        }
        for e in entries {
            XCTAssertTrue(modes[e.name] != nil, "stale entry \\\(e.name): not a rendered inventory command")
            XCTAssertFalse(e.description.isEmpty, e.name)
            XCTAssertFalse(e.description.contains("\n"), "one line: \(e.name)")
            XCTAssertEqual(e.glyph != nil, e.origin == .mathSymbol, "glyphs come with math symbols only: \(e.name)")
        }
        for c in inventory.commands where !c.renders {
            XCTAssertNil(V.byName[c.name], "\\\(c.name) does not render and must not complete")
        }
        XCTAssertEqual(V.byName["\\"]?.mode, .text)
        XCTAssertEqual(V.byName["textbf"]?.mode, .text, "\\textbf works in both modes: one text entry")
        XCTAssertEqual(V.byName["frac"]?.mode, .math)

        // (2) Overrides name inventory commands.
        for name in V.argumentOverrides.keys { XCTAssertNotNil(modes[name], "argument override for unknown \\\(name)") }
        for name in V.snippetOverrides.keys { XCTAssertNotNil(modes[name], "snippet override for unknown \\\(name)") }
        for (name, arguments) in V.argumentOverrides { XCTAssertEqual(V.byName[name]?.arguments, arguments) }
        for c in rendered where V.argumentOverrides[c.name] == nil {
            guard let e = V.byName[c.name], e.mode == c.mode else { continue }
            XCTAssertEqual(e.arguments, c.arguments, "\\\(c.name) shows the compiler's argument shape")
        }

        // Derived tables mirror the inventory in file order.
        let byOrigin = { (o: V.Origin) in rendered.filter { $0.origin == o }.map(\.name) }
        XCTAssertEqual(V.symbols.map(\.0), byOrigin(.mathSymbol))
        XCTAssertEqual(V.symbols.map(\.1), rendered.filter { $0.origin == .mathSymbol }.map { $0.glyph ?? "" })
        XCTAssertEqual(V.operatorNames, byOrigin(.mathOperator))
        XCTAssertEqual(V.environments, inventory.environments.map(\.name))
        XCTAssertEqual(Completion.knownEnvironments, V.environments)
        XCTAssertTrue(V.environments.contains("equation") && V.environments.contains("pmatrix"))

        // Offer order: text entries in file order, then structures, operators, symbols.
        var text: [String] = []
        for c in rendered where c.mode == .text && c.origin != .controlSymbol && !text.contains(c.name) { text.append(c.name) }
        let structures = byOrigin(.mathStructure).filter { V.byName[$0]?.mode == .math }
        XCTAssertEqual(entries.map(\.name), text + ["\\"] + structures + byOrigin(.mathOperator) + byOrigin(.mathSymbol))
        // The first text-mode command in file order, whatever the inventory
        // currently leads with (not necessarily `section`).
        XCTAssertEqual(entries.first?.name, text.first)
    }

    /// Hover documentation names only commands the compiler inventories or
    /// the explicit list of standard LaTeX it documents beyond the compiler.
    func testCommandDocsNameOnlyKnownCommands() {
        typealias Docs = EditorIntelligence.CommandDocs
        let known = Set(Completion.Vocabulary.inventory.commands.filter(\.renders).map(\.name))
        for name in Docs.table.keys {
            XCTAssertTrue(known.contains(name) || Docs.beyondCompiler.contains(name),
                          "CommandDocs documents \\\(name), which the compiler does not inventory; add it to beyondCompiler or drop it")
        }
        for name in Docs.beyondCompiler {
            XCTAssertFalse(known.contains(name), "\\\(name) is rendered by the compiler now; remove it from beyondCompiler")
            XCTAssertNotNil(Docs.table[name], "beyondCompiler names \\\(name) without documentation")
        }
        let environments = Set(Completion.Vocabulary.environments.map { $0.hasSuffix("*") ? String($0.dropLast()) : $0 })
        for name in Docs.environments.keys {
            XCTAssertTrue(environments.contains(name) || Docs.environmentsBeyondCompiler.contains(name),
                          "CommandDocs documents environment \(name), which the compiler does not inventory")
        }
        for name in Docs.environmentsBeyondCompiler {
            XCTAssertFalse(environments.contains(name), "environment \(name) is supported now; remove it from environmentsBeyondCompiler")
            XCTAssertNotNil(Docs.environments[name])
        }
    }

    // MARK: revision-bound metadata (runtime-v1 result and project-index reply)

    private func result(revision: Int, _ messages: [String]) -> RuntimeV1.CompileResult {
        RuntimeV1.CompileResult(projectId: "p", revision: revision, status: .recovered, pages: [],
                                diagnostics: messages.map { .init(severity: .warning, message: $0, source: nil, recovery: nil) },
                                pdfPath: nil)
    }

    /// Wire shape of `crates/preview-controller/STDIO.md` `complete` replies.
    private func indexReply(versions: [String: Int], names: [(String, defs: Int, occ: Int, truncated: Bool)]) -> Data {
        let items = names.map { n -> [String: Any] in
            func loc(_ i: Int) -> [String: Any] { ["path": i % 2 == 0 ? "main.tex" : "parts/body.tex", "revision": versions["main.tex"] ?? 1, "start_byte": i * 10, "end_byte": i * 10 + 3] }
            return ["name": n.0, "definitions": (0..<n.defs).map(loc), "occurrences": (0..<n.occ).map(loc), "locations_truncated": n.truncated]
        }
        return try! JSONSerialization.data(withJSONObject: ["source_versions": versions, "completions": items])
    }

    func testMetadataFromCompileResultParsesDiagnosticsAndCaps() {
        let r = result(revision: 7, [
            "\\newwidget is not supported by this compiler version; unrestricted TeX math mode is not implemented",
            "\\newwidget duplicate mention is ignored",
            "undefined reference 'eq:missing'",
            "environment 'mysteryenv' is not implemented; its body is typeset as plain text",
            "packages amsmath are recognised but not implemented",
            "\\", // no name
        ])
        let m = Completion.Metadata.from(r)
        XCTAssertEqual(m.origin, .compileResult(projectId: "p"))
        XCTAssertEqual(m.revision, 7)
        XCTAssertEqual(m.diagnosticsByCommand, ["newwidget": r.diagnostics[0].message])
        XCTAssertEqual(m.unresolvedReferences, ["eq:missing"])
        XCTAssertEqual(m.diagnosticsByEnvironment, ["mysteryenv": r.diagnostics[3].message])
        XCTAssertFalse(m.truncated)
        XCTAssertTrue(m.labels.isEmpty && m.citations.isEmpty && m.commands.isEmpty, "runtime-v1 carries no vocabulary")

        // Caps: at most 256 entries, 512-character messages.
        let long = "\\big " + String(repeating: "x", count: 2_000)
        let many = result(revision: 1, (0..<300).map { "undefined reference 'k\($0)'" } + [long])
        let capped = Completion.Metadata.from(many)
        XCTAssertEqual(capped.unresolvedReferences.count, Completion.Metadata.Limits.maxDiagnostics)
        XCTAssertTrue(capped.truncated)
        XCTAssertNil(capped.diagnosticsByCommand["big"], "dropped by the entry cap")
        let one = Completion.Metadata.from(result(revision: 1, [long]))
        XCTAssertEqual(one.diagnosticsByCommand["big"]?.count, Completion.Metadata.Limits.maxMessageCharacters + 1)
        XCTAssertTrue(one.diagnosticsByCommand["big"]!.hasSuffix("…"))
    }

    func testProjectIndexReplyDecodesIsBoundedAndRefusesStaleVersions() throws {
        let versions = ["main.tex": 4, "parts/body.tex": 2]
        let data = indexReply(versions: versions, names: [("sec:intro", 1, 3, false), ("sec:dup", 2, 0, true), ("sec:unresolved", 0, 1, false)])
        let m = try Completion.Metadata.decodeProjectIndexReply(data, category: .reference, editorRevision: 9, expectedSourceVersions: versions)
        XCTAssertEqual(m.origin, .projectIndex(sourceVersions: versions))
        XCTAssertEqual(m.revision, 9)
        XCTAssertEqual(m.labels.map(\.name), ["sec:intro", "sec:dup", "sec:unresolved"])
        guard m.labels.count == 3 else { return XCTFail("expected three labels, got \(m.labels.count)") }
        XCTAssertEqual(m.labels[0], .init(name: "sec:intro", definitions: 1, occurrences: 3, locationsTruncated: false, definedIn: "main.tex"))
        XCTAssertEqual(m.labels[1].definitions, 2)
        XCTAssertTrue(m.labels[1].locationsTruncated)
        XCTAssertNil(m.labels[2].definedIn)
        XCTAssertEqual(m.labels[0].detail(noun: "defined", revision: 9), "defined in main.tex · 3 uses · revision 9")
        XCTAssertEqual(m.labels[1].detail(noun: "defined", revision: 9), "defined in main.tex (+1 more) · revision 9")
        XCTAssertEqual(m.labels[2].detail(noun: "defined", revision: 9), "unresolved in the project index · 1 use · revision 9")
        XCTAssertFalse(m.truncated)

        // Other categories land in their own list; unknown categories are refused.
        let cites = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("knuth84", 1, 2, false)]),
                                                                    category: .citation, editorRevision: 9, expectedSourceVersions: versions)
        XCTAssertEqual(cites.citations.map(\.name), ["knuth84"])
        XCTAssertTrue(cites.labels.isEmpty)
        XCTAssertThrowsError(try Completion.Metadata.decodeProjectIndexReply(data, category: .word, editorRevision: 9, expectedSourceVersions: versions))

        // A reply for another snapshot is stale: refused, never partially used.
        XCTAssertThrowsError(try Completion.Metadata.decodeProjectIndexReply(data, category: .reference, editorRevision: 9,
                                                                             expectedSourceVersions: ["main.tex": 5, "parts/body.tex": 2])) { error in
            XCTAssertEqual(error as? Completion.Metadata.DecodeError,
                           .staleSourceVersions(expected: ["main.tex": 5, "parts/body.tex": 2], got: versions))
        }
        // Bounds: item count, name bytes, reply bytes, malformed JSON.
        let big = indexReply(versions: versions, names: (0..<150).map { ("l\($0)", 1, 0, false) } + [(String(repeating: "n", count: 5_000), 1, 0, false)])
        let bounded = try Completion.Metadata.decodeProjectIndexReply(big, category: .reference, editorRevision: 1, expectedSourceVersions: versions)
        XCTAssertEqual(bounded.labels.count, Completion.Metadata.Limits.maxItemsPerCategory)
        XCTAssertTrue(bounded.truncated)
        let longName = indexReply(versions: versions, names: [(String(repeating: "n", count: 5_000), 1, 0, false), ("ok", 1, 0, false)])
        let dropped = try Completion.Metadata.decodeProjectIndexReply(longName, category: .reference, editorRevision: 1, expectedSourceVersions: versions)
        XCTAssertEqual(dropped.labels.map(\.name), ["ok"])
        XCTAssertTrue(dropped.truncated)
        let huge = Data(count: Completion.Metadata.Limits.maxReplyBytes + 1)
        XCTAssertThrowsError(try Completion.Metadata.decodeProjectIndexReply(huge, category: .reference, editorRevision: 1, expectedSourceVersions: versions)) {
            XCTAssertEqual($0 as? Completion.Metadata.DecodeError, .tooLarge(bytes: huge.count))
        }
        XCTAssertThrowsError(try Completion.Metadata.decodeProjectIndexReply(Data("{".utf8), category: .reference, editorRevision: 1, expectedSourceVersions: versions))
    }

    func testMetadataBindsOnlyToItsRevisionAndMergesSameRevision() throws {
        let compiled = Completion.Metadata.from(result(revision: 3, ["undefined reference 'eq:a'"]))
        XCTAssertNil(compiled.bound(to: nil), "unknown editor revision binds nothing")
        XCTAssertNil(compiled.bound(to: 4), "older metadata than the caret's revision is refused")
        XCTAssertNil(compiled.bound(to: 2), "metadata newer than the text is not this text's either")
        XCTAssertEqual(compiled.bound(to: 3), compiled)

        let versions = ["main.tex": 1]
        let index3 = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("eq:b", 1, 1, false)]),
                                                                     category: .reference, editorRevision: 3, expectedSourceVersions: versions)
        let index4 = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("eq:c", 1, 1, false)]),
                                                                     category: .reference, editorRevision: 4, expectedSourceVersions: versions)
        XCTAssertNil(compiled.merged(with: index4), "metadata never straddles revisions")
        let merged = try XCTUnwrap(compiled.merged(with: index3))
        XCTAssertEqual(merged.revision, 3)
        XCTAssertEqual(merged.labels.map(\.name), ["eq:b"])
        XCTAssertEqual(merged.unresolvedReferences, ["eq:a"])
        XCTAssertEqual(merged.origin, compiled.origin, "the receiver's origin is kept")
    }

    func testMetadataContributesProjectLabelsCitationsDeclaredCommandsAndDiagnostics() throws {
        let versions = ["main.tex": 1, "parts/body.tex": 1]
        let labelMeta = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("fig:river", 1, 2, false)]),
                                                                     category: .reference, editorRevision: 5, expectedSourceVersions: versions)
        let cites = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("knuth84", 1, 1, false), ("lamport94", 0, 1, false)]),
                                                                    category: .citation, editorRevision: 5, expectedSourceVersions: versions)
        let cmds = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("newterm", 1, 4, false), ("section", 1, 0, false)]),
                                                                   category: .command, editorRevision: 5, expectedSourceVersions: versions)
        let compiled = Completion.Metadata.from(result(revision: 5, [
            "undefined reference 'fig:local'",
            "environment 'mysteryenv' is not implemented; its body is typeset as plain text",
            "\\newwidget is not supported by this compiler version; unrestricted TeX math mode is not implemented",
        ]))
        let m = try XCTUnwrap(labelMeta.merged(with: cites)?.merged(with: cmds)?.merged(with: compiled))

        // References: document labels first (with the compiler's verdict), then project-wide ones.
        let ref = "\\label{fig:local}\\newwidget \\begin{mysteryenv} \\ref{fi"
        let refs = Completion.suggestions(in: ref, caretUTF16: (ref as NSString).length, metadata: m)
        XCTAssertEqual(labels(refs), ["fig:local", "fig:river"])
        guard refs.count == 2 else { return XCTFail("expected two references, got \(refs.count)") }
        XCTAssertEqual(refs[0].detail, "\\label in this document — undefined when revision 5 compiled")
        XCTAssertEqual(refs[1].detail, "defined in main.tex · 2 uses · revision 5")
        XCTAssertEqual(refs[1].insertText, "fig:river}")

        // Citations: `\bibitem` keys in the document, then project-index keys (unresolved ones say so).
        let cite = "\\bibitem{local01} text \\citep{"
        let c = Completion.suggestions(in: cite, caretUTF16: (cite as NSString).length, metadata: m)
        XCTAssertEqual(labels(c), ["local01", "knuth84", "lamport94"])
        XCTAssertTrue(c.allSatisfy { $0.kind == .citation })
        guard c.count == 3 else { return XCTFail("expected three citations, got \(c.count)") }
        XCTAssertEqual(c[0].detail, "\\bibitem in this document")
        XCTAssertEqual(c[2].detail, "cited but not defined in a declared bibliography source · 1 use · revision 5")
        XCTAssertEqual(c[1].detail, "defined in main.tex (kind not reported by this helper) · 1 use · revision 5", "no snapshot kinds bound here")
        XCTAssertEqual(c[1].insertText, "knuth84}")
        XCTAssertTrue(Completion.suggestions(in: cite, caretUTF16: (cite as NSString).length, metadata: nil).map(\.label) == ["local01"])
        for command in Completion.citationCommands {
            let t = "\\\(command){kn"
            XCTAssertEqual(labels(Completion.suggestions(in: t, caretUTF16: (t as NSString).length, metadata: m)), ["knuth84"], command)
        }

        // Commands: declared project commands after the supported list, deduplicated
        // against it; unsupported document commands carry the bound diagnostic.
        let cmd = ref + "g} \\new"
        let s = Completion.suggestions(in: cmd, caretUTF16: (cmd as NSString).length, metadata: m)
        XCTAssertTrue(labels(s).contains("\\newcommand{\\name}[n]{body}"))
        XCTAssertTrue(labels(s).contains("\\newpage"))
        XCTAssertEqual(s.first { $0.label == "\\newterm" }?.detail, "declared in main.tex · 4 uses · revision 5")
        XCTAssertEqual(s.first { $0.label == "\\newwidget" }?.detail, "not supported by the compiler — " + compiled.diagnosticsByCommand["newwidget"]!)
        // A project `\newcommand` with a builtin's name wins over the static entry:
        // listed once, without the builtin's argument shape, saying so.
        // The builtin `\sec` is spelled exactly as typed, so it still ranks first.
        let sec = "x \\sec"
        let declared = Completion.suggestions(in: sec, caretUTF16: 6, metadata: m)
        XCTAssertEqual(declared.map(\.label), ["\\sec", "\\section"])
        guard declared.count == 2 else { return XCTFail("expected two declared commands, got \(declared.count)") }
        XCTAssertEqual(declared[1].insertText, "\\section")
        XCTAssertEqual(declared[1].detail, "declared in main.tex · revision 5 · overrides the builtin")
        XCTAssertEqual(Completion.suggestions(in: sec, caretUTF16: 6, metadata: nil).map(\.label), ["\\sec", "\\section{...}"])

        // Supported environments say so; others seen in the document carry
        // the compiler's diagnostic for that revision.
        let env = ref + "g} \\begin{myst"
        let e = Completion.suggestions(in: env, caretUTF16: (env as NSString).length, metadata: m)
        XCTAssertEqual(labels(e), ["mysteryenv"])
        guard let eFirst = e.first else { return XCTFail("expected one environment suggestion") }
        XCTAssertEqual(eFirst.detail, "seen in this document — environment 'mysteryenv' is not implemented; its body is typeset as plain text")
    }

    @MainActor
    func testTextViewRefusesMetadataNotBoundToItsEditorRevision() throws {
        let scroll = CompletingTextView.scrollable()
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        let text = "\\newwidget \\neww"
        tv.string = text
        tv.setSelectedRange(NSRange(location: (text as NSString).length, length: 0))
        tv.compileResult = result(revision: 3, ["\\newwidget is not supported by this compiler version"])
        XCTAssertEqual(tv.resultMetadata?.revision, 3)

        func newwidgetDetail() -> String? {
            Completion.suggestions(in: tv.string, caretUTF16: tv.selectedRange().location, metadata: tv.boundMetadata)
                .first { $0.label == "\\newwidget" }?.detail
        }
        // Unknown editor revision: nothing binds.
        XCTAssertNil(tv.editorRevision)
        XCTAssertNil(tv.boundMetadata)
        XCTAssertEqual(newwidgetDetail(), "not supported by the compiler")
        // Editor moved on since the result was compiled: refused.
        tv.editorRevision = 4
        XCTAssertNil(tv.boundMetadata)
        XCTAssertEqual(newwidgetDetail(), "not supported by the compiler")
        // Exact revision: bound and shown.
        tv.editorRevision = 3
        XCTAssertEqual(tv.boundMetadata?.revision, 3)
        XCTAssertEqual(newwidgetDetail(), "not supported by the compiler — \\newwidget is not supported by this compiler version")
        // The synchronous AppKit path binds the same way.
        var index = 0
        XCTAssertEqual(tv.completions(forPartialWordRange: tv.rangeForUserCompletion, indexOfSelectedItem: &index), ["\\newwidget"])

        // Project-index metadata: older than the held one is refused; equal merges; newer replaces.
        let versions = ["main.tex": 1]
        let at3 = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("newwork", 1, 0, false)]),
                                                                  category: .command, editorRevision: 3, expectedSourceVersions: versions)
        let at2 = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("older", 1, 0, false)]),
                                                                  category: .command, editorRevision: 2, expectedSourceVersions: versions)
        let cites3 = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("k", 1, 0, false)]),
                                                                     category: .citation, editorRevision: 3, expectedSourceVersions: versions)
        XCTAssertTrue(tv.accept(projectIndex: at3))
        XCTAssertFalse(tv.accept(projectIndex: at2))
        XCTAssertTrue(tv.accept(projectIndex: cites3))
        XCTAssertEqual(tv.projectIndexMetadata?.commands.map(\.name), ["newwork"])
        XCTAssertEqual(tv.projectIndexMetadata?.citations.map(\.name), ["k"])
        XCTAssertEqual(tv.completions(forPartialWordRange: tv.rangeForUserCompletion, indexOfSelectedItem: &index), ["\\newwork", "\\newwidget"])
        XCTAssertEqual(tv.boundMetadata?.diagnosticsByCommand.count, 1, "compile-result and index metadata merge at the bound revision")
        tv.editorRevision = 4
        XCTAssertNil(tv.boundMetadata)
        XCTAssertEqual(tv.completions(forPartialWordRange: tv.rangeForUserCompletion, indexOfSelectedItem: &index), ["\\newwidget"])
        let at5 = try Completion.Metadata.decodeProjectIndexReply(indexReply(versions: versions, names: [("newer", 1, 0, false)]),
                                                                  category: .command, editorRevision: 5, expectedSourceVersions: versions)
        XCTAssertTrue(tv.accept(projectIndex: at5))
        XCTAssertEqual(tv.projectIndexMetadata?.commands.map(\.name), ["newer"], "a newer revision replaces the held metadata")
    }

    /// `\cite{` states where each index key comes from using only the kinds
    /// the helper declared for the same snapshot: a record in a declared
    /// bibliography ranks first, a `\bibitem` in a LaTeX source next, a key
    /// whose path has no reported kind says so (never a guess from ".bib"),
    /// and a cited-but-undefined key says how to declare the bibliography.
    func testCitationCompletionUsesDeclaredKindsAndNeverInfersThem() throws {
        let versions = ["main.tex": 3, "refs.bib": 1]
        func citations(_ items: [(String, path: String?, defs: Int)]) throws -> Completion.Metadata {
            let payload: [String: Any] = ["source_versions": versions, "completions": items.map { i -> [String: Any] in
                let loc: [String: Any] = ["path": i.path ?? "", "revision": 1, "start_byte": 0, "end_byte": 3]
                return ["name": i.0, "definitions": Array(repeating: loc, count: i.defs), "occurrences": [loc, loc], "locations_truncated": false]
            }]
            return try Completion.Metadata.decodeProjectIndexReply(JSONSerialization.data(withJSONObject: payload), category: .citation,
                                                                   editorRevision: 9, expectedSourceVersions: versions)
        }
        func snapshot(_ kinds: [String: String]?) throws -> Completion.Metadata {
            var payload: [String: Any] = ["project_id": "p", "source_versions": versions, "membership_generation": 2]
            if let kinds { payload["document_kinds"] = kinds }
            return try Completion.Metadata.decodeProjectIndexSnapshot(JSONSerialization.data(withJSONObject: payload), editorRevision: 9,
                                                                      expectedSourceVersions: versions)
        }
        let keys = try citations([("aaa-missing", nil, 0), ("lamport94", "main.tex", 1), ("knuth84", "refs.bib", 1), ("mystery", "notes.bib", 1)])
        let text = "\\bibitem{local} x \\cite{"
        let caret = (text as NSString).length

        // Kinds declared: refs.bib is a bibliography, main.tex is LaTeX, notes.bib was not reported.
        let declared = try XCTUnwrap(keys.merged(with: snapshot(["main.tex": "latex", "refs.bib": "bibliography"])))
        XCTAssertEqual(declared.declaredBibliographies, ["refs.bib"])
        XCTAssertEqual(declared.isDeclaredBibliography("refs.bib"), true)
        XCTAssertEqual(declared.isDeclaredBibliography("main.tex"), false)
        XCTAssertNil(declared.isDeclaredBibliography("notes.bib"), "a path without a reported kind is unknown, not a .bib by name")
        let s1 = Completion.suggestions(in: text, caretUTF16: caret, metadata: declared)
        XCTAssertEqual(labels(s1), ["local", "knuth84", "lamport94", "mystery", "aaa-missing"])
        guard s1.count == 5 else { return XCTFail("expected five suggestions, got \(s1.count)") }
        XCTAssertEqual(s1[0].detail, "\\bibitem in this document")
        XCTAssertEqual(s1[1].detail, "record in refs.bib (declared bibliography) · 2 uses · revision 9")
        XCTAssertEqual(s1[2].detail, "\\bibitem in main.tex · 2 uses · revision 9")
        XCTAssertEqual(s1[3].detail, "defined in notes.bib (kind not reported by this helper) · 2 uses · revision 9")
        XCTAssertEqual(s1[4].detail, "cited but not defined in a declared bibliography source · 2 uses · revision 9")
        XCTAssertTrue(s1.allSatisfy { $0.kind == .citation && $0.insertText == $0.label + "}" })

        // Kinds reported, nothing declared: every defined key is a \bibitem or unknown; the
        // unresolved key says where to declare the .bib.
        let none = try XCTUnwrap(keys.merged(with: snapshot(["main.tex": "latex", "refs.bib": "latex"])))
        XCTAssertEqual(none.declaredBibliographies, [])
        let s2 = Completion.suggestions(in: text, caretUTF16: caret, metadata: none)
        XCTAssertEqual(labels(s2), ["local", "lamport94", "knuth84", "mystery", "aaa-missing"])
        guard s2.count == 5 else { return XCTFail("expected five suggestions, got \(s2.count)") }
        XCTAssertEqual(s2[2].detail, "\\bibitem in refs.bib · 2 uses · revision 9", "refs.bib is a LaTeX source until it is declared")
        XCTAssertEqual(s2[4].detail, "cited but not defined in a declared bibliography source (none declared: Project > Document Kinds) · 2 uses · revision 9")

        // No kinds bound at all (older helper without document_kinds, or none requested).
        XCTAssertNil(try snapshot(nil).documentKinds)
        let s3 = Completion.suggestions(in: text, caretUTF16: caret, metadata: keys)
        XCTAssertEqual(labels(s3), ["local", "lamport94", "knuth84", "mystery", "aaa-missing"])
        guard s3.count == 5 else { return XCTFail("expected five suggestions, got \(s3.count)") }
        XCTAssertEqual(s3[1].detail, "defined in main.tex (kind not reported by this helper) · 2 uses · revision 9")
        XCTAssertEqual(s3[2].detail, "defined in refs.bib (kind not reported by this helper) · 2 uses · revision 9")
        XCTAssertEqual(s3[4].detail, "cited but not defined in a declared bibliography source · 2 uses · revision 9")

        // Kinds never straddle revisions or snapshots.
        XCTAssertNil(keys.merged(with: try Completion.Metadata.decodeProjectIndexSnapshot(
            JSONSerialization.data(withJSONObject: ["source_versions": versions, "document_kinds": [:]] as [String: Any]),
            editorRevision: 10, expectedSourceVersions: versions)))
        XCTAssertThrowsError(try Completion.Metadata.decodeProjectIndexSnapshot(
            JSONSerialization.data(withJSONObject: ["source_versions": ["main.tex": 4, "refs.bib": 1], "document_kinds": [:]] as [String: Any]),
            editorRevision: 9, expectedSourceVersions: versions))
        // Kinds merge with the same-revision compile result and survive a category merge.
        let withResult = try XCTUnwrap(declared.merged(with: Completion.Metadata.from(result(revision: 9, []))))
        XCTAssertEqual(withResult.declaredBibliographies, ["refs.bib"])
    }

    @MainActor
    func testFetcherQueriesThreeCategoriesPlusKindsAndRefusesStaleOrFailedReplies() throws {
        let fetcher = ProjectIndexCompletionFetcher()
        var sent: [(id: String, type: String, payload: [String: Any])] = []
        var next = 1
        let send: (String, [String: Any]) throws -> String = { type, payload in
            defer { next += 1 }
            let id = "pc-\(next)"
            sent.append((id, type, payload))
            return id
        }
        let versions = ["main.tex": 3, "parts/body.tex": 1]
        fetcher.request(sourceVersions: versions, editorRevision: 12, send: send)
        XCTAssertEqual(sent.map(\.type), ["complete", "complete", "complete", "snapshot"])
        guard sent.count == 4 else { return XCTFail("expected four sent requests, got \(sent.count)") }
        XCTAssertEqual(sent.prefix(3).map { $0.payload["category"] as? String }, ["label", "citation", "command"])
        for s in sent.prefix(3) {
            XCTAssertEqual(s.payload["source_versions"] as? [String: Int], versions)
            XCTAssertEqual(s.payload["prefix"] as? String, "")
            XCTAssertEqual(s.payload["limit"] as? Int, 100)
        }
        XCTAssertTrue(sent[3].payload.isEmpty, "snapshot takes no arguments")
        XCTAssertEqual(fetcher.query?.outstanding.count, 4)
        XCTAssertEqual(fetcher.handle(resultID: "other", payload: [:]), .notMine)
        XCTAssertEqual(fetcher.handle(errorID: "other", message: "x"), .notMine)
        XCTAssertEqual(fetcher.handle(errorID: nil, message: "x"), .notMine)

        func reply(_ names: [String], versions: [String: Int]) -> [String: Any] {
            try! JSONSerialization.jsonObject(with: indexReply(versions: versions, names: names.map { ($0, 1, 2, false) })) as! [String: Any]
        }
        func snapshot(_ versions: [String: Int], kinds: [String: String]?) -> [String: Any] {
            var p: [String: Any] = ["project_id": "p", "source_versions": versions, "membership_generation": 1]
            if let kinds { p["document_kinds"] = kinds }
            return p
        }
        // Replies arrive in any order; the query completes when all four are in.
        XCTAssertEqual(fetcher.handle(resultID: "pc-3", payload: reply(["mycmd"], versions: versions)), .pending)
        XCTAssertEqual(fetcher.handle(resultID: "pc-4", payload: snapshot(versions, kinds: ["main.tex": "latex", "parts/body.tex": "latex", "refs.bib": "bibliography"])), .pending)
        XCTAssertEqual(fetcher.handle(resultID: "pc-1", payload: reply(["sec:a"], versions: versions)), .pending)
        guard case .complete(let m) = fetcher.handle(resultID: "pc-2", payload: reply(["knuth84"], versions: versions)) else {
            return XCTFail("expected complete")
        }
        XCTAssertEqual(m.revision, 12)
        XCTAssertEqual(m.labels.map(\.name), ["sec:a"])
        XCTAssertEqual(m.citations.map(\.name), ["knuth84"])
        XCTAssertEqual(m.commands.map(\.name), ["mycmd"])
        XCTAssertEqual(m.origin, .projectIndex(sourceVersions: versions))
        XCTAssertEqual(m.documentKinds, ["main.tex": "latex", "parts/body.tex": "latex", "refs.bib": "bibliography"])
        XCTAssertEqual(m.declaredBibliographies, ["refs.bib"])
        XCTAssertNil(fetcher.query)
        XCTAssertEqual(fetcher.handle(resultID: "pc-2", payload: [:]), .notMine, "a finished query accepts nothing more")

        // A reply for other source versions discards the whole query.
        fetcher.request(sourceVersions: versions, editorRevision: 13, send: send)
        XCTAssertEqual(fetcher.handle(resultID: "pc-5", payload: reply(["sec:a"], versions: versions)), .pending)
        guard case .refused = fetcher.handle(resultID: "pc-6", payload: reply(["k"], versions: ["main.tex": 4, "parts/body.tex": 1])) else {
            return XCTFail("expected refusal")
        }
        XCTAssertNil(fetcher.query)
        XCTAssertEqual(fetcher.handle(resultID: "pc-7", payload: reply(["mycmd"], versions: versions)), .notMine)
        XCTAssertEqual(fetcher.refusals, 1)
        // A snapshot for other source versions, or with an unknown kind, discards it too;
        // a snapshot without document_kinds (older helper) binds no kinds.
        fetcher.request(sourceVersions: versions, editorRevision: 13, send: send)
        guard case .refused = fetcher.handle(resultID: "pc-12", payload: snapshot(["main.tex": 4, "parts/body.tex": 1], kinds: [:])) else {
            return XCTFail("expected stale snapshot refusal")
        }
        fetcher.request(sourceVersions: versions, editorRevision: 13, send: send)
        guard case .refused(let why) = fetcher.handle(resultID: "pc-16", payload: snapshot(versions, kinds: ["x.bib": "bibtex"])) else {
            return XCTFail("expected unknown kind refusal")
        }
        XCTAssertTrue(why.contains("unknown document kind bibtex"), why)
        fetcher.request(sourceVersions: versions, editorRevision: 13, send: send)
        XCTAssertEqual(fetcher.handle(resultID: "pc-20", payload: snapshot(versions, kinds: nil)), .pending)
        XCTAssertNil(fetcher.query?.merged?.documentKinds)
        XCTAssertEqual(fetcher.refusals, 3)
        for id in ["pc-17", "pc-18"] { XCTAssertEqual(fetcher.handle(resultID: id, payload: reply(["n"], versions: versions)), .pending) }
        guard case .complete(let noKinds) = fetcher.handle(resultID: "pc-19", payload: reply(["n"], versions: versions)) else { return XCTFail("complete") }
        XCTAssertNil(noKinds.documentKinds)
        XCTAssertNil(noKinds.declaredBibliographies)

        // The helper's own error ("source versions changed") discards it too.
        fetcher.request(sourceVersions: versions, editorRevision: 14, send: send)
        XCTAssertEqual(fetcher.handle(errorID: "pc-22", message: "source versions changed; refresh snapshot before querying"),
                       .refused("helper error for pc-22: source versions changed; refresh snapshot before querying"))
        XCTAssertNil(fetcher.query)
        // A newer request supersedes an outstanding one.
        fetcher.request(sourceVersions: versions, editorRevision: 15, send: send)
        fetcher.request(sourceVersions: versions, editorRevision: 16, send: send)
        XCTAssertEqual(fetcher.handle(resultID: "pc-26", payload: reply(["x"], versions: versions)), .notMine)
        XCTAssertEqual(fetcher.query?.editorRevision, 16)
        // A send failure leaves no query behind.
        fetcher.request(sourceVersions: versions, editorRevision: 17) { _, _ in throw CocoaError(.fileWriteUnknown) }
        XCTAssertNil(fetcher.query)
    }

    /// A macro the buffer defines is the author's, from the keystroke that
    /// defines it: offered as declared, never "not supported by the compiler",
    /// with no compile result and no project-index reply at all.
    func testDeclaredMacrosAreOfferedAsDeclaredBeforeAnyIndexReply() {
        let preamble = """
        \\newcommand{\\foo}{x}
        \\renewcommand*{\\vec}[1]{\\mathbf{#1}}
        \\providecommand\\brr{y}
        \\DeclareMathOperator{\\Tr}{Tr}
        \\DeclareMathOperator*{\\argmin}{arg\\,min}
        \\def\\baz#1{#1}
        \\let\\qux\\relax
        \\NewDocumentCommand{ \\fig }{m}{#1}
        \\newcommand{\\foo}{again}
        \\zzunknown

        """
        XCTAssertEqual(Completion.declaredCommands(in: preamble), ["foo", "vec", "brr", "Tr", "argmin", "baz", "qux", "fig"])
        // A definer with nothing after it, a non-letter control sequence, an
        // unclosed brace and a `\begin{…}` are not declarations.
        XCTAssertTrue(Completion.declaredCommands(in: "\\newcommand\n\\def\\@x{}\n\\newcommand{\\a\n\\let\\{\\}\n\\begin{foo}").isEmpty)

        func offered(_ typed: String, mathMode: Bool? = nil, elsewhere: [String] = []) -> [Completion.Suggestion] {
            let text = preamble + typed
            return Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: nil,
                                          declaredElsewhere: elsewhere, mathMode: mathMode)
        }
        let f = offered("\\f")
        XCTAssertEqual(f.first?.label, "\\foo", "the author's macro sits above the compiler's \\f… entries, not beyond the cap: \(labels(f))")
        XCTAssertEqual(f.first?.detail, "declared in this document")
        XCTAssertEqual(f.first?.insertText, "\\foo")
        XCTAssertEqual(f.first?.kind, .command)
        XCTAssertEqual(f.filter { $0.label == "\\foo" }.count, 1, "declared once, although used twice")
        XCTAssertTrue(f.contains { $0.label == "\\fig" && $0.detail == "declared in this document" }, "\(labels(f))")
        XCTAssertFalse(f.contains { $0.detail.hasPrefix("not supported") }, "\(f.map(\.detail))")
        XCTAssertEqual(offered("\\T").first { $0.label == "\\Tr" }?.detail, "declared in this document")
        XCTAssertEqual(offered("\\argmi").first?.detail, "declared in this document")
        XCTAssertEqual(offered("\\b").filter { $0.detail == "declared in this document" }.map(\.label), ["\\brr", "\\baz"])
        XCTAssertEqual(offered("\\qu").first { $0.label == "\\qux" }?.detail, "declared in this document")
        // In math mode too, and still under the exact spelling.
        XCTAssertEqual(offered("$\\f", mathMode: true).first?.label, "\\foo")
        let exact = offered("\\frac")
        XCTAssertEqual(exact.first?.insertText, "\\frac", "the exact spelling keeps first place")
        // A macro that shadows a compiler command keeps the compiler's row.
        let vec = offered("\\vec")
        XCTAssertEqual(vec.filter { $0.insertText == "\\vec" }.count, 1)
        XCTAssertNotEqual(vec.first { $0.insertText == "\\vec" }?.detail, "declared in this document")
        // A command the document merely uses stays marked, as before.
        XCTAssertEqual(offered("\\zzu").first?.detail, "not supported by the compiler")
        // Another open document's macros (the scheduler scans them) come after the buffer's.
        let elsewhere = offered("\\f", elsewhere: ["foo", "fjord"])
        XCTAssertEqual(elsewhere.prefix(3).map(\.label), ["\\foo", "\\fig", "\\fjord"])
        XCTAssertEqual(elsewhere[2].detail, "declared in an open document")
        XCTAssertEqual(elsewhere[0].detail, "declared in this document", "the buffer's own declaration wins the label")
        // Once the index has answered for this revision, its richer line replaces the buffer's.
        let indexed = Completion.Metadata(origin: .compileResult(projectId: "p"), revision: 1, commands: [
            .init(name: "foo", definitions: 1, occurrences: 2, locationsTruncated: false, definedIn: "main.tex"),
        ])
        let text = preamble + "\\f"
        let withIndex = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: indexed)
        XCTAssertEqual(withIndex.first?.label, "\\foo")
        XCTAssertEqual(withIndex.first?.detail, "declared in main.tex · 2 uses · revision 1")
    }

    /// The scheduler scans the other open documents for macros only when the
    /// caret is on a command, and hands them to the suggestions as declared.
    @MainActor
    func testSchedulerOffersMacrosDeclaredInOtherOpenDocuments() {
        let exec = ManualExecutor()
        let scheduler = CompletionScheduler(executor: exec.run)
        var delivered: [CompletionScheduler.Outcome] = []
        var req = CompletionScheduler.Request(text: "\\begin{document}\n\\fj", caretUTF16: 20, metadata: nil)
        req.otherDocuments = ["\\newcommand{\\fjord}{Fjord}\n", "\\DeclareMathOperator{\\fjop}{op}"]
        scheduler.schedule(req) { delivered.append($0) }
        exec.runAll()
        spin("delivery") { scheduler.statistics.delivered == 1 }
        XCTAssertEqual(delivered.first?.items.map(\.label), ["\\fjord", "\\fjop"])
        XCTAssertEqual(delivered.first?.items.map(\.detail), Array(repeating: "declared in an open document", count: 2))
    }

    // MARK: `\cite{` straight from the project's .bib files (BibScanner.swift)

    private static let knuthBib = "% refs\n@article{knuth84,\n  title = {Literate {Programming}},\n  author = {Knuth, Donald E.},\n  year = {1984}\n}\n@book{lamport86,\n  author = \"Lamport, Leslie\",\n  year = 1986\n}\n"

    private func bibProject(_ name: String, bib: String = CompletionTests.knuthBib) throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("bib-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        try bib.write(to: dir.appendingPathComponent("refs.bib"), atomically: true, encoding: .utf8)
        return dir
    }

    /// The scheduler reads the `.bib` files `\bibliography{…}` names from the
    /// project root when the caret is in `\cite{` — no helper, no metadata —
    /// and each key carries its entry type, file and title.
    @MainActor
    func testCiteCompletionReadsTheProjectsBibFilesWithoutTheHelper() throws {
        let root = try bibProject("bibliography")
        defer { try? FileManager.default.removeItem(at: root) }
        let text = "\\bibliographystyle{plain}\n\\bibliography{refs}\n\\cite{kn"
        let exec = ManualExecutor()
        let scheduler = CompletionScheduler(executor: exec.run)
        var delivered: [CompletionScheduler.Outcome] = []
        var req = CompletionScheduler.Request(text: text, caretUTF16: (text as NSString).length, metadata: nil)
        req.bibliography = BibScanner.Sources(projectRoot: root, documents: [.init(path: "main.tex", text: text)])
        scheduler.schedule(req) { delivered.append($0) }
        exec.runAll()
        spin("delivery") { scheduler.statistics.delivered == 1 }
        let items = try XCTUnwrap(delivered.first?.items)
        XCTAssertEqual(items.map(\.label), ["knuth84"])
        let knuth = try XCTUnwrap(items.first)
        XCTAssertEqual(knuth.kind, .citation)
        XCTAssertEqual(knuth.insertText, "knuth84}")
        XCTAssertTrue(knuth.detail.contains("@article"), knuth.detail)
        XCTAssertTrue(knuth.detail.contains("refs.bib"), knuth.detail)
        XCTAssertEqual(knuth.detail, "@article · refs.bib")
        XCTAssertEqual(knuth.documentation, "Literate Programming", "the title is the doc line, protective braces stripped")
        XCTAssertEqual(CompletionPopup.displayedDocumentation(for: knuth), "Literate Programming")
        // The whole file with no prefix: the second entry has no title, so author + year is its line.
        let all = Completion.suggestions(in: "\\cite{", caretUTF16: 6, metadata: nil,
                                         bibliographyEntries: BibScanner.entries(for: req.bibliography!))
        XCTAssertEqual(all.map(\.label), ["knuth84", "lamport86"])
        XCTAssertEqual(all[1].detail, "@book · refs.bib")
        XCTAssertEqual(all[1].documentation, "Lamport 1986")
    }

    /// `\addbibresource[…]{refs.bib}` (biblatex) declares the same file; the
    /// parse is cached per file by modification date, so a second request
    /// costs one attribute read.
    @MainActor
    func testCiteCompletionReadsAddbibresourceAndCachesByModificationDate() throws {
        let root = try bibProject("addbibresource")
        defer { try? FileManager.default.removeItem(at: root) }
        let text = "\\addbibresource[datatype=bibtex]{refs.bib}\n\\cite{lam"
        XCTAssertEqual(BibScanner.declaredBibliographies(in: text), ["refs.bib"])
        XCTAssertEqual(BibScanner.declaredBibliographies(in: "\\bibliography{refs, more}\\bibliographystyle{alpha}"), ["refs.bib", "more.bib"])
        let cache = BibScanner.Cache()
        let sources = BibScanner.Sources(projectRoot: root, documents: [.init(path: "main.tex", text: text)])
        let first = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: nil,
                                           bibliographyEntries: BibScanner.entries(for: sources, cache: cache))
        XCTAssertEqual(first.map(\.label), ["lamport86"])
        XCTAssertEqual(first.first?.detail, "@book · refs.bib")
        XCTAssertEqual(cache.parses, 1)
        _ = BibScanner.entries(for: sources, cache: cache)
        XCTAssertEqual(cache.parses, 1, "unchanged file: served from the cache")
        // A rewrite with a new modification date is parsed again.
        let url = root.appendingPathComponent("refs.bib")
        try "@misc{fresh, title = {New}}\n".write(to: url, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.modificationDate: Date().addingTimeInterval(5)], ofItemAtPath: url.path)
        XCTAssertEqual(BibScanner.entries(for: sources, cache: cache).map(\.key), ["fresh"])
        XCTAssertEqual(cache.parses, 2)
        // An open .bib is parsed from its buffer, ahead of the disk, and never read from disk.
        let open = BibScanner.Sources(projectRoot: nil, documents: [.init(path: "main.tex", text: "\\cite{"),
                                                                     .init(path: "notes.bib", text: "@article{buffered, title = {In memory}}")])
        XCTAssertEqual(BibScanner.entries(for: open, cache: cache).map(\.detail), ["@article · notes.bib"])
        // Declared through the helper's Document Kinds (no \bibliography line in any source).
        let declared = BibScanner.Sources(projectRoot: root, declaredPaths: ["refs.bib"], documents: [.init(path: "main.tex", text: "\\cite{")])
        XCTAssertEqual(BibScanner.entries(for: declared, cache: cache).map(\.key), ["fresh"])
    }

    /// With the helper attached its index rows win: a key both know is one
    /// row with the helper's provenance and the record's doc line; a key only
    /// the file has is appended after the index's rows.
    func testCiteCompletionDedupesHelperKeysAgainstTheBibFiles() throws {
        let versions = ["main.tex": 3, "refs.bib": 1]
        let loc: [String: Any] = ["path": "refs.bib", "revision": 1, "start_byte": 0, "end_byte": 3]
        let payload: [String: Any] = ["source_versions": versions, "completions": [
            ["name": "knuth84", "definitions": [loc], "occurrences": [loc], "locations_truncated": false],
        ]]
        let keys = try Completion.Metadata.decodeProjectIndexReply(JSONSerialization.data(withJSONObject: payload), category: .citation,
                                                                   editorRevision: 9, expectedSourceVersions: versions)
        let snapshot = try Completion.Metadata.decodeProjectIndexSnapshot(JSONSerialization.data(withJSONObject: [
            "project_id": "p", "source_versions": versions, "membership_generation": 2,
            "document_kinds": ["main.tex": "latex", "refs.bib": "bibliography"],
        ] as [String: Any]), editorRevision: 9, expectedSourceVersions: versions)
        let metadata = try XCTUnwrap(keys.merged(with: snapshot))
        let entries = [BibScanner.Entry(key: "knuth84", type: "article", path: "refs.bib", title: "Literate Programming"),
                       BibScanner.Entry(key: "kernighan78", type: "book", path: "refs.bib", author: "Kernighan and Ritchie", year: "1978")]
        let text = "\\bibitem{knope} x \\cite{k"
        let out = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: metadata, bibliographyEntries: entries)
        XCTAssertEqual(out.map(\.label), ["knope", "knuth84", "kernighan78"])
        guard out.count == 3 else { return XCTFail("expected three suggestions, got \(out.count)") }
        XCTAssertEqual(out[1].detail, "record in refs.bib (declared bibliography) · 1 use · revision 9", "the helper's row wins")
        XCTAssertEqual(out[1].documentation, "Literate Programming", "…and gains the record's line")
        XCTAssertEqual(out[2].detail, "@book · refs.bib")
        XCTAssertEqual(out[2].documentation, "Kernighan and Ritchie 1978")
        XCTAssertNil(out[0].documentation)
    }

    // MARK: cancellation and stale refusal

    /// Holds jobs until the test runs them, so caret moves and job completion
    /// interleave deterministically.
    @MainActor
    final class ManualExecutor {
        var jobs: [@Sendable () -> Void] = []
        var run: CompletionScheduler.Executor { { [self] job in jobs.append(job) } }
        func runAll() { let j = jobs; jobs = []; j.forEach { $0() } }
    }

    /// Runs the main run loop until `cond` holds (scheduler delivery is a
    /// run-loop block, not a dispatch).
    @MainActor
    private func spin(_ what: String, timeout: TimeInterval = 5, until cond: () -> Bool) {
        let deadline = Date().addingTimeInterval(timeout)
        while !cond(), Date() < deadline {
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.005))
        }
        XCTAssertTrue(cond(), "timed out waiting for \(what)")
    }

    @MainActor
    func testSchedulerRefusesCancelledAndSupersededOutcomes() {
        let exec = ManualExecutor()
        let scheduler = CompletionScheduler(executor: exec.run)
        var delivered: [CompletionScheduler.Outcome] = []
        // An article project: beamer's `\subtitle` would otherwise lead the `\sub` list.
        let req = CompletionScheduler.Request(text: "\\begin{document} \\se", caretUTF16: 20, metadata: nil, projectClass: "article")

        // 1. Explicit cancellation before the job ran: the job is marked, computes nothing, and is refused.
        let g1 = scheduler.schedule(req) { delivered.append($0) }
        XCTAssertEqual(g1, 1)
        scheduler.cancel()
        XCTAssertEqual(scheduler.generation, 2)
        XCTAssertEqual(exec.jobs.count, 1)
        exec.runAll()
        spin("refusal") { scheduler.statistics.refusedStale == 1 }
        XCTAssertTrue(delivered.isEmpty)
        XCTAssertEqual(scheduler.statistics, .init(scheduled: 1, delivered: 0, refusedStale: 1, cancelled: 1))

        // 2. A newer request supersedes the pending one: only the newest outcome is delivered.
        scheduler.schedule(req) { delivered.append($0) }
        let g3 = scheduler.schedule(.init(text: "x \\sub", caretUTF16: 6, metadata: nil, projectClass: "article")) { delivered.append($0) }
        XCTAssertEqual(exec.jobs.count, 2)
        exec.runAll()
        spin("delivery") { scheduler.statistics.delivered == 1 }
        spin("second refusal") { scheduler.statistics.refusedStale == 2 }
        XCTAssertEqual(delivered.count, 1)
        guard let firstDelivered = delivered.first else { return XCTFail("expected one delivered outcome") }
        XCTAssertEqual(firstDelivered.generation, g3)
        XCTAssertEqual(firstDelivered.items.map(\.label),
                       ["\\subsection{...}", "\\subsubsection{...}", "\\subparagraph{...}", "\\substack{a \\\\ b}", "\\subset", "\\subseteq",
                        "\\subseteqq", "\\subsetneqq", "\\subsetneq"])
        XCTAssertEqual(firstDelivered.range, NSRange(location: 2, length: 4))
        XCTAssertEqual(firstDelivered.caretUTF16, 6)
        XCTAssertNil(scheduler.pending)
        XCTAssertEqual(scheduler.statistics, .init(scheduled: 3, delivered: 1, refusedStale: 2, cancelled: 2))

        // 3. Cancellation after the job computed but before delivery ran: still refused.
        scheduler.schedule(req) { delivered.append($0) }
        exec.runAll() // computed; the run-loop delivery block is queued
        scheduler.cancel()
        spin("late refusal") { scheduler.statistics.refusedStale == 3 }
        XCTAssertEqual(delivered.count, 1)
        // 4. Cancelling with nothing pending only advances the generation.
        let before = scheduler.statistics
        scheduler.cancel()
        XCTAssertEqual(scheduler.statistics, before)
    }

    @MainActor
    func testTextViewCancelsOnCaretMoveTextChangeAndResign() throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        let exec = ManualExecutor()
        tv.scheduler = CompletionScheduler(executor: exec.run)
        window.orderFrontRegardless() // never makeKey
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        tv.string = "\\begin{document}\nx \\s"
        let end = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: end, length: 0))

        // Caret moves while the scan is pending: the job is cancelled and its outcome refused.
        tv.requestCompletion()
        XCTAssertEqual(exec.jobs.count, 1)
        tv.setSelectedRange(NSRange(location: 2, length: 0))
        XCTAssertEqual(tv.scheduler.statistics.cancelled, 1)
        exec.runAll()
        spin("refusal") { tv.scheduler.statistics.refusedStale == 1 }
        XCTAssertNil(tv.session)

        // Text changes while pending: same.
        tv.setSelectedRange(NSRange(location: end, length: 0))
        tv.requestCompletion()
        tv.insertText("x", replacementRange: NSRange(location: 0, length: 0))
        XCTAssertEqual(tv.scheduler.statistics.cancelled, 2)
        exec.runAll()
        spin("refusal 2") { tv.scheduler.statistics.refusedStale == 2 }
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.string, "x\\begin{document}\nx \\s")

        // Open a session, then move the caret: closed with the reason, popup gone.
        let caret = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        tv.requestCompletion()
        exec.runAll()
        spin("session") { tv.session != nil }
        // `\s` overflows the cap; the session shows exactly the pure function's list.
        XCTAssertEqual(tv.session?.items.count, Completion.maxSuggestions)
        XCTAssertEqual(tv.session?.items.map(\.label).prefix(2), CompletionTestVocabulary.labels(forPrefix: "s").prefix(2))
        XCTAssertEqual(tv.session?.items, Completion.suggestions(in: tv.string, caretUTF16: caret, metadata: nil))
        XCTAssertEqual(tv.session?.range, NSRange(location: caret - 2, length: 2))
        XCTAssertNil(tv.session?.metadataRevision, "no metadata was bound")
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.lastCloseReason, .caretMoved)

        // Open again, then replace the text programmatically (no didChangeText): closed.
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        tv.requestCompletion()
        exec.runAll()
        spin("session 2") { tv.session != nil }
        tv.textStorage?.replaceCharacters(in: NSRange(location: 0, length: 1), with: "yy")
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.lastCloseReason, .textChanged)

        // An outcome for a stale generation never opens a session even if it is
        // the caret's position: the session generation must match the scheduler's.
        let c2 = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: c2, length: 0))
        tv.requestCompletion()
        exec.runAll()
        spin("session 3") { tv.session != nil }
        let generation = try XCTUnwrap(tv.session?.generation)
        XCTAssertEqual(generation, tv.scheduler.generation)
        // Accepting after the text changed underneath (flag off, so the view sees it) is refused.
        tv.textStorage?.replaceCharacters(in: NSRange(location: 0, length: 0), with: "z")
        XCTAssertNil(tv.session)
        tv.acceptSelectedCompletion()
        XCTAssertEqual(tv.string, "zyy\\begin{document}\nx \\s", "nothing was inserted")

        // Losing first responder closes the list.
        tv.setSelectedRange(NSRange(location: (tv.string as NSString).length, length: 0))
        tv.requestCompletion()
        exec.runAll()
        spin("session 4") { tv.session != nil }
        window.makeFirstResponder(nil)
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.lastCloseReason, .resignedFirstResponder)
    }

    // MARK: keyboard acceptance through the real text view

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
    func testKeyboardChoosesInsertsAndClosesThroughTheRealTextView() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless() // never makeKey: the test must not steal focus
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        tv.allowsUndo = true
        tv.string = "\\begin{document}\nx \\su"
        let end = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: end, length: 0))
        // `\su`'s vocabulary commands, in table order (text entries, then the
        // operator, then the symbols in inventory order) — computed from the
        // same pure function the session uses, so this test tracks the
        // compiler's inventory instead of a hand-copied snapshot of it.
        let suItems = Completion.suggestions(in: tv.string, caretUTF16: end, result: nil).map(\.label)
        let subsetIndex = try XCTUnwrap(suItems.firstIndex(of: "\\subset"))
        let lastIndex = suItems.count - 1

        // ⌃Space opens the list (computed off-main, delivered on the run loop).
        key(tv, " ", code: 49, flags: .control)
        XCTAssertNil(tv.session, "the keystroke path enqueues; it does not scan")
        try await waitUntil("popup") { tv.session != nil }
        XCTAssertEqual(tv.session?.items.map(\.label), suItems)
        XCTAssertEqual(tv.session?.selectedIndex, 0)
        XCTAssertEqual(tv.string, "\\begin{document}\nx \\su", "opening the list never edits the text")

        // ↓ ↓ ↑ choose; the text and caret are untouched while choosing.
        key(tv, "\u{F701}", code: 125)
        key(tv, "\u{F701}", code: 125)
        XCTAssertEqual(tv.session?.selected?.label, suItems[2])
        key(tv, "\u{F700}", code: 126)
        XCTAssertEqual(tv.session?.selected?.label, suItems[1])
        XCTAssertEqual(tv.selectedRange(), NSRange(location: end, length: 0))
        // ↑ from the top wraps to the bottom.
        key(tv, "\u{F700}", code: 126); key(tv, "\u{F700}", code: 126)
        XCTAssertEqual(tv.session?.selected?.label, suItems[lastIndex])
        key(tv, "\u{F701}", code: 125)
        XCTAssertEqual(tv.session?.selected?.label, suItems[0])

        // Typing through the list narrows it and keeps the chosen item when it survives.
        for _ in 0..<subsetIndex { key(tv, "\u{F701}", code: 125) } // walk down to \subset
        XCTAssertEqual(tv.session?.selected?.label, "\\subset")
        key(tv, "b", code: 11)
        XCTAssertEqual(tv.string, "\\begin{document}\nx \\sub")
        let subItems = Completion.suggestions(in: tv.string, caretUTF16: end + 1, result: nil).map(\.label)
        try await waitUntil("narrowed") { tv.session?.items.count == subItems.count }
        XCTAssertEqual(tv.session?.items.map(\.label), subItems)
        XCTAssertEqual(tv.session?.selected?.label, "\\subset")
        XCTAssertEqual(tv.session?.range, NSRange(location: end - 3, length: 4))
        // Delete widens it again.
        key(tv, "\u{7F}", code: 51)
        XCTAssertEqual(tv.string, "\\begin{document}\nx \\su")
        try await waitUntil("widened") { tv.session?.items.count == suItems.count }
        XCTAssertEqual(tv.session?.selected?.label, "\\subset")
        for _ in 0..<subsetIndex { key(tv, "\u{F700}", code: 126) } // walk back up to the top
        XCTAssertEqual(tv.session?.selected?.label, suItems[0])

        // Return inserts the chosen item over the partial token (its argument
        // braces with the caret inside), as one undoable edit, and closes.
        key(tv, "\r", code: 36)
        XCTAssertEqual(tv.string, "\\begin{document}\nx \\subsection{}")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: (tv.string as NSString).length - 1, length: 0))
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.lastCloseReason, .accepted)
        XCTAssertEqual(tv.undoManager?.canUndo, true)

        // Esc closes without inserting, and a late outcome cannot reopen the list.
        tv.string = "\\begin{document}\nx \\su"
        tv.setSelectedRange(NSRange(location: end, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup 2") { tv.session != nil }
        key(tv, "\u{1B}", code: 53)
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.lastCloseReason, .escape)
        XCTAssertEqual(tv.string, "\\begin{document}\nx \\su")
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertNil(tv.session)

        // Esc with no list open is AppKit's `complete:` binding: it opens the list.
        key(tv, "\u{1B}", code: 53)
        try await waitUntil("popup via Esc") { tv.session != nil }
        // Tab chooses the next candidate (never inserts a tab); Enter inserts; ← closes (the caret leaves the token).
        key(tv, "\t", code: 48)
        XCTAssertEqual(tv.string, "\\begin{document}\nx \\su", "Tab moved the choice, the text is untouched")
        XCTAssertEqual(tv.session?.selected?.label, suItems[1])
        key(tv, "\t", code: 48, flags: .shift)
        XCTAssertEqual(tv.session?.selected?.label, suItems[0])
        key(tv, "\u{3}", code: 76) // Enter (keypad)
        XCTAssertEqual(tv.string, "\\begin{document}\nx \\subsection{}")
        XCTAssertNil(tv.session)
        tv.string = "\\begin{document}\nx \\su"
        tv.setSelectedRange(NSRange(location: end, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup 3") { tv.session != nil }
        key(tv, "\u{F702}", code: 123)
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.lastCloseReason, .caretMoved)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: end - 1, length: 0), "the arrow key still moved the caret")

        // Typing a character that ends the token closes the list (no candidates).
        tv.setSelectedRange(NSRange(location: end, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup 4") { tv.session != nil }
        key(tv, " ", code: 49)
        XCTAssertEqual(tv.string, "\\begin{document}\nx \\su ")
        try await waitUntil("closed by space") { tv.session == nil }
        XCTAssertEqual(tv.lastCloseReason, .noCandidates)
        XCTAssertEqual(tv.scheduler.statistics.refusedStale, 0, "every delivered outcome was current")
        // Mouse: a click chooses a row, a double-click accepts it; the popup is a
        // non-activating child window that cannot become key.
        tv.string = "\\begin{document}\nx \\su"
        tv.setSelectedRange(NSRange(location: end, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup 5") { tv.session != nil }
        let popup = tv.completionPopup
        XCTAssertTrue(popup.isVisible)
        XCTAssertFalse(popup.canBecomeKey)
        XCTAssertTrue(popup.parent === window)
        XCTAssertEqual(popup.items.count, suItems.count)
        popup.click(row: subsetIndex)
        XCTAssertEqual(tv.session?.selected?.label, "\\subset")
        popup.click(row: suItems.count + 10) // out of range: ignored
        XCTAssertEqual(tv.session?.selectedIndex, subsetIndex)
        popup.click(row: subsetIndex + 1, double: true)
        XCTAssertEqual(tv.string, "\\begin{document}\nx " + suItems[subsetIndex + 1])
        XCTAssertNil(tv.session)
        XCTAssertFalse(popup.isVisible)
        // ⌘-shortcuts with the list open act on the editor (undo) and close the list.
        tv.string = "\\begin{document}\nx \\su"
        tv.setSelectedRange(NSRange(location: end, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup 6") { tv.session != nil }
        key(tv, "a", code: 0, flags: .command)
        XCTAssertNil(tv.session)
        XCTAssertEqual(tv.lastCloseReason, .caretMoved)
        // The session reports the metadata revision it was bound to.
        tv.compileResult = result(revision: 8, [])
        tv.editorRevision = 8
        tv.string = "x \\s"
        tv.setSelectedRange(NSRange(location: 4, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("bound popup") { tv.session != nil }
        XCTAssertEqual(tv.session?.metadataRevision, 8)
        key(tv, "\u{1B}", code: 53)
    }

    /// Keyboard-only traversal of the open list: Tab / ⇧Tab walk the rows
    /// (wrapping) exactly like ↓ / ↑ while the editor keeps first responder,
    /// its caret and its text; Return inserts the walked-to candidate. What
    /// VoiceOver reads is read back through NSAccessibility from the real
    /// popup: the table is "Completions" with the shared help text (which
    /// names Tab and Shift-Tab), each row's cell describes "candidate, kind,
    /// origin", the selected row follows the choice, and the announcement for
    /// the choice leads with "n of m".
    @MainActor
    func testTabAndShiftTabTraverseTheListWithVoiceOverLabels() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless() // never makeKey
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        tv.projectDocumentClass = { "article" } // beamer's `\subtitle` would otherwise lead the list
        tv.string = "\\begin{document}\nx \\su"
        let end = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: end, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup") { tv.session != nil }
        let items = try XCTUnwrap(tv.session?.items)
        let labels = items.map(\.label)
        XCTAssertEqual(labels, ["\\subsection{...}", "\\subsubsection{...}", "\\subparagraph{...}", "\\substack{a \\\\ b}", "\\sup", "\\subset", "\\subseteq",
                                "\\supset", "\\supseteq", "\\sum", "\\succsim", "\\succcurlyeq"])
        guard labels.count == 12 else { return XCTFail("expected 12 suggestions, got \(labels.count)") }
        let back = labels.count - 2 // where two ⇧Tab from the top land
        let popup = tv.completionPopup
        let table = popup.accessibilityTable
        XCTAssertEqual(table.accessibilityLabel(), CompletionAccessibility.listLabel)
        XCTAssertEqual(table.accessibilityHelp(), CompletionAccessibility.listHelp)
        XCTAssertTrue(CompletionAccessibility.listHelp.contains("Tab and Shift-Tab choose"), CompletionAccessibility.listHelp)
        XCTAssertTrue(AccessibilityCommand.completionList.entry.shortcuts.contains("Tab"))
        XCTAssertTrue(AccessibilityCommand.completionList.entry.shortcuts.contains("⇧Tab"))

        // Tab walks down and wraps to the top; ⇧Tab walks up and wraps to the bottom.
        var walked: [String] = []
        for _ in 0..<labels.count {
            key(tv, "\t", code: 48)
            walked.append(try XCTUnwrap(tv.session?.selected?.label))
        }
        XCTAssertEqual(walked, Array(labels[1...]) + [labels[0]])
        XCTAssertEqual(tv.session?.selectedIndex, 0)
        key(tv, "\t", code: 48, flags: .shift)
        key(tv, "\t", code: 48, flags: .shift)
        XCTAssertEqual(tv.session?.selectedIndex, back)
        XCTAssertEqual(popup.selectedRow, back)
        XCTAssertTrue(window.firstResponder === tv, "the list never takes the keyboard")
        XCTAssertFalse(popup.isKeyWindow)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: end, length: 0), "choosing never moves the caret")
        XCTAssertEqual(tv.string, "\\begin{document}\nx \\su", "no tab character was inserted")

        // Read back what VoiceOver gets from the real popup (legacy attribute
        // API: AppKit answers a table's children with private row proxies).
        popup.contentView?.layoutSubtreeIfNeeded()
        table.display()
        func legacy(_ o: AnyObject?, _ a: NSAccessibility.Attribute) -> Any? { (o as? NSObject)?.accessibilityAttributeValue(a) }
        let children = (table.accessibilityChildren() as? [AnyObject]) ?? []
        let axRows = children.filter { (legacy($0, .role) as? String) == NSAccessibility.Role.row.rawValue }
        XCTAssertEqual(axRows.count, labels.count)
        for (i, row) in axRows.enumerated() {
            let cells = (legacy(row, .children) as? [AnyObject]) ?? []
            XCTAssertEqual(cells.count, 1, "row \(i)")
            XCTAssertEqual(legacy(cells.first, .description) as? String, CompletionPopup.spokenLabel(items[i]), "row \(i)")
            XCTAssertEqual(legacy(row, .index) as? Int, i)
        }
        let selectedRows = (legacy(table, .selectedRows) as? [AnyObject]) ?? []
        XCTAssertEqual(selectedRows.count, 1)
        XCTAssertEqual(selectedRows.first.flatMap { legacy($0, .index) as? Int }, back)
        let announcement = CompletionAccessibility.selectionAnnouncement(index: back, total: labels.count, label: items[back].label,
                                                                          kind: items[back].kind.accessibilityKind, detail: items[back].detail)
        XCTAssertTrue(announcement.hasPrefix("\(back + 1) of \(labels.count): \(labels[back]), command, "), announcement)

        // Return inserts the walked-to candidate over the token and closes.
        key(tv, "\r", code: 36)
        XCTAssertEqual(tv.string, "\\begin{document}\nx " + items[back].insertText)
        XCTAssertNil(tv.session)
        XCTAssertFalse(popup.isVisible)
        XCTAssertEqual(tv.lastCloseReason, .accepted)
        // With no list open, Tab is the editor's own tab again.
        key(tv, "\t", code: 48)
        XCTAssertTrue(tv.string.hasSuffix("\t"), "\(tv.string)")
    }

    @MainActor
    func testSnippetsInsertThroughTheRealTextViewAsOneUndoStep() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless() // never makeKey
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        tv.allowsUndo = true
        let undo = try XCTUnwrap(tv.undoManager)

        func accept(after typing: String, from seed: String) async throws {
            tv.string = seed
            tv.setSelectedRange(NSRange(location: (seed as NSString).length, length: 0))
            undo.removeAllActions()
            for ch in typing { key(tv, String(ch), code: 0) } // typed, so the typing undo group is open
            // GH#256: typing arms the automatic open (#215). Fire it now rather
            // than racing its timer against ⌃Space and Return, then wait until
            // every scan either lifecycle scheduled has resolved, so Return
            // meets the settled session.
            tv.flushAutomaticCompletion()
            key(tv, " ", code: 49, flags: .control)
            try await waitUntil("popup for \(typing)") {
                let s = tv.scheduler.statistics
                return tv.session != nil && s.delivered + s.refusedStale + s.cancelled >= s.scheduled
            }
            key(tv, "\r", code: 36)
            XCTAssertNil(tv.session)
            XCTAssertEqual(tv.lastCloseReason, .accepted)
        }

        // (2) Argument shape: braces inserted, caret inside, one undo step that
        // leaves the typed partial token in place.
        try await accept(after: "\\se", from: "x ")
        XCTAssertEqual(tv.string, "x \\section{}")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 11, length: 0), "caret between the braces")
        XCTAssertTrue(undo.canUndo)
        XCTAssertEqual(undo.undoActionName, "Insert Snippet")
        undo.undo()
        XCTAssertEqual(tv.string, "x \\se", "one ⌘Z removes the whole snippet and keeps the typed token")
        undo.redo()
        XCTAssertEqual(tv.string, "x \\section{}")

        // (1) Environment skeleton with the current line's indentation, caret on
        // the middle line; ⌘Z removes all three lines at once.
        try await accept(after: "\\begin{it", from: "\\begin{document}\n  ")
        let unit = EditorPreferences.shared.indentString // the body sits one unit in (EnvironmentEditingRules)
        XCTAssertEqual(tv.string, "\\begin{document}\n  \\begin{itemize}\n  \(unit)\\item \n  \\end{itemize}") // list template (SnippetTests)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: ("\\begin{document}\n  \\begin{itemize}\n  \(unit)\\item " as NSString).length, length: 0))
        XCTAssertEqual(undo.undoActionName, "Insert Environment")
        undo.undo()
        XCTAssertEqual(tv.string, "\\begin{document}\n  \\begin{it")
        // `\end{…}` for the innermost open environment stays exact.
        try await accept(after: "\\e", from: "\\begin{document}\n\\begin{itemize}\n")
        XCTAssertEqual(tv.string, "\\begin{document}\n\\begin{itemize}\n\\end{itemize}")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: (tv.string as NSString).length, length: 0))
        try await accept(after: "\\end{", from: "\\begin{document}\n\\begin{itemize}\n")
        XCTAssertEqual(tv.string, "\\begin{document}\n\\begin{itemize}\n\\end{itemize}")

        // (3) Label key from the enclosing section, first (only) candidate.
        try await accept(after: "\\label{", from: "\\section{Pages from the river walk}\n")
        XCTAssertEqual(tv.string, "\\section{Pages from the river walk}\n\\label{sec:pages-from-the-river-walk}")
        undo.undo()
        XCTAssertEqual(tv.string, "\\section{Pages from the river walk}\n\\label{")

        // (4) Marked text (IME composition) never triggers a snippet: no list
        // opens while composing, and composing closes an open list.
        tv.string = "x \\se"
        tv.setSelectedRange(NSRange(location: 5, length: 0))
        tv.setMarkedText("か", selectedRange: NSRange(location: 0, length: 1), replacementRange: NSRange(location: 5, length: 0))
        XCTAssertTrue(tv.hasMarkedText())
        key(tv, " ", code: 49, flags: .control)
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertNil(tv.session)
        tv.unmarkText()
        tv.string = "x \\se"
        tv.setSelectedRange(NSRange(location: 5, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup before composing") { tv.session != nil }
        tv.setMarkedText("か", selectedRange: NSRange(location: 0, length: 1), replacementRange: NSRange(location: 5, length: 0))
        XCTAssertNil(tv.session, "composition closed the list")
        tv.acceptSelectedCompletion()
        XCTAssertEqual(tv.string, "x \\seか", "nothing was inserted by completion")
        tv.unmarkText()
    }

    /// 1-minute load average; wall-clock bounds are meaningless on this shared
    /// machine when other lanes are building (observed 11 lanes at load > 60).
    static var loadAverage1: Double {
        var load = [0.0, 0.0, 0.0]
        getloadavg(&load, 3)
        return load[0]
    }

    // MARK: keystroke path cost on the sample document

    private static let demo: String = {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("Samples/demo.tex")
        return try! String(contentsOf: url, encoding: .utf8)
    }()

    /// The synchronous candidate scan on `Samples/demo.tex` stays under 2 ms on
    /// the main thread in every context (the popup computes off-main anyway).
    @MainActor
    func testCandidateComputationOnDemoTexStaysUnderTwoMilliseconds() throws {
        let demo = Self.demo
        XCTAssertGreaterThan(demo.utf8.count, 5_000)
        let ns = demo as NSString
        let endDoc = ns.range(of: "\\end{document}")
        XCTAssertNotEqual(endDoc.location, NSNotFound)
        let metadata = Completion.Metadata.from(result(revision: 1, ["undefined reference 'x'"]))
        // Contexts: command, word, environment, reference — inserted before `\end{document}`.
        let cases: [(String, String)] = [("command", "\\se"), ("word", "not"), ("environment", "\\begin{do"), ("reference", "\\ref{se")]
        var report: [String] = []
        for (name, insert) in cases {
            let text = ns.replacingCharacters(in: NSRange(location: endDoc.location, length: 0), with: insert + "\n")
            let caret = endDoc.location + (insert as NSString).length
            if name != "reference" { // demo.tex has no labels
                XCTAssertFalse(Completion.suggestions(in: text, caretUTF16: caret, metadata: metadata).isEmpty, name)
            }
            var best = Double.infinity, total = 0.0
            let iterations = 50
            for _ in 0..<iterations {
                let t0 = MonotonicClock.nowNs()
                _ = Completion.completionRange(in: text, caretUTF16: caret)
                _ = Completion.suggestions(in: text, caretUTF16: caret, metadata: metadata)
                let ms = Double(MonotonicClock.nowNs() - t0) / 1e6
                best = min(best, ms); total += ms
            }
            let avg = total / Double(iterations)
            report.append("\(name) avg \(String(format: "%.3f", avg)) ms best \(String(format: "%.3f", best)) ms")
            XCTAssertLessThan(avg, 2, "\(name) completion on demo.tex (\(text.utf8.count) B) averaged \(avg) ms")
        }
        print("completion on demo.tex (\(demo.utf8.count) B, main thread, avg of 50): " + report.joined(separator: "; "))
    }

    /// With the list open, each keystroke on demo.tex costs the editor's own
    /// insertion plus a text copy and an enqueue; the scan runs off-main.
    @MainActor
    func testKeystrokeThroughOpenListOnDemoTexDoesNotScanOnMain() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless()
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        let demo = Self.demo as NSString
        let endDoc = demo.range(of: "\\end{document}").location
        tv.string = demo.replacingCharacters(in: NSRange(location: endDoc, length: 0), with: "\\s\n")
        tv.setSelectedRange(NSRange(location: endDoc + 2, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup") { tv.session != nil }
        let firstCompute = try XCTUnwrap(tv.lastOutcome?.computeMs)

        var keystrokeMs: [Double] = []
        let letters: [(String, UInt16)] = [("u", 32), ("b", 11), ("s", 1), ("e", 14), ("c", 8)]
        for (ch, code) in letters {
            let before = tv.scheduler.statistics.scheduled
            let t0 = MonotonicClock.nowNs()
            key(tv, ch, code: code)
            keystrokeMs.append(Double(MonotonicClock.nowNs() - t0) / 1e6)
            XCTAssertEqual(tv.scheduler.statistics.scheduled, before + 1, "the keystroke enqueued exactly one scan")
            XCTAssertNotNil(tv.session, "the list stays open while the scan runs")
            let expected = "\\s" + letters.prefix(keystrokeMs.count).map(\.0).joined()
            try await waitUntil("narrowed to \(expected)") { tv.session?.range.length == (expected as NSString).length && tv.session?.items.first?.label == "\\subsection{...}" }
        }
        let maxKeystroke = keystrokeMs.max()!, bestKeystroke = keystrokeMs.min()!
        print("keystroke through open list on demo.tex: best \(String(format: "%.3f", bestKeystroke)) ms, max \(String(format: "%.3f", maxKeystroke)) ms, all \(keystrokeMs.map { String(format: "%.3f", $0) }); first off-main scan \(String(format: "%.3f", firstCompute)) ms; last \(String(format: "%.3f", tv.lastOutcome?.computeMs ?? -1)) ms")
        key(tv, "\r", code: 36)
        XCTAssertTrue(tv.string.contains("\\subsection{}\n\\end{document}"))
        XCTAssertNil(tv.session)
        // Wall-clock bound on the best of the five keystrokes, only on a machine
        // that is not oversubscribed (the functional checks above always run).
        let load = Self.loadAverage1
        if load > 20 { throw XCTSkip("keystroke wall-clock bound not enforced: 1-minute load average \(String(format: "%.1f", load)) > 20") }
        XCTAssertLessThan(bestKeystroke, 20, "keystroke path with the list open (includes AppKit layout of the insertion)")
    }

    // MARK: performance

    /// Completion on a ~1 MB buffer. The measured average is printed so it can
    /// be reported; the assertion is generous so CI noise does not fail the suite.
    func testCompletionOnOneMegabyteBufferIsFast() throws {
        var text = ""
        text.reserveCapacity(1_100_000)
        let para = "\\section{Introduction} A naïve approach fails because theorem \\textbf{proofs} need \\emph{careful} statements. Résumé of the steps follows here with theory and thesis words.\n\n"
        while text.utf8.count < 1_000_000 { text += para }
        text += "\\begin{itemize} the"
        let caret = (text as NSString).length
        XCTAssertGreaterThan(text.utf8.count, 1_000_000)

        let commandText = text + " \\e"
        let iterations = 20
        var wordMs = 0.0, cmdMs = 0.0, bestWordMs = Double.infinity, bestCmdMs = Double.infinity
        for _ in 0..<iterations {
            let t0 = DispatchTime.now().uptimeNanoseconds
            let words = Completion.suggestions(in: text, caretUTF16: caret, result: nil)
            let t1 = DispatchTime.now().uptimeNanoseconds
            let cmds = Completion.suggestions(in: commandText, caretUTF16: caret + 3, result: nil)
            let t2 = DispatchTime.now().uptimeNanoseconds
            wordMs += Double(t1 - t0) / 1e6
            cmdMs += Double(t2 - t1) / 1e6
            bestWordMs = min(bestWordMs, Double(t1 - t0) / 1e6)
            bestCmdMs = min(bestCmdMs, Double(t2 - t1) / 1e6)
            XCTAssertEqual(words.first?.label, "theorem")
            XCTAssertEqual(cmds.first?.label, "\\end{itemize}")
        }
        wordMs /= Double(iterations); cmdMs /= Double(iterations)
        print("completion latency on \(text.utf8.count)-byte buffer: words \(String(format: "%.2f", wordMs)) ms (best \(String(format: "%.2f", bestWordMs))), commands \(String(format: "%.2f", cmdMs)) ms (best \(String(format: "%.2f", bestCmdMs))) (avg of \(iterations))")
        // Wall-clock bound on the best iteration, only on a machine that is not
        // oversubscribed; the printed numbers are the evidence either way.
        let load = Self.loadAverage1
        if load > 20 { throw XCTSkip("completion latency bound not enforced: 1-minute load average \(String(format: "%.1f", load)) > 20") }
        XCTAssertLessThan(bestWordMs, 20)
        XCTAssertLessThan(bestCmdMs, 20)
        // XCTest's own metric: one word completion plus one command completion per iteration.
        measure {
            _ = Completion.suggestions(in: text, caretUTF16: caret, result: nil)
            _ = Completion.suggestions(in: commandText, caretUTF16: caret + 3, result: nil)
        }
    }

    func testPagedSelectionMovesByAPageAndClampsAtTheEnds() {
        // 12 rows, 5 visible: down 0 → 5 → 10 → 11 → 11; up 11 → 6 → 1 → 0 → 0.
        var i = 0
        var down: [Int] = []
        for _ in 0..<4 { i = Completion.pagedSelection(from: i, pages: 1, pageSize: 5, count: 12); down.append(i) }
        XCTAssertEqual(down, [5, 10, 11, 11])
        var up: [Int] = []
        for _ in 0..<4 { i = Completion.pagedSelection(from: i, pages: -1, pageSize: 5, count: 12); up.append(i) }
        XCTAssertEqual(up, [6, 1, 0, 0])
        XCTAssertEqual(Completion.pagedSelection(from: 3, pages: 1, pageSize: 0, count: 12), 4, "a page is never smaller than one row")
        XCTAssertEqual(Completion.pagedSelection(from: 0, pages: 1, pageSize: 5, count: 0), 0)
        XCTAssertEqual(Completion.pagedSelection(from: 2, pages: 1, pageSize: 20, count: 3), 2, "a page wider than the list lands on the last row")
    }

    /// Page Up / Page Down / Home / End walk the open list without touching
    /// the text or the caret: a page is the rows the popup shows at once
    /// (`CompletionPopup.visibleRows`), the ends clamp instead of wrapping,
    /// Home/End go to the first/last row, the chosen row is scrolled into
    /// view, and Return still inserts what was walked to.
    @MainActor
    func testPageAndHomeEndKeysWalkTheListThroughTheRealTextView() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless() // never makeKey: the test must not steal focus
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        tv.string = "\\begin{document}\nx \\su"
        let end = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: end, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup") { tv.session != nil }
        let items = try XCTUnwrap(tv.session?.items)
        let last = items.count - 1
        XCTAssertEqual(items.count, Completion.maxSuggestions, "`\\su` fills the list")
        let popup = tv.completionPopup
        let page = popup.visibleRows
        XCTAssertTrue((1...Completion.maxSuggestions).contains(page), "\(page)")
        XCTAssertEqual(Completion.pagedSelection(from: 0, pages: 1, pageSize: page, count: items.count), min(page, last))

        // Page Down: one page, then the end, then it stays there (no wrap).
        key(tv, "\u{F72D}", code: 121)
        XCTAssertEqual(tv.session?.selectedIndex, min(page, last))
        key(tv, "\u{F72D}", code: 121); key(tv, "\u{F72D}", code: 121); key(tv, "\u{F72D}", code: 121)
        XCTAssertEqual(tv.session?.selectedIndex, last)
        XCTAssertEqual(popup.selectedRow, last)
        XCTAssertTrue(popup.accessibilityTable.rows(in: popup.accessibilityTable.visibleRect).contains(last), "the chosen row is scrolled into view")
        // Page Up: one page back from the end, then the top, then it stays.
        key(tv, "\u{F72C}", code: 116)
        XCTAssertEqual(tv.session?.selectedIndex, max(last - page, 0))
        key(tv, "\u{F72C}", code: 116); key(tv, "\u{F72C}", code: 116); key(tv, "\u{F72C}", code: 116)
        XCTAssertEqual(tv.session?.selectedIndex, 0)
        // End / Home.
        key(tv, "\u{F72B}", code: 119)
        XCTAssertEqual(tv.session?.selectedIndex, last)
        key(tv, "\u{F72B}", code: 119)
        XCTAssertEqual(tv.session?.selectedIndex, last, "End at the end stays")
        key(tv, "\u{F729}", code: 115)
        XCTAssertEqual(tv.session?.selectedIndex, 0)
        XCTAssertEqual(popup.selectedRow, 0)
        // Throughout: the list stayed open, the text and caret were untouched, the editor kept the keyboard.
        XCTAssertNotNil(tv.session)
        XCTAssertEqual(tv.string, "\\begin{document}\nx \\su")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: end, length: 0))
        XCTAssertTrue(window.firstResponder === tv)
        // ↓ still wraps as before, and Return inserts what End walked to.
        key(tv, "\u{F700}", code: 126)
        XCTAssertEqual(tv.session?.selectedIndex, last, "↑ from the top still wraps")
        key(tv, "\r", code: 36)
        XCTAssertEqual(tv.string, "\\begin{document}\nx " + items[last].insertText)
        XCTAssertNil(tv.session)
        // With no list open, the keys are the editor's own again: nothing is inserted and no list opens.
        let inserted = tv.string
        for (chars, code) in [("\u{F72B}", UInt16(119)), ("\u{F729}", 115), ("\u{F72D}", 121), ("\u{F72C}", 116)] { key(tv, chars, code: code) }
        XCTAssertEqual(tv.string, inserted)
        XCTAssertNil(tv.session)
    }

    /// Every implemented command has a documentation line: the hand-written
    /// `CommandDocs` line where one exists (it wins), otherwise the inventory's
    /// `description` as a sentence. The row and the pane show the inventory
    /// line only when the origin column does not already carry it.
    func testEveryInventoryCommandHasADocumentationLineAndHandWrittenOnesWin() throws {
        typealias Docs = EditorIntelligence.CommandDocs
        var handWritten = 0
        var inventory = 0
        for name in Completion.Vocabulary.names {
            let entry = try XCTUnwrap(Completion.Vocabulary.byName[name])
            let s = Completion.Suggestion(label: entry.label, insertText: "\\" + name, kind: .command, detail: entry.detail)
            let line = try XCTUnwrap(CompletionPopup.documentation(for: s), "\\\(name) has no documentation line")
            if let hand = Docs.documentation(for: name) {
                handWritten += 1
                XCTAssertEqual(line, hand, "\\\(name): the hand-written line wins")
            } else {
                inventory += 1
                XCTAssertEqual(line, CompletionPopup.inventoryDocumentation(for: s))
                XCTAssertTrue(line.hasPrefix(entry.label + ": "), line)
                XCTAssertTrue(line.contains(entry.description), line)
                XCTAssertTrue(line.hasSuffix("."), line)
                // The vocabulary row's origin column already says it, so the row does not repeat it…
                XCTAssertNil(CompletionPopup.displayedDocumentation(for: s), "\\\(name)")
                XCTAssertFalse(CompletionPopup.attributed(s).string.contains(" — " + line), "\\\(name)")
            }
        }
        print("completion documentation: \(handWritten) hand-written + \(inventory) from the inventory = \(handWritten + inventory) of \(Completion.Vocabulary.names.count) commands")
        XCTAssertEqual(handWritten + inventory, Completion.Vocabulary.names.count)
        XCTAssertGreaterThan(inventory, handWritten, "the inventory tier is what gives full coverage")
        // A concrete one with no hand-written line, as `commandSuggestions` builds it.
        let textsl = try XCTUnwrap(Completion.suggestions(in: "x \\textsl", caretUTF16: 9, metadata: nil).first)
        XCTAssertEqual(textsl.insertText, "\\textsl")
        XCTAssertNil(Docs.documentation(for: "textsl"))
        let description = try XCTUnwrap(Completion.Vocabulary.byName["textsl"]?.description)
        XCTAssertEqual(CompletionPopup.documentation(for: textsl), "\\textsl{...}: " + description + ".")
        XCTAssertEqual(CompletionPopup.documentation(for: textsl), "\\textsl{...}: slanted text (typeset as italic).")
        // …but a caller-supplied suggestion whose detail says something else does show it.
        let bare = Completion.Suggestion(label: "\\textsl", insertText: "\\textsl", kind: .command, detail: "supported by this compiler")
        XCTAssertEqual(CompletionPopup.displayedDocumentation(for: bare), CompletionPopup.documentation(for: textsl))
        XCTAssertTrue(CompletionPopup.attributed(bare).string.contains(" — \\textsl{...}: slanted text"))
        // A both-modes command carries its math behaviour too.
        let numrange = Completion.Suggestion(label: "\\numrange", insertText: "\\numrange", kind: .command, detail: "x")
        XCTAssertNil(Docs.documentation(for: "numrange"))
        XCTAssertNotNil(Completion.Vocabulary.byName["numrange"]?.mathDescription)
        XCTAssertTrue(try XCTUnwrap(CompletionPopup.documentation(for: numrange)).contains("; in math: "), CompletionPopup.documentation(for: numrange) ?? "")
        // The hand-written line is found through the label's argument shape
        // (`\section{...}`), which used to defeat the lookup.
        let section = try XCTUnwrap(Completion.suggestions(in: "x \\section", caretUTF16: 10, metadata: nil).first)
        XCTAssertEqual(section.label, "\\section{...}")
        XCTAssertEqual(CompletionPopup.documentation(for: section), Docs.table["section"])
        XCTAssertTrue(CompletionPopup.attributed(section).string.contains(" — \\section{title}: a numbered section heading."))
        // Environments: the hand-written line, else the inventory's.
        let itemize = Completion.Suggestion(label: "itemize", insertText: "itemize}", kind: .environment, detail: "supported by this compiler")
        XCTAssertEqual(CompletionPopup.documentation(for: itemize), "Bulleted list of \\item entries.")
        for name in Completion.knownEnvironments where Docs.environmentDocumentation(for: name) == nil {
            let s = Completion.Suggestion(label: name, insertText: name + "}", kind: .environment, detail: "supported by this compiler")
            let line = try XCTUnwrap(CompletionPopup.documentation(for: s), name)
            XCTAssertTrue(line.hasPrefix("\\begin{\(name)}: "), line)
        }
        XCTAssertNil(CompletionPopup.documentation(for: Completion.Suggestion(label: "figures/a.pdf", insertText: "figures/a.pdf", kind: .command, detail: "graphics file")))
    }

    /// The pane under the list shows the line for the chosen row through the
    /// real popup: the hand-written one leads for `\section`; a command with
    /// none still reads its inventory description.
    @MainActor
    func testTheDocumentationPaneReadsTheLineForTheChosenRow() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless() // never makeKey: the test must not steal focus
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        tv.string = "x \\section"
        tv.setSelectedRange(NSRange(location: 10, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup") { tv.session != nil }
        XCTAssertEqual(tv.session?.selected?.insertText, "\\section")
        let pane = tv.completionPopup.documentation
        XCTAssertEqual(pane.title, "\\section{...}{…}")
        XCTAssertTrue(pane.body.hasPrefix("\\section{title}: a numbered section heading. \\section* is unnumbered. Command · "), pane.body)
        key(tv, "\u{1B}", code: 53)
        tv.string = "x \\textsl"
        tv.setSelectedRange(NSRange(location: 9, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup 2") { tv.session != nil }
        XCTAssertEqual(tv.session?.selected?.insertText, "\\textsl")
        let description = try XCTUnwrap(Completion.Vocabulary.byName["textsl"]?.description)
        XCTAssertTrue(tv.completionPopup.documentation.body.contains(description), tv.completionPopup.documentation.body)
        key(tv, "\u{1B}", code: 53)
    }
}

// MARK: - End to end against the real preview-controller helper

/// The full route: real `flashtex-preview-controller` + real compiler behind
/// `ShellModel`, the parent's fetcher wiring, `SourceEditorView` hosted in a
/// window, and the completion list of the real `CompletingTextView`. Skipped
/// unless `FLASHTEX_PREVIEW_CONTROLLER` and `FLASHTEX_COMPILER` point at
/// built binaries (same convention as `PreviewControllerTests`).
@MainActor
final class CompletionLiveHelperTests: XCTestCase {
    private struct Host: View {
        let model: ShellModel
        var body: some View {
            SourceEditorView(
                text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                selection: model.selection,
                pendingEdit: model.pendingEdit,
                marks: model.editorMarks,
                result: model.result,
                editorRevision: model.editorRevision,
                projectIndexMetadata: model.completionMetadata,
                onCaretChange: { model.caretUTF16 = $0 }
            )
        }
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 20, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timed out waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    private func key(_ tv: NSTextView, _ chars: String, code: UInt16, flags: NSEvent.ModifierFlags = []) {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                                 windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: chars,
                                 charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        tv.keyDown(with: e)
    }

    /// Same as `waitUntil` with 0.2 ms slices, for timing a delivery.
    private func waitTight(_ what: String, timeout: TimeInterval = 20, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timed out waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 200_000)
        }
    }

    /// Inserts `insert` before `\end{document}`, opens the list for it through
    /// the real view, returns the delivered items and removes the probe again.
    private func popupSession(_ tv: CompletingTextView, model: ShellModel, insert: String) async throws -> CompletionSession {
        let ns = tv.string as NSString
        let at = ns.range(of: "\\end{document}").location
        XCTAssertNotEqual(at, NSNotFound)
        tv.setSelectedRange(NSRange(location: at, length: 0))
        tv.insertText(insert, replacementRange: NSRange(location: at, length: 0))
        try await waitUntil("model text") { model.activeText == tv.string }
        try await waitUntil("view revision") { tv.editorRevision == model.editorRevision }
        // The probe edit moved the editor revision: the held index metadata is
        // refused until the helper's preview for this revision brings fresh
        // vocabulary (the compile-result diagnostics may bind earlier).
        try await waitUntil("index metadata for \(insert) revision") { tv.projectIndexMetadata?.revision == model.editorRevision }
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup for \(insert)") { tv.session != nil }
        let session = try XCTUnwrap(tv.session)
        XCTAssertEqual(session.metadataRevision, model.editorRevision, "the list was built from metadata bound to the caret's revision")
        key(tv, "\u{1B}", code: 53)
        tv.insertText("", replacementRange: NSRange(location: at, length: (insert as NSString).length))
        try await waitUntil("model text restored") { model.activeText == tv.string }
        return session
    }

    func testHelperVocabularyReachesThePopupBoundToTheEditorRevision() async throws {
        guard let helper = PreviewControllerTests.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("pc-completion-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = root.appendingPathComponent("project/main.tex")
        // `\bibitem[Knu84]{knuth84}` is the optional-argument form the local scan
        // does not match, so a `\cite{` item for it can only come from the index.
        let source = """
        \\newcommand{\\myterm}{a defined term}
        \\begin{document}
        \\section{Intro}\\label{sec:intro}
        See \\ref{sec:intro} and \\cite{knuth84}: \\myterm.
        \\bibitem[Knu84]{knuth84} Knuth.
        \\end{document}

        """
        try source.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 800, height: 500), styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model))
        window.orderFrontRegardless() // never makeKey
        defer { window.orderOut(nil); model.detachController() }
        try await waitUntil("hosted editor") { TypingBenchDriver.findTextView(in: [window.contentView!]) != nil }
        let tv = try XCTUnwrap(TypingBenchDriver.findTextView(in: [window.contentView!]) as? CompletingTextView)
        window.makeFirstResponder(tv)

        model.attachController(at: helper)
        XCTAssertTrue(model.controllerAttached)
        try await waitUntil("first preview") { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        // The parent requests the vocabulary after every preview; wait for it to bind.
        try await waitUntil("bound metadata") { model.completionMetadata?.revision == model.editorRevision }
        let metadata = try XCTUnwrap(model.completionMetadata)
        let latency = try XCTUnwrap(model.completionFetcher.lastLatencyMs)
        print("live helper: request → bound completion metadata \(String(format: "%.1f", latency)) ms at editor revision \(metadata.revision); labels \(metadata.labels.map(\.name)) citations \(metadata.citations.map(\.name)) commands \(metadata.commands.count)")
        guard case .projectIndex(let versions) = metadata.origin else { return XCTFail("origin \(metadata.origin)") }
        XCTAssertEqual(versions["main.tex"], model.controllerState.durable["main.tex"]?.revision)
        XCTAssertEqual(metadata.labels.map(\.name), ["sec:intro"])
        let label = try XCTUnwrap(metadata.labels.first)
        XCTAssertEqual(label.definitions, 1)
        XCTAssertGreaterThanOrEqual(label.occurrences, 1)
        XCTAssertEqual(label.definedIn, "main.tex")
        XCTAssertEqual(metadata.citations.map(\.name), ["knuth84"])
        let citation = try XCTUnwrap(metadata.citations.first)
        XCTAssertEqual(citation.definitions, 1)
        let myterm = try XCTUnwrap(metadata.commands.first { $0.name == "myterm" })
        XCTAssertEqual(myterm.definitions, 1)
        XCTAssertGreaterThanOrEqual(myterm.occurrences, 1)
        XCTAssertFalse(metadata.truncated)

        // The hosted SourceEditorView handed both the revision and the metadata to the view.
        try await waitUntil("view bound") { tv.editorRevision == model.editorRevision && tv.projectIndexMetadata?.revision == model.editorRevision }
        XCTAssertEqual(tv.boundMetadata?.labels.map(\.name), ["sec:intro"])
        XCTAssertEqual(tv.boundMetadata?.revision, model.editorRevision)

        // `\ref{` lists the label (document scan) with the index agreeing; `\cite{`
        // lists the key only the index knows; `\my` lists the declared command.
        let refs = try await popupSession(tv, model: model, insert: "\\ref{")
        XCTAssertEqual(refs.items.map(\.label), ["sec:intro"])
        XCTAssertEqual(refs.items.first?.kind, .reference)
        let cites = try await popupSession(tv, model: model, insert: "\\cite{")
        XCTAssertEqual(cites.items.map(\.label), ["knuth84"])
        // main.tex is a LaTeX source (the helper's snapshot says so; nothing is declared as a bibliography here).
        XCTAssertEqual(metadata.documentKinds, ["main.tex": "latex"])
        XCTAssertEqual(metadata.declaredBibliographies, [])
        XCTAssertEqual(cites.items.first?.detail, "\\bibitem in main.tex · \(citation.occurrences) use\(citation.occurrences == 1 ? "" : "s") · revision \(cites.metadataRevision ?? -1)")
        let cmds = try await popupSession(tv, model: model, insert: "\\my")
        XCTAssertEqual(cmds.items.map(\.label), ["\\myterm"])
        XCTAssertEqual(cmds.items.first?.detail, "declared in main.tex · \(myterm.occurrences) use\(myterm.occurrences == 1 ? "" : "s") · revision \(cmds.metadataRevision ?? -1)")
        // Pickup through the real route, best of N: with index metadata bound
        // to the caret's revision, ⌃Space on `\cite{` → list showing the key
        // only the index knows. Timed from the keystroke to the session, run
        // loop turning in 0.2 ms slices; printed with the load it ran under.
        var pickupMs: [Double] = []
        var boundMs: [Double] = []
        for _ in 0..<10 {
            let ns = tv.string as NSString
            let at = ns.range(of: "\\end{document}").location
            tv.setSelectedRange(NSRange(location: at, length: 0))
            let edited = MonotonicClock.nowNs()
            tv.insertText("\\cite{", replacementRange: NSRange(location: at, length: 0))
            try await waitUntil("model text") { model.activeText == tv.string }
            try await waitTight("index metadata for the probe revision") { tv.projectIndexMetadata?.revision == model.editorRevision && tv.editorRevision == model.editorRevision }
            boundMs.append(Double(MonotonicClock.nowNs() - edited) / 1e6)
            let t0 = MonotonicClock.nowNs()
            key(tv, " ", code: 49, flags: .control)
            try await waitTight("popup") { tv.session != nil }
            pickupMs.append(Double(MonotonicClock.nowNs() - t0) / 1e6)
            XCTAssertEqual(tv.session?.items.map(\.label), ["knuth84"])
            XCTAssertEqual(tv.session?.metadataRevision, model.editorRevision)
            key(tv, "\u{1B}", code: 53)
            tv.insertText("", replacementRange: NSRange(location: at, length: 6))
            try await waitUntil("model text restored") { model.activeText == tv.string }
        }
        let load = CompletionTests.loadAverage1
        func summary(_ v: [Double], _ f: String) -> String {
            "best \(String(format: f, v.min()!)) / median \(String(format: f, v.sorted()[v.count / 2])) / max \(String(format: f, v.max()!)) ms"
        }
        print("live helper: \\cite{ pickup (⌃Space → list with the index key, best of \(pickupMs.count), 1-min load \(String(format: "%.1f", load))): "
              + summary(pickupMs, "%.2f") + "; edit → index metadata bound (helper compile + 3 complete replies + snapshot): " + summary(boundMs, "%.1f"))
        // Probe edits raced the helper's snapshot: any query answered after the
        // next edit was refused as stale rather than shown (count reported).
        let racedRefusals = model.completionFetcher.refusals
        print("live helper: \(racedRefusals) vocabulary quer\(racedRefusals == 1 ? "y" : "ies") refused because an edit followed the request")
        try await waitUntil("settled") { model.completionMetadata?.revision == model.editorRevision && model.inFlightRevision == nil }
        let boundRevision = model.editorRevision
        let boundVersions: [String: Int]
        if case .projectIndex(let v) = model.completionMetadata!.origin { boundVersions = v } else { return XCTFail("origin") }

        // Stale refusal, helper side: a query naming the previous snapshot is
        // answered with an error and discarded; the bound metadata is unchanged.
        let controller = try XCTUnwrap(model.controller)
        model.completionFetcher.request(sourceVersions: boundVersions.mapValues { $0 - 1 }, editorRevision: boundRevision) { try controller.send($0, $1) }
        try await waitUntil("stale refusal") { model.completionFetcher.refusals == racedRefusals + 1 }
        XCTAssertNil(model.completionFetcher.query)
        XCTAssertEqual(model.completionMetadata?.revision, boundRevision)
        XCTAssertTrue(model.workerLog.contains { $0.contains("completion metadata refused") && $0.contains("source versions changed") }, "\(model.workerLog.suffix(5))")

        // Stale refusal, editor side: an edit after the request moves the editor
        // revision, so the held metadata no longer binds until the next preview
        // brings metadata for the new revision.
        model.updateActiveText(model.activeText.replacingOccurrences(of: "Knuth.", with: "Knuth again."))
        XCTAssertNil(model.completionMetadata?.bound(to: model.editorRevision))
        XCTAssertNil(tv.projectIndexMetadata?.bound(to: model.editorRevision), "index metadata for revision \(boundRevision) is refused at revision \(model.editorRevision)")
        try await waitUntil("view sees new revision") { tv.editorRevision == model.editorRevision }
        // From here the view binds nothing from the old revision: either nothing
        // yet, or metadata the helper already produced for the new revision.
        XCTAssertNotEqual(tv.boundMetadata?.revision, boundRevision)
        try await waitUntil("rebound metadata") { model.completionMetadata?.revision == model.editorRevision }
        XCTAssertGreaterThan(model.completionMetadata!.revision, boundRevision)
        try await waitUntil("view rebound") { tv.projectIndexMetadata?.revision == model.editorRevision }
        XCTAssertEqual(tv.boundMetadata?.citations.map(\.name), ["knuth84"])
        print("live helper: rebound after edit in \(String(format: "%.1f", model.completionFetcher.lastLatencyMs ?? -1)) ms (request → metadata)")
    }

    /// `\cite{` through the real helper with a bibliography declared through
    /// Document Kinds: before the declaration the key cited in main.tex is
    /// offered as unresolved (with the hint to declare the .bib, since the
    /// snapshot says nothing is declared); after `declareBibliography` the
    /// same key is a "record in refs.bib (declared bibliography)" and ranks
    /// first. `refs.bib` is never read as a bibliography because of its name:
    /// the kind comes from the helper's `document_kinds` for that snapshot.
    func testCiteCompletionUsesTheDeclaredBibliographyKind() async throws {
        guard let helper = PreviewControllerTests.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("pc-cite-kinds-\(UUID().uuidString)")
        let dir = root.appendingPathComponent("project")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let tex = dir.appendingPathComponent("main.tex")
        try "\\begin{document}\nMain cites \\cite{knuth84} and \\cite{lamport94}.\n\\bibitem{lamport94} Lamport.\n\\end{document}\n"
            .write(to: tex, atomically: true, encoding: .utf8)
        try "@book{knuth84,\n  author = {Donald E. Knuth},\n  title = {The {\\TeX}book},\n  year = 1984\n}\n"
            .write(to: dir.appendingPathComponent("refs.bib"), atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        defer { unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 800, height: 500), styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model))
        window.orderFrontRegardless() // never makeKey
        defer { window.orderOut(nil); model.detachController() }
        try await waitUntil("hosted editor") { TypingBenchDriver.findTextView(in: [window.contentView!]) != nil }
        let tv = try XCTUnwrap(TypingBenchDriver.findTextView(in: [window.contentView!]) as? CompletingTextView)
        window.makeFirstResponder(tv)
        model.attachController(at: helper)
        try await waitUntil("first preview") { model.result?.revision == model.editorRevision && model.inFlightRevision == nil }
        try await waitUntil("bound metadata") { model.completionMetadata?.revision == model.editorRevision }

        // Nothing declared: the snapshot reports only main.tex as latex.
        let before = try XCTUnwrap(model.completionMetadata)
        XCTAssertEqual(before.documentKinds, ["main.tex": "latex"])
        XCTAssertEqual(before.declaredBibliographies, [])
        let unresolved = try await popupSession(tv, model: model, insert: "\\cite{")
        XCTAssertEqual(unresolved.items.map(\.label), ["lamport94", "knuth84"], "\\bibitem first, the cited-but-undefined key last")
        guard unresolved.items.count == 2 else { return XCTFail("expected two items, got \(unresolved.items.count)") }
        XCTAssertEqual(unresolved.items[0].detail, "\\bibitem in this document")
        XCTAssertTrue(unresolved.items[1].detail.hasPrefix("cited but not defined in a declared bibliography source (none declared: Project > Document Kinds) · "),
                      unresolved.items[1].detail)

        // Declare refs.bib through Document Kinds (the helper's typed open).
        let declared = await model.documentKinds.declareBibliography("refs.bib")
        XCTAssertEqual(declared, .declared(path: "refs.bib"), model.documentKinds.status)
        XCTAssertEqual(model.documentKinds.bibliographyPaths, ["refs.bib"])
        // The next preview (the probe edit's) carries a vocabulary snapshot with the kinds.
        let resolved = try await popupSession(tv, model: model, insert: "\\cite{")
        let after = try XCTUnwrap(model.completionMetadata)
        XCTAssertEqual(after.documentKinds, ["main.tex": "latex", "refs.bib": "bibliography"])
        XCTAssertEqual(after.declaredBibliographies, ["refs.bib"])
        XCTAssertEqual(after.citations.first { $0.name == "knuth84" }?.definedIn, "refs.bib")
        XCTAssertEqual(resolved.items.map(\.label), ["lamport94", "knuth84"])
        let knuth = try XCTUnwrap(resolved.items.first { $0.label == "knuth84" })
        XCTAssertTrue(knuth.detail.hasPrefix("record in refs.bib (declared bibliography) · "), knuth.detail)
        XCTAssertTrue(knuth.detail.hasSuffix("· revision \(resolved.metadataRevision ?? -1)"), knuth.detail)
        // A prefix that only the declared record matches lists just it.
        let kn = try await popupSession(tv, model: model, insert: "\\cite{kn")
        XCTAssertEqual(kn.items.map(\.label), ["knuth84"])
        XCTAssertEqual(kn.items.first?.kind, .citation)
        print("live helper: declared-bibliography cite completion — before: \(unresolved.items.map(\.detail)); after: \(resolved.items.map(\.detail))")
    }

    // MARK: recently used, declared environments, graphics files

    @MainActor
    func testRecentlyUsedCommandsAndEnvironmentsRankFirst() {
        // The ordering rule: recent names lead in recency order, the table
        // follows, names the table does not know are ignored.
        XCTAssertEqual(Completion.prioritising(["a", "b", "c", "d"], recent: ["c", "zzz", "a"]), ["c", "a", "b", "d"])
        XCTAssertEqual(Completion.prioritising(["a", "b"], recent: []), ["a", "b"])
        XCTAssertEqual(Completion.prioritising(["a", "b"], recent: ["zzz"]), ["a", "b"])
        // Through `suggestions`, isolated from the inventory by the `supported:` seam.
        let synthetic = ["tableofcontents", "textbf", "textit", "today"]
        let ordered = Completion.prioritising(synthetic, recent: ["today", "textbf"])
        let t = Completion.suggestions(in: "x \\t", caretUTF16: 4, result: nil, supported: ordered)
        XCTAssertEqual(t.map(\.insertText), ["\\today", "\\textbf", "\\tableofcontents", "\\textit"])
        XCTAssertEqual(t[1].snippet, .init(text: "\\textbf{}", caretUTF16: 8, stops: [9]), "a recent command keeps its snippet")
        XCTAssertEqual(Completion.suggestions(in: "x \\textit", caretUTF16: 9, result: nil, supported: ordered).first?.insertText, "\\textit",
                       "the exact spelling still ranks first")
        // Environments: recent names lead, then the compiler's table; a name
        // neither the table nor the document knows is not invented.
        let text = "\\begin{document}\n\\begin{a"
        let envs = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: nil,
                                          recentEnvironments: ["align*", "nonsense", "array"])
        let table = Completion.knownEnvironments.filter { $0.hasPrefix("a") }
        XCTAssertEqual(envs.map(\.label), ["align*", "array"] + table.filter { $0 != "align*" && $0 != "array" })
        XCTAssertEqual(envs.first?.snippet, .init(text: "align*}\n\n\\end{align*}", caretUTF16: 8, stops: [21]))
        // The store: commands by name, environments by name, closers and
        // keys ignored, a repeat moves to the front, bounded.
        let store = Completion.RecentlyUsed()
        store.record(Completion.Suggestion(label: "\\section{title}", insertText: "\\section", kind: .command, detail: ""))
        store.record(Completion.Suggestion(label: "\\frac{num}{den}", insertText: "\\frac", kind: .command, detail: ""))
        store.record(Completion.Suggestion(label: "itemize", insertText: "itemize}", kind: .environment, detail: ""))
        store.record(Completion.Suggestion(label: "\\end{itemize}", insertText: "\\end{itemize}", kind: .environment, detail: ""))
        store.record(Completion.Suggestion(label: "eq:main", insertText: "eq:main}", kind: .reference, detail: ""))
        store.record(Completion.Suggestion(label: "figures/plot.pdf", insertText: "figures/plot.pdf", kind: .command, detail: ""))
        store.record(Completion.Suggestion(label: "\\section{title}", insertText: "\\section", kind: .command, detail: ""))
        XCTAssertEqual(store.commands, ["section", "frac"])
        XCTAssertEqual(store.environments, ["itemize"])
        for i in 0..<(Completion.RecentlyUsed.limit + 5) {
            store.record(Completion.Suggestion(label: "\\c\(i)", insertText: "\\c\(i)", kind: .command, detail: ""))
        }
        XCTAssertEqual(store.commands.count, Completion.RecentlyUsed.limit)
        XCTAssertEqual(store.commands.first, "c\(Completion.RecentlyUsed.limit + 4)")
    }

    @MainActor
    func testAcceptingACommandRanksItFirstOnTheNextOpen() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless() // never makeKey: the test must not steal focus
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        tv.allowsUndo = true
        tv.string = "x \\su"
        let end = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: end, length: 0))
        let table = Completion.suggestions(in: tv.string, caretUTF16: end, result: nil).map(\.insertText)
        let subsetIndex = try XCTUnwrap(table.firstIndex(of: "\\subset"))
        XCTAssertGreaterThan(subsetIndex, 0, "the inventory does not start `\\su` with \\subset")

        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup") { tv.session != nil }
        XCTAssertEqual(tv.session?.items.map(\.insertText), table, "nothing accepted yet: the inventory's order")
        for _ in 0..<subsetIndex { key(tv, "\u{F701}", code: 125) }
        XCTAssertEqual(tv.session?.selected?.insertText, "\\subset")
        key(tv, "\r", code: 36)
        XCTAssertEqual(tv.string, "x \\subset")
        XCTAssertEqual(tv.recentlyUsed.commands, ["subset"])

        // The next `\su` puts \subset first; the rest keep the table's order.
        tv.string = "x \\su"
        tv.setSelectedRange(NSRange(location: end, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("popup again") { tv.session != nil }
        XCTAssertEqual(tv.session?.items.map(\.insertText), ["\\subset"] + table.filter { $0 != "\\subset" })
        XCTAssertEqual(tv.session?.selectedIndex, 0)
        key(tv, "\u{1B}", code: 53)

        // A bare view keeps its own list; the hosted editor installs the shared one.
        let other = try XCTUnwrap(CompletingTextView.scrollable().documentView as? CompletingTextView)
        XCTAssertTrue(other.recentlyUsed.commands.isEmpty)
        XCTAssertFalse(other.recentlyUsed === tv.recentlyUsed)
    }

    func testDeclaredEnvironmentsAreOfferedAfterBegin() {
        let text = """
        \\newtheorem{lemma}{Lemma}[section]
        \\newtheorem*{remark}{Remark}
        \\newenvironment{proofsketch}{\\begin{proof}[Sketch]}{\\end{proof}}
        \\renewenvironment{abstract}{}{}
        \\begin{document}
        \\begin{le
        """
        XCTAssertEqual(Completion.declaredEnvironments(in: text), ["lemma", "remark", "proofsketch", "abstract"])
        let s = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: nil)
        XCTAssertEqual(s.map(\.label), ["letter", "lemma"], "the compiler's table first, then the document's declarations")
        XCTAssertEqual(s.last?.detail, "declared in this document")
        XCTAssertEqual(s.last?.insertText, "lemma}")
        XCTAssertEqual(s.last?.snippet, .init(text: "lemma}\n\n\\end{lemma}", caretUTF16: 7, stops: [19]))
        // The starred form and `\newenvironment`, before any `\begin{}` uses them.
        let star = text.replacingOccurrences(of: "\\begin{le", with: "\\begin{rem")
        XCTAssertEqual(Completion.suggestions(in: star, caretUTF16: (star as NSString).length, metadata: nil).map(\.label), ["remark"])
        let sketch = text.replacingOccurrences(of: "\\begin{le", with: "\\begin{proofs")
        XCTAssertEqual(Completion.suggestions(in: sketch, caretUTF16: (sketch as NSString).length, metadata: nil).map(\.label), ["proofsketch"])
        // A declaration that is also used is offered once, as declared.
        let used = text.replacingOccurrences(of: "\\begin{le", with: "\\begin{lemma}\n\\end{lemma}\n\\begin{lem")
        let u = Completion.suggestions(in: used, caretUTF16: (used as NSString).length, metadata: nil)
        XCTAssertEqual(u.map(\.label), ["lemma"])
        XCTAssertEqual(u.first?.detail, "declared in this document")
        // `\end{` still closes the innermost open environment first.
        let closing = text.replacingOccurrences(of: "\\begin{le", with: "\\begin{lemma}\n\\end{")
        XCTAssertEqual(Completion.suggestions(in: closing, caretUTF16: (closing as NSString).length, metadata: nil).first?.label, "lemma")
        XCTAssertTrue(Completion.declaredEnvironments(in: "\\newtheorem\n\\newtheorem*\\newenvironment{}").isEmpty)
    }

    func testIncludegraphicsOffersImageFilesUnderTheProjectRoot() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-graphics-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: root) }
        for rel in ["figures/plot.pdf", "figures/plot.png", "img/photo.JPG", "notes.tex", "refs.bib", ".hidden/secret.png", "figures/data.csv"] {
            let url = root.appendingPathComponent(rel)
            try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try Data().write(to: url)
        }
        XCTAssertEqual(Completion.graphicsFiles(under: root), ["figures/plot.pdf", "figures/plot.png", "img/photo.JPG"])
        XCTAssertEqual(Completion.graphicsFiles(under: root, limit: 1).count, 1)
        XCTAssertTrue(Completion.graphicsFiles(under: root.appendingPathComponent("missing")).isEmpty)

        // `\includegraphics{` is a graphics key — also behind its usual optional argument.
        let g = "\\includegraphics[width=2cm]{plo"
        XCTAssertEqual(Completion.token(in: g, caretUTF16: (g as NSString).length), .word(text: "plo", start: 28, end: 31, context: .graphics))
        XCTAssertTrue(Completion.opensAutomatically(Completion.token(in: g, caretUTF16: 31)))
        let files = ["chapters/one.tex", "figures/plot.tex"]
        let s = Completion.suggestions(in: g, caretUTF16: 31, metadata: nil, projectFiles: files, graphicsFiles: Completion.graphicsFiles(under: root))
        XCTAssertEqual(s.map(\.label), ["figures/plot.pdf", "figures/plot.png"])
        XCTAssertEqual(s.first?.detail, "graphics file")
        XCTAssertEqual(s.first?.insertText, "figures/plot.pdf")
        // `\input{` keeps the document list and never lists an image.
        let d = Completion.suggestions(in: "\\input{plo", caretUTF16: 10, metadata: nil, projectFiles: files, graphicsFiles: ["figures/plot.pdf"])
        XCTAssertEqual(d.map(\.label), ["figures/plot.tex"])
        XCTAssertEqual(d.first?.detail, "project document")
        // The optional argument is skipped for every key context.
        XCTAssertEqual(Completion.suggestions(in: "\\usepackage[utf8]{inpu", caretUTF16: 22, metadata: nil).map(\.label), ["inputenc"])
        let cite = "\\bibitem{knuth}\\cite[p.~3]{kn"
        XCTAssertEqual(Completion.suggestions(in: cite, caretUTF16: (cite as NSString).length, metadata: nil).map(\.label), ["knuth"])
        XCTAssertNil(Completion.token(in: "\\item[x]{ab", caretUTF16: 11).flatMap { t -> Completion.Token.Context? in
            if case .word(_, _, _, let c) = t { return c == .none ? nil : c } else { return nil }
        }, "a bracket before a plain brace is not a key context")
    }

    @MainActor
    func testIncludegraphicsListsTheProjectRootsImagesThroughTheTextView() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-graphics-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: root) }
        try FileManager.default.createDirectory(at: root.appendingPathComponent("figures"), withIntermediateDirectories: true)
        try Data().write(to: root.appendingPathComponent("figures/plot.pdf"))
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless() // never makeKey: the test must not steal focus
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        tv.graphicsRoot = { root }
        tv.string = "\\includegraphics[width=\\linewidth]{plo"
        let end = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: end, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("graphics list") { tv.session != nil }
        XCTAssertEqual(tv.session?.items.map(\.label), ["figures/plot.pdf"])
        XCTAssertEqual(tv.session?.items.first?.detail, "graphics file")
        key(tv, "\u{1B}", code: 53)
        // No root (a bare view, an unsaved buffer): nothing is offered, nothing is walked.
        tv.graphicsRoot = { nil }
        tv.string = "\\includegraphics{plo"
        let short = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: short, length: 0))
        key(tv, " ", code: 49, flags: .control)
        try await waitUntil("empty outcome") { tv.lastOutcome?.caretUTF16 == short }
        XCTAssertEqual(tv.lastOutcome?.items, [])
        XCTAssertNil(tv.session)
    }

    func testFontArgumentsOfferInstalledFamilies() {
        // `\setmainfont{`, `\fontspec[opts]{` and friends complete against the
        // families the job listed (the engine's index, `InstalledFonts`); the
        // prefix is fuzzy-matched and the whole family name is inserted.
        let families = ["Georgia", "Helvetica", "Helvetica Neue", "Latin Modern Roman", "Times New Roman"]
        for typed in ["\\setmainfont{Hel", "\\setsansfont{Hel", "\\setmonofont{Hel", "\\setmathfont{Hel", "\\fontspec[Scale=0.9]{Hel"] {
            let s = Completion.suggestions(in: typed, caretUTF16: (typed as NSString).length, metadata: nil, fontFamilies: families)
            XCTAssertEqual(s.map(\.label), ["Helvetica", "Helvetica Neue"], typed)
            XCTAssertEqual(s.first?.detail, "installed font family", typed)
            XCTAssertEqual(s.first?.insertText, "Helvetica", typed)
        }
        let empty = "\\setmainfont{"
        XCTAssertEqual(Completion.suggestions(in: empty, caretUTF16: (empty as NSString).length, metadata: nil, fontFamilies: families).map(\.label), families)
        // Without a listing (no built worker) the argument offers nothing, and
        // an ordinary command's argument is not a font argument.
        XCTAssertEqual(Completion.suggestions(in: empty, caretUTF16: (empty as NSString).length, metadata: nil), [])
        let other = "\\textbf{Hel"
        XCTAssertFalse(Completion.suggestions(in: other, caretUTF16: (other as NSString).length, metadata: nil, fontFamilies: families).map(\.label).contains("Helvetica"))
    }
}
