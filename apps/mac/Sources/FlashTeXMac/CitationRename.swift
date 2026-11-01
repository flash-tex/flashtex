import AppKit
import SwiftUI
import FlashTeXProtocol

/// Reviewed citation-key rename through the durable helper's read-only
/// `plan_citation_rename` / `plan_citation_rename_at` proposals
/// (crates/preview-controller/docs/source-plans.md, schema
/// `flashtex.citation-rename-plan.v1`, kind `citation_key_rename`).
///
/// The helper plans a *lexical* rename over its indexed project: every
/// `\cite`-family reference and the one declared bibliography record for the
/// key (the `.bib` must be declared as a bibliography source through Document
/// Kinds — `model.documentKinds` — the kind is never inferred from a file
/// name here or in the helper). Bibitems alone, duplicate definitions, a
/// malformed record, a collision with an existing key and an invalid key are
/// refused by the helper, and the refusal is shown verbatim with a short
/// explanation.
///
/// The flow is proposal → review → explicit Apply. `Rename Citation…` (Edit
/// menu) opens the panel; if the caret of the active document is on a
/// citation key the helper's lexical `navigate` names the exact key span and
/// the `_at` variant is used, otherwise the user types the old key. Applying
/// reuses the search lane's reviewed-application core
/// (`ProjectSearchClient.applyReviewedEdits`): fresh `snapshot` equal to the
/// plan's version map and membership generation or the whole plan is refused,
/// then one guarded `apply_group` per file with a fresh retained command id
/// (`citation-rename-<UUID>`), byte-verified ranges and per-file outcomes;
/// an uncertain reply is retried with the identical id and payload; the
/// shell's durable state is reconciled with the helper's returned documents
/// and a buffer takes the returned text only while it is still exactly the
/// editor snapshot the command was sent against — text typed during the
/// round trip is kept and resubmitted on top (GH39). Nothing here claims an
/// all-or-nothing project edit.
enum CitationRename {
    static let windowID = "citation-rename"
    static let planSchema = "flashtex.citation-rename-plan.v1"
    static let planKind = "citation_key_rename"
    static let commandPrefix = "citation-rename"
    /// The helper's key rule (project-index `plan_citation_rename`): 1…4096
    /// bytes, no whitespace/control characters, none of `{}()\"%=#@,`.
    static let forbiddenKeyCharacters: Set<Character> = ["{", "}", "(", ")", "\\", "\"", "%", "=", "#", "@", ","]
    static let maxKeyBytes = 4096
    static let noHelperMessage = "Renaming a citation requires the durable helper (flashtex-preview-controller) to be attached and ready."

    /// The helper's proposal, bound to the exact snapshot it was computed on.
    struct Plan: Equatable {
        var oldName: String
        var newName: String
        var projectID: String
        var sourceVersions: [String: Int]
        var membershipGeneration: Int
        /// Helper order: path, then byte offset; nonoverlapping within a file.
        var edits: [ProjectSearch.ReplacementEdit]

        var paths: [String] { Array(Set(edits.map(\.path))).sorted() }
        func edits(in path: String) -> [ProjectSearch.ReplacementEdit] { edits.filter { $0.path == path } }
        var summary: String {
            let n = edits.count, files = paths.count
            return "\(n) occurrence\(n == 1 ? "" : "s") in \(files) file\(files == 1 ? "" : "s")"
        }
        var label: String { "Rename citation “\(oldName)” to “\(newName)”" }
    }

    /// The exact lexical key span under the caret, as the helper named it.
    struct KeySpan: Equatable {
        var name: String
        var location: ShellModel.IndexLocation
    }

    /// Why a typed key cannot be sent (mirrors the helper's rule so a refusal
    /// is explained before a round trip; the helper's check stays authoritative).
    static func keyProblem(_ key: String) -> String? {
        if key.isEmpty { return "the new key is empty" }
        if key.utf8.count > maxKeyBytes { return "the new key is \(key.utf8.count) bytes; the helper accepts at most \(maxKeyBytes)" }
        if let bad = key.first(where: { $0.isWhitespace || forbiddenKeyCharacters.contains($0) || $0.unicodeScalars.contains { $0.properties.generalCategory == .control } }) {
            return "the key contains “\(bad.isWhitespace ? "whitespace" : String(bad))”, which a citation key cannot contain"
        }
        return nil
    }

