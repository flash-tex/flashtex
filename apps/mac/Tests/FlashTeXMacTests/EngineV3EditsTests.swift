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
}
