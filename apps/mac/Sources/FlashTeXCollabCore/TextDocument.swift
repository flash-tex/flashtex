import Foundation

// The text sequence CRDT (proposal §2.3, contract §2).
//
// Storage: runs ("items") of consecutive ids typed left to right, at most
// `CollabLimits.maxRunScalars` scalars each, content as UTF-8, kept in an
// order-statistics B+tree whose nodes sum raw scalars (tombstones
// included), visible scalars, visible UTF-16 units and visible UTF-8 bytes.
// So a visible offset in any of the three units maps to a run in O(log n),
// and a run's rank is a walk to the root. An id maps to its run through a
// per-replica sorted index (binary search).
//
// Integration: FugueMax (Gentle's integrateYjsMod), applied to a whole run
// at once. That is exact: the units after a run's first all have their
// predecessor as left origin, so the per-unit rule never stops inside a
// run (contract §2.3); the Rust oracle integrates unit by unit and the
// shared fixtures prove the two agree.

@usableFromInline final class Item {
    let replica: UInt64
    let counter: UInt64
    var len: Int
    var u16: Int
    var bytes: [UInt8]
    let originLeft: CollabID?
    let originRight: CollabID?
    var deleted: Bool
    unowned(unsafe) var leaf: Node?

    init(replica: UInt64, counter: UInt64, len: Int, u16: Int, bytes: [UInt8],
         originLeft: CollabID?, originRight: CollabID?, deleted: Bool) {
        self.replica = replica
        self.counter = counter
        self.len = len
        self.u16 = u16
        self.bytes = bytes
        self.originLeft = originLeft
        self.originRight = originRight
        self.deleted = deleted
    }

    var firstID: CollabID { CollabID(replica: replica, counter: counter) }
    var lastID: CollabID { CollabID(replica: replica, counter: counter + UInt64(len) - 1) }
    var visLen: Int { deleted ? 0 : len }
    var visU16: Int { deleted ? 0 : u16 }
    var visU8: Int { deleted ? 0 : bytes.count }

    /// Left origin of unit `k` of this run.
    func originLeft(ofUnit k: Int) -> CollabID? {
        k == 0 ? originLeft : CollabID(replica: replica, counter: counter + UInt64(k) - 1)
    }
}

@usableFromInline final class Node {
    unowned(unsafe) var parent: Node?
    let isLeaf: Bool
    var items: [Item] = []
    var children: [Node] = []
    var raw = 0, vis = 0, vu16 = 0, vu8 = 0
    unowned(unsafe) var next: Node?
    unowned(unsafe) var prev: Node?

    init(isLeaf: Bool) { self.isLeaf = isLeaf }

    func add(raw r: Int, vis v: Int, u16 a: Int, u8 b: Int) {
        var n: Node? = self
        while let x = n {
            x.raw += r; x.vis += v; x.vu16 += a; x.vu8 += b
            n = x.parent
        }
    }

    func recomputeFromItems() {
        raw = 0; vis = 0; vu16 = 0; vu8 = 0
        for it in items { raw += it.len; vis += it.visLen; vu16 += it.visU16; vu8 += it.visU8 }
    }

    func recomputeFromChildren() {
        raw = 0; vis = 0; vu16 = 0; vu8 = 0
        for c in children { raw += c.raw; vis += c.vis; vu16 += c.vu16; vu8 += c.vu8 }
    }
}

/// One replica's runs sorted by counter, in chunks so that inserting a
/// split's tail in the middle costs O(chunk), not O(runs).
final class ReplicaRuns {
    static let chunkMax = 512
    var starts: [UInt64] = []
    var chunks: [[Item]] = []
    private(set) var count = 0

    private func chunk(atOrBefore counter: UInt64) -> Int? {
        var lo = 0, hi = starts.count
        while lo < hi {
            let mid = (lo + hi) >> 1
            if starts[mid] <= counter { lo = mid + 1 } else { hi = mid }
        }
        return lo == 0 ? nil : lo - 1
    }

    private static func position(_ items: [Item], after counter: UInt64) -> Int {
        var lo = 0, hi = items.count
        while lo < hi {
            let mid = (lo + hi) >> 1
            if items[mid].counter <= counter { lo = mid + 1 } else { hi = mid }
        }
        return lo
    }

