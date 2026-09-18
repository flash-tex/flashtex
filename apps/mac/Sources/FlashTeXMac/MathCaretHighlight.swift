import Foundation
import FlashTeXProtocol

/// Source → preview for a caret inside math (lane mac-navigation-3).
///
/// Measured on flashtex-render 9aaec57a: there is no `math` item kind. A
/// formula is laid out as ordinary `glyph_run` items (math font, script
/// sizes) plus `rule` items (fraction bars, the `\sqrt` overbar), and EVERY
/// cluster and rule of the formula carries one source range — the whole
/// formula including its delimiters (`$…$`, `\[…\]`). No cluster maps its
/// bytes 1:1, so `V2Geometry.clusters(containing:)` returns every glyph
/// cluster of the formula with no exact caret and the rules not at all.
///
/// `caretHighlights` keeps the exact caret / whole-cluster contract for text
/// and adds the enclosing formula box: when no cluster containing the caret
/// byte maps 1:1 and one source span fans out over several items, the
/// highlight is the union of every item on the page with exactly that span
/// (glyph clusters AND rules). A single non-1:1 cluster (`\'e` → é) keeps
/// the documented whole-cluster fallback. If a producer ever maps a math
/// atom 1:1 (`x` of `\frac{x}{y}` → its own bytes), that exact caret wins.
extension V2Geometry {
    /// The enclosing box of one fanned-out source span.
    struct FormulaBox: Equatable {
        /// The span every member item carries (the formula including delimiters).
        var source: RenderingV2.SourceRange
        /// Page item indices in the box, in page order.
        var itemIndices: [Int]
        var clusterCount: Int
        var ruleCount: Int
        /// Every member rectangle (cluster hit rects and rule rects), in ticks.
        var rects: [RenderingV2.Rect]
        /// Union of `rects`, in ticks.
        var bounds: RenderingV2.Rect
    }

    /// The caret's paragraph on a page: every item whose source span lies
    /// within the paragraph's bytes, merged into one rectangle per line row.
    struct ParagraphBand: Equatable {
        /// The paragraph's bytes (blank-line delimited in the source).
        var source: RenderingV2.SourceRange
        /// One rectangle per row the paragraph occupies on this page, in
        /// ticks, top to bottom: the union of the member rectangles that
        /// overlap vertically (a superscript, a fraction bar and its line
        /// share a row).
        var rows: [RenderingV2.Rect]
        /// Member items on this page (glyph runs with at least one member
        /// cluster, plus rules).
        var itemCount: Int
    }

    enum CaretHighlight: Equatable {
        /// Exact caret bar (1:1 cluster with a caret at that byte) or the
        /// whole-cluster fallback, exactly as before.
        case cluster(CaretMatch)
        /// The caret is inside a formula: the enclosing box of every item
        /// carrying that span.
        case formula(FormulaBox)
        /// The caret's whole paragraph, as a faint band per row (only when
        /// the preview follows the caret; always last in the list, so
        /// `first` keeps meaning the caret itself).
        case paragraph(ParagraphBand)
    }

    /// The bytes of the paragraph containing `byte` of `text`: the caret's
    /// line grown over every adjacent non-blank line, up to the newline
    /// before the next blank line. A blank line holds only spaces, tabs or a
    /// CR. A caret on a blank line (or past the end) gets an empty range.
    static func paragraphBounds(containing byte: Int, in text: String) -> Range<Int> {
        var copy = text
        return copy.withUTF8 { b -> Range<Int> in
            let n = b.count
            let byte = max(0, min(byte, n))
            let nl = UInt8(ascii: "\n")
            func isBlank(_ s: Int, _ e: Int) -> Bool {
                var i = s
                while i < e { let c = b[i]; if c != 0x20, c != 0x09, c != 0x0D { return false }; i += 1 }
                return true
            }
            // The caret's line.
            var lineStart = byte
            while lineStart > 0, b[lineStart - 1] != nl { lineStart -= 1 }
            var lineEnd = byte
            while lineEnd < n, b[lineEnd] != nl { lineEnd += 1 }
            guard !isBlank(lineStart, lineEnd) else { return lineStart..<lineStart }
            // Backwards over non-blank lines: `start - 1` is the newline ending the previous line.
            var start = lineStart
            while start > 0 {
                var previous = start - 1
                while previous > 0, b[previous - 1] != nl { previous -= 1 }
                if isBlank(previous, start - 1) { break }
                start = previous
            }
            // Forwards over non-blank lines: `end` is the newline ending the current last line.
            var end = lineEnd
            while end < n {
                var next = end + 1
                while next < n, b[next] != nl { next += 1 }
                if isBlank(end + 1, next) { break }
                end = next
            }
            return start..<end
        }
    }

