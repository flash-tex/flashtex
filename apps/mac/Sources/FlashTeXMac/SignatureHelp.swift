import AppKit

/// Signature help: while the caret sits inside an argument of `\cmd` — or of a
/// `\begin{env}` — a small non-activating panel shows the argument pattern with
/// the current argument emphasised, plus a line saying what *that* argument is
/// for. The model (`info`) is pure and bounded to the caret's line; the panel is
/// opened by `CompletingTextView` after a `{` or `[` that follows a command
/// name, or on ⌘⇧Space, and dismissed by `}`, Esc or the caret leaving the
/// argument.
///
/// Three things it gets right that a naive "count the groups" version does not:
///
/// - **Optional arguments.** `\includegraphics`'s pattern is `[keys]{file}`.
///   In `\includegraphics{|}` the caret is in `{file}`, not `[keys]`: the
///   active group is matched by bracket *kind*, skipping optionals the user
///   left out, so the emphasis never points one argument too far left.
/// - **Per-argument help.** `argumentHints` says what each group means
///   (`\rule`'s two `{dimension}`s are the width and the height), keyed by
///   command and checked against the compiler's own patterns by
///   `testArgumentHintsMatchTheInventoryPatterns` — if the inventory changes
///   a shape, that gate fails instead of the panel quietly mislabelling it.
/// - **Environments.** `\begin{tabular}{|l|cc|}` is where the column spec is
///   typed and where people most need reminding what `p{width}` and `|` do.
///   The compiler's inventory carries no argument shapes for environments, so
///   `environmentSignatures` supplies them.
enum SignatureHelp {
    enum Kind: Equatable {
        /// `\frac{num}{den}` — a control word's own arguments.
        case command
        /// `\begin{tabular}[pos]{cols}` — the environment's arguments, after
        /// the `\begin{…}` group that names it.
        case environment
    }

    struct Info: Equatable {
        /// Command name without the backslash, or the environment name.
        let command: String
        var kind: Kind = .command
        /// Argument pattern, e.g. `{num}{den}` or `[options]{class}`; empty when unknown.
        let arguments: String
        /// 0-based index into `groups` of the group the caret is in. Equal to
        /// `groups.count` when the caret is past the pattern's last group.
        let activeArgument: Int
        /// One-line documentation of the command or environment.
        let description: String
        /// What the active argument itself is for; nil when nothing is known
        /// about that group.
        var argumentHint: String? = nil
        /// UTF-16 index of the unmatched opener the caret is inside; the help
        /// is dismissed once the caret is at or before it.
        let openerUTF16: Int

        /// Argument groups of `arguments` in order, e.g. `["{num}", "{den}"]`.
        var groups: [String] { SignatureHelp.groups(of: arguments) }

        /// What precedes the pattern: `\frac`, or `\begin{tabular}`.
        var head: String { kind == .environment ? "\\begin{" + command + "}" : "\\" + command }

        /// The line under the pattern: the active argument's own help when
        /// there is one, else the command's documentation.
        var detail: String { argumentHint ?? description }

        /// Display text `\frac{num}{den}` (or `\cmd` when the pattern is
        /// unknown) and the UTF-16 range of the active group inside it (nil
        /// when the caret is past the pattern's last group).
        var display: (text: String, active: NSRange?) {
            var text = head
            var active: NSRange?
            for (i, g) in groups.enumerated() {
                if i == activeArgument { active = NSRange(location: (text as NSString).length, length: (g as NSString).length) }
                text += g
            }
            return (text, active)
        }
    }

    /// Scan limit backwards from the caret (bytes of the caret's line).
    static let scanLimit = 2048

    static func groups(of arguments: String) -> [String] {
        var out: [String] = []
        var current = ""
        var depth = 0
        for c in arguments {
            if c == "{" || c == "[" { if depth == 0 { current = "" }; depth += 1 }
            current.append(c)
            if c == "}" || c == "]" { depth -= 1; if depth == 0 { out.append(current); current = "" } }
        }
        return out
    }

