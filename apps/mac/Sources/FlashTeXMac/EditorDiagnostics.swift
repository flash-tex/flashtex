import Foundation
import FlashTeXProtocol
import FlashTeXAccessibility

/// Turns a `compile_result`'s diagnostics into editor underline marks for one
/// document. Pure: no AppKit. A diagnostic's UTF-8 byte range is rebased across
/// edits made since the compile (`SourceMapping`, one `changedRegion` per call)
/// and dropped when it overlaps the edited region, so a mark is never drawn
/// under the wrong text. Dropped marks are reported as stale with a count the
/// UI can show; the diagnostics themselves stay listed.
///
/// Every mark carries the exact identity of the diagnostic it came from
/// (result id + index in `diagnostics` + original byte span), so the underline,
/// its tooltip and the diagnostics list row can agree on which diagnostic they
/// describe even after the range has been rebased. Diagnostics without a
/// `source` are not marks; the preview pane lists them.
enum EditorDiagnostics {
    /// Stable identity of one diagnostic in one compile result. The byte span
    /// is the one the worker reported (in the compiled text), never the rebased
    /// one, so the identity survives edits that shift the mark.
    struct Identity: Hashable {
        /// Envelope id of the `compile_result` (nil for fixtures with no id).
        let resultID: String?
        /// Index into `CompileResult.diagnostics`.
        let index: Int
        /// The worker's `source` range, as reported.
        let source: RuntimeV1.SourceRange

        /// "id#index@path:start..<end" — one token the list row and the
        /// underline can both compute.
        var key: String {
            "\(resultID ?? "-")#\(index)@\(source.path):\(source.startByte)..<\(source.endByte)"
        }

        // `SourceRange` is only Equatable in the protocol module.
        func hash(into hasher: inout Hasher) {
            hasher.combine(resultID); hasher.combine(index)
            hasher.combine(source.path); hasher.combine(source.startByte); hasher.combine(source.endByte)
        }
    }

    struct Mark: Equatable, Identifiable {
        let identity: Identity
        /// UTF-16 range in the *current* editor text (rebased; snapped outward
        /// to grapheme cluster boundaries so an underline never splits one).
        let nsRange: NSRange
        let severity: RuntimeV1.Severity
        let message: String
        let recovery: String?
        /// Status of the result the diagnostic belongs to. A `recovered`
        /// result rendered provisionally, so its marks always show a recovery
        /// line — and stay errors; recovery never downgrades or hides one.
        let resultStatus: RuntimeV1.Status
        /// One bounded line from the explanation catalogue
        /// (`EditorDiagnostics.Explanation.line`), attached by `attach(_:to:)`
        /// once the helper has answered for this result; nil until then.
        var explanation: String? = nil
        /// Set when the newest result is a failure with no output and this
        /// mark was kept from the last result that had output (`Carried`);
        /// nil for marks of the result currently shown.
        var carried: Carried? = nil

        var id: String { identity.key }
        var diagnosticIndex: Int { identity.index }
        /// The original byte span (for the list row's "bytes a..<b" text).
        var originalSource: RuntimeV1.SourceRange { identity.source }

        /// The recovery line shared by the tooltip, the list row and VoiceOver:
        /// the worker's note, or "no provisional rendering" when the result is
        /// `recovered` but this diagnostic describes none. Nil for `ok`/`failed`
        /// results whose diagnostic has no recovery note.
        var recoveryLine: String? { EditorDiagnostics.recoveryLine(recovery: recovery, status: resultStatus) }

        /// Tooltip text: message, then the recovery line, the explanation
        /// line and the carried-over line when present.
        var toolTip: String {
            message + (recoveryLine.map { "\n↳ " + $0 } ?? "") + (explanation.map { "\n↳ " + $0 } ?? "")
                + (carried.map { "\n↳ " + $0.line } ?? "")
        }

        /// "Error: message — recovery: … — explanation — kept from revision N…"
        /// as the accessibility layer speaks it.
        var spokenDescription: String {
            (severity == .error ? "Error: " : "Warning: ") + message
                + (recoveryLine.map { " — " + $0 } ?? "") + (explanation.map { " — " + $0 } ?? "")
                + (carried.map { " — " + $0.line } ?? "")
        }
    }

    /// Why a mark is shown although it is not from the newest result: the
    /// newest result (`failedRevision`) failed with no output, so the marks
    /// of the last result with output (`revision`) are kept, rebased to the
    /// current text, and flagged — never cleared, never duplicated.
    struct Carried: Equatable {
        let revision: Int
        let failedRevision: Int

        var line: String { "kept from revision \(revision): revision \(failedRevision) failed with no output" }
    }

    /// A diagnostic whose span overlaps the edit made since the compile. Its
    /// underline is withheld (never stretched over new text) until the next
    /// result; the identity lets the list row show it as stale.
    struct Stale: Equatable {
        let identity: Identity
        let severity: RuntimeV1.Severity
        let message: String
    }

    /// One call's outcome: the drawable marks plus what was withheld.
    struct Report: Equatable {
        var marks: [Mark]
        var stale: [Stale]
        /// The edit the marks were rebased across (nil: compiled text unknown
        /// or unchanged).
        var edit: SourceMapping.ChangedRegion?
        /// Set when the marks were kept from an older result because the
        /// newest one failed with no output (see `report(for:resultID:retained:…)`).
        var carried: Carried? = nil

        static let empty = Report(marks: [], stale: [], edit: nil)

        var staleCount: Int { stale.count }
        var staleIdentities: Set<Identity> { Set(stale.map(\.identity)) }

        /// Footer/list caption: the carried-over line, the withheld count, or
        /// both; nil when the marks are current and complete.
        var staleNote: String? {
            switch (carried, withheldNote) {
            case (nil, let w): return w
            case (let c?, nil): return "\(marks.count) underline\(marks.count == 1 ? "" : "s") \(c.line)"
            case (let c?, let w?): return "\(marks.count) underline\(marks.count == 1 ? "" : "s") \(c.line); \(w)"
            }
        }

        /// The withheld count alone, nil when nothing is withheld.
        var withheldNote: String? {
            guard !stale.isEmpty else { return nil }
            let errors = stale.filter { $0.severity == .error }.count
            let what: String
            switch (errors, stale.count - errors) {
            case (0, let w): what = "\(w) warning\(w == 1 ? "" : "s")"
            case (let e, 0): what = "\(e) error\(e == 1 ? "" : "s")"
            case (let e, let w): what = "\(e) error\(e == 1 ? "" : "s") and \(w) warning\(w == 1 ? "" : "s")"
            }
            return "\(what) under edited text not underlined until the next compile"
        }
    }

