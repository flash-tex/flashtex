import Foundation
import FlashTeXProtocol

/// Native consumer of the helper's opt-in display-candidate route
/// (crates/preview-controller/docs/display-forwarding.md, "Native gate";
/// helper main e4c9252+): when the app explicitly enables
/// `display-candidates-v1`, the helper enrolls `display-list-v2` in the
/// producer's layout capabilities and forwards the producer's rendering-v2
/// `display_list` sibling of each accepted compile as
/// `update {kind:"display_candidate", untrusted:true,
/// source_actions_enabled:false, request_id, project_id, compile_revision,
/// source_versions, membership_generation, display_list:<v2 envelope>}`.
///
/// Everything a candidate says is untrusted. The shell paints it in the v2
/// pane only when ALL of these hold:
///
/// 1. the route was acknowledged for exactly this helper session/project
///    (`configure_display_candidates` reply; default OFF; a restart or a new
///    session needs a fresh opt-in);
/// 2. the frame's outer session id is the attached client's, its project id
///    the attached project's, and its `request_id`/`compile_revision`/
///    `source_versions` are exactly those of the v1 preview the shell has
///    APPLIED (the helper delivers v1 before the sibling; a candidate for
///    any other request is stale or foreign and is refused), and its
///    `membership_generation` is not older than the generation the shell
///    last learned from the helper (`snapshot` on `ready`, then every
///    open/detach/project_status). The helper's generation is the project
///    index generation, which advances on EVERY durable source update as
///    well as on membership changes, and `edit`/`apply_group` replies do
///    not carry it — so the learned value is a floor, not an equality: a
///    candidate compiled before a membership change the shell has learned
///    names an older generation and is refused (D2), a candidate arriving
///    BEFORE any generation has been learned is refused as
///    `membership_unknown`, and the shell never claims more than that;
/// 3. the v2 envelope decodes, validates, resolves every font by content
///    hash (GH31 byte identity) and prepares every page OFF the UI thread
///    (`V2Loader.queue`), and its `project_id`/`revision` and every declared
///    document's `sha256`/`byte_length` equal the durable text of the source
///    versions it claims — the text the editor had at the revision the
///    candidate is bound to;
/// 4. immediately before publishing on the main thread, session, project,
///    applied request, active document and helper liveness are rechecked.
///
/// Anything else keeps the v1 preview (and the previously verified v2
/// frame) on screen. Candidate receipt never enables source actions: the v2
/// pane's own navigation gate (buffer hash equality plus the shell's stale
/// refusal) is the only path from a painted frame to the editor, and the
/// candidate's flags are recorded, never consulted to grant anything.
enum DisplayCandidates {
    static let capability = "display-candidates-v1"
    static let operation = "configure_display_candidates"
    static let updateKind = "display_candidate"
    /// The layout capability the helper enrolls on our behalf while enabled.
    static let layoutCapability = "display-list-v2"
    static let previewSourceName = "flashtex-preview-controller"
    /// Typed refusal (prefix of `DisplayCandidateGate.rejection`) for a
    /// candidate that arrives before the shell has learned the project's
    /// membership generation from the helper: the membership comparison is
    /// never skipped, so the first candidate of a session is compared
    /// against the generation learned by the `snapshot` sent on `ready`.
    static let membershipUnknown = "membership_unknown"

    /// `FLASHTEX_DISPLAY_CANDIDATES=1` opts the app in at attach; unset/anything else is OFF.
    static func requested(environment: [String: String] = ProcessInfo.processInfo.environment) -> Bool {
        environment["FLASHTEX_DISPLAY_CANDIDATES"] == "1"
    }

    /// True when the helper's reply confirms the capability in the requested state.
    static func acknowledged(_ payload: PreviewControllerClient.JSONObject, enabled: Bool) -> Bool {
        payload["capability"] as? String == capability && payload["enabled"] as? Bool == enabled
    }
}

/// A decoded `display_candidate` update. The display list is kept as the
/// original bytes of the nested envelope (a raw range of the frame), decoded
/// and validated later off the UI thread.
struct DisplayCandidateFrame {
    var sessionID: String
    var requestID: String
    var projectID: String
    var compileRevision: Int
    /// Durable revision per document the candidate was compiled from.
    var sourceVersions: [String: Int]
    var membershipGeneration: Int
    /// Bytes of the rendering-v2 `display_list` envelope, exactly as forwarded.
    var displayList: Data
    /// Size of the whole helper frame (bookkeeping/evidence).
    var frameBytes: Int
    var receivedNs: UInt64

