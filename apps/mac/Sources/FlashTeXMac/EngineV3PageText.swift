import CoreGraphics
import FlashTeXAccessibility
import FlashTeXProtocol

// The page-text model VoiceOver reads over the engine-v3 preview
// (EngineV3Accessibility.swift): moved here unchanged from
// PreviewV2Accessibility.swift in retirement stage S6a (retirement-plan.md
// §1.4), so the v3 pane keeps it when S6 deletes the v2 pane. `V2PageText`
// is a typealias of this type for the v2 pane's callers until then.

/// The text of one page as VoiceOver reads it: lines in top-to-bottom
/// order, each the run texts in reading order. Pure.
///
/// Grouping is the v1 model's (`AccessibleDocumentModel.lines(of:)`), so
/// both panes read math alike: runs cluster by baseline; a cluster whose
/// runs are small (≤ `scriptSizeRatio` of a larger cluster's size) and
/// whose baseline is within `scriptReachEm` of that larger cluster's is a
/// script of it — a superscript, subscript, footnote mark, fraction part —
/// and joins its line instead of becoming one, so `$a^2+b_1$` reads
/// "a 2 + b 1", not "2" on a line of its own before "a". Within a line the
/// order is left to right by x; scripts stacked at one x read subscript
/// then superscript (`$x_{i}^{2}$`: "x i 2", as it is spoken); a fraction
/// reads numerator, then denominator, at its bar. Beyond v1: display
/// fractions are full-size, so the two clusters a bar separates (each
/// within the bar's span) join the line the bar sits on — `\[ \frac{a+b}{c}
/// = d \]` reads "a + bc = d" — or each other when the fraction stands
/// alone. A display `\sum_{i=0}^{n}`'s lower limit sits well beyond a
/// script's reach and is its own line, as in v1. A change of baseline
/// between consecutive words is a spoken boundary ("term. 1").
enum PreviewPageText {
    /// One run's contribution to a line (page points, y down).
    struct Word: Equatable {
        var itemIndex: Int
        var text: String
        /// Ink box of the run: from its first glyph origin to the last glyph's
        /// advance, one em above the baseline (ascent is not in the list) to
        /// a quarter em below.
        var rect: CGRect
        var fontSizePt: Double
        /// The run's last glyph's baseline: a run may span baselines (the
        /// producer emits `\frac{x}{y}` as one run "xy" and `\sqrt{z}` as one
        /// run "√z"), and the last glyph is the one on the text's own line.
        var baseline: Double
        /// One range per source path: the clusters' ranges merged (min start,
        /// max end), so "Go to source" selects the word, not its first letter.
        var sources: [RuntimeV1.SourceRange]
        var syntheticReason: String?
    }

    struct Line: Equatable {
        var number: Int
        /// In reading order.
        var words: [Word]
        /// The words joined; a space wherever the producer left more than
        /// `wordGapEm` between two consecutive words (an inter-word glue),
        /// nothing for a kerned or italic-corrected split of one word or a
        /// script attached to its base.
        var text: String
        var rect: CGRect
        /// Sources of the line's first run that has any: the "Go to source" target.
        var sources: [RuntimeV1.SourceRange] { words.first { !$0.sources.isEmpty }?.sources ?? [] }
    }

    /// A horizontal gap between runs wider than this fraction of the font
    /// size reads as a word boundary (TeX's interword glue is ~0.33 em; an
    /// italic correction or a kern is under 0.1 em).
    static let wordGapEm = 0.15

    private struct Cluster {
        var baseline: Double
        var size: Double // max font size in the cluster
        var minX: Double
        var maxX: Double
        var words: [Word]
        var parent: Int?
    }

    /// A fraction part sits within its bar's span, this far beyond the bar's
    /// ends at most (in em of the part's size), and within `fractionReachEm`
    /// of the bar vertically; the line the bar belongs to has its baseline
    /// from `axisAboveEm` above the bar to `axisBelowEm` below it (TeX puts
    /// the bar on the math axis, a little above the baseline).
    static let fractionSlackEm = 0.5
    static let fractionReachEm = 1.5
    static let axisAboveEm = 0.75
    static let axisBelowEm = 1.0

