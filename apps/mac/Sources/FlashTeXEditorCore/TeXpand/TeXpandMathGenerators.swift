import Foundation

// The math generators (PLAN M4, §14 "Tier A, math"). Each returns a
// template: literal text for what was given, holes (tabstops with the
// declared default as placeholder, mirrors) for what was not.

extension TeXpand.Generators {
    typealias T = TeXpand

    static func fail(_ c: Call, _ m: String) -> T.ExpandError { T.ExpandError(offset: c.element.offset, message: m) }

    /// The given value of param `name`, or nil (not given).
    static func given(_ c: Call, _ name: String) -> T.TypedParam? {
        guard let p = c.params[name], p.given else { return nil }
        return p.value
    }

    /// The value of param `name`: given, else its default.
    static func value(_ c: Call, _ name: String) -> T.TypedParam? { c.params[name]?.value }

    /// `rows x cols` from the `shape` param (given, else default), or nil.
    static func dims(_ c: Call) -> (rows: String, cols: String, symbolic: Bool)? {
        guard let s = value(c, "shape"), let r = s.field("rows"), let k = s.field("cols") else { return nil }
        return (r, k, s.field("symbolic") == "true")
    }

    static func sub(_ s: String) -> String { (s as NSString).length == 1 ? "_" + s : "_{" + s + "}" }

    // MARK: matrix

    /// `pmat3x3` (tabstops), `:a` (entries a_{ij}), `:I`, `:0`, `:diag:λ`,
    /// `:aug` (an augmented matrix: a bar before the last column), and
    /// symbolic `pmat:mxn:a` with dots. `vector = "column" | "row"` (the
    /// `vec`/`rvec` abbreviations): `vec3:x` is x_1…x_3.
    static func matrix(_ c: Call) throws -> String {
        let env = c.options["env"]?.string ?? "pmatrix"
        let fill = given(c, "fill")?.field("value")
        let arg = given(c, "arg")?.field("value")
        guard let d = dims(c) else { throw fail(c, "`\(c.element.name)` needs a size such as 3x3") }
        var next = 0
        func stop() -> String { next += 1; return "<<\(next)>>" }
        var rows: [[String]] = []

        if let vector = c.options["vector"]?.string {
            guard let n = Int(d.rows), n >= 1, n <= 50 else { throw fail(c, "a vector has 1 to 50 entries") }
            let entries = (1...n).map { i in fill.map { escape($0) + sub(String(i)) } ?? stop() }
            rows = vector == "row" ? [entries] : entries.map { [$0] }
        } else if d.symbolic {
            let m = escape(d.rows), n = escape(d.cols), a = escape(fill ?? "a")
            let comma = (m as NSString).length > 1 || (n as NSString).length > 1
            func e(_ i: String, _ j: String) -> String { a + "_{" + i + (comma ? "," : "") + j + "}" }
            let dots = c.profile["matrix_dots"] == "none" ? ("\\dots", "\\vdots", "\\ddots") : ("\\cdots", "\\vdots", "\\ddots")
            rows = [
                [e("1", "1"), e("1", "2"), dots.0, e("1", n)],
                [e("2", "1"), e("2", "2"), dots.0, e("2", n)],
                [dots.1, dots.1, dots.2, dots.1],
                [e(m, "1"), e(m, "2"), dots.0, e(m, n)],
            ]
        } else {
            guard let r = Int(d.rows), let k = Int(d.cols), r >= 1, k >= 1, r * k <= 400 else {
                throw fail(c, "a matrix has 1 to 400 entries")
            }
            let comma = r > 9 || k > 9
            for i in 1...r {
                var row: [String] = []
                for j in 1...k {
                    switch fill {
                    case nil, "aug"?: row.append(stop())
                    case "I"?:
                        guard r == k else { throw fail(c, "an identity matrix is square") }
                        row.append(i == j ? "1" : "0")
                    case "0"?: row.append("0")
                    case "diag"?:
                        guard r == k else { throw fail(c, "a diagonal matrix is square") }
                        row.append(i == j ? (arg.map { escape($0) + sub(String(i)) } ?? stop()) : "0")
                    case let s?:
                        row.append(escape(s) + "_{\(i)\(comma ? "," : "")\(j)}")
                    }
                }
                rows.append(row)
            }
        }
        let body = T.Grid(rows: rows.map { .cells($0) }).lines().map { "\n  " + $0 }.joined()
        if fill == "aug", let k = Int(d.cols), k >= 2 {
            let (open, close) = delimiters[env] ?? (".", ".")
            let spec = String(repeating: "c", count: k - 1) + "|c"
            return "\\left\(open)\\begin{array}{\(spec)}" + body + "\n\\end{array}\\right\(close)"
        }
        return "\\begin{\(env)}" + body + "\n\\end{\(env)}"
    }

