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
        /// Folder rows (the Project tree's tree mode, ProjectFileTree.swift):
        /// non-nil makes the row an expandable disclosure row holding these
        /// rows. Clicking one toggles it; it never reaches `onSelect`.
        var children: [Row]? = nil
    }

    var rows: [Row]
    var selectedID: String?
    var onSelect: (String) -> Void
    /// Context menu items for a row id; empty = no menu.
    var menuItems: (String) -> [MenuItem] = { _ in [] }
    /// Context menu for a right-click on the tree's empty space (no row under
    /// the pointer); empty = AppKit's default (none).
    var backgroundMenuItems: () -> [MenuItem] = { [] }
    /// Autosave name for column/expansion state; also names the tree for
    /// accessibility.
    var accessibilityLabel: String
    /// Drag-and-drop moves (the project tree; the outline leaves these at
    /// their defaults, so nothing there drags). `dragPath`: the rooted file a
    /// row is dragged as, nil for rows that cannot move. `dropFolder`: the
    /// folder dropping `path` on row `id` (nil id = the tree's empty space)
    /// would land in, nil to refuse the drop. `onMove`: the accepted drop.
    var dragPath: (String) -> String? = { _ in nil }
    var dropFolder: (_ path: String, _ rowID: String?) -> String? = { _, _ in nil }
    var onMove: (_ path: String, _ folder: String) -> Void = { _, _ in }
    /// Folder expansion (rows with `children`): `expandedIDs` is asked for the
    /// folder ids to expand only when `expansionScope` (the project) changes,
    /// never on an ordinary update; every expand/collapse is reported through
    /// `onExpansionChange` so the caller can persist it. A reported set keeps
    /// only the folders in the tree now and the ids `keepsExpansion` vouches
    /// for (folders on disk the tree has not listed yet), so ids of deleted or
    /// renamed folders are pruned. The folders holding `selectedID` expand
    /// whenever the selection moves.
    var expansionScope = ""
    var expandedIDs: (_ scope: String) -> Set<String> = { _ in [] }
    var keepsExpansion: (_ id: String) -> Bool = { _ in false }
    var onExpansionChange: (Set<String>) -> Void = { _ in }

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
        outline.indentationPerLevel = 0 // flat items; Row.indent drives the inset (tree rows: updateNSView)
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
        // Row moves are local drags only (never a file promise to Finder).
        outline.registerForDraggedTypes([Coordinator.rowPasteboardType])
        outline.setDraggingSourceOperationMask(.move, forLocal: true)
        outline.setDraggingSourceOperationMask([], forLocal: false)
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
        let scopeChanged = co.expansionScope != expansionScope
        if scopeChanged {
            co.expansionScope = expansionScope
            co.expanded = expandedIDs(expansionScope)
        }
        if co.rows != rows || scopeChanged { co.reload(rows) }
        co.syncSelection(to: selectedID)
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    @MainActor
    final class Coordinator: NSObject, NSOutlineViewDataSource, NSOutlineViewDelegate {
        /// An outline item: one row, its child items (folders), its parent.
        /// Reference identity is what `NSOutlineView` tracks items by.
        final class Node: NSObject {
            let row: Row
            private(set) var children: [Node] = []
            private(set) weak var parent: Node?
            var isFolder: Bool { row.children != nil }

            init(_ row: Row, parent: Node?, index: inout [String: Node]) {
                self.row = row
                self.parent = parent
                super.init()
                index[row.id] = self
                children = (row.children ?? []).map { Node($0, parent: self, index: &index) }
            }
        }

        var parent: SidebarTree
        var rows: [Row] = []
        private(set) var roots: [Node] = []
        private(set) var nodes: [String: Node] = [:]
        /// Any folder rows: the outline indents by level and shows disclosure
        /// triangles; otherwise it is the flat list `Row.indent` lays out.
        private(set) var hierarchical = false
        var expansionScope: String?
        var expanded: Set<String> = []
        weak var outline: NSOutlineView?
        private var suppressSelectionCallback = false
        private var suppressExpansionCallback = false
        /// The last `selectedID` synced: a folder the user selected (to
        /// expand it from the keyboard) stays selected until it changes.
        private var lastSyncedID: String??
        /// The last selection whose folders were revealed (in tree mode).
        private var lastRevealedID: String??

        init(_ parent: SidebarTree) { self.parent = parent }

        func reload(_ newRows: [Row]) {
            guard let outline else { return }
            let keptFolder = (outline.selectedRow >= 0 ? outline.item(atRow: outline.selectedRow) as? Node : nil)
                .flatMap { $0.isFolder ? $0.row.id : nil }
            let wasHierarchical = hierarchical
            rows = newRows
            var index: [String: Node] = [:]
            roots = newRows.map { Node($0, parent: nil, index: &index) }
            nodes = index
            hierarchical = newRows.contains { $0.children != nil }
            if hierarchical != wasHierarchical { lastRevealedID = nil } // a mode switch reveals the selection again
            outline.indentationPerLevel = hierarchical ? DS.Space.l : 0
            suppressExpansionCallback = true
            defer { suppressExpansionCallback = false }
            outline.reloadData()
            for root in roots { restoreExpansion(root) }
            if let keptFolder, let node = nodes[keptFolder] {
                let row = outline.row(forItem: node)
                if row >= 0 {
                    suppressSelectionCallback = true
                    outline.selectRowIndexes([row], byExtendingSelection: false)
                    suppressSelectionCallback = false
                }
            }
        }

        /// Re-expands `node` (and, under it, the folders that were expanded
        /// when it was last collapsed) from `expanded`.
        private func restoreExpansion(_ node: Node) {
            guard node.isFolder, expanded.contains(node.row.id), let outline else { return }
            outline.expandItem(node)
            for child in node.children { restoreExpansion(child) }
        }

        func outlineView(_ outlineView: NSOutlineView, numberOfChildrenOfItem item: Any?) -> Int {
            guard let item else { return roots.count }
            return (item as? Node)?.children.count ?? 0
        }

        func outlineView(_ outlineView: NSOutlineView, child index: Int, ofItem item: Any?) -> Any {
            guard let item else { return roots[index] }
            return (item as! Node).children[index] // swiftlint:disable:this force_cast
        }

        func outlineView(_ outlineView: NSOutlineView, isItemExpandable item: Any) -> Bool {
            (item as? Node)?.isFolder ?? false
        }

        private func row(for item: Any) -> Row? { (item as? Node)?.row }

        func outlineView(_ outlineView: NSOutlineView, viewFor tableColumn: NSTableColumn?, item: Any) -> NSView? {
            guard let row = row(for: item) else { return nil }
            let cell = (outlineView.makeView(withIdentifier: TreeCellView.reuseID, owner: nil) as? TreeCellView) ?? TreeCellView()
            cell.configure(with: row, hierarchical: hierarchical)
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

        // MARK: expansion

        /// Reports `expanded` to the caller, pruned of folders that are gone.
        private func reportExpansion() {
            expanded = expanded.filter { nodes[$0]?.isFolder == true || parent.keepsExpansion($0) }
            parent.onExpansionChange(expanded)
        }

        func outlineViewItemDidExpand(_ notification: Notification) {
            guard !suppressExpansionCallback, let node = notification.userInfo?["NSObject"] as? Node else { return }
            expanded.insert(node.row.id)
            // Folders under it that were open when it was collapsed open again.
            suppressExpansionCallback = true
            for child in node.children { restoreExpansion(child) }
            suppressExpansionCallback = false
            reportExpansion()
        }

        /// The folder being collapsed: AppKit collapses the open folders
        /// under it first, posting a collapse for each, but keeps their state
        /// for the next expand -- so must `expanded` (and what persists).
        private var collapsing: Node?

        func outlineViewItemWillCollapse(_ notification: Notification) {
            guard !suppressExpansionCallback, collapsing == nil,
                  let node = notification.userInfo?["NSObject"] as? Node else { return }
            collapsing = node
        }

        func outlineViewItemDidCollapse(_ notification: Notification) {
            guard !suppressExpansionCallback, let node = notification.userInfo?["NSObject"] as? Node else { return }
            if let collapsing, collapsing !== node { return } // a folder inside the one collapsing
            collapsing = nil
            expanded.remove(node.row.id)
            reportExpansion()
        }

        /// Expands every folder holding `id`, outermost first, so its row is
        /// visible; reports the change when one had to open.
        private func reveal(_ id: String) {
            guard let outline, let node = nodes[id] else { return }
            var chain: [Node] = []
            var p = node.parent
            while let folder = p { chain.insert(folder, at: 0); p = folder.parent }
            var changed = false
            suppressExpansionCallback = true
            for folder in chain where !outline.isItemExpanded(folder) {
                outline.expandItem(folder)
                changed = expanded.insert(folder.row.id).inserted || changed
            }
            suppressExpansionCallback = false
            if changed { reportExpansion() }
        }

        /// Click anywhere on a row activates it (the old SwiftUI rows were
        /// buttons); selection sync alone would miss re-clicks on the
        /// already-selected row. A folder row toggles instead (a click on its
        /// disclosure triangle is the triangle's own).
        @objc func rowClicked(_ sender: Any?) {
            guard let outline else { return }
            clicked(row: outline.clickedRow, at: NSApp.currentEvent.map { outline.convert($0.locationInWindow, from: nil) })
        }

        /// The click on row `index` at `point` (outline coordinates; nil when
        /// unknown). Split from the action so tests can drive it: AppKit only
        /// sets `clickedRow` inside its own mouse tracking.
        func clicked(row index: Int, at point: NSPoint?) {
            guard let outline, index >= 0,
                  let node = outline.item(atRow: index) as? Node,
                  node.row.selectable else { return }
            if node.isFolder {
                if let point, outline.frameOfOutlineCell(atRow: index).contains(point) { return }
                if outline.isItemExpanded(node) { outline.collapseItem(node) } else { outline.expandItem(node) }
                return
            }
            parent.onSelect(node.row.id)
        }

        /// Keyboard selection (arrows, type-select) activates too; a folder
        /// only takes the selection (Right/Left arrows expand/collapse it).
        func outlineViewSelectionDidChange(_ notification: Notification) {
            guard !suppressSelectionCallback, let outline,
                  outline.selectedRow >= 0,
                  let node = outline.item(atRow: outline.selectedRow) as? Node, !node.isFolder else { return }
            if node.row.id != parent.selectedID { parent.onSelect(node.row.id) }
        }

        func syncSelection(to id: String?) {
            guard let outline else { return }
            let moved = lastSyncedID != .some(id)
            lastSyncedID = .some(id)
            if hierarchical, lastRevealedID != .some(id), let id {
                reveal(id)
                lastRevealedID = .some(id)
            }
            let current = outline.selectedRow
            // A folder the user selected keeps the selection until the active
            // row itself changes.
            if !moved, current >= 0, (outline.item(atRow: current) as? Node)?.isFolder == true { return }
            let index = id.flatMap { nodes[$0] }.map { outline.row(forItem: $0) } ?? -1
            suppressSelectionCallback = true
            defer { suppressSelectionCallback = false }
            if index >= 0 {
                if current != index { outline.selectRowIndexes([index], byExtendingSelection: false) }
                if moved && hierarchical { outline.scrollRowToVisible(index) }
            } else if current >= 0 {
                outline.deselectAll(nil)
            }
        }

        // MARK: drag-and-drop moves

        /// Private pasteboard type carrying the dragged row's rooted path.
        static let rowPasteboardType = NSPasteboard.PasteboardType("dev.flashtex.project-tree-path")

        func outlineView(_ outlineView: NSOutlineView, pasteboardWriterForItem item: Any) -> NSPasteboardWriting? {
            guard let row = row(for: item), row.selectable, let path = parent.dragPath(row.id) else { return nil }
            let pb = NSPasteboardItem()
            pb.setString(path, forType: Self.rowPasteboardType)
            return pb
        }

        /// The dragged path when the drag started in this tree.
        private func draggedPath(_ info: NSDraggingInfo) -> String? {
            guard let source = info.draggingSource as? NSOutlineView, source === outline else { return nil }
            return info.draggingPasteboard.string(forType: Self.rowPasteboardType)
        }

        /// A drop is always *on* a row or on the tree itself (the root) —
        /// never between rows; `validateDrop` retargets AppKit's insertion-gap
        /// proposal accordingly. Flat: a file row stands for its folder. Tree:
        /// the target is the folder row itself, or the folder holding the file
        /// row the pointer is over (that folder is what highlights).
        func outlineView(_ outlineView: NSOutlineView, validateDrop info: NSDraggingInfo, proposedItem item: Any?, proposedChildIndex index: Int) -> NSDragOperation {
            guard let path = draggedPath(info) else { return [] }
            let target = dropTarget(proposed: item as? Node, childIndex: index)
            guard parent.dropFolder(path, target?.row.id) != nil else { return [] }
            outlineView.setDropItem(target, dropChildIndex: NSOutlineViewDropOnItemIndex)
            return .move
        }

        func outlineView(_ outlineView: NSOutlineView, acceptDrop info: NSDraggingInfo, item: Any?, childIndex index: Int) -> Bool {
            guard let path = draggedPath(info),
                  let folder = parent.dropFolder(path, dropTarget(proposed: item as? Node, childIndex: index)?.row.id) else { return false }
            parent.onMove(path, folder)
            return true
        }

        private func dropTarget(proposed: Node?, childIndex: Int) -> Node? {
            guard hierarchical, let proposed else { return proposed }
            if childIndex == NSOutlineViewDropOnItemIndex, !proposed.isFolder { return proposed.parent }
            return proposed
        }

        func menu(forRowAt index: Int) -> NSMenu? {
            guard let node = outline?.item(atRow: index) as? Node else { return nil }
            return menu(items: parent.menuItems(node.row.id))
        }

        func backgroundMenu() -> NSMenu? { menu(items: parent.backgroundMenuItems()) }

        private func menu(items: [MenuItem]) -> NSMenu? {
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
/// row, or for the tree's empty space below the rows (`NSMenu` built at click
/// time — the AppKit path the brief requires).
final class TreeOutlineView: NSOutlineView {
    weak var coordinator: SidebarTree.Coordinator?

    override func menu(for event: NSEvent) -> NSMenu? {
        let point = convert(event.locationInWindow, from: nil)
        let index = row(at: point)
        guard index >= 0 else { return coordinator?.backgroundMenu() ?? super.menu(for: event) }
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

    func configure(with row: SidebarTree.Row, hierarchical: Bool = false) {
        // Tree rows sit after the outline's own disclosure column, which
        // already insets them.
        leadingConstraint.constant = (hierarchical ? DS.Space.xxs : DS.Space.m) + CGFloat(row.indent) * DS.Space.m
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
