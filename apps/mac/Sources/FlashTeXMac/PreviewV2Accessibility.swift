import AppKit
import SwiftUI
import FlashTeXProtocol
import FlashTeXAccessibility

// VoiceOver over the v2 preview pane (the shipped default, PreviewV2View.swift).
// The v1 pane exposes each page as a landmark with a line group per line and
// a static-text element per item (FlashTeXAccessibility/AccessibilityViews.swift);
// the v2 pages were bitmap layers with no accessibility tree at all, so a
// VoiceOver user landed on an unlabelled group and heard nothing of the
// document. This file gives a v2 page the same shape — landmark, lines, "Go to
// source" — from the text the display list already carries: every glyph run's
// `text` (cluster-actualtext, the same strings the searchable PDF export
// replays). Nothing is re-shaped or measured; lines are the runs grouped by
// baseline, words are the runs' texts joined where the producer left a gap.

/// The v2 pane's name for the page-text model, which moved to
/// EngineV3PageText.swift in S6a; S6 deletes this file.
typealias V2PageText = PreviewPageText

/// The v2 pane's producers of words and rules (`RenderingV2.Page` in).
extension PreviewPageText {
    static func words(of page: RenderingV2.Page) -> [Word] {
        var out: [Word] = []
        for (index, item) in page.items.enumerated() {
            guard case .glyphRun(let run) = item, let first = run.glyphs.first, let last = run.glyphs.last,
                  !run.text.isEmpty else { continue }
            let size = RenderingV2.points(run.fontSize)
            let x0 = RenderingV2.points(first.originX)
            let x1 = RenderingV2.points(last.originX &+ last.advanceX)
            let baseline = RenderingV2.points(last.baselineY)
            let rect = CGRect(x: x0, y: baseline - size, width: max(0, x1 - x0), height: size * 1.25)
            var sources: [RenderingV2.SourceRange] = []
            for c in run.clusters {
                for s in c.sources ?? [] {
                    if let i = sources.firstIndex(where: { $0.path == s.path }) {
                        sources[i].startByte = min(sources[i].startByte, s.startByte)
                        sources[i].endByte = max(sources[i].endByte, s.endByte)
                    } else {
                        sources.append(s)
                    }
                }
            }
            out.append(Word(itemIndex: index, text: run.text, rect: rect, fontSizePt: size, baseline: baseline,
                            sources: sources, syntheticReason: run.clusters.first?.syntheticReason))
        }
        return out
    }

    /// The page's rules in points (fraction bars, radical bars): a script
    /// on the far side of a rule that spans it is a fraction part.
    static func rules(of page: RenderingV2.Page) -> [CGRect] {
        page.items.compactMap { item in
            guard case .rule(let r) = item else { return nil }
            return CGRect(x: RenderingV2.points(r.x), y: RenderingV2.points(r.top), width: RenderingV2.points(r.width), height: RenderingV2.points(r.height))
        }
    }

    /// Lines of `page` in reading order (see the type comment). Item order
    /// is not trusted (a producer may emit a footnote before the body).
    static func lines(of page: RenderingV2.Page) -> [Line] {
        lines(words: words(of: page), rules: rules(of: page))
    }
}

/// Invisible, hit-test-free accessibility layer over one v2 page (the v2
/// counterpart of `AccessibilityOverlay`): a landmark whose value is the
/// page's text and whose children are one static-text element per line.
struct PageV2AccessibilityOverlay: NSViewRepresentable {
    let page: RenderingV2.Page
    let pageToken: String
    let totalPages: Int
    let scale: CGFloat
    let onSelect: (V2Geometry.Hit) -> Void

    func makeNSView(context: Context) -> PageV2AXView { PageV2AXView() }

    func updateNSView(_ view: PageV2AXView, context: Context) {
        view.update(page: page, pageToken: pageToken, totalPages: totalPages, scale: scale, onSelect: onSelect)
    }
}

/// The v2 page container. Like `PageAXView` the tree is built the first time
/// an assistive client asks; after that it is updated in place — a zoom
/// moves the elements, a new page token relabels them (rebuilding only when
/// the line count differs) — so VoiceOver's cursor on a line survives typing
/// and zooming. A keystroke that leaves the page's bytes alone (same
/// `pageToken`) touches nothing. Never hit-tested, never drawn.
final class PageV2AXView: NSView {
    private var page: RenderingV2.Page?
    private var pageToken = ""
    private var totalPages = 0
    private var scale: CGFloat = 1
    private var onSelect: (V2Geometry.Hit) -> Void = { _ in }
    private var cachedLines: [V2PageText.Line]?
    private var cachedElements: [PreviewAXElement]?
    /// `layoutChanged` notifications posted (evidence: only when elements changed).
    private(set) var layoutChangesPosted = 0
    private(set) var rebuilds = 0

    override var isFlipped: Bool { true }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override var isOpaque: Bool { false }

