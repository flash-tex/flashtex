import Foundation

/// The compiler's command inventory (`crates/compiler/supported/
/// supported-latex.json`, schema `flashtex-supported-latex/1`) as a
/// completion vocabulary. Both apps bundle a byte-identical copy of that
/// file (`apps/mac/scripts/sync-supported-latex.sh` writes both and CI
/// checks them); this decoder is the iPad's, and mirrors what
/// `Completion.Vocabulary` in the Mac app derives from the same bytes:
/// every rendered command once, in the Mac's offer order — text commands
/// in file order, then math structures, operators and symbols — and the
/// environments in file order.
public struct LaTeXVocabulary: Sendable, Equatable {
    public enum Mode: String, Decodable, Sendable { case text, math }

    public struct Command: Sendable, Equatable {
        /// Name without the leading backslash; `\\` is the name `\`.
        public let name: String
        /// Argument shape shown after the name, e.g. `{key}` or `[options]{class}`.
        public let arguments: String
        public let description: String
        public let mode: Mode
        /// The inventory's `origin` (`text_dispatch`, `math_symbol`, …).
        public let origin: String
        /// Rendered glyph for a math symbol.
        public let glyph: String?
        /// For a command the compiler accepts in both modes (`\textbf`,
        /// `\quad`): the math-mode behaviour, shown after the text one.
        public let mathDescription: String?
        /// The one document class that defines the command, or nil for all.
        public let requiresClass: String?

        public init(name: String, arguments: String, description: String, mode: Mode, origin: String,
                    glyph: String? = nil, mathDescription: String? = nil, requiresClass: String? = nil) {
            self.name = name; self.arguments = arguments; self.description = description; self.mode = mode
            self.origin = origin; self.glyph = glyph; self.mathDescription = mathDescription; self.requiresClass = requiresClass
        }

        /// One documentation line for a completion row.
        public var documentation: String {
            var line = description
            if let mathDescription { line += " — in math: " + mathDescription }
            if let requiresClass { line += " (\(requiresClass) only)" }
            return line
        }
    }

    public struct Environment: Sendable, Equatable {
        public let name: String
        public let description: String
        public let mode: Mode
        public let requiresClass: String?
        public init(name: String, description: String, mode: Mode, requiresClass: String? = nil) {
            self.name = name; self.description = description; self.mode = mode; self.requiresClass = requiresClass
        }
    }

    public static let schema = "flashtex-supported-latex/1"
    public static let empty = LaTeXVocabulary(commands: [], environments: [], compilerVersion: "")

    public let commands: [Command]
    public let environments: [Environment]
    public let compilerVersion: String
    private let byName: [String: Command]

    public init(commands: [Command], environments: [Environment], compilerVersion: String) {
        self.commands = commands
        self.environments = environments
        self.compilerVersion = compilerVersion
        byName = Dictionary(commands.map { ($0.name, $0) }, uniquingKeysWith: { a, _ in a })
    }

    public enum InventoryError: Error, CustomStringConvertible, Equatable {
        case schema(String)
        public var description: String {
            switch self { case .schema(let got): return "unexpected schema \(got); expected \(LaTeXVocabulary.schema)" }
        }
    }

    // MARK: decoding (`flashtex-supported-latex/1`; only the fields the editor uses)

    struct Inventory: Decodable {
        struct Command: Decodable {
            let name: String
            let mode: Mode
            let origin: String
            let arguments: String
            let description: String
            var glyph: String? = nil
            let renders: Bool
            var requiresClass: String? = nil
            enum CodingKeys: String, CodingKey { case name, mode, origin, arguments, description, glyph, renders, requiresClass = "requires_class" }
        }
        struct Environment: Decodable {
            let name: String
            let mode: Mode
            let description: String
            var requiresClass: String? = nil
            enum CodingKeys: String, CodingKey { case name, mode, description, requiresClass = "requires_class" }
        }
        let schema: String
        let compilerVersion: String
        let commands: [Command]
        let environments: [Environment]
        enum CodingKeys: String, CodingKey { case schema, compilerVersion = "compiler_version", commands, environments }
    }

    /// Decodes the inventory bytes; throws on a foreign schema.
    public init(inventoryData data: Data) throws {
        let inventory = try JSONDecoder().decode(Inventory.self, from: data)
        guard inventory.schema == Self.schema else { throw InventoryError.schema(inventory.schema) }
        let rendered = inventory.commands.filter(\.renders)
        let mathDescriptions = Dictionary(rendered.filter { $0.mode == .math }.map { ($0.name, $0.description) }, uniquingKeysWith: { a, _ in a })
        var out: [Command] = []
        var seen = Set<String>()
        func add(_ c: Inventory.Command, mathDescription: String? = nil) {
            guard seen.insert(c.name).inserted else { return }
            out.append(Command(name: c.name, arguments: c.arguments, description: c.description, mode: c.mode, origin: c.origin,
                               glyph: c.glyph, mathDescription: mathDescription, requiresClass: c.requiresClass))
        }
        for c in rendered where c.mode == .text && c.origin != "control_symbol" { add(c, mathDescription: mathDescriptions[c.name]) }
        for c in rendered where c.origin == "control_symbol" && c.name == "\\" { add(c) }
        for origin in ["math_structure", "math_operator", "math_symbol"] {
            for c in rendered where c.origin == origin { add(c) }
        }
        self.init(commands: out,
                  environments: inventory.environments.map { Environment(name: $0.name, description: $0.description, mode: $0.mode, requiresClass: $0.requiresClass) },
                  compilerVersion: inventory.compilerVersion)
    }

