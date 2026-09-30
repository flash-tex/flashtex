import Foundation

extension TeXpand {
    /// `requires = ["pkg", { name = "cleveref", options = ["capitalize"] }]`
    public struct PackageRequirement: Equatable, Hashable, Sendable, CustomStringConvertible {
        public var name: String
        public var options: [String]
        public init(_ name: String, options: [String] = []) { self.name = name; self.options = options }
        /// The preamble line M6 inserts.
        public var description: String {
            options.isEmpty ? "\\usepackage{\(name)}" : "\\usepackage[\(options.joined(separator: ","))]{\(name)}"
        }
    }

    /// A config problem, surfaced in the config file it came from (PLAN §13):
    /// the bad definition is skipped, nothing else breaks.
    public struct Diagnostic: Equatable, Sendable, CustomStringConvertible {
        public enum Severity: String, Equatable, Sendable { case error, warning, note }
        public var severity: Severity
        /// The layer's name: `built-in:lists`, `user`, `project`, …
        public var layer: String
        public var line: Int?
        public var definition: String?
        public var message: String

        public var description: String {
            var s = "\(severity.rawValue): \(layer)"
            if let line { s += ":\(line)" }
            if let definition { s += " [\(definition)]" }
            return s + ": " + message
        }
    }

    /// One definition source. Built-in packs, the user's global file and
    /// the project's `texpand.toml` are all layers; later layers win.
    public struct Layer: Equatable, Sendable {
        public var name: String
        public var source: String
        public init(name: String, source: String) { self.name = name; self.source = source }
    }

    /// A tier A definition (PLAN §6.1).
    public struct Definition: Equatable, Sendable {
        public enum ShapeMode: String, Equatable, Sendable {
            /// `enum4` repeats `default_child` 4 times (`NxM`: N·M, with
            /// `row_break` between rows).
            case children
            /// The shape becomes the `shape` param (matrix generators, M4).
            case param
            /// A shape is an error.
            case none
        }

        public struct Param: Equatable, Sendable {
            public var name: String
            public var type: ParamType
            public var defaultText: String?
        }

        public struct Arg: Equatable, Sendable {
            public var name: String?
            public var defaultText: String?
            /// Absent → nothing at all (no tabstop), present → prefix+value+suffix.
            public var optional = false
            public var prefix = ""
            public var suffix = ""
        }

        /// `when` of a variant (PLAN §6.1): every key given must match.
        public struct Condition: Equatable, Sendable {
            public var packages: [String] = []
            public var classes: [String] = []
            public var scopes: [String] = []
            public var profile: [String: String] = [:]
            public var params: [String: String] = [:]
            public var star: Bool?
            public var modifiers: [String] = []
        }

        public struct Variant: Equatable, Sendable {
            public var when: Condition
            public var body: Template?
            public var generator: String?
            public var requires: [PackageRequirement] = []
        }

        /// `.float` and friends: a wrapper template with `<<body>>`.
        public struct Modifier: Equatable, Sendable {
            public var name: String
            public var body: Template
            public var requires: [PackageRequirement] = []
            public var labelPrefix: String?
        }

        public var name: String
        public var scopes: [String]
        public var leaf = false
        public var instant = false
        public var shapeMode = ShapeMode.none
        /// An abbreviation (`item`, `img+cap`) used when no children are given.
        public var defaultChild: String?
        /// No children and no `default_child`: drop the `<<children>>` line
        /// instead of leaving a tabstop there.
        public var childrenOptional = false
        /// Appended to every child but the last (`\\` between align rows).
        public var childSeparator: String?
        /// A line between the rows of an `NxM` shape.
        public var rowBreak: String?
        /// Scopes children are resolved in, on top of the element's own
        /// (`enum` provides `list`; `eq` provides `math`).
        public var provides: [String] = []
        /// Profile key suffix of the label prefix (`fig` → `label_prefix.fig`).
        public var labelPrefix: String?
        public var requires: [PackageRequirement] = []
        public var params: [Param] = []
        public var args: [Arg] = []
        public var body: Template?
        public var generator: String?
        public var generatorOptions = TOMLTable()
        public var variants: [Variant] = []
        public var modifiers: [String: Modifier] = [:]
        public var summary: String?
        public var pack: String
        public var layer: String
        public var line: Int

