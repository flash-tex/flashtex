import AppKit
import SwiftUI

/// Settings > Themes (lane EDITOR-THEMES): the code theme, the editor chrome
/// colours on top of it, and the display switches, with a live preview drawn
/// from the same lexer and theme the editor uses. Applies live, like every
/// other Settings pane (no OK/Apply). Font family and size stay in the
/// Editor pane; this pane previews them.
struct EditorThemeSettingsView: View {
    @Bindable private var prefs: EditorPreferences
    @Bindable private var library: EditorThemeLibrary
    /// Which appearance's colours the preview shows and the pickers edit.
    @State private var variant: EditorColorTheme.Variant
    @State private var message: String?
    @State private var confirmDelete = false

    @MainActor init() {
        self.init(preferences: .shared, library: .shared)
    }

    init(preferences: EditorPreferences, library: EditorThemeLibrary) {
        prefs = preferences
        self.library = library
        let dark: Bool
        switch preferences.appearance {
        case .system: dark = EditorPreferences.systemAppearanceIsDark()
        case .light: dark = false
        case .dark: dark = true
        }
        _variant = State(initialValue: dark ? .dark : .light)
    }

    private var selectedTheme: EditorColorTheme { EditorPreferences.theme(id: prefs.codeThemeID, library: library) }
    private var effectiveTheme: EditorColorTheme { selectedTheme.applying(prefs.themeOverrides).resolved() }

    var body: some View {
        Form {
            Section("Code theme") {
                Picker("Theme", selection: $prefs.codeThemeID) {
                    ForEach(EditorColorTheme.builtIns) { Text($0.name).tag($0.id) }
                    if !library.userThemes.isEmpty {
                        Divider()
                        ForEach(library.userThemes) { Text($0.name).tag($0.id) }
                    }
                    if library.theme(id: prefs.codeThemeID) == nil {
                        Text("Missing theme (\(prefs.codeThemeID))").tag(prefs.codeThemeID)
                    }
                }
                .accessibilityLabel("Code theme")
                .accessibilityHint("Colours of commands, math, comments and the rest of the source, and of the editor itself. Applies at once to every open editor.")
                Picker("Preview", selection: $variant) {
                    Text("Light").tag(EditorColorTheme.Variant.light)
                    Text("Dark").tag(EditorColorTheme.Variant.dark)
                }
                .pickerStyle(.segmented)
                .accessibilityLabel("Preview and edit the light or dark colours")
                .accessibilityHint("Every theme has light and dark colours; Editor > Editor appearance decides which one the editor shows.")
                ThemePreview(theme: effectiveTheme, variant: variant, font: prefs.font, lineHeight: prefs.lineHeight,
                             lineNumbers: prefs.showLineNumbers, currentLine: prefs.highlightCurrentLine,
                             invisibles: prefs.showInvisibles)
                HStack {
                    Button("Import Theme…") { importTheme() }
                        .accessibilityHint("Adds a theme file (.json) to your themes folder and selects it.")
                    Button("Export Theme…") { exportTheme() }
                        .accessibilityHint("Saves the selected theme, with your colour overrides, as a theme file you can edit and import.")
                    Button("Show Themes Folder") { library.revealFolder() }
                        .accessibilityHint("Opens the folder FlashTeX reads theme files from in Finder.")
                }
                if !selectedTheme.isBuiltIn, library.theme(id: prefs.codeThemeID) != nil {
                    Button("Delete Theme…", role: .destructive) { confirmDelete = true }
                        .accessibilityHint("Removes this theme's file from your themes folder.")
                }
                if let message {
                    Text(message).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                        .accessibilityLabel(message)
                }
                ForEach(library.problems) { problem in
                    Text("\(problem.file): \(problem.message)")
                        .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityWarning)
                }
            }
            Section("Editor colours (\(variant == .light ? "light" : "dark"))") {
                ForEach(EditorColorTheme.Role.chromeRoles, id: \.self) { role in
                    ChromeColorRow(role: role, variant: variant, prefs: prefs, themeColor: selectedTheme.resolved().color(role, variant))
                }
                Button("Use Theme Colours") {
                    var o = prefs.themeOverrides
                    o[variant] = [:]
                    prefs.themeOverrides = o
                }
                .disabled(prefs.themeOverrides[variant].isEmpty)
                .accessibilityHint("Removes every colour override for this appearance.")
            }
            Section("Display") {
                HStack {
                    Slider(value: $prefs.lineHeight, in: EditorPreferences.lineHeightRange, step: 0.05) { Text("Line height") }
                        .accessibilityLabel("Line height")
                        .accessibilityValue(String(format: "%.2f times the font size", prefs.lineHeight))
                    Text(String(format: "%.2f×", prefs.lineHeight)).monospacedDigit()
                        .frame(minWidth: DS.Size.zoomReadoutMinWidth, alignment: .trailing)
                        .accessibilityHidden(true)
                }
                Toggle("Font ligatures", isOn: $prefs.ligatures)
                    .accessibilityHint("When off, character sequences such as -> and != are drawn as separate characters.")
                Toggle("Line numbers", isOn: $prefs.showLineNumbers)
                    .accessibilityHint("Shows line numbers in the gutter. Vim's :set nu and :set nonu change this too.")
                Toggle("Highlight current line", isOn: $prefs.highlightCurrentLine)
                    .accessibilityHint("Draws a band behind the line with the insertion point.")
                Toggle("Show invisible characters", isOn: $prefs.showInvisibles)
                    .accessibilityHint("Draws faint marks for spaces, tabs and line ends. The text itself is unchanged.")
                Text("Font family and size are in the Editor pane; soft wrap and tab width too.")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
            }
        }
        .formStyle(.grouped)
        .frame(width: DS.Layout.settingsWidth)
        .confirmationDialog("Delete the theme “\(selectedTheme.name)”?", isPresented: $confirmDelete) {
            Button("Delete", role: .destructive) {
                do {
                    try library.delete(id: prefs.codeThemeID)
                    prefs.codeThemeID = EditorColorTheme.defaultID
                } catch { message = "Could not delete the theme: \(error.localizedDescription)" }
            }
        } message: { Text("Its file is removed from your themes folder.") }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Theme preferences")
    }

    private func importTheme() {
        switch library.runImportPanel() {
        case .success(let id)?:
            prefs.codeThemeID = id
            message = "Imported “\(library.theme(id: id)?.name ?? id)”."
        case .failure(let error)?:
            message = (error as? EditorColorTheme.FileError)?.description ?? "Could not import: \(error.localizedDescription)"
        case nil: break
        }
    }

    private func exportTheme() {
        var theme = effectiveTheme
        if theme.isBuiltIn || !prefs.themeOverrides.isEmpty {
            theme.id = selectedTheme.id + (prefs.themeOverrides.isEmpty ? "" : "-custom")
            theme.name = selectedTheme.name + (prefs.themeOverrides.isEmpty ? "" : " (custom)")
        }
        theme.isBuiltIn = false
        switch library.runExportPanel(for: theme) {
        case .success(let url)?: message = "Exported to \(url.lastPathComponent)."
        case .failure(let error)?: message = "Could not export: \(error.localizedDescription)"
        case nil: break
        }
    }
}

