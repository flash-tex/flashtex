import FlashTeXCollabCore
import Foundation
import Network

/// The inviting Mac (proposal §3): a TLS 1.3 listener on
/// `_flashtex-collab._tcp`, single-use invites, the hub user's approval of
/// each joiner, and a star relay. The hub is the durable owner but not a
/// sequencer: it integrates and forwards operations; it never orders them.
///
/// Replica binding (collab-v1 §2.1): each connection is bound to the
/// replica id the hub assigned it, and every operation it sends whose id
/// names another replica (or any operation from a viewer) is dropped
/// before the CRDT. Guests accept relayed operations only from the hub,
/// their one connection.
@MainActor
public final class CollabHub {
    public struct JoinRequest: Equatable, Sendable {
        public var displayName: String
        public var deviceKind: String
    }

    public enum Decision: Equatable, Sendable { case allowEdit, allowView, deny }

    public enum Event: Equatable, Sendable {
        case ready(port: UInt16)
        case failed(String)
        case joined(participant: UInt64, name: String)
        case reconnected(participant: UInt64)
        case left(participant: UInt64, reason: String)
        case refused(reason: String)
        /// Operations dropped by replica binding (count).
        case forgedDropped(participant: UInt64, count: Int)
        /// File-map operations from a guest, all dropped: in P1 only the hub
        /// creates, renames or deletes files.
        case fileOpsDropped(participant: UInt64, count: Int)
    }

    /// Caps, as nearby-v1's table.
    public struct Limits: Sendable {
        public var maxConnections = 12
        /// Connections from one address at a time (two instances on one Mac
        /// share 127.0.0.1, and a reconnect may overlap its predecessor).
        public var maxConnectionsPerAddress = 4
        /// Participants, the hub included, joined or waiting for approval.
        public var maxParticipants = 5
        public var joinTimeout: TimeInterval = 10
        public var approvalTimeout: TimeInterval = 120
        /// Largest frame before a connection has joined (a `join` is ~300 B).
        public var preJoinFrameBytes = 4096
        /// Awareness messages a participant may send per second; the excess
        /// is dropped, never fanned out.
        public var awarenessPerSecond: Double = 25
        public init() {}
    }

    struct Member {
        var id: UInt64
        var name: String
        var deviceKind: String
        var colourIndex: Int
        var canEdit: Bool
        var token: String
        var link: ObjectIdentifier?
        var removed = false
    }

    @MainActor final class Link {
        let connection: CollabConnection
        let address: String
        var member: UInt64?
        var awaitingApproval = false
        var outSeq: UInt64 = 0
        /// Token bucket for awareness.
        var awarenessTokens: Double = 0
        var awarenessStamp = Date()
        init(_ c: CollabConnection) {
            connection = c
            address = c.remoteAddress
        }
    }

    public let session: CollabSession
    public let identity: CollabIdentity
    public let sessionID: String
    public var projectName: String
    public var pins: CollabControl.SessionPins
    public var environmentDigest: String
    public var limits = Limits()
    /// Ask the hub user; call the reply exactly once (any time).
    public var approve: (JoinRequest, @escaping (Decision) -> Void) -> Void = { _, reply in reply(.deny) }
    public var onEvent: (Event) -> Void = { _ in }
    /// How long a new invitation stays valid (tests shorten it).
    public var inviteLifetime: TimeInterval = CollabInvite.lifetime

    private var listener: NWListener?
    public private(set) var port: UInt16?
    private var invites: [Data: Date] = [:]
    private var links: [ObjectIdentifier: Link] = [:]
    private var members: [UInt64: Member] = [:]
    private var nextColour = 1

    public init(session: CollabSession, identity: CollabIdentity, sessionID: String = collabHex(UInt64.random(in: 1...UInt64.max)),
                projectName: String, pins: CollabControl.SessionPins, environmentDigest: String) {
        precondition(session.role == .hub)
        self.session = session
        self.identity = identity
        self.sessionID = sessionID
        self.projectName = projectName
        self.pins = pins
        self.environmentDigest = environmentDigest
        session.transmit = { [weak self] out in self?.broadcast(out) }
    }

    // MARK: Lifecycle

