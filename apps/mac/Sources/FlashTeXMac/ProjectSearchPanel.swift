import AppKit
import SwiftUI
import FlashTeXProtocol

/// Native project search through the durable helper's `search_literal`
/// (crates/preview-controller STDIO.md): a case-sensitive raw UTF-8 literal
/// search over the helper's durable source (comments and verbatim text
/// included), no regex, no normalization. The helper is the only route: it
/// searches the durable ledger text at an exact `source_versions` map, so
/// results name the revision they were computed on and are shown with the
/// helper's explicit `termination` (`complete` / `match_limit` /
/// `work_limit`). Partial results are never labelled exhaustive.
///
/// Navigation to a match is refused, never guessed, when the durable
/// versions moved since the search (a durable edit anywhere in the project
/// — the helper's own stale-version rule), when the durable bytes at the
/// range no longer spell the literal, or when the open buffer's bytes at
/// the range differ from the durable text (`Navigation.rebaseExactly`: a
/// local edit overlapping the range is refused, an edit elsewhere is
/// rebased byte-exactly and re-verified — the `navigateExactly` rule). The
/// selection covers whole composed character sequences via
/// `ShellModel.selectIndexLocation` (Navigation.swift).
///
/// Pure helpers live in `ProjectSearch`; `ProjectSearchClient` is the
/// bounded request/reply client on top of `ShellModel.controllerRequest`;
/// `ProjectSearchPanel` is the ⌘⇧F window (`ProjectSearchWindow` scene +
/// `ProjectSearchCommands` menu item are added from `FlashTeXMacApp`).
enum ProjectSearch {
    /// `Window(id:)` of the panel.
    static let windowID = "project-search"
    /// The helper's admissible ranges (STDIO.md).
    static let matchLimitRange = 1...1000
    static let workLimitRange = 1...1_000_000
    static let defaultMaxMatches = 200
    static let defaultMaxWork = 1_000_000
    /// Characters of line context kept on each side of a match in a snippet.
    static let snippetContext = 48
    /// STDIO.md: literals are at most 64 KiB (the helper refuses longer ones).
    static let maxLiteralBytes = 64 * 1024

    /// The helper's explicit termination. Only `complete` is exhaustive.
    enum Termination: Equatable {
        case complete
        case matchLimit
        case workLimit
        case cancelled
        /// A value this client does not know: shown verbatim, never exhaustive.
        case unknown(String)

        init(wire: String) {
            switch wire {
            case "complete": self = .complete
            case "match_limit": self = .matchLimit
            case "work_limit": self = .workLimit
            case "cancelled": self = .cancelled
            default: self = .unknown(wire)
            }
        }

        var isExhaustive: Bool { self == .complete }

        /// Short badge text.
        var badge: String {
            switch self {
            case .complete: return "complete"
            case .matchLimit: return "match limit"
            case .workLimit: return "work limit"
            case .cancelled: return "cancelled"
            case .unknown(let s): return s
            }
        }

        /// What the badge means, for the panel and VoiceOver.
        func explanation(maxMatches: Int, maxWork: Int) -> String {
            switch self {
            case .complete: return "Exhaustive: every match in the durable project source is listed."
            case .matchLimit: return "Partial: the helper stopped at the match limit (\(maxMatches)); more matches may exist."
            case .workLimit: return "Partial: the helper stopped at the work budget (\(maxWork) byte comparisons); more matches may exist."
            case .cancelled: return "Partial: the search was cancelled; more matches may exist."
            case .unknown(let s): return "Partial: unknown termination “\(s)”; not treated as exhaustive."
            }
        }
    }

    /// A match's line, shown with the matched bytes distinguishable from its
    /// context. Line breaks inside a multi-line literal are shown as ⏎.
    struct Snippet: Equatable {
        var before: String
        var match: String
        var after: String
        /// True when `before`/`after` were clipped to `snippetContext` characters.
        var clippedBefore = false
        var clippedAfter = false

        var text: String { (clippedBefore ? "…" : "") + before + match + after + (clippedAfter ? "…" : "") }
    }

    /// One helper match: the exact durable byte range plus its display data.
    struct Match: Identifiable, Equatable {
        var location: ShellModel.IndexLocation
        /// 1-based line of the match start in the durable text (LF-counted; a
        /// `\r\n` file counts its `\n`s), 0 when the text was unavailable.
        var line: Int
        /// 1-based byte column of the match start within its line.
        var column: Int
        var snippet: Snippet?

        var id: String { "\(location.path)@\(location.revision):\(location.start)..<\(location.end)" }
        var path: String { location.path }
    }

    /// The reply to one `search_literal`, bound to the versions it ran on.
    struct Results: Equatable {
        var literal: String
        var sourceVersions: [String: Int]
        /// The `documents` filter the search ran with; nil is the whole project.
        var documents: [String]?
        var matches: [Match]
        var termination: Termination
        var workUsed: Int
        var maxMatches: Int
        var maxWork: Int

        /// "3 matches (complete)" / "200 matches shown — partial (match limit)".
        var summary: String {
            let n = matches.count
            let count = "\(n) match\(n == 1 ? "" : "es")"
            return termination.isExhaustive ? "\(count) (complete)" : "\(count) shown — partial (\(termination.badge))"
        }
    }

    // MARK: - wire

    /// The `search_literal` payload; limits are clamped into the helper's
    /// admissible ranges (a value outside them would be refused as a whole).
    static func request(sourceVersions: [String: Int], literal: String, maxMatches: Int, maxWork: Int, documents: [String]? = nil) -> PreviewControllerClient.JSONObject {
        var payload: PreviewControllerClient.JSONObject = [
            "source_versions": sourceVersions,
            "literal": literal,
            "max_matches": clamp(maxMatches, to: matchLimitRange),
            "max_work": clamp(maxWork, to: workLimitRange),
        ]
        if let documents { payload["documents"] = documents }
        return payload
    }

    static func clamp(_ value: Int, to range: ClosedRange<Int>) -> Int { min(max(value, range.lowerBound), range.upperBound) }

    /// Raw locations from a `search_literal` reply (`{source_versions,
    /// matches:[{path,revision,start_byte,end_byte}], termination, work_used}`).
    struct RawReply: Equatable {
        var sourceVersions: [String: Int]
        var matches: [ShellModel.IndexLocation]
        var termination: Termination
        var workUsed: Int
    }

    static func parse(reply payload: [String: Any]) -> Result<RawReply, ControllerError> {
        guard let versions = payload["source_versions"] as? [String: Int] else { return .failure(.init(message: "search reply has no source_versions")) }
        guard let wire = payload["termination"] as? String else { return .failure(.init(message: "search reply has no termination")) }
        guard let raw = payload["matches"] as? [Any] else { return .failure(.init(message: "search reply has no matches")) }
        var matches: [ShellModel.IndexLocation] = []
        matches.reserveCapacity(raw.count)
        for item in raw {
            guard let loc = ShellModel.IndexLocation(item) else { return .failure(.init(message: "search reply match \(matches.count) is malformed")) }
            guard versions[loc.path] == loc.revision else {
                return .failure(.init(message: "search reply match \(matches.count) names \(loc.path) r\(loc.revision), not the reply's r\(versions[loc.path].map(String.init) ?? "?")"))
            }
            matches.append(loc)
        }
        return .success(RawReply(sourceVersions: versions, matches: matches, termination: Termination(wire: wire),
                                 workUsed: payload["work_used"] as? Int ?? 0))
    }

    // MARK: - text (pure, UTF-8 exact)

