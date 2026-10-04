import Foundation

/// A file's stable identity: 128 random bits, never a path (proposal §2.6).
public struct FileID: Hashable, Comparable, Sendable, CustomStringConvertible {
    public let bytes: [UInt8]

    public init(bytes: [UInt8]) {
        precondition(bytes.count == 16, "a FileID is 16 bytes")
        self.bytes = bytes
    }

    public static func random<G: RandomNumberGenerator>(using g: inout G) -> FileID {
        var b: [UInt8] = []
        for _ in 0..<2 {
            var v = g.next()
            for _ in 0..<8 { b.append(UInt8(truncatingIfNeeded: v)); v >>= 8 }
        }
        return FileID(bytes: b)
    }

    public static func random() -> FileID {
        var g = SystemRandomNumberGenerator()
        return random(using: &g)
    }

    public static func < (a: FileID, b: FileID) -> Bool { a.bytes.lexicographicallyPrecedes(b.bytes) }

    public var hex: String { bytes.map { String(format: "%02x", $0) }.joined() }
    public var description: String { hex }
}

public enum FileKind: UInt8, Sendable { case text = 0, blob = 1 }

/// A content-addressed binary file version.
public struct BlobRef: Equatable, Sendable {
    public var sha256: [UInt8]
    public var bytes: UInt64
    public var mediaType: String

    public init(sha256: [UInt8], bytes: UInt64, mediaType: String) {
        precondition(sha256.count == 32)
        self.sha256 = sha256
        self.bytes = bytes
        self.mediaType = mediaType
    }
}

public enum FileOpKind: Equatable, Sendable {
    case create(kind: FileKind, path: String)
    case setPath(String)
    case setBlob(BlobRef)
    case setDeleted(Bool)
}

/// One file-map change; consumes one counter of its replica.
public struct FileOp: Equatable, Sendable {
    public var id: CollabID
    public var lamport: UInt64
    public var file: FileID
    public var kind: FileOpKind

    public init(id: CollabID, lamport: UInt64, file: FileID, kind: FileOpKind) {
        self.id = id
        self.lamport = lamport
        self.file = file
        self.kind = kind
    }
}

/// A live file as the user sees it.
public struct FileEntryView: Equatable, Sendable {
    public var file: FileID
    /// Where it is materialised: its own path, or a conflict name.
    public var path: String
    public var requestedPath: String
    public var kind: FileKind
    public var blob: BlobRef?
    public var conflict: Bool
}

/// The project's file tree: last-writer-wins registers per `FileID`,
/// stamped (lamport, replica) (contract §2.6).
public final class FileMap {
    struct Stamp: Comparable {
        var lamport: UInt64
        var replica: UInt64
        static func < (a: Stamp, b: Stamp) -> Bool { (a.lamport, a.replica) < (b.lamport, b.replica) }
    }

    struct Entry {
        var kind: FileKind
        var path: (Stamp, String)
        var blob: (Stamp, BlobRef)?
        var deleted: (Stamp, Bool)
    }

    public let replica: UInt64
    public private(set) var lamport: UInt64 = 0
    public private(set) var stateVector = StateVector()
    var entries: [FileID: Entry] = [:]
    var ops: [CollabID: FileOp] = [:]
    var log: [FileOp] = []

    public init(replica: UInt64) { self.replica = replica }

    /// Contract §2.6, over UTF-8 bytes. Never over Characters: a combining
    /// mark after `/` or `\` makes one grapheme of the pair, which would
    /// hide the separator (`"../\u{301}etc"` would pass as one segment).
    public static func isValidPath(_ path: String) -> Bool {
        let b = Array(path.utf8)
        guard !b.isEmpty, b.count <= CollabLimits.maxPathBytes,
              !b.contains(UInt8(ascii: "\\")), !b.contains(0) else { return false }
        let dot = UInt8(ascii: ".")
        return b.split(separator: UInt8(ascii: "/"), omittingEmptySubsequences: false).allSatisfy { seg in
            !seg.isEmpty && !(seg.count == 1 && seg.first == dot) && !(seg.count == 2 && seg.allSatisfy { $0 == dot })
        }
    }

    /// `dir/stem (conflict <replica, 16 hex>-<lamport>).ext`.
    public static func conflictName(_ path: String, lamport: UInt64, replica: UInt64) -> String {
        let scalars = Array(path.utf8)
        let slash = scalars.lastIndex(of: UInt8(ascii: "/"))
        let dir = slash.map { String(decoding: scalars[...$0], as: UTF8.self) } ?? ""
        let name = Array(scalars[(slash.map { $0 + 1 } ?? 0)...])
        let tag = " (conflict \(String(format: "%016llx", replica))-\(lamport))"
        if let d = name.lastIndex(of: UInt8(ascii: ".")), d > 0 {
            return dir + String(decoding: name[..<d], as: UTF8.self) + tag + String(decoding: name[d...], as: UTF8.self)
        }
        return dir + String(decoding: name, as: UTF8.self) + tag
    }

    public func contains(_ file: FileID) -> Bool { entries[file] != nil }
    public func kind(_ file: FileID) -> FileKind? { entries[file]?.kind }
    public func isDeleted(_ file: FileID) -> Bool? { entries[file]?.deleted.1 }

