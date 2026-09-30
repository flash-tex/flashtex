import Foundation

extension TeXpand {
    /// Where an expansion happens (PLAN §3 `ctx`).
    public struct Context: Equatable, Sendable {
        public var scope: ScopeStack
        /// Packages the document already loads (`\usepackage`), which
        /// `requires` does not report and variants may test.
        public var packages: Set<String>
        public var documentClass: String?
        public var indentUnit: String
        /// The indentation of the line the expansion starts on.
        public var baseIndent: String
        /// The wrapped text in wrap mode (M7).
        public var selection: String?

        public init(scope: ScopeStack = .text(), packages: Set<String> = [], documentClass: String? = nil,
                    indentUnit: String = "  ", baseIndent: String = "", selection: String? = nil) {
            self.scope = scope; self.packages = packages; self.documentClass = documentClass
            self.indentUnit = indentUnit; self.baseIndent = baseIndent; self.selection = selection
        }
    }

    public struct Expansion: Equatable, Sendable {
        public var snippet: Snippet
        /// Packages the snippet needs that the document does not load yet.
        public var requires: [PackageRequirement]
        public var warnings: [String]

        public func rendered(_ ctx: Context = Context()) -> String {
            snippet.rendered(baseIndent: ctx.baseIndent, indentUnit: ctx.indentUnit)
        }
    }

    public struct ExpandError: Error, Equatable, Sendable, CustomStringConvertible {
        /// UTF-16 offset in the abbreviation of the element at fault.
        public var offset: Int
        public var message: String
        public var description: String { "at \(offset): \(message)" }
    }

    /// Either way an abbreviation can fail.
    public enum Failure: Error, Equatable, Sendable, CustomStringConvertible {
        case parse(ParseError)
        case expand(ExpandError)

        public var message: String {
            switch self {
            case .parse(let e): return e.message
            case .expand(let e): return e.message
            }
        }
        public var isIncomplete: Bool { if case .parse(let e) = self { return e.kind == .incomplete }; return false }
        public var description: String {
            switch self {
            case .parse(let e): return e.description
            case .expand(let e): return e.description
            }
        }
    }

    /// The headless engine: a registry plus the operations the host needs.
    public struct Engine: Sendable {
        public let registry: Registry
        public var settings: Settings { registry.settings }
        public var diagnostics: [Diagnostic] { registry.diagnostics }

        public init(registry: Registry) { self.registry = registry }
        /// The built-in catalog layered with `layers` (user, then project).
        public init(settings: Settings = Settings(), layers: [Layer] = []) {
            registry = .load(layers: layers, settings: settings)
        }

        public func parse(_ abbreviation: String, in ctx: Context = Context()) -> Result<Abbreviation, ParseError> {
            Parser.parse(abbreviation, oracle: registry.oracle(flags: ctx.scope.flags, documentClass: ctx.documentClass))
        }

        public func expand(_ abbreviation: String, in ctx: Context = Context()) -> Result<Expansion, Failure> {
            switch parse(abbreviation, in: ctx) {
            case .failure(let e): return .failure(.parse(e))
            case .success(let ast): return expand(ast, in: ctx).mapError { .expand($0) }
            }
        }

        public func expand(_ ast: Abbreviation, in ctx: Context = Context()) -> Result<Expansion, ExpandError> {
            var x = Expander(registry: registry, profile: registry.profile, ctx: ctx)
            do {
                let blocks = try x.items(ast.items, frame: .init(flags: ctx.scope.flags, counter: nil, depth: 0))
                let raw = Expander.join(blocks, separator: nil)
                return .success(Expansion(snippet: Expander.finalize(raw), requires: x.missingRequirements(), warnings: x.warnings))
            } catch let e as ExpandError {
                return .failure(e)
            } catch {
                return .failure(ExpandError(offset: 0, message: "\(error)"))
            }
        }

        /// `expand(abbr, ctx) -> String` (PLAN M2): the rendered snippet, or
        /// `error: …` — the golden tests' form.
        public func expandToString(_ abbreviation: String, in ctx: Context = Context()) -> String {
            switch expand(abbreviation, in: ctx) {
            case .success(let e): return e.rendered(ctx)
            case .failure(let f): return "error: " + f.message
            }
        }
    }
}

// MARK: - Expander

