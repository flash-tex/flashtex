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

        var replica: UInt64 {
            switch self {
            case let .file(op): return op.id.replica
            case let .text(_, op): return op.id.replica
            }
        }
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
    public private(set) var pendingCount = 0

    public enum ReceiveError: Error, Equatable {
        case rejected(CollabError)
        case pendingFull
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
    /// cannot apply yet waits (bounded) and is retried after each success.
    /// Returns the operations refused for good.
    @discardableResult
    public func receive(_ sections: [Section]) -> [ReceiveError] {
        var errors: [ReceiveError] = []
        for s in sections {
            switch s {
            case let .fileMap(ops):
                for op in ops { receiveOne(.file(op), doc: .fileMap, &errors) }
            case let .text(f, ops):
                for op in ops { receiveOne(.text(f, op), doc: .text(f), &errors) }
            }
        }
        return errors
    }

    private func receiveOne(_ op: PendingOp, doc: DocRef, _ errors: inout [ReceiveError]) {
        attempt(op, &errors)
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
                    pendingCount -= cut
                    work += list[..<cut].map(\.op)
                }
                if case .file = next, !waitingForFiles.isEmpty {
                    pendingCount -= waitingForFiles.count
                    work += waitingForFiles
                    waitingForFiles = []
                }
            } catch let e as CollabError where e.isRetryable {
                guard pendingCount < CollabLimits.maxPendingOps else {
                    errors.append(.pendingFull)
                    continue
                }
                pendingCount += 1
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
                errors.append(.rejected(e))
            } catch {
                errors.append(.rejected(.malformed("\(error)")))
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