    /// The lazy stack built this page: the Pages rotor may be waiting to
    /// point VoiceOver at it (PreviewPagesRotor.pageViewDidAppear).
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window != nil, page != nil { PreviewPagesRotorLookup.source(near: self)?.previewPageDidAppear(self) }
    }

    func update(page: RenderingV2.Page, pageToken: String, totalPages: Int, scale: CGFloat, onSelect: @escaping (V2Geometry.Hit) -> Void) {
        // Lines are in page coordinates, so only a new page re-derives them;
        // zoom changes the elements' frames, a new page their labels and
        // actions, and the page count only the landmark's own (live) label.
        let firstPage = self.page == nil
        let becameResident = self.page?.resident != true && page.resident // a windowed frame served this page
        let newPage = self.pageToken != pageToken || firstPage
        let rescaled = self.scale != scale
        self.page = page; self.pageToken = pageToken; self.totalPages = totalPages; self.scale = scale
        self.onSelect = onSelect
        if firstPage || becameResident, window != nil { PreviewPagesRotorLookup.source(near: self)?.previewPageDidAppear(self) }
        if newPage { cachedLines = nil }
        guard let elements = cachedElements, newPage || rescaled else { return }
        // Only a client that already read this page has elements to keep.
        if newPage, elements.count != lines.count {
            cachedElements = nil
        } else {
            for (ax, line) in zip(elements, lines) { configure(ax, line: line, relabel: newPage) }
        }
        layoutChangesPosted += 1
        NSAccessibility.post(element: self, notification: .layoutChanged)
    }

    /// Whether an assistive client has asked for this page's tree since the last rebuild.
    var hasBuiltTree: Bool { cachedElements != nil }

    private var lines: [V2PageText.Line] {
        if let cachedLines { return cachedLines }
        let lines = page.map { $0.resident ? V2PageText.lines(of: $0) : [] } ?? []
        cachedLines = lines
        return lines
    }

    override func isAccessibilityElement() -> Bool { true }
    override func accessibilityRole() -> NSAccessibility.Role? { .group }
    override func accessibilitySubrole() -> NSAccessibility.Subrole? { PreviewAccessibility.landmarkSubrole }
    override func accessibilityRoleDescription() -> String? { PreviewAccessibility.pageRoleDescription }
    override func accessibilityLabel() -> String? {
        guard let page else { return nil }
        guard page.resident else { return V2PageText.elidedPageLabel(number: page.number, totalPages: totalPages) }
        return V2PageText.pageLabel(number: page.number, totalPages: totalPages, lineCount: lines.count)
    }
    /// The page's text, one line per line, so "read page" speaks the whole
    /// page from the landmark without walking its children.
    override func accessibilityValue() -> Any? { lines.map(\.text).joined(separator: "\n") }
    override func accessibilityChildren() -> [Any]? { elements() }
    override func accessibilityChildrenInNavigationOrder() -> [NSAccessibilityElementProtocol]? {
        PreviewAXElement.navigationOrder(elements())
    }
    /// The pane's Pages rotor (PreviewPagesRotor.swift): every page of the
    /// document, including the ones the lazy stack has not built.
    override func accessibilityCustomRotors() -> [NSAccessibilityCustomRotor] {
        PreviewPagesRotorLookup.source(near: self)?.previewPagesRotors ?? []
    }

    private func elements() -> [PreviewAXElement] {
        if let cachedElements { return cachedElements }
        rebuilds += 1
        let out: [PreviewAXElement] = lines.map { line in
            let ax = PreviewAXElement.make(role: .staticText, label: "", viewFrame: .zero, pageView: self, parent: self)
            ax.setAccessibilityRoleDescription(PreviewAccessibility.lineRoleDescription)
            configure(ax, line: line, relabel: true)
            return ax
        }
        cachedElements = out
        return out
    }

    /// Frame at the current scale; label, value, help and the "Go to source"
    /// action for `line` when `relabel` (a new page or a fresh element).
    private func configure(_ ax: PreviewAXElement, line: V2PageText.Line, relabel: Bool) {
        ax.setViewFrame(CGRect(x: line.rect.minX * scale, y: line.rect.minY * scale,
                               width: max(1, line.rect.width * scale), height: max(1, line.rect.height * scale)))
        guard relabel, let page else { return }
        ax.setAccessibilityLabel(V2PageText.lineLabel(page: page.number, line: line))
        ax.setAccessibilityValue(line.text)
        ax.setAccessibilityHelp("Page \(page.number), line \(line.number)")
        // The click a mouse user makes on the line's first word: the same
        // navigation path (digest attestation, rebase, the multi-span note).
        let onSelect = self.onSelect
        if let word = line.words.first(where: { !$0.sources.isEmpty || $0.syntheticReason != nil }) {
            let hit = V2Geometry.Hit(itemIndex: word.itemIndex, clusterIndex: 0, text: word.text,
                                     sources: word.sources, syntheticReason: word.syntheticReason,
                                     rect: RenderingV2.Rect(x: V2Geometry.ticks(word.rect.minX), top: V2Geometry.ticks(word.rect.minY),
                                                            width: V2Geometry.ticks(word.rect.width), height: V2Geometry.ticks(word.rect.height)))
            ax.setAccessibilityCustomActions([
                NSAccessibilityCustomAction(name: PreviewAccessibility.goToSourceAction) { onSelect(hit); return true },
            ])
        } else {
            ax.setAccessibilityCustomActions([])
        }
    }
}

extension PageV2AXView: PreviewPageAXTarget, NSAccessibilityElementLoading {
    var previewPageNumber: Int? { page?.number }
    var previewPageIsLoaded: Bool { page?.resident == true }
    func accessibilityElement(withToken token: NSAccessibilityLoadingToken) -> NSAccessibilityElementProtocol? {
        PreviewPagesRotorLookup.source(near: self)?.previewPageElement(forToken: token)
    }
}