struct Expander {
    typealias T = TeXpand
    let registry: T.Registry
    let profile: T.Profile
    let ctx: T.Context
    var nextInstance = 0
    var requirements: [T.PackageRequirement] = []
    var warnings: [String] = []
    /// The flags at the expansion point (variants test `scope` against them).
    let rootFlags: Set<String>
    /// Default children parsed once per (abbreviation, flags).
    var defaultChildCache: [String: TeXpand.Abbreviation] = [:]

    init(registry: T.Registry, profile: T.Profile, ctx: T.Context) {
        self.registry = registry; self.profile = profile; self.ctx = ctx
        rootFlags = ctx.scope.flags
    }

    static let maxInstances = 2000
    static let maxDepth = 24

    struct Key: Hashable { var instance: Int; var name: String }

    indirect enum Raw {
        case text(String)
        case tab(Key, [Raw])
        case choice(Key, [String])
        case mirror(Key, T.MirrorTransform)
        case final
        case newline
        case indent([Raw])
    }

    struct Frame {
        var flags: Set<String>
        /// The innermost repeat's 1-based counter.
        var counter: Int?
        var depth: Int
    }

    func fail(_ offset: Int, _ message: String) -> T.ExpandError { T.ExpandError(offset: offset, message: message) }

    // MARK: sequences

    /// One block per produced element (a group contributes its members').
    mutating func items(_ items: [T.Item], frame: Frame) throws -> [[Raw]] {
        var out: [[Raw]] = []
        for item in items { out += try self.item(item, frame: frame) }
        return out
    }

    mutating func item(_ item: T.Item, frame: Frame) throws -> [[Raw]] {
        var count = 1
        switch item.repeatCount {
        case .count(let n)?: count = n
        case .perLine?:
            throw fail(item.offset, "a bare `*` repeats once per selected line; it works when wrapping a selection (M7)")
        case nil: break
        }
        var out: [[Raw]] = []
        for k in 1...count {
            var f = frame
            if item.repeatCount != nil { f.counter = k }
            switch item.body {
            case .element(let e): out.append(try element(e, children: item.children, frame: f))
            case .group(let inner): out += try items(inner, frame: f)
            }
        }
        return out
    }

    static func join(_ blocks: [[Raw]], separator: String?) -> [Raw] {
        joinChildren(blocks, separator: separator, rowBreaks: [])
    }

    /// Blocks one per line; `separator` ends every block but the last and
    /// the ones next to a row break.
    static func joinChildren(_ blocks: [[Raw]], separator: String?, rowBreaks: Set<Int>) -> [Raw] {
        var out: [Raw] = []
        for (k, b) in blocks.enumerated() {
            if k > 0 { out.append(.newline) }
            out += b
            if let separator, k < blocks.count - 1, !rowBreaks.contains(k), !rowBreaks.contains(k + 1) { out.append(.text(separator)) }
        }
        return out
    }

    // MARK: elements

    struct ParamSlot {
        var value: T.TypedParam?
        var fallback: T.TypedParam?
        var given: Bool
    }

