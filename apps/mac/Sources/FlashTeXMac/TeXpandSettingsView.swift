import FlashTeXEditorCore
import SwiftUI

/// App-level TeXpand settings (docs/texpand/PLAN.md): the switches the
/// owner asked to control — the whole feature (off until reviewed), each
/// tier, the leader, fractions, auto-preamble, the notation profile and the
/// built-in packs. Stored as the core's Codable `TeXpand.Settings` in its
/// own UserDefaults key, like the error lens, so the versioned
/// `EditorPreferences` snapshot stays untouched. A project's `texpand.toml`
/// layers over this copy (and can switch the feature off, never on).
enum TeXpandPreferences {
    static let key = "FlashTeX.TeXpand.settings"
    static let changed = Notification.Name("FlashTeX.TeXpand.changed")

    static var settings: TeXpand.Settings {
        get {
            guard let data = UserDefaults.standard.data(forKey: key),
                  let s = try? JSONDecoder().decode(TeXpand.Settings.self, from: data) else { return TeXpand.Settings() }
            return s
        }
        set {
            if let data = try? JSONEncoder().encode(newValue) { UserDefaults.standard.set(data, forKey: key) }
            NotificationCenter.default.post(name: changed, object: nil)
        }
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
            Text("Off by default. Typing expansion in the editor arrives in a later update; the choices here are saved now. A texpand.toml at the project root can add or change abbreviations, and can turn TeXpand off for that project.")
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
    }

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
