import SwiftUI
import CoreText
import FlashTeXProtocol
import FlashTeXAccessibility

/// Draws `compile_result` pages. Coordinates are points, origin top-left;
/// text items are positioned by baseline, typed `rule` items by their top-left
/// corner (`RuleGeometry`). Clicking a text or rule item navigates to its
/// UTF-8 source range. Unknown kinds are not drawn; the model reports them
/// when a capability set was negotiated.
struct PreviewView: View {
    let result: RuntimeV1.CompileResult
    let dark: Bool
    /// Items under the editor caret, `page number -> item indices` (see `CaretSync`).
    var caretItems: [Int: Set<Int>] = [:]
    /// "Preview follows the caret" (FollowCaret.swift): the caret's page item
    /// in raw page points (`ShellModel.followCaretTargetV1()`), nil when it
    /// has no preview mapping; `followEnabled` mirrors the preference so a
    /// pane with the setting off never schedules a debounce for nothing.
    var followTarget: FollowCaret.Target? = nil
    var followEnabled: Bool = false
    /// `ShellModel.forceRevealInPreview` (⌘⇧J always scrolls, even when `followEnabled` is false).
    var forceRevealToken: Int = 0
    /// Zoom multiplier over the fit-to-width scale (PreviewZoom.swift).
    var zoom: CGFloat = 1
    /// Reports the fit-to-width scale so the shell can compute Actual Size / the percentage.
    var onFitScale: ((CGFloat) -> Void)? = nil
    let onSelect: (RuntimeV1.SourceRange?, String?) -> Void

    var body: some View {
        let _ = TypingBench.shared.willRender(revision: result.revision, pages: result.pages.count)
        GeometryReader { geo in
            let widest = result.pages.map(\.widthPt).max() ?? 612
            // Fit the widest page to the pane (never upscale past 100%), times the zoom.
            let fit = min(1, max(0.2, (geo.size.width - 48) / widest))
            let scale = PreviewZoom.scale(fit: fit, zoom: zoom)
            // Scroll anchoring (PreviewAnchor.swift): the (page, fraction) under the
            // viewport's top edge survives a result with another page count and a
            // pane resize; a result with the same page geometry never moves the scroll.
            let layout = PreviewPageLayout(pages: result.pages.map { PreviewPageLayout.Page(number: $0.number, widthPt: $0.widthPt, heightPt: $0.heightPt) }, scale: scale)
            ScrollView([.vertical, .horizontal]) {
                // Lazy: only pages near the viewport are laid out and drawn;
                // `.equatable()`: a page whose items, caret set and scale did not
                // change keeps its display list, so a keystroke re-draws only the
                // pages whose layout (or source offsets) actually moved.
                VStack(spacing: 24) {
                    ForEach(result.pages, id: \.number) { page in
                        PageView(page: page, totalPages: result.pages.count, dark: dark, caretItems: caretItems[page.number] ?? [], scale: scale,
                                 rulesNegotiated: result.layoutCapabilities?.contains(RuntimeV1.LayoutCapabilities.rulesV1) == true,
                                 onSelect: onSelect)
                            .equatable()
                            .id(page.number)
                    }
                }
                .padding(24)
                .background(PreviewAnchorKeeper(layout: layout, followTarget: followTarget, followEnabled: followEnabled))
            }
            .onChange(of: fit, initial: true) { _, f in onFitScale?(f) }
        }
        .background(dark ? Color(white: 0.12) : Color(nsColor: .windowBackgroundColor))
    }
}

private struct PageView: View, Equatable {
    let page: RuntimeV1.Page
    let totalPages: Int
    let dark: Bool
    var caretItems: Set<Int> = []
    /// Display scale (1 = 1pt per screen point); the preview fits pages to width.
    var scale: CGFloat = 1.0
    var rulesNegotiated = false
    let onSelect: (RuntimeV1.SourceRange?, String?) -> Void

    /// Everything that affects the drawing; `onSelect` is the same closure for
    /// every page and revision, so it is not part of identity.
    static func == (a: PageView, b: PageView) -> Bool {
        a.page == b.page && a.totalPages == b.totalPages && a.dark == b.dark && a.caretItems == b.caretItems && a.scale == b.scale && a.rulesNegotiated == b.rulesNegotiated
    }

    var body: some View {
        let size = CGSize(width: page.widthPt * scale, height: page.heightPt * scale)
        HitTestCanvas(page: page, dark: dark, scale: scale, caretItems: caretItems, rulesNegotiated: rulesNegotiated, onSelect: onSelect)
            .frame(width: size.width, height: size.height)
            .overlay(alignment: .topLeading) { AccessibilityOverlay(page: page, totalPages: totalPages, scale: scale, fontName: { PreviewFonts.postScriptName(size: $0) }, onSelect: onSelect) } // FlashTeXAccessibility
            .background(dark ? Color(white: 0.16) : .white)
            .shadow(radius: 4)
            .overlay(alignment: .bottomTrailing) {
                Text("page \(page.number)")
                    .font(.caption2).foregroundStyle(.secondary).padding(4)
            }
    }
}

