import XCTest
import FlashTeXProtocol
@testable import FlashTeXAccessibility

/// Line/word/character navigation with dual offsets, diagnostics at the caret,
/// and rotor categories over `AccessibleEditorModel`.
final class EditorModelTests: XCTestCase {
    // Line 2 has a decomposed "naïve" (i + U+0308: one Character, 2 scalars,
    // 2 UTF-16 units, 3 UTF-8 bytes) and an emoji (1 scalar, 2 units, 4 bytes).
    static let sample = "\\section{Intro}\nA nai\u{308}ve 🎉 test\nx^2\n"

    func testLinesCarryBothOffsets() {
        let m = AccessibleEditorModel(text: Self.sample)
        XCTAssertEqual(m.lines.count, 4)
        guard m.lines.count == 4 else { return XCTFail("expected four lines, got \(m.lines.count)") }
        XCTAssertEqual(m.lines.map(\.number), [1, 2, 3, 4])
        XCTAssertEqual(m.lines[0].text, "\\section{Intro}")
        XCTAssertEqual(m.lines[0].utf8, 0..<15)
        XCTAssertEqual(m.lines[0].utf16, NSRange(location: 0, length: 15))
        // Line 2: "A nai\u{308}ve 🎉 test" = 14 Characters, 15 scalars, 16 UTF-16 units, 19 UTF-8 bytes.
        XCTAssertEqual(m.lines[1].text.count, 14)
        XCTAssertEqual(m.lines[1].utf8, 16..<35)
        XCTAssertEqual(m.lines[1].utf16, NSRange(location: 16, length: 16))
        XCTAssertEqual(m.lines[2].utf16, NSRange(location: 33, length: 3))
        XCTAssertEqual(m.lines[2].utf8, 36..<39)
        XCTAssertEqual(m.lines[3].text, "")
        XCTAssertEqual(m.lines[3].utf8, Self.sample.utf8.count..<Self.sample.utf8.count)
        XCTAssertEqual(AccessibleEditorModel(text: "").lines.map(\.text), [""])
        XCTAssertEqual(AccessibleEditorModel(text: "a\nb").lines.map(\.text), ["a", "b"])
    }

    func testPositionConversionsRejectMidScalarAndMidSurrogate() {
        let m = AccessibleEditorModel(text: Self.sample)
        // Start of the emoji: UTF-16 16+9 = 25, UTF-8 16+10 = 26.
        XCTAssertEqual(m.position(utf16: 25), .init(utf16: 25, utf8: 26))
        XCTAssertEqual(m.position(utf8: 26), .init(utf16: 25, utf8: 26))
        XCTAssertNil(m.position(utf16: 26), "inside the surrogate pair")
        XCTAssertNil(m.position(utf8: 27), "inside the 4-byte scalar")
        XCTAssertNil(m.position(utf8: 22), "inside the 2-byte combining mark (bytes 21..<23)")
        XCTAssertEqual(m.position(utf8: 21), .init(utf16: 21, utf8: 21), "between i and U+0308 is a scalar boundary")
        XCTAssertEqual(m.position(utf16: 27), .init(utf16: 27, utf8: 30))
        XCTAssertNil(m.position(utf16: -1))
        XCTAssertNil(m.position(utf16: Self.sample.utf16.count + 1))
        XCTAssertEqual(m.position(utf16: Self.sample.utf16.count)?.utf8, Self.sample.utf8.count)
    }

    func testLineDescriptionsAndLineMovement() {
        let mark = AccessibleEditorModel.Mark(nsRange: NSRange(location: 33, length: 3), severity: .error,
                                               message: "math mode is not implemented", recovery: "rendered as text")
        let m = AccessibleEditorModel(text: Self.sample, marks: [mark])
        XCTAssertEqual(m.lineDescription(atUTF16: 0), "Line 1 of 4, column 1: \\section{Intro}")
        XCTAssertEqual(m.lineDescription(atUTF16: 15), "Line 1 of 4, column 16: \\section{Intro}", "caret before the newline is still line 1")
        XCTAssertEqual(m.lineDescription(atUTF16: 27), "Line 2 of 4, column 10: A nai\u{308}ve 🎉 test", "column counts ï and the emoji as one character each")
        XCTAssertEqual(m.lineDescription(atUTF16: 34), "Line 3 of 4, column 2: x^2 — 1 diagnostic on this line")
        XCTAssertEqual(m.lineDescription(atUTF16: 37), "Line 4 of 4, column 1: empty")
        XCTAssertEqual(m.lineDescription(atUTF16: 99), "Caret position 99 is not valid in main.tex.")
        XCTAssertEqual(m.lineDescription(atUTF16: 26), "Caret position 26 is not valid in main.tex.", "mid-surrogate")
        XCTAssertEqual(m.lineStart(fromUTF16: 27, delta: 1), .init(utf16: 33, utf8: 36))
        XCTAssertEqual(m.lineStart(fromUTF16: 27, delta: -1), .init(utf16: 0, utf8: 0))
        XCTAssertEqual(m.lineStart(fromUTF16: 27, delta: -5), .init(utf16: 0, utf8: 0), "clamped")
        XCTAssertEqual(m.lineStart(fromUTF16: 0, delta: 9), .init(utf16: 37, utf8: 40), "clamped to the last (empty) line")
    }