        /// Whether the element can take children (`<<children>>` somewhere,
        /// or a generator that places them).
        public var acceptsChildren: Bool { childrenSlot }
        /// Computed once at load (`acceptsChildren` is asked per keystroke).
        var childrenSlot = false

        func computeChildrenSlot() -> Bool {
            if leaf || instant { return false }
            if generator.map({ Generators.placesChildren($0) }) == true { return true }
            return ([body] + variants.map(\.body)).contains { $0?.hasChildren == true }
        }

        public func matches(_ flags: Set<String>) -> Bool { scopes.contains { flags.contains($0) } }

        /// Specific scopes beat general ones: `env:NAME` over `list`, `list`
        /// over `text`.
        func specificity(_ flags: Set<String>) -> Int {
            scopes.filter { flags.contains($0) }.map { $0.hasPrefix("env:") ? 3 : ["text", "math", "preamble"].contains($0) ? 1 : 2 }.max() ?? 0
        }

        func argIndex(_ ref: Template.ArgRef) -> Int? {
            switch ref {
            case .index(let n): return n - 1
            case .name(let s): return args.firstIndex { $0.name == s }
            }
        }
    }

    /// A pack: a named, separately switchable group of definitions.
    public struct Pack: Equatable, Sendable {
        public var name: String
        public var summary: String
        /// Off unless `packs = [...]` names it (the domain packs).
        public var optIn: Bool
        public var conflicts: [String]
    }

    /// Definitions indexed by name, after layering, `disable` and lint.
    public struct Registry: Sendable {
        public private(set) var definitions: [String: [Definition]] = [:]
        public private(set) var diagnostics: [Diagnostic] = []
        public private(set) var packs: [Pack] = []
        public private(set) var settings: Settings
        public private(set) var profile: Profile
        /// Tier B/C tables seen, loaded from M5 on.
        public private(set) var deferredTables: [String: Int] = [:]

        /// What the parser's oracle needs per definition, precomputed.
        struct OracleEntry: Sendable {
            var scopes: Set<String>
            var leaf: Bool
            var acceptsChildren: Bool
        }
        var oracleEntries: [String: [OracleEntry]] = [:]

        public var allDefinitions: [Definition] {
            definitions.keys.sorted().flatMap { definitions[$0]! }
        }

        /// The definition `name` resolves to under `flags`: among those
        /// whose scope matches, the most specific; ties go to the later layer.
        public func resolve(_ name: String, flags: Set<String>) -> Definition? {
            guard let defs = definitions[name] else { return nil }
            var best: Definition?
            var bestScore = 0
            for d in defs where d.matches(flags) {
                let score = d.specificity(flags)
                if score >= bestScore { best = d; bestScore = score }
            }
            return best
        }

        /// The parser's view of the registry under `flags`.
        public func oracle(flags: Set<String>, documentClass: String? = nil) -> Oracle {
            // Only the names the abbreviation mentions are looked up, and the
            // lookup is a dictionary read plus a scope test.
            let entries = oracleEntries
            let entry: @Sendable (String) -> OracleEntry? = { name in
                guard let list = entries[name] else { return nil }
                return list.first { !$0.scopes.isDisjoint(with: flags) } ?? list.last
            }
            return Oracle(
                isLeaf: { entry($0)?.leaf ?? false },
                acceptsChildren: { entry($0)?.acceptsChildren },
                // Rule 5, widened: in a beamer document an overlay is also
                // accepted outside a frame, because `frame>item*3<+->`
                // creates the frame its items live in.
                allowsOverlay: flags.contains("beamer") || documentClass == "beamer"
            )
        }

        /// Names valid under `flags` that start with `prefix` (the M3
        /// capture's prefix guard).
        public func names(withPrefix prefix: String, flags: Set<String>) -> [String] {
            definitions.filter { $0.key.hasPrefix(prefix) && $0.value.contains { $0.matches(flags) } }.map(\.key).sorted()
        }

        // MARK: loading