    /// The last run starting at or before `counter`.
    func find(_ counter: UInt64) -> Item? {
        guard let c = chunk(atOrBefore: counter) else { return nil }
        let i = Self.position(chunks[c], after: counter)
        return i == 0 ? nil : chunks[c][i - 1]
    }

    func insert(_ item: Item) {
        count += 1
        guard !chunks.isEmpty else {
            starts = [item.counter]
            chunks = [[item]]
            return
        }
        let c = chunk(atOrBefore: item.counter) ?? 0
        if c == chunks.count - 1, let last = chunks[c].last, last.counter < item.counter {
            chunks[c].append(item)
        } else {
            let i = Self.position(chunks[c], after: item.counter)
            chunks[c].insert(item, at: i)
            if i == 0 { starts[c] = item.counter }
        }
        if chunks[c].count > Self.chunkMax {
            let tail = Array(chunks[c][(Self.chunkMax / 2)...])
            chunks[c].removeSubrange((Self.chunkMax / 2)...)
            chunks.insert(tail, at: c + 1)
            starts.insert(tail[0].counter, at: c + 1)
        }
    }
}

struct DeleteRun {
    let counter: UInt64
    let target: CollabID
    var len: UInt64
}

enum LogEntry {
    case insert(id: CollabID, len: UInt64, left: CollabID?, right: CollabID?)
    case delete(id: CollabID, target: CollabID, len: UInt64)

    var id: CollabID {
        switch self {
        case let .insert(id, _, _, _), let .delete(id, _, _): return id
        }
    }

    var len: UInt64 {
        switch self {
        case let .insert(_, n, _, _), let .delete(_, _, n): return n
        }
    }
}

/// One replica of one text file.
public final class TextDocument {
    public let replica: UInt64
    public private(set) var stateVector = StateVector()

    static let leafMax = 64
    static let nodeMax = 32

    var root = Node(isLeaf: true)
    var firstLeaf: Node
    var lastLeaf: Node
    var runs: [UInt64: ReplicaRuns] = [:]
    var deleteRuns: [UInt64: [DeleteRun]] = [:]
    var log: [LogEntry] = []
    var storedBytes = 0

    public init(replica: UInt64) {
        self.replica = replica
        firstLeaf = root
        lastLeaf = root
    }

    // MARK: Queries

    /// Visible length in scalars.
    public var count: Int { root.vis }
    /// Visible length in UTF-16 units (NSString length).
    public var utf16Count: Int { root.vu16 }
    /// Visible length in UTF-8 bytes.
    public var utf8Count: Int { root.vu8 }
    /// Scalars stored, tombstones included.
    public var rawCount: Int { root.raw }
    /// Stored runs (B-tree items).
    public var runCount: Int { runs.values.reduce(0) { $0 + $1.count } }
    /// Operations in the log (after coalescing).
    public var logCount: Int { log.count }

    public var text: String {
        var out: [UInt8] = []
        out.reserveCapacity(root.vu8)
        forEachItem { it in if !it.deleted { out.append(contentsOf: it.bytes) } }
        return String(decoding: out, as: UTF8.self)
    }

    /// FNV-1a 64 over every stored scalar in document order: replica (u64
    /// LE), counter (u64 LE), deleted (u8), scalar (u32 LE). Identical to the
    /// oracle's `TextDoc::digest`.
    public var digest: UInt64 {
        var h = FNV64()
        forEachItem { it in
            var k: UInt64 = 0
            UTF8Scan.forEachScalar(it.bytes) { s in
                h.u64(it.replica)
                h.u64(it.counter + k)
                h.byte(it.deleted ? 1 : 0)
                h.u32(s)
                k += 1
            }
        }
        return h.value
    }

    /// Ids of the visible scalars, in order (tests, diagnostics).
    public var visibleIDs: [CollabID] {
        var out: [CollabID] = []
        forEachItem { it in
            if !it.deleted { for k in 0..<it.len { out.append(CollabID(replica: it.replica, counter: it.counter + UInt64(k))) } }
        }
        return out
    }

    func forEachItem(_ body: (Item) -> Void) {
        var leaf: Node? = firstLeaf
        while let l = leaf {
            for it in l.items { body(it) }
            leaf = l.next
        }
    }

    // MARK: Tree navigation

    func indexInLeaf(_ item: Item) -> Int {
        let items = item.leaf!.items
        var i = 0
        while items[i] !== item { i += 1 }
        return i
    }

