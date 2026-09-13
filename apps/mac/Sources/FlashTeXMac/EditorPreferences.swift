import AppKit
import Observation
import SwiftUI

/// Persisted editor display preferences (owner: mac-preferences).
///
/// One `@Observable` singleton (`EditorPreferences.shared`) with typed,
/// clamped properties. Every setter validates the new value, publishes an
/// observation mutation, bumps `generation`, and writes through to the backing
/// `UserDefaults` under a versioned key set (`FlashTeX.EditorPreferences.v1.*`
/// plus a `schemaVersion` stamp). Absent or invalid stored values load as the
/// documented default (or the nearest valid value) and are written back, so
/// the stored set is always valid after `load()`.
///
/// Integration seam for the source editor (its file is owned by another lane):
/// `apply(to:)` pushes the display preferences into an `NSTextView`, and
/// `observeApplying(to:)` keeps doing so whenever a property changes. Which
/// property needs which `NSTextView` update:
///
/// | property                | NSTextView update (`apply(to:)` does all of these)                      |
/// |-------------------------|--------------------------------------------------------------------------|
/// | `fontFamily`, `fontSize`| `font` (re-fonts the whole storage and the typing attributes) and the   |
/// |                         | tab interval, which is measured in the new font                          |
/// | `tabWidth`              | `defaultParagraphStyle.defaultTabInterval`, typing attributes and the    |
/// |                         | paragraph style of the existing text (attribute-only storage edit: no    |
/// |                         | text change notification, no undo step)                                  |
/// | `indentStyle`           | nothing in AppKit; the editor's Tab key handler reads `indentString`     |
/// | `lineWrapping`          | text container width tracking / size, horizontal resizability and the    |
/// |                         | enclosing scroll view's horizontal scroller                              |
/// | `appearance`            | `appearance` of the enclosing scroll view (or the text view when there   |
/// |                         | is none); dynamic colours (label, text background, insertion point)      |
/// |                         | follow it on the next draw                                               |
/// | `autoCloseBraces`       | nothing in AppKit; the editor's typing handler reads the flag            |
/// | `completionPopup`       | nothing in AppKit; `CompletingTextView.requestCompletion` reads the flag |
/// | `spellCheck`            | nothing here; `LaTeXSpellChecker` observes the flag                      |
///
/// Reading a property inside `withObservationTracking` (or a SwiftUI body)
/// registers for its changes; `generation` changes with every property.
@Observable @MainActor
final class EditorPreferences {
    /// Process-wide instance backed by `UserDefaults.standard`.
    static let shared = EditorPreferences(defaults: .standard)

    // MARK: value types

    enum Appearance: String, CaseIterable, Identifiable, Sendable {
        case system, light, dark
        var id: String { rawValue }
        var label: String {
            switch self { case .system: "System"; case .light: "Light"; case .dark: "Dark" }
        }
        /// The AppKit appearance to install on the editor; nil follows the window.
        var nsAppearance: NSAppearance? {
            switch self {
            case .system: nil
            case .light: NSAppearance(named: .aqua)
            case .dark: NSAppearance(named: .darkAqua)
            }
        }
        /// SwiftUI `.preferredColorScheme` value; nil follows the system.
        var colorScheme: ColorScheme? {
            switch self { case .system: nil; case .light: .light; case .dark: .dark }
        }
        /// Whether the preview's dark toggle should start on, given whether the
        /// system appearance is dark right now (pure, for tests).
        func prefersDarkPreview(systemIsDark: Bool) -> Bool {
            switch self { case .system: systemIsDark; case .light: false; case .dark: true }
        }
    }

    enum IndentStyle: String, CaseIterable, Identifiable, Sendable {
        case spaces, tabs
        var id: String { rawValue }
        var label: String { self == .spaces ? "Spaces" : "Tab character" }
    }

    /// Everything the editor displays, as one Equatable value (for observers
    /// that want to diff, and for tests).
    struct Snapshot: Equatable, Sendable {
        var fontFamily: String?
        var fontSize: Double
        var lineWrapping: Bool
        var tabWidth: Int
        var indentStyle: IndentStyle
        var appearance: Appearance
        var autoCloseBraces: Bool
        var completionPopup: Bool
        var spellCheck: Bool
        var previewFollowsCaret: Bool
    }

    // MARK: defaults and ranges

    static let fontSizeRange: ClosedRange<Double> = 8...36
    static let tabWidthRange: ClosedRange<Int> = 2...8

