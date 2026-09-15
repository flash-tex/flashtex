import Foundation

/// What a hovered cross-reference actually points at (lane mac-lsp-hover).
///
/// `EditorIntelligence.quickInfo` used to answer `\ref{eq:main}` with nothing
/// but "⌘-click to go to \label{eq:main}" — true, but the reader already knew
/// that. These resolvers answer the question hovering asks: *what is over
/// there*. They are pure functions over document text plus a
/// `EditorIntelligence.HoverContext` holding the other open documents and one
/// `fileExists` probe, so every case is testable without a project, a shell
/// model or a file system.
///
/// Numbers are deliberately absent. The editor can see which section or float
/// a label sits in, but not the number LaTeX will print for it: that depends on
/// counters, `\include` order, starred headings, appendices and the class's
/// numbering scheme, none of which the project index reports. Resolving the
/// *kind and text* ("Section “Fourier analysis”") is what actually
/// disambiguates two similar keys, and it is never a guess.
extension EditorIntelligence {
    /// Document knowledge the hover uses beyond the buffer it is in.
    struct HoverContext {
        /// The other documents of the project, in project order: `\label`s and
        /// bibliography entries the active document does not hold.
        var otherDocuments: [Document] = []
        /// Whether a project-relative path names a file that exists.
        /// The default answers "unknown" as `false`, and the hover then says
        /// the path could not be resolved rather than claiming it is missing.
        var fileExists: (String) -> Bool = { _ in false }
        /// Whether `fileExists` can answer at all (there is a project root).
        /// False keeps `\includegraphics` hover silent about existence.
        var canProbeFiles = false

        struct Document: Equatable {
            var path: String
            var text: String
            init(path: String, text: String) { self.path = path; self.text = text }
        }

        init(otherDocuments: [Document] = [], canProbeFiles: Bool = false,
             fileExists: @escaping (String) -> Bool = { _ in false }) {
            self.otherDocuments = otherDocuments
            self.canProbeFiles = canProbeFiles
            self.fileExists = fileExists
        }
    }

    // MARK: - labels

    /// What a `\label{key}` names: the float, math environment or heading that
    /// encloses it, with that item's own text when it has one.
    struct LabelTarget: Equatable {
        /// "Section", "Figure", "Equation", "Theorem", …
        var kind: String
        /// The heading title or the float's `\caption`, LaTeX markup stripped;
        /// nil for an item that carries no text of its own (a bare equation).
        var text: String?
        /// 1-based line of the `\label` itself.
        var line: Int
        /// Project path when the label lives in another document, else nil.
        var path: String?

        /// One line for the hover popover: `Section “Fourier analysis” — line 12`.
        var summary: String {
            var s = kind
            if let text, !text.isEmpty { s += " “" + text + "”" }
            s += " — line \(line)"
            if let path { s += " in " + path }
            return s
        }
    }

    /// Resolves `key` in `text` first, then in the context's other documents.
    static func labelTarget(forKey key: String, in text: String, context: HoverContext = .init()) -> LabelTarget? {
        if let here = labelTarget(forKey: key, inDocument: text) { return here }
        for doc in context.otherDocuments {
            if var found = labelTarget(forKey: key, inDocument: doc.text) {
                found.path = doc.path
                return found
            }
        }
        return nil
    }

    /// Environments that name the thing a label inside them refers to. The
    /// value is what the hover calls it.
    static let labelledEnvironments: [String: String] = [
        "figure": "Figure", "figure*": "Figure", "wrapfigure": "Figure", "subfigure": "Subfigure",
        "table": "Table", "table*": "Table", "longtable": "Table", "sidewaystable": "Table",
        "equation": "Equation", "equation*": "Equation", "align": "Equation", "align*": "Equation",
        "gather": "Equation", "gather*": "Equation", "multline": "Equation", "multline*": "Equation",
        "flalign": "Equation", "alignat": "Equation", "eqnarray": "Equation", "subequations": "Equation",
        "lstlisting": "Listing", "minted": "Listing", "verbatim": "Listing",
        "algorithm": "Algorithm", "algorithmic": "Algorithm",
        "theorem": "Theorem", "lemma": "Lemma", "proposition": "Proposition",
        "corollary": "Corollary", "definition": "Definition", "remark": "Remark", "example": "Example",
    ]

