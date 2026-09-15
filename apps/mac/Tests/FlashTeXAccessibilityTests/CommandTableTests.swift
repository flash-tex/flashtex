import XCTest
@testable import FlashTeXAccessibility

/// The command table must match the README's "Keyboard shortcuts" table exactly
/// (both directions), the menu wiring in the shell source, and the focus order
/// must describe the pane order `ContentView.swift` actually builds.
final class CommandTableTests: XCTestCase {
    static let macRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    static let readme = macRoot.appendingPathComponent("README.md")
    static let shellSources = macRoot.appendingPathComponent("Sources/FlashTeXMac")

    struct ReadmeRow: Equatable {
        var shortcuts: [String]
        var action: String
    }

    /// Rows of the README table: shortcut cell split on " / ", action cell.
    func readmeRows() throws -> [ReadmeRow] {
        let text = try String(contentsOf: Self.readme, encoding: .utf8)
        guard let section = text.range(of: "## Keyboard shortcuts") else { throw XCTSkip("README has no shortcuts table") }
        var out: [ReadmeRow] = []
        for line in text[section.upperBound...].split(separator: "\n", omittingEmptySubsequences: false) {
            if line.hasPrefix("## ") { break }
            guard line.hasPrefix("|") else { continue }
            let cells = line.split(separator: "|", omittingEmptySubsequences: false).map { $0.trimmingCharacters(in: .whitespaces) }
            guard cells.count >= 3, cells[1] != "Shortcut", !cells[1].hasPrefix("---") else { continue }
            out.append(ReadmeRow(shortcuts: cells[1].components(separatedBy: " / "), action: cells[2]))
        }
        return out
    }

    func readmeShortcuts() throws -> [String] { try readmeRows().flatMap(\.shortcuts) }

    func testCommandTableMatchesREADMEShortcuts() throws {
        let readme = try readmeShortcuts()
        XCTAssertGreaterThan(readme.count, 15, "table parsed")
        XCTAssertEqual(Set(readme).count, readme.count, "README shortcuts are unique")
        let table = AccessibilityCommand.allShortcuts
        XCTAssertEqual(Set(readme).subtracting(table), [], "README shortcuts missing from the command table")
        XCTAssertEqual(table.subtracting(readme), [], "command-table shortcuts not documented in the README")
        // Each shortcut belongs to exactly one command.
        let owners = AccessibilityCommand.entries.flatMap { e in e.shortcuts.map { ($0, e.command) } }
        XCTAssertEqual(owners.count, Set(owners.map(\.0)).count)
    }

    /// Every command is one README row (all of its shortcuts in that row) and
    /// every README row is claimed by at least one command: a new menu item
    /// documented in the README without a table entry fails here, and so does
    /// a table entry nobody documented.
    func testEveryCommandHasAREADMERowAndEveryRowACommand() throws {
        let rows = try readmeRows()
        var rowOf: [AccessibilityCommand: Int] = [:]
        for e in AccessibilityCommand.entries {
            let matching = rows.indices.filter { i in e.shortcuts.allSatisfy { rows[i].shortcuts.contains($0) } }
            XCTAssertEqual(matching.count, 1, "\(e.command) \(e.shortcuts) should sit in exactly one README row")
            if let i = matching.first { rowOf[e.command] = i }
        }
        let claimed = Set(rowOf.values)
        for (i, row) in rows.enumerated() {
            XCTAssertTrue(claimed.contains(i), "README row \(row.shortcuts) has no AccessibilityCommand")
        }
        // The row must name what the help calls the command (its title's first
        // word appears in the row's shortcut or action cell), so the help text
        // and the README agree on what each shortcut is for.
        for e in AccessibilityCommand.entries {
            guard let i = rowOf[e.command] else { continue }
            let first = e.title.split(separator: " ").first.map(String.init)?.lowercased() ?? ""
            let haystack = (rows[i].shortcuts.joined(separator: " ") + " " + rows[i].action).lowercased()
            XCTAssertTrue(haystack.contains(first), "\(e.command): README says “\(rows[i].action)”, help says “\(e.title)”")
        }
    }

    // MARK: menu wiring