    /// A short explanation for the helper's refusal codes (the helper's own
    /// message is always shown alongside; nothing is inferred from it).
    static func explain(helperError message: String) -> String {
        let hints: [(String, String)] = [
            ("MissingBibliographyDefinition", "the key has no well-formed record in a declared bibliography source (a \\bibitem alone is not enough; declare the .bib through Document Kinds)"),
            ("AmbiguousCitationDefinition", "the key is defined more than once"),
            ("MalformedBibliographyDefinition", "the key's bibliography record is malformed"),
            ("RenameCollision", "the new key already exists in the project"),
            ("InvalidCitationKey", "the new key is not a valid citation key"),
            ("InvalidCitationRenamePlan", "the span is not a citation key the index knows, or the plan could not be built"),
            ("ReplacementPlanTooLarge", "the rename touches more text than the helper's plan limit"),
            ("unknown operation", "this helper build has no plan_citation_rename (needs crates/preview-controller with source plans)"),
            ("source snapshot changed", "the project changed since the snapshot; plan again"),
            ("unknown source path", "the caret's document is not part of the helper's project"),
        ]
        for (code, hint) in hints where message.contains(code) { return "\(hint) (helper: \(message))" }
        return "helper: \(message)"
    }

    /// Payload of `plan_citation_rename` (typed old key) or
    /// `plan_citation_rename_at` (exact key span).
    static func request(sourceVersions: [String: Int], membershipGeneration: Int, newName: String,
                        oldName: String?, span: ShellModel.IndexLocation?) -> (operation: String, payload: PreviewControllerClient.JSONObject) {
        var payload: PreviewControllerClient.JSONObject = [
            "source_versions": sourceVersions,
            "membership_generation": membershipGeneration,
            "max_bytes": ProjectSearch.planMaxBytes,
            "new_name": newName,
        ]
        if let span {
            payload["path"] = span.path
            payload["start_byte"] = span.start
            payload["end_byte"] = span.end
            return ("plan_citation_rename_at", payload)
        }
        payload["old_name"] = oldName ?? ""
        return ("plan_citation_rename", payload)
    }

