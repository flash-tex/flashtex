import Foundation

/// Paste an image into the LaTeX editor: the pure half (Foundation only, so
/// the iPad can share it). The Mac's `PasteImage.swift` reads the pasteboard
/// and writes the file; everything that decides *what text* goes where lives
/// here and is unit-tested without AppKit:
///
/// - the saved file's name (`baseName(for:)`, `uniqueFileName`) and folder
///   (`imageFolder`, honouring the document's `\graphicspath`);
/// - whether the root document already loads graphicx, and where
///   `\usepackage{graphicx}` goes when it does not (`graphicxInsertion`);
/// - what kind of place the caret is in (`context`): inside a figure-like
///   environment or math the insertion is a bare `\includegraphics`,
///   elsewhere a `figure` environment on lines of its own;
/// - the snippet and the caption placeholder it selects (`snippet`), and the
///   whole edit as one list of replacements in the original text's
///   coordinates (`plan`), which the editor applies as one undo group.
///
/// All offsets are UTF-16 (`NSString`), like the rest of the editor core.
public enum PasteImageFigure {
    // MARK: options

    public struct Options: Equatable, Sendable {
        /// Project-relative folder the image is saved in (`figures`).
        public var folder: String
        /// `\includegraphics` width, e.g. `0.8\linewidth`. Empty: no option.
        /// A value containing `=` is used as the whole option list
        /// (`height=4cm,keepaspectratio`).
        public var width: String
        /// Wrap the image in a `figure` environment with a caption and label
        /// (where the caret allows one; `context`).
        public var wrapInFigure: Bool
        /// The caption text that is inserted and selected.
        public var captionPlaceholder: String
        /// One indentation step of the figure's body.
        public var indentUnit: String

        public static let defaultFolder = "figures"
        public static let defaultWidth = "0.8\\linewidth"

        public init(folder: String = Options.defaultFolder, width: String = Options.defaultWidth,
                    wrapInFigure: Bool = true, captionPlaceholder: String = "Caption", indentUnit: String = "  ") {
            self.folder = folder
            self.width = width
            self.wrapInFigure = wrapInFigure
            self.captionPlaceholder = captionPlaceholder
            self.indentUnit = indentUnit
        }

        /// The option list `\includegraphics[…]` gets (without brackets); nil for none.
        public var graphicsOptions: String? {
            let w = width.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !w.isEmpty else { return nil }
            return w.contains("=") ? w : "width=" + w
        }
    }

    // MARK: file naming

    /// `pasted-YYYYMMDD-HHMMSS` for `date` in `timeZone`.
    public static func baseName(for date: Date, timeZone: TimeZone = .current) -> String {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = timeZone
        let c = calendar.dateComponents([.year, .month, .day, .hour, .minute, .second], from: date)
        func two(_ v: Int?) -> String { let v = v ?? 0; return v < 10 ? "0\(v)" : "\(v)" }
        return "pasted-\(c.year ?? 0)\(two(c.month))\(two(c.day))-\(two(c.hour))\(two(c.minute))\(two(c.second))"
    }

    /// A file stem LaTeX reads without trouble: ASCII letters, digits, `-`
    /// and `_`; everything else (spaces, dots, accents, TeX specials) becomes
    /// one `-`. Empty results fall back to `pasted`. graphicx treats a second
    /// dot as the start of the extension, so dots never survive.
    public static func sanitizedBaseName(_ original: String) -> String {
        var out = ""
        var lastDash = false
        for scalar in original.unicodeScalars {
            let v = scalar.value
            let keep = (v >= 0x30 && v <= 0x39) || (v >= 0x41 && v <= 0x5A) || (v >= 0x61 && v <= 0x7A) || v == 0x5F || v == 0x2D
            if keep && v != 0x2D {
                out.unicodeScalars.append(scalar)
                lastDash = false
            } else if !lastDash, !out.isEmpty {
                out.append("-")
                lastDash = true
            }
        }
        while out.hasSuffix("-") { out.removeLast() }
        return out.isEmpty ? "pasted" : out
    }

    /// `base.ext`, or `base-2.ext`, `base-3.ext`, … — the first name
    /// `exists` says is free. Never overwrites: after 9 999 taken names it
    /// appends a UUID fragment.
    public static func uniqueFileName(base: String, fileExtension ext: String, exists: (String) -> Bool) -> String {
        let first = base + "." + ext
        if !exists(first) { return first }
        for n in 2..<10_000 {
            let name = "\(base)-\(n).\(ext)"
            if !exists(name) { return name }
        }
        return "\(base)-\(UUID().uuidString.prefix(8).lowercased()).\(ext)"
    }

