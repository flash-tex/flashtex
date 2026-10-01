import Foundation

extension TeXpand {
    /// Every switch the owner asked for (PLAN "Adaptations"): a master
    /// enable (off until reviewed), per-tier enables, the leader, the
    /// profile, the fraction trigger, auto-preamble, per-pack and
    /// per-definition disables. Codable so the Mac and iPad can persist the
    /// app-level copy; `texpand.toml` files layer `[settings]` over it.
    public struct Settings: Codable, Equatable, Sendable {
        public enum AutoPreamble: String, Codable, CaseIterable, Sendable { case insert, prompt, off }
        public enum FractionTrigger: String, Codable, CaseIterable, Sendable { case tab, auto, off }
        public enum Tier: String, CaseIterable, Sendable {
            /// A: leader abbreviations (`;enum3` Tab).
            case abbreviations
            /// Instant atoms (`;a` → `\alpha` without Tab).
            case instantAtoms
            /// B: ligatures (`->` → `\to`), automatic on keystroke.
            case ligatures
            /// C: postfix (`x.hat` Tab, `(x+1)/2` Tab).
            case postfix
            /// Context structure editor (M10b).
            case structureEditor
        }

        /// Master switch. Off by default until the feature has been reviewed;
        /// nothing expands while it is off, whatever the tiers say.
        public var enabled = false
        public var abbreviations = true
        public var instantAtoms = true
        /// Off even when the master is on: ligatures fire without Tab.
        public var ligatures = false
        public var postfix = true
        public var structureEditor = true
        public var autoPreamble = AutoPreamble.insert
        /// One character, or empty for bare Tab-triggered abbreviations.
        public var leader = ";"
        public var profile = "default"
        public var fractionTrigger = FractionTrigger.tab
        /// Tier C's fraction operator (owner decision): `//`, so a single `/`
        /// never expands (`a/b` is often a deliberate inline fraction).
        /// `"/"` restores PLAN §9.4's original single-slash form;
        /// `fraction_trigger = "off"` turns the fraction form off.
        public var fractionOperator = "//"
        /// Built-in packs switched off (`floats`, `beamer`, …).
        public var disabledPacks: [String] = []
        /// Opt-in packs (the domain packs, M11) switched on.
        public var enabledPacks: [String] = []
        /// Definitions switched off by name (`sec`, `ref`, …).
        public var disabled: [String] = []

        public init() {}

        /// Whether `tier` is live: the master switch and its own.
        public func isActive(_ tier: Tier) -> Bool {
            guard enabled else { return false }
            switch tier {
            case .abbreviations: return abbreviations
            case .instantAtoms: return abbreviations && instantAtoms
            case .ligatures: return ligatures
            case .postfix: return postfix
            case .structureEditor: return structureEditor
            }
        }

        enum CodingKeys: String, CodingKey {
            case enabled, abbreviations, instantAtoms = "instant_atoms", ligatures, postfix
            case structureEditor = "structure_editor", autoPreamble = "auto_preamble", leader, profile
            case fractionTrigger = "fraction_trigger", fractionOperator = "fraction_operator", disabledPacks = "disabled_packs", enabledPacks = "packs", disabled = "disable"
        }

        /// Missing keys keep their defaults, so a copy persisted by an older
        /// build still decodes.
        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            let d = Settings()
            enabled = try c.decodeIfPresent(Bool.self, forKey: .enabled) ?? d.enabled
            abbreviations = try c.decodeIfPresent(Bool.self, forKey: .abbreviations) ?? d.abbreviations
            instantAtoms = try c.decodeIfPresent(Bool.self, forKey: .instantAtoms) ?? d.instantAtoms
            ligatures = try c.decodeIfPresent(Bool.self, forKey: .ligatures) ?? d.ligatures
            postfix = try c.decodeIfPresent(Bool.self, forKey: .postfix) ?? d.postfix
            structureEditor = try c.decodeIfPresent(Bool.self, forKey: .structureEditor) ?? d.structureEditor
            autoPreamble = (try? c.decodeIfPresent(AutoPreamble.self, forKey: .autoPreamble)) ?? d.autoPreamble
            leader = try c.decodeIfPresent(String.self, forKey: .leader) ?? d.leader
            profile = try c.decodeIfPresent(String.self, forKey: .profile) ?? d.profile
            fractionTrigger = (try? c.decodeIfPresent(FractionTrigger.self, forKey: .fractionTrigger)) ?? d.fractionTrigger
            let op = try c.decodeIfPresent(String.self, forKey: .fractionOperator) ?? d.fractionOperator
            fractionOperator = Self.fractionOperators.contains(op) ? op : d.fractionOperator
            disabledPacks = try c.decodeIfPresent([String].self, forKey: .disabledPacks) ?? d.disabledPacks
            enabledPacks = try c.decodeIfPresent([String].self, forKey: .enabledPacks) ?? d.enabledPacks
            disabled = try c.decodeIfPresent([String].self, forKey: .disabled) ?? d.disabled
        }

        /// The accepted `fraction_operator` values.
        public static let fractionOperators: [String] = ["//", "/"]

