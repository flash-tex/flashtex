import AppKit
import SwiftUI

/// The command palette's results list: a real `NSTableView` (brief §1 — the
/// palette list is an IDE-critical surface), 28pt rows in the Search
/// Everywhere anatomy: typed icon → title → dimmed context → right-aligned
/// shortcut. Full-width selection band; click runs the row; the arrow keys
/// stay in the query field (CommandPalette.swift) and drive `selectedID`.
struct PaletteResultsView: NSViewRepresentable {
    struct Row: Equatable {
        var id: String
        var icon: String
        var iconColor: NSColor
        var title: String
        var context: String
        var shortcut: String?
        /// Editor-key hint rows show a keyboard glyph and cannot run.
        var hint = false
        var accessibilityLabel: String?
    }

    var rows: [Row]
    var selectedID: String?
    /// Single click / Return on a row.
    var onRun: (String) -> Void
    /// Keyboard selection moved by the table itself (type-select).
    var onSelect: (String) -> Void

    func makeNSView(context: Context) -> NSScrollView {
        let table = NSTableView()
        table.headerView = nil
        table.rowHeight = DS.Row.paletteResult
        table.style = .plain
        table.selectionHighlightStyle = .regular
        table.allowsEmptySelection = true
        table.allowsMultipleSelection = false
        table.intercellSpacing = .zero
        table.backgroundColor = .clear
        table.focusRingType = .none
        table.setAccessibilityLabel("Palette results")
        let column = NSTableColumn(identifier: .init("main"))
        column.resizingMask = .autoresizingMask
        table.addTableColumn(column)
        table.delegate = context.coordinator
        table.dataSource = context.coordinator
        table.target = context.coordinator
        table.action = #selector(Coordinator.rowClicked(_:))
        context.coordinator.table = table

        let scroll = NSScrollView()
        scroll.documentView = table
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        let co = context.coordinator
        co.parent = self
        guard let table = co.table else { return }
        if co.rows != rows {
            co.rows = rows
            table.reloadData()
        }
        co.syncSelection(to: selectedID)
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    @MainActor
    final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate {
        var parent: PaletteResultsView
        var rows: [Row] = []
        weak var table: NSTableView?
        private var suppressSelectionCallback = false

        init(_ parent: PaletteResultsView) { self.parent = parent }

        func numberOfRows(in tableView: NSTableView) -> Int { rows.count }

        func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
            guard rows.indices.contains(row) else { return nil }
            let cell = (tableView.makeView(withIdentifier: PaletteCellView.reuseID, owner: nil) as? PaletteCellView) ?? PaletteCellView()
            cell.configure(with: rows[row])
            return cell
        }

        func tableView(_ tableView: NSTableView, rowViewForRow row: Int) -> NSTableRowView? {
            let view = (tableView.makeView(withIdentifier: TreeRowView.reuseID, owner: nil) as? TreeRowView) ?? TreeRowView()
            view.identifier = TreeRowView.reuseID
            return view
        }

        @objc func rowClicked(_ sender: Any?) {
            guard let table, table.clickedRow >= 0, rows.indices.contains(table.clickedRow) else { return }
            parent.onRun(rows[table.clickedRow].id)
        }

        func tableViewSelectionDidChange(_ notification: Notification) {
            guard !suppressSelectionCallback, let table, table.selectedRow >= 0,
                  rows.indices.contains(table.selectedRow) else { return }
            let id = rows[table.selectedRow].id
            if id != parent.selectedID { parent.onSelect(id) }
        }

        func syncSelection(to id: String?) {
            guard let table else { return }
            let index = rows.firstIndex { $0.id == id }
            suppressSelectionCallback = true
            defer { suppressSelectionCallback = false }
            if let index {
                if table.selectedRow != index {
                    table.selectRowIndexes([index], byExtendingSelection: false)
                    table.scrollRowToVisible(index)
                }
            } else if table.selectedRow >= 0 {
                table.deselectAll(nil)
            }
        }
    }
}

/// icon → title → dimmed context → spacer → key cap (rounded bordered mono).
final class PaletteCellView: NSTableCellView {
    static let reuseID = NSUserInterfaceItemIdentifier("PaletteCellView")
    private let iconView = NSImageView()
    private let titleField = NSTextField(labelWithString: "")
    private let contextField = NSTextField(labelWithString: "")
    private let capField = NSTextField(labelWithString: "")
    private let hintIcon = NSImageView()

    init() {
        super.init(frame: .zero)
        identifier = Self.reuseID
        for v in [iconView, titleField, contextField, capField, hintIcon] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            addSubview(v)
        }
        iconView.symbolConfiguration = .init(pointSize: 12, weight: .regular)
        titleField.font = DS.NSFonts.base
        titleField.lineBreakMode = .byTruncatingTail
        titleField.setContentCompressionResistancePriority(.defaultHigh, for: .horizontal)
        contextField.font = DS.NSFonts.secondary
        contextField.textColor = DS.Palette.textSecondary
        contextField.lineBreakMode = .byTruncatingTail
        contextField.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        capField.font = DS.NSFonts.secondaryMono
        capField.textColor = DS.Palette.textSecondary
        capField.alignment = .center
        capField.wantsLayer = true
        capField.layer?.cornerRadius = DS.Radius.control
        capField.layer?.borderWidth = DS.Size.hairline
        capField.setContentHuggingPriority(.required, for: .horizontal)
        capField.setContentCompressionResistancePriority(.required, for: .horizontal)
        hintIcon.symbolConfiguration = .init(pointSize: 11, weight: .regular)
        hintIcon.image = NSImage(systemSymbolName: "keyboard", accessibilityDescription: "editor key")
        NSLayoutConstraint.activate([
            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: DS.Space.l),
            iconView.centerYAnchor.constraint(equalTo: centerYAnchor),
            iconView.widthAnchor.constraint(equalToConstant: DS.Size.fileIcon + 2),
            titleField.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: DS.Space.m),
            titleField.centerYAnchor.constraint(equalTo: centerYAnchor),
            contextField.leadingAnchor.constraint(equalTo: titleField.trailingAnchor, constant: DS.Space.m),
            contextField.centerYAnchor.constraint(equalTo: centerYAnchor),
            hintIcon.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -DS.Space.l),
            hintIcon.centerYAnchor.constraint(equalTo: centerYAnchor),
            capField.trailingAnchor.constraint(equalTo: hintIcon.leadingAnchor, constant: -DS.Space.s),
            capField.centerYAnchor.constraint(equalTo: centerYAnchor),
            capField.leadingAnchor.constraint(greaterThanOrEqualTo: contextField.trailingAnchor, constant: DS.Space.m),
            capField.widthAnchor.constraint(greaterThanOrEqualToConstant: 28),
        ])
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError() }

    func configure(with row: PaletteResultsView.Row) {
        iconView.image = NSImage(systemSymbolName: row.icon, accessibilityDescription: nil)
        iconView.contentTintColor = row.iconColor
        titleField.stringValue = row.title
        titleField.textColor = DS.Palette.textPrimary
        contextField.stringValue = row.context
        capField.isHidden = row.shortcut == nil
        capField.stringValue = row.shortcut.map { " \($0) " } ?? ""
        capField.layer?.borderColor = DS.Palette.componentBorder.cgColor
        hintIcon.isHidden = !row.hint
        hintIcon.contentTintColor = DS.Palette.textTertiary
        setAccessibilityElement(true)
        setAccessibilityRole(.staticText)
        setAccessibilityLabel(row.accessibilityLabel ?? row.title)
    }
}
