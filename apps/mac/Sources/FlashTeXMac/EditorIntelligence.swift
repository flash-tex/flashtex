import AppKit
import SwiftUI
import FlashTeXProtocol

/// Editor intelligence surfaced in the source editor (lane mac-syntax-highlight):
/// the token under a position (from the `SyntaxHighlighter` runs), quick-info
/// for hover (command documentation, reference/file keys, the diagnostics on
/// that position), ⌘-click definition targets (routed to the owner, who calls
/// the model's `Navigation` APIs), and the Return-key insertion (auto-indent,
/// `\begin{env}` closing). All pure; the AppKit pieces — the line-number
/// gutter and the hover popover — are at the end of the file.
enum EditorIntelligence {
    // MARK: tokens

    enum Token: Equatable {
        /// A control sequence (`\section`, `\alpha`); `name` without the backslash.
        case command(name: String, range: NSRange)
        /// A key inside a `\label`/`\ref`/`\cite`-like argument: one key of a
        /// comma-separated list, with the command that owns it.
        case reference(command: String, key: String, range: NSRange)
        /// A path inside `\input`/`\include`/`\includegraphics`-like arguments.
        case file(command: String, path: String, range: NSRange)
        /// An environment name inside `\begin{…}`/`\end{…}`.
        case environment(name: String, range: NSRange)

        var range: NSRange {
            switch self {
            case .command(_, let r), .reference(_, _, let r), .file(_, _, let r), .environment(_, let r): r
            }
        }
    }

    /// The token at `utf16` (a position on or inside it). `highlighter` must
    /// be in sync with `text`; a fresh one is built when nil (tests).
    static func token(in text: NSString, at utf16: Int, highlighter: SyntaxHighlighter? = nil) -> Token? {
        guard utf16 >= 0, utf16 < text.length else { return nil }
        var h = highlighter ?? SyntaxHighlighter()
        if highlighter == nil { h.reset(text) }
        guard h.length == text.length else { return nil }
        let lineRange = h.lineRange(h.line(at: utf16))
        let runs = h.runs(in: lineRange, text: text)
        guard let index = runs.firstIndex(where: { NSLocationInRange(utf16, $0.range) }) else { return nil }
        let run = runs[index]
        switch run.kind {
        case .command, .mathCommand:
            let s = text.substring(with: run.range)
            guard s.count > 1 else { return nil }
            return .command(name: String(s.dropFirst()), range: run.range)
        case .environment:
            return .environment(name: text.substring(with: run.range), range: run.range)
        case .reference, .file:
            // The owning command is the last command run before the opening brace.
            let owner = runs[..<index].last { $0.kind == .command || $0.kind == .mathCommand }
                .map { String(text.substring(with: $0.range).dropFirst()) } ?? ""
            if run.kind == .file {
                return .file(command: owner, path: text.substring(with: run.range).trimmingCharacters(in: .whitespaces), range: run.range)
            }
            // Comma-separated keys: the one under the position, trimmed.
            let arg = text.substring(with: run.range) as NSString
            var start = 0
            var found: NSRange?
            for i in 0...arg.length {
                if i == arg.length || arg.character(at: i) == 0x2C {
                    let r = NSRange(location: run.range.location + start, length: i - start)
                    if NSLocationInRange(utf16, r) || (i == arg.length && utf16 == NSMaxRange(r)) { found = r; break }
                    start = i + 1
                }
            }
            guard let raw = found else { return nil }
            let keyText = text.substring(with: raw)
            let trimmed = keyText.trimmingCharacters(in: .whitespaces)
            guard !trimmed.isEmpty, let sub = keyText.range(of: trimmed) else { return nil }
            let lead = keyText.utf16.distance(from: keyText.utf16.startIndex, to: sub.lowerBound.samePosition(in: keyText.utf16)!)
            return .reference(command: owner, key: trimmed, range: NSRange(location: raw.location + lead, length: (trimmed as NSString).length))
        default:
            return nil
        }
    }

    // MARK: inline math span (hover preview; lane mac-math-hover)

    /// The full span (delimiters included) of the enclosing inline formula
    /// (`$…$` or `\(…\)`) at `utf16`, or nil when the position is not inside
    /// one — including display math (`$$…$$`, `\[…\]`) and math environments,
    /// which the hover preview does not cover. `highlighter` must be in sync
    /// with `text`; a fresh one is built when nil.
    ///
    /// Bounded to at most two lines each way of `utf16`'s line: inline math
    /// never crosses a blank line (the lexer's rule), and a formula the hover
    /// preview shows spans at most two lines, so a wider search would only
    /// ever confirm "too many lines" — which this already reports as nil.
    static func inlineMathSpan(in text: NSString, at utf16: Int, highlighter: SyntaxHighlighter? = nil) -> NSRange? {
        guard utf16 >= 0, utf16 <= text.length else { return nil }
        var h = highlighter ?? SyntaxHighlighter()
        if highlighter == nil { h.reset(text) }
        guard h.length == text.length, h.lineCount > 0 else { return nil }

        let line0 = h.line(at: utf16)
        switch h.modes[line0] {
        case .text, .inlineMath, .parenMath: break
        default: return nil // display math, a math environment, or verbatim: not inline
        }

        // A line at or before `line0`, within two lines of it, that starts in
        // plain text — a safe restart point, since no open formula's start
        // can cross a `.text`-mode line start.
        let lowest = max(0, line0 - 2)
        guard let ln1 = (lowest...line0).first(where: { h.modes[$0] == .text }) else { return nil } // already unclosed for 2+ lines
        let lastLine = min(h.lineCount - 1, line0 + 2)

        var open: (run: SyntaxHighlighter.Run, close: String)?
        for ln in ln1...lastLine {
            for r in h.runs(in: h.lineRange(ln), text: text) where r.kind == .mathDelimiter {
                let token = text.substring(with: r.range)
                if let o = open {
                    guard token == o.close else { continue } // a display delimiter or stray close: not our pair
                    let span = NSRange(location: o.run.range.location, length: NSMaxRange(r.range) - o.run.range.location)
                    if NSLocationInRange(utf16, span) {
                        let openLine = h.line(at: o.run.range.location)
                        guard ln - openLine <= 1 else { return nil } // spans more than two lines
                        return span
                    }
                    open = nil
                } else if token == "$" {
                    open = (r, "$")
                } else if token == "\\(" {
                    open = (r, "\\)")
                }
                // "$$", "\[", "\]", and a stray close with no opener: display math; ignored.
            }
        }
        return nil // no pair encloses `utf16` within the window (unclosed, or not inline math)
    }

