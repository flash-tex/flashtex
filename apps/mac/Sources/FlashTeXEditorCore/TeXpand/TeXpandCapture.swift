import Foundation

extension TeXpand {
    /// The tier A capture state machine (PLAN §9.1), editor-agnostic: the
    /// host reports edits, caret moves, Tab, Esc and focus loss; each call
    /// returns what to show (the capture region, a preview) and, on Tab, the
    /// expansion to apply. The host applies a commit as one undo step and
    /// starts its snippet session; the controller never edits text.
    ///
    ///     Idle ──leader typed──▶ Armed ──letter typed──▶ Capturing
    ///       ▲                      │ anything else          │ Tab (Ok): commit
    ///       └──────────────────────┴────────────────────────┤ Esc: literal + suppression mark
    ///                                                       │ Invalid / prefix miss / newline /
    ///                                                       │ caret leaves / focus lost: cancel
    ///
    /// Additions to §9.1 (PLAN "Adaptations"): Tab while idle expands the
    /// `leader+abbreviation` just before the caret (so an abbreviation typed
    /// before the capture was lost, or with an empty leader, still expands)
    /// unless a suppression mark sits on it — which is what Esc and
    /// undo-to-literal leave behind.
    public final class CaptureController {
        public enum State: Equatable, Sendable {
            case idle
            /// The leader was typed at `leader`.
            case armed(leader: Int)
            /// Capturing `[leader, end)`: the leader plus the abbreviation.
            case capturing(leader: Int, end: Int)
        }

        public enum EditKind: Equatable, Sendable {
            /// A keystroke the user typed.
            case typed
            /// The host changed the text (a completion, a paste, a commit).
            case programmatic
            /// Undo or redo.
            case undo
        }

        /// What to show after an event. `region` and `preview` describe the
        /// whole current state: nil clears them.
        public struct Output: Equatable, Sendable {
            /// The key (Tab, Esc) was handled; do not pass it on.
            public var consumed = false
            /// The capture region to highlight.
            public var region: NSRange?
            /// Ghost text: the expansion as it would be inserted.
            public var preview: String?
            /// Why Tab did not expand (`Incomplete`, an unknown name, …).
            public var diagnostic: String?
            /// Apply this: replace `range` with `snippet` as one undo step.
            public var commit: Commit?

            public init() {}
        }

        public struct Commit: Equatable, Sendable {
            /// The literal `leader+abbreviation` to replace.
            public var range: NSRange
            public var literal: String
            public var snippet: LaTeXSnippet
            /// Packages the expansion needs that the document lacks (M6 inserts them).
            public var requires: [PackageRequirement]
            /// Replace the text and keep the caret where it is (shifted by
            /// the change): instant atoms and ligatures, which fire on the
            /// keystroke after them. Otherwise start a snippet session.
            public var inline = false
        }

        public private(set) var state: State = .idle
        public var engine: Engine
        /// The host's indent unit, for multi-line expansions.
        public var indentUnit = "  "
        /// The project's document class (beamer overlays).
        public var documentClass: () -> String? = { nil }
        /// Packages the document loads (M6 fills this from the preamble).
        public var packages: () -> Set<String> = { [] }
        /// Regions (`leader+abbreviation`) Tab must not expand: left by Esc
        /// and by undoing a commit. Cleared when the caret leaves their
        /// line or the text in them changes.
        public private(set) var suppressed: [NSRange] = []

        let scopeAt: (Int, NSString) -> ScopeStack
        /// The scope where the capture started.
        var captureScope: ScopeStack?
        /// The last commit, for undo-to-literal: where it was and what it replaced.
        var lastCommit: (location: Int, literal: String)?
        /// A ligature whose trigger is a proper prefix of a longer one (`<=`
        /// before `<=>`): it waits for the next keystroke (§9.3). `end` is
        /// where the typed text has reached; `length` is the trigger's.
        var pendingLigature: (start: Int, end: Int, length: Int, ligature: Ligature)?

        public init(engine: Engine, scope: @escaping (Int, NSString) -> ScopeStack) {
            self.engine = engine
            scopeAt = scope
        }

        /// The master switch: off, the controller does nothing at all.
        public var isActive: Bool { engine.settings.enabled }
        var abbreviationsOn: Bool { engine.settings.isActive(.abbreviations) }
        var instantOn: Bool { engine.settings.isActive(.instantAtoms) }
        var ligaturesOn: Bool { engine.settings.isActive(.ligatures) }
        var postfixOn: Bool { engine.settings.isActive(.postfix) }
        var leader: String { engine.settings.leader }
        var leaderLength: Int { leader.utf16.count }

