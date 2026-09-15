import XCTest
import AppKit
import HostedWindows
@testable import FlashTeXAccessibility

/// The completion popup's spoken text, plus the same text read back through
/// the NSAccessibility protocol from an `NSTableView` configured the way the
/// shell's `CompletionPopup` is (label, per-row text field values). The
/// shell's own popup lives in the executable target; the requested diff in
/// the lane handoff applies `CompletionAccessibility` there.
final class CompletionAccessibilityTests: XCTestCase {
    typealias CA = CompletionAccessibility

    func testRowLabelsSpellKindAndOrigin() {
        XCTAssertEqual(CA.rowLabel(label: "\\section", kind: .command, detail: "supported by this compiler"),
                       "\\section, command, supported by this compiler")
        XCTAssertEqual(CA.rowLabel(label: "\\end{itemize}", kind: .environment, detail: "closes the open environment"),
                       "\\end{itemize}, environment, closes the open environment")
        XCTAssertEqual(CA.rowLabel(label: "sec:intro", kind: .reference, detail: "\\label in this document"),
                       "sec:intro, label, \\label in this document")
        XCTAssertEqual(CA.rowLabel(label: "knuth84", kind: .citation, detail: "\\bibitem in this document"),
                       "knuth84, citation, \\bibitem in this document")
        XCTAssertEqual(CA.rowLabel(label: "naïve", kind: .word, detail: "3× in this document"),
                       "naïve, word, 3× in this document")
        XCTAssertEqual(CA.Kind.allCases.map(\.rawValue), ["command", "environment", "reference", "citation", "word"],
                       "raw values mirror the shell's Completion.Kind cases")
        XCTAssertEqual(CA.Kind.allCases.map(\.spoken), ["command", "environment", "label", "citation", "word"])
    }

    func testAnnouncementsLeadWithNOfM() {
        XCTAssertEqual(CA.openedAnnouncement(total: 1), "1 completion")
        XCTAssertEqual(CA.openedAnnouncement(total: 7), "7 completions")
        XCTAssertEqual(CA.selectionAnnouncement(index: 2, total: 7, label: "\\section", kind: .command, detail: "supported by this compiler"),
                       "3 of 7: \\section, command, supported by this compiler")
        XCTAssertEqual(CA.listLabel, "Completions")
        XCTAssertTrue(CA.listHelp.contains("Return inserts"))
        XCTAssertTrue(CA.listHelp.contains("Escape closes"))
    }

    /// A table built like the shell's popup (one text-field column, label
    /// "Completions") exposes the list label and each row's spoken label
    /// through NSAccessibility without VoiceOver.
    @MainActor func testTableRowsReadBackThroughNSAccessibility() throws {
        struct Row { let label: String; let kind: CA.Kind; let detail: String }
        let rows = [Row(label: "\\section", kind: .command, detail: "supported by this compiler"),
                    Row(label: "\\end{itemize}", kind: .environment, detail: "closes the open environment"),
                    Row(label: "sec:intro", kind: .reference, detail: "\\label in this document")]
        final class Source: NSObject, NSTableViewDataSource, NSTableViewDelegate {
            var rows: [Row] = []
            func numberOfRows(in tableView: NSTableView) -> Int { rows.count }
            func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
                let f = NSTextField(labelWithString: rows[row].label + "  " + rows[row].detail)
                f.setAccessibilityLabel(CA.rowLabel(label: rows[row].label, kind: rows[row].kind, detail: rows[row].detail))
                return f
            }
        }
        let source = Source(); source.rows = rows
        let table = NSTableView(frame: NSRect(x: 0, y: 0, width: 420, height: 88))
        table.addTableColumn(NSTableColumn(identifier: .init("completion")))
        table.headerView = nil
        table.dataSource = source; table.delegate = source
        table.setAccessibilityLabel(CA.listLabel)
        table.setAccessibilityHelp(CA.listHelp)
        let scroll = NSScrollView(frame: table.frame); scroll.documentView = table
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 420, height: 88), styleMask: [.borderless], backing: .buffered, defer: false)
        window.contentView?.addSubview(scroll)
        window.orderFrontRegardless() // never key
        defer { window.orderOut(nil) }
        table.reloadData()
        window.contentView?.layoutSubtreeIfNeeded()
        table.display()
        table.selectRowIndexes(IndexSet(integer: 1), byExtendingSelection: false)
        XCTAssertEqual(table.accessibilityRole(), .table)
        XCTAssertEqual(table.accessibilityLabel(), "Completions")
        XCTAssertEqual(table.accessibilityHelp(), CA.listHelp)
        // AppKit answers a table's children with private `NSTableRow` proxies
        // that implement the attribute API, not the Swift protocol getters
        // (and `accessibilityRows()` traps bridging them), so rows and their
        // cells are read through `accessibilityAttributeValue`.
        func legacy(_ o: AnyObject, _ a: NSAccessibility.Attribute) -> Any? { (o as? NSObject)?.accessibilityAttributeValue(a) }
        let children = (table.accessibilityChildren() as? [AnyObject]) ?? []
        let axRows = children.filter { (legacy($0, .role) as? String) == NSAccessibility.Role.row.rawValue }
        XCTAssertEqual(axRows.count, 3)
        for (i, row) in axRows.enumerated() {
            let cells = (legacy(row, .children) as? [AnyObject]) ?? []
            XCTAssertEqual(cells.count, 1, "row \(i) has one cell")
            // The cell proxy speaks the text field's label as its description
            // (what VoiceOver reads when the row is selected); the static-text
            // child underneath carries the visible text as its value.
            XCTAssertEqual(legacy(cells[0], .role) as? String, NSAccessibility.Role.cell.rawValue)
            XCTAssertEqual(legacy(cells[0], .description) as? String,
                           CA.rowLabel(label: rows[i].label, kind: rows[i].kind, detail: rows[i].detail))
            let texts = (legacy(cells[0], .children) as? [AnyObject]) ?? []
            XCTAssertEqual(texts.count, 1, "row \(i)")
            XCTAssertEqual(texts.first.flatMap { legacy($0, .role) as? String }, NSAccessibility.Role.staticText.rawValue)
            XCTAssertEqual(texts.first.flatMap { legacy($0, .value) as? String }, rows[i].label + "  " + rows[i].detail)
            XCTAssertEqual(legacy(row, .index) as? Int, i)
        }
        let selected = (legacy(table, .selectedRows) as? [AnyObject]) ?? []
        XCTAssertEqual(selected.count, 1)
        XCTAssertEqual(selected.first.flatMap { legacy($0, .index) as? Int }, 1)
        XCTAssertEqual(selected.first.flatMap { legacy($0, .selected) as? Bool }, true)
        XCTAssertEqual(CA.selectionAnnouncement(index: 1, total: 3, label: rows[1].label, kind: rows[1].kind, detail: rows[1].detail),
                       "2 of 3: \\end{itemize}, environment, closes the open environment")
    }
}