    /// Environments whose `\caption` gives the label its text.
    static let captionedEnvironments: Set<String> = [
        "figure", "figure*", "wrapfigure", "subfigure", "table", "table*", "longtable",
        "sidewaystable", "lstlisting", "minted", "algorithm",
    ]

    static let sectioningCommands: [String: String] = [
        "part": "Part", "chapter": "Chapter", "section": "Section", "subsection": "Subsection",
        "subsubsection": "Subsubsection", "paragraph": "Paragraph", "subparagraph": "Subparagraph",
    ]

    private static func labelTarget(forKey key: String, inDocument text: String) -> LabelTarget? {
        let ns = text as NSString
        guard let labelAt = argumentOccurrence(of: "label", equalTo: key, in: ns) else { return nil }
        let line = lineNumber(of: labelAt.location, in: ns)
        // The innermost enclosing environment that names something.
        if let env = innermostLabelledEnvironment(in: ns, at: labelAt.location) {
            let kind = labelledEnvironments[env.name] ?? env.name.capitalized
            let caption = captionedEnvironments.contains(env.name)
                ? balancedArgument(of: "caption", in: ns, within: env.body).map(plainText)
                : nil
            return LabelTarget(kind: kind, text: caption, line: line, path: nil)
        }
        // Otherwise the heading it sits under.
        if let heading = precedingHeading(in: ns, before: labelAt.location) {
            return LabelTarget(kind: heading.kind, text: heading.title, line: line, path: nil)
        }
        return LabelTarget(kind: "Label", text: nil, line: line, path: nil)
    }

    // MARK: - bibliography

    /// A bibliography entry a `\cite` key resolves to.
    struct BibliographyEntry: Equatable {
        var key: String
        /// `article`, `book`, … for a BibTeX entry; nil for a `\bibitem`.
        var type: String?
        var author: String?
        var title: String?
        var year: String?
        /// Journal, book title or publisher — whichever the entry gives first.
        var source: String?
        /// The whole `\bibitem` text, when that is all there is.
        var raw: String?
        /// Project path of the file the entry came from.
        var path: String?

        /// `Knuth — The TeXbook (1984) · Addison-Wesley`, or the `\bibitem` text.
        var summary: String {
            var parts: [String] = []
            if let author, !author.isEmpty { parts.append(author) }
            if let title, !title.isEmpty { parts.append(title) }
            var head = parts.joined(separator: " — ")
            if let year, !year.isEmpty { head += head.isEmpty ? year : " (\(year))" }
            if let source, !source.isEmpty { head += head.isEmpty ? source : " · " + source }
            if head.isEmpty { head = raw ?? key }
            return head
        }
    }

    /// The entry for `key`: a `\bibitem` in the buffer first, then every other
    /// document (a `.bib` file is parsed as BibTeX, a `.tex` for `\bibitem`).
    static func bibliographyEntry(forKey key: String, in text: String, context: HoverContext = .init()) -> BibliographyEntry? {
        if let here = bibitem(forKey: key, in: text) { return here }
        for doc in context.otherDocuments {
            if var found = bibEntry(forKey: key, inBibTeX: doc.text) ?? bibitem(forKey: key, in: doc.text) {
                found.path = doc.path
                return found
            }
        }
        return nil
    }

