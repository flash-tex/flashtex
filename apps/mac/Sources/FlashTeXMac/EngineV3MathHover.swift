import AppKit
import CoreGraphics
import FlashTeXPreviewV3
import FlashTeXProtocol

/// The inline math hover preview under the engine-v3 preview (DESIGN §10
/// app parity, gap C24). As on the v2 pane (MathHoverPreview.swift): hovering
/// `$…$`/`\(…\)` crops the formula out of the page bitmap already on screen,
/// padded by `MathHoverPreview.padding` points; nothing is rendered on the
/// hover path, and nothing is shown while the preview is older than the
/// editor text. The formula's glyphs are those whose source position (the
/// span's line, the glyph's byte column) falls inside the formula's bytes.
extension EngineV3Session {
    /// The formula's box (page points, y down) and its page, for the formula
    /// at UTF-8 bytes `start ..< end` of `path` in `text` (the text the
    /// pages were compiled from), on the pages the pane holds.
    func formulaBox(path: String, start: Int, end: Int, in text: String, pages candidates: [Int]) -> (page: Int, box: CGRect)? {
        let (l0, c0) = Self.lineAndColumn(byte: start, in: text)
        let (l1, c1) = Self.lineAndColumn(byte: end, in: text)
        guard l1 - l0 <= 1 else { return nil } // as v2: a formula over at most two lines
        var spans: [UInt32: Int] = [:] // span → line
        for line in l0 ... l1 {
            for s in sourceMap.spans(line: line, where: { self.projectPath(ofEngineFile: $0) == path }) { spans[s] = line }
        }
        guard !spans.isEmpty else { return nil }
        func inside(_ line: Int, _ col: Int) -> Bool {
            (line > l0 || col >= c0) && (line < l1 || col < c1)
        }
        for i in candidates.sorted() {
            guard let ix = sourceIndex(page: i) else { continue }
            var box: CGRect?
            for g in ix.glyphs where g.col != 0xFFFF {
                guard let line = spans[g.span], inside(line, Int(g.col)) else { continue }
                let r = g.ink.isEmpty ? g.cell : g.ink.union(g.cell)
                box = box.map { $0.union(r) } ?? r
            }
            if let box { return (i, box) }
        }
        return nil
    }

    /// The hover image for the formula at `span` (UTF-16) of the active
    /// document, cropped from its page's installed bitmap; nil when the
    /// preview is older than the editor text, nothing on the held pages
    /// shows the formula, or the page's bitmap on screen is not of its
    /// current content (not drawn yet, a stored page, or drawn for content a
    /// compile has since replaced).
    func mathPreviewImage(span: NSRange) -> CGImage? {
        guard let model, let view, model.engineV3Enabled else { return nil }
        let path = model.activePath, text = model.activeText
        // Stale: the pages were compiled from other text (MathHoverPreview's rule).
        guard let compiled = model.compiledDocuments[path], compiled.sameBytes(as: text),
              let bytes = text.utf8ByteRange(of: span),
              let (page, box) = formulaBox(path: path, start: bytes.start, end: bytes.end, in: text, pages: Array(view.heldPageViews.keys)),
              // The bitmap on screen must be of the page's current content
              // (not a stored page, not one drawn for content since replaced).
              view.pageShowsCurrent(page),
              let size = pageSize(page), let image = view.installedImage(page), size.width > 0 else { return nil }
        let pad = MathHoverPreview.padding
        let rect = box.insetBy(dx: -pad, dy: -pad).intersection(CGRect(origin: .zero, size: size))
        let ppp = Double(image.width) / Double(size.width)
        let pixel = CGRect(x: rect.minX * ppp, y: rect.minY * ppp, width: rect.width * ppp, height: rect.height * ppp).integral
            .intersection(CGRect(x: 0, y: 0, width: image.width, height: image.height))
        guard !pixel.isEmpty else { return nil }
        return image.cropping(to: pixel)
    }
}
