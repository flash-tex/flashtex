import AppKit
import Foundation

/// "Reveal in Finder" (sidebar row context menu, #691), File > Show in Finder
/// (⌘⌥R, the active document) and "Reveal Project in Finder" (the tree's
/// empty space, #870). Resolves a project-relative path to its on-disk URL
/// through the same rooted symlink/escape protections
/// `ProjectDocuments.rootedFile` applies to every other read, so the action
/// can never reach outside the project root.
enum RevealInFinder {
    /// Replaceable so tests never call `NSWorkspace`
    /// (DisplayListLinks.swift's `openURL` is the same pattern).
    static var activateFileViewerSelecting: ([URL]) -> Void = { NSWorkspace.shared.activateFileViewerSelecting($0) }

    /// The file to reveal for `path` under `root`, or nil when there is
    /// nothing to show Finder: no project root yet (the entry document was
    /// never saved, so `root` is nil), the path is refused by the rooted-file
    /// check (a symlink on the way, or it resolves outside the root), or the
    /// file no longer exists on disk (deleted or moved out from under the app).
    static func target(path: String, root: URL?) -> URL? {
        guard let root else { return nil }
        guard case .file(let url) = ProjectDocuments.rootedFile(path, under: root) else { return nil }
        guard FileManager.default.fileExists(atPath: url.path) else { return nil }
        return url
    }

    /// Reveals `path` in Finder, selected, if it resolves to a real file
    /// under `root` at the time of the call. Returns whether it did, so a
    /// caller can tell a click that raced a delete/move from an actual reveal.
    @discardableResult
    static func reveal(path: String, root: URL?) -> Bool {
        guard let url = target(path: path, root: root) else { return false }
        activateFileViewerSelecting([url])
        return true
    }

    /// Reveals the project root folder itself, selected in its parent, if
    /// there is one and it still exists on disk (#870). Returns whether it did.
    @discardableResult
    static func revealRoot(_ root: URL?) -> Bool {
        guard let root, FileManager.default.fileExists(atPath: root.path) else { return false }
        activateFileViewerSelecting([root])
        return true
    }
}

extension ShellModel {
    /// File > Show in Finder (⌘⌥R) and the palette's Show in Finder: the
    /// active document's file, selected. The menu item is disabled without a
    /// project root; a document that is not on disk (never saved, deleted or
    /// moved out from under the app) gets a note instead of a silent no-op.
    func showActiveDocumentInFinder() {
        if !RevealInFinder.reveal(path: activePath, root: project.projectRoot) {
            navigationNote = "\(activePath) is not on disk, so there is nothing to show in Finder."
        }
    }
}