        /// Loads `layers` (lowest priority first) over the built-in catalog,
        /// with `settings` as the app-level base the files layer over.
        public static func load(layers: [Layer] = [], settings base: Settings = Settings(), builtIn: [Layer] = Catalog.packs) -> Registry {
            var r = Registry(settings: base, profile: Profile())
            var parsed: [(layer: Layer, root: TOMLTable, isBuiltIn: Bool)] = []
            for (k, layer) in (builtIn + layers).enumerated() {
                do {
                    parsed.append((layer, try parseTOML(layer.source), k < builtIn.count))
                } catch let e as TOMLError {
                    r.diagnostics.append(Diagnostic(severity: .error, layer: layer.name, line: e.line, definition: nil, message: e.message))
                } catch {
                    r.diagnostics.append(Diagnostic(severity: .error, layer: layer.name, line: nil, definition: nil, message: "\(error)"))
                }
            }

            // Pass 1: settings and profiles (they decide which packs load).
            var settings = base
            var namedProfiles = Profile.named
            var profileOverrides: [String: String] = [:]
            for (layer, root, _) in parsed {
                let report = { (sev: Diagnostic.Severity, line: Int?, msg: String) in
                    r.diagnostics.append(Diagnostic(severity: sev, layer: layer.name, line: line, definition: nil, message: msg))
                }
                if let t = root["settings"] {
                    if let t = t.table { settings.apply(t, report: report) } else { report(.error, nil, "`settings` is a table") }
                }
                // A file's top-level `disable` removes what lower layers
                // define (pass 2), so the same file can redefine the name;
                // `[settings] disable` removes a name everywhere.
                if let v = root["disable"], v.array?.allSatisfy({ $0.string != nil }) != true {
                    report(.error, nil, "`disable` is a list of names")
                }
                if let t = root["profiles"]?.table {
                    for (name, value) in t.entries {
                        guard let p = value.table else { report(.error, t.line, "profile `\(name)` is a table"); continue }
                        var values = namedProfiles[name] ?? [:]
                        for (k, v) in p.entries {
                            guard let s = v.string else { report(.error, p.line, "profile value `\(k)` is a string"); continue }
                            values[k] = s
                        }
                        namedProfiles[name] = values
                    }
                }
                if let t = root["profile"]?.table {
                    for (k, v) in t.entries {
                        guard let s = v.string else { report(.error, t.line, "profile value `\(k)` is a string"); continue }
                        profileOverrides[k] = s
                    }
                }
            }
            if namedProfiles[settings.profile] == nil {
                r.diagnostics.append(Diagnostic(severity: .warning, layer: "settings", line: nil, definition: nil,
                                                message: "unknown profile `\(settings.profile)`; using the defaults"))
            }
            var values = Profile.defaults
            for (k, v) in namedProfiles[settings.profile] ?? [:] { values[k] = v }
            for (k, v) in profileOverrides { values[k] = v }
            r.profile = Profile(values)
            r.settings = settings

            // Pass 2: packs and definitions, layer by layer.
            var disabledPacks = Set(settings.disabledPacks)
            var loadedPacks: [String] = []
            for (layer, root, isBuiltIn) in parsed {
                var packName = isBuiltIn ? layer.name : layer.name
                var packOn = true
                if let p = root["pack"]?.table {
                    let name = p["name"]?.string ?? layer.name
                    packName = name
                    let pack = Pack(name: name, summary: p["summary"]?.string ?? "",
                                    optIn: p["opt_in"]?.bool ?? false,
                                    conflicts: p["conflicts"]?.array?.compactMap(\.string) ?? [])
                    r.packs.append(pack)
                    packOn = pack.optIn ? settings.enabledPacks.contains(name) : !disabledPacks.contains(name)
                    if packOn, let clash = pack.conflicts.first(where: { loadedPacks.contains($0) }) {
                        r.diagnostics.append(Diagnostic(severity: .warning, layer: layer.name, line: p.line, definition: nil,
                                                        message: "pack `\(name)` conflicts with `\(clash)`, which is already on; `\(name)` is off"))
                        packOn = false
                    }
                    if packOn { loadedPacks.append(name) } else { disabledPacks.insert(name) }
                }
                guard packOn else { continue }
                // A layer's `disable` removes what lower layers defined.
                if let names = root["disable"]?.array?.compactMap(\.string) {
                    for n in names { r.definitions[n] = nil }
                }
                for (key, value) in root.entries {
                    switch key {
                    case "abbr":
                        guard let tables = value.array?.compactMap(\.table) else {
                            r.diagnostics.append(Diagnostic(severity: .error, layer: layer.name, line: nil, definition: nil, message: "`abbr` must be written [[abbr]]"))
                            continue
                        }
                        var seen: [String: [Set<String>]] = [:]
                        for t in tables {
                            var diags: [Diagnostic] = []
                            guard let d = Definition.load(t, pack: packName, layer: layer.name, diagnostics: &diags) else {
                                r.diagnostics += diags; continue
                            }
                            r.diagnostics += diags
                            let scopeSet = Set(d.scopes)
                            if seen[d.name, default: []].contains(where: { !$0.isDisjoint(with: scopeSet) }) {
                                r.diagnostics.append(Diagnostic(severity: .warning, layer: layer.name, line: t.line, definition: d.name,
                                                                message: "`\(d.name)` is defined twice in overlapping scopes; the later one wins"))
                            }
                            seen[d.name, default: []].append(scopeSet)
                            r.insert(d)
                        }
                    case "ligature", "postfix":
                        r.deferredTables[key, default: 0] += value.array?.count ?? 1
                    case "settings", "profile", "profiles", "disable", "pack":
                        break
                    default:
                        r.diagnostics.append(Diagnostic(severity: .warning, layer: layer.name, line: nil, definition: nil, message: "unknown table `\(key)`"))
                    }
                }
            }
            for (key, n) in r.deferredTables.sorted(by: { $0.key < $1.key }) {
                r.diagnostics.append(Diagnostic(severity: .note, layer: "registry", line: nil, definition: nil,
                                                message: "\(n) [[\(key)]] definition\(n == 1 ? "" : "s") read but not active yet (tier \(key == "ligature" ? "B" : "C") arrives in M5)"))
            }
            for name in settings.disabled { r.definitions[name] = nil }
            r.settings.disabledPacks = Array(disabledPacks).sorted()
            r.lint()
            r.oracleEntries = r.definitions.mapValues { defs in
                defs.map { OracleEntry(scopes: Set($0.scopes), leaf: $0.leaf, acceptsChildren: $0.acceptsChildren) }
            }
            return r
        }

