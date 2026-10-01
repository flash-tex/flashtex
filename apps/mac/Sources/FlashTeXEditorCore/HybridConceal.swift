// Shared, platform-free (Foundation only): the Mac editor draws these spans
// (FlashTeXMac/HybridConcealDisplay.swift); the iPad could reuse the model.
import Foundation

/// Hybrid conceal (lane HYBRID-CONCEAL): which LaTeX constructs on a line are
/// shown as what they mean — `\alpha` as α, `\textbf{x}` as a bold x with the
/// command hidden — while the text itself is never changed. This is the pure
/// model: given a line's text and its syntax runs (`SyntaxHighlighter`, which
/// already knows math from text), it returns the concealable `Span`s. The
/// editor decides which spans are revealed (the caret's line, or the
/// construct the caret touches) and draws the rest.
///
/// Every span stays inside one line, so concealment of a line depends only
/// on that line's text and the mode it starts in, and an edit recomputes the
/// lines the highlighter re-lexed and nothing else.
public enum HybridConceal {
    /// A family of constructs a user can turn on or off on its own.
    public enum Class: String, CaseIterable, Codable, Sendable, Identifiable {
        /// `\alpha` → α, `\Gamma` → Γ, `\varepsilon` → ε (math only).
        case greek
        /// `\leq` → ≤, `\to` → →, `\infty` → ∞, `\in` → ∈, `\sum` → ∑ … (math;
        /// `\ldots`/`\dots` also in text).
        case symbols
        /// `^2` → ², `_i` → ᵢ where Unicode has the character, otherwise a
        /// smaller raised or lowered rendering of a simple script (math only).
        case scripts
        /// `\textbf{x}` bold with the command hidden, `\textit`/`\emph`
        /// italic, `\texttt`, `\mathbf`, `\mathbb{R}` → ℝ, `\mathcal{A}` → 𝒜,
        /// `\mathfrak{g}` → 𝔤.
        case fonts
        /// `\frac{a}{b}` → a⁄b (off by default).
        case fractions
        /// ``` `` ``` → “, `''` → ”, `--` → –, `---` → — (text only).
        case quotesDashes
        /// `\item` → •.
        case items
        /// Comments drawn dimmed (never concealed).
        case comments
        /// `\section{Title}` → § Title in heading style (off by default).
        case sectioning

        public var id: String { rawValue }

        public var label: String {
            switch self {
            case .greek: "Greek letters"
            case .symbols: "Math symbols and operators"
            case .scripts: "Subscripts and superscripts"
            case .fonts: "Font commands"
            case .fractions: "Fractions"
            case .quotesDashes: "Quotes and dashes"
            case .items: "List items"
            case .comments: "Dim comments"
            case .sectioning: "Section headings"
            }
        }

        /// A short example for Settings: source → display.
        public var example: String {
            switch self {
            case .greek: "\\alpha → α, \\Gamma → Γ"
            case .symbols: "\\leq → ≤, \\to → →, \\infty → ∞"
            case .scripts: "x^2 → x², a_i → aᵢ"
            case .fonts: "\\textbf{x} → x in bold, \\mathbb{R} → ℝ"
            case .fractions: "\\frac{a}{b} → a⁄b"
            case .quotesDashes: "``q'' → “q”, -- → –, --- → —"
            case .items: "\\item → •"
            case .comments: "% note, drawn dimmed"
            case .sectioning: "\\section{Intro} → § Intro"
            }
        }
    }

    /// When a concealed construct shows its source.
    public enum Reveal: String, CaseIterable, Codable, Sendable, Identifiable {
        /// Everything on the caret's line, or on any line a selection touches.
        case line
        /// Only a construct the caret or a selection touches.
        case construct
        /// Nothing is concealed ("always raw").
        case never

        public var id: String { rawValue }
        public var label: String {
            switch self {
            case .line: "On the caret’s line"
            case .construct: "At the caret"
            case .never: "Always show source"
            }
        }
    }

