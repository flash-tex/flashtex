import SwiftUI
import FlashTeXProtocol

/// Durable undo history on the helper's edit ledger (crates/preview-controller,
/// STDIO.md "Durable editor history"; crates/edit-ledger/src/history.rs).
///
/// The helper owns the only history: `history_status {path}` returns the undo
/// and redo label stacks plus retention usage, and `undo`/`redo` `{path,
/// command:{command_id, expected_revision, expected_sha256}}` move one step,
/// guarded by the exact current durable revision/hash. Every new action gets a
/// fresh command id; a retry after an uncertain reply reuses the SAME id, and
/// the ledger answers `replayed_command:true` instead of moving twice. Undo
/// advances the source revision (it never rewrites old revisions).
///
/// This file adds no ledger or bridge code: `EditHistoryClient` speaks through
/// the shell's existing `PreviewControllerClient` and the shell's per-request
/// waiters (`ControllerState.awaiting`), and a successful move is adopted into
/// the editor buffer through the shell's durable-document path
/// (`ShellModel.controllerAdoptHistoryResult`, ShellModel+Controller.swift).
/// The text view's own ⌘Z stays the in-memory editor undo; this panel is the
/// explicit durable history that survives relaunches.
///
/// Helper restart / reconciliation: when `ShellModel.controllerAttached` flips
/// to false the panel marks its status unavailable and any command awaiting a
/// reply as uncertain (the ledger may or may not have applied it). When it
/// flips back to true the shell re-reads the durable document and, per its
/// attach policy, resubmits a differing buffer as a new edit; the panel then
/// refreshes the stacks and still offers Retry with the unchanged command id:
/// the reply is either the replayed receipt (already applied before the
/// restart — the current document is adopted, which may already be a newer
/// revision) or `document_conflict` (the revision moved), after which the
/// panel re-reads and the user decides again. Nothing is retried automatically.
enum EditHistory {
    /// Ledger retention limits (history.rs `MAX_HISTORY_*`). Editing is refused
    /// with `history_full` once entries or bytes exceed them; the helper protocol
    /// exposes no retention operation yet, so the panel can only warn early.
    static let maxEntries = 256
    static let maxBytes = 32 * 1024 * 1024
    static let maxCommandIDs = 4096
    /// Usage fraction from which the panel shows a capacity warning.
    static let warningFraction = 0.8

    static func steps(_ n: Int) -> String { n == 1 ? "1 step" : "\(n) steps" }

    /// Ledger retention limits. The helper on main ≥ 64829a0d reports the
    /// actual constants in `history_status.limits`; older helpers omit them and
    /// the built-in values (history.rs `MAX_HISTORY_*`) apply.
    struct Limits: Equatable {
        var entries = EditHistory.maxEntries
        var bytes = EditHistory.maxBytes
        var commandIDs = EditHistory.maxCommandIDs
        /// True when the helper reported its own limits.
        var reported = false

        static func decode(_ limits: [String: Any]?) -> Limits {
            guard let limits else { return Limits() }
            var l = Limits(reported: true)
            if let n = limits["history_entries"] as? Int, n > 0 { l.entries = n }
            if let n = limits["history_bytes"] as? Int, n > 0 { l.bytes = n }
            if let n = limits["permanent_command_ids"] as? Int, n > 0 { l.commandIDs = n }
            return l
        }
    }

    /// Durable identity without text (`history_status.document` on main ≥
    /// 64829a0d): read on the same owner turn as the stacks, so a guarded move
    /// can name exactly the revision the stacks describe.
    struct Identity: Equatable {
        var path: String
        var revision: Int
        var sha256: String

        static func decode(_ doc: [String: Any]?) -> Identity? {
            guard let doc, let path = doc["path"] as? String, let revision = doc["revision"] as? Int,
                  let sha = doc["source_sha256"] as? String else { return nil }
            return Identity(path: path, revision: revision, sha256: sha)
        }
    }

    /// `history_status` reply: `history` (stacks + usage), optional `document`
    /// identity and optional `limits`.
    struct Status: Equatable {
        /// Oldest → newest; the next step to undo is the LAST element.
        var undoLabels: [String]
        /// The next step to redo is the LAST element.
        var redoLabels: [String]
        var permanentCommandIDs: Int
        var historyBytes: Int
        var limits = Limits()
        var identity: Identity?

