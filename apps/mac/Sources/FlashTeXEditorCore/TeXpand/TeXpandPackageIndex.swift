import Foundation

extension TeXpand {
    /// What a root document loads (PLAN §10): its class, its packages
    /// (`\usepackage` and `\RequirePackage`, comma lists and options
    /// included, comments skipped) plus the packages its class implies, and
    /// where a new `\usepackage` line goes.
    public struct PackageIndex: Equatable, Sendable {
        public var documentClass: String?
        public var packages: Set<String>
        /// UTF-16 offset of the line after the last `\usepackage`, else
        /// after `\documentclass`; nil when there is no preamble to extend.
        public var insertionPoint: Int?

        /// Packages a class loads itself (a missing one would be redundant).
        public static let classImplied: [String: Set<String>] = [
            "beamer": ["amsmath", "amssymb", "amsthm", "graphicx", "hyperref", "xcolor", "url"],
            "amsart": ["amsmath", "amssymb", "amsthm", "amsfonts"],
            "amsbook": ["amsmath", "amssymb", "amsthm", "amsfonts"],
            "amsproc": ["amsmath", "amssymb", "amsthm", "amsfonts"],
            "memoir": ["graphicx"],
            "revtex4-2": ["graphicx", "amsmath"],
        ]

        public init(documentClass: String? = nil, packages: Set<String> = [], insertionPoint: Int? = nil) {
            self.documentClass = documentClass; self.packages = packages; self.insertionPoint = insertionPoint
        }

        /// Scans `text` up to `\begin{document}` (or its end).
        public static func scan(_ text: String) -> PackageIndex {
            let ns = text as NSString
            var index = PackageIndex()
            var afterLastPackage: Int?
            var afterClass: Int?
            var lineStart = 0
            while lineStart < ns.length {
                let lineRange = ns.lineRange(for: NSRange(location: lineStart, length: 0))
                let line = ns.substring(with: lineRange)
                let code = stripComment(line)
                if code.contains("\\begin{document}") { break }
                for (cmd, options, arg) in commands(in: code) {
                    switch cmd {
                    case "documentclass":
                        index.documentClass = arg.trimmingCharacters(in: .whitespaces)
                        afterClass = NSMaxRange(lineRange)
                        _ = options
                    case "usepackage", "RequirePackage":
                        for name in arg.split(separator: ",") {
                            let n = name.trimmingCharacters(in: .whitespacesAndNewlines)
                            if !n.isEmpty { index.packages.insert(n) }
                        }
                        afterLastPackage = NSMaxRange(lineRange)
                    default: break
                    }
                }
                lineStart = NSMaxRange(lineRange)
            }
            if let c = index.documentClass { index.packages.formUnion(classImplied[c] ?? []) }
            index.insertionPoint = afterLastPackage ?? afterClass
            return index
        }

        static func stripComment(_ line: String) -> String {
            var out = ""
            var escaped = false
            for c in line {
                if c == "%" && !escaped { break }
                escaped = c == "\\" && !escaped
                out.append(c)
            }
            return out
        }

        /// `\cmd[opts]{arg}` occurrences on one line.
        static func commands(in line: String) -> [(String, String?, String)] {
            var out: [(String, String?, String)] = []
            let pattern = #"\\(documentclass|usepackage|RequirePackage)\s*(?:\[([^\]]*)\])?\s*\{([^}]*)\}"#
            guard let re = try? NSRegularExpression(pattern: pattern) else { return out }
            let ns = line as NSString
            for m in re.matches(in: line, range: NSRange(location: 0, length: ns.length)) {
                let opts = m.range(at: 2).location == NSNotFound ? nil : ns.substring(with: m.range(at: 2))
                out.append((ns.substring(with: m.range(at: 1)), opts, ns.substring(with: m.range(at: 3))))
            }
            return out
        }