    /// Decodes the payload of an `update` whose `kind` is `display_candidate`.
    /// A frame that contradicts the contract (a trusted or source-action
    /// claim, a missing identity, a display list that is not an object) is a
    /// protocol violation, never a candidate; the v1 route is unaffected.
    static func decode(_ line: Data, payload: [String: FastJSON.Value], frameSessionID: String) -> PreviewControllerClient.Event {
        guard case .bool(true)? = payload["untrusted"] else {
            return .protocolViolation("display_candidate does not declare untrusted:true")
        }
        guard case .bool(false)? = payload["source_actions_enabled"] else {
            return .protocolViolation("display_candidate does not declare source_actions_enabled:false")
        }
        guard let requestID = payload["request_id"]?.string, !requestID.isEmpty else {
            return .protocolViolation("display_candidate without request_id")
        }
        guard let projectID = payload["project_id"]?.string, !projectID.isEmpty else {
            return .protocolViolation("display_candidate \(requestID) without project_id")
        }
        guard let compileRevision = payload["compile_revision"]?.int, compileRevision >= 0 else {
            return .protocolViolation("display_candidate \(requestID) without compile_revision")
        }
        guard let generation = payload["membership_generation"]?.int, generation >= 0 else {
            return .protocolViolation("display_candidate \(requestID) without membership_generation")
        }
        let versions = (payload["source_versions"]?.object ?? [:]).compactMapValues(\.int)
        guard !versions.isEmpty else { return .protocolViolation("display_candidate \(requestID) without source_versions") }
        guard case .raw(let range)? = payload["display_list"] else {
            return .protocolViolation("display_candidate \(requestID) without a display_list object")
        }
        let bytes = line.subdata(in: range)
        guard let header = RenderingV2Fast.header(bytes), header.protocolVersion == 2, header.type == "display_list" else {
            return .protocolViolation("display_candidate \(requestID) display_list is not a rendering-v2 display_list envelope")
        }
        return .displayCandidate(DisplayCandidateFrame(sessionID: frameSessionID, requestID: requestID, projectID: projectID,
                                                       compileRevision: compileRevision, sourceVersions: versions,
                                                       membershipGeneration: generation, displayList: bytes,
                                                       frameBytes: line.count, receivedNs: MonotonicClock.nowNs()))
    }
}

/// The v1 preview the shell has applied, as the candidate must name it.
struct DisplayCandidateAppliedPreview: Equatable {
    var requestID: String
    var compileRevision: Int
    var sourceVersions: [String: Int]
    var editorRevision: Int
}

/// Pure admission gate: what a candidate must match at admission and again
/// immediately before paint. Everything is passed in, nothing is read from
/// the model, so the gate is unit-testable without a helper.
struct DisplayCandidateGate: Equatable {
    var negotiated: Bool
    var sessionID: String?
    var projectID: String?
    /// The applied v1 preview (`ShellModel.resultID` is its request id).
    var applied: DisplayCandidateAppliedPreview?
    var appliedResultID: String?
    var activePath: String
    /// Editor revision of the v2 frame currently on screen from this route.
    var displayedEditorRevision: Int?
    /// The project's membership generation as the shell last learned it from
    /// the helper (`ProjectDocuments.membershipGeneration`: the `snapshot`
    /// sent on `ready`, open_document, detach_document, project_status), or
    /// nil while none has been learned in this helper session. Draft contract
    /// L91: the candidate's `membership_generation` must match fresh
    /// caller-owned state. The helper's number is the project index
    /// generation (crates/project-index `VersionSnapshot.generation`), which
    /// advances on every durable source update too, and edit replies do not
    /// carry it, so the learned value is the FLOOR the candidate must reach:
    /// a candidate compiled before an open/detach the shell learned names an
    /// older generation and is refused (D2); a candidate that arrives before
    /// the first generation is learned is refused as `membership_unknown`
    /// rather than admitted unchecked. Equality is not claimed.
    var membershipGeneration: Int? = nil

    /// Why `frame` may not be shown, or nil when it may (so far).
    func rejection(of frame: DisplayCandidateFrame) -> String? {
        guard negotiated else { return "display candidates not negotiated for this session" }
        guard frame.sessionID == sessionID else { return "session \(frame.sessionID) is not the negotiated \(sessionID ?? "none")" }
        guard frame.projectID == projectID else { return "project \(frame.projectID) is not the attached \(projectID ?? "none")" }
        guard let generation = membershipGeneration else {
            return "\(DisplayCandidates.membershipUnknown): the project's membership generation has not been learned from the helper yet (candidate names generation \(frame.membershipGeneration))"
        }
        if frame.membershipGeneration < generation {
            return "membership generation \(frame.membershipGeneration) is older than the project's learned generation \(generation)"
        }
        guard let applied, appliedResultID == frame.requestID, applied.requestID == frame.requestID else {
            return "request \(frame.requestID) is not the applied v1 preview (\(appliedResultID ?? "none"))"
        }
        guard applied.compileRevision == frame.compileRevision else {
            return "compile generation \(frame.compileRevision) is not the applied preview's \(applied.compileRevision)"
        }
        guard applied.sourceVersions == frame.sourceVersions else {
            return "source versions \(frame.sourceVersions) are not the applied preview's \(applied.sourceVersions)"
        }
        guard frame.sourceVersions[activePath] != nil else { return "no source version for the active document \(activePath)" }
        if let shown = displayedEditorRevision, applied.editorRevision < shown {
            return "editor revision \(applied.editorRevision) is older than the displayed v2 revision \(shown)"
        }
        return nil
    }
}

/// Per-attachment bookkeeping plus the observable user intent and status.
/// One instance lives on the model (`ShellModel.displayCandidates`).
@MainActor
@Observable
final class DisplayCandidateState {
    /// User intent (menu toggle / `FLASHTEX_DISPLAY_CANDIDATES=1`); survives
    /// invalidation so a reattach or restart re-negotiates.
    var requested = DisplayCandidates.requested()
    /// Human-readable route status for the status bar / tests.
    var status = "off"
    /// The helper's last refusal or preview_error for the negotiation, if any.
    var lastError: String?
    /// The last candidate the validator refused for the CURRENT applied v1
    /// preview, with the typed source span of the refusal when the validator
    /// named one (a cluster with no glyph), so the pane can point at the
    /// source and offer the v1 preview of that revision as the fallback
    /// (lane mac-navigation-3). Cleared when a later candidate is painted.
    var lastInvalidCandidate: DisplayCandidateRefusal?