        /// Armed or capturing: the host holds back its completion popup.
        public var isCapturing: Bool { state != .idle }

        // MARK: events

        /// `range` (pre-edit coordinates) was replaced by `replacement`;
        /// `text` is the text after the edit.
        public func edited(_ range: NSRange, replacement: String, kind: EditKind, text: NSString) -> Output {
            let delta = replacement.utf16.count - range.length
            // Undo-to-literal: the commit's undo put its literal back.
            if kind == .undo, let c = lastCommit {
                let r = NSRange(location: c.location, length: c.literal.utf16.count)
                if NSMaxRange(r) <= text.length, text.substring(with: r) == c.literal {
                    lastCommit = nil
                    shiftMarks(range, delta)
                    suppressed.append(r)
                    state = .idle
                    return Output()
                }
            }
            shiftMarks(range, delta)
            // The commit's position follows edits before it (the commit's own
            // replacement starts there and leaves it put).
            if var c = lastCommit, NSMaxRange(range) <= c.location {
                c.location += delta
                lastCommit = c
            }
            guard isActive else { state = .idle; pendingLigature = nil; return Output() }
            // A pending ligature lives only as long as typing extends it.
            if let pend = pendingLigature, !(kind == .typed && range.length == 0 && range.location == pend.end) { pendingLigature = nil }
            let typedOne = kind == .typed && range.length == 0 && replacement.utf16.count == 1
            let out = captureStep(range, replacement: replacement, kind: kind, delta: delta, text: text)
            // Tiers B and C act on a typed character no capture holds.
            if state == .idle, out.commit == nil, typedOne, let inline = inlineStep(caret: range.location + 1, text: text) {
                return inline
            }
            return out
        }

        func captureStep(_ range: NSRange, replacement: String, kind: EditKind, delta: Int, text: NSString) -> Output {
            guard abbreviationsOn else { state = .idle; return Output() }
            switch state {
            case .idle:
                return startIfLeader(range, replacement, kind, text)
            case .armed(let p):
                if NSMaxRange(range) <= p {
                    state = .armed(leader: p + delta) // an edit before the leader
                    return Output()
                }
                let after = p + leaderLength
                guard kind == .typed, range.location == after, range.length == 0, let first = replacement.unicodeScalars.first else {
                    state = .idle
                    return startIfLeader(range, replacement, kind, text)
                }
                guard first.isASCII, CharacterSet.letters.contains(first) else {
                    state = .idle // Armed → Idle: the leader stays literal
                    return startIfLeader(range, replacement, kind, text)
                }
                state = .capturing(leader: p, end: after + replacement.utf16.count)
                return evaluate(text)
            case .capturing(let p, let end):
                if kind == .undo { return cancel() }
                if NSMaxRange(range) <= p {
                    state = .capturing(leader: p + delta, end: end + delta)
                    return evaluate(text)
                }
                if range.location < p + leaderLength { return cancel() } // the leader itself changed
                if range.location > end { return evaluate(text) }         // after the region
                // Instant atoms (§9.2): a non-letter right after `;a` commits it.
                if kind == .typed, range.location == end, range.length == 0, let first = replacement.unicodeScalars.first,
                   !(first.isASCII && CharacterSet.letters.contains(first)), let commit = instantCommit(leader: p, end: end, text: text) {
                    state = .idle
                    captureScope = nil
                    var out = Output()
                    out.commit = commit
                    // The character is processed normally: a leader arms again
                    // (at its pre-commit offset; the commit's edit shifts it).
                    if replacement == leader {
                        state = .armed(leader: end)
                        captureScope = scopeAt(end, text)
                    }
                    return out
                }
                if replacement.contains("\n") { return cancel() }         // a newline ends capture
                state = .capturing(leader: p, end: max(p + leaderLength, end + delta))
                return evaluate(text)
            }
        }

        /// The selection changed (after any edit it follows).
        public func cursorMoved(to selection: NSRange, text: NSString) -> Output {
            let caret = selection.location
            if let pend = pendingLigature, selection.length != 0 || caret != pend.end { pendingLigature = nil } // left literal
            // Marks clear when the caret leaves their line.
            suppressed.removeAll { !Self.sameLine($0.location, caret, text) }
            switch state {
            case .idle:
                return Output()
            case .armed(let p):
                if selection.length != 0 || caret != p + leaderLength { state = .idle }
                return Output()
            case .capturing(let p, let end):
                if selection.length != 0 || caret < p + leaderLength || caret > end { return cancel() }
                return evaluate(text)
            }
        }