    static let defaultSnapshot = Snapshot(
        fontFamily: nil, fontSize: 13, lineWrapping: true, tabWidth: 4, indentStyle: .spaces,
        appearance: .system, autoCloseBraces: true, completionPopup: true, spellCheck: true, previewFollowsCaret: true)

    // MARK: storage keys (versioned)

    /// Bump when the meaning or encoding of a stored value changes; add a
    /// migration step in `migrate(_:)` that reads the old keys and writes the
    /// new ones. Values live under `FlashTeX.EditorPreferences.v<N>.<name>`
    /// so old and new sets can coexist during migration.
    nonisolated static let schemaVersion = 1
    nonisolated static let schemaVersionKey = "FlashTeX.EditorPreferences.schemaVersion"

    enum Key: String, CaseIterable {
        case fontFamily, fontSize, lineWrapping, tabWidth, indentStyle, appearance, autoCloseBraces, completionPopup, spellCheck
        case previewFollowsCaret
        var storageKey: String { "FlashTeX.EditorPreferences.v\(EditorPreferences.schemaVersion).\(rawValue)" }
    }

    // MARK: state

    @ObservationIgnored private let defaults: UserDefaults
    @ObservationIgnored private var storage = EditorPreferences.defaultSnapshot
    /// Keys whose stored value was absent or invalid at the last `load()` and
    /// were replaced by a valid one (evidence for tests and the handoff).
    @ObservationIgnored private(set) var lastLoadRepairs: [Key] = []
    /// Incremented on every actual value change and on `load()`; a set that
    /// clamps to the current value is a no-op.
    private(set) var generation = 0

    /// `defaults`: the backing store; tests pass a temporary suite.
    init(defaults: UserDefaults) {
        self.defaults = defaults
        load()
    }

    // MARK: typed, clamped properties

    /// Installed monospaced family, or nil for the system monospaced face.
    /// An unknown or proportional family is rejected in favour of nil.
    var fontFamily: String? {
        get { access(keyPath: \.fontFamily); return storage.fontFamily }
        set { update(\.fontFamily, \.fontFamily, Self.validatedFontFamily(newValue), key: .fontFamily) }
    }

    /// Points, clamped to `fontSizeRange`; non-finite input keeps the default.
    var fontSize: Double {
        get { access(keyPath: \.fontSize); return storage.fontSize }
        set { update(\.fontSize, \.fontSize, Self.clampedFontSize(newValue), key: .fontSize) }
    }

    var lineWrapping: Bool {
        get { access(keyPath: \.lineWrapping); return storage.lineWrapping }
        set { update(\.lineWrapping, \.lineWrapping, newValue, key: .lineWrapping) }
    }

    /// Columns per tab stop, clamped to `tabWidthRange`.
    var tabWidth: Int {
        get { access(keyPath: \.tabWidth); return storage.tabWidth }
        set { update(\.tabWidth, \.tabWidth, Self.clampedTabWidth(newValue), key: .tabWidth) }
    }

    var indentStyle: IndentStyle {
        get { access(keyPath: \.indentStyle); return storage.indentStyle }
        set { update(\.indentStyle, \.indentStyle, newValue, key: .indentStyle) }
    }

    /// Editor appearance; also the default of the preview's dark toggle.
    var appearance: Appearance {
        get { access(keyPath: \.appearance); return storage.appearance }
        set { update(\.appearance, \.appearance, newValue, key: .appearance) }
    }

    /// Consumed by the editor lane's auto-close feature.
    var autoCloseBraces: Bool {
        get { access(keyPath: \.autoCloseBraces); return storage.autoCloseBraces }
        set { update(\.autoCloseBraces, \.autoCloseBraces, newValue, key: .autoCloseBraces) }
    }

    /// Whether the completion list may open (typing, Esc, ⌃Space).
    var completionPopup: Bool {
        get { access(keyPath: \.completionPopup); return storage.completionPopup }
        set { update(\.completionPopup, \.completionPopup, newValue, key: .completionPopup) }
    }

    /// Red underlines for misspelled prose in the source editor (LaTeXSpellCheck.swift).
    var spellCheck: Bool {
        get { access(keyPath: \.spellCheck); return storage.spellCheck }
        set { update(\.spellCheck, \.spellCheck, newValue, key: .spellCheck) }
    }

