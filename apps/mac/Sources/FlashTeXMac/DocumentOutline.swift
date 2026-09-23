import Foundation
import FlashTeXProtocol

/// The sidebar's Outline: a cheap lexical scan of one buffer for sectioning
/// commands, `\begin{…}` environments and `\label{…}`s, in document order.
/// Nothing here parses TeX — it is the same bounded regex approach the
/// accessibility rotor uses (`AccessibleEditorModel.rotorItems`), extended
/// with labels and a nesting level so the sidebar can indent sections.
/// Comments are skipped (a `%` that is not `\%` ends the scanned part of a line).
enum DocumentOutline {
    enum Kind: String, Equatable, CaseIterable {
        case section, environment, label

        var title: String {
            switch self {
            case .section: return "Sections"
            case .environment: return "Environments"
            case .label: return "Labels"
            }
        }
    }

    struct Item: Equatable, Identifiable {
        var kind: Kind
        /// Section command name (`section`, `subsection`…), environment name, or "label".
        var command: String
        /// The braced argument: heading text, environment name, or the label key.
        var title: String
        /// Indentation level: chapter 0, section 1, subsection 2, …; environments nest by their `\begin` depth; labels 0.
        var level: Int
        /// UTF-16 range of the whole command (selecting it reveals the item in the editor).
        var utf16: NSRange
        /// 1-based line of the command's start.
        var line: Int
        /// For a float or theorem-like environment: its `\caption{…}`, the
        /// `\begin{thm}[title]` optional title, or the first words of its body.
        var caption: String? = nil

        var id: String { "\(kind.rawValue):\(utf16.location)" }
        /// Sidebar text: the caption when there is one ("figure: Loss curves").
        var displayTitle: String { caption.map { "\(title): \($0)" } ?? title }
    }

    /// Environments whose caption/title is shown in the outline: floats and
    /// the standard theorem-like names (plus every `\newtheorem{name}` of the buffer).
    static let floatEnvironments: Set<String> = ["figure", "figure*", "table", "table*", "algorithm", "listing", "wrapfigure", "wraptable", "subfigure"]
    static let theoremEnvironments: Set<String> = [
        "theorem", "lemma", "proposition", "corollary", "definition", "remark", "example", "proof", "claim", "conjecture",
        "exercise", "problem", "solution", "axiom", "notation", "assumption", "fact", "observation",
    ]

    /// The current item for follow-caret highlighting: the last sectioning
    /// command at or before `caret`, else the innermost environment whose
    /// `\begin` is at or before it.
    static func current(at caret: Int, in items: [Item]) -> Item? {
        var section: Item?
        for i in items where i.utf16.location <= caret {
            if i.kind == .section { section = i }
        }
        return section ?? items.last { $0.kind == .environment && $0.utf16.location <= caret }
    }

    /// The status bar's LaTeX-semantic breadcrumb (design-principles §5):
    /// the enclosing sectioning chain at `caret`, outermost first —
    /// `Chapter 2 › 2.3 Experimental Setup`. Ancestors are the nearest
    /// preceding sections of strictly lower level; environments and labels
    /// are left out because the lexical scan records no `\end` positions,
    /// and a segment that may already be closed would lie.
    static func breadcrumb(at caret: Int, in items: [Item]) -> [Item] {
        var innermost: Item?
        for i in items where i.utf16.location <= caret {
            if i.kind == .section { innermost = i }
        }
        guard let last = innermost else { return [] }
        var chain = [last]
        var level = last.level
        for i in items.reversed() where i.kind == .section && i.utf16.location < last.utf16.location {
            if i.level < level { chain.insert(i, at: 0); level = i.level }
        }
        return chain
    }

    /// Sectioning commands and their level (chapter 0 … paragraph 4).
    static let sectionLevels: [String: Int] = [
        "part": 0, "chapter": 0, "section": 1, "subsection": 2, "subsubsection": 3, "paragraph": 4, "subparagraph": 5,
    ]

