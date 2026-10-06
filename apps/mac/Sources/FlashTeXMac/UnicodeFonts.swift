import AppKit
import FlashTeXDisplayListV3

// Documents that need Unicode/OpenType fonts (lane P5-FONTSPEC-FALLBACK; the
// retirement plan's §4.4 "Unicode-routed documents" and R10, #1236;
// docs/design/modes/PROPOSAL.md §4.2 and Q1). The new engine is
// pdfLaTeX-compatible: fontspec, unicode-math, polyglossia, xeCJK and the
// like stop under it with their own "requires XeTeX or LuaTeX" error. Until
// the Unicode mode passes its M3 gate the previous engine stays their
// explicit fallback, so such a document is typeset by it, visibly
// (`EngineChoice.Blocker.unicodeFonts`), and never ends in that error.
//
// Two signals, as the modes proposal's §4.2 has them:
//   1. a lexical scan of the preamble (this file's `UnicodeFonts.scan`):
//      the entry document up to `\begin{document}`, and the project's own
//      `.cls`, `.sty` and `\input` files it loads there;
//   2. the engine's own truth: a run of the new engine that reports one of
//      those packages' errors (`UnicodeFonts.need(in:)` over its DIAGs).
// The scan suggests; the user's own choice for the document still decides
// (`Blocker.yieldsToUserChoice`).

/// Why a document needs a Unicode engine, and where that was found.
struct UnicodeFontsNeed: Equatable, Sendable {
    enum Kind: Equatable, Sendable {
        /// `\usepackage{fontspec}` and the like (the package's name).
        case package(String)
        /// `\setmainfont` and the like (the command, without the backslash).
        case command(String)
        /// `% !TEX program = xelatex` (the program named).
        case program(String)
        /// XeTeX's own `\font\x="Name"` or `\font\x=[file.otf]`.
        case fontPrimitive
        /// A run of the new engine stopped on the package's error (its message).
        case engineReport(String)
    }

    var kind: Kind
    /// The file (project-relative) the scan found it in; nil for the engine's report.
    var file: String?

    /// "fontspec", "\setmainfont", "the TeX program xelatex", …
    var what: String {
        switch kind {
        case .package(let p): "the \(p) package"
        case .command(let c): "\\\(c)"
        case .program(let p): "% !TEX program = \(p)"
        case .fontPrimitive: "XeTeX's \\font syntax for system fonts"
        case .engineReport(let m): "a package that stopped the new engine (\(m))"
        }
    }
}

enum UnicodeFonts {
    /// Packages that need XeTeX or LuaTeX (each stops under pdfLaTeX with its
    /// own error; fontspec's is fatal).
    static let packages: Set<String> = [
        "fontspec", "unicode-math", "polyglossia", "xeCJK", "xecjk", "xltxtra", "xunicode", "mathspec",
        "luatexja", "luatexja-fontspec", "luaotfload", "fontsetup", "xepersian", "bidi", "ucharclasses",
        "realscripts", "xgreek", "fontwrap",
    ]

    /// Commands that exist only with fontspec, polyglossia or iftex's
    /// engine checks (a document using them loads one of those packages, or
    /// stops under pdfLaTeX).
    static let commands: Set<String> = [
        "setmainfont", "setsansfont", "setmonofont", "setmathfont", "setromanfont", "setmathrm", "setboldmathrm",
        "newfontfamily", "newfontface", "renewfontfamily", "setfontfamily", "defaultfontfeatures", "fontspec",
        "addfontfeatures", "addfontfeature", "setCJKmainfont", "setCJKsansfont", "setCJKmonofont",
        "setmainlanguage", "setotherlanguage", "setdefaultlanguage", "RequireXeTeX", "RequireLuaTeX", "RequireLuaHBTeX",
        "RequireTUTeX",
    ]

    /// TeX programs a `% !TEX program` magic comment names that are Unicode engines.
    static let programs: Set<String> = ["xelatex", "lualatex", "xetex", "luatex", "lualatex-dev", "xelatex-dev", "luahblatex"]