    /// Parses and validates a citation-rename reply. `oldName` is required
    /// when the user typed it; for the `_at` variant the plan's own
    /// `rename.old_name` is the key (the helper resolved the span). Anything
    /// unexpected refuses the whole plan; nothing is inferred.
    static func parsePlan(reply payload: [String: Any], oldName expectedOld: String?, newName: String) -> Result<Plan, ControllerError> {
        func refuse(_ why: String) -> Result<Plan, ControllerError> { .failure(.init(message: "citation rename plan refused: " + why)) }
        guard let versions = payload["source_versions"] as? [String: Int] else { return refuse("reply has no source_versions") }
        guard let generation = ProjectSearch.exactInt(payload["membership_generation"]) else { return refuse("reply has no membership_generation") }
        guard let plan = payload["plan"] as? [String: Any] else { return refuse("reply has no plan") }
        guard plan["schema"] as? String == planSchema else { return refuse("schema is \(plan["schema"] ?? "missing"), not \(planSchema)") }
        guard plan["kind"] as? String == planKind else { return refuse("kind is \(plan["kind"] ?? "missing"), not \(planKind)") }
        guard plan["proposal_only"] as? Bool == true, plan["requires_user_approval"] as? Bool == true else {
            return refuse("plan is not marked proposal_only + requires_user_approval")
        }
        guard plan["application_order"] as? String == "reverse_byte_offset_per_document" else {
            return refuse("unknown application_order \(plan["application_order"] ?? "missing")")
        }
        guard let snapshot = plan["snapshot"] as? [String: Any], let projectID = snapshot["project_id"] as? String,
              let snapGeneration = ProjectSearch.exactInt(snapshot["generation"]), let snapDocs = snapshot["documents"] as? [[String: Any]] else {
            return refuse("plan snapshot is malformed")
        }
        guard snapGeneration == generation else { return refuse("plan generation \(snapGeneration) is not the reply's \(generation)") }
        var snapVersions: [String: Int] = [:]
        for d in snapDocs {
            guard let file = d["file"] as? String, let rev = ProjectSearch.exactInt(d["revision"]) else { return refuse("plan snapshot document is malformed") }
            snapVersions[file] = rev
        }
        guard snapVersions == versions else { return refuse("plan snapshot versions \(snapVersions) differ from the reply's \(versions)") }
        guard let rename = plan["rename"] as? [String: Any], let oldName = rename["old_name"] as? String, let planNew = rename["new_name"] as? String else {
            return refuse("plan has no rename.old_name/new_name")
        }
        if let expectedOld, expectedOld != oldName { return refuse("plan renames “\(oldName)”, not “\(expectedOld)”") }
        guard planNew == newName, plan["replacement"] as? String == newName else { return refuse("plan new name is not “\(newName)”") }
        guard oldName != newName else { return refuse("old and new key are the same") }
        guard let rawEdits = plan["edits"] as? [[String: Any]] else { return refuse("plan has no edits") }
        var edits: [ProjectSearch.ReplacementEdit] = []
        var lastByPath: [String: Int] = [:]
        for (i, e) in rawEdits.enumerated() {
            guard let file = e["file"] as? String, let rev = ProjectSearch.exactInt(e["revision"]), let start = ProjectSearch.exactInt(e["start_byte"]),
                  let end = ProjectSearch.exactInt(e["end_byte"]), let expected = e["expected_text"] as? String, let repl = e["replacement"] as? String else {
                return refuse("edit \(i) is malformed")
            }
            guard versions[file] == rev else { return refuse("edit \(i) names \(file) r\(rev), not the snapshot's r\(versions[file].map(String.init) ?? "?")") }
            guard start <= end, end - start == oldName.utf8.count, expected == oldName, repl == newName else {
                return refuse("edit \(i) (\(file) \(start)..<\(end)) does not rename exactly “\(oldName)” to “\(newName)”")
            }
            if let last = lastByPath[file], start < last { return refuse("edit \(i) in \(file) overlaps or precedes the previous edit") }
            lastByPath[file] = end
            edits.append(.init(path: file, revision: rev, start: start, end: end, expectedText: expected, replacement: repl))
        }
        return .success(.init(oldName: oldName, newName: newName, projectID: projectID, sourceVersions: versions,
                              membershipGeneration: generation, edits: edits))
    }

    /// Before → after lines per edit, from the durable text at the plan's revisions.
    static func previews(for plan: Plan, texts: [String: String]) -> [ProjectSearch.ReplacementPreview] {
        plan.edits.map { e in
            guard let text = texts[e.path], let at = ProjectSearch.locate(start: e.start, end: e.end, in: text) else {
                return ProjectSearch.ReplacementPreview(edit: e, line: 0, before: nil, after: nil)
            }
            var after = at.snippet
            after.match = e.replacement
            return ProjectSearch.ReplacementPreview(edit: e, line: at.line, before: at.snippet, after: after)
        }
    }

    /// VoiceOver label of one preview row, naming the document's declared kind.
    static func accessibilityLabel(index: Int, count: Int, preview: ProjectSearch.ReplacementPreview, kind: DocumentKind?) -> String {
        let where_ = preview.line > 0 ? "line \(preview.line)" : "bytes \(preview.edit.start) to \(preview.edit.end)"
        let change = preview.before.map { "\($0.text) becomes \(preview.after?.text ?? "")" } ?? "“\(preview.edit.expectedText)” becomes “\(preview.edit.replacement)”"
        let kindText = kind.map { " (\($0.rawValue))" } ?? ""
        return "occurrence \(index + 1) of \(count), \(preview.edit.path)\(kindText), \(where_), \(change)"
    }
}

// MARK: - client

/// One rename panel's state: key location (caret or typed), proposal,
/// review, explicit application and retry. Application and retry go through
/// the search lane's `ProjectSearchClient` core so the two reviewed flows
/// share one implementation of the snapshot guard, the per-file guarded
/// `apply_group`, the retained-payload retry and the reconciliation.
@MainActor
@Observable
final class CitationRenameClient {
    private(set) var model: ShellModel
    /// Reviewed-application engine (snapshot, durable text, apply, retry).
    private(set) var engine: ProjectSearchClient
    /// The key to rename (typed, or filled in from the caret).
    var oldName = ""
    var newName = ""
    /// The exact key span the helper named for the caret; nil when the old
    /// key was typed. Planning with a span uses the `_at` variant.
    private(set) var keySpan: CitationRename.KeySpan?
    private(set) var plan: CitationRename.Plan?
    private(set) var previews: [ProjectSearch.ReplacementPreview] = []
    private(set) var status = ""
    private(set) var outcomes: [ProjectSearch.FileOutcome] = []
    private(set) var isPlanning = false
    private(set) var isApplying = false
    /// Bumped on every accepted application (tests observe it).
    private(set) var applyCount = 0

