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
    static func apply(_ row: RuntimeV1.Diagnostic, code: String, message: String, texts: [String: String]?,
                      undefinedName: String? = nil) -> RuntimeV1.Diagnostic {
        var d = row
        // What a known package defines: that is the explanation, loading it the fix.
        if let hint = packageHint(code: code, message: message, name: undefinedName) {
            d.notes = ["\(hint.what) is defined by the \(hint.package) package, which is not loaded."] + (d.notes ?? [])
            if let at = EngineV3DiagPresent.preambleEnd(prefer: row.source?.path, texts: texts ?? [:]) {
                if let tex = d.help?.message, !tex.isEmpty { d.notes = (d.notes ?? []) + [tex] }
                d.help = .init(message: "add \\usepackage{\(hint.package)}",
                               replacement: .init(startByte: at.at, endByte: at.at, text: "\\usepackage{\(hint.package)}" + at.eol, path: at.path))
            }
            return d
        }
        guard let text = explanation(code: code, message: message) else { return row }
        if code == "latex/file-not-found", let at = missingFileSource(row, message: message, texts: texts) { d.source = at }
        if code == "latex/no-file", d.source == nil, let at = includeSource(message, texts: texts) { d.source = at }
        if code == "latex/multiply-defined-label", let texts { relocateDuplicateLabel(&d, message: message, texts: texts) }
        d.notes = [text] + (d.notes ?? [])
        if let fix = fix(d, code: code, message: message, texts: texts) {
            if let tex = d.help?.message, !tex.isEmpty { d.notes = (d.notes ?? []) + [tex] }
            d.help = fix
        }
        return d
    }

    /// Commands and environments of common packages, each defined by that
    /// package alone (a name several packages define is left out).
    static let packageOf: [String: String] = [
        "\\mathbb": "amssymb", "\\mathfrak": "amssymb", "\\checkmark": "amssymb", "\\varnothing": "amssymb",
        "\\eqref": "amsmath", "\\text": "amsmath", "\\DeclareMathOperator": "amsmath", "\\dfrac": "amsmath", "\\tfrac": "amsmath",
        "\\binom": "amsmath", "\\iint": "amsmath", "\\numberwithin": "amsmath", "\\boldsymbol": "amsmath", "\\operatorname": "amsmath",
        "\\includegraphics": "graphicx", "\\rotatebox": "graphicx", "\\scalebox": "graphicx", "\\resizebox": "graphicx",
        "\\textcolor": "xcolor", "\\color": "xcolor", "\\colorbox": "xcolor", "\\definecolor": "xcolor",
        "\\url": "url", "\\href": "hyperref", "\\hypersetup": "hyperref", "\\autoref": "hyperref",
        "\\toprule": "booktabs", "\\midrule": "booktabs", "\\bottomrule": "booktabs", "\\cmidrule": "booktabs",
        "\\SI": "siunitx", "\\si": "siunitx", "\\num": "siunitx", "\\qty": "siunitx", "\\unit": "siunitx",
        "\\cref": "cleveref", "\\Cref": "cleveref", "\\mathscr": "mathrsfs", "\\bm": "bm", "\\lipsum": "lipsum",
        "\\multirow": "multirow", "\\captionof": "caption", "\\tikz": "tikz", "\\usetikzlibrary": "tikz",
        "\\lstinline": "listings", "\\lstset": "listings", "\\FloatBarrier": "placeins", "\\todo": "todonotes", "\\ce": "mhchem",
        "align": "amsmath", "align*": "amsmath", "gather": "amsmath", "gather*": "amsmath", "multline": "amsmath",
        "multline*": "amsmath", "equation*": "amsmath", "split": "amsmath", "aligned": "amsmath", "cases": "amsmath",
        "pmatrix": "amsmath", "bmatrix": "amsmath", "vmatrix": "amsmath", "Bmatrix": "amsmath", "Vmatrix": "amsmath", "matrix": "amsmath",
        "tikzpicture": "tikz", "lstlisting": "listings", "longtable": "longtable", "tabularx": "tabularx",
        "subfigure": "subcaption", "algorithmic": "algpseudocode", "multicols": "multicol", "wrapfigure": "wrapfig",
    ]

    /// The package that defines the row's undefined command (`name`, as TeX
    /// reports it) or environment, when the table knows it.
    static func packageHint(code: String, message: String, name: String?) -> (what: String, package: String)? {
        switch code {
        case "tex/undefined-control-sequence":
            guard let name, let p = packageOf[name] else { return nil }
            return (name, p)
        case "latex/environment-undefined":
            guard let r = message.range(of: "Environment "), let e = message[r.upperBound...].split(separator: " ").first,
                  let p = packageOf[String(e)] else { return nil }
            return ("The \(e) environment", p)
        default:
            return nil
        }
    }

    /// "No file chap9.tex.": the one `\include{chap9}` (or `\input`,
    /// `\InputIfFileExists`) of it in the texts.
    static func includeSource(_ message: String, texts: [String: String]?) -> RuntimeV1.SourceRange? {
        guard let texts, let r = message.range(of: "No file "), message.hasSuffix(".tex.") else { return nil }
        let stem = NSRegularExpression.escapedPattern(for: String(message[r.upperBound...].dropLast(5)))
        let sites = EngineV3DiagPresent.matches(#"\\(?:include|input|InputIfFileExists)\s*\{\s*"# + stem + #"(?:\.tex)?\s*\}"#, in: texts)
        return sites.count == 1 ? sites[0].source : nil
    }

    /// "Label `a' multiply defined." is reported where LaTeX reads the
    /// `.aux`; the row goes to the second `\label{a}` of the texts, and the
    /// first one becomes a secondary label and a note.
    static func relocateDuplicateLabel(_ d: inout RuntimeV1.Diagnostic, message: String, texts: [String: String]) {
        guard let key = quoted(message, after: "Label `") else { return }
        let sites = EngineV3DiagPresent.matches(#"\\label\s*\{"# + NSRegularExpression.escapedPattern(for: key) + #"\}"#, in: texts)
        guard sites.count >= 2 else { return }
        d.source = sites[1].source
        d.labels = [.init(source: sites[1].source, text: "", primary: true),
                    .init(source: sites[0].source, text: "first defined at \(sites[0].label)", primary: false)]
        d.notes = (d.notes ?? []) + ["\\label{\(key)} is also at \(sites[0].label)."]
    }

    /// The byte range of `key` as one whole entry of a braced list in
    /// `text` (`{key}`, `{a, key}`), when it occurs exactly once so.
    static func keyRange(_ key: String, in text: String) -> Range<Int>? {
        let k = Array(key.utf8), b = Array(text.utf8)
        guard !k.isEmpty, b.count >= k.count else { return nil }
        let opens: Set<UInt8> = [UInt8(ascii: "{"), UInt8(ascii: ","), 0x20], closes: Set<UInt8> = [UInt8(ascii: "}"), UInt8(ascii: ","), 0x20]
        var found: [Range<Int>] = []
        for i in 0 ... b.count - k.count where Array(b[i ..< i + k.count]) == k {
            if i > 0, opens.contains(b[i - 1]), i + k.count < b.count, closes.contains(b[i + k.count]) { found.append(i ..< i + k.count) }
        }
        return found.count == 1 ? found[0] : nil
    }

    /// "Reference `a' on page 1 undefined" -> "a" (`prefix`: "Reference `").
    static func quoted(_ message: String, after prefix: String) -> String? {
        guard let a = message.range(of: prefix), let b = message[a.upperBound...].firstIndex(of: "'") else { return nil }
        let key = String(message[a.upperBound ..< b])
        return key.isEmpty ? nil : key
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
        // Lane DIAG-PARITY: the rest of the common errors and warnings.
        case "tex/overfull-hbox":
            return "TeX could not break this line to fit the text width, so it sticks out into the margin. Usually a long word, URL or \\mbox: rephrase, allow a break with \\- or \\allowbreak, or put the paragraph in \\begin{sloppypar}."
        case "tex/underfull-hbox":
            return "TeX had to stretch the spaces of this line more than it likes. Usually \\\\ or \\newline at the end of a paragraph or after another \\\\: use a blank line or \\vspace instead."
        case "tex/overfull-vbox":
            return "The content is taller than the box or page it is set in, so it sticks out at the bottom."
        case "tex/underfull-vbox":
            return "TeX had to stretch the vertical space of this box or page more than it likes (often a page break before a large float or display)."
        case "tex/extra-alignment-tab":
            return "This row has more cells than the table has columns: one & too many, or a column missing from the column specification (\\begin{tabular}{…})."
        case "tex/double-superscript":
            return "x^a^b is ambiguous: write x^{ab} or {x^a}^b."
        case "tex/double-subscript":
            return "x_a_b is ambiguous: write x_{ab} or {x_a}_b."
        case "tex/missing-right-delimiter":
            return "\\left has no matching \\right in the same formula (or the same line of an align). Add one; \\right. is an invisible one."
        case "tex/extra-right-delimiter":
            return "This \\right has no \\left before it in the same formula (or the same line of an align)."
        case "tex/missing-right-brace":
            return "A group or argument opened with { is still open where it has to end (at the end of the formula, cell or paragraph). Close it before this point."
        case "tex/missing-left-brace":
            return "This command needs its argument in braces here, or a } closes a group that was never opened."
        case "tex/illegal-unit":
            return "A length needs a unit: 2pt, 1.5em, 3cm or 0.5\\textwidth, not a bare number."
        case "tex/missing-number":
            return "TeX expected a number or a length here (such as 2 or 3pt) and found something else."
        case "tex/missing-character":
            return "The font has no glyph for this character, so nothing is printed. Use a command for the symbol, or a font that has it."
        case "latex/unicode-not-set-up":
            return "pdfLaTeX has no definition for this character. Write it as a LaTeX command (symbol packages such as amssymb have most), or declare it with \\DeclareUnicodeCharacter."
        case "latex/caption-outside-float":
            return "\\caption works only inside a figure or table environment. Outside one, use \\captionof{figure}{…} from the caption package."
        case "latex/can-be-used-only-in-preamble":
            return "This command must come before \\begin{document}. Move it to the preamble."
        case "latex/command-already-defined":
            return "\\newcommand never replaces an existing command. Choose another name, or use \\renewcommand if you mean to replace it."
        case "latex/missing-item":
            return "Text in a list must follow an \\item, and a list needs at least one. Add \\item before it."
        case "latex/allowed-only-in-math-mode":
            return "This command only works in math: put it inside $…$, or use its text counterpart (\\textbf for \\mathbf)."
        case "latex/not-in-outer-par-mode":
            return "A figure, table or \\marginpar cannot go inside a box, a minipage or another float. Move it out; in a box, use \\captionof instead of a float."
        case "latex/float-too-large":
            return "This float is taller than the text area, so it runs off the page. Make its content smaller (for example \\includegraphics[height=…])."
        case "latex/undefined-reference":
            return "No \\label{\(quoted(message, after: "Reference `") ?? "…")} exists, so the reference prints ??. Check the key."
        case "latex/undefined-citation":
            return "No bibliography entry has the key \(quoted(message, after: "Citation `") ?? "…"), so the citation prints ?. Check the key against your .bib file or \\bibitem list."
        case "latex/multiply-defined-label":
            return "Two \\label commands use the same key, so \\ref can only point at one of them. Give one of them another key."
        case "latex/rerun":
            return "Labels changed during this compile, so a reference or page number may be out of date until the next one."
        case "latex-font/font-shape-undefined":
            return "The font has no such shape (bold small caps or italic typewriter in Computer Modern, for example), so LaTeX used the nearest one it has."
        case "latex/empty-environment":
            return "The environment has no content. An empty bibliography usually means no \\cite matched an entry of the .bib file."
        case "latex/no-file":
            return "\\include did not find this file, so nothing of it is in the document. Check its name; it must be in the project."
        case _ where code.hasPrefix("package/hyperref/token-not-allowed"):
            return "PDF bookmarks are plain text, so hyperref leaves math and commands out of this title. Give the bookmark a text version with \\texorpdfstring{…}{plain text}."
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
                let eol = text.contains("\r\n") ? "\r\n" : "\n" // the document's own line ending
                return replace(src.startByte, src.startByte, with: "\\end{\(begun)}" + eol, "add \\end{\(begun)} before \\end{document}")
            }
            return replace(src.startByte, src.endByte, with: "\\end{\(begun)}", "close with \\end{\(begun)}")
        case "latex/undefined-reference", "latex/undefined-citation":
            // The key inside the \ref / \cite the range covers, as a whole
            // entry ({key} or one of {a,key,b}), replaced by the one close
            // label or bibliography key of the texts.
            let cite = code == "latex/undefined-citation"
            guard let key = quoted(message, after: cite ? "Citation `" : "Reference `"), let texts else { return nil }
            let known = cite ? EngineV3DiagPresent.citeKeys(in: texts) : EngineV3DiagPresent.labels(in: texts)
            let close = EngineV3Fixes.closestCommands(key, vocabulary: known)
            guard close.count == 1, let at = keyRange(key, in: marked) else { return nil }
            return replace(src.startByte + at.lowerBound, src.startByte + at.upperBound, with: close[0], "did you mean \(close[0])?")
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