    /// Conditionals whose true branch runs only under XeTeX or LuaTeX, and
    /// those whose false branch does (`\ifpdftex … \else <Unicode> \fi`).
    static let unicodeOnlyTrue: Set<String> = ["ifxetex", "ifluatex", "ifXeTeX", "ifLuaTeX", "iftutex", "ifTUTeX", "ifluahbtex", "ifLuaHBTeX"]
    static let unicodeOnlyFalse: Set<String> = ["ifpdftex", "ifPDFTeX"]

    /// Primitives only XeTeX or LuaTeX define (`\ifdefined\XeTeXversion`).
    static let unicodePrimitives: Set<String> = ["XeTeXversion", "XeTeXrevision", "directlua", "luatexversion", "Umathcode"]

    /// Files followed from the entry (nesting of `\documentclass`, `\usepackage`, `\input` of project files).
    static let maxDepth = 6

    // MARK: the scan

    /// The first need for a Unicode engine in the entry document's preamble
    /// (and the project files it loads there), or nil. `read(name)` gives a
    /// project file's text by its name as TeX finds it relative to the
    /// entry's folder (`foo.sty`, `chapters/pre.tex`), nil when the project
    /// has none: a class or package of TeX Live's is not scanned (it is not
    /// the document's choice).
    static func scan(entry text: String, entryName: String = "main.tex", read: (String) -> String?) -> UnicodeFontsNeed? {
        if let p = magicProgram(text) { return UnicodeFontsNeed(kind: .program(p), file: entryName) }
        var visited: Set<String> = [entryName]
        return scanFile(text, name: entryName, wholeFile: false, depth: 0, visited: &visited, read: read)
    }

    /// `% !TEX program = xelatex` (or `TS-program`, any case) in the first lines.
    static func magicProgram(_ text: String) -> String? {
        for line in text.split(separator: "\n", omittingEmptySubsequences: false).prefix(25) {
            let l = line.trimmingCharacters(in: .whitespaces)
            guard l.hasPrefix("%") else {
                if l.isEmpty { continue }
                break // the magic comments come first
            }
            let body = l.drop(while: { $0 == "%" || $0 == " " })
            guard body.lowercased().hasPrefix("!tex") else { continue }
            let rest = body.dropFirst(4).trimmingCharacters(in: .whitespaces)
            guard let eq = rest.firstIndex(of: "=") else { continue }
            let key = rest[..<eq].trimmingCharacters(in: .whitespaces).lowercased()
            guard key == "program" || key == "ts-program" else { continue }
            let value = rest[rest.index(after: eq)...].trimmingCharacters(in: .whitespaces).lowercased()
            let program = String(value.prefix { !$0.isWhitespace })
            if programs.contains(program) { return program }
        }
        return nil
    }

