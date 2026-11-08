import CoreGraphics
import Foundation
import FlashTeXDisplayListV3

// Source mapping for the preview (SyncTeX-equivalent, DESIGN.md §6.1,
// protocol §5.3): every GLYPH carries the span of the SPAN item before it
// (a file and a line, declared in SOURCES) and its byte column. Forward
// search (source → preview) finds the glyphs of a line (and the one nearest
// a column); reverse search (preview → source) finds the glyph under a point.

/// The SOURCES declarations of a connection (or a file of frames): span id →
/// (file, line). A later entry for a span id replaces the earlier one (spans
/// move with their lines across edits).
///
/// `SPAN_SHIFT`s (`span-shift-v1`, protocol §5.3.1) are taken lazily: a
/// shift is appended to `shifts` and no span is touched. A line inserted
/// near the top of a long source moved every later span, each a set removal
/// and insertion on main (about 30 ms for 60,000 spans, debug build). Each
/// span keeps the line it had when the map had taken `epoch` shifts, and a
/// lookup carries a line through the shifts since: forward for a span's
/// line, backward (the lines that land on it) for a line's spans.
///
/// Most spans live in `base`, an immutable table (lines as of epoch 0,
/// shared, never copied by a mutation); spans declared since are in a
/// small overlay that supersedes it. `compacted()`, O(spans) and meant for
/// a background queue (EngineV3Session), folds the overlay and the shifts
/// into a new base.
public struct DL3SourceMap: Sendable {
    /// Spans with their lines as of epoch 0, and their index.
    final class Base: Sendable {
        let entries: [UInt32: Line]
        let byLine: [UInt64: UInt32]
        let more: [UInt64: [UInt32]]
        init(entries: [UInt32: Line] = [:], byLine: [UInt64: UInt32] = [:], more: [UInt64: [UInt32]] = [:]) {
            self.entries = entries; self.byLine = byLine; self.more = more
        }
        static let empty = Base()
        var count: Int { entries.count }
    }
    struct Line: Sendable { var file: UInt32, line: UInt32 }
    private struct Entry: Sendable { var file: UInt32, line: UInt32, epoch: UInt32 }
    private struct Key: Hashable, Sendable { var file: UInt32, line: UInt32, epoch: UInt32 }

    public private(set) var files: [UInt32: String] = [:]
    private var base = Base.empty
    /// The overlay: span → its file and its line as of `epoch` shifts.
    private var entries: [UInt32: Entry] = [:]
    /// (file, line, epoch) → a span declared there; the others there (the host
    /// keeps one live span per line: a retired one may share it) in `more`.
    private var byLine: [Key: UInt32] = [:]
    private var more: [Key: [UInt32]] = [:]
    /// The shifts taken since the base was made: shift `k` takes epoch `k`
    /// to `k + 1`.
    public private(set) var shifts: [DL3SpanShifts.Shift] = []
    private var epoch: UInt32 { UInt32(shifts.count) }
    /// Spans declared since the base was made (a reason to compact).
    public var overlayCount: Int { entries.count }

    public init() {}

    public mutating func apply(_ s: DL3Sources) {
        for (id, path) in s.files { files[id] = path }
        for (span, file, line) in s.spans {
            let e = Entry(file: file, line: line, epoch: epoch)
            if let old = entries.updateValue(e, forKey: span) {
                if old.file == file, old.line == line, old.epoch == e.epoch { continue }
                unindex(span, Key(file: old.file, line: old.line, epoch: old.epoch))
            }
            index(span, Key(file: file, line: line, epoch: e.epoch))
        }
    }

    /// Takes a `SPAN_SHIFT`: O(its shifts), whatever the spans.
    public mutating func shift(_ s: DL3SpanShifts) { shifts += s.shifts }

    public mutating func reset() { self = DL3SourceMap() }

    private mutating func index(_ span: UInt32, _ k: Key) {
        if byLine[k] == nil { byLine[k] = span } else { more[k, default: []].append(span) }
    }

    private mutating func unindex(_ span: UInt32, _ k: Key) {
        if byLine[k] == span {
            if var m = more[k], let last = m.popLast() {
                byLine[k] = last
                more[k] = m.isEmpty ? nil : m
            } else {
                byLine[k] = nil
            }
        } else if var m = more[k], let i = m.firstIndex(of: span) {
            m.remove(at: i)
            more[k] = m.isEmpty ? nil : m
        }
    }

    static func key(_ file: UInt32, _ line: UInt32) -> UInt64 { UInt64(file) << 32 | UInt64(line) }

    /// A line of `file` at epoch `from`, carried through the shifts since.
    private func now(file: UInt32, line: UInt32, from: UInt32) -> UInt32 {
        var l = line
        var k = Int(from)
        while k < shifts.count { l = shifts[k].apply(file: file, line: l); k += 1 }
        return l
    }

    private func place(_ span: UInt32) -> Line? {
        if let e = entries[span] { return Line(file: e.file, line: now(file: e.file, line: e.line, from: e.epoch)) }
        if let b = base.entries[span] { return Line(file: b.file, line: now(file: b.file, line: b.line, from: 0)) }
        return nil
    }

