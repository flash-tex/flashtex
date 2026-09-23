import Foundation
import FlashTeXProtocol

// Shared by the Mac editor (`SourceEditorView.BraceMatcher` is a typealias)
// and the iPad editor; platform-free.

// MARK: brace matching (pure)

/// Finds the partner of the `{}`, `[]` or `$…$` delimiter next to the
/// caret. Works on the UTF-8 bytes of the (native) text, line by line:
/// an escaping backslash hides the next byte (`\{`, `\$`, `\\`), an
/// unescaped `%` hides the rest of the line, and a `\verb<d>…<d>` argument
/// is skipped. `$$` is one token that pairs only with `$$`; a `$` is an
/// opener when an even number of `$` tokens precede it in its paragraph
/// (back to the last blank line) and a closer otherwise; inline math never
/// crosses a blank line. Brackets are matched by kind with depth counting,
/// within `budgetBytes` of the anchor in the search direction.
public enum BraceMatcher {
    struct Token: Equatable { var kind: UInt8; var byte: Int; var length: Int }
    struct Line { var start: Int; var end: Int; var tokens: [Token]; var commentAt: Int?; var verb: [Range<Int>] }
    public struct Match: Equatable {
        public var open: NSRange
        public var close: NSRange
        public init(open: NSRange, close: NSRange) { self.open = open; self.close = close }
    }

    static let budgetBytes = 32_768

    /// Delimiters and their partners.
    public static func closer(for opener: Character) -> Character? {
        switch opener { case "{": return "}"; case "[": return "]"; case "(": return ")"; case "$": return "$"; default: return nil }
    }
    /// `)` and `\` are closers only as the halves of an auto-inserted `\)`/`\]`
    /// (type-over checks the pending-closer list before the character).
    public static func isCloser(_ c: Character) -> Bool { c == "}" || c == "]" || c == "$" || c == ")" || c == "\\" }
    /// A unit of an auto-inserted closer that can never be typed over on its
    /// own: `\` (it starts the next command) and the letters of `\right`.
    /// Only a closer's terminal unit (`)`, `]`, `}`, `|`, `.`, `$`) steps over it.
    public static func isClosingUnitOpener(_ c: Character) -> Bool { c == "\\" || c.isLetter }

    /// The math closer for a `(` or `[` just typed before `caretUTF16`
    /// right after a single backslash (`\(` → `\)`, `\[` → `\]`), when
    /// that opener is code and followed by nothing or whitespace; nil otherwise.
    public static func mathCloser(in text: String, caretUTF16: Int) -> String? {
        guard caretUTF16 >= 2, let index = scalarIndex(text: text, utf16: caretUTF16) else { return nil }
        let p = text.utf8.distance(from: text.utf8.startIndex, to: index)
        var copy = text
        return copy.withUTF8 { b -> String? in
            let opener = p - 1
            guard opener >= 1, b[opener - 1] == UInt8(ascii: "\\"), !escaped(b, at: opener - 1), isCode(b, at: opener - 1) else { return nil }
            let closer: String
            switch b[opener] { case UInt8(ascii: "("): closer = "\\)"; case UInt8(ascii: "["): closer = "\\]"; default: return nil }
            if p < b.count {
                let next = b[p]
                guard next == 0x20 || next == 0x09 || next == 0x0A || next == 0x0D else { return nil }
            }
            return closer
        }
    }