    init(model: ShellModel) {
        self.model = model
        engine = ProjectSearchClient(model: model)
    }

    var helperAvailable: Bool { model.controllerAttached && model.controllerState.ready }
    /// The helper-reported kind of a path (never inferred).
    func kind(of path: String) -> DocumentKind? { model.documentKinds.kind(of: path) }
    /// Command ids retained for uncertain replies (identical retry).
    var retainedCommands: [String: PreviewControllerClient.JSONObject] { engine.retainedCommands }

    /// Forgets the caret span: the next plan uses the typed old key.
    func useTypedKey() { keySpan = nil }

    /// Locates the citation key under the active document's caret through
    /// the helper's lexical `navigate` (the exact symbol span, never a client
    /// guess) and checks with `complete` (category `citation`) that the span
    /// is one of that name's citation occurrences. Refused — with the reason
    /// — when the caret is on no indexed symbol, on a non-citation symbol,
    /// when the buffer is not durable yet, or when the helper is unavailable.
    /// On success `oldName` and `keySpan` are set; nothing is planned yet.
    @discardableResult
    func locateKeyAtCaret() async -> Bool {
        keySpan = nil
        guard helperAvailable else { status = CitationRename.noHelperMessage; return false }
        let path = model.activePath, text = model.activeText
        guard let caretByte = CaretSync.byteOffset(ofCaretUTF16: model.caretUTF16, in: text) else {
            status = "Caret position \(model.caretUTF16) is not valid in \(path)."; return false
        }
        guard let probe = Navigation.helperProbeOffset(in: text, caretByte: caretByte) else {
            status = "The caret is on \\begin/\\end, not on a citation key. Type the old key instead."; return false
        }
        guard let versionsReply = await model.controllerSourceVersions() else { status = CitationRename.noHelperMessage; return false }
        let versions: [String: Int]
        switch versionsReply {
        case .failure(let e): status = "Snapshot refused: \(e.message)"; return false
        case .success(let v): versions = v
        }
        guard let revision = versions[path] else {
            status = "\(path) is not part of the helper's project (indexed: \(versions.keys.sorted().joined(separator: ", ")))."; return false
        }
        guard let durable = await engine.durableText(path: path, revision: revision), durable.sameBytes(as: text) else {
            status = "\(path) has edits the helper has not indexed yet (durable r\(revision)); wait, then try again."; return false
        }
        guard let reply = await model.controllerNavigate(sourceVersions: versions, path: path, byteOffset: probe) else {
            status = CitationRename.noHelperMessage; return false
        }
        switch reply {
        case .staleVersions(let why): status = "Project changed while locating the key (\(why)); try again."; return false
        case .refused(let why): status = "Project index refused the lookup: \(why)"; return false
        case .nothing:
            status = "Caret byte \(caretByte) of \(path) is on no citation key the project index knows. Type the old key instead."; return false
        case .found(let name, let origin, _, _):
            guard origin.path == path else { status = "The index placed the symbol in \(origin.path), not \(path); not used."; return false }
            guard let occ = await model.controllerOccurrences(sourceVersions: versions, category: "citation", name: name) else {
                status = CitationRename.noHelperMessage; return false
            }
            switch occ {
            case .failure(let e): status = "Project index refused the citation lookup: \(e.message)"; return false
            case .success(let all):
                guard all.contains(origin) else {
                    status = "The caret is on “\(name)” (bytes \(origin.start)..<\(origin.end) of \(path)), which the index lists as no citation key. Type the old key instead."
                    return false
                }
            }
            guard ProjectSearch.bytesSpell(name, in: durable, start: origin.start, end: origin.end) else {
                status = "Bytes \(origin.start)..<\(origin.end) of \(path) r\(revision) do not spell “\(name)”; not used."; return false
            }
            oldName = name
            keySpan = .init(name: name, location: origin)
            status = "Citation key “\(name)” at \(path) bytes \(origin.start)..<\(origin.end) (durable r\(revision)). Type the new key, then Plan Rename."
            return true
        }
    }