    /// Whether the preview auto-scrolls to the caret's page item after a
    /// short debounce (FollowCaret.swift; `PreviewAnchorProbe` reads this).
    /// The manual "Reveal Caret in Preview" command (⌘⇧J) always works
    /// regardless of this setting.
    var previewFollowsCaret: Bool {
        get { access(keyPath: \.previewFollowsCaret); return storage.previewFollowsCaret }
        set { update(\.previewFollowsCaret, \.previewFollowsCaret, newValue, key: .previewFollowsCaret) }
    }

    /// All properties at once (registers for every property's changes).
    var snapshot: Snapshot {
        Snapshot(fontFamily: fontFamily, fontSize: fontSize, lineWrapping: lineWrapping, tabWidth: tabWidth,
                 indentStyle: indentStyle, appearance: appearance, autoCloseBraces: autoCloseBraces,
                 completionPopup: completionPopup, spellCheck: spellCheck, previewFollowsCaret: previewFollowsCaret)
    }

    // MARK: derived values

    /// The resolved editor font (never nil: an unavailable family falls back
    /// to the system monospaced face at `fontSize`).
    var font: NSFont { Self.resolveFont(family: fontFamily, size: fontSize) }

    /// What one Tab keystroke inserts under `indentStyle`/`tabWidth`.
    var indentString: String { indentStyle == .tabs ? "\t" : String(repeating: " ", count: tabWidth) }

    /// Default of the preview's dark toggle for the current system appearance.
    var darkPreviewDefault: Bool {
        appearance.prefersDarkPreview(systemIsDark: Self.systemAppearanceIsDark())
    }

    static func systemAppearanceIsDark() -> Bool {
        let appearance = NSApp?.effectiveAppearance ?? NSAppearance.currentDrawing()
        return appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
    }

    // MARK: validation (pure)

    static func clampedFontSize(_ value: Double) -> Double {
        guard value.isFinite else { return defaultSnapshot.fontSize }
        return min(max(value, fontSizeRange.lowerBound), fontSizeRange.upperBound)
    }

    static func clampedTabWidth(_ value: Int) -> Int {
        min(max(value, tabWidthRange.lowerBound), tabWidthRange.upperBound)
    }

    /// The family if it is installed and fixed pitch, else nil (system face).
    static func validatedFontFamily(_ family: String?) -> String? {
        guard let family, !family.isEmpty, let font = installedFont(family: family, size: 13), font.isFixedPitch else { return nil }
        return family
    }

    /// Regular-weight member of `family`, nil when the family is not installed.
    static func installedFont(family: String, size: Double) -> NSFont? {
        let manager = NSFontManager.shared
        guard manager.availableFontFamilies.contains(family) else { return nil }
        return manager.font(withFamily: family, traits: [], weight: 5, size: size)
    }

    static func resolveFont(family: String?, size: Double) -> NSFont {
        let size = clampedFontSize(size)
        if let family, let font = installedFont(family: family, size: size), font.isFixedPitch { return font }
        return .monospacedSystemFont(ofSize: size, weight: .regular)
    }

    /// Installed families whose regular member is fixed pitch, sorted; the
    /// candidates for the font picker (the system face is offered separately).
    static func installedMonospacedFamilies() -> [String] {
        let manager = NSFontManager.shared
        return manager.availableFontFamilies.filter { family in
            manager.font(withFamily: family, traits: [], weight: 5, size: 13)?.isFixedPitch == true
        }.sorted { $0.localizedCaseInsensitiveCompare($1) == .orderedAscending }
    }

    // MARK: persistence