    func local(_ file: FileID, _ kind: FileOpKind) throws -> FileOp {
        let op = FileOp(id: CollabID(replica: replica, counter: stateVector[replica]), lamport: lamport + 1,
                        file: file, kind: kind)
        try apply(op)
        return op
    }

    public func create(_ file: FileID, kind: FileKind, path: String) throws -> FileOp {
        try local(file, .create(kind: kind, path: path))
    }
    public func rename(_ file: FileID, to path: String) throws -> FileOp { try local(file, .setPath(path)) }
    public func setBlob(_ file: FileID, _ blob: BlobRef) throws -> FileOp { try local(file, .setBlob(blob)) }
    public func setDeleted(_ file: FileID, _ deleted: Bool) throws -> FileOp { try local(file, .setDeleted(deleted)) }

    @discardableResult
    public func apply(_ op: FileOp) throws -> ApplyResult {
        let next = stateVector[op.id.replica]
        if op.id.counter < next {
            // Byte-exact: Swift's String == is canonical equivalence, and an
            // NFC and an NFD path are different paths.
            if let seen = ops[op.id], CollabWire.encode(seen) == CollabWire.encode(op) { return .duplicate }
            throw CollabError.idConflict(op.id)
        }
        if op.id.counter > next { throw CollabError.missingDependency(CollabID(replica: op.id.replica, counter: next)) }
        if op.lamport == 0 { throw CollabError.malformed("lamport 0") }
        let stamp = Stamp(lamport: op.lamport, replica: op.id.replica)
        switch op.kind {
        case let .create(kind, path):
            guard Self.isValidPath(path) else { throw CollabError.malformed("invalid path") }
            guard entries[op.file] == nil else { throw CollabError.idConflict(op.id) }
            entries[op.file] = Entry(kind: kind, path: (stamp, path), blob: nil, deleted: (stamp, false))
        case let .setPath(path):
            guard var e = entries[op.file] else { throw CollabError.unknownFile(op.file) }
            guard Self.isValidPath(path) else { throw CollabError.malformed("invalid path") }
            if e.path.0 < stamp { e.path = (stamp, path) }
            entries[op.file] = e
        case let .setBlob(blob):
            guard var e = entries[op.file] else { throw CollabError.unknownFile(op.file) }
            guard e.kind == .blob else { throw CollabError.malformed("blob on a text file") }
            guard blob.mediaType.utf8.count <= CollabLimits.maxMediaTypeBytes else {
                throw CollabError.malformed("media type too long")
            }
            if e.blob.map({ $0.0 < stamp }) ?? true { e.blob = (stamp, blob) }
            entries[op.file] = e
        case let .setDeleted(d):
            guard var e = entries[op.file] else { throw CollabError.unknownFile(op.file) }
            if e.deleted.0 < stamp { e.deleted = (stamp, d) }
            entries[op.file] = e
        }
        lamport = max(lamport, op.lamport)
        stateVector[op.id.replica] = next + 1
        ops[op.id] = op
        log.append(op)
        return .applied
    }

    public func diff(since sv: StateVector) -> [FileOp] { log.filter { !sv.contains($0.id) } }

    /// Live files at their materialised paths, sorted by path; on a shared
    /// path the lowest path stamp keeps it and the rest get conflict names.
    public var files: [FileEntryView] {
        // Keyed by bytes: Swift String equality would merge NFC and NFD.
        var byPath: [[UInt8]: [(Stamp, FileID)]] = [:]
        for (id, e) in entries where !e.deleted.1 { byPath[Array(e.path.1.utf8), default: []].append((e.path.0, id)) }
        var out: [FileEntryView] = []
        for (pathBytes, group) in byPath {
            let path = String(decoding: pathBytes, as: UTF8.self)
            let g = group.sorted { $0.0 != $1.0 ? $0.0 < $1.0 : $0.1 < $1.1 }
            for (k, (stamp, id)) in g.enumerated() {
                let e = entries[id]!
                out.append(FileEntryView(
                    file: id,
                    path: k == 0 ? path : Self.conflictName(path, lamport: stamp.lamport, replica: stamp.replica),
                    requestedPath: path, kind: e.kind, blob: e.blob?.1, conflict: g.count > 1))
            }
        }
        // Byte order, as Rust's String ordering.
        return out.sorted { Array($0.path.utf8).lexicographicallyPrecedes(Array($1.path.utf8)) }
    }

    /// FNV-1a 64 over every entry in FileID byte order (contract §5).
    public var digest: UInt64 {
        var h = FNV64()
        for id in entries.keys.sorted() {
            let e = entries[id]!
            h.bytes(id.bytes)
            h.byte(e.kind.rawValue)
            h.u64(e.path.0.lamport)
            h.u64(e.path.0.replica)
            let p = Array(e.path.1.utf8)
            h.u64(UInt64(p.count))
            h.bytes(p)
            if let (s, b) = e.blob {
                h.byte(1)
                h.u64(s.lamport)
                h.u64(s.replica)
                h.bytes(b.sha256)
                h.u64(b.bytes)
                let m = Array(b.mediaType.utf8)
                h.u64(UInt64(m.count))
                h.bytes(m)
            } else {
                h.byte(0)
            }
            h.u64(e.deleted.0.lamport)
            h.u64(e.deleted.0.replica)
            h.byte(e.deleted.1 ? 1 : 0)
        }
        return h.value
    }
}