        /// A higher layer replaces a definition with the same name and scope set.
        mutating func insert(_ d: Definition) {
            var list = definitions[d.name, default: []]
            list.removeAll { Set($0.scopes) == Set(d.scopes) }
            list.append(d)
            definitions[d.name] = list
        }

        /// Cross-definition checks (PLAN §16): instant-name collisions,
        /// unknown profile keys, default children that do not parse.
        mutating func lint() {
            for (name, defs) in definitions {
                for d in defs where d.instant {
                    if let clash = defs.first(where: { !$0.instant && !Set($0.scopes).isDisjoint(with: d.scopes) }) {
                        diagnostics.append(Diagnostic(severity: .error, layer: d.layer, line: d.line, definition: name,
                                                      message: "instant `\(name)` has the same name as an abbreviation in \(clash.layer) with an overlapping scope; the instant atom is dropped"))
                        definitions[name]?.removeAll { $0 == d }
                    }
                }
                if definitions[name]?.isEmpty == true { definitions[name] = nil }
            }
            let knownProfileKeys = Set(profile.values.keys)
            for d in allDefinitions {
                let templates = [d.body] + d.variants.map(\.body) + d.modifiers.values.map(\.body)
                for case .profile(let key)? in templates.flatMap({ ($0?.holes ?? []).map(Optional.some) }) where !knownProfileKeys.contains(key) {
                    diagnostics.append(Diagnostic(severity: .warning, layer: d.layer, line: d.line, definition: d.name,
                                                  message: "unknown profile key `\(key)` (renders empty)"))
                }
                if let child = d.defaultChild, case .failure(let e) = Parser.parse(child) {
                    diagnostics.append(Diagnostic(severity: .error, layer: d.layer, line: d.line, definition: d.name,
                                                  message: "`default_child` does not parse: \(e.message)"))
                }
            }
        }
    }
}

// MARK: - Definition loading

extension TeXpand.Definition {
    typealias T = TeXpand

    static let knownKeys: Set<String> = [
        "name", "scope", "leaf", "instant", "shape", "default_child", "children_optional", "child_separator",
        "row_break", "provides", "label_prefix", "requires", "params", "args", "body", "generator",
        "generator_opts", "variant", "modifier", "description",
    ]

