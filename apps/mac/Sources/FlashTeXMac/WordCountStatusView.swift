import SwiftUI
import FlashTeXProtocol

/// Status bar item + popover for GH68 (live word count and document
/// statistics). Deliberately self-contained: it reads only `activeText`,
/// `caretUTF16`/`caretLengthUTF16` (already updated live by the editor) and
/// the new `model.wordCount` model, and drives that model's own debounced
/// background scan via `.task`/`.onChange` here — nothing in `ShellModel`
/// besides the `wordCount` property itself, nothing in `ContentView` besides
/// one call to `WordCountStatusItem()`. No coupling to Grok/assistant UI.
struct WordCountStatusItem: View {
    @Environment(ShellModel.self) var model
    @State private var showPopover = false

    var body: some View {
        Button {
            showPopover = true
        } label: {
            Label(labelText, systemImage: "textformat.abc")
        }
        .buttonStyle(.plain)
        .help(helpText)
        .accessibilityIdentifier("status.wordCount")
        .popover(isPresented: $showPopover, arrowEdge: .top) {
            WordCountPopover()
                .environment(model)
        }
        // The debounced/background rescan is scheduled from here, not from
        // ShellModel, so this feature stays confined to this one view.
        .task(id: model.activePath) { model.wordCount.scheduleUpdate(documents: model.documents) }
        .onChange(of: model.chrome.editorRevision) { _, _ in model.wordCount.scheduleUpdate(documents: model.documents) } // throttled (ShellChrome): the scan is debounced anyway
        .onChange(of: model.documents.count) { _, _ in model.wordCount.scheduleUpdate(documents: model.documents) }
    }

    private var selectionCount: Int? {
        guard model.caretLengthUTF16 > 0 else { return nil }
        let range = NSRange(location: model.caretUTF16, length: model.caretLengthUTF16)
        return DocumentStatistics.wordCount(inSelection: model.activeText, utf16Range: range)
    }

    private var labelText: String {
        guard let total = model.wordCount.total?.totalWords else { return "…" }
        if let selected = selectionCount { return "\(selected) of \(total) words" }
        return "\(total) word\(total == 1 ? "" : "s")"
    }

    private var helpText: String {
        if model.wordCount.total?.totalWords == nil { return "Counting words…" }
        return "Word count across \(model.documents.count == 1 ? "this document" : "\(model.documents.count) open documents") — click for the breakdown by section"
    }
}

/// The per-section breakdown popover: total counts (body/header/caption
/// words, math blocks), then one row per heading in document order. A
/// document-set with more than one open file groups rows under a path
/// header (`SectionBreakdown.documentPath`); a single document does not.
struct WordCountPopover: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.m) {
            Text("Document Statistics").font(.headline)
            if let total = model.wordCount.total {
                Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 2) {
                    statRow("Words", "\(total.totalWords)")
                    statRow("  body", "\(total.bodyWords)")
                    statRow("  headers", "\(total.headerWords)")
                    statRow("  captions", "\(total.captionWords)")
                    statRow("Inline math", "\(total.inlineMath)")
                    statRow("Display math", "\(total.displayMath)")
                }
                .font(DS.Fonts.secondary)
                if model.wordCount.sections.isEmpty {
                    Text("No sections in this document.").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                } else {
                    Divider()
                    ScrollView {
                        VStack(alignment: .leading, spacing: DS.Space.xs) {
                            ForEach(Array(model.wordCount.sections.enumerated()), id: \.offset) { _, section in
                                sectionRow(section)
                            }
                        }
                    }
                    .frame(maxHeight: DS.Layout.popoverListMaxHeight)
                }
            } else {
                ProgressView().controlSize(.small)
                Text("Counting…").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
            }
        }
        .padding(DS.Space.l)
        .frame(minWidth: DS.Layout.popoverMinWidth)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Document statistics")
    }

    @ViewBuilder
    private func statRow(_ label: String, _ value: String) -> some View {
        GridRow {
            Text(label).foregroundStyle(label.hasPrefix("  ") ? Color.secondary : Color.primary)
            Text(value).monospacedDigit().gridColumnAlignment(.trailing)
        }
    }

    @ViewBuilder
    private func sectionRow(_ section: DocumentStatistics.SectionBreakdown) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: DS.Space.s) {
            if let path = section.documentPath {
                Text(path).font(DS.Fonts.header).foregroundStyle(DS.Colors.textSecondary)
            }
            Text(section.title)
                .lineLimit(1)
                .padding(.leading, CGFloat(max(0, section.level)) * DS.Space.m)
            Spacer(minLength: 8)
            Text("\(section.counts.totalWords)").monospacedDigit().foregroundStyle(.secondary)
        }
        .font(DS.Fonts.secondary)
    }
}
