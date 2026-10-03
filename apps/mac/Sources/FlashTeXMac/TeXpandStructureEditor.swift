import AppKit
import FlashTeXEditorCore

/// The context structure editor (PLAN M10b, owner addition): ⌃⌘T inside a
/// matrix, tabular, cases or align lays a grid over the environment — a
/// subview of the text view, framed from the TextKit 1 glyph rects, so it
/// scrolls with the text. Cells are fields (Tab/⇧Tab and Return move);
/// the toolbar adds and removes rows and columns (a tabular's column spec
/// follows) and switches the matrix type. ⌃⌘T again, Esc or Done writes the
/// grid back to the source as one undo step ("Edit Structure") with the
/// caret in the cell being edited. Revert closes without changing anything.
@MainActor
final class TeXpandStructureEditor: NSView, NSTextFieldDelegate {
    typealias T = TeXpand

    private weak var textView: CompletingTextView?
    let target: T.StructureTarget
    private(set) var document: T.StructureDocument
    /// The cell with focus: its row and its index in that row. Column
    /// actions map it to columns through the row's spans
    /// (`\multicolumn{n}` covers n).
    private(set) var focus: (row: Int, col: Int)
    private var fields: [[NSTextField]] = []
    private var gridView = NSGridView()
    private let typePopup = NSPopUpButton()
    private let stack = NSStackView()
    private let unit: String
    /// Called after it closes (the adapter forgets it).
    var onClose: () -> Void = {}