        /// Tab with the caret at `selection`. `consumed == false`: the host
        /// goes on down its Tab precedence (snippet stops, indentation).
        public func tab(selection: NSRange, text: NSString) -> Output {
            guard isActive else { return Output() }
            if let pend = pendingLigature { // Tab settles a waiting ligature
                pendingLigature = nil
                var out = fire(pend.ligature, NSRange(location: pend.start, length: pend.length), groups: [], text: text)
                out.consumed = true
                return out
            }
            switch state {
            case .armed:
                state = .idle
                return Output()
            case .capturing(let p, let end):
                return commitOrExplain(leader: p, end: end, text: text, explain: true)
            case .idle:
                guard selection.length == 0 else { return Output() }
                if abbreviationsOn, let p = retroLeader(before: selection.location, text: text) {
                    captureScope = nil
                    let out = commitOrExplain(leader: p, end: selection.location, text: text, explain: false)
                    if out.commit != nil { return out }
                    state = .idle
                }
                // §9.5 step 3: a postfix or fraction before the caret.
                if postfixOn, let commit = postfixCommit(caret: selection.location, text: text) {
                    var out = Output()
                    out.consumed = true
                    out.commit = commit
                    return out
                }
                return Output()
            }
        }

        /// Esc: a capture ends, its text stays literal, and a suppression
        /// mark keeps Tab from expanding it.
        public func escape() -> Output {
            pendingLigature = nil // the trigger stays literal
            switch state {
            case .capturing(let p, let end):
                suppressed.append(NSRange(location: p, length: end - p))
                state = .idle
                var out = Output()
                out.consumed = true
                return out
            case .armed:
                state = .idle
                return Output()
            case .idle:
                return Output()
            }
        }

        /// Focus left the editor: capture cancels silently.
        public func focusLost() -> Output {
            pendingLigature = nil
            return cancel()
        }

        // MARK: internals

        func cancel() -> Output {
            state = .idle
            captureScope = nil
            return Output()
        }

        func startIfLeader(_ range: NSRange, _ replacement: String, _ kind: EditKind, _ text: NSString) -> Output {
            guard kind == .typed, !leader.isEmpty, replacement == leader else { return Output() }
            let p = range.location
            let scope = scopeAt(p, text)
            let flags = scope.flags
            guard !flags.contains("verbatim"), !flags.contains("comment") else { return Output() }
            state = .armed(leader: p)
            captureScope = scope
            return Output()
        }

        /// Re-parses the region; cancels on Invalid or a name no definition
        /// starts with; previews an Ok parse that expands.
        func evaluate(_ text: NSString) -> Output {
            guard case .capturing(let p, let end) = state else { return Output() }
            guard end <= text.length, p + leaderLength <= end,
                  text.substring(with: NSRange(location: p, length: leaderLength)) == leader else { return cancel() }
            let abbr = text.substring(with: NSRange(location: p + leaderLength, length: end - p - leaderLength))
            let ctx = context(at: p, text: text)
            // Prefix guard: a bare name must lead somewhere.
            if !abbr.isEmpty, abbr.unicodeScalars.allSatisfy({ $0.isASCII && CharacterSet.letters.contains($0) }),
               engine.registry.names(withPrefix: abbr, flags: ctx.scope.flags).isEmpty {
                return cancel()
            }
            var out = Output()
            out.region = NSRange(location: p, length: end - p)
            switch engine.parse(abbr, in: ctx) {
            case .failure(let e) where e.kind == .invalid:
                return cancel()
            case .failure:
                return out
            case .success(let ast):
                if case .success(let x) = engine.expand(ast, in: ctx) {
                    out.preview = x.snippet.flattened(baseIndent: ctx.baseIndent, indentUnit: indentUnit).text
                }
                return out
            }
        }