    /// A project-relative folder as it is written into `\includegraphics`:
    /// no leading `./`, no trailing `/`. `""` is the project root. Nil for a
    /// folder that is not safely relative (absolute, `~`, a `..` component)
    /// or that TeX would misread (spaces or special characters).
    public static func normalizedFolder(_ raw: String) -> String? {
        var f = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        while f.hasPrefix("./") { f.removeFirst(2) }
        while f.hasSuffix("/") { f.removeLast() }
        if f == "." { return "" }
        guard !f.hasPrefix("/"), !f.hasPrefix("~") else { return nil }
        let bad = CharacterSet(charactersIn: " \t%#{}\\$&^~\"'")
        guard f.rangeOfCharacter(from: bad) == nil else { return nil }
        let components = f.split(separator: "/", omittingEmptySubsequences: false)
        guard !components.contains(where: { $0 == ".." || ($0.isEmpty && !f.isEmpty) }) else { return nil }
        return f
    }

    /// The `\graphicspath{{a/}{b/}}` directories of `text` (comments
    /// ignored), normalized; directories that are not safely project-relative
    /// are dropped.
    public static func graphicsPaths(in text: String) -> [String] {
        let ns = uncommented(text as NSString)
        let r = ns.range(of: "\\graphicspath")
        guard r.location != NSNotFound else { return [] }
        var i = NSMaxRange(r)
        while i < ns.length, isSpace(ns.character(at: i)) { i += 1 }
        guard i < ns.length, ns.character(at: i) == 0x7B, let close = matchingBrace(in: ns, from: i) else { return [] }
        var dirs: [String] = []
        var j = i + 1
        while j < close {
            if ns.character(at: j) == 0x7B, let inner = matchingBrace(in: ns, from: j), inner <= close {
                let dir = ns.substring(with: NSRange(location: j + 1, length: inner - j - 1))
                if let d = normalizedFolder(dir) { dirs.append(d) }
                j = inner + 1
            } else {
                j += 1
            }
        }
        return dirs
    }

    /// Where the pasted image is saved: the configured folder, unless the
    /// root document declares a `\graphicspath` that does not list it — then
    /// the first of those directories, so the document's own convention wins.
    /// An invalid configured folder falls back to `figures`.
    public static func imageFolder(configured: String, rootText: String) -> String {
        let folder = normalizedFolder(configured) ?? Options.defaultFolder
        let declared = graphicsPaths(in: rootText)
        if declared.isEmpty || declared.contains(folder) { return folder }
        return declared[0]
    }

    /// The `\includegraphics` path for `fileName` in `folder`.
    public static func graphicsPath(folder: String, fileName: String) -> String {
        folder.isEmpty ? fileName : folder + "/" + fileName
    }

    // MARK: graphicx in the preamble

    public struct TextInsertion: Equatable, Sendable {
        public var location: Int
        public var text: String
        public init(location: Int, text: String) { self.location = location; self.text = text }
    }

    /// Whether the preamble of `text` loads graphicx: `\usepackage` or
    /// `\RequirePackage` listing it (comments ignored), or a class that loads
    /// it itself (beamer).
    public static func loadsGraphicx(in text: String) -> Bool {
        let preamble = preambleText(of: text)
        for m in matches(of: packagePattern, in: preamble) {
            let list = preamble.substring(with: m.range(at: 1))
            if list.split(separator: ",").contains(where: { $0.trimmingCharacters(in: .whitespacesAndNewlines) == "graphicx" }) { return true }
        }
        if let cls = matches(of: classPattern, in: preamble).first {
            let name = preamble.substring(with: cls.range(at: 1)).trimmingCharacters(in: .whitespacesAndNewlines)
            if classesLoadingGraphicx.contains(name) { return true }
        }
        return false
    }

    /// Classes that load graphicx themselves.
    public static let classesLoadingGraphicx: Set<String> = ["beamer"]

    /// Where `\usepackage{graphicx}` goes in `text`: on its own line after
    /// the last `\usepackage` of the preamble, else after `\documentclass`.
    /// Nil when graphicx is already loaded, or when `text` has no
    /// `\documentclass` (an included file: its preamble lives elsewhere).
    public static func graphicxInsertion(in text: String) -> TextInsertion? {
        guard !loadsGraphicx(in: text) else { return nil }
        let preamble = preambleText(of: text)
        let anchor = matches(of: usepackagePattern, in: preamble).last ?? matches(of: classPattern, in: preamble).first
        guard let anchor else { return nil }
        let ns = text as NSString
        let end = NSMaxRange(anchor.range)
        let line = ns.lineRange(for: NSRange(location: max(end - 1, 0), length: 0))
        let lineEnd = NSMaxRange(line)
        let endsWithNewline = lineEnd > 0 && lineEnd <= ns.length && isNewline(ns.character(at: lineEnd - 1))
        return TextInsertion(location: lineEnd, text: endsWithNewline ? "\\usepackage{graphicx}\n" : "\n\\usepackage{graphicx}")
    }

