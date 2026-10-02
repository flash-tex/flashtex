import Foundation

extension TeXpand {
    /// Wrap mode's lines (PLAN §9.6): the non-blank lines of a selection,
    /// trimmed, with a leading list marker (`-`, `*`, `+`, `•`, `1.`, `1)`)
    /// stripped when `stripMarkers`.
    public static func wrapLines(_ selection: String, stripMarkers: Bool = true) -> [String] {
        selection.components(separatedBy: "\n").compactMap { raw in
            var line = raw.trimmingCharacters(in: .whitespaces)
            guard !line.isEmpty else { return nil }
            if stripMarkers, let r = line.range(of: #"^(?:[-*+•]|\d+[.)]|\\item)\s+"#, options: .regularExpression) {
                line = String(line[r.upperBound...])
            }
            return line
        }
    }

    /// Relations the `align` wrap transformer aligns on, longest first.
    static let relations = ["\\approx", "\\equiv", "\\leq", "\\geq", "\\neq", "\\sim", "\\cong", "\\propto",
                            "\\le", "\\ge", "\\to", "\\mapsto", "\\iff", "\\implies", "<=", ">=", "=", "<", ">"]

    /// One `align` row: `&` before the first relation at brace depth 0
    /// (`a = b` → `a &= b`); a line with `&` already, or no relation, as is.
    public static func alignRow(_ line: String) -> String {
        guard !line.contains("&") else { return line }
        var depth = 0
        var i = line.startIndex
        while i < line.endIndex {
            let c = line[i]
            if c == "{" { depth += 1 } else if c == "}" { depth -= 1 }
            if depth == 0, let rel = relations.first(where: { line[i...].hasPrefix($0) }) {
                // `\leq` must not be the start of `\leqslant`.
                let end = line.index(i, offsetBy: rel.count)
                let continues = rel.hasPrefix("\\") && end < line.endIndex && line[end].isLetter
                if !continues {
                    let head = line[..<i]
                    return (head.hasSuffix(" ") ? String(head) : String(head) + (head.isEmpty ? "" : " ")) + "&" + line[i...]
                }
            }
            if c == "\\" { i = line.index(after: i); if i < line.endIndex, line[i].isLetter { while i < line.endIndex, line[i].isLetter { i = line.index(after: i) }; continue } }
            if i < line.endIndex { i = line.index(after: i) }
        }
        return line
    }

    /// Table cells from wrapped lines: split on tabs when any line has one,
    /// else on commas at brace depth 0.
    public static func tableCells(_ lines: [String]) -> [[String]] {
        let tabs = lines.contains { $0.contains("\t") }
        return lines.map { line in
            (tabs ? line.components(separatedBy: "\t") : splitTopLevel(line, on: ","))
                .map { $0.trimmingCharacters(in: .whitespaces) }
        }
    }
}
