import Foundation

// collab-v1 framing and messages (docs/contracts/collab-v1.md §3-§4).
// Frame: u32 LE length (kind + body, 1 ... 2^24), u8 kind, body. Operation
// bodies are binary (LEB128 varints); control bodies are JSON objects.

// MARK: - Control payloads (JSON)

/// A participant id on the wire: 16 lowercase hex digits (JSON numbers
/// cannot carry 64 bits).
public typealias ParticipantHex = String

public enum CollabControl {
    public struct Join: Codable, Equatable, Sendable {
        /// base64url(HMAC-SHA256(invite secret, nonce ‖ guest public key)).
        public var inviteProof: String
        public var nonce: String
        public var guestPublicKey: String
        public var displayName: String
        /// "mac" or "ipad".
        public var deviceKind: String
        /// The long-term token from an earlier `join_ack`, on reconnect.
        public var token: String?

        enum CodingKeys: String, CodingKey {
            case inviteProof = "invite_proof", nonce, guestPublicKey = "guest_pubkey"
            case displayName = "display_name", deviceKind = "device_kind", token
        }

        public init(inviteProof: String, nonce: String, guestPublicKey: String, displayName: String,
                    deviceKind: String, token: String? = nil) {
            self.inviteProof = inviteProof; self.nonce = nonce; self.guestPublicKey = guestPublicKey
            self.displayName = displayName; self.deviceKind = deviceKind; self.token = token
        }
    }

    /// What every peer must compile with (proposal §5.3).
    public struct SessionPins: Codable, Equatable, Sendable {
        public var main: String
        public var jobName: String?
        public var sourceDateEpoch: Int64
        public var randomSeed: Int64
        /// "off" in sessions (Q4 default).
        public var shellEscape: String
        public var externalTools: Bool
        /// Read confinement to the project and the TeX trees (Q4 default).
        public var readConfinement: Bool

        enum CodingKeys: String, CodingKey {
            case main, jobName = "job_name", sourceDateEpoch = "source_date_epoch", randomSeed = "random_seed"
            case shellEscape = "shell_escape", externalTools = "external_tools", readConfinement = "read_confinement"
        }

        public init(main: String, jobName: String? = nil, sourceDateEpoch: Int64, randomSeed: Int64,
                    shellEscape: String = "off", externalTools: Bool = true, readConfinement: Bool = true) {
            self.main = main; self.jobName = jobName; self.sourceDateEpoch = sourceDateEpoch
            self.randomSeed = randomSeed; self.shellEscape = shellEscape; self.externalTools = externalTools
            self.readConfinement = readConfinement
        }
    }

    public struct JoinAck: Codable, Equatable, Sendable {
        public var participantID: ParticipantHex
        /// "edit" or "view".
        public var role: String
        public var token: String
        public var colourIndex: Int
        public var pins: SessionPins
        public var environmentDigest: String

        enum CodingKeys: String, CodingKey {
            case participantID = "participant_id", role, token, colourIndex = "colour_index", pins
            case environmentDigest = "environment_digest"
        }

        public init(participantID: ParticipantHex, role: String, token: String, colourIndex: Int,
                    pins: SessionPins, environmentDigest: String) {
            self.participantID = participantID; self.role = role; self.token = token
            self.colourIndex = colourIndex; self.pins = pins; self.environmentDigest = environmentDigest
        }
    }

    public struct Ack: Codable, Equatable, Sendable {
        /// Every `update` with `seq` ≤ this is applied and durable.
        public var through: UInt64
        public init(through: UInt64) { self.through = through }
    }

    /// A `RelativePosition` in JSON: no `replica`/`counter` means no anchor.
    public struct Position: Codable, Equatable, Sendable {
        public var replica: ParticipantHex?
        public var counter: UInt64?
        /// "before" or "after".
        public var assoc: String

        public init(_ rp: RelativePosition) {
            replica = rp.anchor.map { String(format: "%016llx", $0.replica) }
            counter = rp.anchor?.counter
            assoc = rp.assoc == .before ? "before" : "after"
        }