    // MARK: quick info (hover)

    struct QuickInfo: Equatable {
        /// Monospaced headline (`\section`, `eq:main`, `ch/intro.tex`).
        var title: String
        /// What it is ("Command", "Label reference", …).
        var detail: String
        var documentation: String?
        /// Diagnostics at the position (message, then recovery/explanation lines).
        var diagnostics: [Diagnostic] = []
        /// The range the popover is anchored to.
        var range: NSRange

        struct Diagnostic: Equatable {
            var severity: RuntimeV1.Severity
            var message: String
            var lines: [String]
        }
    }

    /// `userDefinition` answers the user's own `\newcommand`/`\def` of a
    /// command name (ShellModel.definitionSummary); shown as a peek under
    /// the standard documentation.
    static func quickInfo(in text: NSString, at utf16: Int, marks: [EditorDiagnostics.Mark] = [],
                          highlighter: SyntaxHighlighter? = nil, userDefinition: (String) -> String? = { _ in nil },
                          context: HoverContext = .init()) -> QuickInfo? {
        let hits = marks.filter { NSLocationInRange(utf16, $0.nsRange) }
        let diagnostics = hits.map { m in
            QuickInfo.Diagnostic(severity: m.severity, message: m.message,
                                 lines: [m.recovery, m.explanation, m.carried?.line].compactMap { $0 })
        }.sorted { $0.severity == .error && $1.severity != .error }
        let token = token(in: text, at: utf16, highlighter: highlighter)
        switch token {
        case .command(let name, let range)?:
            let user = userDefinition(name)
            let doc = [CommandDocs.documentation(for: name), user.map { "Defined: " + $0 + " — ⌘-click to go there." }].compactMap { $0 }
            return QuickInfo(title: "\\" + name, detail: user != nil ? "User command" : CommandDocs.category(for: name),
                             documentation: doc.isEmpty ? nil : doc.joined(separator: "\n"), diagnostics: diagnostics, range: range)
        case .reference(let command, let key, let range)?:
            let isLabel = command == "label"
            let isCite = CommandDocs.citationCommands.contains(command)
            let detail = isLabel ? "Label" : isCite ? "Citation key" : "Label reference"
            // What the key points at, resolved from the buffer and the other
            // open documents (EditorHoverResolution.swift), above the
            // navigation hint — which is the part the reader already knew.
            var lines: [String] = []
            if isCite {
                if let entry = bibliographyEntry(forKey: key, in: text as String, context: context) {
                    lines.append(entry.summary)
                    if let path = entry.path { lines.append("in " + path) }
                } else {
                    lines.append("No bibliography entry found for this key.")
                }
                lines.append("⌘-click to go to the bibliography entry.")
            } else if isLabel {
                lines.append("Referenced with \\ref{\(key)}; ⌘-click a reference to come back here.")
            } else {
                if let target = labelTarget(forKey: key, in: text as String, context: context) {
                    lines.append(target.summary)
                } else {
                    lines.append("No \\label{\(key)} in this document or the open ones.")
                }
                lines.append("⌘-click to go to \\label{\(key)}.")
            }
            return QuickInfo(title: key, detail: detail, documentation: lines.joined(separator: "\n"),
                             diagnostics: diagnostics, range: range)
        case .file(let command, let path, let range)?:
            let detail = command == "includegraphics" ? "Graphics file" : ["usepackage", "RequirePackage"].contains(command) ? "Package"
                : command == "documentclass" ? "Document class" : "Input file"
            var doc: String?
            if command == "includegraphics" {
                doc = resolveGraphics(path, in: text as String, context: context).summary
            } else if ["input", "include", "subfile", "import", "subimport"].contains(command) {
                doc = "⌘-click to open the file."
            }
            return QuickInfo(title: path, detail: detail, documentation: doc, diagnostics: diagnostics, range: range)
        case .environment(let name, let range)?:
            return QuickInfo(title: name, detail: "Environment", documentation: CommandDocs.environmentDocumentation(for: name),
                             diagnostics: diagnostics, range: range)
        case nil:
            guard let first = hits.first else { return nil }
            return QuickInfo(title: first.severity == .error ? "Error" : "Warning", detail: "Diagnostic", documentation: nil,
                             diagnostics: diagnostics, range: first.nsRange)
        }
    }

    // MARK: go to definition

    enum DefinitionTarget: Equatable {
        case label(key: String)
        case citation(key: String)
        case file(path: String, command: String)
        /// `\begin`/`\end` name: the matching partner.
        case environment(name: String)
        /// A control sequence: its `\newcommand`/`\def`/… definition (EditorNavigation.swift).
        case command(name: String)
    }

    /// What ⌘-click at `utf16` navigates to, or nil (plain text).
    static func definitionTarget(in text: NSString, at utf16: Int, highlighter: SyntaxHighlighter? = nil) -> DefinitionTarget? {
        switch token(in: text, at: utf16, highlighter: highlighter) {
        case .reference(let command, let key, _)?:
            return CommandDocs.citationCommands.contains(command) ? .citation(key: key) : .label(key: key)
        case .file(let command, let path, _)?:
            return .file(path: path, command: command)
        case .environment(let name, _)?:
            return .environment(name: name)
        case .command(let name, _)?:
            return name == "begin" || name == "end" ? nil : .command(name: name)
        default:
            return nil
        }
    }

    // MARK: Return key

