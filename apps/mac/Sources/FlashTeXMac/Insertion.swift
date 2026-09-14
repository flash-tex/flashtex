import Foundation
import FlashTeXProtocol

/// A Mac-pinned insertion destination (`destination_id` in the contract).
/// Stores the UTF-8 byte offset in a document at a given editor revision so a
/// later proposal can be rebased or rejected honestly.
struct InsertionAnchor: Equatable {
    let id: String
    let path: String
    let byteOffset: Int
    let revision: Int
    /// Short context after the anchor, used to re-find it if the buffer changed.
    let contextAfter: String
}

/// Pure insertion logic, kept out of the view model for testing.
enum Insertion {
    static let contextLength = 24

    static func makeAnchor(id: String, path: String, text: String, caretUTF16: Int, revision: Int) -> InsertionAnchor? {
        guard let r = Range(NSRange(location: caretUTF16, length: 0), in: text) else { return nil }
        let byte = text.utf8.distance(from: text.utf8.startIndex, to: r.lowerBound)
        let ctx = String(text[r.lowerBound...].prefix(contextLength))
        return InsertionAnchor(id: id, path: path, byteOffset: byte, revision: revision, contextAfter: ctx)
    }

    enum Resolution: Equatable {
        case exact(byteOffset: Int)
        case rebased(byteOffset: Int)
        case needsReselection(String)
    }

    /// Resolves an anchor against the current buffer. Exact if the revision is
    /// unchanged; otherwise re-finds the stored context. Ambiguous or missing
    /// context requires reselection rather than guessing.
    static func resolve(_ anchor: InsertionAnchor, in text: String, revision: Int) -> Resolution {
        if revision == anchor.revision {
            return anchor.byteOffset <= text.utf8.count ? .exact(byteOffset: anchor.byteOffset)
                : .needsReselection("anchor beyond end of buffer")
        }
        if anchor.contextAfter.isEmpty {
            // Anchor was at end of buffer; keep it at end if the buffer still ends the same way.
            return .rebased(byteOffset: text.utf8.count)
        }
        var matches: [Int] = []
        var search = text.startIndex
        while let r = text.range(of: anchor.contextAfter, range: search..<text.endIndex) {
            matches.append(text.utf8.distance(from: text.utf8.startIndex, to: r.lowerBound))
            search = text.index(after: r.lowerBound)
        }
        switch matches.count {
        case 1: return .rebased(byteOffset: matches[0])
        case 0: return .needsReselection("destination text was deleted or changed")
        default:
            if matches.contains(anchor.byteOffset) { return .rebased(byteOffset: anchor.byteOffset) }
            return .needsReselection("destination is ambiguous after edits")
        }
    }

    /// Text to insert: proposal LaTeX, trimmed, with surrounding newlines when the
    /// insertion point is not already at a line boundary.
    static func insertionText(_ latex: String, into text: String, atByte byte: Int) -> String {
        let body = latex.trimmingCharacters(in: .whitespacesAndNewlines)
        let u = text.utf8
        let idx = u.index(u.startIndex, offsetBy: byte)
        let atLineStart = idx == text.startIndex || text[text.index(before: idx)] == "\n"
        let atLineEnd = idx == text.endIndex || text[idx] == "\n"
        return (atLineStart ? "" : "\n") + body + (atLineEnd ? "" : "\n")
    }

    /// Text to insert for a *capture* proposal: the same layout, but only after
    /// the proposal has been made legal for the caret it is landing on
    /// (`CaretContext.normalize`). Returns nil when no safe insertion exists,
    /// with the reason in `advisories`.
    ///
    /// Separate from `insertionText` on purpose: scaffolding inserts
    /// (`\input{…}`) are the app's own text and are not recognised output, so
    /// they are not second-guessed.
    ///
    /// Only a display-math insertion gets the surrounding newlines. Padding an
    /// inline insertion would break a sentence across lines, and padding one
    /// inside a `%` comment would push the text out of the comment entirely.
    static func captureInsertion(_ latex: String, into text: String, atByte byte: Int)
        -> (text: String?, advisories: [String], caret: CaretContext) {
        let caret = CaretContext.derive(text, caretByte: byte)
        let normalized = caret.normalize(latex)
        guard let body = normalized.text else {
            return (nil, normalized.advisories, caret)
        }
        let laidOut = caret.wrap == .display ? insertionText(body, into: text, atByte: byte) : body
        return (laidOut, normalized.advisories, caret)
    }
}