    /// Starts listening (any port unless given). `loopbackOnly` refuses
    /// every interface but loopback (tests). `advertise` registers the
    /// Bonjour service named by the session id.
    public func start(port requested: UInt16? = nil, loopbackOnly: Bool = false, advertise: Bool = true) throws {
        let params = CollabTLS.hubParameters(identity: identity, loopbackOnly: loopbackOnly)
        let nwPort = requested.flatMap { NWEndpoint.Port(rawValue: $0) } ?? .any
        let l = try NWListener(using: params, on: nwPort)
        if advertise {
            l.service = NWListener.Service(name: sessionID, type: CollabTLS.serviceType, domain: nil,
                                           txtRecord: NWTXTRecord(["v": "1"])) // no project name: the LAN sees only that a session exists
        }
        l.stateUpdateHandler = { [weak self] state in
            MainActor.assumeIsolated {
                guard let self else { return }
                switch state {
                case .ready:
                    let p = self.listener?.port?.rawValue ?? 0
                    self.port = p
                    self.onEvent(.ready(port: p))
                case let .failed(error): self.onEvent(.failed("\(error)"))
                default: break
                }
            }
        }
        l.newConnectionHandler = { [weak self] nw in
            MainActor.assumeIsolated {
                guard let self else { nw.cancel(); return }
                self.accept(nw)
            }
        }
        listener = l
        session.startTimers()
        l.start(queue: .main)
    }

    /// Ends the session for everyone: `leave`, then close.
    public func stop() {
        for link in links.values {
            link.connection.send(.leave(.init(reason: "the host ended the session")))
            link.connection.closeAfterFlush(reason: "session ended")
        }
        links = [:]
        listener?.cancel()
        listener = nil
        session.stopTimers()
    }

    // MARK: Invites

    /// A new single-use invite, valid for `CollabInvite.lifetime`.
    public func makeInvite(addresses: [String], hostName: String?) -> CollabInvite {
        let now = Date()
        invites = invites.filter { $0.value > now }
        let secret = CollabInvite.randomBytes(32)
        invites[secret] = now.addingTimeInterval(inviteLifetime)
        return CollabInvite(sessionID: sessionID, secret: secret, fingerprint: identity.fingerprint, projectName: projectName,
                            port: port ?? 0, addresses: addresses, hostName: hostName)
    }

    /// Unused invites still valid.
    public var openInviteCount: Int {
        let now = Date()
        return invites.values.filter { $0 > now }.count
    }

    /// Removes a participant: its token stops working and its connection
    /// closes. Its edits stay (proposal §3.2).
    public func remove(_ participant: UInt64) {
        guard var m = members[participant] else { return }
        m.removed = true
        members[participant] = m
        if let k = m.link, let link = links[k] {
            link.connection.refuse(code: "removed", message: "The host removed you from the session.")
        }
        session.dropPresence(participant)
    }

    public struct MemberInfo: Equatable, Sendable {
        public var id: UInt64
        public var name: String
        public var colourIndex: Int
        public var canEdit: Bool
        public var connected: Bool
    }

    public var memberList: [MemberInfo] {
        members.values.filter { !$0.removed }.sorted { $0.colourIndex < $1.colourIndex }.map {
            MemberInfo(id: $0.id, name: $0.name, colourIndex: $0.colourIndex, canEdit: $0.canEdit, connected: $0.link != nil)
        }
    }

    // MARK: Connections

    private func accept(_ nw: NWConnection) {
        guard links.count < limits.maxConnections else {
            nw.cancel()
            onEvent(.refused(reason: "too many connections"))
            return
        }
        let c = CollabConnection(nw)
        c.maxFrameBytes = limits.preJoinFrameBytes
        let link = Link(c)
        guard links.values.filter({ $0.address == link.address }).count < limits.maxConnectionsPerAddress else {
            nw.cancel()
            onEvent(.refused(reason: "too many connections from \(link.address)"))
            return
        }
        let key = ObjectIdentifier(link)
        links[key] = link
        c.onReady = { [weak self, weak c] in
            guard let self, let c else { return }
            guard CollabTLS.negotiatedVersion(c.nw) == .TLSv13 else { c.close(reason: "not TLS 1.3"); return }
            c.arm(after: self.limits.joinTimeout) { [weak c] in
                c?.refuse(code: "join_timeout", message: "no join within the time allowed")
            }
        }
        c.onMessage = { [weak self] msg in self?.handle(msg, on: key) }
        c.onClosed = { [weak self] reason in self?.closed(key, reason: reason) }
        c.start()
    }

    private func closed(_ key: ObjectIdentifier, reason: String) {
        guard let link = links.removeValue(forKey: key) else { return }
        if let id = link.member, var m = members[id], m.link == key {
            m.link = nil
            members[id] = m
            session.dropPresence(id)
            onEvent(.left(participant: id, reason: reason))
        }
    }