    struct NewlineInsertion: Equatable {
        /// Text replacing the caret.
        var text: String
        /// Caret position after the insertion, relative to its start.
        var caretOffset: Int
        /// The environment closed by this insertion, if any.
        var closedEnvironment: String?
    }

    /// The Return-key insertion at `caret` (an empty selection): a newline
    /// plus the current line's leading whitespace; one more `indentUnit`
    /// after a line that opens an environment (`\begin{env}` with optional
    /// arguments and nothing else after it), and — when `closeEnvironments`
    /// and the buffer has fewer `\end{env}` than `\begin{env}` — the
    /// matching `\end{env}` on the line after the caret.
    static func newline(in text: NSString, caret: Int, indentUnit: String, closeEnvironments: Bool) -> NewlineInsertion {
        let caret = max(0, min(caret, text.length))
        var lineStart = caret
        while lineStart > 0, text.character(at: lineStart - 1) != 0x0A { lineStart -= 1 }
        var lineEnd = caret
        while lineEnd < text.length, text.character(at: lineEnd) != 0x0A { lineEnd += 1 }
        let prefix = text.substring(with: NSRange(location: lineStart, length: caret - lineStart))
        let suffix = text.substring(with: NSRange(location: caret, length: lineEnd - caret))
        let indent = String(prefix.prefix { $0 == " " || $0 == "\t" })
        // `\item …` Return inside a list continues it with a new `\item `; a
        // bare `\item` line (nothing typed) just breaks the line.
        if suffix.allSatisfy({ $0 == " " || $0 == "\t" || $0 == "\r" }), let item = itemContinuation(inLinePrefix: prefix) {
            let insertion = "\n" + indent + item
            return NewlineInsertion(text: insertion, caretOffset: insertion.utf16.count, closedEnvironment: nil)
        }
        guard let env = openingEnvironment(inLinePrefix: prefix) else {
            return NewlineInsertion(text: "\n" + indent, caretOffset: 1 + indent.utf16.count, closedEnvironment: nil)
        }
        var insertion = "\n" + indent + indentUnit
        let caretOffset = insertion.utf16.count
        var closed: String?
        if closeEnvironments, suffix.allSatisfy({ $0 == " " || $0 == "\t" || $0 == "\r" }),
           occurrences(of: "\\begin{\(env)}", in: text) > occurrences(of: "\\end{\(env)}", in: text) {
            insertion += "\n" + indent + "\\end{\(env)}"
            closed = env
        }
        return NewlineInsertion(text: insertion, caretOffset: caretOffset, closedEnvironment: closed)
    }

    /// The environment a line prefix opens: its last `\begin{name}` followed
    /// only by optional `[...]`/`{...}` arguments and whitespace, unless the
    /// same prefix also closes it or is a comment.
    static func openingEnvironment(inLinePrefix prefix: String) -> String? {
        guard let beginRange = prefix.range(of: "\\begin{", options: .backwards) else { return nil }
        let head = prefix[..<beginRange.lowerBound]
        if head.contains("%") && !head.contains("\\%") { return nil } // crude: a comment before \begin
        guard let close = prefix[beginRange.upperBound...].firstIndex(of: "}") else { return nil }
        let name = String(prefix[beginRange.upperBound..<close])
        guard !name.isEmpty, !name.contains("\n"), !name.contains("\\") else { return nil }
        if SyntaxHighlighter.verbatimEnvironments.contains(name) || name == "document" { return nil }
        // After the name: optional argument groups then whitespace only.
        var rest = Substring(prefix[prefix.index(after: close)...])
        while true {
            rest = rest.drop { $0 == " " || $0 == "\t" }
            guard let open = rest.first, open == "[" || open == "{" else { break }
            let closer: Character = open == "[" ? "]" : "}"
            var depth = 0
            var i = rest.startIndex
            var endIndex: Substring.Index?
            while i < rest.endIndex {
                let c = rest[i]
                if c == "\\" { i = rest.index(i, offsetBy: 2, limitedBy: rest.endIndex) ?? rest.endIndex; continue }
                if c == open { depth += 1 } else if c == closer { depth -= 1; if depth == 0 { endIndex = i; break } }
                i = rest.index(after: i)
            }
            guard let endIndex else { return nil }
            rest = rest[rest.index(after: endIndex)...]
        }
        guard rest.allSatisfy({ $0 == " " || $0 == "\t" || $0 == "\r" }) else { return nil }
        if prefix.contains("\\end{\(name)}") { return nil }
        return name
    }

    /// `\item ` (or `\item[…] ` for a description entry) when the line prefix
    /// is a list entry with content after the `\item`; nil for a bare `\item`
    /// (the user is leaving the list) or any other line.
    static func itemContinuation(inLinePrefix prefix: String) -> String? {
        let body = prefix.drop { $0 == " " || $0 == "\t" }
        guard body.hasPrefix("\\item") else { return nil }
        var rest = body.dropFirst(5)
        if rest.first == "[" {
            guard let close = rest.firstIndex(of: "]") else { return nil }
            rest = rest[rest.index(after: close)...]
        } else if let c = rest.first, c.isLetter { return nil } // `\itemize`, `\items`: not an item
        guard rest.contains(where: { $0 != " " && $0 != "\t" }) else { return nil }
        return body.dropFirst(5).first == "[" ? "\\item[] " : "\\item "
    }

    // MARK: ⌘/ line comment

    struct CommentToggle: Equatable {
        var range: NSRange
        var replacement: String
        var selection: NSRange
    }