    /// What the user chose (persisted as JSON by the editor preferences).
    public struct Settings: Equatable, Sendable, Codable {
        /// Master switch.
        public var enabled: Bool
        public var reveal: Reveal
        /// Classes that are on.
        public var classes: Set<Class>
        /// Commands never concealed, by name without the backslash (`textbf`,
        /// `phi`); the non-command constructs by their source (```` `` ````,
        /// `''`, `--`, `---`, `^`, `_`).
        public var denied: Set<String>

        public static let defaultClasses: Set<Class> = [.greek, .symbols, .scripts, .fonts, .quotesDashes, .items, .comments]

        public init(enabled: Bool = false, reveal: Reveal = .line, classes: Set<Class> = Settings.defaultClasses, denied: Set<String> = []) {
            self.enabled = enabled; self.reveal = reveal; self.classes = classes; self.denied = denied
        }

        public static let `default` = Settings()

        /// Whether anything is concealed at all.
        public var concealsAnything: Bool { enabled && reveal != .never && !classes.subtracting([.comments]).isEmpty }
        /// Whether comments are drawn dimmed.
        public var dimsComments: Bool { enabled && classes.contains(.comments) }

        public func isOn(_ c: Class) -> Bool { enabled && classes.contains(c) }

        /// `denied` as the user types it: names separated by commas, spaces
        /// or newlines, a leading backslash optional.
        public static func parseDenyList(_ text: String) -> Set<String> {
            Set(text.split(whereSeparator: { $0 == "," || $0 == " " || $0 == "\n" || $0 == "\t" })
                .map { $0.hasPrefix("\\") && $0.count > 1 ? String($0.dropFirst()) : String($0) }
                .filter { !$0.isEmpty })
        }

        public static func formatDenyList(_ names: Set<String>) -> String {
            names.sorted().map { name in name.first?.isLetter == true ? "\\" + name : name }.joined(separator: ", ")
        }

        public func encoded() -> Data? { try? JSONEncoder().encode(self) }
        public static func decoded(_ data: Data) -> Settings? { try? JSONDecoder().decode(Settings.self, from: data) }