    mutating func element(_ e: T.Element, children: [T.Item], frame: Frame) throws -> [Raw] {
        guard let def = registry.resolve(e.name, flags: frame.flags) else {
            if let other = registry.definitions[e.name]?.first {
                throw fail(e.offset, "`\(e.name)` is not available here (it works in: \(other.scopes.joined(separator: ", ")))")
            }
            throw fail(e.offset, "unknown abbreviation `\(e.name)`")
        }
        guard frame.depth < Self.maxDepth else { throw fail(e.offset, "`\(e.name)` nests too deeply (a default child that contains itself?)") }
        nextInstance += 1
        guard nextInstance <= Self.maxInstances else { throw fail(e.offset, "the expansion is too large (over \(Self.maxInstances) elements)") }
        let inst = nextInstance
        let sub = { (s: String) in Self.substituteCounter(s, frame.counter) }

        // Params.
        if e.shape != nil, def.shapeMode == .none { throw fail(e.offset, "`\(e.name)` takes no size") }
        var given = e.params.map(sub)
        if let shape = e.shape, def.shapeMode == .param {
            // `pmat3x3` fills the param named `shape` (M4's matrix generators).
            guard let k = def.params.firstIndex(where: { $0.name == "shape" }) else {
                throw fail(e.offset, "`\(e.name)` takes a size but declares no `shape` param")
            }
            while given.count < k { given.append("") }
            given.insert(shape.description, at: k)
        } else if def.shapeMode == .param, let k = def.params.firstIndex(where: { $0.name == "shape" }), k < given.count,
                  !given[k].isEmpty, (try? T.parseParam(given[k], as: .shape).get()) == nil {
            // No size in the name and the param there is not one (`int:0..1:x`,
            // `dd:y/x`): the size takes its default and the params move over.
            given.insert("", at: k)
        }
        if given.count > def.params.count {
            if let last = def.params.last, last.type == .raw, !def.params.isEmpty {
                let head = Array(given.prefix(def.params.count - 1))
                given = head + [given.dropFirst(def.params.count - 1).joined(separator: ":")]
            } else if def.params.isEmpty {
                throw fail(e.offset, "`\(e.name)` takes no parameters")
            } else {
                throw fail(e.offset, "`\(e.name)` takes at most \(def.params.count) parameter\(def.params.count == 1 ? "" : "s")")
            }
        }
        // Variant (after the params: `when.param` sees them as given, the size included).
        let variant = def.variants.first { matches($0.when, e, def, given: given) }
        requirements += def.requires + (variant?.requires ?? [])
        let generator = variant.map { $0.body == nil ? ($0.generator ?? def.generator) : $0.generator } ?? def.generator
        let body = variant?.body ?? (variant?.generator == nil ? def.body : nil)

        var params: [String: ParamSlot] = [:]
        for (k, p) in def.params.enumerated() {
            let fallback = p.defaultText.flatMap { try? T.parseParam($0, as: p.type).get() }
            if k < given.count, !given[k].isEmpty {
                switch T.parseParam(given[k], as: p.type) {
                case .success(let v): params[p.name] = ParamSlot(value: v, fallback: fallback, given: true)
                case .failure(let err): throw fail(e.offset, "`\(e.name)`, param `\(p.name)`: \(err.message)")
                }
            } else {
                params[p.name] = ParamSlot(value: nil, fallback: fallback, given: false)
            }
        }

        // Args.
        let args = e.args.map(sub)
        let templates = [body].compactMap { $0 } + e.modifiers.compactMap { def.modifiers[$0]?.body }
        let referenced = templates.flatMap(\.holes).compactMap { h -> Int? in
            if case .arg(let ref, _) = h { return def.argIndex(ref).map { $0 + 1 } }
            return nil
        }.max() ?? 0
        let arity = max(def.args.count, referenced, generator.map { T.Generators.arity($0) } ?? 0)
        if args.count > arity {
            throw fail(e.offset, arity == 0 ? "`\(e.name)` takes no `{…}` arguments" : "`\(e.name)` takes at most \(arity) `{…}` argument\(arity == 1 ? "" : "s")")
        }
        if e.opts.count > 1 { warnings.append("`\(e.name)`: only the first `[…]` is used") }
        if e.overlay != nil, !templates.contains(where: { $0.holes.contains(.overlay) }) {
            throw fail(e.offset, "`\(e.name)` takes no overlay `<…>`")
        }

        // Shape and children.
        var childBlocks: [[Raw]]?
        /// Indices in `childBlocks` of `row_break` lines (no separator around them).
        var rowBreaks = Set<Int>()
        let childFrame = Frame(flags: Self.childFlags(frame.flags, provides: def.provides), counter: frame.counter, depth: frame.depth + 1)
        if !children.isEmpty {
            guard def.acceptsChildren else { throw fail(e.offset, "`\(e.name)` cannot have children") }
            if e.shape != nil, def.shapeMode == .children { warnings.append("`\(e.name)`: the size is ignored when children are given") }
            childBlocks = try items(children, frame: childFrame)
        } else if let child = def.defaultChild, def.acceptsChildren {
            let cacheKey = child + "\u{1}" + childFrame.flags.sorted().joined(separator: ",")
            let ast: T.Abbreviation
            if let cached = defaultChildCache[cacheKey] {
                ast = cached
            } else {
                let oracle = registry.oracle(flags: childFrame.flags, documentClass: ctx.documentClass)
                guard case .success(let parsed) = T.Parser.parse(child, oracle: oracle) else {
                    throw fail(e.offset, "`\(e.name)`: its default child `\(child)` does not parse")
                }
                defaultChildCache[cacheKey] = parsed
                ast = parsed
            }
            if let shape = e.shape, def.shapeMode == .children {
                var blocks: [[Raw]] = []
                for k in 1...shape.count {
                    var f = childFrame
                    f.counter = k
                    blocks += try items(ast.items, frame: f)
                    if let cols = shape.cols, let rb = def.rowBreak, k % cols == 0, k < shape.count {
                        rowBreaks.insert(blocks.count)
                        blocks.append([.text(rb)])
                    }
                }
                childBlocks = blocks
            } else {
                childBlocks = try items(ast.items, frame: childFrame)
            }
        } else if e.shape != nil, def.shapeMode == .children {
            throw fail(e.offset, "`\(e.name)` has no default child to repeat")
        }

        // Modifiers.
        var mods: [T.Definition.Modifier] = []
        for m in e.modifiers {
            guard let mod = def.modifiers[m] else {
                let known = def.modifiers.keys.sorted()
                throw fail(e.offset, "`\(e.name)` has no modifier `.\(m)`" + (known.isEmpty ? "" : " (it has: \(known.map { "." + $0 }.joined(separator: ", ")))"))
            }
            mods.append(mod)
            requirements += mod.requires
        }

        // The template the label goes into: the outermost modifier with a
        // `<<label>>`, else the body.
        let labelOwner = mods.lastIndex { $0.body.hasLabel }
        let childNodes = childBlocks.map { Self.joinChildren($0, separator: def.childSeparator, rowBreaks: rowBreaks) }
        var inputs = Inputs(def: def, element: e, instance: inst, params: params, args: args, children: childNodes,
                            label: nil, wrapped: nil, counter: frame.counter)
        if e.label != nil {
            let bodyHasLabel = body?.hasLabel ?? (generator.map { T.Generators.placesLabel($0) } ?? false)
            guard labelOwner != nil || bodyHasLabel else { throw fail(e.offset, "`\(e.name)` has no place for a label") }
            let prefixKey = labelOwner.flatMap { mods[$0].labelPrefix } ?? def.labelPrefix
            inputs.label = labelNodes(e, def: def, body: body, prefixKey: prefixKey, inst: inst, sub: sub)
        }

        // Body.
        var nodes: [Raw]
        let bodyInputs: Inputs = { var i = inputs; if labelOwner != nil { i.label = nil }; return i }()
        if let generator {
            let call = T.Generators.Call(element: e, definition: def, params: params.mapValues { ($0.value ?? $0.fallback, $0.given) },
                                         args: args, options: def.generatorOptions, packages: ctx.packages, profile: profile)
            let source: String
            do { source = try T.Generators.template(generator, call) } catch let err as T.ExpandError { throw T.ExpandError(offset: e.offset, message: err.message) }
            let tmpl: T.Template
            do { tmpl = try T.Template(source) } catch { throw fail(e.offset, "`\(e.name)`: generator `\(generator)` produced a bad template") }
            nodes = try render(tmpl, bodyInputs)
        } else if let body {
            nodes = try render(body, bodyInputs)
        } else {
            throw fail(e.offset, "`\(e.name)` has nothing to expand to")
        }
        for (k, mod) in mods.enumerated() {
            var mi = inputs
            mi.wrapped = nodes
            mi.localPrefix = "m\(k)."  // a modifier's `<<1>>` is not the body's
            if k != labelOwner { mi.label = nil }
            nodes = try render(mod.body, mi)
        }
        return nodes
    }