    /// Documents longer than this are not scanned for the outline (the scan
    /// is linear, but the sidebar re-scans after edits settle; 2 MiB keeps
    /// every rescan well under a frame on the bench machine).
    static let maxScannedUTF16 = 2 * 1024 * 1024

    private static let pattern: NSRegularExpression = {
        // 1: sectioning name, 2: its argument; 3: begin env; 4: label key.
        let sections = sectionLevels.keys.sorted().joined(separator: "|")
        return try! NSRegularExpression(pattern:
            "\\\\(\(sections))\\*?(?:\\[[^\\]\\n]*\\])?\\{([^{}\\n]*)\\}"
            + "|\\\\begin\\{([A-Za-z*]+)\\}"
            + "|\\\\label\\{([^{}\\n]*)\\}"
            + "|\\\\end\\{([A-Za-z*]+)\\}")
    }()

    /// All items of `text` in document order.
    static func scan(_ text: String) -> [Item] {
        let ns = text as NSString
        guard ns.length <= maxScannedUTF16 else { return [] }
        var out: [Item] = []
        var lineStarts: [Int] = [0]
        var envDepth = 0
        // Line starts once, so each item's line is a binary search.
        ns.enumerateSubstrings(in: NSRange(location: 0, length: ns.length), options: [.byLines, .substringNotRequired]) { _, _, enclosing, _ in
            let next = enclosing.location + enclosing.length
            if next < ns.length { lineStarts.append(next) }
        }
        var searchRanges: [NSRange] = []
        // Strip comments: scan each line only up to an unescaped `%`.
        var cursor = 0
        for (i, start) in lineStarts.enumerated() {
            let end = i + 1 < lineStarts.count ? lineStarts[i + 1] : ns.length
            let line = ns.substring(with: NSRange(location: start, length: end - start))
            var stop = line.utf16.count
            var prevBackslash = false
            for (k, u) in line.utf16.enumerated() {
                if u == 0x25, !prevBackslash { stop = k; break } // '%'
                prevBackslash = (u == 0x5C) && !prevBackslash // '\'
            }
            searchRanges.append(NSRange(location: start, length: stop))
            cursor = end
        }
        _ = cursor
        for range in searchRanges where range.length > 0 {
            for m in pattern.matches(in: text, range: range) {
                func group(_ i: Int) -> String? { m.range(at: i).location == NSNotFound ? nil : ns.substring(with: m.range(at: i)) }
                let line = (lineStarts.lastIndex { $0 <= m.range.location } ?? 0) + 1
                if let name = group(1), let arg = group(2) {
                    out.append(Item(kind: .section, command: name, title: arg.trimmingCharacters(in: .whitespaces),
                                    level: sectionLevels[name] ?? 1, utf16: m.range, line: line))
                } else if let env = group(3) {
                    // The document environment is the page itself, not an outline entry.
                    if env != "document" {
                        out.append(Item(kind: .environment, command: "begin", title: env, level: envDepth, utf16: m.range, line: line))
                        envDepth += 1
                    }
                } else if let key = group(4) {
                    out.append(Item(kind: .label, command: "label", title: key, level: 0, utf16: m.range, line: line))
                } else if let env = group(5), env != "document" {
                    envDepth = max(0, envDepth - 1)
                }
            }
        }
        annotateCaptions(&out, text: ns)
        return out
    }