    // MARK: gap category (coordination/daniel-mac-ui-redesign.md §6.4, heuristic route)

    /// Whether a diagnostic says "FlashTeX does not do this yet" rather than
    /// "the source is wrong": the compiler's not-implemented / not-supported
    /// messages (`packages … are recognised but not implemented`, `\x is not
    /// supported by this compiler version`, `\x is not supported in math mode`,
    /// `environment 'x' is not implemented`, `\includegraphics is unsupported`).
    /// Gaps are listed and marked, but counted apart from errors and warnings
    /// so an all-gap document such as HW1 reads "0 errors, 30 not implemented"
    /// instead of "24 errors". When `code` is present, `unsupported_feature` is
    /// a gap and the other issue-#76 codes are not; unknown codes and a missing
    /// `code` fall back to the message phrases (the compiler's own wording).
    static func isGap(_ message: String) -> Bool {
        gapPhrases.contains { message.contains($0) }
    }
    static func isGap(_ diagnostic: RuntimeV1.Diagnostic) -> Bool {
        if let code = diagnostic.code, !code.isEmpty {
            switch code {
            case "unsupported_feature": return true
            case "unknown_command", "syntax_error", "export_limitation", "fidelity_note", "recovered_input":
                return false
            default: return isGap(diagnostic.message)
            }
        }
        return isGap(diagnostic.message)
    }
    private static let gapPhrases = ["not implemented", "not supported by this compiler version",
                                     "not supported in math mode", "not supported in the document preamble", "is unsupported"]

    /// Errors and warnings that are not gaps, and the gaps, of `diagnostics`.
    static func counts(_ diagnostics: [RuntimeV1.Diagnostic]) -> (errors: Int, warnings: Int, gaps: Int) {
        var errors = 0, warnings = 0, gaps = 0
        for d in diagnostics {
            if isGap(d) { gaps += 1 } else if d.severity == .error { errors += 1 } else { warnings += 1 }
        }
        return (errors, warnings, gaps)
    }

    /// Recovery line for a diagnostic of a result with `status` (see `Mark.recoveryLine`).
    static func recoveryLine(recovery: String?, status: RuntimeV1.Status) -> String? {
        if let recovery { return "recovery: " + recovery }
        return status == .recovered ? "no provisional rendering" : nil
    }

    /// Identity of `result.diagnostics[index]`, nil when it has no source (the
    /// list row for such a diagnostic has no mark to agree with).
    static func identity(resultID: String?, index: Int, in result: RuntimeV1.CompileResult) -> Identity? {
        guard result.diagnostics.indices.contains(index), let source = result.diagnostics[index].source else { return nil }
        return Identity(resultID: resultID, index: index, source: source)
    }

    /// Marks only (see `report` for the stale set).
    /// - Parameters:
    ///   - resultID: envelope id of `result`, part of every mark's identity.
    ///   - path: the document shown in the editor.
    ///   - compiledText: the document text `result` was produced for (nil if unknown).
    ///   - currentText: the current editor buffer; marks are `NSRange`s into it.
    static func marks(for result: RuntimeV1.CompileResult, resultID: String? = nil, path: String,
                      compiledText: String?, currentText: String) -> [Mark] {
        report(for: result, resultID: resultID, path: path, compiledText: compiledText, currentText: currentText).marks
    }

    /// Marks plus the diagnostics withheld as stale. Cost is one byte diff of
    /// the two texts plus O(diagnostics) offset conversions; it never waits on
    /// a compile, so a keystroke can call it synchronously (see
    /// `EditorDiagnosticsTests.testRebaseOfLargeDocumentIsFast`).
    static func report(for result: RuntimeV1.CompileResult, resultID: String? = nil, path: String,
                       compiledText: String?, currentText: String) -> Report {
        let region: SourceMapping.ChangedRegion? = {
            guard let compiledText, !compiledText.sameBytes(as: currentText) else { return nil }
            return SourceMapping.changedRegion(from: compiledText, to: currentText)
        }()
        var marks: [Mark] = []
        var stale: [Stale] = []
        marks.reserveCapacity(result.diagnostics.count)
        for (index, diagnostic) in result.diagnostics.enumerated() {
            guard let source = diagnostic.source, source.path == path else { continue }
            let identity = Identity(resultID: resultID, index: index, source: source)
            var start = source.startByte, end = source.endByte
            if let region {
                guard case .rebased(let s, let e) = SourceMapping.rebase(start: start, end: end, across: region) else {
                    stale.append(Stale(identity: identity, severity: diagnostic.severity, message: diagnostic.message))
                    continue
                }
                start = s; end = e
            }
            // Offsets inside a multi-byte scalar, reversed or out of range are
            // refused (nil): a byte span that is not a valid slice of the
            // current buffer is not drawn anywhere.
            guard let ns = currentText.clusterAlignedNSRange(utf8Start: start, utf8End: end) else { continue }
            marks.append(Mark(identity: identity, nsRange: ns, severity: diagnostic.severity,
                              message: diagnostic.message, recovery: diagnostic.recovery, resultStatus: result.status))
        }
        return Report(marks: marks, stale: stale, edit: region)
    }

    // MARK: keyboard navigation (document order, wrapping, "n of m")

    /// The marks in document order as the accessibility navigator sees them.
    static func navigationItems(_ marks: [Mark]) -> [EditorDiagnosticNavigation.Item] {
        EditorDiagnosticNavigation.ordered(marks.map { m in
            EditorDiagnosticNavigation.Item(id: m.id, nsRange: m.nsRange, severity: m.severity,
                                            message: m.message, recoveryLine: m.recoveryLine, explanation: m.explanation)
        })
    }

    /// Next (or previous) diagnostic from the caret, wrapping around the
    /// document; `currentID` is the mark last navigated to, so two marks at
    /// the same offset are both reachable. Nil when there are no marks.
    static func step(_ marks: [Mark], fromUTF16 caret: Int, forward: Bool, currentID: String? = nil,
                     in text: String? = nil) -> EditorDiagnosticNavigation.Step? {
        let model = text.map { AccessibleEditorModel(text: $0) }
        return EditorDiagnosticNavigation.step(navigationItems(marks), fromUTF16: caret, forward: forward,
                                               currentID: currentID, lineOf: { model?.line(containingUTF16: $0)?.number })
    }
}

// MARK: - Partial output: a failed follow-up keeps the last marks, flagged

extension EditorDiagnostics {
    /// The last result that produced output, remembered by the shell so a
    /// later `failed` result with no pages does not clear the underlines.
    struct Retained: Equatable {
        var resultID: String?
        var result: RuntimeV1.CompileResult
        /// Document texts the retained result was compiled from, by path.
        var compiledDocuments: [String: String]

