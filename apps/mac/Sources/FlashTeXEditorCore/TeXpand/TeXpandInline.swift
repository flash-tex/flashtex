import Foundation

// Tier B (ligatures) and tier C (postfix, fractions): PLAN §6.2, §6.3,
// §9.3, §9.4. Definitions, the backward atom parser, and the pure matchers
// the capture controller calls.

extension TeXpand {
    /// A compiled regular expression that can live in the `Sendable` registry.
    public struct Pattern: @unchecked Sendable, Equatable {
        public let source: String
        let regex: NSRegularExpression
        init(_ source: String) throws {
            self.source = source
            regex = try NSRegularExpression(pattern: source)
        }
        public static func == (a: Pattern, b: Pattern) -> Bool { a.source == b.source }
    }

    /// `[[ligature]]`: a literal `trigger` or a `regex` ending at the caret.
    public struct Ligature: Equatable, Sendable {
        public var trigger: String?
        /// Matched against the text before the caret; anchored at the end.
        public var regex: Pattern?
        /// Replacement; `$1`…`$9` are the regex groups (backslashes are literal).
        public var body: String
        public var scopes: [String]
        /// Must match right before the trigger (`(?<![A-Za-z\\])`).
        public var guardBefore: Pattern?
        /// For `disable` (regex ligatures have no trigger to name them by).
        public var name: String?
        public var requires: [PackageRequirement]
        public var layer: String
        public var line: Int

        var key: String { trigger ?? name ?? ("re:" + (regex?.source ?? "")) }
    }

    /// `[[postfix]]`: `atom.name` Tab → `body` with `<<atom>>` replaced.
    public struct Postfix: Equatable, Sendable {
        public var name: String
        public var body: String
        public var stripParens: Bool
        public var scopes: [String]
        public var requires: [PackageRequirement]
        public var layer: String
        public var line: Int
    }

    // MARK: the backward atom parser (§9.4)

    /// Where the math atom ending at `end` starts, scanning no further back
    /// than `limit`: `base {script}`, where a base is a letter, a digit run,
    /// a control sequence with its brace groups, a balanced `( )`/`[ ]`/`{ }`
    /// or a `\left … \right` pair, and a script is `_`/`^` with one token
    /// or a brace group. Nil when no atom ends there.
    public static func atomStart(in text: NSString, end: Int, limit: Int = 0) -> Int? {
        let lim = max(0, limit)
        guard end > lim, end <= text.length else { return nil }
        var e = end
        // Scripts, right to left.
        while let t = tokenStart(text, e, lim), t > lim, text.character(at: t - 1) == 0x5F || text.character(at: t - 1) == 0x5E {
            e = t - 1
        }
        return baseStart(text, e, lim)
    }

    static func isLetter(_ c: unichar) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
    static func isDigit(_ c: unichar) -> Bool { c >= 0x30 && c <= 0x39 }

    /// A script's operand ending at `e`: a brace group, a control word, or one character.
    static func tokenStart(_ text: NSString, _ e: Int, _ lim: Int) -> Int? {
        guard e > lim else { return nil }
        let c = text.character(at: e - 1)
        if c == 0x7D { return matchOpen(text, e - 1, lim, open: 0x7B, close: 0x7D) }
        if isLetter(c) {
            var r = e - 1
            while r > lim, isLetter(text.character(at: r - 1)) { r -= 1 }
            if r > lim, text.character(at: r - 1) == 0x5C { return r - 1 }
            return e - 1
        }
        if c == 0x20 || c == 0x0A || c == 0x09 || c == 0x5F || c == 0x5E { return nil }
        return e - 1
    }

    /// The opener matching the closer at `close` (index), or nil.
    static func matchOpen(_ text: NSString, _ closeAt: Int, _ lim: Int, open: unichar, close: unichar) -> Int? {
        var depth = 0
        var k = closeAt
        while k >= lim {
            let c = text.character(at: k)
            let escaped = k > lim && text.character(at: k - 1) == 0x5C && !(k > lim + 1 && text.character(at: k - 2) == 0x5C)
            if !escaped {
                if c == close { depth += 1 } else if c == open { depth -= 1; if depth == 0 { return k } }
            }
            k -= 1
        }
        return nil
    }

