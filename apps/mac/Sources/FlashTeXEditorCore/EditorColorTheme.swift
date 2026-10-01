// Shared by the Mac app (SyntaxTheme / EditorThemeRuntime in FlashTeXMac) and
// the iPad app (apps/ios EditorTheme, through the symlinked FlashTeXEditorCore
// target). Platform-free on purpose: colours are sRGB hex values, turned into
// NSColor / UIColor by each app.
import Foundation

/// An editor colour theme (lane EDITOR-THEMES): every `SyntaxHighlighter.Kind`
/// plus the editor chrome (ground, gutter, current line, selection, caret,
/// bracket match, invisibles) and the diagnostic underlines, separately for
/// the light and the dark appearance.
///
/// Built-in themes are below (`builtIns`); users add their own as JSON files
/// (`EditorColorTheme.decode(json:)`, format documented in
/// apps/mac/docs/editor-themes.md). A theme file may leave roles out: they
/// fall back to its `basedOn` theme (FlashTeX by default), and a theme with
/// only one variant uses it for both appearances.
///
/// Colour values of the built-ins come from each theme's published palette
/// (Solarized, GitHub Primer, Atom One, Dracula/Alucard and Nord are all MIT
/// licensed; colour values themselves are facts, not code). Where a family has
/// no official variant for one appearance, the variant is derived from the
/// same palette and says so in its comment.
public struct EditorColorTheme: Equatable, Sendable, Identifiable {
    /// Every colour a theme assigns. Raw values are the JSON keys.
    public enum Role: String, CaseIterable, Codable, Sendable {
        // Syntax: one per `SyntaxHighlighter.Kind` (see `init(kind:)`).
        case command, mathCommand, environment, math, mathDelimiter, number, comment
        case brace, bracket, reference, file, definition, verbatim
        // Diagnostics.
        case error, warning
        // Chrome.
        case background, foreground, gutterBackground, gutterText, gutterActiveText
        case currentLine, selection, caret, bracketMatch, invisibles
        /// Replacement glyphs drawn by hybrid conceal (α for `\alpha` …) in text mode.
        case conceal

        public init(kind: SyntaxHighlighter.Kind) {
            switch kind {
            case .command: self = .command
            case .mathCommand: self = .mathCommand
            case .environment: self = .environment
            case .math: self = .math
            case .mathDelimiter: self = .mathDelimiter
            case .number: self = .number
            case .comment: self = .comment
            case .brace: self = .brace
            case .bracket: self = .bracket
            case .reference: self = .reference
            case .file: self = .file
            case .definition: self = .definition
            case .verbatim: self = .verbatim
            }
        }

        public static let syntaxRoles: [Role] = [.command, .mathCommand, .environment, .math, .mathDelimiter, .number, .comment,
                                                 .brace, .bracket, .reference, .file, .definition, .verbatim, .error, .warning]
        /// What the Settings pane lets a user override on top of the code theme.
        public static let chromeRoles: [Role] = [.background, .foreground, .gutterBackground, .gutterText, .gutterActiveText,
                                                 .currentLine, .selection, .caret, .bracketMatch, .invisibles, .conceal]

        /// Position in `allCases` (a dense index for per-role tables).
        public var index: Int { Self.indices[self]! }
        private static let indices: [Role: Int] = Dictionary(uniqueKeysWithValues: allCases.enumerated().map { ($1, $0) })

        /// Human label for Settings and VoiceOver.
        public var label: String {
            switch self {
            case .command: "Commands"
            case .mathCommand: "Math commands"
            case .environment: "Environment names"
            case .math: "Math"
            case .mathDelimiter: "Math delimiters"
            case .number: "Numbers"
            case .comment: "Comments"
            case .brace: "Braces"
            case .bracket: "Brackets"
            case .reference: "References and citations"
            case .file: "File names"
            case .definition: "Defined names"
            case .verbatim: "Verbatim"
            case .error: "Errors"
            case .warning: "Warnings"
            case .background: "Background"
            case .foreground: "Text"
            case .gutterBackground: "Gutter background"
            case .gutterText: "Line numbers"
            case .gutterActiveText: "Current line number"
            case .currentLine: "Current line"
            case .selection: "Selection"
            case .caret: "Insertion point"
            case .bracketMatch: "Matching bracket"
            case .invisibles: "Invisible characters"
            case .conceal: "Concealed symbols"
            }
        }
    }