    /// Toggles `% ` at the indentation of every line the selection touches:
    /// when every non-blank touched line is already commented the markers are
    /// removed (`% ` or `%`), otherwise each non-blank line gets `% ` after
    /// its leading whitespace. Blank lines are left alone; the selection is
    /// moved to cover the same lines. Nil when nothing would change.
    static func toggleComment(in text: NSString, selection: NSRange) -> CommentToggle? {
        let sel = NSRange(location: max(0, min(selection.location, text.length)),
                          length: max(0, min(selection.length, text.length - min(selection.location, text.length))))
        var start = sel.location
        while start > 0, text.character(at: start - 1) != 0x0A { start -= 1 }
        var end = NSMaxRange(sel)
        if sel.length > 0, end > start, text.character(at: end - 1) == 0x0A { end -= 1 } // a selection ending at a line start excludes that line
        while end < text.length, text.character(at: end) != 0x0A { end += 1 }
        let block = text.substring(with: NSRange(location: start, length: end - start))
        let lines = block.components(separatedBy: "\n")
        let nonBlank = lines.filter { $0.contains { $0 != " " && $0 != "\t" } }
        guard !nonBlank.isEmpty else { return nil }
        let allCommented = nonBlank.allSatisfy { $0.drop { $0 == " " || $0 == "\t" }.hasPrefix("%") }
        let out = lines.map { line -> String in
            let indent = line.prefix { $0 == " " || $0 == "\t" }
            let body = line.dropFirst(indent.count)
            if !body.contains(where: { $0 != " " && $0 != "\t" }) { return line }
            if allCommented {
                let stripped = body.hasPrefix("% ") ? body.dropFirst(2) : body.dropFirst(1)
                return String(indent) + String(stripped)
            }
            return String(indent) + "% " + String(body)
        }.joined(separator: "\n")
        let range = NSRange(location: start, length: end - start)
        let newLength = (out as NSString).length
        // A caret stays on its line (shifted by that line's change); a selection covers the toggled lines.
        let selection = sel.length == 0
            ? NSRange(location: min(max(start, sel.location + newLength - range.length), start + newLength), length: 0)
            : NSRange(location: start, length: newLength)
        return CommentToggle(range: range, replacement: out, selection: selection)
    }

    static func occurrences(of needle: String, in text: NSString) -> Int {
        var count = 0
        var search = NSRange(location: 0, length: text.length)
        while true {
            let r = text.range(of: needle, options: .literal, range: search)
            guard r.location != NSNotFound else { return count }
            count += 1
            search = NSRange(location: NSMaxRange(r), length: text.length - NSMaxRange(r))
        }
    }

    // MARK: command documentation

    enum CommandDocs {
        static let citationCommands: Set<String> = [
            "cite", "citep", "citet", "citeauthor", "citeyear", "citealp", "citealt", "nocite", "parencite", "textcite",
            "autocite", "footcite", "fullcite", "Cite", "Parencite", "Textcite", "Autocite",
        ]

        static func category(for name: String) -> String {
            if citationCommands.contains(name) { return "Citation command" }
            if SyntaxHighlighter.referenceCommands.contains(name) { return name == "label" ? "Label command" : "Reference command" }
            if SyntaxHighlighter.fileCommands.contains(name) { return "File command" }
            if SyntaxHighlighter.definitionCommands.contains(name) { return "Definition" }
            if name.count == 1, !(name.first?.isLetter ?? true) { return "Control symbol" }
            return "Command"
        }

        static func documentation(for name: String) -> String? { table[name] }

        /// Standard LaTeX commands and environments the hover documents
        /// although the compiler does not render them (it diagnoses them, so
        /// the editor still explains what the user typed). Every other `table`
        /// / `environments` key names a command or environment of the
        /// compiler's inventory (`Completion.Vocabulary.inventory`;
        /// `CompletionTests.testCommandDocsNameOnlyKnownCommands`), and a
        /// name listed here must leave the list once the compiler renders it.
        static let beyondCompiler: Set<String> = [
            "chapter", "part", "paragraph", "autoref",
            "def", "newline", "hline", "toprule", "midrule",
            "bottomrule", "multicolumn", "verb", "%", "$", "&", "#", "_", "{", "}",
            "geometry", "onehalfspacing", "doublespacing",
        ]
        static let environmentsBeyondCompiler: Set<String> = [
            "table", "abstract", "minted", "theorem", "tikzpicture", "minipage", "frame", "comment",
        ]

        static func environmentDocumentation(for name: String) -> String? {
            let base = name.hasSuffix("*") ? String(name.dropLast()) : name
            guard let doc = environments[base] else { return nil }
            return name.hasSuffix("*") ? doc + " Starred: unnumbered." : doc
        }

        static let environments: [String: String] = [
            "document": "The document body; everything typeset goes between \\begin{document} and \\end{document}.",
            "equation": "One numbered displayed equation.",
            "align": "Aligned equations, one per line, aligned at &; each line numbered.",
            "gather": "Centred displayed equations, one per line.",
            "multline": "One long equation split over lines: first line left, last line right.",
            "cases": "Piecewise definition with a left brace: value & condition rows.",
            "pmatrix": "Matrix in parentheses; rows end with \\\\, cells separated by &.",
            "bmatrix": "Matrix in square brackets.",
            "matrix": "Matrix without delimiters.",
            "itemize": "Bulleted list of \\item entries.",
            "enumerate": "Numbered list of \\item entries.",
            "description": "List of \\item[term] entries.",
            "figure": "Floating figure; use \\centering, \\includegraphics, \\caption and \\label inside.",
            "table": "Floating table; wrap a tabular in it with \\caption and \\label.",
            "tabular": "Table body: column spec such as {lcr}, cells separated by &, rows by \\\\.",
            "center": "Centred lines.",
            "abstract": "The abstract, typeset before the first section.",
            "verbatim": "Typeset exactly as written, in a monospaced font; no commands are interpreted.",
            "lstlisting": "Source-code listing (listings package); the language comes from the options.",
            "minted": "Source-code listing highlighted by Pygments (minted package).",
            "theorem": "A numbered theorem (declared with \\newtheorem).",
            "proof": "A proof, ended with a QED mark (amsthm).",
            "quote": "An indented quotation.",
            "tikzpicture": "A TikZ drawing.",
            "array": "Math-mode table with a column spec, like tabular.",
            "split": "Splits one numbered equation over aligned lines (inside equation).",
            "aligned": "Aligned block usable inside another math environment.",
            "subequations": "Numbers the equations inside as 1a, 1b, ….",
            "minipage": "A box of the given width in which paragraphs are typeset.",
            "frame": "One Beamer slide.",
            "comment": "Everything inside is skipped (comment package).",
        ]