    static func baseStart(_ text: NSString, _ e: Int, _ lim: Int) -> Int? {
        guard e > lim else { return nil }
        let c = text.character(at: e - 1)
        // `\left( … \right)`
        if let right = rightDelimiterStart(text, e, lim) { return matchLeft(text, right, lim) }
        switch c {
        case 0x29: return matchOpen(text, e - 1, lim, open: 0x28, close: 0x29) // ( )
        case 0x5D: return matchOpen(text, e - 1, lim, open: 0x5B, close: 0x5D) // [ ]
        case 0x7D: // { }, with the groups and the control word before it
            guard var s = matchOpen(text, e - 1, lim, open: 0x7B, close: 0x7D) else { return nil }
            while s > lim, text.character(at: s - 1) == 0x7D || text.character(at: s - 1) == 0x5D {
                let closer = text.character(at: s - 1)
                guard let o = matchOpen(text, s - 1, lim, open: closer == 0x7D ? 0x7B : 0x5B, close: closer) else { break }
                s = o
            }
            var r = s
            while r > lim, isLetter(text.character(at: r - 1)) { r -= 1 }
            if r < s, r > lim, text.character(at: r - 1) == 0x5C { return r - 1 }
            return s
        default:
            if isLetter(c) {
                var r = e - 1
                while r > lim, isLetter(text.character(at: r - 1)) { r -= 1 }
                if r > lim, text.character(at: r - 1) == 0x5C { return r - 1 } // \alpha
                return e - 1 // one letter: `ab.hat` is `a\hat{b}`
            }
            if isDigit(c) {
                var r = e - 1
                while r > lim, isDigit(text.character(at: r - 1)) { r -= 1 }
                return r
            }
            return nil
        }
    }

    /// If `[.., e)` ends with `\right<delim>`, where `\right` starts.
    static func rightDelimiterStart(_ text: NSString, _ e: Int, _ lim: Int) -> Int? {
        for delimLength in [1, 2, 7, 8] where e - delimLength - 6 >= lim { // `)`, `\|`, `\rangle`, `\rfloor`…
            let at = e - delimLength - 6
            if text.substring(with: NSRange(location: at, length: 6)) == "\\right" {
                let delim = text.substring(with: NSRange(location: at + 6, length: delimLength))
                if delimLength == 1 || delim.hasPrefix("\\") { return at }
            }
        }
        return nil
    }

    /// The `\left` matching the `\right` at `right`.
    static func matchLeft(_ text: NSString, _ right: Int, _ lim: Int) -> Int? {
        var depth = 0
        var k = right
        while k >= lim {
            if text.character(at: k) == 0x5C {
                if k + 6 <= text.length, text.substring(with: NSRange(location: k, length: 6)) == "\\right" { depth += 1 }
                if k + 5 <= text.length, text.substring(with: NSRange(location: k, length: 5)) == "\\left" {
                    depth -= 1
                    if depth == 0 { return k }
                }
            }
            k -= 1
        }
        return nil
    }

    /// `(x+1)` → `x+1` (and a bare group `{x+1}` → `x+1`) when the outer
    /// pair matches itself.
    static func stripParens(_ s: String) -> String {
        let ns = s as NSString
        guard ns.length >= 2 else { return s }
        for (open, close) in [(unichar(0x28), unichar(0x29)), (unichar(0x7B), unichar(0x7D))]
        where ns.character(at: 0) == open && ns.character(at: ns.length - 1) == close
            && matchOpen(ns, ns.length - 1, 0, open: open, close: close) == 0 {
            return ns.substring(with: NSRange(location: 1, length: ns.length - 2))
        }
        return s
    }
}

// MARK: - loading

extension TeXpand.Ligature {
    typealias T = TeXpand
    static func load(_ t: T.TOMLTable, layer: String, diagnostics: inout [T.Diagnostic]) -> T.Ligature? {
        let label = t["trigger"]?.string ?? t["name"]?.string
        func err(_ m: String) { diagnostics.append(T.Diagnostic(severity: .error, layer: layer, line: t.line, definition: label, message: m)) }
        for key in t.keys where !["trigger", "regex", "body", "scope", "guard_before", "name", "requires"].contains(key) {
            diagnostics.append(T.Diagnostic(severity: .warning, layer: layer, line: t.line, definition: label, message: "unknown key `\(key)`"))
        }
        guard let body = t["body"]?.string else { err("a ligature needs a `body`"); return nil }
        let trigger = t["trigger"]?.string
        var regex: T.Pattern?
        if let r = t["regex"]?.string {
            let anchored = r.hasSuffix("$") ? r : r + "$"
            do { regex = try T.Pattern(anchored) } catch { err("`regex` does not compile: \(r)"); return nil }
        }
        guard (trigger == nil) != (regex == nil) else { err("a ligature has a `trigger` or a `regex` (one of them)"); return nil }
        if let trigger, trigger.utf16.count < 2 { err("a trigger is at least two characters"); return nil }
        var guardBefore: T.Pattern?
        if let g = t["guard_before"]?.string {
            do { guardBefore = try T.Pattern(g + "$") } catch { err("`guard_before` does not compile: \(g)"); return nil }
        }
        var scopes = ["math"]
        if let s = t["scope"] { scopes = s.string.map { [$0] } ?? s.array?.compactMap(\.string) ?? scopes }
        var failed = false
        let requires = T.Definition.requirements(t["requires"]) { err($0); failed = true }
        if failed { return nil }
        return T.Ligature(trigger: trigger, regex: regex, body: body, scopes: scopes, guardBefore: guardBefore,
                          name: t["name"]?.string, requires: requires, layer: layer, line: t.line)
    }
}