    /// `\bibitem{key} Text…` or `\bibitem[label]{key} Text…` up to the next
    /// `\bibitem` or `\end{thebibliography}`.
    static func bibitem(forKey key: String, in text: String) -> BibliographyEntry? {
        let ns = text as NSString
        var search = NSRange(location: 0, length: ns.length)
        while true {
            let r = ns.range(of: "\\bibitem", options: .literal, range: search)
            guard r.location != NSNotFound else { return nil }
            search = NSRange(location: NSMaxRange(r), length: ns.length - NSMaxRange(r))
            var i = NSMaxRange(r)
            while i < ns.length, ns.character(at: i) == 0x20 || ns.character(at: i) == 0x09 { i += 1 }
            if i < ns.length, ns.character(at: i) == 0x5B { // `[label]`
                guard let close = scanBalanced(in: ns, from: i, open: 0x5B, close: 0x5D) else { continue }
                i = close + 1
                while i < ns.length, ns.character(at: i) == 0x20 || ns.character(at: i) == 0x09 { i += 1 }
            }
            guard i < ns.length, ns.character(at: i) == 0x7B, let close = scanBalanced(in: ns, from: i, open: 0x7B, close: 0x7D) else { continue }
            let found = ns.substring(with: NSRange(location: i + 1, length: close - i - 1)).trimmingCharacters(in: .whitespaces)
            guard found == key else { continue }
            // The entry text: to the next `\bibitem` or the end of the environment.
            var end = ns.length
            for stop in ["\\bibitem", "\\end{thebibliography}"] {
                let tail = NSRange(location: close + 1, length: ns.length - close - 1)
                let s = ns.range(of: stop, options: .literal, range: tail)
                if s.location != NSNotFound { end = min(end, s.location) }
            }
            let body = plainText(ns.substring(with: NSRange(location: close + 1, length: end - close - 1)))
            return BibliographyEntry(key: key, type: nil, author: nil, title: nil, year: nil, source: nil,
                                     raw: body.isEmpty ? nil : body, path: nil)
        }
    }

    /// The BibTeX entry `@type{key, field = {value}, …}`. Values may be braced,
    /// quoted or a bare word; nested braces are honoured.
    static func bibEntry(forKey key: String, inBibTeX text: String) -> BibliographyEntry? {
        let ns = text as NSString
        var i = 0
        while i < ns.length {
            guard ns.character(at: i) == 0x40 else { i += 1; continue } // `@`
            var j = i + 1
            while j < ns.length, isLetter(ns.character(at: j)) { j += 1 }
            let type = ns.substring(with: NSRange(location: i + 1, length: j - i - 1)).lowercased()
            while j < ns.length, isSpace(ns.character(at: j)) { j += 1 }
            guard j < ns.length, ns.character(at: j) == 0x7B, !["comment", "preamble", "string"].contains(type),
                  let entryEnd = scanBalanced(in: ns, from: j, open: 0x7B, close: 0x7D) else { i += 1; continue }
            var k = j + 1
            while k < ns.length, isSpace(ns.character(at: k)) { k += 1 }
            let keyStart = k
            while k < entryEnd, ns.character(at: k) != 0x2C { k += 1 } // `,`
            let found = ns.substring(with: NSRange(location: keyStart, length: k - keyStart)).trimmingCharacters(in: .whitespacesAndNewlines)
            guard found == key else { i = entryEnd + 1; continue }
            let fields = bibFields(in: ns, from: min(k + 1, entryEnd), to: entryEnd)
            return BibliographyEntry(
                key: key, type: type,
                author: fields["author"].map(plainText).map(shortenAuthors),
                title: fields["title"].map(plainText),
                year: fields["year"] ?? fields["date"].map { String($0.prefix(4)) },
                source: ["journal", "journaltitle", "booktitle", "publisher", "school", "institution", "howpublished"]
                    .compactMap { fields[$0] }.first.map(plainText),
                raw: nil, path: nil)
        }
        return nil
    }