        var entries: Int { undoLabels.count + redoLabels.count }
        var entryFraction: Double { Double(entries) / Double(limits.entries) }
        var byteFraction: Double { Double(historyBytes) / Double(limits.bytes) }
        var commandIDFraction: Double { Double(permanentCommandIDs) / Double(limits.commandIDs) }
        /// The tightest of the three limits.
        var usage: Double { max(entryFraction, byteFraction, commandIDFraction) }
        /// The next recorded edit will be refused (`history_full`/`history_ids_full`).
        /// Bytes are a lower bound: the cost of the next entry depends on the
        /// before/after source, so an edit can be refused before this reads full.
        var isFull: Bool {
            entries >= limits.entries || historyBytes >= limits.bytes || permanentCommandIDs >= limits.commandIDs
        }
        var nearCapacity: Bool { usage >= EditHistory.warningFraction }

        var retentionSummary: String {
            "\(entries) of \(limits.entries) steps · \(Self.bytes(historyBytes)) of \(Self.bytes(limits.bytes)) · \(permanentCommandIDs) of \(limits.commandIDs) command ids"
        }

        static func bytes(_ n: Int) -> String {
            if n >= 1024 * 1024 { return String(format: "%.1f MiB", Double(n) / 1024 / 1024) }
            if n >= 1024 { return String(format: "%.0f KiB", Double(n) / 1024) }
            return "\(n) B"
        }

        static func decode(_ payload: [String: Any]) -> Status? {
            guard let h = payload["history"] as? [String: Any],
                  let undo = h["undo_labels"] as? [Any], let redo = h["redo_labels"] as? [Any] else { return nil }
            return Status(undoLabels: undo.compactMap { $0 as? String }, redoLabels: redo.compactMap { $0 as? String },
                          permanentCommandIDs: h["permanent_command_ids"] as? Int ?? 0,
                          historyBytes: h["history_bytes"] as? Int ?? 0,
                          limits: Limits.decode(payload["limits"] as? [String: Any]),
                          identity: Identity.decode(payload["document"] as? [String: Any]))
        }
    }

    /// The ledger's current durable document inside a history result.
    struct Document: Equatable {
        var path: String
        var revision: Int
        var sha256: String
        var text: String

        static func decode(_ doc: [String: Any]) -> Document? {
            guard let path = doc["path"] as? String, let revision = doc["revision"] as? Int,
                  let sha = doc["source_sha256"] as? String, let text = doc["text"] as? String else { return nil }
            return Document(path: path, revision: revision, sha256: sha, text: text)
        }
    }

    /// `undo`/`redo` reply: the ledger `history` result plus separate preview status.
    struct Result: Equatable {
        var document: Document
        /// Revision the command produced (or, when replayed, produced earlier;
        /// `document` is then the CURRENT source, possibly newer).
        var commandRevision: Int
        var replayedCommand: Bool
        var canUndo: Bool
        var canRedo: Bool
        var previewError: String?
        var saveAndSubmitMs: Double?

        static func decode(_ payload: [String: Any]) -> Result? {
            guard let h = payload["history"] as? [String: Any], let doc = h["document"] as? [String: Any],
                  let document = Document.decode(doc), let rev = h["command_revision"] as? Int else { return nil }
            return Result(document: document, commandRevision: rev,
                          replayedCommand: h["replayed_command"] as? Bool ?? false,
                          canUndo: h["can_undo"] as? Bool ?? false, canRedo: h["can_redo"] as? Bool ?? false,
                          previewError: payload["preview_error"] as? String,
                          saveAndSubmitMs: payload["save_and_submit_ms"] as? Double)
        }
    }

    enum Direction: String, Equatable { case undo, redo
        var title: String { self == .undo ? "Undo" : "Redo" }
    }

    /// One history move. The `commandID` is minted once per user action and is
    /// what makes a retry idempotent; the expectation is the exact durable
    /// revision/hash the user saw when they asked.
    struct Command: Equatable {
        var direction: Direction
        var commandID: String
        var path: String
        var expectedRevision: Int
        var expectedSHA256: String

        var payload: [String: Any] {
            ["path": path, "command": ["command_id": commandID, "expected_revision": expectedRevision, "expected_sha256": expectedSHA256]]
        }

        /// 1–128 ASCII letters/digits/hyphens/underscores (ledger `identifier`).
        static func newID() -> String { "mac-history-" + UUID().uuidString.lowercased() }
    }

    /// Helper error messages are `<code>: <text>` (edit-ledger `Error`) or a
    /// free-form controller message; the code decides what the panel does.
    enum Failure: Equatable {
        /// Expectation stale: re-read the document, never retry blindly.
        case documentConflict
        /// The command id already binds a different operation (a client bug).
        case commandIDConflict
        case historyEmpty
        /// Retention limit: the ledger refuses further recorded edits.
        case historyFull
        case historyIDsFull
        case invalidID
        case unknownDocument
        /// No reply can be trusted: helper exited / not attached / send failed.
        case uncertain(String)
        case other(String)