        public var relativePosition: RelativePosition? {
            let a: Assoc
            switch assoc {
            case "before": a = .before
            case "after": a = .after
            default: return nil
            }
            guard let r = replica, let c = counter else {
                return replica == nil && counter == nil ? RelativePosition(anchor: nil, assoc: a) : nil
            }
            guard r.utf8.count == 16, let v = UInt64(r, radix: 16) else { return nil }
            return RelativePosition(anchor: CollabID(replica: v, counter: c), assoc: a)
        }
    }

    /// Presence: ephemeral, never stored; last writer wins by `seq`.
    public struct Awareness: Codable, Equatable, Sendable {
        public var participantID: ParticipantHex
        public var name: String
        public var colourIndex: Int
        /// The file the participant is in (FileID hex), if any.
        public var file: String?
        public var anchor: Position?
        public var head: Position?
        public var previewPage: Int?
        public var following: ParticipantHex?
        public var seq: UInt64

        enum CodingKeys: String, CodingKey {
            case participantID = "participant_id", name, colourIndex = "colour_index", file, anchor, head
            case previewPage = "preview_page", following, seq
        }

        public init(participantID: ParticipantHex, name: String, colourIndex: Int, file: String?,
                    anchor: Position?, head: Position?, previewPage: Int?, following: ParticipantHex?, seq: UInt64) {
            self.participantID = participantID; self.name = name; self.colourIndex = colourIndex
            self.file = file; self.anchor = anchor; self.head = head; self.previewPage = previewPage
            self.following = following; self.seq = seq
        }
    }

    public struct BlobWant: Codable, Equatable, Sendable {
        public var sha256: String
        public var offset: UInt64
        public var length: UInt64
        public init(sha256: String, offset: UInt64, length: UInt64) {
            self.sha256 = sha256; self.offset = offset; self.length = length
        }
    }

    public struct PreviewSubscribe: Codable, Equatable, Sendable {
        /// The Mac whose preview to relay; nil: the hub.
        public var source: ParticipantHex?
        /// Font resources already held, so they are not sent again.
        public var haveFonts: [String]

        enum CodingKeys: String, CodingKey { case source, haveFonts = "have_fonts" }

        public init(source: ParticipantHex?, haveFonts: [String]) { self.source = source; self.haveFonts = haveFonts }
    }

    public struct CompileReport: Codable, Equatable, Sendable {
        public var stateVectorDigest: String
        public var readSetDigest: String
        public var pageHashes: [String]

        enum CodingKeys: String, CodingKey {
            case stateVectorDigest = "state_vector_digest", readSetDigest = "read_set_digest"
            case pageHashes = "page_hashes"
        }

        public init(stateVectorDigest: String, readSetDigest: String, pageHashes: [String]) {
            self.stateVectorDigest = stateVectorDigest; self.readSetDigest = readSetDigest
            self.pageHashes = pageHashes
        }
    }

    public struct Leave: Codable, Equatable, Sendable {
        public var reason: String?
        public init(reason: String?) { self.reason = reason }
    }

    public struct ErrorMessage: Codable, Equatable, Sendable {
        public var code: String
        public var message: String
        public init(code: String, message: String) { self.code = code; self.message = message }
    }
}

// MARK: - Messages

public enum CollabMessage: Equatable, Sendable {
    case join(CollabControl.Join)
    case joinAck(CollabControl.JoinAck)
    case syncRequest([DocVector])
    case syncReply([Section])
    case update(seq: UInt64, sections: [Section])
    case ack(CollabControl.Ack)
    case awareness(CollabControl.Awareness)
    case blobWant(CollabControl.BlobWant)
    case blobChunk(sha256: [UInt8], offset: UInt64, data: [UInt8])
    case previewSubscribe(CollabControl.PreviewSubscribe)
    /// One display-list-v3 frame, verbatim.
    case previewFrame([UInt8])
    case compileReport(CollabControl.CompileReport)
    case leave(CollabControl.Leave)
    case error(CollabControl.ErrorMessage)
    /// A kind this version does not know: answer `error`, do not drop the
    /// connection.
    case unknown(kind: UInt8, body: [UInt8])
}

public enum CollabWireError: Error, Equatable {
    case emptyFrame, frameTooLarge, truncated, badVarint, badUTF8, trailingBytes
    case badTag(UInt8)
    case invalid(String)
    case badJSON(kind: UInt8)
}

