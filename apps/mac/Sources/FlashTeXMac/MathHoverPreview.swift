import AppKit
import CoreGraphics
import Foundation
import SwiftUI
import FlashTeXProtocol

/// Inline math preview on hover (lane mac-math-hover, deferred from
/// mac-editor-dx-3): hovering `$…$`/`\(…\)` crops the formula's already-
/// rendered bitmap out of the current v2 page — nothing is rendered on the
/// hover path itself. Built on the same items-carry-the-formula's-span
/// machinery as the caret's formula box (`V2Geometry.formulaBox`,
/// MathCaretHighlight.swift): `EditorIntelligence.inlineMathSpan` finds the
/// formula's editor range, converted to the UTF-8 byte span every member
/// item's `source` carries, then `V2Geometry.formulaBox` gives their union
/// box on whichever page holds them.
enum MathHoverPreview {
    /// What the owner (`ShellModel`, through `SourceEditorView.mathPreviewContext`)
    /// hands the editor each time hover needs to know the current preview: the
    /// active document's path, the current v2 frame, whether it is stale
    /// (an older revision than the editor text) and the pane's appearance.
    struct Context {
        var path: String
        var frame: V2Frame
        var previewIsStale: Bool
        var dark: Bool
    }

    /// Points of padding added around the formula's box before cropping.
    static let padding: Double = 4

    /// A crop ready to read out of a page's rasterized bitmap: which page,
    /// and the box (points, page space, top-left origin, y down — same
    /// convention as `RenderingV2.Rect`) to crop, already padded and clamped
    /// to the page.
    struct Crop: Equatable {
        var pageIndex: Int
        var rect: RenderingV2.Rect
    }

    /// The crop for the inline formula at `utf16` of `text`, or nil when: the
    /// position is not inside one (`EditorIntelligence.inlineMathSpan`,
    /// which also declines a formula spanning more than two lines); the
    /// preview is stale (an older revision than the editor text); or no
    /// page's items carry that exact byte span (the frame doesn't cover it —
    /// a stale/different revision's frame, or a formula whose bytes were
    /// dropped, e.g. a compile error).
    static func crop(in text: NSString, at utf16: Int, path: String, pages: [RenderingV2.Page],
                     previewIsStale: Bool, highlighter: SyntaxHighlighter? = nil) -> Crop? {
        guard !previewIsStale else { return nil }
        guard let span = EditorIntelligence.inlineMathSpan(in: text, at: utf16, highlighter: highlighter) else { return nil }
        guard let bytes = (text as String).utf8ByteRange(of: span) else { return nil }
        let source = RenderingV2.SourceRange(path: path, startByte: bytes.start, endByte: bytes.end)
        for (index, page) in pages.enumerated() {
            guard let box = V2Geometry.formulaBox(for: source, in: page) else { continue }
            return Crop(pageIndex: index, rect: padded(box.bounds, page: page))
        }
        return nil
    }

    /// `bounds` widened by `padding` points on every side and clamped to the page.
    static func padded(_ bounds: RenderingV2.Rect, page: RenderingV2.Page) -> RenderingV2.Rect {
        let pad = ticks(padding)
        let pageW = ticks(page.widthPt), pageH = ticks(page.heightPt)
        let x0 = max(0, bounds.x - pad)
        let y0 = max(0, bounds.top - pad)
        let x1 = min(pageW, bounds.x &+ bounds.width &+ pad)
        let y1 = min(pageH, bounds.top &+ bounds.height &+ pad)
        return RenderingV2.Rect(x: x0, top: y0, width: max(0, x1 - x0), height: max(0, y1 - y0))
    }

    private static func ticks(_ points: Double) -> Int64 { Int64((points * Double(RenderingV2.ticksPerPoint)).rounded()) }

    /// `rect` (page points) as a pixel rectangle of a bitmap rasterized at
    /// `pixelsPerPoint` — top-left origin, y down, matching how
    /// `GlyphRunRenderer.rasterize`'s bitmap is laid out (screen convention;
    /// `PageV2Marks.viewRect` overlays the same conversion on screen).
    static func pixelRect(_ rect: RenderingV2.Rect, pixelsPerPoint: Double) -> CGRect {
        CGRect(x: RenderingV2.points(rect.x) * pixelsPerPoint, y: RenderingV2.points(rect.top) * pixelsPerPoint,
              width: RenderingV2.points(rect.width) * pixelsPerPoint, height: RenderingV2.points(rect.height) * pixelsPerPoint)
    }

    /// The cropped bitmap for `crop`, read from `frame`'s CURRENT page bitmap
    /// at `pixelsPerPoint`/`dark` in `rasterizer` — a direct read of whatever
    /// is already installed (`V2PageRasterizer.images`, never `image(for:)`,
    /// which would kick off a new rasterization); nil when that bitmap isn't
    /// ready yet, or the padded rect doesn't land inside it. Nothing is drawn.
    @MainActor
    static func image(for crop: Crop, frame: V2Frame, pixelsPerPoint: Double, dark: Bool,
                      rasterizer: V2PageRasterizer) -> CGImage? {
        guard crop.pageIndex < frame.prepared.count else { return nil }
        let token = frame.pageToken(at: crop.pageIndex)
        let key = V2PageRasterizer.Key(pageToken: token, pixelsPerPoint: pixelsPerPoint, dark: dark)
        guard let bitmap = rasterizer.images[key] else { return nil }
        let pixel = pixelRect(crop.rect, pixelsPerPoint: pixelsPerPoint).integral
            .intersection(CGRect(x: 0, y: 0, width: bitmap.width, height: bitmap.height))
        guard !pixel.isEmpty else { return nil }
        return bitmap.cropping(to: pixel)
    }
}

// MARK: - popover

/// Small NSPopover content for the cropped formula bitmap: the image at its
/// natural pixel size (already at the pane's backing scale), no chrome.
struct MathPreviewView: View {
    let image: CGImage

    var body: some View {
        Image(decorative: image, scale: 1, orientation: .up)
            .padding(DS.Space.s)
    }
}

extension HoverController {
    /// Shows `image` (a formula crop) anchored to `range`, replacing whatever
    /// popover is open — the same slot `present(_:in:)` uses, so only one
    /// hover popover is ever on screen. Call in place of `present(_:in:)`
    /// when the position is inside an inline formula.
    func presentMathPreview(_ image: CGImage, range: NSRange, in tv: NSTextView) {
        presentContent(NSHostingController(rootView: MathPreviewView(image: image)), range: range, in: tv)
        recordMathPreview(range)
    }
}