    /// `\label{prefix:name}`; a bare `#` slugs the first argument (or
    /// mirrors its tabstop, slugged, while it is still to be typed).
    mutating func labelNodes(_ e: T.Element, def: T.Definition, body: T.Template?, prefixKey: String?, inst: Int, sub: (String) -> String) -> [Raw] {
        var prefix = ""
        if let key = prefixKey {
            prefix = (profile["label_prefix." + key] ?? key) + (profile["label_sep"] ?? ":")
        }
        var name: [Raw]
        switch e.label {
        case .named(let n)?:
            name = [.text(sub(n))]
        default:
            if let first = e.args.first.map(sub), !first.isEmpty {
                name = [.text(T.slug(first))]
            } else if body?.holes.contains(where: { if case .arg(let r, _) = $0 { return def.argIndex(r) == 0 }; return false }) == true {
                name = [.mirror(Key(instance: inst, name: "arg.0"), .slug)]
            } else {
                name = [.tab(Key(instance: inst, name: "label"), [])]
            }
        }
        return [.text("\\label{" + prefix)] + name + [.text("}")]
    }

    static func childFlags(_ flags: Set<String>, provides: [String]) -> Set<String> {
        var f = flags.union(provides)
        if provides.contains("math") { f.remove("text") }
        if provides.contains("text") { f.remove("math") }
        return f
    }