    /// Reads every key, repairing absent/invalid values to valid ones (written
    /// back), after running the schema migration.
    func load() {
        Self.migrate(defaults)
        var repairs: [Key] = []
        var s = Self.defaultSnapshot

        if let raw = defaults.object(forKey: Key.fontFamily.storageKey) {
            let family = raw as? String
            s.fontFamily = Self.validatedFontFamily(family)
            if family == nil || s.fontFamily != family { repairs.append(.fontFamily) }
        } // absent: the system face, nothing to repair

        if let raw = defaults.object(forKey: Key.fontSize.storageKey) {
            let value = (raw as? NSNumber)?.doubleValue
            s.fontSize = value.map(Self.clampedFontSize) ?? Self.defaultSnapshot.fontSize
            if value != s.fontSize { repairs.append(.fontSize) }
        } else { repairs.append(.fontSize) }

        if let raw = defaults.object(forKey: Key.lineWrapping.storageKey), let value = raw as? Bool {
            s.lineWrapping = value
        } else { repairs.append(.lineWrapping) }

        if let raw = defaults.object(forKey: Key.tabWidth.storageKey) {
            let value = (raw as? NSNumber)?.intValue
            s.tabWidth = value.map(Self.clampedTabWidth) ?? Self.defaultSnapshot.tabWidth
            if value != s.tabWidth { repairs.append(.tabWidth) }
        } else { repairs.append(.tabWidth) }

        if let value = (defaults.object(forKey: Key.indentStyle.storageKey) as? String).flatMap(IndentStyle.init(rawValue:)) {
            s.indentStyle = value
        } else { repairs.append(.indentStyle) }

        if let value = (defaults.object(forKey: Key.appearance.storageKey) as? String).flatMap(Appearance.init(rawValue:)) {
            s.appearance = value
        } else { repairs.append(.appearance) }

        if let value = defaults.object(forKey: Key.autoCloseBraces.storageKey) as? Bool {
            s.autoCloseBraces = value
        } else { repairs.append(.autoCloseBraces) }

        if let value = defaults.object(forKey: Key.completionPopup.storageKey) as? Bool {
            s.completionPopup = value
        } else { repairs.append(.completionPopup) }

        if let value = defaults.object(forKey: Key.spellCheck.storageKey) as? Bool {
            s.spellCheck = value
        } else { repairs.append(.spellCheck) }

        if let value = defaults.object(forKey: Key.previewFollowsCaret.storageKey) as? Bool {
            s.previewFollowsCaret = value
        } else { repairs.append(.previewFollowsCaret) }

        withMutation(keyPath: \.generation) {
            storage = s
            generation += 1
        }
        lastLoadRepairs = repairs
        for key in repairs { write(key) }
    }

    /// Restores every property to its default (persisted).
    func resetToDefaults() {
        let d = Self.defaultSnapshot
        fontFamily = d.fontFamily; fontSize = d.fontSize; lineWrapping = d.lineWrapping; tabWidth = d.tabWidth
        indentStyle = d.indentStyle; appearance = d.appearance; autoCloseBraces = d.autoCloseBraces
        completionPopup = d.completionPopup; spellCheck = d.spellCheck; previewFollowsCaret = d.previewFollowsCaret
    }

    /// Versioned migration. Absent stamp: nothing was ever stored (or only
    /// unversioned pre-release values, which are ignored) → stamp the current
    /// version. A newer stamp is left alone: the values this version
    /// understands are read, the rest preserved for the newer app.
    static func migrate(_ defaults: UserDefaults) {
        let stored = defaults.integer(forKey: schemaVersionKey)
        guard stored < schemaVersion else { return }
        // Future: `if stored < 2 { read v1 keys, write v2 keys }` steps here.
        defaults.set(schemaVersion, forKey: schemaVersionKey)
    }

    private func update<T: Equatable>(_ property: KeyPath<EditorPreferences, T>, _ field: WritableKeyPath<Snapshot, T>,
                                      _ value: T, key: Key) {
        guard storage[keyPath: field] != value else { return }
        withMutation(keyPath: property) {
            withMutation(keyPath: \.generation) {
                storage[keyPath: field] = value
                generation += 1
            }
        }
        write(key)
    }

    private func write(_ key: Key) {
        let k = key.storageKey
        switch key {
        case .fontFamily:
            if let f = storage.fontFamily { defaults.set(f, forKey: k) } else { defaults.removeObject(forKey: k) }
        case .fontSize: defaults.set(storage.fontSize, forKey: k)
        case .lineWrapping: defaults.set(storage.lineWrapping, forKey: k)
        case .tabWidth: defaults.set(storage.tabWidth, forKey: k)
        case .indentStyle: defaults.set(storage.indentStyle.rawValue, forKey: k)
        case .appearance: defaults.set(storage.appearance.rawValue, forKey: k)
        case .autoCloseBraces: defaults.set(storage.autoCloseBraces, forKey: k)
        case .completionPopup: defaults.set(storage.completionPopup, forKey: k)
        case .spellCheck: defaults.set(storage.spellCheck, forKey: k)
        case .previewFollowsCaret: defaults.set(storage.previewFollowsCaret, forKey: k)
        }
    }

    // MARK: NSTextView integration