    struct WiredItem: Equatable {
        var title: String
        var menu: String
        /// README spelling ("⌘⇧K") or nil when the item has no key equivalent.
        var shortcut: String?
    }

    /// Files whose `Commands` bodies wire menu items: the app, `Navigation.swift`
    /// (`CommandMenu("Navigate")`), `EditorMenu.swift` (`CommandMenu("Editor")`) and `ProjectSearchPanel.swift`
    /// (`ProjectSearchCommands`, Edit = `after: .textEditing`). In the panel
    /// file only the text from its `Commands` type on is read, so the
    /// window's own buttons (Search, Next Match ⌘G) are not menu items.
    static let commandFiles = ["FlashTeXMacApp.swift", "Navigation.swift", "ProjectSearchPanel.swift", "DiagnosticsPanel.swift", "CitationRename.swift", "EditorFind.swift", "EditorMenu.swift"]

    /// `Button("Title")` items with their `keyboardShortcut` and enclosing
    /// menu from `FlashTeXMacApp.swift` (File = `replacing: .newItem`,
    /// Edit = `after: .pasteboard` / `.textEditing`), `Navigation.swift`
    /// (`CommandMenu("Navigate")`) and `ProjectSearchPanel.swift`.
    func wiredItems() throws -> [WiredItem] {
        var out: [WiredItem] = []
        for file in Self.commandFiles {
            var text = try String(contentsOf: Self.shellSources.appendingPathComponent(file), encoding: .utf8)
            if file != "FlashTeXMacApp.swift", let r = text.range(of: ": Commands {") { text = String(text[r.lowerBound...]) }
            var menu = "?"
            let lines = text.components(separatedBy: "\n")
            var i = 0
            while i < lines.count {
                let line = lines[i]
                if line.contains("CommandGroup(replacing: .newItem)") || line.contains("CommandGroup(replacing: .printItem)") { menu = "File" }
                else if line.contains("CommandGroup(after: .pasteboard)") || line.contains("CommandGroup(after: .textEditing)") { menu = "Edit" }
                else if line.contains("CommandMenu(\"Navigate\")") { menu = "Navigate" }
                else if line.contains("CommandMenu(\"Editor\")") { menu = "Editor" }
                else if line.contains("CommandGroup(after: .sidebar)") || line.contains("CommandGroup(after: .toolbar)") { menu = "View" }
                else if line.contains("CommandGroup(replacing: .help)") || line.contains("CommandGroup(after: .help)") { menu = "Help" }
                else if line.contains(" Window(\"") || line.contains("WindowGroup(\"") { menu = "?" }
                if let r = line.range(of: "Button(\""), let end = line.range(of: "\")", range: r.upperBound..<line.endIndex) {
                    let raw = String(line[r.upperBound..<end.lowerBound])
                    let title = raw.replacingOccurrences(of: "\\\\", with: "\\")
                    // Modifiers follow on the next lines until the next Button/Divider.
                    var shortcut: String?
                    var j = i + 1
                    while j < lines.count, !lines[j].contains("Button("), !lines[j].contains("Divider()"), !lines[j].contains("CommandGroup"), !lines[j].contains("CommandMenu") {
                        if let s = Self.spell(keyboardShortcutLine: lines[j]) { shortcut = s }
                        j += 1
                    }
                    out.append(WiredItem(title: title, menu: menu, shortcut: shortcut))
                }
                i += 1
            }
        }
        return out
    }

