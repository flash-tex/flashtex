import XCTest
import AppKit
import SwiftUI
import HostedWindows
@testable import FlashTeXMac

/// The Project tool window's tree mode (ProjectFileTree.swift): building the
/// folder tree from the flat rows' paths, the drop/drag mapping of folder
/// rows, per-project expansion persistence, and — hosted — the real
/// `ProjectSection` keeping the selection across a mode switch.
@MainActor
final class ProjectFileTreeTests: XCTestCase {
    private func row(_ path: String, id: String? = nil, title: String? = nil, dimmed: Bool = false, tooltip: String? = nil) -> ProjectFileTree.Item {
        .init(path: path, row: SidebarTree.Row(id: id ?? path, icon: "doc.text", iconColor: .labelColor,
                                                title: title ?? path, dimmed: dimmed, tooltip: tooltip))
    }

    /// Flattens the tree depth-first as "depth:title" for compact asserts.
    private func outline(_ rows: [SidebarTree.Row], depth: Int = 0) -> [String] {
        rows.flatMap { r in ["\(depth):\(r.title)\(r.children == nil ? "" : "/")"] + outline(r.children ?? [], depth: depth + 1) }
    }

    private func find(_ id: String, in rows: [SidebarTree.Row]) -> SidebarTree.Row? {
        for r in rows {
            if r.id == id { return r }
            if let hit = find(id, in: r.children ?? []) { return hit }
        }
        return nil
    }

    // MARK: building

    func testNestingAndFoldersFirstNaturalOrder() {
        let tree = ProjectFileTree.build([
            row("main.tex"),
            row("book/includes/_includes.tex"),
            row("book/front-matter/titlepage.tex"),
            row("book/chapters/ch10.tex"),
            row("book/chapters/ch2.tex"),
            row("book/chapters/Ch1.tex"),
            row("refs.bib"),
            row("book/preface.tex"),
            row("Appendix/a.tex"),
        ])
        XCTAssertEqual(outline(tree), [
            "0:Appendix/", "1:a.tex",
            "0:book/",
            "1:chapters/", "2:Ch1.tex", "2:ch2.tex", "2:ch10.tex",
            "1:front-matter/", "2:titlepage.tex",
            "1:includes/", "2:_includes.tex",
            "1:preface.tex",
            "0:main.tex", "0:refs.bib",
        ])
        let book = try! XCTUnwrap(find("folder:book", in: tree))
        XCTAssertEqual(book.tooltip, "book")
        XCTAssertEqual(book.icon, "folder")
        XCTAssertTrue(book.selectable, "folders take the selection so the arrows expand/collapse them")
        XCTAssertEqual(book.accessibilityLabel, "book, folder, 6 files")
        XCTAssertNotNil(find("folder:book/front-matter", in: tree))
        // File rows keep their ids (the model's paths), so selection, menus,
        // drag and activation are unchanged.
        XCTAssertEqual(find("book/includes/_includes.tex", in: tree)?.title, "_includes.tex")
        XCTAssertEqual(find("book/includes/_includes.tex", in: tree)?.tooltip, "book/includes/_includes.tex")
    }

    func testSingleFileFoldersAreNotCollapsedIntoOneRow() {
        let tree = ProjectFileTree.build([row("a/b/c/only.tex")])
        XCTAssertEqual(outline(tree), ["0:a/", "1:b/", "2:c/", "3:only.tex"])
        XCTAssertEqual(tree[0].id, "folder:a")
        XCTAssertEqual(tree[0].children?[0].id, "folder:a/b")
        XCTAssertEqual(tree[0].children?[0].children?[0].id, "folder:a/b/c")
        XCTAssertEqual(ProjectFileTree.ancestorIDs(of: "a/b/c/only.tex"), ["folder:a", "folder:a/b", "folder:a/b/c"])
        XCTAssertEqual(ProjectFileTree.ancestorIDs(of: "main.tex"), [])
    }

