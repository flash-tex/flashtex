// FlashTeXCollabCore: the live-collaboration data model (collab-v1).
//
// Platform-free (Foundation only; no AppKit/UIKit), shared by the Mac and,
// through a symlink in apps/ios/Packages/FlashTeXPadKit, the iPad. MIT.
// Design: docs/design/live-collab/PROPOSAL.md. Normative wire format and
// semantics: docs/contracts/collab-v1.md. The reference implementation it
// is checked against is crates/collaboration-core (src/v1), through the
// shared fixtures in crates/collaboration-core/tests/fixtures/collab-v1.

import Foundation

/// The id of one unit of an operation: one inserted scalar, one scalar's
/// deletion, or one file-map change. A replica numbers its units 0, 1, 2,
/// ... with no gaps, per document.
public struct CollabID: Hashable, Comparable, Sendable, CustomStringConvertible {
    public var replica: UInt64
    public var counter: UInt64

    public init(replica: UInt64, counter: UInt64) {
        self.replica = replica
        self.counter = counter
    }

    @inlinable public func offset(_ n: UInt64) -> CollabID {
        CollabID(replica: replica, counter: counter &+ n)
    }

    public static func < (a: CollabID, b: CollabID) -> Bool {
        (a.replica, a.counter) < (b.replica, b.counter)
    }

    public var description: String { "\(String(replica, radix: 16))@\(counter)" }
}

/// For each replica, how many of its units are applied: the next counter
/// expected from it.
public struct StateVector: Equatable, Sendable {
    public var entries: [UInt64: UInt64]

    public init(_ entries: [UInt64: UInt64] = [:]) { self.entries = entries }

    @inlinable public subscript(replica: UInt64) -> UInt64 {
        get { entries[replica] ?? 0 }
        set { entries[replica] = newValue }
    }

    @inlinable public func contains(_ id: CollabID) -> Bool { id.counter < self[id.replica] }

    /// Entries in replica order (the wire order).
    public var sorted: [(replica: UInt64, next: UInt64)] {
        entries.sorted { $0.key < $1.key }.map { ($0.key, $0.value) }
    }
}

/// Bounds shared with the Rust oracle (`v1::limits`).
public enum CollabLimits {
    /// Largest text document in UTF-8 bytes, tombstones included
    /// (edit-ledger's MAX_DOCUMENT_BYTES).
    public static let maxDocumentBytes = 8 * 1024 * 1024
    public static let maxPendingOps = 65_536
    /// Most bytes parked in a pending buffer (variable payload + 64 per
    /// op, as the oracle's `pending_weight_*`): large inserts whose
    /// dependencies never arrive cannot grow memory without bound.
    public static let maxPendingBytes = 16 * 1024 * 1024
    public static let maxPathBytes = 1024
    public static let maxMediaTypeBytes = 255
    /// Longest stored run, in scalars. Storage only: a run on the wire may
    /// be longer, and is split into runs of this size, which the unit-wise
    /// semantics make invisible.
    public static let maxRunScalars = 256
    /// Most file-map entries in a project, deleted ones included (the
    /// oracle's `MAX_FILES`). A Live Share host shares at most 200 files;
    /// this leaves room for a long session's deletes and re-creates while
    /// bounding what a hostile hub can make a guest hold.
    public static let maxFiles = 4096
    /// Most stored text, in UTF-8 bytes with tombstones, across all of a
    /// project's text documents (the oracle's `MAX_PROJECT_TEXT_BYTES`):
    /// twice the 32 MiB a Live Share host shares, room for a session's
    /// tombstones; at most eight documents at `maxDocumentBytes`.
    public static let maxProjectTextBytes = 64 * 1024 * 1024
    /// This replica's own edits stop this far below the caps (the oracle's
    /// `LOCAL_FILE_HEADROOM`, `LOCAL_TEXT_HEADROOM`; contract §2.4a).
    public static let localFileHeadroom = 64
    public static let localTextHeadroom = 1024 * 1024
}

/// What a project's text documents have stored together, against its cap;
/// shared by the project and every one of its documents.
final class TextBudget {
    let limit: Int
    /// A local edit's room ends this far below `limit` (contract §2.4a).
    let localHeadroom: Int
    var used = 0
    init(limit: Int, localHeadroom: Int = 0) { self.limit = limit; self.localHeadroom = localHeadroom }
    func fits(_ n: Int, local: Bool = false) -> Bool {
        let room = limit - used
        return local ? n <= room - min(localHeadroom, room) : n <= room
    }
}

/// Why an operation was not applied. None leaves the document changed.
public enum CollabError: Error, Equatable, Sendable {
    /// A unit this depends on is not applied yet (a gap in its replica's
    /// sequence, an origin, a delete target). Retry later.
    case missingDependency(CollabID)
    /// The file is not in the file map yet. Retry later.
    case unknownFile(FileID)
    /// A unit id already applied with different content.
    case idConflict(CollabID)
    /// Structurally invalid.
    case malformed(String)
    /// The document would exceed `CollabLimits.maxDocumentBytes`.
    case documentFull
    case pendingFull
    /// The project would exceed `CollabLimits.maxFiles` entries or
    /// `CollabLimits.maxProjectTextBytes` of stored text.
    case projectFull

    public var isRetryable: Bool {
        switch self {
        case .missingDependency, .unknownFile: return true
        default: return false
        }
    }
}

public enum ApplyResult: Equatable, Sendable {
    case applied
    case duplicate
}

/// 64-bit FNV-1a: the structure digest both implementations compute.
public struct FNV64: Sendable {
    public var value: UInt64 = 0xcbf2_9ce4_8422_2325

    public init() {}

    @inlinable public mutating func byte(_ b: UInt8) {
        value ^= UInt64(b)
        value = value &* 0x0000_0100_0000_01b3
    }

    @inlinable public mutating func bytes<S: Sequence>(_ data: S) where S.Element == UInt8 {
        for b in data { byte(b) }
    }

    @inlinable public mutating func u64(_ v: UInt64) {
        var v = v
        for _ in 0..<8 {
            byte(UInt8(truncatingIfNeeded: v))
            v >>= 8
        }
    }

    @inlinable public mutating func u32(_ v: UInt32) {
        var v = v
        for _ in 0..<4 {
            byte(UInt8(truncatingIfNeeded: v))
            v >>= 8
        }
    }
}

/// SplitMix64, the seeded generator the tests and fixtures use in both
/// languages (`tests/common/mod.rs` has the same one).
public struct SplitMix64: RandomNumberGenerator, Sendable {
    public var state: UInt64

    public init(seed: UInt64) { state = seed }

    public mutating func next() -> UInt64 {
        state = state &+ 0x9e37_79b9_7f4a_7c15
        var z = state
        z = (z ^ (z >> 30)) &* 0xbf58_476d_1ce4_e5b9
        z = (z ^ (z >> 27)) &* 0x94d0_49bb_1331_11eb
        return z ^ (z >> 31)
    }

    /// Uniform in 0..<n, exactly as the Rust tests' `below`.
    public mutating func below(_ n: Int) -> Int { Int(next() % UInt64(n)) }

    public mutating func chance(_ percent: UInt64) -> Bool { next() % 100 < percent }
}