    private func handle(_ msg: CollabMessage, on key: ObjectIdentifier) {
        guard let link = links[key] else { return }
        guard let id = link.member, let member = members[id], !member.removed else {
            if case let .join(j) = msg, !link.awaitingApproval {
                join(j, link: link, key: key)
            } else if case .leave = msg {
                link.connection.close(reason: "left before joining")
            } else {
                link.connection.refuse(code: "not_joined", message: "send join first")
            }
            return
        }
        switch msg {
        case let .syncRequest(vectors):
            for chunk in Self.chunks(session.project.diff(vectors)) { link.connection.send(.syncReply(chunk)) }
        case let .syncReply(sections):
            receive(sections, from: member, link: link)
        case let .update(seq, sections):
            receive(sections, from: member, link: link)
            link.connection.send(.ack(.init(through: seq)))
        case let .awareness(a):
            // Token bucket: presence is cheap but fanned out to everyone.
            let now = Date()
            link.awarenessTokens = min(limits.awarenessPerSecond,
                                       link.awarenessTokens + now.timeIntervalSince(link.awarenessStamp) * limits.awarenessPerSecond)
            link.awarenessStamp = now
            guard link.awarenessTokens >= 1 else { return }
            link.awarenessTokens -= 1
            var v = a
            v.participantID = collabHex(id)
            v.name = member.name
            v.colourIndex = member.colourIndex
            session.receivePresence(v)
            for (k, other) in links where k != key && other.member != nil { other.connection.send(.awareness(v)) }
        case .ack, .compileReport, .blobWant, .blobChunk, .previewSubscribe, .previewFrame:
            break // not used by P1; harmless
        case .leave:
            link.connection.close(reason: "left")
        case .error:
            break
        case .join:
            link.connection.send(.error(.init(code: "already_joined", message: "this connection has joined")))
        case .joinAck:
            link.connection.refuse(code: "unexpected", message: "join_ack is the hub's")
        case let .unknown(kind, _):
            link.connection.send(.error(.init(code: "unknown_kind", message: "unknown message kind \(kind)")))
        }
    }

    private func join(_ j: CollabControl.Join, link: Link, key: ObjectIdentifier) {
        link.connection.disarm()
        // Reconnect with the token the hub issued.
        if let token = j.token, !token.isEmpty {
            guard let (id, m) = members.first(where: { $0.value.token == token && !$0.value.removed }) else {
                link.connection.refuse(code: "token_invalid", message: "This session no longer recognises you; ask for a new invitation.")
                onEvent(.refused(reason: "unknown token"))
                return
            }
            if let old = m.link, old != key, let oldLink = links[old] { oldLink.connection.close(reason: "replaced by a reconnect") }
            admit(id, link: link, key: key, reconnect: true)
            return
        }
        guard let nonce = Base64URL.decode(j.nonce), nonce.count >= 16, let pub = Base64URL.decode(j.guestPublicKey) else {
            link.connection.refuse(code: "invite_invalid", message: "malformed join")
            return
        }
        let now = Date()
        invites = invites.filter { $0.value > now }
        let proof = Data(j.inviteProof.utf8)
        guard let secret = invites.keys.first(where: {
            CollabTLS.constantTimeEqual(Data(CollabInvite.proof(secret: $0, nonce: nonce, guestPublicKey: pub).utf8), proof)
        }) else {
            link.connection.refuse(code: "invite_invalid", message: "This invitation is not valid: it was used already, has expired, or belongs to another session.")
            onEvent(.refused(reason: "invalid invite proof"))
            return
        }
        invites.removeValue(forKey: secret) // single use, whatever the host decides
        let pending = links.values.filter(\.awaitingApproval).count
        guard members.values.filter({ !$0.removed }).count + pending < limits.maxParticipants - 1 else {
            link.connection.refuse(code: "session_full", message: "The session is full.")
            return
        }
        let request = JoinRequest(displayName: String(j.displayName.prefix(64)), deviceKind: String(j.deviceKind.prefix(16)))
        link.awaitingApproval = true
        link.connection.arm(after: limits.approvalTimeout) { [weak link] in
            link?.connection.refuse(code: "join_timeout", message: "The host did not answer.")
        }
        var answered = false
        approve(request) { [weak self, weak link] decision in
            guard let self, let link, !answered, !link.connection.isClosed else { return }
            answered = true
            link.awaitingApproval = false
            link.connection.disarm()
            guard decision != .deny else {
                link.connection.refuse(code: "join_denied", message: "The host declined your request.")
                return
            }
            var id: UInt64
            repeat { id = UInt64.random(in: 1...UInt64.max) } while id == self.session.replica || self.members[id] != nil
            let colour = self.nextColour % 8
            self.nextColour += 1
            self.members[id] = Member(id: id, name: request.displayName, deviceKind: request.deviceKind, colourIndex: colour,
                                      canEdit: decision == .allowEdit,
                                      token: Base64URL.encode(CollabInvite.randomBytes(32)))
            self.admit(id, link: link, key: key, reconnect: false)
        }
    }