        init(resultID: String?, result: RuntimeV1.CompileResult, compiledDocuments: [String: String]) {
            self.resultID = resultID; self.result = result; self.compiledDocuments = compiledDocuments
        }
    }

    /// True when `result` is a failure that produced no output, so the marks
    /// of the last result with output are kept (flagged) rather than cleared.
    /// Any result with pages, and any `ok`/`recovered` result — an empty
    /// document compiles `ok` to no pages and no marks — replaces them.
    static func keepsPreviousMarks(_ result: RuntimeV1.CompileResult) -> Bool {
        result.status == .failed && result.pages.isEmpty
    }

    /// The `Retained` record to remember after binding `result`: the result
    /// itself when it produced output, otherwise the previous record
    /// (retention never chains through failures, so one failed follow-up or
    /// ten keep the same marks exactly once).
    static func retained(after result: RuntimeV1.CompileResult, resultID: String?, compiledDocuments: [String: String],
                         previous: Retained?) -> Retained? {
        keepsPreviousMarks(result) ? previous : Retained(resultID: resultID, result: result, compiledDocuments: compiledDocuments)
    }

    /// `report(for:…)` that survives a failed follow-up: when `latest` is a
    /// failure with no output and `retained` holds an earlier result with
    /// output, the marks are the retained result's, rebased from the text it
    /// was compiled from to `currentText` and flagged `carried`; any sourced
    /// diagnostic of the failed result itself (e.g. a span the compiler
    /// refused) is a fresh, unflagged mark alongside them. Otherwise this is
    /// exactly `report(for:resultID:path:compiledText:currentText:)`.
    static func report(for latest: RuntimeV1.CompileResult, resultID: String?, retained: Retained?, path: String,
                       compiledText: String?, currentText: String) -> Report {
        let fresh = report(for: latest, resultID: resultID, path: path, compiledText: compiledText, currentText: currentText)
        guard keepsPreviousMarks(latest), let retained, !keepsPreviousMarks(retained.result) else { return fresh }
        var kept = report(for: retained.result, resultID: retained.resultID, path: path,
                          compiledText: retained.compiledDocuments[path], currentText: currentText)
        let carried = Carried(revision: retained.result.revision, failedRevision: latest.revision)
        for i in kept.marks.indices { kept.marks[i].carried = carried }
        kept.marks = fresh.marks + kept.marks
        kept.stale = fresh.stale + kept.stale
        kept.carried = carried
        if kept.edit == nil { kept.edit = fresh.edit }
        return kept
    }
}

// MARK: - Identical diagnostics grouped (count + per-occurrence jump)

extension EditorDiagnostics {
    /// Diagnostics of one result with the same severity and message, in the
    /// order of their first occurrence; `occurrences` are indices into
    /// `result.diagnostics` in document order (by path in `documentOrder`,
    /// then start byte; unsourced last), so "occurrence k of n" is stable
    /// and each one can be jumped to on its own.
    struct Group: Equatable, Identifiable {
        let severity: RuntimeV1.Severity
        let message: String
        /// Snake_case `code` when every occurrence has the same one; nil when
        /// the group was keyed off message text (no code, or mixed).
        let code: String?
        /// The recovery note when every occurrence has the same one, else nil.
        let recovery: String?
        let occurrences: [Int]

        var count: Int { occurrences.count }
        /// Index of the first occurrence (document order); the row's explanation and quick fix use it.
        var first: Int { occurrences[0] }
        var id: String {
            if let code, !code.isEmpty { return "\(severity.rawValue):\(code):\(message)" }
            return "\(severity.rawValue):\(message)"
        }
        /// "12× \in is not supported in math mode", or just the message for one.
        var title: String { count > 1 ? "\(count)× " + message : message }
    }

    /// Groups `result.diagnostics` by (severity, message). `documentOrder`
    /// orders occurrences across documents (unknown paths after known ones).
    static func groups(of result: RuntimeV1.CompileResult, documentOrder: [String] = []) -> [Group] {
        groups(of: result.diagnostics, documentOrder: documentOrder)
    }

    /// `groups(of:)` over any diagnostics list (the panel shows the result's
    /// diagnostics followed by the shell's own layout diagnostics).
    static func groups(of diagnostics: [RuntimeV1.Diagnostic], documentOrder: [String] = []) -> [Group] {
        func rank(_ path: String) -> Int { documentOrder.firstIndex(of: path) ?? documentOrder.count }
        var order: [String] = []
        var members: [String: [Int]] = [:]
        for (i, d) in diagnostics.enumerated() {
            let key = groupKey(d)
            if members[key] == nil { order.append(key); members[key] = [] }
            members[key]!.append(i)
        }
        func before(_ a: Int, _ b: Int) -> Bool {
            switch (diagnostics[a].source, diagnostics[b].source) {
            case (nil, nil): return a < b
            case (nil, _): return false
            case (_, nil): return true
            case (let x?, let y?):
                if x.path != y.path { return rank(x.path) != rank(y.path) ? rank(x.path) < rank(y.path) : x.path < y.path }
                return x.startByte != y.startByte ? x.startByte < y.startByte : a < b
            }
        }
        var groups: [Group] = order.map { key in
            let sorted = members[key]!.sorted(by: before)
            let recoveries = Set(sorted.map { diagnostics[$0].recovery })
            let codes = Set(sorted.map { diagnostics[$0].code ?? "" })
            return Group(severity: diagnostics[sorted[0]].severity, message: diagnostics[sorted[0]].message,
                         code: (codes.count == 1 && !codes.contains("")) ? diagnostics[sorted[0]].code : nil,
                         recovery: recoveries.count == 1 ? diagnostics[sorted[0]].recovery : nil, occurrences: sorted)
        }
        groups.sort { before($0.first, $1.first) }
        return groups
    }

    /// The source of occurrence `k` (zero-based, document order) of `group`,
    /// nil when out of range or unsourced.
    static func occurrence(_ k: Int, of group: Group, in result: RuntimeV1.CompileResult) -> RuntimeV1.SourceRange? {
        occurrence(k, of: group, in: result.diagnostics)
    }

    static func occurrence(_ k: Int, of group: Group, in diagnostics: [RuntimeV1.Diagnostic]) -> RuntimeV1.SourceRange? {
        guard group.occurrences.indices.contains(k), diagnostics.indices.contains(group.occurrences[k]) else { return nil }
        return diagnostics[group.occurrences[k]].source
    }

