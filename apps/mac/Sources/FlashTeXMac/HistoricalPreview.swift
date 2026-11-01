import Foundation
import FlashTeXProtocol

/// Native consumer of the helper's proposed `completed_snapshot` side channel
/// (crates/preview-controller/docs/completed-snapshot-proposal.md; accepted
/// with binding consumer conditions on issue #2 comment 5645120420).
///
/// A completed snapshot is a fully validated compile result the helper
/// finished for an OLDER source than the one it is compiling now. The shell
/// may paint it while it visibly says a newer revision is compiling, so a
/// typing burst shows progress instead of the last pre-burst preview. It is
/// display only: it grants no navigation, diagnostic jump, caret sync,
/// capture destination or export authority — an explicit `historicalPreview`
/// flag on the model disables those until a current preview replaces it.
///
/// Wire (helper side published on agent/commander-preview-performance/
/// preview-performance ab945e6, "Negotiated stdio adapter" in the proposal;
/// Tests/FlashTeXMacTests/Fixtures/fake_preview_controller.py speaks the same
/// wire for the adversarial cases a real helper never produces):
///
/// 1. After `ready`, and only when the app opted in
///    (`FLASHTEX_COMPLETED_SNAPSHOTS=1`; default OFF), native sends
///    `configure_completed_snapshots {capability:"completed-snapshots-v1", enabled:true}`.
///    The helper answers `result {capability, enabled:true}`; an older helper
///    answers `error "unknown operation"`. Nothing historical is accepted, and
///    no token is sent, until that acknowledgement. `restart`/`close` and a
///    new helper session reset the negotiation on both sides.
/// 2. Once negotiated, every `edit`/`compile` carries an opaque
///    `source_binding_token` (1..128 UTF-8 bytes) that binds that submission
///    to the editor revision it came from (`SourceBindingToken`); the helper
///    echoes it verbatim. In this mode the adapter releases the next edit on
///    the durable receipt instead of holding it for the preview: every
///    keystroke is its own durable edit and the helper reports completed
///    older compiles through this channel instead of `stale`.
/// 3. `update {kind:"completed_snapshot", project_id, session_id, request_id,
///    compile_revision, source_versions, current_compile_revision,
///    is_current:false, source_actions_enabled:false, source_binding_token,
///    result:<unchanged compile_result envelope>}` is decoded as
///    `PreviewControllerClient.Event.completedSnapshot(HistoricalFrame)`.
///
/// Native guarantees: at most one pending historical frame; session id,
/// project id and token are rechecked immediately before painting (after the
/// queued UI hop); a monotonic display floor never paints an older compile
/// generation over a newer displayed one; close, exit, reopen and a document
/// switch invalidate queued deliveries.
enum CompletedSnapshots {
    static let capability = "completed-snapshots-v1"
    static let operation = "configure_completed_snapshots"
    static let updateKind = "completed_snapshot"
    static let maxTokenBytes = 128
    static let previewSourceName = "flashtex-preview-controller"

    static func negotiationPayload() -> PreviewControllerClient.JSONObject {
        ["capability": capability, "enabled": true]
    }

    /// True when the helper's reply to the negotiation confirms the capability enabled.
    static func acknowledged(_ payload: PreviewControllerClient.JSONObject) -> Bool {
        payload["capability"] as? String == capability && payload["enabled"] as? Bool == true
    }

    /// `FLASHTEX_COMPLETED_SNAPSHOTS=1` opts the app in; unset/anything else is OFF.
    static func requested(environment: [String: String] = ProcessInfo.processInfo.environment) -> Bool {
        environment["FLASHTEX_COMPLETED_SNAPSHOTS"] == "1"
    }
}

/// Opaque-to-the-helper token native attaches to each submission:
/// `ftx1:<nonce>:<editor revision>`. The nonce is fresh per negotiation, so a
/// token minted for an earlier helper session (or an earlier negotiation of
/// the same session) never binds a frame to an editor revision of this one.
/// This is provenance, not authentication: the helper cannot forge a valid
/// editor revision it was never told, and a mismatch is simply refused.
struct SourceBindingToken: Equatable {
    static let prefix = "ftx1"
    var nonce: String
    var editorRevision: Int

    static func makeNonce() -> String {
        var bytes = [UInt8](repeating: 0, count: 8)
        for i in bytes.indices { bytes[i] = UInt8.random(in: .min ... .max) }
        return bytes.map { String(format: "%02x", $0) }.joined()
    }

    var encoded: String { "\(Self.prefix):\(nonce):\(editorRevision)" }

