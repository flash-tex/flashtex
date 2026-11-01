import Foundation
import FlashTeXEditorCore

/// Completion for the iPad editor. Local: nearby-v1 does not carry the
/// Mac's revision-bound completion metadata (preview-controller `complete`,
/// compile_result vocabulary), so this ranks what the document itself proves
/// plus the compiler's command inventory — the same bundled
/// `supported-latex.json` the Mac decodes (`LaTeXVocabulary`, shared in
/// FlashTeXEditorCore), so the two editors offer the same commands with the
/// same one-line documentation and the same snippets.
///
/// Works on UTF-8 bytes; a caret out of range yields nothing, never a trap.
/// Pure and `Sendable`: `PadModel` runs it off the main actor from a snapshot.
public enum LocalCompletion {
    public enum Kind: String, Equatable, Sendable { case environmentClose, command, environment, reference, citation }

    public struct Suggestion: Equatable, Identifiable, Sendable {
        public var id: String { "\(kind.rawValue):\(text)" }
        public var kind: Kind
        /// What the row shows and, without a snippet, what is inserted.
        public var text: String
        /// One line of documentation for the row.
        public var detail: String
        /// UTF-8 byte range of the prefix the suggestion replaces.
        public var replaceStart: Int
        public var replaceEnd: Int
        /// When set, accepting inserts this instead of `text` and places the
        /// caret inside it; Tab visits its later stops.
        public var snippet: LaTeXSnippet? = nil

        public init(kind: Kind, text: String, detail: String, replaceStart: Int, replaceEnd: Int, snippet: LaTeXSnippet? = nil) {
            self.kind = kind; self.text = text; self.detail = detail
            self.replaceStart = replaceStart; self.replaceEnd = replaceEnd; self.snippet = snippet
        }

        /// The text an acceptance inserts.
        public var insertion: String { snippet?.text ?? text }
    }

    /// What the editor knows beyond the text: the vocabulary, whether the
    /// caret is in math (nil when unknown), and the environment rules that
    /// shape `\begin{` skeletons.
    public struct Context: Sendable {
        public var vocabulary: LaTeXVocabulary
        public var mathMode: Bool?
        public var indentUnit: String
        public var rules: EnvironmentEditingRules
        public init(vocabulary: LaTeXVocabulary = .empty, mathMode: Bool? = nil, indentUnit: String = "    ",
                    rules: EnvironmentEditingRules = .conventional) {
            self.vocabulary = vocabulary; self.mathMode = mathMode; self.indentUnit = indentUnit; self.rules = rules
        }
        public static let sourceOnly = Context()
    }

    public static let maxSuggestions = 12

    /// Source-only suggestions (the original contract; tests and callers
    /// without a vocabulary).
    public static func suggestions(in text: String, caretByte: Int, limit: Int = maxSuggestions) -> [Suggestion] {
        suggestions(in: text, caretByte: caretByte, context: .sourceOnly, limit: limit)
    }