    func next(_ item: Item) -> Item? {
        let leaf = item.leaf!
        let i = indexInLeaf(item)
        if i + 1 < leaf.items.count { return leaf.items[i + 1] }
        var l = leaf.next
        while let x = l {
            if let f = x.items.first { return f }
            l = x.next
        }
        return nil
    }

    func prev(_ item: Item) -> Item? {
        let leaf = item.leaf!
        let i = indexInLeaf(item)
        if i > 0 { return leaf.items[i - 1] }
        var l = leaf.prev
        while let x = l {
            if let f = x.items.last { return f }
            l = x.prev
        }
        return nil
    }

    var firstItem: Item? { firstLeaf.items.first }
    var lastItem: Item? { lastLeaf.items.last }

    /// Raw scalars before `item`.
    func rank(_ item: Item) -> Int {
        var r = 0
        let leaf = item.leaf!
        for it in leaf.items {
            if it === item { break }
            r += it.len
        }
        var child = leaf
        while let p = child.parent {
            for c in p.children {
                if c === child { break }
                r += c.raw
            }
            child = p
        }
        return r
    }

    /// Visible scalars before `item`.
    func visibleRank(_ item: Item) -> Int {
        var r = 0
        let leaf = item.leaf!
        for it in leaf.items {
            if it === item { break }
            r += it.visLen
        }
        var child = leaf
        while let p = child.parent {
            for c in p.children {
                if c === child { break }
                r += c.vis
            }
            child = p
        }
        return r
    }

    func lookup(_ id: CollabID) -> (Item, Int)? {
        guard let it = runs[id.replica]?.find(id.counter) else { return nil }
        let off = Int(id.counter - it.counter)
        return off < it.len ? (it, off) : nil
    }

    /// Raw index of the scalar `id` (which must exist).
    func rankOf(_ id: CollabID) -> Int {
        let (it, off) = lookup(id)!
        return rank(it) + off
    }

    /// The visible run holding visible scalar `pos` (0 ≤ pos < count) and
    /// the offset in it.
    func findVisible(_ pos: Int) -> (Item, Int) {
        var n = root
        var p = pos
        while !n.isLeaf {
            var chosen = n.children[n.children.count - 1]
            for c in n.children {
                if p < c.vis { chosen = c; break }
                p -= c.vis
            }
            n = chosen
        }
        for it in n.items {
            let v = it.visLen
            if p < v { return (it, p) }
            p -= v
        }
        fatalError("findVisible: position \(pos) past the end")
    }

    // MARK: Tree mutation

    func place(_ item: Item, in leaf: Node, at index: Int, addSums: Bool) {
        leaf.items.insert(item, at: index)
        item.leaf = leaf
        if addSums { leaf.add(raw: item.len, vis: item.visLen, u16: item.visU16, u8: item.visU8) }
        if leaf.items.count > Self.leafMax { splitLeaf(leaf) }
    }

    func place(_ item: Item, before dest: Item?) {
        if let d = dest {
            place(item, in: d.leaf!, at: indexInLeaf(d), addSums: true)
        } else {
            place(item, in: lastLeaf, at: lastLeaf.items.count, addSums: true)
        }
    }

    func place(_ item: Item, after a: Item) {
        place(item, in: a.leaf!, at: indexInLeaf(a) + 1, addSums: true)
    }

    func splitLeaf(_ leaf: Node) {
        let mid = leaf.items.count / 2
        let right = Node(isLeaf: true)
        right.items = Array(leaf.items[mid...])
        leaf.items.removeSubrange(mid...)
        for it in right.items { it.leaf = right }
        right.recomputeFromItems()
        leaf.raw -= right.raw; leaf.vis -= right.vis; leaf.vu16 -= right.vu16; leaf.vu8 -= right.vu8
        right.next = leaf.next
        right.prev = leaf
        leaf.next?.prev = right
        leaf.next = right
        if lastLeaf === leaf { lastLeaf = right }
        insertSibling(right, after: leaf)
    }

    func splitNode(_ node: Node) {
        let mid = node.children.count / 2
        let right = Node(isLeaf: false)
        right.children = Array(node.children[mid...])
        node.children.removeSubrange(mid...)
        for c in right.children { c.parent = right }
        right.recomputeFromChildren()
        node.raw -= right.raw; node.vis -= right.vis; node.vu16 -= right.vu16; node.vu8 -= right.vu8
        insertSibling(right, after: node)
    }