    /// Whether bytes `start..<end` of `text` are exactly the literal's bytes.
    static func bytesSpell(_ literal: String, in text: String, start: Int, end: Int) -> Bool {
        guard start >= 0, end >= start, end <= text.utf8.count, end - start == literal.utf8.count else { return false }
        var t = text, l = literal
        return t.withUTF8 { tb in
            l.withUTF8 { lb in
                lb.count == 0 || memcmp(tb.baseAddress! + start, lb.baseAddress!, lb.count) == 0
            }
        }
    }

    /// Line (1-based), byte column (1-based) and the clipped line snippet for
    /// bytes `start..<end` of `text`. Nil when the range is not a valid scalar-
    /// aligned range of `text`. Context is clipped on `Character` boundaries,
    /// so a snippet never begins or ends inside a composed character sequence.
    static func locate(start: Int, end: Int, in text: String, context: Int = snippetContext) -> (line: Int, column: Int, snippet: Snippet)? {
        guard let matchRange = text.rangeOfUTF8(start: start, end: end) else { return nil }
        var t = text
        let (line, lineStart, lineEnd): (Int, Int, Int) = t.withUTF8 { b in
            let n = b.count
            var line = 1, lineStart = 0
            for i in 0..<start where b[i] == UInt8(ascii: "\n") { line += 1; lineStart = i + 1 }
            var lineEnd = end
            while lineEnd < n, b[lineEnd] != UInt8(ascii: "\n") { lineEnd += 1 }
            if lineEnd > end, b[lineEnd - 1] == UInt8(ascii: "\r") { lineEnd -= 1 }
            return (line, lineStart, lineEnd)
        }
        guard let beforeRange = text.rangeOfUTF8(start: lineStart, end: start),
              let afterRange = text.rangeOfUTF8(start: end, end: lineEnd) else { return nil }
        var snippet = Snippet(before: String(text[beforeRange]), match: String(text[matchRange]), after: String(text[afterRange]))
        snippet.match = snippet.match.replacingOccurrences(of: "\r\n", with: "⏎").replacingOccurrences(of: "\n", with: "⏎")
        if snippet.before.count > context { snippet.before = String(snippet.before.suffix(context)); snippet.clippedBefore = true }
        if snippet.after.count > context { snippet.after = String(snippet.after.prefix(context)); snippet.clippedAfter = true }
        return (line, start - lineStart + 1, snippet)
    }

    /// Display matches for the raw locations, using the durable text of each
    /// document when the caller could read it (`texts[path]` at the reply's
    /// revision); a match whose text is unavailable keeps line 0 and no snippet.
    static func matches(from raw: [ShellModel.IndexLocation], texts: [String: String]) -> [Match] {
        raw.map { loc in
            guard let text = texts[loc.path], let at = locate(start: loc.start, end: loc.end, in: text) else {
                return Match(location: loc, line: 0, column: 0, snippet: nil)
            }
            return Match(location: loc, line: at.line, column: at.column, snippet: at.snippet)
        }
    }

    /// VoiceOver label of a row: "match n of m, path, line l, snippet".
    static func accessibilityLabel(index: Int, count: Int, match: Match) -> String {
        var parts = ["match \(index + 1) of \(count)", match.path]
        parts.append(match.line > 0 ? "line \(match.line)" : "line unknown")
        parts.append(match.snippet?.text ?? "text unavailable")
        return parts.joined(separator: ", ")
    }

    /// The paths whose durable version differs between two maps, for messages.
    static func changedVersions(from old: [String: Int], to new: [String: Int]) -> [String] {
        Set(old.keys).union(new.keys).filter { old[$0] != new[$0] }.sorted().map { path in
            "\(path) r\(old[path].map(String.init) ?? "–")→r\(new[path].map(String.init) ?? "–")"
        }
    }

    static let noHelperMessage = "Search requires the durable helper (flashtex-preview-controller) to be attached and ready."
}

// MARK: - client

/// Bounded client for one search panel: one `search_literal` in flight, one
/// bounded automatic retry after the helper's stale-version error, explicit
/// limits, and navigation that re-reads the helper's current versions before
/// selecting anything. Runs on the main actor with the shell model.
@MainActor
@Observable
final class ProjectSearchClient {
    /// Where to search: the whole helper project, or only the active document
    /// (the contract's optional `documents` filter).
    enum Scope: String, CaseIterable, Identifiable {
        case project = "Whole project"
        case activeDocument = "Active document"
        var id: String { rawValue }
    }

    private(set) var model: ShellModel
    var query = ""
    var scope: Scope = .project
    var maxMatches = ProjectSearch.defaultMaxMatches
    var maxWork = ProjectSearch.defaultMaxWork
    private(set) var results: ProjectSearch.Results?
    /// What the panel says under the field: helper state, errors, refusals, navigation notes.
    private(set) var status = ""
    private(set) var isSearching = false
    var selectedID: ProjectSearch.Match.ID?
    /// Bumped on every accepted navigation (tests observe it).
    private(set) var navigationCount = 0
    /// Bounded time to wait for a `document` read the helper answers asynchronously.
    var documentReadTimeout: TimeInterval = 2
    /// Reviewed replacement (plan_literal_replacement → apply_group), see the extension below.
    var replacement = ""
    private(set) var plan: ProjectSearch.ReplacementPlan?
    private(set) var planPreviews: [ProjectSearch.ReplacementPreview] = []
    private(set) var planStatus = ""
    private(set) var applyOutcomes: [ProjectSearch.FileOutcome] = []
    private(set) var isApplying = false
    /// Exact `apply_group` payloads by command id until their reply arrived
    /// (an uncertain reply is retried with the identical id and payload).
    private(set) var retainedCommands: [String: PreviewControllerClient.JSONObject] = [:]

    init(model: ShellModel) { self.model = model }

    var helperAvailable: Bool { model.controllerAttached && model.controllerState.ready }

    var selectedIndex: Int? {
        guard let selectedID, let results else { return nil }
        return results.matches.firstIndex { $0.id == selectedID }
    }

    var selectedMatch: ProjectSearch.Match? { selectedIndex.map { results!.matches[$0] } }

    /// Whether the helper's versions moved since `results` were computed, as
    /// far as this session knows (durable receipts); the authoritative check
    /// is the fresh `snapshot` taken when navigating.
    var knownStaleNote: String? {
        guard let results else { return nil }
        let changed = results.sourceVersions.compactMap { path, rev -> String? in
            guard let durable = model.controllerState.durable[path]?.revision, durable != rev else { return nil }
            return "\(path) r\(rev)→r\(durable)"
        }.sorted()
        return changed.isEmpty ? nil : "Project changed since this search (\(changed.joined(separator: ", "))); search again."
    }

    /// Return in the field: search when the query is not what the list shows,
    /// otherwise go to the selected match.
    func submit() {
        if let results, results.literal == query, results.documents == (scope == .activeDocument ? [model.activePath] : nil), !results.matches.isEmpty {
            Task { await navigateToSelected() }
        } else {
            Task { await search() }
        }
    }

    func moveSelection(by delta: Int) {
        guard let results, !results.matches.isEmpty else { return }
        let next = min(max((selectedIndex ?? (delta > 0 ? -1 : results.matches.count)) + delta, 0), results.matches.count - 1)
        selectedID = results.matches[next].id
    }

