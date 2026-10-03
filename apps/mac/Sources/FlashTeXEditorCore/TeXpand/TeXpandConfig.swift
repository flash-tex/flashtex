import Foundation

extension TeXpand {
    /// The six config layers (PLAN §13), lowest priority first:
    /// 1. definitions synthesized from the project's macros (M9),
    /// 2. the built-in catalog,
    /// 3. domain packs (pack files; opt-in packs need `packs = [...]`),
    /// 4. the user's global `texpand.toml`,
    /// 5. the project's `texpand.toml`,
    /// 6. the document's `% !texpand` magic comments.
    /// A higher layer replaces a definition with the same name and scopes,
    /// `disable` removes lower ones, profile keys merge key-wise and
    /// settings layer over the app's.
    public struct Config: Equatable, Sendable {
        public var synthesized: [Layer] = []
        public var packs: [Layer] = []
        public var user: Layer?
        public var project: Layer?
        public var magic: Layer?

        public init(synthesized: [Layer] = [], packs: [Layer] = [], user: Layer? = nil, project: Layer? = nil, magic: Layer? = nil) {
            self.synthesized = synthesized; self.packs = packs; self.user = user; self.project = project; self.magic = magic
        }

        /// Layers 3–6 in order (1 and 2 are passed to `Registry.load` apart).
        public var upperLayers: [Layer] { packs + [user, project, magic].compactMap { $0 } }

        public func registry(settings: Settings) -> Registry {
            Registry.load(layers: upperLayers, settings: settings, builtIn: synthesized + Catalog.packs)
        }
    }

    /// `% !texpand key=value …` lines in a document's first lines (PLAN §13
    /// layer 6), as a config layer. Keys: `profile`, `leader`, `disable`
    /// (comma list), `packs`, `fraction_operator`, `fraction_trigger`,
    /// `auto_preamble`, and the kinds as `on`/`off` (`ligatures=on`). Like a
    /// file, a magic comment can switch TeXpand off but never on.
    public static func magicComments(in text: String, lines: Int = 30) -> Layer? {
        var settings: [String] = []
        var disable: [String] = []
        for raw in text.split(separator: "\n", maxSplits: lines, omittingEmptySubsequences: false).prefix(lines) {
            let line = raw.trimmingCharacters(in: .whitespaces)
            guard line.hasPrefix("%") else { continue }
            let body = line.drop { $0 == "%" }.trimmingCharacters(in: .whitespaces)
            guard body.lowercased().hasPrefix("!texpand") else { continue }
            for token in body.dropFirst("!texpand".count).split(whereSeparator: \.isWhitespace) {
                guard let eq = token.firstIndex(of: "=") else { continue }
                let key = String(token[..<eq]).lowercased()
                let value = String(token[token.index(after: eq)...])
                switch key {
                case "disable":
                    disable += value.split(separator: ",").map(String.init)
                case "packs", "disabled_packs":
                    settings.append("\(key) = " + TOMLValue.array(value.split(separator: ",").map { .string(String($0)) }).tomlDescription)
                case "profile", "leader", "fraction_operator", "fraction_trigger", "auto_preamble":
                    settings.append("\(key) = " + TOMLValue.quote(value))
                case "enabled", "abbreviations", "instant_atoms", "ligatures", "postfix", "structure_editor":
                    let on = ["on", "true", "yes", "1"].contains(value.lowercased())
                    settings.append("\(key) = \(on)")
                default:
                    settings.append("\(key) = " + TOMLValue.quote(value)) // reported as an unknown setting
                }
            }
        }
        guard !settings.isEmpty || !disable.isEmpty else { return nil }
        var source = ""
        if !disable.isEmpty { source += "disable = " + TOMLValue.array(disable.map { .string($0) }).tomlDescription + "\n" }
        if !settings.isEmpty { source += "[settings]\n" + settings.joined(separator: "\n") + "\n" }
        return Layer(name: "% !texpand", source: source)
    }
}
