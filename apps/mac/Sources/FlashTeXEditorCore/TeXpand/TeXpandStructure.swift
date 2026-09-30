import Foundation

// The context structure editor (owner addition, PLAN M10b): find the grid
// environment around the caret, edit it as rows and cells, and write it
// back as one replacement. The editors draw the grid; everything here is
// pure and shared.

extension TeXpand {
    /// A kind of grid the structure editor opens. Each can be switched off
    /// with `disable = ["structure:NAME"]` (or the whole editor with
    /// `structure_editor = false`).
    public struct StructureProvider: Equatable, Sendable {
        public var name: String
        public var environments: [String]
        /// A column spec argument follows `\begin{env}` (after `leadingArgs`).
        public var hasColspec: Bool
        /// Mandatory arguments before the spec (`tabularx`'s width,
        /// `alignat`'s count): kept as they are.
        public var leadingArgs: [String: Int]
        /// Environments the type can be switched between (`pmatrix` ↔ `bmatrix`).
        public var family: [String]

        public static let builtIns: [StructureProvider] = [
            StructureProvider(name: "matrix",
                              environments: ["matrix", "pmatrix", "bmatrix", "Bmatrix", "vmatrix", "Vmatrix", "smallmatrix"],
                              hasColspec: false, leadingArgs: [:],
                              family: ["matrix", "pmatrix", "bmatrix", "Bmatrix", "vmatrix", "Vmatrix"]),
            StructureProvider(name: "tabular",
                              environments: ["tabular", "tabular*", "tabularx", "tabulary", "array", "longtable"],
                              hasColspec: true, leadingArgs: ["tabular*": 1, "tabularx": 1, "tabulary": 1], family: []),
            StructureProvider(name: "cases", environments: ["cases", "dcases", "rcases"], hasColspec: false, leadingArgs: [:],
                              family: ["cases", "dcases", "rcases"]),
            StructureProvider(name: "align",
                              environments: ["align", "align*", "gather", "gather*", "alignat", "alignat*", "flalign", "flalign*",
                                             "aligned", "gathered", "split", "eqnarray", "eqnarray*"],
                              hasColspec: false, leadingArgs: ["alignat": 1, "alignat*": 1], family: []),
        ]

        /// The providers `settings` leave on.
        public static func enabled(_ settings: Settings) -> [StructureProvider] {
            guard settings.isActive(.structureEditor) else { return [] }
            return builtIns.filter { !settings.disabled.contains("structure:" + $0.name) }
        }
    }

    /// A grid environment in the text, parsed.
    public struct StructureTarget: Equatable, Sendable {
        public var provider: StructureProvider
        /// `\begin{…}` through `\end{…}`.
        public var range: NSRange
        public var document: StructureDocument
        /// The `\begin` line's indentation.
        public var indent: String
    }

    /// An environment's editable form: its name, its arguments and its grid.
    public struct StructureDocument: Equatable, Sendable {
        public var environment: String
        /// `[pos]` and arguments before the spec, verbatim (`{\linewidth}`).
        public var leading: String
        public var colspec: ColumnSpec?
        public var grid: Grid

        /// The number of columns: the spec's, or the widest row.
        public var columnCount: Int { max(colspec?.columnCount ?? 0, grid.columnCount, 1) }

        /// Every cell row padded to the column count (what the editor shows).
        public var cells: [[String]] {
            let n = columnCount
            return grid.rows.compactMap { if case .cells(let c) = $0 { return c + Array(repeating: "", count: max(0, n - c.count)) }; return nil }
        }

        /// Index in `grid.rows` of cell row `r`.
        func rowIndex(_ r: Int) -> Int? {
            var k = -1
            for (i, row) in grid.rows.enumerated() { if case .cells = row { k += 1; if k == r { return i } } }
            return nil
        }

        public mutating func setCell(_ r: Int, _ c: Int, _ text: String) {
            guard let i = rowIndex(r), case .cells(var cells) = grid.rows[i] else { return }
            while cells.count <= c { cells.append("") }
            cells[c] = text
            grid.rows[i] = .cells(cells)
        }

