import Combine
import Foundation
import FlashTeXProtocol

/// Captures received from paired companions while no bridge is attached, kept
/// in memory only. Nothing here is journaled, so acknowledgements say
/// `durable: false`; with a bridge attached (`ShellModel+Bridge.swift`) the
/// capture is forwarded there instead and its durable acknowledgement returned.
@MainActor
final class NearbyInbox: ObservableObject {
    private(set) var received: [RuntimeV1.CaptureSubmit] = []
    private(set) var lastCaptureId: String?
    private(set) var lastNote: String?
    static let maxRetained = 50
    /// Retained base64 bytes across `received`; oldest captures are dropped first.
    static let maxRetainedBytes = 64 * 1024 * 1024

    /// Stores a capture; a repeated `capture_id` with an identical payload is
    /// acknowledged again, a different payload for a known id is refused.
    func store(_ submit: RuntimeV1.CaptureSubmit) -> Result<NearbyV1.CaptureReceived, NearbyV1.ErrorPayload> {
        if let existing = received.first(where: { $0.captureId == submit.captureId }) {
            guard existing == submit else {
                lastNote = "Refused \(submit.captureId): different payload for a known capture_id."
                return .failure(.init(code: "capture_id_conflict", message: "capture_id \(submit.captureId) already received with a different payload"))
            }
            lastNote = "Duplicate \(submit.captureId) acknowledged again."
        } else {
            received.append(submit)
            if received.count > Self.maxRetained { received.removeFirst(received.count - Self.maxRetained) }
            while received.count > 1, received.reduce(0, { $0 + $1.image.dataBase64.utf8.count }) > Self.maxRetainedBytes {
                received.removeFirst()
            }
            lastNote = "Received \(submit.captureId) (\(submit.image.mimeType), \(submit.image.dataBase64.utf8.count) base64 bytes); not journaled."
        }
        lastCaptureId = submit.captureId
        return .success(.init(captureId: submit.captureId, durable: false, hasProposal: false, applied: false))
    }

    func noteForwarded(_ submit: RuntimeV1.CaptureSubmit, _ ack: NearbyV1.CaptureReceived) {
        lastCaptureId = submit.captureId
        lastNote = "Forwarded \(submit.captureId) to the bridge (durable: \(ack.durable))."
    }
}

extension NearbyV1.ErrorPayload: Error {}

extension ShellModel: CaptureSink, DestinationProvider {
    /// Accepts a capture from a paired companion while no bridge is attached.
    /// `durable` is false because only the bridge can promise a journaled receipt.
    func receiveNearbyCapture(_ submit: RuntimeV1.CaptureSubmit) -> Result<NearbyV1.CaptureReceived, NearbyV1.ErrorPayload> {
        nearbyInbox.store(submit)
    }

    /// Forwards to the attached bridge (transfer-v1 `capture_submit`) and
    /// returns its acknowledgement; bridge `error` envelopes pass through.
    func forwardNearbyCapture(_ submit: RuntimeV1.CaptureSubmit, pairId: String? = nil) async -> Result<NearbyV1.CaptureReceived, NearbyV1.ErrorPayload> {
        // The Captures panel shows the row the moment the bytes are here.
        captureInbox.received(submit, pairId: pairId, autoPinned: submit.destinationId == captureInbox.autoPinnedDestinationId)
        guard let bridge, bridge.running else { return receiveNearbyCapture(submit) }
        do {
            let ack = try await bridge.submit(submit)
            let received = NearbyV1.CaptureReceived(captureId: ack.captureId, durable: ack.durable,
                                                    hasProposal: ack.hasProposal, applied: ack.applied)
            nearbyInbox.noteForwarded(submit, received)
            // Fluid path: the companion already said what it wants, so the
            // conversion starts now; the proposal lands in the panel and the
            // review queue, and insertion still waits for the explicit click.
            if CaptureInboxFeature.autoConvert, bridge.conversionEnabled, !ack.hasProposal, !ack.applied {
                Task { await convertCaptureForInbox(captureId: ack.captureId) }
            }
            return .success(received)
        } catch let f as BridgeClient.Failure {
            captureInbox.noteFailure(submit.captureId, "bridge refused the capture: \(f.text)")
            if case .bridge(let e) = f { return .failure(.init(code: e.code, message: e.message)) }
            return .failure(.init(code: "unavailable", message: "bridge: \(f.text)"))
        } catch {
            return .failure(.init(code: "unavailable", message: "bridge: \(error)"))
        }
    }