    /// `snapshot` → `search_literal {source_versions, literal, max_matches,
    /// max_work}`; the reply's locations are shown with their durable line
    /// and snippet (durable text read through `document` when this session
    /// has not seen that revision). One automatic retry after the helper's
    /// stale-version error (the user was typing); the second refusal is shown.
    func search() async {
        let literal = query
        guard !isSearching else { status = "A search is already running."; return }
        guard helperAvailable else { results = nil; status = ProjectSearch.noHelperMessage; return }
        guard !literal.isEmpty else { results = nil; status = "Type a literal to search for (case-sensitive, no regex)."; return }
        guard literal.utf8.count <= ProjectSearch.maxLiteralBytes else {
            results = nil; status = "The literal is \(literal.utf8.count) bytes; the helper accepts at most \(ProjectSearch.maxLiteralBytes)."; return
        }
        isSearching = true
        status = "Searching the durable project source…"
        defer { isSearching = false }
        var attempt = 0
        while true {
            attempt += 1
            guard let versionsReply = await model.controllerSourceVersions() else { results = nil; status = ProjectSearch.noHelperMessage; return }
            let versions: [String: Int]
            switch versionsReply {
            case .failure(let e): results = nil; status = "Snapshot refused: \(e.message)"; return
            case .success(let v): versions = v
            }
            let documents: [String]? = scope == .activeDocument ? [model.activePath] : nil
            if let documents, let missing = documents.first(where: { versions[$0] == nil }) {
                results = nil
                status = "\(missing) is not part of the helper's project (indexed: \(versions.keys.sorted().joined(separator: ", ")))."
                return
            }
            let payload = ProjectSearch.request(sourceVersions: versions, literal: literal, maxMatches: maxMatches, maxWork: maxWork, documents: documents)
            guard let reply = await model.controllerRequest("search_literal", payload) else { results = nil; status = ProjectSearch.noHelperMessage; return }
            switch reply {
            case .failure(let e):
                if e.message.contains("source versions changed"), attempt == 1 { continue } // one bounded retry
                results = nil
                status = e.message.contains("source versions changed")
                    ? "Project changed while searching (\(e.message)); search again."
                    : "Search refused by the helper: \(e.message)"
                return
            case .success(let dict):
                switch ProjectSearch.parse(reply: dict) {
                case .failure(let why): results = nil; status = "Search reply refused: \(why.message)"; return
                case .success(let raw):
                    var texts: [String: String] = [:]
                    for path in Set(raw.matches.map(\.path)).sorted() {
                        if let rev = raw.sourceVersions[path], let text = await durableText(path: path, revision: rev) { texts[path] = text }
                    }
                    let matches = ProjectSearch.matches(from: raw.matches, texts: texts)
                    let out = ProjectSearch.Results(literal: literal, sourceVersions: raw.sourceVersions, documents: documents, matches: matches,
                                                    termination: raw.termination, workUsed: raw.workUsed,
                                                    maxMatches: ProjectSearch.clamp(maxMatches, to: ProjectSearch.matchLimitRange),
                                                    maxWork: ProjectSearch.clamp(maxWork, to: ProjectSearch.workLimitRange))
                    results = out
                    selectedID = matches.first?.id
                    let unread = matches.filter { $0.snippet == nil }.count
                    let scopeName: String = documents?.joined(separator: ", ") ?? "the project"
                    var line = "\(out.summary) for “\(literal)” in \(scopeName) (durable source, work \(raw.workUsed))"
                    if unread > 0 { line += "; \(unread) without text (durable revision not readable)" }
                    if attempt > 1 { line += "; retried once after the project changed" }
                    status = line
                    return
                }
            }
        }
    }

    /// The durable text of `path` at `revision`: what this session recorded
    /// from `document`/`edit` results, else a `document` read (the parent's
    /// handler records it) awaited for a bounded time. Nil when unavailable
    /// or when the helper is at another revision by the time it answers.
    func durableText(path: String, revision: Int) async -> String? {
        if let text = model.controllerState.textByDurable[path]?[revision] { return text }
        guard let controller = model.controller, controller.isRunning else { return nil }
        _ = try? controller.document(path: path)
        let deadline = Date().addingTimeInterval(documentReadTimeout)
        while model.controllerState.textByDurable[path]?[revision] == nil, Date() < deadline, model.controllerAttached {
            try? await Task.sleep(nanoseconds: 5_000_000)
        }
        return model.controllerState.textByDurable[path]?[revision]
    }

    /// ⌘G in the panel: advance the selection (wrapping) and go there.
    func navigateNext() async {
        guard let results, !results.matches.isEmpty else { status = "No matches to step through."; return }
        let next = ((selectedIndex ?? -1) + 1) % results.matches.count
        selectedID = results.matches[next].id
        await navigate(to: next, of: results)
    }

    func navigateToSelected() async {
        guard let results, let index = selectedIndex else { status = "No match selected."; return }
        await navigate(to: index, of: results)
    }

    /// Goes to match `index` exactly or refuses with the reason:
    /// 1. the helper's current `snapshot` must equal the versions the search
    ///    ran on (a durable edit anywhere since is the stale-version rule);
    /// 2. the durable bytes at the range must still spell the literal;
    /// 3. `ShellModel.selectIndexLocation` maps the durable range onto the
    ///    open buffer with `Navigation.rebaseExactly` (refusing a range that
    ///    overlaps a local edit) and selects whole composed characters,
    ///    opening a project document that is not open in the window.
    func navigate(to index: Int, of results: ProjectSearch.Results) async {
        guard index >= 0, index < results.matches.count else { status = "No match \(index + 1)."; return }
        let match = results.matches[index]
        let label = "Match \(index + 1) of \(results.matches.count)\(results.termination.isExhaustive ? "" : "+") for “\(results.literal)”"
        guard helperAvailable, let versionsReply = await model.controllerSourceVersions() else { status = ProjectSearch.noHelperMessage; return }
        let fresh: [String: Int]
        switch versionsReply {
        case .failure(let e): status = "\(label): snapshot refused (\(e.message)); not navigated."; return
        case .success(let v): fresh = v
        }
        guard fresh == results.sourceVersions else {
            let changed = ProjectSearch.changedVersions(from: results.sourceVersions, to: fresh).joined(separator: ", ")
            status = "\(label): project changed since this search (\(changed)); search again."
            return
        }
        guard let durable = await durableText(path: match.path, revision: match.location.revision) else {
            status = "\(label): durable revision \(match.location.revision) of \(match.path) could not be read; not navigated."
            return
        }
        guard ProjectSearch.bytesSpell(results.literal, in: durable, start: match.location.start, end: match.location.end) else {
            status = "\(label): bytes \(match.location.start)..<\(match.location.end) of \(match.path) r\(match.location.revision) no longer spell the literal; search again."
            return
        }
        let before = model.selection?.token
        await model.selectIndexLocation(match.location, versions: fresh, label: label)
        if let sel = model.selection, sel.token != before, sel.path == match.path {
            navigationCount += 1
            status = model.navigationNote ?? label
        } else {
            status = model.navigationNote.map { "Not navigated — " + $0 } ?? "\(label): not navigated."
        }
    }
}

// MARK: - reviewed literal replacement (plan_literal_replacement → apply_group)

/// The helper's read-only replacement proposal (`plan_literal_replacement`,
/// crates/preview-controller/docs/source-plans.md, schema
/// `flashtex.literal-replacement-plan.v1`) and the client's obligations when
/// the user applies it: every offset/revision in the plan is a decimal string
/// parsed as an exact integer; the plan is shown per file with exact
/// before/after text; applying refreshes `snapshot` and requires the plan's
/// full version map and membership generation; each file is one
/// `apply_group` with a fresh retained command id, the reviewed
/// revision/hash and `removed_text = expected_text`, verified byte-for-byte
/// against the durable text first; outcomes are per file — `apply_group`
/// guards one document and nothing here claims an all-or-nothing project
/// edit. The helper never authenticates approval: the Apply button after the
/// displayed plan is the approval, and a plan is never applied automatically.
extension ProjectSearch {
    static let planSchema = "flashtex.literal-replacement-plan.v1"
    /// `max_bytes` for the serialized proposal (1…8 MiB).
    static let planMaxBytes = 4 * 1024 * 1024