        static let table: [String: String] = [
            "documentclass": "\\documentclass[options]{class}: the document type (article, report, book, beamer, …).",
            "usepackage": "\\usepackage[options]{package}: loads a package in the preamble.",
            "begin": "\\begin{env}: opens an environment; closed by the matching \\end{env}.",
            "end": "\\end{env}: closes the innermost open environment of that name.",
            "section": "\\section{title}: a numbered section heading. \\section* is unnumbered.",
            "subsection": "\\subsection{title}: a numbered subsection heading.",
            "subsubsection": "\\subsubsection{title}: a third-level heading.",
            "chapter": "\\chapter{title}: a chapter heading (report and book classes).",
            "part": "\\part{title}: a part heading, above chapters.",
            "paragraph": "\\paragraph{title}: a run-in heading.",
            "title": "\\title{text}: the document title, typeset by \\maketitle.",
            "author": "\\author{names}: the author line, typeset by \\maketitle; separate authors with \\and.",
            "date": "\\date{text}: the date for \\maketitle (\\today for the current date).",
            "maketitle": "Typesets the title block from \\title, \\author and \\date.",
            "tableofcontents": "Inserts the table of contents (needs a second run).",
            "label": "\\label{key}: names the current section, equation, figure or table for \\ref.",
            "ref": "\\ref{key}: the number of the labelled item.",
            "eqref": "\\eqref{key}: the equation number in parentheses (amsmath).",
            "pageref": "\\pageref{key}: the page of the labelled item.",
            "autoref": "\\autoref{key}: the item's type and number, e.g. \"Section 2\" (hyperref).",
            "cref": "\\cref{key}: the item's type and number (cleveref); \\Cref capitalises.",
            "cite": "\\cite[note]{keys}: cites bibliography entries by key.",
            "citep": "\\citep{keys}: parenthetical citation (natbib).",
            "citet": "\\citet{keys}: textual citation, \"Author (year)\" (natbib).",
            "bibliography": "\\bibliography{file}: the BibTeX database(s) and where the list is printed.",
            "bibliographystyle": "\\bibliographystyle{style}: the BibTeX style (plain, abbrv, alpha, …).",
            "input": "\\input{file}: inserts the file's contents here, as if typed.",
            "include": "\\include{file}: inserts the file on a new page; pairs with \\includeonly.",
            "includegraphics": "\\includegraphics[width=…]{file}: places an image (graphicx).",
            "caption": "\\caption{text}: the caption of a figure or table; put \\label after it.",
            "centering": "Centres the rest of the current group (use inside figure/table).",
            "textbf": "\\textbf{text}: bold.",
            "textit": "\\textit{text}: italic.",
            "emph": "\\emph{text}: emphasis (italic, upright inside italic).",
            "texttt": "\\texttt{text}: monospaced.",
            "textsc": "\\textsc{text}: small capitals.",
            "underline": "\\underline{text}: underlined.",
            "item": "\\item: the next list entry; \\item[label] gives it a custom label.",
            "footnote": "\\footnote{text}: a numbered footnote.",
            "newcommand": "\\newcommand{\\name}[n]{body}: defines a macro with n arguments (#1 … #n).",
            "renewcommand": "\\renewcommand{\\name}[n]{body}: redefines an existing macro.",
            "providecommand": "\\providecommand{\\name}{body}: defines the macro only if it does not exist.",
            "newenvironment": "\\newenvironment{name}{begin}{end}: defines an environment.",
            "newtheorem": "\\newtheorem{name}{Title}: declares a theorem-like environment.",
            "DeclareMathOperator": "\\DeclareMathOperator{\\name}{text}: an upright operator like \\sin.",
            "def": "\\def\\name{body}: TeX primitive macro definition (prefer \\newcommand).",
            "frac": "\\frac{num}{den}: a fraction.",
            "dfrac": "\\dfrac{num}{den}: a display-style fraction (amsmath).",
            "sqrt": "\\sqrt[n]{x}: a (nth) root.",
            "sum": "Summation sign; limits with _{…}^{…}.",
            "prod": "Product sign; limits with _{…}^{…}.",
            "int": "Integral sign; limits with _{…}^{…}.",
            "lim": "Limit operator; the subscript goes below in display style.",
            "infty": "∞",
            "alpha": "α", "beta": "β", "gamma": "γ", "delta": "δ", "epsilon": "ϵ", "varepsilon": "ε", "theta": "θ", "lambda": "λ",
            "mu": "μ", "pi": "π", "sigma": "σ", "phi": "ϕ", "varphi": "φ", "omega": "ω", "Omega": "Ω", "Delta": "Δ", "Sigma": "Σ",
            "cdot": "·  (multiplication dot)", "times": "×", "leq": "≤", "geq": "≥", "neq": "≠", "approx": "≈", "equiv": "≡",
            "in": "∈", "subset": "⊂", "subseteq": "⊆", "cup": "∪", "cap": "∩", "forall": "∀", "exists": "∃", "rightarrow": "→", "to": "→",
            "Rightarrow": "⇒", "leftarrow": "←", "Leftrightarrow": "⇔", "partial": "∂", "nabla": "∇", "pm": "±", "ldots": "…", "cdots": "⋯",
            "mathbb": "\\mathbb{R}: blackboard bold (amssymb).",
            "mathbf": "\\mathbf{x}: bold math.",
            "mathrm": "\\mathrm{d}: upright math.",
            "mathcal": "\\mathcal{L}: calligraphic capitals.",
            "text": "\\text{words}: text inside math (amsmath).",
            "left": "\\left( … \\right): delimiters sized to the content.",
            "right": "Closes a \\left delimiter.",
            "hat": "\\hat{x}: a hat accent.", "bar": "\\bar{x}: a bar accent.", "vec": "\\vec{x}: an arrow accent.",
            "operatorname": "\\operatorname{name}: an upright operator (amsmath).",
            "binom": "\\binom{n}{k}: a binomial coefficient (amsmath).",
            "quad": "A 1 em horizontal space.", "qquad": "A 2 em horizontal space.",
            "hspace": "\\hspace{length}: horizontal space.", "vspace": "\\vspace{length}: vertical space.",
            "newpage": "Starts a new page.", "clearpage": "Ends the page and flushes pending floats.",
            "noindent": "Suppresses the paragraph indent.",
            "linebreak": "Breaks the line (justified).", "newline": "Breaks the line (ragged).",
            "\\": "Line break; \\\\[length] adds vertical space.",
            "hline": "A horizontal rule in a tabular.",
            "toprule": "The top rule of a booktabs table.", "midrule": "A booktabs rule between header and body.", "bottomrule": "The bottom rule of a booktabs table.",
            "multicolumn": "\\multicolumn{n}{align}{text}: a cell spanning n columns.",
            "verb": "\\verb|text|: inline verbatim between any two identical delimiters.",
            "url": "\\url{address}: a typeset (and linked) URL.",
            "href": "\\href{url}{text}: a hyperlink (hyperref).",
            "today": "The current date.",
            "LaTeX": "The LaTeX logo.", "TeX": "The TeX logo.",
            "%": "A literal percent sign.", "$": "A literal dollar sign.", "&": "A literal ampersand.", "#": "A literal hash.",
            "_": "A literal underscore.", "{": "A literal left brace.", "}": "A literal right brace.", ",": "A thin space (math and text).",
            "setlength": "\\setlength{\\parameter}{length}: sets a length such as \\parindent.",
            "geometry": "\\geometry{options}: page margins (geometry package).",
            "graphicspath": "\\graphicspath{{dir/}}: where \\includegraphics looks for files.",
            "pagestyle": "\\pagestyle{style}: headers and footers (plain, empty, headings).",
            "onehalfspacing": "1.5 line spacing (setspace).", "doublespacing": "Double line spacing (setspace).",
        ]
    }
}