    /// Asks the helper for the read-only proposal (`_at` when a caret span is
    /// held, else by typed old key) and shows it per file. Nothing is edited.
    func planRename() async {
        guard !isPlanning, !isApplying else { return }
        plan = nil; previews = []; outcomes = []
        guard helperAvailable else { status = CitationRename.noHelperMessage; return }
        let newName = self.newName
        if let why = CitationRename.keyProblem(newName) { status = "Cannot plan: \(why)."; return }
        let typedOld: String? = keySpan == nil ? oldName : nil
        if let typedOld, typedOld.isEmpty { status = "Type the citation key to rename, or place the caret on one and choose Rename Citation… again."; return }
        if (keySpan?.name ?? oldName) == newName { status = "The new key equals the old key; nothing to change."; return }
        isPlanning = true
        defer { isPlanning = false }
        // Explicit declarations only: the helper reports kinds, the shell never infers them.
        _ = await model.documentKinds.refresh()
        let bib = model.documentKinds.bibliographyPaths
        guard !bib.isEmpty else {
            status = "No bibliography source is declared for this project (\(model.documentKinds.status)). Declare the .bib through Document Kinds first; the helper renames only keys with a declared record."
            return
        }
        status = "Asking the helper for a rename proposal…"
        guard let snapReply = await engine.controllerSnapshot() else { status = CitationRename.noHelperMessage; return }
        let snap: ProjectSearchClient.Snapshot
        switch snapReply {
        case .failure(let e): status = "Snapshot refused: \(e.message)"; return
        case .success(let s): snap = s
        }
        if let span = keySpan, snap.versions[span.location.path] != span.location.revision {
            status = "\(span.location.path) changed since the key was located (durable r\(span.location.revision)→r\(snap.versions[span.location.path].map(String.init) ?? "–")); place the caret on the key again."
            keySpan = nil
            return
        }
        let (operation, payload) = CitationRename.request(sourceVersions: snap.versions, membershipGeneration: snap.generation, newName: newName,
                                                          oldName: typedOld, span: keySpan?.location)
        guard let reply = await model.controllerRequest(operation, payload) else { status = CitationRename.noHelperMessage; return }
        switch reply {
        case .failure(let e):
            status = "Rename refused: " + CitationRename.explain(helperError: e.message)
        case .success(let dict):
            switch CitationRename.parsePlan(reply: dict, oldName: typedOld ?? keySpan?.name, newName: newName) {
            case .failure(let e): status = e.message
            case .success(let p):
                var texts: [String: String] = [:]
                for path in p.paths { if let t = await engine.durableText(path: path, revision: p.sourceVersions[path] ?? -1) { texts[path] = t } }
                plan = p
                previews = CitationRename.previews(for: p, texts: texts)
                let kinds = p.paths.map { "\($0) r\(p.sourceVersions[$0]!)\(kind(of: $0).map { " (\($0.rawValue))" } ?? "")" }.joined(separator: ", ")
                status = p.edits.isEmpty
                    ? "The helper proposes no edit for “\(p.oldName)”."
                    : "Proposal: \(p.summary), renaming “\(p.oldName)” to “\(p.newName)” at durable \(kinds). Nothing is changed until you click Apply."
            }
        }
    }

    /// The user's Apply after the plan was shown: the shared reviewed
    /// application (snapshot guard, one guarded `apply_group` per file,
    /// reconciliation), per-file outcomes, then the preview catches up.
    func applyRename() async {
        guard let plan, !plan.edits.isEmpty else { status = "No proposal to apply."; return }
        guard !isApplying else { return }
        isApplying = true
        defer { isApplying = false }
        outcomes = []
        switch await engine.applyReviewedEdits(plan.edits, sourceVersions: plan.sourceVersions, membershipGeneration: plan.membershipGeneration,
                                               label: plan.label, commandPrefix: CitationRename.commandPrefix) {
        case .failure(let why):
            status = why.message
            if why.message.hasPrefix("Project changed") { self.plan = nil; previews = [] }
            return
        case .success(let done):
            outcomes = done
        }
        let applied = outcomes.filter { if case .applied = $0.state { return true } else { return false } }.count
        let refused = outcomes.count - applied
        status = "\(plan.label): \(applied) of \(outcomes.count) file\(outcomes.count == 1 ? "" : "s") applied"
            + (refused > 0 ? ", \(refused) not applied (see below; files are applied one by one, never all-or-nothing)" : "")
            + ". Undo is the ledger's grouped undo per file."
        self.plan = nil; previews = []; keySpan = nil
        if applied > 0 {
            applyCount += 1
            await engine.settleAfterApply()
        }
    }