    /// Index into `groups` of the group the caret's opener corresponds to,
    /// given the bracket kinds of every group the user has actually written up
    /// to and including the caret's own (`"["` or `"{"`).
    ///
    /// The match is by kind, walking the pattern forward and skipping optional
    /// `[…]` groups the user left out, so `\includegraphics{|}` lands on
    /// `{file}` and `\includegraphics[width=2cm]{|}` lands there too. Returns
    /// `groups.count` when the written arguments run past the pattern.
    static func activeIndex(groups: [String], written: [Character]) -> Int {
        guard !written.isEmpty else { return 0 }
        var pattern = 0
        for (n, kind) in written.enumerated() {
            let isLast = n == written.count - 1
            // Skip pattern optionals the user did not write.
            while pattern < groups.count, groups[pattern].first == "[", kind == "{" { pattern += 1 }
            guard pattern < groups.count else { return groups.count }
            // A written `[` where the pattern wants `{` is an optional the
            // pattern does not list (`\begin{figure}[htbp]` on a command shape):
            // it consumes no pattern group.
            if groups[pattern].first != kind {
                if isLast { return groups.count }
                continue
            }
            if isLast { return pattern }
            pattern += 1
        }
        return groups.count
    }

    /// One written argument group before the caret: its bracket kind and, for
    /// the `\begin{…}` case, its text.
    private struct Written {
        var kind: Character
        var text: String
    }

    /// The command or environment whose argument the caret is in, or nil when
    /// the caret is not inside an unmatched `{`/`[` on its line, the group is
    /// not an argument of a control word, or nothing is known about it.
    static func info(in text: String, caretUTF16: Int) -> Info? {
        let ns = text as NSString
        guard caretUTF16 >= 0, caretUTF16 <= ns.length else { return nil }
        var lineStart = caretUTF16
        while lineStart > 0, caretUTF16 - lineStart < scanLimit, ns.character(at: lineStart - 1) != 0x0A { lineStart -= 1 }
        func isEscaped(_ i: Int) -> Bool {
            var n = 0
            var j = i - 1
            while j >= lineStart, ns.character(at: j) == 0x5C { n += 1; j -= 1 } // `\`
            return n % 2 == 1
        }
        // 1. The unmatched opener before the caret.
        var depth = 0
        var opener: Int?
        var i = caretUTF16 - 1
        while i >= lineStart {
            let c = ns.character(at: i)
            if c == 0x25, !isEscaped(i) { return nil } // `%`: the caret is in a comment
            if (c == 0x7D || c == 0x5D), !isEscaped(i) { depth += 1 } // `}` `]`
            else if (c == 0x7B || c == 0x5B), !isEscaped(i) { // `{` `[`
                if depth == 0 { opener = i; break }
                depth -= 1
            }
            i -= 1
        }
        guard let opener else { return nil }
        for k in lineStart..<opener where ns.character(at: k) == 0x25 && !isEscaped(k) { return nil } // commented out
        // 2. Closed argument groups between the command name and this opener,
        //    innermost last, with their bracket kinds and text.
        var closed: [Written] = []
        var j = opener - 1
        while true {
            while j >= lineStart, ns.character(at: j) == 0x20 || ns.character(at: j) == 0x09 { j -= 1 }
            guard j >= lineStart, ns.character(at: j) == 0x7D || ns.character(at: j) == 0x5D, !isEscaped(j) else { break }
            var d = 0
            var k = j
            var matched = false
            while k >= lineStart {
                let c = ns.character(at: k)
                if (c == 0x7D || c == 0x5D), !isEscaped(k) { d += 1 }
                else if (c == 0x7B || c == 0x5B), !isEscaped(k) { d -= 1; if d == 0 { matched = true; break } }
                k -= 1
            }
            guard matched else { return nil }
            closed.insert(Written(kind: ns.character(at: k) == 0x7B ? "{" : "[",
                                  text: ns.substring(with: NSRange(location: k + 1, length: j - k - 1))), at: 0)
            j = k - 1
        }
        // 3. The control word: letters ending at j, preceded by `\`. A trailing
        //    `*` belongs to the name's starred form, not to a group.
        if j >= lineStart, ns.character(at: j) == 0x2A { j -= 1 }
        let nameEnd = j + 1
        var nameStart = nameEnd
        while nameStart > lineStart {
            let c = ns.character(at: nameStart - 1)
            guard (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) else { break }
            nameStart -= 1
        }
        guard nameStart < nameEnd, nameStart > lineStart, ns.character(at: nameStart - 1) == 0x5C, !isEscaped(nameStart - 1) else { return nil }
        let name = ns.substring(with: NSRange(location: nameStart, length: nameEnd - nameStart))
        let openerKind: Character = ns.character(at: opener) == 0x7B ? "{" : "["
        // 4. `\begin{env}…` with the environment already named: the groups after
        //    it are the environment's own arguments.
        if name == "begin", let first = closed.first, first.kind == "{" {
            let env = first.text.trimmingCharacters(in: .whitespaces)
            if let signature = environmentSignature(for: env) {
                let written = closed.dropFirst().map(\.kind) + [openerKind]
                let groups = groups(of: signature.arguments)
                let active = activeIndex(groups: groups, written: Array(written))
                return Info(command: env, kind: .environment, arguments: signature.arguments, activeArgument: active,
                            description: signature.description, argumentHint: signature.hints.indices.contains(active) ? signature.hints[active] : nil,
                            openerUTF16: opener)
            }
        }
        let entry = Completion.Vocabulary.byName[name]
        let doc = EditorIntelligence.CommandDocs.documentation(for: name) ?? entry?.description
        guard entry != nil || doc != nil else { return nil }
        let arguments = entry?.arguments ?? ""
        let groups = groups(of: arguments)
        let active = activeIndex(groups: groups, written: closed.map(\.kind) + [openerKind])
        let hints = argumentHints[name]
        return Info(command: name, kind: .command, arguments: arguments, activeArgument: active,
                    description: doc ?? entry?.description ?? "",
                    argumentHint: hints?.indices.contains(active) == true ? hints?[active] : nil,
                    openerUTF16: opener)
    }

