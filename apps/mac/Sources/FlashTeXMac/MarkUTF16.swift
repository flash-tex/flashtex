import Foundation
import FlashTeXProtocol

/// UTF-16 ranges for diagnostic marks without a walk of the editor's new text
/// on every keystroke (APP-KEYPATH).
///
/// Every keystroke gives the model a new `String`, and the marks are rebased
/// onto it (`EditorDiagnostics.report`, from `ShellModel.editorMarks` in the
/// editor pane's body). A mark's UTF-16 range was `NSRange(r, in: current)`:
/// the first UTF-16 offset asked of a non-ASCII native string builds its
/// UTF-16 breadcrumbs, a walk of the whole text. Measured 4.6 ms at 3 MB
/// (`swiftc -O`, mac-m5pro-kabir), on every keystroke, against ~1 ms for the
/// buffer copy itself.
///
/// Here the offsets come from an *anchor*: an earlier text of the document
/// whose breadcrumbs were built once, off the main thread (normally the
/// compiled text the marks came from). Bytes before the changed region have
/// the anchor's offsets; bytes after it, the anchor's shifted by the region's
/// UTF-16 delta; bytes inside it are counted in the region alone. The result
/// is exactly `NSRange(r, in: current)` (`MarkUTF16Tests` fuzzes it). Where
/// no anchor is warm yet, or the texts differ by more than `maxRegionBytes`,
/// the current text is walked as before.
final class MarkUTF16Anchors: @unchecked Sendable {
    static let shared = MarkUTF16Anchors()

    /// Texts this size or smaller are converted directly (cheap either way).
    static let minBytes = 4096
    /// An anchor further than this from the current text is not used.
    static let maxRegionBytes = 256 * 1024
    /// Anchors kept (each one text and its breadcrumbs, ~1.1x the text).
    static let capacity = 2

    private let lock = NSLock()
    private var warm: [String] = []
    private var warming: [String] = []
    private var walkedBytes = 0
    private var anchoredReports = 0

    /// Bytes of non-ASCII current texts walked from their start for a UTF-16
    /// offset (the breadcrumb build): the per-keystroke O(document) work
    /// this type removes. Tests and evidence.
    var freshWalkBytes: Int { lock.withLock { walkedBytes } }
    /// Reports whose ranges came from an anchor (tests and evidence).
    var anchored: Int { lock.withLock { anchoredReports } }

    func resetCounters() { lock.withLock { walkedBytes = 0; anchoredReports = 0 } }

    /// Drops every anchor (tests).
    func reset() { lock.withLock { warm = []; warming = []; walkedBytes = 0; anchoredReports = 0 } }

    /// Waits until no anchor is being prepared (tests).
    func settle(timeout: TimeInterval = 10) {
        let deadline = Date().addingTimeInterval(timeout)
        while lock.withLock({ !warming.isEmpty }), Date() < deadline { usleep(1000) }
    }

    func noteWalk(_ bytes: Int) { lock.withLock { walkedBytes += bytes } }
    func noteAnchored() { lock.withLock { anchoredReports += 1 } }

    /// The warm anchors, most recently warmed first.
    func anchors() -> [String] { lock.withLock { warm } }

    func isWarm(_ text: String) -> Bool { lock.withLock { warm.contains { MarkUTF16.sameStorage($0, text) } } }

    /// Builds `text`'s UTF-16 breadcrumbs on a utility queue, then makes it
    /// an anchor. A text already warm or being warmed is left alone.
    func prepare(_ text: String) {
        let start: Bool = lock.withLock {
            if warm.contains(where: { MarkUTF16.sameStorage($0, text) })
                || warming.contains(where: { MarkUTF16.sameStorage($0, text) }) { return false }
            warming.append(text)
            return true
        }
        guard start else { return }
        DispatchQueue.global(qos: .utility).async { [self] in
            _ = text.utf16.count // builds and caches the breadcrumbs in the string's storage (thread-safe)
            lock.withLock {
                warming.removeAll { MarkUTF16.sameStorage($0, text) }
                warm.insert(text, at: 0)
                if warm.count > Self.capacity { warm.removeLast(warm.count - Self.capacity) }
            }
        }
    }
}

/// One report's conversion of scalar-aligned ranges of `current` to UTF-16.
/// The anchor is chosen at the first range: a report with no mark on the
/// document does nothing here.
final class MarkUTF16 {
    private let current: String
    private let compiled: String?
    private let region: SourceMapping.ChangedRegion?
    private let anchors: MarkUTF16Anchors
    private var chosen = false
    private var map: Map?

    /// `region` is `changedRegion(from: compiled, to: current)`, nil when
    /// there is no compiled text or it has the same bytes.
    init(current: String, compiled: String?, region: SourceMapping.ChangedRegion?, anchors: MarkUTF16Anchors = .shared) {
        self.current = current
        self.compiled = compiled
        self.region = region
        self.anchors = anchors
    }

    /// `NSRange(r, in: current)`, for `r` on unicode-scalar boundaries.
    func nsRange(_ r: Range<String.Index>) -> NSRange {
        if !chosen { chosen = true; map = makeMap() }
        let u = current.utf8
        guard let map, let lo = map.utf16(ofByte: u.distance(from: u.startIndex, to: r.lowerBound)),
              let hi = map.utf16(ofByte: u.distance(from: u.startIndex, to: r.upperBound)) else {
            return NSRange(r, in: current)
        }
        return NSRange(location: lo, length: hi - lo)
    }