        /// A new empty row after cell row `r` (at the top when `r` is -1).
        public mutating func addRow(after r: Int) {
            let row = Grid.Row.cells(Array(repeating: "", count: columnCount))
            if r < 0 { grid.rows.insert(row, at: grid.rows.firstIndex { if case .cells = $0 { return true }; return false } ?? 0); return }
            guard let i = rowIndex(r) else { grid.rows.append(row); return }
            grid.rows.insert(row, at: i + 1)
        }

        public mutating func removeRow(_ r: Int) {
            guard cells.count > 1, let i = rowIndex(r) else { return }
            grid.rows.remove(at: i)
        }

        /// A new empty column after column `c` (first when -1); the spec gains
        /// a column of the same kind as its neighbour.
        public mutating func addColumn(after c: Int) {
            let n = columnCount
            grid.rows = grid.rows.map { row in
                guard case .cells(var cells) = row else { return row }
                while cells.count < n { cells.append("") }
                cells.insert("", at: min(c + 1, cells.count))
                return .cells(cells)
            }
            colspec?.insertColumn(after: c)
        }

        public mutating func removeColumn(_ c: Int) {
            guard columnCount > 1 else { return }
            grid.rows = grid.rows.map { row in
                guard case .cells(var cells) = row, c < cells.count else { return row }
                cells.remove(at: c)
                return .cells(cells)
            }
            colspec?.removeColumn(c)
        }

        /// Source text of the whole environment, and each cell's start offset
        /// in it (for placing the caret back in the edited cell).
        public func render(indent: String, unit: String) -> (text: String, cellOffsets: [[Int]]) {
            var out = "\\begin{\(environment)}" + leading
            if let colspec { out += "{" + colspec.text + "}" }
            var offsets: [[Int]] = []
            let n = columnCount
            var rows = grid.rows
            if let last = rows.lastIndex(where: { if case .cells = $0 { return true }; return false }) {
                // Pad cell rows so every row has every column's `&`.
                for i in rows.indices { if case .cells(let c) = rows[i], c.count < n, i <= last { rows[i] = .cells(c + Array(repeating: "", count: n - c.count)) } }
            }
            for (k, row) in rows.enumerated() {
                out += "\n" + indent + unit
                switch row {
                case .rule(let r): out += r
                case .cells(let cells):
                    var starts: [Int] = []
                    for (j, cell) in cells.enumerated() {
                        if j > 0 { out += " & " }
                        starts.append((out as NSString).length)
                        out += cell
                    }
                    offsets.append(starts)
                    if k < rows.count - 1 { out += " \\\\" }
                }
            }
            out += "\n" + indent + "\\end{\(environment)}"
            return (out, offsets)
        }
    }

    /// A tabular column spec as columns and the decorations between them
    /// (`|`, `@{…}`, `>{…}`), so columns can be added and removed in place.
    public struct ColumnSpec: Equatable, Sendable {
        public enum Part: Equatable, Sendable {
            case column(String)
            case other(String)
        }
        public var parts: [Part]

        public init?(_ text: String) {
            var parts: [Part] = []
            let cs = Array(text)
            var i = 0
            func group() -> String? {
                guard i < cs.count, cs[i] == "{" else { return nil }
                var depth = 0
                let a = i
                while i < cs.count {
                    if cs[i] == "{" { depth += 1 } else if cs[i] == "}" { depth -= 1; if depth == 0 { i += 1; return String(cs[a..<i]) } }
                    i += 1
                }
                return nil
            }
            while i < cs.count {
                let c = cs[i]
                i += 1
                switch c {
                case " ", "\t": continue
                case "|": parts.append(.other("|"))
                case "@", "!", ">", "<":
                    guard let g = group() else { return nil }
                    parts.append(.other(String(c) + g))
                case "p", "m", "b":
                    guard let g = group() else { return nil }
                    parts.append(.column(String(c) + g))
                case "*":
                    guard let count = group(), let inner = group(), let n = Int(count.dropFirst().dropLast()),
                          let innerSpec = ColumnSpec(String(inner.dropFirst().dropLast())) else { return nil }
                    for _ in 0..<n { parts += innerSpec.parts }
                case _ where c.isLetter:
                    parts.append(.column(String(c)))
                default:
                    return nil
                }
            }
            guard parts.contains(where: { if case .column = $0 { return true }; return false }) else { return nil }
            self.parts = parts
        }