    /// `.keyboardShortcut("k", modifiers: [.command, .shift])` → "⌘⇧K"; `.keyboardShortcut("o")` → "⌘O";
    /// `.keyboardShortcut(.upArrow, modifiers: [.command, .option])` → "⌘⌥↑"; `.leftArrow` → "⌘⌥←".
    static func spell(keyboardShortcutLine line: String) -> String? {
        let key: String
        let rest: Substring
        if let r = line.range(of: ".keyboardShortcut(\"") {
            let after = line[r.upperBound...]
            guard let q = after.firstIndex(of: "\"") else { return nil }
            key = String(after[..<q]).uppercased()
            rest = after[q...]
        } else if line.contains(".keyboardShortcut(.upArrow") {
            key = "↑"
            rest = line[...]
        } else if line.contains(".keyboardShortcut(.downArrow") {
            key = "↓"
            rest = line[...]
        } else if line.contains(".keyboardShortcut(.leftArrow") {
            key = "←"
            rest = line[...]
        } else if line.contains(".keyboardShortcut(.rightArrow") {
            key = "→"
            rest = line[...]
        } else {
            return nil
        }
        var mods = "⌘"
        if let m = rest.range(of: "modifiers: [") {
            let list = rest[m.upperBound...]
            mods = ""
            if list.contains(".control") { mods += "⌃" }
            if list.contains(".command") { mods += "⌘" }
            if list.contains(".option") { mods += "⌥" }
            if list.contains(".shift") { mods += "⇧" }
        }
        return mods + key
    }

    func testMenuItemsMatchTheShellWiring() throws {
        let wired = try wiredItems()
        XCTAssertGreaterThan(wired.count, 20, "menu items parsed from the shell source")
        for e in AccessibilityCommand.entries {
            guard let item = e.menuItem else {
                // Undo and Settings (⌘,) are system items; completion, toggle
                // comment, signature help, the preview click and the search
                // window's ⌘G are not menu items.
                XCTAssertTrue([.undo, .completion, .completionList, .toggleComment, .signatureHelp, .selectPreviewItemSource, .editorPreferences, .nextSearchMatch].contains(e.command), "\(e.command) has no menu item")
                continue
            }
            let matches = wired.filter { $0.title == item }
            XCTAssertEqual(matches.count, 1, "\(e.command): Button(\"\(item)\") wired once in the shell")
            guard let w = matches.first else { continue }
            XCTAssertTrue(e.menu.hasPrefix(w.menu), "\(e.command): table says menu \(e.menu), shell wires it under \(w.menu)")
            if let key = w.shortcut {
                XCTAssertEqual(e.shortcuts, [key], "\(e.command): shell key equivalent")
            } else {
                XCTAssertEqual(e.shortcuts, ["\(w.menu) > \(item)"], "\(e.command): no key equivalent, so the README names the menu path")
            }
            // "Requires" claims a .disabled() modifier and vice versa.
            let escaped = item.replacingOccurrences(of: "\\", with: "\\\\")
            let texts = try Self.commandFiles.map { try String(contentsOf: Self.shellSources.appendingPathComponent($0), encoding: .utf8) }
            let text = try XCTUnwrap(texts.first { $0.contains("Button(\"\(escaped)\")") }, item)
            let start = try XCTUnwrap(text.range(of: "Button(\"\(escaped)\")"))
            let tail = text[start.upperBound...]
            let next = tail.range(of: "Button(")?.lowerBound ?? tail.endIndex
            let disabled = tail[..<next].contains(".disabled(")
            XCTAssertEqual(e.requires != nil, disabled, "\(e.command): help says requires \(e.requires ?? "nothing"), wiring \(disabled ? "has" : "has no") .disabled()")
        }
        // Every shell item with a key equivalent is in the table (menu items
        // without one, e.g. Detach Worker, are not keyboard workflow steps).
        let tableItems = Set(AccessibilityCommand.entries.compactMap(\.menuItem))
        for w in wired where w.shortcut != nil {
            XCTAssertTrue(tableItems.contains(w.title), "shell item “\(w.title)” (\(w.shortcut!)) is missing from AccessibilityCommand")
        }
    }

    func testEntriesAreDiscoverable() {
        for e in AccessibilityCommand.entries {
            XCTAssertFalse(e.title.isEmpty)
            XCTAssertFalse(e.shortcuts.isEmpty, e.title)
            XCTAssertFalse(e.menu.isEmpty, e.title)
            XCTAssertGreaterThan(e.description.count, 20, e.title)
            XCTAssertTrue(e.helpLine.hasPrefix(e.title + " — "), e.helpLine)
        }
        XCTAssertEqual(AccessibilityCommand.helpLines.count, AccessibilityCommand.allCases.count)
        XCTAssertEqual(AccessibilityCommand.nextDiagnostic.entry.helpLine,
                       "Next diagnostic — ⌘⇧] (Navigate): Selects the next diagnostic with a source in the active document (wrapping); refused if its span was edited since the compile. Requires a compile result.")
        XCTAssertEqual(AccessibilityCommand.completion.entry.shortcuts, ["Esc", "⌃Space"])
        XCTAssertEqual(AccessibilityCommand.helpLines, AccessibilityCommand.helpLines, "deterministic")
    }

