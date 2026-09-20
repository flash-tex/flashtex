import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac
@testable import FlashTeXEditorCore

/// `protocol/fixtures/caret-context-v1.json` drives this suite and
/// `crates/bridge/tests/caret_context.rs`; `scripts/caret_context_oracle.py`
/// checks the same table against pdflatex. One table, three consumers: the two
/// implementations cannot drift from each other, and neither can drift from
/// what pdflatex actually accepts.
final class CaretContextTests: XCTestCase {
    struct Case: Decodable {
        var name: String
        var preamble: String?
        var before: String
        var after: String
        var context: Expected
        var proposal: String
        var insert: String?
        var advisories: [String]
    }
    struct Expected: Decodable {
        var mode: String
        var delimiter: String?
        var environment: String?
        var environments: [String]
        var amsmath: Bool
        var wrap: String
    }
    struct Table: Decodable {
        var preamble: String
        var cases: [Case]
    }

    static var table: Table {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        let data = try! Data(contentsOf: url.appendingPathComponent("protocol/fixtures/caret-context-v1.json"))
        return try! JSONDecoder().decode(Table.self, from: data)
    }

    /// The document each case's caret lives in, and the caret's byte offset.
    private func document(_ c: Case, _ table: Table) -> (String, Int) {
        let preamble = c.preamble ?? table.preamble
        let opening = "\\begin{document}\n"
        return (preamble + opening + c.before + c.after,
                (preamble + opening + c.before).utf8.count)
    }

    func testDerivesEveryFixtureContext() {
        let table = Self.table
        for c in table.cases {
            let (text, caret) = document(c, table)
            let got = CaretContext.derive(text, caretByte: caret)
            XCTAssertEqual(got.mode.rawValue, c.context.mode, "\(c.name): mode")
            XCTAssertEqual(got.wrap.rawValue, c.context.wrap, "\(c.name): wrap")
            XCTAssertEqual(got.delimiter, c.context.delimiter, "\(c.name): delimiter")
            XCTAssertEqual(got.amsmath, c.context.amsmath, "\(c.name): amsmath")
            // `document` is open in every real file; the fixture lists only the
            // environments inside the body.
            XCTAssertEqual(got.environment == "document" ? nil : got.environment,
                           c.context.environment, "\(c.name): environment")
            XCTAssertEqual(got.environments.filter { $0 != "document" },
                           c.context.environments, "\(c.name): environments")
        }
    }

    func testNormalizesEveryFixtureProposal() {
        for c in Self.table.cases {
            let context = CaretContext(
                mode: CaretMode(rawValue: c.context.mode)!,
                delimiter: c.context.delimiter, environment: c.context.environment,
                environments: c.context.environments, amsmath: c.context.amsmath,
                wrap: CaretWrap(rawValue: c.context.wrap)!)
            let got = context.normalize(c.proposal)
            XCTAssertEqual(got.text, c.insert, "\(c.name): inserted text")
            for advisory in c.advisories {
                XCTAssertTrue(got.advisories.contains(advisory),
                              "\(c.name): missing advisory \(advisory), got \(got.advisories)")
            }
            if c.advisories.isEmpty {
                XCTAssertTrue(got.advisories.isEmpty, "\(c.name): unexpected \(got.advisories)")
            }
        }
    }

    /// The property the owner's report is really about, checked beyond the
    /// table: whatever a recogniser returns, the text inserted at a math caret
    /// never carries a math delimiter, and no insertion is ever unbalanced.
    func testNoInsertionEverDoubleWrapsOrUnbalances() {
        let proposals = ["$x^2$", "$$x^2$$", "\\( x^2 \\)", "\\[ x^2 \\]",
                         "\\begin{equation} x^2 \\end{equation}", "$\\frac{a}{b}$", "x^2",
                         "\\[\\[x\\]\\]", "$$ \\[ x \\] $$"]
        let carets = [("Let $a + ", " + b$.\n"), ("\\[ a + ", " + b \\]\n"),
                      ("\\begin{equation}\n", "\n\\end{equation}\n"), ("Prose ", " prose.\n"),
                      ("\\begin{tabular}{cc} a & ", " \\\\ \\end{tabular}\n")]
        for (before, after) in carets {
            let text = "\\documentclass{article}\n\\begin{document}\n" + before + after
            let caret = (text.range(of: before).map { text.distance(from: text.startIndex, to: $0.upperBound) })!
            let byte = String(text.prefix(caret)).utf8.count
            let context = CaretContext.derive(text, caretByte: byte)
            for proposal in proposals {
                guard let inserted = context.normalize(proposal).text else { continue }
                if context.wrap == .alreadyMath {
                    XCTAssertFalse(inserted.contains("$") || inserted.contains("\\[") || inserted.contains("\\("),
                                   "double wrap at \(before) from \(proposal): \(inserted)")
                }
                if context.wrap == .inline {
                    XCTAssertFalse(inserted.contains("\\["),
                                   "display math in a cell at \(before) from \(proposal): \(inserted)")
                }
                let spliced = before + inserted + after
                XCTAssertEqual(spliced.filter { $0 == "$" }.count % 2, 0,
                               "unbalanced $ at \(before) from \(proposal): \(inserted)")
            }
        }
    }