    /// The `\right…` partner for the delimiter just typed before
    /// `caretUTF16` when it completes `\left(`, `\left[`, `\left\{`,
    /// `\left|` or `\left.` (the `\left` unescaped and code, the
    /// delimiter followed by nothing, whitespace or a closing
    /// delimiter); nil otherwise. Math-mode gating is the caller's
    /// (`\left` is a math-only command; in text it is a plain error).
    public static func leftRightCloser(in text: String, caretUTF16: Int) -> String? {
        guard caretUTF16 >= 6, let index = scalarIndex(text: text, utf16: caretUTF16) else { return nil }
        let p = text.utf8.distance(from: text.utf8.startIndex, to: index)
        var copy = text
        return copy.withUTF8 { b -> String? in
            let opener = p - 1
            guard opener >= 5 else { return nil }
            let closer: String
            let leftEnd: Int // the byte after `\left`
            switch b[opener] {
            case UInt8(ascii: "("): closer = "\\right)"; leftEnd = opener
            case UInt8(ascii: "["): closer = "\\right]"; leftEnd = opener
            case UInt8(ascii: "|"): closer = "\\right|"; leftEnd = opener
            case UInt8(ascii: "."): closer = "\\right."; leftEnd = opener
            case UInt8(ascii: "{"):
                guard b[opener - 1] == UInt8(ascii: "\\") else { return nil }
                closer = "\\right\\}"; leftEnd = opener - 1
            default: return nil
            }
            let start = leftEnd - 5
            guard start >= 0, b[start] == UInt8(ascii: "\\"), !escaped(b, at: start), isCode(b, at: start),
                  b[start + 1] == UInt8(ascii: "l"), b[start + 2] == UInt8(ascii: "e"),
                  b[start + 3] == UInt8(ascii: "f"), b[start + 4] == UInt8(ascii: "t") else { return nil }
            if p < b.count {
                let next = b[p]
                let allowedNext = next == 0x20 || next == 0x09 || next == 0x0A || next == 0x0D
                    || next == UInt8(ascii: "}") || next == UInt8(ascii: "]") || next == UInt8(ascii: ")") || next == UInt8(ascii: "$")
                guard allowedNext else { return nil }
            }
            return closer
        }
    }

    public static func match(in text: String, caretUTF16: Int) -> Match? {
        guard let index = scalarIndex(text: text, utf16: caretUTF16) else { return nil }
        let p = text.utf8.distance(from: text.utf8.startIndex, to: index)
        var copy = text
        let bytes: (open: Range<Int>, close: Range<Int>)? = copy.withUTF8 { b in match(bytes: b, caret: p) }
        guard let bytes,
              let open = text.nsRange(utf8Bytes: .init(path: "", startByte: bytes.open.lowerBound, endByte: bytes.open.upperBound)),
              let close = text.nsRange(utf8Bytes: .init(path: "", startByte: bytes.close.lowerBound, endByte: bytes.close.upperBound))
        else { return nil }
        return Match(open: open, close: close)
    }

    /// Whether the (ASCII) delimiter just typed before `caretUTF16` is code
    /// (not escaped, not in a comment or `\verb`) and is followed by nothing,
    /// whitespace or a closing delimiter — the cases where auto-closing helps.
    public static func autoCloseAllowed(in text: String, caretUTF16: Int) -> Bool {
        guard caretUTF16 >= 1, let index = scalarIndex(text: text, utf16: caretUTF16) else { return false }
        let p = text.utf8.distance(from: text.utf8.startIndex, to: index)
        var copy = text
        return copy.withUTF8 { b in
            let opener = p - 1
            guard opener >= 0, isCode(b, at: opener), !escaped(b, at: opener) else { return false }
            if p < b.count {
                let next = b[p]
                let allowedNext = next == 0x20 || next == 0x09 || next == 0x0A || next == 0x0D
                    || next == UInt8(ascii: "}") || next == UInt8(ascii: "]") || next == UInt8(ascii: ")") || next == UInt8(ascii: "$")
                guard allowedNext else { return false }
            }
            // `$` only opens new inline math: an odd count of single `$`
            // tokens earlier in the paragraph means this one closes math
            // already open there, so it must not get a second `$` paired
            // onto it (GH74: "$ not inside math already opened by $").
            if b[opener] == UInt8(ascii: "$") {
                return dollarsBefore(b, beforeByte: opener, length: 1) % 2 == 0
            }
            return true
        }
    }

    // MARK: byte-level scanning

    static func isCode(_ b: UnsafeBufferPointer<UInt8>, at p: Int) -> Bool {
        let line = parse(b, containing: p)
        if let c = line.commentAt, p > c { return false }
        return !line.verb.contains { $0.contains(p) }
    }

    /// An odd run of backslashes ends right before `p`.
    static func escaped(_ b: UnsafeBufferPointer<UInt8>, at p: Int) -> Bool {
        var n = 0
        var i = p - 1
        while i >= 0, b[i] == UInt8(ascii: "\\") { n += 1; i -= 1 }
        return n % 2 == 1
    }

