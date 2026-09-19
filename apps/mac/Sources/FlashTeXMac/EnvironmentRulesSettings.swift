import SwiftUI

/// Settings > Environments: the Return-key rules per environment
/// (EnvironmentEditingRules.swift). One toggle for the environments the
/// table does not name, then a row per named environment — its name,
/// whether its body is indented, and what each new body line starts with.
/// Edits write the whole rule set back through the binding (persisted as
/// JSON by `EditorPreferences`); a blank name is dropped on the way in.
struct EnvironmentRulesSection: View {
    @Binding var rules: EnvironmentEditingRules
    /// Rows being edited; a blank name stays here until the user fills it
    /// in (the binding drops it), so a fresh row is not deleted under the
    /// caret.
    @State private var rows: [EnvironmentEditingRules.Rule] = []

    var body: some View {
        Section("Return inside an environment") {
            Toggle("Indent inside other environments", isOn: $rules.indentByDefault)
                .accessibilityHint("Return after \\begin of an environment the table does not list starts the body one indent level deeper.")
        }
        Section("Per environment") {
            VStack(alignment: .leading, spacing: DS.Space.xs) {
                HStack(spacing: DS.Space.m) {
                    Text("Environment").frame(width: 130, alignment: .leading)
                    Text("Indent").frame(width: 50)
                    Text("New line starts with").frame(maxWidth: .infinity, alignment: .leading)
                }
                .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                ForEach($rows) { $row in
                    HStack(spacing: DS.Space.m) {
                        TextField("name", text: $row.environment)
                            .textFieldStyle(.roundedBorder)
                            .frame(width: 130)
                            .accessibilityLabel("Environment name")
                        Toggle("", isOn: $row.indent)
                            .labelsHidden()
                            .frame(width: 50)
                            .accessibilityLabel("Indent the body of \\(row.environment)")
                        TextField("nothing", text: $row.newLine)
                            .textFieldStyle(.roundedBorder)
                            .font(.body.monospaced())
                            .accessibilityLabel("Text each new line of \\(row.environment) starts with")
                        Button {
                            rows.removeAll { $0.id == row.id }
                            commit()
                        } label: { Image(systemName: "minus.circle") }
                            .buttonStyle(.borderless)
                            .accessibilityLabel("Remove the rule for \\(row.environment)")
                    }
                }
                HStack {
                    Button {
                        rows.append(EnvironmentEditingRules.Rule(environment: "", indent: rules.indentByDefault, newLine: ""))
                    } label: { Label("Add Environment", systemImage: "plus") }
                        .accessibilityHint("Adds a row: type the environment name, choose whether its body is indented and what each new line starts with, such as \\item followed by a space.")
                    Spacer()
                    Button("Conventional Rules") {
                        rows = EnvironmentEditingRules.conventional.rules
                        rules = EnvironmentEditingRules.conventional
                    }
                    .accessibilityHint("Restores the shipped rules: everything indented except document; list entries start with \\item.")
                }
                Text("Return after \\begin{…} starts the body on a new line with this indentation and text; the completion skeletons, Wrap in Environment and Re-indent follow the same rules. A starred environment uses its unstarred rule; verbatim bodies are never touched.")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
            }
        }
        .onAppear { rows = rules.rules }
        .onChange(of: rules) { _, new in
            // An outside change (Restore Defaults): reload unless the table
            // is mid-edit on a row the binding would drop.
            if new.normalized() != EnvironmentEditingRules(indentByDefault: new.indentByDefault, rules: rows).normalized() { rows = new.rules }
        }
        .onChange(of: rows) { _, _ in commit() }
    }

    private func commit() {
        let next = EnvironmentEditingRules(indentByDefault: rules.indentByDefault, rules: rows).normalized()
        if next != rules { rules = next }
    }
}