    @ObservationIgnored var sessionID: String?
    @ObservationIgnored var projectID: String?
    @ObservationIgnored var negotiationRequestID: String?
    @ObservationIgnored var negotiationEnabling = false
    @ObservationIgnored private(set) var negotiated = false
    @ObservationIgnored var restartRequestID: String?
    /// The v1 preview the shell applied last (set by applyControllerPreview).
    @ObservationIgnored var applied: DisplayCandidateAppliedPreview?
    /// Newest admitted candidate waiting for validation (older ones dropped).
    @ObservationIgnored private(set) var pending: DisplayCandidateFrame?
    @ObservationIgnored private(set) var pendingActivePath: String?
    /// The candidate being validated off-main, with its load ticket.
    @ObservationIgnored var validating: (frame: DisplayCandidateFrame, ticket: Int, editorRevision: Int)?
    /// Request id of the candidate whose unread durable texts were requested
    /// from the helper once (never re-requested for the same candidate).
    @ObservationIgnored var fetchedTextsFor: String?
    /// Editor revision of the v2 frame this route last published.
    @ObservationIgnored private(set) var displayedEditorRevision: Int?
    /// Work held back until the sibling of `requestID` arrived (or the bound
    /// elapsed): the helper discards a queued optional frame whenever a
    /// required reply is enqueued before its writer's 2 ms idle window, so
    /// the shell keeps the required channel quiet right after a v1 preview.
    @ObservationIgnored var deferred: (requestID: String, works: [() -> Void], timeout: DispatchWorkItem)?
    @ObservationIgnored private(set) var deferredReleasedByCandidate = 0
    @ObservationIgnored private(set) var deferredReleasedByTimeout = 0
    @ObservationIgnored private(set) var deferredReleasedBySupersession = 0
    @ObservationIgnored private(set) var deferredReleasedByInvalidation = 0

    enum DeferredRelease { case candidate, timeout, invalidated }

    /// Runs (in order) and clears the held-back work; nothing when nothing is held.
    func releaseDeferred(_ why: DeferredRelease) {
        guard let d = deferred else { return }
        d.timeout.cancel()
        deferred = nil
        switch why {
        case .candidate: deferredReleasedByCandidate += 1
        case .timeout: deferredReleasedByTimeout += 1
        case .invalidated: deferredReleasedByInvalidation += 1
        }
        for work in d.works { work() }
    }

    /// A newer request's hold starts: takes the older request's held work
    /// (in order) WITHOUT running it, so the caller carries it into the new
    /// hold. Running it here would put required traffic (a completion fetch)
    /// on the wire exactly while the newer request's sibling is expected, and
    /// the helper evicts a queued optional frame on any required reply — the
    /// newer candidate was lost for good (GH-799: startup preview, then ⌘B).
    func supersedeDeferred() -> [() -> Void] {
        guard let d = deferred else { return [] }
        d.timeout.cancel()
        deferred = nil
        deferredReleasedBySupersession += 1
        return d.works
    }
    // Counters for evidence and tests.
    @ObservationIgnored private(set) var received = 0
    @ObservationIgnored private(set) var refused = 0
    @ObservationIgnored private(set) var dropped = 0
    @ObservationIgnored private(set) var invalid = 0
    @ObservationIgnored private(set) var published = 0
    @ObservationIgnored private(set) var lastRefusal: String?
    /// Validation (decode + fonts + prepare) wall time of the last published frame.
    @ObservationIgnored private(set) var lastValidationMs: Double?

    var isNegotiated: Bool { negotiated }

    /// Forgets negotiation, pending work and the display floor: the helper is
    /// gone, restarted, or a new one is being attached.
    func invalidate() {
        sessionID = nil; projectID = nil
        negotiationRequestID = nil
        negotiationEnabling = false
        negotiated = false
        restartRequestID = nil
        applied = nil
        if pending != nil { dropped += 1 }
        pending = nil; pendingActivePath = nil
        validating = nil
        displayedEditorRevision = nil
        // Held work (completion refresh, in-flight edit release) runs, never drops:
        // each piece guards the controller it was queued for.
        releaseDeferred(.invalidated)
    }

    func beginNegotiation(sessionID: String, projectID: String, requestID: String, enabling: Bool) {
        self.sessionID = sessionID
        self.projectID = projectID
        negotiationRequestID = requestID
        negotiationEnabling = enabling
    }

    /// The helper's reply to the negotiation; nil when `requestID` is not it.
    func acknowledge(requestID: String, payload: PreviewControllerClient.JSONObject) -> Bool? {
        guard requestID == negotiationRequestID else { return nil }
        negotiationRequestID = nil
        let ok = DisplayCandidates.acknowledged(payload, enabled: negotiationEnabling)
        negotiated = ok && negotiationEnabling
        if !negotiated { dropPending() }
        return ok
    }

    func refuseNegotiation(requestID: String) -> Bool {
        guard requestID == negotiationRequestID else { return false }
        negotiationRequestID = nil
        negotiated = false
        dropPending()
        return true
    }

