import Foundation

/// Exact compile correlation on the helper route (crates/preview-controller
/// STDIO.md §"Edit admission correlation", helper from main 55bcf12).
///
/// A full or metadata `edit` result carries a nullable `compile_request_id` /
/// `compile_revision` pair naming the compile the helper admitted for that
/// edit. The pair is an identity, not a promise: the compile may be queued,
/// superseded, discarded, or followed by a `failed` update. The shell's
/// hold-until-preview release therefore matches the asynchronous updates by
/// that request id instead of comparing the helper's compile generation
/// against the document's durable revision (two different counters; the
/// mac-core-review lane's reported-only finding 4).
///
/// When the pair is null (older helper, `history` results, or a submission
/// that failed with `preview_error`) the previous numeric comparison remains
/// as the fallback, exactly as before.
struct ControllerCompileAdmission: Equatable {
    /// The helper's request id for the admitted compile; the `request_id` of
    /// the `preview`/`stale`/`discarded`/`failed`/`cancelled` update it ends in.
    var requestID: String
    /// The helper's compile generation for that request. Nil after a
    /// `superseded` rebinding: the superseding request's generation is not on
    /// the wire, and the id alone is sufficient identity.
    var compileRevision: Int?

    /// The pair from an `edit` result payload; nil when either field is
    /// absent or JSON null (`NSNull`), or when the revision is not an integer.
    static func from(editResult payload: [String: Any]) -> ControllerCompileAdmission? {
        guard let id = payload["compile_request_id"] as? String, !id.isEmpty,
              let revision = payload["compile_revision"] as? Int else { return nil }
        return ControllerCompileAdmission(requestID: id, compileRevision: revision)
    }
}

enum AdmissionCorrelation {
    /// Whether an update releases the in-flight edit, with the reason logged.
    enum Decision: Equatable {
        case release(String)
        case hold(String)

        var releases: Bool { if case .release = self { return true } else { return false } }
        var reason: String {
            switch self {
            case .release(let why), .hold(let why): return why
            }
        }
    }

    /// Update kinds that end an admitted compile: exactly one of them arrives
    /// for each admitted request id (crates/document-runtime `fail`/`cancel`
    /// emit one per queued request; the helper turns a completed compile into
    /// `preview`, `stale` or `discarded`).
    static let terminalKinds: Set<String> = ["preview", "stale", "discarded", "failed", "cancelled"]

    /// Decides whether `update {kind, request_id, compile_revision?}` releases
    /// the in-flight edit.
    ///
    /// - `admitted`: the pair recorded from the edit's reply. Non-nil: release
    ///   only when the update names that request id (and, when both sides
    ///   carry a compile revision, the same one); a mismatch never releases,
    ///   whatever the numbers say.
    /// - nil: the numeric fallback of the pre-correlation shell —
    ///   `stale`/`discarded` release when `compile_revision >= durableRevision`,
    ///   `failed`/`cancelled` release any durable in-flight edit, `preview`
    ///   releases when the compiled version of the in-flight path is
    ///   `>= durableRevision` (`previewVersion`).
    /// - `durableRevision`: the in-flight edit's durable revision, nil until
    ///   its reply arrived (nothing is released before that in either mode).
    static func decision(kind: String, requestID: String?, compileRevision: Int?, previewVersion: Int? = nil,
                         admitted: ControllerCompileAdmission?, durableRevision: Int?) -> Decision {
        guard terminalKinds.contains(kind) else { return .hold("\(kind) is not a compile outcome") }
        guard let durableRevision else { return .hold("\(kind) before the in-flight edit is durable") }
        if let admitted {
            guard let requestID else { return .hold("\(kind) without request_id (admitted \(admitted.requestID))") }
            guard requestID == admitted.requestID else {
                return .hold("\(kind) \(requestID) is not the admitted compile \(admitted.requestID)")
            }
            if let want = admitted.compileRevision, let got = compileRevision, got != want {
                return .hold("\(kind) \(requestID) carries compile_revision \(got), admitted \(want)")
            }
            return .release("\(kind) \(requestID) is the admitted compile (generation \(admitted.compileRevision.map(String.init) ?? "-"), durable r\(durableRevision))")
        }
        switch kind {
        case "stale", "discarded":
            guard let compileRevision else { return .hold("\(kind) without compile_revision (no admission recorded)") }
            guard compileRevision >= durableRevision else {
                return .hold("\(kind) generation \(compileRevision) < durable r\(durableRevision) (numeric fallback)")
            }
            return .release("\(kind) generation \(compileRevision) >= durable r\(durableRevision) (numeric fallback, no admission recorded)")
        case "preview":
            guard let previewVersion, previewVersion >= durableRevision else {
                return .hold("preview for durable r\(previewVersion.map(String.init) ?? "-") < r\(durableRevision) (numeric fallback)")
            }
            return .release("preview for durable r\(previewVersion) >= r\(durableRevision) (numeric fallback, no admission recorded)")
        default: // failed, cancelled: the compiler session is gone for every admitted compile
            return .release("\(kind) releases the durable in-flight edit (no admission recorded)")
        }
    }