        static func classify(_ message: String) -> Failure {
            let code = message.split(separator: ":", maxSplits: 1).first.map { $0.trimmingCharacters(in: .whitespaces) } ?? ""
            switch code {
            case "document_conflict": return .documentConflict
            case "command_id_conflict": return .commandIDConflict
            case "history_empty": return .historyEmpty
            case "history_full": return .historyFull
            case "history_ids_full": return .historyIDsFull
            case "invalid_id": return .invalidID
            default:
                if message == "unknown document" { return .unknownDocument }
                if message.hasPrefix("helper exited") || message.hasPrefix("not admitted") || message.contains("was not admitted") {
                    return .uncertain(message)
                }
                return .other(message)
            }
        }

        var isCapacity: Bool { self == .historyFull || self == .historyIDsFull }

        var description: String {
            switch self {
            case .documentConflict: return "The durable document moved since you looked; the current revision was re-read. Check the stacks and choose again."
            case .commandIDConflict: return "This command id already names another operation; a fresh action is required."
            case .historyEmpty: return "Nothing to move in that direction."
            case .historyFull: return "History is full (\(EditHistory.maxEntries) steps or \(Status.bytes(EditHistory.maxBytes))): the ledger refuses new edits until retention is applied. This helper offers no retention operation yet."
            case .historyIDsFull: return "The permanent command-id limit (\(EditHistory.maxCommandIDs)) is reached; no further undo/redo can be recorded."
            case .invalidID: return "The command id was refused by the ledger."
            case .unknownDocument: return "The helper does not know this document."
            case .uncertain(let why): return "No reply (\(why)). The step may or may not have been applied; Retry sends the same command id, which the ledger answers exactly once."
            case .other(let m): return m
            }
        }
    }

    /// One row of a stack: consecutive typing steps collapse into a run so the
    /// list reads as actions, while `steps` says how many undo presses it is.
    struct Row: Identifiable, Equatable {
        enum Kind: Equatable { case typing, capture, reload, group }
        var id: String
        var kind: Kind
        var title: String
        var detail: String
        var steps: Int
        /// Position of the row's newest step from the top of its stack (0 = next).
        var distance: Int

        var symbol: String {
            switch kind {
            case .typing: return "keyboard"
            case .capture: return "camera.viewfinder"
            case .reload: return "arrow.clockwise.circle"
            case .group: return "square.stack.3d.up"
            }
        }

        func accessibilityLabel(direction: Direction) -> String {
            let position = distance == 0 ? "next \(direction.rawValue)" : "\(distance) steps below the next \(direction.rawValue)"
            let count = EditHistory.steps(steps)
            return "\(title), \(count), \(position). \(detail)"
        }
    }

    /// What the ledger labels mean: `edit` (typing and explicit disk reloads)
    /// records "Source edit"; `apply_reviewed` records "Capture <id>"; groups
    /// carry their own label. Session annotations (see
    /// `EditHistoryClient.noteNextLabel`) refine typing vs. reload where the
    /// panel witnessed the action; the ledger itself does not distinguish them.
    static func rows(labels: [String], annotations: [String?], stack: Direction) -> [Row] {
        var rows: [Row] = []
        var i = labels.count - 1
        while i >= 0 {
            let label = labels[i]
            let annotation = i < annotations.count ? annotations[i] : nil
            let distance = labels.count - 1 - i
            if label == "Source edit", annotation == nil {
                var j = i
                while j - 1 >= 0, labels[j - 1] == "Source edit", (j - 1 < annotations.count ? annotations[j - 1] : nil) == nil { j -= 1 }
                let steps = i - j + 1
                rows.append(Row(id: "\(stack.rawValue)-\(i)", kind: .typing, title: steps == 1 ? "Typing" : "Typing run",
                                detail: steps == 1 ? "one durable source edit" : "\(steps) durable source edits, one step each",
                                steps: steps, distance: distance))
                i = j - 1
                continue
            }
            let row: Row
            if let annotation {
                let kind: Row.Kind = annotation.lowercased().contains("reload") ? .reload : .group
                row = Row(id: "\(stack.rawValue)-\(i)", kind: kind, title: annotation, detail: "recorded by the ledger as “\(label)”", steps: 1, distance: distance)
            } else if label.hasPrefix("Capture ") {
                row = Row(id: "\(stack.rawValue)-\(i)", kind: .capture, title: "Capture insertion", detail: label, steps: 1, distance: distance)
            } else {
                row = Row(id: "\(stack.rawValue)-\(i)", kind: .group, title: label, detail: "grouped edit", steps: 1, distance: distance)
            }
            rows.append(row)
            i -= 1
        }
        return rows
    }
}

/// Bounded client: at most one `history_status` and one move in flight, each
/// with a reply deadline; a move that gets no trustworthy reply becomes
/// `uncertain` and is only ever resent by the user with the same command id.
@MainActor
@Observable
final class EditHistoryClient {
    enum Phase: Equatable {
        case idle
        case sending(EditHistory.Direction)
        case uncertain(EditHistory.Direction)
        /// No helper (or not ready): the durable history cannot be shown.
        case unavailable(String)
    }

