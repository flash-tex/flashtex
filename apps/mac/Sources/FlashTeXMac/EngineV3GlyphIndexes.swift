import Foundation
import FlashTeXDisplayListV3
import FlashTeXPreviewV3

/// The per-page glyph indexes of forward and reverse search
/// (`DL3SourceIndex`, about 100 bytes a glyph), bounded.
///
/// Before, every page forward search scanned kept its index for good: the
/// caret follow on a page near the end of a 1,000-page document held an
/// index of every page before it (ESTIMATE 116 MB on infdesc, 300 MB on a
/// dense 1,000 pages; mem-research §2.9 #10), and nothing dropped them on a
/// stop or another project (so a lookup could answer with the last
/// project's glyphs).
///
/// Now:
/// * At most `capacity` indexes are held, least recently used out.
/// * Each page also keeps the spans its glyphs carry (sorted, a few hundred
///   bytes): forward search asks those which pages can show a line and
///   builds the full index only for them, so scanning to the caret's page
///   builds one index, not one per page before it.
/// * The owner (EngineV3Session) drops a page's entries when the page
///   changes or goes, and all of them on a stop or another project.
struct EngineV3GlyphIndexes {
    static let defaultCapacity = 16

    let capacity: Int
    private var indexes: [Int: DL3SourceIndex] = [:]
    /// Pages with an index, least recently used first (at most `capacity`).
    private var order: [Int] = []
    /// Page → the span ids its GLYPH items carry, sorted and unique: a
    /// superset of the spans its index has (a glyph whose font did not
    /// resolve is not indexed), so it never hides a page that shows a line.
    private var spans: [Int: [UInt32]] = [:]
    /// Indexes built (tests: a scan builds one per page that shows the line).
    private(set) var builds = 0

    init(capacity: Int = Self.defaultCapacity) { self.capacity = max(1, capacity) }

    /// Indexes held now.
    var count: Int { indexes.count }
    /// Pages whose span summary is held.
    var summarized: Int { spans.count }
    /// The pages with an index (tests).
    var indexedPages: Set<Int> { Set(indexes.keys) }

    /// Page `i`'s index if held (no page lookup: the owner drops entries
    /// when its pages change).
    mutating func cached(_ i: Int) -> DL3SourceIndex? {
        guard let ix = indexes[i] else { return nil }
        touch(i)
        return ix
    }

    /// Page `i` was used: last out of the order.
    private mutating func touch(_ i: Int) {
        guard order.last != i, let k = order.firstIndex(of: i) else { return }
        order.remove(at: k)
        order.append(i)
    }

    /// Page `i`'s index, built from `page` when not held.
    mutating func index(_ i: Int, of page: DL3PreparedPage) -> DL3SourceIndex {
        if let ix = cached(i) { return ix }
        let ix = DL3SourceIndex(page)
        builds &+= 1
        indexes[i] = ix
        order.append(i)
        while order.count > capacity { indexes[order.removeFirst()] = nil }
        return ix
    }

    /// Whether page `i` can show any of `wanted`: nil when its summary is
    /// not held (the caller passes the page to `summarize`).
    func mayShow(_ i: Int, _ wanted: Set<UInt32>) -> Bool? {
        guard let have = spans[i] else { return nil }
        for s in wanted where Self.contains(have, s) { return true }
        return false
    }

    /// Records page `i`'s span summary; whether it can show any of `wanted`.
    mutating func summarize(_ i: Int, of page: DL3PreparedPage, _ wanted: Set<UInt32>) -> Bool {
        let have = Self.glyphSpans(page.page)
        spans[i] = have
        for s in wanted where Self.contains(have, s) { return true }
        return false
    }

    /// Page `i` changed or went.
    mutating func invalidate(_ i: Int) {
        spans[i] = nil
        if indexes.removeValue(forKey: i) != nil { order.removeAll { $0 == i } }
    }

    /// Pages `n` and after went (a shorter document).
    mutating func removePages(from n: Int) {
        spans = spans.filter { $0.key < n }
        for i in order where i >= n { indexes[i] = nil }
        order.removeAll { $0 >= n }
    }

    mutating func removeAll() {
        indexes = [:]; order = []; spans = [:]
    }

    /// Memory pressure: drops every index but those of `keep` (the pages the
    /// pane holds); they are built again on their next lookup.
    mutating func trim(keeping keep: Set<Int>) {
        for i in order where !keep.contains(i) { indexes[i] = nil }
        order.removeAll { !keep.contains($0) }
    }

    /// The span ids a page's glyphs carry (sorted, unique).
    static func glyphSpans(_ page: DL3Page) -> [UInt32] {
        var set = Set<UInt32>()
        var span: UInt32 = 0, added: UInt32? = nil
        for it in page.items {
            switch it {
            case .span(let s): span = s
            case .glyph:
                if added != span { set.insert(span); added = span }
            default: break
            }
        }
        return set.sorted()
    }

    static func contains(_ sorted: [UInt32], _ x: UInt32) -> Bool {
        var lo = 0, hi = sorted.count
        while lo < hi {
            let mid = (lo + hi) >> 1
            if sorted[mid] < x { lo = mid + 1 } else { hi = mid }
        }
        return lo < sorted.count && sorted[lo] == x
    }
}
