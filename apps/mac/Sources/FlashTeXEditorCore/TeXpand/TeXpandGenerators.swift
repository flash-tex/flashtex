import Foundation

extension TeXpand {
    /// Built-in computed expansions (PLAN §3 "Generators"). A generator
    /// returns a template (holes and all), which the expander renders like
    /// any definition body — so tabstops, labels, children and profile keys
    /// work the same way in computed output. M2 ships the two the text
    /// catalog needs (`table`, `columns`); the math generators (matrix,
    /// sequence, rotation) are M4, scripted ones M11.
    public enum Generators {
        public static let known: Set<String> = ["table", "columns"]

        public struct Call {
            public var element: Element
            public var definition: Definition
            /// Declared params: the given value (or the default's), and
            /// whether it was given.
            public var params: [String: (value: TypedParam?, given: Bool)]
            public var args: [String]
            public var options: TOMLTable
        }

        /// Whether the generator's output has a `<<children>>` hole.
        static func placesChildren(_ name: String) -> Bool { name == "columns" }
        /// Whether the generator's output has a `<<label>>` hole.
        static func placesLabel(_ name: String) -> Bool { false }
        /// `{…}` arguments the generator reads.
        static func arity(_ name: String) -> Int { name == "table" ? 1 : 0 }

        static func template(_ name: String, _ call: Call) throws -> String {
            switch name {
            case "table": return try table(call)
            case "columns": return columns(call)
            default: throw ExpandError(offset: call.element.offset, message: "unknown generator `\(name)`")
            }
        }

        /// Literal user text inside a generated template.
        static func escape(_ s: String) -> String { s.replacingOccurrences(of: "<<", with: "\\<<") }

        /// `tab:lcr:4`, `btab:lrr:5{Name,Score,Time}`: a tabular of `rows`
        /// data rows with one tabstop per cell; `booktabs = true` adds the
        /// rules and a header row (from the argument, else tabstops).
        static func table(_ c: Call) throws -> String {
            let spec = c.params["spec"]?.value
            guard let ncols = spec?.field("ncols").flatMap(Int.init), ncols > 0 else {
                throw ExpandError(offset: c.element.offset, message: "`\(c.element.name)` needs a column specification such as `lcr`")
            }
            let rows = c.params["rows"]?.value?.field("value").flatMap(Int.init) ?? 3
            guard rows >= 1, rows <= 200 else { throw ExpandError(offset: c.element.offset, message: "a table has 1 to 200 rows") }
            let booktabs = c.options["booktabs"]?.bool ?? false
            var next = 0
            func stops(_ n: Int) -> [String] { (0..<n).map { _ in next += 1; return "<<\(next)>>" } }

            var grid: [Grid.Row] = []
            if booktabs { grid.append(.rule("\\toprule")) }
            if let header = c.args.first {
                let cells = splitTopLevel(header, on: ",").map { escape($0.trimmingCharacters(in: .whitespaces)) }
                guard cells.count == ncols else {
                    throw ExpandError(offset: c.element.offset, message: "the header has \(cells.count) cell\(cells.count == 1 ? "" : "s") but the table has \(ncols) column\(ncols == 1 ? "" : "s")")
                }
                grid.append(.cells(cells))
                grid.append(.rule(booktabs ? "\\midrule" : "\\hline"))
            } else if booktabs {
                grid.append(.cells(stops(ncols)))
                grid.append(.rule("\\midrule"))
            }
            for _ in 0..<rows { grid.append(.cells(stops(ncols))) }
            if booktabs { grid.append(.rule("\\bottomrule")) }

            let env = c.options["env"]?.string ?? "tabular"
            var out = "\\begin{\(env)}{<<p.spec.value>>}"
            for line in Grid(rows: grid).lines() { out += "\n  " + line }
            return out + "\n\\end{\(env)}"
        }

        /// `cols:6,4`: a beamer `columns` environment, one `column` per width
        /// (`6` → 0.6\textwidth, `45` → 0.45\textwidth, a length such as
        /// `3cm` as written). Children go in the first column.
        static func columns(_ c: Call) -> String {
            let widths = c.params["widths"]?.value?.lists["items"] ?? ["5", "5"]
            var out = "\\begin{columns}<<opt>>"
            for (k, w) in widths.enumerated() {
                out += "\n  \\begin{column}{\(escape(width(w)))}"
                out += "\n    " + (k == 0 ? "<<children>>" : "<<\(k)>>")
                out += "\n  \\end{column}"
            }
            return out + "\n\\end{columns}"
        }

        static func width(_ w: String) -> String {
            let t = w.trimmingCharacters(in: .whitespaces)
            if t.allSatisfy(\.isNumber), (1...2).contains(t.count) { return "0.\(t)\\textwidth" }
            if t.contains(where: \.isLetter) || t.contains("\\") { return t }
            return t + "\\textwidth"
        }
    }
}