    public static func suggestions(in text: String, caretByte: Int, context: Context, limit: Int = maxSuggestions) -> [Suggestion] {
        let bytes = Array(text.utf8)
        guard caretByte >= 0, caretByte <= bytes.count else { return [] }

        // 1. Word/command prefix under the caret.
        var start = caretByte
        while start > 0, isWordByte(bytes[start - 1]) { start -= 1 }
        var isCommand = false
        if start > 0, bytes[start - 1] == 0x5C /* \ */ { isCommand = true; start -= 1 }
        let prefix = String(decoding: bytes[start..<caretByte], as: UTF8.self)

        // 2. Argument context: `\begin{`, `\end{`, `\ref{`, `\cite{`.
        var argCommand: String?
        if !isCommand, start >= 1, bytes[start - 1] == 0x7B {
            let brace = start - 1
            var s = brace
            while s > 0, isWordByte(bytes[s - 1]) { s -= 1 }
            if s > 0, bytes[s - 1] == 0x5C { argCommand = String(decoding: bytes[s..<brace], as: UTF8.self) }
        }

        var out: [Suggestion] = []
        var seen = Set<String>()
        func add(_ kind: Kind, _ t: String, _ detail: String, snippet: LaTeXSnippet? = nil) {
            guard seen.insert("\(kind.rawValue):\(t)").inserted else { return }
            out.append(Suggestion(kind: kind, text: t, detail: detail, replaceStart: start, replaceEnd: caretByte, snippet: snippet))
        }

        let openEnvs = LaTeXEditing.openEnvironments(in: text, beforeByte: caretByte).map(\.name)
        let documentClass = Self.documentClass(in: bytes)
        // A class-scoped command or environment (`\frametitle`, beamer's
        // `frame`) is offered unless the document declares another class.
        func offered(_ requiresClass: String?) -> Bool {
            LaTeXVocabulary.classOffers(requiresClass, documentClass: documentClass)
        }

        if let argCommand {
            switch argCommand {
            case "begin", "end":
                let closing = argCommand == "end"
                let indent = closing ? "" : lineIndent(bytes, beforeByte: start)
                func envSnippet(_ name: String) -> LaTeXSnippet? {
                    closing ? nil : LaTeXSnippets.environment(name, indent: indent, unit: context.indentUnit, rules: context.rules)
                }
                if closing { for env in openEnvs.reversed() where env.hasPrefix(prefix) { add(.environment, env, "closes open \\begin{\(env)}") } }
                for n in names(after: "\\begin{", in: text) where n.hasPrefix(prefix) {
                    let doc = context.vocabulary.environments.first { $0.name == n }?.description ?? "seen in document"
                    add(.environment, n, doc, snippet: envSnippet(n))
                }
                for e in ranked(context.vocabulary.environments.filter { offered($0.requiresClass) }, prefix: prefix, name: \.name) {
                    add(.environment, e.name, e.description, snippet: envSnippet(e.name))
                }
            case "ref", "eqref", "pageref", "autoref", "cref", "Cref":
                for n in names(after: "\\label{", in: text) where n.hasPrefix(prefix) { add(.reference, n, "label in document") }
            case "cite", "citep", "citet", "parencite", "textcite", "autocite", "nocite":
                for n in citationKeys(in: text) where n.hasPrefix(prefix) { add(.citation, n, "cited in document") }
            default: break
            }
            return Array(out.prefix(limit))
        }

        if isCommand {
            for env in openEnvs.reversed() { let t = "\\end{\(env)}"; if t.hasPrefix(prefix) { add(.environmentClose, t, "closes open \\begin{\(env)}") } }
            let name = String(prefix.dropFirst())
            for c in commands(in: text) where c.hasPrefix(prefix) && c != prefix {
                let entry = context.vocabulary.command(named: String(c.dropFirst()))
                if let entry, !LaTeXVocabulary.allows(entry, mathMode: context.mathMode) || !offered(entry.requiresClass) { continue }
                add(.command, c, entry?.documentation ?? "typed elsewhere in this document", snippet: entry.flatMap(snippet(for:)))
            }
            let pool = context.vocabulary.commands.filter { LaTeXVocabulary.allows($0, mathMode: context.mathMode) && offered($0.requiresClass) }
            for entry in ranked(pool, prefix: name, name: \.name) {
                add(.command, "\\" + entry.name, entry.documentation, snippet: snippet(for: entry))
            }
            return Array(out.prefix(limit))
        }
        return []
    }

    /// The Mac's argument skeleton for a vocabulary entry (`\frac{}{}` with
    /// the caret in the first group and a Tab stop in the second).
    static func snippet(for entry: LaTeXVocabulary.Command) -> LaTeXSnippet? {
        LaTeXSnippets.argument(name: entry.name, arguments: entry.arguments)
    }

    /// The entries whose name starts with `prefix`: the exact name first,
    /// then shorter names before longer ones (`\frac` before `\frametitle`
    /// for `\fr`), ties in the vocabulary's offer order.
    static func ranked<T>(_ entries: [T], prefix: String, name: (T) -> String) -> [T] {
        entries.enumerated()
            .filter { name($0.element).hasPrefix(prefix) }
            .sorted { a, b in
                let an = name(a.element), bn = name(b.element)
                if (an == prefix) != (bn == prefix) { return an == prefix }
                if an.count != bn.count { return an.count < bn.count }
                return a.offset < b.offset
            }
            .map(\.element)
    }