    // MARK: - per-argument help

    /// What each argument group of a command is for, in the order of the
    /// compiler inventory's own pattern for that command
    /// (`Completion.Vocabulary.byName[name].arguments`). Checked against those
    /// patterns by `SignatureHelpTests.testArgumentHintsMatchTheInventoryPatterns`,
    /// so a command whose shape changes fails the suite instead of silently
    /// labelling the wrong group. Commands with one self-explanatory argument
    /// (`\label{key}`, `\hspace{dimension}`) are deliberately absent: the
    /// pattern already says it, and the doc line is better used for the
    /// command's own description.
    static let argumentHints: [String: [String]] = [
        // `{num}{den}`
        "frac": ["the numerator, above the rule", "the denominator, below the rule"],
        "dfrac": ["the numerator (display size)", "the denominator (display size)"],
        "tfrac": ["the numerator (text size)", "the denominator (text size)"],
        // `{n}{k}`
        "binom": ["the top value, n", "the bottom value, k"],
        // `[index]{x}`
        "sqrt": ["the root index — 3 for a cube root; leave it out for a square root", "what to take the root of"],
        // `*[keys]{file}`
        "includegraphics": ["comma-separated keys: width=, height=, scale=, angle=, trim=, clip, keepaspectratio",
                            "the image, without an extension — graphicx picks .pdf, .png, .jpg, .eps in that order"],
        // `[options]{class}`
        "documentclass": ["class options: a paper size (a4paper), a font size (11pt), twocolumn, twoside, draft",
                          "the class: article, report, book, letter, beamer"],
        // `[options]{a,b,c}`
        "usepackage": ["options for this package, comma separated", "package names, comma separated"],
        // `{\name}[n]{body}`
        "newcommand": ["the new command, with its backslash", "how many arguments it takes, 0 to 9", "the body; #1 … #n are the arguments"],
        "renewcommand": ["the command to redefine, with its backslash", "how many arguments it takes, 0 to 9",
                         "the new body; #1 … #n are the arguments"],
        "providecommand": ["the command to define if it does not exist", "how many arguments it takes, 0 to 9",
                           "default for the first argument, making it optional", "the body"],
        // `{env}[n][default]{begin}{end}`
        "newenvironment": ["the environment name, without braces", "how many arguments it takes, 0 to 9",
                           "default for the first argument, making it optional", "what \\begin{env} expands to", "what \\end{env} expands to"],
        // `{env}[counter]{name}`
        "newtheorem": ["the environment name, e.g. theorem", "an existing counter to share numbering with",
                       "the printed heading, e.g. Theorem"],
        // `[raise]{dimension}{dimension}`
        "rule": ["how far above the baseline to raise it", "the width", "the height"],
        // `{url}{text}`
        "href": ["the target URL", "the text the reader clicks"],
        // `{\length}{dimension}`
        "setlength": ["the length to set, with its backslash — \\parindent, \\parskip, \\textwidth",
                      "the new value with a unit: pt, mm, cm, in, em, ex"],
        // `[note]{keys}`
        "cite": ["a note printed after the citation, e.g. p. 42", "bibliography keys, comma separated"],
        // `*{\name}{text}`
        "DeclareMathOperator": ["the operator command to define, with its backslash",
                                "how it is printed, upright — the starred form takes limits like \\sum"],
    ]