        func commitOrExplain(leader p: Int, end: Int, text: NSString, explain: Bool) -> Output {
            var out = Output()
            guard end <= text.length, p + leaderLength <= end else { return cancel() }
            let abbr = text.substring(with: NSRange(location: p + leaderLength, length: end - p - leaderLength))
            let ctx = context(at: p, text: text)
            switch engine.expand(abbr, in: ctx) {
            case .success(let x):
                let range = NSRange(location: p, length: end - p)
                let snippet = x.snippet.flattened(baseIndent: ctx.baseIndent, indentUnit: indentUnit).latexSnippet
                out.consumed = true
                out.commit = Commit(range: range, literal: text.substring(with: range), snippet: snippet, requires: x.requires)
                lastCommit = (p, text.substring(with: range))
                state = .idle
                captureScope = nil
            case .failure(let f):
                guard explain else { return Output() }
                if case .parse(let e) = f, e.kind == .invalid { return cancel() }
                out.consumed = true // Incomplete (or an expansion error): no Tab character
                out.diagnostic = f.message
                out.region = NSRange(location: p, length: end - p)
            }
            return out
        }

        /// The commit for instant atom `leader…end` (`;a`), if it is one here.
        func instantCommit(leader p: Int, end: Int, text: NSString) -> Commit? {
            guard instantOn, end <= text.length, p + leaderLength < end else { return nil }
            let name = text.substring(with: NSRange(location: p + leaderLength, length: end - p - leaderLength))
            let ctx = context(at: p, text: text)
            guard let def = engine.registry.resolve(name, flags: ctx.scope.flags), def.instant,
                  case .success(let x) = engine.expand(name, in: ctx) else { return nil }
            let body = x.snippet.flattened(baseIndent: ctx.baseIndent, indentUnit: indentUnit).text
            let range = NSRange(location: p, length: end - p)
            let literal = text.substring(with: range)
            lastCommit = (p, literal)
            return Commit(range: range, literal: literal, snippet: LaTeXSnippet(text: body, caretUTF16: (body as NSString).length),
                          requires: x.requires, inline: true)
        }

        func context(at p: Int, text: NSString) -> Context {
            let scope = captureScope ?? scopeAt(p, text)
            return Context(scope: scope, packages: packages(), documentClass: documentClass(),
                           indentUnit: indentUnit, baseIndent: Self.lineIndent(at: p, text: text))
        }

        /// `leader+abbreviation` ending at `caret` on its line, with no
        /// whitespace in it; nil when there is none or it is suppressed.
        func retroLeader(before caret: Int, text: NSString) -> Int? {
            guard caret <= text.length else { return nil }
            var start = caret
            while start > 0 {
                let c = text.character(at: start - 1)
                if c == 0x20 || c == 0x09 || c == 0x0A || c == 0x0D { break }
                start -= 1
            }
            guard start < caret else { return nil }
            var p: Int?
            if leader.isEmpty {
                p = start // bare abbreviations: the whole run
            } else {
                let run = text.substring(with: NSRange(location: start, length: caret - start)) as NSString
                let r = run.range(of: leader, options: .backwards)
                if r.location != NSNotFound { p = start + r.location }
            }
            guard let p, p + leaderLength < caret else { return nil }
            let first = text.character(at: p + leaderLength)
            guard (first >= 0x41 && first <= 0x5A) || (first >= 0x61 && first <= 0x7A) else { return nil }
            guard !suppressed.contains(where: { $0.location == p }) else { return nil }
            let flags = scopeAt(p, text).flags
            guard !flags.contains("verbatim"), !flags.contains("comment") else { return nil }
            return p
        }

        func shiftMarks(_ range: NSRange, _ delta: Int) {
            suppressed = suppressed.compactMap { r in
                if NSMaxRange(range) < r.location || (NSMaxRange(range) == r.location && range.length > 0) {
                    return NSRange(location: r.location + delta, length: r.length)
                }
                if range.location > NSMaxRange(r) { return r }
                return nil // an edit in or touching the region clears its mark
            }
        }

        static func lineIndent(at p: Int, text: NSString) -> String {
            var s = p
            while s > 0, text.character(at: s - 1) != 0x0A { s -= 1 }
            var e = s
            while e < text.length, text.character(at: e) == 0x20 || text.character(at: e) == 0x09 { e += 1 }
            return text.substring(with: NSRange(location: s, length: e - s))
        }

        static func sameLine(_ a: Int, _ b: Int, _ text: NSString) -> Bool {
            let lo = max(0, min(a, b, text.length)), hi = min(max(a, b), text.length)
            guard lo < hi else { return true }
            return text.range(of: "\n", options: .literal, range: NSRange(location: lo, length: hi - lo)).location == NSNotFound
        }
    }
}