    func gate(activePath: String, appliedResultID: String?, membershipGeneration: Int? = nil) -> DisplayCandidateGate {
        DisplayCandidateGate(negotiated: negotiated, sessionID: sessionID, projectID: projectID, applied: applied,
                             appliedResultID: appliedResultID, activePath: activePath, displayedEditorRevision: displayedEditorRevision,
                             membershipGeneration: membershipGeneration)
    }

    enum Admission: Equatable { case queued, replacedPending(String), refused(String) }

    /// Keeps at most one pending candidate: a newer arrival replaces an older one.
    func admit(_ frame: DisplayCandidateFrame, gate: DisplayCandidateGate) -> Admission {
        received += 1
        if let why = gate.rejection(of: frame) {
            refused += 1; lastRefusal = why
            return .refused(why)
        }
        let replaced = pending?.requestID
        pending = frame
        pendingActivePath = gate.activePath
        if let replaced { dropped += 1; return .replacedPending(replaced) }
        return .queued
    }

    func takePending(activePath: String) -> DisplayCandidateFrame? {
        defer { pending = nil; pendingActivePath = nil }
        guard let frame = pending else { return nil }
        guard pendingActivePath == activePath else { dropped += 1; return nil }
        return frame
    }

    /// Puts a taken candidate back (its durable texts are being read) unless
    /// a newer one arrived meanwhile (then the held one is dropped, counted).
    func hold(_ frame: DisplayCandidateFrame, activePath: String) {
        guard pending == nil else { dropped += 1; return }
        pending = frame; pendingActivePath = activePath
    }

    func dropPending() {
        if pending != nil { dropped += 1 }
        pending = nil; pendingActivePath = nil
    }

    func noteRefused(_ why: String) { refused += 1; lastRefusal = why }
    func noteInvalid(_ why: String) { invalid += 1; lastRefusal = why }
    func noteDropped() { dropped += 1 }
    func notePublished(editorRevision: Int, validationMs: Double) {
        displayedEditorRevision = max(displayedEditorRevision ?? 0, editorRevision)
        published += 1
        lastValidationMs = validationMs
    }
}

/// One validator refusal the pane can act on: the request, the editor
/// revision whose v1 preview is the fallback frame, and the source span the
/// validator named (nil for refusals without one).
struct DisplayCandidateRefusal: Equatable {
    var requestID: String
    var editorRevision: Int
    var why: String
    var source: RuntimeV1.SourceRange?
}

/// Off-main validation of one candidate: pure, no model access.
enum DisplayCandidateValidator {
    enum Outcome {
        case verified(V2Frame)
        /// `source` is the span the rendering-v2 validator named, if any.
        case refused(String, source: RuntimeV1.SourceRange? = nil)
    }

    /// Decodes/validates the envelope (RenderingV2), resolves fonts by
    /// content hash and prepares pages, then binds the envelope to the exact
    /// durable text of each source version the candidate names: same
    /// project, the envelope revision is the helper's compile generation,
    /// every declared document has that text's byte length and SHA-256, and
    /// no document is declared that the versions do not cover.
    static func validate(_ frame: DisplayCandidateFrame, texts: [String: String], store: V2FontStore = .shared) -> Outcome {
        let prepared: V2Frame
        switch V2Loader.prepare(data: frame.displayList, store: store) {
        case .failed(let error): return .refused("[\(error.code)] \(error.message)", source: error.source)
        case .loaded(let f): prepared = f
        }
        let list = prepared.list
        guard list.projectId == frame.projectID else {
            return .refused("display_list is for project \(list.projectId); the candidate names \(frame.projectID)")
        }
        guard list.revision == frame.compileRevision else {
            return .refused("display_list revision \(list.revision) is not the candidate's compile generation \(frame.compileRevision)")
        }
        guard !list.documents.isEmpty else { return .refused("display_list declares no documents") }
        for doc in list.documents {
            guard frame.sourceVersions[doc.path] != nil else {
                return .refused("display_list declares \(doc.path), which the candidate's source versions do not cover")
            }
            guard let text = texts[doc.path] else {
                return .refused("no durable text retained for \(doc.path) r\(frame.sourceVersions[doc.path] ?? -1)")
            }
            guard Int64(text.utf8.count) == doc.byteLength else {
                return .refused("\(doc.path): byte_length \(doc.byteLength) differs from the durable text (\(text.utf8.count) bytes)")
            }
            guard SourceDigest.sha256Hex(text) == doc.sha256 else {
                return .refused("\(doc.path): sha256 \(doc.sha256.prefix(12))… differs from the durable text")
            }
        }
        return .verified(prepared)
    }
}

// MARK: - model integration

extension ShellModel {
    /// True once the helper acknowledged the route for the attached session.
    var displayCandidatesNegotiated: Bool { displayCandidates.isNegotiated }

    /// `requestedLayoutCapabilities` as the applied v1 result must be checked
    /// against: while candidates are negotiated the helper enrolls
    /// `display-list-v2` on our behalf, so an accepted `display-list-v2` is
    /// expected, not a violation. Hook for `applyControllerPreview`.
    func displayCandidatesLayoutRequested(_ requested: [String]) -> [String] {
        guard displayCandidates.isNegotiated || displayCandidates.negotiationRequestID != nil,
              !requested.contains(DisplayCandidates.layoutCapability) else { return requested }
        return requested + [DisplayCandidates.layoutCapability]
    }

    /// The layout capabilities to send with `configure_layout` on the helper
    /// route: `display-list-v2` is the helper's to enroll (its
    /// `configure_layout` refuses it while candidates are OFF). Hook for the
    /// `ready` handler.
    func displayCandidatesConfigureLayoutCapabilities(_ requested: [String]) -> [String] {
        requested.filter { $0 != DisplayCandidates.layoutCapability }
    }