    /// Pushes font, tab interval, wrapping and appearance into `textView`
    /// (see the table in the type comment). Attribute-only storage edits: no
    /// text-change notification, no undo step, temporary attributes (the
    /// diagnostic marks) untouched. Safe to call on every change; cost is one
    /// attribute pass over the storage when the font or tab width changed.
    func apply(to textView: NSTextView) {
        let font = self.font
        let tabInterval = Self.tabInterval(columns: tabWidth, font: font)
        let style = (textView.defaultParagraphStyle ?? NSParagraphStyle.default).mutableCopy() as! NSMutableParagraphStyle
        style.tabStops = []
        style.defaultTabInterval = tabInterval
        // Font first: `NSText.font` re-fonts the whole storage and the typing
        // attributes; the paragraph style then goes to both as well.
        if textView.font != font { textView.font = font }
        textView.defaultParagraphStyle = style
        textView.typingAttributes[.paragraphStyle] = style
        textView.typingAttributes[.font] = font
        if let storage = textView.textStorage, storage.length > 0 {
            let whole = NSRange(location: 0, length: storage.length)
            var same = true
            storage.enumerateAttribute(.paragraphStyle, in: whole) { value, range, stop in
                if (value as? NSParagraphStyle)?.defaultTabInterval != tabInterval || range != whole { same = false; stop.pointee = true }
            }
            if !same { storage.addAttribute(.paragraphStyle, value: style, range: whole) }
        }

        Self.setLineWrapping(lineWrapping, on: textView)

        let host: NSView = textView.enclosingScrollView ?? textView
        let wanted = appearance.nsAppearance
        if host.appearance?.name != wanted?.name { host.appearance = wanted }
    }

    /// Width of `columns` spaces in `font` (the advance of a space; a
    /// monospaced font's uniform advance).
    static func tabInterval(columns: Int, font: NSFont) -> CGFloat {
        let space = (" " as NSString).size(withAttributes: [.font: font]).width
        return CGFloat(columns) * max(space, 1)
    }

    static func setLineWrapping(_ wrap: Bool, on textView: NSTextView) {
        guard let container = textView.textContainer else { return }
        let scroll = textView.enclosingScrollView
        if wrap {
            // Before the first layout the clip view is empty; tracking fills the
            // width in on the first resize, so only a real width is pushed now.
            let width = scroll?.contentView.bounds.width ?? textView.bounds.width
            container.widthTracksTextView = true
            textView.isHorizontallyResizable = false
            textView.autoresizingMask = [.width]
            if width > 0 {
                container.containerSize = NSSize(width: width, height: CGFloat.greatestFiniteMagnitude)
                textView.frame.size.width = width
            }
            scroll?.hasHorizontalScroller = false
        } else {
            container.widthTracksTextView = false
            container.containerSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
            textView.isHorizontallyResizable = true
            textView.autoresizingMask = []
            textView.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
            scroll?.hasHorizontalScroller = true
        }
        textView.isVerticallyResizable = true
        textView.needsLayout = true
    }

    /// Applies now and after every later change, until the token is released
    /// (store it in the editor's coordinator). The text view is held weakly.
    func observeApplying(to textView: NSTextView) -> ObservationToken {
        let token = ObservationToken()
        apply(to: textView)
        arm(token, textView)
        return token
    }

    private func arm(_ token: ObservationToken, _ textView: NSTextView) {
        withObservationTracking { [weak self] in _ = self?.snapshot } onChange: { [weak self, weak token, weak textView] in
            Task { @MainActor in
                guard let self, let token, token.isActive, let textView else { return }
                self.apply(to: textView)
                token.applications += 1
                self.arm(token, textView)
            }
        }
    }

    /// Cancels the observation when released or `cancel()`led.
    @MainActor
    final class ObservationToken {
        fileprivate var isActive = true
        /// Number of re-applications delivered after the initial one (evidence).
        fileprivate(set) var applications = 0
        func cancel() { isActive = false }
        deinit { isActive = false }
    }
}

// MARK: - Settings scene

/// The Settings window body: `Settings { EditorPreferencesView() }` in the
/// app. Every control is a standard focusable SwiftUI control (Tab moves
/// between them) with an explicit accessibility label/hint for VoiceOver.
struct EditorPreferencesView: View {
    @Bindable private var prefs: EditorPreferences
    @State private var families: [String] = []