// MARK: - line-number gutter

/// Line numbers and diagnostic markers beside the editor (an `NSRulerView`
/// installed as the scroll view's vertical ruler). Line numbers come from
/// the syntax model's line table (binary search per visible line, never a
/// scan of the buffer); only the visible fragments are drawn.
@MainActor
final class LineNumberGutter: NSRulerView {
    /// Line index → worst severity on that line.
    private(set) var severities: [Int: RuntimeV1.Severity] = [:]
    /// Lines whose only marks are FlashTeX gaps (`EditorDiagnostics.isGap`):
    /// a faint grey tick, never a red or orange dot.
    private(set) var gapLines: Set<Int> = []
    /// Current line (caret), highlighted in the gutter.
    var currentLine: Int? { didSet { if currentLine != oldValue { setNeedsRedraw() } } }
    /// Hybrid relative numbering for Vim users (`EditorPreferences.relativeLineNumbers`,
    /// off by default and independent of whether Vim keybindings are on).
    var relativeLineNumbers = false { didSet { if relativeLineNumbers != oldValue { setNeedsRedraw() } } }
    /// Line indices (0-based) that start a foldable region (EditorFolding.swift).
    var foldableLines: Set<Int> = [] { didSet { if foldableLines != oldValue { setNeedsRedraw() } } }
    /// Line indices that are currently folded.
    var foldedLines: Set<Int> = [] { didSet { if foldedLines != oldValue { setNeedsRedraw() } } }
    /// Toggle the fold whose header is this 0-based line.
    var onToggleFold: ((Int) -> Void)?
    /// Test seam, like `CaretFollow.enabledOverride`: a hosted editor reads the
    /// shared preferences, which a test cannot inject into. Set it in `setUp`
    /// and clear it in `tearDown`.
    nonisolated(unsafe) static var relativeOverride: Bool?
    /// The model that answers "which line is this offset on".
    var lineTable: (() -> SyntaxHighlighter)?
    private var digits = 2
    /// Counts the times the gutter has asked to be redrawn. `needsDisplay` is
    /// not observable from a test — AppKit re-dirties a view that is in a
    /// window, and ignores the flag on a view that is not — so the request
    /// itself is counted, which is the thing the caller controls.
    private(set) var redrawRequests = 0

    private func setNeedsRedraw() {
        redrawRequests += 1
        needsDisplay = true
    }

    init(scrollView: NSScrollView) {
        super.init(scrollView: scrollView, orientation: .verticalRuler)
        clientView = scrollView.documentView
        ruleThickness = 40
    }

    @available(*, unavailable) required init(coder: NSCoder) { fatalError() }

    var textView: NSTextView? { clientView as? NSTextView }

    /// Recomputes the marker per line from `marks` (errors win).
    func update(marks: [EditorDiagnostics.Mark]) {
        guard let table = lineTable?() else { return }
        var result: [Int: RuntimeV1.Severity] = [:]
        var gaps: Set<Int> = []
        for mark in marks {
            guard mark.nsRange.location >= 0, mark.nsRange.location <= table.length else { continue }
            let line = table.line(at: mark.nsRange.location)
            if EditorDiagnostics.isGap(mark.message) { gaps.insert(line); continue }
            if result[line] != .error { result[line] = mark.severity }
        }
        if result != severities || gaps != gapLines { severities = result; gapLines = gaps; setNeedsRedraw() }
    }

    /// Adjusts the width to the line count and the editor font.
    func layoutIfNeeded(lineCount: Int) {
        let wanted = max(2, String(lineCount).count)
        let font = numberFont
        let digitWidth = ("8" as NSString).size(withAttributes: [.font: font]).width
        let thickness = ceil(CGFloat(wanted) * digitWidth + 30) // marker column + padding
        if wanted != digits || abs(thickness - ruleThickness) > 0.5 {
            digits = wanted
            ruleThickness = thickness
            setNeedsRedraw()
        }
    }