    /// Reads and validates one `[[abbr]]` table; nil (with an error) when it
    /// cannot be used.
    static func load(_ t: T.TOMLTable, pack: String, layer: String, diagnostics: inout [T.Diagnostic]) -> T.Definition? {
        let name = t["name"]?.string ?? ""
        var failed = false
        func report(_ sev: T.Diagnostic.Severity, _ msg: String, line: Int? = nil) {
            diagnostics.append(T.Diagnostic(severity: sev, layer: layer, line: line ?? t.line, definition: name.isEmpty ? nil : name, message: msg))
            if sev == .error { failed = true }
        }
        guard !name.isEmpty else { report(.error, "a definition needs a `name`"); return nil }
        guard name.utf16.allSatisfy({ T.Parser.isLetter($0) }) else {
            report(.error, "`\(name)`: names are ASCII letters only (a size, `!` or `:` follows the name)"); return nil
        }
        for key in t.keys where !knownKeys.contains(key) { report(.warning, "unknown key `\(key)`") }

        var scopes = ["text"]
        if let s = t["scope"] {
            if let str = s.string { scopes = [str] } else if let a = s.array?.compactMap(\.string), !a.isEmpty { scopes = a } else { report(.error, "`scope` is a string or a list of strings") }
        }
        var d = T.Definition(name: name, scopes: scopes, pack: pack, layer: layer, line: t.line)
        d.leaf = t["leaf"]?.bool ?? false
        d.instant = t["instant"]?.bool ?? false
        d.summary = t["description"]?.string
        if let s = t["shape"] {
            guard let v = s.string.flatMap(ShapeMode.init(rawValue:)) else { report(.error, "`shape` is \"children\", \"param\" or \"none\""); return nil }
            d.shapeMode = v
        }
        d.defaultChild = t["default_child"]?.string
        d.childrenOptional = t["children_optional"]?.bool ?? false
        d.childSeparator = t["child_separator"]?.string
        d.rowBreak = t["row_break"]?.string
        d.provides = t["provides"]?.array?.compactMap(\.string) ?? []
        d.labelPrefix = t["label_prefix"]?.string
        d.requires = requirements(t["requires"], report: { report(.error, $0) })
        d.summary = t["description"]?.string

        // Params.
        for p in t["params"]?.array ?? [] {
            guard let pt = p.table, let pname = pt["name"]?.string else { report(.error, "each param is { name = \"…\", type = \"…\" }"); continue }
            let typeName = pt["type"]?.string ?? "raw"
            guard let type = T.ParamType(rawValue: typeName) else {
                report(.error, "param `\(pname)`: unknown type `\(typeName)` (\(T.ParamType.allCases.map(\.rawValue).joined(separator: ", ")))"); continue
            }
            let def = pt["default"]?.string
            if let def, !def.isEmpty, case .failure(let e) = T.parseParam(def, as: type) {
                report(.error, "param `\(pname)`: the default does not parse: \(e.message)")
            }
            d.params.append(Param(name: pname, type: type, defaultText: def))
        }
        // Args.
        for a in t["args"]?.array ?? [] {
            if let s = a.string { d.args.append(Arg(name: s)); continue }
            guard let at = a.table else { report(.error, "each arg is a name or { name = …, default = …, optional = … }"); continue }
            var arg = Arg(name: at["name"]?.string, defaultText: at["default"]?.string)
            arg.optional = at["optional"]?.bool ?? false
            arg.prefix = at["prefix"]?.string ?? ""
            arg.suffix = at["suffix"]?.string ?? ""
            d.args.append(arg)
        }

        func template(_ v: T.TOMLValue?, what: String, line: Int? = nil) -> T.Template? {
            guard let v else { return nil }
            guard let s = v.string else { report(.error, "\(what) is a string", line: line); return nil }
            do { return try T.Template(s) } catch let e as T.Template.TemplateError {
                report(.error, "\(what): \(e.message)", line: line); return nil
            } catch { report(.error, "\(what): \(error)", line: line); return nil }
        }
        d.body = template(t["body"], what: "`body`")
        d.generator = t["generator"]?.string
        if let g = d.generator, !T.Generators.known.contains(g) { report(.error, "unknown generator `\(g)`") }
        d.generatorOptions = t["generator_opts"]?.table ?? T.TOMLTable()
        if d.body == nil && d.generator == nil && !failed { report(.error, "a definition needs a `body` or a `generator`") }

        for v in t["variant"]?.array ?? [] {
            guard let vt = v.table else { report(.error, "`variant` must be written [[abbr.variant]]"); continue }
            var variant = Variant(when: condition(vt["when"]?.table, report: { report(.error, $0, line: vt.line) }))
            variant.body = template(vt["body"], what: "variant `body`", line: vt.line)
            variant.generator = vt["generator"]?.string
            variant.requires = requirements(vt["requires"], report: { report(.error, $0, line: vt.line) })
            d.variants.append(variant)
        }
        if let mods = t["modifier"]?.table {
            for (mname, value) in mods.entries {
                guard let mt = value.table, let body = template(mt["body"], what: "modifier `\(mname)` `body`", line: mt.line) else {
                    if value.table == nil { report(.error, "modifier `\(mname)` is a table with a `body`") }
                    continue
                }
                if !body.hasBody { report(.error, "modifier `\(mname)` has no `<<body>>` hole", line: mt.line) }
                d.modifiers[mname] = Modifier(name: mname, body: body,
                                              requires: requirements(mt["requires"], report: { report(.error, $0, line: mt.line) }),
                                              labelPrefix: mt["label_prefix"]?.string)
            }
        }

        // Template lint: holes must refer to declared things.
        let declared = Dictionary(d.params.map { ($0.name, $0.type) }, uniquingKeysWith: { a, _ in a })
        let templates: [T.Template] = ([d.body] + d.variants.map(\.body)).compactMap { $0 } + d.modifiers.values.map(\.body)
        for tmpl in templates {
            for h in tmpl.holes {
                switch h {
                case .param(let pname, let field):
                    guard let type = declared[pname] else { report(.error, "`<<p.\(pname).\(field)>>` names an undeclared param"); continue }
                    let head = String(field.split(separator: ".").first ?? "")
                    if !T.TypedParam.fieldNames(of: type).contains(head) {
                        report(.error, "`<<p.\(pname).\(field)>>`: a \(type.rawValue) param has no field `\(head)`")
                    }
                case .arg(.name(let a), _):
                    if !d.args.contains(where: { $0.name == a }) { report(.error, "`<<arg.\(a)>>` names an undeclared argument") }
                case .children where d.leaf:
                    report(.warning, "a leaf cannot have children; `<<children>>` stays empty")
                default: break
                }
            }
        }
        if d.instant {
            if !d.params.isEmpty || !d.args.isEmpty || d.body?.hasChildren == true || d.leaf {
                report(.error, "an instant atom takes no params, arguments or children")
            }
        }
        if d.defaultChild != nil && d.shapeMode == .none && d.body?.hasChildren != true && d.generator == nil {
            report(.warning, "`default_child` without a `<<children>>` hole is never used")
        }
        d.childrenSlot = d.computeChildrenSlot()
        return failed ? nil : d
    }

