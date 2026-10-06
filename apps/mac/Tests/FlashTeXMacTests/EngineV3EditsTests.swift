import Foundation
import XCTest
@testable import FlashTeXMac

/// The byte splices the engine-v3 preview sends (EngineV3Session.swift): the
/// slow path's prefix/suffix diff and the fast path's arithmetic on the text
/// storage's edited range must both reproduce the new text from the old.
final class EngineV3EditsTests: XCTestCase {
    func apply(_ old: String, offset: Int, delete: Int, insert: String) -> String {
        var b = Array(old.utf8)
        b.replaceSubrange(offset ..< offset + delete, with: Array(insert.utf8))
        return String(decoding: b, as: UTF8.self)
    }

    func testSpliceReproducesTheNewText() {
        let cases: [(String, String)] = [
            ("hello world", "hello brave world"),
            ("hello world", "hello"),
            ("naïve café", "naïve cafés"),
            ("αβγ", "αδγ"),        // a 2-byte character replaced: never split
            ("日本語", "日本人語"),
            ("", "x"), ("x", ""), ("same", "same"),
            (String(repeating: "abcdefgh", count: 100), String(repeating: "abcdefgh", count: 50) + "Z" + String(repeating: "abcdefgh", count: 50)),
        ]
        for (old, new) in cases {
            let s = EngineV3Edits.splice(old: Array(old.utf8), new: Array(new.utf8))
            let insert = String(decoding: Array(new.utf8)[s.insertRange], as: UTF8.self)
            XCTAssertEqual(apply(old, offset: s.offset, delete: s.delete, insert: insert), new, "\(old) → \(new)")
            XCTAssertNotNil(String(validatingUTF8: Array(insert.utf8).map { CChar(bitPattern: $0) } + [0]), "insert splits a character")
        }
    }

    /// The fast path: from the storage's new text, its edited range and
    /// change in length, plus the byte length the host holds.
    func testFastPathArithmetic() {
        let cases: [(String, NSRange, String)] = [ // old text, old UTF-16 range replaced, replacement
            ("naïve café text", NSRange(location: 6, length: 0), "x"),
            ("naïve café text", NSRange(location: 9, length: 1), ""),       // delete é
            ("日本語 and more", NSRange(location: 1, length: 1), "ab"),
            ("emoji 😀 here", NSRange(location: 6, length: 2), "🎉!"),
        ]
        for (old, oldRange, repl) in cases {
            let newText = (old as NSString).replacingCharacters(in: oldRange, with: repl) as NSString
            let edited = NSRange(location: oldRange.location, length: (repl as NSString).length)
            let delta = edited.length - oldRange.length
            let base = old.utf8.count
            let prefix = EngineV3Edits.utf8Count(newText, NSRange(location: 0, length: edited.location))
            let insert = newText.substring(with: edited)
            let oldLength = edited.length - delta
            let delete: Int
            if oldLength == 0 { delete = 0 } else {
                let suffix = EngineV3Edits.utf8Count(newText, NSRange(location: NSMaxRange(edited), length: newText.length - NSMaxRange(edited)))
                delete = base - prefix - suffix
            }
            XCTAssertEqual(apply(old, offset: prefix, delete: delete, insert: insert), newText as String, "\(old) \(oldRange) \(repl)")
        }
    }

    /// LIVE-30MS: the fast path's anchors spare counting the whole text, and
    /// a run of edits (typing, backspaces, replacements, jumps elsewhere,
    /// multi-unit characters) carried through them gives exactly the splices
    /// counted from the ends, which reproduce the text.
    func testAnchoredSplicesEqualCountedOnes() {
        var rng = SystemRandomNumberGenerator()
        let pieces = ["x", "ab", "é", "日本", "😀", "a😀b", "\\cmd ", "\n", ""]
        for round in 0 ..< 20 {
            var text = String(repeating: "lorem ipsum dolor ñ 語 😀 sit amet\n", count: 40 + round)
            var anchors: EngineV3Edits.Anchors?
            var caret = (text as NSString).length / 2
            caret = (text as NSString).rangeOfComposedCharacterSequence(at: caret).location
            for _ in 0 ..< 200 {
                let ns = text as NSString
                let roll = Int.random(in: 0 ..< 10, using: &rng)
                var loc = caret, len = 0
                if roll == 0 { loc = Int.random(in: 0 ... ns.length, using: &rng) } // a jump
                if loc < ns.length { loc = ns.rangeOfComposedCharacterSequence(at: loc).location }
                if roll < 3, loc > 0 { // a backspace (or a few)
                    let back = min(loc, Int.random(in: 1 ... 3, using: &rng))
                    let from = ns.rangeOfComposedCharacterSequence(at: loc - back).location
                    len = loc - from; loc = from
                } else if roll == 3, loc < ns.length { // a replacement
                    let e = min(ns.length, loc + Int.random(in: 1 ... 4, using: &rng))
                    len = NSMaxRange(ns.rangeOfComposedCharacterSequence(at: e - 1)) - loc
                }
                let repl = roll < 3 ? "" : pieces.randomElement(using: &rng)!
                let base = text.utf8.count
                let new = ns.replacingCharacters(in: NSRange(location: loc, length: len), with: repl) as NSString
                let edited = NSRange(location: loc, length: (repl as NSString).length)
                let delta = edited.length - len
                let got = EngineV3Edits.fastSplice(text: new, edited: edited, delta: delta, base: base, anchors: anchors)
                let full = EngineV3Edits.fastSplice(text: new, edited: edited, delta: delta, base: base, anchors: nil)
                XCTAssertEqual(got.prefix, full.prefix, "round \(round): prefix")
                XCTAssertEqual(got.delete, full.delete, "round \(round): delete")
                XCTAssertEqual(got.total, full.total, "round \(round): total")
                XCTAssertEqual(apply(text, offset: got.prefix, delete: got.delete, insert: repl), new as String)
                anchors = got.anchors
                text = new as String
                caret = NSMaxRange(edited)
            }
        }
    }

    /// The slow path's byte check of the fast path's last splice.
    func testFastCheckHolds() {
        let text = "naïve 😀 café"
        let b = Array(text.utf8)
        XCTAssertTrue(EngineV3Session.fastCheckHolds((6, Array(b[6 ..< 10])), text))
        XCTAssertFalse(EngineV3Session.fastCheckHolds((5, Array(b[6 ..< 10])), text))
        XCTAssertFalse(EngineV3Session.fastCheckHolds((b.count - 1, [0x41, 0x42]), text))
        XCTAssertTrue(EngineV3Session.fastCheckHolds(nil, text))
    }
}
