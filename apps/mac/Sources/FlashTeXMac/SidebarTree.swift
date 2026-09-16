import AppKit
import SwiftUI

/// The sidebar's tree component: a real `NSOutlineView`
/// (context/PROMPT-appearance-overhaul.md §1.1 — `List` on macOS is not
/// lazy, has no selection API and no type-select), styled to the IntelliJ
/// tree: 24pt rows, full-width selection band in the muted JetBrains blue
/// that keeps the row's own text colour, hover wash, colour-coded type
/// icons, dimmed trailing detail. Rows are plain `NSTableCellView`s — no
/// SwiftUI per row. Context menus are `NSMenu` built on demand.
///
/// The component is presentation-only: rows come in as value structs and
/// clicks go out through callbacks, so the model seams (`switchOrNote`,
/// `reveal(outlineItem:)`, scaffold sheets) are exactly the ones the old
/// SwiftUI rows used.
struct SidebarTree: NSViewRepresentable {
    struct Row: Equatable, Sendable {
        var id: String
        var icon: String
        var iconColor: NSColor
        var title: String
        /// Dimmed title (closed includes, empty states).
        var dimmed = false
        /// Dimmed trailing detail (durable revision, line number).
        var trailing: String?
        /// The modified (unsaved) dot after the title.
        var modified = false
        /// Extra indentation steps (tree indent token per step).
        var indent = 0
        var tooltip: String?
        var accessibilityLabel: String?
        /// Non-interactive caption rows (empty states).
        var selectable = true
    }

    var rows: [Row]
    var selectedID: String?
    var onSelect: (String) -> Void
    /// Context menu items for a row id; empty = no menu.
    var menuItems: (String) -> [MenuItem] = { _ in [] }
    /// Autosave name for column/expansion state; also names the tree for
    /// accessibility.
    var accessibilityLabel: String

    struct MenuItem {
        var title: String
        var action: (() -> Void)?
        var separator = false
        static let divider = MenuItem(title: "", action: nil, separator: true)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let outline = TreeOutlineView()
        outline.coordinator = context.coordinator
        outline.headerView = nil
        outline.rowHeight = DS.Row.tree
        outline.indentationPerLevel = 0 // flat items; Row.indent drives the inset
        outline.style = .plain
        outline.selectionHighlightStyle = .regular
        outline.allowsEmptySelection = true
        outline.allowsMultipleSelection = false
        outline.intercellSpacing = .zero
        outline.backgroundColor = .clear
        outline.focusRingType = .none
        outline.setAccessibilityLabel(accessibilityLabel)
        let column = NSTableColumn(identifier: .init("main"))
        column.resizingMask = .autoresizingMask
        outline.addTableColumn(column)
        outline.outlineTableColumn = column
        outline.delegate = context.coordinator
        outline.dataSource = context.coordinator
        outline.target = context.coordinator
        outline.action = #selector(Coordinator.rowClicked(_:))
        context.coordinator.outline = outline

        let scroll = NSScrollView()
        scroll.documentView = outline
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        scroll.automaticallyAdjustsContentInsets = false
        scroll.contentInsets = .init(top: 0, left: 0, bottom: DS.Space.xs, right: 0)
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        let co = context.coordinator
        co.parent = self
        guard let outline = co.outline else { return }
        if co.rows != rows {
            co.rows = rows
            outline.reloadData()
        }
        co.syncSelection(to: selectedID)
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    @MainActor
    final class Coordinator: NSObject, NSOutlineViewDataSource, NSOutlineViewDelegate {
        var parent: SidebarTree
        var rows: [Row] = []
        weak var outline: NSOutlineView?
        private var suppressSelectionCallback = false

        init(_ parent: SidebarTree) { self.parent = parent }

        // Flat: every row is a root child, none expandable.
        func outlineView(_ outlineView: NSOutlineView, numberOfChildrenOfItem item: Any?) -> Int {
            item == nil ? rows.count : 0
        }

        func outlineView(_ outlineView: NSOutlineView, child index: Int, ofItem item: Any?) -> Any {
            index // items are row indices; Row values live in `rows`
        }

        func outlineView(_ outlineView: NSOutlineView, isItemExpandable item: Any) -> Bool { false }

