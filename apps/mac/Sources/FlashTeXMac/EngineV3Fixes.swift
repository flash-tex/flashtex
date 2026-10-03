import Foundation
import FlashTeXProtocol

/// Mechanical fixes for the engine-v3 preview's diagnostics (DESIGN §10 app
/// parity, gaps B10 and B11). The old engine attached a `help.replacement`
/// to an unknown command when exactly one known command was closest
/// ("did you mean \textbf?"), and to `\2` (the digit without the
/// backslash); the Problems panel's "Fix…" and Tab at the caret apply it.
/// TeX's diag-v1 rows carry the undefined control sequence's exact range
/// (`tex/undefined-control-sequence`), so the same rule is applied here,
/// over the same command inventory the editor completes from (the bundled
/// `supported-latex.json`), with the old compiler's distance and tie rules
/// (`crates/compiler/src/vocabulary.rs`: Damerau distance ≤ 1 for names of
/// up to 3 characters, else ≤ 2; the nearest length wins; a tie is no fix).
/// TeX's own help text moves to the row's notes. Pure.
enum EngineV3Fixes {
    static let undefinedCode = "tex/undefined-control-sequence"

    /// The control sequence TeX names as undefined: the one at the end of
    /// what it had read of the innermost input level (the top line of its
    /// error context). Nil when that level, or any level, is a macro: the
    /// diagnostic's range is then the macro call in the document (`\\mycite`),
    /// not the undefined name inside its definition, and rewriting the call
    /// would break a correct user macro.
    static func undefinedName(trace: [(kind: String, before: String)]) -> String? {
        guard let inner = trace.first, !trace.contains(where: { $0.kind == "macro" }) else { return nil }
        let before = inner.before.trimmingCharacters(in: .whitespaces)
        guard let slash = before.lastIndex(of: "\\") else { return nil }
        let cs = String(before[slash...])
        let name = cs.dropFirst()
        guard !name.isEmpty, name.count == 1 || name.allSatisfy({ $0.isASCII && $0.isLetter }) else { return nil }
        return cs
    }

    /// `row` with its fix attached when it is TeX's undefined control
    /// sequence and the document text at its range is exactly the name TeX
    /// reports (`named`); `texts` are the texts the compile read.
    static func fix(_ row: RuntimeV1.Diagnostic, named: String?, texts: [String: String],
                    vocabulary: [String] = Completion.defaultSupported) -> RuntimeV1.Diagnostic {
        attach([row], named: [named], texts: texts, vocabulary: vocabulary)[0]
    }

    /// The rows with fixes attached (`named[i]`: the name TeX reports for row i).
    static func attach(_ rows: [RuntimeV1.Diagnostic], named: [String?], texts: [String: String],
                       vocabulary: [String] = Completion.defaultSupported) -> [RuntimeV1.Diagnostic] {
        zip(rows, named).map { row, named in
            guard row.code == undefinedCode, let named, let src = row.source, let text = texts[src.path],
                  let cs = text.utf8Slice(src.startByte, src.endByte), cs == named, cs.hasPrefix("\\"), cs.count >= 2 else { return row }
            let name = String(cs.dropFirst())
            var d = row
            let texHelp = d.help?.message
            if name.count == 1, let c = name.first, c.isASCII, c.isNumber {
                d.suggestion = name
                d.help = .init(message: "did you mean `\(name)` (without the backslash)?",
                               replacement: .init(startByte: src.startByte, endByte: src.endByte, text: name, path: src.path))
            } else if name.allSatisfy({ $0.isASCII && $0.isLetter }) {
                let close = closestCommands(name, vocabulary: vocabulary)
                switch close.count {
                case 0: return row
                case 1:
                    let fix = "\\" + close[0]
                    d.suggestion = fix
                    d.help = .init(message: "did you mean \(fix)?",
                                   replacement: .init(startByte: src.startByte, endByte: src.endByte, text: fix, path: src.path))
                default:
                    d.help = .init(message: "did you mean one of \(close.map { "\\" + $0 }.joined(separator: ", "))? (no automatic fix: they are equally close)")
                }
            } else {
                return row
            }
            if let texHelp, !texHelp.isEmpty { d.notes = (d.notes ?? []) + [texHelp] }
            return d
        }
    }

    /// The known commands closest to `name` (the old compiler's rule).
    static func closestCommands(_ name: String, vocabulary: [String]) -> [String] {
        let width = name.count
        let limit = width <= 3 ? 1 : 2
        var best = Int.max
        var matches: [String] = []
        for candidate in vocabulary {
            if candidate == name { return [] }
            guard abs(candidate.count - width) <= limit else { continue }
            let d = distance(name, candidate)
            guard d <= limit else { continue }
            if d < best { best = d; matches = [candidate] } else if d == best, !matches.contains(candidate) { matches.append(candidate) }
        }
        if let w = matches.map({ abs($0.count - width) }).min() { matches.removeAll { abs($0.count - width) != w } }
        return matches
    }

    /// Optimal string alignment distance (adjacent transpositions count one).
    static func distance(_ a: String, _ b: String) -> Int {
        let a = Array(a), b = Array(b)
        guard abs(a.count - b.count) <= 2 else { return Int.max }
        var d = [[Int]](repeating: [Int](repeating: 0, count: b.count + 1), count: a.count + 1)
        for i in 0 ... a.count { d[i][0] = i }
        for j in 0 ... b.count { d[0][j] = j }
        if a.isEmpty || b.isEmpty { return d[a.count][b.count] }
        for i in 1 ... a.count {
            for j in 1 ... b.count {
                let cost = a[i - 1] == b[j - 1] ? 0 : 1
                var v = min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + cost)
                if i > 1, j > 1, a[i - 1] == b[j - 2], a[i - 2] == b[j - 1] { v = min(v, d[i - 2][j - 2] + 1) }
                d[i][j] = v
            }
        }
        return d[a.count][b.count]
    }

    /// The file a LaTeX "File `x' not found" error names
    /// (`latex/file-not-found`: `\input`, `\usepackage`, `\documentclass`).
    static func missingFile(in message: String) -> String? {
        guard let open = message.range(of: "LaTeX Error: File `"), let close = message.range(of: "' not found", range: open.upperBound ..< message.endIndex) else { return nil }
        let name = String(message[open.upperBound ..< close.lowerBound])
        return name.isEmpty ? nil : name
    }
}

extension String {
    /// The text of UTF-8 bytes `start ..< end`, nil when out of range or not on scalar boundaries.
    func utf8Slice(_ start: Int, _ end: Int) -> String? {
        guard start >= 0, start <= end, end <= utf8.count else { return nil }
        let u = utf8
        guard let a = u.index(u.startIndex, offsetBy: start, limitedBy: u.endIndex),
              let b = u.index(u.startIndex, offsetBy: end, limitedBy: u.endIndex),
              let s = String(u[a ..< b]) else { return nil }
        return s
    }
}