        /// Why a leader is refused, or nil when it is usable. A letter or
        /// digit would start an abbreviation name; `\` shares the namespace
        /// of real commands (PLAN §2).
        public static func leaderProblem(_ leader: String) -> String? {
            guard !leader.isEmpty else { return nil }
            guard leader.count == 1, let c = leader.first else { return "the leader is one character (or empty)" }
            if c.isLetter || c.isNumber { return "a letter or digit cannot be the leader" }
            if c.isWhitespace { return "whitespace cannot be the leader" }
            if c == "\\" { return "`\\` cannot be the leader: it shares a namespace with real commands" }
            return nil
        }

        /// Layers a `[settings]` table over these settings. `enabled` in a
        /// file can only switch the feature off: the master switch belongs to
        /// the person using the app, not to a project someone else wrote.
        mutating func apply(_ t: TOMLTable, report: (Diagnostic.Severity, Int?, String) -> Void) {
            for (key, value) in t.entries {
                func bool(_ set: (inout Settings, Bool) -> Void) {
                    guard let b = value.bool else { report(.error, t.line, "`\(key)` takes true or false"); return }
                    set(&self, b)
                }
                func strings() -> [String]? {
                    guard let a = value.array, a.allSatisfy({ $0.string != nil }) else {
                        report(.error, t.line, "`\(key)` takes a list of strings"); return nil
                    }
                    return a.compactMap(\.string)
                }
                switch key {
                case "enabled":
                    bool { s, b in if !b { s.enabled = false } }
                case "abbreviations": bool { $0.abbreviations = $1 }
                case "instant_atoms": bool { $0.instantAtoms = $1 }
                case "ligatures": bool { $0.ligatures = $1 }
                case "postfix": bool { $0.postfix = $1 }
                case "structure_editor": bool { $0.structureEditor = $1 }
                case "auto_preamble":
                    guard let v = value.string.flatMap(AutoPreamble.init(rawValue:)) else {
                        report(.error, t.line, "`auto_preamble` is \"insert\", \"prompt\" or \"off\""); continue
                    }
                    autoPreamble = v
                case "fraction_trigger":
                    guard let v = value.string.flatMap(FractionTrigger.init(rawValue:)) else {
                        report(.error, t.line, "`fraction_trigger` is \"tab\", \"auto\" or \"off\""); continue
                    }
                    fractionTrigger = v
                case "fraction_operator":
                    guard let v = value.string, Self.fractionOperators.contains(v) else {
                        report(.error, t.line, "`fraction_operator` is \"//\" or \"/\" (turn fractions off with fraction_trigger = \"off\")"); continue
                    }
                    fractionOperator = v
                case "leader":
                    guard let v = value.string else { report(.error, t.line, "`leader` is a string"); continue }
                    if let problem = Self.leaderProblem(v) { report(.error, t.line, problem); continue }
                    leader = v
                case "profile":
                    guard let v = value.string else { report(.error, t.line, "`profile` is a string"); continue }
                    profile = v
                case "packs": if let v = strings() { enabledPacks = Array(Set(enabledPacks).union(v)).sorted() }
                case "disabled_packs": if let v = strings() { disabledPacks = Array(Set(disabledPacks).union(v)).sorted() }
                case "disable": if let v = strings() { disabled = Array(Set(disabled).union(v)).sorted() }
                default:
                    report(.warning, t.line, "unknown setting `\(key)`")
                }
            }
        }
    }

    /// Notation profile (PLAN §7): flat key/values templates read as
    /// `<<profile.KEY>>`. Named profiles layer over the defaults.
    public struct Profile: Equatable, Sendable {
        public var values: [String: String]
        public init(_ values: [String: String] = Profile.defaults) { self.values = values }

        public subscript(key: String) -> String? { values[key] }

        public static let defaults: [String: String] = [
            "diff_d": "d",
            "frac": "\\frac",
            "vector": "\\vec",
            "transpose": "^\\top",
            "set_sep": "\\mid",
            "expectation": "\\mathbb{E}",
            "probability": "\\Pr",
            "matrix_dots": "amsmath",
            "ref_cmd": "\\cref",
            "cite_cmd": "\\cite",
            "ligature_trailing_space": "true",
            "label_sep": ":",
            "label_prefix.fig": "fig",
            "label_prefix.tab": "tab",
            "label_prefix.eq": "eq",
            "label_prefix.sec": "sec",
            "label_prefix.ch": "ch",
            "label_prefix.thm": "thm",
            "label_prefix.lem": "lem",
            "label_prefix.alg": "alg",
            "label_prefix.lst": "lst",
        ]

        /// Built-in named profiles, as overrides of the defaults.
        public static let named: [String: [String: String]] = [
            "default": [:],
            // Upright differentials and operators (ISO 80000-2 style).
            "upright": ["diff_d": "\\mathrm{d}", "expectation": "\\mathrm{E}", "transpose": "^\\mathsf{T}"],
            // Bold vectors and a plain ^T, common in engineering venues.
            "bold": ["vector": "\\mathbf", "transpose": "^T"],
        ]
    }
}