    // MARK: - environments

    /// An environment's own arguments, which the compiler's inventory does not
    /// carry (it records environments by name, mode and description only).
    struct EnvironmentSignature: Equatable {
        /// Pattern in the same shape as a command's, e.g. `[pos]{cols}`.
        var arguments: String
        /// One hint per group of `arguments`.
        var hints: [String]
        /// One line about the environment itself.
        var description: String
    }

    static let columnSpecHint = "column spec, one letter per column: l c r align, p{width} wraps, "
        + "| draws a rule, @{…} sets what goes between — e.g. {|l|cc|}"

    static let floatPlacementHint = "where the float may go: h here, t top of a page, b bottom, p a page of floats, "
        + "! ignore the size limits — e.g. [htbp]"

    static func environmentSignature(for name: String) -> EnvironmentSignature? {
        environmentSignatures[name] ?? environmentSignatures[name.hasSuffix("*") ? String(name.dropLast()) : name]
    }

    static let environmentSignatures: [String: EnvironmentSignature] = [
        "tabular": .init(arguments: "[pos]{cols}", hints: ["vertical alignment against the surrounding line: t, b, or c (the default)", columnSpecHint],
                         description: "Table body: cells separated by &, rows ended by \\\\."),
        "tabularx": .init(arguments: "{width}{cols}", hints: ["the total width the table is stretched to", columnSpecHint + "; X columns share the slack"],
                          description: "Table stretched to a given width (tabularx)."),
        "longtable": .init(arguments: "[pos]{cols}", hints: ["vertical alignment: t, b or c", columnSpecHint],
                           description: "A table that may break across pages (longtable)."),
        "array": .init(arguments: "[pos]{cols}", hints: ["vertical alignment: t, b or c", columnSpecHint],
                       description: "Math-mode table with a column spec, like tabular."),
        "figure": .init(arguments: "[placement]", hints: [floatPlacementHint],
                        description: "Floating figure; use \\centering, \\includegraphics, \\caption and \\label inside."),
        "table": .init(arguments: "[placement]", hints: [floatPlacementHint],
                       description: "Floating table; wrap a tabular in it with \\caption and \\label."),
        "algorithm": .init(arguments: "[placement]", hints: [floatPlacementHint], description: "A floating algorithm."),
        "wrapfigure": .init(arguments: "{placement}{width}",
                            hints: ["which side the text wraps around: r or l (R or L to allow floating)", "the width reserved for the figure"],
                            description: "A figure the paragraph text flows around (wrapfig)."),
        "minipage": .init(arguments: "[pos][height][inner-pos]{width}",
                          hints: ["how the box lines up with the surrounding line: t, b or c",
                                  "a fixed height for the box", "how the content sits inside that height: t, b, c or s",
                                  "the box's width, e.g. 0.48\\textwidth"],
                          description: "A box of the given width in which paragraphs are typeset."),
        "thebibliography": .init(arguments: "{widest-label}",
                                 hints: ["the widest entry label, which sets the indent — 9 for under ten entries, 99 for under a hundred"],
                                 description: "Hand-written bibliography of \\bibitem entries."),
        "lstlisting": .init(arguments: "[options]",
                            hints: ["listings keys: language=, caption=, label=, firstline=, numbers=left"],
                            description: "Source-code listing (listings package)."),
        "minted": .init(arguments: "[options]{language}",
                        hints: ["minted keys: linenos, fontsize=, bgcolor=, highlightlines=", "the language to highlight, e.g. python"],
                        description: "Source-code listing highlighted by Pygments (minted package)."),
        "tikzpicture": .init(arguments: "[options]",
                             hints: ["TikZ keys applied to the whole picture: scale=, every node/.style=, x=, y="],
                             description: "A TikZ drawing."),
        "itemize": .init(arguments: "[options]", hints: ["enumitem keys: label=, itemsep=, topsep=, leftmargin=, nosep"],
                         description: "Bulleted list of \\item entries."),
        "enumerate": .init(arguments: "[options]", hints: ["enumitem keys: label=\\alph*), start=, itemsep=, nosep"],
                           description: "Numbered list of \\item entries."),
        "description": .init(arguments: "[options]", hints: ["enumitem keys: style=, labelwidth=, font="],
                             description: "List of \\item[term] entries."),
        "multicols": .init(arguments: "{columns}", hints: ["how many columns, 2 to 10"], description: "Balanced multi-column text (multicol)."),
        "frame": .init(arguments: "[options]{title}", hints: ["Beamer frame options: fragile, plain, allowframebreaks, t", "the slide title"],
                       description: "One Beamer slide."),
        "subfigure": .init(arguments: "[pos]{width}", hints: ["vertical alignment: t, b or c", "the sub-figure's width, e.g. 0.48\\textwidth"],
                           description: "One panel of a figure (subcaption)."),
        "alignat": .init(arguments: "{pairs}", hints: ["how many column pairs the alignment has"],
                         description: "Aligned equations with a chosen number of alignment points."),
    ]
}