    private static func scanFile(_ text: String, name: String, wholeFile: Bool, depth: Int,
                                 visited: inout Set<String>, read: (String) -> String?) -> UnicodeFontsNeed? {
        let toks = tokens(text)
        // Engine conditionals: a branch that runs only under XeTeX or LuaTeX
        // does not count (`\ifxetex \usepackage{fontspec} \else … \fi`).
        // Each `\if…` pushes; `\else` flips; `\fi` pops.
        var stack: [(unicodeOnly: Bool, skipWhenTrue: Bool, inElse: Bool)] = []
        var skipping: Bool { stack.contains { $0.unicodeOnly && ($0.skipWhenTrue != $0.inElse) } }
        var i = 0
        func argument(at j: inout Int) -> String? {
            // optional [..] arguments first, then one {..} group
            while j < toks.count, case .other("[") = toks[j] {
                var depth = 0
                while j < toks.count {
                    if case .other("[") = toks[j] { depth += 1 }
                    if case .other("]") = toks[j] { depth -= 1; if depth == 0 { j += 1; break } }
                    j += 1
                }
            }
            guard j < toks.count, case .group(let g) = toks[j] else { return nil }
            j += 1
            return g
        }
        func word(at j: Int) -> String? {
            if j < toks.count, case .word(let w) = toks[j] { return w }
            return nil
        }
        func isGroup(at j: Int) -> Bool {
            if j < toks.count, case .group = toks[j] { return true }
            return false
        }
        while i < toks.count {
            let t = toks[i]
            i += 1
            if case .group(let g) = t {
                // a group no command here took as its argument (`\AtBeginDocument{…}`,
                // `\IfFileExists{…}{\usepackage{fontspec}}{}`): its commands count
                if !skipping, let need = scanFile(g, name: name, wholeFile: true, depth: depth, visited: &visited, read: read) {
                    return need
                }
                continue
            }
            guard case .word(let w) = t else { continue }
            if !wholeFile, w == "begin", i < toks.count, case .group("document") = toks[i] { return nil }
            if w == "newif" || w == "let" || w == "def" || w == "gdef" || w == "edef" || w == "xdef" {
                i += 1 // `\newif\iffoo`, `\let\ifx…`: the next word is defined, not run
                continue
            }
            if w == "else" { if !stack.isEmpty { stack[stack.count - 1].inElse = true }; continue }
            if w == "fi" { if !stack.isEmpty { stack.removeLast() }; continue }
            // A TeX conditional (`\if…`, ended by `\fi`); an `\if…` taking a
            // brace group is a command (`\iftoggle{…}`, `\ifthenelse{…}`), not one.
            if w.hasPrefix("if"), !isGroup(at: i) {
                var t = unicodeOnlyTrue.contains(w), f = unicodeOnlyFalse.contains(w)
                // `\ifdefined\XeTeXversion …`, `\ifx\XeTeXversion\undefined …`
                if let n = word(at: i), unicodePrimitives.contains(n) {
                    if w == "ifdefined" { t = true }
                    if w == "ifx", let m = word(at: i + 1), m == "undefined" || m == "relax" || m == "@undefined" { f = true }
                }
                stack.append((t || f, t, false))
                continue
            }
            if skipping { continue }
            switch w {
            case "usepackage", "RequirePackage", "RequirePackageWithOptions":
                guard let arg = argument(at: &i) else { continue }
                for p in arg.split(separator: ",").map({ $0.trimmingCharacters(in: .whitespacesAndNewlines) }) where !p.isEmpty {
                    if packages.contains(p) { return UnicodeFontsNeed(kind: .package(p), file: name) }
                    if let need = follow(p + ".sty", depth: depth, visited: &visited, read: read) { return need }
                }
            case "documentclass", "LoadClass", "LoadClassWithOptions":
                guard let arg = argument(at: &i) else { continue }
                let c = arg.trimmingCharacters(in: .whitespacesAndNewlines)
                if !c.isEmpty, let need = follow(c + ".cls", depth: depth, visited: &visited, read: read) { return need }
            case "input", "include", "InputIfFileExists":
                var j = i
                guard let arg = argument(at: &j) else { continue }
                i = j
                let f = arg.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !f.isEmpty else { continue }
                let candidates = (f as NSString).pathExtension.isEmpty ? [f + ".tex", f] : [f]
                for c in candidates where read(c) != nil {
                    if let need = follow(c, depth: depth, visited: &visited, read: read) { return need }
                    break
                }
            case "font":
                // XeTeX's `\font\x="Name"` / `\font\x=[file.otf]` (pdfTeX takes neither).
                if i < toks.count, case .word = toks[i] {
                    var j = i + 1
                    if j < toks.count, case .other("=") = toks[j] { j += 1 }
                    if j < toks.count, case .other(let c) = toks[j], c == "\"" || c == "[" {
                        return UnicodeFontsNeed(kind: .fontPrimitive, file: name)
                    }
                }
            default:
                if commands.contains(w) { return UnicodeFontsNeed(kind: .command(w), file: name) }
            }
        }
        return nil
    }