    /// "3 of 12: main.tex line 41" (line from `texts[path]`, the compiled
    /// text, when known; else "bytes a..<b"); "3 of 12: no source" when
    /// unsourced. One label per menu item of the per-occurrence jump.
    static func occurrenceLabel(_ k: Int, of group: Group, in result: RuntimeV1.CompileResult,
                                texts: [String: String] = [:]) -> String {
        occurrenceLabel(k, of: group, in: result.diagnostics, texts: texts)
    }

    static func occurrenceLabel(_ k: Int, of group: Group, in diagnostics: [RuntimeV1.Diagnostic],
                                texts: [String: String] = [:]) -> String {
        let prefix = "\(k + 1) of \(group.count): "
        guard let s = occurrence(k, of: group, in: diagnostics) else { return prefix + "no source" }
        if let text = texts[s.path], let line = lineNumber(ofByte: s.startByte, in: text) {
            return prefix + "\(s.path) line \(line)"
        }
        return prefix + "\(s.path) bytes \(s.startByte)..<\(s.endByte)"
    }

    /// One-based line containing byte `offset` of `text` (LF-counted); nil
    /// when out of range.
    static func lineNumber(ofByte offset: Int, in text: String) -> Int? {
        guard offset >= 0, offset <= text.utf8.count else { return nil }
        var line = 1
        for b in text.utf8.prefix(offset) where b == 0x0A { line += 1 }
        return line
    }

    /// Grouping key: `(severity, code, message)` when `code` is present, else
    /// today's `(severity, message)`. Same-code diagnostics with different
    /// messages stay separate rows.
    static func groupKey(_ d: RuntimeV1.Diagnostic) -> String {
        if let code = d.code, !code.isEmpty { return "\(d.severity.rawValue):\(code):\(d.message)" }
        return "\(d.severity.rawValue):\(d.message)"
    }
}

extension String {
    /// UTF-16 range for a UTF-8 byte span, widened outward to the grapheme
    /// clusters containing its ends so the range never splits one (a combining
    /// mark typed after the last marked character joins the underline instead
    /// of being cut in half). Nil when the bytes are not a valid scalar-aligned
    /// slice of the string.
    func clusterAlignedNSRange(utf8Start start: Int, utf8End end: Int) -> NSRange? {
        guard var r = rangeOfUTF8(start: start, end: end) else { return nil }
        if String.Index(r.lowerBound, within: self) == nil {
            // `index(after:)` rounds an unaligned index down to its cluster
            // first, so this is the start of the cluster containing lowerBound.
            r = index(before: index(after: r.lowerBound))..<r.upperBound
        }
        if r.upperBound < endIndex, String.Index(r.upperBound, within: self) == nil {
            r = r.lowerBound..<index(after: r.upperBound)
        }
        return NSRange(r, in: self)
    }
}

// MARK: - Explanations (crates/diagnostic-explanations)

/// Consumer of `flashtex-diagnostic-explanations`, the offline, deterministic
/// explanation catalogue for compiler diagnostics (no provider, no network).
/// The crate is a Rust library; the app talks to it through the JSON Lines
/// helper `flashtex-explain` (request/reply shape below, one line each).
/// Replies are bounded in bytes, count and text length, decoded once, cached
/// per result id, and attached to marks as one extra line; a fetch never runs
/// on the caller's thread, so typing is never blocked by an explanation.
extension EditorDiagnostics {
    /// One explanation, decoded from the crate's fixed-key JSON
    /// (`catalog_id, title, category, severity, message, why, what_happened,
    /// suggestions, context`). Text fields are truncated to `ExplanationLimits`.
    struct Explanation: Equatable, Decodable {
        struct Edit: Equatable, Decodable {
            var path: String
            var startByte: Int
            var endByte: Int
            var replacement: String
            enum CodingKeys: String, CodingKey { case path, startByte = "start_byte", endByte = "end_byte", replacement }
        }
        struct Suggestion: Equatable, Decodable {
            var text: String
            /// "low" | "medium" | "high" (unknown values are kept verbatim).
            var confidence: String
            /// Candidate edits; nothing is ever applied by this consumer.
            var edits: [Edit]
        }
        struct Context: Equatable, Decodable {
            var path: String
            var startByte: Int
            var endByte: Int
            var text: String
            var spanStart: Int
            var spanEnd: Int
            var line: Int
            var column: Int
            var spanInBounds: Bool
            enum CodingKeys: String, CodingKey {
                case path, startByte = "start_byte", endByte = "end_byte", text, spanStart = "span_start", spanEnd = "span_end"
                case line, column, spanInBounds = "span_in_bounds"
            }
        }

        /// Catalogue entry id, nil when the message fell back to `unknown`.
        var catalogID: String?
        var title: String
        var category: String
        var severity: String
        var message: String
        var why: String
        var whatHappened: String
        var suggestions: [Suggestion]
        var context: Context?

        enum CodingKeys: String, CodingKey {
            case catalogID = "catalog_id", title, category, severity, message, why, whatHappened = "what_happened", suggestions, context
        }

        var isCatalogued: Bool { catalogID != nil }

        /// The one line shown under the mark, in the list row and spoken:
        /// "explain: <title> — <why>" for catalogued entries, "explain: <title>
        /// (not in the catalogue)" otherwise. Bounded to
        /// `ExplanationLimits.maxLineCharacters`.
        var line: String {
            let body = isCatalogued ? "\(title) — \(why)" : "\(title) (not in the catalogue)"
            return ExplanationLimits.truncate("explain: " + body, to: ExplanationLimits.maxLineCharacters)
        }
    }

    /// Bounds applied to every reply before it is cached. A reply outside
    /// `maxReplyBytes` / `maxCount` is refused whole; text fields are cut.
    enum ExplanationLimits {
        static let maxReplyBytes = 4 * 1024 * 1024
        static let maxCount = 2000
        static let maxLineCharacters = 240
        static let maxTitleCharacters = 120
        static let maxParagraphCharacters = 600
        static let maxSuggestions = 4
        static let maxEditsPerSuggestion = 8
        static let maxContextCharacters = 2000
        /// Results kept in an `ExplanationCache`.
        static let maxCachedResults = 8

        static func truncate(_ s: String, to limit: Int) -> String {
            guard s.count > limit else { return s }
            return String(s.prefix(max(0, limit - 1))) + "…"
        }