    static func lineBounds(_ b: UnsafeBufferPointer<UInt8>, containing p: Int) -> (start: Int, end: Int) {
        var start = min(p, b.count)
        while start > 0, b[start - 1] != 0x0A { start -= 1 }
        var end = min(p, b.count)
        while end < b.count, b[end] != 0x0A { end += 1 }
        return (start, end)
    }

    static func parse(_ b: UnsafeBufferPointer<UInt8>, containing p: Int) -> Line {
        let (start, end) = lineBounds(b, containing: p)
        return parse(b, start: start, end: end)
    }

    static func parse(_ b: UnsafeBufferPointer<UInt8>, start: Int, end: Int) -> Line {
        var line = Line(start: start, end: end, tokens: [], commentAt: nil, verb: [])
        var i = start
        while i < end {
            let c = b[i]
            if c == UInt8(ascii: "\\") {
                // \verb<d>…<d> (also \verb*): the argument is not code.
                if i + 5 < end, b[i + 1] == UInt8(ascii: "v"), b[i + 2] == UInt8(ascii: "e"), b[i + 3] == UInt8(ascii: "r"), b[i + 4] == UInt8(ascii: "b") {
                    var j = i + 5
                    if j < end, b[j] == UInt8(ascii: "*") { j += 1 }
                    if j < end, !isLetter(b[j]) {
                        let d = b[j]
                        var k = j + 1
                        while k < end, b[k] != d { k += 1 }
                        line.verb.append(i..<min(k + 1, end))
                        i = min(k + 1, end)
                        continue
                    }
                }
                i += 2 // the escaped byte is literal (\{ \} \$ \% \\)
                continue
            }
            if c == UInt8(ascii: "%") { line.commentAt = i; break }
            if c == UInt8(ascii: "$") {
                if i + 1 < end, b[i + 1] == UInt8(ascii: "$") { line.tokens.append(Token(kind: c, byte: i, length: 2)); i += 2 }
                else { line.tokens.append(Token(kind: c, byte: i, length: 1)); i += 1 }
                continue
            }
            if c == UInt8(ascii: "{") || c == UInt8(ascii: "}") || c == UInt8(ascii: "[") || c == UInt8(ascii: "]") {
                line.tokens.append(Token(kind: c, byte: i, length: 1))
            }
            i += 1
        }
        return line
    }

    static func isLetter(_ c: UInt8) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
    static func isBlank(_ b: UnsafeBufferPointer<UInt8>, _ line: Line) -> Bool {
        var i = line.start
        while i < line.end { if b[i] != 0x20 && b[i] != 0x09 && b[i] != 0x0D { return false }; i += 1 }
        return true
    }

    static func match(bytes b: UnsafeBufferPointer<UInt8>, caret p: Int) -> (open: Range<Int>, close: Range<Int>)? {
        let line = parse(b, containing: p)
        if let c = line.commentAt, p > c { return nil }
        if line.verb.contains(where: { $0.contains(p) }) { return nil }
        guard let anchorIndex = line.tokens.firstIndex(where: { $0.byte + $0.length == p })
                ?? line.tokens.firstIndex(where: { $0.byte == p }) else { return nil }
        let anchor = line.tokens[anchorIndex]
        let range = anchor.byte..<(anchor.byte + anchor.length)
        switch anchor.kind {
        case UInt8(ascii: "{"), UInt8(ascii: "["):
            let close = anchor.kind == UInt8(ascii: "{") ? UInt8(ascii: "}") : UInt8(ascii: "]")
            return search(b, from: line, tokenIndex: anchorIndex, forward: true, open: anchor.kind, close: close, math: nil)
                .map { (range, $0) }
        case UInt8(ascii: "}"), UInt8(ascii: "]"):
            let open = anchor.kind == UInt8(ascii: "}") ? UInt8(ascii: "{") : UInt8(ascii: "[")
            return search(b, from: line, tokenIndex: anchorIndex, forward: false, open: open, close: anchor.kind, math: nil)
                .map { ($0, range) }
        default: // $ or $$
            let forward = dollarsBefore(b, line: line, tokenIndex: anchorIndex, length: anchor.length) % 2 == 0
            guard let partner = search(b, from: line, tokenIndex: anchorIndex, forward: forward, open: anchor.kind, close: anchor.kind, math: anchor.length) else { return nil }
            return forward ? (range, partner) : (partner, range)
        }
    }

