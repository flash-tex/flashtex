import SwiftUI
import FlashTeXProtocol

/// The bottom Problems panel (mac-ui-redesign): a header with counts, a
/// severity filter and a close button, above the grouped diagnostics list
/// from DiagnosticsPanel.swift (`DiagnosticsListView`: selection, Return /
/// Esc / ⌘C, "N places", explanation lines and Fix… quick fixes). The list
/// receives the full `displayedDiagnostics` and only hides groups by
/// severity, so explanation and quick-fix indices stay those of the result.
/// View > Toggle Problems (⌘⇧M) and the sidebar's Problems rows show it.
struct ProblemsPanel: View {
    @Environment(ShellModel.self) var model

    static let identifier = "problems.panel"
    /// Split-pane bounds: the header plus two rows at least; the ideal opens a
    /// compact list rather than ~1/3 of the minimum window (audit #76). The
    /// 40 % cap in ContentView still stops a large window from swallowing the
    /// editor. A stored AppStorage height cannot be told from a user resize,
    /// so an existing one is left as-is.
    static let minHeight: CGFloat = DS.Layout.problemsMinHeight
    static let idealHeight: CGFloat = DS.Layout.problemsIdealHeight

    var body: some View {
        @Bindable var model = model
        // `problemsList` / `resultStatus` are assigned only when they change
        // (ShellModel); `displayedDiagnostics` reads `result`, which every
        // reply replaces, and this panel's List re-laid out with each one.
        let diags = model.problemsList
        let summary = EditorDiagnostics.summary(diags)
        let (errors, warnings, gaps) = EditorDiagnostics.counts(diags)
        VStack(spacing: 0) {
            HStack(spacing: DS.Space.m) {
                // Chips drop their zeros (UI pass 2), but VoiceOver reads every
                // bucket through summary(), so an all-gap document is never
                // announced as having no error count (the #76 complaint).
                HStack(spacing: DS.Space.m) {
                    Label("Problems", systemImage: "exclamationmark.triangle").font(DS.Fonts.header)
                    if errors > 0 { Label("\(errors)", systemImage: "xmark.octagon.fill").foregroundStyle(DS.Colors.severityError).font(DS.Fonts.secondary) }
                    if warnings > 0 { Label("\(warnings)", systemImage: "exclamationmark.triangle.fill").foregroundStyle(DS.Colors.severityWarning).font(DS.Fonts.secondary) }
                    if gaps > 0 {
                        Label("\(gaps) not implemented", systemImage: "puzzlepiece.extension").foregroundStyle(DS.Colors.textSecondary).font(DS.Fonts.secondary)
                            .help("Commands, packages or environments FlashTeX does not implement yet — not mistakes in the source")
                    }
                    if diags.isEmpty { Text("none").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary) }
                }
                .accessibilityElement(children: .ignore)
                .accessibilityLabel("Problems, \(summary)")
                if let status = model.resultStatus, status != .ok {
                    Text(status == .recovered ? "recovered: preview shown with provisional rendering" : "compile failed: the previous preview is kept")
                        .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityWarning).lineLimit(1)
                }
                Spacer()
                Picker("Show", selection: $model.problemsSeverityFilter) {
                    Text("All").tag(RuntimeV1.Severity?.none)
                    Text("Errors").tag(RuntimeV1.Severity?.some(.error))
                    Text("Warnings").tag(RuntimeV1.Severity?.some(.warning))
                }
                .pickerStyle(.segmented).labelsHidden().controlSize(.small).fixedSize()
                .help("Filter the list by severity; counts above are for every diagnostic")
                .accessibilityLabel("Problems severity filter")
                Button {
                    model.problemsVisible = false
                } label: { Image(systemName: "xmark").font(DS.Fonts.header) }
                    .buttonStyle(.borderless)
                    .help("Hide the Problems panel (⌘⇧M shows it again)")
                    .accessibilityLabel("Hide Problems")
            }
            .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xs)
            .background(.bar)
            Divider()
            if diags.isEmpty {
                ContentUnavailableView {
                    Label("No problems", systemImage: "checkmark.circle")
                } description: {
                    Text(model.resultStatus == nil ? "Compile results list their diagnostics here; the preview is never hidden by them." : "The last compile reported no diagnostics.")
                }
                .frame(maxWidth: .infinity, minHeight: DS.Layout.diagnosticsListMinHeight, maxHeight: .infinity)
            } else {
                problemsList(diags)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Problems, \(summary)")
        .accessibilityIdentifier(Self.identifier)
    }

    private func problemsList(_ diags: [RuntimeV1.Diagnostic]) -> some View {
        // Grouped rows with a selection, Return / Esc / ⌘C, "N places" and the
        // spoken group count/occurrence (DiagnosticsPanel.swift, mac-diagnostics-3).
        DiagnosticsListView(diagnostics: diags, panel: model.problemsPanel,
                            severityFilter: model.problemsSeverityFilter, showsHeader: false, maxHeight: .infinity)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
