import CoreGraphics
import FlashTeXProtocol

/// Resolves the caret's page item into a `FollowCaret.Target` for each
/// preview pane (lane mac-follow-caret). Both return nil — the documented
/// "no mapping" case — for a caret in the preamble, a comment, or a region
/// the applied result/frame does not cover; the AppKit side (`PreviewAnchorProbe`)
/// then makes a quiet no-op of it (`FollowCaret.Reason.noMapping`).
///
/// Rects are in RAW page points (scale 1, top-down from the page's top-left),
/// the same convention `PreviewPageLayout` scales and positions pages in; the
/// probe multiplies by the pane's current `layout.scale` and offsets by the
/// page's frame to get the document-coordinate rect `FollowCaret.decide` compares
/// against the visible rect.
extension ShellModel {
    /// The caret's page item on the v1 (runtime-v1) preview pane, first page
    /// hit in document order — the same item `revealCaretInPreview()` selects.
    func followCaretTargetV1() -> FollowCaret.Target? {
        guard let result, let byte = caretByte,
              let hit = caretIndex()?.itemsContaining(byte: byte).first,
              let page = result.pages.first(where: { $0.number == hit.page }) else { return nil }
        let rulesNegotiated = result.layoutCapabilities?.contains(RuntimeV1.LayoutCapabilities.rulesV1) == true
        let rects = PreviewHitRects.compute(page: page, scale: 1, rulesNegotiated: rulesNegotiated)
        guard let hitRect = rects.first(where: { $0.index == hit.index }) else { return nil }
        return FollowCaret.Target(page: hit.page, rect: hitRect.rect)
    }

    /// The caret's highlight on the v2 (display-list) preview pane: the union
    /// of whatever `MathCaretHighlight.caretHighlights` draws for the caret on
    /// the first page that has one (exact caret bar + cluster hit rects, or
    /// the enclosing formula box).
    func followCaretTargetV2() -> FollowCaret.Target? {
        guard let byte = caretByte, let frame = displayListV2?.frame else { return nil }
        for page in frame.list.pages {
            let highlights = V2Geometry.caretHighlights(containing: byte, path: activePath, in: page)
            guard !highlights.isEmpty else { continue }
            var rects: [CGRect] = []
            for h in highlights {
                switch h {
                case .cluster(let m):
                    if let k = m.caret {
                        rects.append(CGRect(x: RenderingV2.points(k.x) - 0.75, y: RenderingV2.points(k.top),
                                            width: 1.5, height: RenderingV2.points(k.height)))
                    }
                    rects.append(contentsOf: m.hitRects.map(Self.rawRect))
                case .formula(let box):
                    rects.append(Self.rawRect(box.bounds))
                }
            }
            guard var union = rects.first else { continue }
            for r in rects.dropFirst() { union = union.union(r) }
            return FollowCaret.Target(page: page.number, rect: union)
        }
        return nil
    }

    private static func rawRect(_ r: RenderingV2.Rect) -> CGRect {
        CGRect(x: RenderingV2.points(r.x), y: RenderingV2.points(r.top), width: RenderingV2.points(r.width), height: RenderingV2.points(r.height))
    }
}
