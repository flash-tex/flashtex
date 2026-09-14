import SwiftUI
import FlashTeXProtocol

/// The bottom Problems panel (mac-ui-redesign): a header with counts, a
/// severity filter and a close button, above the grouped diagnostics list
/// from DiagnosticsPanel.swift (`DiagnosticsListView`: selection, Return /
/// Esc / ⌘C, "N places", explanation lines and Fix… quick fixes). The list
/// receives the full `displayedDiagnostics` and only hides groups by
/// severity, so explanation and quick-fix indices stay those of the result.
/// When `compile_result.diagnostics` is empty, that list is the live v2
/// frame mapped through `asRuntimeV1` (`ShellModel.producerDiagnostics`);
/// when both sources list rows, runtime-v1 wins and the v2 sibling is not
/// concatenated. View > Toggle Problems (⌘⇧M) and the sidebar's Problems
/// rows show it.
struct ProblemsPanel: View {
    @Environment(ShellModel.self) var model

    static let identifier = "problems.panel"
    /// Split-pane bounds: the header plus two rows at least; the ideal shows ~5 grouped rows.
    static let minHeight: CGFloat = 120
    static let idealHeight: CGFloat = 260

    var body: some View {
        @Bindable var model = model
        // `problemsList` / `resultStatus` are assigned only when they change
        // (ShellModel); `displayedDiagnostics` reads `result`, which every
        // reply replaces, and this panel's List re-laid out with each one.
        let diags = model.problemsList
        let (errors, warnings, gaps) = EditorDiagnostics.counts(diags)
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                Label("Problems", systemImage: "exclamationmark.triangle").font(.caption.bold())
                if errors > 0 { Label("\(errors)", systemImage: "xmark.octagon.fill").foregroundStyle(.red).font(.caption) }
                if warnings > 0 { Label("\(warnings)", systemImage: "exclamationmark.triangle.fill").foregroundStyle(.orange).font(.caption) }
                if gaps > 0 {
                    Label("\(gaps) not implemented", systemImage: "puzzlepiece.extension").foregroundStyle(.secondary).font(.caption)
                        .help("Commands, packages or environments FlashTeX does not implement yet — not mistakes in the source")
                }
                if diags.isEmpty { Text("none").font(.caption).foregroundStyle(.secondary) }
                if let status = model.resultStatus, status != .ok {
                    Text(status == .recovered ? "recovered: preview shown with provisional rendering" : "compile failed: the previous preview is kept")
                        .font(.caption).foregroundStyle(.orange).lineLimit(1)
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
                } label: { Image(systemName: "xmark").font(.caption.bold()) }
                    .buttonStyle(.borderless)
                    .help("Hide the Problems panel (⌘⇧M shows it again)")
                    .accessibilityLabel("Hide Problems")
            }
            .padding(.horizontal, 10).padding(.vertical, 4)
            .background(.bar)
            Divider()
            if diags.isEmpty {
                ContentUnavailableView {
                    Label("No problems", systemImage: "checkmark.circle")
                } description: {
                    Text(model.resultStatus == nil ? "Compile results list their diagnostics here; the preview is never hidden by them." : "The last compile reported no diagnostics.")
                }
                .frame(maxWidth: .infinity, minHeight: 80, maxHeight: .infinity)
            } else {
                problemsList(diags)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Problems")
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