/// The signature-help panel: one line for the pattern, then the active
/// argument's own help — which is a sentence, not a label, so it wraps to at
/// most `docLines` lines and the panel sizes itself to what it needs.
@MainActor
final class SignatureHelpPanel: NSPanel {
    static let width: CGFloat = 560
    /// Height with a one-line doc; `height(for:)` grows it for a wrapped one.
    static let height: CGFloat = 44
    static let docLines = 3
    private let pattern = NSTextField(labelWithString: "")
    private let doc = NSTextField(labelWithString: "")
    private(set) var info: SignatureHelp.Info?

    init() {
        super.init(contentRect: NSRect(x: 0, y: 0, width: Self.width, height: Self.height),
                   styleMask: [.nonactivatingPanel, .borderless], backing: .buffered, defer: false)
        isFloatingPanel = true
        hidesOnDeactivate = false
        level = .popUpMenu
        hasShadow = true
        isReleasedWhenClosed = false
        isExcludedFromWindowsMenu = true
        animationBehavior = .none
        let content = NSVisualEffectView(frame: NSRect(x: 0, y: 0, width: Self.width, height: Self.height))
        content.material = .popover
        content.state = .active
        content.wantsLayer = true
        content.layer?.cornerRadius = 6
        pattern.font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
        pattern.lineBreakMode = .byTruncatingTail
        pattern.frame = NSRect(x: 10, y: Self.height - 22, width: Self.width - 20, height: 17)
        pattern.autoresizingMask = [.width]
        pattern.setAccessibilityIdentifier("signature-help-pattern")
        content.addSubview(pattern)
        doc.font = NSFont.systemFont(ofSize: 11)
        doc.textColor = .secondaryLabelColor
        doc.lineBreakMode = .byWordWrapping
        doc.usesSingleLineMode = false
        doc.maximumNumberOfLines = Self.docLines
        doc.cell?.wraps = true
        doc.frame = NSRect(x: 10, y: 4, width: Self.width - 20, height: 15)
        doc.autoresizingMask = [.width]
        doc.setAccessibilityIdentifier("signature-help-doc")
        content.addSubview(doc)
        contentView = content
        setAccessibilityLabel("Signature help")
    }