    func insertSibling(_ new: Node, after child: Node) {
        if let p = child.parent {
            var i = 0
            while p.children[i] !== child { i += 1 }
            p.children.insert(new, at: i + 1)
            new.parent = p
            if p.children.count > Self.nodeMax { splitNode(p) }
        } else {
            let r = Node(isLeaf: false)
            r.children = [child, new]
            child.parent = r
            new.parent = r
            r.recomputeFromChildren()
            root = r
        }
    }

    func indexRun(_ item: Item) {
        if let r = runs[item.replica] {
            r.insert(item)
        } else {
            let r = ReplicaRuns()
            r.insert(item)
            runs[item.replica] = r
        }
    }

    /// Split `item` before its unit `k` (0 < k < len); returns the tail.
    @discardableResult
    func split(_ item: Item, at k: Int) -> Item {
        let b = UTF8Scan.byteIndex(item.bytes, scalar: k)
        let tailBytes = Array(item.bytes[b...])
        let headBytes = Array(item.bytes[..<b])
        let headU16 = UTF8Scan.utf16Count(headBytes)
        let tail = Item(replica: item.replica, counter: item.counter + UInt64(k), len: item.len - k,
                        u16: item.u16 - headU16, bytes: tailBytes,
                        originLeft: CollabID(replica: item.replica, counter: item.counter + UInt64(k) - 1),
                        originRight: item.originRight, deleted: item.deleted)
        item.bytes = headBytes
        item.len = k
        item.u16 = headU16
        place(tail, in: item.leaf!, at: indexInLeaf(item) + 1, addSums: false)
        indexRun(tail)
        return tail
    }

    func markDeleted(_ item: Item) {
        guard !item.deleted else { return }
        item.leaf!.add(raw: 0, vis: -item.len, u16: -item.u16, u8: -item.bytes.count)
        item.deleted = true
    }

    /// Insert a run (already id-assigned) before `dest` (nil: the end),
    /// extending the previous run when the run continues it, and storing
    /// it in chunks of at most `maxRunScalars`.
    func insertRun(before dest: Item?, id: CollabID, originLeft: CollabID?, originRight: CollabID?,
                   bytes: [UInt8]) {
        let maxRun = CollabLimits.maxRunScalars
        var start = 0
        var counter = id.counter
        var prevItem: Item? = dest.map { prev($0) } ?? lastItem
        var left = originLeft
        while start < bytes.count {
            // Take up to maxRun scalars (fewer if prev has room).
            var room = maxRun
            var extend = false
            if let p = prevItem, p.replica == id.replica, !p.deleted, p.counter + UInt64(p.len) == counter,
               p.originRight == originRight, left == p.lastID, p.len < maxRun {
                extend = true
                room = maxRun - p.len
            }
            var end = start
            var scalars = 0
            while end < bytes.count {
                if UTF8Scan.isLead(bytes[end]) {
                    if scalars == room { break }
                    scalars += 1
                }
                end += 1
            }
            let chunk = Array(bytes[start..<end])
            let u16 = UTF8Scan.utf16Count(chunk)
            if extend, let p = prevItem {
                p.bytes.append(contentsOf: chunk)
                p.len += scalars
                p.u16 += u16
                p.leaf!.add(raw: scalars, vis: scalars, u16: u16, u8: chunk.count)
            } else {
                let it = Item(replica: id.replica, counter: counter, len: scalars, u16: u16, bytes: chunk,
                              originLeft: left, originRight: originRight, deleted: false)
                if let p = prevItem { place(it, after: p) } else { place(it, before: dest) }
                indexRun(it)
                prevItem = it
            }
            counter += UInt64(scalars)
            left = CollabID(replica: id.replica, counter: counter - 1)
            start = end
        }
        storedBytes += bytes.count
    }

    // MARK: Local edits

    /// Insert `text` before visible scalar `position`; returns the operation
    /// to broadcast (nil if `text` is empty).
    @discardableResult
    public func insert(_ text: String, at position: Int) throws -> TextOp? {
        precondition(position >= 0 && position <= count, "insert position out of range")
        if text.isEmpty { return nil }
        let bytes = Array(text.utf8)
        guard storedBytes + bytes.count <= CollabLimits.maxDocumentBytes else { throw CollabError.documentFull }
        var right: Item? = nil
        var left: Item? = lastItem
        if position < count {
            let (it, off) = findVisible(position)
            right = off > 0 ? split(it, at: off) : it
            left = prev(right!)
        }
        return localInsert(bytes: bytes, text: text, left: left, right: right)
    }