    struct ReplacementEdit: Equatable {
        var path: String
        var revision: Int
        var start: Int
        var end: Int
        var expectedText: String
        var replacement: String
    }

    struct ReplacementPlan: Equatable {
        var literal: String
        var replacement: String
        var projectID: String
        var sourceVersions: [String: Int]
        var membershipGeneration: Int
        var documents: [String]?
        var workUsed: Int
        /// Helper order: path, then byte offset; nonoverlapping within a file.
        var edits: [ReplacementEdit]

        var paths: [String] { Array(Set(edits.map(\.path))).sorted() }
        func edits(in path: String) -> [ReplacementEdit] { edits.filter { $0.path == path } }
        var summary: String {
            let n = edits.count, files = paths.count
            return "\(n) replacement\(n == 1 ? "" : "s") in \(files) file\(files == 1 ? "" : "s")"
        }
    }

    /// A decimal-string (or integral JSON number) field as an exact `Int`;
    /// never through floating point (source-plans.md).
    static func exactInt(_ value: Any?) -> Int? {
        if let s = value as? String {
            guard !s.isEmpty, s.count <= 18, s.allSatisfy({ $0.isASCII && $0.isNumber }) else { return nil }
            return Int(s)
        }
        if let n = value as? NSNumber, CFNumberIsFloatType(n) == false { return n.intValue }
        return nil
    }

    /// Parses and validates a `plan_literal_replacement` reply for the literal
    /// and replacement the user asked for. Anything unexpected refuses the
    /// whole plan; nothing is inferred.
    static func parsePlan(reply payload: [String: Any], literal: String, replacement: String) -> Result<ReplacementPlan, ControllerError> {
        func refuse(_ why: String) -> Result<ReplacementPlan, ControllerError> { .failure(.init(message: "replacement plan refused: " + why)) }
        guard let versions = payload["source_versions"] as? [String: Int] else { return refuse("reply has no source_versions") }
        guard let generation = exactInt(payload["membership_generation"]) else { return refuse("reply has no membership_generation") }
        guard let plan = payload["plan"] as? [String: Any] else { return refuse("reply has no plan") }
        guard plan["schema"] as? String == planSchema else { return refuse("schema is \(plan["schema"] ?? "missing"), not \(planSchema)") }
        guard plan["proposal_only"] as? Bool == true, plan["requires_user_approval"] as? Bool == true else {
            return refuse("plan is not marked proposal_only + requires_user_approval")
        }
        guard plan["application_order"] as? String == "reverse_byte_offset_per_document" else {
            return refuse("unknown application_order \(plan["application_order"] ?? "missing")")
        }
        guard let snapshot = plan["snapshot"] as? [String: Any], let projectID = snapshot["project_id"] as? String,
              let snapGeneration = exactInt(snapshot["generation"]), let snapDocs = snapshot["documents"] as? [[String: Any]] else {
            return refuse("plan snapshot is malformed")
        }
        guard snapGeneration == generation else { return refuse("plan generation \(snapGeneration) is not the reply's \(generation)") }
        var snapVersions: [String: Int] = [:]
        for d in snapDocs {
            guard let file = d["file"] as? String, let rev = exactInt(d["revision"]) else { return refuse("plan snapshot document is malformed") }
            snapVersions[file] = rev
        }
        guard snapVersions == versions else { return refuse("plan snapshot versions \(snapVersions) differ from the reply's \(versions)") }
        guard let search = plan["search"] as? [String: Any] else { return refuse("plan has no search") }
        guard search["literal"] as? String == literal else { return refuse("plan literal is not “\(literal)”") }
        guard search["termination"] as? String == "complete" else { return refuse("plan search is not complete") }
        guard plan["replacement"] as? String == replacement else { return refuse("plan replacement is not “\(replacement)”") }
        let documents = search["documents"] as? [String]
        let workUsed = exactInt(search["work_used"]) ?? 0
        guard let rawEdits = plan["edits"] as? [[String: Any]] else { return refuse("plan has no edits") }
        var edits: [ReplacementEdit] = []
        var lastByPath: [String: Int] = [:]
        for (i, e) in rawEdits.enumerated() {
            guard let file = e["file"] as? String, let rev = exactInt(e["revision"]), let start = exactInt(e["start_byte"]),
                  let end = exactInt(e["end_byte"]), let expected = e["expected_text"] as? String, let repl = e["replacement"] as? String else {
                return refuse("edit \(i) is malformed")
            }
            guard versions[file] == rev else { return refuse("edit \(i) names \(file) r\(rev), not the snapshot's r\(versions[file].map(String.init) ?? "?")") }
            guard start <= end, end - start == literal.utf8.count, expected == literal, repl == replacement else {
                return refuse("edit \(i) (\(file) \(start)..<\(end)) does not replace exactly “\(literal)” with “\(replacement)”")
            }
            if let last = lastByPath[file], start < last { return refuse("edit \(i) in \(file) overlaps or precedes the previous edit") }
            lastByPath[file] = end
            edits.append(.init(path: file, revision: rev, start: start, end: end, expectedText: expected, replacement: repl))
        }
        return .success(.init(literal: literal, replacement: replacement, projectID: projectID, sourceVersions: versions,
                              membershipGeneration: generation, documents: documents, workUsed: workUsed, edits: edits))
    }

    /// Every edit's bytes must spell its expected text in `text` (the durable
    /// text at the edit's revision); nil when all do, else the first reason.
    static func verify(_ edits: [ReplacementEdit], in text: String) -> String? {
        for e in edits where !bytesSpell(e.expectedText, in: text, start: e.start, end: e.end) {
            return "bytes \(e.start)..<\(e.end) of \(e.path) r\(e.revision) no longer spell “\(e.expectedText)”"
        }
        return nil
    }

    /// The text after applying `edits` (same original snapshot, applied in
    /// reverse byte order); nil when a range is not scalar-aligned.
    static func applied(_ edits: [ReplacementEdit], to text: String) -> String? {
        var out = text
        for e in edits.sorted(by: { $0.start > $1.start }) {
            guard let r = out.rangeOfUTF8(start: e.start, end: e.end) else { return nil }
            out.replaceSubrange(r, with: e.replacement)
        }
        return out
    }

    /// One document's `apply_group` payload (STDIO.md): `removed_text` is the
    /// reviewed `expected_text`, ranges are the original snapshot's.
    static func applyGroupPayload(path: String, commandID: String, expectedRevision: Int, expectedSHA256: String,
                                  label: String, edits: [ReplacementEdit]) -> PreviewControllerClient.JSONObject {
        ["path": path,
         "command": ["command_id": commandID, "expected_revision": expectedRevision, "expected_sha256": expectedSHA256, "label": label,
                     "edits": edits.map { ["start_byte": $0.start, "end_byte": $0.end, "removed_text": $0.expectedText, "replacement": $0.replacement] as [String: Any] }] as [String: Any]]
    }

    /// Before/after view of one edit on its durable line.
    struct ReplacementPreview: Identifiable, Equatable {
        var edit: ReplacementEdit
        var line: Int
        var before: Snippet?
        var after: Snippet?
        var id: String { "\(edit.path)@\(edit.revision):\(edit.start)..<\(edit.end)" }
    }

    static func previews(for plan: ReplacementPlan, texts: [String: String]) -> [ReplacementPreview] {
        plan.edits.map { e in
            guard let text = texts[e.path], let at = locate(start: e.start, end: e.end, in: text) else {
                return ReplacementPreview(edit: e, line: 0, before: nil, after: nil)
            }
            var after = at.snippet
            after.match = e.replacement.replacingOccurrences(of: "\r\n", with: "⏎").replacingOccurrences(of: "\n", with: "⏎")
            return ReplacementPreview(edit: e, line: at.line, before: at.snippet, after: after)
        }
    }

