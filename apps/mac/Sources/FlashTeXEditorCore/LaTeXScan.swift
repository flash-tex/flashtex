import Foundation

/// The lexical scan both editors share, moved here from the Mac's
/// `EditorNavigation` (which forwards to it) so the iPad's linked
/// `\begin`/`\end` name editing (`LinkedEnvironmentEditing`) uses the same
/// rules. (TeXpand's branch moves the same scan here with the pair helpers;
/// the lexer below is identical to that version.)
///
/// The rule set: a `%` that is not `\%` comments the rest of its line,
/// `\begin{verbatim}`-like environments (`SyntaxHighlighter.verbatimEnvironments`)
/// and `\verb<d>…<d>` are skipped whole, and `\\` / `\{` / `\%` are escapes.
/// Nothing here parses TeX. All offsets are UTF-16.
public enum LaTeXScan {
    // MARK: lexical scan

    /// One control sequence with its complete braced argument, in document
    /// order: `\name{arg}` (`range` spans the backslash through `}`), or a
    /// bare `\name` (`argRange` nil) for every other command.
    public struct Use: Equatable, Sendable {
        public var name: String
        /// The whole `\name` (or `\name{arg}`) span.
        public var range: NSRange
        /// The text between the braces, when the command has a braced argument right after its name.
        public var arg: String?
        public var argRange: NSRange?
        public init(name: String, range: NSRange, arg: String?, argRange: NSRange?) {
            self.name = name; self.range = range; self.arg = arg; self.argRange = argRange
        }
    }

    /// Every command of `text` outside comments and verbatim, with its
    /// immediate braced argument when one follows (whitespace allowed
    /// between name and `{` only for `\begin`/`\end`, which LaTeX permits).
    public static func uses(in text: NSString) -> [Use] {
        var out: [Use] = []
        let n = text.length
        var i = 0
        var verbatimUntil: String? // `\end{name}` that closes the skipped block
        func isLetter(_ c: unichar) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
        while i < n {
            if let closer = verbatimUntil {
                // Skip to the matching `\end{name}`; the `\end` itself is emitted.
                let r = text.range(of: closer, options: .literal, range: NSRange(location: i, length: n - i))
                guard r.location != NSNotFound else { break }
                i = r.location
                verbatimUntil = nil
                // fall through to lex the `\end`
            }
            let c = text.character(at: i)
            if c == 0x25 { // '%'
                while i < n, text.character(at: i) != 0x0A { i += 1 }
                continue
            }
            guard c == 0x5C else { i += 1; continue } // '\'
            var j = i + 1
            while j < n, isLetter(text.character(at: j)) { j += 1 }
            if j == i + 1 { // control symbol: `\\`, `\%`, `\{`
                i = min(n, i + 2)
                continue
            }
            if j < n, text.character(at: j) == 0x2A { j += 1 } // `\newcommand*`, `\section*`
            let name = text.substring(with: NSRange(location: i + 1, length: j - i - 1))
            if name == "verb" || name == "verb*", j < n { // `\verb<d>…<d>` to the delimiter or line end
                let d = text.character(at: j)
                var k = j + 1
                while k < n, text.character(at: k) != d, text.character(at: k) != 0x0A { k += 1 }
                out.append(Use(name: name, range: NSRange(location: i, length: min(n, k + 1) - i), arg: nil, argRange: nil))
                i = min(n, k + 1)
                continue
            }
            var k = j
            if name == "begin" || name == "end" { while k < n, text.character(at: k) == 0x20 { k += 1 } }
            if k < n, text.character(at: k) == 0x7B { // '{'
                var m = k + 1
                var depth = 1
                while m < n {
                    let d = text.character(at: m)
                    if d == 0x5C { m += 2; continue }
                    if d == 0x7B { depth += 1 } else if d == 0x7D { depth -= 1; if depth == 0 { break } }
                    if d == 0x0A, depth > 0, name == "begin" || name == "end" || name == "label" { break } // an env/label name never spans lines
                    m += 1
                }
                if m < n, text.character(at: m) == 0x7D {
                    let arg = text.substring(with: NSRange(location: k + 1, length: m - k - 1))
                    out.append(Use(name: name, range: NSRange(location: i, length: m + 1 - i), arg: arg, argRange: NSRange(location: k + 1, length: m - k - 1)))
                    if name == "begin", SyntaxHighlighter.verbatimEnvironments.contains(arg) || arg == "comment" { verbatimUntil = "\\end{\(arg)}" }
                    i = k + 1 // keep lexing inside the argument: `\newcommand{\foo}` also yields the `\foo` use
                    continue
                }
            }
            out.append(Use(name: name, range: NSRange(location: i, length: j - i), arg: nil, argRange: nil))
            i = j
        }
        return out
    }
}