    init(target: T.StructureTarget, textView: CompletingTextView, focus: (row: Int, col: Int), unit: String) {
        self.target = target
        self.document = target.document
        self.textView = textView
        self.unit = unit
        let cells = target.document.cells
        let row = min(max(0, focus.row), max(0, cells.count - 1))
        self.focus = (row, min(max(0, focus.col), max(0, (cells.indices.contains(row) ? cells[row].count : 1) - 1)))
        super.init(frame: .zero)
        wantsLayer = true
        layer?.backgroundColor = NSColor.textBackgroundColor.cgColor
        layer?.borderColor = NSColor.controlAccentColor.cgColor
        layer?.borderWidth = 1.5
        layer?.cornerRadius = 6
        setAccessibilityRole(.group)
        setAccessibilityLabel("Structure editor for \(target.document.environment)")

        let toolbar = NSStackView()
        toolbar.orientation = .horizontal
        toolbar.spacing = 6
        if !target.provider.family.isEmpty {
            typePopup.addItems(withTitles: target.provider.family)
            typePopup.selectItem(withTitle: target.document.environment.replacingOccurrences(of: "*", with: ""))
            typePopup.target = self
            typePopup.action = #selector(switchType(_:))
            typePopup.setAccessibilityLabel("Environment type")
            toolbar.addArrangedSubview(typePopup)
        } else {
            toolbar.addArrangedSubview(NSTextField(labelWithString: target.document.environment))
        }
        for (title, label, action) in [("+ Row", "Add row below", #selector(addRow(_:))), ("− Row", "Remove row", #selector(removeRow(_:))),
                                       ("+ Col", "Add column right", #selector(addColumn(_:))), ("− Col", "Remove column", #selector(removeColumn(_:))),
                                       ("Revert", "Close without changes", #selector(revert(_:))), ("Done", "Write the grid to the source", #selector(done(_:)))] {
            let b = NSButton(title: title, target: self, action: action)
            b.bezelStyle = .rounded
            b.controlSize = .small
            b.setAccessibilityLabel(label)
            toolbar.addArrangedSubview(b)
        }
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 6
        stack.edgeInsets = NSEdgeInsets(top: 6, left: 6, bottom: 6, right: 6)
        stack.addArrangedSubview(toolbar)
        stack.addArrangedSubview(gridView)
        stack.translatesAutoresizingMaskIntoConstraints = false
        addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: leadingAnchor), stack.trailingAnchor.constraint(equalTo: trailingAnchor),
            stack.topAnchor.constraint(equalTo: topAnchor), stack.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
        rebuildGrid()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) is not supported") }

    override var isFlipped: Bool { true }

    // MARK: layout

    /// Over the environment's glyphs, at least as wide as its text.
    func place() {
        guard let tv = textView, let lm = tv.layoutManager, let tc = tv.textContainer else { return }
        let glyphs = lm.glyphRange(forCharacterRange: target.range, actualCharacterRange: nil)
        var rect = lm.boundingRect(forGlyphRange: glyphs, in: tc)
        rect.origin.x += tv.textContainerOrigin.x
        rect.origin.y += tv.textContainerOrigin.y
        layoutSubtreeIfNeeded()
        let fit = stack.fittingSize
        frame = NSRect(x: rect.minX, y: rect.minY, width: max(fit.width, min(rect.width, tv.bounds.width - rect.minX)), height: max(fit.height, rect.height))
    }

    /// One field per cell, each under the columns it covers: a
    /// `\multicolumn{n}` field is merged across n grid columns, so later
    /// cells line up with their own columns.
    private func rebuildGrid() {
        let replacement = NSGridView()
        replacement.columnSpacing = 4
        replacement.rowSpacing = 4
        stack.insertArrangedSubview(replacement, at: stack.arrangedSubviews.firstIndex(of: gridView) ?? 1)
        stack.removeArrangedSubview(gridView)
        gridView.removeFromSuperview()
        gridView = replacement
        let cells = document.cells
        let columns = document.columnCount
        fields = cells.enumerated().map { r, row in
            row.enumerated().map { j, text in
                let f = StructureCellField(string: text)
                f.font = textView?.font ?? .monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)
                f.delegate = self
                f.tag = r * 1000 + j
                f.onFocus = { [weak self] in self?.focus = (r, j) }
                let start = T.Grid.column(ofCell: j, in: row)
                let span = T.Grid.span(of: [text])
                f.setAccessibilityLabel(span > 1 ? "Row \(r + 1), columns \(start + 1) to \(start + span)" : "Row \(r + 1), column \(start + 1)")
                f.widthAnchor.constraint(greaterThanOrEqualToConstant: 60).isActive = true
                return f
            }
        }
        for (r, row) in fields.enumerated() {
            var views: [NSView] = []
            var merges: [NSRange] = []
            for (j, field) in row.enumerated() {
                let span = min(T.Grid.span(of: [cells[r][j]]), max(1, columns - views.count))
                if span > 1 { merges.append(NSRange(location: views.count, length: span)) }
                views.append(field)
                views += Array(repeating: NSGridCell.emptyContentView, count: span - 1)
            }
            views += Array(repeating: NSGridCell.emptyContentView, count: max(0, columns - views.count))
            gridView.addRow(with: views)
            for m in merges { gridView.mergeCells(inHorizontalRange: m, verticalRange: NSRange(location: r, length: 1)) }
        }
        place()
        focusCell(focus.row, focus.col)
    }

    /// The field of cell `j` in row `r` (tests).
    func field(_ r: Int, _ j: Int) -> NSTextField? {
        r >= 0 && r < fields.count && j >= 0 && j < fields[r].count ? fields[r][j] : nil
    }

    /// The grid column the field of cell `j` in row `r` starts in (tests).
    func gridColumn(_ r: Int, _ j: Int) -> Int? {
        guard let f = field(r, j), let column = gridView.cell(for: f)?.column else { return nil }
        return gridView.index(of: column)
    }

    /// Focuses cell `j` of row `r` (clamped).
    func focusCell(_ r: Int, _ j: Int) {
        guard !fields.isEmpty else { return }
        let row = min(max(0, r), fields.count - 1)
        guard !fields[row].isEmpty else { return }
        let col = min(max(0, j), fields[row].count - 1)
        focus = (row, col)
        window?.makeFirstResponder(fields[row][col])
    }

    /// Focuses the cell of row `r` under column `c`.
    func focusColumn(_ r: Int, _ c: Int) {
        let row = min(max(0, r), max(0, fields.count - 1))
        focusCell(row, document.cellIndex(atColumn: c, inRow: row))
    }

    /// The focused cell's first and last columns.
    private var focusColumns: (first: Int, last: Int) {
        let first = document.column(ofCell: focus.col, inRow: focus.row)
        return (first, first + document.span(ofCell: focus.col, inRow: focus.row) - 1)
    }

    // MARK: actions

    @objc func addRow(_ sender: Any?) {
        let column = focusColumns.first
        document.addRow(after: focus.row)
        focus = (focus.row + 1, document.cellIndex(atColumn: column, inRow: focus.row + 1))
        rebuildGrid()
    }
    @objc func removeRow(_ sender: Any?) {
        let column = focusColumns.first
        document.removeRow(focus.row)
        let row = min(focus.row, max(0, document.cells.count - 1))
        focus = (row, document.cellIndex(atColumn: column, inRow: row))
        rebuildGrid()
    }
    /// A column right of the focused cell (after the last column it spans).
    @objc func addColumn(_ sender: Any?) {
        let after = focusColumns.last
        document.addColumn(after: after)
        focus.col = document.cellIndex(atColumn: after + 1, inRow: focus.row)
        rebuildGrid()
    }
    /// Removes the focused cell's first column (a `\multicolumn` there narrows).
    @objc func removeColumn(_ sender: Any?) {
        let first = focusColumns.first
        document.removeColumn(first)
        focus.col = document.cellIndex(atColumn: first, inRow: focus.row)
        rebuildGrid()
    }
    @objc func switchType(_ sender: NSPopUpButton) {
        guard let name = sender.titleOfSelectedItem else { return }
        switchEnvironment(to: name)
    }
    @objc func done(_ sender: Any?) { close(apply: true) }
    @objc func revert(_ sender: Any?) { close(apply: false) }

    func setCell(_ r: Int, _ c: Int, _ text: String) {
        document.setCell(r, c, text)
        if r < fields.count, c < fields[r].count { fields[r][c].stringValue = text }
    }

    func switchEnvironment(to name: String) {
        let star = document.environment.hasSuffix("*") && !name.hasSuffix("*") && target.provider.name != "matrix" ? "*" : ""
        document.environment = name + star
        setAccessibilityLabel("Structure editor for \(document.environment)")
    }

    /// Writes the grid back (when `apply` and something changed) and
    /// returns focus to the source with the caret in the focused cell.
    func close(apply: Bool) {
        guard let tv = textView, superview != nil else { return }
        removeFromSuperview()
        tv.window?.makeFirstResponder(tv)
        if apply, document != target.document {
            let out = document.render(indent: target.indent, unit: unit)
            let row = min(focus.row, out.cellOffsets.count - 1)
            let caret = row >= 0 && focus.col < out.cellOffsets[row].count ? target.range.location + out.cellOffsets[row][focus.col] : target.range.location
            tv.replaceTeXpandStructure(target.range, with: out.text, caret: caret)
        } else {
            tv.setSelectedRange(NSRange(location: target.range.location, length: 0))
        }
        onClose()
    }

    // MARK: NSTextFieldDelegate

    func controlTextDidChange(_ note: Notification) {
        guard let f = note.object as? NSTextField else { return }
        document.setCell(f.tag / 1000, f.tag % 1000, f.stringValue)
    }

    func controlTextDidBeginEditing(_ note: Notification) {
        guard let f = note.object as? NSTextField else { return }
        focus = (f.tag / 1000, f.tag % 1000)
    }

    func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
        let (r, c) = (control.tag / 1000, control.tag % 1000)
        let cols = r < fields.count ? fields[r].count : 1
        switch selector {
        case #selector(NSResponder.insertTab(_:)):
            if c + 1 < cols { focusCell(r, c + 1) } else if r + 1 < fields.count { focusCell(r + 1, 0) } else { focusCell(0, 0) }
        case #selector(NSResponder.insertBacktab(_:)):
            if c > 0 { focusCell(r, c - 1) } else if r > 0 { focusCell(r - 1, fields[r - 1].count - 1) }
        case #selector(NSResponder.insertNewline(_:)):
            // The cell below in the same column, whatever spans either row has.
            if r + 1 < fields.count { focusColumn(r + 1, document.column(ofCell: c, inRow: r)) } else { addRow(nil) }
        case #selector(NSResponder.cancelOperation(_:)):
            close(apply: true)
        default:
            return false
        }
        return true
    }
}

