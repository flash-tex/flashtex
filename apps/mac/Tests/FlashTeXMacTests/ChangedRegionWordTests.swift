import XCTest
import FlashTeXProtocol

/// `SourceMapping.changedRegion` compares a word at a time (APP-EDITOR-INSTANT:
/// the editor's marks are rebased per keystroke over the whole document); its
/// result is the byte loop's, edits at every position and length included.
final class ChangedRegionWordTests: XCTestCase {
    /// The byte-at-a-time algorithm it replaced.
    static func reference(_ old: String, _ new: String) -> (Int, Int, Int) {
        let o = Array(old.utf8), n = Array(new.utf8)
        func cont(_ b: UInt8) -> Bool { b & 0xC0 == 0x80 }
        var p = 0
        while p < o.count, p < n.count, o[p] == n[p] { p += 1 }
        while p > 0, (p < o.count && cont(o[p])) || (p < n.count && cont(n[p])) { p -= 1 }
        var s = 0
        while s < o.count - p, s < n.count - p, o[o.count - 1 - s] == n[n.count - 1 - s] { s += 1 }
        while s > 0, cont(o[o.count - s]) { s -= 1 }
        return (p, o.count - s, n.count - s)
    }

    func testMatchesTheByteLoop() {
        var rng = SystemRandomNumberGenerator()
        let alphabet: [String] = ["a", "b", "x", " ", "\n", "é", "€", "𝔸", "\\", "{"]
        func text(_ k: Int) -> String { (0 ..< k).map { _ in alphabet[Int.random(in: 0 ..< alphabet.count, using: &rng)] }.joined() }
        for _ in 0 ..< 2000 {
            let base = text(Int.random(in: 0 ..< 60, using: &rng))
            var chars = Array(base)
            let at = Int.random(in: 0 ... chars.count, using: &rng)
            let del = Int.random(in: 0 ... min(5, chars.count - at), using: &rng)
            chars.replaceSubrange(at ..< at + del, with: Array(text(Int.random(in: 0 ..< 6, using: &rng))))
            let edited = String(chars)
            let r = SourceMapping.changedRegion(from: base, to: edited)
            let (s, oe, ne) = Self.reference(base, edited)
            XCTAssertEqual([r.startByte, r.oldEndByte, r.newEndByte], [s, oe, ne], "\(base.debugDescription) → \(edited.debugDescription)")
        }
    }

    func testRepeatedTextAroundTheEdit() {
        let page = String(repeating: "abcdefgh", count: 1000)
        for at in [0, 1, 7, 8, 9, 4000, 7999, 8000] {
            var c = Array(page); c.insert("x", at: at)
            let edited = String(c)
            let r = SourceMapping.changedRegion(from: page, to: edited)
            let (s, oe, ne) = Self.reference(page, edited)
            XCTAssertEqual([r.startByte, r.oldEndByte, r.newEndByte], [s, oe, ne], "insert at \(at)")
        }
    }
}