    static let delimiters: [String: (String, String)] = [
        "pmatrix": ("(", ")"), "bmatrix": ("[", "]"), "Bmatrix": ("\\{", "\\}"),
        "vmatrix": ("|", "|"), "Vmatrix": ("\\|", "\\|"), "matrix": (".", "."),
    ]

    // MARK: sequence

    static let joiners = ["\\cdot", "\\times", "\\otimes", "\\oplus", "\\cup", "\\cap", "\\wedge", "\\vee",
                          "\\leq", "\\geq", "\\le", "\\ge", "\\subseteq", "\\subset", "+", "-", "=", "<", ">", ","]

    /// `seq:x_1..n` → `x_1, x_2, \ldots, x_n`; `seq:x_1+..+n` → `x_1 + x_2
    /// + \cdots + x_n`: any joiner written on both sides of `..`.
    static func sequence(_ c: Call) throws -> String {
        let text = value(c, "s")?.field("value") ?? "x_1..n"
        guard let dots = T.topLevel(text, find: "..") else { throw fail(c, "a sequence is `x_1..n` or `x_1+..+n`") }
        let ws = CharacterSet.whitespaces
        var left = String(text[..<dots]).trimmingCharacters(in: ws), right = String(text[text.index(dots, offsetBy: 2)...]).trimmingCharacters(in: ws)
        var op = ","
        if let j = joiners.first(where: { left.hasSuffix($0) && right.hasPrefix($0) }) {
            op = j
            left = String(left.dropLast(j.count)).trimmingCharacters(in: ws)
            right = String(right.dropFirst(j.count)).trimmingCharacters(in: ws)
        }
        guard !left.isEmpty, !right.isEmpty else { throw fail(c, "a sequence needs a first and a last term") }
        let isGiven = given(c, "s") != nil
        var base: String, firstIndex: String?
        if let us = left.lastIndex(of: "_") {
            base = String(left[..<us])
            firstIndex = String(left[left.index(after: us)...]).trimmingCharacters(in: CharacterSet(charactersIn: "{}"))
        } else {
            base = left
        }
        let ell = op == "," ? "\\ldots" : "\\cdots"
        let join = op == "," ? ", " : " \(op) "
        guard let first = firstIndex else {
            return [escape(left), ell, escape(right)].joined(separator: join)
        }
        let second = Int(first).map { String($0 + 1) } ?? first + "+1"
        let last = right.contains("_") ? right : nil
        if isGiven {
            let b = escape(base)
            return [b + sub(first), b + sub(second), ell, last.map(escape) ?? b + sub(escape(right))].joined(separator: join)
        }
        // Not given: the base and the last index are fields; the base mirrors.
        return ["<<1:\(base)>>" + sub(first), "<<=1>>" + sub(second), ell, "<<=1>>_{<<2:\(right)>>}"].joined(separator: join)
    }

    // MARK: rotation

    /// `rot2:θ` (2-D), `rot:z:θ` (3-D about x, y or z).
    static func rotation(_ c: Call) throws -> String {
        let env = c.options["env"]?.string ?? "pmatrix"
        let n = Int(dims(c)?.rows ?? "3") ?? 3
        let a = given(c, "a")?.field("value"), b = given(c, "b")?.field("value")
        var axis = "z", angle: String?
        if n == 2 {
            angle = a
        } else if n == 3 {
            if let a, ["x", "y", "z"].contains(a) { axis = a; angle = b } else { angle = a }
        } else {
            throw fail(c, "a rotation is 2-D (rot2) or 3-D (rot:x|y|z)")
        }
        var first = true
        func theta() -> String {
            if let angle { return escape(angle) }
            defer { first = false }
            return first ? "<<1:\\theta>>" : "<<=1>>"
        }
        func cos() -> String { "\\cos " + theta() }
        func sin() -> String { "\\sin " + theta() }
        let rows: [[String]]
        if n == 2 {
            rows = [[cos(), "-" + sin()], [sin(), cos()]]
        } else {
            switch axis {
            case "x": rows = [["1", "0", "0"], ["0", cos(), "-" + sin()], ["0", sin(), cos()]]
            case "y": rows = [[cos(), "0", sin()], ["0", "1", "0"], ["-" + sin(), "0", cos()]]
            default: rows = [[cos(), "-" + sin(), "0"], [sin(), cos(), "0"], ["0", "0", "1"]]
            }
        }
        let body = T.Grid(rows: rows.map { .cells($0) }).lines().map { "\n  " + $0 }.joined()
        return "\\begin{\(env)}" + body + "\n\\end{\(env)}"
    }

    // MARK: integral