/// A grid cell that reports gaining focus, so a click (not only typing)
/// makes it the cell the row and column actions act on.
final class StructureCellField: NSTextField {
    var onFocus: () -> Void = {}

    override func becomeFirstResponder() -> Bool {
        let ok = super.becomeFirstResponder()
        if ok { onFocus() }
        return ok
    }
}

extension CompletingTextView {
    /// The structure editor's write-back: one undo step, the caret at `caret`.
    func replaceTeXpandStructure(_ range: NSRange, with text: String, caret: Int) {
        breakUndoCoalescing()
        guard shouldChangeText(in: range, replacementString: text) else { return }
        textStorage?.replaceCharacters(in: range, with: text)
        didChangeText()
        undoManager?.setActionName("Edit Structure")
        setSelectedRange(NSRange(location: min(caret, (string as NSString).length), length: 0))
        breakUndoCoalescing()
    }
}

extension TeXpandEditor {
    /// ⌃⌘T: opens the structure editor on the grid around the caret, or
    /// closes (writing back) the open one. False when the caret is in no
    /// grid (or the editor is off), so the command opens the prompt.
    func toggleStructureEditor() -> Bool {
        if let open = structureEditor {
            open.close(apply: true)
            return true
        }
        ensureFresh()
        guard let settings = controller?.engine.settings, let storage = textViewForPrompt.textStorage else { return false }
        let text = storage.mutableString
        let caret = textViewForPrompt.selectedRange().location
        guard let target = TeXpand.structure(at: caret, in: text, providers: TeXpand.StructureProvider.enabled(settings)) else { return false }
        let editor = TeXpandStructureEditor(target: target, textView: textViewForPrompt,
                                            focus: Self.cell(at: caret, in: text, target: target), unit: EditorPreferences.shared.indentString)
        editor.onClose = { [weak self] in self?.structureEditor = nil }
        structureEditor = editor
        textViewForPrompt.addSubview(editor)
        editor.place()
        editor.focusCell(editor.focus.row, editor.focus.col)
        announce("Editing \(target.document.environment): Tab moves between cells, Escape returns to the source")
        return true
    }

    /// The cell the caret is in: `\\` before it counts rows, `&` since the
    /// last `\\` counts columns (at brace depth 0; rule rows skipped).
    static func cell(at caret: Int, in text: NSString, target: TeXpand.StructureTarget) -> (row: Int, col: Int) {
        let upTo = max(0, min(caret, NSMaxRange(target.range)) - target.range.location)
        let head = text.substring(with: NSRange(location: target.range.location, length: upTo))
        let parts = TeXpand.Grid.split(head, on: "\\\\")
        let row = max(0, parts.count - 1)
        let col = max(0, TeXpand.Grid.split(parts.last ?? "", on: "&").count - 1)
        return (row, col)
    }
}
