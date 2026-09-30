import FlashTeXEditorCore
import SwiftUI

/// App-level TeXpand settings (docs/texpand/PLAN.md): the switches the
/// owner asked to control — the whole feature (off until reviewed), each
/// tier, the leader, fractions, auto-preamble, the notation profile and the
/// built-in packs. Stored as the core's Codable `TeXpand.Settings` in its
/// own UserDefaults key, like the error lens, so the versioned
/// `EditorPreferences` snapshot stays untouched. A project's `texpand.toml`
/// layers over this copy (and can switch the feature off, never on).
@MainActor
enum TeXpandPreferences {
    static let key = "FlashTeX.TeXpand.settings"
    static let changed = Notification.Name("FlashTeX.TeXpand.changed")

    /// Tests set this instead of writing the user's defaults.
    static var override: TeXpand.Settings? {
        didSet { cached = nil; NotificationCenter.default.post(name: changed, object: nil) }
    }
    private static var cached: TeXpand.Settings?

    static var settings: TeXpand.Settings {
        get {
            if let override { return override }
            if let cached { return cached }
            let decoded = UserDefaults.standard.data(forKey: key).flatMap { try? JSONDecoder().decode(TeXpand.Settings.self, from: $0) }
            let s = decoded ?? TeXpand.Settings()
            cached = s
            return s
        }
        set {
            if let data = try? JSONEncoder().encode(newValue) { UserDefaults.standard.set(data, forKey: key) }
            cached = newValue
            NotificationCenter.default.post(name: changed, object: nil)
        }
    }

    private static var engineCache: [(settings: TeXpand.Settings, config: TeXpand.Config, engine: TeXpand.Engine)] = []

    /// One engine per settings and config value, shared by the editors that
    /// have the same (loading the catalog parses its TOML). A few are kept:
    /// open documents may carry different magic comments.
    static func engine(for settings: TeXpand.Settings, config: TeXpand.Config = TeXpand.Config()) -> TeXpand.Engine {
        if let c = engineCache.first(where: { $0.settings == settings && $0.config == config }) { return c.engine }
        let e = TeXpand.Engine(registry: config.registry(settings: settings))
        engineCache.append((settings, config, e))
        if engineCache.count > 8 { engineCache.removeFirst() }
        return e
    }

    // MARK: config files (PLAN §13 layers 3–5)

    /// The user's global `texpand.toml` (layer 4). Tests point it elsewhere.
    static var userConfigURL: URL = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("FlashTeX/texpand.toml")
    /// Domain pack files, `*.toml` (layer 3).
    static var packsDirectory: URL { userConfigURL.deletingLastPathComponent().appendingPathComponent("texpand-packs") }

    /// A config file as a layer named by its path, and its modification date.
    static func layer(at url: URL, name: String) -> (layer: TeXpand.Layer, modified: Date)? {
        guard let attrs = try? FileManager.default.attributesOfItem(atPath: url.path),
              let source = try? String(contentsOf: url, encoding: .utf8) else { return nil }
        return (TeXpand.Layer(name: name, source: source), attrs[.modificationDate] as? Date ?? .distantPast)
    }

    static func modified(_ url: URL) -> Date? {
        (try? FileManager.default.attributesOfItem(atPath: url.path))?[.modificationDate] as? Date
    }

    static func packLayers() -> [TeXpand.Layer] {
        let files = (try? FileManager.default.contentsOfDirectory(at: packsDirectory, includingPropertiesForKeys: nil)) ?? []
        return files.filter { $0.pathExtension == "toml" }.sorted { $0.lastPathComponent < $1.lastPathComponent }
            .compactMap { layer(at: $0, name: "texpand-packs/" + $0.lastPathComponent)?.layer }
    }

    /// The user file's problems, for Settings.
    static func userDiagnostics() -> [TeXpand.Diagnostic] {
        guard let user = layer(at: userConfigURL, name: "texpand.toml (user)")?.layer else { return [] }
        return TeXpand.Config(user: user).registry(settings: settings).diagnostics.filter { $0.layer == user.name }
    }

    /// Creates the user file with a commented starter if it is missing.
    static func ensureUserFile() {
        let fm = FileManager.default
        guard !fm.fileExists(atPath: userConfigURL.path) else { return }
        try? fm.createDirectory(at: userConfigURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        let starter = """
        # TeXpand user configuration (docs/texpand/PLAN.md §13). Project files
        # (texpand.toml at the project root) and % !texpand magic comments
        # layer over this one.
        #
        # [settings]
        # leader = ";"
        # profile = "upright"
        # disable = ["xx"]
        #
        # [[abbr]]
        # name = "hello"
        # body = 'Hello, <<1:world>>!'

        """
        try? starter.write(to: userConfigURL, atomically: true, encoding: .utf8)
    }

    /// The built-in packs, for the per-pack switches.
    static let builtInPacks: [TeXpand.Pack] = TeXpand.Registry.load().packs
}

/// Settings > Abbreviations.
struct TeXpandSettingsSection: View {
    @State private var settings = TeXpandPreferences.settings