    /// `showConversion`: the Capture conversion section
    /// (ConversionPreferencesView.swift) — on in the app; the editor-table
    /// accessibility tests host the editor controls alone
    /// (`PanelFocusOrder.panels[0]` enumerates only those; see
    /// apps/mac/docs/capture-conversion.md, accessibility note).
    private let showConversion: Bool

    @MainActor init() { prefs = .shared; showConversion = true }
    init(preferences: EditorPreferences, showConversion: Bool = false) { prefs = preferences; self.showConversion = showConversion }

    var body: some View {
        Form {
            Section("Font") {
                Picker("Family", selection: $prefs.fontFamily) {
                    Text("System monospaced").tag(String?.none)
                    ForEach(families, id: \.self) { family in Text(family).tag(String?.some(family)) }
                }
                .accessibilityLabel("Editor font family")
                .accessibilityHint("Only installed monospaced fonts are offered; an unavailable font falls back to the system monospaced face.")
                HStack {
                    Slider(value: $prefs.fontSize, in: EditorPreferences.fontSizeRange, step: 1) { Text("Size") }
                        .accessibilityLabel("Editor font size")
                        .accessibilityValue("\(Int(prefs.fontSize)) points")
                    Stepper(value: $prefs.fontSize, in: EditorPreferences.fontSizeRange, step: 1) {
                        Text("\(Int(prefs.fontSize)) pt").monospacedDigit().frame(minWidth: 40, alignment: .trailing)
                    }
                    .accessibilityLabel("Editor font size stepper")
                    .accessibilityValue("\(Int(prefs.fontSize)) points")
                }
                Text("\\section{Sample} $x^2 + y^2 = z^2$")
                    .font(Font(prefs.font))
                    .lineLimit(1)
                    .accessibilityLabel("Font sample")
                    .accessibilityValue("\(prefs.fontFamily ?? "System monospaced") at \(Int(prefs.fontSize)) points")
            }
            Section("Layout") {
                Toggle("Wrap long lines", isOn: $prefs.lineWrapping)
                    .accessibilityHint("Off shows a horizontal scroller instead of wrapping.")
                Stepper(value: $prefs.tabWidth, in: EditorPreferences.tabWidthRange) {
                    Text("Tab width: \(prefs.tabWidth) columns")
                }
                .accessibilityLabel("Tab width")
                .accessibilityValue("\(prefs.tabWidth) columns")
                Picker("Indent with", selection: $prefs.indentStyle) {
                    ForEach(EditorPreferences.IndentStyle.allCases) { Text($0.label).tag($0) }
                }
                .pickerStyle(.radioGroup)
                .accessibilityLabel("Indent style")
            }
            Section("Appearance") {
                Picker("Editor appearance", selection: $prefs.appearance) {
                    ForEach(EditorPreferences.Appearance.allCases) { Text($0.label).tag($0) }
                }
                .pickerStyle(.segmented)
                .accessibilityLabel("Editor appearance")
                .accessibilityHint("Also the default of the preview's dark toggle for new windows.")
            }
            Section("Typing") {
                Toggle("Auto-close brackets & math", isOn: $prefs.autoCloseBraces)
                    .accessibilityHint("Typing {, (, [ or $ inserts the matching closer and places the caret between them.")
                Toggle("Show completion list", isOn: $prefs.completionPopup)
                    .accessibilityHint("When off, the list never opens; Control-Space and Escape do nothing.")
                Toggle("Check spelling", isOn: $prefs.spellCheck)
                    .accessibilityHint("Underlines misspelled words in prose; commands, math, comments and labels are skipped.")
                ErrorLensPreferenceRows() // inline diagnostic text at line ends (ErrorLens.swift)
            }
            Section("Preview") {
                Toggle("Preview follows the caret", isOn: $prefs.previewFollowsCaret)
                    .accessibilityHint("Scrolls the preview to the caret's page item shortly after you move the caret or stop typing. \"Reveal Caret in Preview\" (⌘⇧J) always works regardless of this setting.")
            }
            if showConversion { ConversionPreferencesSection() } // provider picker, model, API key (Keychain) (ConversionPreferencesView.swift)
            Section {
                Button("Restore Defaults") { prefs.resetToDefaults() }
                    .accessibilityHint("Resets every editor preference to its default value.")
            }
        }
        .formStyle(.grouped)
        .frame(width: 460)
        .onAppear { if families.isEmpty { families = EditorPreferences.installedMonospacedFamilies() } }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Editor preferences")
    }
}