    // MARK: focus order

    /// Body text of `struct Name` in `ContentView.swift`, up to the next top-level struct.
    static func structBody(_ name: String, in text: String) throws -> Substring {
        let start = try XCTUnwrap(text.range(of: "struct \(name): View"), "\(name) exists in ContentView.swift")
        let rest = text[start.upperBound...]
        let end = rest.range(of: "\nprivate struct ")?.lowerBound ?? rest.range(of: "\nstruct ")?.lowerBound ?? rest.endIndex
        return rest[..<end]
    }

    func testFocusOrderMatchesContentViewPaneOrder() throws {
        let text = try String(contentsOf: Self.shellSources.appendingPathComponent("ContentView.swift"), encoding: .utf8)
        // 1. The window is the flat tool-window shell: the rail first, then the
        //    tool column, then the HSplitView places the editor column before
        //    the preview column, and the Problems panel follows both.
        let root = try Self.structBody("ContentView", in: text)
        let rail = try XCTUnwrap(root.range(of: "ToolRail("))
        let sidebar = try XCTUnwrap(root.range(of: "WorkspaceSidebar(projectVisible:"))
        let editor = try XCTUnwrap(root.range(of: "EditorPane()"))
        let preview = try XCTUnwrap(root.range(of: "PreviewPane()"))
        let problems = try XCTUnwrap(root.range(of: "ProblemsPanel()"))
        XCTAssertLessThan(rail.lowerBound, sidebar.lowerBound)
        XCTAssertLessThan(sidebar.lowerBound, editor.lowerBound)
        XCTAssertLessThan(editor.lowerBound, preview.lowerBound)
        XCTAssertLessThan(preview.lowerBound, problems.lowerBound)
        XCTAssertTrue(root[sidebar.upperBound..<editor.lowerBound].contains("HSplitView"))
        // 2. Within each container the markers appear in the table's order, and
        //    the table lists the containers in the window's order.
        let containers = ["ContentView", "WorkspaceSidebar", "EditorPane", "PreviewPane", "ProblemsPanel"]
        var flattened: [String] = []
        for c in containers {
            let panes = FocusOrder.panes.filter { $0.container == c }
            XCTAssertFalse(panes.isEmpty, c)
            let file = try XCTUnwrap(Set(panes.map(\.sourceFile)).count == 1 ? panes[0].sourceFile : nil, "\(c): one source file")
            let source = try String(contentsOf: Self.shellSources.appendingPathComponent(file), encoding: .utf8)
            let body = try Self.structBody(c, in: source)
            var cursor = body.startIndex
            for p in panes {
                let r = try XCTUnwrap(body.range(of: p.sourceMarker, range: cursor..<body.endIndex),
                                      "\(p.name): \(p.sourceMarker) after the previous pane in \(c)")
                cursor = r.upperBound
                flattened.append(p.name)
            }
        }
        XCTAssertEqual(flattened, FocusOrder.panes.map(\.name), "table order is the view order")
        XCTAssertEqual(Set(FocusOrder.panes.map(\.container)), Set(containers))
        // 3. No focusable pane in the source is missing from the table.
        for marker in ["ProjectSection()", "DocumentTabBar()", "SourceEditorView(", "CaptureBar()", "BridgeBar()", "PreviewView(", "problemsList("] {
            XCTAssertTrue(FocusOrder.panes.contains { $0.sourceMarker == marker }, marker)
        }
        XCTAssertEqual(FocusOrder.description, "Tool rail → Sidebar → Tabs → Editor → Capture bar → Bridge bar → Preview → Problems")
        // The tool column stacks Project above Outline (Problems left the
        // column: its homes are the bottom panel and the status-bar badge)
        // and the Problems panel hosts the shared diagnostics list.
        let sidebarSource = try String(contentsOf: Self.shellSources.appendingPathComponent("WorkspaceSidebar.swift"), encoding: .utf8)
        let sidebarBody = try Self.structBody("WorkspaceSidebar", in: sidebarSource)
        var cursor = sidebarBody.startIndex
        for section in ["ProjectSection()", "OutlineSection("] {
            let r = try XCTUnwrap(sidebarBody.range(of: section, range: cursor..<sidebarBody.endIndex), section)
            cursor = r.upperBound
        }
        XCTAssertFalse(sidebarBody.contains("ProblemsSection"), "Problems has left the tool column")
        let problemsSource = try String(contentsOf: Self.shellSources.appendingPathComponent("ProblemsPanel.swift"), encoding: .utf8)
        XCTAssertTrue(problemsSource.contains("DiagnosticsListView(diagnostics: diags, panel: model.problemsPanel"), "the Problems panel hosts the diagnostics list")
    }