    func testRootFilesOnlyStayAFlatLevel() {
        let tree = ProjectFileTree.build([row("main.tex"), row("intro.tex"), row("refs.bib")])
        XCTAssertEqual(outline(tree), ["0:intro.tex", "0:main.tex", "0:refs.bib"])
        XCTAssertTrue(tree.allSatisfy { $0.children == nil })
        XCTAssertTrue(ProjectFileTree.build([]).isEmpty)
    }

    func testOddPaths() {
        let tree = ProjectFileTree.build([
            row("my chapters/ü ñ (draft) #1.tex"),
            row("./dot.tex"),
            row("a//double.tex"),
            row("../shared/outside.sty"),
            row("/abs/p.sty"),
            row("weird:colon/x.tex"),
            row("emoji 📚/n.tex"),
            row("trailing/"),
        ])
        let lines = outline(tree)
        XCTAssertEqual(Array(lines.prefix(8)), [
            "0:a/", "1:double.tex",
            "0:emoji 📚/", "1:n.tex",
            "0:my chapters/", "1:ü ñ (draft) #1.tex",
            "0:weird:colon/", "1:x.tex",
        ], "folders first, in Finder order")
        XCTAssertEqual(Set(lines.dropFirst(8)), ["0:/abs/p.sty", "0:../shared/outside.sty", "0:dot.tex", "0:trailing"])
        // Out-of-project paths keep their full path at the top level.
        XCTAssertEqual(find("../shared/outside.sty", in: tree)?.title, "../shared/outside.sty")
        XCTAssertEqual(find("/abs/p.sty", in: tree)?.title, "/abs/p.sty")
        XCTAssertNotNil(find("folder:my chapters", in: tree))
        XCTAssertEqual(find("./dot.tex", in: tree)?.title, "dot.tex")
        XCTAssertEqual(find("a//double.tex", in: tree)?.title, "double.tex")
        XCTAssertEqual(find("folder:weird:colon", in: tree)?.children?.first?.title, "x.tex")
        // A path ending in "/" is a file row named after its last component.
        XCTAssertEqual(find("trailing/", in: tree)?.title, "trailing")
        XCTAssertNil(ProjectFileTree.components(of: "../x.tex"))
        XCTAssertNil(ProjectFileTree.components(of: "/x.tex"))
        XCTAssertEqual(ProjectFileTree.components(of: "./a/./b.tex"), ["a", "b.tex"])
    }

    func testTitleSuffixTooltipAndDimmedFolders() {
        let tree = ProjectFileTree.build([
            row("ch/missing.tex", id: "missing:ch/missing:main.tex", title: "ch/missing.tex — missing, create", dimmed: true,
                tooltip: "\\input{ch/missing} from main.tex has no file"),
            row("ch/closed.tex", id: "closed:ch/closed.tex", dimmed: true, tooltip: "ch/closed.tex · entry document"),
            row("open/a.tex"),
        ])
        let missing = try! XCTUnwrap(find("missing:ch/missing:main.tex", in: tree))
        XCTAssertEqual(missing.title, "missing.tex — missing, create")
        XCTAssertEqual(missing.tooltip, "ch/missing.tex\n\\input{ch/missing} from main.tex has no file", "the full path leads the tooltip")
        XCTAssertEqual(find("closed:ch/closed.tex", in: tree)?.tooltip, "ch/closed.tex · entry document", "already led by the path")
        XCTAssertEqual(find("folder:ch", in: tree)?.dimmed, true, "every file under it is outside the compile")
        XCTAssertEqual(find("folder:open", in: tree)?.dimmed, false)
    }