    func localInsert(bytes: [UInt8], text: String, left: Item?, right: Item?) -> TextOp {
        let id = CollabID(replica: replica, counter: stateVector[replica])
        let n = UInt64(UTF8Scan.scalarCount(bytes))
        let ol = left?.lastID, or = right?.firstID
        insertRun(before: right, id: id, originLeft: ol, originRight: or, bytes: bytes)
        stateVector[replica] = id.counter + n
        appendLog(.insert(id: id, len: n, left: ol, right: or))
        return .insert(id: id, originLeft: ol, originRight: or, content: text)
    }

    /// Delete `length` visible scalars from `position`; returns one
    /// operation per run of consecutive target ids.
    @discardableResult
    public func delete(at position: Int, length: Int) -> [TextOp] {
        precondition(position >= 0 && length >= 0 && position + length <= count, "delete range out of range")
        if length == 0 { return [] }
        var spans: [(CollabID, UInt64)] = []
        var (it, off) = findVisible(position)
        if off > 0 { it = split(it, at: off) }
        var remaining = length
        while true {
            if !it.deleted {
                if it.len > remaining { split(it, at: remaining) }
                markDeleted(it)
                if let last = spans.last, last.0.replica == it.replica, last.0.counter + last.1 == it.counter {
                    spans[spans.count - 1].1 += UInt64(it.len)
                } else {
                    spans.append((it.firstID, UInt64(it.len)))
                }
                remaining -= it.len
                if remaining == 0 { break }
            }
            it = next(it)!
        }
        return spans.map { localDeleteOp(target: $0.0, len: $0.1) }
    }

    func localDeleteOp(target: CollabID, len: UInt64) -> TextOp {
        let id = CollabID(replica: replica, counter: stateVector[replica])
        recordDelete(id: id, target: target, len: len)
        stateVector[replica] = id.counter + len
        appendLog(.delete(id: id, target: target, len: len))
        return .delete(id: id, target: target, length: len)
    }

    func recordDelete(id: CollabID, target: CollabID, len: UInt64) {
        // Mutated in place: copying the list out and back is O(n) per delete.
        if let last = deleteRuns[id.replica]?.last, last.counter + last.len == id.counter,
           last.target.replica == target.replica, last.target.counter + last.len == target.counter {
            deleteRuns[id.replica]![deleteRuns[id.replica]!.count - 1].len += len
        } else {
            deleteRuns[id.replica, default: []].append(DeleteRun(counter: id.counter, target: target, len: len))
        }
    }

    func appendLog(_ e: LogEntry) {
        if let last = log.last {
            switch (last, e) {
            case let (.insert(a, an, al, ar), .insert(b, bn, bl, br))
                where a.replica == b.replica && a.counter + an == b.counter && ar == br
                    && bl == CollabID(replica: a.replica, counter: b.counter - 1):
                _ = al; _ = bn
                log[log.count - 1] = .insert(id: a, len: an + bn, left: al, right: ar)
                return
            case let (.delete(a, at, an), .delete(b, bt, bn))
                where a.replica == b.replica && a.counter + an == b.counter
                    && at.replica == bt.replica && at.counter + an == bt.counter:
                log[log.count - 1] = .delete(id: a, target: at, len: an + bn)
                return
            default: break
            }
        }
        log.append(e)
    }

    // MARK: Remote edits