    /// Attributed pattern: the command bold, the active argument in the accent colour.
    static func attributed(_ info: SignatureHelp.Info) -> NSAttributedString {
        let (text, active) = info.display
        let s = NSMutableAttributedString(string: text, attributes: [
            .font: NSFont.monospacedSystemFont(ofSize: 12, weight: .regular), .foregroundColor: NSColor.labelColor,
        ])
        s.addAttribute(.font, value: NSFont.monospacedSystemFont(ofSize: 12, weight: .bold),
                       range: NSRange(location: 0, length: min((info.head as NSString).length, (text as NSString).length)))
        if let active {
            s.addAttributes([.font: NSFont.monospacedSystemFont(ofSize: 12, weight: .bold),
                             .foregroundColor: NSColor.controlAccentColor,
                             .underlineStyle: NSUnderlineStyle.single.rawValue], range: active)
        }
        return s
    }

    var patternText: String { pattern.stringValue }
    var docText: String { doc.stringValue }

    /// Panel height for `text` in the doc line: the wrapped height, capped at
    /// `docLines`, plus the pattern line and the padding.
    static func height(for text: String) -> CGFloat {
        let font = NSFont.systemFont(ofSize: 11)
        let bounding = (text as NSString).boundingRect(
            with: NSSize(width: width - 20, height: .greatestFiniteMagnitude),
            options: [.usesLineFragmentOrigin, .usesFontLeading], attributes: [.font: font])
        let lineHeight = ceil(font.ascender - font.descender + font.leading)
        let lines = min(CGFloat(docLines), max(1, ceil(bounding.height / max(lineHeight, 1))))
        return 29 + lines * lineHeight
    }

    func show(_ info: SignatureHelp.Info, above caretRect: NSRect, parent: NSWindow) {
        self.info = info
        pattern.attributedStringValue = Self.attributed(info)
        doc.stringValue = info.detail
        let height = Self.height(for: info.detail)
        doc.frame = NSRect(x: 10, y: 4, width: Self.width - 20, height: height - 27)
        pattern.frame = NSRect(x: 10, y: height - 22, width: Self.width - 20, height: 17)
        var origin = NSPoint(x: caretRect.minX - 6, y: caretRect.maxY + 4)
        if let screen = parent.screen ?? NSScreen.main {
            let visible = screen.visibleFrame
            if origin.y + height > visible.maxY { origin.y = caretRect.minY - height - 4 } // flip below
            origin.x = min(max(origin.x, visible.minX), max(visible.minX, visible.maxX - Self.width))
        }
        let target = NSRect(origin: origin, size: NSSize(width: Self.width, height: height))
        if frame != target { setFrame(target, display: false) }
        if self.parent !== parent {
            self.parent?.removeChildWindow(self)
            parent.addChildWindow(self, ordered: .above)
        }
        if !isVisible { orderFront(nil) }
    }

    func hide() {
        info = nil
        if parent != nil { parent?.removeChildWindow(self) }
        if isVisible { orderOut(nil) }
    }
}