    /// Normalising twice changes nothing: `normalize` runs once, at review, and
    /// `insertable` re-checks when the edit is issued.
    func testNormalizingIsIdempotent() {
        for c in Self.table.cases {
            let context = CaretContext(
                mode: CaretMode(rawValue: c.context.mode)!,
                delimiter: c.context.delimiter, environment: c.context.environment,
                amsmath: c.context.amsmath, wrap: CaretWrap(rawValue: c.context.wrap)!)
            guard let once = context.normalize(c.proposal).text else { continue }
            XCTAssertEqual(context.normalize(once).text, once, "\(c.name): not idempotent")
            XCTAssertEqual(context.insertable(once), once, "\(c.name): refused at insertion")
        }
    }

    /// `CaretContext` decides how captures are wrapped and `SyntaxHighlighter`
    /// decides what the editor paints as math. They read the same documents, so
    /// a disagreement is a bug in one of them; this is what keeps the two
    /// environment lists and delimiter rules honest as either side changes.
    func testAgreesWithTheSyntaxHighlighter() {
        let documents = [
            "Plain text with $a + b$ inline and more text.\n",
            "\\[ x^2 \\] then text, then \\begin{equation} y \\end{equation} done.\n",
            "\\begin{verbatim}\ncost $5 here\n\\end{verbatim}\nafter\n",
            "% a comment with $5 in it\ntext after\n",
            "It costs \\$5 and then $x$ and text.\n",
            "\\begin{equation}\\begin{aligned} a &= b \\end{aligned}\\end{equation}\nafter\n",
            "text \\( u \\) text $$ v $$ text\n",
        ]
        for text in documents {
            let ns = text as NSString
            var highlighter = SyntaxHighlighter()
            highlighter.reset(ns)
            for utf16 in 0...ns.length {
                let byte = String(text.prefix(utf16)).utf8.count
                let caret = CaretContext.derive(text, caretByte: byte)
                let mode = highlighter.mode(at: utf16, text: ns)
                // The highlighter distinguishes more math spellings than the
                // caret context needs; compare the question both answer.
                XCTAssertEqual(caret.mode == .inlineMath || caret.mode == .displayMath, mode.isMath,
                               "math disagreement at \(utf16) in \(text.debugDescription): "
                                 + "caret \(caret.mode.rawValue) vs highlighter \(mode)")
                if case .verbatim = mode {
                    XCTAssertEqual(caret.mode, .verbatim,
                                   "verbatim disagreement at \(utf16) in \(text.debugDescription)")
                }
            }
        }
    }

    /// The sentence the recogniser is given must name the rule: that half of the
    /// fix is what stops bad LaTeX being produced at all.
    func testPromptSentenceStatesTheRule() {
        let cases = [("Let $a + ", "NO delimiters"),
                     ("\\begin{tabular}{cc} a & ", "NOT legal here"),
                     ("\\begin{verbatim}\n", "literally"),
                     ("Prose.\n\n", "\\[")]
        for (before, expected) in cases {
            let text = "\\documentclass{article}\n\\begin{document}\n" + before + "\n"
            let byte = ("\\documentclass{article}\n\\begin{document}\n" + before).utf8.count
            let sentence = CaretContext.derive(text, caretByte: byte).describe()
            XCTAssertTrue(sentence.contains(expected), "caret \(before): \(sentence)")
        }
    }

    /// A capture landing in a `%` comment must stay in that comment: the
    /// newline padding `insertionText` adds for display math would push it out.
    func testCommentInsertionKeepsTheTextInsideTheComment() {
        let text = "text % note: \nmore text\n"
        let byte = "text % note: ".utf8.count
        let result = Insertion.captureInsertion("x^2", into: text, atByte: byte)
        XCTAssertEqual(result.text, "x^2")
        XCTAssertEqual(result.caret.mode, .comment)
        let spliced = String(text.prefix(byte)) + (result.text ?? "") + String(text.dropFirst(byte))
        XCTAssertTrue(spliced.hasPrefix("text % note: x^2\n"), spliced)
    }