    static func accessibilityLabel(index: Int, count: Int, preview: ReplacementPreview) -> String {
        let where_ = preview.line > 0 ? "line \(preview.line)" : "bytes \(preview.edit.start) to \(preview.edit.end)"
        let change = preview.before.map { "\($0.text) becomes \(preview.after?.text ?? "")" } ?? "“\(preview.edit.expectedText)” becomes “\(preview.edit.replacement)”"
        return "replacement \(index + 1) of \(count), \(preview.edit.path), \(where_), \(change)"
    }

    /// Per-file result of applying a plan; never summarized as all-or-nothing.
    struct FileOutcome: Identifiable, Equatable {
        enum State: Equatable {
            case applied(revision: Int, commandID: String, note: String)
            case refused(String)
            /// Delivery uncertain (helper exited, reply lost): retry with the
            /// same command id and payload, never a new id.
            case uncertain(String, commandID: String)
        }
        var path: String
        var state: State
        var id: String { path }

        var description: String {
            switch state {
            case .applied(let rev, _, let note): return "\(path): applied as durable r\(rev)" + note
            case .refused(let why): return "\(path): not applied — \(why)"
            case .uncertain(let why, let id): return "\(path): uncertain — \(why); retry command \(id) unchanged"
            }
        }
    }
}

extension ProjectSearchClient {
    struct Snapshot: Equatable {
        var versions: [String: Int]
        var generation: Int
    }

    /// `snapshot` with its membership generation (plans need both).
    func controllerSnapshot() async -> Result<Snapshot, ControllerError>? {
        guard let reply = await model.controllerRequest("snapshot", [:]) else { return nil }
        switch reply {
        case .failure(let e): return .failure(e)
        case .success(let payload):
            guard let versions = payload["source_versions"] as? [String: Int], let generation = ProjectSearch.exactInt(payload["membership_generation"]) else {
                return .failure(.init(message: "snapshot reply has no source_versions/membership_generation"))
            }
            return .success(.init(versions: versions, generation: generation))
        }
    }

    /// Asks the helper for a read-only replacement proposal for the current
    /// query/scope and `replacement`, and shows it. Nothing is edited.
    func planReplacement() async {
        let literal = query, replacement = self.replacement
        plan = nil; planPreviews = []; applyOutcomes = []
        guard helperAvailable else { planStatus = ProjectSearch.noHelperMessage; return }
        guard !literal.isEmpty else { planStatus = "Type the literal to replace first."; return }
        guard literal != replacement else { planStatus = "The replacement equals the literal; nothing to change."; return }
        planStatus = "Asking the helper for a replacement proposal…"
        guard let snapReply = await controllerSnapshot() else { planStatus = ProjectSearch.noHelperMessage; return }
        let snap: Snapshot
        switch snapReply {
        case .failure(let e): planStatus = "Snapshot refused: \(e.message)"; return
        case .success(let s): snap = s
        }
        let documents: [String]? = scope == .activeDocument ? [model.activePath] : nil
        var payload = ProjectSearch.request(sourceVersions: snap.versions, literal: literal, maxMatches: maxMatches, maxWork: maxWork, documents: documents)
        payload["membership_generation"] = snap.generation
        payload["max_bytes"] = ProjectSearch.planMaxBytes
        payload["replacement"] = replacement
        guard let reply = await model.controllerRequest("plan_literal_replacement", payload) else { planStatus = ProjectSearch.noHelperMessage; return }
        switch reply {
        case .failure(let e):
            planStatus = e.message == "unknown operation"
                ? "This helper build has no plan_literal_replacement (needs crates/preview-controller from main ≥ 4e15783)."
                : "Replacement plan refused by the helper: \(e.message)"
        case .success(let dict):
            switch ProjectSearch.parsePlan(reply: dict, literal: literal, replacement: replacement) {
            case .failure(let e): planStatus = e.message
            case .success(let p):
                var texts: [String: String] = [:]
                for path in p.paths { if let t = await durableText(path: path, revision: p.sourceVersions[path] ?? -1) { texts[path] = t } }
                plan = p
                planPreviews = ProjectSearch.previews(for: p, texts: texts)
                planStatus = p.edits.isEmpty
                    ? "No occurrence of “\(literal)” in the durable source; nothing to replace."
                    : "Proposal: \(p.summary), replacing “\(literal)” with “\(replacement)” at durable "
                        + p.paths.map { "\($0) r\(p.sourceVersions[$0]!)" }.joined(separator: ", ")
                        + ". Nothing is changed until you click Apply."
            }
        }
    }

    /// The user's Apply: re-checks the snapshot, then one guarded
    /// `apply_group` per file (see the extension comment). Per-file outcomes
    /// are listed; the search is re-run afterwards so the list shows what
    /// remains. Only called from the Apply button after the plan was shown.
    func applyPlan() async {
        guard let plan, !plan.edits.isEmpty else { planStatus = "No proposal to apply."; return }
        guard !isApplying else { return }
        isApplying = true
        defer { isApplying = false }
        applyOutcomes = []
        let label = "Replace “\(plan.literal)” with “\(plan.replacement)”"
        switch await applyReviewedEdits(plan.edits, sourceVersions: plan.sourceVersions, membershipGeneration: plan.membershipGeneration,
                                        label: label, commandPrefix: "search-replace") {
        case .failure(let why):
            planStatus = why.message
            if why.message.hasPrefix("Project changed") { self.plan = nil; planPreviews = [] }
            return
        case .success(let outcomes):
            applyOutcomes = outcomes
        }
        let applied = applyOutcomes.filter { if case .applied = $0.state { return true } else { return false } }.count
        let refused = applyOutcomes.count - applied
        planStatus = "\(label): \(applied) of \(applyOutcomes.count) file\(applyOutcomes.count == 1 ? "" : "s") applied"
            + (refused > 0 ? ", \(refused) not applied (see below; files are applied one by one, never all-or-nothing)" : "")
            + ". Undo is the ledger's grouped undo per file."
        self.plan = nil; planPreviews = []
        if applied > 0 {
            await settleAfterApply()
            await search()
        }
    }

    // MARK: reusable reviewed-application core (search replace and citation rename)

    /// The reviewed-application contract shared by every helper plan
    /// (source-plans.md "Applying a reviewed proposal in a native client"):
    /// the fresh `snapshot` must equal the plan's full version map and
    /// membership generation (else the whole plan is refused with a
    /// "Project changed since this proposal …" message and nothing is sent),
    /// then one guarded `apply_group` per file in path order, each with a
    /// fresh retained command id `<commandPrefix>-<UUID>`, the reviewed
    /// revision/hash, byte-verified ranges and `removed_text = expected_text`.
    /// Returns the per-file outcomes; never an all-or-nothing verdict.
    func applyReviewedEdits(_ edits: [ProjectSearch.ReplacementEdit], sourceVersions: [String: Int], membershipGeneration: Int,
                            label: String, commandPrefix: String) async -> Result<[ProjectSearch.FileOutcome], ControllerError> {
        guard !edits.isEmpty else { return .failure(.init(message: "No proposal to apply.")) }
        guard helperAvailable, let snapReply = await controllerSnapshot() else { return .failure(.init(message: ProjectSearch.noHelperMessage)) }
        switch snapReply {
        case .failure(let e): return .failure(.init(message: "Snapshot refused: \(e.message); nothing applied."))
        case .success(let snap):
            guard snap.versions == sourceVersions, snap.generation == membershipGeneration else {
                let changed = ProjectSearch.changedVersions(from: sourceVersions, to: snap.versions).joined(separator: ", ")
                return .failure(.init(message: "Project changed since this proposal (\(changed.isEmpty ? "membership generation \(membershipGeneration)→\(snap.generation)" : changed)); nothing applied — plan again."))
            }
        }
        var outcomes: [ProjectSearch.FileOutcome] = []
        for path in Array(Set(edits.map(\.path))).sorted() {
            let fileEdits = edits.filter { $0.path == path }
            let revision = sourceVersions[path] ?? -1
            outcomes.append(await applyGroup(path: path, revision: revision, edits: fileEdits,
                                             label: label + " (\(fileEdits.count) in \(path))", commandPrefix: commandPrefix))
        }
        return .success(outcomes)
    }