    /// `$` tokens of `length` before the anchor in its paragraph.
    static func dollarsBefore(_ b: UnsafeBufferPointer<UInt8>, line: Line, tokenIndex: Int, length: Int) -> Int {
        var count = line.tokens[..<tokenIndex].filter { $0.kind == UInt8(ascii: "$") && $0.length == length }.count
        var current = line
        var scanned = 0
        while current.start > 0, scanned < budgetBytes {
            let previous = parse(b, containing: current.start - 1)
            if isBlank(b, previous) { break }
            count += previous.tokens.filter { $0.kind == UInt8(ascii: "$") && $0.length == length }.count
            scanned += current.start - previous.start
            current = previous
        }
        return count
    }

    /// Single (`length` 1) `$` tokens strictly before byte offset `p` in
    /// `p`'s paragraph (back to the last blank line), bounded like the
    /// token-index overload above. Even means a `$` typed at `p` would
    /// open new inline math; odd means it closes math already open
    /// earlier in the paragraph (`autoCloseAllowed`'s `$` gate).
    static func dollarsBefore(_ b: UnsafeBufferPointer<UInt8>, beforeByte p: Int, length: Int) -> Int {
        let line = parse(b, containing: p)
        var count = line.tokens.filter { $0.kind == UInt8(ascii: "$") && $0.length == length && $0.byte < p }.count
        var current = line
        var scanned = 0
        while current.start > 0, scanned < budgetBytes {
            let previous = parse(b, containing: current.start - 1)
            if isBlank(b, previous) { break }
            count += previous.tokens.filter { $0.kind == UInt8(ascii: "$") && $0.length == length }.count
            scanned += current.start - previous.start
            current = previous
        }
        return count
    }

    /// Depth-counted search for the partner token; `math` is the `$` token
    /// length to pair (no nesting, stops at a blank line).
    static func search(_ b: UnsafeBufferPointer<UInt8>, from line: Line, tokenIndex: Int, forward: Bool,
                       open: UInt8, close: UInt8, math: Int?) -> Range<Int>? {
        var depth = 1
        var current = line
        var tokens = forward ? Array(line.tokens[(tokenIndex + 1)...]) : Array(line.tokens[..<tokenIndex].reversed())
        var scanned = 0
        while true {
            for t in tokens {
                if let math {
                    if t.kind == UInt8(ascii: "$"), t.length == math { return t.byte..<(t.byte + t.length) }
                    continue
                }
                if t.kind == (forward ? open : close) { depth += 1 }
                else if t.kind == (forward ? close : open) {
                    depth -= 1
                    if depth == 0 { return t.byte..<(t.byte + t.length) }
                }
            }
            if forward {
                guard current.end < b.count, scanned < budgetBytes else { return nil }
                let next = parse(b, containing: current.end + 1)
                scanned += next.end - current.end
                current = next
            } else {
                guard current.start > 0, scanned < budgetBytes else { return nil }
                let previous = parse(b, containing: current.start - 1)
                scanned += current.start - previous.start
                current = previous
            }
            if math != nil, isBlank(b, current) { return nil }
            tokens = forward ? current.tokens : current.tokens.reversed()
        }
    }

    /// `String.Index` of a UTF-16 offset when it sits on a scalar boundary
    /// (the same check as the Mac editor's `SourceEditorView.scalarIndex`).
    public static func scalarIndex(text: String, utf16: Int) -> String.Index? {
        guard utf16 >= 0, utf16 <= text.utf16.count,
              let index = text.utf16.index(text.utf16.startIndex, offsetBy: utf16, limitedBy: text.utf16.endIndex),
              index.samePosition(in: text.unicodeScalars) != nil else { return nil }
        return index
    }
}