    /// The label for logical line `line` (0-based), given the caret's line.
    ///
    /// Vim's hybrid `number` + `relativenumber`: the caret's own line keeps its
    /// absolute number, so you always know where you are, while every other
    /// line shows its distance in *logical* lines — the count `5j` and `3k`
    /// take. Plain `relativenumber` would print `0` on the current line, which
    /// is the less useful of the two and not what most people mean by this.
    /// Falls back to absolute numbering when there is no caret line.
    static func label(line: Int, currentLine: Int?, relative: Bool) -> String {
        guard relative, let current = currentLine, line != current else { return String(line + 1) }
        return String(abs(line - current))
    }

    var numberFont: NSFont {
        let size = max(9, (textView?.font?.pointSize ?? 13) - 1.5)
        return NSFont.monospacedDigitSystemFont(ofSize: size, weight: .regular)
    }

    override func drawHashMarksAndLabels(in rect: NSRect) {
        guard let tv = textView, let lm = tv.layoutManager, let container = tv.textContainer, let table = lineTable?() else { return }
        (tv.backgroundColor).setFill()
        bounds.fill()
        // Hairline separator.
        NSColor.separatorColor.withAlphaComponent(0.5).setFill()
        NSRect(x: bounds.maxX - 1, y: bounds.minY, width: 1, height: bounds.height).fill()

        let visible = tv.visibleRect
        let glyphs = lm.glyphRange(forBoundingRect: visible, in: container)
        let chars = lm.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
        let length = tv.textStorage?.length ?? 0
        guard table.length == length else { return }
        let font = numberFont
        let numberAttrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: SyntaxTheme.gutterText]
        let currentAttrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: SyntaxTheme.gutterCurrentText]
        var line = table.line(at: chars.location)
        let numberRight = bounds.maxX - 6
        let inset = tv.textContainerInset
        while line < table.lineCount {
            let start = table.lineStarts[line]
            if start > NSMaxRange(chars) || (start == length && length > 0 && line > 0 && tv.textStorage?.string.hasSuffix("\n") == false) { break }
            let fragment: NSRect
            if start < length {
                let g = lm.glyphIndexForCharacter(at: start)
                guard g < lm.numberOfGlyphs else { break }
                fragment = lm.lineFragmentRect(forGlyphAt: g, effectiveRange: nil)
            } else {
                fragment = lm.extraLineFragmentRect
                if fragment.isEmpty { break }
            }
            let inText = fragment.offsetBy(dx: inset.width, dy: inset.height)
            let inRuler = convert(inText, from: tv)
            guard inRuler.maxY >= rect.minY - 20, inRuler.minY <= rect.maxY + 20 else {
                if inRuler.minY > rect.maxY + 20 { break }
                line += 1; continue
            }
            let label = Self.label(line: line, currentLine: currentLine, relative: relativeLineNumbers) as NSString
            let attrs = line == currentLine ? currentAttrs : numberAttrs
            let size = label.size(withAttributes: attrs)
            let baselineAdjust = (fragment.height - size.height) / 2
            label.draw(at: NSPoint(x: numberRight - size.width, y: inRuler.minY + baselineAdjust), withAttributes: attrs)
            if foldableLines.contains(line) {
                drawFoldMark(folded: foldedLines.contains(line), midY: inRuler.midY)
            }
            if let severity = severities[line] {
                let d: CGFloat = 7
                let dot = NSRect(x: 6, y: inRuler.midY - d / 2, width: d, height: d)
                (severity == .error ? NSColor.systemRed : NSColor.systemOrange).setFill()
                NSBezierPath(ovalIn: dot).fill()
            } else if gapLines.contains(line) {
                NSColor.tertiaryLabelColor.setFill()
                NSBezierPath(ovalIn: NSRect(x: 7.5, y: inRuler.midY - 2, width: 4, height: 4)).fill()
            }
            line += 1
        }
    }

    /// Disclosure triangle in the marker column: collapsed ▶ when folded, ▼ when open.
    private func drawFoldMark(folded: Bool, midY: CGFloat) {
        let r = NSRect(x: 3, y: midY - 4, width: 8, height: 8)
        NSColor.secondaryLabelColor.setFill()
        let path = NSBezierPath()
        if folded {
            path.move(to: NSPoint(x: r.minX + 1, y: r.minY + 1))
            path.line(to: NSPoint(x: r.maxX - 1, y: r.midY))
            path.line(to: NSPoint(x: r.minX + 1, y: r.maxY - 1))
        } else {
            path.move(to: NSPoint(x: r.minX + 1, y: r.minY + 2))
            path.line(to: NSPoint(x: r.maxX - 1, y: r.minY + 2))
            path.line(to: NSPoint(x: r.midX, y: r.maxY - 1))
        }
        path.close()
        path.fill()
    }

    override func mouseDown(with event: NSEvent) {
        let p = convert(event.locationInWindow, from: nil)
        guard p.x <= 16, let onToggleFold, let tv = textView, let lm = tv.layoutManager,
              let container = tv.textContainer, let table = lineTable?() else {
            super.mouseDown(with: event); return
        }
        let inText = convert(p, to: tv)
        let index = tv.characterIndexForInsertion(at: NSPoint(x: tv.visibleRect.minX + 1, y: inText.y))
        _ = (lm, container)
        let line = table.line(at: min(index, max(0, table.length - 1)))
        guard foldableLines.contains(line) else { super.mouseDown(with: event); return }
        onToggleFold(line)
    }
}

// MARK: - hover popover