    private func admit(_ id: UInt64, link: Link, key: ObjectIdentifier, reconnect: Bool) {
        guard var m = members[id] else { return }
        m.link = key
        members[id] = m
        link.member = id
        link.connection.maxFrameBytes = CollabWire.maxFrame
        link.awarenessTokens = limits.awarenessPerSecond
        link.connection.send(.joinAck(.init(participantID: collabHex(id), role: m.canEdit ? "edit" : "view", token: m.token,
                                            colourIndex: m.colourIndex, pins: pins, environmentDigest: environmentDigest)))
        // Both directions: the guest asks for what it lacks; the hub asks
        // for anything the guest made offline.
        link.connection.send(.syncRequest(session.project.stateVectors))
        link.connection.send(.awareness(session.localAwareness))
        onEvent(reconnect ? .reconnected(participant: id) : .joined(participant: id, name: m.name))
    }

    /// Replica binding, then integrate and relay.
    private func receive(_ sections: [Section], from member: Member, link: Link) {
        var kept: [Section] = []
        var dropped = 0
        var fileOps = 0
        for s in sections {
            switch s {
            case let .fileMap(ops):
                // P1 shares no creates, renames or deletes: a guest's file-map
                // operation never reaches the CRDT (or the host's disk).
                fileOps += ops.count
            case let .text(f, ops):
                let ok = member.canEdit ? ops.filter { $0.id.replica == member.id } : []
                dropped += ops.count - ok.count
                if !ok.isEmpty { kept.append(.text(f, ok)) }
            }
        }
        if dropped > 0 { onEvent(.forgedDropped(participant: member.id, count: dropped)) }
        if fileOps > 0 { onEvent(.fileOpsDropped(participant: member.id, count: fileOps)) }
        guard !kept.isEmpty else { return }
        let errors = session.integrate(kept)
        for (k, other) in links where other !== link && other.member != nil {
            _ = k
            other.outSeq += 1
            other.connection.send(.update(seq: other.outSeq, sections: kept))
        }
        if errors.contains(where: { if case .pendingFull = $0 { return true } else { return false } }) {
            link.connection.send(.syncRequest(session.project.stateVectors))
        }
    }

    private func broadcast(_ out: CollabSession.Outbound) {
        for link in links.values where link.member != nil {
            switch out {
            case let .sections(s):
                for chunk in Self.chunks(s) {
                    link.outSeq += 1
                    link.connection.send(.update(seq: link.outSeq, sections: chunk))
                }
            case let .awareness(a):
                link.connection.send(.awareness(a))
            }
        }
    }

    /// Splits sections into frames of about `budget` bytes (a large join
    /// sync must stay far below the 16 MiB frame cap).
    static func chunks(_ sections: [Section], budget: Int = 1 << 20) -> [[Section]] {
        var out: [[Section]] = []
        var cur: [Section] = []
        var size = 0
        func push(_ s: Section, _ w: Int) {
            if size + w > budget, !cur.isEmpty { out.append(cur); cur = []; size = 0 }
            if case let .text(f, ops) = s, case let .text(g, prev)? = cur.last, f == g {
                cur[cur.count - 1] = .text(f, prev + ops)
            } else if case let .fileMap(ops) = s, case let .fileMap(prev)? = cur.last {
                cur[cur.count - 1] = .fileMap(prev + ops)
            } else {
                cur.append(s)
            }
            size += w
        }
        for s in sections {
            switch s {
            case let .fileMap(ops):
                for op in ops { push(.fileMap([op]), 64) }
            case let .text(f, ops):
                for op in ops {
                    if case let .insert(_, _, _, content) = op { push(.text(f, [op]), 48 + content.utf8.count) } else { push(.text(f, [op]), 48) }
                }
            }
        }
        if !cur.isEmpty { out.append(cur) }
        return out
    }
}