    /// `@` → the counter, `@0` → the counter from 0, `@@` → `@`; untouched
    /// outside a repeat.
    static func substituteCounter(_ s: String, _ counter: Int?) -> String {
        guard let counter, s.contains("@") else { return s }
        var out = ""
        var i = s.startIndex
        while i < s.endIndex {
            let c = s[i]
            let next = s.index(after: i)
            if c == "@" {
                if next < s.endIndex, s[next] == "@" { out += "@"; i = s.index(after: next); continue }
                if next < s.endIndex, s[next] == "0" { out += String(counter - 1); i = s.index(after: next); continue }
                out += String(counter)
            } else {
                out.append(c)
            }
            i = next
        }
        return out
    }

    func matches(_ c: T.Definition.Condition, _ e: T.Element, _ def: T.Definition, given params: [String]) -> Bool {
        if !c.packages.allSatisfy({ ctx.packages.contains($0) }) { return false }
        if !c.classes.isEmpty, !c.classes.contains(ctx.documentClass ?? "") { return false }
        if !c.scopes.isEmpty, !c.scopes.contains(where: { rootFlags.contains($0) }) { return false }
        for (k, v) in c.profile where profile[k] != v { return false }
        for (name, v) in c.params {
            guard let k = def.params.firstIndex(where: { $0.name == name }) else { return false }
            let given = k < params.count && !params[k].isEmpty ? params[k] : (def.params[k].defaultText ?? "")
            if given != v { return false }
        }
        if let star = c.star, star != e.star { return false }
        if !c.modifiers.allSatisfy({ e.modifiers.contains($0) }) { return false }
        return true
    }

    // MARK: templates

    struct Inputs {
        var def: T.Definition
        var element: T.Element
        var instance: Int
        var params: [String: ParamSlot]
        var args: [String]
        /// Expanded children, already one per line.
        var children: [Raw]?
        var label: [Raw]?
        var wrapped: [Raw]?
        var counter: Int?
        /// Namespace of the template's own `<<N>>` stops.
        var localPrefix = ""
    }

    mutating func render(_ t: T.Template, _ x: Inputs) throws -> [Raw] {
        var kept: [(level: Int, nodes: [Raw])] = []
        for line in t.lines {
            var nodes: [Raw] = []
            var holes = 0, emptyHoles = 0, blankText = true
            for part in line.parts {
                switch part {
                case .text(let s):
                    nodes.append(.text(s))
                    if !s.allSatisfy({ $0 == " " || $0 == "\t" }) { blankText = false }
                case .hole(let h):
                    let r = try hole(h, x)
                    holes += 1
                    if r.isEmpty { emptyHoles += 1 }
                    nodes += r
                }
            }
            if holes > 0, holes == emptyHoles, blankText { continue } // `<<label>>` with no label
            kept.append((line.level, nodes))
        }
        var out: [Raw] = []
        for (k, line) in kept.enumerated() {
            if k == 0 { out += line.nodes; continue }
            var block: [Raw] = [.newline] + line.nodes
            for _ in 0..<line.level { block = [.indent(block)] }
            out += block
        }
        return out
    }

