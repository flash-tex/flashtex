import CryptoKit
import FlashTeXCollabCore
import Foundation
import Network

/// A joining participant (proposal §3.2, §3.4): connects to the hub the
/// invite names (pinning its key), proves the invite, waits for the hub
/// user's approval, then syncs. Local operations go through an outbox the
/// hub acknowledges; a dropped connection is retried with backoff (0.5 s
/// doubling to 30 s) using the token the hub issued, and both sides resync
/// by state vector, so nothing typed offline is lost while the app runs.
@MainActor
public final class CollabGuest {
    public enum State: Equatable, Sendable {
        case connecting
        case awaitingApproval
        case joined
        case reconnecting(attempt: Int)
        /// For good: declined, removed, invalid invite, session ended, or left.
        case ended(String)
    }

    public let invite: CollabInvite
    public let displayName: String
    public let deviceKind: String
    public private(set) var state: State = .connecting { didSet { if state != oldValue { onStateChange(state) } } }
    public private(set) var session: CollabSession?
    public private(set) var joinAck: CollabControl.JoinAck?
    public var onStateChange: (State) -> Void = { _ in }
    /// The first `join_ack`: the session exists from here on.
    public var onJoined: (CollabSession, CollabControl.JoinAck) -> Void = { _, _ in }
    /// Overrides where to connect (tests); nil uses the invite.
    public var endpointsOverride: [NWEndpoint]?

    /// The live connection (internal for tests that misbehave on purpose).
    var connection: CollabConnection?
    private var token: String?
    private var outbox: [(seq: UInt64, sections: [Section])] = []
    private var nextSeq: UInt64 = 1
    private var attempt = 0
    private var leaving = false
    private let signingKey = Curve25519.Signing.PrivateKey()
    /// Updates the outbox may hold before the oldest are dropped (a resync
    /// on reconnect recovers them from the CRDT anyway).
    static let maxOutboxSections = 4096

    public init(invite: CollabInvite, displayName: String, deviceKind: String = "mac") {
        self.invite = invite
        self.displayName = displayName
        self.deviceKind = deviceKind
    }

    /// Where to try, in order: loopback when the invite was made on this
    /// machine (two instances on one Mac), the hub's addresses, then its
    /// Bonjour service.
    var endpoints: [NWEndpoint] {
        if let e = endpointsOverride { return e }
        guard let port = NWEndpoint.Port(rawValue: invite.port) else { return [] }
        var out: [NWEndpoint] = []
        if let h = invite.hostName, h == CollabGuest.localHostName {
            out.append(.hostPort(host: "127.0.0.1", port: port))
        }
        for a in invite.addresses { out.append(.hostPort(host: NWEndpoint.Host(a), port: port)) }
        out.append(.service(name: invite.sessionID, type: CollabTLS.serviceType, domain: "local.", interface: nil))
        return out
    }

    /// This machine's name as invites carry it.
    public static var localHostName: String { ProcessInfo.processInfo.hostName }

    public func connect() {
        leaving = false
        attemptConnect(endpoints, index: 0)
    }

    /// Leaves for good: `leave`, close, no reconnect.
    public func leave() {
        leaving = true
        connection?.send(.leave(.init(reason: "left")))
        connection?.closeAfterFlush(reason: "left")
        connection = nil
        session?.stopTimers()
        state = .ended("You left the session.")
    }

    private func attemptConnect(_ list: [NWEndpoint], index: Int) {
        guard !leaving else { return }
        guard index < list.count else { scheduleReconnect(); return }
        let params = CollabTLS.guestParameters(pinned: invite.fingerprint)
        let c = CollabConnection(NWConnection(to: list[index], using: params))
        connection = c
        var opened = false
        c.onReady = { [weak self, weak c] in
            guard let self, let c, self.connection === c else { return }
            opened = true
            self.attempt = 0
            self.sendJoin(on: c)
        }
        c.onMessage = { [weak self, weak c] msg in
            guard let self, let c, self.connection === c else { return }
            self.handle(msg, on: c)
        }
        c.onClosed = { [weak self, weak c] reason in
            guard let self, let c, self.connection === c else { return }
            self.connection = nil
            if case .ended = self.state { return }
            if !opened { self.attemptConnect(list, index: index + 1); return } // next endpoint
            self.scheduleReconnect(after: reason)
        }
        c.start(handshakeTimeout: 4)
    }