private struct HitTestCanvas: View {
    let page: RuntimeV1.Page
    let dark: Bool
    let scale: CGFloat
    /// Indices into `page.items` to mark as containing the editor caret.
    var caretItems: Set<Int> = []
    /// Whether `rules-v1` was accepted for this result. Only when it was NOT
    /// does the legacy U+2500 approximation apply to text runs.
    var rulesNegotiated = false
    let onSelect: (RuntimeV1.SourceRange?, String?) -> Void

    @State private var hover: Int?

    /// Hit rects keyed by `page.items` index, computed on demand from the
    /// cached line metrics (`PreviewHitRects`) — not published from the draw
    /// closure, which would re-invalidate the view after every paint.
    private var hitRects: [PreviewHitRects.Hit] {
        PreviewHitRects.compute(page: page, scale: scale, rulesNegotiated: rulesNegotiated)
    }

    var body: some View {
        Canvas { context, _ in
            let cache = PreviewTextCache.shared
            let ink: Color = dark ? .white : .black
            let inkCG: CGColor = dark ? CGColor(gray: 1, alpha: 1) : CGColor(gray: 0, alpha: 1)
            for (index, item) in page.items.enumerated() {
                if case .rule(let rule) = item {
                    // Typed rule: top-left anchored contract geometry (dark preview only recolors).
                    let rect = RuleGeometry.previewRect(rule, scale: scale)
                    context.fill(Path(rect), with: .color(ink))
                    if hover == index {
                        context.fill(Path(rect.insetBy(dx: -2, dy: -2)), with: .color(Color.accentColor.opacity(0.25)))
                    }
                    continue
                }
                guard case .text(let t) = item else { continue }
                if !rulesNegotiated, let r = RuleConvention.rect(for: t) {
                    // Legacy approximation of the compiler's fraction bars.
                    let rect = CGRect(x: r.x * scale, y: r.y * scale, width: r.width * scale, height: max(0.5, r.height * scale))
                    context.fill(Path(rect), with: .color(ink))
                    continue
                }
                // Draw with the hinted face when the result carries one (font-hints-v1),
                // else the face the layout was measured with (Latin Modern by default,
                // Times fallback). One cached CTLine per distinct (text, face, size).
                let name = PreviewFonts.resolve(hint: t.font, size: t.fontSizePt).postScriptName
                let line = cache.line(for: PreviewTextCache.key(text: t.text, postScriptName: name, size: t.fontSizePt * scale))
                let baselineY = t.baselineYPt * scale
                let rect = CGRect(x: t.xPt * scale, y: baselineY - line.ascent, width: line.width, height: line.ascent + line.descent)
                if caretItems.contains(index) {
                    // Secondary (caret) highlight: subtle fill plus an underline.
                    context.fill(Path(rect.insetBy(dx: -2, dy: -1)),
                                 with: .color(Color.accentColor.opacity(0.15)))
                    let y = rect.maxY + 1
                    var underline = Path()
                    underline.move(to: CGPoint(x: rect.minX, y: y))
                    underline.addLine(to: CGPoint(x: rect.maxX, y: y))
                    context.stroke(underline, with: .color(Color.accentColor), lineWidth: 1.5)
                }
                if hover == index {
                    context.fill(Path(rect.insetBy(dx: -2, dy: -1)),
                                 with: .color(Color.accentColor.opacity(0.25)))
                }
                context.withCGContext { cg in
                    // The canvas context is y-down; flip the text matrix so glyphs
                    // stand upright and the pen sits on the item's baseline.
                    cg.textMatrix = CGAffineTransform(scaleX: 1, y: -1)
                    cg.setFillColor(inkCG)
                    cg.textPosition = CGPoint(x: t.xPt * scale, y: baselineY)
                    CTLineDraw(line.line, cg)
                }
            }
            TypingBench.shared.didDraw(page: page.number) // paint instrumentation (TypingBench.swift)
        }
        .contentShape(Rectangle())
        .onContinuousHover { phase in
            switch phase {
            case .active(let p):
                let h = PreviewHitRects.hit(hitRects, at: p)?.index
                if h != hover { hover = h }
            case .ended: hover = nil
            }
        }
        .onTapGesture { location in
            if let hit = PreviewHitRects.hit(hitRects, at: location) {
                onSelect(hit.source, hit.text)
            }
        }
    }
}
