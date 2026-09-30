import Foundation

extension TeXpand.Catalog {
    /// The math, instant-atom, ligature and postfix packs (PLAN §14, M4–M5).
    static let mathPacks: [TeXpand.Layer] = [
        TeXpand.Layer(name: "built-in:math", source: math),
        TeXpand.Layer(name: "built-in:greek", source: greek),
        TeXpand.Layer(name: "built-in:ligatures", source: ligatures),
        TeXpand.Layer(name: "built-in:postfix", source: postfix),
    ]

    static let math = #"""
    [pack]
    name = "math"
    summary = "Math: big operators, integrals, derivatives, matrices, delimiters"

    [[abbr]]
    name = "sum"
    scope = ["math"]
    leaf = true
    params = [{ name = "r", type = "range", default = "i=1..n" }]
    description = "sum:i=1..n"
    body = '\sum_{<<p.r.var>>=<<p.r.lo>>}^{<<p.r.hi>>} <<0>>'

    [[abbr]]
    name = "prod"
    scope = ["math"]
    leaf = true
    params = [{ name = "r", type = "range", default = "i=1..n" }]
    description = "prod:i=1..n"
    body = '\prod_{<<p.r.var>>=<<p.r.lo>>}^{<<p.r.hi>>} <<0>>'

    [[abbr]]
    name = "bigcup"
    scope = ["math"]
    leaf = true
    params = [{ name = "r", type = "range", default = "i=1..n" }]
    description = "bigcup:i=1..n"
    body = '\bigcup_{<<p.r.var>>=<<p.r.lo>>}^{<<p.r.hi>>} <<0>>'

    [[abbr]]
    name = "bigcap"
    scope = ["math"]
    leaf = true
    params = [{ name = "r", type = "range", default = "i=1..n" }]
    description = "bigcap:i=1..n"
    body = '\bigcap_{<<p.r.var>>=<<p.r.lo>>}^{<<p.r.hi>>} <<0>>'

    [[abbr]]
    name = "int"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "integral"
    params = [{ name = "shape", type = "shape", default = "1" }, { name = "a", type = "raw" }, { name = "b", type = "raw" }]
    description = "int:a..b:x, int:D:x, int2:D:x,y, int3"

    [[abbr]]
    name = "oint"
    scope = ["math"]
    leaf = true
    params = [{ name = "c", type = "raw", default = "C" }, { name = "v", type = "raw", default = "s" }]
    description = "oint:C:s"
    body = '\oint_{<<p.c.value>>} <<1>> \,<<profile.diff_d>><<p.v.value>>'

    [[abbr]]
    name = "lim"
    scope = ["math"]
    leaf = true
    params = [{ name = "a", type = "arrow", default = "x->0" }]
    description = "lim:x->0"
    body = '\lim_{<<p.a.lhs>> \to <<p.a.rhs>>} <<0>>'

    [[abbr]]
    name = "dd"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "derivative"
    params = [{ name = "shape", type = "shape", default = "1" }, { name = "q", type = "ratio", default = "y/x" }]
    description = "dd:y/x, dd2:y/x (profile.diff_d; \\dv with physics)"

    [[abbr]]
    name = "pd"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "derivative"
    generator_opts = { partial = true }
    params = [{ name = "shape", type = "shape", default = "1" }, { name = "q", type = "ratio", default = "f/x" }]
    description = "pd:f/x, pd:f/x,y, pd2:f/x (\\pdv with physics)"

    [[abbr]]
    name = "seq"
    scope = ["math"]
    leaf = true
    generator = "sequence"
    params = [{ name = "s", type = "raw", default = "x_1..n" }]
    description = "seq:x_1..n, seq:x_1+..+n"

    [[abbr]]
    name = "set"
    scope = ["math"]
    leaf = true
    params = [{ name = "s", type = "pred", default = "x|P(x)" }]
    description = "set:x|x>0 (profile.set_sep)"
    body = '\{ <<p.s.lhs>> <<profile.set_sep>> <<p.s.rhs>> \}'