    struct Pending: Equatable {
        var command: EditHistory.Command
        /// Request ids of every attempt (the first and each Retry); a reply to
        /// any of them settles the command.
        var requestIDs: [String]
        var sentAt: Date
        /// Editor revision when the command was issued; adoption only replaces
        /// the buffer when it has not moved since (otherwise the shell's normal
        /// resubmission reconciles the buffer on top of the durable result).
        var editorRevision: Int
        var attempts: Int { requestIDs.count }
    }

    private(set) var status: EditHistory.Status?
    /// Path `status` describes.
    private(set) var statusPath: String?
    private(set) var phase: Phase = .idle
    private(set) var pending: Pending?
    private(set) var lastResult: EditHistory.Result?
    private(set) var lastFailure: EditHistory.Failure?
    /// One-line status for the panel (also mirrored to the shell log).
    private(set) var note: String?
    private(set) var refreshing = false
    /// Session-local labels aligned with `status.undoLabels` / `redoLabels`
    /// (nil = only the ledger label is known). Bounded by the stack sizes.
    private(set) var undoAnnotations: [String?] = []
    private(set) var redoAnnotations: [String?] = []
    private var pendingAnnotation: String?
    /// Direction of the move this client completed since the last status read.
    private var movedSinceStatus: EditHistory.Direction?
    private var refreshQueued = false
    private var refreshRequestID: String?
    private var timeoutTask: Task<Void, Never>?
    /// Seconds without a reply after which a move is uncertain. The helper
    /// answers a move in ~10 ms (fsync); 10 s covers a stalled compile queue.
    var replyTimeout: TimeInterval = 10
    private(set) weak var model: ShellModel?

    init() {}

    func bind(_ model: ShellModel) {
        self.model = model
        controllerAttachmentChanged(model.controllerAttached)
    }

    // MARK: derived state

    var isBusy: Bool {
        if case .sending = phase { return true }
        return false
    }
    var isUncertain: Bool {
        if case .uncertain = phase { return true }
        return false
    }
    private var helperReady: Bool {
        guard let model, let controller = model.controller, controller.isRunning, model.controllerState.ready else { return false }
        return true
    }
    /// The buffer equals the durable text for the active path and no edit is in
    /// flight: a move issued now names exactly what the user sees.
    var bufferIsDurable: Bool {
        guard let model, helperReady, model.controllerState.inFlight == nil,
              let durable = model.controllerState.durable[model.activePath],
              let text = model.controllerState.textByDurable[model.activePath]?[durable.revision] else { return false }
        return text.sameBytes(as: model.activeText)
    }
    var canUndo: Bool { canMove && (status?.undoLabels.isEmpty == false) }
    var canRedo: Bool { canMove && (status?.redoLabels.isEmpty == false) }
    private var canMove: Bool {
        guard let model, phase == .idle, pending == nil, bufferIsDurable, let statusPath, statusPath == model.activePath else { return false }
        return true
    }
    var undoRows: [EditHistory.Row] { EditHistory.rows(labels: status?.undoLabels ?? [], annotations: undoAnnotations, stack: .undo) }
    var redoRows: [EditHistory.Row] { EditHistory.rows(labels: status?.redoLabels ?? [], annotations: redoAnnotations, stack: .redo) }
    /// The shell's own edit refusal when it names a retention limit
    /// (`controllerStatus` is "edit refused: history_full: …").
    var shellCapacityRefusal: EditHistory.Failure? {
        guard let s = model?.controllerStatus, s.hasPrefix("edit refused: ") else { return nil }
        let f = EditHistory.Failure.classify(String(s.dropFirst("edit refused: ".count)))
        return f.isCapacity ? f : nil
    }
    /// Capacity message when retention is near or at its limit.
    var capacityWarning: String? {
        if let f = lastFailure, f.isCapacity { return f.description }
        if let f = shellCapacityRefusal { return "The last edit was refused: " + f.description }
        guard let status else { return nil }
        if status.isFull { return EditHistory.Failure.historyFull.description }
        if status.nearCapacity { return String(format: "History retention at %.0f%% (%@); the ledger refuses new edits once it is full.", status.usage * 100, status.retentionSummary) }
        return nil
    }

    // MARK: annotations

    /// Names the NEXT recorded step where the ledger's own label would say only
    /// "Source edit" (e.g. the shell calls `noteNextLabel("Reload from disk")`
    /// before an explicit reload). Attached when the next refresh observes the
    /// undo stack grow by exactly one; dropped otherwise. Session-local.
    func noteNextLabel(_ label: String) { pendingAnnotation = label }

