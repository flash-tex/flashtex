import XCTest
@testable import FlashTeXMac

/// Inline math preview on hover (lane mac-math-hover, deferred from
/// mac-editor-dx-3): the span-finding pure function
/// (`EditorIntelligence.inlineMathSpan`). The engine's crop of the page is
/// covered by `EngineV3MathHoverTests`.
final class MathHoverTests: XCTestCase {
    typealias EI = EditorIntelligence

    // MARK: inlineMathSpan (pure)

    func testInlineMathSpanDollarAndParen() {
        // Indices: 0123456789…: "Text $a+b$ and \(c\) end." — `$a+b$` is
        // 5..<10, `\(c\)` (a literal backslash-paren, not an escape) is 15..<20.
        let s = "Text $a+b$ and \\(c\\) end." as NSString
        // Anywhere inside `$a+b$`, including on a delimiter, resolves to the
        // whole formula (delimiters included).
        for at in 5...9 { XCTAssertEqual(EI.inlineMathSpan(in: s, at: at), NSRange(location: 5, length: 5), "at \(at)") }
        XCTAssertNil(EI.inlineMathSpan(in: s, at: 4)) // the space before "$"
        XCTAssertNil(EI.inlineMathSpan(in: s, at: 10)) // the space after "$"
        for at in 15...19 { XCTAssertEqual(EI.inlineMathSpan(in: s, at: at), NSRange(location: 15, length: 5), "at \(at)") } // \(c\)
        XCTAssertNil(EI.inlineMathSpan(in: s, at: 20)) // the space after "\)"
        XCTAssertNil(EI.inlineMathSpan(in: s, at: 0)) // plain text
    }

    func testInlineMathSpanExcludesDisplayMathAndEnvironments() {
        XCTAssertNil(EI.inlineMathSpan(in: "$$a+b$$" as NSString, at: 3))
        XCTAssertNil(EI.inlineMathSpan(in: "\\[a+b\\]" as NSString, at: 3))
        XCTAssertNil(EI.inlineMathSpan(in: "\\begin{align}a+b\\end{align}" as NSString, at: 14))
    }

    func testInlineMathSpanTwoLinesOkThreeLinesNil() {
        let two = "$a+\nb$" as NSString // "$a+\n" / "b$": 2 lines, closes on the second
        XCTAssertEqual(EI.inlineMathSpan(in: two, at: 1), NSRange(location: 0, length: two.length))
        XCTAssertEqual(EI.inlineMathSpan(in: two, at: two.length - 1), NSRange(location: 0, length: two.length))
        let three = "$a\n+\nb$" as NSString // opens, then two more line breaks before it closes: 3 lines
        XCTAssertNil(EI.inlineMathSpan(in: three, at: 1))
        XCTAssertNil(EI.inlineMathSpan(in: three, at: three.length - 1))
    }
}