    /// `name = value` pairs of one entry body.
    private static func bibFields(in ns: NSString, from start: Int, to end: Int) -> [String: String] {
        var out: [String: String] = [:]
        var i = start
        while i < end {
            while i < end, !isLetter(ns.character(at: i)) { i += 1 }
            let nameStart = i
            while i < end, isLetter(ns.character(at: i)) || ns.character(at: i) == 0x5F { i += 1 }
            guard i > nameStart else { break }
            let name = ns.substring(with: NSRange(location: nameStart, length: i - nameStart)).lowercased()
            while i < end, isSpace(ns.character(at: i)) { i += 1 }
            guard i < end, ns.character(at: i) == 0x3D else { continue } // `=`
            i += 1
            while i < end, isSpace(ns.character(at: i)) { i += 1 }
            guard i < end else { break }
            let c = ns.character(at: i)
            var value = ""
            if c == 0x7B, let close = scanBalanced(in: ns, from: i, open: 0x7B, close: 0x7D), close <= end {
                value = ns.substring(with: NSRange(location: i + 1, length: close - i - 1))
                i = close + 1
            } else if c == 0x22 { // `"`
                var j = i + 1
                while j < end, ns.character(at: j) != 0x22 { j += 1 }
                value = ns.substring(with: NSRange(location: i + 1, length: max(0, j - i - 1)))
                i = min(j + 1, end)
            } else {
                let s = i
                while i < end, ns.character(at: i) != 0x2C, !isSpace(ns.character(at: i)) { i += 1 }
                value = ns.substring(with: NSRange(location: s, length: i - s))
            }
            let collapsed = value.split(whereSeparator: { $0 == "\n" || $0 == "\r" || $0 == "\t" || $0 == " " }).joined(separator: " ")
            if out[name] == nil, !collapsed.isEmpty { out[name] = collapsed }
            while i < end, ns.character(at: i) != 0x2C { i += 1 }
            i += 1
        }
        return out
    }

    /// `Knuth, Donald E. and Lamport, Leslie` → `Knuth and Lamport`;
    /// three or more become `Knuth et al.`
    static func shortenAuthors(_ authors: String) -> String {
        let names = authors.components(separatedBy: " and ").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        guard !names.isEmpty else { return authors }
        func surname(_ name: String) -> String {
            if let comma = name.firstIndex(of: ",") { return String(name[..<comma]).trimmingCharacters(in: .whitespaces) }
            return name.split(separator: " ").last.map(String.init) ?? name
        }
        switch names.count {
        case 1: return surname(names[0])
        case 2: return surname(names[0]) + " and " + surname(names[1])
        default: return surname(names[0]) + " et al."
        }
    }

    // MARK: - graphics

    /// Where `\includegraphics{path}` actually reads from.
    struct GraphicsResolution: Equatable {
        /// The project-relative path that exists, when one does.
        var resolved: String?
        /// The paths tried, in order — shown when none of them exists.
        var candidates: [String]
        /// True when no file probe was possible (no project root).
        var unknown: Bool

        var summary: String {
            if unknown { return "Resolved against the project root when the document is saved." }
            if let resolved { return "Resolves to " + resolved }
            let tried = candidates.prefix(3).joined(separator: ", ")
            return "No file found — tried " + tried + (candidates.count > 3 ? ", …" : "")
        }
    }

    /// Extensions `\includegraphics` appends when the path has none, in the
    /// order graphicx tries them for pdflatex.
    static let graphicsExtensions = ["pdf", "png", "jpg", "jpeg", "eps"]

    /// `\graphicspath{{figures/}{img/}}` directories of a document, in order,
    /// plus the document's own directory (the empty prefix) last — which is
    /// where graphicx looks when `\graphicspath` is not set or misses.
    static func graphicsSearchPaths(in text: String) -> [String] {
        let ns = text as NSString
        guard let arg = balancedArgument(of: "graphicspath", in: ns, within: NSRange(location: 0, length: ns.length)) else { return [""] }
        var dirs: [String] = []
        let a = arg as NSString
        var i = 0
        while i < a.length {
            guard a.character(at: i) == 0x7B, let close = scanBalanced(in: a, from: i, open: 0x7B, close: 0x7D) else { i += 1; continue }
            var dir = a.substring(with: NSRange(location: i + 1, length: close - i - 1)).trimmingCharacters(in: .whitespacesAndNewlines)
            if !dir.isEmpty, !dir.hasSuffix("/") { dir += "/" }
            if !dir.isEmpty { dirs.append(dir) }
            i = close + 1
        }
        return dirs.isEmpty ? [""] : dirs + [""]
    }