public enum CollabWire {
    public static let maxFrame = 1 << 24

    public enum Kind {
        public static let join: UInt8 = 0x01
        public static let joinAck: UInt8 = 0x02
        public static let syncRequest: UInt8 = 0x03
        public static let syncReply: UInt8 = 0x04
        public static let update: UInt8 = 0x05
        public static let ack: UInt8 = 0x06
        public static let awareness: UInt8 = 0x07
        public static let blobWant: UInt8 = 0x08
        public static let blobChunk: UInt8 = 0x09
        public static let previewSubscribe: UInt8 = 0x0A
        public static let previewFrame: UInt8 = 0x0B
        public static let compileReport: UInt8 = 0x0C
        public static let leave: UInt8 = 0x0D
        public static let error: UInt8 = 0x0E
    }

    // MARK: Writer

    struct Writer {
        var out: [UInt8] = []

        mutating func u8(_ v: UInt8) { out.append(v) }
        mutating func varint(_ v: UInt64) {
            var v = v
            while v >= 0x80 { out.append(UInt8(v & 0x7F) | 0x80); v >>= 7 }
            out.append(UInt8(v))
        }
        mutating func bytes(_ b: [UInt8]) { varint(UInt64(b.count)); out += b }
        mutating func string(_ s: String) { bytes(Array(s.utf8)) }
        mutating func id(_ id: CollabID) { varint(id.replica); varint(id.counter) }
        mutating func optID(_ id: CollabID?) {
            if let id { u8(1); self.id(id) } else { u8(0) }
        }
        mutating func doc(_ d: DocRef) {
            switch d {
            case .fileMap: u8(0)
            case let .text(f): u8(1); out += f.bytes
            }
        }
        mutating func textOp(_ op: TextOp) {
            switch op {
            case let .insert(id, l, r, content):
                u8(1); self.id(id); optID(l); optID(r); string(content)
            case let .delete(id, target, n):
                u8(2); self.id(id); self.id(target); varint(n)
            }
        }
        mutating func fileOp(_ op: FileOp) {
            id(op.id); varint(op.lamport); out += op.file.bytes
            switch op.kind {
            case let .create(kind, path): u8(1); u8(kind.rawValue); string(path)
            case let .setPath(p): u8(2); string(p)
            case let .setBlob(b): u8(3); out += b.sha256; varint(b.bytes); string(b.mediaType)
            case let .setDeleted(d): u8(4); u8(d ? 1 : 0)
            }
        }
        mutating func sections(_ s: [Section]) {
            varint(UInt64(s.count))
            for sec in s {
                switch sec {
                case let .fileMap(ops):
                    doc(.fileMap); varint(UInt64(ops.count)); ops.forEach { fileOp($0) }
                case let .text(f, ops):
                    doc(.text(f)); varint(UInt64(ops.count)); ops.forEach { textOp($0) }
                }
            }
        }
    }

    static func json<T: Encodable>(_ v: T) -> [UInt8] {
        let e = JSONEncoder()
        e.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        return Array((try? e.encode(v)) ?? Data("{}".utf8))
    }

    /// The canonical bytes of one file op (duplicate verification).
    static func encode(_ op: FileOp) -> [UInt8] {
        var w = Writer()
        w.fileOp(op)
        return w.out
    }