    /// After at least one file applied: let the preview catch up with the
    /// moved durable text (a preview for the applied revision may have raced
    /// the buffer update) and, only when the active buffer is exactly
    /// durable, `compile` — never a submission of the user's pending edits.
    func settleAfterApply() async {
        let deadline = Date().addingTimeInterval(1)
        while model.result?.revision != model.editorRevision, Date() < deadline, model.controllerAttached {
            try? await Task.sleep(nanoseconds: 20_000_000)
        }
        if model.result?.revision != model.editorRevision, model.autoCompile, let d = model.controllerState.durable[model.activePath],
           model.controllerState.textByDurable[model.activePath]?[d.revision]?.sameBytes(as: model.activeText) == true {
            model.controllerCompile()
        }
    }

    /// One document's guarded `apply_group`, then reconciliation of the
    /// shell's durable state and buffer with the helper's returned document
    /// (validated against the editor snapshot taken at the send: `reconcile`).
    private func applyGroup(path: String, revision: Int, edits: [ProjectSearch.ReplacementEdit], label: String, commandPrefix: String) async -> ProjectSearch.FileOutcome {
        func refused(_ why: String) -> ProjectSearch.FileOutcome { .init(path: path, state: .refused(why)) }
        // An edit still in flight for this path would make the reviewed hash stale: wait, bounded.
        let flightDeadline = Date().addingTimeInterval(2)
        while model.controllerState.inFlight?.path == path, Date() < flightDeadline { try? await Task.sleep(nanoseconds: 10_000_000) }
        if model.controllerState.inFlight?.path == path { return refused("an edit of \(path) is still in flight") }
        guard let durableText = await durableText(path: path, revision: revision) else { return refused("durable revision \(revision) could not be read") }
        if let why = ProjectSearch.verify(edits, in: durableText) { return refused(why) }
        guard let durable = model.controllerState.durable[path], durable.revision == revision else {
            return refused("durable revision moved to r\(model.controllerState.durable[path]?.revision ?? -1) before applying")
        }
        if let open = model.documents.first(where: { $0.path == path }), !open.text.sameBytes(as: durableText) {
            return refused("the open buffer has edits that are not durable yet (durable r\(revision)); wait, then plan again")
        }
        let commandID = "\(commandPrefix)-\(UUID().uuidString)"
        let payload = ProjectSearch.applyGroupPayload(path: path, commandID: commandID, expectedRevision: revision,
                                                      expectedSHA256: durable.sha256, label: label, edits: edits)
        retainedCommands[commandID] = payload
        return await sendApplyGroup(path: path, commandID: commandID, payload: payload, edits: edits)
    }

    /// The exact editor state an `apply_group` is sent against (GH39): the
    /// editor revision and the byte-exact buffer of `path` at the send. The
    /// reply is reconciled against THIS, never against the helper's command
    /// revision, the returned document revision/hash or a compile admission —
    /// those are different authorities and none of them can say whether the
    /// user typed meanwhile.
    @MainActor
    struct EditorSnapshot: Equatable {
        var editorRevision: Int
        var text: String

        /// Whether the open buffer of `path` (the active buffer, or an open
        /// non-active document) still equals this snapshot.
        func stillCurrent(in model: ShellModel, path: String) -> Bool {
            if path == model.activePath {
                return model.editorRevision == editorRevision && model.activeText.sameBytes(as: text)
            }
            guard let open = model.documents.first(where: { $0.path == path }) else { return true } // not open: nothing local to protect
            return open.text.sameBytes(as: text)
        }

        static func take(_ model: ShellModel, path: String) -> EditorSnapshot {
            let text = path == model.activePath ? model.activeText : (model.documents.first(where: { $0.path == path })?.text ?? "")
            return .init(editorRevision: model.editorRevision, text: text)
        }
    }

    /// `controllerRequest` (Navigation.swift) that also yields the request id,
    /// so the adopted history result can be recorded as the in-flight edit its
    /// follow-up preview binds to (exactly what the history panel does).
    private func controllerRequestTracked(_ type: String, _ payload: PreviewControllerClient.JSONObject) async -> (id: String, reply: Result<[String: Any], ControllerError>)? {
        guard let controller = model.controller, controller.isRunning, model.controllerState.ready else { return nil }
        let id: String
        do { id = try controller.send(type, payload) } catch { return ("", .failure(.init(message: "\(type) failed to send: \(error.localizedDescription)"))) } // nothing was sent: no id
        return await withCheckedContinuation { cont in
            model.controllerState.awaiting[id] = { cont.resume(returning: (id, $0)) }
        }
    }

    private func sendApplyGroup(path: String, commandID: String, payload: PreviewControllerClient.JSONObject, edits: [ProjectSearch.ReplacementEdit]) async -> ProjectSearch.FileOutcome {
        let snapshot = EditorSnapshot.take(model, path: path) // recorded BEFORE the send; the await below is the GH39 window
        guard let (requestID, reply) = await controllerRequestTracked("apply_group", payload) else {
            return .init(path: path, state: .uncertain("helper detached before the reply", commandID: commandID))
        }
        switch reply {
        case .failure(let e):
            if e.message.hasPrefix("helper exited") || e.message.contains("not admitted") {
                return .init(path: path, state: .uncertain(e.message, commandID: commandID))
            }
            return .init(path: path, state: .refused("helper refused apply_group: \(e.message)"))
        case .success(let dict):
            guard let history = dict["history"] as? [String: Any], let doc = history["document"] as? [String: Any],
                  doc["path"] as? String == path, let newRevision = doc["revision"] as? Int,
                  let text = doc["text"] as? String, doc["source_sha256"] is String else {
                return .init(path: path, state: .uncertain("apply_group reply has no history.document for \(path)", commandID: commandID))
            }
            retainedCommands.removeValue(forKey: commandID)
            var note = ""
            if let reviewed = edits.first.flatMap({ model.controllerState.textByDurable[path]?[$0.revision] }),
               let expected = ProjectSearch.applied(edits, to: reviewed), !expected.sameBytes(as: text) {
                note = " (helper text differs from the locally expected result; the helper's is authoritative)"
            }
            if history["replayed_command"] as? Bool == true { note += " (replayed: the ledger had already applied this command)" }
            if let e = dict["preview_error"] as? String { note += " (preview error: \(e))" }
            if !reconcile(path: path, document: doc, requestID: requestID, payload: dict, snapshot: snapshot) {
                note += " (the editor moved during the apply: your newer text is kept, not overwritten, and is resubmitted on top of durable r\(newRevision))"
            }
            return .init(path: path, state: .applied(revision: newRevision, commandID: commandID, note: note))
        }
    }