        static func bounded(_ x: Explanation) -> Explanation {
            var x = x
            x.title = truncate(x.title, to: maxTitleCharacters)
            x.why = truncate(x.why, to: maxParagraphCharacters)
            x.whatHappened = truncate(x.whatHappened, to: maxParagraphCharacters)
            x.message = truncate(x.message, to: maxParagraphCharacters)
            x.suggestions = x.suggestions.prefix(maxSuggestions).map { s in
                var s = s
                s.text = truncate(s.text, to: maxParagraphCharacters)
                s.edits = s.edits.prefix(maxEditsPerSuggestion).map { e in
                    var e = e
                    e.replacement = truncate(e.replacement, to: maxContextCharacters)
                    return e
                }
                return s
            }
            if var c = x.context { c.text = truncate(c.text, to: maxContextCharacters); x.context = c }
            return x
        }
    }

    enum ExplanationFailure: Error, Equatable {
        /// The reply line is larger than `ExplanationLimits.maxReplyBytes`.
        case oversized(bytes: Int)
        /// The reply is not the documented shape.
        case malformed(String)
        /// The helper answered for a different number of diagnostics.
        case countMismatch(expected: Int, actual: Int)
        /// The helper answered `type: error`.
        case helper(code: String, message: String)
        /// Process/transport failure (`LineProcessFailure.text`).
        case transport(String)
        /// No helper executable is configured or found.
        case unavailable

        var text: String {
            switch self {
            case .oversized(let n): "explanation reply of \(n) bytes exceeds \(ExplanationLimits.maxReplyBytes)"
            case .malformed(let s): "malformed explanation reply: \(s)"
            case .countMismatch(let e, let a): "explanation reply has \(a) entries for \(e) diagnostics"
            case .helper(let c, let m): "flashtex-explain error \(c): \(m)"
            case .transport(let s): "flashtex-explain: \(s)"
            case .unavailable: "flashtex-explain is not available"
            }
        }
    }

    /// Decodes the `explanations` array of one reply line (or a bare array)
    /// and applies the bounds. `expectedCount` is the result's diagnostic
    /// count: a reply for a different number is refused, since entries are
    /// matched to diagnostics by index. `maxBytes` defaults to the limit.
    static func decodeExplanations(_ data: Data, expectedCount: Int,
                                   maxBytes: Int = ExplanationLimits.maxReplyBytes) throws -> [Explanation] {
        guard data.count <= maxBytes else { throw ExplanationFailure.oversized(bytes: data.count) }
        struct Reply: Decodable { var explanations: [Explanation] }
        let decoded: [Explanation]
        do {
            let first = data.first { $0 != 0x20 && $0 != 0x0A && $0 != 0x0D && $0 != 0x09 }
            if first == UInt8(ascii: "[") {
                decoded = try JSONDecoder().decode([Explanation].self, from: data)
            } else {
                decoded = try JSONDecoder().decode(Reply.self, from: data).explanations
            }
        } catch {
            throw ExplanationFailure.malformed(Self.describe(error))
        }
        guard decoded.count == expectedCount, decoded.count <= ExplanationLimits.maxCount else {
            throw ExplanationFailure.countMismatch(expected: expectedCount, actual: decoded.count)
        }
        return decoded.map(ExplanationLimits.bounded)
    }

    private static func describe(_ error: Error) -> String {
        guard let d = error as? DecodingError else { return String(describing: error) }
        func path(_ c: DecodingError.Context) -> String { c.codingPath.map(\.stringValue).joined(separator: ".") }
        switch d {
        case .keyNotFound(let k, let c): return "missing key '\(k.stringValue)' at '\(path(c))'"
        case .typeMismatch(let t, let c): return "type mismatch (\(t)) at '\(path(c))'"
        case .valueNotFound(let t, let c): return "null for \(t) at '\(path(c))'"
        case .dataCorrupted(let c): return "not JSON: \(c.debugDescription)"
        @unknown default: return String(describing: d)
        }
    }

    /// Explanations kept per result id, bounded to
    /// `ExplanationLimits.maxCachedResults` results (oldest evicted). The
    /// diagnostics of a result never change, so an entry is never refreshed.
    struct ExplanationCache: Equatable {
        private(set) var entries: [String: [Explanation]] = [:]
        private var order: [String] = []
        let maxResults: Int
        /// Keyed by (diagnostic, source sha) across results (ExplanationMemo.swift):
        /// outlives the per-result entries so an unchanged diagnostic is never
        /// fetched twice.
        private(set) var memo = ExplanationMemo()

        init(maxResults: Int = ExplanationLimits.maxCachedResults) { self.maxResults = maxResults }

        mutating func store(_ explanations: [Explanation], for resultID: String) {
            if entries[resultID] == nil {
                order.append(resultID)
                while order.count > maxResults, let oldest = order.first {
                    order.removeFirst(); entries[oldest] = nil
                }
            }
            entries[resultID] = explanations
        }

        /// `store` plus memoizing each explanation under its diagnostic's
        /// (diagnostic, source sha) key so a later result can reuse it.
        mutating func store(_ explanations: [Explanation], for resultID: String,
                            result: RuntimeV1.CompileResult, documents: [RuntimeV1.Document]) {
            store(explanations, for: resultID)
            memo.remember(explanations, for: result, documents: documents)
        }

        /// Fills `resultID` from the memo when every diagnostic of `result` was
        /// already explained for exactly these document texts; true when it
        /// did (no helper request is needed), false when a fetch is required.
        /// A result already stored is left alone (true).
        mutating func reuse(for resultID: String, result: RuntimeV1.CompileResult, documents: [RuntimeV1.Document]) -> Bool {
            if entries[resultID] != nil { return true }
            guard let list = memo.recall(for: result, documents: documents) else { return false }
            store(list, for: resultID)
            return true
        }

        subscript(resultID: String?) -> [Explanation]? {
            guard let resultID else { return nil }
            return entries[resultID]
        }

        func explanation(resultID: String?, index: Int) -> Explanation? {
            guard let list = self[resultID], list.indices.contains(index) else { return nil }
            return list[index]
        }

        var count: Int { entries.count }
    }

    /// `report` with each mark's `explanation` line set from `explanations`
    /// (matched by diagnostic index; nil leaves the marks untouched). Marks
    /// `carried` from an older result belong to that result's diagnostics,
    /// so they take their lines from `carried` (the cache entry for the
    /// retained result id), never from the newest result's. O(marks): this
    /// is all a keystroke pays once the helper has answered.
    static func attach(_ explanations: [Explanation]?, carried carriedExplanations: [Explanation]? = nil,
                       to report: Report) -> Report {
        let current = (explanations?.isEmpty == false) ? explanations : nil
        let kept = (carriedExplanations?.isEmpty == false) ? carriedExplanations : nil
        guard current != nil || kept != nil else { return report }
        var out = report
        for i in out.marks.indices {
            guard let list = out.marks[i].carried == nil ? current : kept else { continue }
            let index = out.marks[i].diagnosticIndex
            out.marks[i].explanation = list.indices.contains(index) ? list[index].line : nil
        }
        return out
    }
}

