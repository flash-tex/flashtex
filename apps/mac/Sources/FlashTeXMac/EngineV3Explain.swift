import Foundation
import FlashTeXProtocol

/// Plain-language explanations and mechanical quick fixes for TeX's
/// diagnostics under the new engine (lane ERROR-RECOVERY gap 5; DESIGN §10
/// app parity B9/B10). The old engine's messages were sentences ("inline
/// math is missing its closing '$'", "unmatched '}' — no group is open
/// here", "\end{enumerate} does not match \begin{itemize}"); TeX's are
/// terse ("Missing $ inserted."). Each row keeps TeX's message (it is what
/// pdfLaTeX prints) and gains the explanation as its first note, keyed by
/// the stable diag-v1 code; TeX's own help text stays a note.
///
/// A fix (`help.replacement`, the Problems panel's "Fix…" and Tab at the
/// caret) is attached only where it is mechanical and checked against the
/// text the compile read: the row's range must be exactly the text the fix
/// replaces. Pure.
enum EngineV3Explain {
    /// The row with its explanation and, where safe, its fix. `message` is
    /// TeX's message (not the row's, which may carry the recovered mark).
    static func apply(_ row: RuntimeV1.Diagnostic, code: String, message: String, texts: [String: String]?) -> RuntimeV1.Diagnostic {
        guard let text = explanation(code: code, message: message) else { return row }
        var d = row
        if code == "latex/file-not-found", let at = missingFileSource(row, message: message, texts: texts) { d.source = at }
        d.notes = [text] + (d.notes ?? [])
        if let fix = fix(d, code: code, message: message, texts: texts) {
            if let tex = d.help?.message, !tex.isEmpty { d.notes = (d.notes ?? []) + [tex] }
            d.help = fix
        }
        return d
    }

    /// `\X` of a message, or "a command" for LaTeX's internal names (`\@xdblarg`, `\text@command`).
    static func command(in message: String, after prefix: String) -> String {
        guard let r = message.range(of: prefix) else { return "a command" }
        let name = message[r.upperBound...].prefix { !$0.isWhitespace && $0 != "." }
        guard name.hasPrefix("\\"), name.count > 1, !name.contains("@") else { return "a command" }
        return String(name)
    }

    static func explanation(code: String, message: String) -> String? {
        switch code {
        case "tex/undefined-control-sequence":
            return "TeX does not know this command: a typo, or the package that defines it is not loaded."
        case "tex/missing-dollar":
            return "Math-only syntax (^, _ or a math symbol) is used outside math, or a $ is not closed. TeX added a $ to go on."
        case "tex/too-many-right-braces":
            return "This } closes no group: there is no { before it that is still open."
        case "tex/misplaced-alignment-tab":
            return "& separates the columns of a table. To print an ampersand in text, write \\&."
        case "tex/file-ended-while-scanning":
            let cs = command(in: message, after: "use of ")
            return "The argument of \(cs) opens with { but is never closed, so TeX read to the end of the file and stopped. Close the brace."
        case "tex/paragraph-ended-before-argument-complete":
            let cs = command(in: message, after: "before ")
            return "A blank line (a paragraph end) came inside the argument of \(cs) before its closing }. Close the brace before the blank line."
        case "tex/display-math-should-end-with-dollars":
            return "A display (\\[ … \\] or $$ … $$) is not closed before the paragraph ends."
        case "tex/emergency-stop", "tex/fatal-error-no-output":
            return "pdfLaTeX stops here and writes no PDF. The pages after this point are those of the last good compile, shown stale."
        case "tex/capacity-exceeded":
            return "TeX ran out of one of its fixed limits; this is usually a command that calls itself without end."
        case "latex/environment-mismatch":
            if let (begun, ended) = mismatch(message), ended == "document" {
                return "\\begin{\(begun)} is never closed: \\end{document} came first. Add \\end{\(begun)}."
            }
            return "Environments close in the order they were opened: the \\end here does not match the open \\begin."
        case "latex/environment-undefined":
            return "LaTeX does not know this environment: a typo, or the package that defines it is not loaded."
        case "latex/file-not-found":
            if let name = missingFile(message), name.hasSuffix(".sty") || name.hasSuffix(".cls") {
                return "No \(name.hasSuffix(".sty") ? "package" : "class") named \((name as NSString).deletingPathExtension) is installed or in the project: check its name."
            }
            return "The file is not in the project (or its name or folder is misspelt)."
        case "latex/lonely-item":
            return "\\item is only allowed inside a list: itemize, enumerate or description."
        case "latex/there-no-line-here-to-end":
            return "\\\\ ends a line, but there is no line to end here (at the start of a paragraph, or after a blank line). For vertical space use \\vspace or a blank line."
        default:
            return nil
        }
    }