    public enum Variant: String, CaseIterable, Codable, Sendable {
        case light, dark
    }

    /// An sRGB colour with alpha, 8 bits a channel. JSON: `"#RRGGBB"` or `"#RRGGBBAA"`.
    public struct Color: Hashable, Sendable, Codable, CustomStringConvertible {
        public var red: UInt8, green: UInt8, blue: UInt8, alpha: UInt8
        public init(red: UInt8, green: UInt8, blue: UInt8, alpha: UInt8 = 255) {
            self.red = red; self.green = green; self.blue = blue; self.alpha = alpha
        }
        /// 0xRRGGBB, opaque.
        public init(_ rgb: UInt32) {
            self.init(red: UInt8((rgb >> 16) & 0xFF), green: UInt8((rgb >> 8) & 0xFF), blue: UInt8(rgb & 0xFF))
        }
        /// `#RGB`, `#RRGGBB` or `#RRGGBBAA` (the `#` is optional); nil otherwise.
        public init?(hex: String) {
            var s = hex.trimmingCharacters(in: .whitespaces)
            if s.hasPrefix("#") { s.removeFirst() }
            guard s.allSatisfy(\.isHexDigit) else { return nil }
            if s.count == 3 { s = s.map { "\($0)\($0)" }.joined() }
            guard s.count == 6 || s.count == 8, let v = UInt32(s, radix: 16) else { return nil }
            if s.count == 6 { self.init(v) } else {
                self.init(red: UInt8(v >> 24), green: UInt8((v >> 16) & 0xFF), blue: UInt8((v >> 8) & 0xFF), alpha: UInt8(v & 0xFF))
            }
        }
        public var hex: String {
            let rgb = String(format: "#%02X%02X%02X", red, green, blue)
            return alpha == 255 ? rgb : rgb + String(format: "%02X", alpha)
        }
        public var description: String { hex }
        /// Components 0…1 (for NSColor / UIColor).
        public var components: (red: Double, green: Double, blue: Double, alpha: Double) {
            (Double(red) / 255, Double(green) / 255, Double(blue) / 255, Double(alpha) / 255)
        }
        public func withAlpha(_ a: Double) -> Color {
            Color(red: red, green: green, blue: blue, alpha: UInt8(max(0, min(255, (a * 255).rounded()))))
        }
        /// WCAG relative luminance (sRGB), for contrast checks.
        public var luminance: Double {
            func lin(_ c: UInt8) -> Double {
                let v = Double(c) / 255
                return v <= 0.04045 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4)
            }
            return 0.2126 * lin(red) + 0.7152 * lin(green) + 0.0722 * lin(blue)
        }
        /// WCAG contrast ratio against `other` (alpha ignored).
        public func contrast(with other: Color) -> Double {
            let a = luminance, b = other.luminance
            return (max(a, b) + 0.05) / (min(a, b) + 0.05)
        }

