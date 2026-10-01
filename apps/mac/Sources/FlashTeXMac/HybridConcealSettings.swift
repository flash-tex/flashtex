import SwiftUI

/// Settings > Conceal (lane HYBRID-CONCEAL): the master switch, when the
/// source shows again, which constructs are concealed, and the commands that
/// never are. Applies live to every open editor, like every other pane.
struct HybridConcealSettingsView: View {
    @Bindable private var prefs: EditorPreferences
    /// The deny list as typed; parsed into `conceal.denied` on every change.
    @State private var denyText: String

    @MainActor init() { self.init(preferences: .shared) }

    init(preferences: EditorPreferences) {
        prefs = preferences
        _denyText = State(initialValue: HybridConceal.Settings.formatDenyList(preferences.conceal.denied))
    }

    private func binding<T>(_ path: WritableKeyPath<HybridConceal.Settings, T>) -> Binding<T> {
        Binding(get: { prefs.conceal[keyPath: path] }, set: { var c = prefs.conceal; c[keyPath: path] = $0; prefs.conceal = c })
    }

    private func classBinding(_ cls: HybridConceal.Class) -> Binding<Bool> {
        Binding(get: { prefs.conceal.classes.contains(cls) }, set: { on in
            var c = prefs.conceal
            if on { c.classes.insert(cls) } else { c.classes.remove(cls) }
            prefs.conceal = c
        })
    }

    var body: some View {
        Form {
            Section {
                Toggle("Conceal LaTeX markup", isOn: binding(\.enabled))
                    .accessibilityHint("Shows \\alpha as α, \\textbf{x} as a bold x and similar, until the insertion point reaches them. The text itself never changes.")
                Picker("Show the source", selection: binding(\.reveal)) {
                    ForEach(HybridConceal.Reveal.allCases) { Text($0.label).tag($0) }
                }
                .pickerStyle(.radioGroup)
                .disabled(!prefs.conceal.enabled)
                .accessibilityLabel("When concealed markup shows its source")
                .accessibilityHint("On the caret's line: the whole line shows its source while the insertion point or a selection is on it. At the caret: only the construct the insertion point touches. Always: nothing is concealed.")
                Text("Display only: copying, Find, undo, saving, compiling and VoiceOver always use the source.")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
            }
            Section("Conceal") {
                ForEach(HybridConceal.Class.allCases) { cls in
                    Toggle(isOn: classBinding(cls)) {
                        VStack(alignment: .leading, spacing: DS.Space.xxs) {
                            Text(cls.label)
                            Text(cls.example).font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textSecondary)
                        }
                    }
                    .disabled(!prefs.conceal.enabled)
                    .accessibilityLabel(cls.label)
                    .accessibilityHint("For example \(cls.example).")
                }
            }
            Section("Never conceal") {
                TextField("Commands", text: $denyText, prompt: Text("\\textbf, \\phi, --"))
                    .font(DS.Fonts.mono)
                    .disabled(!prefs.conceal.enabled)
                    .onChange(of: denyText) { _, new in
                        var c = prefs.conceal
                        c.denied = HybridConceal.Settings.parseDenyList(new)
                        if c != prefs.conceal { prefs.conceal = c }
                    }
                    .accessibilityLabel("Commands never concealed")
                    .accessibilityHint("Names separated by commas, for example \\textbf, \\phi. Also -- , --- , `` , '' , ^ and _ for dashes, quotes and scripts.")
                Text("For example, conceal Greek letters but keep \\phi, or keep \\textbf as typed.")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
            }
            Section {
                Button("Restore Conceal Defaults") {
                    prefs.conceal = HybridConceal.Settings(enabled: prefs.conceal.enabled)
                    denyText = ""
                }
                .accessibilityHint("Turns the default kinds of markup back on and clears the list of commands never concealed; the master switch stays as it is.")
            }
        }
        .formStyle(.grouped)
        .frame(width: DS.Layout.settingsWidth)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Conceal preferences")
    }
}