    /// Menu / automation: turn the route on or off for the attached helper.
    /// Enabling shows the v2 pane so the frames have somewhere to paint.
    func setDisplayCandidates(_ on: Bool) {
        displayCandidates.requested = on
        if on { previewV2 = true }
        displayCandidatesNegotiate()
    }

    /// Sends the opt-in (or opt-out) for this helper session. Never waits;
    /// the acknowledgement arrives as a `result` for the returned id. Called
    /// on `ready` (after `configure_layout`, so the helper's layout set is
    /// the one it enrolls `display-list-v2` into), on a toggle, and after a
    /// restart (fresh opt-in required).
    func displayCandidatesNegotiate() {
        guard let controller, controller.isRunning, controllerState.ready else {
            displayCandidates.status = displayCandidates.requested ? "requested (waiting for helper)" : "off"
            return
        }
        let want = displayCandidates.requested
        if !want, !displayCandidates.isNegotiated, displayCandidates.negotiationRequestID == nil {
            displayCandidates.status = "off"
            return
        }
        if want { displayCandidatesLearnMembershipGeneration() } // the first candidate is gated on this snapshot
        do {
            let id = try controller.configureDisplayCandidates(enabled: want)
            displayCandidates.beginNegotiation(sessionID: controller.config.sessionID, projectID: controller.config.projectID,
                                               requestID: id, enabling: want)
            displayCandidates.status = want ? "requested \(DisplayCandidates.capability) (\(id))" : "disabling (\(id))"
            log("display-candidate: \(want ? "requested" : "disabling") \(DisplayCandidates.capability) (\(id))")
        } catch {
            displayCandidates.status = "negotiation failed to send: \(error.localizedDescription)"
            log("display-candidate: " + displayCandidates.status)
        }
    }

    /// Learns the project's authoritative membership generation from the
    /// helper (`snapshot`, adopted by `ProjectDocuments.adoptSnapshot`) so
    /// the first candidate of this session is compared, never admitted
    /// unchecked. Sent synchronously with the opt-in — on `ready` (before the
    /// `document` request whose reply admits the first compile, so the
    /// helper answers it before any candidate exists), on a toggle and after
    /// a restart. The reply is routed through `controllerState.awaiting`; a
    /// helper that exits or is detached fails the waiter and the generation
    /// is forgotten with the rest of the route (`displayCandidatesInvalidate`).
    func displayCandidatesLearnMembershipGeneration() {
        guard let controller, controller.isRunning, controllerState.ready else { return }
        let id: String
        do { id = try controller.send("snapshot", [:]) } catch {
            log("display-candidate: snapshot failed to send (\(error.localizedDescription)); candidates stay refused as \(DisplayCandidates.membershipUnknown)")
            return
        }
        let session = controller.config.sessionID
        controllerState.awaiting[id] = { [weak self] reply in
            guard let self, self.controller?.config.sessionID == session else { return } // a later session learns its own
            switch reply {
            case .success(let payload):
                if let snapshot = self.project.adoptSnapshot(payload) {
                    self.log("display-candidate: learned membership generation \(snapshot.generation) (\(snapshot.versions.count) member(s), \(id))")
                } else {
                    self.log("display-candidate: membership generation not learned (\(self.project.status)); candidates stay refused as \(DisplayCandidates.membershipUnknown)")
                }
            case .failure(let e):
                self.log("display-candidate: membership generation not learned (\(e.message)); candidates stay refused as \(DisplayCandidates.membershipUnknown)")
            }
        }
    }

    /// A `result` frame: true when it answered the negotiation or a restart (consumed).
    func displayCandidatesHandle(resultID id: String, payload: PreviewControllerClient.JSONObject) -> Bool {
        if let ok = displayCandidates.acknowledge(requestID: id, payload: payload) {
            let previewError = payload["preview_error"] as? String
            displayCandidates.lastError = previewError
            if ok, displayCandidates.negotiationEnabling {
                displayCandidates.status = "enabled" + (previewError.map { "; preview error: \($0)" } ?? "")
                log("display-candidate: helper acknowledged \(DisplayCandidates.capability)" + (previewError.map { " (preview_error: \($0))" } ?? ""))
            } else if ok {
                displayCandidates.status = "off"
                log("display-candidate: helper disabled \(DisplayCandidates.capability)")
                displayCandidatesRevertPane()
            } else {
                displayCandidates.status = "not acknowledged (\(payload.keys.sorted().joined(separator: ",")))"
                log("display-candidate: helper did not acknowledge \(DisplayCandidates.capability) (\(payload.keys.sorted().joined(separator: ",")))")
            }
            return true
        }
        if id == displayCandidates.restartRequestID {
            displayCandidates.restartRequestID = nil
            log("display-candidate: helper restarted; candidates need a fresh opt-in")
            displayCandidatesNegotiate() // fresh opt-in after restart when still requested
            return true
        }
        return false
    }

    /// An `error` frame: true when it answered the negotiation or a restart (consumed).
    func displayCandidatesHandle(errorID id: String?, message: String) -> Bool {
        guard let id else { return false }
        if displayCandidates.refuseNegotiation(requestID: id) {
            displayCandidates.lastError = message
            displayCandidates.status = "refused: \(message)"
            log("display-candidate: helper refused \(DisplayCandidates.operation): \(message)")
            return true
        }
        if id == displayCandidates.restartRequestID {
            displayCandidates.restartRequestID = nil
            displayCandidates.status = "restart refused: \(message)"
            log("display-candidate: restart refused: \(message)")
            return true
        }
        return false
    }