    /// Reconciles the shell with the helper's post-apply document (GH39).
    /// The durable identity (revision/hash/text) is recorded FIRST and
    /// unconditionally through `controllerAdoptHistoryResult` — the same
    /// single writer the history panel uses, so `document`/`edit` results,
    /// undo/redo and grouped applies record durability identically. The
    /// buffer is replaced with the returned text ONLY when it is still exactly
    /// the `snapshot` the command was sent against (same editor revision and
    /// bytes; the active document then goes through `updateActiveText` —
    /// editor revision, bridge, auto-compile — and no edit is sent because the
    /// buffer is already durable). When the editor moved during the round trip
    /// the local text is kept and the normal `controllerSubmitEdit` path
    /// resubmits it on top of the new durable revision: the helper's command
    /// revision, the returned document revision/hash and any compile admission
    /// are never permission to overwrite newer local text. Returns false when
    /// the editor had moved (the buffer was kept).
    private func reconcile(path: String, document: [String: Any], requestID: String, payload: [String: Any], snapshot: EditorSnapshot) -> Bool {
        let current = snapshot.stillCurrent(in: model, path: path)
        let revision = document["revision"] as? Int ?? -1
        // `Int.min` never equals an editor revision, so the buffer is kept even
        // if the revision matched but the bytes did not (never observed; the
        // byte check is the belt to the revision's braces).
        model.controllerAdoptHistoryResult(document, requestID: requestID, payload: payload,
                                           issuedAtEditorRevision: current ? snapshot.editorRevision : Int.min)
        if current, path != model.activePath, let i = model.documents.firstIndex(where: { $0.path == path }),
           let text = document["text"] as? String, !model.documents[i].text.sameBytes(as: text) {
            model.documents[i].text = text // an open non-active document that still equals the snapshot
        }
        if !current {
            // The returned document derives from the editor state at the send
            // plus the reviewed replacement, not from the current buffer: bind
            // its preview to the send-time editor revision so it is shown as
            // stale until the resubmitted buffer's own preview arrives.
            model.controllerState.editorRevisionByDurable[path, default: [:]][revision] = snapshot.editorRevision
            model.log("\(path): editor moved during apply_group (editor revision \(snapshot.editorRevision) → \(model.editorRevision)); durable r\(revision) recorded, local text kept and resubmitted")
        }
        return current
    }

    /// Retries an uncertain file with its retained command id and payload
    /// (never a new id): the ledger replays or refuses it exactly.
    func retryUncertain(commandID: String) async {
        guard retainedCommands[commandID] != nil else { return }
        guard helperAvailable else { planStatus = ProjectSearch.noHelperMessage; return }
        guard let outcome = await retryRetained(commandID: commandID) else { return }
        if let i = applyOutcomes.firstIndex(where: { $0.path == outcome.path }) { applyOutcomes[i] = outcome } else { applyOutcomes.append(outcome) }
        planStatus = outcome.description
    }

    /// Re-sends the retained `apply_group` payload under `commandID` exactly
    /// (identical id and payload); nil when no such command is retained or
    /// the helper is unavailable. Shared with the citation rename client.
    func retryRetained(commandID: String) async -> ProjectSearch.FileOutcome? {
        guard let payload = retainedCommands[commandID], let path = payload["path"] as? String,
              let command = payload["command"] as? [String: Any], let rawEdits = command["edits"] as? [[String: Any]] else { return nil }
        guard helperAvailable else { return nil }
        let revision = command["expected_revision"] as? Int ?? -1
        let edits = rawEdits.compactMap { e -> ProjectSearch.ReplacementEdit? in
            guard let s = e["start_byte"] as? Int, let en = e["end_byte"] as? Int, let r = e["removed_text"] as? String, let rp = e["replacement"] as? String else { return nil }
            return .init(path: path, revision: revision, start: s, end: en, expectedText: r, replacement: rp)
        }
        return await sendApplyGroup(path: path, commandID: commandID, payload: payload, edits: edits)
    }
}

// MARK: - view

/// The ⌘⇧F window: a literal field, the result list with explicit
/// termination, and the refusal/navigation note. Return searches (new query)
/// or goes to the selected match; ↑/↓ move the selection from the field or
/// the list; Esc closes the window.
struct ProjectSearchPanel: View {
    static let windowID = ProjectSearch.windowID
    @Environment(ShellModel.self) private var model
    @Environment(\.dismissWindow) private var dismissWindow
    @State private var client: ProjectSearchClient?
    @FocusState private var fieldFocused: Bool

    var body: some View {
        Group {
            if let client {
                ProjectSearchBody(client: client, fieldFocused: $fieldFocused)
            } else {
                ProgressView().onAppear {
                    let c = ProjectSearchClient(model: model)
                    client = c
                    // Automation evidence only (launch-check screenshots): seed a
                    // query and run it once the helper is ready (bounded wait).
                    if let seed = ProcessInfo.processInfo.environment["FLASHTEX_SEARCH_QUERY"], !seed.isEmpty {
                        c.query = seed
                        if let n = ProcessInfo.processInfo.environment["FLASHTEX_SEARCH_MAX_MATCHES"].flatMap(Int.init) { c.maxMatches = n }
                        Task {
                            let deadline = Date().addingTimeInterval(10)
                            while !c.helperAvailable, Date() < deadline { try? await Task.sleep(nanoseconds: 50_000_000) }
                            await c.search()
                            if let r = ProcessInfo.processInfo.environment["FLASHTEX_SEARCH_REPLACEMENT"], !r.isEmpty {
                                c.replacement = r
                                await c.planReplacement() // proposal only; Apply stays a click
                            }
                        }
                    }
                }
            }
        }
        .frame(minWidth: DS.Layout.searchWindowMinWidth, minHeight: DS.Layout.searchWindowMinHeight)
        .onAppear { fieldFocused = true }
        .background {
            // Esc anywhere in the window closes it (the cancel action).
            Button("Close") { dismissWindow(id: Self.windowID) }
                .keyboardShortcut(.cancelAction)
                .hidden()
        }
    }
}