    /// Fills `caption` for floats and theorem-like environments: the first
    /// `\caption{…}` before the matching `\end`, else the `[title]` right
    /// after `\begin{…}`, else the body's first words (≤ 60 characters).
    static func annotateCaptions(_ items: inout [Item], text ns: NSString) {
        var theoremNames = theoremEnvironments
        for m in newtheoremPattern.matches(in: ns as String, range: NSRange(location: 0, length: ns.length)) {
            theoremNames.insert(ns.substring(with: m.range(at: 1)))
        }
        for i in items.indices where items[i].kind == .environment {
            let name = items[i].title
            let isFloat = floatEnvironments.contains(name), isTheorem = theoremNames.contains(name)
            guard isFloat || isTheorem else { continue }
            let start = NSMaxRange(items[i].utf16)
            let endMarker = ns.range(of: "\\end{\(name)}", options: .literal, range: NSRange(location: start, length: ns.length - start))
            let bodyEnd = endMarker.location == NSNotFound ? ns.length : endMarker.location
            let body = NSRange(location: start, length: bodyEnd - start)
            if isFloat {
                let cap = ns.range(of: "\\caption{", options: .literal, range: body)
                guard cap.location != NSNotFound, let arg = bracedArgument(in: ns, openBrace: NSMaxRange(cap) - 1) else { continue }
                items[i].caption = summarize(arg)
            } else {
                if start < ns.length, ns.character(at: start) == 0x5B, // `[title]`
                   case let close = ns.range(of: "]", options: .literal, range: NSRange(location: start, length: bodyEnd - start)), close.location != NSNotFound {
                    items[i].caption = summarize(ns.substring(with: NSRange(location: start + 1, length: close.location - start - 1)))
                } else {
                    let words = ns.substring(with: body).trimmingCharacters(in: .whitespacesAndNewlines)
                    if !words.isEmpty { items[i].caption = summarize(words) }
                }
            }
        }
    }

    private static let newtheoremPattern = try! NSRegularExpression(pattern: "\\\\newtheorem\\*?\\{([A-Za-z*]+)\\}")

    /// The balanced `{…}` argument starting at `openBrace`, or nil when unclosed.
    static func bracedArgument(in ns: NSString, openBrace: Int) -> String? {
        guard openBrace < ns.length, ns.character(at: openBrace) == 0x7B else { return nil }
        var depth = 0
        var i = openBrace
        while i < ns.length {
            let c = ns.character(at: i)
            if c == 0x5C { i += 2; continue }
            if c == 0x7B { depth += 1 } else if c == 0x7D { depth -= 1; if depth == 0 { return ns.substring(with: NSRange(location: openBrace + 1, length: i - openBrace - 1)) } }
            i += 1
        }
        return nil
    }

    /// One line, whitespace collapsed, ≤ 60 characters with an ellipsis.
    static func summarize(_ s: String) -> String {
        let flat = s.split(whereSeparator: \.isWhitespace).joined(separator: " ")
        return flat.count > 60 ? String(flat.prefix(59)) + "…" : flat
    }

    /// Items of one kind, in document order.
    static func items(_ kind: Kind, in items: [Item]) -> [Item] { items.filter { $0.kind == kind } }

    /// Counts per kind for the sidebar headers.
    static func counts(_ items: [Item]) -> [Kind: Int] {
        var out: [Kind: Int] = [:]
        for i in items { out[i.kind, default: 0] += 1 }
        return out
    }
}

extension ShellModel {
    /// Outline of the active buffer (rescanned on each read; the sidebar
    /// reads it only when the editor revision changes).
    var outline: [DocumentOutline.Item] { DocumentOutline.scan(activeText) }

    /// Sidebar/outline navigation: select the item's command in the editor
    /// (switching documents first when it belongs to another member). The
    /// same `Selection` token the preview click uses, so the editor scrolls
    /// to it and keeps the keyboard.
    func reveal(outlineItem item: DocumentOutline.Item, in path: String? = nil) {
        if let path, path != activePath {
            if case .refused(let why) = project.switchDocument(to: path) { navigationNote = why; return }
        }
        let ns = (activeText as NSString)
        guard item.utf16.location + item.utf16.length <= ns.length else {
            navigationNote = "\(item.title) moved: the outline is older than the buffer."; return
        }
        caretUTF16 = item.utf16.location
        caretLengthUTF16 = item.utf16.length
        selection = .init(path: activePath, nsRange: item.utf16, token: (selection?.token ?? 0) + 1)
        navigationNote = "\(item.kind == .section ? "\\\(item.command)" : item.kind == .environment ? "\\begin{\(item.title)}" : "\\label{\(item.title)}") at line \(item.line)."
    }
}