    private func realign(old: EditHistory.Status?, new: EditHistory.Status) {
        guard let old, statusPath == model?.activePath else {
            undoAnnotations = Array(repeating: nil, count: new.undoLabels.count)
            redoAnnotations = Array(repeating: nil, count: new.redoLabels.count)
            pendingAnnotation = nil
            return
        }
        let du = new.undoLabels.count - old.undoLabels.count, dr = new.redoLabels.count - old.redoLabels.count
        // A move this client completed since the last status is the only way to
        // tell "redo" (undo +1, redo −1) from "one new edit cleared one redo".
        let moved = movedSinceStatus
        movedSinceStatus = nil
        if du == 0, dr == 0 {
            // unchanged
        } else if moved == .undo, du == -1, dr == 1 {
            // top of undo moved to top of redo
            let a = undoAnnotations.popLast() ?? nil
            redoAnnotations.append(a)
        } else if moved == .redo, du == 1, dr == -1 {
            let a = redoAnnotations.popLast() ?? nil
            undoAnnotations.append(a)
        } else if du > 0, new.redoLabels.isEmpty {
            // New recorded edits: redo cleared, annotations appended.
            undoAnnotations = Array(undoAnnotations.prefix(old.undoLabels.count)) + Array(repeating: nil, count: du)
            redoAnnotations = []
            if du == 1, let pendingAnnotation, new.undoLabels.last == "Source edit" { undoAnnotations[undoAnnotations.count - 1] = pendingAnnotation }
        } else {
            undoAnnotations = Array(repeating: nil, count: new.undoLabels.count)
            redoAnnotations = Array(repeating: nil, count: new.redoLabels.count)
        }
        pendingAnnotation = nil
        if undoAnnotations.count != new.undoLabels.count { undoAnnotations = Array(repeating: nil, count: new.undoLabels.count) }
        if redoAnnotations.count != new.redoLabels.count { redoAnnotations = Array(repeating: nil, count: new.redoLabels.count) }
    }

    // MARK: status

    /// The last status carried an identity equal to the shell's durable snapshot
    /// for the active path: the stacks it describes are still the current ones.
    var statusIsCurrent: Bool {
        guard let model, let status, statusPath == model.activePath, let identity = status.identity,
              let durable = model.controllerState.durable[model.activePath] else { return false }
        return identity.path == model.activePath && identity.revision == durable.revision && identity.sha256 == durable.sha256
    }

    /// Number of `history_status` requests sent (tests: the same-turn identity
    /// lets the panel skip a round trip when nothing durable changed).
    private(set) var statusRequests = 0

    /// `history_status` for the active path; coalesced (one in flight, the
    /// newest request re-runs after the reply). Every history change advances
    /// the durable revision, so when the last status came with an identity
    /// (main ≥ 64829a0d) that still equals the shell's durable snapshot, the
    /// stacks cannot have changed and no request is sent unless `force`d.
    func refresh(force: Bool = false) {
        guard let model else { return }
        guard helperReady, let controller = model.controller else {
            phase = .unavailable(model.controllerAttached ? "preview controller not ready" : "no preview controller attached")
            return
        }
        if refreshing { refreshQueued = true; return }
        if !force, statusIsCurrent { return }
        let path = model.activePath
        do {
            statusRequests += 1
            let id = try controller.send("history_status", ["path": path])
            refreshing = true
            refreshRequestID = id
            model.controllerState.awaiting[id] = { [weak self] reply in
                guard let self else { return }
                self.refreshing = false
                self.refreshRequestID = nil
                switch reply {
                case .success(let payload):
                    guard let status = EditHistory.Status.decode(payload) else {
                        self.setNote("history_status reply is missing fields"); return
                    }
                    let old = self.statusPath == path ? self.status : nil
                    self.statusPath = path
                    self.realign(old: old, new: status)
                    self.status = status
                    if case .unavailable = self.phase { self.phase = .idle }
                case .failure(let e):
                    let failure = EditHistory.Failure.classify(e.message)
                    self.lastFailure = failure
                    self.setNote("history_status refused: \(failure.description)")
                }
                if self.refreshQueued { self.refreshQueued = false; self.refresh() }
            }
        } catch {
            setNote("history_status failed to send: \(error.localizedDescription)")
        }
    }

    // MARK: moves

    func undo() { move(.undo) }
    func redo() { move(.redo) }