    /// One complete frame.
    public static func encode(_ msg: CollabMessage) -> [UInt8] {
        var w = Writer()
        let kind: UInt8
        switch msg {
        case let .join(j): kind = Kind.join; w.out = json(j)
        case let .joinAck(j): kind = Kind.joinAck; w.out = json(j)
        case let .syncRequest(vs):
            kind = Kind.syncRequest
            w.varint(UInt64(vs.count))
            for v in vs {
                w.doc(v.doc)
                let entries = v.vector.sorted
                w.varint(UInt64(entries.count))
                for e in entries { w.varint(e.replica); w.varint(e.next) }
            }
        case let .syncReply(s): kind = Kind.syncReply; w.sections(s)
        case let .update(seq, s): kind = Kind.update; w.varint(seq); w.sections(s)
        case let .ack(a): kind = Kind.ack; w.out = json(a)
        case let .awareness(a): kind = Kind.awareness; w.out = json(a)
        case let .blobWant(b): kind = Kind.blobWant; w.out = json(b)
        case let .blobChunk(sha, offset, data):
            kind = Kind.blobChunk; w.out += sha; w.varint(offset); w.bytes(data)
        case let .previewSubscribe(p): kind = Kind.previewSubscribe; w.out = json(p)
        case let .previewFrame(f): kind = Kind.previewFrame; w.out = f
        case let .compileReport(c): kind = Kind.compileReport; w.out = json(c)
        case let .leave(l): kind = Kind.leave; w.out = json(l)
        case let .error(e): kind = Kind.error; w.out = json(e)
        case let .unknown(k, body): kind = k; w.out = body
        }
        let len = UInt32(w.out.count + 1)
        var frame: [UInt8] = [UInt8(len & 0xFF), UInt8(len >> 8 & 0xFF), UInt8(len >> 16 & 0xFF), UInt8(len >> 24), kind]
        frame += w.out
        return frame
    }

    // MARK: Reader

    struct Reader {
        let b: [UInt8]
        var i: Int
        let end: Int

        mutating func u8() throws -> UInt8 {
            guard i < end else { throw CollabWireError.truncated }
            defer { i += 1 }
            return b[i]
        }
        mutating func take(_ n: Int) throws -> [UInt8] {
            guard end - i >= n else { throw CollabWireError.truncated }
            defer { i += n }
            return Array(b[i..<i + n])
        }
        mutating func varint() throws -> UInt64 {
            var v: UInt64 = 0
            var shift: UInt64 = 0
            while shift < 70 {
                let byte = try u8()
                if shift == 63 && byte > 1 { throw CollabWireError.badVarint }
                v |= UInt64(byte & 0x7F) << shift
                if byte & 0x80 == 0 {
                    if byte == 0 && shift > 0 { throw CollabWireError.badVarint }
                    return v
                }
                shift += 7
            }
            throw CollabWireError.badVarint
        }
        mutating func count() throws -> Int {
            let n = try varint()
            guard n <= UInt64(end - i) else { throw CollabWireError.truncated }
            return Int(n)
        }
        mutating func string() throws -> String {
            let raw = try take(count())
            // Decoding repairs invalid UTF-8 with U+FFFD; a repair changes
            // the bytes, so a round trip that differs means invalid input.
            let s = String(decoding: raw, as: UTF8.self)
            guard s.utf8.elementsEqual(raw) else { throw CollabWireError.badUTF8 }
            return s
        }
        mutating func id() throws -> CollabID { CollabID(replica: try varint(), counter: try varint()) }
        mutating func optID() throws -> CollabID? {
            switch try u8() {
            case 0: return nil
            case 1: return try id()
            case let t: throw CollabWireError.badTag(t)
            }
        }
        mutating func fileID() throws -> FileID { FileID(bytes: try take(16)) }
        mutating func doc() throws -> DocRef {
            switch try u8() {
            case 0: return .fileMap
            case 1: return .text(try fileID())
            case let t: throw CollabWireError.badTag(t)
            }
        }
        mutating func textOp() throws -> TextOp {
            switch try u8() {
            case 1:
                let id = try self.id(), l = try optID(), r = try optID(), content = try string()
                if content.isEmpty { throw CollabWireError.invalid("empty insert") }
                return .insert(id: id, originLeft: l, originRight: r, content: content)
            case 2:
                let id = try self.id(), target = try self.id(), n = try varint()
                if n == 0 { throw CollabWireError.invalid("empty delete") }
                return .delete(id: id, target: target, length: n)
            case let t: throw CollabWireError.badTag(t)
            }
        }
        mutating func fileOp() throws -> FileOp {
            let id = try self.id(), lamport = try varint(), file = try fileID()
            let kind: FileOpKind
            switch try u8() {
            case 1:
                guard let k = FileKind(rawValue: try u8()) else { throw CollabWireError.badTag(b[i - 1]) }
                kind = .create(kind: k, path: try string())
            case 2: kind = .setPath(try string())
            case 3:
                let sha = try take(32), n = try varint(), mt = try string()
                kind = .setBlob(BlobRef(sha256: sha, bytes: n, mediaType: mt))
            case 4:
                switch try u8() {
                case 0: kind = .setDeleted(false)
                case 1: kind = .setDeleted(true)
                case let t: throw CollabWireError.badTag(t)
                }
            case let t: throw CollabWireError.badTag(t)
            }
            return FileOp(id: id, lamport: lamport, file: file, kind: kind)
        }
        mutating func sections() throws -> [Section] {
            let n = try count()
            var out: [Section] = []
            for _ in 0..<n {
                switch try doc() {
                case .fileMap:
                    var ops: [FileOp] = []
                    for _ in 0..<(try count()) { ops.append(try fileOp()) }
                    out.append(.fileMap(ops))
                case let .text(f):
                    var ops: [TextOp] = []
                    for _ in 0..<(try count()) { ops.append(try textOp()) }
                    out.append(.text(f, ops))
                }
            }
            return out
        }
    }