/// One chrome role: a colour well (the override, or the theme's colour) and
/// a reset button while the role is overridden.
private struct ChromeColorRow: View {
    let role: EditorColorTheme.Role
    let variant: EditorColorTheme.Variant
    @Bindable var prefs: EditorPreferences
    let themeColor: EditorColorTheme.Color?

    private var override: EditorColorTheme.Color? { prefs.themeOverrides[variant][role] }

    var body: some View {
        let current = override ?? themeColor ?? EditorColorTheme.Color(0x808080)
        HStack {
            ColorPicker(role.label, selection: Binding(
                get: { Color(nsColor: EditorThemeRuntime.nsColor(current)) },
                set: { new in
                    guard let c = Self.themeColor(from: new) else { return }
                    var o = prefs.themeOverrides
                    o[variant][role] = c
                    prefs.themeOverrides = o
                }), supportsOpacity: true)
            .accessibilityLabel("\(role.label), \(variant.rawValue) appearance")
            .accessibilityValue(override == nil ? "theme colour \(current.hex)" : "custom colour \(current.hex)")
            if override != nil {
                Button {
                    var o = prefs.themeOverrides
                    o[variant][role] = nil
                    prefs.themeOverrides = o
                } label: { Image(systemName: "arrow.uturn.backward") }
                .buttonStyle(.borderless)
                .help("Use the theme's colour")
                .accessibilityLabel("Use the theme's \(role.label.lowercased()) colour")
            }
        }
    }

    static func themeColor(from color: Color) -> EditorColorTheme.Color? {
        guard let srgb = NSColor(color).usingColorSpace(.sRGB) else { return nil }
        func byte(_ v: CGFloat) -> UInt8 { UInt8(max(0, min(255, (v * 255).rounded()))) }
        return EditorColorTheme.Color(red: byte(srgb.redComponent), green: byte(srgb.greenComponent),
                                      blue: byte(srgb.blueComponent), alpha: byte(srgb.alphaComponent))
    }
}

/// A few lines of LaTeX coloured by the editor's own lexer and `theme`,
/// with the gutter, current line, a selection and the caret, in `variant`.
/// Static: it never hosts a text view.
struct ThemePreview: View {
    let theme: EditorColorTheme
    let variant: EditorColorTheme.Variant
    let font: NSFont
    let lineHeight: Double
    let lineNumbers: Bool
    let currentLine: Bool
    let invisibles: Bool