extension TeXpand.Postfix {
    typealias T = TeXpand
    static func load(_ t: T.TOMLTable, layer: String, diagnostics: inout [T.Diagnostic]) -> T.Postfix? {
        let name = t["name"]?.string ?? ""
        func err(_ m: String) { diagnostics.append(T.Diagnostic(severity: .error, layer: layer, line: t.line, definition: name.isEmpty ? nil : name, message: m)) }
        for key in t.keys where !["name", "body", "strip_parens", "scope", "requires"].contains(key) {
            diagnostics.append(T.Diagnostic(severity: .warning, layer: layer, line: t.line, definition: name, message: "unknown key `\(key)`"))
        }
        guard !name.isEmpty, name.utf16.allSatisfy(T.Parser.isLetter) else { err("a postfix needs a `name` of letters"); return nil }
        guard let body = t["body"]?.string else { err("a postfix needs a `body`"); return nil }
        guard body.contains("<<atom>>") else { err("a postfix `body` needs `<<atom>>`"); return nil }
        do { _ = try T.Template(body.replacingOccurrences(of: "<<atom>>", with: "x")) } catch let e as T.Template.TemplateError {
            err("`body`: \(e.message)"); return nil
        } catch { err("`body`: \(error)"); return nil }
        var scopes = ["math"]
        if let s = t["scope"] { scopes = s.string.map { [$0] } ?? s.array?.compactMap(\.string) ?? scopes }
        var failed = false
        let requires = T.Definition.requirements(t["requires"]) { err($0); failed = true }
        if failed { return nil }
        return T.Postfix(name: name, body: body, stripParens: t["strip_parens"]?.bool ?? false, scopes: scopes,
                         requires: requires, layer: layer, line: t.line)
    }
}

// MARK: - rendering a free-standing template (postfix bodies)

extension TeXpand.Engine {
    /// Renders `source` (holes allowed: tabstops, `<<0>>`, `<<profile.KEY>>`)
    /// outside any abbreviation, for postfix bodies.
    public func render(template source: String, in ctx: TeXpand.Context) -> TeXpand.Snippet? {
        guard let template = try? TeXpand.Template(source) else { return nil }
        var x = Expander(registry: registry, profile: registry.profile, ctx: ctx)
        let def = TeXpand.Definition(name: "postfix", scopes: ["math"], pack: "", layer: "", line: 0)
        let inputs = Expander.Inputs(def: def, element: TeXpand.Element(name: "postfix"), instance: 1, params: [:], args: [],
                                     children: nil, label: nil, wrapped: nil, counter: nil)
        guard let raw = try? x.render(template, inputs) else { return nil }
        return Expander.finalize(raw)
    }

    /// A ligature's replacement: `$n` groups filled, plus the profile's
    /// trailing space after a control word (`\to ` so `a\to b` stays apart).
    func ligatureBody(_ l: TeXpand.Ligature, groups: [String]) -> String {
        var out = ""
        var chars = Array(l.body)
        var i = 0
        while i < chars.count {
            if chars[i] == "$", i + 1 < chars.count, let n = chars[i + 1].wholeNumberValue {
                out += n < groups.count ? groups[n] : ""
                i += 2
                continue
            }
            out.append(chars[i])
            i += 1
        }
        chars = []
        if registry.profile["ligature_trailing_space"] != "false", !out.hasSuffix(" "),
           let range = out.range(of: #"\\[A-Za-z]+$"#, options: .regularExpression), !range.isEmpty {
            out += " "
        }
        return out
    }
}