        private func row(for item: Any) -> Row? {
            guard let index = item as? Int, rows.indices.contains(index) else { return nil }
            return rows[index]
        }

        func outlineView(_ outlineView: NSOutlineView, viewFor tableColumn: NSTableColumn?, item: Any) -> NSView? {
            guard let row = row(for: item) else { return nil }
            let cell = (outlineView.makeView(withIdentifier: TreeCellView.reuseID, owner: nil) as? TreeCellView) ?? TreeCellView()
            cell.configure(with: row)
            return cell
        }

        func outlineView(_ outlineView: NSOutlineView, rowViewForItem item: Any) -> NSTableRowView? {
            let view = (outlineView.makeView(withIdentifier: TreeRowView.reuseID, owner: nil) as? TreeRowView) ?? TreeRowView()
            view.identifier = TreeRowView.reuseID
            return view
        }

        func outlineView(_ outlineView: NSOutlineView, shouldSelectItem item: Any) -> Bool {
            row(for: item)?.selectable ?? false
        }

        /// Click anywhere on a row activates it (the old SwiftUI rows were
        /// buttons); selection sync alone would miss re-clicks on the
        /// already-selected row.
        @objc func rowClicked(_ sender: Any?) {
            guard let outline, outline.clickedRow >= 0,
                  let row = row(for: outline.item(atRow: outline.clickedRow) as Any),
                  row.selectable else { return }
            parent.onSelect(row.id)
        }

        /// Keyboard selection (arrows, type-select) activates too.
        func outlineViewSelectionDidChange(_ notification: Notification) {
            guard !suppressSelectionCallback, let outline,
                  outline.selectedRow >= 0,
                  let row = row(for: outline.item(atRow: outline.selectedRow) as Any) else { return }
            if row.id != parent.selectedID { parent.onSelect(row.id) }
        }

        func syncSelection(to id: String?) {
            guard let outline else { return }
            let index = rows.firstIndex { $0.id == id }
            let current = outline.selectedRow
            suppressSelectionCallback = true
            defer { suppressSelectionCallback = false }
            if let index {
                if current != index { outline.selectRowIndexes([index], byExtendingSelection: false) }
            } else if current >= 0 {
                outline.deselectAll(nil)
            }
        }

        func menu(forRowAt index: Int) -> NSMenu? {
            guard rows.indices.contains(index) else { return nil }
            let items = parent.menuItems(rows[index].id)
            guard !items.isEmpty else { return nil }
            let menu = NSMenu()
            for item in items {
                if item.separator {
                    menu.addItem(.separator())
                } else {
                    let mi = NSMenuItem(title: item.title, action: #selector(MenuTrampoline.fire), keyEquivalent: "")
                    let trampoline = MenuTrampoline(action: item.action ?? {})
                    mi.target = trampoline
                    mi.representedObject = trampoline // keep it alive with the item
                    menu.addItem(mi)
                }
            }
            return menu
        }
    }
}

/// Objective-C target for programmatic `NSMenuItem`s built from closures.
private final class MenuTrampoline: NSObject {
    let action: () -> Void
    init(action: @escaping () -> Void) { self.action = action }
    @objc func fire() { action() }
}

/// Outline view that asks the coordinator for a context menu at the clicked
/// row (`NSMenu` built at click time — the AppKit path the brief requires).
final class TreeOutlineView: NSOutlineView {
    weak var coordinator: SidebarTree.Coordinator?

    override func menu(for event: NSEvent) -> NSMenu? {
        let point = convert(event.locationInWindow, from: nil)
        let index = row(at: point)
        guard index >= 0 else { return super.menu(for: event) }
        return coordinator?.menu(forRowAt: index) ?? super.menu(for: event)
    }
}

/// Full-width selection band and hover wash in the Islands vocabulary; an
/// unfocused window's selection reads visibly weaker (§14).
final class TreeRowView: NSTableRowView {
    static let reuseID = NSUserInterfaceItemIdentifier("TreeRowView")
    private var hovering = false { didSet { if hovering != oldValue { needsDisplay = true } } }
    private var trackingArea: NSTrackingArea?

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let trackingArea { removeTrackingArea(trackingArea) }
        let area = NSTrackingArea(rect: bounds, options: [.mouseEnteredAndExited, .activeAlways, .inVisibleRect], owner: self)
        addTrackingArea(area)
        trackingArea = area
    }