    /// Retries an uncertain file with its retained command id and payload
    /// (never a new id): the ledger replays or refuses it exactly.
    func retryUncertain(commandID: String) async {
        guard engine.retainedCommands[commandID] != nil else { status = "No retained command \(commandID)."; return }
        guard helperAvailable else { status = CitationRename.noHelperMessage; return }
        guard let outcome = await engine.retryRetained(commandID: commandID) else { return }
        if let i = outcomes.firstIndex(where: { $0.path == outcome.path }) { outcomes[i] = outcome } else { outcomes.append(outcome) }
        status = outcome.description
        if case .applied = outcome.state { applyCount += 1; await engine.settleAfterApply() }
    }
}

// MARK: - view

/// The Rename Citation window: old key (from the caret or typed), new key,
/// "Plan Rename" (read-only proposal per file with before → after and the
/// document's declared kind) and "Apply" — the explicit approval — with
/// per-file outcomes and identical retry for uncertain ones. Esc closes.
struct CitationRenamePanel: View {
    static let windowID = CitationRename.windowID
    @Environment(ShellModel.self) private var model
    @Environment(\.dismissWindow) private var dismissWindow
    @State private var client: CitationRenameClient?
    @FocusState private var newKeyFocused: Bool

    var body: some View {
        Group {
            if let client {
                CitationRenameBody(client: client, newKeyFocused: $newKeyFocused)
            } else {
                ProgressView().onAppear {
                    let c = CitationRenameClient(model: model)
                    client = c
                    Task {
                        // The caret's key, when the helper is ready (bounded wait); otherwise the user types it.
                        let deadline = Date().addingTimeInterval(3)
                        while !c.helperAvailable, Date() < deadline { try? await Task.sleep(nanoseconds: 50_000_000) }
                        await c.locateKeyAtCaret()
                        if let seed = ProcessInfo.processInfo.environment["FLASHTEX_CITATION_RENAME_NEW"], !seed.isEmpty {
                            if let old = ProcessInfo.processInfo.environment["FLASHTEX_CITATION_RENAME_OLD"], !old.isEmpty { c.useTypedKey(); c.oldName = old }
                            c.newName = seed
                            await c.planRename() // proposal only; Apply stays a click
                        }
                    }
                }
            }
        }
        .frame(minWidth: DS.Layout.citationWindowMinWidth, minHeight: DS.Layout.citationWindowMinHeight)
        .onAppear { newKeyFocused = true }
        .background {
            Button("Close") { dismissWindow(id: Self.windowID) }
                .keyboardShortcut(.cancelAction)
                .hidden()
        }
    }
}

private struct CitationRenameBody: View {
    @Bindable var client: CitationRenameClient
    var newKeyFocused: FocusState<Bool>.Binding

