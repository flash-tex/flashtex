import AppKit
import SwiftUI
import FlashTeXProtocol

/// The Problems list as an AppKit table that makes only the rows on screen
/// (APP-MAIN-THREAD). A SwiftUI `List` made every row's view at each update
/// (its table sizes all rows): a long book's 3,000 groups were 3,000 row
/// bodies per compile reply, so #1890 had to show only the first 200. Here
/// every group is a row, unselected rows have one height (their message on
/// one line; the selected row opens to its full text and details), and a new
/// model reloads the table, which asks for the visible rows' views alone.
///
/// The rows are `DiagnosticRow`s hosted in reused cells; the selection is
/// the panel's (`DiagnosticsPanelState.selection`, a group id), so it follows
/// its group across compiles. Return goes to the selected group's current
/// occurrence, Esc gives the keyboard back to the editor, ⌘C copies the
/// selection (or every problem) as text.
struct ProblemsTable: NSViewRepresentable {
    let model: ShellModel
    let problems: ProblemsModel
    /// The groups shown: `problems.groups(filter)`.
    let groups: [EditorDiagnostics.Group]
    let filter: RuntimeV1.Severity?
    let status: RuntimeV1.Status
    let panel: DiagnosticsPanelState
    /// `panel.selection`, read by the panel's body so a change updates the table.
    let selection: String?
    /// `ShellModel.problemsRevision`: the rows' model or line labels changed.
    let revision: Int

    /// Unselected rows: one line (`DiagnosticRow`'s minimum height) with the List's insets: 28 pt.
    static let rowHeight: CGFloat = DS.Row.problem - 2 * DS.Space.xxs + 2 * verticalInset
    static let verticalInset: CGFloat = DS.Space.xs
    static let horizontalInset: CGFloat = DS.Space.m

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeNSView(context: Context) -> NSScrollView {
        let table = ProblemsTableView()
        let column = NSTableColumn(identifier: .init("problem"))
        column.resizingMask = .autoresizingMask
        table.addTableColumn(column)
        table.headerView = nil
        table.style = .plain
        table.backgroundColor = .clear
        table.intercellSpacing = .zero
        table.rowSizeStyle = .custom
        table.rowHeight = Self.rowHeight
        table.allowsEmptySelection = true
        table.allowsMultipleSelection = false
        table.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
        table.gridStyleMask = []
        table.delegate = context.coordinator
        table.dataSource = context.coordinator
        table.setAccessibilityIdentifier(DiagnosticsListView.listIdentifier)
        table.setAccessibilityLabel(DiagnosticsListView.listLabel)
        let scroll = NSScrollView()
        scroll.documentView = table
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.borderType = .noBorder
        context.coordinator.table = table
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        context.coordinator.show(self)
    }