    /// Parses a token minted under `nonce`; nil for any other token.
    static func parse(_ token: String, nonce: String) -> SourceBindingToken? {
        guard Self.isWellFormed(token) else { return nil }
        let parts = token.split(separator: ":", omittingEmptySubsequences: false)
        guard parts.count == 3, parts[0] == prefix, parts[1] == nonce, let rev = Int(parts[2]), rev >= 0 else { return nil }
        return SourceBindingToken(nonce: nonce, editorRevision: rev)
    }

    /// 1..128 UTF-8 bytes (the helper's `validate_token`); contents are opaque.
    static func isWellFormed(_ token: String) -> Bool {
        !token.isEmpty && token.utf8.count <= CompletedSnapshots.maxTokenBytes
    }
}

/// A decoded `completed_snapshot` update. Read-only; never a current preview.
struct HistoricalFrame {
    var projectID: String
    var sessionID: String
    var requestID: String
    var compileRevision: Int
    /// The ORIGINATING durable revisions (not the helper's latest).
    var sourceVersions: [String: Int]
    var currentCompileRevision: Int
    var sourceBindingToken: String
    var result: RuntimeV1.Envelope<RuntimeV1.CompileResult>

    /// Decodes the payload of an `update` whose `kind` is `completed_snapshot`.
    /// `line` is the whole frame (the result is a raw byte range into it, read
    /// once by the typed compile_result reader like a `preview` update).
    /// Anything that contradicts the contract — a claim of currency or of
    /// source-action authority, a generation not older than the current one,
    /// a missing or oversized token, a session other than the frame's — is a
    /// protocol violation, never a frame.
    static func decode(_ line: Data, payload: [String: FastJSON.Value], frameSessionID: String) -> PreviewControllerClient.Event {
        guard let projectID = payload["project_id"]?.string, !projectID.isEmpty else {
            return .protocolViolation("completed_snapshot without project_id")
        }
        guard let sessionID = payload["session_id"]?.string else {
            return .protocolViolation("completed_snapshot without session_id")
        }
        guard sessionID == frameSessionID else {
            return .protocolViolation("completed_snapshot for session \(sessionID) inside a frame for \(frameSessionID)")
        }
        guard let requestID = payload["request_id"]?.string, !requestID.isEmpty else {
            return .protocolViolation("completed_snapshot without request_id")
        }
        guard let compileRevision = payload["compile_revision"]?.int, let current = payload["current_compile_revision"]?.int else {
            return .protocolViolation("completed_snapshot \(requestID) without compile_revision/current_compile_revision")
        }
        guard compileRevision < current else {
            return .protocolViolation("completed_snapshot \(requestID) generation \(compileRevision) is not older than current \(current)")
        }
        guard case .bool(false)? = payload["is_current"] else {
            return .protocolViolation("completed_snapshot \(requestID) does not declare is_current:false")
        }
        guard case .bool(false)? = payload["source_actions_enabled"] else {
            return .protocolViolation("completed_snapshot \(requestID) does not declare source_actions_enabled:false")
        }
        guard let token = payload["source_binding_token"]?.string, SourceBindingToken.isWellFormed(token) else {
            return .protocolViolation("completed_snapshot \(requestID) without a well-formed source_binding_token (≤ \(CompletedSnapshots.maxTokenBytes) bytes)")
        }
        let versions = (payload["source_versions"]?.object ?? [:]).compactMapValues(\.int)
        guard !versions.isEmpty else { return .protocolViolation("completed_snapshot \(requestID) without source_versions") }
        let env: RuntimeV1.Envelope<RuntimeV1.CompileResult>
        do {
            switch payload["result"] {
            case .raw(let range)?:
                do { env = try FastJSON.compileResultEnvelope(line, range: range) }
                catch { env = try RuntimeV1.decodeCompileResultReference(line.subdata(in: range)) }
            case nil:
                return .protocolViolation("completed_snapshot \(requestID) without result")
            default:
                return .protocolViolation("completed_snapshot \(requestID) result is not an object")
            }
        } catch {
            return .protocolViolation("completed_snapshot \(requestID): \(error)")
        }
        guard env.protocolVersion == RuntimeV1.protocolVersion, env.type == "compile_result" else {
            return .protocolViolation("completed_snapshot \(requestID) carries \(env.type) v\(env.protocolVersion)")
        }
        return .completedSnapshot(HistoricalFrame(projectID: projectID, sessionID: sessionID, requestID: requestID,
                                                  compileRevision: compileRevision, sourceVersions: versions,
                                                  currentCompileRevision: current, sourceBindingToken: token, result: env))
    }
}