    /// Every span where it is now (tests; O(spans)).
    public var spans: [UInt32: (file: UInt32, line: UInt32)] {
        var out: [UInt32: (file: UInt32, line: UInt32)] = [:]
        for span in base.entries.keys where entries[span] == nil { if let p = place(span) { out[span] = (p.file, p.line) } }
        for span in entries.keys { if let p = place(span) { out[span] = (p.file, p.line) } }
        return out
    }

    /// Where a span points: the file's path (as the engine read it) and the line.
    public func location(of span: UInt32) -> (path: String, line: Int)? {
        guard span != 0, let p = place(span), let path = files[p.file] else { return nil }
        return (path, Int(p.line))
    }

    /// The spans of `line` in files for which `matches(path)` holds: at
    /// each epoch, from now back, the spans declared then on a line the
    /// shifts since carry to `line` (a shift's preimage of `x` is `x` below
    /// its `from`, and `x - delta` when that is at or past `from`), and at
    /// epoch 0 the base's that the overlay does not supersede.
    public func spans(line: Int, where matches: (String) -> Bool) -> Set<UInt32> {
        var out = Set<UInt32>()
        guard line >= 0, line <= Int(UInt32.max) else { return out }
        for (id, path) in files where matches(path) {
            var lines: Set<UInt32> = [UInt32(line)]
            var e = epoch
            while true {
                for l in lines {
                    let k = Key(file: id, line: l, epoch: e)
                    if let s = byLine[k] { out.insert(s) }
                    if let m = more[k] { out.formUnion(m) }
                }
                if e == 0 {
                    for l in lines {
                        let k = Self.key(id, l)
                        if let s = base.byLine[k], entries[s] == nil { out.insert(s) }
                        for s in base.more[k] ?? [] where entries[s] == nil { out.insert(s) }
                    }
                    break
                }
                e -= 1
                let sh = shifts[Int(e)]
                guard sh.file == id else { continue }
                var before = Set<UInt32>()
                for x in lines {
                    if x < sh.from { before.insert(x) }
                    let y = Int(x) - sh.delta
                    if y >= Int(sh.from), y <= Int(UInt32.max) { before.insert(UInt32(y)) }
                }
                lines = before
                if lines.isEmpty { break }
            }
        }
        return out
    }

    /// The same spans with the overlay and the shifts folded into a new
    /// base: no shift pending. O(spans), off main.
    public func compacted() -> DL3SourceMap {
        var entries: [UInt32: Line] = [:]
        entries.reserveCapacity(base.count + self.entries.count)
        for span in base.entries.keys where self.entries[span] == nil { if let p = place(span) { entries[span] = p } }
        for span in self.entries.keys { if let p = place(span) { entries[span] = p } }
        var byLine: [UInt64: UInt32] = [:], more: [UInt64: [UInt32]] = [:]
        byLine.reserveCapacity(entries.count)
        for (span, p) in entries {
            let k = Self.key(p.file, p.line)
            if byLine[k] == nil { byLine[k] = span } else { more[k, default: []].append(span) }
        }
        var m = DL3SourceMap()
        m.files = files
        m.base = Base(entries: entries, byLine: byLine, more: more)
        return m
    }
}

/// One glyph of a page, for hit testing and highlighting.
public struct DL3GlyphRef: Sendable {
    public var span: UInt32
    /// Byte column in the span's line (`0xFFFF`: unknown).
    public var col: UInt16
    /// The glyph's origin (page points, y down from the top-left).
    public var origin: CGPoint
    /// Its cell: the advance by a 1 em band on the baseline (hit testing, highlight).
    public var cell: CGRect
    /// Its ink (the outline's bounding box), empty for a blank glyph.
    public var ink: CGRect
    /// The GLYPH item's font id and code (its text, for VoiceOver).
    public var font: UInt16 = 0
    public var code: UInt16 = 0
}

/// The glyphs of one page, in painting order.
public struct DL3SourceIndex: Sendable {
    public let glyphs: [DL3GlyphRef]
    /// Span id → its glyphs' indexes in `glyphs` (painting order): a line's
    /// glyphs without a pass over the page's.
    let bySpan: [UInt32: [Int32]]

