import Foundation

// Tiers B and C inside the capture controller: the ligature matcher with
// deferred commit (§9.3), postfix and fractions (§9.4). Each returns the
// commit for the host to apply; none edits text.

extension TeXpand.CaptureController {
    typealias T = TeXpand

    /// After a typed character (the caret just after it), outside a capture.
    func inlineStep(caret: Int, text: NSString) -> Output? {
        let scope = scopeAt(caret - 1, text)
        let flags = scope.flags
        guard !flags.contains("verbatim"), !flags.contains("comment") else { pendingLigature = nil; return nil }
        if postfixOn, engine.settings.fractionTrigger == .auto, flags.contains("math"),
           let commit = fractionCommit(caret: caret, text: text, scope: scope, typed: true) {
            pendingLigature = nil
            var out = Output()
            out.commit = commit
            return out
        }
        if ligaturesOn { return ligatureStep(caret: caret, text: text, flags: flags) }
        return nil
    }

    // MARK: ligatures (§9.3)

    func activeLigatures(_ flags: Set<String>) -> [T.Ligature] {
        engine.registry.ligatures.filter { !Set($0.scopes).isDisjoint(with: flags) }
    }

    func ligatureStep(caret: Int, text: NSString, flags: Set<String>) -> Output? {
        let ligatures = activeLigatures(flags)
        let triggers = ligatures.filter { $0.trigger != nil }
        func str(_ a: Int, _ b: Int) -> String { text.substring(with: NSRange(location: a, length: b - a)) }

        // A waiting trigger: extend it, fire the longer one, or settle it.
        if let pend = pendingLigature {
            let typed = str(pend.start, caret)
            let extending = triggers.filter { $0.trigger!.hasPrefix(typed) }
            if !extending.isEmpty {
                if let exact = extending.first(where: { $0.trigger == typed }), guardAllows(exact, start: pend.start, text: text) {
                    if extending.count > 1 {
                        pendingLigature = (pend.start, caret, (typed as NSString).length, exact)
                        return nil
                    }
                    pendingLigature = nil
                    return fire(exact, NSRange(location: pend.start, length: caret - pend.start), groups: [], text: text)
                }
                pendingLigature = (pend.start, caret, pend.length, pend.ligature)
                return nil
            }
            // Not heading for a longer trigger: the waiting one fires in place
            // and the new character stays after it.
            pendingLigature = nil
            return fire(pend.ligature, NSRange(location: pend.start, length: pend.length), groups: [], text: text)
        }

        let lineStart = Self.lineStart(caret, text)
        // Longest trigger ending at the caret.
        let sorted = triggers.sorted { ($0.trigger! as NSString).length > ($1.trigger! as NSString).length }
        for lig in sorted {
            let t = lig.trigger!
            let n = (t as NSString).length
            let start = caret - n
            guard start >= lineStart, str(start, caret) == t else { continue }
            let r = NSRange(location: start, length: n)
            guard !isSuppressed(r), guardAllows(lig, start: start, text: text) else { continue }
            if triggers.contains(where: { $0.trigger != t && $0.trigger!.hasPrefix(t) }) {
                pendingLigature = (start, caret, n, lig) // deferred: `<=` may become `<=>`
                return nil
            }
            return fire(lig, r, groups: [], text: text)
        }
        // Regex ligatures, only when no trigger fired.
        let line = str(lineStart, caret)
        for lig in ligatures {
            guard let re = lig.regex?.regex,
                  let m = re.firstMatch(in: line, range: NSRange(location: 0, length: (line as NSString).length)) else { continue }
            let r = NSRange(location: lineStart + m.range.location, length: m.range.length)
            guard r.length > 0, !isSuppressed(r) else { continue }
            let groups = (0..<m.numberOfRanges).map { k -> String in
                let g = m.range(at: k)
                return g.location == NSNotFound ? "" : (line as NSString).substring(with: g)
            }
            return fire(lig, r, groups: groups, text: text)
        }
        return nil
    }

    func guardAllows(_ lig: T.Ligature, start: Int, text: NSString) -> Bool {
        guard let re = lig.guardBefore?.regex else { return true }
        let ls = Self.lineStart(start, text)
        let before = text.substring(with: NSRange(location: ls, length: start - ls))
        return re.firstMatch(in: before, range: NSRange(location: 0, length: (before as NSString).length)) != nil
    }

    func isSuppressed(_ r: NSRange) -> Bool {
        suppressed.contains { NSIntersectionRange($0, r).length > 0 || $0.location == r.location }
    }

    func fire(_ lig: T.Ligature, _ range: NSRange, groups: [String], text: NSString) -> Output {
        let body = engine.ligatureBody(lig, groups: groups)
        let literal = text.substring(with: range)
        lastCommit = (range.location, literal)
        var out = Output()
        out.commit = Commit(range: range, literal: literal, snippet: LaTeXSnippet(text: body, caretUTF16: (body as NSString).length),
                            requires: lig.requires, inline: true)
        return out
    }

    // MARK: postfix and fractions (§9.4)

