import XCTest
@testable import FlashTeXMac

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
}