    /// `int:a..b:x` (a definite integral), `int:D:x` (over a domain),
    /// `int2:D:x,y`, `int3:V:x,y,z` (\iint, \iiint).
    static func integral(_ c: Call) throws -> String {
        let n = Int(dims(c)?.rows ?? "1") ?? 1
        guard (1...3).contains(n) else { throw fail(c, "integrals go up to int3") }
        let cmd = ["\\int", "\\iint", "\\iiint"][n - 1]
        let a = given(c, "a")?.field("value"), b = given(c, "b")?.field("value")
        var next = 0
        func stop(_ d: String) -> String { next += 1; return "<<\(next):\(d)>>" }
        var head = cmd
        if n == 1, let a, let dots = T.topLevel(a, find: "..") {
            head += "_{" + escape(String(a[..<dots])) + "}^{" + escape(String(a[a.index(dots, offsetBy: 2)...])) + "}"
        } else if let a {
            head += "_{" + escape(a) + "}"
        } else if n == 1 {
            head += "_{" + stop("a") + "}^{" + stop("b") + "}"
        } else {
            head += "_{" + stop("D") + "}"
        }
        next += 1
        let integrand = "<<\(next)>>"
        let defaults = ["x", "y", "z"]
        let vars = b.map { T.splitTopLevel($0, on: ",").map { escape($0.trimmingCharacters(in: .whitespaces)) } }
            ?? (0..<n).map { stop(defaults[$0]) }
        let differentials = vars.map { "\\,<<profile.diff_d>>" + $0 }.joined()
        return head + " " + integrand + " " + differentials
    }

    // MARK: derivatives

    /// `dd:y/x`, `dd2:y/x` (order 2), `pd:f/x`, `pd:f/x,y` (mixed); the
    /// physics package's `\dv`/`\pdv` when the document loads it.
    static func derivative(_ c: Call) throws -> String {
        let partial = c.options["partial"]?.bool ?? false
        let order = Int(dims(c)?.rows ?? "1") ?? 1
        guard order >= 1, order <= 9 else { throw fail(c, "a derivative's order is 1 to 9") }
        let q = value(c, "q")
        let dens = q?.lists["dens"]?.count ?? 1
        let num = "<<p.q.num>>"
        let den = { (i: Int) in "<<p.q.dens.\(i)>>" }
        if c.packages.contains("physics") {
            let cmd = partial ? "\\pdv" : "\\dv"
            if dens > 1 { return cmd + "{" + num + "}" + (0..<dens).map { "{" + den($0) + "}" }.joined() }
            return cmd + (order > 1 ? "[\(order)]" : "") + "{" + num + "}{" + den(0) + "}"
        }
        let d = partial ? "\\partial" : (c.profile["diff_d"] ?? "d")
        func spaced(_ s: String) -> String { s.range(of: #"\\[A-Za-z]+$"#, options: .regularExpression) != nil ? s + " " : s }
        let total = dens > 1 ? dens : order
        let top = spaced(d + (total > 1 ? "^{\(total)}" : "")) + num
        let bottom: String
        if dens > 1 {
            bottom = (0..<dens).map { spaced(d) + den($0) }.joined(separator: " ")
        } else {
            bottom = spaced(d) + den(0) + (order > 1 ? "^{\(order)}" : "")
        }
        return (c.profile["frac"] ?? "\\frac") + "{" + top + "}{" + bottom + "}"
    }

    // MARK: tikz-cd and exact sequences

    /// `cd:2x2`: a commutative diagram grid, arrows right and down.
    static func tikzcd(_ c: Call) throws -> String {
        guard let d = dims(c), let r = Int(d.rows), let k = Int(d.cols), r >= 1, k >= 1, r * k <= 64 else {
            throw fail(c, "a diagram is 1x1 to 8x8")
        }
        var next = 0
        var rows: [[String]] = []
        for i in 0..<r {
            rows.append((0..<k).map { j in
                next += 1
                var cell = "<<\(next)>>"
                if j < k - 1 { cell += " \\arrow[r]" }
                if i < r - 1 { cell += " \\arrow[d]" }
                return cell
            })
        }
        return "\\begin{tikzcd}" + T.Grid(rows: rows.map { .cells($0) }).lines().map { "\n  " + $0 }.joined() + "\n\\end{tikzcd}"
    }

    /// `ses:A,B,C` → `0 \to A \to B \to C \to 0`.
    static func exact(_ c: Call) -> String {
        let items = given(c, "items")?.lists["items"]?.map { escape($0.trimmingCharacters(in: .whitespaces)) }
            ?? ["<<1:A>>", "<<2:B>>", "<<3:C>>"]
        return (["0"] + items + ["0"]).joined(separator: " \\to ")
    }
}