    // MARK: - approval-time wrap (lane lane-capture-flow)

    private func wrapping(_ before: String, _ after: String, _ latex: String) -> WrapDecision {
        CaretContext.wrapping(for: latex, in: before + after, atByte: before.utf8.count)
    }

    /// Rule 1: caret in text on its own line + a formula → display math on its own lines.
    func testWrappingFormulaOnItsOwnLineBecomesDisplayMath() {
        let d = wrapping("Some prose.\n\n", "\n\nMore prose.\n", "x = 2y + 1")
        XCTAssertEqual(d, .wrap(.init(kind: .displayMath, prefix: "\\[ ", suffix: " \\]")))
        XCTAssertEqual(d.wrapping?.applied(to: "x = 2y + 1"), "\\[ x = 2y + 1 \\]")
        // Indented on an otherwise blank line: the block still gets its own lines.
        let indented = wrapping("Some prose.\n\n  ", "\n\nMore prose.\n", "\\frac{a}{b}")
        XCTAssertEqual(indented.wrapping?.prefix, "\n\\[ ")
        XCTAssertEqual(indented.wrapping?.suffix, " \\]")
    }

    /// Rule 2: caret inside a sentence (or a tabular cell) + a formula → inline math.
    func testWrappingFormulaInsideASentenceOrCellBecomesInlineMath() {
        XCTAssertEqual(wrapping("We know that ", " holds for all n.\n", "x^2 + y^2"),
                       .wrap(.init(kind: .inlineMath, prefix: "$", suffix: "$")))
        XCTAssertEqual(wrapping("\\begin{tabular}{c}\n", " \\\\\n\\end{tabular}\n", "a + b").wrapping?.kind, .inlineMath)
    }

    /// Rule 3: caret already in math → bare; a proposal with its own delimiters cannot be wrapped into legality.
    func testWrappingAtAMathCaretIsBareAndRefusesDelimitedProposals() {
        XCTAssertEqual(wrapping("Let $a + ", "$ hold.\n", "x^2"), .wrap(.init(kind: .asIs, prefix: "", suffix: "")))
        XCTAssertEqual(wrapping("\\begin{align}\n", "\n\\end{align}\n", "y = mx + c").wrapping?.kind, .asIs)
        guard case .unsafe(let why) = wrapping("Let $a + ", "$ hold.\n", "$x^2$") else { return XCTFail("must refuse") }
        XCTAssertTrue(why.contains("inside math"), why)
        XCTAssertTrue(why.contains("move the caret"), why)
    }

    /// Rule 4: a proposal that already carries delimiters or an environment is never wrapped again.
    func testWrappingNeverDoubleWraps() {
        XCTAssertEqual(wrapping("We know that ", " holds.\n", "$x^2$"), .wrap(.init(kind: .asIs, prefix: "", suffix: "")))
        XCTAssertEqual(wrapping("Some prose.\n\n", "\n\nMore.\n", "\\[ x^2 \\]"), .wrap(.init(kind: .asIs, prefix: "", suffix: "")))
        XCTAssertEqual(wrapping("Hello", " world\n", "\\begin{center}x\\end{center}"),
                       .wrap(.init(kind: .asIs, prefix: "\n", suffix: "\n")), "a block mid-line gets its own lines")
        XCTAssertEqual(wrapping("Some prose.\n\n", "\n\nMore.\n", "\\begin{equation}x\\end{equation}"),
                       .wrap(.init(kind: .asIs, prefix: "", suffix: "")), "an environment is never wrapped")
        // Display math where only inline fits (mid-sentence, a cell) is the one
        // thing a wrap cannot fix; the reason says where to put the caret.
        for display in ["\\[ x^2 \\]", "\\begin{equation}x\\end{equation}"] {
            guard case .unsafe(let why) = wrapping("We know that ", " holds.\n", display) else { return XCTFail("must refuse \(display)") }
            XCTAssertTrue(why.contains("own line"), why)
        }
    }

    /// Rule 5: a tikzpicture outside any figure is inserted bare on its own lines; no automatic figure.
    func testWrappingTikzPictureIsBareOnItsOwnLines() {
        let tikz = "\\begin{tikzpicture}\\draw (0,0) -- (1,1);\\end{tikzpicture}"
        let d = wrapping("Hello", " world\n", tikz)
        XCTAssertEqual(d, .wrap(.init(kind: .asIs, prefix: "\n", suffix: "\n")))
        XCTAssertFalse(d.wrapping!.applied(to: tikz).contains("figure"))
        XCTAssertEqual(wrapping("Hello\n", "\nworld\n", tikz), .wrap(.init(kind: .asIs, prefix: "", suffix: "")), "already on its own line")
    }