    func testWordTokensAndNavigation() {
        let m = AccessibleEditorModel(text: Self.sample)
        let line2 = m.tokens(in: m.lines[1]).filter { $0.kind != .whitespace }
        XCTAssertEqual(line2.map(\.text), ["A", "nai\u{308}ve", "🎉", "test"])
        XCTAssertEqual(line2.map(\.kind), [.word, .word, .symbol, .word])
        guard line2.count == 4 else { return XCTFail("expected four tokens, got \(line2.count)") }
        XCTAssertEqual(line2[1].utf16, NSRange(location: 18, length: 6))
        XCTAssertEqual(line2[1].utf8, 18..<25)
        XCTAssertEqual(line2[2].utf16, NSRange(location: 25, length: 2))
        XCTAssertEqual(line2[2].utf8, 26..<30)
        let line1 = m.tokens(in: m.lines[0])
        XCTAssertEqual(line1.map(\.text), ["\\section", "{", "Intro", "}"])
        XCTAssertEqual(line1.map(\.kind), [.command, .punctuation, .word, .punctuation])
        let line3 = m.tokens(in: m.lines[2])
        XCTAssertEqual(line3.map(\.kind), [.word, .symbol, .word])

        XCTAssertEqual(m.wordDescription(atUTF16: 20), "word “nai\u{308}ve”, 5 characters")
        XCTAssertEqual(m.wordDescription(atUTF16: 24), "word “nai\u{308}ve”, 5 characters", "caret at the end of a word still names it")
        XCTAssertEqual(m.wordDescription(atUTF16: 25), "symbol 🎉, PARTY POPPER")
        XCTAssertEqual(m.wordDescription(atUTF16: 3), "command \\section")
        XCTAssertEqual(m.wordDescription(atUTF16: 8), "punctuation {, LEFT CURLY BRACKET")
        XCTAssertEqual(m.wordDescription(atUTF16: 34), "symbol ^, CIRCUMFLEX ACCENT")
        XCTAssertEqual(m.wordDescription(atUTF16: 37), "No word at the caret.")

        // Forward: from inside \section, skip the brace to "Intro", then "A", "naïve", emoji, "test", "x", "^", "2", end.
        var hops: [Int] = []
        var pos = 0
        while let next = m.nextWordStart(fromUTF16: pos) { hops.append(next.utf16); pos = next.utf16 }
        XCTAssertEqual(hops, [9, 16, 18, 25, 28, 33, 34, 35])
        XCTAssertEqual(m.nextWordStart(fromUTF16: 18), .init(utf16: 25, utf8: 26))
        XCTAssertEqual(m.previousWordStart(fromUTF16: 27), .init(utf16: 25, utf8: 26), "inside the emoji's following space → the emoji")
        XCTAssertEqual(m.previousWordStart(fromUTF16: 25), .init(utf16: 18, utf8: 18))
        XCTAssertEqual(m.previousWordStart(fromUTF16: 20), .init(utf16: 18, utf8: 18), "inside a word → its start")
        XCTAssertNil(m.previousWordStart(fromUTF16: 0))
        XCTAssertNil(m.nextWordStart(fromUTF16: 26), "invalid caret")
    }

    func testCharacterNavigationAndDescriptions() {
        let m = AccessibleEditorModel(text: Self.sample)
        XCTAssertEqual(m.characterDescription(atUTF16: 20),
                       "i\u{308}, LATIN SMALL LETTER I, COMBINING DIAERESIS; 2 scalars, 2 UTF-16 units, 3 UTF-8 bytes")
        XCTAssertEqual(m.characterDescription(atUTF16: 25), "🎉, PARTY POPPER; 1 scalar, 2 UTF-16 units, 4 UTF-8 bytes")
        XCTAssertEqual(m.characterDescription(atUTF16: 17), "space, SPACE; 1 scalar, 1 UTF-16 unit, 1 UTF-8 byte")
        XCTAssertEqual(m.characterDescription(atUTF16: 15), "line break, U+000A; 1 scalar, 1 UTF-16 unit, 1 UTF-8 byte")
        XCTAssertEqual(m.characterDescription(atUTF16: 0), "\\, REVERSE SOLIDUS; 1 scalar, 1 UTF-16 unit, 1 UTF-8 byte")
        XCTAssertEqual(m.characterDescription(atUTF16: Self.sample.utf16.count), "end of document")
        XCTAssertEqual(m.characterDescription(atUTF16: 26), "Caret position 26 is not valid in main.tex.")
        // Grapheme steps: over the combining sequence (+2 units/+3 bytes) and the emoji (+2/+4).
        XCTAssertEqual(m.nextCharacter(fromUTF16: 20), .init(utf16: 22, utf8: 23))
        XCTAssertEqual(m.previousCharacter(fromUTF16: 22), .init(utf16: 20, utf8: 20))
        XCTAssertEqual(m.nextCharacter(fromUTF16: 25), .init(utf16: 27, utf8: 30))
        XCTAssertEqual(m.previousCharacter(fromUTF16: 27), .init(utf16: 25, utf8: 26))
        XCTAssertNil(m.nextCharacter(fromUTF16: Self.sample.utf16.count))
        XCTAssertNil(m.previousCharacter(fromUTF16: 0))
    }