        /// The edit that adds what `requires` lacks: `\usepackage` lines at
        /// the insertion point. Nil when nothing is missing or there is no
        /// preamble. Idempotent: a second call after applying it finds
        /// everything loaded.
        public func insertion(for requires: [PackageRequirement], in text: String) -> (location: Int, text: String)? {
            var seen = Set<String>()
            let missing = requires.filter { !packages.contains($0.name) && seen.insert($0.name).inserted }
            guard !missing.isEmpty, let at = insertionPoint else { return nil }
            let ns = text as NSString
            // A preamble whose last line has no newline gets one first.
            let needsBreak = at > 0 && at <= ns.length && ns.character(at: at - 1) != 0x0A
            let lines = missing.map(\.description).joined(separator: "\n") + "\n"
            return (at, (needsBreak ? "\n" : "") + lines)
        }
    }

    /// What to do about a commit's missing packages (`auto_preamble`).
    public enum PreambleAction: Equatable, Sendable {
        case none
        /// Insert `text` at `location` of the current document, in the
        /// expansion's undo step.
        case insert(location: Int, text: String)
        /// Ask first; on yes, insert as `insert` would.
        case prompt(missing: [PackageRequirement], location: Int, text: String)
        /// Tell the author: the root is another file, has no preamble, or
        /// insertion is off.
        case notice(missing: [PackageRequirement])
    }

    /// Decides the preamble edit for `requires` (already filtered against
    /// what the root loads). `rootIsCurrent`: the root document is the one
    /// being edited (only then can the edit share the expansion's undo step).
    public static func preambleAction(for requires: [PackageRequirement], mode: Settings.AutoPreamble,
                                      rootIsCurrent: Bool, rootText: String) -> PreambleAction {
        guard !requires.isEmpty, mode != .off else { return .none }
        let index = PackageIndex.scan(rootText)
        var seen = Set<String>()
        let missing = requires.filter { !index.packages.contains($0.name) && seen.insert($0.name).inserted }
        guard !missing.isEmpty else { return .none }
        guard rootIsCurrent, let edit = index.insertion(for: missing, in: rootText) else { return .notice(missing: missing) }
        return mode == .prompt ? .prompt(missing: missing, location: edit.location, text: edit.text) : .insert(location: edit.location, text: edit.text)
    }

    /// Root-file resolution (PLAN §10): `% !TEX root = …` in the current
    /// file's first lines, then the project's main file, then the file
    /// itself. Paths are the host's (relative to the project root or
    /// absolute); a magic root is resolved against the current file's folder.
    public static func rootPath(current: String, currentText: String, projectMain: String?) -> String {
        let head = currentText.split(separator: "\n", maxSplits: 20, omittingEmptySubsequences: false).prefix(20)
        for line in head {
            let t = line.trimmingCharacters(in: .whitespaces)
            guard t.hasPrefix("%") else { continue }
            let body = t.drop { $0 == "%" }.trimmingCharacters(in: .whitespaces)
            guard body.lowercased().hasPrefix("!tex root") , let eq = body.firstIndex(of: "=") else { continue }
            var file = body[body.index(after: eq)...].trimmingCharacters(in: .whitespaces)
            if !file.lowercased().hasSuffix(".tex") { file += ".tex" }
            if file.hasPrefix("/") { return file }
            let dir = (current as NSString).deletingLastPathComponent
            return normalized(dir.isEmpty ? file : (dir as NSString).appendingPathComponent(file))
        }
        return projectMain ?? current
    }

    /// `a/./b/../c` → `a/c`, for relative paths too (which `standardizingPath` leaves alone).
    static func normalized(_ path: String) -> String {
        var out: [String] = []
        for part in path.split(separator: "/", omittingEmptySubsequences: true) {
            if part == "." { continue }
            if part == "..", let last = out.last, last != ".." { out.removeLast(); continue }
            out.append(String(part))
        }
        return (path.hasPrefix("/") ? "/" : "") + out.joined(separator: "/")
    }
}