    private static func follow(_ file: String, depth: Int, visited: inout Set<String>, read: (String) -> String?) -> UnicodeFontsNeed? {
        guard depth < maxDepth, visited.insert(file).inserted, let text = read(file) else { return nil }
        // A class or package file is all preamble; an `\input` file of the
        // preamble is read up to a `\begin{document}` it may hold.
        let whole = file.hasSuffix(".sty") || file.hasSuffix(".cls")
        return scanFile(text, name: file, wholeFile: whole, depth: depth + 1, visited: &visited, read: read)
    }

    // MARK: tokens

    enum Token: Equatable {
        /// A control word (`\usepackage` → `usepackage`; `@` is a letter, as in
        /// package files).
        case word(String)
        /// A `{…}` group's text (balanced, comments removed).
        case group(String)
        /// Any other character that matters here (`[`, `]`, `=`, `"`).
        case other(Character)
    }

    /// The text as control words, brace groups and the few characters the
    /// scan reads; comments (`%` to the end of the line, unless escaped) are dropped.
    static func tokens(_ text: String) -> [Token] {
        var out: [Token] = []
        let s = Array(text.unicodeScalars)
        var i = 0
        func isLetter(_ c: Unicode.Scalar) -> Bool { (c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c == "@" }
        while i < s.count {
            let c = s[i]
            switch c {
            case "%":
                while i < s.count, s[i] != "\n" { i += 1 }
            case "\\":
                i += 1
                guard i < s.count else { break }
                if isLetter(s[i]) {
                    let start = i
                    while i < s.count, isLetter(s[i]) { i += 1 }
                    out.append(.word(String(String.UnicodeScalarView(s[start ..< i]))))
                } else {
                    i += 1 // a control symbol (`\%`, `\{`): nothing the scan reads
                }
            case "{":
                var depth = 0
                var g = String.UnicodeScalarView()
                i += 1
                while i < s.count {
                    let d = s[i]
                    if d == "\\", i + 1 < s.count { g.append(d); g.append(s[i + 1]); i += 2; continue }
                    if d == "%" { while i < s.count, s[i] != "\n" { i += 1 }; continue }
                    if d == "{" { depth += 1 }
                    if d == "}" { if depth == 0 { i += 1; break }; depth -= 1 }
                    g.append(d)
                    i += 1
                }
                out.append(.group(String(g)))
            case "[", "]", "=", "\"":
                out.append(.other(Character(c)))
                i += 1
            default:
                i += 1
            }
        }
        return out
    }

    // MARK: the engine's report

    /// The messages the Unicode-only packages stop the new engine with
    /// (measured: the host's DIAGs for each, TeX Live 2026).
    static let reportSignatures = [
        "requires either XeTeX or LuaTeX", // fontspec (fatal), so also polyglossia, mathspec, …
        "Package unicode-math Error: Cannot be run with pdf", // unicode-math
        "requires XeTeX to function", // xeCJK (critical)
        "is required to compile this document", // iftex's \RequireXeTeX / \RequireLuaTeX
        "Use XeLaTeX or LuaLaTeX instead",
    ]

    /// Whether a diagnostic of the new engine says the document needs XeTeX or LuaTeX.
    static func need(message: String, detail: String? = nil, frames: [String] = []) -> UnicodeFontsNeed? {
        let text = message + "\n" + (detail ?? "")
        if reportSignatures.contains(where: { text.contains($0) }) || frames.contains("\\IFTEX@Require") {
            let short = message.trimmingCharacters(in: .whitespacesAndNewlines)
            return UnicodeFontsNeed(kind: .engineReport(short.count > 90 ? String(short.prefix(90)) + "…" : short), file: nil)
        }
        return nil
    }

    static func need(in d: DL3Diag) -> UnicodeFontsNeed? {
        need(message: d.message, detail: d.detail, frames: d.trace.compactMap(\.name))
    }
}

// MARK: - the window

extension ShellModel {
    /// The open document's need for a Unicode engine from its preamble
    /// (`UnicodeFonts.scan`): the entry's text as the editor has it, and the
    /// project files it loads there (an open buffer, else the file under the
    /// project root; never outside it). Cached by `documentsRevision`.
    func unicodeFontsNeed() -> UnicodeFontsNeed? {
        let entry = project.entryPath
        if let s = unicodeFontsScan, s.revision == documentsRevision, s.entry == entry { return s.need }
        let root = project.projectRoot
        let open = Dictionary(documents.map { ($0.path, $0.text) }, uniquingKeysWith: { a, _ in a })
        let entryDir = (entry as NSString).deletingLastPathComponent
        func read(_ name: String) -> String? {
            let joined = entryDir.isEmpty ? name : (entryDir as NSString).appendingPathComponent(name)
            let rel = (joined as NSString).standardizingPath
            guard !rel.hasPrefix("/"), !rel.hasPrefix("..") else { return nil }
            if let t = open[rel] { return t }
            guard let root else { return nil }
            let url = root.appendingPathComponent(rel)
            guard let size = (try? FileManager.default.attributesOfItem(atPath: url.path))?[.size] as? Int, size < 4 << 20 else { return nil }
            return try? String(contentsOf: url, encoding: .utf8)
        }
        let need = UnicodeFonts.scan(entry: entryText, entryName: (entry as NSString).lastPathComponent, read: read)
        unicodeFontsScan = (documentsRevision, entry, need)
        return need
    }