    var body: some View {
        Section("TeXpand") {
            Toggle("Expand abbreviations", isOn: $settings.enabled)
                .accessibilityHint("Master switch for TeXpand, which turns short abbreviations such as ;enum3 followed by Tab into LaTeX structure. Off by default.")
            Text("Off by default. When on, type an abbreviation after the leader, such as ;enum3, and press Tab. Esc keeps what you typed, and ⌘Z after an expansion brings it back. Instant atoms, ligatures, postfix and the structure editor arrive in later updates; their switches are saved now.")
                .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
        }
        Section("Kinds") {
            Toggle("Abbreviations after the leader", isOn: $settings.abbreviations)
                .accessibilityHint("The leader character, a name and Tab: ;enum3, ;fig#arch, ;tab:lcr:4.")
            Toggle("Instant atoms", isOn: $settings.instantAtoms)
                .disabled(!settings.abbreviations)
                .accessibilityHint("Short math atoms expand without Tab, for example ;a to \\alpha.")
            Toggle("Ligatures in math", isOn: $settings.ligatures)
                .accessibilityHint("Character sequences in math change as you type, for example -> to \\to. Off by default.")
            Toggle("Postfix in math", isOn: $settings.postfix)
                .accessibilityHint("x.hat then Tab gives \\hat{x}; a//b then Tab gives a fraction.")
            Toggle("Structure editor for matrices and tables", isOn: $settings.structureEditor)
                .accessibilityHint("A shortcut inside a matrix, tabular, cases or align opens a grid editor over it.")
        }
        .disabled(!settings.enabled)
        Section("Behaviour") {
            TextField("Leader", text: leader)
                .font(.body.monospaced())
                .accessibilityHint("The character that starts an abbreviation. One character, or empty for abbreviations triggered by Tab alone.")
            Picker("Fractions", selection: $settings.fractionTrigger) {
                Text("On Tab").tag(TeXpand.Settings.FractionTrigger.tab)
                Text("As you type").tag(TeXpand.Settings.FractionTrigger.auto)
                Text("Off").tag(TeXpand.Settings.FractionTrigger.off)
            }
            .accessibilityHint("When a//b becomes a fraction.")
            Picker("Fraction operator", selection: $settings.fractionOperator) {
                Text("//  (a/b stays as typed)").tag("//")
                Text("/").tag("/")
            }
            .accessibilityHint("Which characters mark a fraction. With //, a single slash never changes.")
            Picker("Missing packages", selection: $settings.autoPreamble) {
                Text("Add to the preamble").tag(TeXpand.Settings.AutoPreamble.insert)
                Text("Ask first").tag(TeXpand.Settings.AutoPreamble.prompt)
                Text("Leave the preamble alone").tag(TeXpand.Settings.AutoPreamble.off)
            }
            .accessibilityHint("What happens when an expansion needs a package the document does not load.")
            Picker("Notation", selection: $settings.profile) {
                ForEach(TeXpand.Profile.named.keys.sorted(), id: \.self) { Text($0.capitalized).tag($0) }
            }
            .accessibilityHint("Notation conventions such as an upright d in derivatives.")
        }
        .disabled(!settings.enabled)
        Section("Packs") {
            ForEach(TeXpandPreferences.builtInPacks, id: \.name) { pack in
                Toggle(pack.summary.isEmpty ? pack.name : pack.summary, isOn: packBinding(pack.name))
                    .accessibilityHint("Abbreviations in the \(pack.name) pack.")
            }
        }
        .disabled(!settings.enabled)
        .onChange(of: settings) { _, new in TeXpandPreferences.settings = new }
        Section("Configuration") {
            Text("Your texpand.toml adds and changes abbreviations everywhere; a texpand.toml at a project's root, and % !texpand lines at the top of a document, layer over it. Changes apply on save.")
                .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
            HStack {
                Button("Open User Configuration") {
                    TeXpandPreferences.ensureUserFile()
                    NSWorkspace.shared.open(TeXpandPreferences.userConfigURL)
                }
                .accessibilityHint("Opens texpand.toml in Application Support, creating it with commented examples if it is missing.")
                Spacer()
                Button("Check") { problems = TeXpandPreferences.userDiagnostics() }
                    .accessibilityHint("Lists problems in your texpand.toml.")
            }
            ForEach(problems.indices, id: \.self) { k in
                Text(problems[k].description)
                    .font(DS.Fonts.secondary)
                    .foregroundStyle(problems[k].severity == .error ? Color.red : DS.Colors.textSecondary)
            }
        }
        .onAppear { problems = TeXpandPreferences.userDiagnostics() }
    }

    @State private var problems: [TeXpand.Diagnostic] = []

    /// Refuses letters, digits, whitespace and `\` (TeXpand.Settings.leaderProblem).
    private var leader: Binding<String> {
        Binding(get: { settings.leader }, set: { new in
            let v = String(new.suffix(1))
            if TeXpand.Settings.leaderProblem(v) == nil { settings.leader = v }
        })
    }

    private func packBinding(_ name: String) -> Binding<Bool> {
        Binding(get: { !settings.disabledPacks.contains(name) }, set: { on in
            settings.disabledPacks.removeAll { $0 == name }
            if !on { settings.disabledPacks.append(name); settings.disabledPacks.sort() }
        })
    }
}