/// The `flashtex-explain` helper: one request line
/// `{"id","type":"explain","compile_result":<payload>,"documents":[{path,text}],"supported":[…]}`,
/// one reply line `{"id","type":"explanations","explanations":[…]}` or
/// `{"id","type":"error","error":{code,message}}`. Runs on private pipes via
/// `LineProcessClient`; completions are delivered on `queue` (main by default).
final class ExplanationClient {
    typealias Explanation = EditorDiagnostics.Explanation
    typealias Failure = EditorDiagnostics.ExplanationFailure

    static let label = "explain"
    /// Default deadline for one reply; the crate answers in milliseconds.
    static let defaultTimeout: TimeInterval = 10

    let executable: URL
    let maxReplyBytes: Int
    private let client: LineProcessClient
    private let queue: DispatchQueue

    /// `FLASHTEX_EXPLAIN`, then `flashtex-explain` beside the app executable,
    /// then the repository's `crates/diagnostic-explanations/target/{release,debug}`.
    @MainActor static func locate() -> URL? {
        let fm = FileManager.default
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_EXPLAIN"], fm.isExecutableFile(atPath: env) {
            return URL(fileURLWithPath: env)
        }
        if let bundled = Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("flashtex-explain"),
           fm.isExecutableFile(atPath: bundled.path) {
            return bundled
        }
        guard let root = ShellModel.locateRepoRoot() else { return nil }
        for profile in ["release", "debug"] {
            let url = root.appendingPathComponent("crates/diagnostic-explanations/target/\(profile)/flashtex-explain")
            if fm.isExecutableFile(atPath: url.path) { return url }
        }
        return nil
    }

    init(executable: URL, arguments: [String] = [], queue: DispatchQueue = .main,
         maxReplyBytes: Int = EditorDiagnostics.ExplanationLimits.maxReplyBytes,
         events: @escaping (String) -> Void = { _ in }) throws {
        self.executable = executable
        self.queue = queue
        self.maxReplyBytes = maxReplyBytes
        struct Header: Decodable { var id: String?; var type: String; var error: TransferV1.ErrorPayload? }
        client = try LineProcessClient(
            executable: executable, arguments: arguments, label: Self.label, queue: queue,
            classify: { line in
                guard let h = try? JSONDecoder().decode(Header.self, from: line) else { return nil }
                return .init(id: h.id, type: h.type, error: h.error)
            },
            events: { event in
                let text: String
                switch event {
                case .stderr(let s): text = "stderr: " + s.trimmingCharacters(in: .whitespacesAndNewlines)
                case .protocolViolation(let s): text = s
                case .unsolicited(let id, let type, _): text = "unsolicited \(type) reply for \(id ?? "?")"
                case .exited(let code): text = "exited (\(code))"
                }
                events(text)
            })
    }

    var isRunning: Bool { client.isRunning }
    func terminate() { client.terminate() }

    private struct Request: Encodable {
        var id: String
        var type = "explain"
        var compileResult: RuntimeV1.CompileResult
        var documents: [RuntimeV1.Document]
        var supported: [String]
        enum CodingKeys: String, CodingKey { case id, type, compileResult = "compile_result", documents, supported }
    }

    /// Asks for explanations of every diagnostic of `result` (compiled from
    /// `documents`). The reply is decoded and bounded, then delivered on
    /// `queue`; the caller stores it in an `ExplanationCache` under the
    /// result's id. Nothing here blocks the caller beyond encoding the request.
    func explain(result: RuntimeV1.CompileResult, documents: [RuntimeV1.Document], supported: [String],
                 timeout: TimeInterval? = ExplanationClient.defaultTimeout,
                 completion: @escaping (Result<[Explanation], Failure>) -> Void) {
        let id = client.makeID()
        let line: Data
        do {
            let enc = JSONEncoder()
            enc.outputFormatting = [.withoutEscapingSlashes]
            var data = try enc.encode(Request(id: id, compileResult: result, documents: documents, supported: supported))
            data.append(0x0A)
            line = data
        } catch {
            queue.async { completion(.failure(.transport("request encoding failed: \(error)"))) }
            return
        }
        let expected = result.diagnostics.count
        let maxBytes = maxReplyBytes
        client.enqueue(id: id, line: line, expected: "explanations", timeout: timeout) { outcome in
            switch outcome {
            case .failure(.bridge(let e)):
                completion(.failure(.helper(code: e.code, message: e.message)))
            case .failure(let f):
                completion(.failure(.transport(f.text)))
            case .success(let reply):
                do {
                    completion(.success(try EditorDiagnostics.decodeExplanations(reply, expectedCount: expected, maxBytes: maxBytes)))
                } catch let f as Failure {
                    completion(.failure(f))
                } catch {
                    completion(.failure(.malformed(String(describing: error))))
                }
            }
        }
    }
}

// MARK: - Reviewed quick fixes from explanation suggestions

extension EditorDiagnostics {
    /// Turns one explanation suggestion's `edits` (byte ranges into the
    /// *compiled* text) into a preview and ONE grouped replacement for the
    /// *current* editor text. Every edit is rebased byte-exactly through
    /// `SourceMapping` and refused when it overlaps an edit made since the
    /// compile, when the bytes it targets no longer match, when it is not a
    /// scalar-aligned range, or when the suggestion's edits overlap each
    /// other. Nothing is applied here: the caller shows the preview and only
    /// then hands `Grouped` to the editor as a single undoable edit.
    enum QuickFix {
        /// One rebased edit, as the editor sees it (UTF-16 in the current text).
        struct Replacement: Equatable {
            /// Byte range in the current text.
            var byteRange: Range<Int>
            /// The same range in UTF-16 units.
            var nsRange: NSRange
            var text: String
        }

        /// The one grouped edit: the covering range of all replacements in
        /// the current text and the text that replaces it. Applying it is
        /// byte-identical to applying every replacement.
        struct Grouped: Equatable {
            var path: String
            var nsRange: NSRange
            var byteRange: Range<Int>
            /// Bytes of the covering range at preparation time.
            var before: String
            var text: String
            /// The individual replacements, ascending, non-overlapping.
            var replacements: [Replacement]

            /// True when `currentText` still holds `before` at `byteRange`, so
            /// the grouped edit can be applied without re-preparing.
            func matches(_ currentText: String) -> Bool {
                guard let r = currentText.rangeOfUTF8(start: byteRange.lowerBound, end: byteRange.upperBound) else { return false }
                return String(currentText[r]).sameBytes(as: before)
            }