    /// The pinned anchor as the companion sees it, or nil when nothing is
    /// pinned. With a bridge attached the bridge's anchor is authoritative
    /// (`base_revision` = its pinned revision; `nil` once an edit dropped it —
    /// the bridge would refuse the local anchor's id); otherwise the local
    /// anchor. See `NearbyDestination.swift`.
    var nearbyDestination: NearbyV1.Destination? {
        announcedNearbyDestination(bridgeAttached: bridgeAttached, bridgeAnchor: bridgeDestination, localAnchor: anchor)
    }

    // MARK: CaptureSink / DestinationProvider (called from the listener queue)

    nonisolated func submit(_ envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, reply: @escaping (Data) -> Void) {
        Task { @MainActor in
            switch await self.forwardNearbyCapture(envelope.payload) {
            case .success(let ack): reply(NearbyV1.line(id: envelope.id, type: "capture_received", ack))
            case .failure(let err): reply(NearbyV1.errorLine(id: envelope.id, code: err.code, message: err.message))
            }
        }
    }

    nonisolated func currentDestination(_ reply: @escaping (NearbyV1.Destination?) -> Void) {
        Task { @MainActor in reply(await self.nearbyDestinationPinningCaretIfNeeded()) }
    }

    /// What `hello_ack` / `destination_query` announce (lane mac-capture-fluid):
    /// the pin when one is valid; otherwise the caret, pinned right now on the
    /// companion's behalf (locally and, when attached, on the bridge, awaited
    /// so the id the companion binds to is one the bridge knows). The user
    /// never has to press ⌘⌥P; an explicit pin still overrides until an edit
    /// drops it. Off with `FLASHTEX_CAPTURE_CARET_DESTINATION=0`.
    func nearbyDestinationPinningCaretIfNeeded() async -> NearbyV1.Destination? {
        if let d = nearbyDestination { return d }
        guard CaptureInboxFeature.caretDestination else { return nil }
        guard historicalRefusal(of: "pinning an insertion point") == nil,
              let anchor = Insertion.makeAnchor(id: "mac-caret-\(nextAnchorNumber)", path: activePath,
                                                text: activeText, caretUTF16: caretUTF16, revision: editorRevision)
        else { return nil }
        nextAnchorNumber += 1
        self.anchor = anchor
        captureInbox.autoPinnedDestinationId = anchor.id
        if let bridge, bridge.running {
            let end = activeText.utf8ByteRange(of: NSRange(location: caretUTF16, length: caretLengthUTF16))?.end ?? anchor.byteOffset
            guard await bridgePinAndWait(destinationId: anchor.id, path: anchor.path, revision: anchor.revision,
                                         startByte: anchor.byteOffset, endByte: max(end, anchor.byteOffset)) != nil else { return nil }
        }
        captureNote = "Insertion point: the caret (\(anchor.path) byte \(anchor.byteOffset)); pin (⌘⌥P) to override."
        return nearbyDestination
    }

    /// True while the announced destination is the automatic caret pin (or nothing is pinned yet).
    var captureDestinationIsAutomatic: Bool {
        guard let d = nearbyDestination else { return true }
        return d.destinationId == captureInbox.autoPinnedDestinationId
    }

    /// Opening the Captures panel: advertise (a paired iPad connects without
    /// the Nearby window) and attach the discovered bridge when none is
    /// attached, so the first capture converts instead of waiting in the inbox.
    func prepareCaptureInbox(nearby: NearbyState) {
        if !nearby.isAdvertising { nearby.startAdvertising() }
        if !bridgeAttached, CaptureInboxFeature.autoAttachBridge { attachDiscoveredBridge() }
    }

    nonisolated func captureStatus(_ envelope: RuntimeV1.Envelope<NearbyV1.CaptureStatusRequest>, reply: @escaping (Data) -> Void) {
        Task { @MainActor in
            switch await self.nearbyCaptureStatus(captureId: envelope.payload.captureId) {
            case .success(let ack): reply(NearbyV1.line(id: envelope.id, type: "capture_status_ack", ack))
            case .failure(let err): reply(NearbyV1.errorLine(id: envelope.id, code: err.code, message: err.message))
            }
        }
    }