    [[abbr]]
    name = "map"
    scope = ["math"]
    leaf = true
    params = [{ name = "f", type = "raw", default = "f" }, { name = "a", type = "arrow", default = "A->B" }]
    description = "map:f:A->B, map:f:x|->x^2"
    body = '<<p.f.value>> \colon <<p.a.lhs>> <<p.a.cmd>> <<p.a.rhs>>'

    [[abbr]]
    name = "paren"
    scope = ["math"]
    leaf = true
    params = [{ name = "x", type = "raw" }]
    body = '\left( <<p.x.value>> \right)'

    [[abbr]]
    name = "brack"
    scope = ["math"]
    leaf = true
    params = [{ name = "x", type = "raw" }]
    body = '\left[ <<p.x.value>> \right]'

    [[abbr]]
    name = "brace"
    scope = ["math"]
    leaf = true
    params = [{ name = "x", type = "raw" }]
    body = '\left\{ <<p.x.value>> \right\}'

    [[abbr]]
    name = "norm"
    scope = ["math"]
    leaf = true
    params = [{ name = "x", type = "raw" }]
    body = '\left\| <<p.x.value>> \right\|'

    [[abbr]]
    name = "abs"
    scope = ["math"]
    leaf = true
    params = [{ name = "x", type = "raw" }]
    body = '\left| <<p.x.value>> \right|'

    [[abbr]]
    name = "floor"
    scope = ["math"]
    leaf = true
    params = [{ name = "x", type = "raw" }]
    body = '\left\lfloor <<p.x.value>> \right\rfloor'

    [[abbr]]
    name = "ceil"
    scope = ["math"]
    leaf = true
    params = [{ name = "x", type = "raw" }]
    body = '\left\lceil <<p.x.value>> \right\rceil'

    [[abbr]]
    name = "angle"
    scope = ["math"]
    leaf = true
    params = [{ name = "x", type = "raw" }]
    body = '\left\langle <<p.x.value>> \right\rangle'

    [[abbr]]
    name = "braket"
    scope = ["math"]
    leaf = true
    params = [{ name = "a", type = "raw" }, { name = "b", type = "raw" }]
    description = "braket:a:b (\\braket with physics)"
    body = '\left\langle <<p.a.value>> \middle| <<p.b.value>> \right\rangle'
    [[abbr.variant]]
    when = { package = "physics" }
    body = '\braket{<<p.a.value>>}{<<p.b.value>>}'

    [[abbr]]
    name = "mat"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "matrix"
    generator_opts = { env = "matrix" }
    requires = ["amsmath"]
    params = [{ name = "shape", type = "shape", default = "3x3" }, { name = "fill", type = "raw" }, { name = "arg", type = "raw" }]
    description = "matrix: mat2x3, fills :a :I :0 :diag:λ :aug, symbolic mat:mxn:a"

    [[abbr]]
    name = "pmat"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "matrix"
    generator_opts = { env = "pmatrix" }
    requires = ["amsmath"]
    params = [{ name = "shape", type = "shape", default = "3x3" }, { name = "fill", type = "raw" }, { name = "arg", type = "raw" }]
    description = "pmatrix: pmat3x3, pmat3:I, pmat:mxn:a"

    [[abbr]]
    name = "bmat"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "matrix"
    generator_opts = { env = "bmatrix" }
    requires = ["amsmath"]
    params = [{ name = "shape", type = "shape", default = "3x3" }, { name = "fill", type = "raw" }, { name = "arg", type = "raw" }]
    description = "bmatrix"

    [[abbr]]
    name = "vmat"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "matrix"
    generator_opts = { env = "vmatrix" }
    requires = ["amsmath"]
    params = [{ name = "shape", type = "shape", default = "3x3" }, { name = "fill", type = "raw" }, { name = "arg", type = "raw" }]
    description = "vmatrix (determinant)"