    /// Byte → UTF-16 offset in `current`, through an anchor and the region
    /// that differs between them (`identical`: the same bytes).
    struct Map {
        let anchor: String
        let current: String
        let start: Int, oldEnd: Int, newEnd: Int
        let identical: Bool
        let u16Start: Int, u16OldEnd: Int, u16Replacement: Int

        /// O(1) once `s`'s breadcrumbs exist (or `s` is ASCII).
        static func u16(_ s: String, _ byte: Int) -> Int {
            let u = s.utf8
            return s.utf16.distance(from: s.utf16.startIndex, to: u.index(u.startIndex, offsetBy: byte))
        }

        /// UTF-16 units of UTF-8 bytes `a..<b`: one per scalar, two for a 4-byte one.
        static func u16Count(_ s: String, _ a: Int, _ b: Int) -> Int? {
            s.utf8.withContiguousStorageIfAvailable { bytes in
                var n = 0
                for k in a ..< b {
                    let c = bytes[k]
                    if c & 0xC0 != 0x80 { n += c >= 0xF0 ? 2 : 1 }
                }
                return n
            }
        }

        func utf16(ofByte b: Int) -> Int? {
            if identical || b <= start { return Self.u16(anchor, b) }
            if b >= newEnd { return Self.u16(anchor, b - newEnd + oldEnd) - u16OldEnd + u16Start + u16Replacement }
            return Self.u16Count(current, start, b).map { u16Start + $0 }
        }
    }

    private func makeMap() -> Map? {
        let count = current.utf8.count
        guard count > MarkUTF16Anchors.minBytes, Self.isNativeUTF8(current) else { return nil }
        if anchors.isWarm(current) { return nil } // its own offsets are already O(1)
        let warm = anchors.anchors()
        // The compiled text is the anchor the marks rebase from anyway.
        if let compiled { anchors.prepare(compiled) }
        var candidates: [(anchor: String, region: SourceMapping.ChangedRegion?, given: Bool)] = []
        if let compiled, warm.contains(where: { Self.sameStorage($0, compiled) }) { candidates.append((compiled, region, true)) }
        // Another warm text (an earlier compile's, or this text a few keys
        // ago) when its length is near enough to be the same document.
        for a in warm where !(compiled.map { Self.sameStorage(a, $0) } ?? false)
            && abs(a.utf8.count - count) <= MarkUTF16Anchors.maxRegionBytes { candidates.append((a, nil, false)) }
        for c in candidates where Self.isNativeUTF8(c.anchor) {
            let r: SourceMapping.ChangedRegion?
            if c.given { r = c.region } else if c.anchor.sameBytes(as: current) { r = nil } else {
                r = SourceMapping.changedRegion(from: c.anchor, to: current)
            }
            if let r, max(r.oldEndByte, r.newEndByte) - r.startByte > MarkUTF16Anchors.maxRegionBytes { continue }
            anchors.noteAnchored()
            return Self.map(anchor: c.anchor, region: r, current: current)
        }
        // The conversions build `current`'s breadcrumbs (an ASCII text needs
        // none). With no compiled text to wait for, this text becomes the
        // anchor for the keys after it.
        if !Self.isASCII(current) { anchors.noteWalk(count) }
        if compiled == nil { anchors.prepare(current) }
        return nil
    }

    private static func map(anchor: String, region r: SourceMapping.ChangedRegion?, current: String) -> Map? {
        guard let r else {
            return Map(anchor: anchor, current: current, start: 0, oldEnd: 0, newEnd: 0, identical: true,
                       u16Start: 0, u16OldEnd: 0, u16Replacement: 0)
        }
        guard let replacement = Map.u16Count(current, r.startByte, r.newEndByte) else { return nil }
        return Map(anchor: anchor, current: current, start: r.startByte, oldEnd: r.oldEndByte, newEnd: r.newEndByte, identical: false,
                   u16Start: Map.u16(anchor, r.startByte), u16OldEnd: Map.u16(anchor, r.oldEndByte), u16Replacement: replacement)
    }

    static func isNativeUTF8(_ s: String) -> Bool { s.utf8.withContiguousStorageIfAvailable { _ in true } ?? false }

    static func isASCII(_ s: String) -> Bool {
        s.utf8.withContiguousStorageIfAvailable { b in
            var i = 0
            let n = b.count
            if let p = b.baseAddress {
                while i + 8 <= n {
                    if UnsafeRawPointer(p + i).loadUnaligned(as: UInt64.self) & 0x8080_8080_8080_8080 != 0 { return false }
                    i += 8
                }
            }
            while i < n { if b[i] >= 0x80 { return false }; i += 1 }
            return true
        } ?? false
    }

    /// The same UTF-8 storage (not merely equal bytes): O(1).
    static func sameStorage(_ a: String, _ b: String) -> Bool {
        guard let pa = a.utf8.withContiguousStorageIfAvailable({ ($0.baseAddress, $0.count) }),
              let pb = b.utf8.withContiguousStorageIfAvailable({ ($0.baseAddress, $0.count) }),
              pa.1 > 15 else { return false } // small strings live inline: no identity
        return pa.0 == pb.0 && pa.1 == pb.1
    }
}