        public init(from decoder: Decoder) throws {
            let s = try decoder.singleValueContainer().decode(String.self)
            guard let c = Color(hex: s) else {
                throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "not a #RRGGBB[AA] colour: \(s)"))
            }
            self = c
        }
        public func encode(to encoder: Encoder) throws {
            var c = encoder.singleValueContainer()
            try c.encode(hex)
        }
    }

    /// One appearance's colours. A syntax role left out is drawn in the plain
    /// text colour; a chrome role left out falls back to FlashTeX's.
    public struct Palette: Equatable, Sendable {
        public var colors: [Role: Color]
        public init(_ colors: [Role: Color] = [:]) { self.colors = colors }
        public subscript(role: Role) -> Color? {
            get { colors[role] }
            set { colors[role] = newValue }
        }
        /// `self` with every role it lacks taken from `base`.
        public func filled(from base: Palette) -> Palette {
            Palette(colors.merging(base.colors) { mine, _ in mine })
        }
    }

    public var id: String
    public var name: String
    public var light: Palette
    public var dark: Palette
    /// True for the themes shipped with the app (not editable or removable).
    public var isBuiltIn: Bool

    public init(id: String, name: String, light: Palette, dark: Palette, isBuiltIn: Bool = false) {
        self.id = id; self.name = name; self.light = light; self.dark = dark; self.isBuiltIn = isBuiltIn
    }

    public func palette(_ variant: Variant) -> Palette { variant == .light ? light : dark }

    /// `role`'s colour in `variant`. Syntax roles may be nil (plain text);
    /// chrome roles never are after `resolved()`.
    public func color(_ role: Role, _ variant: Variant) -> Color? { palette(variant)[role] }

    /// Every chrome/diagnostic role filled from FlashTeX's palette of the same
    /// appearance (syntax roles stay as the theme leaves them).
    public func resolved() -> EditorColorTheme {
        var t = self
        let base = EditorColorTheme.flashtex
        for variant in Variant.allCases {
            var p = t.palette(variant)
            for role in Role.allCases where !Role.syntaxRoles.contains(role) || role == .error || role == .warning {
                if p[role] == nil { p[role] = base.palette(variant)[role] }
            }
            if variant == .light { t.light = p } else { t.dark = p }
        }
        return t
    }

    /// `self` with `overrides` applied on top (the Settings chrome overrides).
    public func applying(_ overrides: Overrides) -> EditorColorTheme {
        guard !overrides.isEmpty else { return self }
        var t = self
        for (role, color) in overrides.light { t.light[role] = color }
        for (role, color) in overrides.dark { t.dark[role] = color }
        return t
    }

    /// Per-appearance colour overrides a user sets in Settings on top of the
    /// selected theme ("tied to the code theme with overrides allowed").
    public struct Overrides: Equatable, Sendable, Codable {
        public var light: [Role: Color]
        public var dark: [Role: Color]
        public init(light: [Role: Color] = [:], dark: [Role: Color] = [:]) { self.light = light; self.dark = dark }
        public var isEmpty: Bool { light.isEmpty && dark.isEmpty }
        public subscript(variant: Variant) -> [Role: Color] {
            get { variant == .light ? light : dark }
            set { if variant == .light { light = newValue } else { dark = newValue } }
        }

        enum CodingKeys: String, CodingKey { case light, dark }
        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            light = Self.roles(try c.decodeIfPresent([String: Color].self, forKey: .light) ?? [:])
            dark = Self.roles(try c.decodeIfPresent([String: Color].self, forKey: .dark) ?? [:])
        }
        public func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(Dictionary(uniqueKeysWithValues: light.map { ($0.rawValue, $1) }), forKey: .light)
            try c.encode(Dictionary(uniqueKeysWithValues: dark.map { ($0.rawValue, $1) }), forKey: .dark)
        }
        static func roles(_ raw: [String: Color]) -> [Role: Color] {
            Dictionary(uniqueKeysWithValues: raw.compactMap { k, v in Role(rawValue: k).map { ($0, v) } })
        }
        public func encoded() -> Data? { try? JSONEncoder().encode(self) }
        public static func decoded(_ data: Data) -> Overrides? { try? JSONDecoder().decode(Overrides.self, from: data) }
    }

    // MARK: JSON theme files

    /// The on-disk format (`formatVersion` 1):
    ///
    /// ```json
    /// { "formatVersion": 1, "name": "Paper", "basedOn": "flashtex",
    ///   "light": { "background": "#FFFFF8", "command": "#0033B3" },
    ///   "dark":  { "background": "#1B1B1B", "command": "#CF8E6D" } }
    /// ```
    ///
    /// Unknown keys are ignored (a newer app's roles in an older app), so are
    /// unknown roles; a malformed colour is an error naming it.
    struct File: Codable {
        var formatVersion: Int?
        var id: String?
        var name: String
        var basedOn: String?
        var light: [String: String]?
        var dark: [String: String]?
    }

    public enum FileError: Error, Equatable, CustomStringConvertible {
        case notJSON(String)
        case noVariant
        case badColor(role: String, value: String)
        case unsupportedVersion(Int)
        public var description: String {
            switch self {
            case .notJSON(let why): "Not a FlashTeX theme file: \(why)"
            case .noVariant: "The theme has neither a \"light\" nor a \"dark\" palette."
            case .badColor(let role, let value): "“\(value)” is not a colour for \(role) (use #RRGGBB or #RRGGBBAA)."
            case .unsupportedVersion(let v): "Theme format \(v) is newer than this FlashTeX understands (1)."
            }
        }
    }

    public static let fileFormatVersion = 1

    /// Parses a theme file. `fallbackID` names it when the file carries no
    /// `id` (the file name, for themes in the user's folder).
    public static func decode(json data: Data, fallbackID: String) throws -> EditorColorTheme {
        let file: File
        do { file = try JSONDecoder().decode(File.self, from: data) } catch {
            throw FileError.notJSON(String(describing: error).prefix(200).description)
        }
        if let v = file.formatVersion, v > fileFormatVersion { throw FileError.unsupportedVersion(v) }
        func palette(_ raw: [String: String]?) throws -> Palette? {
            guard let raw else { return nil }
            var p = Palette()
            for (key, value) in raw {
                guard let role = Role(rawValue: key) else { continue }
                guard let c = Color(hex: value) else { throw FileError.badColor(role: key, value: value) }
                p[role] = c
            }
            return p
        }
        let light = try palette(file.light), dark = try palette(file.dark)
        guard light != nil || dark != nil else { throw FileError.noVariant }
        let base = builtIn(id: file.basedOn ?? "") ?? flashtex
        let l = (light ?? dark!).filled(from: base.light)
        let d = (dark ?? light!).filled(from: base.dark)
        let id = (file.id?.isEmpty == false ? file.id! : fallbackID)
        return EditorColorTheme(id: id, name: file.name.isEmpty ? id : file.name, light: l, dark: d)
    }

    /// The theme as a complete theme file (every role both appearances), for Export.
    public func encodedJSON() -> Data {
        func raw(_ p: Palette) -> [String: String] { Dictionary(uniqueKeysWithValues: p.colors.map { ($0.rawValue, $1.hex) }) }
        let file = File(formatVersion: Self.fileFormatVersion, id: id, name: name, basedOn: nil, light: raw(light), dark: raw(dark))
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return (try? encoder.encode(file)) ?? Data()
    }

    // MARK: built-in themes

    public static func builtIn(id: String) -> EditorColorTheme? { builtIns.first { $0.id == id } }

    public static let defaultID = "flashtex"

    public static let builtIns: [EditorColorTheme] = [flashtex, solarized, github, one, dracula, nord]

    private static func p(_ pairs: [(Role, UInt32)], alpha: [Role: UInt8] = [:]) -> Palette {
        var palette = Palette()
        for (role, rgb) in pairs {
            var c = Color(rgb)
            if let a = alpha[role] { c.alpha = a }
            palette[role] = c
        }
        return palette
    }

    /// FlashTeX: JetBrains' code vocabulary (IntelliJ Light / the New UI Dark)
    /// on the Islands editor surface — the values the editor always used.
    public static let flashtex = EditorColorTheme(
        id: "flashtex", name: "FlashTeX",
        light: p([(.command, 0x0033B3), (.mathCommand, 0x00627A), (.environment, 0x00627A), (.math, 0x871094),
                  (.mathDelimiter, 0x871094), (.number, 0x1750EB), (.comment, 0x8C8C8C), (.brace, 0x6C707E),
                  (.bracket, 0x6C707E), (.reference, 0x067D17), (.file, 0x067D17), (.definition, 0x9E880D),
                  (.error, 0xE51400), (.warning, 0xBF8803),
                  (.background, 0xFFFFFF), (.foreground, 0x080808), (.gutterBackground, 0xFFFFFF), (.gutterText, 0xAEB3C2),
                  (.gutterActiveText, 0x6C707E), (.currentLine, 0xF5F8FE), (.selection, 0xD0DFFE), (.caret, 0x080808),
                  (.bracketMatch, 0x93D9D9), (.invisibles, 0xC9CCD6), (.conceal, 0x871094)]),
        dark: p([(.command, 0xCF8E6D), (.mathCommand, 0x2AACB8), (.environment, 0x56A8F5), (.math, 0xC77DBB),
                 (.mathDelimiter, 0xC77DBB), (.number, 0x2AACB8), (.comment, 0x7A7E85), (.brace, 0x9DA0A8),
                 (.bracket, 0x9DA0A8), (.reference, 0x6AAB73), (.file, 0x6AAB73), (.definition, 0xB3AE60),
                 (.error, 0xF14C4C), (.warning, 0xCCA700),
                 (.background, 0x191A1C), (.foreground, 0xBCBEC4), (.gutterBackground, 0x191A1C), (.gutterText, 0x4B5059),
                 (.gutterActiveText, 0x9DA0A8), (.currentLine, 0x1F2024), (.selection, 0x2A4371), (.caret, 0xBCBEC4),
                 (.bracketMatch, 0x3B514D), (.invisibles, 0x3E4148), (.conceal, 0xC77DBB)]),
        isBuiltIn: true)

    /// Solarized (Ethan Schoonover, MIT): light on base3, dark on base03,
    /// the same eight accents in both.
    public static let solarized = EditorColorTheme(
        id: "solarized", name: "Solarized",
        light: p([(.command, 0x859900), (.mathCommand, 0xCB4B16), (.environment, 0x268BD2), (.math, 0x6C71C4),
                  (.mathDelimiter, 0x6C71C4), (.number, 0xD33682), (.comment, 0x93A1A1), (.brace, 0x93A1A1),
                  (.bracket, 0x93A1A1), (.reference, 0x2AA198), (.file, 0x2AA198), (.definition, 0xB58900),
                  (.error, 0xDC322F), (.warning, 0xB58900),
                  (.background, 0xFDF6E3), (.foreground, 0x657B83), (.gutterBackground, 0xEEE8D5), (.gutterText, 0x93A1A1),
                  (.gutterActiveText, 0x586E75), (.currentLine, 0xEEE8D5), (.selection, 0xDDD6C1), (.caret, 0x586E75),
                  (.bracketMatch, 0xD3CBB7), (.invisibles, 0xD3CBB7), (.conceal, 0x6C71C4)]),
        dark: p([(.command, 0x859900), (.mathCommand, 0xCB4B16), (.environment, 0x268BD2), (.math, 0x6C71C4),
                 (.mathDelimiter, 0x6C71C4), (.number, 0xD33682), (.comment, 0x586E75), (.brace, 0x586E75),
                 (.bracket, 0x586E75), (.reference, 0x2AA198), (.file, 0x2AA198), (.definition, 0xB58900),
                 (.error, 0xDC322F), (.warning, 0xB58900),
                 (.background, 0x002B36), (.foreground, 0x839496), (.gutterBackground, 0x073642), (.gutterText, 0x586E75),
                 (.gutterActiveText, 0x93A1A1), (.currentLine, 0x073642), (.selection, 0x274642), (.caret, 0x93A1A1),
                 (.bracketMatch, 0x2E4A50), (.invisibles, 0x284B54), (.conceal, 0x6C71C4)]),
        isBuiltIn: true)

    /// GitHub Light / GitHub Dark (Primer "prettylights" syntax, MIT).
    public static let github = EditorColorTheme(
        id: "github", name: "GitHub",
        light: p([(.command, 0xCF222E), (.mathCommand, 0x8250DF), (.environment, 0x116329), (.math, 0x0550AE),
                  (.mathDelimiter, 0x0550AE), (.number, 0x0550AE), (.comment, 0x6E7781), (.brace, 0x57606A),
                  (.bracket, 0x57606A), (.reference, 0x0A3069), (.file, 0x0A3069), (.definition, 0x953800),
                  (.error, 0xD1242F), (.warning, 0x9A6700),
                  (.background, 0xFFFFFF), (.foreground, 0x1F2328), (.gutterBackground, 0xFFFFFF), (.gutterText, 0x8C959F),
                  (.gutterActiveText, 0x1F2328), (.currentLine, 0xF6F8FA), (.selection, 0xB6D7FF), (.caret, 0x0969DA),
                  (.bracketMatch, 0xC6EFCF), (.invisibles, 0xD0D7DE), (.conceal, 0x0550AE)]),
        dark: p([(.command, 0xFF7B72), (.mathCommand, 0xD2A8FF), (.environment, 0x7EE787), (.math, 0x79C0FF),
                 (.mathDelimiter, 0x79C0FF), (.number, 0x79C0FF), (.comment, 0x8B949E), (.brace, 0x8B949E),
                 (.bracket, 0x8B949E), (.reference, 0xA5D6FF), (.file, 0xA5D6FF), (.definition, 0xFFA657),
                 (.error, 0xF85149), (.warning, 0xD29922),
                 (.background, 0x0D1117), (.foreground, 0xE6EDF3), (.gutterBackground, 0x0D1117), (.gutterText, 0x6E7681),
                 (.gutterActiveText, 0xE6EDF3), (.currentLine, 0x161B22), (.selection, 0x264F78), (.caret, 0x2F81F7),
                 (.bracketMatch, 0x1F4A2C), (.invisibles, 0x30363D), (.conceal, 0x79C0FF)]),
        isBuiltIn: true)

    /// Atom One Light / One Dark (MIT).
    public static let one = EditorColorTheme(
        id: "one", name: "One Light / One Dark",
        light: p([(.command, 0xA626A4), (.mathCommand, 0x0184BC), (.environment, 0xC18401), (.math, 0x4078F2),
                  (.mathDelimiter, 0x4078F2), (.number, 0x986801), (.comment, 0xA0A1A7), (.brace, 0x696C77),
                  (.bracket, 0x696C77), (.reference, 0x50A14F), (.file, 0x50A14F), (.definition, 0xE45649),
                  (.error, 0xCA1243), (.warning, 0xC18401),
                  (.background, 0xFAFAFA), (.foreground, 0x383A42), (.gutterBackground, 0xFAFAFA), (.gutterText, 0x9D9D9F),
                  (.gutterActiveText, 0x383A42), (.currentLine, 0xF0F0F1), (.selection, 0xE5E5E6), (.caret, 0x526FFF),
                  (.bracketMatch, 0xDBDBDC), (.invisibles, 0xD3D3D3), (.conceal, 0x4078F2)]),
        dark: p([(.command, 0xC678DD), (.mathCommand, 0x56B6C2), (.environment, 0xE5C07B), (.math, 0x61AFEF),
                 (.mathDelimiter, 0x61AFEF), (.number, 0xD19A66), (.comment, 0x5C6370), (.brace, 0x7F848E),
                 (.bracket, 0x7F848E), (.reference, 0x98C379), (.file, 0x98C379), (.definition, 0xE06C75),
                 (.error, 0xF44747), (.warning, 0xE5C07B),
                 (.background, 0x282C34), (.foreground, 0xABB2BF), (.gutterBackground, 0x282C34), (.gutterText, 0x4B5263),
                 (.gutterActiveText, 0xABB2BF), (.currentLine, 0x2C313C), (.selection, 0x3E4451), (.caret, 0x528BFF),
                 (.bracketMatch, 0x515A6B), (.invisibles, 0x3B4048), (.conceal, 0x61AFEF)]),
        isBuiltIn: true)

    /// Dracula (dark) and its official light counterpart Alucard (MIT).
    public static let dracula = EditorColorTheme(
        id: "dracula", name: "Dracula / Alucard",
        light: p([(.command, 0xA3144D), (.mathCommand, 0x036A96), (.environment, 0x14710A), (.math, 0x644AC9),
                  (.mathDelimiter, 0x644AC9), (.number, 0x644AC9), (.comment, 0x635D97), (.brace, 0x6C6A80),
                  (.bracket, 0x6C6A80), (.reference, 0x846E15), (.file, 0x846E15), (.definition, 0xA34D14),
                  (.error, 0xCB3A2A), (.warning, 0xA34D14),
                  (.background, 0xFFFBEB), (.foreground, 0x1F1F1F), (.gutterBackground, 0xFFFBEB), (.gutterText, 0x8F8AAE),
                  (.gutterActiveText, 0x1F1F1F), (.currentLine, 0xF5EFD7), (.selection, 0xCFCFDE), (.caret, 0x1F1F1F),
                  (.bracketMatch, 0xDEDCEB), (.invisibles, 0xD9D5C3), (.conceal, 0x644AC9)]),
        dark: p([(.command, 0xFF79C6), (.mathCommand, 0x8BE9FD), (.environment, 0x50FA7B), (.math, 0xBD93F9),
                 (.mathDelimiter, 0xBD93F9), (.number, 0xBD93F9), (.comment, 0x6272A4), (.brace, 0xA4A8C4),
                 (.bracket, 0xA4A8C4), (.reference, 0xF1FA8C), (.file, 0xF1FA8C), (.definition, 0xFFB86C),
                 (.error, 0xFF5555), (.warning, 0xFFB86C),
                 (.background, 0x282A36), (.foreground, 0xF8F8F2), (.gutterBackground, 0x282A36), (.gutterText, 0x6272A4),
                 (.gutterActiveText, 0xF8F8F2), (.currentLine, 0x44475A), (.selection, 0x44475A), (.caret, 0xF8F8F2),
                 (.bracketMatch, 0x5A5E7A), (.invisibles, 0x424450), (.conceal, 0xBD93F9)],
                alpha: [.currentLine: 0x80]),
        isBuiltIn: true)

    /// Nord (Arctic Ice Studio, MIT) — dark on Polar Night. Nord ships no light
    /// editor theme; the light variant is derived on Snow Storm with the Frost
    /// and Aurora hues darkened for contrast.
    public static let nord = EditorColorTheme(
        id: "nord", name: "Nord",
        light: p([(.command, 0x5E81AC), (.mathCommand, 0x3E7A8C), (.environment, 0x4F8584), (.math, 0x8F5E8A),
                  (.mathDelimiter, 0x8F5E8A), (.number, 0x8F5E8A), (.comment, 0x7B889E), (.brace, 0x6D7688),
                  (.bracket, 0x6D7688), (.reference, 0x5F7F45), (.file, 0x5F7F45), (.definition, 0xB35F3F),
                  (.error, 0xBF616A), (.warning, 0xA8832E),
                  (.background, 0xECEFF4), (.foreground, 0x2E3440), (.gutterBackground, 0xE5E9F0), (.gutterText, 0xA3ABB9),
                  (.gutterActiveText, 0x3B4252), (.currentLine, 0xE5E9F0), (.selection, 0xD8DEE9), (.caret, 0x2E3440),
                  (.bracketMatch, 0xC9D1DE), (.invisibles, 0xD0D6E0), (.conceal, 0x8F5E8A)]),
        dark: p([(.command, 0x81A1C1), (.mathCommand, 0x88C0D0), (.environment, 0x8FBCBB), (.math, 0xB48EAD),
                 (.mathDelimiter, 0xB48EAD), (.number, 0xB48EAD), (.comment, 0x616E88), (.brace, 0xA0A8B7),
                 (.bracket, 0xA0A8B7), (.reference, 0xA3BE8C), (.file, 0xA3BE8C), (.definition, 0xD08770),
                 (.error, 0xBF616A), (.warning, 0xEBCB8B),
                 (.background, 0x2E3440), (.foreground, 0xD8DEE9), (.gutterBackground, 0x2E3440), (.gutterText, 0x4C566A),
                 (.gutterActiveText, 0xD8DEE9), (.currentLine, 0x3B4252), (.selection, 0x434C5E), (.caret, 0xD8DEE9),
                 (.bracketMatch, 0x4C566A), (.invisibles, 0x434C5E), (.conceal, 0xB48EAD)]),
        isBuiltIn: true)
}