        public var text: String {
            parts.map { switch $0 { case .column(let s), .other(let s): return s } }.joined()
        }

        public var columnCount: Int { parts.filter { if case .column = $0 { return true }; return false }.count }

        func partIndex(ofColumn c: Int) -> Int? {
            var k = -1
            for (i, p) in parts.enumerated() { if case .column = p { k += 1; if k == c { return i } } }
            return nil
        }

        mutating func insertColumn(after c: Int) {
            let like: String
            if let i = partIndex(ofColumn: max(0, c)), case .column(let s) = parts[i] { like = s } else { like = "l" }
            if c < 0 {
                parts.insert(.column(like), at: partIndex(ofColumn: 0) ?? 0)
            } else if let i = partIndex(ofColumn: c) {
                // After the column and a `|` that follows it, keeping rules between columns.
                var at = i + 1
                if at < parts.count, parts[at] == .other("|") {
                    parts.insert(contentsOf: [.column(like), .other("|")], at: at + 1)
                    return
                }
                at = min(at, parts.count)
                parts.insert(.column(like), at: at)
            } else {
                parts.append(.column(like))
            }
        }

        mutating func removeColumn(_ c: Int) {
            guard let i = partIndex(ofColumn: c) else { return }
            parts.remove(at: i)
            // A rule left between two rules or at an end goes with it.
            if i < parts.count, parts[i] == .other("|"), i == 0 || parts[i - 1] == .other("|") { parts.remove(at: i) }
            else if i > 0, i == parts.count, parts[i - 1] == .other("|"), i - 1 > 0, parts[i - 2] == .other("|") { parts.remove(at: i - 1) }
        }
    }

    /// The innermost grid environment around `caret` that `providers`
    /// handle, parsed; nil outside one (the command then opens the prompt).
    public static func structure(at caret: Int, in text: NSString, providers: [StructureProvider]) -> StructureTarget? {
        let pairs = LaTeXScan.environmentPairs(in: text).filter { p in
            guard let end = p.end else { return false }
            return p.begin.location <= caret && caret <= NSMaxRange(end) && providers.contains { $0.environments.contains(p.name) }
        }
        guard let pair = pairs.max(by: { $0.begin.location < $1.begin.location }), let end = pair.end,
              let provider = providers.first(where: { $0.environments.contains(pair.name) }) else { return nil }
        // Arguments after `\begin{env}`.
        var i = NSMaxRange(pair.begin)
        func skipSpaces() { while i < end.location, text.character(at: i) == 0x20 { i += 1 } }
        func group(open: unichar, close: unichar) -> String? {
            skipSpaces()
            guard i < end.location, text.character(at: i) == open else { return nil }
            var depth = 0
            let a = i
            while i < end.location {
                let c = text.character(at: i)
                if c == open { depth += 1 } else if c == close { depth -= 1; if depth == 0 { i += 1; return text.substring(with: NSRange(location: a, length: i - a)) } }
                i += 1
            }
            return nil
        }
        var leading = ""
        if let pos = group(open: 0x5B, close: 0x5D) { leading += pos } // [t]
        for _ in 0..<(provider.leadingArgs[pair.name] ?? 0) {
            guard let g = group(open: 0x7B, close: 0x7D) else { return nil }
            leading += g
        }
        var colspec: ColumnSpec?
        if provider.hasColspec {
            guard let g = group(open: 0x7B, close: 0x7D), let spec = ColumnSpec(String(g.dropFirst().dropLast())) else { return nil }
            colspec = spec
        }
        let body = text.substring(with: NSRange(location: i, length: end.location - i))
        var lineStart = pair.begin.location
        while lineStart > 0, text.character(at: lineStart - 1) != 0x0A { lineStart -= 1 }
        var e = lineStart
        while e < pair.begin.location, text.character(at: e) == 0x20 || text.character(at: e) == 0x09 { e += 1 }
        let indent = text.substring(with: NSRange(location: lineStart, length: e - lineStart))
        let document = StructureDocument(environment: pair.name, leading: leading, colspec: colspec, grid: Grid.parse(body))
        return StructureTarget(provider: provider, range: NSRange(location: pair.begin.location, length: NSMaxRange(end) - pair.begin.location),
                               document: document, indent: indent)
    }
}