    /// A v1 preview was applied (hook at the end of `applyControllerPreview`):
    /// this is the only request a candidate may correlate to from now on.
    func displayCandidatesNotePreview(_ update: PreviewControllerClient.PreviewUpdate, editorRevision: Int) {
        displayCandidates.applied = DisplayCandidateAppliedPreview(requestID: update.requestID, compileRevision: update.compileRevision,
                                                                   sourceVersions: update.sourceVersions, editorRevision: editorRevision)
    }

    /// Bound on how long required requests are held back for a sibling
    /// (`FLASHTEX_DISPLAY_CANDIDATES_WAIT_MS`, default 80).
    static let displayCandidateSiblingWaitMs: Double = {
        if let s = ProcessInfo.processInfo.environment["FLASHTEX_DISPLAY_CANDIDATES_WAIT_MS"], let ms = Double(s), ms >= 0 { return ms }
        return 80
    }()

    /// Whether the next edit's release is also held for the sibling
    /// (`FLASHTEX_DISPLAY_CANDIDATES_HOLD=0` disables; default on). Without
    /// the hold, the helper drops nearly every candidate under continuous
    /// typing: the next edit changes its current source before the sibling
    /// is checked, and a candidate is only forwarded for unchanged source
    /// (measured: 3 of 102 candidates forwarded in a 200-keystroke burst).
    static let displayCandidateHoldsRelease = ProcessInfo.processInfo.environment["FLASHTEX_DISPLAY_CANDIDATES_HOLD"] != "0"