    /// Apply one operation. Units already applied are verified and skipped;
    /// a gap or a missing dependency throws a retryable error and changes
    /// nothing.
    @discardableResult
    public func apply(_ op: TextOp) throws -> ApplyResult {
        let id = op.id
        let len = op.length
        if len == 0 { throw CollabError.malformed("empty operation") }
        let next = stateVector[id.replica]
        if id.counter > next { throw CollabError.missingDependency(CollabID(replica: id.replica, counter: next)) }
        let dup = min(next - id.counter, len)
        var op = op
        if dup > 0 {
            try verifyDuplicate(op, units: dup)
            if dup == len { return .duplicate }
            op = op.withoutPrefix(dup)
        }
        switch op {
        case let .insert(id, ol, or, content):
            if let l = ol { try requireScalar(l) }
            if let r = or { try requireScalar(r) }
            let bytes = Array(content.utf8)
            guard storedBytes + bytes.count <= CollabLimits.maxDocumentBytes else { throw CollabError.documentFull }
            try integrate(id: id, originLeft: ol, originRight: or, bytes: bytes)
            stateVector[id.replica] = id.counter + op.length
            appendLog(.insert(id: id, len: op.length, left: ol, right: or))
        case let .delete(id, target, n):
            try requireScalars(target, n)
            var k: UInt64 = 0
            while k < n {
                let (it0, off) = lookup(target.offset(k))!
                let need = Int(n - k)
                if it0.deleted {
                    k += UInt64(min(it0.len - off, need))
                    continue
                }
                let it = off > 0 ? split(it0, at: off) : it0
                if it.len > need { split(it, at: need) }
                markDeleted(it)
                k += UInt64(it.len)
            }
            recordDelete(id: id, target: target, len: n)
            stateVector[id.replica] = id.counter + n
            appendLog(.delete(id: id, target: target, len: n))
        }
        return .applied
    }

    func requireScalar(_ id: CollabID) throws {
        if lookup(id) != nil { return }
        if stateVector.contains(id) { throw CollabError.malformed("reference to a deletion unit") }
        throw CollabError.missingDependency(id)
    }

    func requireScalars(_ target: CollabID, _ n: UInt64) throws {
        var k: UInt64 = 0
        while k < n {
            guard let (it, off) = lookup(target.offset(k)) else {
                try requireScalar(target.offset(k))
                return
            }
            k += UInt64(it.len - off)
        }
    }

    func verifyDuplicate(_ op: TextOp, units n: UInt64) throws {
        switch op {
        case let .insert(id, ol, or, content):
            var k: UInt64 = 0
            for s in content.unicodeScalars {
                if k == n { break }
                let uid = id.offset(k)
                guard let (it, off) = lookup(uid) else { throw CollabError.idConflict(uid) }
                let wantLeft = k == 0 ? ol : id.offset(k - 1)
                let stored = UTF8Scan.scalar(it.bytes, at: UTF8Scan.byteIndex(it.bytes, scalar: off))
                if stored != s.value || it.originLeft(ofUnit: off) != wantLeft || it.originRight != or {
                    throw CollabError.idConflict(uid)
                }
                k += 1
            }
        case let .delete(id, target, _):
            let list = deleteRuns[id.replica] ?? []
            for k in 0..<n {
                let c = id.counter + k
                var lo = 0, hi = list.count
                while lo < hi {
                    let mid = (lo + hi) >> 1
                    if list[mid].counter <= c { lo = mid + 1 } else { hi = mid }
                }
                guard lo > 0 else { throw CollabError.idConflict(id.offset(k)) }
                let run = list[lo - 1]
                guard c < run.counter + run.len, run.target.offset(c - run.counter) == target.offset(k) else {
                    throw CollabError.idConflict(id.offset(k))
                }
            }
        }
    }

    /// FugueMax for a whole run (contract §2.3).
    func integrate(id: CollabID, originLeft: CollabID?, originRight: CollabID?, bytes: [UInt8]) throws {
        var leftItem: Item? = nil
        if let l = originLeft {
            let (it, off) = lookup(l)!
            if off < it.len - 1 { split(it, at: off + 1) }
            leftItem = it
        }
        var rightItem: Item? = nil
        if let r = originRight {
            let (it, off) = lookup(r)!
            rightItem = off > 0 ? split(it, at: off) : it
            // A malformed op can name a right origin inside the left's run,
            // and that split moves the left origin into the tail.
            if let l = originLeft { leftItem = lookup(l)!.0 }
        }
        var cursor: Item? = leftItem.map { next($0) } ?? firstItem
        var dest = cursor
        if cursor !== rightItem {
            let leftRank = leftItem.map { rank($0) + $0.len - 1 } ?? -1
            let rightRank = rightItem.map { rank($0) } ?? root.raw
            if leftRank >= rightRank { throw CollabError.malformed("origins out of order") }
            var scanning = false
            while true {
                if !scanning { dest = cursor }
                guard let o = cursor, o !== rightItem else { break }
                let ol = o.originLeft.map { rankOf($0) } ?? -1
                if ol < leftRank { break }
                if ol == leftRank {
                    let orr = o.originRight.map { rankOf($0) } ?? root.raw
                    if orr < rightRank {
                        scanning = true
                    } else if orr == rightRank {
                        if id.replica < o.replica { break }
                        scanning = false
                    } else {
                        scanning = false
                    }
                }
                cursor = next(o)
            }
        }
        insertRun(before: dest, id: id, originLeft: originLeft, originRight: originRight, bytes: bytes)
    }

