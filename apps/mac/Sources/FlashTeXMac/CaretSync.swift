import Foundation
import FlashTeXProtocol

/// Source → preview sync: which preview items the editor caret is inside.
/// Pure so it can be tested without a view or a model.
///
/// Containment is exact in runtime-v1 terms: an item with source
/// `start_byte..<end_byte` contains the caret byte `b` when
/// `start_byte <= b < end_byte`; an empty range contains only `b == start_byte`.
/// Bytes are never rounded, so a caret between two scalars of one grapheme
/// cluster (`e` + U+0301, a ZWJ emoji sequence) still resolves to the item
/// that owns the whole cluster, and a caret right after an item (the gap
/// before the next word) resolves to nothing.
enum CaretSync {
    /// Linear scan over every page. O(items); fine for one-off lookups and
    /// tests. The shell's per-keystroke path uses `Index` instead.
    static func itemsContaining(byte: Int, path: String,
                                in result: RuntimeV1.CompileResult) -> [(page: Int, index: Int)] {
        var hits: [(page: Int, index: Int)] = []
        for page in result.pages {
            for (index, item) in page.items.enumerated() {
                guard case .text(let t) = item, let s = t.source, s.path == path else { continue }
                if contains(start: s.startByte, end: s.endByte, byte: byte) { hits.append((page.number, index)) }
            }
        }
        return hits
    }

    /// Convenience for the preview: item indices grouped by page number.
    static func indicesByPage(byte: Int, path: String,
                              in result: RuntimeV1.CompileResult) -> [Int: Set<Int>] {
        var out: [Int: Set<Int>] = [:]
        for hit in itemsContaining(byte: byte, path: path, in: result) {
            out[hit.page, default: []].insert(hit.index)
        }
        return out
    }

    @inline(__always)
    static func contains(start: Int, end: Int, byte: Int) -> Bool {
        start <= byte && byte < max(end, start + 1)
    }

    /// UTF-8 byte offset of an editor caret given in UTF-16 units. A caret
    /// that AppKit could never produce — inside a surrogate pair, or past the
    /// end of the text — is rounded down to the start of the composed
    /// character sequence it falls in, or refused (`nil`) when out of range.
    /// A caret between two scalars of one cluster (which `NSTextView` allows
    /// for combining marks) is kept exactly, since the byte is still inside
    /// the owning item.
    static func byteOffset(ofCaretUTF16 caret: Int, in text: String) -> Int? {
        let ns = text as NSString
        guard caret >= 0, caret <= ns.length else { return nil }
        var location = caret
        if caret < ns.length, UTF16.isTrailSurrogate(ns.character(at: caret)) {
            location = ns.rangeOfComposedCharacterSequence(at: caret).location
        }
        return text.utf8ByteRange(of: NSRange(location: location, length: 0))?.start
    }

    // MARK: - sorted interval index

    /// Items of one document, sorted by start byte, with a running maximum of
    /// end bytes so a containment query walks back only over entries whose
    /// span can still reach the byte. Build: O(n log n). Query: O(log n + m)
    /// where m is the number of entries visited, i.e. the hits plus any
    /// earlier entries shadowed by a longer span that started before them.
    /// Compiler items are disjoint in source order (a word is one item; a
    /// hyphenated word split across pages repeats the same span), so m is the
    /// hit count and a caret move costs one binary search. `CaretSyncTests`
    /// measures this for 10 000 items.
    struct Index {
        struct Entry: Equatable {
            let start: Int
            let end: Int     // effective end: max(end_byte, start_byte + 1)
            let page: Int
            let index: Int
        }

        let path: String
        let entries: [Entry]            // sorted by (start, end, page, index)
        private let prefixMaxEnd: [Int] // prefixMaxEnd[i] = max(entries[0...i].end)

        /// Text items whose source names `path`, in `result`.
        init(result: RuntimeV1.CompileResult, path: String) {
            var entries: [Entry] = []
            for page in result.pages {
                for (index, item) in page.items.enumerated() {
                    guard case .text(let t) = item, let s = t.source, s.path == path else { continue }
                    entries.append(Entry(start: s.startByte, end: max(s.endByte, s.startByte + 1), page: page.number, index: index))
                }
            }
            entries.sort { a, b in
                if a.start != b.start { return a.start < b.start }
                if a.end != b.end { return a.end < b.end }
                if a.page != b.page { return a.page < b.page }
                return a.index < b.index
            }
            var running = Int.min
            var prefix: [Int] = []
            prefix.reserveCapacity(entries.count)
            for e in entries { running = max(running, e.end); prefix.append(running) }
            self.path = path
            self.entries = entries
            self.prefixMaxEnd = prefix
        }

        var count: Int { entries.count }

        /// Hits in document order (page number, then item index).
        func itemsContaining(byte: Int) -> [(page: Int, index: Int)] {
            var hits: [Entry] = []
            // Upper bound: first entry whose start is > byte.
            var lo = 0, hi = entries.count
            while lo < hi {
                let mid = (lo + hi) >> 1
                if entries[mid].start <= byte { lo = mid + 1 } else { hi = mid }
            }
            var i = lo - 1
            while i >= 0, prefixMaxEnd[i] > byte {
                if entries[i].end > byte { hits.append(entries[i]) }
                i -= 1
            }
            hits.sort { a, b in a.page != b.page ? a.page < b.page : a.index < b.index }
            return hits.map { ($0.page, $0.index) }
        }

        func indicesByPage(byte: Int) -> [Int: Set<Int>] {
            var out: [Int: Set<Int>] = [:]
            for hit in itemsContaining(byte: byte) { out[hit.page, default: []].insert(hit.index) }
            return out
        }
    }
}

// MARK: - model cache

/// Memoized `CaretSync.Index` per shell model. Keyed weakly by the model so a
/// released model never aliases a later one at the same address; the value
/// key mirrors `ShellModel.editorMarksCache` (result id + revision + path) with
/// an item-count fingerprint. When the parent moves this into a stored
/// `ShellModel` property the map table goes away.
@MainActor
private let caretIndexCache = NSMapTable<ShellModel, CaretIndexBox>(
    keyOptions: [.weakMemory, .objectPointerPersonality], valueOptions: .strongMemory)

private final class CaretIndexBox {
    struct Key: Equatable { var resultID: String?; var revision: Int; var path: String; var pages: Int; var items: Int }
    let key: Key
    let index: CaretSync.Index
    init(key: Key, index: CaretSync.Index) { self.key = key; self.index = index }
}

extension ShellModel {
    /// Sorted index of the current result's items for the active document,
    /// rebuilt only when the result or the active document changes.
    func caretIndex() -> CaretSync.Index? {
        guard let result else { return nil }
        let key = CaretIndexBox.Key(resultID: resultID, revision: result.revision, path: activePath,
                                    pages: result.pages.count, items: result.pages.reduce(0) { $0 + $1.items.count })
        if let box = caretIndexCache.object(forKey: self), box.key == key { return box.index }
        let index = CaretSync.Index(result: result, path: activePath)
        caretIndexCache.setObject(CaretIndexBox(key: key, index: index), forKey: self)
        return index
    }

    /// `caretItems` through the sorted index: O(log n) per caret move instead
    /// of a scan over every page. Same values as `CaretSync.indicesByPage`.
    var exactCaretItems: [Int: Set<Int>] {
        guard historicalPreview == nil else { return [:] } // no caret sync onto an older snapshot
        guard let index = caretIndex(), let byte = CaretSync.byteOffset(ofCaretUTF16: caretUTF16, in: activeText) else { return [:] }
        return index.indicesByPage(byte: byte)
    }
}