    [[abbr]]
    name = "Vmat"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "matrix"
    generator_opts = { env = "Vmatrix" }
    requires = ["amsmath"]
    params = [{ name = "shape", type = "shape", default = "3x3" }, { name = "fill", type = "raw" }, { name = "arg", type = "raw" }]
    description = "Vmatrix (norm)"

    [[abbr]]
    name = "vec"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "matrix"
    generator_opts = { env = "pmatrix", vector = "column" }
    requires = ["amsmath"]
    params = [{ name = "shape", type = "shape", default = "3" }, { name = "fill", type = "raw" }]
    description = "column vector: vec3:x"

    [[abbr]]
    name = "rvec"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "matrix"
    generator_opts = { env = "pmatrix", vector = "row" }
    requires = ["amsmath"]
    params = [{ name = "shape", type = "shape", default = "3" }, { name = "fill", type = "raw" }]
    description = "row vector: rvec3:x"

    [[abbr]]
    name = "rot"
    scope = ["math"]
    leaf = true
    shape = "param"
    generator = "rotation"
    requires = ["amsmath"]
    params = [{ name = "shape", type = "shape", default = "3" }, { name = "a", type = "raw" }, { name = "b", type = "raw" }]
    description = "rotation matrix: rot:z:\\theta, rot2:\\theta"

    [[abbr]]
    name = "cd"
    scope = ["math", "text"]
    leaf = true
    shape = "param"
    generator = "tikzcd"
    requires = ["tikz-cd"]
    params = [{ name = "shape", type = "shape", default = "2x2" }]
    description = "tikz-cd grid: cd:2x2"

    [[abbr]]
    name = "ses"
    scope = ["math"]
    leaf = true
    generator = "exact"
    params = [{ name = "items", type = "list", default = "A,B,C" }]
    description = "short exact sequence: ses:A,B,C"

    [[abbr]]
    name = "qty"
    scope = ["math", "text"]
    leaf = true
    requires = ["siunitx"]
    params = [{ name = "v", type = "raw" }, { name = "u", type = "raw" }]
    description = "siunitx quantity: qty:9.8:m/s^2 (units passed through)"
    body = '\qty{<<p.v.value>>}{<<p.u.value>>}'
    """#

    /// Instant atoms (§9.2): commit when the next typed character is not a
    /// letter, so `;a^2` is `\alpha^2` without Tab.
    static let greek: String = {
        let lower: [(String, String)] = [
            ("a", "alpha"), ("b", "beta"), ("g", "gamma"), ("d", "delta"), ("e", "epsilon"), ("ve", "varepsilon"),
            ("z", "zeta"), ("h", "eta"), ("th", "theta"), ("i", "iota"), ("k", "kappa"), ("l", "lambda"), ("m", "mu"),
            ("n", "nu"), ("x", "xi"), ("p", "pi"), ("r", "rho"), ("s", "sigma"), ("t", "tau"), ("u", "upsilon"),
            ("ph", "phi"), ("vph", "varphi"), ("ch", "chi"), ("ps", "psi"), ("w", "omega"),
            ("G", "Gamma"), ("D", "Delta"), ("Th", "Theta"), ("L", "Lambda"), ("X", "Xi"), ("P", "Pi"), ("S", "Sigma"),
            ("U", "Upsilon"), ("Ph", "Phi"), ("Ps", "Psi"), ("W", "Omega"),
            ("inf", "infty"), ("nab", "nabla"), ("par", "partial"), ("ell", "ell"),
        ]
        var out = "[pack]\nname = \"greek\"\nsummary = \"Instant atoms: Greek letters and \\\\infty \\\\nabla \\\\partial \\\\ell\"\n"
        for (abbr, cmd) in lower {
            out += "\n[[abbr]]\nname = \"\(abbr)\"\nscope = [\"math\"]\ninstant = true\nbody = '\\\(cmd)'\n"
        }
        return out
    }()

    static let ligatures = #"""
    [pack]
    name = "ligatures"
    summary = "Math ligatures: -> <= != ... xx x1"