    /// Every path `\includegraphics{path}` would try, in graphicx's order.
    static func graphicsCandidates(for path: String, in text: String) -> [String] {
        let trimmed = path.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return [] }
        let hasExtension = graphicsExtensions.contains((trimmed as NSString).pathExtension.lowercased())
        let roots = trimmed.hasPrefix("/") ? [""] : graphicsSearchPaths(in: text)
        var out: [String] = []
        for root in roots {
            if hasExtension {
                out.append(root + trimmed)
            } else {
                for ext in graphicsExtensions { out.append(root + trimmed + "." + ext) }
            }
        }
        return out
    }

    static func resolveGraphics(_ path: String, in text: String, context: HoverContext = .init()) -> GraphicsResolution {
        let candidates = graphicsCandidates(for: path, in: text)
        guard context.canProbeFiles else { return GraphicsResolution(resolved: nil, candidates: candidates, unknown: true) }
        let hit = candidates.first(where: context.fileExists)
        return GraphicsResolution(resolved: hit, candidates: candidates, unknown: false)
    }

    // MARK: - scanning helpers

    private static func isLetter(_ c: unichar) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
    private static func isSpace(_ c: unichar) -> Bool { c == 0x20 || c == 0x09 || c == 0x0A || c == 0x0D }

    /// Index of the closer matching the opener at `from`, honouring nesting
    /// and `\{` escapes; nil when unbalanced.
    static func scanBalanced(in ns: NSString, from: Int, open: unichar, close: unichar) -> Int? {
        guard from < ns.length, ns.character(at: from) == open else { return nil }
        var depth = 0
        var i = from
        while i < ns.length {
            let c = ns.character(at: i)
            if c == 0x5C { i += 2; continue } // `\x` is one escaped token
            if c == open { depth += 1 } else if c == close { depth -= 1; if depth == 0 { return i } }
            i += 1
        }
        return nil
    }

    /// Range of the `\name{…}` whose argument is exactly `value`, or nil.
    /// The range spans the backslash through the closing brace.
    private static func argumentOccurrence(of name: String, equalTo value: String, in ns: NSString) -> NSRange? {
        var search = NSRange(location: 0, length: ns.length)
        let needle = "\\" + name + "{"
        while true {
            let r = ns.range(of: needle, options: .literal, range: search)
            guard r.location != NSNotFound else { return nil }
            search = NSRange(location: NSMaxRange(r), length: ns.length - NSMaxRange(r))
            // `\mylabel{…}` must not match `\label{…}`: the byte before the
            // backslash may not be a letter, and the backslash not escaped.
            let before = r.location - 1
            if before >= 0, ns.character(at: before) == 0x5C { continue }
            guard let close = scanBalanced(in: ns, from: NSMaxRange(r) - 1, open: 0x7B, close: 0x7D) else { continue }
            let arg = ns.substring(with: NSRange(location: NSMaxRange(r), length: close - NSMaxRange(r)))
            guard arg.trimmingCharacters(in: .whitespacesAndNewlines) == value else { continue }
            return NSRange(location: r.location, length: close - r.location + 1)
        }
    }

    /// The argument of the first `\name{…}` inside `within`, markup intact.
    static func balancedArgument(of name: String, in ns: NSString, within: NSRange) -> String? {
        let needle = "\\" + name + "{"
        var search = within
        while true {
            let r = ns.range(of: needle, options: .literal, range: search)
            guard r.location != NSNotFound else { return nil }
            guard let close = scanBalanced(in: ns, from: NSMaxRange(r) - 1, open: 0x7B, close: 0x7D), close < NSMaxRange(within) else {
                search = NSRange(location: NSMaxRange(r), length: NSMaxRange(within) - NSMaxRange(r))
                if search.length <= 0 { return nil }
                continue
            }
            return ns.substring(with: NSRange(location: NSMaxRange(r), length: close - NSMaxRange(r)))
        }
    }

    /// The innermost `\begin{env}` open at `utf16` whose name is in
    /// `labelledEnvironments`, with the body range of that environment.
    private static func innermostLabelledEnvironment(in ns: NSString, at utf16: Int) -> (name: String, body: NSRange)? {
        var stack: [(name: String, bodyStart: Int)] = []
        var i = 0
        while i < utf16 {
            let c = ns.character(at: i)
            if c == 0x25 { // `%`: skip the comment
                while i < ns.length, ns.character(at: i) != 0x0A { i += 1 }
                continue
            }
            guard c == 0x5C else { i += 1; continue }
            var j = i + 1
            while j < ns.length, isLetter(ns.character(at: j)) { j += 1 }
            let word = ns.substring(with: NSRange(location: i + 1, length: j - i - 1))
            guard word == "begin" || word == "end", j < ns.length, ns.character(at: j) == 0x7B,
                  let close = scanBalanced(in: ns, from: j, open: 0x7B, close: 0x7D) else {
                i = max(j, i + 2)
                continue
            }
            let env = ns.substring(with: NSRange(location: j + 1, length: close - j - 1)).trimmingCharacters(in: .whitespaces)
            if word == "begin" { stack.append((env, close + 1)) } else if stack.last?.name == env { stack.removeLast() }
            i = close + 1
        }
        guard let top = stack.last(where: { labelledEnvironments[$0.name] != nil }) else { return nil }
        // The body runs to this environment's `\end`, or to the end of the text.
        let tail = NSRange(location: top.bodyStart, length: ns.length - top.bodyStart)
        let end = ns.range(of: "\\end{" + top.name + "}", options: .literal, range: tail)
        let bodyEnd = end.location == NSNotFound ? ns.length : end.location
        return (top.name, NSRange(location: top.bodyStart, length: max(0, bodyEnd - top.bodyStart)))
    }

    /// The last sectioning command before `utf16`, with its title.
    private static func precedingHeading(in ns: NSString, before utf16: Int) -> (kind: String, title: String)? {
        var best: (kind: String, title: String)?
        var i = 0
        while i < utf16 {
            let c = ns.character(at: i)
            if c == 0x25 {
                while i < ns.length, ns.character(at: i) != 0x0A { i += 1 }
                continue
            }
            guard c == 0x5C else { i += 1; continue }
            var j = i + 1
            while j < ns.length, isLetter(ns.character(at: j)) { j += 1 }
            var word = ns.substring(with: NSRange(location: i + 1, length: j - i - 1))
            var starred = false
            if j < ns.length, ns.character(at: j) == 0x2A { starred = true; j += 1 } // `\section*`
            guard let kind = sectioningCommands[word] else { i = max(j, i + 2); continue }
            var k = j
            if k < ns.length, ns.character(at: k) == 0x5B, let close = scanBalanced(in: ns, from: k, open: 0x5B, close: 0x5D) { k = close + 1 }
            guard k < ns.length, ns.character(at: k) == 0x7B, let close = scanBalanced(in: ns, from: k, open: 0x7B, close: 0x7D) else {
                i = max(j, i + 2); continue
            }
            word = ns.substring(with: NSRange(location: k + 1, length: close - k - 1))
            best = (starred ? kind + " (unnumbered)" : kind, plainText(word))
            i = close + 1
        }
        return best
    }

    static func lineNumber(of utf16: Int, in ns: NSString) -> Int {
        var line = 1
        var i = 0
        while i < min(utf16, ns.length) {
            if ns.character(at: i) == 0x0A { line += 1 }
            i += 1
        }
        return line
    }

    /// LaTeX markup stripped for display: `\emph{Best} \(x\)` → `Best x`.
    /// A command's braced argument is kept (that is the text); the command
    /// name, braces, `$` and runs of whitespace go.
    static func plainText(_ s: String) -> String {
        var out = ""
        var i = s.startIndex
        while i < s.endIndex {
            let c = s[i]
            if c == "\\" {
                var j = s.index(after: i)
                while j < s.endIndex, s[j].isLetter { j = s.index(after: j) }
                if j == s.index(after: i), j < s.endIndex {
                    out.append(s[j]) // `\&`, `\%`: drop the escape, keep the character
                    j = s.index(after: j)
                } else if j < s.endIndex, s[j] == " " {
                    j = s.index(after: j) // a control word swallows the space after it
                }
                i = j // the control word itself is dropped; its `{argument}` is kept
                continue
            }
            if c == "{" || c == "}" || c == "$" { i = s.index(after: i); continue }
            out.append(c)
            i = s.index(after: i)
        }
        return out.split(whereSeparator: { $0 == " " || $0 == "\n" || $0 == "\t" || $0 == "\r" })
            .joined(separator: " ")
    }
}
