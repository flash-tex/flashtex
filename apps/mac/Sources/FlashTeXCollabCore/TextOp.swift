import Foundation

/// One collab-v1 text operation. Each consumes `length` consecutive counters
/// of its replica (contract §2).
public enum TextOp: Equatable, Sendable {
    /// Insert `content` (one or more scalars) between the raw neighbours the
    /// author saw, tombstones included (`nil`: the start, the end). Unit k
    /// of the run has id `id + k`, left origin `k == 0 ? originLeft : id +
    /// k - 1`, and right origin `originRight`.
    case insert(id: CollabID, originLeft: CollabID?, originRight: CollabID?, content: String)
    /// Tombstone the `length` scalars `target, target + 1, ...`.
    case delete(id: CollabID, target: CollabID, length: UInt64)

    public var id: CollabID {
        switch self {
        case let .insert(id, _, _, _), let .delete(id, _, _): return id
        }
    }

    /// Units (counters) consumed.
    public var length: UInt64 {
        switch self {
        case let .insert(_, _, _, content): return UInt64(content.unicodeScalars.count)
        case let .delete(_, _, n): return n
        }
    }

    /// The same operation without its first `skip` units (0 < skip < length).
    public func withoutPrefix(_ skip: UInt64) -> TextOp {
        switch self {
        case let .insert(id, _, right, content):
            var scalars = String.UnicodeScalarView()
            scalars.append(contentsOf: content.unicodeScalars.dropFirst(Int(skip)))
            return .insert(id: id.offset(skip), originLeft: id.offset(skip - 1), originRight: right,
                           content: String(scalars))
        case let .delete(id, target, n):
            return .delete(id: id.offset(skip), target: target.offset(skip), length: n - skip)
        }
    }
}

/// Which side of its anchor a `RelativePosition` sticks to.
public enum Assoc: UInt8, Sendable, Codable {
    /// Just before the anchor (moves with the text on its right). With no
    /// anchor: the end of the document.
    case before = 0
    /// Just after the anchor (moves with the text on its left). With no
    /// anchor: the start.
    case after = 1
}

/// A position that survives concurrent edits (carets, selections, folds,
/// marks, undo anchors): a scalar named by id, and a side.
public struct RelativePosition: Hashable, Sendable {
    public var anchor: CollabID?
    public var assoc: Assoc

    public init(anchor: CollabID?, assoc: Assoc) {
        self.anchor = anchor
        self.assoc = assoc
    }
}

// MARK: - UTF-8 helpers (content is stored and carried as UTF-8)

@usableFromInline enum UTF8Scan {
    @inlinable static func isLead(_ b: UInt8) -> Bool { b & 0xC0 != 0x80 }

    /// Scalars in `bytes`.
    @inlinable static func scalarCount<C: Collection>(_ bytes: C) -> Int where C.Element == UInt8 {
        var n = 0
        for b in bytes where isLead(b) { n += 1 }
        return n
    }

    /// UTF-16 units in `bytes`: one per scalar, two for a 4-byte one.
    @inlinable static func utf16Count<C: Collection>(_ bytes: C) -> Int where C.Element == UInt8 {
        var n = 0
        for b in bytes where isLead(b) { n += b >= 0xF0 ? 2 : 1 }
        return n
    }

    /// Byte index of scalar `k` (`bytes.count` when k is the scalar count).
    @inlinable static func byteIndex(_ bytes: [UInt8], scalar k: Int) -> Int {
        if k == 0 { return 0 }
        var seen = 0
        var i = 0
        while i < bytes.count {
            if isLead(bytes[i]) {
                if seen == k { return i }
                seen += 1
            }
            i += 1
        }
        return bytes.count
    }

    /// The scalar whose lead byte is at `i` (valid UTF-8).
    @inlinable static func scalar(_ bytes: [UInt8], at i: Int) -> UInt32 {
        let b0 = UInt32(bytes[i])
        if b0 < 0x80 { return b0 }
        if b0 < 0xE0 { return (b0 & 0x1F) << 6 | UInt32(bytes[i + 1] & 0x3F) }
        if b0 < 0xF0 {
            return (b0 & 0x0F) << 12 | UInt32(bytes[i + 1] & 0x3F) << 6 | UInt32(bytes[i + 2] & 0x3F)
        }
        return (b0 & 0x07) << 18 | UInt32(bytes[i + 1] & 0x3F) << 12
            | UInt32(bytes[i + 2] & 0x3F) << 6 | UInt32(bytes[i + 3] & 0x3F)
    }

    /// Decode the scalars of valid UTF-8.
    @inlinable static func forEachScalar(_ bytes: [UInt8], _ body: (UInt32) -> Void) {
        var i = 0
        let n = bytes.count
        while i < n {
            let b0 = UInt32(bytes[i])
            if b0 < 0x80 {
                body(b0); i += 1
            } else if b0 < 0xE0 {
                body((b0 & 0x1F) << 6 | UInt32(bytes[i + 1] & 0x3F)); i += 2
            } else if b0 < 0xF0 {
                body((b0 & 0x0F) << 12 | UInt32(bytes[i + 1] & 0x3F) << 6 | UInt32(bytes[i + 2] & 0x3F)); i += 3
            } else {
                body((b0 & 0x07) << 18 | UInt32(bytes[i + 1] & 0x3F) << 12
                     | UInt32(bytes[i + 2] & 0x3F) << 6 | UInt32(bytes[i + 3] & 0x3F)); i += 4
            }
        }
    }
}

/// One change to a document's visible text (`TextDocument.changeObserver`):
/// replace `length` UTF-16 units at `location` with `text`.
public struct TextChange: Equatable, Sendable {
    public var location: Int
    public var length: Int
    public var text: String

    public init(location: Int, length: Int, text: String) {
        self.location = location
        self.length = length
        self.text = text
    }
}