        enum CodingKeys: String, CodingKey { case enabled, reveal, classes, denied }
        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            enabled = try c.decodeIfPresent(Bool.self, forKey: .enabled) ?? false
            reveal = (try? c.decodeIfPresent(Reveal.self, forKey: .reveal)) ?? .line
            // Unknown class names (a newer app's) are dropped, not fatal.
            let raw = (try? c.decodeIfPresent([String].self, forKey: .classes)) ?? nil
            classes = raw.map { Set($0.compactMap(Class.init(rawValue:))) } ?? Settings.defaultClasses
            let names: [String]? = (try? c.decodeIfPresent([String].self, forKey: .denied)) ?? nil
            denied = Set(names ?? [])
        }
        public func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(enabled, forKey: .enabled)
            try c.encode(reveal, forKey: .reveal)
            try c.encode(classes.map(\.rawValue).sorted(), forKey: .classes)
            try c.encode(denied.sorted(), forKey: .denied)
        }
    }

    /// How replacement text or kept text is drawn.
    public enum Style: UInt8, Sendable {
        /// Text mode, the plain text colour (quotes, dashes, •).
        case text
        /// Math, the theme's conceal colour (α, ≤, ℝ).
        case math
        /// A raised, smaller script (non-Unicode superscript).
        case superscript
        /// A lowered, smaller script (non-Unicode subscript).
        case `subscript`
        case bold
        case italic
        /// A heading: bold kept text, a § marker.
        case heading
    }

    /// One thing done to part of a span.
    public enum Action: Equatable, Sendable {
        /// Show `text` in place of the range (drawn over one hidden glyph).
        case replace(String, Style)
        /// Hide the range.
        case hide
        /// Keep the range's own glyphs, drawn in a style (bold, italic).
        case style(Style)
    }

    public struct Piece: Equatable, Sendable {
        public var range: NSRange
        public var action: Action
        public init(_ range: NSRange, _ action: Action) { self.range = range; self.action = action }
    }

    /// One concealable construct: its whole source range (what the caret
    /// reveals in construct mode), its class, the name the deny list matches,
    /// and the pieces that draw it. Pieces are disjoint and sorted.
    public struct Span: Equatable, Sendable {
        public var range: NSRange
        public var cls: Class
        public var name: String
        public var pieces: [Piece]
        public init(range: NSRange, cls: Class, name: String, pieces: [Piece]) {
            self.range = range; self.cls = cls; self.name = name; self.pieces = pieces
        }
    }

    // MARK: tables

    public static let greek: [String: String] = [
        "alpha": "α", "beta": "β", "gamma": "γ", "delta": "δ", "epsilon": "ϵ", "varepsilon": "ε", "zeta": "ζ", "eta": "η",
        "theta": "θ", "vartheta": "ϑ", "iota": "ι", "kappa": "κ", "varkappa": "ϰ", "lambda": "λ", "mu": "μ", "nu": "ν",
        "xi": "ξ", "omicron": "ο", "pi": "π", "varpi": "ϖ", "rho": "ρ", "varrho": "ϱ", "sigma": "σ", "varsigma": "ς",
        "tau": "τ", "upsilon": "υ", "phi": "ϕ", "varphi": "φ", "chi": "χ", "psi": "ψ", "omega": "ω",
        "Gamma": "Γ", "Delta": "Δ", "Theta": "Θ", "Lambda": "Λ", "Xi": "Ξ", "Pi": "Π", "Sigma": "Σ", "Upsilon": "Υ",
        "Phi": "Φ", "Psi": "Ψ", "Omega": "Ω", "digamma": "ϝ",
        "varGamma": "𝛤", "varDelta": "𝛥", "varTheta": "𝛩", "varLambda": "𝛬", "varXi": "𝛯", "varPi": "𝛱",
        "varSigma": "𝛴", "varUpsilon": "𝛶", "varPhi": "𝛷", "varPsi": "𝛹", "varOmega": "𝛺",
    ]

    /// Math-mode symbols and operators.
    public static let mathSymbols: [String: String] = [
        // relations
        "leq": "≤", "le": "≤", "geq": "≥", "ge": "≥", "neq": "≠", "ne": "≠", "approx": "≈", "equiv": "≡", "sim": "∼",
        "simeq": "≃", "cong": "≅", "propto": "∝", "ll": "≪", "gg": "≫", "prec": "≺", "succ": "≻", "preceq": "⪯",
        "succeq": "⪰", "perp": "⊥", "parallel": "∥", "mid": "∣", "models": "⊨", "vdash": "⊢", "dashv": "⊣",
        "leqslant": "⩽", "geqslant": "⩾", "doteq": "≐", "asymp": "≍", "coloneqq": "≔",
        // sets and logic
        "in": "∈", "notin": "∉", "ni": "∋", "subset": "⊂", "supset": "⊃", "subseteq": "⊆", "supseteq": "⊇",
        "subsetneq": "⊊", "supsetneq": "⊋", "cup": "∪", "cap": "∩", "setminus": "∖", "emptyset": "∅",
        "varnothing": "∅", "forall": "∀", "exists": "∃", "nexists": "∄", "neg": "¬", "lnot": "¬", "land": "∧",
        "wedge": "∧", "lor": "∨", "vee": "∨", "top": "⊤", "bot": "⊥", "bigcup": "⋃", "bigcap": "⋂",
        "sqcup": "⊔", "sqcap": "⊓", "uplus": "⊎", "complement": "∁",
        // arrows
        "to": "→", "rightarrow": "→", "leftarrow": "←", "gets": "←", "leftrightarrow": "↔", "Rightarrow": "⇒",
        "Leftarrow": "⇐", "Leftrightarrow": "⇔", "implies": "⟹", "impliedby": "⟸", "iff": "⟺", "mapsto": "↦",
        "longrightarrow": "⟶", "longleftarrow": "⟵", "longmapsto": "⟼", "uparrow": "↑", "downarrow": "↓",
        "hookrightarrow": "↪", "hookleftarrow": "↩", "rightharpoonup": "⇀", "nearrow": "↗", "searrow": "↘",
        "twoheadrightarrow": "↠", "rightleftharpoons": "⇌",
        // operators
        "sum": "∑", "prod": "∏", "coprod": "∐", "int": "∫", "iint": "∬", "iiint": "∭", "oint": "∮", "partial": "∂",
        "nabla": "∇", "infty": "∞", "pm": "±", "mp": "∓", "times": "×", "div": "÷", "cdot": "⋅", "cdots": "⋯",
        "ldots": "…", "dots": "…", "vdots": "⋮", "ddots": "⋱", "circ": "∘", "bullet": "∙", "ast": "∗", "star": "⋆",
        "oplus": "⊕", "otimes": "⊗", "ominus": "⊖", "odot": "⊙", "bigoplus": "⨁", "bigotimes": "⨂", "sqrt": "√",
        "wr": "≀", "dagger": "†", "ddagger": "‡", "amalg": "⨿",
        // letters and misc
        "hbar": "ℏ", "ell": "ℓ", "Re": "ℜ", "Im": "ℑ", "aleph": "ℵ", "wp": "℘", "angle": "∠", "prime": "′",
        "degree": "°", "triangle": "△", "square": "□", "Box": "□", "diamond": "⋄", "lozenge": "◊", "checkmark": "✓",
        "langle": "⟨", "rangle": "⟩", "lceil": "⌈", "rceil": "⌉", "lfloor": "⌊", "rfloor": "⌋", "|": "‖",
        "Vert": "‖", "vert": "|", "backslash": "∖", "sharp": "♯", "flat": "♭", "natural": "♮", "clubsuit": "♣",
        "diamondsuit": "♢", "heartsuit": "♡", "spadesuit": "♠", "imath": "ı", "jmath": "ȷ",
    ]

    /// Symbols concealed in text mode too.
    public static let textSymbols: [String: String] = [
        "ldots": "…", "dots": "…", "textellipsis": "…", "S": "§", "P": "¶", "copyright": "©", "textregistered": "®",
        "texttrademark": "™", "textdegree": "°", "dag": "†", "ddag": "‡", "pounds": "£", "euro": "€", "textbullet": "•",
        "textendash": "–", "textemdash": "—", "textbackslash": "\\", "quad": " ",
    ]

    /// Commands whose one braced argument is kept and drawn in a style.
    static let styledArgument: [String: Style] = [
        "textbf": .bold, "mathbf": .bold, "boldsymbol": .bold, "bm": .bold, "bf": .bold,
        "textit": .italic, "emph": .italic, "mathit": .italic, "textsl": .italic,
        "texttt": .text, "mathtt": .text, "textrm": .text, "mathrm": .text, "textsf": .text, "mathsf": .text,
        "textup": .text, "textnormal": .text, "text": .text, "mbox": .text, "operatorname": .text,
    ]

    /// Letter alphabets with Unicode forms (`\mathbb{R}` → ℝ).
    static let alphabets: [String: (upper: UInt32, lower: UInt32?, exceptions: [Character: String])] = [
        "mathbb": (0x1D538, 0x1D552, ["C": "ℂ", "H": "ℍ", "N": "ℕ", "P": "ℙ", "Q": "ℚ", "R": "ℝ", "Z": "ℤ"]),
        "mathcal": (0x1D49C, nil, ["B": "ℬ", "E": "ℰ", "F": "ℱ", "H": "ℋ", "I": "ℐ", "L": "ℒ", "M": "ℳ", "R": "ℛ"]),
        "mathscr": (0x1D49C, nil, ["B": "ℬ", "E": "ℰ", "F": "ℱ", "H": "ℋ", "I": "ℐ", "L": "ℒ", "M": "ℳ", "R": "ℛ"]),
        "mathfrak": (0x1D504, 0x1D51E, ["C": "ℭ", "H": "ℌ", "I": "ℑ", "R": "ℜ", "Z": "ℨ"]),
    ]

    static let superscripts: [Character: Character] = [
        "0": "⁰", "1": "¹", "2": "²", "3": "³", "4": "⁴", "5": "⁵", "6": "⁶", "7": "⁷", "8": "⁸", "9": "⁹",
        "+": "⁺", "-": "⁻", "=": "⁼", "(": "⁽", ")": "⁾", "n": "ⁿ", "i": "ⁱ",
        "a": "ᵃ", "b": "ᵇ", "c": "ᶜ", "d": "ᵈ", "e": "ᵉ", "f": "ᶠ", "g": "ᵍ", "h": "ʰ", "j": "ʲ", "k": "ᵏ",
        "l": "ˡ", "m": "ᵐ", "o": "ᵒ", "p": "ᵖ", "r": "ʳ", "s": "ˢ", "t": "ᵗ", "u": "ᵘ", "v": "ᵛ", "w": "ʷ",
        "x": "ˣ", "y": "ʸ", "z": "ᶻ", "A": "ᴬ", "B": "ᴮ", "D": "ᴰ", "E": "ᴱ", "G": "ᴳ", "H": "ᴴ", "I": "ᴵ",
        "J": "ᴶ", "K": "ᴷ", "L": "ᴸ", "M": "ᴹ", "N": "ᴺ", "O": "ᴼ", "P": "ᴾ", "R": "ᴿ", "T": "ᵀ", "U": "ᵁ",
        "V": "ⱽ", "W": "ᵂ", "*": "*", "'": "′",
    ]

    static let subscripts: [Character: Character] = [
        "0": "₀", "1": "₁", "2": "₂", "3": "₃", "4": "₄", "5": "₅", "6": "₆", "7": "₇", "8": "₈", "9": "₉",
        "+": "₊", "-": "₋", "=": "₌", "(": "₍", ")": "₎", "a": "ₐ", "e": "ₑ", "h": "ₕ", "i": "ᵢ", "j": "ⱼ",
        "k": "ₖ", "l": "ₗ", "m": "ₘ", "n": "ₙ", "o": "ₒ", "p": "ₚ", "r": "ᵣ", "s": "ₛ", "t": "ₜ", "u": "ᵤ",
        "v": "ᵥ", "x": "ₓ",
    ]

    static let sectionLevels: [String: Int] = [
        "part": 0, "chapter": 0, "section": 1, "subsection": 2, "subsubsection": 3, "paragraph": 4, "subparagraph": 4,
    ]

    // MARK: spans

    /// Concealable spans of the line `lineRange` of `text`, given the syntax
    /// `runs` covering that line (`SyntaxHighlighter.runs(in:text:)` for it),
    /// with `settings`' classes and deny list applied. Reveal is not applied
    /// here. Spans are sorted by location and never cross the line.
    public static func spans(in text: NSString, lineRange: NSRange, runs: [SyntaxHighlighter.Run], settings: Settings) -> [Span] {
        guard settings.concealsAnything, lineRange.length > 0 else { return [] }
        var builder = Builder(text: text, line: lineRange, runs: runs, settings: settings)
        builder.run()
        return builder.spans.sorted { $0.range.location < $1.range.location }
    }

    private struct Builder {
        let text: NSString
        let line: NSRange
        let runs: [SyntaxHighlighter.Run]
        let settings: Settings
        var spans: [Span] = []
        /// Line end excluding the newline.
        let end: Int

        init(text: NSString, line: NSRange, runs: [SyntaxHighlighter.Run], settings: Settings) {
            self.text = text; self.line = line; self.runs = runs; self.settings = settings
            var e = NSMaxRange(line)
            while e > line.location, text.character(at: e - 1) == 0x0A || text.character(at: e - 1) == 0x0D { e -= 1 }
            end = e
        }

        /// The class is on and neither `name` nor its unstarred form is denied.
        func on(_ c: Class, _ name: String) -> Bool {
            guard settings.classes.contains(c), !settings.denied.contains(name) else { return false }
            return !(name.hasSuffix("*") && settings.denied.contains(String(name.dropLast())))
        }
        func char(_ i: Int) -> UInt16 { text.character(at: i) }

        mutating func add(_ range: NSRange, _ cls: Class, _ name: String, _ pieces: [Piece]) {
            spans.append(Span(range: range, cls: cls, name: name, pieces: pieces))
        }

        /// `{…}` starting at `open` (a `{`), matched on this line; the index of
        /// its `}` or nil.
        func group(at open: Int) -> Int? {
            guard open < end, char(open) == 0x7B else { return nil }
            var depth = 0
            var i = open
            while i < end {
                let c = char(i)
                if c == 0x5C { i += 2; continue }
                if c == 0x7B { depth += 1 }
                if c == 0x7D { depth -= 1; if depth == 0 { return i } }
                i += 1
            }
            return nil
        }

        /// The one-token argument after a control word at `after`: a braced
        /// group (`\mathbb{R}`), or TeX's undelimited single token
        /// (`\mathbb R`, `\mathbf v_1`, `\mathbb 1`), with the spaces before it.
        /// `open` is what precedes the content (the command and `{`, or the
        /// command and its spaces); `close` is the `}` or nil when unbraced.
        func argument(after: Int, start: Int) -> (open: NSRange, content: NSRange, close: NSRange?)? {
            if let close = group(at: after) {
                return (NSRange(location: start, length: after + 1 - start),
                        NSRange(location: after + 1, length: close - after - 1),
                        NSRange(location: close, length: 1))
            }
            var i = after
            while i < end, char(i) == 0x20 || char(i) == 0x09 { i += 1 }
            guard i < end else { return nil }
            let c = char(i)
            let isAlnum = (c >= 0x30 && c <= 0x39) || (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A)
            guard isAlnum else { return nil } // `\mathbf\alpha`, `\bf}`, `^`, `$`…: not concealed
            return (NSRange(location: start, length: i - start), NSRange(location: i, length: 1), nil)
        }

        mutating func run() {
            var covered: [NSRange] = [] // run ranges, for the text-mode scan
            var index = 0
            while index < runs.count {
                let run = runs[index]
                defer { index += 1 }
                covered.append(run.range)
                switch run.kind {
                case .command, .mathCommand:
                    command(run, math: run.kind == .mathCommand)
                case .math:
                    scripts(in: run.range)
                default:
                    break
                }
            }
            textMode(outside: covered)
        }

        // MARK: commands

        mutating func command(_ run: SyntaxHighlighter.Run, math: Bool) {
            let r = run.range
            guard r.length >= 2, char(r.location) == 0x5C else { return }
            let name = text.substring(with: NSRange(location: r.location + 1, length: r.length - 1))
            let after = NSMaxRange(r)
            // A control word followed by a letter can't happen (the lexer took
            // them all); `\alpha2` is fine, `\alpha` then `b` is fine.
            if math, let g = greek[name] {
                guard on(.greek, name) else { return }
                add(r, .greek, name, [Piece(r, .replace(g, .math))])
                return
            }
            if math, let s = mathSymbols[name] {
                guard on(.symbols, name) else { return }
                add(r, .symbols, name, [Piece(r, .replace(s, .math))])
                return
            }
            if !math, let s = textSymbols[name] {
                guard on(.symbols, name) else { return }
                add(r, .symbols, name, [Piece(r, .replace(s, .text))])
                return
            }
            if name == "item" {
                guard !math, on(.items, name) else { return }
                // `\item` and one following space become "• ".
                var hidden = r
                if after < end, char(after) == 0x20 { hidden.length += 1 }
                add(hidden, .items, name, [Piece(hidden, .replace(hidden.length > r.length ? "• " : "•", .text))])
                return
            }
            if let level = sectionLevels[name.hasSuffix("*") ? String(name.dropLast()) : name] {
                guard !math, on(.sectioning, name), let close = group(at: after) else { return }
                let marker = String(repeating: "§", count: max(1, level)) + " "
                let open = NSRange(location: r.location, length: after + 1 - r.location)
                add(NSRange(location: r.location, length: close + 1 - r.location), .sectioning, name, [
                    Piece(open, .replace(marker, .heading)),
                    Piece(NSRange(location: after + 1, length: close - after - 1), .style(.heading)),
                    Piece(NSRange(location: close, length: 1), .hide),
                ])
                return
            }
            if let alphabet = alphabets[name] {
                guard math, on(.fonts, name), let arg = argument(after: after, start: r.location), arg.content.length >= 1 else { return }
                let content = text.substring(with: arg.content)
                guard let mapped = Self.mapAlphabet(content, alphabet) else { return }
                let whole = NSRange(location: r.location, length: NSMaxRange(arg.close ?? arg.content) - r.location)
                add(whole, .fonts, name, [Piece(whole, .replace(mapped, .math))])
                return
            }
            if let style = styledArgument[name] {
                // `\bf` is a declaration (`{\bf x}`): only its braced form is an argument.
                guard on(.fonts, name), let arg = name == "bf"
                        ? group(at: after).map({ (NSRange(location: r.location, length: after + 1 - r.location), NSRange(location: after + 1, length: $0 - after - 1), Optional(NSRange(location: $0, length: 1))) })
                        : argument(after: after, start: r.location),
                      arg.content.length > 0 else { return }
                var pieces = [Piece(arg.open, .hide)]
                if style == .bold || style == .italic {
                    pieces.append(Piece(arg.content, .style(style)))
                }
                if let close = arg.close { pieces.append(Piece(close, .hide)) }
                add(NSRange(location: r.location, length: NSMaxRange(arg.close ?? arg.content) - r.location), .fonts, name, pieces)
                return
            }
            if math, name == "frac" || name == "dfrac" || name == "tfrac" || name == "cfrac" {
                guard on(.fractions, name), let c1 = group(at: after), let c2 = group(at: c1 + 1), c1 > after + 1, c2 > c1 + 2 else { return }
                add(NSRange(location: r.location, length: c2 + 1 - r.location), .fractions, name, [
                    Piece(NSRange(location: r.location, length: after + 1 - r.location), .hide),
                    Piece(NSRange(location: c1, length: 2), .replace("⁄", .math)),
                    Piece(NSRange(location: c2, length: 1), .hide),
                ])
            }
        }

        static func mapAlphabet(_ content: String, _ a: (upper: UInt32, lower: UInt32?, exceptions: [Character: String])) -> String? {
            var out = ""
            for ch in content {
                if let e = a.exceptions[ch] { out += e; continue }
                guard let ascii = ch.asciiValue else { return nil }
                if ascii >= 0x41, ascii <= 0x5A { out.unicodeScalars.append(Unicode.Scalar(a.upper + UInt32(ascii - 0x41))!); continue }
                if ascii >= 0x61, ascii <= 0x7A, let lower = a.lower { out.unicodeScalars.append(Unicode.Scalar(lower + UInt32(ascii - 0x61))!); continue }
                if a.upper == 0x1D538, ascii >= 0x30, ascii <= 0x39 { out.unicodeScalars.append(Unicode.Scalar(0x1D7D8 + UInt32(ascii - 0x30))!); continue } // 𝟘…𝟡
                return nil
            }
            return out.isEmpty ? nil : out
        }

        // MARK: scripts (inside math runs)

        mutating func scripts(in r: NSRange) {
            var i = r.location
            let stop = min(NSMaxRange(r), end)
            while i < stop {
                let c = char(i)
                guard c == 0x5E || c == 0x5F else { i += 1; continue } // ^ _
                let sup = c == 0x5E
                let name = sup ? "^" : "_"
                guard on(.scripts, name), i + 1 < end else { i += 1; continue }
                let next = char(i + 1)
                var content: NSRange
                var whole: NSRange
                if next == 0x7B, let close = group(at: i + 1) { // ^{…}
                    content = NSRange(location: i + 2, length: close - i - 2)
                    whole = NSRange(location: i, length: close + 1 - i)
                } else if next != 0x5C, next != 0x20, next != 0x7D, next != 0x0A, next < 0x80 { // ^2, _i
                    content = NSRange(location: i + 1, length: 1)
                    whole = NSRange(location: i, length: 2)
                } else { i += 1; continue }
                guard content.length > 0, content.length <= 12 else { i = NSMaxRange(whole); continue }
                let s = text.substring(with: content)
                let table = sup ? superscripts : subscripts
                if s.allSatisfy({ table[$0] != nil }) {
                    add(whole, .scripts, name, [Piece(whole, .replace(String(s.map { table[$0]! }), .math))])
                } else if s.allSatisfy({ $0.isLetter || $0.isNumber || "+-=*,.'()".contains($0) }), s.unicodeScalars.allSatisfy(\.isASCII) {
                    add(whole, .scripts, name, [Piece(whole, .replace(s, sup ? .superscript : .subscript))])
                }
                i = NSMaxRange(whole)
            }
        }

        // MARK: text mode (characters no run covers)

        mutating func textMode(outside covered: [NSRange]) {
            guard settings.classes.contains(.quotesDashes) else { return }
            var cursor = line.location
            var gaps: [NSRange] = []
            for r in covered.sorted(by: { $0.location < $1.location }) {
                if r.location > cursor { gaps.append(NSRange(location: cursor, length: r.location - cursor)) }
                cursor = max(cursor, NSMaxRange(r))
            }
            if cursor < end { gaps.append(NSRange(location: cursor, length: end - cursor)) }
            for gap in gaps {
                var i = gap.location
                let stop = min(NSMaxRange(gap), end)
                while i < stop {
                    let c = char(i)
                    if c == 0x2D, i + 1 < stop, char(i + 1) == 0x2D { // -- / ---
                        let triple = i + 2 < stop && char(i + 2) == 0x2D
                        let r = NSRange(location: i, length: triple ? 3 : 2)
                        let name = triple ? "---" : "--"
                        if on(.quotesDashes, name) { add(r, .quotesDashes, name, [Piece(r, .replace(triple ? "—" : "–", .text))]) }
                        i = NSMaxRange(r)
                        while i < stop, char(i) == 0x2D { i += 1 } // ---- stays as typed after the first three
                        continue
                    }
                    if c == 0x60, i + 1 < stop, char(i + 1) == 0x60 { // ``
                        let r = NSRange(location: i, length: 2)
                        if on(.quotesDashes, "``") { add(r, .quotesDashes, "``", [Piece(r, .replace("“", .text))]) }
                        i += 2; continue
                    }
                    if c == 0x27, i + 1 < stop, char(i + 1) == 0x27 { // ''
                        let r = NSRange(location: i, length: 2)
                        if on(.quotesDashes, "''") { add(r, .quotesDashes, "''", [Piece(r, .replace("”", .text))]) }
                        i += 2; continue
                    }
                    i += 1
                }
            }
        }
    }

    // MARK: reveal

    /// Whether `span` shows its source for a selection, in `reveal` mode.
    /// `revealedLines` are the line ranges the caret or selection touches
    /// (line mode); `selection` is the selection itself (construct mode: a
    /// caret touching either end of the span, or a selection overlapping it).
    public static func isRevealed(_ span: Span, reveal: Reveal, selection: NSRange, revealedLines: [NSRange]) -> Bool {
        switch reveal {
        case .never: return true
        case .line:
            return revealedLines.contains { NSLocationInRange(span.range.location, $0) }
        case .construct:
            if selection.length == 0 {
                return selection.location >= span.range.location && selection.location <= NSMaxRange(span.range)
            }
            return NSIntersectionRange(selection, span.range).length > 0
                || (selection.location <= span.range.location && NSMaxRange(selection) >= NSMaxRange(span.range))
        }
    }
}