    // MARK: caret context

    public enum Placement: Equatable, Sendable {
        /// A `figure` environment on lines of its own.
        case figure
        /// A bare `\includegraphics` at the caret.
        case bare
    }

    public struct Context: Equatable, Sendable {
        /// Inside `$…$`, `\[…\]` or a math environment.
        public var inMath: Bool
        /// Inside a figure-like environment (`figureEnvironments`).
        public var inFigure: Bool
        /// Non-blank text before / after the caret on its line.
        public var textBefore: Bool
        public var textAfter: Bool
        /// The caret line's leading whitespace.
        public var indent: String

        public init(inMath: Bool = false, inFigure: Bool = false, textBefore: Bool = false, textAfter: Bool = false, indent: String = "") {
            self.inMath = inMath
            self.inFigure = inFigure
            self.textBefore = textBefore
            self.textAfter = textAfter
            self.indent = indent
        }

        public func placement(_ options: Options) -> Placement {
            options.wrapInFigure && !inMath && !inFigure ? .figure : .bare
        }
    }

    /// Environments in which a pasted image is a bare `\includegraphics`: a
    /// float or box that already places it (a figure inside a figure is an
    /// error; inside a table, minipage or tabular cell it is the image that is
    /// wanted, not another float).
    public static let figureEnvironments: Set<String> = [
        "figure", "figure*", "subfigure", "wrapfigure", "SCfigure", "sidewaysfigure", "marginfigure",
        "table", "table*", "subtable", "wraptable", "minipage", "center",
        "tabular", "tabular*", "tabularx", "tabulary", "longtable",
    ]

    /// What kind of place `caret` is in. `mathMode` is the editor's own
    /// answer when it has one (its in-sync `SyntaxHighlighter`); nil lexes
    /// `text` whole with the same highlighter.
    public static func context(in text: String, caret: Int, mathMode: Bool? = nil) -> Context {
        let ns = text as NSString
        let c = min(max(caret, 0), ns.length)
        var ctx = Context()
        if let m = mathMode {
            ctx.inMath = m
        } else if ns.length > 0 {
            var h = SyntaxHighlighter()
            h.reset(ns)
            ctx.inMath = h.mode(at: c, text: ns).isMath
        }
        if let byte = LaTeXEditing.utf8Offset(of: c, in: text) {
            ctx.inFigure = LaTeXEditing.openEnvironments(in: text, beforeByte: byte).contains { figureEnvironments.contains($0.name) }
        }
        let line = ns.lineRange(for: NSRange(location: c, length: 0))
        var contentEnd = NSMaxRange(line)
        while contentEnd > line.location, isNewline(ns.character(at: contentEnd - 1)) { contentEnd -= 1 }
        var indentEnd = line.location
        while indentEnd < contentEnd, isSpace(ns.character(at: indentEnd)) { indentEnd += 1 }
        ctx.indent = ns.substring(with: NSRange(location: line.location, length: indentEnd - line.location))
        ctx.textBefore = c > indentEnd
        var k = max(c, line.location)
        while k < contentEnd, isSpace(ns.character(at: k)) { k += 1 }
        ctx.textAfter = k < contentEnd
        return ctx
    }

    // MARK: snippet

    public struct Snippet: Equatable, Sendable {
        public var text: String
        /// What to select afterwards, relative to the snippet's start: the
        /// caption placeholder in a figure, else an empty range at the end.
        public var selection: NSRange
    }

    /// The text inserted at the caret for `path` (project-relative) with
    /// `label` (`fig:<label>`), placed per `context`.
    public static func snippet(path: String, label: String, options: Options, context: Context) -> Snippet {
        let graphic = "\\includegraphics" + (options.graphicsOptions.map { "[\($0)]" } ?? "") + "{\(path)}"
        guard context.placement(options) == .figure else {
            let n = (graphic as NSString).length
            return Snippet(text: graphic, selection: NSRange(location: n, length: 0))
        }
        let body = context.indent + options.indentUnit
        var text = context.textBefore ? "\n" + context.indent : ""
        text += "\\begin{figure}[htbp]\n"
        text += body + "\\centering\n"
        text += body + graphic + "\n"
        text += body + "\\caption{"
        let captionStart = (text as NSString).length
        text += options.captionPlaceholder + "}\n"
        text += body + "\\label{fig:\(label)}\n"
        text += context.indent + "\\end{figure}"
        if context.textAfter { text += "\n" + context.indent }
        return Snippet(text: text, selection: NSRange(location: captionStart, length: (options.captionPlaceholder as NSString).length))
    }

    // MARK: the whole edit