    func testHundredsOfFilesBuildQuickly() {
        var items: [ProjectFileTree.Item] = []
        for part in 0..<10 { for ch in 0..<50 { items.append(row("book/part\(part)/chapter\(ch)/body.tex")) } }
        let start = Date()
        let tree = ProjectFileTree.build(items)
        XCTAssertLessThan(Date().timeIntervalSince(start), 0.5)
        XCTAssertEqual(tree.count, 1)
        XCTAssertEqual(tree[0].children?.count, 10)
        XCTAssertEqual(tree[0].children?[0].children?.map(\.title).prefix(3), ["chapter0", "chapter1", "chapter2"])
        XCTAssertEqual(tree[0].children?[0].children?.last?.title, "chapter49")
    }

    // MARK: drag/drop and persistence

    func testFolderRowsAreDropTargetsButNeverDragged() {
        XCTAssertEqual(ProjectTreeMove.dropFolder(rowID: "folder:book/includes"), "book/includes")
        XCTAssertNil(ProjectTreeMove.path(forRowID: "folder:book"))
        XCTAssertNil(ProjectTreeMove.path(forRowID: "packages:folder:packages/cancel"))
        XCTAssertNil(ProjectTreeMove.dropFolder(rowID: "packages:group"))
        XCTAssertEqual(MoveTarget.resolve(path: "main/a.tex", intoFolder: ProjectTreeMove.dropFolder(rowID: "folder:book")!), .success("book/a.tex"))
    }

    func testExpansionPersistsPerProject() throws {
        let suite = "flashtex-tree-\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        XCTAssertEqual(ProjectTreeExpansion.load(scope: "/p/one", defaults: defaults), [])
        ProjectTreeExpansion.save(["folder:book", "folder:book/includes"], scope: "/p/one", defaults: defaults, now: Date(timeIntervalSince1970: 1))
        ProjectTreeExpansion.save(["folder:x"], scope: "/p/two", defaults: defaults, now: Date(timeIntervalSince1970: 2))
        XCTAssertEqual(ProjectTreeExpansion.load(scope: "/p/one", defaults: defaults), ["folder:book", "folder:book/includes"])
        XCTAssertEqual(ProjectTreeExpansion.load(scope: "/p/two", defaults: defaults), ["folder:x"])
        ProjectTreeExpansion.save([], scope: "", defaults: defaults) // untitled: never stored
        XCTAssertEqual((defaults.dictionary(forKey: ProjectTreeExpansion.defaultsKey) ?? [:]).count, 2)
        for i in 0..<(ProjectTreeExpansion.limit + 5) {
            ProjectTreeExpansion.save(["folder:a"], scope: "/p/n\(i)", defaults: defaults, now: Date(timeIntervalSince1970: Double(1000 + i)))
        }
        XCTAssertEqual((defaults.dictionary(forKey: ProjectTreeExpansion.defaultsKey) ?? [:]).count, ProjectTreeExpansion.limit)
        XCTAssertEqual(ProjectTreeExpansion.load(scope: "/p/one", defaults: defaults), [], "the oldest project is forgotten first")
        XCTAssertEqual(ProjectTreeExpansion.load(scope: "/p/n\(ProjectTreeExpansion.limit + 4)", defaults: defaults), ["folder:a"])
    }

    // MARK: the model's rows

    private var tmp: URL!

