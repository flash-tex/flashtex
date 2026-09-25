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

/// The text of one v2 page as VoiceOver reads it: lines in top-to-bottom
/// order, each the run texts on one baseline in left-to-right order. Pure.
enum V2PageText {
    /// One run's contribution to a line (page points, y down).
    struct Word: Equatable {
        var itemIndex: Int
        var text: String
        /// Ink box of the run: from its first glyph origin to the last glyph's
        /// advance, one em above the baseline (ascent is not in the list) to
        /// a quarter em below.
        var rect: CGRect
        var fontSizePt: Double
        /// One range per source path: the clusters' ranges merged (min start,
        /// max end), so "Go to source" selects the word, not its first letter.
        var sources: [RenderingV2.SourceRange]
        var syntheticReason: String?
    }

    struct Line: Equatable {
        var number: Int
        var words: [Word]
        /// The words joined; a space wherever the producer left more than
        /// `wordGapEm` between two runs (an inter-word glue), nothing for a
        /// kerned or italic-corrected split of one word.
        var text: String
        var rect: CGRect
        /// Sources of the line's first run that has any: the "Go to source" target.
        var sources: [RenderingV2.SourceRange] { words.first { !$0.sources.isEmpty }?.sources ?? [] }
    }

    /// Runs whose baselines differ by less than this fraction of the larger
    /// font size sit on one line (a superscript is its own line; TeX never
    /// nudges a baseline within a line by more than rounding).
    static let sameLineEm = 0.25
    /// A horizontal gap between runs wider than this fraction of the font
    /// size reads as a word boundary (TeX's interword glue is ~0.33 em; an
    /// italic correction or a kern is under 0.1 em).
    static let wordGapEm = 0.15

    static func words(of page: RenderingV2.Page) -> [Word] {
        var out: [Word] = []
        for (index, item) in page.items.enumerated() {
            guard case .glyphRun(let run) = item, let first = run.glyphs.first, let last = run.glyphs.last,
                  !run.text.isEmpty else { continue }
            let size = RenderingV2.points(run.fontSize)
            let x0 = RenderingV2.points(first.originX)
            let x1 = RenderingV2.points(last.originX &+ last.advanceX)
            let baseline = RenderingV2.points(first.baselineY)
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
            out.append(Word(itemIndex: index, text: run.text, rect: rect, fontSizePt: size,
                            sources: sources, syntheticReason: run.clusters.first?.syntheticReason))
        }
        return out
    }

    /// Lines of `page` in reading order. Item order is not trusted (a
    /// producer may emit a footnote before the body): runs are sorted by
    /// baseline, then x.
    static func lines(of page: RenderingV2.Page) -> [Line] {
        let sorted = words(of: page).sorted { a, b in
            let ya = a.rect.maxY - a.fontSizePt * 0.25, yb = b.rect.maxY - b.fontSizePt * 0.25 // baselines
            if abs(ya - yb) > sameLineEm * max(a.fontSizePt, b.fontSizePt) { return ya < yb }
            return a.rect.minX < b.rect.minX
        }
        var lines: [[Word]] = []
        for word in sorted {
            if let lastWord = lines.last?.last,
               abs(baseline(lastWord) - baseline(word)) <= sameLineEm * max(lastWord.fontSizePt, word.fontSizePt) {
                lines[lines.count - 1].append(word)
            } else {
                lines.append([word])
            }
        }
        return lines.enumerated().map { i, words in
            var text = ""
            var previous: Word?
            for word in words {
                if let previous, word.rect.minX - previous.rect.maxX > wordGapEm * min(previous.fontSizePt, word.fontSizePt) {
                    text += " "
                }
                text += word.text
                previous = word
            }
            let rect = words.dropFirst().reduce(words[0].rect) { $0.union($1.rect) }
            return Line(number: i + 1, words: words, text: text, rect: rect)
        }
    }

    private static func baseline(_ w: Word) -> Double { w.rect.maxY - w.fontSizePt * 0.25 }