    public struct Plan: Equatable, Sendable {
        /// Replacements in the original text's coordinates (apply last-first).
        public var edits: [LaTeXEditing.LineEdit]
        /// The selection after every edit is applied.
        public var selection: NSRange
        public var placement: Placement
        /// Whether `edits` include `\usepackage{graphicx}`.
        public var addsGraphicx: Bool
    }

    /// The paste as edits of `text`: the snippet replacing `selection`, and —
    /// when `ensureGraphicx` (the buffer is the root document) — the graphicx
    /// line in its preamble. A graphicx insertion point inside the replaced
    /// selection is skipped (`addsGraphicx` false) rather than edited twice.
    public static func plan(text: String, selection: NSRange, path: String, label: String, options: Options,
                            mathMode: Bool? = nil, ensureGraphicx: Bool) -> Plan {
        let ns = text as NSString
        let start = min(max(selection.location, 0), ns.length)
        let replaced = NSRange(location: start, length: min(max(selection.length, 0), ns.length - start))
        let ctx = context(in: text, caret: start, mathMode: mathMode)
        let snip = snippet(path: path, label: label, options: options, context: ctx)
        var edits = [LaTeXEditing.LineEdit(range: replaced, replacement: snip.text)]
        var shift = 0
        var adds = false
        if ensureGraphicx, let g = graphicxInsertion(in: text),
           g.location <= replaced.location || g.location >= NSMaxRange(replaced) {
            if g.location == replaced.location {
                // Same start: one replacement, so the order two edits at one
                // offset would be applied in never matters.
                edits[0].replacement = g.text + snip.text
            } else {
                edits.append(LaTeXEditing.LineEdit(range: NSRange(location: g.location, length: 0), replacement: g.text))
            }
            if g.location <= replaced.location { shift = (g.text as NSString).length }
            adds = true
        }
        let sel = NSRange(location: replaced.location + shift + snip.selection.location, length: snip.selection.length)
        return Plan(edits: edits, selection: sel, placement: ctx.placement(options), addsGraphicx: adds)
    }

    // MARK: scanning helpers

    private static let packagePattern = regex("\\\\(?:usepackage|RequirePackage)\\s*(?:\\[[^\\]]*\\])?\\s*\\{([^}]*)\\}")
    private static let usepackagePattern = regex("\\\\usepackage\\s*(?:\\[[^\\]]*\\])?\\s*\\{[^}]*\\}")
    private static let classPattern = regex("\\\\documentclass\\s*(?:\\[[^\\]]*\\])?\\s*\\{([^}]*)\\}")

    private static func regex(_ pattern: String) -> NSRegularExpression {
        // The patterns are constants; a failure is a programming error caught by the tests.
        try! NSRegularExpression(pattern: pattern)
    }

    private static func matches(of re: NSRegularExpression, in ns: NSString) -> [NSTextCheckingResult] {
        re.matches(in: ns as String, range: NSRange(location: 0, length: ns.length))
    }

    /// `text` with comments blanked and everything from the first
    /// `\begin{document}` on cut; offsets are unchanged up to the cut.
    static func preambleText(of text: String) -> NSString {
        let ns = uncommented(text as NSString)
        let r = ns.range(of: "\\begin{document}")
        return r.location == NSNotFound ? ns : ns.substring(to: r.location) as NSString
    }

    /// `ns` with every comment (an unescaped `%` to its line end) replaced by
    /// spaces, so UTF-16 offsets still line up with the original.
    static func uncommented(_ ns: NSString) -> NSString {
        let n = ns.length
        guard n > 0, ns.range(of: "%").location != NSNotFound else { return ns }
        var units = [unichar](repeating: 0, count: n)
        ns.getCharacters(&units, range: NSRange(location: 0, length: n))
        var i = 0
        var inComment = false
        var backslashes = 0
        while i < n {
            let u = units[i]
            if inComment {
                if isNewline(u) { inComment = false } else { units[i] = 0x20 }
            } else if u == 0x25, backslashes % 2 == 0 {
                inComment = true
                units[i] = 0x20
            }
            backslashes = (!inComment && u == 0x5C) ? backslashes + 1 : 0
            i += 1
        }
        return NSString(characters: units, length: n)
    }

    private static func matchingBrace(in ns: NSString, from open: Int) -> Int? {
        var depth = 0
        var i = open
        while i < ns.length {
            let u = ns.character(at: i)
            if u == 0x5C { i += 2; continue }
            if u == 0x7B { depth += 1 }
            else if u == 0x7D { depth -= 1; if depth == 0 { return i } }
            i += 1
        }
        return nil
    }

    @inline(__always) static func isSpace(_ u: unichar) -> Bool { u == 0x20 || u == 0x09 }
    @inline(__always) static func isNewline(_ u: unichar) -> Bool { u == 0x0A || u == 0x0D }
}
