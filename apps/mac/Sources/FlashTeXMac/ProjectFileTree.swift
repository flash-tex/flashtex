import AppKit
import SwiftUI

/// The Project tool window's tree mode (owner request 2026-10-03): the same
/// rows `ProjectSection.rows` lists flat — open members, closed includes,
/// package inputs, missing includes — grouped under expandable folder rows
/// built from their project-relative paths. Nothing here touches the disk:
/// the tree is a pure function of the rows the model already produced.
enum ProjectFileTree {
    /// One file row and the path that places it in the tree.
    struct Item: Equatable {
        var path: String
        var row: SidebarTree.Row
    }

    /// Folder row ids: `folder:` + the folder's path under the tree's base.
    static let folderPrefix = "folder:"

    /// Builds the tree: folders first, then files, each level in natural
    /// (Finder) order. A file row's title becomes its file name (any suffix
    /// after the path, such as "— missing, create", is kept); its tooltip
    /// leads with the full relative path. Absolute paths and paths that climb
    /// out of the project (`..`) stay at the top level under their full path.
    /// `idPrefix` names folder ids (a subtree such as resolved packages uses
    /// its own, so its folders never collide with the project's).
    static func build(_ items: [Item], idPrefix: String = folderPrefix) -> [SidebarTree.Row] {
        let root = Folder(name: "", path: "")
        for item in items {
            let parts = components(of: item.path)
            guard let parts, let leaf = parts.last else {
                root.files.append(leafRow(item, name: item.path))
                continue
            }
            var folder = root
            for name in parts.dropLast() { folder = folder.child(named: name) }
            folder.files.append(leafRow(item, name: leaf))
        }
        return root.rows(idPrefix: idPrefix)
    }

    /// Path components with empty and `.` segments dropped; nil when the
    /// path is absolute or climbs out of the project with `..`.
    static func components(of path: String) -> [String]? {
        if path.hasPrefix("/") || path.hasPrefix("~") { return nil }
        let parts = path.split(separator: "/", omittingEmptySubsequences: true).map(String.init).filter { $0 != "." }
        if parts.isEmpty || parts.contains("..") { return nil }
        return parts
    }

    /// The folder ids holding `path`, outermost first (what auto-expands).
    static func ancestorIDs(of path: String, idPrefix: String = folderPrefix) -> [String] {
        guard let parts = components(of: path) else { return [] }
        return parts.dropLast().indices.map { idPrefix + parts[...$0].joined(separator: "/") }
    }

    /// Finder's order: case-insensitive, numbers by value (`ch2` < `ch10`).
    static func precedes(_ a: String, _ b: String) -> Bool {
        switch a.localizedStandardCompare(b) {
        case .orderedAscending: return true
        case .orderedDescending: return false
        case .orderedSame: return a < b
        }
    }

    private static func leafRow(_ item: Item, name: String) -> SidebarTree.Row {
        var row = item.row
        if row.title.hasPrefix(item.path) {
            row.title = name + row.title.dropFirst(item.path.count)
        }
        if let tip = row.tooltip {
            if !tip.hasPrefix(item.path) { row.tooltip = item.path + "\n" + tip }
        } else {
            row.tooltip = item.path
        }
        row.indent = 0
        return row
    }

    private final class Folder {
        let name: String
        let path: String
        var folders: [String: Folder] = [:]
        var files: [SidebarTree.Row] = []

        init(name: String, path: String) { self.name = name; self.path = path }

        func child(named name: String) -> Folder {
            if let f = folders[name] { return f }
            let f = Folder(name: name, path: path.isEmpty ? name : path + "/" + name)
            folders[name] = f
            return f
        }

        func rows(idPrefix: String) -> [SidebarTree.Row] {
            let subfolders = folders.values.sorted { ProjectFileTree.precedes($0.name, $1.name) }.map { $0.row(idPrefix: idPrefix) }
            let leaves = files.sorted { a, b in
                a.title == b.title ? a.id < b.id : ProjectFileTree.precedes(a.title, b.title)
            }
            return subfolders + leaves
        }

        func row(idPrefix: String) -> SidebarTree.Row {
            let children = rows(idPrefix: idPrefix)
            let fileCount = countFiles()
            // Folders are structure, not state: every folder title reads in
            // the primary text colour. (They used to dim when every file under
            // them was a closed include, so the few holding an open file stood
            // out as if bold — owner report 2026-10-04.) File rows keep their
            // own dimming.
            return SidebarTree.Row(
                id: idPrefix + path,
                icon: "folder",
                iconColor: DS.Palette.textSecondary,
                title: name,
                tooltip: path,
                accessibilityLabel: "\(name), folder, \(fileCount) file\(fileCount == 1 ? "" : "s")",
                children: children)
        }

        private func countFiles() -> Int { files.count + folders.values.reduce(0) { $0 + $1.countFiles() } }
    }
}

// MARK: - setting

/// The tree/flat switch (Settings > Editor > Sidebar and the Project header).
enum ProjectTreeMode {
    static let defaultsKey = "FlashTeX.sidebar.projectFolderTree"
    /// On by default: folders as a tree; off is the flat list of paths.
    static let defaultValue = true
}

/// Folder expansion per project, keyed by the project root's path.
enum ProjectTreeExpansion {
    static let defaultsKey = "FlashTeX.sidebar.projectTreeExpanded"
    /// Projects remembered; the least recently changed is dropped first.
    static let limit = 64

    static func load(scope: String, defaults: UserDefaults = .standard) -> Set<String> {
        guard !scope.isEmpty, let all = defaults.dictionary(forKey: defaultsKey),
              let entry = all[scope] as? [String: Any], let ids = entry["ids"] as? [String] else { return [] }
        return Set(ids)
    }

    /// Whether a project folder id (`folder:` + rooted path) names a folder
    /// on disk: such an id survives pruning while the tree has not listed the
    /// folder yet (the closure is still being discovered); a deleted or
    /// renamed folder's id does not.
    static func folderExists(id: String, root: URL?) -> Bool {
        guard let root, id.hasPrefix(ProjectFileTree.folderPrefix) else { return false }
        let path = String(id.dropFirst(ProjectFileTree.folderPrefix.count))
        guard ProjectFileTree.components(of: path) != nil else { return false }
        var isDirectory: ObjCBool = false
        return FileManager.default.fileExists(atPath: root.appendingPathComponent(path).path, isDirectory: &isDirectory)
            && isDirectory.boolValue
    }

    static func save(_ ids: Set<String>, scope: String, defaults: UserDefaults = .standard, now: Date = Date()) {
        guard !scope.isEmpty else { return }
        var all = defaults.dictionary(forKey: defaultsKey) ?? [:]
        all[scope] = ["ids": ids.sorted(), "at": now.timeIntervalSince1970] as [String: Any]
        if all.count > limit {
            let stamp = { (k: String) in ((all[k] as? [String: Any])?["at"] as? Double) ?? 0 }
            for key in all.keys.sorted(by: { stamp($0) < stamp($1) }).prefix(all.count - limit) { all[key] = nil }
        }
        defaults.set(all, forKey: defaultsKey)
    }
}

/// Settings > Editor > Sidebar.
struct ProjectTreeSettingsRows: View {
    @AppStorage(ProjectTreeMode.defaultsKey) private var tree = ProjectTreeMode.defaultValue

    var body: some View {
        Toggle("Show project files as a folder tree", isOn: $tree)
            .accessibilityHint("On groups the Project sidebar's files under expandable folders; off lists every file by its full path.")
        Text("Off lists each file by its path relative to the project, in one flat list.")
            .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
    }
}