    /// `update {kind:"superseded", request_id, by_id}`: the admitted compile was
    /// replaced in the queue by a later admission of the same document before
    /// it was dispatched; no outcome will ever name it. Returns the rebinding
    /// to the superseding request (whose outcome then releases the edit), or
    /// nil when the update does not name the admitted compile.
    static func rebinding(kind: String, requestID: String?, byID: String?,
                          admitted: ControllerCompileAdmission?) -> ControllerCompileAdmission? {
        guard kind == "superseded", let admitted, let requestID, requestID == admitted.requestID,
              let byID, !byID.isEmpty, byID != requestID else { return nil }
        return ControllerCompileAdmission(requestID: byID, compileRevision: nil)
    }
}

extension ShellModel {
    /// `.update(kind:payload:)` from the helper: rebinding on `superseded`,
    /// then the release decision for the in-flight edit (logged either way
    /// when an admission is recorded, so a mismatched id leaves a trace).
    /// The caller releases the pipeline when this returns true.
    func controllerAdmissionReleases(kind: String, payload: PreviewControllerClient.JSONObject) -> Bool {
        guard let inFlight = controllerState.inFlight else { return false }
        let requestID = payload["request_id"] as? String
        if let rebound = AdmissionCorrelation.rebinding(kind: kind, requestID: requestID, byID: payload["by_id"] as? String,
                                                        admitted: inFlight.admitted) {
            controllerState.inFlight?.admitted = rebound
            log("controller superseded admitted compile \(requestID ?? "-") by \(rebound.requestID); the in-flight edit now waits for that outcome")
            return false
        }
        let decision = AdmissionCorrelation.decision(kind: kind, requestID: requestID, compileRevision: payload["compile_revision"] as? Int,
                                                     admitted: inFlight.admitted, durableRevision: inFlight.durableRevision)
        if decision.releases || inFlight.admitted != nil {
            log("controller \(decision.releases ? "release" : "hold"): \(decision.reason) (in flight revision \(inFlight.editorRevision))")
        }
        return decision.releases
    }

    /// `preview` update: same decision keyed by the preview's request id, with
    /// the compiled version of the in-flight path as the numeric fallback.
    func controllerAdmissionReleases(preview update: PreviewControllerClient.PreviewUpdate) -> Bool {
        guard let inFlight = controllerState.inFlight else { return false }
        let decision = AdmissionCorrelation.decision(kind: "preview", requestID: update.requestID, compileRevision: update.compileRevision,
                                                     previewVersion: update.sourceVersions[inFlight.path],
                                                     admitted: inFlight.admitted, durableRevision: inFlight.durableRevision)
        if decision.releases || inFlight.admitted != nil {
            log("controller \(decision.releases ? "release" : "hold"): \(decision.reason) (in flight revision \(inFlight.editorRevision))")
        }
        return decision.releases
    }
}