    private var canPlan: Bool { client.helperAvailable && !client.isPlanning && !client.isApplying && !client.newName.isEmpty && (client.keySpan != nil || !client.oldName.isEmpty) }

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.m) {
            HStack {
                TextField("Citation key to rename", text: $client.oldName)
                    .textFieldStyle(.roundedBorder)
                    .disabled(client.keySpan != nil)
                    .accessibilityLabel("Citation key to rename")
                    .accessibilityHint(client.keySpan != nil ? "Taken from the caret; choose Use Typed Key to type another" : "The exact key as written in the source")
                if let span = client.keySpan {
                    Text("from caret: \(span.location.path) bytes \(span.location.start)..<\(span.location.end)").font(.caption).foregroundStyle(.secondary)
                    Button("Use Typed Key") { client.useTypedKey() }.controlSize(.small)
                } else {
                    Button("Use Caret") { Task { await client.locateKeyAtCaret() } }.controlSize(.small)
                        .disabled(!client.helperAvailable)
                        .help("Locate the citation key under the active document's caret through the project index")
                }
            }
            HStack {
                TextField("New key (no spaces, braces, commas, quotes…)", text: $client.newName)
                    .textFieldStyle(.roundedBorder)
                    .focused(newKeyFocused)
                    .onSubmit { Task { await client.planRename() } }
                    .accessibilityLabel("New citation key")
                    .accessibilityHint("Plan Rename asks the helper for a read-only proposal; nothing changes until Apply")
                Button("Plan Rename") { Task { await client.planRename() } }
                    .disabled(!canPlan)
                    .help("Ask the helper for a read-only proposal covering every reference and the declared bibliography record")
                if let plan = client.plan, !plan.edits.isEmpty {
                    Button("Apply \(plan.summary)") { Task { await client.applyRename() } }
                        .disabled(client.isApplying || !client.helperAvailable)
                        .help("Approve the proposal shown below: one guarded durable edit group per file, never all-or-nothing")
                }
            }
            if !client.helperAvailable {
                Label(CitationRename.noHelperMessage, systemImage: "exclamationmark.triangle")
                    .foregroundStyle(DS.Colors.severityWarning)
                    .accessibilityLabel(CitationRename.noHelperMessage)
            }
            if !client.previews.isEmpty {
                List {
                    ForEach(Array(client.previews.enumerated()), id: \.element.id) { (index: Int, preview: ProjectSearch.ReplacementPreview) in
                        HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                            Text(preview.line > 0 ? "\(preview.edit.path):\(preview.line)" : preview.edit.path)
                                .font(.caption.monospaced()).foregroundStyle(.secondary)
                                .frame(width: DS.Layout.searchPathColumnWidth, alignment: .leading).lineLimit(1)
                            Text(client.kind(of: preview.edit.path)?.rawValue ?? "kind?")
                                .font(DS.Fonts.secondary).padding(.horizontal, DS.Space.xs).padding(.vertical, DS.Size.hairline)
                                .background(DS.Colors.textSecondary.opacity(DS.State.hairlineOpacity), in: Capsule())
                            if let b = preview.before, let a = preview.after {
                                (Text(b.before) + Text(b.match).strikethrough().foregroundColor(.red) + Text(" → ") + Text(a.match).bold().foregroundColor(.accentColor) + Text(a.after))
                                    .font(.body.monospaced()).lineLimit(1)
                            } else {
                                Text("bytes \(preview.edit.start)..<\(preview.edit.end): “\(preview.edit.expectedText)” → “\(preview.edit.replacement)”")
                                    .font(.caption).lineLimit(1)
                            }
                        }
                        .accessibilityElement(children: .ignore)
                        .accessibilityLabel(CitationRename.accessibilityLabel(index: index, count: client.previews.count, preview: preview, kind: client.kind(of: preview.edit.path)))
                    }
                }
                .frame(minHeight: DS.Layout.diagnosticsListMinHeight)
                .accessibilityLabel("Rename proposal, \(client.plan?.summary ?? "")")
            } else {
                Spacer()
            }
            ForEach(client.outcomes) { outcome in
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
            Text(client.status)
                .font(.caption)
                .foregroundStyle(client.status.contains("refused") || client.status.contains("nothing applied") || client.status.hasPrefix("Cannot") ? .orange : .secondary)
                .lineLimit(4)
                .textSelection(.enabled)
                .accessibilityLabel("Rename status: \(client.status)")
        }
        .padding(DS.Space.l)
    }
}

// MARK: - scene and menu (added from FlashTeXMacApp with one line each)

/// `CitationRenameWindow(model: model)` in the app's scene body.
struct CitationRenameWindow: Scene {
    var model: ShellModel

    var body: some Scene {
        Window("Rename Citation", id: CitationRename.windowID) {
            CitationRenamePanel().environment(model)
        }
        .defaultSize(width: 760, height: 480)
    }
}

/// `CitationRenameCommands(openWindow: openWindow)` in the app's `.commands`:
/// Edit > Rename Citation… (no default shortcut) opens the panel, which
/// locates the key under the caret when there is one.
struct CitationRenameCommands: Commands {
    var openWindow: OpenWindowAction

    var body: some Commands {
        CommandGroup(after: .textEditing) {
            Button("Rename Citation…") { openWindow(id: CitationRename.windowID) }
        }
    }
}