    static func unjson<T: Decodable>(_ t: T.Type, _ body: [UInt8], kind: UInt8) throws -> T {
        do { return try JSONDecoder().decode(t, from: Data(body)) } catch { throw CollabWireError.badJSON(kind: kind) }
    }

    /// Decode one frame from the start of `buf`: nil if it has not all
    /// arrived, else the message and the bytes it used.
    public static func decode(_ buf: [UInt8]) throws -> (CollabMessage, Int)? {
        guard buf.count >= 4 else { return nil }
        let len = Int(buf[0]) | Int(buf[1]) << 8 | Int(buf[2]) << 16 | Int(buf[3]) << 24
        if len == 0 { throw CollabWireError.emptyFrame }
        if len > maxFrame { throw CollabWireError.frameTooLarge }
        guard buf.count >= 4 + len else { return nil }
        let kind = buf[4]
        var r = Reader(b: buf, i: 5, end: 4 + len)
        let body = Array(buf[5..<4 + len])
        let msg: CollabMessage
        switch kind {
        case Kind.syncRequest:
            var vs: [DocVector] = []
            for _ in 0..<(try r.count()) {
                let d = try r.doc()
                var sv = StateVector()
                for _ in 0..<(try r.count()) {
                    let replica = try r.varint()
                    sv[replica] = try r.varint()
                }
                vs.append(DocVector(doc: d, vector: sv))
            }
            msg = .syncRequest(vs)
        case Kind.syncReply: msg = .syncReply(try r.sections())
        case Kind.update:
            let seq = try r.varint()
            msg = .update(seq: seq, sections: try r.sections())
        case Kind.blobChunk:
            let sha = try r.take(32), offset = try r.varint(), data = try r.take(try r.count())
            msg = .blobChunk(sha256: sha, offset: offset, data: data)
        case Kind.previewFrame: r.i = r.end; msg = .previewFrame(body)
        case Kind.join: r.i = r.end; msg = .join(try unjson(CollabControl.Join.self, body, kind: kind))
        case Kind.joinAck: r.i = r.end; msg = .joinAck(try unjson(CollabControl.JoinAck.self, body, kind: kind))
        case Kind.ack: r.i = r.end; msg = .ack(try unjson(CollabControl.Ack.self, body, kind: kind))
        case Kind.awareness: r.i = r.end; msg = .awareness(try unjson(CollabControl.Awareness.self, body, kind: kind))
        case Kind.blobWant: r.i = r.end; msg = .blobWant(try unjson(CollabControl.BlobWant.self, body, kind: kind))
        case Kind.previewSubscribe:
            r.i = r.end; msg = .previewSubscribe(try unjson(CollabControl.PreviewSubscribe.self, body, kind: kind))
        case Kind.compileReport:
            r.i = r.end; msg = .compileReport(try unjson(CollabControl.CompileReport.self, body, kind: kind))
        case Kind.leave: r.i = r.end; msg = .leave(try unjson(CollabControl.Leave.self, body, kind: kind))
        case Kind.error: r.i = r.end; msg = .error(try unjson(CollabControl.ErrorMessage.self, body, kind: kind))
        default: r.i = r.end; msg = .unknown(kind: kind, body: body)
        }
        guard r.i == r.end else { throw CollabWireError.trailingBytes }
        return (msg, 4 + len)
    }
}