    /// nearby-v1 `capture_status` (additive): what the Mac knows about one
    /// companion capture. With a bridge attached the bridge's `capture_status`
    /// row is authoritative for the proposal text, insertion and rejection;
    /// the session's live state supplies `converting` / `failed` / `uncertain`,
    /// which the bridge row does not carry. Without a bridge the in-memory
    /// inbox answers `received`. Nothing is inferred: a capture neither knows
    /// is `unknown_capture`.
    func nearbyCaptureStatus(captureId: String) async -> Result<NearbyV1.CaptureStatusAck, NearbyV1.ErrorPayload> {
        if let bridge, bridge.running {
            let local = bridge.capture(captureId)
            var row: TransferV1.CaptureStatus?
            do { row = try await bridge.status(captureId: captureId) } catch let f as BridgeClient.Failure {
                // A bridge that does not know the id (or a transient failure)
                // leaves only the local state; an inbox-only capture falls through.
                if local == nil, !f.isTransient, nearbyInbox.received.contains(where: { $0.captureId == captureId }) {
                    return .success(.init(captureId: captureId, state: .received, durable: false, note: "in the Mac's inbox; not journaled by the bridge"))
                }
                if local == nil { return .failure(.init(code: "unknown_capture", message: "bridge: \(f.text)")) }
            } catch {
                if local == nil { return .failure(.init(code: "unavailable", message: "bridge: \(error)")) }
            }
            return .success(Self.captureStatusAck(captureId: captureId, local: local, row: row))
        }
        if nearbyInbox.received.contains(where: { $0.captureId == captureId }) {
            return .success(.init(captureId: captureId, state: .received, durable: false,
                                  note: "in the Mac's in-memory inbox; attach a capture bridge on the Mac (Edit > Attach Capture Bridge) to convert it"))
        }
        return .failure(.init(code: "unknown_capture", message: "capture \(captureId) is not known to this Mac"))
    }

    /// Pure mapping (tested): bridge row first (`applied` → inserted,
    /// `rejected`, `proposal` → proposal_ready), then the live session state
    /// for the phases the row cannot show.
    static func captureStatusAck(captureId: String, local: BridgeSession.Capture?, row: TransferV1.CaptureStatus?) -> NearbyV1.CaptureStatusAck {
        let latex = row?.proposal?.latex
        if let a = row?.applied {
            return .init(captureId: captureId, state: .inserted, durable: true, latex: latex,
                         note: local?.note ?? "inserted on the Mac (edit \(a.editId))", newRevision: a.newRevision)
        }
        if row?.rejected == true {
            return .init(captureId: captureId, state: .rejected, durable: true, latex: latex, note: local?.note ?? "rejected on the Mac")
        }
        switch local?.state {
        case .converting: return .init(captureId: captureId, state: .converting, durable: true, note: local?.note)
        case .failed: return .init(captureId: captureId, state: .failed, durable: true, latex: latex, note: local?.note)
        case .needsReselection:
            return .init(captureId: captureId, state: .failed, durable: true, latex: latex,
                         note: local?.note ?? "the pinned destination changed; reselect on the Mac")
        case .uncertain: return .init(captureId: captureId, state: .uncertain, durable: false, note: local?.note)
        case .applied, .confirmed:
            return .init(captureId: captureId, state: .inserted, durable: true, latex: latex, note: local?.note)
        case .rejected: return .init(captureId: captureId, state: .rejected, durable: true, latex: latex, note: local?.note)
        default: break
        }
        if latex != nil || local?.state == .proposed || local?.state == .prepared {
            return .init(captureId: captureId, state: .proposalReady, durable: true, latex: latex,
                         note: local?.note ?? "proposal awaiting review on the Mac")
        }
        return .init(captureId: captureId, state: .journaled, durable: true, note: local?.note ?? "journaled by the bridge; not converted yet (Edit > Convert Capture on the Mac)")
    }

    // MARK: nearby-v1 `capture_insert` (additive)

    nonisolated func captureInsert(_ envelope: RuntimeV1.Envelope<NearbyV1.CaptureInsertRequest>, reply: @escaping (Data) -> Void) {
        Task { @MainActor in
            switch await self.nearbyCaptureInsert(captureId: envelope.payload.captureId,
                                                  approvedDigest: envelope.payload.approvedLatexSha256) {
            case .success(let ack): reply(NearbyV1.line(id: envelope.id, type: "capture_insert_ack", ack))
            case .failure(let err): reply(NearbyV1.errorLine(id: envelope.id, code: err.code, message: err.message))
            }
        }
    }