            /// The full text after the edit, nil when `currentText` changed
            /// since preparation (never guesses).
            func applied(to currentText: String) -> String? {
                guard matches(currentText), let r = currentText.rangeOfUTF8(start: byteRange.lowerBound, end: byteRange.upperBound)
                else { return nil }
                var out = currentText
                out.replaceSubrange(r, with: text)
                return out
            }
        }

        struct Preview {
            var path: String
            var suggestionText: String
            var confidence: String
            /// Ascending, non-overlapping, in the current text.
            var replacements: [Replacement]
            /// Whole lines of the current text around the edits (UTF-16 range
            /// in the current text), and the same lines after the edit.
            var snippetRange: NSRange
            var before: String
            var after: String
            /// The single edit the editor applies; never applied automatically.
            var grouped: Grouped
            /// Yields the grouped replacement for the editor (one undoable edit).
            var apply: () -> Grouped

            var summary: String {
                let n = replacements.count
                return "\(suggestionText) (\(confidence) confidence, \(n) edit\(n == 1 ? "" : "s"))"
            }
        }

        enum Refusal: Error, Equatable {
            /// The suggestion has no mechanical form (advice only).
            case noEdits
            /// An edit targets a document other than `path`.
            case otherDocument(path: String)
            /// The compiled text for this document is unknown; edits cannot be trusted.
            case noCompiledText
            /// Edit `index` is not a scalar-aligned, in-bounds range of the compiled text.
            case invalidRange(edit: Int)
            /// Edit `index` spans text edited since the compile.
            case overlapsEdit(edit: Int)
            /// The bytes edit `index` targets are not the ones the explanation saw.
            case bytesChanged(edit: Int, expected: String, actual: String)
            /// Edits `a` and `b` of the suggestion overlap each other.
            case editsOverlap(a: Int, b: Int)

            var text: String {
                switch self {
                case .noEdits: "this suggestion is advice only; there is nothing to apply"
                case .otherDocument(let p): "the fix edits \(p), not the active document"
                case .noCompiledText: "the compiled text is not known; recompile before applying a fix"
                case .invalidRange(let i): "edit \(i + 1) is not a valid range of the compiled text"
                case .overlapsEdit(let i): "edit \(i + 1) spans text changed since the compile; recompile to refresh the fix"
                case .bytesChanged(let i, let e, let a): "edit \(i + 1) expected “\(e)” but the text reads “\(a)”; recompile to refresh the fix"
                case .editsOverlap(let a, let b): "edits \(a + 1) and \(b + 1) of this suggestion overlap; not applied"
                }
            }
        }

        /// Prepares suggestion `suggestion` of `explanation` for `path`.
        /// `compiledText` is the text the compile (and the explanation) ran
        /// on; `currentText` is the editor buffer now.
        static func prepare(_ explanation: Explanation, suggestion index: Int = 0, path: String,
                            in currentText: String, compiledText: String?) -> Result<Preview, Refusal> {
            guard explanation.suggestions.indices.contains(index) else { return .failure(.noEdits) }
            let suggestion = explanation.suggestions[index]
            guard !suggestion.edits.isEmpty else { return .failure(.noEdits) }
            if let other = suggestion.edits.first(where: { $0.path != path }) { return .failure(.otherDocument(path: other.path)) }
            guard let compiledText else { return .failure(.noCompiledText) }
            let compiledBytes = Array(compiledText.utf8)
            let currentBytes = Array(currentText.utf8)
            let region: SourceMapping.ChangedRegion? = compiledText.sameBytes(as: currentText) ? nil
                : SourceMapping.changedRegion(from: compiledText, to: currentText)

            // Rebase each edit, checking the bytes it targets twice: against
            // the explanation's own context excerpt (what the crate saw) and
            // against the current buffer (what the editor will replace).
            var replacements: [(index: Int, Replacement)] = []
            for (i, edit) in suggestion.edits.enumerated() {
                guard edit.startByte <= edit.endByte, compiledText.rangeOfUTF8(start: edit.startByte, end: edit.endByte) != nil
                else { return .failure(.invalidRange(edit: i)) }
                let original = String(decoding: compiledBytes[edit.startByte..<edit.endByte], as: UTF8.self)
                if let c = explanation.context, c.path == path,
                   edit.startByte >= c.startByte, edit.endByte <= c.endByte {
                    let ctx = Array(c.text.utf8)
                    let lo = edit.startByte - c.startByte, hi = edit.endByte - c.startByte
                    if hi <= ctx.count {
                        let seen = String(decoding: ctx[lo..<hi], as: UTF8.self)
                        if !seen.sameBytes(as: original) { return .failure(.bytesChanged(edit: i, expected: seen, actual: original)) }
                    }
                }
                var start = edit.startByte, end = edit.endByte
                if let region {
                    guard case .rebased(let s, let e) = SourceMapping.rebase(start: start, end: end, across: region)
                    else { return .failure(.overlapsEdit(edit: i)) }
                    start = s; end = e
                }
                guard let r = currentText.rangeOfUTF8(start: start, end: end) else { return .failure(.invalidRange(edit: i)) }
                let now = String(decoding: currentBytes[start..<end], as: UTF8.self)
                guard now.sameBytes(as: original) else { return .failure(.bytesChanged(edit: i, expected: original, actual: now)) }
                replacements.append((i, Replacement(byteRange: start..<end, nsRange: NSRange(r, in: currentText), text: edit.replacement)))
            }
            // Ascending by start; insertions at one offset keep the suggestion's order.
            let ordered = replacements.enumerated().sorted { a, b in
                a.element.1.byteRange.lowerBound != b.element.1.byteRange.lowerBound
                    ? a.element.1.byteRange.lowerBound < b.element.1.byteRange.lowerBound : a.offset < b.offset
            }.map(\.element)
            for k in ordered.indices.dropFirst() where ordered[k].1.byteRange.lowerBound < ordered[k - 1].1.byteRange.upperBound {
                return .failure(.editsOverlap(a: ordered[k - 1].index, b: ordered[k].index))
            }
            let reps = ordered.map(\.1)

            // The grouped edit: covering range, rewritten once.
            let lo = reps[0].byteRange.lowerBound, hi = reps.map(\.byteRange.upperBound).max()!
            var grouped = ""
            var cursor = lo
            for r in reps {
                grouped += String(decoding: currentBytes[cursor..<r.byteRange.lowerBound], as: UTF8.self)
                grouped += r.text
                cursor = r.byteRange.upperBound
            }
            grouped += String(decoding: currentBytes[cursor..<hi], as: UTF8.self)
            let coveringIndex = currentText.rangeOfUTF8(start: lo, end: hi)!
            let before = String(currentText[coveringIndex])
            let group = Grouped(path: path, nsRange: NSRange(coveringIndex, in: currentText), byteRange: lo..<hi,
                                before: before, text: grouped, replacements: reps)

            // Snippet: whole lines around the covering range, before and after.
            var lineLo = lo, lineHi = hi
            while lineLo > 0, currentBytes[lineLo - 1] != 0x0A { lineLo -= 1 }
            // A covering range that ends just after a newline stays on its
            // own lines instead of pulling in the next one.
            let endsAfterNewline = hi > lo && currentBytes[hi - 1] == 0x0A
            if endsAfterNewline { lineHi = hi - 1 } else {
                while lineHi < currentBytes.count, currentBytes[lineHi] != 0x0A { lineHi += 1 }
            }
            let snippetIndex = currentText.rangeOfUTF8(start: lineLo, end: lineHi)!
            let beforeSnippet = String(currentText[snippetIndex])
            var afterSnippet = String(decoding: currentBytes[lineLo..<lo], as: UTF8.self) + grouped
            if endsAfterNewline { if afterSnippet.hasSuffix("\n") { afterSnippet.removeLast() } }
            else { afterSnippet += String(decoding: currentBytes[hi..<lineHi], as: UTF8.self) }
            let preview = Preview(path: path, suggestionText: suggestion.text, confidence: suggestion.confidence,
                                  replacements: reps, snippetRange: NSRange(snippetIndex, in: currentText),
                                  before: beforeSnippet, after: afterSnippet, grouped: group, apply: { group })
            return .success(preview)
        }
    }