    /// Issues one move with a fresh command id, guarded by the exact durable
    /// revision/hash of the active path. Refused locally (no request) while a
    /// command is pending, the buffer is not yet durable, or the stack is empty.
    private func move(_ direction: EditHistory.Direction) {
        guard let model else { return }
        guard pending == nil else { setNote("a \(pending!.command.direction.rawValue) is still pending; retry or wait for its reply"); return }
        guard bufferIsDurable, let durable = model.controllerState.durable[model.activePath] else {
            setNote("the buffer is not durable yet; wait for the helper's receipt"); return
        }
        guard let status, statusPath == model.activePath else { setNote("history not read yet"); refresh(); return }
        // The stacks were read together with a durable identity (main ≥ 64829a0d):
        // they describe exactly that revision. If the shell's durable snapshot has
        // moved since, the rows on screen are stale — re-read instead of guessing.
        if let identity = status.identity, identity.revision != durable.revision || identity.sha256 != durable.sha256 {
            setNote("the stacks describe r\(identity.revision) but the durable document is r\(durable.revision); re-reading")
            refresh()
            return
        }
        guard direction == .undo ? !status.undoLabels.isEmpty : !status.redoLabels.isEmpty else {
            setNote("nothing to \(direction.rawValue)"); return
        }
        issue(EditHistory.Command(direction: direction, commandID: EditHistory.Command.newID(), path: model.activePath,
                                  expectedRevision: durable.revision, expectedSHA256: durable.sha256))
    }

    /// Issues an explicit command (the panel's moves build theirs from the
    /// current durable snapshot; tests and future group edits pass their own).
    /// Refused while another command is pending.
    func issue(_ command: EditHistory.Command) {
        guard let model else { return }
        guard pending == nil else { setNote("a \(pending!.command.direction.rawValue) is still pending; retry or wait for its reply"); return }
        pending = Pending(command: command, requestIDs: [], sentAt: Date(), editorRevision: model.editorRevision)
        lastFailure = nil
        send(command)
    }

    /// Resends the pending command UNCHANGED (same command id and expectation).
    /// Allowed whenever a command is pending: an extra delivery is idempotent
    /// through the ledger's permanent command ids.
    func resend() {
        guard let pending else { setNote("nothing to retry"); return }
        send(pending.command)
    }

    private func send(_ command: EditHistory.Command) {
        guard let model, var pending, pending.command == command else { return }
        guard helperReady, let controller = model.controller else {
            phase = .uncertain(command.direction)
            self.pending = pending
            lastFailure = .uncertain("helper not attached")
            setNote(lastFailure!.description)
            return
        }
        do {
            let id = try controller.send(command.direction.rawValue, command.payload)
            pending.requestIDs.append(id)
            pending.sentAt = Date()
            self.pending = pending
            phase = .sending(command.direction)
            setNote("\(command.direction.title) sent (attempt \(pending.attempts), r\(command.expectedRevision) expected)")
            model.controllerState.awaiting[id] = { [weak self] reply in
                self?.settle(requestID: id, reply: reply)
            }
            armTimeout(for: command.commandID, requestID: id)
        } catch {
            phase = .uncertain(command.direction)
            self.pending = pending
            lastFailure = .uncertain("send failed: \(error.localizedDescription)")
            setNote(lastFailure!.description)
        }
    }