    /// Runs `work` now unless a sibling of `requestID` is expected, in which
    /// case it runs when that candidate arrives (admitted or refused) or after
    /// `displayCandidateSiblingWaitMs`. Two reasons the required channel must
    /// stay quiet after a v1 preview: the helper's output slot discards a
    /// queued optional frame whenever a required reply is enqueued before its
    /// writer's 2 ms idle window (output_delivery.rs `try_send`), and the
    /// helper forwards a candidate only while its current source is the one
    /// the candidate was compiled from (display.rs), so an edit admitted
    /// before the sibling is checked invalidates it. Hooks: the completion
    /// refresh and (when `displayCandidateHoldsRelease`) the in-flight edit
    /// release in `applyControllerPreview`. Held work for an older request
    /// is carried, ahead of `work`, into a newer request's hold and runs when
    /// THAT hold ends (never dropped, never run inside the newer sibling's window).
    func displayCandidatesAfterSibling(of requestID: String, acceptedLayout: [String], holdsRelease: Bool = false, _ work: @escaping () -> Void) {
        guard displayCandidates.isNegotiated, acceptedLayout.contains(DisplayCandidates.layoutCapability),
              !holdsRelease || Self.displayCandidateHoldsRelease else { work(); return }
        if displayCandidates.deferred?.requestID == requestID {
            displayCandidates.deferred?.works.append(work)
            return
        }
        let carried = displayCandidates.supersedeDeferred()
        let timeout = DispatchWorkItem { [weak self] in
            guard let self, self.displayCandidates.deferred?.requestID == requestID else { return }
            self.displayCandidates.releaseDeferred(.timeout)
        }
        displayCandidates.deferred = (requestID, carried + [work], timeout)
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.displayCandidateSiblingWaitMs / 1000, execute: timeout)
    }

    /// The sibling of `requestID` arrived (whatever the gate says): release
    /// the held-back work for it on the next main-loop pass, after admission.
    private func displayCandidatesReleaseDeferred(for requestID: String) {
        guard displayCandidates.deferred?.requestID == requestID else { return }
        DispatchQueue.main.async { [weak self] in
            guard let self, self.displayCandidates.deferred?.requestID == requestID else { return }
            self.displayCandidates.releaseDeferred(.candidate)
        }
    }

    /// Restarts the helper's compiler. The helper disables candidates on
    /// restart; a fresh opt-in is sent when the reply arrives and the route
    /// is still requested. Pending and in-flight candidates are void.
    func displayCandidatesRestartHelper() {
        guard let controller, controller.isRunning else { return }
        displayCandidates.dropPending()
        displayCandidates.validating = nil
        displayCandidates.applied = nil
        _ = displayCandidates.refuseNegotiation(requestID: displayCandidates.negotiationRequestID ?? "")
        displayCandidates.invalidate()
        displayCandidates.status = "restarting helper compiler"
        do {
            displayCandidates.restartRequestID = try controller.restart()
            log("display-candidate: restart requested (\(displayCandidates.restartRequestID ?? "-"))")
        } catch {
            displayCandidates.status = "restart failed to send: \(error.localizedDescription)"
        }
    }

    /// Close, exit or reattach (hook in `detachController` / `.exited`):
    /// negotiation, pending and in-flight candidates are void. A frame
    /// already painted stays until the pane is reset or a new one arrives.
    func displayCandidatesInvalidate(reason: String) {
        if displayCandidates.isNegotiated || displayCandidates.pending != nil || displayCandidates.validating != nil {
            log("display-candidate: invalidated (\(reason))")
        }
        displayCandidates.invalidate()
        project.forgetMembership() // the next helper session's generation is learned afresh on its ready
        displayCandidates.status = displayCandidates.requested ? "requested (helper \(reason))" : "off"
    }

    /// A decoded candidate (hook: `case .displayCandidate(let c): handleDisplayCandidate(c)`).
    /// Admitted against the applied v1 preview (at most one pending), then
    /// validated off-main; a newer arrival replaces an unvalidated older one.
    func handleDisplayCandidate(_ frame: DisplayCandidateFrame) {
        displayCandidatesReleaseDeferred(for: frame.requestID)
        let gate = displayCandidates.gate(activePath: activePath, appliedResultID: resultID, membershipGeneration: project.membershipGeneration)
        switch displayCandidates.admit(frame, gate: gate) {
        case .refused(let why):
            log("display-candidate: refused \(frame.requestID) (generation \(frame.compileRevision), \(frame.frameBytes) B): \(why)")
            displayCandidates.status = "enabled; refused \(frame.requestID): \(why)"
            return
        case .replacedPending(let old):
            log("display-candidate: \(frame.requestID) replaces pending \(old)")
        case .queued:
            log("display-candidate: admitted \(frame.requestID) (generation \(frame.compileRevision), \(frame.frameBytes) B)")
        }
        if TypingBench.isBenchActive { FlashTeXLog.write("display-candidate: received \(frame.requestID) generation \(frame.compileRevision) (\(frame.frameBytes) B) at \(frame.receivedNs)") }
        displayCandidatesStartPending()
    }

    /// Starts validating the pending candidate unless one is already in
    /// flight (it starts when that one delivers).
    private func displayCandidatesStartPending() {
        guard displayCandidates.validating == nil, let frame = displayCandidates.takePending(activePath: activePath) else { return }
        guard let editorRev = displayCandidateEditorRevision(of: frame) else {
            displayCandidates.noteRefused("no editor revision for \(activePath) r\(frame.sourceVersions[activePath] ?? -1)")
            log("display-candidate: refused \(frame.requestID): versions unknown to this session")
            return
        }
        // The exact durable text of every version the candidate names. A version
        // this session has not read yet (an include the helper discovered and
        // compiled before the window opened it — FLASHTEX_OPEN_INCLUDES=1 / Open
        // All Includes race the first candidate) is read from the helper's ledger
        // (`document`; the controller handler records it) once, bounded, with the
        // candidate held pending; still-missing text is then refused as before.
        var texts: [String: String] = [:]
        for (path, rev) in frame.sourceVersions { if let t = controllerState.textByDurable[path]?[rev] { texts[path] = t } }
        let missing = frame.sourceVersions.keys.filter { texts[$0] == nil }.sorted()
        if !missing.isEmpty, displayCandidates.fetchedTextsFor != frame.requestID, controllerAttached {
            displayCandidates.fetchedTextsFor = frame.requestID
            displayCandidates.hold(frame, activePath: activePath)
            displayCandidates.status = "enabled; reading \(missing.joined(separator: ", ")) for \(frame.requestID)"
            log("display-candidate: \(frame.requestID) names \(missing.map { "\($0) r\(frame.sourceVersions[$0] ?? -1)" }.joined(separator: ", ")) not read by this session; reading from the helper")
            for path in missing { _ = try? controller?.document(path: path) }
            Task { @MainActor [weak self] in
                let deadline = Date().addingTimeInterval(2)
                while let self, self.controllerAttached, Date() < deadline,
                      missing.contains(where: { self.controllerState.textByDurable[$0]?[frame.sourceVersions[$0] ?? -1] == nil }) {
                    try? await Task.sleep(nanoseconds: 5_000_000)
                }
                self?.displayCandidatesStartPending()
            }
            return
        }
        let ticket = V2Loader.issueTicket()
        let source = V2Source.worker(requestID: frame.requestID, projectId: frame.projectID, revision: editorRev, line: frame.displayList)
        displayCandidates.validating = (frame, ticket, editorRev)
        displayCandidates.status = "enabled; validating \(frame.requestID) (revision \(editorRev))"
        previewV2 = true
        let previousSource = displayListV2?.source
        displayListV2 = .loading(source, ticket: ticket, previous: displayListV2?.frame, previousSource: previousSource)
        let t0 = MonotonicClock.nowNs()
        if TypingBench.isBenchActive { FlashTeXLog.write("display-candidate: validating \(frame.requestID) as revision \(editorRev) ticket \(ticket) at \(t0)") }
        let rasterHint = V2PageRasterizer.shared.rasterHint
        V2Loader.queue.async {
            let validated = DisplayCandidateValidator.validate(frame, texts: texts)
            let t1 = MonotonicClock.nowNs()
            let outcome: DisplayCandidateValidator.Outcome
            var prerastered: V2Loader.Prerastered?
            if case .verified(var verified) = validated {
                // The v2 pane and the typing bench key paints by the editor
                // revision (the v1 result is rebound the same way in
                // applyControllerPreview); the compile generation stays in the log.
                verified.list.revision = editorRev
                outcome = .verified(verified)
                if let hint = rasterHint { prerastered = V2Loader.preraster(verified, hint: hint) }
            } else {
                outcome = validated
            }
            let t2 = MonotonicClock.nowNs()
            let validationMs = Double(t1 &- t0) / 1e6
            let prerasteredResult = prerastered
            V2Loader.deliverOnMain {
                MainActor.assumeIsolated {
                    if TypingBench.isBenchActive {
                        let reused = { if case .verified(let f) = outcome { "\(f.reusedPages)/\(f.prepared.count)" } else { "-" } }()
                        FlashTeXLog.write("display-candidate: validated \(frame.requestID) in \(validationMs) ms (reused pages \(reused)), prerastered \(prerasteredResult?.images.count ?? 0) page(s) in \(Double(t2 &- t1) / 1e6) ms, delivered \(Double(MonotonicClock.nowNs() &- t2) / 1e6) ms later")
                    }
                    self.displayCandidatesDeliver(ticket: ticket, frame: frame, editorRevision: editorRev, source: source,
                                                  previousSource: previousSource, outcome: outcome, prerastered: prerasteredResult, validationMs: validationMs)
                }
            }
        }
    }

    /// The editor revision the candidate's active-document version came from.
    func displayCandidateEditorRevision(of frame: DisplayCandidateFrame) -> Int? {
        guard let rev = frame.sourceVersions[activePath] else { return nil }
        return controllerState.editorRevisionByDurable[activePath]?[rev]
    }

    /// Main thread, after off-main validation: the paint-time recheck, then
    /// publish through the shared v2 delivery (ticket-guarded). A refused or
    /// stale candidate restores the previously verified frame (nothing
    /// unverified is ever on screen) and starts the next pending one.
    private func displayCandidatesDeliver(ticket: Int, frame: DisplayCandidateFrame, editorRevision: Int, source: V2Source,
                                          previousSource: V2Source?, outcome: DisplayCandidateValidator.Outcome,
                                          prerastered: V2Loader.Prerastered?, validationMs: Double) {
        defer {
            if displayCandidates.validating?.ticket == ticket { displayCandidates.validating = nil }
            displayCandidatesStartPending()
        }
        guard displayListV2?.ticket == ticket else {
            displayCandidates.noteDropped()
            log("display-candidate: dropped \(frame.requestID): superseded before publish (ticket \(ticket) is not current)")
            return
        }
        func restorePrevious() {
            if case .loading(_, _, let previous, _, _)? = displayListV2, let previous, let previousSource {
                displayListV2 = .loaded(previous, previousSource)
            } else {
                displayListV2 = nil
            }
        }
        switch outcome {
        case .refused(let why, let refusedSource):
            displayCandidates.noteInvalid(why)
            displayCandidates.lastInvalidCandidate = DisplayCandidateRefusal(requestID: frame.requestID, editorRevision: editorRevision, why: why, source: refusedSource)
            displayCandidates.status = "enabled; invalid \(frame.requestID): \(why)"
            log("display-candidate: invalid \(frame.requestID) (generation \(frame.compileRevision)): \(why)")
            restorePrevious()
            return
        case .verified(let verified):
            // Paint-time recheck: the identity that admitted the candidate must still hold.
            guard let controller, controller.isRunning else {
                displayCandidates.noteRefused("helper is gone"); restorePrevious()
                log("display-candidate: dropped \(frame.requestID) at paint: helper is gone")
                return
            }
            guard controller.config.sessionID == frame.sessionID, controller.config.projectID == frame.projectID else {
                displayCandidates.noteRefused("helper session/project changed before paint"); restorePrevious()
                log("display-candidate: dropped \(frame.requestID) at paint: helper session/project changed")
                return
            }
            if let why = displayCandidates.gate(activePath: activePath, appliedResultID: resultID, membershipGeneration: project.membershipGeneration).rejection(of: frame) {
                displayCandidates.noteRefused(why); restorePrevious()
                log("display-candidate: dropped \(frame.requestID) at paint: \(why)")
                return
            }
            guard displayCandidateEditorRevision(of: frame) == editorRevision else {
                displayCandidates.noteRefused("editor revision binding changed before paint"); restorePrevious()
                log("display-candidate: dropped \(frame.requestID) at paint: revision binding changed")
                return
            }
            if let prerastered { V2PageRasterizer.shared.preinstall(prerastered, frame: verified) }
            V2Loader.notePublished()
            displayListV2 = .loaded(verified, source)
            displayCandidates.lastInvalidCandidate = nil
            displayCandidates.notePublished(editorRevision: editorRevision, validationMs: validationMs)
            let current = editorRevision == self.editorRevision
            displayCandidates.status = String(format: "enabled; painted %@ as revision %d (generation %d, %d page(s), validated in %.1f ms)%@",
                                              frame.requestID, editorRevision, frame.compileRevision, verified.list.pages.count, validationMs,
                                              current ? "" : "; editor at revision \(self.editorRevision)")
            captureNote = "Helper display candidate \(frame.requestID): \(verified.list.pages.count) page(s), \(verified.fonts.count) font(s) resolved by content hash; untrusted, source actions stay gated by the buffer hash"
            if TypingBench.isBenchActive { FlashTeXLog.write("display-candidate: published \(frame.requestID) revision \(editorRevision) at \(MonotonicClock.nowNs())") }
            log("display-candidate: painted \(frame.requestID) as revision \(editorRevision) (generation \(frame.compileRevision), durable \(frame.sourceVersions))")
        }
    }

    /// After an explicit disable: a frame from this route stays on screen
    /// only while it is the applied result's sibling; the pane keeps showing
    /// it until the next v1 result, when nothing v2 is current any more.
    private func displayCandidatesRevertPane() {
        displayCandidates.dropPending()
        displayCandidates.validating = nil
    }
}