    /// Rule 6: text proposals are inserted bare.
    func testWrappingProseIsBare() {
        XCTAssertEqual(wrapping("Hello ", "world\n", "see figure 2"), .wrap(.init(kind: .asIs, prefix: "", suffix: "")))
        XCTAssertEqual(wrapping("Some prose.\n\n", "\n\nMore.\n", "Theorem 2 follows."), .wrap(.init(kind: .asIs, prefix: "", suffix: "")))
    }

    /// Rule 7: verbatim and comments take the transcription exactly.
    func testWrappingInVerbatimOrCommentIsLiteral() {
        XCTAssertEqual(wrapping("\\begin{verbatim}\n", "\n\\end{verbatim}\n", "x = y + 1"), .wrap(.init(kind: .asIs, prefix: "", suffix: "")))
        XCTAssertEqual(wrapping("% note ", "\n", "x = y + 1"), .wrap(.init(kind: .asIs, prefix: "", suffix: "")))
    }

    /// The owner's `x = 2y + 1` has no structural command; its letters-and-
    /// operators shape still makes it a formula. Placeholders and prose stay prose.
    func testLooksLikeFormulaSeparatesFormulasFromProse() {
        for formula in ["x = 2y + 1", "a + b = c", "f(x) = 3x - 1", "sin(x) + cos(x) = 1", "\\mathbb{R} = X", "n! = n(n-1)!", "a < b"] {
            XCTAssertTrue(CaretContext.looksLikeFormula(formula), formula)
        }
        for prose in ["see figure 2", "<F>", "12", "x", "A = the set", "Hello world", "for all real numbers", "-x"] {
            XCTAssertFalse(CaretContext.looksLikeFormula(prose), prose)
        }
        XCTAssertEqual(CaretContext(wrap: .display).normalize("x = 2y + 1").text, "\\[ x = 2y + 1 \\]")
        XCTAssertEqual(CaretContext(wrap: .inline).normalize("x = 2y + 1").text, "$x = 2y + 1$")
    }

    /// The wire form carries the kind label the bridge journals with the edit.
    func testWrappingWireFormCarriesTheKind() {
        let w = InsertionWrapping(kind: .displayMath, prefix: "\\[ ", suffix: " \\]").wire
        XCTAssertEqual(w, .init(prefix: "\\[ ", suffix: " \\]", kind: "display_math"))
        XCTAssertEqual(w.applied(to: "x"), "\\[ x \\]")
    }

    // MARK: - the shell's mirror of a caret-mode anchor

    /// A caret anchor follows edits like the caret (bridge `AnchorMode::Caret`);
    /// a fixed one is invalidated by an insertion at it (the owner's bug).
    func testCaretAnchorFollowsTypingAtItWhileAFixedAnchorIsDropped() {
        let binding = TransferV1.AnchorBinding(projectId: "p", path: "main.tex", revision: 1, startByte: 5, endByte: 5, sourceSha256: String(repeating: "0", count: 64))
        let caret = TransferV1.Anchor(destinationId: "mac-caret-1", projectId: "p", path: "main.tex", pinnedRevision: 1, currentRevision: 1,
                                      startByte: 5, endByte: 5, valid: true, binding: binding, mode: .caret)
        let fixed = TransferV1.Anchor(destinationId: "mac-anchor-1", projectId: "p", path: "main.tex", pinnedRevision: 1, currentRevision: 1,
                                      startByte: 5, endByte: 5, valid: true, binding: binding, mode: nil)
        let typed = DestinationTracking.follow(caret, startByte: 5, endByte: 5, replacementBytes: 2, revision: 2)
        XCTAssertEqual([typed.startByte, typed.endByte, typed.currentRevision], [7, 7, 2])
        XCTAssertTrue(typed.valid)
        let spanned = DestinationTracking.follow(typed, startByte: 2, endByte: 9, replacementBytes: 1, revision: 3)
        XCTAssertEqual([spanned.startByte, spanned.endByte], [3, 3], "collapsed after the replacement")
        XCTAssertTrue(spanned.valid)
        let after = DestinationTracking.follow(spanned, startByte: 4, endByte: 5, replacementBytes: 0, revision: 4)
        XCTAssertEqual(after.startByte, 3, "an edit after it leaves it alone")
        let dropped = DestinationTracking.follow(fixed, startByte: 5, endByte: 5, replacementBytes: 2, revision: 2)
        XCTAssertFalse(dropped.valid)
    }
}