    /// The companion's Insert tap. This is the *same* approval the Captures
    /// panel's Insert button performs — `insertCaptureFromInbox`, which is
    /// `approveBridgeProposal` / `approveProposal` — reached from the iPad
    /// instead of from the Mac. No new insertion semantics: the bridge still
    /// prepares the edit, the Mac still applies exactly one undoable edit
    /// against a matching revision and hash, and the ledger still refuses a
    /// second edit for the same capture.
    ///
    /// What makes this an approval rather than an automatic insertion is
    /// `approvedDigest`: the SHA-256 of the proposal text the companion
    /// actually displayed (`capture_status_ack.latex`). A proposal that has
    /// changed since — re-converted after a stale context, say — no longer
    /// hashes the same, so the tap is refused with `proposal_changed` and the
    /// companion must read the new text before approving it. transfer-v1's
    /// "require explicit review approval for the currently displayed proposal"
    /// therefore still holds; only the display moved to the iPad.
    func nearbyCaptureInsert(captureId: String, approvedDigest: String) async
        -> Result<NearbyV1.CaptureInsertAck, NearbyV1.ErrorPayload> {
        guard let item = captureInbox.items.first(where: { $0.id == captureId }) else {
            return .failure(.init(code: "unknown_capture", message: "capture \(captureId) is not in this Mac's Captures inspector"))
        }
        if appliedCaptureIDs.contains(captureId) {
            // Terminal and idempotent: a retried tap after a dropped reply
            // reports the existing insertion; it never produces a second edit.
            return .success(.init(captureId: captureId, state: .inserted,
                                  note: "already inserted on the Mac; no second edit was made"))
        }
        if item.rejected {
            return .success(.init(captureId: captureId, state: .rejected, note: "rejected on the Mac; start a new capture"))
        }
        guard let proposal = captureInboxProposal(item) else {
            return .failure(.init(code: "no_proposal", message: "capture \(captureId) has no proposal awaiting review on the Mac"))
        }
        guard NearbyV1.proposalDigest(proposal.latex) == approvedDigest else {
            return .failure(.init(code: "proposal_changed",
                                  message: "the Mac's proposal for \(captureId) is not the text this companion approved; read it again (capture_status) before inserting"))
        }
        let outcome = await insertCaptureFromInbox(item, latex: proposal.latex)
        switch outcome {
        case .inserted(let byteOffset):
            return .success(.init(captureId: captureId, state: .inserted, newRevision: editorRevision,
                                  note: "inserted on the Mac at byte \(byteOffset)"))
        case .duplicate:
            return .success(.init(captureId: captureId, state: .inserted,
                                  note: "already inserted on the Mac; no second edit was made"))
        case .needsReselection(let why):
            return .success(.init(captureId: captureId, state: .failed,
                                  note: "the insertion point changed (\(why)); pin again on the Mac and resend"))
        case .noAnchor:
            return .success(.init(captureId: captureId, state: .failed,
                                  note: "the Mac has no insertion point; pin one (⌘⌥P) and resend"))
        case .refused(let why):
            return .failure(.init(code: "insert_refused", message: "the Mac refused the insertion: \(why)"))
        }
    }
}

/// Switches for the fluid capture path (lane mac-capture-fluid); each defaults on.
enum CaptureInboxFeature {
    static func flag(_ name: String, environment: [String: String] = ProcessInfo.processInfo.environment) -> Bool {
        environment[name] != "0"
    }
    /// A nearby capture is converted as soon as the bridge journals it.
    static var autoConvert: Bool { flag("FLASHTEX_CAPTURE_AUTO_CONVERT") }
    /// With nothing pinned, the caret is pinned for the companion on demand.
    static var caretDestination: Bool { caretDestinationOverride ?? flag("FLASHTEX_CAPTURE_CARET_DESTINATION") }
    /// Tests of the explicit-pin contract set this to false.
    nonisolated(unsafe) static var caretDestinationOverride: Bool?
    /// Opening the Captures panel attaches the discovered bridge.
    static var autoAttachBridge: Bool { flag("FLASHTEX_CAPTURES_AUTO_ATTACH") }
    /// Launch advertises when a companion is already paired (pure; tested).
    static func autoAdvertise(pairs: Int, environment: [String: String] = ProcessInfo.processInfo.environment) -> Bool {
        pairs > 0 && flag("FLASHTEX_NEARBY_AUTO_ADVERTISE", environment: environment)
    }
}
