import Foundation

/// Which document of a project a section or state vector is about.
public enum DocRef: Hashable, Sendable {
    case fileMap
    case text(FileID)
}

/// Operations for one document, as `update` and `sync_reply` carry them.
public enum Section: Equatable, Sendable {
    case fileMap([FileOp])
    case text(FileID, [TextOp])
}

public struct DocVector: Equatable, Sendable {
    public var doc: DocRef
    public var vector: StateVector

    public init(doc: DocRef, vector: StateVector) {
        self.doc = doc
        self.vector = vector
    }
}

/// A whole project: the file map, a text CRDT per text file, a bounded
/// pending buffer for out-of-order delivery, and state-vector sync.
public final class CollabProject {
    public let replica: UInt64
    public let files: FileMap
    var texts: [FileID: TextDocument] = [:]

    enum PendingOp {
        case file(FileOp)
        case text(FileID, TextOp)

        var doc: DocRef {
            switch self {
            case .file: return .fileMap
            case let .text(f, _): return .text(f)
            }
        }

        var id: CollabID {
            switch self {
            case let .file(op): return op.id
            case let .text(_, op): return op.id
            }
        }

        var replica: UInt64 { id.replica }

        /// Exact identity for deduplication: an exact redelivery is not
        /// parked twice (the oracle compares the operations for equality).
        var key: ParkedKey {
            switch self {
            case let .file(op): return ParkedKey(doc: .fileMap, bytes: CollabWire.encode(op))
            case let .text(f, op): return ParkedKey(doc: .text(f), bytes: CollabWire.encode(op))
            }
        }

        /// What it counts against `CollabLimits.maxPendingBytes`.
        var weight: Int {
            switch self {
            case let .file(op):
                switch op.kind {
                case let .create(_, path), let .setPath(path): return 64 + path.utf8.count
                case let .setBlob(b): return 64 + b.mediaType.utf8.count
                case .setDeleted: return 64
                }
            case let .text(_, op):
                if case let .insert(_, _, _, content) = op { return 64 + content.utf8.count }
                return 64
            }
        }
    }

    struct ParkedKey: Hashable {
        var doc: DocRef
        var bytes: [UInt8]
    }

    /// Operations waiting for a replica's sequence in one document to pass
    /// a counter: woken exactly when it does, so out-of-order delivery costs
    /// one retry per operation, not a rescan of the whole buffer.
    struct WaitKey: Hashable {
        var doc: DocRef
        var replica: UInt64
    }

    var waiting: [WaitKey: [(need: UInt64, op: PendingOp)]] = [:]
    /// Text operations whose file is not in the file map yet.
    var waitingForFiles: [PendingOp] = []
    var parked: Set<ParkedKey> = []
    /// Distinct parked operations.
    public private(set) var pendingCount = 0
    public private(set) var pendingBytes = 0

    /// An operation refused for good, named by its document and id.
    public enum ReceiveError: Error, Equatable {
        case rejected(DocRef, CollabID, CollabError)
        case pendingFull(DocRef, CollabID)

        /// The contract's name for the refusal (fixtures compare these).
        public var kindName: String {
            switch self {
            case .pendingFull: return "pending_full"
            case let .rejected(_, _, e):
                switch e {
                case .missingDependency: return "missing_dependency"
                case .unknownFile: return "unknown_file"
                case .idConflict: return "id_conflict"
                case .malformed: return "malformed"
                case .documentFull: return "document_full"
                case .pendingFull: return "pending_full"
                }
            }
        }

        public var doc: DocRef {
            switch self {
            case let .rejected(d, _, _), let .pendingFull(d, _): return d
            }
        }

        public var op: CollabID {
            switch self {
            case let .rejected(_, id, _), let .pendingFull(_, id): return id
            }
        }
    }

    public init(replica: UInt64) {
        self.replica = replica
        files = FileMap(replica: replica)
    }

    public func text(_ file: FileID) -> TextDocument? { texts[file] }

    /// Text files in FileID order.
    public var textFileIDs: [FileID] { texts.keys.sorted() }

    private func noteFileOp(_ op: FileOp) {
        if case .create(.text, _) = op.kind, texts[op.file] == nil {
            texts[op.file] = TextDocument(replica: replica)
        }
    }

    // MARK: Local edits (each returns the section to broadcast)

    public func fileOp(_ body: (FileMap) throws -> FileOp) throws -> Section {
        let op = try body(files)
        noteFileOp(op)
        return .fileMap([op])
    }

    public func insert(_ text: String, at position: Int, in file: FileID) throws -> Section? {
        guard let doc = texts[file], let op = try doc.insert(text, at: position) else { return nil }
        return .text(file, [op])
    }