    /// `x.hat`, `(x+1).sqrt`, `\alpha_i.hat`, or a fraction, ending at the caret.
    func postfixCommit(caret: Int, text: NSString) -> Commit? {
        let scope = scopeAt(caret, text)
        let flags = scope.flags
        guard !flags.contains("verbatim"), !flags.contains("comment") else { return nil }
        let limit = Self.mathLimit(scope, caret: caret, text: text)
        var ls = caret
        while ls > limit, T.isLetter(text.character(at: ls - 1)) { ls -= 1 }
        if ls < caret, ls - 1 > limit, text.character(at: ls - 1) == 0x2E {
            let name = text.substring(with: NSRange(location: ls, length: caret - ls))
            if let pf = engine.registry.postfixes[name], !Set(pf.scopes).isDisjoint(with: flags),
               let s = T.atomStart(in: text, end: ls - 1, limit: limit), s < ls - 1 {
                var atom = text.substring(with: NSRange(location: s, length: ls - 1 - s))
                if pf.stripParens { atom = T.stripParens(atom) }
                let source = pf.body.replacingOccurrences(of: "<<atom>>", with: atom.replacingOccurrences(of: "<<", with: "\\<<"))
                let ctx = T.Context(scope: scope, packages: packages(), documentClass: documentClass(),
                                    indentUnit: indentUnit, baseIndent: Self.lineIndent(at: s, text: text))
                if let snippet = engine.render(template: source, in: ctx) {
                    let range = NSRange(location: s, length: caret - s)
                    let literal = text.substring(with: range)
                    lastCommit = (s, literal)
                    return Commit(range: range, literal: literal,
                                  snippet: snippet.flattened(baseIndent: ctx.baseIndent, indentUnit: indentUnit).latexSnippet,
                                  requires: pf.requires)
                }
            }
        }
        guard flags.contains("math"), engine.settings.fractionTrigger != .off else { return nil }
        return fractionCommit(caret: caret, text: text, scope: scope, typed: false)
    }

    /// `a//b` → `\frac{a}{b}`; `a//` → `\frac{a}{|}`. The operator is the
    /// owner's `fraction_operator` (`//` by default, so `a/b` never
    /// changes). `typed`: the auto trigger, right after the operator.
    func fractionCommit(caret: Int, text: NSString, scope: T.ScopeStack, typed: Bool) -> Commit? {
        let op = engine.settings.fractionOperator
        let n = (op as NSString).length
        let limit = Self.mathLimit(scope, caret: caret, text: text)
        func str(_ a: Int, _ b: Int) -> String { text.substring(with: NSRange(location: a, length: b - a)) }
        var numEnd: Int
        var den: NSRange?
        if caret - n >= limit, str(caret - n, caret) == op {
            numEnd = caret - n
        } else {
            guard !typed, let ds = T.atomStart(in: text, end: caret, limit: limit), ds < caret,
                  ds - n >= limit, str(ds - n, ds) == op else { return nil }
            numEnd = ds - n
            den = NSRange(location: ds, length: caret - ds)
        }
        // With `/` as the operator, the second slash of `//` is not one.
        if op == "/", numEnd > limit, text.character(at: numEnd - 1) == 0x2F { return nil }
        guard let ns = T.atomStart(in: text, end: numEnd, limit: limit), ns < numEnd else { return nil }
        let frac = engine.registry.profile["frac"] ?? "\\frac"
        let num = T.stripParens(str(ns, numEnd))
        let snippet: LaTeXSnippet
        if let den {
            let s = frac + "{" + num + "}{" + T.stripParens(text.substring(with: den)) + "}"
            snippet = LaTeXSnippet(text: s, caretUTF16: (s as NSString).length)
        } else {
            let s = frac + "{" + num + "}{}"
            let length = (s as NSString).length
            snippet = LaTeXSnippet(text: s, caretUTF16: length - 1, stops: [length])
        }
        let range = NSRange(location: ns, length: caret - ns)
        let literal = text.substring(with: range)
        lastCommit = (ns, literal)
        return Commit(range: range, literal: literal, snippet: snippet, requires: [])
    }

    // MARK: helpers

    static func lineStart(_ at: Int, _ text: NSString) -> Int {
        var s = min(at, text.length)
        while s > 0, text.character(at: s - 1) != 0x0A { s -= 1 }
        return s
    }

    /// How far back an atom may reach: the current line, and not past the
    /// start of the innermost math region.
    static func mathLimit(_ scope: T.ScopeStack, caret: Int, text: NSString) -> Int {
        let ls = lineStart(caret, text)
        let math = scope.frames.last { f in
            switch f.kind {
            case .inlineMath, .displayMath: return true
            case .environment(let name):
                let base = name.hasSuffix("*") ? String(name.dropLast()) : name
                return T.ScopeStack.environmentClasses[base]?.contains("math") == true || SyntaxHighlighter.mathEnvironments.contains(name)
            default: return false
            }
        }
        guard let body = math?.bodyStart, body <= caret else { return ls }
        return max(ls, body)
    }
}