    /// The grouping itself, over any producer's words and rules (page points,
    /// y down): the v2 pane's runs, or the engine-v3 pane's glyphs
    /// (EngineV3Accessibility.swift), so both panes read a page alike.
    static func lines(words: [Word], rules: [CGRect]) -> [Line] {
        // 1. Cluster by baseline (v1: `baselineTolerancePt`).
        var clusters: [Cluster] = []
        for word in words {
            if let ci = clusters.firstIndex(where: { abs($0.baseline - word.baseline) <= AccessibleDocumentModel.baselineTolerancePt }) {
                clusters[ci].words.append(word)
                clusters[ci].size = max(clusters[ci].size, word.fontSizePt)
                clusters[ci].minX = min(clusters[ci].minX, word.rect.minX)
                clusters[ci].maxX = max(clusters[ci].maxX, word.rect.maxX)
            } else {
                clusters.append(Cluster(baseline: word.baseline, size: word.fontSizePt, minX: word.rect.minX, maxX: word.rect.maxX, words: [word], parent: nil))
            }
        }
        clusters.sort { $0.baseline != $1.baseline ? $0.baseline < $1.baseline : $0.words[0].itemIndex < $1.words[0].itemIndex }

        // 2. Attach small clusters to the nearest larger neighbour within reach
        //    (v1: `scriptSizeRatio`, `scriptReachEm`; a script never starts
        //    more than an em left of its base).
        for ci in clusters.indices {
            let c = clusters[ci]
            var best: (index: Int, reach: Double)?
            for pi in clusters.indices where pi != ci {
                let p = clusters[pi]
                guard c.size <= p.size * AccessibleDocumentModel.scriptSizeRatio else { continue }
                let reach = abs(c.baseline - p.baseline) / p.size
                guard reach <= AccessibleDocumentModel.scriptReachEm, c.minX >= p.minX - p.size else { continue }
                if best == nil || reach < best!.reach { best = (pi, reach) }
            }
            clusters[ci].parent = best?.index
        }
        func root(_ i: Int) -> Int {
            var i = i, hops = 0
            while let p = clusters[i].parent, hops < clusters.count { i = p; hops += 1 }
            return i
        }

        // 2b. Display fractions: full-size parts the size rule never attaches.
        //     For each bar with a cluster within its span just above and one
        //     just below, both join the cluster on the bar's axis next to it
        //     (`= d`), else the denominator joins the numerator. A radical's
        //     bar has nothing above it within its span; a footnote rule has
        //     nothing above it within its span either.
        for rule in rules {
            func within(_ c: Cluster) -> Bool {
                c.minX >= rule.minX - fractionSlackEm * c.size && c.maxX <= rule.maxX + fractionSlackEm * c.size
            }
            let above = clusters.indices.filter { within(clusters[$0]) && clusters[$0].baseline < rule.minY && rule.minY - clusters[$0].baseline <= fractionReachEm * clusters[$0].size }
                .min { abs(clusters[$0].baseline - rule.minY) < abs(clusters[$1].baseline - rule.minY) }
            let below = clusters.indices.filter { within(clusters[$0]) && clusters[$0].baseline > rule.minY && clusters[$0].baseline - rule.minY <= fractionReachEm * clusters[$0].size }
                .min { abs(clusters[$0].baseline - rule.minY) < abs(clusters[$1].baseline - rule.minY) }
            guard let num = above, let den = below, root(num) != root(den) || clusters[den].parent == nil else { continue }
            let axis = clusters.indices.filter { i in
                i != num && i != den && root(i) != num && root(i) != den
                    && clusters[i].baseline >= rule.minY - axisAboveEm * clusters[i].size && clusters[i].baseline <= rule.minY + axisBelowEm * clusters[i].size
                    && (clusters[i].minX <= rule.maxX + 2 * clusters[i].size && clusters[i].maxX >= rule.minX - 2 * clusters[i].size)
            }.min { a, b in
                func gap(_ i: Int) -> Double { max(0, max(clusters[i].minX - rule.maxX, rule.minX - clusters[i].maxX)) }
                return gap(a) < gap(b)
            }
            if let axis {
                if clusters[num].parent == nil { clusters[num].parent = axis }
                if clusters[den].parent == nil { clusters[den].parent = axis }
            } else if clusters[den].parent == nil {
                clusters[den].parent = root(num)
            }
        }

        // 3. Lines from root clusters in baseline order; members read left to
        //    right, a fraction numerator-then-denominator at its bar (v1's keys:
        //    anchor x, part order, own x — then, stacked scripts at one x, the
        //    lower first — and item index).
        var lines: [Line] = []
        for ri in clusters.indices where clusters[ri].parent == nil {
            let rootBaseline = clusters[ri].baseline
            var keyed: [(key: (Double, Int, Double, Double, Int), word: Word)] = []
            for mi in clusters.indices where root(mi) == ri {
                for word in clusters[mi].words {
                    var anchorX = word.rect.minX, order = 1
                    if mi != ri {
                        let above = word.baseline < rootBaseline
                        // Numerator/denominator: a rule spans the word's x and sits on the
                        // far side of it, between the word and the line's own baseline
                        // (a display bar sits a little above the axis line's baseline).
                        if let rule = rules.first(where: { r in
                            word.rect.minX >= r.minX - 0.5 && word.rect.minX <= r.maxX + 0.5
                                && (above ? (word.baseline < r.minY && r.minY <= rootBaseline + axisAboveEm * clusters[ri].size)
                                          : (word.baseline > r.minY && r.minY >= rootBaseline - axisBelowEm * clusters[ri].size))
                        }) {
                            anchorX = rule.minX
                            order = above ? 0 : 2
                        }
                    }
                    keyed.append(((anchorX, order, word.rect.minX, -word.baseline, word.itemIndex), word))
                }
            }
            keyed.sort { a, b in
                if a.key.0 != b.key.0 { return a.key.0 < b.key.0 }
                if a.key.1 != b.key.1 { return a.key.1 < b.key.1 }
                if a.key.2 != b.key.2 { return a.key.2 < b.key.2 }
                if a.key.3 != b.key.3 { return a.key.3 < b.key.3 }
                return a.key.4 < b.key.4
            }
            let ordered = keyed.map(\.word)
            var text = ""
            var previous: Word?
            for word in ordered {
                if let previous, word.rect.minX - previous.rect.maxX > wordGapEm * min(previous.fontSizePt, word.fontSizePt)
                    || abs(word.baseline - previous.baseline) > AccessibleDocumentModel.baselineTolerancePt {
                    text += " "
                }
                text += word.text
                previous = word
            }
            let rect = ordered.dropFirst().reduce(ordered[0].rect) { $0.union($1.rect) }
            lines.append(Line(number: lines.count + 1, words: ordered, text: text, rect: rect))
        }
        return lines
    }

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