    public func delete(at position: Int, length: Int, in file: FileID) -> Section? {
        guard let doc = texts[file] else { return nil }
        let ops = doc.delete(at: position, length: length)
        return ops.isEmpty ? nil : .text(file, ops)
    }

    // MARK: Remote edits

    private func tryApply(_ op: PendingOp) throws -> ApplyResult {
        switch op {
        case let .file(f):
            let r = try files.apply(f)
            noteFileOp(f)
            return r
        case let .text(file, t):
            guard let doc = texts[file] else { throw CollabError.unknownFile(file) }
            return try doc.apply(t)
        }
    }

    /// Apply sections from a peer, in any order and with duplicates. What
    /// cannot apply yet waits (bounded by count and bytes) and is retried
    /// when the dependency it named arrives. Returns the operations refused
    /// for good.
    @discardableResult
    public func receive(_ sections: [Section]) -> [ReceiveError] {
        var errors: [ReceiveError] = []
        for s in sections {
            switch s {
            case let .fileMap(ops):
                for op in ops { attempt(.file(op), &errors) }
            case let .text(f, ops):
                for op in ops { attempt(.text(f, op), &errors) }
            }
        }
        return errors
    }

    private func unpark(_ ops: [PendingOp]) {
        for op in ops {
            parked.remove(op.key)
            pendingCount -= 1
            pendingBytes -= op.weight
        }
    }

    /// Try one operation; park it if it must wait, and wake what its
    /// success unblocks.
    private func attempt(_ op: PendingOp, _ errors: inout [ReceiveError]) {
        var work = [op]
        while let next = work.popLast() {
            do {
                guard try tryApply(next) == .applied else { continue }
                let key = WaitKey(doc: next.doc, replica: next.replica)
                // Each list is sorted by `need`, so the ready ones are a prefix.
                if let list = waiting[key], let first = list.first, first.need < stateVector(of: key.doc)[key.replica] {
                    let sv = stateVector(of: key.doc)[key.replica]
                    let cut = list.firstIndex { $0.need >= sv } ?? list.count
                    waiting[key] = cut == list.count ? nil : Array(list[cut...])
                    let ready = list[..<cut].map(\.op)
                    unpark(ready)
                    work += ready
                }
                if case .file = next, !waitingForFiles.isEmpty {
                    let ready = waitingForFiles
                    waitingForFiles = []
                    unpark(ready)
                    work += ready
                }
            } catch let e as CollabError where e.isRetryable {
                let k = next.key
                if parked.contains(k) { continue }
                guard pendingCount < CollabLimits.maxPendingOps,
                      pendingBytes + next.weight <= CollabLimits.maxPendingBytes else {
                    errors.append(.pendingFull(next.doc, next.id))
                    continue
                }
                parked.insert(k)
                pendingCount += 1
                pendingBytes += next.weight
                switch e {
                case let .missingDependency(id):
                    let key = WaitKey(doc: next.doc, replica: id.replica)
                    var lo = 0, hi = waiting[key]?.count ?? 0
                    while lo < hi {
                        let mid = (lo + hi) >> 1
                        if waiting[key]![mid].need <= id.counter { lo = mid + 1 } else { hi = mid }
                    }
                    waiting[key, default: []].insert((id.counter, next), at: lo)
                default:
                    waitingForFiles.append(next)
                }
            } catch let e as CollabError {
                errors.append(.rejected(next.doc, next.id, e))
            } catch {
                errors.append(.rejected(next.doc, next.id, .malformed("\(error)")))
            }
        }
    }

    func stateVector(of doc: DocRef) -> StateVector {
        switch doc {
        case .fileMap: return files.stateVector
        case let .text(f): return texts[f]?.stateVector ?? StateVector()
        }
    }

    // MARK: Sync

    public var stateVectors: [DocVector] {
        [DocVector(doc: .fileMap, vector: files.stateVector)]
            + textFileIDs.map { DocVector(doc: .text($0), vector: texts[$0]!.stateVector) }
    }

    /// What a peer with `remote` state vectors lacks: the file map first.
    public func diff(_ remote: [DocVector]) -> [Section] {
        func sv(_ d: DocRef) -> StateVector { remote.first { $0.doc == d }?.vector ?? StateVector() }
        var out: [Section] = []
        let fops = files.diff(since: sv(.fileMap))
        if !fops.isEmpty { out.append(.fileMap(fops)) }
        for f in textFileIDs {
            let ops = texts[f]!.diff(since: sv(.text(f)))
            if !ops.isEmpty { out.append(.text(f, ops)) }
        }
        return out
    }

    /// The file map's digest, then each text file's id and digest in
    /// FileID order (the oracle's `Project::digest`).
    public var digest: UInt64 {
        var h = FNV64()
        h.u64(files.digest)
        for f in textFileIDs {
            h.bytes(f.bytes)
            h.u64(texts[f]!.digest)
        }
        return h.value
    }
}