    /// Every item of `page` whose sources for `path` lie within `paragraph`,
    /// as one band of row rectangles; nil when nothing on the page does.
    static func paragraphBand(for paragraph: Range<Int>, path: String, in page: RenderingV2.Page) -> ParagraphBand? {
        guard !paragraph.isEmpty else { return nil }
        @inline(__always) func inside(_ s: RenderingV2.SourceRange) -> Bool {
            s.path == path && s.startByte >= paragraph.lowerBound && s.endByte <= paragraph.upperBound
                && (s.startByte < s.endByte || paragraph.contains(s.startByte))
        }
        var rects: [RenderingV2.Rect] = []
        var items = 0
        for item in page.items {
            switch item {
            case .rule(let r):
                guard r.sources?.contains(where: inside) == true else { continue }
                items += 1
                rects.append(RenderingV2.Rect(x: r.x, top: r.top, width: r.width, height: r.height))
            case .glyphRun(let run):
                var member = false
                for c in run.clusters where c.sources?.contains(where: inside) == true {
                    member = true
                    rects.append(contentsOf: c.hitRects)
                }
                if member { items += 1 }
            case .image, .path:
                continue // figures are not part of a text band
            }
        }
        guard !rects.isEmpty else { return nil }
        return ParagraphBand(source: RenderingV2.SourceRange(path: path, startByte: paragraph.lowerBound, endByte: paragraph.upperBound),
                             rows: rows(merging: rects), itemCount: items)
    }

    /// Merges `rects` into one rectangle per row: sorted by top, a rectangle
    /// joins the current row when it overlaps it vertically, else opens the
    /// next one. Each row is the union of its members.
    static func rows(merging rects: [RenderingV2.Rect]) -> [RenderingV2.Rect] {
        var rows: [RenderingV2.Rect] = []
        for r in rects.sorted(by: { $0.top < $1.top }) {
            if let last = rows.last, r.top < last.top &+ last.height {
                let x = min(last.x, r.x), top = min(last.top, r.top)
                let right = max(last.x &+ last.width, r.x &+ r.width), bottom = max(last.top &+ last.height, r.top &+ r.height)
                rows[rows.count - 1] = RenderingV2.Rect(x: x, top: top, width: right &- x, height: bottom &- top)
            } else {
                rows.append(r)
            }
        }
        return rows
    }

    /// Whether `match` maps its source bytes 1:1 onto its text bytes (an
    /// exact caret is derivable for some byte of it).
    private static func isOneToOne(_ cluster: RenderingV2.Cluster, _ source: RenderingV2.SourceRange) -> Bool {
        source.endByte - source.startByte == cluster.textEndByte - cluster.textStartByte
    }