    private func scheduleReconnect(after reason: String? = nil) {
        guard !leaving else { return }
        if case .ended = state { return }
        // Before the first join_ack nothing is held: an unreachable hub on
        // the first try ends here, with the reason.
        guard session != nil else {
            state = .ended("Could not reach the host\(reason.map { " (\($0))" } ?? ""). Check that both Macs are on the same network and the invitation is current.")
            return
        }
        attempt += 1
        state = .reconnecting(attempt: attempt)
        let delay = min(30, 0.5 * pow(2, Double(attempt - 1)))
        DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self] in
            MainActor.assumeIsolated {
                guard let self, !self.leaving, self.connection == nil else { return }
                self.attemptConnect(self.endpoints, index: 0)
            }
        }
    }

    private func sendJoin(on c: CollabConnection) {
        let nonce = CollabInvite.randomBytes(24)
        let pub = signingKey.publicKey.rawRepresentation
        let proof = token == nil ? CollabInvite.proof(secret: invite.secret, nonce: nonce, guestPublicKey: pub) : ""
        c.send(.join(.init(inviteProof: proof, nonce: Base64URL.encode(nonce), guestPublicKey: Base64URL.encode(pub),
                           displayName: displayName, deviceKind: deviceKind, token: token)))
        if session == nil { state = .awaitingApproval }
    }

    private func handle(_ msg: CollabMessage, on c: CollabConnection) {
        switch msg {
        case let .joinAck(ack):
            guard let id = collabParseHex(ack.participantID) else { c.close(reason: "bad participant id"); return }
            token = ack.token
            joinAck = ack
            let first = session == nil
            if first {
                let s = CollabSession(role: .guest, replica: id, name: displayName, colourIndex: ack.colourIndex)
                s.canEdit = ack.role == "edit"
                s.transmit = { [weak self] out in self?.transmit(out) }
                session = s
                s.startTimers()
            }
            state = .joined
            guard let s = session else { return }
            c.send(.syncRequest(s.project.stateVectors))
            for item in outbox { c.send(.update(seq: item.seq, sections: item.sections)) }
            c.send(.awareness(s.localAwareness))
            if first { onJoined(s, ack) }
        case let .syncRequest(vectors):
            guard let s = session else { return }
            for chunk in CollabHub.chunks(s.project.diff(vectors)) { c.send(.syncReply(chunk)) }
        case let .syncReply(sections):
            receive(sections, on: c)
        case let .update(seq, sections):
            receive(sections, on: c)
            c.send(.ack(.init(through: seq)))
        case let .ack(a):
            outbox.removeAll { $0.seq <= a.through }
        case let .awareness(a):
            session?.receivePresence(a)
        case let .leave(l):
            leaving = true
            state = .ended(l.reason ?? "The host ended the session.")
            c.close(reason: "the host left")
            session?.stopTimers()
        case let .error(e):
            switch e.code {
            case "invite_invalid", "join_denied", "removed", "token_invalid", "session_full", "join_timeout":
                leaving = true
                state = .ended(e.message)
                session?.stopTimers()
            default: break
            }
        case .join:
            c.send(.error(.init(code: "unexpected", message: "join is the guest's")))
        case let .unknown(kind, _):
            c.send(.error(.init(code: "unknown_kind", message: "unknown message kind \(kind)")))
        case .blobWant, .blobChunk, .previewSubscribe, .previewFrame, .compileReport:
            break
        }
    }

    private func receive(_ sections: [Section], on c: CollabConnection) {
        guard let s = session else { return }
        let errors = s.integrate(sections)
        if errors.contains(where: { if case .pendingFull = $0 { return true } else { return false } }) {
            c.send(.syncRequest(s.project.stateVectors))
        }
    }

    private func transmit(_ out: CollabSession.Outbound) {
        switch out {
        case let .sections(sections):
            for chunk in CollabHub.chunks(sections) {
                let seq = nextSeq
                nextSeq += 1
                outbox.append((seq, chunk))
                if outbox.count > Self.maxOutboxSections { outbox.removeFirst(outbox.count - Self.maxOutboxSections) }
                if state == .joined { connection?.send(.update(seq: seq, sections: chunk)) }
            }
        case let .awareness(a):
            if state == .joined { connection?.send(.awareness(a)) }
        }
    }

    /// Updates sent and not yet acknowledged (tests).
    public var unacknowledged: Int { outbox.count }
}