    func testDiagnosticsAtCaret() {
        let marks = [
            AccessibleEditorModel.Mark(nsRange: NSRange(location: 33, length: 3), severity: .error,
                                       message: "math mode is not implemented", recovery: "rendered as text"),
            AccessibleEditorModel.Mark(nsRange: NSRange(location: 0, length: 8), severity: .warning,
                                       message: "heading weight not reproducible", recovery: nil),
        ]
        let m = AccessibleEditorModel(text: Self.sample, marks: marks)
        XCTAssertEqual(m.diagnosticDescription(atUTF16: 35), "Error: math mode is not implemented — recovery: rendered as text")
        XCTAssertEqual(m.diagnosticDescription(atUTF16: 36), "Error: math mode is not implemented — recovery: rendered as text", "caret at the mark's end")
        XCTAssertEqual(m.diagnosticDescription(atUTF16: 8), "Warning: heading weight not reproducible")
        XCTAssertNil(m.diagnosticDescription(atUTF16: 20))
        XCTAssertEqual(m.diagnostics(atUTF16: 0).map(\.message), ["heading weight not reproducible"])
    }

    func testRotorCategories() {
        let text = """
        \\documentclass{article}
        \\begin{document}
        \\section{Introduction}
        \\begin{itemize}
        \\begin{itemize}
        \\end{itemize}
        \\end{itemize}
        \\subsection*{Détail}
        \\begin{equation}
        x^2
        \\end{align}
        \\end{document}
        """
        let byte = Array(text.utf8).count - "\\end{document}".utf8.count // start of \end{document}
        let mark = AccessibleEditorModel.Mark(nsRange: NSRange(location: (text as NSString).range(of: "x^2").location, length: 3),
                                              severity: .error, message: "math mode is not implemented", recovery: "rendered as text")
        let m = AccessibleEditorModel(text: text, marks: [mark], anchor: .init(id: "a1", byteOffset: byte))

        XCTAssertEqual(m.rotorItems(.headings).map(\.label), ["Section “Introduction”, level 1", "Subsection “Détail”, level 2"])
        XCTAssertEqual(m.rotorItems(.headings).map(\.line), [3, 8])
        guard m.rotorItems(.headings).count == 2 else { return XCTFail("expected two heading rotor items, got \(m.rotorItems(.headings).count)") }
        let heading2 = m.rotorItems(.headings)[1]
        XCTAssertEqual(String(text[Range(heading2.utf16, in: text)!]), "\\subsection*{Détail}")
        XCTAssertEqual(heading2.utf8.count, "\\subsection*{Détail}".utf8.count)

        XCTAssertEqual(m.rotorItems(.environments).map(\.label), [
            "Environment document, lines 2 to 12",
            "Environment itemize, lines 4 to 7",
            "Environment itemize, lines 5 to 6",
            "Environment equation, unclosed",
            "Stray \\end{align} with no \\begin",
        ])
        XCTAssertEqual(m.rotorItems(.diagnostics).map(\.label), ["Error at line 10: math mode is not implemented — recovery: rendered as text"])
        XCTAssertEqual(m.rotorItems(.captures).map(\.label), ["Insertion point a1, line 12, column 1, byte \(byte)"])
        XCTAssertEqual(AccessibleEditorModel(text: text).rotorItems(.captures), [])
        XCTAssertEqual(AccessibleEditorModel.RotorCategory.allCases.map(\.title), ["Headings", "Environments", "Diagnostics", "Captures"])

        // Cycling wraps in both directions.
        let first = m.rotorItems(.headings)[0], second = m.rotorItems(.headings)[1]
        XCTAssertEqual(m.nextRotorItem(.headings, fromUTF16: 0, forward: true), first)
        XCTAssertEqual(m.nextRotorItem(.headings, fromUTF16: first.utf16.location, forward: true), second)
        XCTAssertEqual(m.nextRotorItem(.headings, fromUTF16: second.utf16.location, forward: true), first, "wraps")
        XCTAssertEqual(m.nextRotorItem(.headings, fromUTF16: 0, forward: false), second, "wraps backwards")
        XCTAssertNil(AccessibleEditorModel(text: text).nextRotorItem(.captures, fromUTF16: 0, forward: true))
        XCTAssertNotNil(m.nextRotorItem(.captures, fromUTF16: 0, forward: true))
    }

    func testDeterministic() {
        let a = AccessibleEditorModel(text: Self.sample), b = AccessibleEditorModel(text: Self.sample)
        XCTAssertEqual(a, b)
        XCTAssertEqual(a.tokens, b.tokens)
        XCTAssertEqual(a.rotorItems(.environments), b.rotorItems(.environments))
    }
}