    static func requirements(_ v: T.TOMLValue?, report: (String) -> Void) -> [T.PackageRequirement] {
        guard let v else { return [] }
        guard let a = v.array else { report("`requires` is a list"); return [] }
        return a.compactMap { item in
            if let s = item.string { return T.PackageRequirement(s) }
            if let t = item.table, let n = t["name"]?.string {
                return T.PackageRequirement(n, options: t["options"]?.array?.compactMap(\.string) ?? [])
            }
            report("each requirement is a package name or { name = …, options = [...] }")
            return nil
        }
    }

    static func condition(_ t: T.TOMLTable?, report: (String) -> Void) -> Condition {
        var c = Condition()
        guard let t else { return c }
        func strings(_ v: T.TOMLValue) -> [String] { v.string.map { [$0] } ?? v.array?.compactMap(\.string) ?? [] }
        func map(_ v: T.TOMLValue) -> [String: String] {
            Dictionary((v.table?.entries ?? []).compactMap { e in e.value.string.map { (e.key, $0) } }, uniquingKeysWith: { a, _ in a })
        }
        for (key, value) in t.entries {
            switch key {
            case "package": c.packages = strings(value)
            case "class": c.classes = strings(value)
            case "scope": c.scopes = strings(value)
            case "profile": c.profile = map(value)
            case "param": c.params = map(value)
            case "star": c.star = value.bool
            case "modifier": c.modifiers = strings(value)
            default: report("unknown `when` key `\(key)` (package, class, scope, profile, param, star, modifier)")
            }
        }
        return c
    }
}