    func testFocusOrderHelpText() {
        XCTAssertEqual(FocusOrder.panes.count, 8)
        for p in FocusOrder.panes {
            XCTAssertGreaterThan(p.rationale.count, 30, p.name)
            XCTAssertGreaterThan(p.contents.count, 20, p.name)
        }
        XCTAssertTrue(FocusOrder.helpLines[0].hasPrefix("1. Tool rail: "))
        XCTAssertTrue(FocusOrder.helpLines[1].hasPrefix("2. Sidebar: "))
        XCTAssertTrue(FocusOrder.helpLines[3].hasPrefix("4. Editor: "))
        XCTAssertTrue(FocusOrder.helpLines[7].hasPrefix("8. Problems: "))
        XCTAssertEqual(FocusOrder.statusLines.count, 3)
        XCTAssertTrue(FocusOrder.statusLines[0].hasPrefix("Toolbar: "))
        XCTAssertTrue(FocusOrder.statusLines[0].contains("⌘⇧P"), "the palette is discoverable from the toolbar line")
    }

    // MARK: help window

    func testHelpViewCoversEveryCommandAndMenu() {
        let menus = AccessibilityHelpView.menus
        XCTAssertEqual(menus.map(\.menu), ["FlashTeX", "File", "File / toolbar", "Edit", "View", "Editor", "Navigate", "Preview", "Help", "Find in Project window"])
        XCTAssertEqual(menus.flatMap(\.entries).count, AccessibilityCommand.allCases.count)
        XCTAssertEqual(AccessibilityHelpView.windowID, "a11y-help")
        XCTAssertEqual(AccessibilityHelpView.menuItem, "FlashTeX Accessibility Help")
        // The VoiceOver notes cover each focusable pane and the completion popup.
        let notes = AccessibilityHelpView.voiceOverNotes.joined(separator: "\n")
        for p in FocusOrder.panes where p.name != "Bridge bar" { XCTAssertTrue(notes.contains(p.name + ":"), p.name) }
        XCTAssertTrue(notes.contains("Completion popup"))
        XCTAssertTrue(notes.contains("Command palette (⌘⇧P)"))
        XCTAssertTrue(notes.contains(CompletionAccessibility.listLabel))
        XCTAssertTrue(notes.contains(PreviewAccessibility.goToSourceAction))
        XCTAssertTrue(notes.contains(DiagnosticRowAccessibility.noSourceHint))
        // Each panel has a VoiceOver note and a focus-order line.
        for p in PanelFocusOrder.panels {
            XCTAssertTrue(notes.contains(p.name + " ("), p.name)
            XCTAssertTrue(PanelFocusOrder.helpLines.contains { $0.hasPrefix(p.name + " (") }, p.name)
        }
    }

    // MARK: panel focus order