    [[ligature]]
    trigger = "->"
    body = '\to'
    [[ligature]]
    trigger = "<-"
    body = '\gets'
    [[ligature]]
    trigger = "=>"
    body = '\implies'
    [[ligature]]
    trigger = "<=>"
    body = '\iff'
    [[ligature]]
    trigger = "|->"
    body = '\mapsto'
    [[ligature]]
    trigger = "!="
    body = '\neq'
    [[ligature]]
    trigger = "<="
    body = '\leq'
    [[ligature]]
    trigger = ">="
    body = '\geq'
    [[ligature]]
    trigger = "~="
    body = '\simeq'
    [[ligature]]
    trigger = "~~"
    body = '\approx'
    [[ligature]]
    trigger = "..."
    body = '\dots'
    [[ligature]]
    trigger = "xx"
    body = '\times'
    guard_before = '(?<![A-Za-z\\])'
    [[ligature]]
    trigger = "**"
    body = '\cdot'
    [[ligature]]
    trigger = "|-"
    body = '\vdash'
    [[ligature]]
    trigger = "|="
    body = '\models'
    [[ligature]]
    trigger = "[["
    body = '\llbracket'
    requires = ["stmaryrd"]
    [[ligature]]
    trigger = "]]"
    body = '\rrbracket'
    requires = ["stmaryrd"]
    [[ligature]]
    trigger = "<<"
    body = '\ll'
    [[ligature]]
    trigger = ">>"
    body = '\gg'
    [[ligature]]
    name = "x1"
    regex = '(?<![A-Za-z\\])([A-Za-z])(\d)$'
    body = '$1_$2'
    [[ligature]]
    name = "x12"
    regex = '(?<![A-Za-z\\])([A-Za-z])_(\d)(\d)$'
    body = '$1_{$2$3}'
    """#

    static let postfix = #"""
    [pack]
    name = "postfix"
    summary = "Postfix: x.hat, (a+b).sqrt, A.T, and the // fraction"

    [[postfix]]
    name = "hat"
    body = '\hat{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "bar"
    body = '\bar{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "dot"
    body = '\dot{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "ddot"
    body = '\ddot{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "vec"
    body = '<<profile.vector>>{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "tilde"
    body = '\tilde{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "bb"
    body = '\mathbb{<<atom>>}'
    strip_parens = true
    requires = ["amssymb"]
    [[postfix]]
    name = "cal"
    body = '\mathcal{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "frak"
    body = '\mathfrak{<<atom>>}'
    strip_parens = true
    requires = ["amssymb"]
    [[postfix]]
    name = "bf"
    body = '\mathbf{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "rm"
    body = '\mathrm{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "sf"
    body = '\mathsf{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "tt"
    body = '\mathtt{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "T"
    body = '<<atom>><<profile.transpose>>'
    [[postfix]]
    name = "inv"
    body = '<<atom>>^{-1}'
    [[postfix]]
    name = "conj"
    body = '\overline{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "sqrt"
    body = '\sqrt{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "lr"
    body = '\left(<<atom>>\right)'
    strip_parens = true
    [[postfix]]
    name = "box"
    body = '\boxed{<<atom>>}'
    strip_parens = true
    [[postfix]]
    name = "ub"
    body = '\underbrace{<<atom>>}_{<<1>>}'
    strip_parens = true
    [[postfix]]
    name = "ob"
    body = '\overbrace{<<atom>>}^{<<1>>}'
    strip_parens = true
    [[postfix]]
    name = "abs"
    body = '\left|<<atom>>\right|'
    strip_parens = true
    [[postfix]]
    name = "norm"
    body = '\left\|<<atom>>\right\|'
    strip_parens = true
    """#
}