    /// The class of `\documentclass[options]{class}`, or nil when the text
    /// declares none (a fragment, or the caret's file is not the root).
    static func documentClass(in bytes: [UInt8]) -> String? {
        let marker = Array("\\documentclass".utf8)
        var i = 0
        while i + marker.count <= bytes.count {
            if bytes[i] == 0x25 { while i < bytes.count, bytes[i] != 0x0A { i += 1 }; continue } // `%` comment
            if bytes[i] == 0x5C, Array(bytes[i..<(i + marker.count)]) == marker,
               i + marker.count == bytes.count || !isWordByte(bytes[i + marker.count]) {
                var k = i + marker.count
                while k < bytes.count, bytes[k] == 0x20 || bytes[k] == 0x09 || bytes[k] == 0x0A { k += 1 }
                if k < bytes.count, bytes[k] == 0x5B {
                    while k < bytes.count, bytes[k] != 0x5D { k += 1 }
                    k += 1
                    while k < bytes.count, bytes[k] == 0x20 || bytes[k] == 0x09 || bytes[k] == 0x0A { k += 1 }
                }
                guard k < bytes.count, bytes[k] == 0x7B else { return nil }
                let start = k + 1
                var end = start
                while end < bytes.count, bytes[end] != 0x7D, bytes[end] != 0x0A { end += 1 }
                guard end < bytes.count, bytes[end] == 0x7D else { return nil }
                let cls = String(decoding: bytes[start..<end], as: UTF8.self).trimmingCharacters(in: .whitespaces)
                return cls.isEmpty ? nil : cls
            }
            i += 1
        }
        return nil
    }

    static func isWordByte(_ b: UInt8) -> Bool {
        (b >= 0x41 && b <= 0x5A) || (b >= 0x61 && b <= 0x7A) || (b >= 0x30 && b <= 0x39) || b == 0x2A || b == 0x3A || b == 0x2D || b == 0x5F
    }

    /// Leading whitespace of the line containing `byte`.
    static func lineIndent(_ bytes: [UInt8], beforeByte byte: Int) -> String {
        var lineStart = min(byte, bytes.count)
        while lineStart > 0, bytes[lineStart - 1] != 0x0A { lineStart -= 1 }
        var end = lineStart
        while end < bytes.count, bytes[end] == 0x20 || bytes[end] == 0x09 { end += 1 }
        return String(decoding: bytes[lineStart..<end], as: UTF8.self)
    }

    /// Environments opened before the end of `text` and not yet closed, in order.
    public static func openEnvironments(in text: String) -> [String] {
        LaTeXEditing.openEnvironments(in: text, beforeByte: text.utf8.count).map(\.name)
    }

    static func names(after marker: String, in text: String) -> [String] {
        var out: [String] = []
        var search = text.startIndex
        while let r = text.range(of: marker, range: search..<text.endIndex) {
            if let close = text[r.upperBound...].firstIndex(of: "}") {
                let n = String(text[r.upperBound..<close])
                if !n.isEmpty, !out.contains(n) { out.append(n) }
                search = close
            } else { break }
        }
        return out
    }

    /// Keys of every `\cite…{a, b}` in the document, once each.
    static func citationKeys(in text: String) -> [String] {
        var out: [String] = []
        for marker in ["\\cite{", "\\citep{", "\\citet{", "\\parencite{", "\\textcite{", "\\autocite{"] {
            for group in names(after: marker, in: text) {
                for key in group.split(separator: ",") {
                    let k = key.trimmingCharacters(in: .whitespaces)
                    if !k.isEmpty, !out.contains(k) { out.append(k) }
                }
            }
        }
        return out
    }

    /// Distinct `\command` tokens in the document, most frequent first.
    public static func commands(in text: String) -> [String] {
        var counts: [String: Int] = [:]
        var order: [String] = []
        let bytes = Array(text.utf8)
        var i = 0
        while i < bytes.count {
            if bytes[i] == 0x5C {
                var j = i + 1
                while j < bytes.count, (bytes[j] >= 0x41 && bytes[j] <= 0x5A) || (bytes[j] >= 0x61 && bytes[j] <= 0x7A) { j += 1 }
                if j > i + 1 {
                    let t = String(decoding: bytes[i..<j], as: UTF8.self)
                    if counts[t] == nil { order.append(t) }
                    counts[t, default: 0] += 1
                }
                i = max(j, i + 1)
            } else { i += 1 }
        }
        return order.sorted { (counts[$0]!, $1) > (counts[$1]!, $0) }
    }
}