    /// The panel tables list every interactive control of the four panel
    /// views in source order (SwiftUI's Tab order): every marker is found in
    /// that order, every `Button(`/`Toggle(`/`Picker(`/`Slider(`/`Stepper(`/
    /// `TextField(`/`List(` declaration of the view body is covered by a
    /// marker (the search window's hidden Esc button is exempt), and each
    /// panel's command is in the command table with its window's shortcut.
    func testPanelFocusOrderMatchesThePanelSources() throws {
        XCTAssertEqual(PanelFocusOrder.panels.map(\.name), ["Settings", "Durable History", "Find in Project", "Nearby Companion"])
        let controlPattern = try NSRegularExpression(pattern: #"\b(Button|Toggle|Picker|Slider|Stepper|TextField|List)\("#)
        for panel in PanelFocusOrder.panels {
            let text = try String(contentsOf: Self.shellSources.appendingPathComponent(panel.sourceFile), encoding: .utf8)
            // 1. Markers appear in table order.
            var cursor = text.startIndex
            var markerOffsets: [Int] = []
            for c in panel.controls {
                let r = try XCTUnwrap(text.range(of: c.sourceMarker, range: cursor..<text.endIndex),
                                      "\(panel.name): \(c.name) marker “\(c.sourceMarker)” after the previous control")
                cursor = r.upperBound
                markerOffsets.append(text.distance(from: text.startIndex, to: r.lowerBound))
                XCTAssertFalse(c.name.isEmpty)
            }
            // 2. Every control declaration in the view body is covered: it is
            //    a marker itself, or the next marker after it is an
            //    accessibilityIdentifier line with no other control between.
            let body = Self.viewBody(of: panel, in: text)
            let bodyOffset = text.distance(from: text.startIndex, to: text.range(of: body)!.lowerBound)
            let ns = body as NSString
            let matches = controlPattern.matches(in: body, range: NSRange(location: 0, length: ns.length))
            for (k, m) in matches.enumerated() {
                let at = bodyOffset + body.distance(from: body.startIndex, to: Range(m.range, in: body)!.lowerBound)
                let nextAt = k + 1 < matches.count
                    ? bodyOffset + body.distance(from: body.startIndex, to: Range(matches[k + 1].range, in: body)!.lowerBound)
                    : Int.max
                let snippet = ns.substring(with: NSRange(location: m.range.location, length: min(48, ns.length - m.range.location)))
                let region = text[text.index(text.startIndex, offsetBy: at)..<text.index(text.startIndex, offsetBy: min(nextAt, text.count))]
                if region.contains(".hidden()") { continue } // the search window's Esc close button
                let covered = markerOffsets.contains { $0 >= at && $0 < nextAt }
                XCTAssertTrue(covered, "\(panel.name): control “\(snippet.replacingOccurrences(of: "\n", with: " "))” has no PanelFocusOrder entry")
            }
            // 3. The panel's command is in the table; its shortcut leads the help line.
            XCTAssertTrue(AccessibilityCommand.allCases.contains(panel.command))
            XCTAssertTrue(panel.helpLine.contains(panel.command.entry.shortcuts[0]))
        }
        XCTAssertEqual(PanelFocusOrder.helpLines.count, 4)
        XCTAssertTrue(PanelFocusOrder.helpLines[3].hasPrefix("Nearby Companion (⌘⇧N): opens with focus on Show Pairing Code (Return)"), PanelFocusOrder.helpLines[3])
        XCTAssertTrue(PanelFocusOrder.helpLines[3].contains("Advertise on the local network (switch) → "), PanelFocusOrder.helpLines[3])
        XCTAssertTrue(PanelFocusOrder.helpLines[0].hasPrefix("Settings (⌘,): opens with focus on the font family pop-up. Tab order: Editor font family → "))
    }

    /// The text of the panel's SwiftUI view declarations: from the first view
    /// struct to the file's scene/commands types (which wire menu items, not
    /// panel controls).
    static func viewBody(of panel: PanelFocusOrder.Panel, in text: String) -> String {
        let starts = ["struct EditorPreferencesView", "struct EditHistoryPanel", "struct ProjectSearchPanel", "struct NearbyView"]
        guard let start = starts.compactMap({ text.range(of: $0)?.lowerBound }).min() else { return text }
        let rest = text[start...]
        let end = rest.range(of: ": Scene {")?.lowerBound ?? rest.range(of: ": Commands {")?.lowerBound ?? rest.endIndex
        return String(rest[..<end])
    }
}