    /// Mechanical Fix… from `help.replacement` (preferred) or `suggestion`
    /// over the diagnostic `source`. Feeds the same `QuickFix.prepare` path
    /// as explanation-service edits — one grouped undoable replacement.
    static func mechanicalEdit(for d: RuntimeV1.Diagnostic) -> Explanation.Edit? {
        if let r = d.help?.replacement {
            guard let path = d.path(of: r) else { return nil }
            guard r.startByte <= r.endByte else { return nil }
            return .init(path: path, startByte: r.startByte, endByte: r.endByte, replacement: r.text)
        }
        if let suggestion = d.suggestion, let source = d.source {
            return .init(path: source.path, startByte: source.startByte, endByte: source.endByte, replacement: suggestion)
        }
        return nil
    }

    /// True when the diagnostic's mechanical edit is in-bounds for
    /// `currentText`, targets `path`, and the compile revision still matches
    /// the editor — the Fix… affordance is hidden otherwise. Advice-only help
    /// and a stale buffer never show it.
    ///
    /// The edit comes from `mechanicalEdit`, so this accepts both forms: a
    /// `help.replacement`, and the flat `suggestion` over the diagnostic's own
    /// `source`. Gating on `help.replacement` alone made the affordance
    /// unreachable in the shipping configuration — `ShellModel`
    /// `locateDefaultProducer` attaches `flashtex-render`, whose
    /// `display::Diagnostic` carries no `help` field at all, so the wire has
    /// only `suggestion`. The compiler sets `suggestion` at exactly one site
    /// (`Diagnostic::command_error`), where the span is the command token, so
    /// replacing `source` with it is the same edit `help.replacement` would
    /// have carried.
    static func canApplyHelpReplacement(_ d: RuntimeV1.Diagnostic, path: String, currentText: String,
                                        compiledRevision: Int?, editorRevision: Int) -> Bool {
        guard compiledRevision == editorRevision else { return false }
        guard let edit = mechanicalEdit(for: d) else { return false }
        guard edit.path == path else { return false }
        guard edit.startByte >= 0, edit.startByte <= edit.endByte else { return false }
        return currentText.rangeOfUTF8(start: edit.startByte, end: edit.endByte) != nil
    }

    /// A mechanical fix offered for the diagnostic the caret is sitting on, so
    /// the editor can show it inline and let Tab accept it.
    ///
    /// The caret counts as "on" the diagnostic at both ends inclusive: an
    /// author who has just finished typing a word leaves the caret directly
    /// after it, and a fix that vanished there would be useless.
    struct CaretFix: Equatable {
        /// Index into the model's `displayedDiagnostics`, so accepting it can
        /// reuse the Problems panel's `previewQuickFix`/`applyQuickFix` path.
        var diagnosticIndex: Int
        /// What the hint says Tab will do.
        var title: String
        /// The text Tab inserts.
        var replacement: String
        /// UTF-8 range the replacement covers.
        var startByte: Int
        var endByte: Int
    }

    /// The first diagnostic in document order whose span covers `caretByte` and
    /// carries an applicable mechanical edit; `nil` when the caret is not on
    /// one. `nil` must leave Tab alone — a fix the author cannot see must never
    /// change what the key does.
    static func fixOffered(at caretByte: Int, in diagnostics: [RuntimeV1.Diagnostic],
                           path: String, currentText: String,
                           compiledRevision: Int?, editorRevision: Int) -> CaretFix? {
        for (index, d) in diagnostics.enumerated() {
            guard let source = d.source, source.path == path else { continue }
            guard caretByte >= source.startByte, caretByte <= source.endByte else { continue }
            guard canApplyHelpReplacement(d, path: path, currentText: currentText,
                                          compiledRevision: compiledRevision,
                                          editorRevision: editorRevision),
                  let edit = mechanicalEdit(for: d) else { continue }
            return CaretFix(diagnosticIndex: index,
                            title: d.help?.message ?? "Replace with \(edit.replacement)",
                            replacement: edit.replacement,
                            startByte: edit.startByte, endByte: edit.endByte)
        }
        return nil
    }


    /// `QuickFix.prepare` for the diagnostic's mechanical edit (same refusal
    /// cases as an explanation suggestion).
    static func prepareHelpReplacement(_ d: RuntimeV1.Diagnostic, path: String,
                                       in currentText: String, compiledText: String?) -> Result<QuickFix.Preview, QuickFix.Refusal> {
        guard let edit = mechanicalEdit(for: d) else { return .failure(.noEdits) }
        let text = d.help?.message ?? d.suggestion ?? "Apply suggested fix"
        let explanation = Explanation(
            catalogID: d.code, title: text, category: d.code ?? "diagnostic",
            severity: d.severity.rawValue, message: d.message, why: text, whatHappened: d.recovery ?? "",
            suggestions: [.init(text: text, confidence: "high", edits: [edit])], context: nil)
        return QuickFix.prepare(explanation, path: path, in: currentText, compiledText: compiledText)
    }
}