    private func armTimeout(for commandID: String, requestID: String) {
        timeoutTask?.cancel()
        let delay = replyTimeout
        timeoutTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: UInt64(max(0, delay) * 1_000_000_000))
            guard !Task.isCancelled, let self, let pending = self.pending, pending.command.commandID == commandID,
                  pending.requestIDs.last == requestID, case .sending(let d) = self.phase else { return }
            self.phase = .uncertain(d)
            self.lastFailure = .uncertain(String(format: "no reply within %.1f s", delay))
            self.setNote(self.lastFailure!.description)
        }
    }

    private func settle(requestID: String, reply: Result<[String: Any], ControllerError>) {
        guard let model, let pending, pending.requestIDs.contains(requestID) else { return }
        switch reply {
        case .success(let payload):
            guard let result = EditHistory.Result.decode(payload) else {
                setNote("\(pending.command.direction.title) reply is missing fields; re-reading the document")
                finish(rereading: true)
                return
            }
            timeoutTask?.cancel()
            lastResult = result
            movedSinceStatus = result.replayedCommand ? nil : pending.command.direction
            let doc: [String: Any] = ["path": result.document.path, "revision": result.document.revision,
                                      "source_sha256": result.document.sha256, "text": result.document.text]
            model.controllerAdoptHistoryResult(doc, requestID: requestID, payload: payload,
                                               issuedAtEditorRevision: pending.editorRevision)
            var line = "\(pending.command.direction.title) applied: durable r\(result.commandRevision)"
            if result.replayedCommand { line += " (already applied earlier; adopted current r\(result.document.revision))" }
            if let e = result.previewError { line += "; preview error: \(e)" }
            setNote(line)
            finish(rereading: false)
        case .failure(let e):
            let failure = EditHistory.Failure.classify(e.message)
            lastFailure = failure
            switch failure {
            case .uncertain:
                timeoutTask?.cancel()
                phase = .uncertain(pending.command.direction)
                setNote("\(pending.command.direction.title): \(failure.description)")
            case .documentConflict:
                timeoutTask?.cancel()
                setNote("\(pending.command.direction.title) refused: \(failure.description)")
                finish(rereading: true)
            default:
                timeoutTask?.cancel()
                setNote("\(pending.command.direction.title) refused: \(failure.description)")
                finish(rereading: false)
            }
        }
    }

    /// Clears the pending command; optionally re-reads the durable document
    /// through the shell (its reply goes through the shell's own adoption).
    private func finish(rereading: Bool) {
        pending = nil
        phase = .idle
        if rereading, let model, let controller = model.controller, controller.isRunning {
            _ = try? controller.document(path: model.activePath)
        }
        refresh(force: true)
    }

    /// Drops the pending command without sending anything (the user gives up on
    /// an uncertain step; the ledger keeps its permanent id either way).
    func discardPending() {
        guard pending != nil else { return }
        timeoutTask?.cancel()
        pending = nil
        phase = helperReady ? .idle : .unavailable("no preview controller attached")
        setNote("pending command discarded; the stacks show the durable state")
        if helperReady { finish(rereading: true) }
    }

    // MARK: helper lifecycle

    /// Called when `ShellModel.controllerAttached` changes (and on bind).
    func controllerAttachmentChanged(_ attached: Bool) {
        if attached {
            if case .uncertain = phase {
                // Keep the uncertain command; the user may Retry with the same id.
            } else if pending == nil {
                phase = .idle
            }
            refreshing = false
            refreshRequestID = nil
            // The shell reads the document on `ready`; a status read races that
            // only in ordering, and is coalesced behind later refreshes.
            refresh(force: true)
        } else {
            timeoutTask?.cancel()
            refreshing = false
            refreshRequestID = nil
            if let pending {
                phase = .uncertain(pending.command.direction)
                lastFailure = .uncertain("helper detached before the reply")
                setNote(lastFailure!.description)
            } else {
                phase = .unavailable("no preview controller attached")
            }
        }
    }

    private func setNote(_ line: String) {
        note = line
        model?.log("history: " + line)
    }
}

// MARK: - view