    override func mouseEntered(with event: NSEvent) { hovering = true }
    override func mouseExited(with event: NSEvent) { hovering = false }

    override func drawSelection(in dirtyRect: NSRect) {
        let focused = window?.isKeyWindow ?? false
        (focused ? DS.NSColors.selectionFocused : DS.NSColors.selectionUnfocused).setFill()
        bounds.fill()
    }

    override func drawBackground(in dirtyRect: NSRect) {
        super.drawBackground(in: dirtyRect)
        if hovering, !isSelected {
            DS.NSColors.hover.setFill()
            bounds.fill()
        }
    }
}

/// icon → title → (modified dot) → spacer → dimmed trailing. Plain AppKit
/// views, laid out once and reused.
final class TreeCellView: NSTableCellView {
    static let reuseID = NSUserInterfaceItemIdentifier("TreeCellView")
    private let iconView = NSImageView()
    private let titleField = NSTextField(labelWithString: "")
    private let trailingField = NSTextField(labelWithString: "")
    private let dot = NSView()
    private var leadingConstraint: NSLayoutConstraint!

    init() {
        super.init(frame: .zero)
        identifier = Self.reuseID
        iconView.translatesAutoresizingMaskIntoConstraints = false
        iconView.symbolConfiguration = .init(pointSize: 12, weight: .regular)
        titleField.translatesAutoresizingMaskIntoConstraints = false
        titleField.font = DS.NSFonts.base
        titleField.lineBreakMode = .byTruncatingMiddle
        titleField.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        trailingField.translatesAutoresizingMaskIntoConstraints = false
        trailingField.font = DS.NSFonts.secondaryMono
        trailingField.textColor = DS.Palette.textTertiary
        trailingField.setContentHuggingPriority(.required, for: .horizontal)
        trailingField.setContentCompressionResistancePriority(.required, for: .horizontal)
        dot.translatesAutoresizingMaskIntoConstraints = false
        dot.wantsLayer = true
        dot.layer?.cornerRadius = DS.Size.modifiedDot / 2
        addSubview(iconView)
        addSubview(titleField)
        addSubview(dot)
        addSubview(trailingField)
        leadingConstraint = iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: DS.Space.m)
        NSLayoutConstraint.activate([
            leadingConstraint,
            iconView.centerYAnchor.constraint(equalTo: centerYAnchor),
            iconView.widthAnchor.constraint(equalToConstant: DS.Size.fileIcon + 2),
            titleField.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: DS.Space.s),
            titleField.centerYAnchor.constraint(equalTo: centerYAnchor),
            dot.leadingAnchor.constraint(equalTo: titleField.trailingAnchor, constant: DS.Space.s),
            dot.centerYAnchor.constraint(equalTo: centerYAnchor),
            dot.widthAnchor.constraint(equalToConstant: DS.Size.modifiedDot),
            dot.heightAnchor.constraint(equalToConstant: DS.Size.modifiedDot),
            trailingField.leadingAnchor.constraint(greaterThanOrEqualTo: dot.trailingAnchor, constant: DS.Space.s),
            trailingField.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -DS.Space.m),
            trailingField.centerYAnchor.constraint(equalTo: centerYAnchor),
        ])
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError() }

    func configure(with row: SidebarTree.Row) {
        leadingConstraint.constant = DS.Space.m + CGFloat(row.indent) * DS.Space.m
        iconView.image = NSImage(systemSymbolName: row.icon, accessibilityDescription: nil)
        iconView.contentTintColor = row.iconColor
        titleField.stringValue = row.title
        titleField.textColor = row.dimmed ? DS.Palette.textSecondary : DS.Palette.textPrimary
        trailingField.stringValue = row.trailing ?? ""
        dot.isHidden = !row.modified
        dot.layer?.backgroundColor = DS.Palette.statusModified.cgColor
        toolTip = row.tooltip
        setAccessibilityElement(true)
        setAccessibilityRole(.staticText)
        setAccessibilityLabel(row.accessibilityLabel ?? row.title)
    }
}
