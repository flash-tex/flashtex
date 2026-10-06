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
//
// The scan counts *uses* only. A name that is tested or defined is not a use
// (`\ifdefined\setmainfont`, `\ifx\fontspec\undefined`,
// `\providecommand\setmainfont[1]{}`, etoolbox's `\ifdef{\setmainfont}`), and
// branches that run only under XeTeX or LuaTeX, or only once such a package is
// loaded, are not read (`\ifxetex`, `\iftutex`, `\ifdefined\XeTeXversion`,
// `\@ifpackageloaded{fontspec}{…}`, `\IfPackageLoadedTF{fontspec}{…}{…}`), nor
// is `\iffalse … \fi`. A conditional it does not know reads both branches.

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

    /// Names whose being defined means XeTeX or LuaTeX (or one of the
    /// packages above) is running: a branch taken when one is defined is not
    /// read (`\ifdefined\XeTeXversion …`, `\ifx\setmainfont\undefined … \else …`).
    static let unicodeNames: Set<String> = commands.union(["XeTeXversion", "XeTeXrevision", "directlua", "luatexversion", "Umathcode"])

    /// TeX programs a `% !TEX program` magic comment names that are Unicode engines.
    static let programs: Set<String> = ["xelatex", "lualatex", "xetex", "luatex", "lualatex-dev", "xelatex-dev", "luahblatex"]

    /// Conditionals whose true branch runs only under XeTeX or LuaTeX, and
    /// those whose false branch does (`\ifpdftex … \else <Unicode> \fi`).
    static let unicodeOnlyTrue: Set<String> = ["ifxetex", "ifluatex", "ifXeTeX", "ifLuaTeX", "iftutex", "ifTUTeX", "ifluahbtex", "ifLuaHBTeX"]
    static let unicodeOnlyFalse: Set<String> = ["ifpdftex", "ifPDFTeX"]

    /// `\if…` commands that take brace arguments and need no `\fi` (etoolbox,
    /// ifthen, babel, biblatex): not TeX conditionals. Any other `\if…`
    /// followed directly by a `{` is taken as one of these too.
    static let commandIfs: Set<String> = [
        "ifdef", "ifundef", "ifcsdef", "ifcsundef", "ifdefmacro", "ifcsmacro", "ifdefparam", "ifcsparam", "ifdefprefix",
        "ifcsprefix", "ifdefprotected", "ifcsprotected", "ifdefltxprotect", "ifcsltxprotect", "ifdefempty", "ifcsempty",
        "ifdefvoid", "ifcsvoid", "ifdefequal", "ifcsequal", "ifdefstring", "ifcsstring", "ifdefstrequal", "ifcsstrequal",
        "ifdefcounter", "ifcscounter", "ifltxcounter", "ifdeflength", "ifcslength", "ifdefdimen", "ifcsdimen",
        "ifbool", "ifboolexpr", "ifboolexpe", "iftoggle", "ifnumcomp", "ifnumequal", "ifnumgreater", "ifnumless",
        "ifnumodd", "ifdimcomp", "ifdimequal", "ifdimgreater", "ifdimless", "ifstrequal", "ifstrempty", "ifblank",
        "ifrmnum", "ifinlist", "ifinlistcs", "ifthenelse", "iflanguage", "ifentrytype", "ifcategory", "ifkeyword",
    ]

    /// Commands that define the name after them (that name is not a use).
    static let definers: Set<String> = [
        "def", "gdef", "edef", "xdef", "newcommand", "renewcommand", "providecommand", "DeclareRobustCommand",
        "newrobustcmd", "renewrobustcmd", "providerobustcmd", "NewDocumentCommand", "RenewDocumentCommand",
        "ProvideDocumentCommand", "DeclareDocumentCommand", "NewCommandCopy", "RenewCommandCopy", "newif",
        "DeclareTextCommand", "ProvideTextCommand",
    ]

    /// Files followed from the entry (nesting of `\documentclass`, `\usepackage`, `\input` of project files).
    static let maxDepth = 6

    // MARK: the scan

    /// The first need for a Unicode engine in the entry document's preamble
    /// (and the project files it loads there), or nil. `read(name)` gives a
    /// project file's text by its name as TeX finds it relative to the
    /// entry's folder (`foo.sty`, `chapters/pre.tex`), nil when the project
    /// has none: a class or package of TeX Live's is not scanned (it is not
    /// the document's choice).
    static func scan(entry text: String, entryName: String = "main.tex", read: @escaping (String) -> String?) -> UnicodeFontsNeed? {
        if let p = magicProgram(text) { return UnicodeFontsNeed(kind: .program(p), file: entryName) }
        var s = Scanner(read: read, visited: [entryName])
        return s.scan(tokens(text, preambleOnly: true), file: entryName, depth: 0)
    }

    /// `% !TEX program = xelatex` (or `TS-program`, any case) in the first
    /// lines (after a byte-order mark, if any).
    static func magicProgram(_ text: String) -> String? {
        let t = text.hasPrefix("\u{FEFF}") ? String(text.dropFirst()) : text
        for line in t.split(separator: "\n", omittingEmptySubsequences: false).prefix(25) {
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

    /// One scan: the files it has read, and the reader.
    struct Scanner {
        let read: (String) -> String?
        var visited: Set<String>

        /// A conditional being read: whether its true / false branch is not
        /// read (it runs only under a Unicode engine, or never), and which
        /// branch the scan is in.
        struct Cond { var skipTrue: Bool; var skipFalse: Bool; var inElse = false }

        mutating func scan(_ toks: [Token], file: String, depth: Int) -> UnicodeFontsNeed? {
            var stack: [Cond] = []
            var skipping: Bool { stack.contains { $0.inElse ? $0.skipFalse : $0.skipTrue } }
            var i = 0
            func word(at j: Int) -> String? {
                if j < toks.count, case .word(let w) = toks[j] { return w }
                return nil
            }
            func group(at j: Int) -> String? {
                if j < toks.count, case .group(let g) = toks[j] { return g }
                return nil
            }
            /// Past `[…]` options at `j`.
            func skipOptions(_ j: inout Int) {
                while j < toks.count, case .other("[") = toks[j] {
                    var depth = 0
                    while j < toks.count {
                        if case .other("[") = toks[j] { depth += 1 }
                        if case .other("]") = toks[j] { depth -= 1; if depth == 0 { j += 1; break } }
                        j += 1
                    }
                }
            }
            /// The name a reference or a definition names: `\x` or `{\x}`.
            func name(at j: Int) -> String? {
                if let w = word(at: j) { return w }
                if let g = group(at: j) {
                    let t = g.trimmingCharacters(in: .whitespacesAndNewlines)
                    return t.hasPrefix("\\") ? String(t.dropFirst()) : t
                }
                return nil
            }
            /// A group scanned as code that may run.
            func scanGroup(_ g: String, _ s: inout Scanner) -> UnicodeFontsNeed? {
                s.scan(UnicodeFonts.tokens(g, preambleOnly: false), file: file, depth: depth)
            }
            while i < toks.count {
                let t = toks[i]
                i += 1
                if case .group(let g) = t {
                    // a group no command here took (`\AtBeginDocument{…}`, a macro's
                    // body, `\IfFileExists{…}{…}{…}`): its commands may run
                    if !skipping, let need = scanGroup(g, &self) { return need }
                    continue
                }
                guard case .word(let w) = t else { continue }

                // conditionals (tracked even while skipping, so `\fi`s pair up)
                if w == "else" || w == "or" { if !stack.isEmpty { stack[stack.count - 1].inElse = true }; continue }
                if w == "fi" { if !stack.isEmpty { stack.removeLast() }; continue }
                if w.hasPrefix("if"), !commandIfs.contains(w), group(at: i) == nil {
                    var c = Cond(skipTrue: false, skipFalse: false)
                    switch w {
                    case "iffalse": c.skipTrue = true
                    case "iftrue": c.skipFalse = true
                    case "ifdefined":
                        if let n = word(at: i) { i += 1; if UnicodeFonts.unicodeNames.contains(n) { c.skipTrue = true } }
                    case "ifx":
                        // `\ifx\a\b`: both operands are references
                        let a = word(at: i), b = word(at: i + 1)
                        i += (a != nil ? 1 : 0) + (b != nil ? 1 : 0)
                        if let a, UnicodeFonts.unicodeNames.contains(a), let b, ["undefined", "relax", "@undefined"].contains(b) {
                            c.skipFalse = true
                        }
                    case "ifcsname":
                        while i < toks.count, word(at: i) != "endcsname" { i += 1 }
                        i += 1
                    default:
                        if UnicodeFonts.unicodeOnlyTrue.contains(w) { c.skipTrue = true }
                        if UnicodeFonts.unicodeOnlyFalse.contains(w) { c.skipFalse = true }
                    }
                    stack.append(c)
                    continue
                }
                if skipping { continue }

                // references and definitions: the name after them is not a use
                if w == "let" || w == "futurelet" {
                    i += 1 // the name defined
                    if case .other("=") = (i < toks.count ? toks[i] : .other(" ")) { i += 1 }
                    i += 1 // the name it takes the meaning of
                    continue
                }
                if UnicodeFonts.definers.contains(w) {
                    if name(at: i) != nil { i += 1 } // the body, a group, is scanned as code
                    continue
                }
                if UnicodeFonts.commandIfs.contains(w) || (w.hasPrefix("if") && group(at: i) != nil) {
                    // etoolbox's `\ifdef{\setmainfont}{T}{F}`: the first argument is
                    // a reference; a branch taken only when a Unicode-only name is
                    // defined is not read.
                    guard let n = name(at: i) else { continue }
                    i += 1
                    let unicode = UnicodeFonts.unicodeNames.contains(n)
                    let skipTrue = unicode && ["ifdef", "ifcsdef", "ifdefmacro", "ifcsmacro"].contains(w)
                    let skipFalse = unicode && ["ifundef", "ifcsundef"].contains(w)
                    if let tg = group(at: i) {
                        i += 1
                        if !skipTrue, let need = scanGroup(tg, &self) { return need }
                        if let fg = group(at: i) {
                            i += 1
                            if !skipFalse, let need = scanGroup(fg, &self) { return need }
                        }
                    }
                    continue
                }
                switch w {
                case "@ifpackageloaded", "IfPackageLoadedTF", "IfPackageLoadedT", "IfPackageLoadedF", "@ifclassloaded",
                     "@ifundefined":
                    // `\@ifpackageloaded{fontspec}{T}{F}`: T runs only once such a
                    // package is loaded (that load is the use, read where it is)
                    guard let arg = group(at: i) else { continue }
                    i += 1
                    let n = arg.trimmingCharacters(in: .whitespacesAndNewlines)
                    let loadedOnly = w != "@ifundefined" && UnicodeFonts.packages.contains(n)
                    let definedOnly = w == "@ifundefined" && UnicodeFonts.unicodeNames.contains(n)
                    let hasTrue = w != "IfPackageLoadedF"
                    let hasFalse = w != "IfPackageLoadedT"
                    if hasTrue, let tg = group(at: i) {
                        i += 1
                        if !loadedOnly, let need = scanGroup(tg, &self) { return need }
                    }
                    if hasFalse, let fg = group(at: i) {
                        i += 1
                        if !definedOnly, let need = scanGroup(fg, &self) { return need }
                    }
                case "usepackage", "RequirePackage", "RequirePackageWithOptions":
                    var j = i
                    skipOptions(&j)
                    guard let arg = group(at: j) else { continue }
                    i = j + 1
                    for p in arg.split(separator: ",").map({ $0.trimmingCharacters(in: .whitespacesAndNewlines) }) where !p.isEmpty {
                        if UnicodeFonts.packages.contains(p) { return UnicodeFontsNeed(kind: .package(p), file: file) }
                        if let need = follow([p + ".sty"], depth: depth) { return need }
                    }
                case "documentclass", "LoadClass", "LoadClassWithOptions":
                    var j = i
                    skipOptions(&j)
                    guard let arg = group(at: j) else { continue }
                    i = j + 1
                    let c = arg.trimmingCharacters(in: .whitespacesAndNewlines)
                    if !c.isEmpty, let need = follow([c + ".cls"], depth: depth) { return need }
                case "input", "include", "InputIfFileExists", "subfile", "import", "subimport":
                    // `\input{pre}`, `\input pre` (the tokens give it as a group too),
                    // `\import{dir/}{file}`
                    guard var f = group(at: i)?.trimmingCharacters(in: .whitespacesAndNewlines) else { continue }
                    i += 1
                    if w == "import" || w == "subimport", let g = group(at: i) {
                        i += 1
                        f = (f as NSString).appendingPathComponent(g.trimmingCharacters(in: .whitespacesAndNewlines))
                    }
                    guard !f.isEmpty else { continue }
                    let candidates = (f as NSString).pathExtension.isEmpty ? [f + ".tex", f] : [f]
                    if let need = follow(candidates, depth: depth) { return need }
                case "font":
                    // XeTeX's `\font\x="Name"` / `\font\x=[file.otf]` (pdfTeX takes neither).
                    if word(at: i) != nil {
                        var j = i + 1
                        if j < toks.count, case .other("=") = toks[j] { j += 1 }
                        if j < toks.count, case .other(let c) = toks[j], c == "\"" || c == "[" {
                            return UnicodeFontsNeed(kind: .fontPrimitive, file: file)
                        }
                    }
                default:
                    if UnicodeFonts.commands.contains(w) { return UnicodeFontsNeed(kind: .command(w), file: file) }
                }
            }
            return nil
        }

        /// The first of `candidates` the project has, scanned once.
        mutating func follow(_ candidates: [String], depth: Int) -> UnicodeFontsNeed? {
            guard depth < UnicodeFonts.maxDepth else { return nil }
            for file in candidates {
                guard !visited.contains(file), let text = read(file) else { continue }
                visited.insert(file)
                // A class or package file is all preamble; an `\input` file of
                // the preamble is read up to a `\begin{document}` it may hold.
                let whole = file.hasSuffix(".sty") || file.hasSuffix(".cls")
                return scan(UnicodeFonts.tokens(text, preambleOnly: !whole), file: file, depth: depth + 1)
            }
            return nil
        }
    }

    // MARK: tokens

    enum Token: Equatable {
        /// A control word (`\usepackage` → `usepackage`; `@` is a letter, as in
        /// package files).
        case word(String)
        /// A `{…}` group's text (balanced, comments removed); also the file
        /// name of `\input file` without braces.
        case group(String)
        /// Any other character that matters here (`[`, `]`, `=`, `"`).
        case other(Character)
    }

    /// The text as control words, brace groups and the few characters the
    /// scan reads; comments (`%` to the end of the line, unless escaped) are
    /// dropped. With `preambleOnly`, it stops at `\begin{document}` (nothing
    /// after it is read).
    static func tokens(_ text: String, preambleOnly: Bool) -> [Token] {
        var out: [Token] = []
        var s = Array(text.unicodeScalars)
        if s.first == "\u{FEFF}" { s.removeFirst() }
        var i = 0
        func isLetter(_ c: Unicode.Scalar) -> Bool { (c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c == "@" }
        func isSpace(_ c: Unicode.Scalar) -> Bool { c == " " || c == "\t" || c == "\n" || c == "\r" }
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
                    let w = String(String.UnicodeScalarView(s[start ..< i]))
                    out.append(.word(w))
                    if w == "input" {
                        // `\input file` (TeX's own syntax): the name, as a group
                        var j = i
                        while j < s.count, isSpace(s[j]) { j += 1 }
                        if j < s.count, s[j] != "{", s[j] != "\\", s[j] != "%" {
                            let n = j
                            while j < s.count, !isSpace(s[j]), s[j] != "\\", s[j] != "%", s[j] != "}" { j += 1 }
                            out.append(.group(String(String.UnicodeScalarView(s[n ..< j]))))
                            i = j
                        }
                    }
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
                let group = String(g)
                if preambleOnly, group == "document", out.last == .word("begin") { out.removeLast(); return out }
                out.append(.group(group))
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
        // fontspec (fatal), so also polyglossia, mathspec, …: TeX breaks the
        // message after "or" ("…requires either XeTeX or" / "(fontspec) LuaTeX.")
        "package requires either XeTeX or",
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

// MARK: - project files

/// The project files the scan reads, by path and modification date, so that
/// an unchanged `.sty` is not read again at each check. Thread-safe: the
/// checks after edits run off the main thread.
final class UnicodeFontsFileCache: @unchecked Sendable {
    private let lock = NSLock()
    private var files: [String: (date: Date, text: String?)] = [:]

    /// The text of the file at `url` (UTF-8, under 4 MiB), nil when there is none.
    func text(_ url: URL) -> String? {
        let path = url.path
        guard let attrs = try? FileManager.default.attributesOfItem(atPath: path),
              (attrs[.type] as? FileAttributeType) == .typeRegular,
              let size = attrs[.size] as? Int, size < 4 << 20 else { return nil }
        let date = attrs[.modificationDate] as? Date ?? .distantPast
        lock.lock()
        if let e = files[path], e.date == date { lock.unlock(); return e.text }
        lock.unlock()
        let text = try? String(contentsOf: url, encoding: .utf8)
        lock.lock()
        files[path] = (date, text)
        lock.unlock()
        return text
    }
}

extension UnicodeFonts {
    /// The scan of an entry document in a project (pure but for reading the
    /// project's files under `root`; callable off the main thread):
    /// `open` holds the editor's texts by project path, which win over disk.
    static func scan(entryPath: String, entryText: String, open: [String: String], root: URL?,
                     cache: UnicodeFontsFileCache) -> UnicodeFontsNeed? {
        let entryDir = (entryPath as NSString).deletingLastPathComponent
        return scan(entry: entryText, entryName: (entryPath as NSString).lastPathComponent) { name in
            let joined = entryDir.isEmpty ? name : (entryDir as NSString).appendingPathComponent(name)
            let rel = (joined as NSString).standardizingPath
            guard !rel.hasPrefix("/"), !rel.hasPrefix("..") else { return nil }
            if let t = open[rel] { return t }
            guard let root else { return nil }
            return cache.text(root.appendingPathComponent(rel))
        }
    }
}

// MARK: - the window

extension ShellModel {
    /// The open document's need for a Unicode engine from its preamble
    /// (`UnicodeFonts.scan`): the entry's text as the editor has it, and the
    /// project files it loads there (an open buffer, else the file under the
    /// project root; never outside it). Cached by `documentsRevision`; the
    /// checks after edits fill the cache off the main thread
    /// (`scheduleUnicodeFontsCheck`), so this scans here only at an open.
    func unicodeFontsNeed() -> UnicodeFontsNeed? {
        let entry = project.entryPath
        if let s = unicodeFontsScan, s.revision == documentsRevision, s.entry == entry { return s.need }
        let need = UnicodeFonts.scan(entryPath: entry, entryText: entryText, open: unicodeFontsOpenTexts(),
                                     root: project.projectRoot, cache: unicodeFontsFiles)
        unicodeFontsScan = (documentsRevision, entry, need)
        return need
    }

    private func unicodeFontsOpenTexts() -> [String: String] {
        Dictionary(documents.map { ($0.path, $0.text) }, uniquingKeysWith: { a, _ in a })
    }

    /// An edit may have added or removed what makes the document need a
    /// Unicode engine: scan again once typing pauses (0.8 s), off the main
    /// thread, and only where the rule can apply (the new engine preferred,
    /// not forced, not the user's own choice). Cheap per keystroke: one
    /// timer, re-armed.
    func scheduleUnicodeFontsCheck() {
        let c = engineChoice
        guard c.preferred == .new, !c.isForced, c.source != .user, !engineChoicePending else { return }
        unicodeFontsCheck?.cancel()
        let work = DispatchWorkItem { [weak self] in
            MainActor.assumeIsolated { self?.startUnicodeFontsScan() }
        }
        unicodeFontsCheck = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.8, execute: work)
    }

    /// The scan for the current texts on a utility queue, then the rules on
    /// the main thread (`recheckUnicodeFonts`), unless the texts changed meanwhile.
    func startUnicodeFontsScan(completion: (@MainActor () -> Void)? = nil) {
        unicodeFontsCheck = nil
        let revision = documentsRevision, entry = project.entryPath, text = entryText
        let open = unicodeFontsOpenTexts(), root = project.projectRoot, cache = unicodeFontsFiles
        DispatchQueue.global(qos: .utility).async {
            let need = UnicodeFonts.scan(entryPath: entry, entryText: text, open: open, root: root, cache: cache)
            DispatchQueue.main.async { [weak self] in
                MainActor.assumeIsolated {
                    guard let self else { return }
                    if self.documentsRevision == revision, self.project.entryPath == entry {
                        self.unicodeFontsScan = (revision, entry, need)
                        self.recheckUnicodeFonts()
                    }
                    completion?()
                }
            }
        }
    }

    /// The rules again for the open document after an edit (as
    /// `manifestDidRefresh` after a manifest change): a document that now
    /// needs Unicode fonts falls back, announced; one that no longer does
    /// returns to the new engine, announced too (never silently). The
    /// engine's report is dropped once the scan no longer finds what it
    /// found when the report came (the user deleted `\usepackage{fontspec}`).
    func recheckUnicodeFonts() {
        unicodeFontsCheck = nil
        guard !engineChoicePending, engineChoiceDocument == documentURL else { return }
        if let r = engineHostNeedsUnicode, r.document == documentURL, unicodeFontsNeed() != r.scanned {
            engineHostNeedsUnicode = nil
        }
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
    /// choice was forced or is the user's own for this document. The report
    /// holds until the document is opened again, the user chooses an engine,
    /// or the scan stops finding what it found now.
    func engineV3NeedsUnicodeFonts(_ need: UnicodeFontsNeed) {
        let c0 = engineChoice
        guard !c0.isForced, c0.preferred == .new, c0.source != .user, c0.blocker == nil else { return }
        engineHostNeedsUnicode = (documentURL, need, unicodeFontsNeed())
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