/// Quick-info popover after the pointer rests on a token or diagnostic.
/// The owner installs it on the text view and answers `info(at:)`; the
/// popover closes on pointer exit, any key, text change or scroll.
@MainActor
final class HoverController: NSResponder {
    static let delay: TimeInterval = 0.45
    var info: (Int) -> EditorIntelligence.QuickInfo? = { _ in nil }
    /// Inline math preview (MathHoverPreview.swift): the formula's cropped
    /// bitmap and its range, when `index` is inside one. Tried before `info`,
    /// so a formula's crop wins over a command's quick info inside it (e.g.
    /// `\alpha`); the owner's closure already applies `previewIsStale` and
    /// the "reads only the current bitmap" rule, so nil here just means "no
    /// preview" — hover falls back to `info`.
    var mathPreview: (Int) -> (image: CGImage, range: NSRange)? = { _ in nil }
    private weak var textView: NSTextView?
    private var trackingArea: NSTrackingArea?
    private var timer: Timer?
    private var popover: NSPopover?
    /// Range of the token the open popover describes.
    private(set) var shownRange: NSRange?
    /// Evidence for tests: infos presented.
    private(set) var presented: [EditorIntelligence.QuickInfo] = []
    /// Evidence for tests: math-preview ranges presented.
    private(set) var presentedMathPreviews: [NSRange] = []
    private var lastPoint: NSPoint = .zero

    func install(on tv: NSTextView) {
        textView = tv
        let area = NSTrackingArea(rect: .zero, options: [.mouseMoved, .mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect], owner: self, userInfo: nil)
        tv.addTrackingArea(area)
        trackingArea = area
    }

    override func mouseMoved(with event: NSEvent) {
        guard let tv = textView else { return }
        lastPoint = tv.convert(event.locationInWindow, from: nil)
        if let shownRange, let index = characterIndex(at: lastPoint), NSLocationInRange(index, shownRange) { return }
        dismiss()
        timer?.invalidate()
        let t = Timer(timeInterval: Self.delay, repeats: false) { [weak self] _ in MainActor.assumeIsolated { self?.fire() } }
        RunLoop.main.add(t, forMode: .common)
        timer = t
    }

    override func mouseExited(with event: NSEvent) { timer?.invalidate(); dismiss() }

    func dismiss() {
        timer?.invalidate()
        guard let popover else { return }
        popover.close()
        self.popover = nil
        shownRange = nil
    }

    /// Character index under `point` (text-view coordinates), nil when the
    /// point is beside the text rather than on a glyph.
    func characterIndex(at point: NSPoint) -> Int? {
        guard let tv = textView, let lm = tv.layoutManager, let container = tv.textContainer else { return nil }
        let inContainer = NSPoint(x: point.x - tv.textContainerInset.width, y: point.y - tv.textContainerInset.height)
        var fraction: CGFloat = 0
        let glyph = lm.glyphIndex(for: inContainer, in: container, fractionOfDistanceThroughGlyph: &fraction)
        guard glyph < lm.numberOfGlyphs else { return nil }
        let rect = lm.boundingRect(forGlyphRange: NSRange(location: glyph, length: 1), in: container)
        guard rect.insetBy(dx: -2, dy: -2).contains(inContainer) else { return nil }
        return lm.characterIndexForGlyph(at: glyph)
    }

    private func fire() {
        guard let tv = textView, tv.window != nil, !tv.hasMarkedText(), let index = characterIndex(at: lastPoint) else { return }
        if let math = mathPreview(index) {
            presentMathPreview(math.image, range: math.range, in: tv)
        } else if let info = info(index) {
            present(info, in: tv)
        }
    }

    func present(_ info: EditorIntelligence.QuickInfo, in tv: NSTextView) {
        presentContent(NSHostingController(rootView: QuickInfoView(info: info)), range: info.range, in: tv)
        presented.append(info)
        if presented.count > 32 { presented.removeFirst(presented.count - 32) }
    }

    /// Anchors `controller`'s view over `range`, replacing whatever popover
    /// is open — the one hover-popover slot every hover surface shares (a
    /// math preview and quick info never show at once).
    func presentContent(_ controller: NSViewController, range: NSRange, in tv: NSTextView) {
        guard let lm = tv.layoutManager, let container = tv.textContainer else { return }
        dismiss()
        let glyphs = lm.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
        var anchor = lm.boundingRect(forGlyphRange: glyphs, in: container)
        anchor = anchor.offsetBy(dx: tv.textContainerInset.width, dy: tv.textContainerInset.height)
        let p = NSPopover()
        p.behavior = .applicationDefined
        p.animates = false
        p.contentViewController = controller
        popover = p
        shownRange = range
        p.show(relativeTo: anchor, of: tv, preferredEdge: .maxY)
    }

    /// Records a presented math preview for `presentedMathPreviews` (evidence
    /// for tests); `presentedMathPreviews`'s setter is file-private, so
    /// `presentMathPreview(_:range:in:)` (MathHoverPreview.swift) goes through this.
    func recordMathPreview(_ range: NSRange) {
        presentedMathPreviews.append(range)
        if presentedMathPreviews.count > 32 { presentedMathPreviews.removeFirst(presentedMathPreviews.count - 32) }
    }
}

/// Popover content: title, kind, documentation, diagnostics.
struct QuickInfoView: View {
    let info: EditorIntelligence.QuickInfo

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(info.title).font(.system(.body, design: .monospaced).weight(.semibold)).lineLimit(2)
                Text(info.detail).font(.caption).foregroundStyle(.secondary)
            }
            if let doc = info.documentation {
                Text(doc).font(.callout).foregroundStyle(.primary).fixedSize(horizontal: false, vertical: true)
            }
            ForEach(Array(info.diagnostics.enumerated()), id: \.offset) { _, d in
                Divider()
                HStack(alignment: .top, spacing: 6) {
                    Image(systemName: d.severity == .error ? "xmark.octagon.fill" : "exclamationmark.triangle.fill")
                        .foregroundStyle(d.severity == .error ? Color.red : Color.orange)
                        .accessibilityLabel(d.severity == .error ? "Error" : "Warning")
                    VStack(alignment: .leading, spacing: 3) {
                        Text(d.message).font(.callout).fixedSize(horizontal: false, vertical: true)
                        ForEach(Array(d.lines.enumerated()), id: \.offset) { _, line in
                            Text(line).font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                        }
                    }
                }
            }
        }
        .padding(10)
        .frame(minWidth: 180, maxWidth: 380, alignment: .leading)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Quick info: \(info.title), \(info.detail)")
    }
}