    // MARK: Sync

    /// Every unit not covered by `sv`, in a causal order.
    public func diff(since sv: StateVector) -> [TextOp] {
        var out: [TextOp] = []
        for e in log {
            let start = e.id.counter
            let end = start + e.len
            let have = sv[e.id.replica]
            if end <= have { continue }
            let skip = have > start ? have - start : 0
            switch e {
            case let .insert(id, len, left, right):
                let first = id.offset(skip)
                let content = String(decoding: gatherBytes(first, count: Int(len - skip)), as: UTF8.self)
                out.append(.insert(id: first, originLeft: skip > 0 ? id.offset(skip - 1) : left,
                                   originRight: right, content: content))
            case let .delete(id, target, len):
                out.append(.delete(id: id.offset(skip), target: target.offset(skip), length: len - skip))
            }
        }
        return out
    }

    /// The content of `count` consecutive scalar ids from `first`.
    func gatherBytes(_ first: CollabID, count: Int) -> [UInt8] {
        var out: [UInt8] = []
        var k = 0
        while k < count {
            let (it, off) = lookup(first.offset(UInt64(k)))!
            let take = min(it.len - off, count - k)
            let a = UTF8Scan.byteIndex(it.bytes, scalar: off)
            let b = take == it.len - off ? it.bytes.count : UTF8Scan.byteIndex(it.bytes, scalar: off + take)
            out.append(contentsOf: it.bytes[a..<b])
            k += take
        }
        return out
    }

    // MARK: Positions and units

    public func relativePosition(at position: Int, assoc: Assoc) -> RelativePosition {
        switch assoc {
        case .after:
            guard position > 0 else { return RelativePosition(anchor: nil, assoc: .after) }
            let (it, off) = findVisible(position - 1)
            return RelativePosition(anchor: CollabID(replica: it.replica, counter: it.counter + UInt64(off)), assoc: .after)
        case .before:
            guard position < count else { return RelativePosition(anchor: nil, assoc: .before) }
            let (it, off) = findVisible(position)
            return RelativePosition(anchor: CollabID(replica: it.replica, counter: it.counter + UInt64(off)), assoc: .before)
        }
    }

    /// The visible scalar offset a relative position stands for now; nil if
    /// its anchor is unknown here.
    public func resolve(_ rp: RelativePosition) -> Int? {
        guard let a = rp.anchor else { return rp.assoc == .after ? 0 : count }
        guard let (it, off) = lookup(a) else { return nil }
        let before = visibleRank(it) + (it.deleted ? 0 : off)
        return rp.assoc == .before ? before : before + (it.deleted ? 0 : 1)
    }

    /// UTF-16 offset of visible scalar offset `scalar`.
    public func utf16Offset(ofScalar scalar: Int) -> Int {
        unitOffset(ofScalar: scalar, utf16: true)
    }

    /// UTF-8 offset of visible scalar offset `scalar`.
    public func utf8Offset(ofScalar scalar: Int) -> Int {
        unitOffset(ofScalar: scalar, utf16: false)
    }

    func unitOffset(ofScalar scalar: Int, utf16: Bool) -> Int {
        precondition(scalar >= 0 && scalar <= count)
        if scalar == count { return utf16 ? root.vu16 : root.vu8 }
        var n = root
        var p = scalar
        var acc = 0
        while !n.isLeaf {
            var chosen = n.children[n.children.count - 1]
            for c in n.children {
                if p < c.vis { chosen = c; break }
                p -= c.vis
                acc += utf16 ? c.vu16 : c.vu8
            }
            n = chosen
        }
        for it in n.items {
            if p < it.visLen {
                let b = UTF8Scan.byteIndex(it.bytes, scalar: p)
                return acc + (utf16 ? UTF8Scan.utf16Count(it.bytes[..<b]) : b)
            }
            p -= it.visLen
            acc += utf16 ? it.visU16 : it.visU8
        }
        fatalError("unitOffset: past the end")
    }

