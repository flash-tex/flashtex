import Foundation

/// Local-only undo expressed in CRDT ids (proposal §2.4).
///
/// An undo step remembers the id spans its user inserted and the id spans
/// they deleted. Undoing it deletes those of its own insertions that are
/// still visible (never anyone else's characters, even ones typed inside
/// them) and re-inserts each deleted piece, as new operations, right after
/// its own tombstone. The step's inverse goes on the redo stack, so redo is
/// the same procedure. Remote operations never enter either stack.
///
/// Grouping: each `record` is one step unless it falls between
/// `beginGroup` and `endGroup` (a capture insertion, a linked
/// `\begin`/`\end` rename). Typing coalescing is the editor's (P1).
public final class TextUndoManager {
    struct Span {
        var first: CollabID
        var count: UInt64
    }

    struct Step {
        var inserted: [Span] = []
        var deleted: [Span] = []
        var isEmpty: Bool { inserted.isEmpty && deleted.isEmpty }

        mutating func add(_ ops: [TextOp]) {
            for op in ops {
                switch op {
                case let .insert(id, _, _, _): inserted.append(Span(first: id, count: op.length))
                case let .delete(_, target, n): deleted.append(Span(first: target, count: n))
                }
            }
        }
    }

    public let document: TextDocument
    private var undoStack: [Step] = []
    private var redoStack: [Step] = []
    private var open: Step?
    private var depth = 0
    /// Original unit -> the copy that restored it.
    private var copies: [CollabID: CollabID] = [:]
    public var limit = 256

    public init(document: TextDocument) { self.document = document }

    public var canUndo: Bool { !undoStack.isEmpty }
    public var canRedo: Bool { !redoStack.isEmpty }

    public func beginGroup() {
        if depth == 0 { open = Step() }
        depth += 1
    }

    public func endGroup() {
        precondition(depth > 0, "endGroup without beginGroup")
        depth -= 1
        if depth == 0, let s = open {
            open = nil
            push(s)
        }
    }

    /// Record the operations of one local edit (what `insert`, `delete` or
    /// `replace` returned). A new edit clears the redo stack.
    public func record(_ ops: [TextOp]) {
        guard !ops.isEmpty else { return }
        if depth > 0 {
            open!.add(ops)
        } else {
            var s = Step()
            s.add(ops)
            push(s)
        }
    }

    private func push(_ s: Step) {
        guard !s.isEmpty else { return }
        undoStack.append(s)
        let trimmed = undoStack.count > limit
        if trimmed { undoStack.removeFirst(undoStack.count - limit) }
        let cleared = !redoStack.isEmpty
        redoStack.removeAll()
        if trimmed || cleared { prune() }
    }

    /// Copies recorded (tests check pruning keeps this bounded).
    public var copyCount: Int { copies.count }

    /// Keep only the copies reachable from a span still on either stack, so
    /// the map shrinks with the step limit instead of growing forever.
    private func prune() {
        var keep = Set<CollabID>()
        for s in undoStack + redoStack {
            for span in s.inserted + s.deleted {
                for k in 0..<span.count {
                    var u = span.first.offset(k)
                    while let c = copies[u], keep.insert(u).inserted { u = c }
                }
            }
        }
        copies = copies.filter { keep.contains($0.key) }
    }

    /// Undo the latest step; returns the operations to broadcast.
    public func undo() -> [TextOp] {
        guard let s = undoStack.popLast() else { return [] }
        let (ops, inverse) = revert(s)
        if !inverse.isEmpty { redoStack.append(inverse) }
        return ops
    }

    public func redo() -> [TextOp] {
        guard let s = redoStack.popLast() else { return [] }
        let (ops, inverse) = revert(s)
        if !inverse.isEmpty { undoStack.append(inverse) }
        return ops
    }

    /// The span with each deleted unit replaced by the copy an undo or redo
    /// restored it as (transitively), so a later step that names the
    /// original reaches the text that stands for it now (Yjs's
    /// `followRedone`).
    func followCopies(_ span: Span) -> [Span] {
        var out: [Span] = []
        for k in 0..<span.count {
            var u = span.first.offset(k)
            var hops = 0
            while document.isDeleted(u) == true, let c = copies[u], hops < 10_000 {
                u = c
                hops += 1
            }
            if let last = out.last, last.first.replica == u.replica, last.first.counter + last.count == u.counter {
                out[out.count - 1].count += 1
            } else {
                out.append(Span(first: u, count: 1))
            }
        }
        return out
    }

    /// `span` minus every span in `others` (same replica, counter ranges).
    static func subtract(_ span: Span, _ others: [Span]) -> [Span] {
        var pieces = [span]
        for o in others where o.first.replica == span.first.replica {
            let oa = o.first.counter, ob = o.first.counter + o.count
            pieces = pieces.flatMap { p -> [Span] in
                let pa = p.first.counter, pb = p.first.counter + p.count
                if ob <= pa || pb <= oa { return [p] }
                var out: [Span] = []
                if pa < oa { out.append(Span(first: p.first, count: oa - pa)) }
                if ob < pb { out.append(Span(first: CollabID(replica: p.first.replica, counter: ob), count: pb - ob)) }
                return out
            }
        }
        return pieces
    }

    private func revert(_ s: Step) -> ([TextOp], Step) {
        var ops: [TextOp] = []
        // Re-insert first, deleting second: positions are ids, so the order
        // only decides which operations come first on the wire.
        // Text the step both inserted and deleted is gone either way: it is
        // not re-inserted.
        for span in s.deleted.reversed() {
            for piece in Self.subtract(span, s.inserted) {
                for (orig, op) in document.reinsertDeleted(piece.first, count: piece.count) {
                    for k in 0..<op.length { copies[orig.offset(k)] = op.id.offset(k) }
                    ops.append(op)
                }
            }
        }
        for span in s.inserted.reversed() {
            for live in followCopies(span) {
                ops += document.deleteVisible(live.first, count: live.count)
            }
        }
        var inverse = Step()
        inverse.add(ops)
        return (ops, inverse)
    }
}