    /// Every item of `page` whose sources contain exactly `span`, as one box.
    static func formulaBox(for span: RenderingV2.SourceRange, in page: RenderingV2.Page) -> FormulaBox? {
        var indices: [Int] = [], rects: [RenderingV2.Rect] = []
        var clusters = 0, rules = 0
        for (index, item) in page.items.enumerated() {
            switch item {
            case .rule(let r):
                guard r.sources?.contains(span) == true else { continue }
                rules += 1
                indices.append(index)
                rects.append(RenderingV2.Rect(x: r.x, top: r.top, width: r.width, height: r.height))
            case .glyphRun(let run):
                var member = false
                for c in run.clusters where c.sources?.contains(span) == true {
                    clusters += 1; member = true
                    rects.append(contentsOf: c.hitRects)
                }
                if member { indices.append(index) }
            case .image, .path:
                continue // images and TikZ paths are never part of a formula box
            }
        }
        guard let first = rects.first else { return nil }
        var minX = first.x, minTop = first.top, maxX = first.x &+ first.width, maxBottom = first.top &+ first.height
        for r in rects.dropFirst() {
            minX = min(minX, r.x); minTop = min(minTop, r.top)
            maxX = max(maxX, r.x &+ r.width); maxBottom = max(maxBottom, r.top &+ r.height)
        }
        return FormulaBox(source: span, itemIndices: indices, clusterCount: clusters, ruleCount: rules, rects: rects,
                          bounds: RenderingV2.Rect(x: minX, top: minTop, width: maxX &- minX, height: maxBottom &- minTop))
    }

    /// Highlights for the caret at `byte` of `path` on `page`:
    /// - every 1:1 cluster containing the byte, as `.cluster` (exact caret
    ///   when the producer supplied one at that byte, else its rects);
    /// - otherwise, for each distinct fanned-out span containing the byte
    ///   (carried by two or more items, or by any rule), one `.formula` box;
    /// - a span carried by a single glyph cluster stays `.cluster` (fallback);
    /// - with `paragraph` (the caret's paragraph bytes, `paragraphBounds`),
    ///   one trailing `.paragraph` band when any item of the page lies in it.
    static func caretHighlights(containing byte: Int, path: String, in page: RenderingV2.Page,
                                paragraph: Range<Int>? = nil) -> [CaretHighlight] {
        var out = caretItemHighlights(containing: byte, path: path, in: page)
        if let paragraph, let band = paragraphBand(for: paragraph, path: path, in: page) { out.append(.paragraph(band)) }
        return out
    }

    private static func caretItemHighlights(containing byte: Int, path: String, in page: RenderingV2.Page) -> [CaretHighlight] {
        let matches = clusters(containing: byte, path: path, in: page)
        var exact: [CaretHighlight] = []
        var fanned: [RenderingV2.SourceRange] = []
        var single: [CaretHighlight] = []
        for m in matches {
            guard case .glyphRun(let run) = page.items[m.itemIndex] else { continue }
            let c = run.clusters[m.clusterIndex]
            guard let source = (c.sources ?? []).first(where: { s in
                s.path == path && (s.startByte == s.endByte ? byte == s.startByte : (s.startByte <= byte && byte < s.endByte))
            }) else { continue }
            if isOneToOne(c, source) {
                exact.append(.cluster(m))
            } else if !fanned.contains(source) {
                fanned.append(source)
                if let box = formulaBox(for: source, in: page), box.clusterCount + box.ruleCount > 1 {
                    single.append(.formula(box))
                } else {
                    single.append(.cluster(m))
                }
            } else {
                // Another cluster of a span already boxed: covered by the box.
            }
        }
        return exact.isEmpty ? single : exact
    }
}

extension ShellModel {
    /// What the v2 pane highlights for the editor caret, for the note and
    /// tests: the formula box(es) the caret is inside on the current frame,
    /// with page numbers. Empty when the caret is in no formula.
    func caretFormulaBoxes() -> [(page: Int, box: V2Geometry.FormulaBox)] {
        guard let frame = displayListV2?.frame, let byte = caretByte else { return [] }
        var out: [(page: Int, box: V2Geometry.FormulaBox)] = []
        for page in frame.list.pages {
            for h in V2Geometry.caretHighlights(containing: byte, path: activePath, in: page) {
                if case .formula(let box) = h { out.append((page.number, box)) }
            }
        }
        return out
    }
}