/// The panel: undo/redo stacks (newest first), retention usage, explicit
/// Undo/Redo, and Retry for an uncertain command. Every row and control carries
/// a VoiceOver label naming the action, its step count and its position.
struct EditHistoryPanel: View {
    static let windowID = "edit-history"
    @Environment(ShellModel.self) var model
    @State private var client = EditHistoryClient()

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.m) {
            header
            controls
            if let warning = client.capacityWarning {
                Label(warning, systemImage: "exclamationmark.triangle.fill")
                    .font(.caption).foregroundStyle(DS.Colors.severityWarning)
                    .accessibilityLabel("Retention warning: \(warning)")
                    .accessibilityIdentifier("history.capacity")
            }
            retention
            if let note = client.note {
                Text(note).font(.caption).foregroundStyle(client.lastFailure == nil ? Color.secondary : DS.Colors.severityWarning)
                    .lineLimit(3).textSelection(.enabled)
                    .accessibilityLabel("History status: \(note)")
                    .accessibilityIdentifier("history.note")
            }
            Divider()
            stacks
        }
        .padding(DS.Space.l)
        .frame(minWidth: DS.Layout.historyWindowMinWidth, minHeight: DS.Layout.historyWindowMinHeight)
        .onAppear { client.bind(model); client.refresh() }
        .onChange(of: model.controllerAttached) { _, attached in client.controllerAttachmentChanged(attached) }
        .onChange(of: model.activePath) { _, _ in client.refresh() }
        // The shell's status line changes on every durable receipt; that is the
        // observable signal the stacks moved (ControllerState itself is not observed).
        .onChange(of: model.controllerStatus) { _, _ in client.refresh() }
        .onChange(of: model.editorRevision) { _, _ in client.refresh() }
    }

    private var header: some View {
        HStack {
            Text("Durable history").font(.headline)
            Spacer()
            Text(model.controllerAttached ? "\(model.activePath) · durable r\(model.controllerState.durable[model.activePath]?.revision ?? 0)" : "no preview controller")
                .font(.caption).foregroundStyle(.secondary)
                .accessibilityLabel(model.controllerAttached ? "Durable revision \(model.controllerState.durable[model.activePath]?.revision ?? 0) of \(model.activePath)" : "No preview controller attached")
            Button { client.refresh(force: true) } label: { Image(systemName: "arrow.clockwise") }
                .controlSize(.small)
                .disabled(!model.controllerAttached)
                .help("Re-read the undo/redo stacks (history_status)")
                .accessibilityLabel("Refresh history")
                .accessibilityIdentifier("history.refresh")
        }
    }

    private var controls: some View {
        HStack(spacing: DS.Space.m) {
            Button { client.undo() } label: { Label("Undo", systemImage: "arrow.uturn.backward") }
                .disabled(!client.canUndo)
                .help(undoHelp)
                .accessibilityLabel(client.status.map { "Undo, \($0.undoLabels.count) steps available" } ?? "Undo")
                .accessibilityHint("Moves the durable source one step back, guarded by the current revision")
                .accessibilityIdentifier("history.undo")
            Button { client.redo() } label: { Label("Redo", systemImage: "arrow.uturn.forward") }
                .disabled(!client.canRedo)
                .help("Redo the newest undone step through the ledger")
                .accessibilityLabel(client.status.map { "Redo, \($0.redoLabels.count) steps available" } ?? "Redo")
                .accessibilityIdentifier("history.redo")
            if client.isBusy { ProgressView().controlSize(.small).accessibilityLabel("Waiting for the helper's reply") }
            if client.isUncertain, let pending = client.pending {
                Button { client.resend() } label: { Label("Retry \(pending.command.direction.title)", systemImage: "arrow.triangle.2.circlepath") }
                    .help("Resend the same command id (\(pending.command.commandID)); the ledger applies it at most once")
                    .accessibilityLabel("Retry the uncertain \(pending.command.direction.rawValue) with the same command id, attempt \(pending.attempts + 1)")
                    .accessibilityIdentifier("history.retry")
                Button("Discard") { client.discardPending() }
                    .help("Forget the uncertain command and re-read the durable state")
                    .accessibilityLabel("Discard the uncertain \(pending.command.direction.rawValue)")
                    .accessibilityIdentifier("history.discard")
            }
            Spacer()
            if model.controllerAttached, !client.bufferIsDurable {
                Text("buffer not durable yet").font(.caption).foregroundStyle(DS.Colors.severityWarning)
                    .accessibilityLabel("The editor buffer is not durable yet; undo and redo wait for the helper's receipt")
            }
        }
    }

    private var undoHelp: String {
        guard let s = client.status, let last = s.undoLabels.last else { return "Undo the newest durable step through the ledger" }
        return "Undo “\(last)” through the ledger (the source revision advances)"
    }

    @ViewBuilder private var retention: some View {
        if let s = client.status {
            VStack(alignment: .leading, spacing: DS.Space.xxs) {
                ProgressView(value: min(1, s.usage))
                    .tint(s.isFull ? .red : s.nearCapacity ? .orange : .accentColor)
                    .accessibilityLabel(String(format: "Retention usage %.0f percent", s.usage * 100))
                    .accessibilityIdentifier("history.retention")
                Text(s.retentionSummary).font(.caption2).foregroundStyle(.secondary)
                    .accessibilityLabel("Retention: \(s.retentionSummary)")
            }
        }
    }

    private var stacks: some View {
        List {
            Section {
                if client.undoRows.isEmpty {
                    Text("nothing to undo").font(.caption).foregroundStyle(.secondary)
                        .accessibilityLabel("Undo stack is empty")
                }
                ForEach(client.undoRows) { row in HistoryRowView(row: row, direction: .undo) }
            } header: { Text("Undo (\(EditHistory.steps(client.status?.undoLabels.count ?? 0)), newest first)") }
            Section {
                if client.redoRows.isEmpty {
                    Text("nothing to redo").font(.caption).foregroundStyle(.secondary)
                        .accessibilityLabel("Redo stack is empty")
                }
                ForEach(client.redoRows) { row in HistoryRowView(row: row, direction: .redo) }
            } header: { Text("Redo (\(EditHistory.steps(client.status?.redoLabels.count ?? 0)), next first)") }
        }
        .listStyle(.inset)
        .accessibilityIdentifier("history.stacks")
    }
}

private struct HistoryRowView: View {
    let row: EditHistory.Row
    let direction: EditHistory.Direction

    var body: some View {
        HStack(spacing: DS.Space.m) {
            Image(systemName: row.symbol).foregroundStyle(DS.Colors.textSecondary).frame(width: DS.Size.inlineIconButton)
            VStack(alignment: .leading, spacing: DS.Size.hairline) {
                Text(row.title).font(.body)
                Text(row.detail).font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            if row.steps > 1 {
                Text("×\(row.steps)").font(.caption.monospacedDigit()).foregroundStyle(.secondary)
            }
            if row.distance == 0 {
                Text("next").font(.caption2).padding(.horizontal, DS.Space.xs).background(.quaternary, in: Capsule())
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(row.accessibilityLabel(direction: direction))
        .accessibilityIdentifier("history.\(row.id)")
    }
}
