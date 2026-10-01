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
public struct DL3SourceMap: Sendable {
    public private(set) var files: [UInt32: String] = [:]
    public private(set) var spans: [UInt32: (file: UInt32, line: UInt32)] = [:]
    /// (file id, line) → span ids.
    private var byLine: [UInt64: Set<UInt32>] = [:]

    public init() {}

    public mutating func apply(_ s: DL3Sources) {
        for (id, path) in s.files { files[id] = path }
        for (span, file, line) in s.spans {
            if let old = spans[span] { byLine[Self.key(old.file, old.line)]?.remove(span) }
            spans[span] = (file, line)
            byLine[Self.key(file, line), default: []].insert(span)
        }
    }

    public mutating func reset() { self = DL3SourceMap() }

    static func key(_ file: UInt32, _ line: UInt32) -> UInt64 { UInt64(file) << 32 | UInt64(line) }

    /// Where a span points: the file's path (as the engine read it) and the line.
    public func location(of span: UInt32) -> (path: String, line: Int)? {
        guard span != 0, let s = spans[span], let path = files[s.file] else { return nil }
        return (path, Int(s.line))
    }

    /// The spans of `line` in files for which `matches(path)` holds.
    public func spans(line: Int, where matches: (String) -> Bool) -> Set<UInt32> {
        var out = Set<UInt32>()
        for (id, path) in files where matches(path) { out.formUnion(byLine[Self.key(id, UInt32(line))] ?? []) }
        return out
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
}

/// The glyphs of one page, in painting order.
public struct DL3SourceIndex: Sendable {
    public let glyphs: [DL3GlyphRef]

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
                guard let font = prepared.fonts[f], Int(code) < 256 else { continue }
                let g = font.glyphs[Int(code)]
                let key = UInt64(f) << 16 | UInt64(code)
                let met: (bbox: CGRect, advance: CGFloat, upem: CGFloat)
                if let hit = metrics[key] { met = hit } else {
                    var glyph = g, box = CGRect.zero, adv: Int32 = 0
                    _ = font.cgFont.getGlyphBBoxes(glyphs: &glyph, count: 1, bboxes: &box)
                    _ = font.cgFont.getGlyphAdvances(glyphs: &glyph, count: 1, advances: &adv)
                    met = (box, CGFloat(adv), CGFloat(max(font.cgFont.unitsPerEm, 1)))
                    metrics[key] = met
                }
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
                let cell = rect(0, -0.25, max(met.advance / s, 0.25), 0.75)
                out.append(DL3GlyphRef(span: span, col: col, origin: CGPoint(x: ox, y: oy), cell: cell, ink: ink))
            default: break
            }
        }
        glyphs = out
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
        let all = glyphs.filter { spans.contains($0.span) }
        guard let col, !all.isEmpty else { return all }
        let known = all.filter { $0.col != 0xFFFF }
        guard !known.isEmpty else { return all }
        let before = known.filter { Int($0.col) <= col }
        if let g = before.max(by: { $0.col < $1.col }) { return [g] }
        return [known.min(by: { $0.col < $1.col })!]
    }

    /// The union of the glyphs' cells (a line's box on the page).
    public static func box(_ gs: [DL3GlyphRef]) -> CGRect? {
        gs.reduce(nil as CGRect?) { acc, g in acc.map { $0.union(g.cell) } ?? g.cell }
    }
}