    mutating func hole(_ h: T.Template.Hole, _ x: Inputs) throws -> [Raw] {
        let key = { (name: String) in Key(instance: x.instance, name: name) }
        switch h {
        case .tabstop(let n, let d): return [.tab(key(x.localPrefix + "t\(n)"), d.map { [.text($0)] } ?? [])]
        case .choice(let n, let options): return [.choice(key(x.localPrefix + "t\(n)"), options)]
        case .mirror(let n, let tr): return [.mirror(key(x.localPrefix + "t\(n)"), tr)]
        case .final: return [.final]
        case .children:
            if let c = x.children { return c }
            return x.def.childrenOptional ? [] : [.tab(key("children"), [])]
        case .selection:
            if let s = ctx.selection { return [.text(s)] }
            return [.tab(key("selection"), [])]
        case .body:
            return x.wrapped ?? []
        case .arg(let ref, let d):
            guard let idx = x.def.argIndex(ref) else { return [] }
            let decl = idx < x.def.args.count ? x.def.args[idx] : nil
            let pre = decl?.prefix ?? "", post = decl?.suffix ?? ""
            if idx < x.args.count {
                return [.text(pre + x.args[idx] + post)]
            }
            if decl?.optional == true { return [] }
            let placeholder = d ?? decl?.defaultText
            return (pre.isEmpty ? [] : [.text(pre)]) + [.tab(key("arg.\(idx)"), placeholder.map { [.text($0)] } ?? [])] + (post.isEmpty ? [] : [.text(post)])
        case .opt:
            return x.element.opts.first.map { [.text("[" + Self.substituteCounter($0, x.counter) + "]")] } ?? []
        case .optValue(let d):
            if let o = x.element.opts.first { return [.text(Self.substituteCounter(o, x.counter))] }
            return [.tab(key("opt"), d.map { [.text($0)] } ?? [])]
        case .overlay:
            return x.element.overlay.map { [.text("<" + $0 + ">")] } ?? []
        case .label:
            return x.label ?? []
        case .star:
            return x.element.star ? [.text("*")] : []
        case .param(let name, let field):
            guard let slot = x.params[name] else { return [] }
            if slot.given, let v = slot.value {
                if let s = v.field(field) { return [.text(s)] }
                // A field the given param leaves out (`sum:1..n` has no var):
                // the default's, as a tabstop.
            }
            let d = slot.fallback?.field(field)
            return [.tab(key("p.\(name).\(field)"), d.map { [.text($0)] } ?? [])]
        case .profile(let k):
            return [.text(profile[k] ?? "")]
        case .counter(let zeroBased):
            let c = x.counter ?? 1
            return [.text(String(zeroBased ? c - 1 : c))]
        }
    }

    // MARK: output

    /// Global tabstop numbering in document order (PLAN §5): the first
    /// occurrence of a key is its tabstop, later ones mirror it. Only the
    /// first `$0` survives.
    static func finalize(_ raw: [Raw]) -> T.Snippet {
        var order: [Key: Int] = [:]
        func number(_ ns: [Raw]) {
            for n in ns {
                switch n {
                case .tab(let k, let ph):
                    if order[k] == nil { order[k] = order.count + 1 }
                    number(ph)
                case .choice(let k, _):
                    if order[k] == nil { order[k] = order.count + 1 }
                case .indent(let inner): number(inner)
                default: break
                }
            }
        }
        number(raw)
        var emitted = Set<Key>()
        var finalSeen = false
        func convert(_ ns: [Raw]) -> [T.SnippetNode] {
            var out: [T.SnippetNode] = []
            for n in ns {
                switch n {
                case .text(let s):
                    if case .text(let prev)? = out.last { out[out.count - 1] = .text(prev + s) } else if !s.isEmpty { out.append(.text(s)) }
                case .tab(let k, let ph):
                    let i = order[k]!
                    if emitted.insert(k).inserted { out.append(.tabstop(index: i, placeholder: convert(ph))) } else { out.append(.mirror(index: i, transform: .none)) }
                case .choice(let k, let options):
                    let i = order[k]!
                    if emitted.insert(k).inserted { out.append(.choice(index: i, options: options)) } else { out.append(.mirror(index: i, transform: .none)) }
                case .mirror(let k, let tr):
                    if let i = order[k] { out.append(.mirror(index: i, transform: tr)) }
                case .final:
                    if !finalSeen { finalSeen = true; out.append(.final) }
                case .newline: out.append(.newline)
                case .indent(let inner): out.append(.indent(convert(inner)))
                }
            }
            return out
        }
        return T.Snippet(convert(raw))
    }

    func missingRequirements() -> [T.PackageRequirement] {
        var seen = Set<String>()
        return requirements.filter { !ctx.packages.contains($0.name) && seen.insert($0.name).inserted }
    }
}