/// What the preview shows while a historical frame is painted. Observable on
/// the model (`ShellModel.historicalPreview`); nil means the displayed result
/// is current (or a fixture) and source actions are allowed.
struct HistoricalDisplay: Equatable {
    /// Editor revision the painted result was compiled from (from the token).
    var shownEditorRevision: Int
    /// Editor revision the helper is compiling now (the in-flight edit's, else the buffer's).
    var compilingEditorRevision: Int
    var shownCompileRevision: Int
    var currentCompileRevision: Int
    var requestID: String
    var resultID: String

    /// The visible banner text (issue #2 condition 4).
    var label: String { "revision \(shownEditorRevision) shown — revision \(compilingEditorRevision) compiling" }
}

/// Per-attachment bookkeeping for the side channel: negotiation, the token
/// nonce, the single pending frame and the monotonic display floor. Not
/// observable; lives on the model as `@ObservationIgnored`.
struct HistoricalPreviewState {
    /// Whether the app asks for the channel at all (default OFF; survives invalidation).
    var requested = CompletedSnapshots.requested()
    /// Editor revision of the newest submission that carried a token.
    private(set) var lastSubmittedEditorRevision: Int?
    /// Helper session/project this negotiation belongs to (from the client config).
    var sessionID: String?
    var projectID: String?
    var nonce = SourceBindingToken.makeNonce()
    var negotiationRequestID: String?
    private(set) var negotiated = false
    /// The one pending frame (newest wins; older pending frames are dropped).
    private(set) var pending: HistoricalFrame?
    /// `activePath` when the pending frame was admitted; a switch invalidates it.
    private(set) var pendingActivePath: String?
    var paintScheduled = false
    /// Highest compile generation displayed in this session (current or historical).
    private(set) var displayFloor: Int?
    // Counters for status and tests.
    private(set) var painted = 0
    private(set) var dropped = 0
    private(set) var refused = 0

    enum Admission: Equatable { case queued, replacedPending(String), refused(String) }

    var isNegotiated: Bool { negotiated }

    /// Forgets negotiation, the pending frame and the floor: the helper is
    /// gone (close/exit) or a new one is being attached.
    mutating func invalidate() {
        sessionID = nil; projectID = nil
        negotiationRequestID = nil
        negotiated = false
        pending = nil; pendingActivePath = nil
        displayFloor = nil
        lastSubmittedEditorRevision = nil
        nonce = SourceBindingToken.makeNonce()
    }

    /// Starts a negotiation for the helper session; nothing is accepted before
    /// `acknowledge` sees the capability in the reply.
    mutating func beginNegotiation(sessionID: String, projectID: String, requestID: String) {
        invalidate()
        self.sessionID = sessionID
        self.projectID = projectID
        negotiationRequestID = requestID
    }

    /// The helper's reply to the negotiation; returns whether it was accepted.
    mutating func acknowledge(requestID: String, payload: PreviewControllerClient.JSONObject) -> Bool? {
        guard requestID == negotiationRequestID else { return nil }
        negotiationRequestID = nil
        negotiated = CompletedSnapshots.acknowledged(payload)
        return negotiated
    }

    /// The helper refused the negotiation (an older helper: "unknown operation").
    mutating func refuseNegotiation(requestID: String) -> Bool {
        guard requestID == negotiationRequestID else { return false }
        negotiationRequestID = nil
        negotiated = false
        return true
    }

    /// Token for a submission from `editorRevision`; nil until negotiated so an
    /// older helper sees exactly the wire it always did.
    mutating func token(forEditorRevision editorRevision: Int) -> String? {
        guard negotiated else { return nil }
        lastSubmittedEditorRevision = max(lastSubmittedEditorRevision ?? 0, editorRevision)
        return SourceBindingToken(nonce: nonce, editorRevision: editorRevision).encoded
    }

    /// The editor revision a frame's token binds it to, or nil when the token
    /// was not minted for this negotiation.
    func editorRevision(of frame: HistoricalFrame) -> Int? {
        SourceBindingToken.parse(frame.sourceBindingToken, nonce: nonce)?.editorRevision
    }

    /// Why `frame` may not be shown now, or nil when it may. Checked on
    /// admission (cheap early drop) and again immediately before painting.
    func rejection(of frame: HistoricalFrame, activePath: String, displayedEditorRevision: Int?) -> String? {
        guard negotiated else { return "completed snapshots not negotiated" }
        guard frame.sessionID == sessionID else { return "session \(frame.sessionID) is not the negotiated \(sessionID ?? "none")" }
        guard frame.projectID == projectID else { return "project \(frame.projectID) is not the attached \(projectID ?? "none")" }
        guard let editorRevision = editorRevision(of: frame) else { return "token \(frame.sourceBindingToken) was not minted for this session" }
        if let floor = displayFloor, frame.compileRevision <= floor {
            return "generation \(frame.compileRevision) is not newer than the displayed generation \(floor)"
        }
        if let displayed = displayedEditorRevision, editorRevision < displayed {
            return "editor revision \(editorRevision) is older than the displayed revision \(displayed)"
        }
        guard frame.sourceVersions[activePath] != nil else { return "no source version for the active document \(activePath)" }
        return nil
    }