    /// Visible scalar offset of UTF-16 offset `u` (rounded down to a scalar
    /// boundary inside a surrogate pair).
    public func scalarOffset(ofUTF16 u: Int) -> Int { scalarOffset(ofUnit: u, utf16: true) }

    /// Visible scalar offset of UTF-8 offset `b` (rounded down inside a
    /// scalar).
    public func scalarOffset(ofUTF8 b: Int) -> Int { scalarOffset(ofUnit: b, utf16: false) }

    func scalarOffset(ofUnit u: Int, utf16: Bool) -> Int {
        let total = utf16 ? root.vu16 : root.vu8
        precondition(u >= 0 && u <= total)
        if u == total { return root.vis }
        var n = root
        var p = u
        var acc = 0
        while !n.isLeaf {
            var chosen = n.children[n.children.count - 1]
            for c in n.children {
                let cu = utf16 ? c.vu16 : c.vu8
                if p < cu { chosen = c; break }
                p -= cu
                acc += c.vis
            }
            n = chosen
        }
        for it in n.items {
            let iu = utf16 ? it.visU16 : it.visU8
            if p < iu {
                var units = 0
                var scalars = 0
                for b in it.bytes where UTF8Scan.isLead(b) {
                    let w = utf16 ? (b >= 0xF0 ? 2 : 1) : (b < 0x80 ? 1 : b < 0xE0 ? 2 : b < 0xF0 ? 3 : 4)
                    if units + w > p { break }
                    units += w
                    scalars += 1
                }
                return acc + scalars
            }
            p -= iu
            acc += it.visLen
        }
        fatalError("scalarOffset: past the end")
    }

    /// Replace a UTF-16 range (an NSTextView/UITextView edit) with `text`:
    /// the delete operations, then the insert. The editor's entry point.
    public func replace(utf16Range range: Range<Int>, with text: String) throws -> [TextOp] {
        let a = scalarOffset(ofUTF16: range.lowerBound)
        let b = scalarOffset(ofUTF16: range.upperBound)
        var ops = delete(at: a, length: b - a)
        if let ins = try insert(text, at: a) { ops.append(ins) }
        return ops
    }

    // MARK: Undo support (TextUndoManager)

    /// Delete whichever scalars of the id span are still visible.
    func deleteVisible(_ first: CollabID, count n: UInt64) -> [TextOp] {
        var spans: [(CollabID, UInt64)] = []
        var k: UInt64 = 0
        while k < n {
            guard let (it0, off) = lookup(first.offset(k)) else { break }
            let need = Int(n - k)
            if it0.deleted {
                k += UInt64(min(it0.len - off, need))
                continue
            }
            let it = off > 0 ? split(it0, at: off) : it0
            if it.len > need { split(it, at: need) }
            markDeleted(it)
            if let last = spans.last, last.0.counter + last.1 == it.counter {
                spans[spans.count - 1].1 += UInt64(it.len)
            } else {
                spans.append((it.firstID, UInt64(it.len)))
            }
            k += UInt64(it.len)
        }
        return spans.map { localDeleteOp(target: $0.0, len: $0.1) }
    }

    /// True if scalar `id` is a tombstone; nil if unknown here.
    func isDeleted(_ id: CollabID) -> Bool? { lookup(id).map { $0.0.deleted } }

    /// Insert a copy of each deleted piece of the id span right after its
    /// own tombstone, as new local operations; each with the first id of
    /// the piece it copies.
    func reinsertDeleted(_ first: CollabID, count n: UInt64) -> [(CollabID, TextOp)] {
        var ops: [(CollabID, TextOp)] = []
        var k: UInt64 = 0
        while k < n {
            guard let (it0, off) = lookup(first.offset(k)) else { break }
            let take = min(it0.len - off, Int(n - k))
            let it = off > 0 ? split(it0, at: off) : it0
            if it.len > take { split(it, at: take) }
            k += UInt64(take)
            guard it.deleted, storedBytes + it.bytes.count <= CollabLimits.maxDocumentBytes else { continue }
            let bytes = it.bytes
            ops.append((it.firstID, localInsert(bytes: bytes, text: String(decoding: bytes, as: UTF8.self),
                                                left: it, right: next(it))))
        }
        return ops
    }
}