    public init(_ prepared: DL3PreparedPage) {
        let page = prepared.page
        var out: [DL3GlyphRef] = []
        out.reserveCapacity(page.items.count)
        var span: UInt32 = 0
        var m = DL3Matrix.identity
        var metrics: [UInt64: (bbox: CGRect, advance: CGFloat, upem: CGFloat)] = [:]
        for it in page.items {
            switch it {
            case .span(let s): span = s
            case .matrix(let n): m = page.matrix(n)
            case .glyph(let f, let code, let x, let y, let col):
                guard let font = prepared.fonts[f], font.glyphIdCount > 0 || Int(code) < 256 else { continue }
                let g: CGGlyph = font.cgFont == nil ? 1 : font.glyph(Int(code))
                let key = UInt64(f) << 16 | UInt64(code)
                let met: (bbox: CGRect, advance: CGFloat, upem: CGFloat)
                if let hit = metrics[key] { met = hit } else if let t3 = font.type3?[Int(code)] {
                    // Type 3: the mask's rect in glyph space; the FontMatrix maps it (upem 1).
                    met = (t3.mask == nil ? .zero : t3.rect, t3.rect.maxX, 1)
                    metrics[key] = met
                } else if let cg = font.cgFont {
                    var glyph = g, box = CGRect.zero, adv: Int32 = 0
                    _ = cg.getGlyphBBoxes(glyphs: &glyph, count: 1, bboxes: &box)
                    _ = cg.getGlyphAdvances(glyphs: &glyph, count: 1, advances: &adv)
                    met = (box, CGFloat(adv), CGFloat(max(cg.unitsPerEm, 1)))
                    metrics[key] = met
                } else { continue }
                let t = font.fontTransform
                let a = m.a, b = m.b, c = m.c, d = m.d
                let ox = Double(x) / K, oy = Double(y) / K
                // A point (u, w) of text space (after the FontMatrix) → page space, y down.
                func map(_ u: Double, _ w: Double) -> CGPoint {
                    let p = CGPoint(x: u, y: w).applying(t)
                    return CGPoint(x: ox + p.x * a + p.y * c, y: oy - (p.x * b + p.y * d))
                }
                func rect(_ u0: Double, _ w0: Double, _ u1: Double, _ w1: Double) -> CGRect {
                    let ps = [map(u0, w0), map(u1, w0), map(u0, w1), map(u1, w1)]
                    let xs = ps.map(\.x), ys = ps.map(\.y)
                    return CGRect(x: xs.min()!, y: ys.min()!, width: xs.max()! - xs.min()!, height: ys.max()! - ys.min()!)
                }
                let s = met.upem
                let ink = met.bbox.isEmpty || g == 0 ? .zero
                    : rect(met.bbox.minX / s, met.bbox.minY / s, met.bbox.maxX / s, met.bbox.maxY / s)
                // The cell: the advance by a 1 em band (Type 3: its text-space em is the FontMatrix's).
                let em = font.type3 == nil ? 1.0 : 1 / max(abs(font.fontTransform.a), 1e-9)
                let cell = font.type3 == nil ? rect(0, -0.25, max(met.advance / s, 0.25), 0.75)
                    : rect(0, -0.25 * em, max(met.advance, 0.25 * em), 0.75 * em)
                out.append(DL3GlyphRef(span: span, col: col, origin: CGPoint(x: ox, y: oy), cell: cell, ink: ink, font: f, code: code))
            default: break
            }
        }
        glyphs = out
        var by: [UInt32: [Int32]] = [:]
        for (i, g) in out.enumerated() { by[g.span, default: []].append(Int32(i)) }
        bySpan = by
    }

    /// Reverse search: the glyph under `point` (page points), else the
    /// nearest one within `maxDistance`.
    public func hit(_ point: CGPoint, maxDistance: CGFloat = 16) -> DL3GlyphRef? {
        var best: DL3GlyphRef?, bestD = CGFloat.infinity
        for g in glyphs where g.span != 0 {
            let r = g.cell
            if r.contains(point) { return g }
            let dx = max(r.minX - point.x, 0, point.x - r.maxX), dy = max(r.minY - point.y, 0, point.y - r.maxY)
            let dd = dx * dx + dy * dy
            if dd < bestD { bestD = dd; best = g }
        }
        return bestD <= maxDistance * maxDistance ? best : nil
    }

    /// Forward search: the glyphs of `spans`; with `col`, only the glyph at
    /// (or the last before) that column, when the columns are known.
    public func glyphs(of spans: Set<UInt32>, col: Int? = nil) -> [DL3GlyphRef] {
        var at: [Int32] = []
        for s in spans { if let i = bySpan[s] { at += i } }
        if spans.count > 1 { at.sort() } // painting order, as a filter over the page gave
        let all = at.map { glyphs[Int($0)] }
        guard let col, !all.isEmpty else { return all }
        return Self.pick(all, col: col).map { [$0] } ?? all
    }

    /// The glyph at `col` of a line's glyphs (painting order): the last one
    /// at or before it, else the first after it; nil when no column is known.
    /// Several glyphs can carry one column (an inline formula's all carry its
    /// closing `$`): at that column, the first of them; after it, the last,
    /// so a caret after the `$` stands after the formula.
    public static func pick(_ glyphs: [DL3GlyphRef], col: Int) -> DL3GlyphRef? {
        let known = glyphs.filter { $0.col != 0xFFFF }
        guard !known.isEmpty else { return nil }
        let before = known.filter { Int($0.col) <= col }
        if let top = before.map(\.col).max() {
            let tied = before.filter { $0.col == top }
            return Int(top) < col ? tied.last : tied.first
        }
        return known.min(by: { $0.col < $1.col })
    }

    /// The union of the glyphs' cells (a line's box on the page).
    public static func box(_ gs: [DL3GlyphRef]) -> CGRect? {
        gs.reduce(nil as CGRect?) { acc, g in acc.map { $0.union(g.cell) } ?? g.cell }
    }
}