    /// Keeps at most one pending frame: a newer arrival replaces an older one.
    mutating func admit(_ frame: HistoricalFrame, activePath: String, displayedEditorRevision: Int?) -> Admission {
        if let why = rejection(of: frame, activePath: activePath, displayedEditorRevision: displayedEditorRevision) {
            refused += 1
            return .refused(why)
        }
        if let old = pending, old.compileRevision >= frame.compileRevision {
            refused += 1
            return .refused("generation \(frame.compileRevision) is not newer than the pending \(old.compileRevision)")
        }
        let replaced = pending?.requestID
        pending = frame
        pendingActivePath = activePath
        if let replaced { dropped += 1; return .replacedPending(replaced) }
        return .queued
    }

    /// Takes the pending frame for painting (nil when the document switched
    /// since admission: queued deliveries are invalidated by a switch).
    mutating func takePending(activePath: String) -> HistoricalFrame? {
        defer { pending = nil; pendingActivePath = nil }
        guard let frame = pending else { return nil }
        guard pendingActivePath == activePath else { dropped += 1; return nil }
        return frame
    }

    mutating func dropPending() {
        if pending != nil { dropped += 1 }
        pending = nil; pendingActivePath = nil
    }

    mutating func notePainted(compileRevision: Int) {
        displayFloor = max(displayFloor ?? 0, compileRevision)
        painted += 1
    }

    /// A current preview (or any newer generation) was displayed: raise the
    /// floor so a delayed historical callback can never repaint over it.
    mutating func noteCurrentDisplayed(compileRevision: Int) {
        displayFloor = max(displayFloor ?? 0, compileRevision)
        dropPending()
    }

    mutating func noteRefused() { refused += 1 }
}

// MARK: - model integration

extension ShellModel {
    /// True while the displayed result is a completed older snapshot.
    var isHistoricalPreview: Bool { historicalPreview != nil }

    /// Why a source action is refused right now, or nil when it is allowed.
    /// `action` names the action for the note ("navigation", "export", …).
    func historicalRefusal(of action: String) -> String? {
        guard let h = historicalPreview else { return nil }
        return "\(action.prefix(1).uppercased() + action.dropFirst()) is unavailable while the preview shows historical revision \(h.shownEditorRevision) (revision \(h.compilingEditorRevision) compiling); it returns with the current preview."
    }

    /// True once the helper acknowledged the channel for the attached session:
    /// submissions carry tokens and the adapter releases edits on the durable
    /// receipt (see ShellModel+Controller.applyDurableDocument).
    var historicalNegotiated: Bool { historicalState.isNegotiated }

    /// On `ready`: requests `completed-snapshots-v1` for this helper session
    /// when the app opted in (`FLASHTEX_COMPLETED_SNAPSHOTS=1`). Never waits;
    /// the acknowledgement arrives as a `result` for the returned id.
    func historicalNegotiate() {
        guard let controller else { return }
        historicalState.invalidate()
        guard historicalState.requested else { return }
        do {
            let id = try controller.send(CompletedSnapshots.operation, CompletedSnapshots.negotiationPayload())
            historicalState.beginNegotiation(sessionID: controller.config.sessionID, projectID: controller.config.projectID, requestID: id)
            log("historical: requested \(CompletedSnapshots.capability) (\(id))")
        } catch {
            log("historical: negotiation failed to send: \(error.localizedDescription)")
        }
    }

    /// The token for a submission from `editorRevision` (nil until negotiated).
    func historicalToken(forEditorRevision editorRevision: Int) -> String? {
        historicalState.token(forEditorRevision: editorRevision)
    }

    /// A `result` frame: true when it was the negotiation reply (consumed).
    func historicalHandle(resultID id: String, payload: PreviewControllerClient.JSONObject) -> Bool {
        guard let accepted = historicalState.acknowledge(requestID: id, payload: payload) else { return false }
        log(accepted ? "historical: helper acknowledged \(CompletedSnapshots.capability)"
                     : "historical: helper did not negotiate \(CompletedSnapshots.capability) (\(payload.keys.sorted().joined(separator: ",")))")
        return true
    }