    /// An edit may have added or removed what makes the document need a
    /// Unicode engine: check again once typing pauses (0.8 s), and only
    /// where the rule can apply (the new engine preferred, not forced, not
    /// the user's own choice). Cheap per keystroke: one timer, re-armed.
    func scheduleUnicodeFontsCheck() {
        let c = engineChoice
        guard c.preferred == .new, !c.isForced, c.source != .user, !engineChoicePending else { return }
        unicodeFontsCheck?.cancel()
        let work = DispatchWorkItem { [weak self] in
            MainActor.assumeIsolated { self?.recheckUnicodeFonts() }
        }
        unicodeFontsCheck = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.8, execute: work)
    }

    /// The rules again for the open document after an edit (as
    /// `manifestDidRefresh` after a manifest change): a document that now
    /// needs Unicode fonts falls back, announced; one that no longer does
    /// returns to the new engine, announced too (never silently).
    func recheckUnicodeFonts() {
        unicodeFontsCheck = nil
        guard !engineChoicePending, engineChoiceDocument == documentURL else { return }
        var c = engineChoice
        guard c.preferred == .new, !c.isForced else { return }
        c.applyRule(engineBlocker())
        guard c != engineChoice else { return }
        var ended = false
        if case .unicodeFonts = engineChoice.blocker, c.blocker == nil { ended = true }
        engineFallbackDismissed = false
        applyEngineChoice(c)
        if ended { announceEngine("This document no longer needs Unicode fonts: the new engine typesets it.") }
    }

    /// A run of the new engine stopped on a package that needs XeTeX or
    /// LuaTeX (fontspec's fatal error, unicode-math's, xeCJK's, iftex's
    /// `\RequireXeTeX`): the document falls back, with the reason, unless the
    /// choice was forced or is the user's own for this document.
    func engineV3NeedsUnicodeFonts(_ need: UnicodeFontsNeed) {
        let c0 = engineChoice
        guard !c0.isForced, c0.preferred == .new, c0.source != .user, c0.blocker == nil else { return }
        engineHostNeedsUnicode = (documentURL, need)
        var c = c0
        c.applyRule(.unicodeFonts(need))
        engineFallbackDismissed = false
        applyEngineChoice(c)
    }

    /// Says `message` to VoiceOver and keeps it (tests read `engineAnnouncements`).
    func announceEngine(_ message: String) {
        FlashTeXLog.write("engine choice: \(message)")
        engineAnnouncements.append(message)
        NSAccessibility.post(element: NSApplication.shared, notification: .announcementRequested,
                             userInfo: [.announcement: message, .priority: NSAccessibilityPriorityLevel.high.rawValue])
    }
}