    override func setUp() {
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("tree-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: tmp)
        UserDefaults.standard.removeObject(forKey: ProjectTreeMode.defaultsKey)
    }

    /// A book-shaped project on disk: the entry includes four files in nested
    /// folders; `book/front-matter/titlepage.tex` is opened and active.
    private func bookProject() async throws -> (ShellModel, URL) {
        let m = ShellModel()
        m.detachWorker()
        let root = tmp.appendingPathComponent("book project")
        let files = [
            "book/includes/_includes.tex": "\\newcommand{\\x}{x}\n",
            "book/front-matter/titlepage.tex": "Title\n",
            "book/chapters/ch2.tex": "Two\n",
            "book/chapters/ch10.tex": "Ten\n",
        ]
        for (path, text) in files {
            let url = root.appendingPathComponent(path)
            try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try text.write(to: url, atomically: true, encoding: .utf8)
        }
        let entry = root.appendingPathComponent("main.tex")
        try """
        \\documentclass{book}
        \\input{book/includes/_includes}
        \\begin{document}
        \\input{book/front-matter/titlepage}
        \\input{book/chapters/ch2}
        \\input{book/chapters/ch10}
        \\end{document}
        """.write(to: entry, atomically: true, encoding: .utf8)
        XCTAssertEqual(m.openTex(at: entry), .opened)
        let ok = await m.openAndSwitch("book/front-matter/titlepage.tex", role: .included(from: "main.tex")) { XCTFail($0) }
        XCTAssertTrue(ok)
        m.flushChrome()
        return (m, root)
    }

    func testProjectSectionTreeRowsGroupTheModelsRows() async throws {
        let (m, _) = try await bookProject()
        XCTAssertEqual(m.activePath, "book/front-matter/titlepage.tex")
        let args = (listing: m.project.listing, kinds: m.documentKinds, closure: m.project.discoverClosure())
        let flat = ProjectSection.rows(listing: args.listing, kinds: args.kinds, closure: args.closure, packages: m.manifest.rows, activePath: m.activePath)
        let tree = ProjectSection.treeRows(listing: args.listing, kinds: args.kinds, closure: args.closure, packages: m.manifest.rows, activePath: m.activePath)
        XCTAssertEqual(flat.map(\.title), ["main.tex", "book/front-matter/titlepage.tex",
                                           "book/includes/_includes.tex", "book/chapters/ch2.tex", "book/chapters/ch10.tex"],
                       "flat mode is today's list")
        XCTAssertEqual(outline(tree), [
            "0:book/",
            "1:chapters/", "2:ch2.tex", "2:ch10.tex",
            "1:front-matter/", "2:titlepage.tex",
            "1:includes/", "2:_includes.tex",
            "0:main.tex",
        ])
        // The same file rows, same ids/icons/states — only the title and
        // tooltip differ.
        let leaves = flat.map { f in (f, find(f.id, in: tree)) }
        for (f, t) in leaves {
            let t = try XCTUnwrap(t, f.id)
            XCTAssertEqual(t.icon, f.icon, f.id)
            XCTAssertEqual(t.dimmed, f.dimmed, f.id)
            XCTAssertEqual(t.accessibilityLabel, f.accessibilityLabel, f.id)
        }
        XCTAssertEqual(find("main.tex", in: tree)?.icon, "doc.text.fill", "the root document keeps its badge")
        XCTAssertEqual(find("closed:book/chapters/ch2.tex", in: tree)?.dimmed, true, "a closed include stays dimmed")
        XCTAssertEqual(find("folder:book/chapters", in: tree)?.dimmed, true, "a folder of closed includes reads dimmed")
        XCTAssertEqual(find("folder:book/front-matter", in: tree)?.dimmed, false)
        m.files.detachHelper()
    }

    // MARK: hosted: the real ProjectSection

    private static func descendants(_ v: NSView) -> [NSView] { v.subviews + v.subviews.flatMap(descendants) }

    private func selectedID(_ outline: NSOutlineView) -> String? {
        guard outline.selectedRow >= 0 else { return nil }
        return (outline.item(atRow: outline.selectedRow) as? SidebarTree.Coordinator.Node)?.row.id
    }

    private func waitFor(_ what: String, _ condition: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(10)
        while !condition(), Date() < deadline { try await Task.sleep(nanoseconds: 30_000_000) }
        XCTAssertTrue(condition(), what)
    }

    func testTogglingTheModeKeepsTheSelectionAndRevealsTheActiveFile() async throws {
        let (m, root) = try await bookProject()
        let scope = m.project.projectRoot?.path ?? ""
        defer {
            if var all = UserDefaults.standard.dictionary(forKey: ProjectTreeExpansion.defaultsKey) {
                all[scope] = nil
                UserDefaults.standard.set(all, forKey: ProjectTreeExpansion.defaultsKey)
            }
        }
        XCTAssertFalse(scope.isEmpty)
        XCTAssertTrue(scope.hasSuffix(root.lastPathComponent))
        UserDefaults.standard.set(true, forKey: ProjectTreeMode.defaultsKey)

        let hostView = NSHostingView(rootView: ProjectSection().environment(m))
        hostView.frame = NSRect(x: 0, y: 0, width: 280, height: 420)
        let window = HostedWindowSupport.window(contentRect: hostView.frame, styleMask: [.titled, .closable])
        window.isReleasedWhenClosed = false
        window.contentView = hostView
        window.orderFrontRegardless() // never makeKey
        defer { window.orderOut(nil) }
        hostView.layoutSubtreeIfNeeded()

        var found: NSOutlineView?
        try await waitFor("the Project tree is hosted") {
            found = Self.descendants(hostView).compactMap { $0 as? NSOutlineView }.first { $0.accessibilityLabel() == "Project tree" }
            return found != nil
        }
        let outline = try XCTUnwrap(found)
        let active = "book/front-matter/titlepage.tex"
        let coordinator = try XCTUnwrap(outline.dataSource as? SidebarTree.Coordinator)
        func node(_ id: String) -> SidebarTree.Coordinator.Node? { coordinator.nodes[id] }

        // Tree mode: the folders holding the active file opened; the
        // selection is the active file; the sibling folders stay closed.
        try await waitFor("tree rows") { node("folder:book") != nil }
        try await waitFor("active file selected in the tree") { self.selectedID(outline) == active }
        XCTAssertTrue(outline.isItemExpanded(node("folder:book")))
        XCTAssertTrue(outline.isItemExpanded(node("folder:book/front-matter")))
        XCTAssertFalse(outline.isItemExpanded(node("folder:book/includes")))
        XCTAssertEqual(outline.level(forRow: outline.selectedRow), 2)
        XCTAssertGreaterThan(outline.indentationPerLevel, 0)
        XCTAssertEqual(ProjectTreeExpansion.load(scope: scope), ["folder:book", "folder:book/front-matter"], "auto-expansion persists")
        // Accessibility: a real outline (rows report disclosure levels).
        XCTAssertEqual(outline.accessibilityRole(), .outline)

        // Flat mode: the old list, same selection.
        UserDefaults.standard.set(false, forKey: ProjectTreeMode.defaultsKey)
        try await waitFor("flat rows") { node("folder:book") == nil && coordinator.roots.count == 5 }
        XCTAssertEqual(selectedID(outline), active, "switching to flat keeps the selection")
        XCTAssertEqual(outline.level(forRow: outline.selectedRow), 0)
        XCTAssertEqual(outline.indentationPerLevel, 0)
        XCTAssertEqual(m.activePath, active, "a mode switch never switches documents")

        // Back to the tree: same selection, the user's expansion restored.
        UserDefaults.standard.set(true, forKey: ProjectTreeMode.defaultsKey)
        try await waitFor("tree rows again") { node("folder:book") != nil }
        try await waitFor("selection kept") { self.selectedID(outline) == active }
        XCTAssertTrue(outline.isItemExpanded(node("folder:book/front-matter")))

        // A folder takes the selection without switching documents, and a
        // user's collapse/expand persists.
        let includes = try XCTUnwrap(node("folder:book/includes"))
        outline.selectRowIndexes([outline.row(forItem: includes)], byExtendingSelection: false)
        outline.expandItem(includes)
        XCTAssertEqual(m.activePath, active)
        XCTAssertTrue(ProjectTreeExpansion.load(scope: scope).contains("folder:book/includes"))
        outline.collapseItem(includes)
        XCTAssertFalse(ProjectTreeExpansion.load(scope: scope).contains("folder:book/includes"))
        m.files.detachHelper()
    }
}