    /// "Page 3 of 12, 40 lines" — the v1 page label's shape
    /// (`AccessibleDocumentModel.PageSummary`), so both panes read alike.
    static func pageLabel(number: Int, totalPages: Int, lineCount: Int) -> String {
        "Page \(number)\(totalPages > 0 ? " of \(totalPages)" : ""), \(lineCount) line\(lineCount == 1 ? "" : "s")"
    }

    /// An elided page of a windowed frame (display-list-v2-window): the
    /// document's length is known, the page's content is not.
    static func elidedPageLabel(number: Int, totalPages: Int) -> String {
        "Page \(number)\(totalPages > 0 ? " of \(totalPages)" : ""), not loaded"
    }

    static func lineLabel(page: Int, line: Line) -> String { "Page \(page), line \(line.number): \(line.text)" }
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
/// an assistive client asks and dropped when the page changes; a keystroke
/// that leaves the page's bytes alone (same `pageToken`) keeps it. Never
/// hit-tested, never drawn.
final class PageV2AXView: NSView {
    private var page: RenderingV2.Page?
    private var pageToken = ""
    private var totalPages = 0
    private var scale: CGFloat = 1
    private var onSelect: (V2Geometry.Hit) -> Void = { _ in }
    private var cachedLines: [V2PageText.Line]?
    private var cachedElements: [PreviewAXElement]?

    override var isFlipped: Bool { true }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override var isOpaque: Bool { false }

    func update(page: RenderingV2.Page, pageToken: String, totalPages: Int, scale: CGFloat, onSelect: @escaping (V2Geometry.Hit) -> Void) {
        let changed = self.pageToken != pageToken || self.totalPages != totalPages || self.scale != scale || self.page == nil
        self.page = page; self.pageToken = pageToken; self.totalPages = totalPages; self.scale = scale
        self.onSelect = onSelect
        guard changed else { return }
        let hadTree = cachedElements != nil
        cachedLines = nil; cachedElements = nil
        // Only a client that already read this page needs to hear about the change.
        if hadTree { NSAccessibility.post(element: self, notification: .layoutChanged) }
    }

    /// Whether an assistive client has asked for this page's tree since the last change.
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

    private func elements() -> [PreviewAXElement] {
        if let cachedElements { return cachedElements }
        guard let page else { return [] }
        let onSelect = self.onSelect
        let out: [PreviewAXElement] = lines.map { line in
            let frame = CGRect(x: line.rect.minX * scale, y: line.rect.minY * scale,
                               width: max(1, line.rect.width * scale), height: max(1, line.rect.height * scale))
            let ax = PreviewAXElement.make(role: .staticText, label: V2PageText.lineLabel(page: page.number, line: line),
                                           viewFrame: frame, pageView: self, parent: self)
            ax.setAccessibilityRoleDescription(PreviewAccessibility.lineRoleDescription)
            ax.setAccessibilityValue(line.text)
            ax.setAccessibilityHelp("Page \(page.number), line \(line.number)")
            // The click a mouse user makes on the line's first word: the same
            // navigation path (digest attestation, rebase, the multi-span note).
            if let word = line.words.first(where: { !$0.sources.isEmpty || $0.syntheticReason != nil }) {
                let hit = V2Geometry.Hit(itemIndex: word.itemIndex, clusterIndex: 0, text: word.text,
                                         sources: word.sources, syntheticReason: word.syntheticReason,
                                         rect: RenderingV2.Rect(x: V2Geometry.ticks(word.rect.minX), top: V2Geometry.ticks(word.rect.minY),
                                                                width: V2Geometry.ticks(word.rect.width), height: V2Geometry.ticks(word.rect.height)))
                ax.setAccessibilityCustomActions([
                    NSAccessibilityCustomAction(name: PreviewAccessibility.goToSourceAction) { onSelect(hit); return true },
                ])
            }
            return ax
        }
        cachedElements = out
        return out
    }
}