    public func command(named name: String) -> Command? { byName[name] }

    /// Text commands the compiler also accepts inside math: the label and
    /// reference family and `\\`, the row break of every math grid.
    public static let mathAllowedTextCommands: Set<String> = ["label", "ref", "eqref", "pageref", "\\"]

    /// Whether `command` belongs in a list at the caret's mode: in math the
    /// text-only commands are out, in text the math-only ones are, and an
    /// unknown mode (nil) hides nothing (the Mac's `Completion.allows`).
    public static func allows(_ command: Command, mathMode: Bool?) -> Bool {
        guard let mathMode else { return true }
        switch command.mode {
        case .math: return mathMode
        case .text: return !mathMode || command.mathDescription != nil || mathAllowedTextCommands.contains(command.name)
        }
    }

    public static func == (a: LaTeXVocabulary, b: LaTeXVocabulary) -> Bool {
        a.commands == b.commands && a.environments == b.environments && a.compilerVersion == b.compilerVersion
    }
}

/// Replacement text plus the caret position inside it and the further
/// placeholders Tab visits, all UTF-16 offsets into `text` (the Mac's
/// `Completion.Snippet`).
public struct LaTeXSnippet: Equatable, Sendable {
    public var text: String
    public var caretUTF16: Int
    /// Further placeholders, in Tab order, after the caret's; the editor
    /// visits them with Tab and leaves the snippet after the last.
    public var stops: [Int]
    public init(text: String, caretUTF16: Int, stops: [Int] = []) {
        self.text = text; self.caretUTF16 = caretUTF16; self.stops = stops
    }
}

/// Snippet rules shared with the Mac's completion (Completion.swift
/// `argumentSnippet`, `environmentSnippet`, `snippetOverrides`).
public enum LaTeXSnippets {
    /// Whole-snippet overrides for commands whose insertion is not the brace
    /// skeleton of their argument shape.
    public static let overrides: [String: LaTeXSnippet] = [
        "left": LaTeXSnippet(text: "\\left( \\right)", caretUTF16: 6, stops: [14]),
    ]

    /// `\name` plus its argument shape as an insertion: every `{…}` of
    /// `arguments` becomes `{}`, `[…]` optionals are dropped, the caret lands
    /// in the first braces and Tab visits the later ones, then leaves. Nil
    /// when the shape has no braced argument.
    public static func argument(name: String, arguments: String) -> LaTeXSnippet? {
        if let override = overrides[name] { return override }
        var out = "\\" + name
        var stops: [Int] = []
        var i = arguments.startIndex
        while i < arguments.endIndex {
            let c = arguments[i]
            if c == "[" {
                i = arguments[i...].firstIndex(of: "]").map(arguments.index(after:)) ?? arguments.endIndex
            } else if c == "{" {
                out += "{"
                stops.append((out as NSString).length)
                out += "}"
                i = arguments[i...].firstIndex(of: "}").map(arguments.index(after:)) ?? arguments.endIndex
            } else {
                i = arguments.index(after: i)
            }
        }
        guard let caret = stops.first else { return nil }
        return LaTeXSnippet(text: out, caretUTF16: caret, stops: Array(stops.dropFirst()) + [(out as NSString).length])
    }

    /// Body skeleton inserted after `\begin{` for `name`: the caret on the
    /// middle line — indented one `unit` when `rules` indent that body — or
    /// inside the first placeholder of a richer template (`figure`/`table`),
    /// the later placeholders as Tab stops. The middle line starts with the
    /// rules' line template for the environment (`\item ` in a list).
    /// Offsets are UTF-16 into the inserted text, which starts right after
    /// the `\begin{` the user typed.
    public static func environment(_ name: String, indent: String, unit: String = "", rules: EnvironmentEditingRules = .conventional) -> LaTeXSnippet {
        let nl = "\n" + indent
        let body = rules.indentsBody(of: name) ? unit : ""
        let base = name.hasSuffix("*") ? String(name.dropLast()) : name
        var lines: [String]
        switch base {
        case "figure": lines = ["\(name)}", "\\centering", "\\includegraphics[width=0.8\\linewidth]{⟨⟩}", "\\caption{⟨⟩}", "\\label{fig:⟨⟩}", "\\end{\(name)}"]
        case "table": lines = ["\(name)}", "\\centering", "\\begin{tabular}{⟨⟩}", "\\end{tabular}", "\\caption{⟨⟩}", "\\label{tab:⟨⟩}", "\\end{\(name)}"]
        default:
            let template = rules.newLineText(in: name)
            let at = EnvironmentEditingRules.caretOffset(in: template)
            let ns = template as NSString
            var middle = ns.substring(to: at) + "⟨⟩" + ns.substring(from: at)
            if at < ns.length { middle += "⟨⟩" }
            lines = ["\(name)}", middle, "\\end{\(name)}"]
        }
        var out = ""
        var stops: [Int] = []
        for (i, line) in lines.enumerated() {
            if i > 0 { out += nl }
            if i > 0, i < lines.count - 1 { out += body }
            var rest = Substring(line)
            while let r = rest.range(of: "⟨⟩") {
                out += rest[..<r.lowerBound]
                stops.append((out as NSString).length)
                rest = rest[r.upperBound...]
            }
            out += rest
        }
        let caret = stops.first ?? (out as NSString).length
        return LaTeXSnippet(text: out, caretUTF16: caret, stops: Array(stops.dropFirst()) + [(out as NSString).length])
    }
}