    /// "\begin{A} on input line N ended by \end{B}." → (A, B).
    static func mismatch(_ message: String) -> (String, String)? {
        func arg(after p: String) -> String? {
            guard let r = message.range(of: p) else { return nil }
            let rest = message[r.upperBound...]
            guard let close = rest.firstIndex(of: "}") else { return nil }
            return String(rest[..<close])
        }
        guard let a = arg(after: "\\begin{"), let b = arg(after: "\\end{") else { return nil }
        return (a, b)
    }

    /// "… on input line 13 ended by …" → 13.
    static func openedLine(_ message: String) -> Int? {
        guard let r = message.range(of: "on input line ") else { return nil }
        return Int(message[r.upperBound...].prefix { $0.isNumber })
    }

    /// The text of 1-based `line`.
    static func lineText(_ text: String, _ line: Int) -> Substring? {
        let lines = text.split(separator: "\n", omittingEmptySubsequences: false)
        return line >= 1 && line <= lines.count ? lines[line - 1] : nil
    }

    /// "File `x.sty' not found." → "x.sty".
    static func missingFile(_ message: String) -> String? {
        guard let a = message.range(of: "File `"), let b = message.range(of: "' not found", range: a.upperBound ..< message.endIndex) else { return nil }
        return String(message[a.upperBound ..< b.lowerBound])
    }

    /// LaTeX reports a missing package after `\usepackage` has looked
    /// ahead for an optional date, so TeX's place is the next line's
    /// command. The name itself, as a whole argument entry (`{x}`, `{a,x}`),
    /// last before the end of the reported line and at most 400 bytes back,
    /// is where the row belongs.
    static func missingFileSource(_ row: RuntimeV1.Diagnostic, message: String, texts: [String: String]?) -> RuntimeV1.SourceRange? {
        guard let src = row.source, let text = texts?[src.path], let file = missingFile(message) else { return nil }
        let stem = file.hasSuffix(".sty") || file.hasSuffix(".cls") || file.hasSuffix(".tex") ? (file as NSString).deletingPathExtension : file
        let bytes = Array(text.utf8)
        guard src.endByte <= bytes.count, !stem.isEmpty else { return nil }
        let lineEnd = bytes[src.endByte...].firstIndex(of: 0x0A) ?? bytes.count
        let from = max(0, src.startByte - 400)
        let want = Array(stem.utf8)
        var i = lineEnd - want.count
        while i >= from {
            if Array(bytes[i ..< i + want.count]) == want {
                let before = i > 0 ? bytes[i - 1] : 0, after = i + want.count < bytes.count ? bytes[i + want.count] : 0
                let opens: Set<UInt8> = [UInt8(ascii: "{"), UInt8(ascii: ",")], closes: Set<UInt8> = [UInt8(ascii: "}"), UInt8(ascii: ","), UInt8(ascii: ".")]
                if opens.contains(before), closes.contains(after) {
                    return .init(path: src.path, startByte: i, endByte: i + want.count)
                }
            }
            i -= 1
        }
        return nil
    }

    static func fix(_ row: RuntimeV1.Diagnostic, code: String, message: String, texts: [String: String]?) -> RuntimeV1.Diagnostic.Help? {
        guard let src = row.source, let text = texts?[src.path], let marked = text.utf8Slice(src.startByte, src.endByte) else { return nil }
        func replace(_ start: Int, _ end: Int, with s: String, _ label: String) -> RuntimeV1.Diagnostic.Help {
            .init(message: label, replacement: .init(startByte: start, endByte: end, text: s, path: src.path))
        }
        switch code {
        case "tex/too-many-right-braces" where marked == "}":
            return replace(src.startByte, src.endByte, with: "", "remove the unmatched }")
        case "tex/misplaced-alignment-tab" where marked == "&":
            return replace(src.startByte, src.endByte, with: "\\&", "write \\& for an ampersand")
        case "latex/environment-mismatch":
            // Only an environment the text opens itself on the line LaTeX names
            // (not `document` closed by an unknown \end, not `\[`'s equation*).
            guard let (begun, ended) = mismatch(message), begun != ended, begun != "document", marked == "\\end{\(ended)}",
                  let line = openedLine(message), lineText(text, line)?.contains("\\begin{\(begun)}") == true else { return nil }
            if ended == "document" {
                return replace(src.startByte, src.startByte, with: "\\end{\(begun)}\n", "add \\end{\(begun)} before \\end{document}")
            }
            return replace(src.startByte, src.endByte, with: "\\end{\(begun)}", "close with \\end{\(begun)}")
        case "latex/file-not-found":
            // the range is the package's name (`missingFileSource`)
            guard let file = missingFile(message), file.hasSuffix(".sty"), marked == (file as NSString).deletingPathExtension else { return nil }
            let close = EngineV3Fixes.closestCommands(marked, vocabulary: Completion.knownPackages)
            guard close.count == 1 else { return nil }
            return replace(src.startByte, src.endByte, with: close[0], "did you mean \(close[0])?")
        default:
            return nil
        }
    }
}
