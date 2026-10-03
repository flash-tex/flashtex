import AppKit
import QuartzCore
import FlashTeXPreviewV3

/// The caret on the page in the engine-v3 preview (DESIGN §10 app parity,
/// gap C12): the v2 pane's "you are here", always shown, not only as the
/// flash of an explicit forward search. A faint band over each row of the
/// caret's source line, the caret's glyph tinted, and a caret bar at the
/// caret's column (before the glyph that starts there, else after the last
/// glyph before it), in page points. Pure apart from the source index.
struct EngineV3CaretMark: Equatable {
    var page: Int
    /// The caret bar's x and its row (nil when the engine knew no column).
    var bar: CGRect?
    var glyph: CGRect?
    /// One rect per row of the caret's line on that page.
    var band: [CGRect]

    /// Rows of `glyphs`: grouped by baseline (within half a point), each the union of its cells.
    static func rows(_ glyphs: [DL3GlyphRef]) -> [CGRect] {
        var rows: [(y: CGFloat, rect: CGRect)] = []
        for g in glyphs {
            if let i = rows.firstIndex(where: { abs($0.y - g.origin.y) <= 0.5 }) { rows[i].rect = rows[i].rect.union(g.cell) } else { rows.append((g.origin.y, g.cell)) }
        }
        return rows.sorted { $0.y < $1.y }.map(\.rect)
    }

    /// The mark from a page's glyphs of the caret's line and the caret's
    /// column, nil when the line has no glyph there.
    static func make(page: Int, line glyphs: [DL3GlyphRef], index: DL3SourceIndex, spans: Set<UInt32>, col: Int) -> EngineV3CaretMark? {
        guard !glyphs.isEmpty else { return nil }
        var mark = EngineV3CaretMark(page: page, bar: nil, glyph: nil, band: rows(glyphs))
        if let g = index.glyphs(of: spans, col: col).first, g.col != 0xFFFF {
            let x = Int(g.col) >= col ? g.cell.minX : g.cell.maxX
            mark.bar = CGRect(x: x, y: g.cell.minY, width: 0, height: g.cell.height)
            mark.glyph = g.cell
        }
        return mark
    }
}

extension EngineV3Session {
    /// Where the caret at `byte` of `path` is drawn, looking only at `pages`
    /// (the ones the pane holds: a per-keystroke cost bounded by the view).
    func caretMark(path: String, byte: Int, in text: String, pages candidates: [Int]) -> EngineV3CaretMark? {
        let (line, col) = Self.lineAndColumn(byte: byte, in: text)
        let spans = sourceMap.spans(line: line) { self.projectPath(ofEngineFile: $0) == path }
        guard !spans.isEmpty else { return nil }
        for i in candidates.sorted() {
            guard let ix = sourceIndex(page: i) else { continue }
            let gs = ix.glyphs(of: spans)
            if let m = EngineV3CaretMark.make(page: i, line: gs, index: ix, spans: spans, col: col) { return m }
        }
        return nil
    }
}

extension EngineV3PagesView {
    /// The caret moved, or the pages changed: works out the mark again
    /// (only when the inputs differ) and draws it.
    func setCaret(path: String?, utf16: Int, stamp: Int) {
        let key = CaretKey(path: path, utf16: utf16, stamp: stamp, pages: heldPageIndexes)
        guard key != caretKey else { return }
        caretKey = key
        var mark: EngineV3CaretMark?
        if let path, let session, let model = session.model, path == model.activePath,
           let byte = CaretSync.byteOffset(ofCaretUTF16: utf16, in: model.activeText) {
            mark = session.caretMark(path: path, byte: byte, in: model.activeText, pages: heldPageIndexes)
        }
        if mark != caretMark { caretMark = mark; drawCaretMark() }
    }

    struct CaretKey: Equatable { var path: String?; var utf16: Int; var stamp: Int; var pages: [Int] }

    /// Draws `caretMark` at the current layout (also after a relayout).
    func drawCaretMark() {
        wantsLayer = true
        CATransaction.begin(); CATransaction.setDisableActions(true)
        defer { CATransaction.commit() }
        let layer = caretMarkLayer
        if layer.superlayer == nil { self.layer?.addSublayer(layer) }
        guard let m = caretMark, let origin = viewPoint(page: m.page, .zero), let s = viewPoint(page: m.page, CGPoint(x: 1, y: 0)).map({ $0.x - origin.x }) else {
            layer.isHidden = true
            return
        }
        func view(_ r: CGRect) -> CGRect { CGRect(x: origin.x + r.minX * s, y: origin.y + r.minY * s, width: r.width * s, height: r.height * s) }
        let dark = pageAppearance == .dark
        let accent = NSColor.controlAccentColor
        let band = CGMutablePath()
        let overhang = DS.Preview.paragraphBandOverhang * s
        for row in m.band { band.addRoundedRect(in: view(row).insetBy(dx: -overhang, dy: -overhang / 2), cornerWidth: 2, cornerHeight: 2) }
        layer.band.path = band
        layer.band.fillColor = accent.withAlphaComponent(dark ? DS.Preview.paragraphBandOpacityDark : DS.Preview.paragraphBandOpacity).cgColor
        layer.glyph.path = m.glyph.map { CGPath(rect: view($0), transform: nil) }
        layer.glyph.fillColor = accent.withAlphaComponent(DS.Preview.hoverHighlightOpacity).cgColor
        layer.bar.path = m.bar.map { b in CGPath(rect: view(b).insetBy(dx: -0.75, dy: 0), transform: nil) }
        layer.bar.fillColor = accent.cgColor
        layer.frame = bounds
        for l in [layer.band, layer.glyph, layer.bar] { l.frame = layer.bounds }
        layer.isHidden = false
    }
}

/// The caret mark's layers, above the pages (document coordinates).
final class EngineV3CaretMarkLayer: CALayer {
    let band = CAShapeLayer(), glyph = CAShapeLayer(), bar = CAShapeLayer()
    override init() {
        super.init()
        zPosition = 8
        for l in [band, glyph, bar] { addSublayer(l) }
    }
    override init(layer: Any) { super.init(layer: layer) }
    required init?(coder: NSCoder) { fatalError() }
}