    @MainActor
    final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate {
        weak var table: ProblemsTableView?
        private var shown: ProblemsTable?
        private var selectedRow: Int?
        private var selectedHeight: CGFloat = ProblemsTable.rowHeight
        /// Set while the table's selection is set from the panel's (not the user's).
        private var syncing = false
        private static let cellID = NSUserInterfaceItemIdentifier("problems.row")

        func show(_ next: ProblemsTable) {
            guard let table else { return }
            let last = shown
            shown = next
            table.onReturn = { [weak model = next.model, panel = next.panel] in model?.goToSelectedOccurrence(panel: panel) }
            table.onEscape = { [weak model = next.model] in model?.returnKeyboardToEditor() }
            table.copyText = { [weak model = next.model, panel = next.panel] in model?.diagnosticsCopyText(panel: panel) ?? "" }
            next.panel.table = table
            table.setAccessibilityValue(next.problems.spokenSummary)
            // Another model, its line labels, the filter or the status: the
            // table asks again for the rows on screen (and only those).
            let reload = last.map { $0.revision != next.revision || $0.problems !== next.problems
                || $0.status != next.status || $0.filter != next.filter } ?? true
            let row = next.selection.flatMap { id in next.groups.firstIndex { $0.id == id } }
            if reload || row != selectedRow {
                selectedHeight = row.map { measure(next, row: $0, width: table.bounds.width) } ?? ProblemsTable.rowHeight
            }
            if reload {
                selectedRow = row
                table.reloadData()
            } else if row != selectedRow {
                // The selected row opens to its details, the last one closes.
                var changed = IndexSet()
                if let r = selectedRow, r < next.groups.count { changed.insert(r) }
                if let row { changed.insert(row) }
                selectedRow = row
                if !changed.isEmpty { table.noteHeightOfRows(withIndexesChanged: changed) }
            }
            let want = row.map { IndexSet(integer: $0) } ?? IndexSet()
            if table.selectedRowIndexes != want {
                syncing = true
                table.selectRowIndexes(want, byExtendingSelection: false)
                syncing = false
            }
        }

        /// The height of row `row` opened (selected): its content at the table's width.
        private func measure(_ t: ProblemsTable, row: Int, width: CGFloat) -> CGFloat {
            let sizing = NSHostingController(rootView: content(t, row: row).fixedSize(horizontal: false, vertical: true))
            let size = sizing.sizeThatFits(in: CGSize(width: max(width, 200), height: 10_000))
            return max(ProblemsTable.rowHeight, ceil(size.height))
        }

        private func content(_ t: ProblemsTable, row: Int) -> ProblemsRowContent {
            let g = t.groups[row], diags = t.problems.diagnostics
            return ProblemsRowContent(row: DiagnosticRow(group: g, occurrences: g.occurrences.map { diags[$0] }, total: diags.count,
                                                         status: t.status, panel: t.panel),
                                      model: t.model)
        }

        func numberOfRows(in tableView: NSTableView) -> Int { shown?.groups.count ?? 0 }

        func tableView(_ tableView: NSTableView, heightOfRow row: Int) -> CGFloat {
            row == selectedRow ? selectedHeight : ProblemsTable.rowHeight
        }

        func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
            guard let t = shown, t.groups.indices.contains(row) else { return nil }
            let cell: ProblemsRowCell
            if let reused = tableView.makeView(withIdentifier: Self.cellID, owner: nil) as? ProblemsRowCell {
                cell = reused
                cell.host.rootView = content(t, row: row)
            } else {
                cell = ProblemsRowCell(content(t, row: row))
                cell.identifier = Self.cellID
            }
            return cell
        }

        func tableViewSelectionDidChange(_ notification: Notification) {
            guard !syncing, let table, let t = shown else { return }
            let row = table.selectedRow
            let id = t.groups.indices.contains(row) ? t.groups[row].id : nil
            if t.panel.selection != id { t.panel.selection = id }
        }

        func tableViewColumnDidResize(_ notification: Notification) {
            guard let table, let t = shown, let row = selectedRow, t.groups.indices.contains(row) else { return }
            let h = measure(t, row: row, width: table.bounds.width)
            if h != selectedHeight { selectedHeight = h; table.noteHeightOfRows(withIndexesChanged: [row]) }
        }
    }
}

/// A row's SwiftUI content with the model it reads (a hosted view has no
/// environment of its own) and the List's insets.
struct ProblemsRowContent: View {
    let row: DiagnosticRow
    let model: ShellModel

    var body: some View {
        row.padding(.horizontal, ProblemsTable.horizontalInset).padding(.vertical, ProblemsTable.verticalInset)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .environment(model)
    }
}

/// A reused cell: one hosting view filling it.
final class ProblemsRowCell: NSTableCellView {
    let host: NSHostingView<ProblemsRowContent>

    init(_ content: ProblemsRowContent) {
        host = NSHostingView(rootView: content)
        super.init(frame: .zero)
        host.translatesAutoresizingMaskIntoConstraints = false
        host.sizingOptions = []
        addSubview(host)
        NSLayoutConstraint.activate([
            host.leadingAnchor.constraint(equalTo: leadingAnchor), host.trailingAnchor.constraint(equalTo: trailingAnchor),
            host.topAnchor.constraint(equalTo: topAnchor), host.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not from a nib") }
}

/// The table: Return, Esc and ⌘C as the panel's List had them.
final class ProblemsTableView: NSTableView {
    var onReturn: (() -> Void)?
    var onEscape: (() -> Void)?
    var copyText: (() -> String)?

    override func keyDown(with event: NSEvent) {
        switch event.keyCode {
        case 36, 76: onReturn?() // Return, keypad Enter
        case 53: onEscape?()
        default: super.keyDown(with: event)
        }
    }

    @objc func copy(_ sender: Any?) {
        guard let text = copyText?(), !text.isEmpty else { NSSound.beep(); return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}