    static let sample = """
    \\documentclass{article}
    \\usepackage{amsmath} % maths
    \\begin{document}
    \\section{Results}\\label{sec:res}
    Let $\\alpha \\leq 2$ and see~\\cite{knuth}.
    \\begin{equation}
      \\int_0^1 x^2\\,dx = \\frac{1}{3}
    \\end{equation}
    \\end{document}
    """
    static let currentLineIndex = 4
    /// Selected characters on line 3 (`Results`).
    static let selectionLine = 3
    static let selectionRange = 9..<16

    private func color(_ role: EditorColorTheme.Role) -> Color {
        Color(nsColor: theme.color(role, variant).map(EditorThemeRuntime.nsColor) ?? .textColor)
    }

    private var lines: [AttributedString] {
        let ns = Self.sample as NSString
        let runs = SyntaxHighlighter.runs(of: ns)
        var result: [AttributedString] = []
        var start = 0
        for (index, raw) in Self.sample.split(separator: "\n", omittingEmptySubsequences: false).enumerated() {
            let line = String(raw)
            let length = (line as NSString).length
            var a = AttributedString(line)
            a.foregroundColor = color(.foreground)
            for run in runs where run.range.location >= start && NSMaxRange(run.range) <= start + length {
                let local = NSRange(location: run.range.location - start, length: run.range.length)
                guard let r = Range(local, in: line), let lo = AttributedString.Index(r.lowerBound, within: a),
                      let hi = AttributedString.Index(r.upperBound, within: a),
                      let c = theme.color(EditorColorTheme.Role(kind: run.kind), variant) else { continue }
                a[lo..<hi].foregroundColor = Color(nsColor: EditorThemeRuntime.nsColor(c))
            }
            if index == Self.selectionLine, let r = Range(NSRange(location: Self.selectionRange.lowerBound, length: Self.selectionRange.count), in: line),
               let lo = AttributedString.Index(r.lowerBound, within: a), let hi = AttributedString.Index(r.upperBound, within: a) {
                a[lo..<hi].backgroundColor = color(.selection)
            }
            if invisibles {
                for (i, ch) in line.enumerated() where ch == " " {
                    let idx = a.characters.index(a.startIndex, offsetBy: i)
                    a.characters.replaceSubrange(idx..<a.characters.index(after: idx), with: "·")
                    a[idx..<a.characters.index(after: idx)].foregroundColor = color(.invisibles)
                }
                var mark = AttributedString("¬")
                mark.foregroundColor = color(.invisibles)
                a += mark
            }
            result.append(a)
            start += length + 1
        }
        return result
    }

    var body: some View {
        let rowHeight = ceil(font.ascender - font.descender + font.leading) * lineHeight
        let numberFont = Font(NSFont.monospacedDigitSystemFont(ofSize: max(9, font.pointSize - 1.5), weight: .regular))
        HStack(spacing: 0) {
            if lineNumbers {
                VStack(alignment: .trailing, spacing: 0) {
                    ForEach(Array(lines.indices), id: \.self) { i in
                        Text("\(i + 1)").font(numberFont)
                            .foregroundStyle(i == Self.currentLineIndex ? color(.gutterActiveText) : color(.gutterText))
                            .frame(height: rowHeight)
                    }
                }
                .padding(.horizontal, DS.Space.s)
                .background(color(.gutterBackground))
            }
            VStack(alignment: .leading, spacing: 0) {
                ForEach(Array(lines.enumerated()), id: \.offset) { i, line in
                    HStack(spacing: 0) {
                        Text(line).font(Font(font)).lineLimit(1).fixedSize()
                        if i == Self.currentLineIndex {
                            Rectangle().fill(color(.caret)).frame(width: 1.5, height: font.pointSize * 1.2)
                        }
                        Spacer(minLength: 0)
                    }
                    .frame(height: rowHeight)
                    .padding(.leading, DS.Space.m)
                    .background(currentLine && i == Self.currentLineIndex ? color(.currentLine) : .clear)
                }
            }
            .clipped()
        }
        .background(color(.background))
        .clipShape(RoundedRectangle(cornerRadius: DS.Radius.control))
        .overlay(RoundedRectangle(cornerRadius: DS.Radius.control).stroke(DS.Colors.componentBorder))
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Theme preview")
        .accessibilityValue("\(theme.name), \(variant.rawValue) colours: background \(theme.color(.background, variant)?.hex ?? "default"), commands \(theme.color(.command, variant)?.hex ?? "plain"), math \(theme.color(.math, variant)?.hex ?? "plain")")
    }
}