private struct ProjectSearchBody: View {
    @Bindable var client: ProjectSearchClient
    var fieldFocused: FocusState<Bool>.Binding

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.m) {
            HStack {
                TextField("Find in project (case-sensitive literal, no regex)", text: $client.query)
                    .textFieldStyle(.roundedBorder)
                    .focused(fieldFocused)
                    .onSubmit { client.submit() }
                    .onKeyPress(.upArrow) { client.moveSelection(by: -1); return .handled }
                    .onKeyPress(.downArrow) { client.moveSelection(by: 1); return .handled }
                    .accessibilityLabel("Literal to find in the project, case-sensitive")
                    .accessibilityHint("Return searches the durable project source or goes to the selected match; up and down arrows move the selection")
                Button("Search") { Task { await client.search() } }
                    .disabled(client.isSearching || client.query.isEmpty)
                Button("Go to Match") { Task { await client.navigateToSelected() } }
                    .disabled(client.selectedMatch == nil)
                Button("Next Match") { Task { await client.navigateNext() } }
                    .keyboardShortcut("g", modifiers: .command)
                    .disabled(client.results?.matches.isEmpty ?? true)
                    .help("Select the next match (wrapping) and go there (⌘G while this window is key)")
            }
            HStack(spacing: DS.Space.l) {
                Text("Case-sensitive literal; the helper has no regex or normalization.").font(.caption).foregroundStyle(.secondary)
                Picker("Scope", selection: $client.scope) {
                    ForEach(ProjectSearchClient.Scope.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.menu).font(.caption).fixedSize()
                .accessibilityLabel("Search scope")
                Spacer()
                Stepper("Max matches: \(client.maxMatches)", value: $client.maxMatches, in: ProjectSearch.matchLimitRange, step: 50)
                    .font(.caption)
                    .help("The helper's match limit (1–1000); a search stopped here is labelled partial")
            }
            if !client.helperAvailable {
                Label(ProjectSearch.noHelperMessage, systemImage: "exclamationmark.triangle")
                    .foregroundStyle(DS.Colors.severityWarning)
                    .accessibilityLabel(ProjectSearch.noHelperMessage)
            }
            if let results = client.results {
                HStack(spacing: DS.Space.m) {
                    Text(results.summary).bold()
                    Text(results.termination.badge)
                        .font(DS.Fonts.secondary).padding(.horizontal, DS.Space.s).padding(.vertical, DS.Space.xxs)
                        .background((results.termination.isExhaustive ? DS.Colors.severitySuccess : DS.Colors.severityWarning).opacity(DS.State.badgeFillOpacity), in: Capsule())
                        .help(results.termination.explanation(maxMatches: results.maxMatches, maxWork: results.maxWork))
                    if !results.termination.isExhaustive {
                        Text("not exhaustive").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityWarning)
                    }
                    Spacer()
                    Text("durable " + results.sourceVersions.keys.sorted().map { "\($0) r\(results.sourceVersions[$0]!)" }.joined(separator: ", "))
                        .font(.caption).foregroundStyle(.secondary).lineLimit(1)
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel(results.summary + ". " + results.termination.explanation(maxMatches: results.maxMatches, maxWork: results.maxWork))
                if let stale = client.knownStaleNote {
                    Label(stale, systemImage: "clock.arrow.circlepath").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityWarning)
                }
                List(selection: $client.selectedID) {
                    ForEach(Array(results.matches.enumerated()), id: \.element.id) { (index: Int, match: ProjectSearch.Match) in
                        ProjectSearchRow(index: index, count: results.matches.count, match: match)
                            .tag(match.id)
                    }
                }
                .onKeyPress(.return) { client.submit(); return .handled }
                .accessibilityLabel("Search results, \(results.summary)")
            } else {
                Spacer()
            }
            ProjectSearchReplaceSection(client: client)
            Text(client.status)
                .font(.caption)
                .foregroundStyle(client.status.hasPrefix("Not navigated") || client.status.contains("refused") ? .orange : .secondary)
                .lineLimit(3)
                .textSelection(.enabled)
                .accessibilityLabel("Search status: \(client.status)")
        }
        .padding(DS.Space.l)
    }
}

/// Reviewed replacement: a replacement field, "Plan Replacement" (read-only
/// proposal from the helper, shown per file with before → after), and
/// "Apply" — the explicit approval — with per-file outcomes afterwards.
private struct ProjectSearchReplaceSection: View {
    @Bindable var client: ProjectSearchClient

    private var canPlan: Bool {
        guard let r = client.results else { return false }
        return client.helperAvailable && r.literal == client.query && r.termination.isExhaustive && !r.matches.isEmpty && !client.isApplying
    }

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.s) {
            HStack {
                TextField("Replace with (exact bytes)", text: $client.replacement)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Replacement text")
                    .accessibilityHint("Plan Replacement asks the helper for a read-only proposal; nothing changes until Apply")
                Button("Plan Replacement") { Task { await client.planReplacement() } }
                    .disabled(!canPlan)
                    .help("Ask the helper for a read-only proposal for every complete match; partial searches cannot be planned")
                if let plan = client.plan, !plan.edits.isEmpty {
                    Button("Apply \(plan.summary)") { Task { await client.applyPlan() } }
                        .disabled(client.isApplying || !client.helperAvailable)
                        .help("Approve the proposal shown below: one guarded durable edit group per file, never all-or-nothing")
                }
            }
            if !client.planPreviews.isEmpty {
                List {
                    ForEach(Array(client.planPreviews.enumerated()), id: \.element.id) { (index: Int, preview: ProjectSearch.ReplacementPreview) in
                        HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                            Text(preview.line > 0 ? "\(preview.edit.path):\(preview.line)" : preview.edit.path)
                                .font(.caption.monospaced()).foregroundStyle(.secondary)
                                .frame(width: DS.Layout.searchPathColumnWidth, alignment: .leading).lineLimit(1)
                            if let b = preview.before, let a = preview.after {
                                (Text(b.before) + Text(b.match).strikethrough().foregroundColor(.red) + Text(" → ") + Text(a.match).bold().foregroundColor(.accentColor) + Text(a.after))
                                    .font(.body.monospaced()).lineLimit(1)
                            } else {
                                Text("bytes \(preview.edit.start)..<\(preview.edit.end): “\(preview.edit.expectedText)” → “\(preview.edit.replacement)”")
                                    .font(.caption).lineLimit(1)
                            }
                        }
                        .accessibilityElement(children: .ignore)
                        .accessibilityLabel(ProjectSearch.accessibilityLabel(index: index, count: client.planPreviews.count, preview: preview))
                    }
                }
                .frame(minHeight: DS.Layout.searchPreviewMinHeight, maxHeight: DS.Layout.searchPreviewMaxHeight)
                .accessibilityLabel("Replacement proposal, \(client.plan?.summary ?? "")")
            }
            ForEach(client.applyOutcomes) { outcome in
                HStack(spacing: DS.Space.s) {
                    Text(outcome.description).font(.caption)
                        .foregroundStyle({ () -> Color in if case .applied = outcome.state { return .secondary } else { return .orange } }())
                    if case .uncertain(_, let id) = outcome.state {
                        Button("Retry \(outcome.path)") { Task { await client.retryUncertain(commandID: id) } }
                            .controlSize(.small)
                    }
                }
                .accessibilityElement(children: .combine)
            }
            if !client.planStatus.isEmpty {
                Text(client.planStatus).font(.caption)
                    .foregroundStyle(client.planStatus.contains("refused") || client.planStatus.contains("nothing applied") ? .orange : .secondary)
                    .lineLimit(3).textSelection(.enabled)
                    .accessibilityLabel("Replacement status: \(client.planStatus)")
            }
        }
    }
}

private struct ProjectSearchRow: View {
    var index: Int
    var count: Int
    var match: ProjectSearch.Match

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
            Text(match.line > 0 ? "\(match.path):\(match.line)" : match.path)
                .font(.caption.monospaced()).foregroundStyle(.secondary)
                .frame(width: DS.Layout.searchPathColumnWidth, alignment: .leading).lineLimit(1)
            if let s = match.snippet {
                (Text(s.clippedBefore ? "…" : "") + Text(s.before) + Text(s.match).bold().foregroundColor(.accentColor) + Text(s.after) + Text(s.clippedAfter ? "…" : ""))
                    .font(.body.monospaced()).lineLimit(1)
            } else {
                Text("bytes \(match.location.start)..<\(match.location.end) (text unavailable)").font(.caption).foregroundStyle(.secondary)
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(ProjectSearch.accessibilityLabel(index: index, count: count, match: match))
    }
}

// MARK: - scene and menu (added from FlashTeXMacApp with one line each)

/// `ProjectSearchWindow(model: model)` in the app's scene body.
struct ProjectSearchWindow: Scene {
    var model: ShellModel

    var body: some Scene {
        Window("Find in Project", id: ProjectSearch.windowID) {
            ProjectSearchPanel().environment(model)
        }
        .defaultSize(width: 760, height: 600)
    }
}

/// `ProjectSearchCommands(openWindow: openWindow)` in the app's `.commands`:
/// ⌘⇧F opens (or brings forward) the search window.
struct ProjectSearchCommands: Commands {
    var openWindow: OpenWindowAction

    var body: some Commands {
        CommandGroup(after: .textEditing) {
            Button("Find in Project…") { openWindow(id: ProjectSearch.windowID) }
                .keyboardShortcut("f", modifiers: [.command, .shift])
        }
    }
}