    /// An `error` frame: true when it answered the negotiation (consumed).
    func historicalHandle(errorID id: String?, message: String) -> Bool {
        guard let id, historicalState.refuseNegotiation(requestID: id) else { return false }
        log("historical: helper refused \(CompletedSnapshots.operation): \(message)")
        return true
    }

    /// A decoded `completed_snapshot`: admitted (at most one pending) and
    /// painted on the next main-loop pass after a recheck. Never paints inline:
    /// the recheck after the queued hop is the contract's paint-time check.
    func historicalReceive(_ frame: HistoricalFrame) {
        let displayed = previewSource == .fixture ? nil : result?.revision
        switch historicalState.admit(frame, activePath: activePath, displayedEditorRevision: displayed) {
        case .refused(let why):
            log("historical: refused \(frame.requestID) (generation \(frame.compileRevision)): \(why)")
            return
        case .replacedPending(let old):
            log("historical: \(frame.requestID) replaces pending \(old)")
        case .queued:
            break
        }
        guard !historicalState.paintScheduled else { return }
        historicalState.paintScheduled = true
        DispatchQueue.main.async { [weak self] in self?.historicalPaintPending() }
    }

    /// Paints the pending frame if every check still holds now.
    func historicalPaintPending() {
        historicalState.paintScheduled = false
        guard let frame = historicalState.takePending(activePath: activePath) else { return }
        guard let controller, controller.isRunning else {
            historicalState.noteRefused()
            log("historical: dropped \(frame.requestID): helper is gone")
            return
        }
        guard controller.config.sessionID == frame.sessionID, controller.config.projectID == frame.projectID else {
            historicalState.noteRefused()
            log("historical: dropped \(frame.requestID): helper session/project changed before paint")
            return
        }
        let displayed = previewSource == .fixture ? nil : result?.revision
        if let why = historicalState.rejection(of: frame, activePath: activePath, displayedEditorRevision: displayed) {
            historicalState.noteRefused()
            log("historical: dropped \(frame.requestID) at paint: \(why)")
            return
        }
        guard let editorRev = historicalState.editorRevision(of: frame) else { return }
        var incoming = frame.result.payload
        incoming.revision = editorRev
        let requested = requestedLayoutCapabilities
        if let violation = LayoutNegotiation.violation(in: incoming, requested: requested) {
            historicalState.noteRefused()
            log("historical: rejected \(frame.requestID): \(violation)")
            return
        }
        result = incoming
        resultID = frame.result.id
        previewSource = .worker(CompletedSnapshots.previewSourceName)
        bindLayout(of: incoming, requested: requested)
        // The originating text when still retained; never the current buffer.
        var compiled: [String: String] = [:]
        for (path, rev) in frame.sourceVersions { if let t = controllerState.textByDurable[path]?[rev] { compiled[path] = t } }
        setCompiledDocuments(compiled)
        let compiling = controllerState.inFlight?.editorRevision ?? historicalState.lastSubmittedEditorRevision ?? editorRevision
        historicalPreview = HistoricalDisplay(shownEditorRevision: editorRev, compilingEditorRevision: compiling,
                                              shownCompileRevision: frame.compileRevision,
                                              currentCompileRevision: frame.currentCompileRevision,
                                              requestID: frame.requestID, resultID: frame.result.id)
        historicalState.notePainted(compileRevision: frame.compileRevision)
        selection = nil
        if TypingBench.isBenchActive { FlashTeXLog.write("compile: applied historical revision \(editorRev) at \(MonotonicClock.nowNs())") }
        workerStatus = String(format: "historical revision %d (helper generation %d of %d): %@, %d diagnostics",
                              editorRev, frame.compileRevision, frame.currentCompileRevision,
                              incoming.status.rawValue, incoming.diagnostics.count)
        log("historical: painted \(frame.requestID) as revision \(editorRev); \(historicalPreview!.label)")
    }

    /// A current preview was applied for helper generation `compileRevision`:
    /// clears the flag and raises the floor. Call after `result` is set.
    func historicalNoteCurrentPreview(compileRevision: Int) {
        historicalState.noteCurrentDisplayed(compileRevision: compileRevision)
        historicalPreview = nil
    }

    /// Close, exit or reattach: queued deliveries and the negotiation are
    /// void. The painted result (if historical) keeps its flag until a current
    /// result replaces it, so source actions stay disabled on old content.
    func historicalInvalidate(reason: String) {
        if historicalState.pending != nil || historicalState.isNegotiated { log("historical: invalidated (\(reason))") }
        historicalState.invalidate()
    }
}
